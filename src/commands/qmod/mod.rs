use std::path::PathBuf;

use clap::{Args, Subcommand};
use color_eyre::{eyre::ensure, Result};

use qpm_package::{
    extensions::package_metadata::PackageMetadataExtensions,
    models::{dependency::SharedPackageConfig, package::PackageConfig},
};
use qpm_qmod::models::mod_json::ModJson;

use crate::models::{
    mod_json::ModJsonExtensions,
    package::{PackageConfigExtensions, SharedPackageConfigExtensions},
};

use super::Command;

mod build;
mod create;
mod edit;

#[derive(Args, Debug, Clone)]

pub struct QmodCommand {
    #[clap(subcommand)]
    pub op: QmodOperation,
}

#[derive(Subcommand, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum QmodOperation {
    /// Create a "mod.template.json" that you can pre-fill with certain values that will be used to then generate your final mod.json when you run 'qpm qmod build'
    ///
    /// Some properties are not settable through the qmod create command, these properties are either editable through the package, or not at all
    Create(create::CreateQmodJsonOperationArgs),
    /// This will parse the `mod.template.json` and process it, then finally export a `mod.json` for packaging and deploying.
    Build(build::BuildQmodOperationArgs),
    /// Edit your mod.template.json from the command line, mostly intended for edits on github actions
    ///
    /// Some properties are not editable through the qmod edit command, these properties are either editable through the package, or not at all
    Edit(edit::EditQmodJsonCommand),
}

impl Command for QmodCommand {
    fn execute(self) -> Result<()> {
        match self.op {
            QmodOperation::Create(q) => create::execute_qmod_create_operation(q),
            QmodOperation::Build(b) => build::execute_qmod_build_operation(b),
            QmodOperation::Edit(e) => e.execute(),
        }
    }
}
