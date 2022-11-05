use clap::{Args, Subcommand};
use color_eyre::{eyre::{Context, bail}, Result};
use owo_colors::OwoColorize;
use qpm_package::models::{extra::{PackageDependencyModifier}, package::{PackageConfig, PackageDependency}};
use semver::VersionReq;

use crate::{
    models::package::PackageConfigExtensions,
    repository::{multi::MultiDependencyRepository, Repository},
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
}

#[derive(Args, Debug, Clone)]
pub struct DependencyOperationAddArgs {
    /// Id of the dependency as listed on qpackages
    pub id: String,

    /// optional version of the dependency that you want to add
    #[clap(short, long)]
    pub version: Option<VersionReq>,

    /// Additional data for the dependency (as a valid json object)
    #[clap(long)]
    pub additional_data: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct DependencyOperationRemoveArgs {
    /// Id of the dependency as listed on qpackages
    pub id: String,
}

impl Command for DependencyCommand {
    fn execute(self) -> color_eyre::Result<()> {
        match self.op {
            DependencyOperation::Add(a) => add_dependency(a),
            DependencyOperation::Remove(r) => remove_dependency(r),
        }
    }
}

fn add_dependency(dependency_args: DependencyOperationAddArgs) -> Result<()> {
    if dependency_args.id == "yourmom" {
        bail!("The dependency was too big to add, we can't add this one!");
    }

    let repo = MultiDependencyRepository::useful_default_new()?;

    let versions = repo
        .get_package_versions(&dependency_args.id)
        .context("No version found for dependency")?;

    if versions.is_none() || versions.clone().unwrap().is_empty() {
        bail!(
            "Package {} does not seem to exist qpackages, please make sure you spelled it right, and that it's an actual package!",
            dependency_args.id.bright_green()
        );
    }

    let version = match dependency_args.version {
        Option::Some(v) => v,
        // if no version given, use ^latest instead, should've specified a version idiot
        Option::None => {
            semver::VersionReq::parse(&format!("^{}", versions.unwrap().first().unwrap().version)).unwrap()
        }
    };

    let additional_data = match &dependency_args.additional_data {
        Option::Some(d) => serde_json::from_str(d)?,
        Option::None => Default::default(),
    };

    put_dependency(&dependency_args.id, version, &additional_data)
}

fn put_dependency(
    id: &str,
    version: VersionReq,
    additional_data: &PackageDependencyModifier,
) -> Result<()> {
    println!(
        "Adding dependency with id {} and version {}",
        id.bright_red(),
        version.bright_blue()
    );

    let mut package = PackageConfig::read(".")?;
    let dep = PackageDependency {
        id: id.to_string(),
        version_range: version,
        additional_data: additional_data.clone(),
    };
    package.dependencies.push(dep);
    package.write(".")?;
    Ok(())
}

fn remove_dependency(dependency_args: DependencyOperationRemoveArgs) -> Result<()> {
    let mut package = PackageConfig::read(".")?;
    package.dependencies.retain(|p| p.id != dependency_args.id);
    package.write(".")?;
    Ok(())
}
