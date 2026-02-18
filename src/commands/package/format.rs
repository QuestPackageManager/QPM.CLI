use std::fs;

use clap::Args;
use color_eyre::{Result, eyre::ContextCompat};
use qpm_package::models::{
    package::PackageConfig, shared_package::SharedPackageConfig, triplet::base_triplet_id,
};

use crate::{commands::Command, models::package::PackageConfigExtensions};

#[derive(Args, Debug, Clone)]

pub struct FormatArgs {
    /// Triplet to format the package for
    #[clap(long, short)]
    pub triplet: Option<String>,
}

impl Command for FormatArgs {
    fn execute(self) -> color_eyre::Result<()> {
        reserialize_package(false)?;
        Ok(())
    }
}

pub fn reserialize_package(sort: bool) -> Result<()> {
    let mut package = PackageConfig::read(".")?;
    let triplet = package
        .triplets
        .get_triplet_standalone_mut(&base_triplet_id())
        .context("Failed to get triplet settings")?;

    if sort {
        // Sort the dependencies by id
        // triplet.dependencies.sort_by(|a, b| a.id.cmp(&b.id));
    }

    // Write the package back to the file
    package.write(".")?;

    // Check if the qpm.shared.json file exists
    if fs::exists("qpm.shared.json").unwrap_or(false) {
        // Read the shared package
        let shared_package = SharedPackageConfig::read(".")?;

        if sort {
            // Sort the dependencies by id
            // shared_package
            //     .config
            //     .dependencies
            //     .sort_by(|a, b| a.id.cmp(&b.id));
        }

        // Write the shared package back to the file
        shared_package.write(".")?;
    }
    Ok(())
}
