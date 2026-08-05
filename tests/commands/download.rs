use color_eyre::Result;
use std::path::Path;

use crate::common;

#[test]
fn test_download_adb() -> Result<()> {
    let adb_name = if cfg!(windows) { "adb.exe" } else { "adb" };

    common::test_command_check_files(
        &["download", "adb"],
        Path::new("test_cmd/_dumb"),
        &["platform-tools", &format!("platform-tools/{adb_name}")],
    )?;

    Ok(())
}
