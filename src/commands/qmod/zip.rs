use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::{PathBuf};

use clap::Args;
use itertools::Itertools;

use qpm_qmod::models::mod_json::ModJson;

use crate::commands::qmod::manifest::{generate_qmod_manifest, ManifestQmodOperationArgs};
use crate::models::mod_json::ModJsonExtensions;
use crate::models::package::PackageConfigExtensions;
use crate::terminal::colors::QPMColor;

use qpm_package::models::dependency::SharedPackageConfig;

use qpm_package::models::package::PackageConfig;

use color_eyre::eyre::ensure;

use color_eyre::Result;

#[derive(Args, Debug, Clone)]
pub struct ZipQmodOperationArgs {
    ///
    /// Tells QPM to exclude mods from being listed as copied mod or libs dependencies
    ///
    #[clap(long = "exclude_libs")]
    pub exclude_libs: Option<Vec<String>>,

    ///
    /// Tells QPM to include mods from being listed as copied mod or libs dependencies
    /// Does not work with `exclude_libs` combined
    ///
    #[clap(long = "include_libs")]
    pub include_libs: Option<Vec<String>>,

    ///
    /// Adds directories for qpm to look for files. Not recursive
    ///
    ///
    #[clap(long = "includes")]
    pub include_dirs: Option<Vec<PathBuf>>,

    ///
    /// Forcefully includes a file in the zip
    ///
    #[clap(long = "includes")]
    pub include_files: Option<Vec<PathBuf>>,

    #[clap(long, default_value = "false")]
    pub(crate) offline: bool,

    #[clap()]
    pub(crate) out_target: Option<PathBuf>,
}

pub(crate) fn execute_qmod_zip_operation(build_parameters: ZipQmodOperationArgs) -> Result<()> {
    ensure!(std::path::Path::new("mod.template.json").exists(),
        "No mod.template.json found in the current directory, set it up please :) Hint: use \"qmod create\"");

    println!("Generating mod.json file from template using qpm.shared.json...");
    let package = PackageConfig::read(".")?;
    let shared_package = SharedPackageConfig::read(".")?;

    let new_manifest = generate_qmod_manifest(
        &package,
        shared_package,
        ManifestQmodOperationArgs {
            exclude_libs: build_parameters.exclude_libs.clone(),
            include_libs: build_parameters.include_libs.clone(),
            offline: build_parameters.offline,
        },
    )?;

    let include_dirs = build_parameters
        .include_dirs
        .unwrap_or(package.workspace.qmod_include_dirs);

    let include_files = build_parameters
        .include_files
        .unwrap_or(package.workspace.qmod_include_files);

    let qmod_out = build_parameters
        .out_target
        .or(package.workspace.qmod_output)
        .expect("No qmod output provided");

    let look_for_files = |s: &str| {
        include_dirs
            .iter()
            .find(|path| path.join(s).exists())
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "No file found for {s} in directories {}",
                    include_dirs.iter().map(|s| s.display()).join(";")
                )
            })
    };

    let file_copies_list = new_manifest
        .file_copies
        .iter()
        .map(|c| look_for_files(&c.name));
    let late_mod_list = new_manifest
        .late_mod_files
        .iter()
        .map(|c| look_for_files(c));
    let early_mod_list = new_manifest.mod_files.iter().map(|c| look_for_files(c));
    let lib_list = new_manifest.library_files.iter().map(|c| look_for_files(c));

    let extra_files = include_files.iter().cloned();

    let combined_files = file_copies_list
        .chain(late_mod_list)
        .chain(early_mod_list)
        .chain(lib_list)
        .chain(extra_files)
        .unique()
        .collect_vec();

    let out_target_qmod = qmod_out.with_extension("qmod");

    println!(
        "Writing qmod zip {}",
        out_target_qmod.to_string_lossy().file_path_color()
    );
    println!(
        "Using files: {}",
        combined_files
            .iter()
            .map(|s| format!("\t{}", s.to_string_lossy().file_path_color()))
            .join("\n")
    );
    let mut zip_file = File::create(out_target_qmod)?;

    let mut zip = zip::ZipWriter::new(&mut zip_file);

    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for file in combined_files {
        println!("Adding file {}", file.to_string_lossy().file_path_color());
        // 50kb
        let contents = String::with_capacity(1024 * 50);
        read_to_string(&file)?;

        zip.start_file(file.file_name().unwrap().to_string_lossy(), options)?;
        zip.write_all(contents.as_bytes())?;
    }

    zip.start_file(ModJson::get_result_name(), options)?;
    serde_json::to_writer_pretty(&mut zip, &new_manifest)?;
    // Apply the changes you've made.
    // Dropping the `ZipWriter` will have the same effect, but may silently fail
    zip.finish()?;

    Ok(())
}
