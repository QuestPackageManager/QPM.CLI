use color_eyre::Result;
use itertools::Itertools;

use crate::models::{dependency::{SharedPackageConfig}, backend::PackageVersion};

use super::{local::FileRepository, qpackages::QPMRepository, Repository};

pub fn default_repositories() -> Result<Vec<Box<dyn Repository>>> {
    // TODO: Make file repository cached
    let file_repository = Box::new(FileRepository::read()?);
    let qpm_repository = Box::new(QPMRepository::default());
    Ok(vec![file_repository, qpm_repository])
}

pub struct MultiDependencyProvider {
    repositories: Vec<Box<dyn Repository>>,
}

impl MultiDependencyProvider {
    // Repositories sorted in order
    pub fn new(repositories: Vec<Box<dyn Repository>>) -> Self {
        Self { repositories }
    }

    pub fn useful_default_new() -> Result<Self> {
        Ok(MultiDependencyProvider::new(default_repositories()?))
    }
}

///
/// Merge multiple repositories into one
/// Allow fetching from multiple backends
///
impl Repository for MultiDependencyProvider {
    // get versions of all repositories
    fn get_package_versions(&self, id: &str) -> Result<Option<Vec<PackageVersion>>> {
        // double flat map???? rust weird
        // TODO: Propagate error
        let result: Vec<PackageVersion> = self
            .repositories
            .iter()
            .map(|r| r.get_package_versions(id))
            .flatten_ok()
            .flatten()
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
            .find_map(|r| Some(r.get_package(id, version)));

        if let Some(o) = opt {
            return Ok(o?);
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

    fn add_to_cache(&mut self, config: SharedPackageConfig, permanent: bool) -> Result<()> {
        todo!()
    }
}
