#![cfg(feature = "network_test")]

use qpm_cli::services::qpm_version::{UpdateTarget, resolve_update_url};

/// Hits the real GitHub branches API for `main` - either this build's commit already matches
/// the tip of `main` (AlreadyLatest), or it resolves to a download URL naming that branch.
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
