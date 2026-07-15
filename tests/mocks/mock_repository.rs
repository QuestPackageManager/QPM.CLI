use std::{cell::RefCell, collections::HashMap, rc::Rc};

use qpm_cli::repository::{Artifact, Repository};
use qpm_package::models::package::{DependencyId, PackageConfig};
use semver::Version;

/// A fully in-memory `Repository`, for testing repository-composition logic
/// (`MultiDependencyRepository`, `MemcachedRepository`) without touching the filesystem or
/// network. Unlike `FileRepository`, nothing here ever reads/writes the real global config
/// directory.
///
/// Cheaply `Clone`-able (shares the same underlying data), so a test can keep a handle to
/// mutate packages after handing the repository off to something that takes ownership of it
/// (e.g. `MemcachedRepository::new`) - useful for proving caching by observing whether a
/// later read reflects a mutation made through the other handle.
#[derive(Default, Clone)]
pub struct MockRepository(Rc<RefCell<Inner>>);

#[derive(Default)]
struct Inner {
    packages: HashMap<DependencyId, HashMap<Version, PackageConfig>>,
    online: bool,
    downloaded: Vec<(DependencyId, Version)>,
}

impl MockRepository {
    pub fn new(online: bool) -> Self {
        let repo = Self::default();
        repo.0.borrow_mut().online = online;
        repo
    }

    pub fn with_package(self, config: PackageConfig) -> Self {
        self.insert(config);
        self
    }

    pub fn insert(&self, config: PackageConfig) {
        self.0
            .borrow_mut()
            .packages
            .entry(config.id.clone())
            .or_default()
            .insert(config.version.clone(), config);
    }

    pub fn remove(&self, id: &DependencyId, version: &Version) {
        if let Some(versions) = self.0.borrow_mut().packages.get_mut(id) {
            versions.remove(version);
        }
    }

    /// Every package `download_to_cache` was called with, in call order.
    pub fn downloaded(&self) -> Vec<(DependencyId, Version)> {
        self.0.borrow().downloaded.clone()
    }
}

impl Repository for MockRepository {
    fn get_package_names(&self) -> color_eyre::Result<Vec<DependencyId>> {
        Ok(self.0.borrow().packages.keys().cloned().collect())
    }

    fn get_package_versions(&self, id: &DependencyId) -> color_eyre::Result<Option<Vec<Version>>> {
        Ok(self.0.borrow().packages.get(id).map(|versions| {
            let mut versions: Vec<Version> = versions.keys().cloned().collect();
            versions.sort();
            versions.reverse();
            versions
        }))
    }

    fn get_package(
        &self,
        id: &DependencyId,
        version: &Version,
    ) -> color_eyre::Result<Option<Artifact>> {
        Ok(self
            .0
            .borrow()
            .packages
            .get(id)
            .and_then(|m| m.get(version))
            .cloned()
            .map(|config| Artifact {
                config,
                qpkg_checksum: None,
            }))
    }

    fn add_to_db_cache(
        &mut self,
        config: PackageConfig,
        _qpkg_checksum: Option<String>,
        _permanent: bool,
    ) -> color_eyre::Result<()> {
        self.insert(config);
        Ok(())
    }

    fn is_online(&self) -> bool {
        self.0.borrow().online
    }

    fn download_to_cache(&mut self, config: &PackageConfig) -> color_eyre::Result<bool> {
        self.0
            .borrow_mut()
            .downloaded
            .push((config.id.clone(), config.version.clone()));
        Ok(true)
    }

    fn write_repo(&self) -> color_eyre::Result<()> {
        Ok(())
    }
}
