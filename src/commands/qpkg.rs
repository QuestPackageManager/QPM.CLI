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

use std::{
    collections::HashMap,
    fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use clap::Args;
use color_eyre::eyre::Context;
use qpm_package::models::{
    package::PackageConfig,
    qpkg::{QPKG_JSON, QPkg},
};
use zip::{ZipWriter, write::FileOptions};

use crate::{
    commands::build::BuildCommand, models::package::PackageConfigExtensions,
    repository::local::FileRepository, terminal::colors::QPMColor,
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

        let tmp = package.dependencies_directory.join("tmp");

        fs::create_dir_all(&tmp).with_context(|| {
            format!(
                "Failed to create temporary directory: {}",
                tmp.display().file_path_color()
            )
        })?;

        let tmp_out = tmp.join(&out);

        let file = std::fs::File::create(&tmp_out)?;
        let buf_file = BufWriter::new(file);
        let mut zip = ZipWriter::new(buf_file);

        let options = FileOptions::<()>::default();

        // Add headers from sharedDirectory to ZIP archive (public C++ API for dependents)
        zip.add_directory_from_path(&package.shared_directory, options)
            .context("Failed to add shared directory to QPKG zip")?;

        for entry in walkdir::WalkDir::new(&package.shared_directory)
            .min_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            // remove the project prefix from the path
            let rel_path = entry
                .path()
                .strip_prefix(package.shared_directory.parent().unwrap_or(Path::new("")))
                .unwrap();
            if self.verbose {
                println!(
                    "Adding shared file: {}",
                    rel_path.display().file_path_color()
                );
            }

            zip.start_file_from_path(rel_path, options)
                .with_context(|| {
                    format!(
                        "Failed to add file {} to QPKG zip",
                        rel_path.display().file_path_color()
                    )
                })?;

            let bytes = std::fs::read(entry.path()).context("Failed to read shared file")?;
            zip.write_all(&bytes)
                .context("Failed to write shared file to QPKG zip")?;
        }

        let binaries = package.workspace.out_binaries.clone().unwrap_or_default();

        // src -> dst zip
        let binaries_map: HashMap<PathBuf, PathBuf> = binaries
            .iter()
            .map(|binary| {
                // extern/build/{binary}
                let binary_name = binary.file_name().unwrap_or_default();
                let binary_path = build_dir.join(binary_name);

                let zip_path = Path::new("bin").join(binary_name);

                if !binary_path.exists() {
                    panic!(
                        "Binary {} does not exist (looking in {}). `qpm2 build` must be run first.",
                        binary.display().file_path_color(),
                        binary_path.display().file_path_color()
                    );
                }
                (binary_path, zip_path)
            })
            .collect();

        for (src, dst) in &binaries_map {
            zip.start_file_from_path(dst, options)
                .expect("Failed to start file in QPKG zip");

            let bytes = std::fs::read(src).expect("Failed to read binary file");
            zip.write_all(&bytes)
                .expect("Failed to write binary file to QPKG zip");
        }

        let files = binaries_map.into_values().collect();

        let qpkg = QPkg {
            shared_dir: package.shared_directory.clone(),
            config: package,
            files,
        };

        zip.start_file(QPKG_JSON, options)
            .context("Failed to start file in QPKG zip")?;
        serde_json::to_writer_pretty(&mut zip, &qpkg)
            .context("Failed to write QPKG JSON to zip")?;
        zip.finish()?;

        // move the temporary file to the final output location
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
