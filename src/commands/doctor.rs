use std::{env, fs::File};

use clap::Args;
use color_eyre::{
    Help, Result,
    eyre::{anyhow, bail},
};
use owo_colors::OwoColorize;

use super::Command;

// look_path returns a boolean indicating if the binary can be found in $PATH.
#[cfg(unix)]
fn look_path(path: &str) -> Result<bool, std::env::VarError> {
    use std::path::Path;

    std::env::var("PATH").map(|paths| {
        paths
            .split(':')
            .map(|p| format!("{p}/{path}"))
            .any(|p| Path::new(&p).exists())
    })
}

#[cfg(windows)]
fn look_path(path: &str) -> Result<bool, std::env::VarError> {
    use std::path::Path;

    std::env::var("PATH").map(|paths| {
        paths
            .split(';')
            .map(|p| format!("{p}/{path}"))
            .any(|p| Path::new(&p).with_extension("exe").exists())
    })
}

#[derive(Args)]
pub struct DoctorCommand {}
impl Command for DoctorCommand {
    fn execute(self) -> Result<()> {
        let cmake = look_path("cmake")?;
        let ninja = look_path("ninja")?;
        let adb = look_path("adb")?;

        let qpm = look_path("qpm")?;

        if !cmake {
            eprintln!(
                "CMake is not installed in path! Use winget or your OS package manager to install CMake."
            )
        } else {
            println!("Cmake found!");
        }

        if !ninja {
            eprintln!(
                "Ninja is not installed in path! Use {} to download ninja",
                "qpm download ninja".yellow()
            )
        } else {
            println!("Ninja found!");
        }

        if !qpm {
            eprintln!("Qpm not found in path!")
        } else {
            println!("Qpm found!");
        }

        if !adb {
            eprintln!(
                "ADB not installed in path. Use {} to download ADB",
                "qpm download adb".yellow()
            )
        } else {
            println!("ADB found!");
        }

        if File::open("./qpm.json").is_ok() {
            let ndk_path = env::var("ANDROID_NDK_HOME");

            if ndk_path.is_ok() {
                println!("NDK {} found in path!", ndk_path.unwrap());
            } else if let Err(err) = ndk_path
                && File::open("./ndkpath.txt").is_err()
            {
                return Err(anyhow!(
                    "No ndkpath.txt or ANDROID_NDK_HOME environment variable found!"
                )
                .error(err));
            }
        };

        if cmake && adb && qpm && ninja {
            println!("{}", "Everything looks good!".green());
        } else {
            bail!("Some functionality is missing")
        }
        Ok(())
    }
}
