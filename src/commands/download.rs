use std::{
    fs::File,
    io::{copy, Cursor, Read},
};

use bytes::Bytes;
use clap::{Args, Subcommand};
use color_eyre::{eyre::bail, Result};
use owo_colors::OwoColorize;
use zip::ZipArchive;

use crate::{network::agent::download_file_report, terminal::colors::QPMColor};

use super::Command;

#[cfg(target_os = "linux")]
const NINJA_DOWNLOAD: &str =
    "https://github.com/ninja-build/ninja/releases/latest/download/ninja-linux.zip";

#[cfg(target_os = "macos")]
const NINJA_DOWNLOAD: &str =
    "https://github.com/ninja-build/ninja/releases/latest/download/ninja-mac.zip";

#[cfg(windows)]
const NINJA_DOWNLOAD: &str =
    "https://github.com/ninja-build/ninja/releases/latest/download/ninja-win.zip";

/// CMAKE
/// TODO: Extract tars on Linux/Mac

#[cfg(windows)]
const CMAKE_DOWNLOAD: &str =
    "https://github.com/Kitware/CMake/releases/download/v3.24.2/cmake-3.24.2-windows-x86_64.zip";

#[cfg(target_os = "linux")]
const CMAKE_DOWNLOAD: &str =
    "https://github.com/Kitware/CMake/releases/download/v3.24.2/cmake-3.24.2-linux-x86_64.tar.gz";

#[cfg(target_os = "macos")]
const CMAKE_DOWNLOAD: &str = "https://github.com/Kitware/CMake/releases/download/v3.24.2/cmake-3.24.2-macos-universal.tar.gz";

#[derive(Args, Debug, Clone)]
pub struct Download {
    #[clap(subcommand)]
    pub op: DownloadOperation,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DownloadOperation {
    Ninja,
    #[clap(name = "cmake")]
    CMake,
}

impl Command for Download {
    fn execute(self) -> Result<()> {
        let download = self.op;

        let url = match download {
            DownloadOperation::Ninja => NINJA_DOWNLOAD,
            DownloadOperation::CMake => CMAKE_DOWNLOAD,
        };

        let exe = std::env::current_exe()?;
        let final_path = exe.parent().unwrap();

        let bytes: Bytes = download_file_report(url, |_, _| {})?.into();
        let buffer = Cursor::new(bytes);

        // Extract to tmp folde
        let mut archive = ZipArchive::new(buffer)?;

        if archive.len() == 1 {
            // Extract to tmp folder
            // let inner_bytes = bytes::Bytes::from(<zip::read::ZipFile<'_> as Into<bytes::Bytes>>::into(archive.by_index(0)?));
            let mut inner_archive = archive.by_index(0)?;
            let mut inner_bytes = Vec::new();

            inner_archive.read_to_end(&mut inner_bytes)?;

            let inner_buffer = Cursor::<bytes::Bytes>::new(bytes::Bytes::from(inner_bytes));
            drop(inner_archive);
            archive = ZipArchive::new(inner_buffer)?;
        }

        match download {
            DownloadOperation::Ninja => archive.extract(final_path)?,
            DownloadOperation::CMake => {
                let mut file = File::create(if cfg!(windows) {
                    final_path.join("cmake").with_extension("exe")
                } else {
                    final_path.join("cmake")
                })?;

                let name = archive
                    .file_names()
                    .find(|i| {
                        if cfg!(windows) {
                            i.ends_with("cmake.exe")
                        } else {
                            i.ends_with("cmake")
                        }
                    })
                    .map(|s| s.to_string());

                if name.is_none() {
                    bail!("Unable to find cmake binary in archive");
                }

                let mut cmake_bin = archive.by_name(name.unwrap().as_str())?; // 2nd file /cmake-wehauehw/bin/cmake.exe

                copy(&mut cmake_bin, &mut file)?;
            }
        }

        println!(
            "Sucessfully downloaded and extracted {:?} into {:?}",
            download.cyan(),
            &final_path.file_path_color()
        );

        Ok(())
    }
}
