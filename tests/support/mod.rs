//! Helpers shared by the test binaries: build package configs and `.qpkg` archives, and (with
//! the `cli` feature) run `qpm2` in-process against a temp workspace and cache.
//!
//! Include with `#[path = "support/mod.rs"] mod support;`.

// Shared by several test binaries, each of which uses only some of these helpers
#![allow(dead_code)]

use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use qpm_cli::models::{config::UserConfig, qpkg_file::QpkgFile};
use qpm_package::models::{
    package::{DependencyId, PackageConfig, PackageDependency},
    shared_package::SharedDependencyInfo,
};
use semver::{Version, VersionReq};

/// Commands run relative to the process-wide current directory, so tests in one binary
/// (which run on parallel threads) must take turns.
#[cfg(feature = "cli")]
static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A cache in `cache`, copying instead of symlinking so tests don't depend on symlink support
pub fn user_config(cache: &Path) -> UserConfig {
    UserConfig {
        cache: Some(cache.to_path_buf()),
        symlink: Some(false),
        ..UserConfig::default()
    }
}

/// Takes the lock on the process's current directory. Hold it for as long as a test depends
/// on the current directory, and run commands under it with [`run_qpm_unlocked`].
#[cfg(feature = "cli")]
pub fn lock_cwd() -> std::sync::MutexGuard<'static, ()> {
    CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs a qpm2 command line in-process with `config`, from `cwd`, the way `main` does
#[cfg(feature = "cli")]
pub fn run_qpm(config: &UserConfig, cwd: &Path, args: &[&str]) -> color_eyre::Result<()> {
    let _guard = lock_cwd();
    run_qpm_unlocked(config, cwd, args)
}

/// [`run_qpm`] for a caller already holding [`lock_cwd`]
#[cfg(feature = "cli")]
pub fn run_qpm_unlocked(config: &UserConfig, cwd: &Path, args: &[&str]) -> color_eyre::Result<()> {
    use clap::Parser;
    use qpm_cli::{
        commands::{Command, Opt},
        context::QpmContext,
    };

    let previous = std::env::current_dir()?;
    std::env::set_current_dir(cwd)?;

    let result = (|| {
        let opt = Opt::try_parse_from(std::iter::once("qpm2").chain(args.iter().copied()))?;
        let ctx = QpmContext::new(config.clone(), opt.offline);
        opt.command.expect("a subcommand").execute(&ctx)
    })();

    std::env::set_current_dir(previous)?;
    result
}

/// HTTP client for tests; only makes requests where a test serves or mocks them
pub fn test_agent() -> ureq::Agent {
    qpm_cli::services::network::build_agent(qpm_cli::services::network::DEFAULT_TIMEOUT_MS)
}

pub fn id(s: &str) -> DependencyId {
    DependencyId(s.to_string())
}

/// A package with the conventional `shared`/`extern` layout and the given binaries
pub fn package(name: &str, version: Version, out_binaries: Option<&[&str]>) -> PackageConfig {
    let mut config = PackageConfig {
        id: id(name),
        version,
        shared_directory: "shared".into(),
        dependencies_directory: "extern".into(),
        ..Default::default()
    };
    config.workspace.out_binaries = out_binaries.map(|b| b.iter().map(PathBuf::from).collect());
    config
}

pub fn depend_on(config: &mut PackageConfig, dep: &str, range: &str) {
    config.dependencies.insert(
        id(dep),
        PackageDependency {
            version_range: VersionReq::parse(range).unwrap(),
            ..Default::default()
        },
    );
}

/// A lock file entry pinning `version`, with nothing else recorded
pub fn locked(version: Version) -> SharedDependencyInfo {
    SharedDependencyInfo {
        restored_version: version,
        qpkg_url: None,
        qpkg_checksum: None,
        restored_binaries: vec![],
        restored_env: Default::default(),
    }
}

/// The `{name}.h` header every package built by these helpers ships
pub fn header_name(config: &PackageConfig) -> String {
    format!("{}.h", config.id.0)
}

/// A `.qpkg` for `config` in memory, with one header and a fake file for each `out_binaries`
pub fn qpkg_bytes(config: &PackageConfig) -> Vec<u8> {
    let header = header_name(config);
    let headers: Vec<(&Path, &[u8])> = vec![(Path::new(&header), b"// header\n")];
    let binaries: Vec<(PathBuf, Vec<u8>)> = config
        .workspace
        .out_binaries
        .iter()
        .flatten()
        .map(|b| (b.clone(), format!("binary {}", b.display()).into_bytes()))
        .collect();
    let binaries: Vec<(&Path, &[u8])> = binaries
        .iter()
        .map(|(p, c)| (p.as_path(), c.as_slice()))
        .collect();

    QpkgFile::create(Cursor::new(Vec::new()), config.clone(), headers, binaries)
        .expect("building the qpkg should succeed")
        .into_inner()
        .into_inner()
}

/// Writes `config` as a `qpm2.json` in a fresh workspace dir under `parent`
pub fn workspace(parent: &Path, config: &PackageConfig) -> PathBuf {
    use qpm_cli::models::package::PackageConfigExtensions;

    let dir = parent.join(&config.id.0);
    fs::create_dir_all(&dir).unwrap();
    config.write(&dir).unwrap();
    dir
}

/// Lists the file names directly inside `dir`, sorted; empty if it doesn't exist
pub fn file_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut names: Vec<String> = entries
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

