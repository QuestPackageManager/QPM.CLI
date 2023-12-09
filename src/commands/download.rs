use std::io::Cursor;

use bytes::Bytes;
use clap::{Args, Subcommand};
use color_eyre::Result;
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
const ADB_DOWNLOAD: &str =
    "https://dl.google.com/android/repository/platform-tools-latest-windows.zip";

#[cfg(target_os = "linux")]
const ADB_DOWNLOAD: &str =
    "https://dl.google.com/android/repository/platform-tools-latest-linux.zip";

#[cfg(target_os = "macos")]
const ADB_DOWNLOAD: &str =
    "https://dl.google.com/android/repository/platform-tools-latest-darwin.zip";

#[derive(Args, Debug, Clone)]
pub struct Download {
    #[clap(subcommand)]
    pub op: DownloadOperation,
}

#[derive(Subcommand, Debug, Clone, PartialEq, PartialOrd)]
pub enum DownloadOperation {
    Ninja,
    ADB,
}

impl Command for Download {
    fn execute(self) -> Result<()> {
        let download = self.op;

        let url = match download {
            DownloadOperation::Ninja => NINJA_DOWNLOAD,
            DownloadOperation::ADB => ADB_DOWNLOAD,
        };

        let exe = std::env::current_exe()?;
        let final_path = exe.parent().unwrap();

        let bytes: Bytes = download_file_report(url, |_, _| {})?.into();
        let buffer = Cursor::new(bytes);

        // Extract to tmp folde
        let mut archive = ZipArchive::new(buffer)?;

        // if download == DownloadOperation::ADB && archive.len() == 1 {
        //     // Extract to tmp folder
        //     // let inner_bytes = bytes::Bytes::from(<zip::read::ZipFile<'_> as Into<bytes::Bytes>>::into(archive.by_index(0)?));
        //     let mut inner_archive = archive.by_index(0)?;
        //     let mut inner_bytes = Vec::new();

        //     inner_archive.read_to_end(&mut inner_bytes)?;

        //     let inner_buffer = Cursor::<bytes::Bytes>::new(bytes::Bytes::from(inner_bytes));
        //     drop(inner_archive);
        //     archive = ZipArchive::new(inner_buffer)?;
        // }

        match download {
            DownloadOperation::Ninja => archive.extract(final_path)?,
            DownloadOperation::ADB => archive.extract(final_path)?,
        }

        println!(
            "Sucessfully downloaded and extracted {:?} into {:?}",
            download.cyan(),
            &final_path.file_path_color()
        );

        Ok(())
    }
}
