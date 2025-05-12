use color_eyre::{Result, eyre::bail};
use itertools::Itertools;

use qpm_package::models::{
    backend::PackageVersion, dependency::SharedPackageConfig, package::PackageConfig,
};

use super::Repository;

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
    fn get_package_versions(&self, id: &str) -> Result<Option<Vec<PackageVersion>>> {
        // double flat map???? rust weird
        // TODO: Propagate error
        let result: Vec<PackageVersion> = self
            .repositories
            .iter()
            .filter_map(|r| r.get_package_versions(id).expect("Failed to get versions"))
            .flatten()
            .unique()
            .sorted_by(|a, b| a.version.cmp(&b.version))
            .rev() // highest first
            .collect();

        if result.is_empty() {
            return Ok(None);
        }

        Ok(Some(result))

        // let mut result: Vec<PackageVersion> = vec![];
        // for repo_result in self.repositories.iter().map(|r| r.get_package_versions(id)) {
        //     if let Some(r) = repo_result? {
        //         result.extend_from_slice(&r)
        //     }
        // }

        // let compute_result = result.into_iter().unique().collect_vec();

        // if compute_result.is_empty() {
        //     return Ok(None);
        // }

        // Ok(Some(compute_result))
    }

    // get package from the first repository that has it
    fn get_package(
        &self,
        id: &str,
        version: &semver::Version,
    ) -> Result<Option<SharedPackageConfig>> {
        let opt = self
            .repositories
            .iter()
            .map(|r| r.get_package(id, version).expect("Unable to get package"))
            .find(|r| r.is_some());

        match opt {
            Some(o) => Ok(o),
            _ => Ok(None),
        }
    }

    fn get_package_names(&self) -> Result<Vec<String>> {
        Ok(self
            .repositories
            .iter()
            .flat_map(|r| r.get_package_names().expect("Unable to get package names"))
            .unique()
            .collect::<Vec<String>>())
    }

    fn download_to_cache(&mut self, config: &PackageConfig) -> Result<bool> {
        match self
            .repositories
            .iter_mut()
            .filter(|r| {
                r.get_package(&config.info.id, &config.info.version)
                    .expect("Unable to get package")
                    .is_some()
            })
            .find_map(|r| {
                r.download_to_cache(config)
                    .expect("Unable to download to cache")
                    .then_some(true)
            }) {
            Some(v) => Ok(v),
            None => bail!(
                "No repository found that has package {}:{}",
                config.info.id,
                config.info.version
            ),
        }
    }

    fn add_to_db_cache(&mut self, config: SharedPackageConfig, permanent: bool) -> Result<()> {
        if permanent {
            #[cfg(debug_assertions)]
            println!("Warning, adding to cache permanently to multiple repos!",);
        }
        self.repositories
            .iter_mut()
            .try_for_each(|r| r.add_to_db_cache(config.clone(), permanent))?;
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
