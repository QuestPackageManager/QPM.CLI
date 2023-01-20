use std::{env, fs::File};

use clap::Args;
use color_eyre::{
    eyre::{anyhow, bail},
    Help, Result,
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
            .map(|p| format!("{}/{}", p, path))
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

        let qpm_rust = look_path("qpm-rust")?;

        if !cmake {
            bail!("CMake is not installed in path!")
        } else {
            println!("Cmake found!");
        }

        if !ninja {
            bail!("Ninja is not installed in path!")
        } else {
            println!("Ninja found!");
        }

        if !qpm_rust {
            bail!("QPM-Rust not found in path!")
        } else {
            println!("QPM-Rust found!");
        }

        if !adb {
            bail!("ADB not installed in path")
        } else {
            println!("ADB found!");
        }

        if File::open("./qpm.json").is_ok() {
            let ndk_path = env::var("ANDROID_NDK_HOME");

            if ndk_path.is_ok() {
                println!("NDK {} found in path!", ndk_path.unwrap());
            } else if let Err(err) = ndk_path && File::open("./ndkpath.txt").is_err() {
                return Err(anyhow!(
                    "No ndkpath.txt or ANDROID_NDK_HOME environment variable found!"
                )
                .error(err));
            }
        };

        println!("{}", "Everything looks good!".green());

        Ok(())
    }
}
