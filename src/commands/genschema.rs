use clap::Args;

use crate::{
    models::{config::UserConfig, toolchain::ToolchainData}, repository::local::FileRepository
};

use super::Command;

#[derive(Args, Clone, Debug)]
pub struct GenSchemaCommand { }

impl GenSchemaCommand {
    fn write_schema<T: ?Sized + schemars::JsonSchema>(name: &str) -> color_eyre::Result<()> {
        let schema_json = schemars::schema_for!(T);
        let schema = serde_json::to_string_pretty(&schema_json).unwrap();
        std::fs::write(name, schema).expect(&format!("Failed to write {}", name));
        Ok(())
    }
}

impl Command for GenSchemaCommand {
    fn execute(self) -> color_eyre::Result<()> {
        Self::write_schema::<UserConfig>("qpm.settings.schema.json")?;
        Self::write_schema::<FileRepository>("qpm.repository.schema.json")?;
        Self::write_schema::<ToolchainData>("qpm.toolchain.schema.json")?;
        Ok(())
    }
}
