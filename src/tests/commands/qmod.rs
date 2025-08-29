use crate::tests::framework::common;
use color_eyre::eyre::Result;
use std::path::Path;

#[test]
fn test_qmod_manifest() -> Result<()> {
    common::test_command(
        &["qmod", "manifest"],
        Path::new("test_cmd/qmod_manifest.in"),
        Path::new("test_cmd/qmod_manifest.out"),
    )?;

    Ok(())
}
#[test]
fn test_qmod_zip() -> Result<()> {
    // For qmod_zip, we only check that the output file exists, not compare directories
    common::test_command_check_files(
        &["qmod", "zip", "MyTestMod.qmod"], // Specify output filename
        Path::new("test_cmd/qmod_zip.in"),
        &["MyTestMod.qmod"],
    )?;
    Ok(())
}
