use std::{path::PathBuf, rc::Rc};

use color_eyre::Result;
use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};

use qpm_package::models::package::{DependencyId, PackageConfig};

use self::{
    file::FileRepository, memcached::MemcachedRepository, multi::MultiDependencyRepository,
    qpackages::QPMRepository,
};

pub mod file;
pub mod memcached;
pub mod multi;
pub mod qpackages;

/// A package as returned by a repository lookup: its config, plus the sha256 checksum of the
/// QPKG archive it came from and where to download that archive, when the repository knows.
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Default, PartialEq, Eq)]
pub struct Artifact {
    pub config: PackageConfig,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub qpkg_checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub qpkg_url: Option<String>,
}

/// A package's versions, highest first. Shared rather than copied: the resolver asks for the
/// same list many times, and the memoizing layer hands out the same allocation each time.
pub type Versions = Rc<[Version]>;

pub trait Repository {
    fn get_package_names(&self) -> Result<Vec<DependencyId>>;

    /// Get the package versions for a given package id
    /// Returns None if the package is not found in any repository
    /// Ordered by version descending
    fn get_package_versions(&self, id: &DependencyId) -> Result<Option<Versions>>;

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

    /// Downloads `artifact` into the cache if this repository can provide it.
    /// Does not download dependencies. Takes the artifact from a previous lookup, so no
    /// repository needs to look the package up again to find where to download it from.
    /// Returns false if this repository doesn't have it, true if it's now in the cache.
    fn download_to_cache(&mut self, agent: &ureq::Agent, artifact: &Artifact) -> Result<bool>;

    fn write_repo(&self) -> Result<()>;
}

/// Builds the standard repository stack rooted at `cache_root`: the local cache first, then
/// qpackages. Offline, only the local cache is used, and it won't download missing packages.
/// CLI code usually goes through `QpmContext::repositories` instead.
pub fn default_repository_stack(
    offline: bool,
    cache_root: PathBuf,
    agent: &ureq::Agent,
) -> Result<MemcachedRepository<MultiDependencyRepository>> {
    let file_repository = FileRepository::read(cache_root)?;

    let repos: Vec<Box<dyn Repository>> = if offline {
        vec![Box::new(file_repository)]
    } else {
        vec![
            Box::new(file_repository),
            Box::new(QPMRepository::new(agent.clone())),
        ]
    };

    Ok(MemcachedRepository::new(MultiDependencyRepository::new(
        repos,
    )))
}
