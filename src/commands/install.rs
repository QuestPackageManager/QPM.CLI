use std::path::PathBuf;

use clap::Args;
use color_eyre::eyre::{ContextCompat, bail};
use qpm_package::models::shared_package::SharedPackageConfig;

use crate::{
    models::package::PackageConfigExtensions,
    repository::{self, local::FileRepository},
};

use super::Command;

#[derive(Args, Debug, Clone)]
pub struct InstallCommand {
    #[clap(long, default_value = "false")]
    offline: bool,

    #[clap(long, default_value = "false")]
    pub no_validate: bool,
}

impl Command for InstallCommand {
    fn execute(self) -> color_eyre::Result<()> {
        println!("Publishing package to local file repository");

        // let package = PackageConfig::read(".")?;
        let repo = repository::useful_default_new(self.offline)?;
        let shared_package = SharedPackageConfig::read(".")?;

        let project_folder = PathBuf::from(".").canonicalize()?;
        let restored_triplet_id = shared_package.restored_triplet;

        let restored_triplet = shared_package
            .config
            .triplets
            .get_triplet_settings(&restored_triplet_id)
            .context("Failed to get triplet")?;

        let binaries = restored_triplet.out_binaries.unwrap_or_default();

        if !self.no_validate {
            println!("Skipping validation of binaries");
        } else {
            for binary in &binaries {
                if !binary.exists() {
                    bail!("Binary file {} does not exist", binary.display());
                }
            }
        }

        let mut file_repo = FileRepository::read()?;
        FileRepository::copy_to_cache(
            &shared_package.config,
            &restored_triplet_id,
            project_folder,
            binaries,
            false,
        )?;

        file_repo.add_artifact_and_cache(shared_package.config, true)?;

        file_repo.write()?;
        Ok(())
    }
}
