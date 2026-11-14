use assert_fs::TempDir;
use assert_fs::prelude::*;
use bstr::ByteSlice;
use color_eyre::eyre::{Context, ensure};
use fs_extra::dir::{self, CopyOptions};
use predicates::prelude::*;
use qpm_cli::models::config::UserConfig;
use std::fs;
use std::path::Path;

use crate::support;

/// The fixture's own `qpm.settings.json` over the defaults, never the user's global settings.
/// Fixtures point the cache at `./qpm_junk`, inside the temp dir.
fn fixture_config(dir: &Path) -> color_eyre::Result<UserConfig> {
    let path = dir.join(UserConfig::config_file_name());
    if !path.exists() {
        return Ok(UserConfig::default());
    }
    let workspace: UserConfig = serde_json::from_slice(&fs::read(&path)?)
        .with_context(|| format!("Failed to read fixture settings {path:?}"))?;
    Ok(workspace.over(UserConfig::default()))
}

/// Single test function that uses assert_fs and fs_extra to test a command
pub fn test_command(
    args: &[&str],
    input_dir: &Path,
    expected_dir: &Path,
) -> color_eyre::Result<TempDir> {
    // held for the whole test: fixture copying and comparison also use relative paths
    let _guard = support::lock_cwd();

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

    // Run the command in-process against the temp directory
    support::run_qpm_unlocked(&fixture_config(temp.path())?, temp.path(), args)
        .wrap_err_with(|| format!("Command {args:?} failed"))?;

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
    // held for the whole test: fixture copying and comparison also use relative paths
    let _guard = support::lock_cwd();

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

    // Run the command in-process against the temp directory
    support::run_qpm_unlocked(&fixture_config(temp.path())?, temp.path(), args)
        .wrap_err_with(|| format!("Command {args:?} failed"))?;

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

        // Binary artifacts (.so, .a, zipped .qmod, ...) must be compared byte-for-byte;
        // CRLF/path-separator normalization is only meaningful for the text files this
        // suite generates (qpm.json, ndkpath.txt, ...) and would silently corrupt the
        // comparison if applied to arbitrary binary content that happens to contain the
        // same byte sequences.
        if !is_binary(&expected_content) && !is_binary(&actual_content) {
            // Normalize line endings and path separators to ensure platform-independent
            // comparison against fixtures committed in canonical (LF, forward-slash) form
            expected_content = normalize_line_endings(expected_content);
            actual_content = normalize_line_endings(actual_content);
            actual_content = normalize_path_separators(actual_content);
        }

        // Same heuristic git uses: a NUL byte in the first few KB means binary content.
        fn is_binary(content: &[u8]) -> bool {
            content[..content.len().min(8000)].contains(&0)
        }

        // Helper function to normalize line endings to \n
        fn normalize_line_endings(content: Vec<u8>) -> Vec<u8> {
            // if not windows, just return the content
            if cfg!(not(windows)) {
                return content;
            }

            content.replace(b"\r\n", "\n").replace(b"\r", b"\n")
        }

        // Commands like `ndk pin` write OS-native paths (e.g. into ndkpath.txt) into
        // output files; normalize backslashes so Windows output matches the
        // forward-slash fixtures committed for every platform.
        fn normalize_path_separators(content: Vec<u8>) -> Vec<u8> {
            if cfg!(not(windows)) {
                return content;
            }

            content.replace(b"\\", "/")
        }

        ensure!(
            expected_content == actual_content,
            "File {rel_path:?} does not match expected file at {:?}.",
            entry.path()
        );
    }

    Ok(())
}
