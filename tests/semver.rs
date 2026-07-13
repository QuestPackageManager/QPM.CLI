use qpm_cli::resolver::semver::{VersionWrapper, req_to_range};
use semver::{Version, VersionReq};

fn range_contains(req: &str, version: &str) -> bool {
    let range = req_to_range(VersionReq::parse(req).unwrap());
    range.contains(&VersionWrapper(Version::parse(version).unwrap()))
}

/// `=1.2.3` (explicit exact operator) should match only that exact version.
#[test]
fn exact_matches_only_that_version() {
    assert!(range_contains("=1.2.3", "1.2.3"));
    assert!(!range_contains("=1.2.3", "1.2.4"));
    assert!(!range_contains("=1.2.3", "1.2.2"));
}

/// A bare version requirement (no operator) defaults to caret behavior, per the `semver`
/// crate's Cargo-compatible convention - "1.2.3" means the same thing as "^1.2.3".
#[test]
fn bare_version_defaults_to_caret() {
    assert!(range_contains("1.2.3", "1.2.3"));
    assert!(range_contains("1.2.3", "1.9.9"));
    assert!(!range_contains("1.2.3", "1.2.2"));
    assert!(!range_contains("1.2.3", "2.0.0"));
}

/// `^1.2.3` should allow minor/patch bumps within major version 1, but reject anything
/// below 1.2.3 or at/above 2.0.0.
#[test]
fn caret_allows_minor_and_patch_bumps_but_not_major() {
    assert!(range_contains("^1.2.3", "1.2.3"));
    assert!(range_contains("^1.2.3", "1.9.9"));
    assert!(!range_contains("^1.2.3", "1.2.2"));
    assert!(!range_contains("^1.2.3", "2.0.0"));
}

/// Caret is stricter for 0.x releases: `^0.5.0` should behave like `>=0.5.0, <0.6.0`
/// (only patch bumps allowed, no minor bumps), per semver's pre-1.0 instability convention.
#[test]
fn caret_zero_major_only_allows_patch_bumps() {
    assert!(range_contains("^0.5.0", "0.5.0"));
    assert!(range_contains("^0.5.9", "0.5.9"));
    assert!(!range_contains("^0.5.0", "0.6.0"));
    assert!(!range_contains("^0.5.0", "0.4.9"));
}

/// Caret is strictest for 0.0.x releases: `^0.0.3` should match only 0.0.3 exactly, since
/// every component is considered potentially breaking below 0.1.0.
#[test]
fn caret_zero_zero_major_minor_is_exact() {
    assert!(range_contains("^0.0.3", "0.0.3"));
    assert!(!range_contains("^0.0.3", "0.0.4"));
    assert!(!range_contains("^0.0.3", "0.1.0"));
}

/// `~1.2.3` should allow patch bumps only (not minor), unlike caret.
#[test]
fn tilde_allows_patch_bumps_only() {
    assert!(range_contains("~1.2.3", "1.2.3"));
    assert!(range_contains("~1.2.3", "1.2.9"));
    assert!(!range_contains("~1.2.3", "1.3.0"));
    assert!(!range_contains("~1.2.3", "1.2.2"));
}

/// `>=`, `>`, `<`, `<=` should behave as ordinary inequality comparisons against the version.
#[test]
fn comparison_operators() {
    assert!(range_contains(">=1.2.3", "1.2.3"));
    assert!(range_contains(">=1.2.3", "99.0.0"));
    assert!(!range_contains(">=1.2.3", "1.2.2"));

    assert!(range_contains(">1.2.3", "1.2.4"));
    assert!(!range_contains(">1.2.3", "1.2.3"));

    assert!(range_contains("<2.0.0", "1.9.9"));
    assert!(!range_contains("<2.0.0", "2.0.0"));

    assert!(range_contains("<=2.0.0", "2.0.0"));
    assert!(!range_contains("<=2.0.0", "2.0.1"));
}

/// `1.2.*` should behave like `1.2.x` (any patch within that minor version).
#[test]
fn wildcard() {
    assert!(range_contains("1.2.*", "1.2.5"));
    assert!(!range_contains("1.2.*", "1.3.0"));
}

/// A comma-separated requirement (e.g. `>=1.0.0, <1.5.0`) should intersect all of its
/// comparators, not just apply the last one.
#[test]
fn multiple_comparators_intersect() {
    assert!(range_contains(">=1.0.0, <1.5.0", "1.0.0"));
    assert!(range_contains(">=1.0.0, <1.5.0", "1.4.9"));
    assert!(!range_contains(">=1.0.0, <1.5.0", "1.5.0"));
    assert!(!range_contains(">=1.0.0, <1.5.0", "0.9.9"));
}
