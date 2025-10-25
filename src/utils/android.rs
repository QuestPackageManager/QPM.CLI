use std::{collections::HashMap, env, fs, io::Cursor, path::PathBuf};

use bytes::{BufMut, BytesMut};
use color_eyre::Result;
use owo_colors::OwoColorize;
use semver::{BuildMetadata, Prerelease, Version};

use zip::ZipArchive;

use crate::{
    models::{
        android_repo::{AndroidRepositoryManifest, Archive, RemotePackage},
        config::get_combine_config,
    },
    network::agent::{download_file, download_file_report, get_agent},
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
    let build = BuildMetadata::new(&format!(
        "preview-{}",
        &ndk.revision.preview.unwrap_or(0).to_string()
    ))
    .unwrap();

    Version {
        major: ndk.revision.major.unwrap_or(0),
        minor: ndk.revision.minor.unwrap_or(0),
        patch: ndk.revision.micro.unwrap_or(0),
        pre: Prerelease::EMPTY,
        build,
    }
}

#[inline(always)]
pub fn get_ndk_str_versions(
    manifest: &AndroidRepositoryManifest,
) -> HashMap<Version, &RemotePackage> {
    get_ndk_str_versions_str(manifest)
        .into_iter()
        .map(|(s, p)| {
            (
                Version::parse(s)
                    .unwrap_or_else(|e| panic!("Unable to parse version string {s}: {e}")),
                p,
            )
        })
        .collect()
}
pub fn get_ndk_str_versions_str(
    manifest: &AndroidRepositoryManifest,
) -> HashMap<&str, &RemotePackage> {
    manifest
        .remote_package
        .iter()
        .filter(|&p| p.path.starts_with("ndk;") && get_host_archive(p).is_some())
        .map(|p| (p.path.split_once(';').unwrap().1, p))
        .collect()
}

///
/// Gets the archive matching the triplet for the current OS
///
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

pub fn download_ndk_version(ndk: &RemotePackage, show_progress: bool) -> Result<PathBuf> {
    let archive = get_host_archive(ndk).expect("Could not find ndk for current os and arch");

    let archive_url = format!("{ANDROID_DL_URL}/{}", archive.complete.url);

    println!(
        "Downloading {} from {}, this may take some time",
        get_ndk_version(ndk).blue(),
        &archive_url.yellow()
    );

    let dir = get_combine_config()
        .ndk_download_path
        .clone()
        .expect("No NDK download path set");
    let _name = &archive.complete.url;

    let mut bytes = BytesMut::new().writer();
    match show_progress {
        true => {
            download_file_report(&archive_url, &mut bytes, |_, _| {})?;
        }
        false => {
            download_file(&archive_url, &mut bytes, |_, _| {})?;
        }
    }
    println!("Extracting ndk");
    let buffer = Cursor::new(bytes.into_inner());

    // Extract to tmp folde
    let mut archive = ZipArchive::new(buffer)?;
    let extract_path = dir.join(archive.by_index(0)?.name());

    archive.extract(&dir)?;

    println!(
        "Downloaded {} to {}",
        get_ndk_version(ndk).green(),
        extract_path.to_str().unwrap().file_path_color()
    );

    // Rename to use friendly NDK version name
    let final_path = dir.join(get_ndk_version(ndk).to_string());

    fs::rename(&extract_path, &final_path)?;

    Ok(final_path)
}
