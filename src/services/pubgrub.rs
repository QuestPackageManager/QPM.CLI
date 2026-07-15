use std::{cmp::Reverse, error::Error, fmt::{Display, Formatter}};

use super::semver::{VersionWrapper, req_to_range};
use crate::{
    repository::{Artifact, Repository},
    terminal::colors::QPMColor,
};
use color_eyre::{
    Result,
    eyre::{ContextCompat},
};
use owo_colors::OwoColorize;
use pubgrub::{Dependencies, DependencyProvider, PackageResolutionStatistics};
use qpm_package::models::package::{DependencyId, PackageConfig};

/// A dependency resolved by pubgrub: the concrete package config chosen for a version, plus
/// the sha256 checksum of the QPKG archive it came from, when the repository knows one.
pub type ResolvedDependency = Artifact;

pub struct PackageDependencyResolver<'a, 'b, R>
where
    R: Repository,
{
    pub root: &'a PackageConfig,
    pub repo: &'b R,
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
