use crate::common;
use color_eyre::eyre::Result;
use std::path::Path;

// These use libil2cpp because it's what the qpm2 backend currently serves.

#[test]
fn test_dependency_add() -> Result<()> {
    common::test_command(
        &["dependency", "add", "libil2cpp", "--version", "^0.5.0"],
        Path::new("test_cmd/dep_add.in"),
        Path::new("test_cmd/dep_add.out"),
    )?;
    Ok(())
}

#[test]
fn test_dependency_remove() -> Result<()> {
    common::test_command(
        &["dependency", "remove", "libil2cpp"],
        Path::new("test_cmd/dep_remove.in"),
        Path::new("test_cmd/dep_remove.out"),
    )?;
    Ok(())
}

/// There is no `update` command; adding an existing dependency replaces its range
#[test]
fn test_dependency_update() -> Result<()> {
    common::test_command(
        &["dependency", "add", "libil2cpp", "--version", "^0.5.0"],
        Path::new("test_cmd/dep_update.in"),
        Path::new("test_cmd/dep_update.out"),
    )?;
    Ok(())
}

/// Downloads into the fixture's own cache (`./qpm_junk/cache`), not the user's
#[test]
fn test_dependency_download_specific() -> Result<()> {
    let out = common::test_command_check_files(
        &["dependency", "download", "libil2cpp", "--version", "0.5.0"],
        Path::new("test_cmd/dep_download_specific.in"),
        &["qpm_junk/cache/libil2cpp/0.5.0/src"],
    )?;

    assert!(
        out.path()
            .join("qpm_junk/cache/qpm.repository.json")
            .exists()
    );
    Ok(())
}

#[test]
fn test_dependency_download_recursive() -> Result<()> {
    common::test_command_check_files(
        &[
            "dependency",
            "download",
            "libil2cpp",
            "--version",
            "0.5.0",
            "--recursive",
        ],
        Path::new("test_cmd/dep_download_recursive.in"),
        &["qpm_junk/cache/libil2cpp/0.5.0/src"],
    )?;
    Ok(())
}
