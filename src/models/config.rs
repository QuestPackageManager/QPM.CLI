use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    sync,
};

use color_eyre::{eyre::Context, Result};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::utils::json;

static COMBINED_CONFIG: sync::OnceLock<UserConfig> = sync::OnceLock::new();

pub fn get_combine_config() -> &'static UserConfig {
    COMBINED_CONFIG.get_or_init(|| UserConfig::read_combine().unwrap())
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[allow(non_snake_case)]
#[serde(rename_all = "camelCase")]
pub struct UserConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symlink: Option<bool>,
    /// Path where ndk downloads are stored
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ndk_download_path: Option<PathBuf>,
}

impl UserConfig {
    pub fn config_file_name() -> PathBuf {
        "qpm.settings.json".into()
    }

    pub fn global_config_path() -> PathBuf {
        Self::global_config_dir().join(Self::config_file_name())
    }

    pub fn global_config_dir() -> PathBuf {
        dirs::config_dir().unwrap().join("QPM-RS")
    }

    pub fn read_global() -> Result<Self> {
        fs::create_dir_all(Self::global_config_path().parent().unwrap())?;

        if !Self::global_config_path().exists() {
            let def = Self::default();
            def.write(false)?;
            return Ok(def);
        }

        let path = Self::global_config_path();

        let file = File::open(&path)
            .with_context(|| format!("Unable to open global config file at {path:?}"))?;
        json::json_from_reader_fast(BufReader::new(file))
            .with_context(|| format!("Unable to deserialize global config file at {path:?}"))
    }

    pub fn read_workspace() -> Result<Option<Self>> {
        let path = Path::new(".").join(Self::config_file_name());
        if !path.exists() {
            return Ok(None);
        }

        let file = File::options()
            .read(true)
            .open(&path)
            .with_context(|| format!("Unable to open workspace config file at {path:?}"))?;

        let config = json::json_from_reader_fast(BufReader::new(file))
            .with_context(|| format!("Unable to deserialize workspace config file at {path:?}"))?;

        Ok(Some(config))
    }

    pub fn write(&self, workspace: bool) -> Result<()> {
        let path = if workspace {
            Path::new(".").join(Self::config_file_name())
        } else {
            Self::global_config_path()
        };

        let mut file = File::create(&path)
            .with_context(|| format!("Unable to write config file at {path:?}"))?;
        serde_json::to_writer_pretty(&mut file, self)
            .with_context(|| format!("Unable to serialize global config file at {path:?}"))?;

        Ok(())
    }

    pub fn read_combine() -> Result<Self> {
        let global = Self::read_global()?;
        let local = Self::read_workspace()?;

        Ok(match local {
            Some(local) => Self {
                cache: local.cache.or(global.cache),
                timeout: local.timeout.or(global.timeout),
                symlink: local.symlink.or(global.symlink),
                ndk_download_path: local.ndk_download_path.or(global.ndk_download_path),
            },
            None => global,
        })
    }

    pub fn get_ndk_installed(&self) -> WalkDir {
        let dir = get_combine_config()
            .ndk_download_path
            .as_ref()
            .expect("No NDK download path set");
        WalkDir::new(dir).max_depth(1)
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            symlink: Some(true),
            cache: Some(dirs::data_dir().unwrap().join("QPM-RS").join("cache")),
            timeout: Some(60000),
            ndk_download_path: Some(dirs::data_dir().unwrap().join("QPM-RS").join("ndk")),
        }
    }
}

#[inline]
pub fn get_keyring() -> keyring::Entry {
    keyring::Entry::new("qpm", "github").unwrap()
}
#[inline]
pub fn get_publish_keyring() -> keyring::Entry {
    keyring::Entry::new("qpm", "publish").unwrap()
}
