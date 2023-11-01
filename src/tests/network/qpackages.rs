use color_eyre::{Report, Result};
use itertools::Itertools;
use qpm_package::models::{
    dependency::SharedPackageConfig,
    extra::PackageDependencyModifier,
    package::{PackageConfig, PackageDependency, PackageMetadata},
};
use semver::{Version, VersionReq};

use crate::{
    repository::{qpackages::QPMRepository, Repository},
    resolver::dependency,
};

#[test]
fn get_artifact_packages() -> Result<()> {
    let repo = QPMRepository::default();
    let names = repo.get_package_names()?;

    assert!(!names.is_empty());
    Ok(())
}
#[test]
fn get_artifact_package_versions() -> Result<()> {
    let repo = QPMRepository::default();
    let versions = repo.get_package_versions("beatsaber-hook")?;

    assert_ne!(versions, None);
    Ok(())
}

#[test]
fn get_artifact() -> Result<()> {
    let repo = QPMRepository::default();
    let version = Version::new(3, 14, 0);
    let p = repo.get_package("beatsaber-hook", &version)?;

    assert_ne!(p, None);
    let unwrapped_p = p.unwrap();

    assert_eq!(unwrapped_p.config.info.id, "beatsaber-hook");
    assert_eq!(unwrapped_p.config.info.version, version);
    Ok(())
}

#[test]
fn resolve() -> Result<()> {
    // TODO: Fix
    return Ok(());
    let repo = QPMRepository::default();
    let version = Version::new(0, 33, 0);
    let p = repo.get_package("codegen", &version)?;

    assert_ne!(p, None);
    let unwrapped_p = p.unwrap();

    let resolved = dependency::resolve(&unwrapped_p.config, &repo)?.collect_vec();

    assert!(!resolved.is_empty());

    let bs_hooks_dep = unwrapped_p
        .config
        .dependencies
        .iter()
        .find(|b| b.id == "beatsaber-hook");
    assert_ne!(bs_hooks_dep, None);

    let bs_hooks = resolved
        .iter()
        .find(|b| b.config.info.id == "beatsaber-hook");

    assert_ne!(bs_hooks, None);

    assert!(bs_hooks_dep
        .unwrap()
        .version_range
        .matches(&bs_hooks.unwrap().config.info.version));

    println!(
        "Resolved deps: {:?}",
        resolved.iter().map(|s| s.config.info.id.clone())
    );

    Ok(())
}

#[test]
fn resolve_fail() -> Result<()> {
    let repo = QPMRepository::default();
    let p = SharedPackageConfig {
        config: PackageConfig {
            shared_dir: Default::default(),
            dependencies_dir: Default::default(),
            info: PackageMetadata {
                name: "T".to_string(),
                id: "t".to_string(),
                version: Version::new(0, 0, 0),
                url: Default::default(),
                additional_data: Default::default(),
            },
            dependencies: vec![
                PackageDependency {
                    id: "beatsaber-hook".to_string(),
                    version_range: VersionReq::parse(">1.0.0").unwrap(),
                    additional_data: PackageDependencyModifier::default(),
                },
                PackageDependency {
                    id: "beatsaber-hook".to_string(),
                    version_range: VersionReq::parse("<1.0.0").unwrap(),
                    additional_data: PackageDependencyModifier::default(),
                },
            ],
            workspace: Default::default(),
            ..Default::default()
        },
        restored_dependencies: vec![],
    };

    let resolved = dependency::resolve(&p.config, &repo);

    assert!(resolved.is_err());
    let report: Report = resolved.err().unwrap();
    println!("{report:?}");

    Ok(())
}
