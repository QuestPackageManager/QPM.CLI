#![cfg(feature = "cli")]

//! CLI commands that work entirely offline against a temp cache and workspace

#[path = "support/mod.rs"]
mod support;

use std::fs;

use qpm_cli::models::package_files::PackageIdPath;
use semver::Version;
use support::{depend_on, id, package, qpkg_bytes, run_qpm, user_config, workspace};
use tempfile::tempdir;

/// `cache clear <id>` removes the package's files and its index entry, so an offline restore
/// can no longer resolve it
#[test]
fn cache_clear_package_makes_it_unresolvable() {
    let root = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let user = user_config(cache.path());

    let dep = package("cleared", Version::new(1, 0, 0), Some(&["libcleared.so"]));
    let qpkg = root.path().join("cleared.qpkg");
    fs::write(&qpkg, qpkg_bytes(&dep)).unwrap();
    run_qpm(&user, root.path(), &["install", "--path", qpkg.to_str().unwrap()]).unwrap();

    let mut consumer = package("consumer", Version::new(1, 0, 0), None);
    depend_on(&mut consumer, "cleared", "^1.0.0");
    let consumer_dir = workspace(root.path(), &consumer);
    run_qpm(&user, &consumer_dir, &["restore", "--offline"])
        .expect("restore should work while the package is cached");

    run_qpm(&user, root.path(), &["cache", "clear", "cleared"]).unwrap();
    assert!(
        !PackageIdPath::new(id("cleared"))
            .versions_path(cache.path())
            .exists()
    );

    // --update so the lock file from the first restore isn't used
    let err = run_qpm(&user, &consumer_dir, &["restore", "--offline", "--update"])
        .expect_err("the cleared package should no longer resolve");
    assert!(format!("{err:?}").contains("cleared"), "{err:?}");
}

#[test]
fn scripts_unknown_name_errors() {
    let root = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let dir = workspace(root.path(), &package("scripted", Version::new(1, 0, 0), None));

    let err = run_qpm(&user_config(cache.path()), &dir, &["scripts", "missing"])
        .expect_err("an undefined script should fail");
    assert!(
        format!("{err:?}").contains("Could not find script missing"),
        "{err:?}"
    );
}

/// Supplied arguments are substituted into the script's `$N` placeholders
#[cfg(unix)]
#[test]
fn scripts_substitute_supplied_arguments() {
    let root = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let mut config = package("scripted", Version::new(1, 0, 0), None);
    config
        .workspace
        .scripts
        .insert("make".into(), vec!["touch $0 $1".into()]);
    let dir = workspace(root.path(), &config);

    run_qpm(
        &user_config(cache.path()),
        &dir,
        &["scripts", "make", "first.txt", "second.txt"],
    )
    .expect("the script should run");

    assert!(dir.join("first.txt").exists());
    assert!(dir.join("second.txt").exists());
}
