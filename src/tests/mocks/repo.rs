use std::collections::HashMap;

use qpm_package::models::{
    package::{DependencyId, PackageConfig, PackageDependency},
    shared_package::SharedPackageConfig,
};
use semver::{Version, VersionReq};

use crate::repository::local::FileRepository;

pub fn build_artifact_nodeps(name: &str, ver: Version) -> SharedPackageConfig {
    build_artifact_and_depends(name, ver, &[])
}

pub fn build_artifact_and_depend(
    name: &str,
    ver: Version,
    shared_dep: &SharedPackageConfig,
    range: VersionReq,
) -> SharedPackageConfig {
    build_artifact_and_depends(name, ver, &[(shared_dep, range)])
}

pub fn build_artifact_and_depends(
    name: &str,
    ver: Version,
    deps: &[(&SharedPackageConfig, VersionReq)],
) -> SharedPackageConfig {
    let dependencies = deps
        .iter()
        .map(|(dep, range)| {
            (
                dep.config.id.clone(),
                PackageDependency {
                    version_range: range.clone(),
                    ..Default::default()
                },
            )
        })
        .collect();

    SharedPackageConfig {
        restored_dependencies: Default::default(),
        env: Default::default(),
        config: PackageConfig {
            id: DependencyId(name.to_string()),
            version: ver,
            shared_directory: "shared".into(),
            dependencies_directory: "extern".into(),
            additional_data: Default::default(),
            dependencies,
            ..Default::default()
        },
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
                    a.config.id.clone(),
                    HashMap::from([(a.config.version.clone(), a.config.clone())]),
                )
            })
            .into_iter()
            .collect(),
    }
}
