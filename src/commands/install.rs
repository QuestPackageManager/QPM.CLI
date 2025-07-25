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

        let package = PackageConfig::read(".")?;
        let repo = repository::useful_default_new(self.offline)?;
        let shared_package = SharedPackageConfig::read(".")?;

        let mut file_repo = FileRepository::read()?;
        file_repo.add_artifact_and_cache(
            shared_package,
            PathBuf::from(".").canonicalize()?,
            self.binaries.unwrap_or_default(),
            true,
            true,
        )?;
        file_repo.write()?;
        Ok(())
    }
}
