use std::path::PathBuf;

use bytes::{BufMut, BytesMut};
use color_eyre::{Report, Result, eyre::OptionExt};
use itertools::Itertools;
use qpm_package::models::{
    dependency::SharedPackageConfig,
    extra::PackageDependencyModifier,
    package::{PackageConfig, PackageDependency, PackageMetadata},
};
use semver::{Version, VersionReq};

use qpm_cli::{
    network::agent::download_file_report,
    repository::{self, Repository, local::FileRepository, qpackages::QPMRepository},
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
fn download_package_binary() -> Result<()> {
    let repo = QPMRepository::default();
    let id: &str = "codegen";
    let versions = repo.get_package_versions(id)?.ok_or_eyre("No versions")?;
    let version = &versions.first().unwrap().version;
    let package = repo
        .get_package(id, version)?
        .ok_or_eyre(format!("No package found for {id}/{version:?}"))?;

    let link = package
        .config
        .info
        .additional_data
        .so_link
        .ok_or_eyre("Binary SO not found")?;

    let mut pre_bytes = BytesMut::new().writer();
    download_file_report(&link, &mut pre_bytes, |_, _| {})?;

    let final_bytes = pre_bytes.into_inner();

    let result = String::from_utf8_lossy(&final_bytes);
    println!("Result {result}");

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

    assert!(
        bs_hooks_dep
            .unwrap()
            .version_range
            .matches(&bs_hooks.unwrap().config.info.version)
    );

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
            version: PackageConfig::default().version,

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

#[test]
fn resolve_redownload_cache() -> Result<()> {
    let workspace_tmp_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or(std::env::temp_dir());

    fn get_repo() -> Result<impl Repository> {
        let mut file_repo = FileRepository::read()?;
        if let Some(bs) = file_repo.artifacts.get_mut("beatsaber-hook") {
            bs.remove(&Version::new(5, 1, 9));
        }
        file_repo.write()?;

        let repo = repository::useful_default_new(false)?;

        Ok(repo)
    }

    let shared_package = SharedPackageConfig {
        config: PackageConfig {
            version: PackageConfig::default().version,

            shared_dir: Default::default(),
            dependencies_dir: workspace_tmp_dir.join("extern"),
            info: PackageMetadata {
                name: "T".to_string(),
                id: "t".to_string(),
                version: Version::new(0, 0, 0),
                url: Default::default(),
                additional_data: Default::default(),
            },
            dependencies: vec![PackageDependency {
                id: "beatsaber-hook".to_string(),
                version_range: VersionReq::parse("=5.1.9").unwrap(),
                additional_data: PackageDependencyModifier::default(),
            }],
            workspace: Default::default(),
            ..Default::default()
        },
        restored_dependencies: vec![],
    };

    let path = FileRepository::get_package_cache_path("beatsaber-hook", &Version::new(5, 1, 9));
    let lib_path = path.join("lib").join("libbeatsaber-hook_5_1_9.so");

    let resolved = {
        let mut repo = get_repo()?;

        let resolved = dependency::resolve(&shared_package.config, &repo)
            .unwrap()
            .collect_vec();

        dependency::restore(&workspace_tmp_dir, &shared_package, &resolved, &mut repo)?;

        println!("Lib path: {lib_path:?}");

        assert!(lib_path.exists());
        std::fs::remove_file(&lib_path)?;

        resolved
    };

    {
        let mut repo = get_repo()?;
        assert!(!lib_path.exists());

        dependency::restore(&workspace_tmp_dir, &shared_package, &resolved, &mut repo)?;
        assert!(lib_path.exists());
    }

    Ok(())
}
