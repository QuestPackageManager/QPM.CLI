use color_eyre::Result;
use serde::{Deserialize, Serialize};

use super::agent::{self};

const GITHUB_OWNER: &str = "QuestPackageManager";
const GITHUB_REPO: &str = "QPM.CLI";
const GITHUB_ACTION: &str = "cargo-build";

#[cfg(windows)]
const GITHUB_ARTIFACT_NAME: &str = "windows-qpm.exe";

#[cfg(target_os = "linux")]
const GITHUB_ARTIFACT_NAME: &str = "linux-qpm";

#[cfg(target_os = "macos")]
const GITHUB_ARTIFACT_NAME: &str = "macos-qpm";

#[derive(Serialize, Deserialize)]
pub struct GithubBranchResponse {
    pub name: String,
    pub commit: GithubBranchCommitResponse,
}

#[derive(Serialize, Deserialize)]
pub struct GithubBranchCommitResponse {
    pub sha: String,
}

#[derive(Serialize, Deserialize)]
pub struct GithubCommitDiffResponse {
    pub ahead_by: i32,
    pub behind_by: i32,
    pub total_commits: i32,
    pub status: String,
    pub commits: Vec<GithubCommitDiffCommitResponse>,
}

#[derive(Serialize, Deserialize)]
pub struct GithubCommitDiffCommitResponse {
    pub commit: GithubCommitDiffCommitDataResponse,
}

#[derive(Serialize, Deserialize)]
pub struct GithubCommitDiffCommitDataResponse {
    pub message: String,
}

pub fn get_github_branch(branch: &str) -> Result<GithubBranchResponse, agent::Error> {
    agent::get(&format!(
        "https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/branches/{branch}"
    ))
}
pub fn get_github_commit_diff(
    old: &str,
    new: &str,
) -> Result<GithubCommitDiffResponse, agent::Error> {
    agent::get(&format!(
        "https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/compare/{old}...{new}"
    ))
}

pub fn download_github_artifact_url(sha: &str) -> String {
    format!(
        "https://nightly.link/{GITHUB_OWNER}/{GITHUB_REPO}/workflows/{GITHUB_ACTION}/{sha}/{GITHUB_ARTIFACT_NAME}.zip"
    )
}

pub fn nightly_github_artifact_url() -> String {
    #[cfg(windows)]
    return "https://github.com/QuestPackageManager/QPM.CLI/releases/download/bleeding/qpm-windows-x64.zip".to_string();

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "https://github.com/QuestPackageManager/QPM.CLI/releases/download/bleeding/qpm-macos-x64.zip".to_string();

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "https://github.com/QuestPackageManager/QPM.CLI/releases/download/bleeding/qpm-macos-arm64.zip".to_string();

    #[cfg(target_os = "linux")]
    return "https://github.com/QuestPackageManager/QPM.CLI/releases/download/bleeding/qpm-linux-x64.zip".to_string();
}
