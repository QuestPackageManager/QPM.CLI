use std::path::PathBuf;

use clap::Args;
use color_eyre::eyre::Context;
use qpm_package::{models::{dependency::SharedPackageConfig, package::PackageConfig}, extensions::package_metadata::PackageMetadataExtensions};

use crate::{
    models::{
        package::{PackageConfigExtensions, SharedPackageConfigExtensions}
    },
    repository::{local::FileRepository, multi::MultiDependencyRepository},
};

use super::Command;

#[derive(Args, Debug, Clone)]
pub struct InstallCommand {
    pub binary_path: Option<PathBuf>,
    pub debug_binary_path: Option<PathBuf>,

    #[clap(long)]
    pub cmake_build: Option<bool>,

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

        let mut binary_path = self.binary_path;
        let mut debug_binary_path = self.debug_binary_path;

        let header_only = shared_package
            .config
            .info
            .additional_data
            .headers_only
            .unwrap_or(false);
        #[cfg(debug_assertions)]
        println!("Header only: {header_only}");

        // TODO: Handle static library
        if !header_only {
            if binary_path.is_none() && self.cmake_build.unwrap_or(true) {
                binary_path = Some(
                    PathBuf::from(format!(
                        "./build/{}",
                        shared_package.config.info.get_so_name().file_name().unwrap().to_string_lossy()
                    ))
                    .canonicalize().context("Failed to retrieve release binary for publishing since it is not header only")?,
                );
            }

            if debug_binary_path.is_none() && self.cmake_build.unwrap_or(true) {
                debug_binary_path = Some(
                    PathBuf::from(format!(
                        "./build/debug/{}",
                        shared_package.config.info.get_so_name().file_name().unwrap().to_string_lossy()
                    ))
                    .canonicalize().context("Failed to retrieve debug binary for publishing since it is not header only")?,
                );
            }
        }

        if let Some(p) = &debug_binary_path {
            if !p.exists() {
                println!("Could not find debug binary {p:?}, skipping")
            }
        }

        if let Some(p) = &binary_path {
            if !p.exists() {
                println!("Could not find binary {p:?}, skipping")
            }
        }

        let mut file_repo = FileRepository::read()?;
        file_repo.add_artifact_and_cache(
            shared_package,
            PathBuf::from(".").canonicalize()?,
            binary_path,
            debug_binary_path,
            true,
            true,
        )?;
        file_repo.write()?;
        Ok(())
    }
}
