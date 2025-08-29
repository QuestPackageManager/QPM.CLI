use std::path::{Path, PathBuf};

use clap::Args;
use qpm_package::models::shared_package::SharedPackageConfig;
use qpm_package::models::triplet::{QPM_ENV_GAME_ID, QPM_ENV_GAME_VERSION};

use qpm_qmod::models::mod_json::ModJson;

use crate::models::mod_json::{ModJsonExtensions, PreProcessingData};
use crate::models::package::{PackageConfigExtensions, SharedPackageConfigExtensions};
use crate::repository;

use qpm_package::models::package::PackageConfig;

use color_eyre::eyre::{ContextCompat, ensure};

use color_eyre::Result;

#[derive(Args, Debug, Clone)]
pub struct ManifestQmodOperationArgs {
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

    #[clap(long, default_value = "false")]
    pub(crate) offline: bool,
}

// This will parse the `qmod.template.json` and process it, then finally export a `qmod.json` for packaging and deploying.
pub(crate) fn execute_qmod_manifest_operation(
    build_parameters: ManifestQmodOperationArgs,
) -> Result<()> {
    let package = PackageConfig::read(".")?;
    let shared_package = SharedPackageConfig::read(".")?;

    let new_json = generate_qmod_manifest(&package, shared_package, build_parameters)?;
    // Write mod.json
    new_json.write(&PathBuf::from(ModJson::get_result_name()))?;
    Ok(())
}

pub(crate) fn generate_qmod_manifest(
    package: &PackageConfig,
    shared_package: SharedPackageConfig,
    build_parameters: ManifestQmodOperationArgs,
) -> Result<ModJson> {
    let shared_triplet = shared_package.get_restored_triplet();
    let triplet = package
        .triplets
        .get_merged_triplet(&shared_package.restored_triplet)
        .context("Restored triplet not in package config")?;

    let mod_template = triplet
        .qmod_template
        .as_deref()
        .unwrap_or_else(|| Path::new("mod.template.json"));

    ensure!(
        mod_template.exists(),
        "QMod template file {} does not exist. Hint: use \"qmod create\"",
        mod_template.display()
    );

    println!(
        "Generating mod.json file from template {} using qpm.shared.json...",
        mod_template.display()
    );

    let env = &shared_triplet.env;

    let game_version = env.get(QPM_ENV_GAME_VERSION);
    let game_id = env.get(QPM_ENV_GAME_ID);

    let binaries = triplet
        .out_binaries
        .iter()
        .flatten()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    let mod_id = triplet
        .qmod_id
        .as_ref()
        .unwrap_or(&shared_package.config.id.0);

    let preprocess_data = PreProcessingData {
        version: shared_package.config.version.to_string(),
        mod_id: mod_id.clone(),

        game_id: game_id.cloned(),
        game_version: game_version.cloned(),

        binaries,

        additional_env: env.clone(),
    };
    let mut existing_json = ModJson::read_and_preprocess(preprocess_data, mod_template)?;

    let repo = repository::useful_default_new(build_parameters.offline)?;
    let template_mod_json: ModJson = shared_package.to_mod_json(&repo);

    // Merge the existing json with the template mod json
    existing_json = ModJson::merge_modjson(existing_json, template_mod_json);

    if let Some(excluded) = build_parameters.exclude_libs {
        let exclude_filter = |lib_name: &String| -> bool {
            // returning false means don't include
            // don't include anything that is excluded
            !excluded.iter().any(|s| lib_name == s)
        };

        existing_json.mod_files.retain(exclude_filter);
        existing_json.library_files.retain(exclude_filter);
        existing_json.late_mod_files.retain(exclude_filter);
        existing_json.mod_files.retain(exclude_filter);
        // whitelist libraries
    } else if let Some(included) = build_parameters.include_libs {
        let include_filter = |lib_name: &String| -> bool {
            // returning false means don't include
            // only include anything that is specified included
            included.iter().any(|s| lib_name == s)
        };

        existing_json.mod_files.retain(include_filter);
        existing_json.library_files.retain(include_filter);
        existing_json.mod_files.retain(include_filter);
        existing_json.late_mod_files.retain(include_filter);
    }
    Ok(existing_json)
}
