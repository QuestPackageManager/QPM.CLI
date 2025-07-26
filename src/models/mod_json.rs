use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use color_eyre::{Result, eyre::Context};

use itertools::Itertools;
use qpm_qmod::models::mod_json::{ModDependency, ModJson};

use crate::{commands::Opt, utils::json};

use super::schemas::{SchemaLinks, WithSchema};

pub trait ModJsonExtensions: Sized {
    fn get_template_name() -> &'static str;
    fn get_result_name() -> &'static str;
    fn get_template_path() -> PathBuf;
    fn read_and_preprocess(preprocess_data: PreProcessingData) -> Result<Self>;

    fn read(path: &Path) -> Result<Self>;
    fn write(&self, path: &Path) -> Result<()>;

    fn merge_modjson(
        existing_json: ModJson,
        template_mod_json: ModJson,
        add_default_binary: bool,
    ) -> ModJson;
}

pub struct PreProcessingData {
    pub version: String,
    pub mod_id: String,

    pub binaries: Vec<String>,

    pub game_id: Option<String>,
    pub game_version: Option<String>,

    pub additional_env: HashMap<String, String>,
}

impl ModJsonExtensions for ModJson {
    fn get_template_name() -> &'static str {
        "mod.template.json"
    }

    fn get_result_name() -> &'static str {
        "mod.json"
    }

    fn get_template_path() -> std::path::PathBuf {
        PathBuf::new().join(Self::get_template_name())
    }

    fn read_and_preprocess(preprocess_data: PreProcessingData) -> Result<Self> {
        let mut file = File::open(Self::get_template_name()).context("Opening mod.json failed")?;

        // Get data
        let mut json = String::with_capacity(file.metadata()?.len() as usize);
        file.read_to_string(&mut json).expect("Reading data failed");

        // Pre process
        let processsed = preprocess(json, preprocess_data);

        serde_json::from_str(&processsed).context("Deserializing package failed")
    }

    fn read(path: &Path) -> Result<ModJson> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("Opening ModJson at {path:?} failed"))?;

        json::json_from_reader_fast(BufReader::new(file))
            .with_context(|| format!("Unable to deserialize ModJson at {path:?}"))
    }

    fn write(&self, path: &Path) -> Result<()> {
        let file = File::create(path)
            .with_context(|| format!("Unable to create ModJson file at {path:?}"))?;
        serde_json::to_writer_pretty(
            file,
            &WithSchema {
                schema: SchemaLinks::MOD_CONFIG,
                value: self,
            },
        )
        .with_context(|| format!("Unable to deserialize ModJson file at {path:?}"))?;
        Ok(())
    }

    fn merge_modjson(
        mut existing_json: ModJson,
        mut template_mod_json: ModJson,
        add_default_binary: bool,
    ) -> ModJson {
        let existing_binaries: HashSet<String> = existing_json
            .library_files
            .iter()
            .chain(existing_json.mod_files.iter())
            .chain(existing_json.late_mod_files.iter())
            .cloned()
            .collect();
        let existing_dependencies: HashMap<String, ModDependency> = existing_json
            .dependencies
            .iter()
            .cloned()
            .map(|d| (d.id.clone(), d))
            .collect();

        // Remove entries we already declare in existing ModJsonw
        template_mod_json
            .late_mod_files
            .retain(|s| !existing_binaries.contains(s));
        template_mod_json
            .mod_files
            .retain(|s| !existing_binaries.contains(s));
        template_mod_json
            .library_files
            .retain(|s| !existing_binaries.contains(s));
        template_mod_json
            .dependencies
            .retain(|d| !existing_dependencies.contains_key(&d.id));

        if add_default_binary {
            insert_default_mod_binary(&mut existing_json, &mut template_mod_json);
        };

        existing_json
            .library_files
            .append(&mut template_mod_json.library_files);

        existing_json
            .dependencies
            .append(&mut template_mod_json.dependencies);

        // Remove duplicates
        existing_json.mod_files = existing_json.mod_files.into_iter().unique().collect();
        existing_json.late_mod_files = existing_json.late_mod_files.into_iter().unique().collect();
        existing_json.library_files = existing_json.library_files.into_iter().unique().collect();
        existing_json.dependencies = existing_json
            .dependencies
            .into_iter()
            .unique_by(|d| d.id.clone())
            .collect();

        existing_json
    }
}
fn preprocess(s: String, preprocess_data: PreProcessingData) -> String {
    let mut env = s.replace("${version}", &preprocess_data.version)
        .replace("${mod_id}", &preprocess_data.mod_id)
        // .replace("${mod_name}", &preprocess_data.mod_name)
        .replace(
            "${game_id}",
            &preprocess_data.game_id.unwrap_or("".to_string()),
        )
        .replace(
            "${game_version}",
            &preprocess_data.game_version.unwrap_or("".to_string()),
        );

    for env_var in preprocess_data.additional_env {
        let key = env_var.0;
        let value = env_var.1;
        // Replace all occurrences of ${QPM_key} with value
        env = env.replace(&format!("${{QPM_{key}}}"), &value);
    }

    env
    // .replace(
    //     "${binary}",
    //     preprocess_data
    //         .binary
    //         .unwrap_or("${binary}".to_string())
    //         .as_str(),
    // )
}

fn insert_default_mod_binary(existing_json: &mut ModJson, template_mod_json: &mut ModJson) {
    // put it all under library
    let is_library = existing_json.is_library.unwrap_or(false);
    if is_library {
        existing_json
            .library_files
            .append(&mut template_mod_json.late_mod_files);
        existing_json
            .library_files
            .append(&mut template_mod_json.mod_files);
        return;
    }

    existing_json
        .mod_files
        .append(&mut template_mod_json.mod_files);
    existing_json
        .late_mod_files
        .append(&mut template_mod_json.late_mod_files);
}
