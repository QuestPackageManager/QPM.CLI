#![cfg(feature = "network_test")]

use qpm_cli::services::github::{
    bleeding_release_github_artifact_url, download_github_artifact_url, get_github_branch,
    get_github_commit_diff,
};
use qpm_cli::services::network::get_agent;

#[test]
fn get_github_branch_main_is_reachable() {
    let branch = get_github_branch("main").expect("GET branches/main should succeed");

    assert_eq!(branch.name, "main");
    assert!(!branch.commit.sha.is_empty());
}

#[test]
fn get_github_commit_diff_same_ref_is_reachable_and_identical() {
    let diff =
        get_github_commit_diff("main", "main").expect("GET compare/main...main should succeed");

    assert_eq!(diff.status, "identical");
    assert_eq!(diff.ahead_by, 0);
    assert_eq!(diff.behind_by, 0);
}

/// The nightly.link artifact URL for whatever commit is currently at the tip of `main` should
/// resolve to a real artifact, not a 404 - catches the workflow/artifact name constants in
/// `services::github` drifting out of sync with the actual GitHub Actions workflow.
#[test]
#[ignore = "depends on live GitHub Actions artifact state for the tip of main; skip by default"]
fn download_github_artifact_url_for_latest_main_commit_does_not_404() {
    let branch = get_github_branch("main").expect("GET branches/main should succeed");
    let url = download_github_artifact_url(&branch.commit.sha);

    let status = get_agent()
        .head(&url)
        .call()
        .expect("request should complete")
        .status();

    assert_ne!(status.as_u16(), 404, "artifact url {url} returned 404");
}

/// The `bleeding` GitHub release must always carry an asset for whatever OS this test is
/// compiled for - it's the fallback `qpm2 update` target when no branch is specified.
#[test]
#[ignore = "depends on live GitHub release asset state; skip by default"]
fn bleeding_release_github_artifact_url_does_not_404() {
    let url = bleeding_release_github_artifact_url();

    let status = get_agent()
        .head(&url)
        .call()
        .expect("request should complete")
        .status();

    assert_ne!(status.as_u16(), 404, "release asset url {url} returned 404");
}
