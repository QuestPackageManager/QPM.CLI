use crate::tests::framework::common;
use color_eyre::eyre::Result;
use std::path::Path;

#[test]
fn test_dependency_add() -> Result<()> {
    common::test_command(
        &["dependency", "add", "beatsaber-hook", "--version", "5.1.9"],
        Path::new("test_cmd/dep_add.in"),
        Path::new("test_cmd/dep_add.out"),
    )?;
    Ok(())
}

#[test]
fn test_dependency_remove() -> Result<()> {
    common::test_command(
        &["dependency", "remove", "beatsaber-hook"],
        Path::new("test_cmd/dep_remove.in"),
        Path::new("test_cmd/dep_remove.out"),
    )?;
    Ok(())
}
#[test]
fn test_dependency_update() -> Result<()> {
    // The 'update' command no longer exists, use 'add' instead to update a dependency
    common::test_command(
        &["dependency", "add", "beatsaber-hook", "--version", "^5.1.9"],
        Path::new("test_cmd/dep_update.in"),
        Path::new("test_cmd/dep_update.out"),
    )?;
    Ok(())
}

#[test]
fn test_dependency_download_recursive() -> color_eyre::Result<()> {
    let out = common::test_command(
        &[
            "dependency",
            "download",
            "beatsaber-hook",
            "--version",
            "5.1.9",
            "--recursive",
        ],
        Path::new("test_cmd/dep_download_recursive.in"),
        Path::new("test_cmd/dep_download_recursive.out"),
    )?;

    let qpm_junk_path = out.join("qpm_junk/cache");
    assert!(qpm_junk_path.exists(), "qpm_junk directory does not exist");

    // Check that specific dependency folders exist
    let libil2cpp_path = qpm_junk_path.join("libil2cpp");
    let beatsaber_hook_path = qpm_junk_path.join("beatsaber-hook");
    assert!(
        libil2cpp_path.exists(),
        "libil2cpp directory does not exist"
    );
    assert!(
        beatsaber_hook_path.exists(),
        "beatsaber-hook directory does not exist"
    );

    Ok(())
}

#[test]
fn test_dependency_download_specific() -> color_eyre::Result<()> {
    let out = common::test_command(
        &[
            "dependency",
            "download",
            "beatsaber-hook",
            "--version",
            "5.1.9",
        ],
        Path::new("test_cmd/dep_download_specific.in"),
        Path::new("test_cmd/dep_download_specific.out"),
    )?;

    let qpm_junk_path = out.join("qpm_junk/cache");
    assert!(qpm_junk_path.exists(), "qpm_junk directory does not exist");

    // Check that specific dependency folders exist
    let beatsaber_hook_path = qpm_junk_path.join("beatsaber-hook");
    assert!(
        beatsaber_hook_path.exists(),
        "beatsaber-hook directory does not exist"
    );

    Ok(())
}

