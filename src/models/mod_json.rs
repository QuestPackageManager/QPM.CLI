use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use color_eyre::{Result, eyre::Context};

use itertools::Itertools;
use qpm_qmod::models::mod_json::{ModDependency, ModJson};

use crate::utils::json;

use super::schemas::{SchemaLinks, WithSchema};

pub trait ModJsonExtensions: Sized {
    fn get_result_name() -> &'static str;
    fn read_and_preprocess(preprocess_data: PreProcessingData, file: &Path) -> Result<Self>;

    fn read(path: &Path) -> Result<Self>;
    fn write(&self, path: &Path) -> Result<()>;

    fn merge_modjson(existing_json: ModJson, template_mod_json: ModJson) -> ModJson;
}

pub struct PreProcessingData {
    pub version: String,
    pub mod_id: String,

    pub binaries: Vec<String>,

    pub game_id: Option<String>,
    pub game_version: Option<String>,

    pub additional_env: HashMap<String, String>,
}

impl PreProcessingData {
    fn preprocess(self, s: String) -> String {
        let mut env = s
            .replace("${version}", &self.version)
            .replace("${mod_id}", &self.mod_id)
            // .replace("${mod_name}", &self.mod_name)
            .replace("${game_id}", &self.game_id.unwrap_or("".to_string()))
            .replace(
                "${game_version}",
                &self.game_version.unwrap_or("".to_string()),
            );

        for env_var in self.additional_env {
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
}

impl ModJsonExtensions for ModJson {
    fn get_result_name() -> &'static str {
        "mod.json"
    }

    fn read_and_preprocess(preprocess_data: PreProcessingData, path: &Path) -> Result<Self> {
        let mut file = File::open(path).context("Opening mod.json failed")?;

        // Get data
        let mut json = String::with_capacity(file.metadata()?.len() as usize);
        file.read_to_string(&mut json)
            .context("Reading data failed")?;

        // Pre process
        let processsed = preprocess_data.preprocess(json);

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

    fn merge_modjson(mut existing_json: ModJson, mut template_mod_json: ModJson) -> ModJson {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use qpm_qmod::models::mod_json::{ModDependency, ModJson};
    use semver::VersionReq;

    use super::{ModJsonExtensions, PreProcessingData};

    fn data(game_id: Option<&str>, game_version: Option<&str>) -> PreProcessingData {
        PreProcessingData {
            version: "1.2.3".into(),
            mod_id: "my-mod".into(),
            binaries: vec![],
            game_id: game_id.map(Into::into),
            game_version: game_version.map(Into::into),
            additional_env: HashMap::from([("FOO".to_string(), "bar".to_string())]),
        }
    }

    fn dep(id: &str, range: &str) -> ModDependency {
        ModDependency {
            version_range: VersionReq::parse(range).unwrap(),
            id: id.into(),
            mod_link: None,
            required: None,
        }
    }

    #[test]
    fn preprocess_replaces_known_placeholders() {
        let out = data(Some("com.beatgames.beatsaber"), Some("1.40.0"))
            .preprocess("${mod_id}@${version} for ${game_id} ${game_version}".into());
        assert_eq!(out, "my-mod@1.2.3 for com.beatgames.beatsaber 1.40.0");
    }

    #[test]
    fn preprocess_blanks_missing_game_info() {
        let out = data(None, None).preprocess("[${game_id}][${game_version}]".into());
        assert_eq!(out, "[][]");
    }

    #[test]
    fn preprocess_substitutes_qpm_prefixed_env_only() {
        let out = data(None, None).preprocess("${QPM_FOO} ${FOO} ${QPM_MISSING} ${unknown}".into());
        assert_eq!(out, "bar ${FOO} ${QPM_MISSING} ${unknown}");
    }

    #[test]
    fn merge_drops_template_binaries_already_declared_anywhere() {
        let existing = ModJson {
            mod_files: vec!["libmain.so".into()],
            late_mod_files: vec!["liblate.so".into()],
            library_files: vec!["libshared.so".into()],
            ..Default::default()
        };
        let template = ModJson {
            library_files: vec![
                "libmain.so".into(),
                "liblate.so".into(),
                "libshared.so".into(),
                "libnew.so".into(),
            ],
            ..Default::default()
        };

        let merged = ModJson::merge_modjson(existing, template);

        assert_eq!(merged.mod_files, vec!["libmain.so"]);
        assert_eq!(merged.late_mod_files, vec!["liblate.so"]);
        assert_eq!(merged.library_files, vec!["libshared.so", "libnew.so"]);
    }

    #[test]
    fn merge_keeps_existing_dependency_on_id_clash() {
        let existing = ModJson {
            dependencies: vec![dep("a", "^1.0.0")],
            ..Default::default()
        };
        let template = ModJson {
            dependencies: vec![dep("a", "^2.0.0"), dep("b", "^0.1.0")],
            ..Default::default()
        };

        let merged = ModJson::merge_modjson(existing, template);

        let deps: Vec<_> = merged
            .dependencies
            .iter()
            .map(|d| (d.id.as_str(), d.version_range.to_string()))
            .collect();
        assert_eq!(deps, vec![("a", "^1.0.0".into()), ("b", "^0.1.0".into())]);
    }

    /// Only library files and dependencies come from the template; its mod files never do
    #[test]
    fn merge_does_not_take_template_mod_files() {
        let template = ModJson {
            mod_files: vec!["libtemplate.so".into()],
            late_mod_files: vec!["libtemplate_late.so".into()],
            ..Default::default()
        };

        let merged = ModJson::merge_modjson(ModJson::default(), template);

        assert!(merged.mod_files.is_empty());
        assert!(merged.late_mod_files.is_empty());
    }

    #[test]
    fn merge_dedupes_existing_lists() {
        let existing = ModJson {
            mod_files: vec!["a.so".into(), "a.so".into()],
            late_mod_files: vec!["b.so".into(), "b.so".into()],
            dependencies: vec![dep("x", "^1.0.0"), dep("x", "^2.0.0")],
            ..Default::default()
        };

        let merged = ModJson::merge_modjson(existing, ModJson::default());

        assert_eq!(merged.mod_files, vec!["a.so"]);
        assert_eq!(merged.late_mod_files, vec!["b.so"]);
        assert_eq!(merged.dependencies.len(), 1);
        assert_eq!(merged.dependencies[0].version_range.to_string(), "^1.0.0");
    }
}
