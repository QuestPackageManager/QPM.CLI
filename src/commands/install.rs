use std::{
    fs::File,
    io::{BufReader, Cursor},
    path::PathBuf,
};

use bytes::{BufMut, BytesMut};
use clap::Args;
use color_eyre::eyre::{Context, ContextCompat, bail};
use qpm_package::models::shared_package::SharedPackageConfig;

use crate::{
    models::package::PackageConfigExtensions, network::agent::download_file_report,
    repository::local::FileRepository, terminal::colors::QPMColor,
};

use super::Command;

#[derive(Args, Debug, Clone)]
pub struct InstallCommand {
    #[clap(long, default_value = "false")]
    offline: bool,

    /// Path to the qpkg file to install
    #[clap(long = "path")]
    qpkg_path: Option<PathBuf>,

    /// URL of the qpkg to install
    #[clap(long = "url")]
    qpkg_url: Option<String>,

    #[clap(long, default_value = "false")]
    pub no_validate: bool,

    /// Whether to install the qpkg as a legacy package
    #[clap(long, default_value = "false")]
    pub legacy: bool,
}

impl Command for InstallCommand {
    fn execute(self) -> color_eyre::Result<()> {
        if self.legacy {
            return self.local_install();
        }
        self.qpkg_install()?;
        Ok(())
    }
}

impl InstallCommand {
    fn qpkg_install(&self) -> Result<(), color_eyre::eyre::Error> {
        let package = if let Some(qpkg_path) = &self.qpkg_path {
            println!("Installing qpkg from path: {}", qpkg_path.display());

            let qpkg_file = File::open(qpkg_path).context("Failed to open qpkg file")?;
            let qpkg_file = BufReader::new(qpkg_file);

            FileRepository::install_qpkg(qpkg_file, true).context("Installing qpkg zip failed")?
        } else if let Some(qpkg_url) = &self.qpkg_url {
            println!("Installing qpkg from URL: {qpkg_url}");

            let mut bytes = BytesMut::new().writer();
            download_file_report(qpkg_url, &mut bytes, |_, _| {})
                .context("Downloading qpkg file failed")?;

            let cursor = Cursor::new(bytes.get_ref());

            FileRepository::install_qpkg(cursor, true).context("Installing qpkg zip failed")?
        } else {
            bail!("Either --path or --url must be provided to install a qpkg");
        };

        println!(
            "Successfully installed qpkg: {}:{}",
            package.id.dependency_id_color(),
            package.version.version_id_color()
        );

        Ok(())
    }

    fn local_install(&self) -> Result<(), color_eyre::eyre::Error> {
        println!("Publishing package to local file repository");
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
