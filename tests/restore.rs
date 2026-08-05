use std::{collections::HashMap, fs};

use qpm_cli::{
    models::package::PackageConfigExtensions,
    models::package_files::PackageIdPath,
    repository::{
        Artifact,
        file::{FileRepository, FileRepositoryRegistry},
    },
    services::restore::PackageRestorer,
};
use qpm_package::models::package::{DependencyId, PackageConfig, PackageDependency};
use semver::{Version, VersionReq};
use tempfile::tempdir;

/// Builds an on-disk cache layout for `dep_config` as if it had already been downloaded:
/// `{cache_root}/{id}/{version}/{qpm2.json, src/, lib/}`, and registers it in the returned
/// registry so `FileRepository::get_package`/`download_to_cache` see it as present.
fn seed_cache(
    cache_root: &std::path::Path,
    dep_config: &PackageConfig,
) -> (DependencyId, HashMap<Version, Artifact>) {
    let dep_path = PackageIdPath::new(dep_config.id.clone()).version(dep_config.version.clone());
    let base = dep_path.base_path(cache_root);

    fs::create_dir_all(dep_path.src_path(cache_root)).unwrap();
    fs::write(dep_path.src_path(cache_root).join("foo.h"), b"// header\n").unwrap();

    fs::create_dir_all(dep_path.binaries_path(cache_root)).unwrap();
    fs::write(
        dep_path.binaries_path(cache_root).join("libfoo.so"),
        b"binary contents",
    )
    .unwrap();

    dep_config.write(&base).unwrap();

    (
        dep_config.id.clone(),
        HashMap::from([(
            dep_config.version.clone(),
            Artifact {
                config: dep_config.clone(),
                qpkg_checksum: None,
            },
        )]),
    )
}

/// End-to-end exercise of `PackageRestorer::resolve` + `restore` against a real on-disk cache
/// (no network, no mocked business logic): a dependency is pre-seeded into a tmp cache dir as
/// if already downloaded, resolved via pubgrub, then restored into a tmp workspace - proving
/// headers and binaries actually land where the rest of the toolchain (extern/includes,
/// extern/libs) expects them.
#[test]
fn restore_copies_cached_headers_and_binaries_into_the_workspace() {
    let cache_dir = tempdir().unwrap();
    let workspace_dir = tempdir().unwrap();
    let cache_root = cache_dir.path();

    let dep_id = DependencyId("libfoo".to_string());
    let dep_version = Version::new(1, 0, 0);

    let mut dep_config = PackageConfig {
        id: dep_id.clone(),
        version: dep_version.clone(),
        shared_directory: "shared".into(),
        dependencies_directory: "extern".into(),
        ..Default::default()
    };
    dep_config.workspace.out_binaries = Some(vec!["libfoo.so".into()]);

    let (artifact_id, artifact_versions) = seed_cache(cache_root, &dep_config);

    let registry = FileRepositoryRegistry {
        artifacts: HashMap::from([(artifact_id, artifact_versions)]),
    };

    let mut repo = FileRepository::new(cache_root.to_path_buf(), registry);

    let root_config = PackageConfig {
        id: DependencyId("root".to_string()),
        version: Version::new(0, 1, 0),
        shared_directory: "shared".into(),
        dependencies_directory: "extern".into(),
        dependencies: HashMap::from([(
            dep_id,
            PackageDependency {
                version_range: VersionReq::STAR,
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    let restorer =
        PackageRestorer::resolve(root_config, &repo).expect("dependency resolution should succeed");
    assert_eq!(restorer.resolved_deps().len(), 1);

    let file_repo = repo.clone();
    restorer
        .restore(workspace_dir.path(), &mut repo, &file_repo)
        .expect("restore should succeed against the pre-seeded cache");

    let extern_dir = workspace_dir.path().join("extern");
    let binary_path = extern_dir.join("libs").join("libfoo.so");
    let header_path = extern_dir.join("includes").join("libfoo").join("foo.h");

    assert!(binary_path.exists(), "expected {binary_path:?} to exist");
    assert!(header_path.exists(), "expected {header_path:?} to exist");
    assert_eq!(
        fs::read(&binary_path).unwrap(),
        b"binary contents",
        "restored binary content should match the cached copy"
    );
}
