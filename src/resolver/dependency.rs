use std::{
    cmp::Reverse,
    error::Error,
    fmt::{Display, Formatter},
    path::Path,
    time::Instant,
};

use super::semver::{VersionWrapper, req_to_range};
use crate::{
    models::package::SharedPackageConfigExtensions,
    repository::{Repository, local::FileRepository},
    terminal::colors::QPMColor,
};
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, bail},
};
use pubgrub::{
    DefaultStringReporter, Dependencies, DependencyProvider, PackageResolutionStatistics,
    PubGrubError, Reporter,
};
use qpm_package::models::{
    package::{DependencyId, PackageConfig},
    shared_package::{SharedPackageConfig, SharedTriplet},
    triplet::{PackageTriplet, TripletId},
};

/// Represents a resolved dependency
/// A tuple of (PackageConfig, TripletId)
pub struct ResolvedDependency(pub PackageConfig, pub TripletId);

impl ResolvedDependency {
    pub fn get_triplet_settings(&self) -> &PackageTriplet {
        self.0
            .triplets
            .specific_triplets
            .get(&self.1)
            .expect("Triplet should always exist in the package's triplets")
    }
}

pub struct PackageDependencyResolver<'a, 'b, R>
where
    R: Repository,
{
    root: &'a PackageConfig,
    root_triplet: &'a TripletId,
    repo: &'b R,
}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PubgrubDependencyTarget(pub DependencyId, pub TripletId);

impl Display for PubgrubDependencyTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}",
            self.0.0.dependency_id_color(),
            self.1.0.triplet_id_color()
        )
    }
}

impl<R: Repository> PackageDependencyResolver<'_, '_, R> {
    pub fn get_triplet_config(&self) -> &PackageTriplet {
        self.root
            .triplets
            .specific_triplets
            .get(self.root_triplet)
            .expect("Root triplet should always exist in the root package's triplets")
    }
}

impl<R: Repository> DependencyProvider for PackageDependencyResolver<'_, '_, R> {
    type P = PubgrubDependencyTarget;
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
        package_triplet: &PubgrubDependencyTarget,
        version: &VersionWrapper,
    ) -> Result<Dependencies<Self::P, Self::VS, Self::M>, PubgrubErrorWrapper> {
        // Root dependencies
        if package_triplet.0 == self.root.id
            && package_triplet.1 == *self.root_triplet
            && version == &self.root.version
        {
            // resolve dependencies of root
            let triplet = self
                .root
                .triplets
                .get_triplet_settings(&package_triplet.1)
                .expect("Root triplet should always exist in the root package's triplets");

            let deps = triplet
                .dependencies
                .iter()
                // add dev dependencies as well
                .chain(triplet.dev_dependencies.iter())
                .map(|(dep_id, dep)| {
                    let pubgrub_dep = PubgrubDependencyTarget(dep_id.clone(), dep.triplet.clone());

                    let range = req_to_range(dep.version_range.clone());
                    (pubgrub_dep, range)
                })
                .collect();
            return Ok(Dependencies::Available(deps));
        }

        // Find dependencies of dependencies
        let target_pkg = self
            .repo
            .get_package(&package_triplet.0, &version.clone().into())?
            .with_context(|| {
                format!("Could not find package {package_triplet} with version {version}")
            })?;

        let target_triplet = target_pkg
            .triplets
            .get_triplet_settings(&package_triplet.1)
            .with_context(|| {
                format!(
                    "Could not find triplet {} for package {}",
                    package_triplet.1.triplet_id_color(),
                    package_triplet.0.dependency_id_color()
                )
            })?;

        let deps = target_triplet
            .dependencies
            .iter()
            .map(|(dep_id, dep)| {
                let id = PubgrubDependencyTarget(dep_id.clone(), dep.triplet.clone());
                let range = req_to_range(dep.version_range.clone());
                (id, range)
            })
            .collect();
        Ok(Dependencies::Available(deps))
    }

    fn choose_version(
        &self,
        package: &PubgrubDependencyTarget,
        range: &pubgrub::Ranges<VersionWrapper>,
    ) -> Result<Option<VersionWrapper>, PubgrubErrorWrapper> {
        if package.0 == self.root.id {
            return Ok(Some(self.root.version.clone().into()));
        }

        let Some(dependencies) = self.repo.get_package_versions(&package.0)? else {
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
        if package.0 == self.root.id {
            return (0, Reverse(0));
        }

        // Get versions available for the package, if none return default priority
        let Ok(Some(versions)) = self.repo.get_package_versions(&package.0) else {
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
/// The iterator will return every dependency required by the package + triplet
pub fn resolve<'a>(
    root: &'a PackageConfig,
    repository: &'a impl Repository,
    triplet: &TripletId,
) -> Result<impl Iterator<Item = ResolvedDependency> + 'a> {
    let resolver = PackageDependencyResolver {
        root,
        root_triplet: triplet,
        repo: repository,
    };
    let time = Instant::now();
    let target = PubgrubDependencyTarget(root.id.clone(), triplet.clone());
    let result = match pubgrub::resolve(&resolver, target, root.version.clone()) {
        Ok(deps) => Ok(deps.into_iter().filter_map(move |(id, version)| {
            if id.0 == root.id && version == root.version {
                return None;
            }

            let package = repository
                .get_package(&id.0, &version.into())
                .expect("Failed to get package")?;
            Some(ResolvedDependency(package, id.1))
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
    triplet: &TripletId,
    resolved_deps: &[ResolvedDependency],
    repository: &mut impl Repository,
) -> Result<()> {
    for ResolvedDependency(dep, dep_triplet) in resolved_deps {
        println!(
            "Pulling {}:{} ({})",
            &dep.id.0.dependency_id_color(),
            &dep.version.to_string().dependency_version_color(),
            dep_triplet.0.triplet_id_color()
        );
        repository.download_to_cache(dep).with_context(|| {
            format!(
                "Requesting {}:{}",
                dep.id.0.dependency_id_color(),
                dep.version.version_id_color()
            )
        })?;
        repository.add_to_db_cache(dep.clone(), true)?;
    }

    repository.write_repo()?;

    println!("Copying now {}", triplet.triplet_id_color());
    FileRepository::copy_from_cache(
        &shared_package.config,
        triplet,
        resolved_deps,
        workspace.as_ref(),
    )?;

    shared_package.try_write_toolchain(repository)?;

    Ok(())
}

pub fn locked_resolve<'a, R: Repository>(
    root: &'a SharedPackageConfig,
    repository: &'a R,
    triplet: &'a SharedTriplet,
) -> Result<impl Iterator<Item = ResolvedDependency> + 'a> {
    // TODO: ensure restored dependencies take precedence over
    let packages = triplet.restored_dependencies.iter().map(|(dep_id, dep)| {
        let shared_package = repository
            .get_package(dep_id, &dep.restored_version)
            .unwrap_or_else(|e| {
                panic!(
                    "Encountered an issue resolving for package {}:{} {e:#?}",
                    dep_id.0, dep.restored_version
                )
            })
            .unwrap_or_else(|| {
                panic!("No package found for {}:{}", dep_id.0, dep.restored_version)
            });

        ResolvedDependency(shared_package, dep.restored_triplet.clone())
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
