use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use color_eyre::{
    Result, Section,
    eyre::{Context, bail, eyre},
};
use itertools::Itertools;
use qpm_package::models::package::PackageConfig;
use semver::{Version, VersionReq};
use walkdir::WalkDir;

use crate::{
    models::android_repo::{AndroidRepositoryManifest, RemotePackage},
    services::android::{
        download_ndk_version, get_android_manifest, get_ndk_str_versions, get_ndk_str_versions_str,
    },
};

/// Resolves the installed NDK path satisfying the package's NDK requirement, if any.
pub fn resolve_ndk_version(package: &PackageConfig, ndk_download_path: &Path) -> Option<PathBuf> {
    let ndk_requirement = package.workspace.ndk.as_ref()?;

    let ndk_installed_path_opt = WalkDir::new(ndk_download_path)
        .max_depth(1)
        .into_iter()
        .flatten()
        .sorted_by(|a, b| a.file_name().cmp(b.file_name()))
        .rev() // descending
        .find(|s| {
            Version::parse(s.file_name().to_str().unwrap())
                .is_ok_and(|version| ndk_requirement.matches(&version))
        });

    Some(ndk_installed_path_opt?.into_path())
}

/// Finds an NDK matching `version` exactly, falling back to a fuzzy match against `version`
/// parsed as a version requirement.
pub fn fuzzy_match_ndk<'a>(
    manifest: &'a AndroidRepositoryManifest,
    version: &str,
) -> Result<(String, &'a RemotePackage)> {
    let ndks_str = get_ndk_str_versions_str(manifest);

    let ndk = ndks_str.get(version);
    match ndk {
        Some(ndk) => Ok((version.to_string(), ndk)),
        None => {
            let fuzzy_version_range = VersionReq::parse(version)?;
            range_match_ndk(manifest, &fuzzy_version_range)
        }
    }
}

/// Finds the highest available NDK version satisfying `fuzzy_version_range`.
pub fn range_match_ndk<'a>(
    manifest: &'a AndroidRepositoryManifest,
    fuzzy_version_range: &VersionReq,
) -> Result<(String, &'a RemotePackage)> {
    let ndks = get_ndk_str_versions(manifest);
    let ndks_versions = ndks.keys().sorted().rev().collect_vec();
    let matching_version_opt = ndks_versions
        .iter()
        .find(|probe| fuzzy_version_range.matches(probe));

    match matching_version_opt {
        Some(matching_version) => Ok((
            matching_version.to_string(),
            ndks.get(matching_version).unwrap(),
        )),
        None => {
            bail!("Could not find any ndk version matching requirement {fuzzy_version_range}");
        }
    }
}

/// Determines which NDK version to pin: the highest installed version satisfying `version`
/// (parsed as a requirement) when `online` is false, or a fuzzy manifest match when `online`
/// is true (allowing versions that aren't installed yet).
pub fn resolve_pin_version(
    version: &str,
    online: bool,
    installed_ndks: &[PathBuf],
) -> Result<String> {
    if online {
        let manifest = get_android_manifest()?;
        return Ok(fuzzy_match_ndk(&manifest, version)?.0);
    }

    let version_req = VersionReq::parse(version)?;
    let ndk_installed_path = installed_ndks
        .iter()
        .sorted_by(|a, b| a.file_name().cmp(&b.file_name()))
        .rev()
        .find(|n| {
            Version::parse(n.file_name().unwrap().to_str().unwrap())
                .is_ok_and(|v| version_req.matches(&v))
        })
        .ok_or_else(|| eyre!("No NDK version installed that satisfies {version_req}"))?;

    Ok(ndk_installed_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string())
}

/// Resolves the on-disk path of an NDK satisfying `package`'s requirement, downloading a
/// matching version if `download` is true and none is installed. Errors (with a download
/// suggestion) if no NDK satisfies the requirement and downloading wasn't requested.
pub fn resolve_ndk_for_package(
    package: &PackageConfig,
    download: bool,
    quiet: bool,
    ndk_download_path: &Path,
) -> Result<PathBuf> {
    let ndk_requirement = package
        .workspace
        .ndk
        .clone()
        .ok_or_else(|| eyre!("No NDK requirement set in project"))?;

    if let Some(ndk_installed_path) = resolve_ndk_version(package, ndk_download_path) {
        return Ok(ndk_installed_path);
    }

    if download {
        let manifest = get_android_manifest()?;
        let (_version, ndk) = range_match_ndk(&manifest, &ndk_requirement)?;
        return download_ndk_version(ndk, !quiet, ndk_download_path);
    }

    let mut report = eyre!("No NDK version installed that satisfies {ndk_requirement}")
        .note("-d/--download not set, not downloading!");

    // look up a version suitable to work with, allowing this to work offline
    let manifest = get_android_manifest().ok();
    let suggested_version = manifest
        .as_ref()
        .and_then(|manifest| range_match_ndk(manifest, &ndk_requirement).ok());

    if let Some((suggested_version, _)) = &suggested_version {
        report = report.suggestion(format!("qpm ndk download {suggested_version}"));
    }

    Err(report)
}

/// Writes the resolved NDK path to `ndkpath.txt` in the current directory
pub fn apply_ndk(ndk_installed_path: &Path) -> Result<()> {
    let mut ndk_file = File::create("./ndkpath.txt").context("Unable to open ndkpath.txt")?;
    write!(ndk_file, "{}", ndk_installed_path.to_str().unwrap())?;
    ndk_file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::models::android_repo::{Archive, ArchivesType, CompleteType, RevisionType};

    use super::*;

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

    fn ndk_package(
        path: &str,
        major: u64,
        minor: u64,
        micro: u64,
        with_host_archive: bool,
    ) -> RemotePackage {
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
                archive: if with_host_archive {
                    vec![archive]
                } else {
                    vec![]
                },
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
}
