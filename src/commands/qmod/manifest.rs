use std::path::PathBuf;

use clap::Args;
use qpm_package::extensions::package_metadata::PackageMetadataExtensions;
use semver::VersionReq;

use qpm_qmod::models::mod_json::ModJson;

use crate::models::mod_json::{ModJsonExtensions, PreProcessingData};
use crate::models::package::{PackageConfigExtensions, SharedPackageConfigExtensions};

use qpm_package::models::dependency::SharedPackageConfig;

use qpm_package::models::package::PackageConfig;

use color_eyre::eyre::ensure;

use color_eyre::Result;

#[derive(Args, Debug, Clone)]
pub struct BuildQmodOperationArgs {
    #[clap(long = "isLibrary")]
    pub is_library: Option<bool>,

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
pub(crate) fn execute_qmod_build_operation(build_parameters: BuildQmodOperationArgs) -> Result<()> {
    ensure!(std::path::Path::new("mod.template.json").exists(),
        "No mod.template.json found in the current directory, set it up please :) Hint: use \"qmod create\"");

    println!("Generating mod.json file from template using qpm.shared.json...");
    let package = PackageConfig::read(".")?;
    let shared_package = SharedPackageConfig::read(".")?;

    let is_header_only = shared_package
        .config
        .info
        .additional_data
        .headers_only
        .unwrap_or(false);
    let binary = (!is_header_only).then(|| {
        shared_package
            .config
            .info
            .get_so_name()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    });

    // Parse template mod.template.json
    let preprocess_data = PreProcessingData {
        version: shared_package.config.info.version.to_string(),
        mod_id: shared_package.config.info.id.clone(),
        mod_name: shared_package.config.info.name.clone(),
        binary,
    };

    let mut existing_json = ModJson::read_and_preprocess(preprocess_data)?;
    existing_json.is_library = build_parameters.is_library.or(existing_json.is_library);

    let template_mod_json: ModJson = shared_package.to_mod_json();
    let legacy_0_1_0 = package.matches_version(&VersionReq::parse("^0.1.0")?);

    existing_json = ModJson::merge_modjson(existing_json, template_mod_json, legacy_0_1_0);

    if let Some(excluded) = build_parameters.exclude_libs {
        let exclude_filter = |lib_name: &String| -> bool {
            // returning false means don't include
            // don't include anything that is excluded
            !excluded.iter().any(|s| lib_name == s)
        };

        existing_json.mod_files.retain(exclude_filter);
        existing_json.library_files.retain(exclude_filter);
        // whitelist libraries
    } else if let Some(included) = build_parameters.include_libs {
        let include_filter = |lib_name: &String| -> bool {
            // returning false means don't include
            // only include anything that is specified included
            included.iter().any(|s| lib_name == s)
        };

        existing_json.mod_files.retain(include_filter);
        existing_json.library_files.retain(include_filter);
    }

    // Write mod.json
    existing_json.write(&PathBuf::from(ModJson::get_result_name()))?;
    Ok(())
}
