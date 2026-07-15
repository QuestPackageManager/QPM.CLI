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
    services::android::{download_ndk_version, get_android_manifest, get_ndk_str_versions, get_ndk_str_versions_str},
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
        None => bail!("Could not find any ndk version matching requirement {fuzzy_version_range}"),
    }
}

/// Determines which NDK version to pin: the highest installed version satisfying `version`
/// (parsed as a requirement) when `online` is false, or a fuzzy manifest match when `online`
/// is true (allowing versions that aren't installed yet).
pub fn resolve_pin_version(version: &str, online: bool, installed_ndks: &[PathBuf]) -> Result<String> {
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

    Ok(ndk_installed_path.file_name().unwrap().to_str().unwrap().to_string())
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
    let suggested_version =
        manifest.as_ref().and_then(|manifest| range_match_ndk(manifest, &ndk_requirement).ok());

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
