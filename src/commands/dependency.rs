use super::package::format::reserialize_package;
use clap::{Args, Subcommand};
use color_eyre::{
    Result,
    eyre::{Context, bail},
};
use owo_colors::OwoColorize;
use qpm_package::models::{
    dependency::SharedPackageConfig, extra::PackageDependencyModifier, package::{PackageConfig, PackageDependency}
};
use semver::{Version, VersionReq};

use crate::{
    models::package::{PackageConfigExtensions, SharedPackageConfigExtensions},
    repository::{self, local::FileRepository, Repository},
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

    /// Resolve all dependencies of the package
    #[clap(long, default_value = "false")]
    pub recursive: bool,
}

#[derive(Args, Debug, Clone)]
pub struct DependencyOperationRemoveArgs {
    /// Id of the dependency as listed on qpackages
    pub id: String,

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
        if self.id == "yourmom" {
            bail!("The dependency was too big to add, we can't add this one!");
        }

        let repo = repository::useful_default_new(self.offline)?;

        let versions = repo
            .get_package_versions(&self.id)
            .context("No version found for dependency")?;

        if versions.is_none() || versions.as_ref().unwrap().is_empty() {
            bail!(
                "Package {} does not seem to exist qpackages, please make sure you spelled it right, and that it's an actual package!",
                self.id.bright_green()
            );
        }

        let version = match self.version {
            Option::Some(v) => v,
            // if no version given, use ^latest instead, should've specified a version idiot
            Option::None => semver::VersionReq::parse(&format!(
                "^{}",
                versions.unwrap().first().unwrap().version
            ))
            .unwrap(),
        };

        let additional_data = match &self.additional_data {
            Option::Some(d) => Some(serde_json::from_str(d)?),
            Option::None => None,
        };

        put_dependency(&self.id, version, additional_data, self.sort)
    }
}

fn put_dependency(
    id: &str,
    version: VersionReq,
    additional_data: Option<PackageDependencyModifier>,
    sort: bool,
) -> Result<()> {
    println!(
        "Adding dependency with id {} and version {}",
        id.dependency_id_color(),
        version.dependency_version_color()
    );

    let mut package = PackageConfig::read(".")?;
    let existing_dep = package.dependencies.iter_mut().find(|d| d.id == id);

    let dep = PackageDependency {
        id: id.to_string(),
        version_range: version,
        additional_data: existing_dep
            .as_ref()
            .map(|d| &d.additional_data)
            .cloned()
            .or(additional_data)
            .unwrap_or_default(),
    };

    match existing_dep {
        // overwrite existing dep
        Some(existing_dep) => {
            println!("Dependency already in qpm.json, updating!");
            *existing_dep = dep
        }
        // add dep
        None => package.dependencies.push(dep),
    }

    if sort {
        package.dependencies.sort_by(|a, b| a.id.cmp(&b.id));
    }

    package.write(".")?;
    Ok(())
}

fn remove_dependency(dependency_args: DependencyOperationRemoveArgs) -> Result<()> {
    let mut package = PackageConfig::read(".")?;
    package.dependencies.retain(|p| p.id != dependency_args.id);

    if dependency_args.sort {
        package.dependencies.sort_by(|a, b| a.id.cmp(&b.id));
    }

    package.write(".")?;
    Ok(())
}

fn download_dependency(dependency_args: DependencyOperationDownloadArgs) -> Result<()> {
    let mut repository = repository::useful_default_new(false)?;
    let version = {
        if (dependency_args.version.is_none()) {
            let versions = repository.get_package_versions(&dependency_args.id);

            if versions.is_err() || versions.as_ref().unwrap().iter().count() == 0 {
                panic!(
                    "Package {} does not seem to exist, please make sure you spelled it right.",
                    dependency_args.id.dependency_id_color()
                );
            }

            // return the latest version
            versions.unwrap().unwrap().first().unwrap().version.clone()
        } else {
            dependency_args.version.unwrap()
        }
    };

    let dep = repository.get_package(&dependency_args.id, &version);

    if dep.is_err() || dep.as_ref().unwrap().is_none() {
        panic!(
            "Package {}:{} does not exist, please make sure you spelled it right.",
            dependency_args.id.dependency_id_color(),
            version.dependency_version_color()
        );
    }

    let dep = dep.unwrap().unwrap();


    if dependency_args.recursive {
        if let Ok(resolved_deps) = SharedPackageConfig::resolve_from_package(dep.config.clone(), &repository) {
            let resolved_deps = resolved_deps.1;

                for dep in resolved_deps {
                    println!(
                        "Pulling {}:{}",
                        dep.config.info.id.dependency_id_color(),
                        dep.config
                            .info
                            .version
                            .to_string()
                            .dependency_version_color()
                    );
                    repository.download_to_cache(&dep.config).with_context(|| {
                        format!(
                            "Requesting {}:{}",
                            dep.config.info.id.dependency_id_color(),
                            dep.config.info.version.version_id_color()
                        )
                    })?;
                    repository.add_to_db_cache(dep.clone(), true)?;
                }

                repository.write_repo()?;
        }
    }

    println!(
        "Pulling {}:{}",
        dep.config.info.id.dependency_id_color(),
        dep.config
            .info
            .version
            .to_string()
            .dependency_version_color()
    );
    repository.download_to_cache(&dep.config).with_context(|| {
        format!(
            "Requesting {}:{}",
            dep.config.info.id.dependency_id_color(),
            dep.config.info.version.version_id_color()
        )
    })?;
    repository.add_to_db_cache(dep.clone(), true)?;

    repository.write_repo()?;

    Ok(())
}
