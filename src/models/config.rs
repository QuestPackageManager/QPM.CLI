use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    sync,
};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

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
        dirs::config_dir().unwrap().join("QPM-Rs")
    }

    pub fn read_global() -> Result<Self> {
        fs::create_dir_all(Self::global_config_path().parent().unwrap())?;

        if !Self::global_config_path().exists() {
            let def = Self::default();
            def.write(false)?;
            return Ok(def);
        }

        let file = File::open(Self::global_config_path())?;
        json::json_from_reader_fast(BufReader::new(file))
    }

    pub fn read_workspace() -> Result<Option<Self>> {
        let path = Path::new(".").join(Self::config_file_name());
        if !path.exists() {
            return Ok(None);
        }

        let file = File::options().read(true).open(path)?;
        Ok(Some(json::json_from_reader_fast(BufReader::new(file))?))
    }

    pub fn write(&self, workspace: bool) -> Result<()> {
        let path = if workspace {
            Path::new(".").join(Self::config_file_name())
        } else {
            Self::global_config_path()
        };

        let mut file = File::create(path)?;
        serde_json::to_writer_pretty(&mut file, self)?;

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
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            symlink: Some(true),
            cache: Some(dirs::data_dir().unwrap().join("QPM-Rs").join("cache")),
            timeout: Some(5000),
            ndk_download_path: Some(dirs::data_dir().unwrap().join("QPM-Rs").join("ndk")),
        }
    }
}

#[inline]
pub fn get_keyring() -> keyring::Entry {
    keyring::Entry::new("qpm", "github")
}
#[inline]
pub fn get_publish_keyring() -> keyring::Entry {
    keyring::Entry::new("qpm", "publish")
}
