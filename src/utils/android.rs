use std::{collections::HashMap, env, io::Cursor};

use bytes::Bytes;
use color_eyre::Result;
use owo_colors::OwoColorize;
use semver::{BuildMetadata, Prerelease, Version};

use zip::ZipArchive;

use crate::{
    models::{
        android_repo::{AndroidRepositoryManifest, Archive, RemotePackage},
        config::get_combine_config,
    },
    network::agent::{download_file_report, get_agent},
    terminal::colors::QPMColor,
};

const ANDROID_REPO_MANIFEST: &str = "https://dl.google.com/android/repository/repository2-3.xml";
const ANDROID_DL_URL: &str = "https://dl.google.com/android/repository";

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

pub fn get_ndk_str_versions(manifest: &AndroidRepositoryManifest) -> HashMap<&str, &RemotePackage> {
    manifest
        .remote_package
        .iter()
        .filter_map(|p| {
            // if NDK and compatible with host machine
            (p.path.starts_with("ndk;") && get_host_archive(p).is_some())
                .then(|| (p.path.split_once(';').unwrap().1, p))
        })
        .collect()
}

pub fn get_host_archive(ndk: &RemotePackage) -> Option<&Archive> {
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macosx"
    } else if cfg!(windows) {
        "windows"
    } else {
        panic!("Unsupported os!")
    };

    let arch = if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
        "x64"
    } else {
        env::consts::ARCH
    };

    ndk.archives.archive.iter().find(|a| {
        a.host_os.as_ref().is_some_and(|s| s == os)
            && (a.host_arch.is_none() || a.host_arch.as_ref().is_some_and(|s| s == arch))
    })
}

pub fn download_ndk_version(ndk: &RemotePackage) -> Result<()> {
    let archive = get_host_archive(ndk).expect("Could not find ndk for current os and arch");

    let archive_url = format!("{ANDROID_DL_URL}/{}", archive.complete.url);

    println!(
        "Downloading {} from {}, this may take some time",
        get_ndk_version(ndk).blue(),
        &archive_url.yellow()
    );

    let dir = get_combine_config()
        .ndk_download_path
        .as_ref()
        .expect("No NDK download path set");
    let _name = &archive.complete.url;

    let bytes: Bytes = download_file_report(&archive_url, |_, _| {})?.into();

    println!("Extracting ndk");
    let buffer = Cursor::new(bytes);

    // Extract to tmp folde
    let mut archive = ZipArchive::new(buffer)?;
    let final_path = dir.join(archive.by_index(0)?.name());

    archive.extract(dir)?;

    println!(
        "Downloaded {} to {}",
        get_ndk_version(ndk).green(),
        final_path.to_str().unwrap().file_path_color()
    );
    Ok(())
}
