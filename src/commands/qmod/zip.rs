use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use clap::Args;
use itertools::Itertools;

use owo_colors::OwoColorize;
use qpm_package::extensions::workspace::WorkspaceConfigExtensions;
use qpm_qmod::models::mod_json::ModJson;

use crate::commands::qmod::manifest::{ManifestQmodOperationArgs, generate_qmod_manifest};
use crate::commands::scripts;
use crate::models::mod_json::ModJsonExtensions;
use crate::models::package::PackageConfigExtensions;
use crate::models::schemas::{SchemaLinks, WithSchema};
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
    #[clap(short = 'i', long = "include_dirs")]
    pub include_dirs: Option<Vec<PathBuf>>,

    ///
    /// Forcefully includes a file in the zip
    ///
    #[clap(short = 'f', long = "include_files")]
    pub include_files: Option<Vec<PathBuf>>,

    #[clap(long, default_value = "false")]
    pub(crate) offline: bool,

    /// Run the clean script before building
    #[clap(long = "clean", default_value = "false")]
    pub(crate) clean: bool,

    /// Don't run the build script
    #[clap(long = "skip_build", default_value = "false")]
    pub(crate) skip_build: bool,

    #[clap()]
    pub(crate) out_target: Option<PathBuf>,
}

fn get_relative_pathbuf(path: PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Canonicalize the given path
    let canonicalized_path = fs::canonicalize(&path)?;

    // Get the current directory
    let current_dir = env::current_dir()?;

    // Canonicalize the current directory
    let current_dir = fs::canonicalize(&current_dir)?;

    // Compute the relative path
    let relative_path = pathdiff::diff_paths(&canonicalized_path, &current_dir)
        .ok_or("Failed to compute relative path")?;

    Ok(relative_path)
}

pub(crate) fn execute_qmod_zip_operation(build_parameters: ZipQmodOperationArgs) -> Result<()> {
    ensure!(
        std::path::Path::new("mod.template.json").exists(),
        "No mod.template.json found in the current directory, set it up please :) Hint: use \"qmod create\""
    );
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

    if build_parameters.clean {
        // Run clean script
        let clean_script = &package.workspace.get_clean();
        if let Some(clean_script) = clean_script
        {
            println!("Running clean script");
            scripts::invoke_script(clean_script, &[], &package)?;
        }
    }

    // Run build script
    let build_script = &package.workspace.get_build();
    if let Some(build_script) = build_script
        && !build_parameters.skip_build
    {
        println!("Running build script");
        scripts::invoke_script(build_script, &[], &package)?;
    }

    let include_dirs = build_parameters
        .include_dirs
        .unwrap_or(package.workspace.qmod_include_dirs);

    let include_files = build_parameters
        .include_files
        .unwrap_or(package.workspace.qmod_include_files);

    let qmod_out = build_parameters
        .out_target
        .or(package.workspace.qmod_output)
        .unwrap_or(format!("./{}", package.info.id).into());

    let look_for_files = |s: &str| {
        include_dirs
            .iter()
            .map(|path| path.join(s))
            .find(|path| path.exists())
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

    let cover_image = new_manifest.cover_image.as_ref().map(PathBuf::from);

    let combined_files = file_copies_list
        .chain(late_mod_list)
        .chain(early_mod_list)
        .chain(lib_list)
        .chain(extra_files)
        .chain(cover_image)
        .map(|p| get_relative_pathbuf(p.to_path_buf()).unwrap())
        .unique()
        .collect_vec();

    let out_target_qmod = qmod_out.with_extension("qmod");

    println!(
        "Writing qmod zip {}",
        out_target_qmod.to_string_lossy().file_path_color()
    );
    println!(
        "Using files: \n{}",
        combined_files
            .iter()
            .map(|s| format!("\t{}", s.to_string_lossy().file_path_color()))
            .join("\n")
    );
    let mut zip_file = File::create(&out_target_qmod)?;

    let mut zip = zip::ZipWriter::new(&mut zip_file);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(9));
    for file in combined_files {
        println!("Adding file {}", file.to_string_lossy().green());

        // 50kb
        let contents = fs::read(&file)?;

        zip.start_file(file.file_name().unwrap().to_string_lossy(), options)?;
        zip.write_all(contents.as_slice())?;
    }

    zip.start_file(ModJson::get_result_name(), options)?;
    serde_json::to_writer_pretty(
        &mut zip,
        &WithSchema {
            schema: SchemaLinks::MOD_CONFIG,
            value: new_manifest,
        },
    )?;
    // Apply the changes you've made.
    // Dropping the `ZipWriter` will have the same effect, but may silently fail
    zip.finish()?;

    println!("Wrote zip file to {}", out_target_qmod.display().blue());

    Ok(())
}
