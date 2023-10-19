use std::{
    fs::File,
    io::{Read, BufReader},
    path::{Path, PathBuf},
};

use color_eyre::{eyre::Context, Result};
use qpm_package::models::dependency::{SharedPackageConfig, Dependency, SharedDependency};
use qpm_qmod::models::mod_json::{ModJson, ModDependency};

use crate::utils::json;

pub trait ModJsonExtensions: Sized {
    fn get_template_name() -> &'static str;
    fn get_result_name() -> &'static str;
    fn get_template_path() -> PathBuf;
    fn read_and_preprocess(preprocess_data: PreProcessingData) -> Result<Self>;

    fn read(path: &Path) -> Result<Self>;
    fn write(&self, path: &Path) -> Result<()>;
}

pub struct PreProcessingData {
    pub version: String,
    pub mod_id: String,
    pub mod_name: String,
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
        let file = std::fs::File::open(path).context("Opening mod.json failed")?;

        json::json_from_reader_fast(BufReader::new(file))
    }

    fn write(&self, path: &Path) -> Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}
fn preprocess(s: String, preprocess_data: PreProcessingData) -> String {
    s.replace("${version}", &preprocess_data.version)
        .replace("${mod_id}", &preprocess_data.mod_id)
        .replace("${mod_name}", &preprocess_data.mod_name)
}