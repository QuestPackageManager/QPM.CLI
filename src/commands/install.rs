use std::{fs::File, io::BufReader, path::PathBuf};

use clap::Args;
use color_eyre::eyre::{Context, bail};
use qpm_package::models::shared_package::SharedPackageConfig;

use crate::{
    models::package::PackageConfigExtensions, repository::local::FileRepository,
    terminal::colors::QPMColor,
};

use super::Command;

#[derive(Args, Debug, Clone)]
pub struct InstallCommand {
    /// Offline mode repository access
    #[clap(long, default_value = "false")]
    offline: bool,

    /// Path to the qpkg file to install
    #[clap(short = 'p', long = "path")]
    qpkg_path: Option<PathBuf>,

    /// URL of the qpkg to install
    #[clap(short = 'u', long = "url")]
    qpkg_url: Option<String>,

    /// Override the version of the qpkg to install
    #[clap(long = "version")]
    pub version_override: Option<String>,

    /// Whether to skip validation of the binaries
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
    fn qpkg_install(self) -> Result<(), color_eyre::eyre::Error> {
        let version = self
            .version_override
            .map(|v| {
                v.parse::<semver::Version>()
                    .context("Failed to parse version override")
            })
            .transpose()?;

        let package = if let Some(qpkg_path) = &self.qpkg_path {
            println!("Installing qpkg from path: {}", qpkg_path.display());

            let qpkg_file = File::open(qpkg_path).context("Failed to open qpkg file")?;
            let qpkg_file = BufReader::new(qpkg_file);

            FileRepository::install_qpkg(qpkg_file, true, version)
                .context("Installing qpkg zip failed")?
        } else if let Some(qpkg_url) = &self.qpkg_url {
            println!("Installing qpkg from URL: {qpkg_url}");

            FileRepository::install_qpkg_from_url(qpkg_url, None, true, version)
                .context("Installing qpkg from URL failed")?
        } else {
            bail!("Either --path or --url must be provided to install a qpkg");
        };

        println!(
            "Successfully installed qpkg: {}:{}",
            package.config.id.dependency_id_color(),
            package.config.version.version_id_color()
        );

        Ok(())
    }

    fn local_install(&self) -> Result<(), color_eyre::eyre::Error> {
        println!("Publishing package to local file repository");
        let shared_package = SharedPackageConfig::read(".")?;
        let project_folder = PathBuf::from(".").canonicalize()?;
        let binaries = shared_package.config.workspace.out_binaries.clone().unwrap_or_default();
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
        FileRepository::copy_to_cache(&shared_package.config, project_folder, binaries, false)?;
        // legacy install copies already-built binaries directly, there's no qpkg archive to checksum
        file_repo.add_artifact_and_cache(shared_package.config, None, true)?;
        file_repo.write()?;
        Ok(())
    }
}
