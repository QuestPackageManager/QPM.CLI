use std::path::PathBuf;

use clap::{Args, Subcommand};
use semver::Version;

mod edit;

use crate::data::{
    mod_json::{ModJson, PreProcessingData},
    package::{PackageConfig, SharedPackageConfig},
};

#[derive(Args, Debug, Clone)]

pub struct Qmod {
    #[clap(subcommand)]
    pub op: QmodOperation,
}

/// Some properties are not editable through the qmod create command, these properties are either editable through the package, or not at all
#[derive(Args, Debug, Clone)]

pub struct CreateQmodJsonOperationArgs {
    /// The schema version this mod was made for, ex. '0.1.1'
    #[clap(long = "qpversion")]
    pub schema_version: Option<Version>,
    /// Author of the mod, ex. 'RedBrumbler'
    #[clap(long)]
    pub author: Option<String>,
    /// Optional slot for if you ported a mod, ex. 'Fern'
    #[clap(long)]
    pub porter: Option<String>,
    /// id of the package the mod is for, ex. 'com.beatgames.beatsaber'
    #[clap(long = "packageID")]
    pub package_id: Option<String>,
    /// Version of the package, ex. '1.1.0'
    #[clap(long = "packageVersion")]
    pub package_version: Option<String>,
    /// description for the mod, ex. 'The best mod to exist ever!'
    #[clap(long)]
    pub description: Option<String>,
    /// optional cover image filename, ex. 'cover.png'
    #[clap(long = "coverImage")]
    pub cover_image: Option<String>,
    #[clap(long = "isLibrary")]
    pub is_library: Option<bool>,
}

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
}

#[derive(Subcommand, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum QmodOperation {
    /// Create a "mod.template.json" that you can pre-fill with certain values that will be used to then generate your final mod.json when you run 'qpm qmod build'
    ///
    /// Some properties are not settable through the qmod create command, these properties are either editable through the package, or not at all
    Create(CreateQmodJsonOperationArgs),
    /// This will parse the `mod.template.json` and process it, then finally export a `mod.json` for packaging and deploying.
    Build(BuildQmodOperationArgs),
    /// Edit your mod.template.json from the command line, mostly intended for edits on github actions
    ///
    /// Some properties are not editable through the qmod edit command, these properties are either editable through the package, or not at all
    Edit(edit::EditQmodJsonOperationArgs),
}

pub fn execute_qmod_operation(operation: Qmod) {
    match operation.op {
        QmodOperation::Create(q) => execute_qmod_create_operation(q),
        QmodOperation::Build(b) => execute_qmod_build_operation(b),
        QmodOperation::Edit(e) => edit::execute_qmod_edit_operation(e),
    }
}

fn execute_qmod_create_operation(create_parameters: CreateQmodJsonOperationArgs) {
    let schema_version = match create_parameters.schema_version {
        Option::Some(s) => s,
        Option::None => Version::new(1, 0, 0),
    };

    let json = ModJson {
        schema_version,
        name: "${mod_name}".to_string(),
        id: "${mod_id}".to_string(),
        author: create_parameters
            .author
            .unwrap_or_else(|| "---".to_string()),
        porter: create_parameters.porter,
        version: "${version}".to_string(),
        package_id: create_parameters.package_id,
        package_version: create_parameters.package_version,
        description: Some(
            create_parameters
                .description
                .unwrap_or_else(|| "${mod_id}, version ${version}!".to_string()),
        ),
        cover_image: create_parameters.cover_image,
        is_library: create_parameters.is_library,
        dependencies: Default::default(),
        mod_files: Default::default(),
        library_files: Default::default(),
        file_copies: Default::default(),
        copy_extensions: Default::default(),
    };

    json.write(PathBuf::from(ModJson::get_template_name()));
}

// This will parse the `qmod.template.json` and process it, then finally export a `qmod.json` for packaging and deploying.
fn execute_qmod_build_operation(build_parameters: BuildQmodOperationArgs) {
    assert!(std::path::Path::new("mod.template.json").exists(),
        "No mod.template.json found in the current directory, set it up please :) Hint: use \"qmod create\"");

    println!("Generating mod.json file from template...");
    let package = PackageConfig::read();
    let shared_package = SharedPackageConfig::from_package(&package);

    let mut mod_json: ModJson = shared_package.into();

    // Parse template mod.template.json
    let preprocess_data = PreProcessingData {
        version: package.info.version.to_string(),
        mod_id: package.info.id,
        mod_name: package.info.name,
    };

    let mut existing_json = ModJson::read_and_preprocess(&preprocess_data);
    if let Some(is_library) = build_parameters.is_library {
        existing_json.is_library = Some(is_library);
    }

    // if it's a library, append to libraryFiles, else to modFiles
    if existing_json.is_library.unwrap_or(false) {
        existing_json.library_files.append(&mut mod_json.mod_files);
    } else {
        existing_json.mod_files.append(&mut mod_json.mod_files);
    }

    existing_json
        .dependencies
        .append(&mut mod_json.dependencies);
    existing_json
        .library_files
        .append(&mut mod_json.library_files);

    if let Some(excluded) = build_parameters.exclude_libs {
        let exclude_filter = |lib_name: &String| -> bool {
            // returning false means don't include
            // don't include anything that is excluded
            if excluded.iter().any(|s| lib_name == s) {
                return false;
            }

            true
        };

        existing_json.mod_files.retain(exclude_filter);
        existing_json.library_files.retain(exclude_filter);
    } else if let Some(included) = build_parameters.include_libs {
        let include_filter = |lib_name: &String| -> bool {
            // returning false means don't include
            // only include anything that is specified included
            if included.iter().any(|s| lib_name == s) {
                return true;
            }

            false
        };

        existing_json.mod_files.retain(include_filter);
        existing_json.library_files.retain(include_filter);
    }

    // handled by preprocessing
    // existing_json.id = mod_json.id;
    // existing_json.version = mod_json.version;

    // Write mod.json
    existing_json.write(PathBuf::from(ModJson::get_result_name()));
}
