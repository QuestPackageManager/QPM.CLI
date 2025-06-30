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
        )?;
        Ok(())
    }
}
