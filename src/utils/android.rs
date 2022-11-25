use std::{arch, collections::HashMap, env, fmt::format, io::Cursor, iter::Filter, os};

use bytes::Bytes;
use color_eyre::Result;
use semver::{BuildMetadata, Prerelease, Version};
use serde_xml_rs::{from_str, to_string};
use zip::ZipArchive;

use crate::{
    models::{
        android_repo::{AndroidRepositoryManifest, RemotePackage},
        config::get_combine_config,
    },
    network::agent::{download_file, download_file_report, get_agent},
};

const ANDROID_REPO_MANIFEST: &str = "https://dl.google.com/android/repository/repository2-3.xml";
const ANDROID_DL_URL: &str = "https://dl.google.com/android/repository/";

pub fn get_android_manifest() -> Result<AndroidRepositoryManifest> {
    let response = get_agent().get(ANDROID_REPO_MANIFEST).send()?;

    Ok(serde_xml_rs::from_reader(response)?)
}

pub fn get_ndk_packages(manifest: &AndroidRepositoryManifest) -> Vec<&RemotePackage> {
    manifest
        .remote_package
        .iter()
        .filter(|p| p.path.starts_with("ndk;"))
        .collect()
}

pub fn get_ndk_version(ndk: &RemotePackage) -> Version {
    Version {
        major: ndk.revision.major.unwrap_or(0),
        minor: ndk.revision.minor.unwrap_or(0),
        patch: ndk.revision.micro.unwrap_or(0),
        pre: Prerelease::new(&ndk.revision.preview.unwrap_or(0).to_string()).unwrap(),
        build: BuildMetadata::default(),
    }
}

pub fn get_ndk_versions(manifest: &AndroidRepositoryManifest) -> HashMap<Version, &RemotePackage> {
    manifest
        .remote_package
        .iter()
        .filter_map(|p| p.path.starts_with("ndk;").then(|| (get_ndk_version(p), p)))
        .collect()
}
pub fn get_ndk_str_versions(
    manifest: &AndroidRepositoryManifest,
) -> HashMap<&str, &RemotePackage> {
    manifest
        .remote_package
        .iter()
        .filter_map(|p| {
            p.path
                .starts_with("ndk;")
                .then(|| (p.path.split_once(";").unwrap().1, p))
        })
        .collect()
}

pub fn download_ndk_version(ndk: &RemotePackage) -> Result<()> {
    let os = if cfg!(linux) {
        "linux"
    } else if cfg!(macos) {
        "macosx"
    } else if cfg!(windows) {
        "windows"
    } else {
        panic!("Unsupported os!")
    };

    let arch = env::consts::ARCH;

    let archive = ndk
        .archives
        .archive
        .iter()
        .find(|a| {
            a.host_os.as_ref().expect("No os?") == os
                && a.host_arch.as_ref().expect("No arch?") == arch
        })
        .expect("Could not find ndk for current os and arch");

    let archive_url = format!("{ANDROID_DL_URL}/{}", archive.complete.url);

    println!(
        "Downloading {} from {}, this may take some time",
        get_ndk_version(ndk),
        &archive_url
    );

    let dir = get_combine_config().ndk_download_path.as_ref().unwrap();
    let name = &archive.complete.url;
    let download_file_path = dir.join(name).with_extension("");

    let bytes: Bytes = download_file_report(&archive_url, |_, _| {})?.into();

    let buffer = Cursor::new(bytes);

    // Extract to tmp folde
    let mut archive = ZipArchive::new(buffer)?;

    archive.extract(download_file_path)?;

    Ok(())
}
