use std::{
    fs::File,
    io::{copy, Cursor, Read},
};

use clap::{Args, Subcommand};
use color_eyre::{eyre::bail, Result};
use itertools::Itertools;
use owo_colors::OwoColorize;
use zip::ZipArchive;

use crate::{
    network::agent::get_agent,
    terminal::colors::QPMColor,
    utils::android::{
        download_ndk_version, get_android_manifest, get_ndk_str_versions,
    },
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
    Available,
}

#[derive(Args, Debug, Clone)]
pub struct DownloadArgs {
    version: String,
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
            NdkOperation::Available => {
                let manifest = get_android_manifest()?;
                get_ndk_str_versions(&manifest)
                    .iter()
                    .sorted_by(|a, b| a.0.cmp(b.0))
                    .rev()
                    .take(5)
                    .for_each(|(v, p)| println!("{} -> {}", v.blue(), p.display_name.purple()))
            }
            NdkOperation::List => todo!(),
        }

        Ok(())
    }
}
