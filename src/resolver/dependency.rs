use std::{cmp::Reverse, error::Error, fmt::{Display, Formatter}, path::Path, time::Instant};

use super::semver::{VersionWrapper, req_to_range};
use crate::{
    models::package::SharedPackageConfigExtensions,
    repository::{Artifact, Repository, local::FileRepository},
    terminal::colors::QPMColor,
};
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, bail},
};
use owo_colors::OwoColorize;
use pubgrub::{
    DefaultStringReporter, Dependencies, DependencyProvider, PackageResolutionStatistics,
    PubGrubError, Reporter,
};
use qpm_package::models::{
    package::{DependencyId, PackageConfig},
    shared_package::SharedPackageConfig,
};

/// A dependency resolved by pubgrub: the concrete package config chosen for a version, plus
/// the sha256 checksum of the QPKG archive it came from, when the repository knows one.
pub type ResolvedDependency = Artifact;

pub struct PackageDependencyResolver<'a, 'b, R>
where
    R: Repository,
{
    root: &'a PackageConfig,
    repo: &'b R,
}

impl<R: Repository> DependencyProvider for PackageDependencyResolver<'_, '_, R> {
    type P = DependencyId;
    type V = VersionWrapper;
    type VS = pubgrub::Ranges<VersionWrapper>;
    type M = String;

    // TODO: Color_eyre error handling
    type Err = PubgrubErrorWrapper;

    /// The type returned from `prioritize`. The resolver does not care what type this is
    /// as long as it can pick a largest one and clone it.
    ///
    /// [`Reverse`](std::cmp::Reverse) can be useful if you want to pick the package with
    /// the fewest versions that match the outstanding constraint.
    type Priority = (u32, Reverse<usize>);

    fn get_dependencies(
        &self,
        package: &DependencyId,
        version: &VersionWrapper,
    ) -> Result<Dependencies<Self::P, Self::VS, Self::M>, PubgrubErrorWrapper> {
        // Root dependencies
        if *package == self.root.id && version == &self.root.version {
            let deps = self
                .root
                .dependencies
                .iter()
                // add dev dependencies as well
                .chain(self.root.dev_dependencies.iter())
                .map(|(dep_id, dep)| {
                    let range = req_to_range(dep.version_range.clone());
                    (dep_id.clone(), range)
                })
                .collect();
            return Ok(Dependencies::Available(deps));
        }

        // Find dependencies of dependencies
        let target_pkg = self
            .repo
            .get_package(package, &version.clone().into())?
            .with_context(|| format!("Could not find package {package} with version {version}"))?
            .config;

        let deps = target_pkg
            .dependencies
            .iter()
            // TODO: remove any private dependencies
            // .filter(|dep| !dep.1.version_range.additional_data.is_private.unwrap_or(false))
            .inspect(|(dep_id, _)| {
                if **dep_id == self.root.id {
                    println!(
                        "{}",
                        format!(
                            "Warning: Package {} depends on root package {}",
                            target_pkg.id.dependency_id_color(),
                            self.root.id.dependency_id_color()
                        )
                        .yellow()
                    );
                }
            })
            // skip root package to avoid circular deps
            .filter(|dep| *dep.0 != self.root.id)
            .map(|(dep_id, dep)| {
                let range = req_to_range(dep.version_range.clone());
                (dep_id.clone(), range)
            })
            .collect();
        Ok(Dependencies::Available(deps))
    }

    fn choose_version(
        &self,
        package: &DependencyId,
        range: &pubgrub::Ranges<VersionWrapper>,
    ) -> Result<Option<VersionWrapper>, PubgrubErrorWrapper> {
        if *package == self.root.id {
            return Ok(Some(self.root.version.clone().into()));
        }

        let Some(dependencies) = self.repo.get_package_versions(package)? else {
            return Ok(None);
        };

        let chosen = dependencies
            .iter()
            .map(|version| VersionWrapper::from(version.clone()))
            .find(|version| range.contains(version));

        Ok(chosen)
    }

    fn prioritize(
        &self,
        package: &Self::P,
        range: &Self::VS,
        package_statistics: &PackageResolutionStatistics,
    ) -> Self::Priority {
        if *package == self.root.id {
            return (0, Reverse(0));
        }

        // Get versions available for the package, if none return default priority
        let Ok(Some(versions)) = self.repo.get_package_versions(package) else {
            return (package_statistics.conflict_count(), Reverse(0));
        };

        // Count versions that satisfy the range constraint
        let version_count = versions
            .into_iter()
            .filter(|v| range.contains(&VersionWrapper(v.clone())))
            .count();

        // If no versions satisfy the constraint, use maximum priority
        if version_count == 0 {
            return (u32::MAX, Reverse(0));
        }

        // Prioritize packages that have had more conflicts first
        // and versions with fewer options (using Reverse) second
        (package_statistics.conflict_count(), Reverse(version_count))
    }
}

/// Resolve dependencies for a package using pubgrub
/// This will return an iterator of resolved dependencies
/// The iterator will return every dependency required by the package
pub fn resolve<'a>(
    root: &'a PackageConfig,
    repository: &'a impl Repository,
) -> Result<impl Iterator<Item = ResolvedDependency> + 'a> {
    let resolver = PackageDependencyResolver {
        root,
        repo: repository,
    };
    let time = Instant::now();
    let result = match pubgrub::resolve(&resolver, root.id.clone(), root.version.clone()) {
        Ok(deps) => Ok(deps.into_iter().filter_map(move |(id, version)| {
            if id == root.id && version == root.version {
                return None;
            }

            repository
                .get_package(&id, &version.into())
                .expect("Failed to get package")
        })),

        Err(PubGrubError::NoSolution(tree)) => {
            let report = DefaultStringReporter::report(&tree);
            bail!("failed to resolve dependencies: \n{}", report)
        }
        Err(err) => {
            bail!("pubgrub: {err}\n{err:?}");
        }
    };

    let sw = time.elapsed();
    println!("Took {}ms to dependency resolve", sw.as_millis());
    result
}

/// Restore dependencies for a package
/// This will download the dependencies to the cache and copy them to the workspace
/// It will also generate the toolchain JSON file if specified
/// Returns an error if any dependency fails to download or copy
pub fn restore<P: AsRef<Path>>(
    workspace: P,
    shared_package: &SharedPackageConfig,
    resolved_deps: &[ResolvedDependency],
    repository: &mut impl Repository,
) -> Result<()> {
    for dep in resolved_deps {
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

    FileRepository::copy_from_cache(&shared_package.config, resolved_deps, workspace.as_ref())?;

    shared_package.try_write_toolchain(repository)?;

    Ok(())
}

pub fn locked_resolve<'a, R: Repository>(
    shared_package: &'a SharedPackageConfig,
    repository: &'a R,
) -> Result<impl Iterator<Item = ResolvedDependency> + 'a> {
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

    Ok(packages)
}

pub struct PubgrubErrorWrapper(color_eyre::Report);

impl Error for PubgrubErrorWrapper {}

impl Display for PubgrubErrorWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::fmt::Debug for PubgrubErrorWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl From<color_eyre::Report> for PubgrubErrorWrapper {
    fn from(err: color_eyre::Report) -> Self {
        Self(err)
    }
}
