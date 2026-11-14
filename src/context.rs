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
/// and the HTTP client configured from it (absent when running `--offline`).
///
/// Built once in `main` and passed down explicitly, instead of living in process-wide
/// globals. Tests build their own with [`QpmContext::new`] to isolate each invocation.
#[derive(Clone, Debug)]
pub struct QpmContext {
    config: UserConfig,
    agent: Option<ureq::Agent>,
}

impl QpmContext {
    /// `offline` disables all network access: [`Self::agent`] is `None`
    pub fn new(config: UserConfig, offline: bool) -> Self {
        let agent = (!offline).then(|| build_agent(config.timeout.unwrap_or(DEFAULT_TIMEOUT_MS)));
        Self { config, agent }
    }

    /// Reads the combined (global + workspace) user config from disk.
    /// Intended for the real CLI entry point; tests use [`QpmContext::new`].
    pub fn load(offline: bool) -> Result<Self> {
        let config = UserConfig::read_combine().context("Failed to read qpm.settings.json")?;
        Ok(Self::new(config, offline))
    }

    pub fn is_offline(&self) -> bool {
        self.agent.is_none()
    }

    /// Shared HTTP client using the configured connect timeout, or `None` when offline
    pub fn agent(&self) -> Option<&ureq::Agent> {
        self.agent.as_ref()
    }

    /// The HTTP client, for operations that can't work without the network
    pub fn online_agent(&self) -> Result<&ureq::Agent> {
        self.agent
            .as_ref()
            .ok_or_eyre("This needs network access, but --offline was given")
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

    /// Builds the standard repository stack (local cache + qpackages, or only the local cache
    /// when offline)
    pub fn repositories(&self) -> Result<MemcachedRepository<MultiDependencyRepository>> {
        repository::default_repository_stack(self.cache_root()?.to_path_buf(), self.agent())
    }
}
