use color_eyre::Result;
use itertools::Itertools;
use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};

use qpm_package::models::package::{DependencyId, PackageConfig};

use self::{
    local::FileRepository, memcached::MemcachedRepository, multi::MultiDependencyRepository,
    qpackages::QPMRepository,
};

pub mod local;
pub mod memcached;
pub mod multi;
pub mod qpackages;

/// A package as returned by a repository lookup: its config, plus the sha256 checksum of the
/// QPKG archive it came from, when the repository is able to determine one.
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    pub config: PackageConfig,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub qpkg_checksum: Option<String>,
}

pub trait Repository {
    fn get_package_names(&self) -> Result<Vec<DependencyId>>;

    /// Get the package versions for a given package id
    /// Returns None if the package is not found in any repository
    /// Ordered by version descending
    fn get_package_versions(&self, id: &DependencyId) -> Result<Option<Vec<Version>>>;

    fn get_package(&self, id: &DependencyId, version: &Version) -> Result<Option<Artifact>>;
    // add to the db cache
    // this just stores the shared config itself, not the package
    // qpkg_checksum is the sha256 checksum of the source QPKG archive, if known
    fn add_to_db_cache(
        &mut self,
        config: PackageConfig,
        qpkg_checksum: Option<String>,
        permanent: bool,
    ) -> Result<()>;

    /// Returns true if the repository uses a network connection to retrieve data
    fn is_online(&self) -> bool;

    // downloads if not in cache
    // What if we wanted to have a qpackages mirror or a new backend? ;)
    // Does not download dependencies
    // false if not downloaded, true if download complete or already downloaded
    fn download_to_cache(&mut self, config: &PackageConfig) -> Result<bool>;

    fn write_repo(&self) -> Result<()>;
}

pub fn default_repositories() -> Result<Vec<Box<dyn Repository>>> {
    // TODO: Make file repository cached
    let file_repository = Box::new(FileRepository::read()?);
    let qpm_repository = Box::<QPMRepository>::default();
    Ok(vec![file_repository, qpm_repository])
}

pub fn useful_default_new(offline: bool) -> Result<MemcachedRepository<MultiDependencyRepository>> {
    let repos: Vec<Box<dyn Repository>> = match offline {
        // offline
        true => default_repositories()?
            .into_iter()
            .filter(|r| !r.is_online())
            .collect_vec(),
        // online
        false => default_repositories()?,
    };

    let multi_dependency_repository = MultiDependencyRepository::new(repos);
    let memcached = MemcachedRepository::new(multi_dependency_repository);
    Ok(memcached)
}
