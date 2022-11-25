use std::{
    fs::{self, File},
    io::{Read, Write},
};

use clap::Subcommand;
use color_eyre::Result;
use owo_colors::OwoColorize;
use qpm_package::models::package::PackageConfig;
use walkdir::WalkDir;

use crate::{
    models::{config::get_combine_config, package::PackageConfigExtensions},
    repository::local::FileRepository,
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
    Clear,
    /// Lists versions for each cached package
    List,
    /// Shows you the current cache path
    Path,
    /// Fixes some dependencies that use technically wrong include paths
    LegacyFix,
}

impl Command for CacheCommand {
    fn execute(self) -> color_eyre::Result<()> {
        match self.op {
            CacheOperation::Clear => clear()?,
            CacheOperation::List => list(),
            CacheOperation::Path => path(),
            CacheOperation::LegacyFix => legacy_fix()?,
        };
        Ok(())
    }
}

fn clear() -> Result<()> {
    let config = get_combine_config();
    let path = config.cache.as_ref().unwrap();
    fs::remove_dir_all(path)?;
    FileRepository::clear()?;
    Ok(())
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
        let qpm_path = path.join("qpm.json");
        if !qpm_path.exists() {
            continue;
        }
        let shared_path = path.join(PackageConfig::read(&qpm_path)?.shared_dir);

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
                    Err(_e) => {
                        #[cfg(debug_assertions)]
                        println!(
                            "reading file {} to string failed: {}",
                            entry_path.display().bright_yellow(),
                            _e
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
