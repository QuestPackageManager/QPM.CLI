use super::package::format::reserialize_package;
use clap::{Args, Subcommand};
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, bail},
};
use qpm_package::models::{
    package::{DependencyId, PackageConfig},
    shared_package::SharedPackageConfig,
    triplet::{PackageTripletDependency, TripletId, base_triplet_id},
};
use semver::{Version, VersionReq};

use crate::{
    models::package::{PackageConfigExtensions, SharedPackageConfigExtensions},
    repository::{self, Repository},
    terminal::colors::QPMColor,
};

use super::Command;

#[derive(Args, Debug, Clone)]
pub struct DependencyCommand {
    #[clap(subcommand)]
    pub op: DependencyOperation,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DependencyOperation {
    /// Add a dependency
    Add(DependencyOperationAddArgs),
    /// Remove a dependency
    Remove(DependencyOperationRemoveArgs),
    /// Download a dependency to the local cache
    Download(DependencyOperationDownloadArgs),
    /// Sort the dependencies alphabetically
    Sort,
}

#[derive(Args, Debug, Clone)]
pub struct DependencyOperationAddArgs {
    #[clap(long, default_value = "false")]
    offline: bool,

    /// Id of the dependency as listed on qpackages
    pub id: String,

    /// Triplet to add the dependency to, if not specified, the restored triplet is used
    #[clap(long, short)]
    pub triplet: Option<String>,

    /// optional version of the dependency that you want to add
    #[clap(short, long)]
    pub version: Option<VersionReq>,

    /// Additional data for the dependency (as a valid json object)
    #[clap(long)]
    pub additional_data: Option<String>,

    /// If the dependencies should be sorted after removing
    #[clap(long, default_value = "false")]
    pub sort: bool,
}

#[derive(Args, Debug, Clone)]
pub struct DependencyOperationDownloadArgs {
    /// Id of the dependency as listed on qpackages
    pub id: String,

    /// version of the dependency that you want to download
    #[clap(short, long)]
    pub version: Option<Version>,

    /// Triplet to download the dependency for, if not specified, the restored triplet is used
    #[clap(long, short)]
    pub triplet: Option<String>,

    /// Resolve all dependencies of the package
    #[clap(long, default_value = "false")]
    pub recursive: bool,
}

#[derive(Args, Debug, Clone)]
pub struct DependencyOperationRemoveArgs {
    /// Id of the dependency as listed on qpackages
    pub id: String,

    /// Triplet to remove the dependency from, if not specified, the restored triplet is used
    #[clap(long, short)]
    pub triplet: Option<String>,

