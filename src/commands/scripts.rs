use std::process::Stdio;

use clap::Args;

use color_eyre::eyre::{ContextCompat, anyhow, bail};
use itertools::Itertools;
use qpm_arg_tokenizer::arg::Expression;
use qpm_package::models::{
    package::PackageConfig,
    triplet::{TripletId, default_triplet_id},
};

use crate::{models::package::PackageConfigExtensions, utils::ndk};

use super::Command;

#[derive(Args)]
pub struct ScriptsCommand {
    script: String,

    args: Option<Vec<String>>,

    #[clap(long, short)]
    triplet: Option<String>,
}

impl Command for ScriptsCommand {
    fn execute(self) -> color_eyre::Result<()> {
        let package = PackageConfig::read(".")?;

        let scripts = &package.workspace.scripts;

        let script = scripts.get(&self.script);

        let Some(script) = script else {
            bail!("Could not find script {}", self.script);
        };

        let supplied_args = self.args.unwrap_or_default();

        let triplet_id = self.triplet.map(TripletId).unwrap_or(default_triplet_id());

        invoke_script(script, &supplied_args, &package, &triplet_id)?;

        Ok(())
    }
}

pub fn invoke_script(
    script_commands: &[String],
    supplied_args: &[String],
    package: &PackageConfig,
    triplet_id: &TripletId,
) -> Result<(), color_eyre::eyre::Error> {
    let triplet = package
        .triplets
        .get_triplet_settings(triplet_id)
        .context("Failed to get triplet settings")?;

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

        // Set the environment variables for the script
        c.envs(
            triplet
                .env
                .iter()
                .map(|(k, v)| (format!("QPM_{k}"), v.as_str())),
        );

        // QPM defined environment variables
        c.env("QPM_ACTIVE_TRIPLET", triplet_id.to_string())
            .env("QPM_QMOD_ID", triplet.qmod_id.as_deref().unwrap_or(package.id.0.as_str()))
            .env("QPM_PACKAGE_ID", package.id.to_string())
            .env("QPM_PACKAGE_VERSION", package.version.to_string());

        // Set the environment variable for Android NDK home if provided
        if let Some(path) = &android_ndk_home {
            c.env("ANDROID_NDK_HOME", path);
        }

        c.spawn()?.wait()?.exit_ok()?;
    }
    Ok(())
}
