use color_eyre::{Result, eyre::bail};
use itertools::Itertools;

use qpm_package::models::package::{DependencyId, PackageConfig};
use semver::Version;

use super::{Artifact, Repository, Versions};

pub struct MultiDependencyRepository {
    repositories: Vec<Box<dyn Repository>>,
}

impl MultiDependencyRepository {
    // Repositories sorted in order
    pub fn new(repositories: Vec<Box<dyn Repository>>) -> Self {
        Self { repositories }
    }
}

///
/// Merge multiple repositories into one
/// Allow fetching from multiple backends
///
impl Repository for MultiDependencyRepository {
    // get versions of all repositories
    fn get_package_versions(&self, id: &DependencyId) -> Result<Option<Versions>> {
        let mut per_repo = self
            .repositories
            .iter()
            .filter_map(|r| r.get_package_versions(id).transpose())
            .collect::<Result<Vec<_>>>()?;

        // common case: only one repository knows the package, share its list as-is
        if per_repo.len() <= 1 {
            return Ok(per_repo.pop().filter(|v| !v.is_empty()));
        }

        let result: Vec<Version> = per_repo
            .iter()
            .flat_map(|versions| versions.iter())
            .unique()
            .sorted_by(|a, b| b.cmp(a)) // highest first
            .cloned()
            .collect();

        if result.is_empty() {
            return Ok(None);
        }

        Ok(Some(result.into()))
    }

    // get package from the first repository that has it
    fn get_package(
        &self,
        id: &DependencyId,
        version: &semver::Version,
    ) -> Result<Option<Artifact>> {
        for repo in &self.repositories {
            if let Some(artifact) = repo.get_package(id, version)? {
                return Ok(Some(artifact));
            }
        }
        Ok(None)
    }

    fn get_package_names(&self) -> Result<Vec<DependencyId>> {
        let names = self
            .repositories
            .iter()
            .map(|r| r.get_package_names())
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .unique()
            .collect();

        Ok(names)
    }

    /// Asks each repository in order; the first that can provide the package downloads it
    fn download_to_cache(&mut self, agent: &ureq::Agent, artifact: &Artifact) -> Result<bool> {
        for repo in &mut self.repositories {
            if repo.download_to_cache(agent, artifact)? {
                return Ok(true);
            }
        }

        let config = &artifact.config;
        bail!(
            "No repository found that has package {}:{}",
            config.id,
            config.version
        );
    }

    fn add_to_db_cache(
        &mut self,
        config: PackageConfig,
        qpkg_checksum: Option<String>,
        permanent: bool,
    ) -> Result<()> {
        if permanent {
            #[cfg(debug_assertions)]
            println!("Warning, adding to cache permanently to multiple repos!",);
        }
        self.repositories.iter_mut().try_for_each(|r| {
            r.add_to_db_cache(config.clone(), qpkg_checksum.clone(), permanent)
        })?;
        Ok(())
    }

    fn write_repo(&self) -> Result<()> {
        self.repositories.iter().try_for_each(|r| r.write_repo())?;
        Ok(())
    }

    fn is_online(&self) -> bool {
        self.repositories.iter().any(|r| r.is_online())
    }
}
