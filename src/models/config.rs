use color_eyre::Result;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[allow(non_snake_case)]
#[serde(rename_all = "camelCase")]
pub struct UserConfig {

}

impl UserConfig {
    fn read() -> Result<Self> {

        Ok(UserConfig {

        })
    }

    fn write(&self) -> Result<()> {

        Ok(())
    }
}