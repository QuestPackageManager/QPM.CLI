//! QPKG command: Creates a distributable package containing headers and binaries.
//!
//! Process:
//! 1. Build mod (unless --no-build is set)
//! 2. Create ZIP archive containing:
//!    - Headers from sharedDirectory
//!    - Binaries from workspace.outBinaries
//!    - Manifest (qpm2.qpkg.json) with package metadata
//! 3. Output as {package-id}.qpkg ready for distribution
//!
//! The QPKG is used by other projects as a dependency via `qpm restore`.

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use clap::Args;
use color_eyre::eyre::Context;
use qpm_package::models::package::PackageConfig;

use crate::{
    commands::build::BuildCommand,
    models::{package::PackageConfigExtensions, qpkg_file::QpkgFile},
    repository::local::FileRepository,
    terminal::colors::QPMColor,
};

use super::Command;

/// Creates a QPKG package: builds mod, zips headers + binaries + manifest
#[derive(Args, Clone, Debug)]
pub struct QPkgCommand {
    /// Directory storing the built binaries, as {binary_name}
    #[clap(short, long = "input-bins")]
    pub input_bin_dir: Option<String>,

    /// Skip building before creating the QPKG file
    #[clap(short = 'n', long = "no-build", default_value = "false")]
    pub no_build: bool,

    /// Whether to create a qmod file when building the QPKG. Requires building (i.e. `--no-build` not set)
    #[clap(long, default_value = "false")]
    pub qmod: bool,

    /// Offline mode repository access
    #[clap(long, default_value = "false")]
    pub offline: bool,

    /// Verbose output
    #[clap(short, long, default_value = "false")]
    pub verbose: bool,

    /// Where to output the QPKG file
    pub qpkg_output: Option<PathBuf>,

    /// Whether to resolve NDK
    #[clap(long, default_value = "false")]
    pub resolve_ndk: bool,
}

impl Command for QPkgCommand {
    /// Creates a QPKG ZIP archive: build (optional) -> collect headers/binaries -> zip -> finalize
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;

        let build_dir = self
            .input_bin_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| FileRepository::build_path(&package.dependencies_directory));

        if !self.no_build {
            let command = BuildCommand {
                args: None,
                offline: self.offline,
                out_dir: Some(build_dir.clone()),
                qmod: self.qmod,
                build_script: None,
                ndk_resolve: self.resolve_ndk,
            };

            command.execute().context("Failed to build qpkg")?;
        }

        let out = self
            .qpkg_output
            .as_deref()
            .unwrap_or(Path::new(&package.id.0))
            .with_extension("qpkg");

        if self.verbose {
            println!("Creating QPKG from {} and {}", package.shared_directory.display(), build_dir.display());
        }

        let tmp = package.dependencies_directory.join("tmp");

        fs::create_dir_all(&tmp).with_context(|| {
            format!(
                "Failed to create temporary directory: {}",
                tmp.display().file_path_color()
            )
        })?;

        let tmp_out = tmp.join(&out);

        let file = std::fs::File::create(&tmp_out).context("Failed to create temporary QPKG file")?;
        let buffer = BufWriter::new(file);

        // Create QPKG using QpkgFile from header and binary directories
        QpkgFile::create_from_paths(buffer, package.clone(), &package.shared_directory, &build_dir)
            .context("Failed to create QPKG")?;

        // Move the temporary file to the final output location
        std::fs::rename(tmp_out, &out).with_context(|| {
            format!(
                "Failed to move temporary QPKG file to final output: {}",
                out.display().file_path_color()
            )
        })?;

        println!(
            "QPKG file created successfully at {}",
            out.display().file_path_color()
        );

        Ok(())
    }
}
