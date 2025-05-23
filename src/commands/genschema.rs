use clap::Args;
use color_eyre::eyre::Context;

use crate::{
    models::{config::UserConfig, schemas::SchemaLinks, toolchain::ToolchainData},
    repository::local::FileRepository,
};

use super::Command;

#[derive(Args, Clone, Debug)]
pub struct GenSchemaCommand {}

impl GenSchemaCommand {
    fn write_schema<T: ?Sized + schemars::JsonSchema>(url: &str) -> color_eyre::Result<()> {
        let schema_json = schemars::schema_for!(T);
        let schema = serde_json::to_string_pretty(&schema_json).unwrap();
        let name = url.rsplit('/').next().expect("Invalid URL");
        let err_msg = format!("Failed to write {name}");
        std::fs::write(name, schema).context(err_msg)?;
        Ok(())
    }
}

impl Command for GenSchemaCommand {
    fn execute(self) -> color_eyre::Result<()> {
        Self::write_schema::<UserConfig>(SchemaLinks::USER_CONFIG)?;
        Self::write_schema::<FileRepository>(SchemaLinks::FILE_REPOSITORY)?;
        Self::write_schema::<ToolchainData>(SchemaLinks::TOOLCHAIN_DATA)?;
        Ok(())
    }
}
