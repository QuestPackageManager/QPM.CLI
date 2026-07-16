use color_eyre::{Result, eyre::bail};
use itertools::Itertools;

use qpm_package::models::package::{DependencyId, PackageConfig};
use semver::Version;

use super::{Artifact, Repository};

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
    fn get_package_versions(&self, id: &DependencyId) -> Result<Option<Vec<Version>>> {
        let result: Vec<Version> = self
            .repositories
            .iter()
            .map(|r| r.get_package_versions(id))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .flatten()
            .unique()
            .sorted_by(|a, b| a.cmp(b))
            .rev() // highest first
            .collect();

        if result.is_empty() {
            return Ok(None);
        }

        Ok(Some(result))
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

    fn download_to_cache(&mut self, config: &PackageConfig) -> Result<bool> {
        for repo in &mut self.repositories {
            if repo.get_package(&config.id, &config.version)?.is_none() {
                continue;
            }
            if repo.download_to_cache(config)? {
                return Ok(true);
            }
        }

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
