//! QPKG checksum validation: what `install_qpkg` records, what a download is verified
//! against, and what a lock file pins. Downloads are served by a tiny local HTTP server, so
//! no real network is used.

#[path = "support/mod.rs"]
mod support;

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Cursor, Write},
    net::TcpListener,
    thread,
};

use qpm_cli::{
    models::package_files::PackageIdPath,
    repository::{
        Artifact, Repository,
        file::{FileRepository, FileRepositoryRegistry},
    },
    services::restore::PackageRestorer,
};
use qpm_package::models::{
    package::PackageConfig,
    shared_package::{SharedDependencyInfo, SharedPackageConfig},
};
use semver::Version;
use sha2::{Digest, Sha256};
use support::{depend_on, id, locked, package, qpkg_bytes, test_agent};
use tempfile::tempdir;

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn sample() -> PackageConfig {
    package("summed", Version::new(1, 0, 0), Some(&["libsummed.so"]))
}

/// Serves `body` to every request on a fresh localhost port; returns the URL
fn serve(body: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/pkg.qpkg", listener.local_addr().unwrap());
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            // drain the request headers
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|n| n > 0) && line != "\r\n" {
                line.clear();
            }
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(&body);
        }
    });
    url
}

fn assert_not_installed(repo: &FileRepository, config: &PackageConfig) {
    assert!(repo.get_artifact(&config.id, &config.version).is_none());
    assert!(
        !PackageIdPath::new(config.id.clone())
            .versions_path(repo.root())
            .exists(),
        "nothing should be extracted into the cache"
    );
}

#[test]
fn install_qpkg_records_the_archive_sha256() {
    let cache = tempdir().unwrap();
    let bytes = qpkg_bytes(&sample());
    let mut repo = FileRepository::read(cache.path().to_path_buf()).unwrap();

    let artifact = repo.install_qpkg(Cursor::new(bytes.clone()), false, None).unwrap();

    assert_eq!(artifact.qpkg_checksum, Some(sha256_hex(&bytes)));
    // and it's persisted in the index
    let reread = FileRepository::read(cache.path().to_path_buf()).unwrap();
    let stored = reread
        .get_artifact(&id("summed"), &Version::new(1, 0, 0))
        .unwrap();
    assert_eq!(stored.qpkg_checksum, Some(sha256_hex(&bytes)));
}

#[test]
fn download_with_matching_checksum_installs() {
    let cache = tempdir().unwrap();
    let bytes = qpkg_bytes(&sample());
    let url = serve(bytes.clone());
    let mut repo = FileRepository::read(cache.path().to_path_buf()).unwrap();

    let artifact = repo
        .install_qpkg_from_url(&test_agent(), &url, Some(&sha256_hex(&bytes)), false, None)
        .unwrap();

    assert_eq!(artifact.config, sample());
    assert_eq!(artifact.qpkg_url.as_deref(), Some(url.as_str()));
}

#[test]
fn checksum_comparison_ignores_hex_case() {
    let cache = tempdir().unwrap();
    let bytes = qpkg_bytes(&sample());
    let url = serve(bytes.clone());
    let mut repo = FileRepository::read(cache.path().to_path_buf()).unwrap();

    repo.install_qpkg_from_url(
        &test_agent(),
        &url,
        Some(&sha256_hex(&bytes).to_uppercase()),
        false,
        None,
    )
    .expect("an uppercase checksum should still match");
}

#[test]
fn download_without_checksum_skips_verification() {
    let cache = tempdir().unwrap();
    let bytes = qpkg_bytes(&sample());
    let url = serve(bytes.clone());
    let mut repo = FileRepository::read(cache.path().to_path_buf()).unwrap();

    let artifact = repo
        .install_qpkg_from_url(&test_agent(), &url, None, false, None)
        .unwrap();
    assert_eq!(artifact.qpkg_checksum, Some(sha256_hex(&bytes)));
}

#[test]
fn download_with_wrong_checksum_fails_and_installs_nothing() {
    let cache = tempdir().unwrap();
    let url = serve(qpkg_bytes(&sample()));
    let mut repo = FileRepository::read(cache.path().to_path_buf()).unwrap();

    let err = repo
        .install_qpkg_from_url(&test_agent(), &url, Some(&sha256_hex(b"other")), false, None)
        .expect_err("a mismatched checksum must fail");

    assert!(format!("{err:?}").contains("Checksum mismatch"), "{err:?}");
    assert_not_installed(&repo, &sample());
}

