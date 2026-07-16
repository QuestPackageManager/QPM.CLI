use qpm_cli::models::android_repo::{
    AndroidRepositoryManifest, Archive, ArchivesType, CompleteType, RemotePackage, RevisionType,
};
use qpm_cli::services::android::{
    get_host_archive, get_ndk_packages, get_ndk_str_versions, get_ndk_str_versions_str,
    get_ndk_version,
};
use semver::Version;

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
