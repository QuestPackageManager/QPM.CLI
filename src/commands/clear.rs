use std::{fs, path::Path};

use clap::Args;
use color_eyre::Result;
use owo_colors::OwoColorize;
use qpm_package::models::package::PackageConfig;
use walkdir::WalkDir;

use crate::models::package::PackageConfigExtensions;

use super::Command;

#[derive(Args)]
pub struct ClearCommand {}

impl Command for ClearCommand {
    fn execute(self) -> color_eyre::Result<()> {
        remove_dependencies_dir()?;
        remove("qpm.shared.json")?;
        remove("extern.cmake")?;
        remove("qpm_defines.cmake")?;
        remove("mod.json")?;
        Ok(())
    }
}

fn remove(p: &str) -> Result<()> {
    if !Path::new(p).exists() {
        return Ok(());
    }

    fs::remove_file(p)?;
    Ok(())
}

fn remove_dependencies_dir() -> Result<()> {
    let package = PackageConfig::read(".")?;
    let extern_path = Path::new(&package.dependencies_dir);

    if !extern_path.exists() {
        return Ok(());
    }

    let current_path = Path::new(".");

    let extern_path_canonical = extern_path.canonicalize()?;

    // If extern is "" or ".." etc. or is a path that is an
    // ancestor of the current directory, fail fast
    if current_path.canonicalize()? == extern_path_canonical
        || current_path
            .ancestors()
            .any(|path| path.exists() && path.canonicalize().unwrap() == extern_path_canonical)
    {
        panic!(
            "Current path {:?} would be deleted since extern path {:?} is an ancestor or empty",
            current_path.canonicalize().bright_yellow(),
            extern_path.bright_red()
        );
    }

    for entry in WalkDir::new(extern_path_canonical).min_depth(1) {
        let path = entry?.into_path();
        #[cfg(debug_assertions)]
        println!("Path: {}", path.display().bright_yellow());
        if path.is_symlink() {
            if path.is_dir() {
                #[cfg(debug_assertions)]
                println!("Was symlink dir!");
                if let Err(e) = symlink::remove_symlink_dir(&path) {
                    println!(
                        "Failed to remove symlink for directory {}: {}",
                        path.display().bright_yellow(),
                        e
                    );
                }
            } else if path.is_file() {
                #[cfg(debug_assertions)]
                println!("Was symlink file!");
                if let Err(e) = symlink::remove_symlink_file(&path) {
                    println!(
                        "Failed to remove symlink for file {}: {}",
                        path.display().bright_yellow(),
                        e
                    );
                }
            } else {
                #[cfg(debug_assertions)]
                println!("Was broken symlink!");
                if let Err(ed) = std::fs::remove_dir(&path)
                    && let Err(ef) = std::fs::remove_file(&path)
                {
                    println!(
                        "Failed to remove broken symlink for {}:\nAttempt 1 (dir):{}\nAttempt 2 (file):{}",
                        path.display().bright_yellow(),
                        ed,
                        ef
                    );
                }
            }
        }
    }

    fs::remove_dir_all(&package.dependencies_dir)?;
    Ok(())
}
