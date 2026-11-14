use std::{collections::HashMap, env, fs, io::BufReader, path::Path, path::PathBuf};

use color_eyre::Result;
use owo_colors::OwoColorize;
use semver::{BuildMetadata, Prerelease, Version};

use zip::ZipArchive;

use crate::{
    models::android_repo::{AndroidRepositoryManifest, Archive, RemotePackage},
    services::network::download_to_tempfile,
    terminal::colors::QPMColor,
};

const ANDROID_REPO_MANIFEST: &str = "https://dl.google.com/android/repository/repository2-3.xml";
const ANDROID_DL_URL: &str = "https://dl.google.com/android/repository";

pub fn get_android_manifest(agent: &ureq::Agent) -> Result<AndroidRepositoryManifest> {
    let response = agent
        .get(ANDROID_REPO_MANIFEST)
        .call()?
        .into_body()
        .into_reader();

    Ok(quick_xml::de::from_reader(std::io::BufReader::new(response))?)
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
        ndk.revision.preview.unwrap_or(0).to_string()
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

pub fn download_ndk_version(
    agent: &ureq::Agent,
    ndk: &RemotePackage,
    show_progress: bool,
    ndk_download_path: &Path,
) -> Result<PathBuf> {
    let archive = get_host_archive(ndk).expect("Could not find ndk for current os and arch");

    let archive_url = format!("{ANDROID_DL_URL}/{}", archive.complete.url);

    println!(
        "Downloading {} from {}, this may take some time",
        get_ndk_version(ndk).blue(),
        archive_url.yellow()
    );

    let _name = &archive.complete.url;

    // NDK archives are hundreds of MB; stream to disk instead of memory
    let file = download_to_tempfile(agent, &archive_url, show_progress)?;
    println!("Extracting ndk");
    let buffer = BufReader::new(file);

    // Extract to tmp folde
    let mut archive = ZipArchive::new(buffer)?;
    let extract_path = ndk_download_path.join(archive.by_index(0)?.name()?.as_ref());

    archive.extract(ndk_download_path)?;

    println!(
        "Downloaded {} to {}",
        get_ndk_version(ndk).green(),
        extract_path.to_str().unwrap().file_path_color()
    );

    // Rename to use friendly NDK version name
    let final_path = ndk_download_path.join(get_ndk_version(ndk).to_string());

    fs::rename(&extract_path, &final_path)?;

    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::android_repo::{ArchivesType, CompleteType, RevisionType};

    fn archive_for(os: &str, arch: Option<&str>) -> Archive {
        Archive {
            host_os: Some(os.to_string()),
            host_arch: arch.map(|a| a.to_string()),
            complete: CompleteType {
                size: Some(1234),
                checksum: "deadbeef".to_string(),
                url: format!("android-ndk-{os}.zip"),
            },
        }
    }

    fn ndk_package(
        path: &str,
        major: u64,
        minor: u64,
        micro: u64,
        archives: Vec<Archive>,
    ) -> RemotePackage {
        RemotePackage {
            path: path.to_string(),
            archives: ArchivesType { archive: archives },
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

    #[test]
    fn get_ndk_packages_filters_to_ndk_paths_only() {
        let manifest = AndroidRepositoryManifest {
            license: vec![],
            remote_package: vec![
                ndk_package(
                    "ndk;25.1.0",
                    25,
                    1,
                    0,
                    vec![archive_for(host_os_name(), None)],
                ),
                ndk_package("platforms;android-30", 30, 0, 0, vec![]),
            ],
        };

        let ndks = get_ndk_packages(&manifest);
        assert_eq!(ndks.len(), 1);
        assert_eq!(ndks[0].path, "ndk;25.1.0");
    }

    #[test]
    fn get_ndk_version_reads_revision_fields() {
        let pkg = ndk_package("ndk;25.1.8937393", 25, 1, 8937393, vec![]);
        let version = get_ndk_version(&pkg);
        assert_eq!(version.major, 25);
        assert_eq!(version.minor, 1);
        assert_eq!(version.patch, 8937393);
    }

    #[test]
    fn get_host_archive_matches_current_os() {
        let pkg = ndk_package(
            "ndk;25.1.0",
            25,
            1,
            0,
            vec![
                archive_for("bogus-os", None),
                archive_for(host_os_name(), None),
            ],
        );

        let archive = get_host_archive(&pkg);
        assert!(archive.is_some());
        assert_eq!(archive.unwrap().host_os.as_deref(), Some(host_os_name()));
    }

    #[test]
    fn get_host_archive_none_when_no_matching_os() {
        let pkg = ndk_package("ndk;25.1.0", 25, 1, 0, vec![archive_for("bogus-os", None)]);
        assert!(get_host_archive(&pkg).is_none());
    }

    #[test]
    fn get_ndk_str_versions_parses_path_suffix_as_version() {
        let manifest = AndroidRepositoryManifest {
            license: vec![],
            remote_package: vec![ndk_package(
                "ndk;25.1.0",
                25,
                1,
                0,
                vec![archive_for(host_os_name(), None)],
            )],
        };

        let by_str = get_ndk_str_versions_str(&manifest);
        assert!(by_str.contains_key("25.1.0"));

        let by_version = get_ndk_str_versions(&manifest);
        assert!(by_version.contains_key(&Version::new(25, 1, 0)));
    }

    #[test]
    fn get_ndk_str_versions_excludes_packages_without_host_archive() {
        let manifest = AndroidRepositoryManifest {
            license: vec![],
            remote_package: vec![ndk_package(
                "ndk;25.1.0",
                25,
                1,
                0,
                vec![archive_for("bogus-os", None)],
            )],
        };

        assert!(get_ndk_str_versions_str(&manifest).is_empty());
    }

    /// Trimmed from the real repository2-3.xml: namespaced root, attributes, text content,
    /// elements with attributes deserialized as plain strings, and ignored unknown elements
    const MANIFEST_XML: &str = r#"<?xml version='1.0' encoding='utf-8'?>
<sdk:sdk-repository xmlns:sdk="http://schemas.android.com/sdk/android/repo/repository2/03" xmlns:generic="http://schemas.android.com/repository/android/generic/02" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <license id="android-sdk-license" type="text">Terms and Conditions</license>
  <channel id="channel-0">stable</channel>
  <remotePackage path="ndk;27.3.13750724">
    <type-details xsi:type="generic:genericDetailsType"/>
    <revision>
      <major>27</major>
      <minor>3</minor>
      <micro>13750724</micro>
    </revision>
    <display-name>NDK (Side by side) 27.3.13750724</display-name>
    <uses-license ref="android-sdk-license"/>
    <channelRef ref="channel-0"/>
    <archives>
      <archive>
        <complete>
          <size>663956036</size>
          <checksum type="sha1">22105e410cf29afcf163760cc95522b9fb981121</checksum>
          <url>android-ndk-r27d-linux.zip</url>
        </complete>
        <host-os>linux</host-os>
      </archive>
    </archives>
  </remotePackage>
</sdk:sdk-repository>"#;

    #[test]
    fn parses_the_android_repository_manifest_format() {
        let manifest: AndroidRepositoryManifest =
            quick_xml::de::from_reader(MANIFEST_XML.as_bytes()).unwrap();

        assert_eq!(manifest.license.len(), 1);
        assert_eq!(manifest.license[0].id, "android-sdk-license");
        assert_eq!(manifest.license[0].typ, "text");
        assert_eq!(manifest.license[0].value, "Terms and Conditions");

        let [pkg] = manifest.remote_package.as_slice() else {
            panic!("expected one package: {:?}", manifest.remote_package);
        };
        assert_eq!(pkg.path, "ndk;27.3.13750724");
        assert_eq!(pkg.display_name, "NDK (Side by side) 27.3.13750724");
        assert_eq!(
            (pkg.revision.major, pkg.revision.minor, pkg.revision.micro, pkg.revision.preview),
            (Some(27), Some(3), Some(13750724), None)
        );
        assert_eq!(pkg.uses_license.as_ref().unwrap().license, "android-sdk-license");
        assert_eq!(pkg.channel.as_ref().unwrap().channel, "channel-0");

        let [archive] = pkg.archives.archive.as_slice() else {
            panic!("expected one archive");
        };
        assert_eq!(archive.host_os.as_deref(), Some("linux"));
        assert_eq!(archive.host_arch, None);
        assert_eq!(archive.complete.size, Some(663956036));
        assert_eq!(archive.complete.checksum, "22105e410cf29afcf163760cc95522b9fb981121");
        assert_eq!(archive.complete.url, "android-ndk-r27d-linux.zip");
    }
}
