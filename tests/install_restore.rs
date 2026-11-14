#![cfg(feature = "cli")]

//! Regression test: a QPKG installed locally with `qpm2 install --path` must be resolvable by
//! a later `qpm2 restore --offline`.
//!
//! Older qpm2 builds put the files in `<cache>/<id>/<version>` but recorded the package in a
//! separate registry under the OS config dir, while restore only read `<cache>/qpm.repository.json`.
//! The files were on disk (`qpm2 cache list` showed them) yet resolving failed with
//! "there is no version of <id> in <range>".

#[path = "support/mod.rs"]
mod support;

use std::{collections::HashMap, path::PathBuf};

use qpm_cli::models::package::PackageConfigExtensions;
use qpm_package::models::{
    package::{DependencyId, PackageConfig, PackageDependency},
    shared_package::SharedPackageConfig,
};
use semver::{Version, VersionReq};
use support::{package, qpkg_bytes, run_qpm, user_config};
use tempfile::tempdir;

/// A local-only package (not on qpackages), like the reporter's `capstone`
fn local_package() -> PackageConfig {
    package("localpkg", Version::new(0, 1, 0), Some(&["liblocalpkg.so"]))
}

#[test]
fn locally_installed_qpkg_is_restorable_offline() {
    let cache = tempdir().unwrap();
    let workspace = tempdir().unwrap();
    let config = user_config(cache.path());

    // qpm2 install --path localpkg.qpkg
    let qpkg_path: PathBuf = workspace.path().join("localpkg.qpkg");
    std::fs::write(&qpkg_path, qpkg_bytes(&local_package())).unwrap();
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
    let config = user_config(cache.path());

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
