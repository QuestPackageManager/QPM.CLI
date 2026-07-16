use std::{fs, path::PathBuf};

use qpm_cli::models::android_repo::{
    Archive, ArchivesType, AndroidRepositoryManifest, CompleteType, RemotePackage, RevisionType,
};
use qpm_cli::services::ndk::{fuzzy_match_ndk, range_match_ndk, resolve_ndk_version, resolve_pin_version};
use qpm_package::models::package::PackageConfig;
use semver::VersionReq;

fn host_os_name() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macosx"
    } else if cfg!(windows) {
        "windows"
    } else {
        panic!("unsupported test os")
    }
}

fn ndk_package(path: &str, major: u64, minor: u64, micro: u64, with_host_archive: bool) -> RemotePackage {
    let archive = Archive {
        host_os: Some(host_os_name().to_string()),
        host_arch: None,
        complete: CompleteType {
            size: Some(1),
            checksum: "deadbeef".to_string(),
            url: format!("{path}.zip"),
        },
    };

    RemotePackage {
        path: path.to_string(),
        archives: ArchivesType {
            archive: if with_host_archive { vec![archive] } else { vec![] },
        },
        revision: RevisionType {
            major: Some(major),
            minor: Some(minor),
            micro: Some(micro),
            preview: None,
        },
        display_name: format!("NDK {major}.{minor}.{micro}"),
        uses_license: None,
        channel: None,
    }
}

fn sample_manifest() -> AndroidRepositoryManifest {
    AndroidRepositoryManifest {
        license: vec![],
        remote_package: vec![
            ndk_package("ndk;24.0.0", 24, 0, 0, true),
            ndk_package("ndk;25.1.0", 25, 1, 0, true),
            ndk_package("ndk;26.0.0", 26, 0, 0, true),
        ],
    }
}

/// Creates tmp subdirectories named after each version under a fresh tmp dir, standing in for
/// already-installed NDKs on disk.
fn installed_ndk_dirs(versions: &[&str]) -> (PathBuf, Vec<PathBuf>) {
    let root = std::env::temp_dir().join(format!(
        "qpm-ndk-test-{}-{}",
        std::process::id(),
        versions.join("-")
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let paths = versions
        .iter()
        .map(|v| {
            let p = root.join(v);
            fs::create_dir_all(&p).unwrap();
            p
        })
        .collect();

    (root, paths)
}

#[test]
fn resolve_ndk_version_picks_highest_installed_satisfying_requirement() {
    let (root, _paths) = installed_ndk_dirs(&["24.0.0", "25.1.0", "26.0.0"]);

    let mut package = PackageConfig::default();
    package.workspace.ndk = Some(VersionReq::parse(">=25.0.0, <26.0.0").unwrap());

    let resolved = resolve_ndk_version(&package, &root);

    assert_eq!(resolved.unwrap().file_name().unwrap(), "25.1.0");
}

#[test]
fn resolve_ndk_version_none_when_no_requirement() {
    let (root, _paths) = installed_ndk_dirs(&["25.1.0"]);
    let package = PackageConfig::default();

    assert!(resolve_ndk_version(&package, &root).is_none());
}

#[test]
fn resolve_ndk_version_none_when_nothing_installed_matches() {
    let (root, _paths) = installed_ndk_dirs(&["24.0.0"]);

    let mut package = PackageConfig::default();
    package.workspace.ndk = Some(VersionReq::parse("^25").unwrap());

    assert!(resolve_ndk_version(&package, &root).is_none());
}

#[test]
fn fuzzy_match_ndk_exact_version_string_hits_directly() {
    let manifest = sample_manifest();

    let (matched, ndk) = fuzzy_match_ndk(&manifest, "25.1.0").unwrap();

    assert_eq!(matched, "25.1.0");
    assert_eq!(ndk.path, "ndk;25.1.0");
}

#[test]
fn fuzzy_match_ndk_falls_back_to_range_matching() {
    let manifest = sample_manifest();

    let (matched, ndk) = fuzzy_match_ndk(&manifest, ">=25.0.0, <26.0.0").unwrap();

    assert_eq!(matched, "25.1.0");
    assert_eq!(ndk.path, "ndk;25.1.0");
}

#[test]
fn range_match_ndk_picks_highest_matching_version() {
    let manifest = sample_manifest();
    let req = VersionReq::parse(">=24.0.0").unwrap();

    let (matched, _ndk) = range_match_ndk(&manifest, &req).unwrap();

    assert_eq!(matched, "26.0.0");
}

#[test]
fn range_match_ndk_errors_when_nothing_matches() {
    let manifest = sample_manifest();
    let req = VersionReq::parse(">=100.0.0").unwrap();

    assert!(range_match_ndk(&manifest, &req).is_err());
}

#[test]
fn resolve_pin_version_offline_picks_highest_installed_satisfying_version() {
    let (_root, paths) = installed_ndk_dirs(&["24.0.0", "25.1.0", "26.0.0"]);

    let pinned = resolve_pin_version(">=25.0.0, <26.0.0", false, &paths).unwrap();

    assert_eq!(pinned, "25.1.0");
}

#[test]
fn resolve_pin_version_offline_errors_when_nothing_installed_matches() {
    let (_root, paths) = installed_ndk_dirs(&["24.0.0"]);

    assert!(resolve_pin_version("^25", false, &paths).is_err());
}
