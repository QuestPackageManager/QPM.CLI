use std::path::{PathBuf};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[allow(non_snake_case)]
#[serde(rename_all = "camelCase")]
pub struct UserConfig {
    pub cache: PathBuf,
    pub timeout: u32,
}

impl UserConfig {
    fn read() -> Result<Self> {
        Ok(UserConfig {
            cache: todo!(),
            timeout: todo!(),
        })
    }

    fn write(&self) -> Result<()> {
        Ok(())
    }

    pub fn read_combine() -> Self {
        todo!()
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            cache: Default::default(),
            timeout: Default::default(),
        }
    }
}
