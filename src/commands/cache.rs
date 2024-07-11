use std::{
    fs::{self, File},
    io::{Read, Write},
};

use clap::Subcommand;
use color_eyre::{eyre::Context, Result};
use owo_colors::OwoColorize;
use qpm_package::models::package::PackageConfig;
use semver::Version;
use walkdir::WalkDir;

use crate::{
    models::{config::get_combine_config, package::PackageConfigExtensions},
    repository::local::FileRepository,
    terminal::colors::QPMColor,
};

use super::Command;

#[derive(clap::Args, Debug, Clone)]

pub struct CacheCommand {
    /// Clear the cache
    #[clap(subcommand)]
    pub op: CacheOperation,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CacheOperation {
    /// Clear the cache
    Clear(ClearCommand),
    /// Lists versions for each cached package
    List,
    /// Shows you the current cache path
    Path,
    /// Fixes some dependencies that use technically wrong include paths
    LegacyFix,
}

#[derive(clap::Args, Debug, Clone)]

pub struct ClearCommand {
    pub package: Option<String>,
    pub version: Option<String>,
}

impl Command for CacheCommand {
    fn execute(self) -> color_eyre::Result<()> {
        match self.op {
            CacheOperation::Clear(c) => clear(c)?,
            CacheOperation::List => list(),
            CacheOperation::Path => path(),
            CacheOperation::LegacyFix => legacy_fix()?,
        };
        Ok(())
    }
}

fn clear(clear_params: ClearCommand) -> Result<()> {
    match (clear_params.package, clear_params.version) {
        (Some(package), None) => {
            let mut file_repo = FileRepository::read()?;
            file_repo.remove_package_versions(&package)?;
            println!(
                "Sucessfully removed all versions of {}",
                package.dependency_id_color()
            );
            file_repo.write()?;
            Ok(())
        }
        (Some(package), Some(version_str)) => {
            let mut file_repo = FileRepository::read()?;
            let version = Version::parse(&version_str).context("version parse")?;
            file_repo.remove_package(&package, &version)?;
            println!(
                "Sucessfully removed {}/{}",
                package.dependency_id_color(),
                version.version_id_color()
            );
            file_repo.write()?;
            Ok(())
        }
        // clear all
        _ => {
            let config = get_combine_config();
            let path = config.cache.as_ref().unwrap();
            fs::remove_dir_all(path)?;
            FileRepository::clear()?;
            Ok(())
        }
    }
}

fn path() {
    let config = get_combine_config();
    println!(
        "Config path is: {}",
        config.cache.as_ref().unwrap().display().bright_yellow()
    );
}

fn list() {
    let config = get_combine_config();
    let path = config.cache.as_ref().unwrap();

    for dir in WalkDir::new(path).max_depth(2).min_depth(1) {
        let unwrapped = dir.unwrap();
        if unwrapped.depth() == 1 {
            println!(
                "package {}:",
                unwrapped.file_name().to_string_lossy().bright_red()
            );
        } else {
            println!(
                " - {}",
                unwrapped.file_name().to_string_lossy().bright_green()
            );
        }
    }
}

fn legacy_fix() -> Result<()> {
    for entry in WalkDir::new(get_combine_config().cache.as_ref().unwrap())
        .min_depth(2)
        .max_depth(2)
    {
        let path = entry.unwrap().into_path().join("src");
        println!("{}", path.display());
        let qpm_path = &path;
        if !qpm_path.exists() {
            continue;
        }
        let shared_path = path.join(PackageConfig::read(qpm_path)?.shared_dir);

        for entry in WalkDir::new(shared_path) {
            let entry_path = entry.unwrap().into_path();
            if entry_path.is_file() {
                let mut file = match File::open(&entry_path) {
                    Ok(o) => o,
                    Err(e) => panic!(
                        "Opening file {} to read failed: {}",
                        entry_path.display().bright_yellow(),
                        e
                    ),
                };

                let mut buf: String = "".to_string();
                match file.read_to_string(&mut buf) {
                    Ok(_) => {}
                    Err(e) => {
                        #[cfg(debug_assertions)]
                        println!(
                            "reading file {} to string failed: {}",
                            entry_path.display().bright_yellow(),
                            e
                        );
                        continue;
                    }
                };
                fs_extra::file::remove(&entry_path)?;
                let mut file = std::fs::File::create(&entry_path)?;
                file.write_all(
                    buf.replace(
                        "#include \"extern/beatsaber-hook/",
                        "#include \"beatsaber-hook/",
                    )
                    .as_bytes(),
                )?;
            }
        }
    }

    Ok(())
}