/// The path a normal restore takes for a dependency that isn't cached yet
#[test]
fn download_to_cache_with_wrong_checksum_fails_and_installs_nothing() {
    let cache = tempdir().unwrap();
    let url = serve(qpkg_bytes(&sample()));
    let mut repo = FileRepository::read(cache.path().to_path_buf()).unwrap();

    let artifact = Artifact {
        config: sample(),
        qpkg_checksum: Some(sha256_hex(b"other")),
        qpkg_url: Some(url),
    };
    let err = repo
        .download_to_cache(&test_agent(), &artifact)
        .expect_err("a mismatched checksum must fail");

    assert!(format!("{err:?}").contains("Checksum mismatch"), "{err:?}");
    assert_not_installed(&repo, &sample());
}

/// A root depending on `summed`, locked to `locked_checksum`, against a repository that
/// has `summed` with `repo_checksum`
fn locked_resolve(
    locked_checksum: Option<&str>,
    repo_checksum: Option<&str>,
) -> color_eyre::Result<()> {
    let mut repo = FileRepository::new(
        tempdir().unwrap().path().to_path_buf(),
        FileRepositoryRegistry::default(),
    );
    repo.add_artifact_to_map(sample(), repo_checksum.map(Into::into), true)
        .unwrap();

    let mut root = package("root", Version::new(1, 0, 0), None);
    depend_on(&mut root, "summed", "^1.0.0");
    let shared = SharedPackageConfig {
        config: root,
        restored_dependencies: HashMap::from([(
            id("summed"),
            SharedDependencyInfo {
                qpkg_checksum: locked_checksum.map(Into::into),
                ..locked(Version::new(1, 0, 0))
            },
        )]),
        env: Default::default(),
    };

    PackageRestorer::locked_resolve(&shared, &repo).map(|_| ())
}

#[test]
fn locked_resolve_rejects_a_checksum_different_from_the_lock_file() {
    let err = locked_resolve(Some("aaaa"), Some("bbbb")).expect_err("checksums differ");
    let msg = format!("{err:?}");
    assert!(msg.contains("Checksum mismatch"), "{msg}");
    assert!(msg.contains("aaaa") && msg.contains("bbbb"), "{msg}");
}

#[test]
fn locked_resolve_accepts_matching_or_unknown_checksums() {
    locked_resolve(Some("abcd"), Some("abcd")).unwrap();
    locked_resolve(Some("ABCD"), Some("abcd")).unwrap();
    locked_resolve(None, Some("abcd")).unwrap();
    locked_resolve(Some("abcd"), None).unwrap();
    locked_resolve(None, None).unwrap();
}

/// End to end: tampering with the checksum in qpm2.shared.json makes the locked restore fail
#[cfg(feature = "cli")]
#[test]
fn restore_fails_when_the_lock_file_checksum_was_changed() {
    use qpm_cli::models::package::PackageConfigExtensions;
    use support::{run_qpm, user_config, workspace};

    let root = tempdir().unwrap();
    let cache = tempdir().unwrap();
    let user = user_config(cache.path());

    let qpkg = root.path().join("summed.qpkg");
    std::fs::write(&qpkg, qpkg_bytes(&sample())).unwrap();
    run_qpm(&user, root.path(), &["install", "--path", qpkg.to_str().unwrap()]).unwrap();

    let mut consumer = package("consumer", Version::new(1, 0, 0), None);
    depend_on(&mut consumer, "summed", "^1.0.0");
    let dir = workspace(root.path(), &consumer);
    run_qpm(&user, &dir, &["restore", "--offline"]).unwrap();

    let mut shared = SharedPackageConfig::read(&dir).unwrap();
    let locked = shared.restored_dependencies.get_mut(&id("summed")).unwrap();
    assert!(
        locked.qpkg_checksum.is_some(),
        "restore should lock the checksum"
    );
    locked.qpkg_checksum = Some(sha256_hex(b"tampered"));
    shared.write(&dir).unwrap();

    let err = run_qpm(&user, &dir, &["restore", "--offline"])
        .expect_err("the locked restore should notice the checksum change");
    assert!(format!("{err:?}").contains("Checksum mismatch"), "{err:?}");
}
