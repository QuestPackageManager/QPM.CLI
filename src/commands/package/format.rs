use std::fs;

use clap::Args;
use qpm_package::models::{dependency::SharedPackageConfig, package::PackageConfig};
use color_eyre::Result;

use crate::{
    commands::Command,
    models::package::PackageConfigExtensions,
};

#[derive(Args, Debug, Clone)]

pub struct FormatArgs { }

impl Command for FormatArgs {
    fn execute(self) -> color_eyre::Result<()> {
        reserialize_package(false)?;
        Ok(())
    }
}

pub fn reserialize_package(sort: bool) -> Result<()> {
    let mut package = PackageConfig::read(".")?;

    if sort {
        // Sort the dependencies by id
        package.dependencies.sort_by(|a, b| a.id.cmp(&b.id));
    }

    // Write the package back to the file
    package.write(".")?;

    // Check if the qpm.shared.json file exists
    if fs::exists("qpm.shared.json").unwrap_or(false) {
        // Read the shared package
        let mut shared_package = SharedPackageConfig::read(".")?;

        if sort {
            // Sort the dependencies by id
            shared_package.config.dependencies.sort_by(|a, b| a.id.cmp(&b.id));
        }

        // Write the shared package back to the file
        shared_package.write(".")?;
    }
    Ok(())
}
