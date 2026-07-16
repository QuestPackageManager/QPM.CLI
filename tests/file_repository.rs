use std::{fs, io::Cursor, path::Path};

use qpm_cli::{
    models::qpkg_file::QpkgFile,
    repository::{
        Repository,
        file::{FileRepository, FileRepositoryRegistry},
    },
};
use qpm_package::models::package::{DependencyId, PackageConfig};
use semver::Version;
use tempfile::tempdir;

fn sample_qpkg_config() -> PackageConfig {
    let mut config = PackageConfig {
        id: DependencyId("libfoo".to_string()),
        version: Version::new(1, 0, 0),
        shared_directory: "shared".into(),
        dependencies_directory: "extern".into(),
        ..Default::default()
    };
    config.workspace.out_binaries = Some(vec!["libfoo.so".into()]);
    config
}

/// Builds a real, valid `.qpkg` zip in memory (manifest + one header + one binary), matching
/// what `qpm2 qpkg` would actually produce.
fn build_qpkg(config: PackageConfig) -> Cursor<Vec<u8>> {
    let headers: Vec<(&Path, &[u8])> = vec![(Path::new("foo.h"), b"// header content\n")];
    let binaries: Vec<(&Path, &[u8])> = vec![(Path::new("libfoo.so"), b"binary content")];

    QpkgFile::create(Cursor::new(Vec::new()), config, headers, binaries)
        .expect("building the in-memory qpkg should succeed")
        .into_inner()
}

/// Full lifecycle against a real on-disk cache (no mocked business logic): build a real
/// `.qpkg` archive, install it into a fresh `FileRepository`, confirm the registry and cache
/// directory reflect the install, then re-read the repository from disk (a fresh instance, as
/// a later `qpm2` invocation would) and confirm `download_to_cache` recognizes the package as
/// already present without needing to fetch anything.
#[test]
fn install_qpkg_then_read_back_from_a_fresh_file_repository() {
    let config = sample_qpkg_config();
    let qpkg = build_qpkg(config.clone());

    let cache_dir = tempdir().unwrap();
    let mut file_repo = FileRepository::new(
        cache_dir.path().to_path_buf(),
        FileRepositoryRegistry::default(),
    );

    let artifact = file_repo
        .install_qpkg(qpkg, false, None)
        .expect("installing a well-formed qpkg should succeed");

    assert_eq!(artifact.config.id, config.id);
    assert_eq!(artifact.config.version, config.version);
    assert!(artifact.qpkg_checksum.is_some());

    // The in-memory registry should reflect the install immediately.
    let in_memory = file_repo
        .get_artifact(&config.id, &config.version)
        .expect("artifact should be registered in the repository right after install");
    assert_eq!(in_memory.config.id, config.id);

    // The header and binary should have actually landed on disk where the rest of the
    // toolchain (collect_files_of_package / restore) expects them.
    let files = file_repo
        .collect_files_of_package(&artifact.config)
        .expect("cached files should be discoverable right after install");
    assert!(files.headers.exists());
    assert_eq!(files.binaries.len(), 1);
    assert!(files.binaries[0].exists());
    assert_eq!(fs::read(&files.binaries[0]).unwrap(), b"binary content");

    // Simulate a later, separate `qpm2` invocation: read the repository fresh from disk
    // rather than reusing the in-memory instance that performed the install.
    let mut reread_repo = FileRepository::read(cache_dir.path().to_path_buf())
        .expect("re-reading the repository from disk should succeed");

    let reread_artifact = reread_repo
        .get_artifact(&config.id, &config.version)
        .expect("the installed artifact should persist across a fresh read of the registry");
    assert_eq!(reread_artifact.config.id, config.id);
    assert_eq!(reread_artifact.config.version, config.version);

    let already_cached = reread_repo
        .download_to_cache(&reread_artifact.config.clone())
        .expect("download_to_cache should succeed for an already-installed package");
    assert!(
        already_cached,
        "a freshly installed, freshly re-read qpkg should be recognized as already cached, \
         not require re-downloading"
    );
}

/// Installing the same package id/version twice without `overwrite_existing` should fail
/// instead of silently clobbering the existing cache entry.
#[test]
fn install_qpkg_refuses_to_overwrite_by_default() {
    let config = sample_qpkg_config();

    let cache_dir = tempdir().unwrap();
    let mut file_repo = FileRepository::new(
        cache_dir.path().to_path_buf(),
        FileRepositoryRegistry::default(),
    );

    file_repo
        .install_qpkg(build_qpkg(config.clone()), false, None)
        .expect("first install should succeed");

    let second_install = file_repo.install_qpkg(build_qpkg(config), false, None);

    assert!(
        second_install.is_err(),
        "installing the same id/version twice without overwrite_existing should fail"
    );
}

/// With `overwrite_existing: true`, re-installing the same id/version should succeed and
/// replace the cached contents.
#[test]
fn install_qpkg_overwrites_when_requested() {
    let config = sample_qpkg_config();

    let cache_dir = tempdir().unwrap();
    let mut file_repo = FileRepository::new(
        cache_dir.path().to_path_buf(),
        FileRepositoryRegistry::default(),
    );

    file_repo
        .install_qpkg(build_qpkg(config.clone()), false, None)
        .expect("first install should succeed");

    let second_install = file_repo
        .install_qpkg(build_qpkg(config.clone()), true, None)
        .expect("overwrite install should succeed");

    assert_eq!(second_install.config.id, config.id);
}
