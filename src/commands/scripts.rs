use std::process::Stdio;

use clap::Args;

use color_eyre::eyre::{anyhow, bail};
use itertools::Itertools;
use qpm_arg_tokenizer::arg::Expression;
use qpm_package::models::package::PackageConfig;

use crate::models::package::PackageConfigExtensions;

use super::Command;

#[derive(Args)]
pub struct ScriptsCommand {
    script: String,

    args: Option<Vec<String>>,
}

impl Command for ScriptsCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;

        let scripts = package.workspace.scripts;

        let script = scripts.get(&self.script);

        if script.is_none() {
            bail!("Could not find script {}", self.script);
        }

        let supplied_args = self.args.unwrap_or_default();

        for command_str in script.unwrap() {
            let split = command_str.split_once(' ');

            let exec = match split {
                Some(s) => s.0,
                None => command_str,
            };

            let args: Vec<String> = match split {
                Some(s) => {
                    let expression = s.1;
                    let tokenized_args = Expression::parse(expression);

                    let formatted_args = tokenized_args
                        .replace(
                            supplied_args
                                .iter()
                                .map(|s| s.as_str())
                                .collect_vec()
                                .as_slice(),
                        )
                        .map_err(|e| anyhow!("{}", e))?;

                    formatted_args
                        .split(' ')
                        .map(|s| s.to_string())
                        .filter(|s| s.trim() != "")
                        .collect_vec()
                }
                None => vec![],
            };

            let mut c = std::process::Command::new(exec);
            c.args(args)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());

            c.spawn()?.wait()?.exit_ok()?;
        }

        Ok(())
    }
}
