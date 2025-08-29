use assert_fs::TempDir;
use color_eyre::{Result, eyre::Context};
use fs_extra::dir::{self, CopyOptions};
use std::path::Path;

use crate::tests::framework::common;

#[test]
fn test_download_adb() -> Result<()> {
    let adb_name = if cfg!(windows) { "adb.exe" } else { "adb" };

    let temp = common::test_command_check_files(
        &["download", "adb"],
        Path::new("test_cmd/_dumb"),
        &["platform-tools", &format!("platform-tools/{adb_name}")],
    )?;

    Ok(())
}
