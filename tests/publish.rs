mod mocks;

use std::collections::HashMap;

use qpm_cli::services::publish::PackagePublisher;
use qpm_package::models::{
    package::{DependencyId, PackageConfig, PackageDependency},
    shared_package::{SharedDependencyInfo, SharedPackageConfig},
};
use semver::{Version, VersionReq};

use mocks::mock_repository::MockRepository;

fn dependency_info(version: Version) -> SharedDependencyInfo {
    SharedDependencyInfo {
        restored_version: version,
        qpkg_url: None,
        qpkg_checksum: None,
        restored_binaries: vec![],
        restored_env: Default::default(),
    }
}

/// `PackagePublisher::validate` checks every dependency before ever touching the network
/// (downloading the qpkg to verify it), so these failure paths are fully exercisable offline.
#[test]
fn validate_fails_when_a_restored_dependency_is_missing_from_the_repo() {
    let repo = MockRepository::new(true);

    let dep_id = DependencyId("beatsaber-hook".to_string());
    let package = PackageConfig {
        id: DependencyId("my-mod".to_string()),
        version: Version::new(1, 0, 0),
        dependencies: HashMap::from([(
            dep_id.clone(),
            PackageDependency {
                version_range: VersionReq::STAR,
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    let shared_package = SharedPackageConfig {
        config: package.clone(),
        restored_dependencies: HashMap::from([(dep_id, dependency_info(Version::new(5, 1, 9)))]),
        env: Default::default(),
    };

    // Note: repo has no packages registered at all, so the restored dependency lookup fails.
    let result = PackagePublisher::validate(
        package,
        &shared_package,
        "https://example.invalid/does-not-matter.qpkg".to_string(),
        &repo,
    );

    let err = result.expect_err("validate should fail when the repo doesn't have the dependency");
    assert!(
        err.to_string().contains("not available on qpackages"),
        "unexpected error: {err}"
    );
}

#[test]
fn validate_fails_when_a_declared_dependency_was_never_restored() {
    let repo = MockRepository::new(true);

    let dep_id = DependencyId("beatsaber-hook".to_string());
    let package = PackageConfig {
        id: DependencyId("my-mod".to_string()),
        version: Version::new(1, 0, 0),
        dependencies: HashMap::from([(
            dep_id,
            PackageDependency {
                version_range: VersionReq::STAR,
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    // restored_dependencies is empty, even though `package.dependencies` declares one.
    let shared_package = SharedPackageConfig {
        config: package.clone(),
        restored_dependencies: HashMap::new(),
        env: Default::default(),
    };

    let result = PackagePublisher::validate(
        package,
        &shared_package,
        "https://example.invalid/does-not-matter.qpkg".to_string(),
        &repo,
    );

    let err = result.expect_err("validate should fail when a dependency was never restored");
    assert!(
        err.to_string().contains("not found in restored dependencies"),
        "unexpected error: {err}"
    );
}

#[test]
fn validate_fails_when_restored_version_does_not_satisfy_declared_range() {
    let dep_id = DependencyId("beatsaber-hook".to_string());
    let restored_version = Version::new(3, 0, 0);

    let dep_config = PackageConfig {
        id: dep_id.clone(),
        version: restored_version.clone(),
        ..Default::default()
    };

    let repo = MockRepository::new(true).with_package(dep_config);

    let package = PackageConfig {
        id: DependencyId("my-mod".to_string()),
        version: Version::new(1, 0, 0),
        dependencies: HashMap::from([(
            dep_id.clone(),
            PackageDependency {
                // Declared range requires >=5.0.0, but the restored version is 3.0.0.
                version_range: VersionReq::parse(">=5.0.0").unwrap(),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    let shared_package = SharedPackageConfig {
        config: package.clone(),
        restored_dependencies: HashMap::from([(dep_id, dependency_info(restored_version))]),
        env: Default::default(),
    };

    let result = PackagePublisher::validate(
        package,
        &shared_package,
        "https://example.invalid/does-not-matter.qpkg".to_string(),
        &repo,
    );

    let err =
        result.expect_err("validate should fail when the restored version doesn't satisfy the range");
    assert!(
        err.to_string().contains("does not satisfy stated range"),
        "unexpected error: {err}"
    );
}

#[test]
fn validate_passes_dependency_checks_then_fails_on_unreachable_qpkg_url() {
    let dep_id = DependencyId("beatsaber-hook".to_string());
    let restored_version = Version::new(5, 1, 9);

    let dep_config = PackageConfig {
        id: dep_id.clone(),
        version: restored_version.clone(),
        ..Default::default()
    };

    let repo = MockRepository::new(true).with_package(dep_config);

    let package = PackageConfig {
        id: DependencyId("my-mod".to_string()),
        version: Version::new(1, 0, 0),
        dependencies: HashMap::from([(
            dep_id.clone(),
            PackageDependency {
                version_range: VersionReq::parse(">=5.0.0").unwrap(),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    let shared_package = SharedPackageConfig {
        config: package.clone(),
        restored_dependencies: HashMap::from([(dep_id, dependency_info(restored_version))]),
        env: Default::default(),
    };

    // Dependency checks all pass here; the only remaining failure is the QPKG download, which
    // hits a URL that can never resolve to a real host.
    let result = PackagePublisher::validate(
        package,
        &shared_package,
        "https://example.invalid/does-not-exist.qpkg".to_string(),
        &repo,
    );

    let err = result.expect_err("validate should fail once it tries to download the qpkg");
    assert!(
        err.to_string().contains("Validating QPKG failed"),
        "unexpected error: {err}"
    );
}
