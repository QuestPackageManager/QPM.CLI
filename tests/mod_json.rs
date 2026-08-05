mod mocks;

use std::collections::HashMap;

use qpm_cli::models::package::SharedPackageConfigExtensions;
use qpm_package::models::{
    package::{DependencyId, PackageConfig, PackageDependency, QmodConfig, QmodDependencyMode},
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

/// A dependency's resolved package config with no `qmod.downloadUrl` set (e.g. a locally
/// installed nightly qpkg with no separate qmod release) - the common case for a bare/dev
/// build.
fn dep_config_without_qmod_link(id: DependencyId, version: Version) -> PackageConfig {
    PackageConfig {
        id,
        version,
        qmod: QmodConfig {
            download_url: None,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// When a dependency has no qmod release of its own (e.g. a local nightly build), it should
/// not appear in the generated mod.json - even though the depending project's dependency
/// entry never asked to exclude it.
#[test]
fn to_mod_json_excludes_dependency_whose_resolved_config_has_no_qmod_download_url() {
    let dep_id = DependencyId("nightly-lib".to_string());
    let restored_version = Version::new(1, 0, 0);

    let repo = MockRepository::new(true)
        .with_package(dep_config_without_qmod_link(dep_id.clone(), restored_version.clone()));

    let package = PackageConfig {
        id: DependencyId("my-mod".to_string()),
        version: Version::new(1, 0, 0),
        dependencies: HashMap::from([(
            dep_id.clone(),
            PackageDependency {
                version_range: VersionReq::STAR,
                // Not explicitly excluded - left at the default, unset mode.
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    let shared_package = SharedPackageConfig {
        config: package,
        restored_dependencies: HashMap::from([(dep_id, dependency_info(restored_version))]),
        env: Default::default(),
    };

    let mod_json = shared_package.to_mod_json(&repo).unwrap();

    assert!(
        mod_json.dependencies.is_empty(),
        "expected no mod dependencies since the resolved config has no qmod download url, got {:?}",
        mod_json.dependencies
    );
}

/// When that same dependency instead resolves to a version that does have a qmod release
/// (e.g. the officially published version, instead of a local nightly build), it should
/// appear in the generated mod.json as an auto-installable dependency, marked required to
/// match the depending project's own setting for it. Nothing in the depending project needs
/// to change for this to happen - it's purely a consequence of which version got resolved.
#[test]
fn to_mod_json_includes_dependency_once_its_resolved_config_has_a_qmod_download_url() {
    let dep_id = DependencyId("lib".to_string());
    let restored_version = Version::new(1, 0, 0);

    let dep_config = PackageConfig {
        id: dep_id.clone(),
        version: restored_version.clone(),
        qmod: QmodConfig {
            download_url: Some("https://example.invalid/lib.qmod".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    let repo = MockRepository::new(true).with_package(dep_config);

    let package = PackageConfig {
        id: DependencyId("my-mod".to_string()),
        version: Version::new(1, 0, 0),
        dependencies: HashMap::from([(
            dep_id.clone(),
            PackageDependency {
                version_range: VersionReq::STAR,
                qmod: Some(QmodDependencyMode::Required),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    let shared_package = SharedPackageConfig {
        config: package,
        restored_dependencies: HashMap::from([(
            dep_id.clone(),
            dependency_info(restored_version),
        )]),
        env: Default::default(),
    };

    let mod_json = shared_package.to_mod_json(&repo).unwrap();

    assert_eq!(mod_json.dependencies.len(), 1);
    let mod_dep = &mod_json.dependencies[0];
    assert_eq!(mod_dep.id, dep_id.0);
    assert_eq!(
        mod_dep.mod_link.as_deref(),
        Some("https://example.invalid/lib.qmod")
    );
    assert_eq!(mod_dep.required, Some(true));
}

/// When a dependency is explicitly marked `qmod: none`, it should not appear in the
/// generated mod.json even if its resolved version does have a qmod release available.
#[test]
fn to_mod_json_excludes_dependency_explicitly_marked_qmod_none_even_with_a_download_url() {
    let dep_id = DependencyId("lib".to_string());
    let restored_version = Version::new(1, 0, 0);

    let dep_config = PackageConfig {
        id: dep_id.clone(),
        version: restored_version.clone(),
        qmod: QmodConfig {
            download_url: Some("https://example.invalid/lib.qmod".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    let repo = MockRepository::new(true).with_package(dep_config);

    let package = PackageConfig {
        id: DependencyId("my-mod".to_string()),
        version: Version::new(1, 0, 0),
        dependencies: HashMap::from([(
            dep_id.clone(),
            PackageDependency {
                version_range: VersionReq::STAR,
                qmod: Some(QmodDependencyMode::None),
                ..Default::default()
            },
        )]),
        ..Default::default()
    };

    let shared_package = SharedPackageConfig {
        config: package,
        restored_dependencies: HashMap::from([(dep_id, dependency_info(restored_version))]),
        env: Default::default(),
    };

    let mod_json = shared_package.to_mod_json(&repo).unwrap();

    assert!(mod_json.dependencies.is_empty());
}
