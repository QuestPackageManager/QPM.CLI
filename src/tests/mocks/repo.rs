use std::collections::HashMap;

use qpm_package::models::{
    dependency::{Dependency, SharedDependency, SharedPackageConfig},
    extra::AdditionalPackageMetadata,
    package::{PackageConfig, PackageDependency, PackageMetadata},
};
use semver::{Version, VersionReq};

use crate::repository::local::FileRepository;

pub fn build_artifact_nodeps(name: &str, ver: Version) -> SharedPackageConfig {
    SharedPackageConfig {
        config: PackageConfig {
            shared_dir: "shared".into(),
            workspace: Default::default(),
            dependencies_dir: "extern".into(),
            info: PackageMetadata {
                name: name.to_string(),
                id: name.to_string(),
                url: None,
                version: ver,
                additional_data: Default::default(),
            },
            dependencies: vec![],
            ..Default::default()
        },
        restored_dependencies: vec![],
    }
}
pub fn build_artifact_and_depend(
    name: &str,
    ver: Version,
    shared_dep: &SharedPackageConfig,
    range: VersionReq,
) -> SharedPackageConfig {
    let dep = Dependency {
        id: shared_dep.config.info.id.clone(),
        version_range: range.clone(),
        additional_data: shared_dep.config.info.additional_data.clone(),
    };
    let p_dep = PackageDependency {
        id: shared_dep.config.info.id.clone(),
        version_range: range,
        additional_data: Default::default(),
    };
    SharedPackageConfig {
        config: PackageConfig {
            workspace: Default::default(),
            shared_dir: "shared".into(),

            dependencies_dir: "extern".into(),
            info: PackageMetadata {
                name: name.to_string(),
                id: name.to_string(),
                url: None,
                version: ver,
                additional_data: Default::default(),
            },
            dependencies: vec![p_dep],
            ..Default::default()
        },
        restored_dependencies: vec![SharedDependency {
            dependency: dep,
            version: shared_dep.config.info.version.clone(),
        }],
        ..Default::default()
    }
}
pub fn build_artifact_and_depends(
    name: &str,
    ver: Version,
    deps: &[(&SharedPackageConfig, VersionReq)],
) -> SharedPackageConfig {
    SharedPackageConfig {
        config: PackageConfig {
            workspace: Default::default(),
            shared_dir: "shared".into(),

            dependencies_dir: "extern".into(),
            info: PackageMetadata {
                name: name.to_string(),
                id: name.to_string(),
                url: None,
                version: ver,
                additional_data: AdditionalPackageMetadata::default(),
            },
            dependencies: deps
                .iter()
                .map(|(shared_config, range)| PackageDependency {
                    id: shared_config.config.info.id.clone(),
                    version_range: range.clone(),
                    additional_data: Default::default(),
                })
                .collect(),
            ..Default::default()
        },
        restored_dependencies: deps
            .iter()
            .map(|(shared_config, range)| SharedDependency {
                dependency: Dependency {
                    id: shared_config.config.info.id.clone(),
                    version_range: range.clone(),
                    additional_data: shared_config.config.info.additional_data.clone(),
                },
                version: shared_config.config.info.version.clone(),
            })
            .collect(),
    }
}

pub fn get_mock_repository() -> FileRepository {
    let artifact1 = build_artifact_nodeps("artifact1", Version::new(0, 1, 0));
    let artifact2 = build_artifact_nodeps("artifact2", Version::new(0, 1, 0));
    let artifact3 = build_artifact_and_depend(
        "artifact3",
        Version::new(0, 1, 0),
        &artifact1,
        VersionReq::STAR,
    );
    // example of a dependency hierarchy
    let artifact4 = build_artifact_and_depends(
        "artifact4",
        Version::new(0, 1, 0),
        &[
            (&artifact1, VersionReq::STAR),
            (&artifact2, VersionReq::STAR),
            (&artifact3, VersionReq::STAR),
        ],
    );
    // unmatchabl dependency
    let artifact5 = build_artifact_and_depend(
        "artifact5",
        Version::new(0, 1, 0),
        &artifact4,
        VersionReq::parse(">=1.0.0").unwrap(),
    );

    FileRepository {
        artifacts: [artifact1, artifact2, artifact3, artifact4, artifact5]
            .map(|a| {
                (
                    a.config.info.id.clone(),
                    HashMap::from([(a.config.info.version.clone(), a)]),
                )
            })
            .into_iter()
            .collect(),
    }
}
