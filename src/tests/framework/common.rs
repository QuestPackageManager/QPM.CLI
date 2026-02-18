use assert_cmd::Command;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use color_eyre::eyre::Context;
use color_eyre::eyre::ensure;
use fs_extra::dir::{self, CopyOptions};
use gix::bstr::ByteSlice;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

/// Single test function that uses assert_fs and fs_extra to test a command
pub fn test_command(
    args: &[&str],
    input_dir: &Path,
    expected_dir: &Path,
) -> color_eyre::Result<TempDir> {
    // Create a temporary directory using assert_fs
    let temp = TempDir::new().wrap_err("Failed to create temporary directory")?;

    // Copy input directory to temp directory using fs_extra
    // Use options that preserve line endings and binary content exactly
    let copy_options = CopyOptions::new()
        .overwrite(true)
        .content_only(true)
        .copy_inside(true); // Ensures directory structure is maintained

    dir::copy(input_dir, temp.path(), &copy_options)
        .wrap_err_with(|| format!("Failed to copy from {:?} to {:?}", input_dir, temp.path()))?;

    // Run the command using assert_cmd
    Command::cargo_bin("qpm2")
        .wrap_err("Failed to find qpm binary")?
        .args(args)
        .current_dir(temp.path())
        .env("QPM_DISABLE_GLOBAL_CONFIG", "1") // Set test environment variable to disable global config
        .assert()
        .success();

    // Check if we should update expected output
    if std::env::var_os("QPM_TEST_UPDATE").is_some_and(|v| v == "1") {
        println!("Updating expected output for args: {args:?}");
        if expected_dir.exists() {
            fs::remove_dir_all(expected_dir)
                .wrap_err_with(|| format!("Failed to remove expected dir: {expected_dir:?}"))?;
        }
        fs::create_dir_all(expected_dir)
            .wrap_err_with(|| format!("Failed to create expected dir: {expected_dir:?}"))?;
        dir::copy(temp.path(), expected_dir, &copy_options).wrap_err_with(|| {
            format!(
                "Failed to copy from {:?} to {:?}",
                temp.path(),
                expected_dir
            )
        })?;
        return Ok(temp);
    }

    // Compare the output directory with the expected directory
    assert_directory_equal(expected_dir, &temp)
        .wrap_err_with(|| format!("Args {args:?} content directory did not match"))?;

    Ok(temp)
}

/// Function to check for specific output files without comparing content
pub fn test_command_check_files(
    args: &[&str],
    input_dir: &Path,
    files_to_check: &[&str],
) -> color_eyre::Result<TempDir> {
    // Create a temporary directory
    let temp = TempDir::new().wrap_err("Failed to create temporary directory")?;

    // Copy input directory to temp directory using fs_extra
    // Use options that preserve line endings and binary content exactly
    let copy_options = CopyOptions::new()
        .overwrite(true)
        .content_only(true)
        .copy_inside(true); // Ensures directory structure is maintained

    dir::copy(input_dir, temp.path(), &copy_options)
        .wrap_err_with(|| format!("Failed to copy from {:?} to {:?}", input_dir, temp.path()))?;

    // Run the command
    Command::cargo_bin("qpm2")
        .wrap_err("Failed to find qpm binary")?
        .args(args)
        .current_dir(temp.path())
        .env("QPM_DISABLE_GLOBAL_CONFIG", "1") // Set test environment variable to disable global config
        .assert()
        .success();

    // Check that the specified files exist using assert_fs predicates
    for file in files_to_check {
        temp.child(file).assert(predicates::path::exists());
    }

    Ok(temp)
}

/// Compare two directories to ensure they match
pub fn assert_directory_equal(expected: &Path, actual: &TempDir) -> color_eyre::Result<()> {
    actual.assert(predicate::path::is_dir());

    // Use walkdir to recursively walk through the expected directory
    for entry in walkdir::WalkDir::new(expected)
        .min_depth(1)
        .contents_first(true)
        .into_iter()
        .filter_entry(|e| e.file_type().is_file())
    {
        let entry = entry.wrap_err("Failed to read directory entry")?;
        // Only compare files (not directories)
        if !entry.file_type().is_file() {
            continue;
        }

        let rel_path = entry
            .path()
            .strip_prefix(expected)
            .wrap_err_with(|| format!("Failed to get relative path for {:?}", entry.path()))?;
        let actual_path = actual.join(rel_path);

        // Skip if entry doesn't exist in actual directory
        ensure!(
            actual_path.exists(),
            "Path {rel_path:?} does not exist in actual directory"
        );

        // Read file contents as bytes to handle non-UTF8 content
        let mut expected_content = fs::read(entry.path())
            .wrap_err_with(|| format!("Failed to read expected file: {:?}", entry.path()))?;
        let mut actual_content = fs::read(&actual_path)
            .wrap_err_with(|| format!("Failed to read actual file: {actual_path:?}"))?;

        // Normalize line endings in text files to ensure platform-independent comparison
        // Convert all line endings to \n for comparison
        expected_content = normalize_line_endings(expected_content);
        actual_content = normalize_line_endings(actual_content);

        // Helper function to normalize line endings to \n
        fn normalize_line_endings(content: Vec<u8>) -> Vec<u8> {
            // if not windows, just return the content
            if cfg!(not(windows)) {
                return content;
            }

            content.replace(b"\r\n", "\n").replace(b"\r", b"\n")
            // let mut normalized = Vec::with_capacity(content.len());
            // let mut i = 0;
            // while i < content.len() {
            //     if content[i] == b'\r' && i + 1 < content.len() && content[i + 1] == b'\n' {
            //         // Replace CRLF with LF
            //         normalized.push(b'\n');
            //         i += 2;
            //     } else if content[i] == b'\r' {
            //         // Replace CR with LF
            //         normalized.push(b'\n');
            //         i += 1;
            //     } else {
            //         normalized.push(content[i]);
            //         i += 1;
            //     }
            // }
            // normalized
        }

        ensure!(
            expected_content == actual_content,
            "File {rel_path:?} does not match expected file at {:?}.",
            entry.path()
        );
    }

    Ok(())
}
