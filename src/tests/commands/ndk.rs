use crate::tests::framework::common;
use color_eyre::eyre::Result;
use std::path::Path;
#[test]
fn test_ndk_download() -> Result<()> {
    common::test_command(
        &["ndk", "download", "25.2.9519653"],
        Path::new("test_cmd/ndk_download.in"),
        Path::new("test_cmd/ndk_download.out"),
    )?;

    Ok(())
}
#[test]
fn test_ndk_pin() -> Result<()> {
    common::test_command(
        &["ndk", "pin", "26", "--online"],
        Path::new("test_cmd/ndk_pin.in"),
        Path::new("test_cmd/ndk_pin.out"),
    )?;

    Ok(())
}
#[test]
fn test_ndk_resolve() -> Result<()> {
    common::test_command(
        &["ndk", "resolve", "-d"], // Add download flag to avoid failure
        Path::new("test_cmd/ndk_resolve.in"),
        Path::new("test_cmd/ndk_resolve.out"),
    )?;

    Ok(())
}
