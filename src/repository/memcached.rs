use color_eyre::Result;

use semver::Version;
use std::{cell::RefCell, collections::HashMap};

use qpm_package::models::package::{DependencyId, PackageConfig};

use super::{Artifact, Repository};

pub struct MemcachedRepository<R: Repository> {
    // interior mutability - pubgrub's DependencyProvider trait only gives us `&self` to cache
    // lookups behind
    packages_cache: RefCell<HashMap<DependencyId, HashMap<Version, Artifact>>>,
    versions_cache: RefCell<HashMap<DependencyId, Vec<Version>>>,
    package_list: RefCell<Option<Vec<DependencyId>>>,

    inner_repo: R,
}

impl<R: Repository> MemcachedRepository<R> {
    // Repositories sorted in order
    pub fn new(inner_repo: R) -> Self {
        Self {
            inner_repo,
            versions_cache: Default::default(),
            package_list: Default::default(),
            packages_cache: Default::default(),
        }
    }
}

impl<R: Repository> Repository for MemcachedRepository<R> {
    fn get_package_names(&self) -> Result<Vec<DependencyId>> {
        if let Some(cached) = self.package_list.borrow().clone() {
            return Ok(cached);
        }

        let inner_package_names = self.inner_repo.get_package_names()?;
        *self.package_list.borrow_mut() = Some(inner_package_names.clone());

        Ok(inner_package_names)
    }

    fn get_package_versions(&self, id: &DependencyId) -> Result<Option<Vec<Version>>> {
        if let Some(cached) = self.versions_cache.borrow().get(id).cloned() {
            return Ok(Some(cached));
        }

        let versions = self.inner_repo.get_package_versions(id)?;

        if let Some(versions) = &versions {
            self.versions_cache
                .borrow_mut()
                .insert(id.clone(), versions.clone());
        }

        Ok(versions)
    }

    fn get_package(&self, id: &DependencyId, version: &Version) -> Result<Option<Artifact>> {
        if let Some(cached) = self
            .packages_cache
            .borrow()
            .get(id)
            .and_then(|f| f.get(version))
            .cloned()
        {
            return Ok(Some(cached));
        }

        let artifact = self.inner_repo.get_package(id, version)?;

        if let Some(artifact) = &artifact {
            self.packages_cache
                .borrow_mut()
                .entry(artifact.config.id.clone())
                .or_default()
                .insert(artifact.config.version.clone(), artifact.clone());
        }

        Ok(artifact)
    }

    fn add_to_db_cache(
        &mut self,
        config: PackageConfig,
        qpkg_checksum: Option<String>,
        permanent: bool,
    ) -> Result<()> {
        self.inner_repo
            .add_to_db_cache(config, qpkg_checksum, permanent)
    }

    fn download_to_cache(&mut self, config: &PackageConfig) -> Result<bool> {
        self.inner_repo.download_to_cache(config)
    }

    fn write_repo(&self) -> Result<()> {
        self.inner_repo.write_repo()
    }

    fn is_online(&self) -> bool {
        false
    }
}
