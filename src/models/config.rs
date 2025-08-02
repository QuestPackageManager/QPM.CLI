use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    sync,
};

use color_eyre::{Result, eyre::Context};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::utils::json;

use super::schemas::{SchemaLinks, WithSchema};

static COMBINED_CONFIG: sync::OnceLock<UserConfig> = sync::OnceLock::new();

pub fn get_combine_config() -> &'static UserConfig {
    COMBINED_CONFIG.get_or_init(|| UserConfig::read_combine().unwrap())
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, Hash, PartialEq, Eq)]
#[allow(non_snake_case)]
#[serde(rename_all = "camelCase")]
#[schemars(description = "User configuration for QPM-RS")]
pub struct UserConfig {
    /// Path where cache is stored
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<PathBuf>,

    /// Timeout for http requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,

    /// Whether to symlink or copy files
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
        dirs::config_dir().unwrap().join("QPM-RS2")
    }

    pub fn read_global() -> Result<Self> {
        // During tests, use a default configuration instead of reading from the global file
        if std::env::var("QPM_DISABLE_GLOBAL_CONFIG").is_ok() {
            return Ok(Self::default());
        }

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
        // During tests, don't write to the global configuration file
        if !workspace && std::env::var("QPM_DISABLE_GLOBAL_CONFIG").is_ok() {
            return Ok(());
        }

        let path = if workspace {
            Path::new(".").join(Self::config_file_name())
        } else {
            Self::global_config_path()
        };

        let mut file = File::create(&path)
            .with_context(|| format!("Unable to write config file at {path:?}"))?;
        serde_json::to_writer_pretty(
            &mut file,
            &WithSchema {
                schema: SchemaLinks::USER_CONFIG,
                value: self,
            },
        )
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
            cache: Some(dirs::data_dir().unwrap().join("QPM-RS2").join("cache")),
            timeout: Some(60000),
            ndk_download_path: Some(ndk_default_path()),
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

#[cfg(windows)]
pub fn ndk_default_path() -> PathBuf {
    // Android studio NDK location
    dirs::data_local_dir()
        .unwrap()
        .join("Android")
        .join("Sdk")
        .join("ndk")
    // C:\Users\<UserName>\AppData\Local\Android\Sdk\ndk\*
}


#[cfg(not(windows))]
pub fn ndk_default_path() -> PathBuf {
    dirs::data_dir().unwrap().join("QPM-RS2").join("ndk")
}