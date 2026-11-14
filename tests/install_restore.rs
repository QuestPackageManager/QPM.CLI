#![cfg(feature = "cli")]

//! Regression test: a QPKG installed locally with `qpm2 install --path` must be resolvable by
//! a later `qpm2 restore --offline`.
//!
//! Older qpm2 builds put the files in `<cache>/<id>/<version>` but recorded the package in a
//! separate registry under the OS config dir, while restore only read `<cache>/qpm.repository.json`.
//! The files were on disk (`qpm2 cache list` showed them) yet resolving failed with
//! "there is no version of <id> in <range>".

use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
    sync::Mutex,
};

use clap::Parser;
use qpm_cli::{
    commands::{Command, Opt},
    context::QpmContext,
    models::{config::UserConfig, package::PackageConfigExtensions, qpkg_file::QpkgFile},
};
use qpm_package::models::{
    package::{DependencyId, PackageConfig, PackageDependency},
    shared_package::SharedPackageConfig,
};
use semver::{Version, VersionReq};
use tempfile::tempdir;

/// A local-only package (not on qpackages), like the reporter's `capstone`
fn local_package() -> PackageConfig {
    let mut config = PackageConfig {
        id: DependencyId("localpkg".to_string()),
        version: Version::new(0, 1, 0),
        shared_directory: "shared".into(),
        dependencies_directory: "extern".into(),
        ..Default::default()
    };
    config.workspace.out_binaries = Some(vec!["liblocalpkg.so".into()]);
    config
}

fn write_qpkg(config: PackageConfig, path: &Path) {
    let headers: Vec<(&Path, &[u8])> = vec![(Path::new("localpkg.h"), b"// header\n")];
    let binaries: Vec<(&Path, &[u8])> = vec![(Path::new("liblocalpkg.so"), b"binary")];
    let file = File::create(path).unwrap();
    QpkgFile::create(file, config, headers, binaries).expect("creating the qpkg should succeed");
}

/// Commands run relative to the process-wide current directory, so tests in this binary
/// (which run in parallel threads) must take turns.
static CWD_LOCK: Mutex<()> = Mutex::new(());

/// Runs a qpm2 command line in-process with `config`, from `cwd`, the way `main` does
fn run_qpm(config: &UserConfig, cwd: &Path, args: &[&str]) -> color_eyre::Result<()> {
    let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

#[test]
fn locally_installed_qpkg_is_restorable_offline() {
    let cache = tempdir().unwrap();
    let workspace = tempdir().unwrap();
    let config = UserConfig {
        cache: Some(cache.path().to_path_buf()),
        // copy, so the test doesn't depend on symlink support
        symlink: Some(false),
        ..UserConfig::default()
    };

    // qpm2 install --path localpkg.qpkg
    let qpkg_path: PathBuf = workspace.path().join("localpkg.qpkg");
    write_qpkg(local_package(), &qpkg_path);
    run_qpm(
        &config,
        workspace.path(),
        &["install", "--path", qpkg_path.to_str().unwrap()],
    )
    .expect("installing the local qpkg should succeed");

    // a project depending on it
    let project = PackageConfig {
        id: DependencyId("flamingo".to_string()),
        version: Version::new(1, 0, 0),
        shared_directory: "shared".into(),
        dependencies_directory: "extern".into(),
        dependencies: HashMap::from([(
            DependencyId("localpkg".to_string()),
            PackageDependency {
                version_range: VersionReq::parse(">=0.1.0, <0.2.0").unwrap(),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };
    project.write(workspace.path()).unwrap();

    // qpm2 restore --offline
    run_qpm(&config, workspace.path(), &["restore", "--offline"])
        .expect("offline restore should resolve the locally installed package");

    let shared = SharedPackageConfig::read(workspace.path()).unwrap();
    let restored = shared
        .restored_dependencies
        .get(&DependencyId("localpkg".to_string()))
        .expect("localpkg should be in the lock file");
    assert_eq!(restored.restored_version, Version::new(0, 1, 0));
}

/// With `--offline` there's no HTTP client, so commands that can only work online fail
/// up front with a clear message instead of attempting a request.
#[test]
fn network_only_command_refuses_offline() {
    let cache = tempdir().unwrap();
    let workspace = tempdir().unwrap();
    let config = UserConfig {
        cache: Some(cache.path().to_path_buf()),
        ..UserConfig::default()
    };

    let err = run_qpm(
        &config,
        workspace.path(),
        &[
            "--offline",
            "install",
            "--url",
            "https://example.invalid/pkg.qpkg",
        ],
    )
    .expect_err("installing from a URL offline should fail");
    assert!(
        format!("{err:?}").contains("--offline"),
        "error should explain that --offline prevents it: {err:?}"
    );
}
