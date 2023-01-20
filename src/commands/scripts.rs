use std::{process::Stdio};

use clap::Args;

use color_eyre::eyre::{bail};
use itertools::Itertools;
use qpm_package::models::{package::PackageConfig};

use crate::{
    models::{
        package::{PackageConfigExtensions},
    },
};

use super::Command;

#[derive(Args)]
pub struct ScriptsCommand {
    script: String,
}

impl Command for ScriptsCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;

        let scripts = package.workspace.map(|w| w.scripts);

        if scripts.is_none() {
            bail!("No scripts defined in qpm.json::workspace::scripts");
        }

        let scripts_unwrapped = scripts.unwrap();

        let script = scripts_unwrapped.get(&self.script);

        if script.is_none() {
            bail!("Could not find script {}", self.script);
        }

        for command_str in script.unwrap() {
            let split = command_str.split_once(' ');

            let exec = match split {
                Some(s) => s.0,
                None => command_str,
            };

            let args = match split {
                Some(s) => s.1.split(' ').collect_vec(),
                None => vec![],
            };

            let mut c = std::process::Command::new(exec);
            c
                .args(args)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());

            c.spawn()?.wait()?.exit_ok()?;
        }

        Ok(())
    }
}
