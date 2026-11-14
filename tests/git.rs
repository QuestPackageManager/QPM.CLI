use std::{fs, process::Command};

use qpm_cli::services::git::{check_git, clone};
use tempfile::tempdir;

#[test]
fn check_git_succeeds_when_git_is_on_path() {
    check_git().expect("git must be installed and on PATH to run this test suite");
}

/// Exercises the real `git clone` subprocess against a local repository (no network needed):
/// creates a real git repo on disk with a commit, clones it through `services::git::clone`,
/// and verifies the checked-out working tree actually contains the committed file.
#[test]
fn clone_checks_out_a_real_local_repository() {
    check_git().expect("git must be installed and on PATH to run this test suite");

    let tmp = tempdir().unwrap();

    // Named `source.git` on disk so that `clone()`'s unconditional `.git` suffix, appended to
    // the url `.../source`, resolves back to this exact path.
    let repo_dir = tmp.path().join("source.git");
    fs::create_dir_all(&repo_dir).unwrap();

    let run_git = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(&repo_dir)
            .status()
            .expect("failed to run git");
        assert!(status.success(), "git {args:?} failed");
    };

    run_git(&["init", "--initial-branch=main"]);
    run_git(&["config", "user.email", "test@example.com"]);
    run_git(&["config", "user.name", "Test"]);
    fs::write(repo_dir.join("README.md"), "hello from the test repo").unwrap();
    run_git(&["add", "README.md"]);
    run_git(&["commit", "-m", "initial commit"]);

    let url = tmp.path().join("source").to_str().unwrap().to_string();
    let out_dir = tmp.path().join("out");

    let cloned = clone(url, None, &out_dir, None).expect("clone should succeed");

    assert!(cloned);
    assert!(out_dir.join("README.md").exists());
    assert_eq!(
        fs::read_to_string(out_dir.join("README.md")).unwrap(),
        "hello from the test repo"
    );
}
