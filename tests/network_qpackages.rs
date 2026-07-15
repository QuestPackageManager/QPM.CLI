#![cfg(feature = "network_test")]

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use color_eyre::{Report, Result};
use itertools::Itertools;
use qpm_package::models::package::{DependencyId, PackageConfig, PackageDependency};
use semver::{Version, VersionReq};

use qpm_cli::{
    models::package_files::PackageIdPath,
    repository::{self, Repository, file::FileRepository, qpackages::QPMRepository},
    services::restore::PackageRestorer,
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
    let versions = repo.get_package_versions(&DependencyId("beatsaber-hook".to_owned()))?;

    assert_ne!(versions, None);
    Ok(())
}
// #[test]
// fn download_package_binary() -> Result<()> {
//     let repo = QPMRepository::default();
//     let id = DependencyId("beatsaber-hook".to_owned());
//     let versions = repo.get_package_versions(&id)?.ok_or_eyre("No versions")?;
//     let version = &versions.first().unwrap();
//     let package = repo
//         .get_package(id, version)?
//         .ok_or_eyre(format!("No package found for {id}/{version:?}"))?;

//     let link = package.out_binaries.is_some();

//     let mut pre_bytes = BytesMut::new().writer();
//     download_file_report(&link, &mut pre_bytes, |_, _| {})?;

//     let final_bytes = pre_bytes.into_inner();

//     let result = String::from_utf8_lossy(&final_bytes);
//     println!("Result {result}");

//     Ok(())
// }

#[test]
fn get_artifact() -> Result<()> {
    let repo = QPMRepository::default();
    let version = Version::new(3, 14, 0);
    let p = repo.get_package(&DependencyId("beatsaber-hook".to_owned()), &version)?;

    assert_ne!(p, None);
    let unwrapped_p = p.unwrap();

    assert_eq!(
        unwrapped_p.config.id,
        DependencyId("beatsaber-hook".to_string())
    );
    assert_eq!(unwrapped_p.config.version, version);
    Ok(())
}

#[test]
fn resolve() -> Result<()> {
    let repo = QPMRepository::default();
    let version = Version::new(6, 4, 0);
    let p = repo.get_package(&DependencyId("beatsaber-hook".to_owned()), &version)?;

    assert_ne!(p, None);
    let unwrapped_p = p.unwrap();

    let restorer = PackageRestorer::resolve(unwrapped_p.config.clone(), &repo)?;
    let resolved = restorer.resolved_deps();

    assert!(!resolved.is_empty());

    let paper_dep = unwrapped_p
        .config
        .dependencies
        .iter()
        .find(|(_, dep)| dep.version_range.to_string().contains("paper"));
    assert!(paper_dep.is_some());

    let paper = resolved.iter().find(|rd| rd.config.id.0.contains("paper"));

    assert!(paper.is_some());

    assert!(
        paper_dep
            .unwrap()
            .1
            .version_range
            .matches(&paper.unwrap().config.version)
    );

    println!(
        "Resolved deps: {:?}",
        resolved.iter().map(|s| s.config.id.clone()).collect_vec()
    );

    Ok(())
}

#[test]
fn resolve_fail() -> Result<()> {
    let repo = QPMRepository::default();
    let p = PackageConfig {
        shared_directory: Default::default(),
        dependencies_directory: Default::default(),
        id: DependencyId("t".to_string()),
        version: Version::new(0, 0, 0),
        dependencies: HashMap::from([
            (
                DependencyId("beatsaber-hook".to_string()),
                PackageDependency {
                    version_range: VersionReq::parse(">=5.1.9").unwrap(),
                    ..Default::default()
                },
            ),
            (
                DependencyId("beatsaber-hook".to_string()),
                PackageDependency {
                    version_range: VersionReq::parse("<5.1.9").unwrap(),
                    ..Default::default()
                },
            ),
        ]),
        workspace: Default::default(),
        ..Default::default()
    };

    let resolved = PackageRestorer::resolve(p, &repo);

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
    let cache_root = workspace_tmp_dir.join("cache");

    fn get_repo(cache_root: &Path) -> Result<impl Repository> {
        let mut file_repo = FileRepository::read(cache_root.to_path_buf())?;
        if let Some(bs) = file_repo
            .artifacts_mut()
            .get_mut(&DependencyId("beatsaber-hook".to_owned()))
        {
            bs.remove(&Version::new(5, 1, 9));
        }
        file_repo.write()?;

        let repo = repository::useful_default_new_at(false, cache_root.to_path_buf())?;

        Ok(repo)
    }

    let package = PackageConfig {
        id: DependencyId("t".to_string()),
        version: Version::new(0, 0, 0),
        shared_directory: Default::default(),
        dependencies_directory: workspace_tmp_dir.join("extern"),
        dependencies: HashMap::from([(
            DependencyId("beatsaber-hook".to_string()),
            PackageDependency {
                version_range: VersionReq::parse("=5.1.9").unwrap(),
                ..Default::default()
            },
        )]),
        workspace: Default::default(),
        ..Default::default()
    };

    let package_path = PackageIdPath::new(package.id.clone());
    let package_version_path = package_path.version(package.version.clone());

    let lib_path =
        package_version_path.binary_path(&cache_root, Path::new("libbeatsaber-hook_5_1_9.so"));

    let restorer = {
        let repo = get_repo(&cache_root)?;
        PackageRestorer::resolve(package.clone(), &repo).unwrap()
    };

    {
        let mut repo = get_repo(&cache_root)?;
        let file_repo = FileRepository::read(cache_root.clone())?;

        restorer.restore(&workspace_tmp_dir, &mut repo, &file_repo)?;

        println!("Lib path: {lib_path:?}");

        assert!(lib_path.exists());
        std::fs::remove_file(&lib_path)?;
    }

    {
        let mut repo = get_repo(&cache_root)?;
        let file_repo = FileRepository::read(cache_root.clone())?;
        assert!(!lib_path.exists());

        restorer.restore(&workspace_tmp_dir, &mut repo, &file_repo)?;
        assert!(lib_path.exists());
    }

    Ok(())
}
