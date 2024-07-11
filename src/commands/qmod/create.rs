use clap::Args;
use semver::Version;

use std::path::PathBuf;

use qpm_qmod::models::mod_json::ModJson;

use color_eyre::Result;

use crate::models::mod_json::ModJsonExtensions;

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

pub(crate) fn execute_qmod_create_operation(
    create_parameters: CreateQmodJsonOperationArgs,
) -> Result<()> {
    let schema_version = match &create_parameters.schema_version {
        Option::Some(s) => s.clone(),
        Option::None => Version::new(1, 1, 0),
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
        ..Default::default()
    };

    json.write(&PathBuf::from(ModJson::get_template_name()))?;
    Ok(())
}
