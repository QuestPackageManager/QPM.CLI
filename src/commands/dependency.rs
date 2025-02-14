use std::fs;

use clap::{Args, Subcommand};
use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use owo_colors::OwoColorize;
use qpm_package::models::{
    dependency::SharedPackageConfig, extra::PackageDependencyModifier, package::{PackageConfig, PackageDependency}
};
use semver::VersionReq;

use crate::{
    models::package::PackageConfigExtensions,
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
}

#[derive(Args, Debug, Clone)]
pub struct DependencyOperationRemoveArgs {
    /// Id of the dependency as listed on qpackages
    pub id: String,
}

impl Command for DependencyCommand {
    fn execute(self) -> color_eyre::Result<()> {
        match self.op {
            DependencyOperation::Add(a) => a.execute(),
            DependencyOperation::Remove(r) => remove_dependency(r),
            DependencyOperation::Sort => sort_dependencies(),
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

        put_dependency(&self.id, version, additional_data)
    }
}

fn put_dependency(
    id: &str,
    version: VersionReq,
    additional_data: Option<PackageDependencyModifier>,
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

    package.write(".")?;
    Ok(())
}

fn remove_dependency(dependency_args: DependencyOperationRemoveArgs) -> Result<()> {
    let mut package = PackageConfig::read(".")?;
    package.dependencies.retain(|p| p.id != dependency_args.id);
    package.write(".")?;
    Ok(())
}

fn sort_dependencies() -> Result<()> {
    let mut package = PackageConfig::read(".")?;

    // Sort the dependencies by id
    package.dependencies.sort_by(|a, b| a.id.cmp(&b.id));

    // Write the package back to the file
    package.write(".")?;

    // Check if the qpm.shared.json file exists
    if fs::exists("qpm.shared.json").unwrap_or(false) {
        // Read the shared package
        let mut shared_package = SharedPackageConfig::read(".")?;

        // Sort the dependencies by id
        shared_package.config.dependencies.sort_by(|a, b| a.id.cmp(&b.id));

        // Write the shared package back to the file
        shared_package.write(".")?;
    }
    Ok(())
}
