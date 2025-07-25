use std::path::PathBuf;

use clap::Args;
use color_eyre::eyre::Context;
use qpm_package::{
    extensions::package_metadata::PackageMetadataExtensions,
    models::{package::PackageConfig, shared_package::SharedPackageConfig},
};

use crate::{
    models::package::{PackageConfigExtensions, SharedPackageConfigExtensions},
    repository::{self, local::FileRepository},
};

use super::Command;

#[derive(Args, Debug, Clone)]
pub struct InstallCommand {
    #[clap(long, default_value = "false")]
    offline: bool,

    pub binaries: Option<Vec<PathBuf>>,
}

impl Command for InstallCommand {
    fn execute(self) -> color_eyre::Result<()> {
        println!("Publishing package to local file repository");

        // let package = PackageConfig::read(".")?;
        let repo = repository::useful_default_new(self.offline)?;
        let shared_package = SharedPackageConfig::read(".")?;

        let project_folder = PathBuf::from(".").canonicalize()?;

        let mut file_repo = FileRepository::read()?;
        FileRepository::copy_to_cache(
            &shared_package.config,
            &shared_package.restored_triplet,
            project_folder,
            self.binaries.clone().unwrap_or_default(),
            false,
        )?;

        file_repo.add_artifact_and_cache(
            shared_package.config,
            true,
        )?;

        file_repo.write()?;
        Ok(())
    }
}
