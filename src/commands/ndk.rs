use std::env;

use clap::{Args, Subcommand};
use color_eyre::{eyre::bail, Result};
use itertools::Itertools;
use owo_colors::OwoColorize;
use walkdir::WalkDir;

use crate::{
    models::config::get_combine_config,
    utils::android::{download_ndk_version, get_android_manifest, get_ndk_str_versions},
};

use super::Command;

#[derive(Args, Debug, Clone)]
pub struct Ndk {
    #[clap(subcommand)]
    pub op: NdkOperation,
}

#[derive(Subcommand, Debug, Clone)]
pub enum NdkOperation {
    Download(DownloadArgs),
    List,
    Available(AvailableArgs),
    Env(EnvArgs),
}

#[derive(Args, Debug, Clone)]
pub struct DownloadArgs {
    version: String,
}

#[derive(Args, Debug, Clone)]
pub struct AvailableArgs {
    page: usize,
}

#[derive(Args, Debug, Clone)]
pub struct EnvArgs {
    #[clap(subcommand)]
    op: EnvOperation,
}

#[derive(Subcommand, Debug, Clone)]
pub enum EnvOperation {
    Set(EnvSetArgs),
    Get,
}

#[derive(Args, Debug, Clone)]
pub struct EnvSetArgs {
    ndk: String,
}

impl Command for Ndk {
    fn execute(self) -> Result<()> {
        match self.op {
            NdkOperation::Download(d) => {
                let manifest = get_android_manifest()?;
                let ndks = get_ndk_str_versions(&manifest);

                let ndk = ndks.get(d.version.as_str());
                match ndk {
                    Some(ndk) => download_ndk_version(ndk)?,
                    None => bail!("Could not find ndk version {}", d.version),
                }
            }
            NdkOperation::Available(a) => {
                let manifest = get_android_manifest()?;
                let amount_per_page = 5;

                let skip = (a.page - 1).max(0) * amount_per_page;

                println!("Page: {amount_per_page}");

                get_ndk_str_versions(&manifest)
                    .iter()
                    .sorted_by(|a, b| a.0.cmp(b.0))
                    .rev()
                    .skip(skip)
                    .take(5)
                    .for_each(|(v, p)| println!("{} -> {}", v.blue(), p.display_name.purple()))
            }
            NdkOperation::List => {
                let dir = get_combine_config()
                    .ndk_download_path
                    .as_ref()
                    .expect("No NDK download path set");

                WalkDir::new(dir)
                    .max_depth(1)
                    .into_iter()
                    .try_collect::<_, Vec<_>, _>()?
                    .into_iter()
                    .filter(|p| p.path().is_dir())
                    .for_each(|p| {
                        println!(
                            "{} -> {}",
                            p.file_name().to_str().unwrap(),
                            p.path().to_str().unwrap()
                        )
                    })
            }
            NdkOperation::Env(e) => {
                match e.op {
                    EnvOperation::Set(e) => {
                        env::set_var("NDK_ANDROID_HOME", &e.ndk);
                        println!("Android ndk home set to {}", e.ndk.green())

                    },
                    EnvOperation::Get => {
                        let ndk_home = env::var("NDK_ANDROID_HOME")?;
                        println!("Android ndk home is set to {}", ndk_home.green())
                    },
                }

            },
        }

        Ok(())
    }
}
