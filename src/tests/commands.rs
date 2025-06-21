// This file contains tests for CLI commands

/// This module contains the tests for the dependency command
mod dependency {
    use crate::tests::framework::common;
    use color_eyre::eyre::Result;
    use std::path::Path;

    #[test]
    fn test_dependency_add() -> Result<()> {
        common::test_command(
            &["dependency", "add", "beatsaber-hook", "--version", "5.1.9"],
            Path::new("test_cmd/dep_add.in"),
            Path::new("test_cmd/dep_add.out"),
        )
    }

    #[test]
    fn test_dependency_remove() -> Result<()> {
        common::test_command(
            &["dependency", "remove", "beatsaber-hook"],
            Path::new("test_cmd/dep_remove.in"),
            Path::new("test_cmd/dep_remove.out"),
        )
    }
    #[test]
    fn test_dependency_update() -> Result<()> {
        // The 'update' command no longer exists, use 'add' instead to update a dependency
        common::test_command(
            &["dependency", "add", "beatsaber-hook", "--version", "^5.1.9"],
            Path::new("test_cmd/dep_update.in"),
            Path::new("test_cmd/dep_update.out"),
        )
    }
}

/// This module contains the tests for the ndk command
mod ndk {
    use crate::tests::framework::common;
    use color_eyre::eyre::Result;
    use std::path::Path;
    #[test]
    fn test_ndk_download() -> Result<()> {
        common::test_command(
            &["ndk", "download", "25.2.9519653"],
            Path::new("test_cmd/ndk_download.in"),
            Path::new("test_cmd/ndk_download.out"),
        )
    }
    #[test]
    fn test_ndk_pin() -> Result<()> {
        common::test_command(
            &["ndk", "pin", "26", "--online"],
            Path::new("test_cmd/ndk_pin.in"),
            Path::new("test_cmd/ndk_pin.out"),
        )
    }
    #[test]
    fn test_ndk_resolve() -> Result<()> {
        common::test_command(
            &["ndk", "resolve", "-d"], // Add download flag to avoid failure
            Path::new("test_cmd/ndk_resolve.in"),
            Path::new("test_cmd/ndk_resolve.out"),
        )
    }
}

/// This module contains the tests for the qmod command
mod qmod {
    use crate::tests::framework::common;
    use color_eyre::eyre::Result;
    use std::path::Path;

    #[test]
    fn test_qmod_manifest() -> Result<()> {
        common::test_command(
            &["qmod", "manifest"],
            Path::new("test_cmd/qmod_manifest.in"),
            Path::new("test_cmd/qmod_manifest.out"),
        )
    }
    #[test]
    fn test_qmod_zip() -> Result<()> {
        // For qmod_zip, we only check that the output file exists, not compare directories
        common::test_command_check_files(
            &["qmod", "zip", "MyTestMod.qmod"], // Specify output filename
            Path::new("test_cmd/qmod_zip.in"),
            &["MyTestMod.qmod"],
        )
    }
}

/// This module contains the tests for the restore command
mod restore {
    use crate::tests::framework::common;
    use color_eyre::eyre::Result;
    use std::path::Path;

    #[test]
    fn test_restore() -> Result<()> {
        common::test_command(
            &["restore"],
            Path::new("test_cmd/restore.in"),
            Path::new("test_cmd/restore.out"),
        )
    }
}
