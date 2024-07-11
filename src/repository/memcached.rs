use color_eyre::Result;

use semver::Version;
use std::{cell::UnsafeCell, collections::HashMap};

use qpm_package::models::{
    backend::PackageVersion, dependency::SharedPackageConfig, package::PackageConfig,
};

use super::Repository;

pub struct MemcachedRepository<R: Repository> {
    // interior mutability
    packages_cache: UnsafeCell<HashMap<String, HashMap<Version, SharedPackageConfig>>>,
    versions_cache: UnsafeCell<HashMap<String, Vec<PackageVersion>>>,
    package_list: UnsafeCell<Option<Vec<String>>>,

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
    fn get_package_names(&self) -> Result<Vec<String>> {
        let package_list_opt = self.package_list.get_mut_safe();

        if package_list_opt.is_none() {
            let inner_package_names = self.inner_repo.get_package_names()?;
            *package_list_opt = Some(inner_package_names);
        }

        Ok(package_list_opt.clone().unwrap())
    }

    fn get_package_versions(&self, id: &str) -> Result<Option<Vec<PackageVersion>>> {
        let cache = self.versions_cache.get_mut_safe().get(id);

        if let Some(c) = cache {
            return Ok(Some(c.clone()));
        }

        let versions = self.inner_repo.get_package_versions(id)?;

        if let Some(versions) = &versions {
            self.versions_cache
                .get_mut_safe()
                .entry(id.to_string())
                .insert_entry(versions.clone());
        }

        Ok(versions)
    }

    fn get_package(&self, id: &str, version: &Version) -> Result<Option<SharedPackageConfig>> {
        let cache = self
            .packages_cache
            .get_safe()
            .get(id)
            .and_then(|f| f.get(version));

        if let Some(c) = cache {
            return Ok(Some(c.clone()));
        }

        let config = self.inner_repo.get_package(id, version)?;

        if let Some(config) = &config {
            self.packages_cache
                .get_mut_safe()
                .entry(config.config.info.id.clone())
                .or_default()
                .entry(config.config.info.version.clone())
                .insert_entry(config.clone());
        }

        Ok(config)
    }

    fn add_to_db_cache(&mut self, config: SharedPackageConfig, permanent: bool) -> Result<()> {
        self.inner_repo.add_to_db_cache(config, permanent)
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
trait UnsafeCellExt<T>: Sized {
    fn get_safe(&self) -> &T;

    #[allow(clippy::mut_from_ref)]
    fn get_mut_safe(&self) -> &mut T;
}

impl<T> UnsafeCellExt<T> for UnsafeCell<T> {
    fn get_safe(&self) -> &T {
        unsafe { &*self.get() }
    }

    fn get_mut_safe(&self) -> &mut T {
        unsafe { &mut *self.get() }
    }
}
