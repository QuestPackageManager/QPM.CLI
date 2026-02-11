use std::process::Stdio;

use clap::Args;

use color_eyre::eyre::{anyhow, bail};
use itertools::Itertools;
use qpm_arg_tokenizer::arg::Expression;
use qpm_package::models::package::PackageConfig;

use crate::{models::package::PackageConfigExtensions, utils::ndk};

use super::Command;

#[derive(Args)]
#[command(disable_help_flag = true)]
pub struct ScriptsCommand {
    script: String,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

impl Command for ScriptsCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;

        let scripts = &package.workspace.scripts;

        let script = scripts.get(&self.script);

        if script.is_none() {
            bail!("Could not find script {}", self.script);
        }

        let Some(script) = script else {
            return Ok(());
        };

        invoke_script(script, &self.args, &package)?;

        Ok(())
    }
}

pub fn invoke_script(
    script_commands: &[String],
    supplied_args: &[String],
    package: &PackageConfig,
) -> Result<(), color_eyre::eyre::Error> {
    let android_ndk_home = ndk::resolve_ndk_version(package);

    for command_str in script_commands {
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

        // Set the environment variable for Android NDK home if provided
        if let Some(path) = &android_ndk_home {
            c.env("ANDROID_NDK_HOME", path);
        }

        let code = c.spawn()?.wait()?.code().unwrap_or_else(|| 1);
        if code != 0 {
            break;
        }
    }
    Ok(())
}
