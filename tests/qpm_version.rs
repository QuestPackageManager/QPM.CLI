use qpm_cli::services::github::bleeding_release_github_artifact_url;
use qpm_cli::services::qpm_version::{UpdateTarget, current_version, resolve_update_url};

#[test]
fn current_version_reports_nonempty_branch_and_commit() {
    let version = current_version();

    assert!(!version.branch.is_empty());
    assert!(!version.commit.is_empty());
}

/// `resolve_update_url(None)` never touches the network - it should deterministically resolve
/// to the same URL `bleeding_release_github_artifact_url` produces directly.
#[test]
fn resolve_update_url_without_branch_points_at_bleeding_release() {
    let target =
        resolve_update_url(None).expect("no network call needed for the bleeding release path");

    match target {
        UpdateTarget::DownloadUrl(url) => assert_eq!(url, bleeding_release_github_artifact_url()),
        UpdateTarget::AlreadyLatest => panic!("expected a download url, not AlreadyLatest"),
    }
}

/// Hits the real GitHub branches API for `main` - either this build's commit already matches
/// the tip of `main` (AlreadyLatest), or it resolves to a download URL naming that branch.
#[cfg(feature = "network_test")]
#[test]
fn resolve_update_url_with_branch_hits_the_real_github_api() {
    let target = resolve_update_url(Some("main")).expect("GET branches/main should succeed");

    match target {
        UpdateTarget::AlreadyLatest => {}
        UpdateTarget::DownloadUrl(url) => {
            assert!(
                url.contains("main"),
                "expected the artifact url to reference the branch: {url}"
            );
        }
    }
}
