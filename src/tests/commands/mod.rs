use std::path::{Path, PathBuf};

use color_eyre::eyre::Result;

use crate::models::config::UserConfig;

#[test]
fn trycmd() {
    trycmd::TestCases::new()
        .case("README.md")
        .case("test_cmd/*.toml")
        .case("test_cmd/*.trycmd");
}

///
/// Setup the environment for local isolated cache tests
/// 
fn setup_qpm() -> Result<PathBuf> {
    let s: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();

    let random_dir = PathBuf::from(env!("CARGO").to_string())
        .join("target")
        .join("tests")
        .join(format!("test-package-{s}"));

    let config = UserConfig {
        cache: Some(random_dir.join("cache").to_path_buf()),
        timeout: None,
        symlink: None,
        ndk_download_path: Some(random_dir.join("ndk").to_path_buf()),
    };

    config.write(random_dir.join(UserConfig::config_file_name()))?;
}
