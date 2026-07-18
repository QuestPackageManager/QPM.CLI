use std::{borrow::Cow, path::Path, time::Instant};

use color_eyre::eyre::{Context, Result, bail};
use itertools::Itertools;
use pubgrub::{DefaultStringReporter, PubGrubError, Reporter};
use qpm_package::models::{
    package::PackageConfig,
    shared_package::{SharedDependencyInfo, SharedPackageConfig},
};

use crate::{
    models::package::SharedPackageConfigExtensions,
    repository::{Repository, file::FileRepository},
    services::pubgrub::{PackageDependencyResolver, ResolvedDependency},
    terminal::colors::QPMColor,
};

pub struct PackageRestorer<'a> {
    shared_package: Cow<'a, SharedPackageConfig>,
    pub resolved_deps: Box<[ResolvedDependency]>,
}

impl<'a> PackageRestorer<'a> {
    pub fn shared_package(&self) -> &SharedPackageConfig {
        &self.shared_package
    }

    pub fn resolved_deps(&self) -> &[ResolvedDependency] {
        &self.resolved_deps
    }

    pub fn take_resolved_deps(self) -> Box<[ResolvedDependency]> {
        self.resolved_deps
    }

    /// Resolve dependencies for a package using pubgrub
    /// This will return an iterator of resolved dependencies
    /// The iterator will return every dependency required by the package
    pub fn resolve(config: PackageConfig, repository: &impl Repository) -> Result<Self> {
        let resolver = PackageDependencyResolver {
            root: &config,
            repo: repository,
        };
        let time = Instant::now();
        let result = match pubgrub::resolve(&resolver, config.id.clone(), config.version.clone()) {
            Ok(deps) => deps.into_iter().filter_map(|(id, version)| {
                if id == config.id && version == config.version {
                    return None;
                }

                repository
                    .get_package(&id, &version.into())
                    .expect("Failed to get package")
            }),

            Err(PubGrubError::NoSolution(tree)) => {
                let report = DefaultStringReporter::report(&tree);
                bail!("failed to resolve dependencies: \n{}", report);
            }
            Err(err) => {
                bail!("pubgrub: {err}\n{err:?}");
            }
        };

        let sw = time.elapsed();
        println!("Took {}ms to dependency resolve", sw.as_millis());

        let resolved_dependencies = result.collect_vec();

        let restored_dependencies = resolved_dependencies
            .iter()
            .map(|resolved_dep| {
                let dependency = config.get_dependency(&resolved_dep.config.id);
                let qpkg_url = dependency.and_then(|dep| dep.qpkg_url.clone());

                let shared_dependency_info = SharedDependencyInfo {
                    restored_version: resolved_dep.config.version.clone(),
                    qpkg_url,
                    qpkg_checksum: resolved_dep.qpkg_checksum.clone(),
                    restored_binaries: resolved_dep
                        .config
                        .workspace
                        .out_binaries
                        .clone()
                        .unwrap_or_default(),
                    restored_env: resolved_dep
                        .config
                        .workspace
                        .env
                        .clone()
                        .unwrap_or_default(),
                };
                (resolved_dep.config.id.clone(), shared_dependency_info)
            })
            .collect();

        let shared_package_config = SharedPackageConfig {
            env: config.workspace.env.clone().unwrap_or_default(),
            config,
            restored_dependencies,
        };

        Ok(Self {
            resolved_deps: resolved_dependencies.into(),
            shared_package: Cow::Owned(shared_package_config),
        })
    }

    /// TODO: DOc
    pub fn locked_resolve(
        shared_package: &'a SharedPackageConfig,
        repository: &impl Repository,
    ) -> Result<Self> {
        // TODO: ensure restored dependencies take precedence over
        let packages = shared_package
            .restored_dependencies
            .iter()
            .map(|(dep_id, dep)| {
                repository
                    .get_package(dep_id, &dep.restored_version)
                    .unwrap_or_else(|e| {
                        panic!(
                            "Encountered an issue resolving for package {}:{} {e:#?}",
                            dep_id.0, dep.restored_version
                        )
                    })
                    .unwrap_or_else(|| {
                        panic!("No package found for {}:{}", dep_id.0, dep.restored_version)
                    })
            });

        Ok(Self {
            resolved_deps: packages.collect(),
            shared_package: Cow::Borrowed(shared_package),
        })
    }

    /// Restore dependencies for a package
    /// This will download the dependencies to the cache and copy them to the workspace
    /// It will also generate the toolchain JSON file if specified
    /// Returns an error if any dependency fails to download or copy
    pub fn restore<P: AsRef<Path>>(
        &self,
        workspace: P,
        repository: &mut impl Repository,
        file_repo: &FileRepository,
    ) -> color_eyre::Result<()> {
        for dep in &self.resolved_deps {
            println!(
                "Pulling {}:{}",
                dep.config.id.0.dependency_id_color(),
                dep.config.version.to_string().dependency_version_color(),
            );
            repository.download_to_cache(&dep.config).with_context(|| {
                format!(
                    "Requesting {}:{}",
                    dep.config.id.0.dependency_id_color(),
                    dep.config.version.version_id_color()
                )
            })?;
            repository.add_to_db_cache(dep.config.clone(), dep.qpkg_checksum.clone(), true)?;
        }

        repository.write_repo()?;

        file_repo.copy_from_cache(
            &self.shared_package.config,
            &self.resolved_deps,
            workspace.as_ref(),
        )?;

        self.shared_package
            .try_write_toolchain(repository, file_repo)?;

        Ok(())
    }
}
