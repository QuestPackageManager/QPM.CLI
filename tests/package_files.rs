use std::path::Path;

use qpm_cli::models::package_files::PackageIdPath;
use qpm_package::models::package::DependencyId;
use semver::Version;

const ROOT: &str = "/cache";

fn version_path(id: &str, version: &str) -> qpm_cli::models::package_files::PackageVersionPath {
    PackageIdPath::new(DependencyId(id.to_string())).version(Version::parse(version).unwrap())
}

/// `base_path()` should be the cache dir, then the package id, then the version - in that
/// nesting order.
#[test]
fn base_path_nests_under_versions_path_and_id() {
    let path = version_path("some-pkg", "1.2.3");
    let root = Path::new(ROOT);

    let versions_path = path.versions_path(root);
    assert!(
        versions_path.ends_with("some-pkg"),
        "expected {versions_path:?} to end with the package id"
    );

    let base_path = path.base_path(root);
    assert_eq!(base_path, versions_path.join("1.2.3"));
}

/// `src_path`/`tmp_path`/`binaries_path` are all one level directly under `base_path`, named
/// `src`, `tmp`, and `lib` respectively.
#[test]
fn subdirectories_nest_directly_under_base_path() {
    let path = version_path("some-pkg", "1.2.3");
    let root = Path::new(ROOT);
    let base_path = path.base_path(root);

    assert_eq!(path.src_path(root), base_path.join("src"));
    assert_eq!(path.tmp_path(root), base_path.join("tmp"));
    assert_eq!(path.binaries_path(root), base_path.join("lib"));
}

/// `qpm_json_dir`/`qpkg_json_dir` both point at `base_path` itself (the qpm2.json and
/// qpm2.qpkg.json files live alongside the src/tmp/lib subdirectories, not inside a
/// subdirectory of their own).
#[test]
fn json_dirs_are_the_base_path_itself() {
    let path = version_path("some-pkg", "1.2.3");
    let root = Path::new(ROOT);

    assert_eq!(path.qpm_json_dir(root), path.base_path(root));
    assert_eq!(path.qpkg_json_dir(root), path.base_path(root));
}

/// `binary_path` should drop any directory components from the given binary path and just
/// join the file name onto `binaries_path` - the source binary might come from an arbitrary
/// build output location, but the cache always stores it flat.
#[test]
fn binary_path_uses_only_the_file_name() {
    let path = version_path("some-pkg", "1.2.3");
    let root = Path::new(ROOT);

    let binary_path = path.binary_path(root, Path::new("some/nested/build/dir/libfoo.so"));

    assert_eq!(binary_path, path.binaries_path(root).join("libfoo.so"));
}

/// Different package ids and versions must not collide onto the same cache path.
#[test]
fn different_ids_and_versions_produce_different_paths() {
    let a = version_path("pkg-a", "1.0.0");
    let b = version_path("pkg-b", "1.0.0");
    let a2 = version_path("pkg-a", "2.0.0");
    let root = Path::new(ROOT);

    assert_ne!(a.base_path(root), b.base_path(root));
    assert_ne!(a.base_path(root), a2.base_path(root));
}
