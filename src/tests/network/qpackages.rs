use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use color_eyre::{Report, Result};
use itertools::Itertools;
use qpm_package::models::{
    package::{DependencyId, PackageConfig, PackageDependency},
    shared_package::SharedPackageConfig,
};
use semver::{Version, VersionReq};

use qpm_cli::{
    models::package_files::PackageIdPath,
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

    assert_eq!(unwrapped_p.id, DependencyId("beatsaber-hook".to_string()));
    assert_eq!(unwrapped_p.version, version);
    Ok(())
}

#[test]
fn resolve() -> Result<()> {
    let repo = QPMRepository::default();
    let version = Version::new(6, 4, 0);
    let p = repo.get_package(&DependencyId("beatsaber-hook".to_owned()), &version)?;

    assert_ne!(p, None);
    let unwrapped_p = p.unwrap();

    let resolved = dependency::resolve(&unwrapped_p, &repo)?.collect_vec();

    assert!(!resolved.is_empty());

    let paper_dep = unwrapped_p
        .dependencies
        .iter()
        .find(|(_, dep)| dep.version_range.to_string().contains("paper"));
    assert!(paper_dep.is_some());

    let paper = resolved.iter().find(|rd| rd.id.0.contains("paper"));

    assert!(paper.is_some());

    assert!(
        paper_dep
            .unwrap()
            .1
            .version_range
            .matches(&paper.unwrap().version)
    );

    println!(
        "Resolved deps: {:?}",
        resolved.iter().map(|s| s.id.clone()).collect_vec()
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

    let resolved = dependency::resolve(&p, &repo);

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
        if let Some(bs) = file_repo
            .artifacts
            .get_mut(&DependencyId("beatsaber-hook".to_owned()))
        {
            bs.remove(&Version::new(5, 1, 9));
        }
        file_repo.write()?;

        let repo = repository::useful_default_new(false)?;

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

    let shared_package = SharedPackageConfig {
        config: package.clone(),
        restored_dependencies: Default::default(),
        env: Default::default(),
    };

    let package_path = PackageIdPath::new(package.id.clone());
    let package_version_path = package_path.version(package.version.clone());

    let lib_path = package_version_path.binary_path(Path::new("libbeatsaber-hook_5_1_9.so"));

    let resolved = {
        let mut repo = get_repo()?;

        let resolved = dependency::resolve(&package, &repo).unwrap().collect_vec();

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
