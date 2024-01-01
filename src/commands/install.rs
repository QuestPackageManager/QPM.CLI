use std::path::PathBuf;

use clap::Args;

use qpm_package::models::{dependency::SharedPackageConfig, package::PackageConfig};

use crate::{
    models::package::{PackageConfigExtensions, SharedPackageConfigExtensions},
    repository::{local::FileRepository, multi::MultiDependencyRepository},
};

use super::Command;

#[derive(Args, Debug, Clone)]
pub struct InstallCommand {
    pub binary_path: Option<PathBuf>,
    pub static_path: Option<PathBuf>,

    #[clap(default_value = "false", long, short)]
    pub locked: bool, // pub additional_folders: Vec<String> // todo

    #[clap(long, default_value = "false")]
    offline: bool,

    #[clap(long, default_value = "false")]
    pub update: bool, // pub additional_folders: Vec<String> // todo
}

impl Command for InstallCommand {
    fn execute(self) -> color_eyre::Result<()> {
        println!("Publishing package to local file repository");

        let package = PackageConfig::read(".")?;
        let repo = MultiDependencyRepository::useful_default_new(self.offline)?;
        let shared_package = match !self.update {
            true => SharedPackageConfig::read(".")?,
            false => SharedPackageConfig::resolve_from_package(package, &repo)?.0,
        };

        if self.update {
            println!("Not using lock file, updating dependencies and writing!");
            shared_package.write(".")?;
        } else {
            println!("Using lock file");
        }

        let binary_pathbuf = self.binary_path.or_else(|| {
            shared_package
                .config
                .info
                .additional_data
                .dynamic_lib_out
                .clone()
        });
        let static_pathbuf = self.static_path.or_else(|| {
            shared_package
                .config
                .info
                .additional_data
                .static_lib_out
                .clone()
        });

        let header_only = shared_package
            .config
            .info
            .additional_data
            .headers_only
            .unwrap_or(false);
        #[cfg(debug_assertions)]
        println!("Header only: {header_only}");

        if let Some(p) = &binary_pathbuf {
            if !p.exists() {
                println!("Could not find binary {p:?}, skipping")
            }
        }

        let mut file_repo = FileRepository::read()?;

        let binary_path = binary_pathbuf.as_deref();
        let static_binary_path = static_pathbuf.as_deref();

        file_repo.add_artifact_and_cache(
            shared_package,
            &PathBuf::from(".").canonicalize()?,
            binary_path,
            static_binary_path,
            true,
            true,
        )?;
        file_repo.write()?;
        Ok(())
    }
}
