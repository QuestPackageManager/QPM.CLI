use color_eyre::{eyre::bail, Result};
use itertools::Itertools;

use qpm_package::models::{
    backend::PackageVersion, dependency::SharedPackageConfig, package::PackageConfig,
};

use super::{local::FileRepository, qpackages::QPMRepository, Repository};

pub fn default_repositories() -> Result<Vec<Box<dyn Repository>>> {
    // TODO: Make file repository cached
    let file_repository = Box::new(FileRepository::read()?);
    let qpm_repository = Box::<QPMRepository>::default();
    Ok(vec![file_repository, qpm_repository])
}

pub struct MultiDependencyRepository {
    repositories: Vec<Box<dyn Repository>>,
}

impl MultiDependencyRepository {
    // Repositories sorted in order
    pub fn new(repositories: Vec<Box<dyn Repository>>) -> Self {
        Self { repositories }
    }

    pub fn useful_default_new() -> Result<Self> {
        Ok(MultiDependencyRepository::new(default_repositories()?))
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
            .filter_map(|r| r.get_package_versions(id).unwrap())
            .flatten()
            .unique()
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
            .map(|r| r.get_package(id, version))
            .find(|r| r.as_ref().is_ok_and(|o| o.is_some()));

        if let Some(o) = opt {
            return o;
        }

        Ok(None)
    }

    fn get_package_names(&self) -> Result<Vec<String>> {
        Ok(self
            .repositories
            .iter()
            .map(|r| r.get_package_names())
            .flatten_ok()
            .flatten()
            .unique()
            .collect::<Vec<String>>())
    }

    fn download_to_cache(&mut self, config: &PackageConfig) -> Result<()> {
        let first_repo_opt = self.repositories.iter_mut().try_find(|r| -> Result<bool> {
            Ok(r.get_package(&config.info.id, &config.info.version)?
                .is_some())
        })?;

        match first_repo_opt {
            Some(first_repo) => first_repo.download_to_cache(config),
            None => bail!(
                "No repository found that has package {}:{}",
                config.info.id,
                config.info.version
            ),
        }
    }

    fn add_to_db_cache(&mut self, config: SharedPackageConfig, permanent: bool) -> Result<()> {
        if permanent {
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
}
