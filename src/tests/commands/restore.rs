use crate::tests::framework::common;
use color_eyre::eyre::Result;
use std::path::Path;

#[test]
fn test_restore() -> Result<()> {
    common::test_command(
        &["restore"],
        Path::new("test_cmd/restore.in"),
        Path::new("test_cmd/restore.out"),
    )?;
    Ok(())
}