    /// If the dependencies should be sorted after removing
    #[clap(long, default_value = "false")]
    pub sort: bool,
}

impl Command for DependencyCommand {
    fn execute(self) -> color_eyre::Result<()> {
        match self.op {
            DependencyOperation::Add(a) => a.execute(),
            DependencyOperation::Remove(r) => remove_dependency(r),
            DependencyOperation::Download(f) => download_dependency(f),
            DependencyOperation::Sort => reserialize_package(true),
        }
    }
}

impl Command for DependencyOperationAddArgs {
    fn execute(self) -> Result<()> {
        let id = DependencyId(self.id);

        let repo = repository::useful_default_new(self.offline)?;

        let versions = repo
            .get_package_versions(&id)
            .context("No version found for dependency")?;

        if versions.is_none() || versions.as_ref().unwrap().is_empty() {
            bail!(
                "Package {} does not seem to exist qpackages, please make sure you spelled it right, and that it's an actual package!",
                id.dependency_id_color()
            );
        }

        let version = match self.version {
            Option::Some(v) => v,
            // if no version given, use ^latest instead, should've specified a version idiot
            Option::None => {
                semver::VersionReq::parse(&format!("^{}", versions.unwrap().first().unwrap()))
                    .unwrap()
            }
        };

        let additional_data = match &self.additional_data {
            Option::Some(d) => Some(serde_json::from_str(d)?),
            Option::None => None,
        };

        let triplet = self
            .triplet
            .map(TripletId)
            .or_else(|| {
                let shared_package = SharedPackageConfig::read(".");
                let restored = shared_package.ok()?.restored_triplet;
                Some(restored)
            });

        put_dependency(
            &id,
            triplet.as_ref(),
            version,
            additional_data,
            self.sort,
        )
    }
}

fn put_dependency(
    id: &DependencyId,
    triplet: Option<&TripletId>,
    version: VersionReq,
    new_triplet_dep: Option<PackageTripletDependency>,
    sort: bool,
) -> Result<()> {
    println!(
        "Adding dependency with id {} and version {}",
        id.dependency_id_color(),
        version.dependency_version_color()
    );

    let mut package = PackageConfig::read(".")?;
    let triplet = match triplet {
        Some(triplet) => package
            .triplets
            .specific_triplets
            .get_mut(triplet)
            .context("Triplet not found")?,
        None => &mut package.triplets.base,
    };

    let existing_dep = triplet.dependencies.get(id);

    if existing_dep.is_some() {
        println!("Dependency already in qpm.json, updating!");
    }

    let dep = PackageTripletDependency {
        version_range: version,
        ..new_triplet_dep
            .or(existing_dep.cloned())
            .unwrap_or_default()
    };
    triplet.dependencies.insert(id.clone(), dep);

    // if sort {
    //     triplet.dependencies.sort_by(|a, b| a.id.cmp(&b.id));
    // }

    package.write(".")?;
    Ok(())
}

fn remove_dependency(dependency_args: DependencyOperationRemoveArgs) -> Result<()> {
    let mut package = PackageConfig::read(".")?;

    let triplet = match dependency_args.triplet {
        Some(triplet) => package
            .triplets
            .specific_triplets
            .get_mut(&TripletId(triplet))
            .context("Triplet not found")?,
        None => &mut package.triplets.base,
    };

    triplet
        .dependencies
        .retain(|p, _| p.0 != dependency_args.id);

    // if dependency_args.sort {
    //     triplet.dependencies.sort_by(|a, b| a.id.cmp(&b.id));
    // }

    package.write(".")?;
    Ok(())
}

fn download_dependency(dependency_args: DependencyOperationDownloadArgs) -> Result<()> {
    let id = DependencyId(dependency_args.id);

    let mut repository = repository::useful_default_new(false)?;
    let version = match dependency_args.version {
        Some(v) => v,
        _ => {
            let versions = repository.get_package_versions(&id)?.with_context(|| {
                format!(
                    "Package {} does not seem to exist, please make sure you spelled it right.",
                    id.dependency_id_color()
                )
            })?;

            // return the latest version
            versions.first().expect("No versions?").clone()
        }
    };

    let package = repository.get_package(&id, &version)?.with_context(|| {
        format!(
            "Failed to resolve package {}:{}",
            id.dependency_id_color(),
            version.dependency_version_color()
        )
    })?;

    let version = package.version.clone();

    // if recursive is true, resolve the dependencies of the package
    if dependency_args.recursive
        && let Ok(resolved_deps) = SharedPackageConfig::resolve_from_package(
            package.clone(),
            dependency_args.triplet.map(TripletId),
            &repository,
        )
    {
        let resolved_deps = resolved_deps.1;

        for (_triplet, triplet_deps) in resolved_deps {
            for dep in triplet_deps {
                println!(
                    "Pulling {}:{}",
                    id.dependency_id_color(),
                    version.to_string().dependency_version_color()
                );
                repository.download_to_cache(&dep.0).with_context(|| {
                    format!(
                        "Requesting {}:{}",
                        id.dependency_id_color(),
                        version.version_id_color()
                    )
                })?;
                repository.add_to_db_cache(dep.0, true)?;
            }
        }

        repository.write_repo()?;
    }

    println!(
        "Pulling {}:{}",
        id.dependency_id_color(),
        version.to_string().dependency_version_color()
    );
    repository.download_to_cache(&package).with_context(|| {
        format!(
            "Requesting {}:{}",
            id.dependency_id_color(),
            version.version_id_color()
        )
    })?;
    repository.add_to_db_cache(package, true)?;

    repository.write_repo()?;

    Ok(())
}
