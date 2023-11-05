
use serde::{Deserialize, Serialize};

use super::agent::get_agent;

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

pub fn get_github_branch(branch: &str) -> color_eyre::Result<GithubBranchResponse> {
    let res = get_agent()
        .get(format!(
            "https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/branches/{branch}"
        ).as_str())
        .call()?
        .into_json()?;
    Ok(res)
}
pub fn get_github_commit_diff(
    old: &str,
    new: &str,
) -> color_eyre::Result<GithubCommitDiffResponse> {
    let res = get_agent()
        .get(format!(
            "https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/compare/{old}...{new}"
        ).as_str())
        .call()?
        .into_json()?;
    Ok(res)
}

pub fn download_github_artifact_url(sha: &str) -> String {
    format!(
            "https://nightly.link/{GITHUB_OWNER}/{GITHUB_REPO}/workflows/{GITHUB_ACTION}/{sha}/{GITHUB_ARTIFACT_NAME}.zip"
        )
}
