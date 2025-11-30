use std::{
    cmp::Reverse,
    error::Error,
    fmt::{Display, Formatter},
    path::Path,
    time::Instant,
};

use crate::{
    models::package::SharedPackageConfigExtensions,
    repository::{Repository, local::FileRepository},
    terminal::colors::QPMColor,
    utils::cmake::write_cmake,
};
use color_eyre::{
    Result,
    eyre::{Context, bail},
};
use itertools::Itertools;
use owo_colors::OwoColorize;
use pubgrub::{
    DefaultStringReporter, Dependencies, DependencyProvider, PackageResolutionStatistics,
    PubGrubError, Reporter,
};
use qpm_package::models::{dependency::SharedPackageConfig, package::PackageConfig};

use super::semver::{VersionWrapper, req_to_range};
pub struct PackageDependencyResolver<'a, 'b, R>
where
    R: Repository,
{
    root: &'a PackageConfig,
    repo: &'b R,
}
impl<R: Repository> DependencyProvider for PackageDependencyResolver<'_, '_, R> {
    type P = String;
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
        package: &String,
        version: &VersionWrapper,
    ) -> Result<Dependencies<Self::P, Self::VS, Self::M>, PubgrubErrorWrapper> {
        // Root dependencies
        if package == &self.root.info.id && version == &self.root.info.version {
            // resolve dependencies of root
            let deps = self
                .root
                .dependencies
                .iter()
                .map(|dep| {
                    let id = &dep.id;
                    let range = req_to_range(dep.version_range.clone());
                    (id.clone(), range)
                })
                .collect();
            return Ok(Dependencies::Available(deps));
        }

        // Find dependencies of dependencies
        let pkg = self
            .repo
            .get_package(package, &version.clone().into())
            .with_context(|| format!("Could not find package {package} with version {version}"))?
            .unwrap();

        let deps = pkg
            .config
            .dependencies
            .into_iter()
            // remove any private dependencies
            .filter(|dep| !dep.additional_data.is_private.unwrap_or(false))
            .inspect(|dep| {
                if dep.id == self.root.info.id {
                    println!(
                        "{}",
                        format!(
                            "Warning: Package {} depends on root package {}",
                            package.dependency_id_color(),
                            self.root.info.id.dependency_id_color()
                        )
                        .yellow()
                    );
                }
            })
            // skip root package to avoid circular deps
            .filter(|dep| dep.id != self.root.info.id)
            .map(|dep| {
                let id = dep.id;
                let range = req_to_range(dep.version_range);
                (id, range)
            })
            .collect();
        Ok(Dependencies::Available(deps))
    }

    fn choose_version(
        &self,
        package: &String,
        range: &pubgrub::Ranges<VersionWrapper>,
    ) -> Result<Option<VersionWrapper>, PubgrubErrorWrapper> {
        if *package == self.root.info.id {
            return Ok(Some(self.root.info.version.clone().into()));
        }

        let Some(dependencies) = self.repo.get_package_versions(package)? else {
            return Ok(None);
        };

        let chosen = dependencies
            .iter()
            .map(|version| VersionWrapper::from(version.version.clone()))
            .find(|version| range.contains(version));

        Ok(chosen)
    }

    fn prioritize(
        &self,
        package: &Self::P,
        range: &Self::VS,
        package_statistics: &PackageResolutionStatistics,
    ) -> Self::Priority {
        if *package == self.root.info.id {
            return (0, Reverse(0));
        }

        // Get versions available for the package, if none return default priority
        let Ok(Some(versions)) = self.repo.get_package_versions(package) else {
            return (package_statistics.conflict_count(), Reverse(0));
        };

        // Count versions that satisfy the range constraint
        let version_count = versions
            .iter()
            .filter(|v| range.contains(&v.version.clone().into()))
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

pub fn resolve<'a>(
    root: &'a PackageConfig,
    repository: &'a impl Repository,
) -> Result<impl Iterator<Item = SharedPackageConfig> + 'a> {
    let resolver = PackageDependencyResolver {
        root,
        repo: repository,
    };
    let time = Instant::now();
    let result = match pubgrub::resolve(&resolver, root.info.id.clone(), root.info.version.clone())
    {
        Ok(deps) => Ok(deps.into_iter().filter_map(move |(id, version)| {
            if id == root.info.id && version == root.info.version {
                return None;
            }

            repository.get_package(&id, &version.into()).unwrap()
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

pub fn restore<P: AsRef<Path>>(
    workspace: P,
    shared_package: &SharedPackageConfig,
    resolved_deps: &[SharedPackageConfig],
    repository: &mut impl Repository,
) -> Result<()> {
    for dep in resolved_deps {
        println!(
            "Pulling {}:{}",
            &dep.config.info.id.dependency_id_color(),
            &dep.config
                .info
                .version
                .to_string()
                .dependency_version_color()
        );
        repository.download_to_cache(&dep.config).with_context(|| {
            format!(
                "Requesting {}:{}",
                dep.config.info.id.dependency_id_color(),
                dep.config.info.version.version_id_color()
            )
        })?;
        repository.add_to_db_cache(dep.clone(), true)?;
    }

    repository.write_repo()?;

    println!("Copying now");
    FileRepository::copy_from_cache(&shared_package.config, resolved_deps, workspace.as_ref())?;

    write_cmake(shared_package, repository)?;
    shared_package.try_write_toolchain(repository)?;

    Ok(())
}

pub fn locked_resolve<'a, R: Repository>(
    root: &'a SharedPackageConfig,
    repository: &'a R,
) -> Result<impl Iterator<Item = SharedPackageConfig> + 'a> {
    // TODO: ensure restored dependencies take precedence over
    Ok(root
        .restored_dependencies
        .iter()
        .map(|d| {
            repository
                .get_package(&d.dependency.id, &d.version)
                .unwrap_or_else(|e| {
                    panic!(
                        "Encountered an issue resolving for package {}:{}, {e}",
                        d.dependency.id, d.version
                    )
                })
                .unwrap_or_else(|| panic!("No package found for {}:{}", d.dependency.id, d.version))
        })
        .dedup_by(|x, y| x.config.info.id == y.config.info.id))
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
