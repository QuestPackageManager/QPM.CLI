use std::path::Path;

use color_eyre::{
    Result,
    eyre::{Context, OptionExt},
};

use crate::{
    models::config::UserConfig,
    repository::{
        self, file::FileRepository, memcached::MemcachedRepository,
        multi::MultiDependencyRepository,
    },
    services::network::{DEFAULT_TIMEOUT_MS, build_agent},
};

/// Per-invocation state shared by commands: the combined (global + workspace) user config
/// and the HTTP client configured from it.
///
/// Built once in `main` and passed down explicitly, instead of living in process-wide
/// globals. Tests build their own with [`QpmContext::new`] to isolate each invocation.
#[derive(Clone, Debug)]
pub struct QpmContext {
    config: UserConfig,
    agent: ureq::Agent,
}

impl QpmContext {
    pub fn new(config: UserConfig) -> Self {
        let agent = build_agent(config.timeout.unwrap_or(DEFAULT_TIMEOUT_MS));
        Self { config, agent }
    }

    /// Shared HTTP client, using the configured connect timeout
    pub fn agent(&self) -> &ureq::Agent {
        &self.agent
    }

    /// Reads the combined (global + workspace) user config from disk.
    /// Intended for the real CLI entry point; tests use [`QpmContext::new`].
    pub fn load() -> Result<Self> {
        let config = UserConfig::read_combine().context("Failed to read qpm.settings.json")?;
        Ok(Self::new(config))
    }

    pub fn config(&self) -> &UserConfig {
        &self.config
    }

    /// Root directory packages are cached under. Also holds the file repository registry.
    pub fn cache_root(&self) -> Result<&Path> {
        self.config
            .cache
            .as_deref()
            .ok_or_eyre("No cache path set in qpm.settings.json")
    }

    pub fn ndk_download_path(&self) -> Result<&Path> {
        self.config
            .ndk_download_path
            .as_deref()
            .ok_or_eyre("No NDK download path set in qpm.settings.json")
    }

    /// Whether dependencies are symlinked into the workspace (true) or copied (false)
    pub fn symlink(&self) -> bool {
        self.config.symlink.unwrap_or(true)
    }

    /// Reads the local file repository (the package cache registry)
    pub fn file_repository(&self) -> Result<FileRepository> {
        FileRepository::read(self.cache_root()?.to_path_buf())
    }

    /// Builds the standard repository stack (local cache + remote, or local-only when offline)
    pub fn repositories(
        &self,
        offline: bool,
    ) -> Result<MemcachedRepository<MultiDependencyRepository>> {
        repository::default_repository_stack(offline, self.cache_root()?.to_path_buf(), &self.agent)
    }
}
