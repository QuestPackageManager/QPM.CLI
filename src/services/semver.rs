use std::fmt;

use pubgrub::Range;
use semver::{Comparator, Op, Prerelease, VersionReq};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VersionWrapper(pub semver::Version);

pub fn req_to_range(req: VersionReq) -> Range<VersionWrapper> {
    let mut range = Range::full();
    for comparator in req.comparators {
        let next = match comparator {
            Comparator {
                op: Op::Exact,
                major,
                minor: Some(minor),
                patch: Some(patch),
                pre,
            } => exact_xyz(major, minor, patch, pre),
            Comparator {
                op: Op::Exact,
                major,
                minor: Some(minor),
                ..
            } => exact_xy(major, minor),
            Comparator {
                op: Op::Exact,
                major,
                ..
            } => exact_x(major),

            Comparator {
                op: Op::Greater,
                major,
                minor: Some(minor),
                patch: Some(patch),
                pre,
            } => greater_xyz(major, minor, patch, pre),
            Comparator {
                op: Op::Greater,
                major,
                minor: Some(minor),
                ..
            } => greater_xy(major, minor),
            Comparator {
                op: Op::Greater,
                major,
                ..
            } => greater_x(major),

            Comparator {
                op: Op::GreaterEq,
                major,
                minor: Some(minor),
                patch: Some(patch),
                pre,
            } => greater_eq_xyz(major, minor, patch, pre),
            Comparator {
                op: Op::GreaterEq,
                major,
                minor: Some(minor),
                ..
            } => greater_eq_xy(major, minor),
            Comparator {
                op: Op::GreaterEq,
                major,
                ..
            } => greater_eq_x(major),

            Comparator {
                op: Op::Less,
                major,
                minor: Some(minor),
                patch: Some(patch),
                pre,
            } => less_xyz(major, minor, patch, pre),
            Comparator {
                op: Op::Less,
                major,
                minor: Some(minor),
                ..
            } => less_xy(major, minor),
            Comparator {
                op: Op::Less,
                major,
                ..
            } => less_x(major),

            Comparator {
                op: Op::LessEq,
                major,
                minor: Some(minor),
                patch: Some(patch),
                pre,
            } => less_eq_xyz(major, minor, patch, pre),
            Comparator {
                op: Op::LessEq,
                major,
                minor: Some(minor),
                ..
            } => less_eq_xy(major, minor),
            Comparator {
                op: Op::LessEq,
                major,
                ..
            } => less_eq_x(major),

            Comparator {
                op: Op::Tilde,
                major,
                minor: Some(minor),
                patch: Some(patch),
                pre,
            } => tilde_xyz(major, minor, patch, pre),
            Comparator {
                op: Op::Tilde,
                major,
                minor: Some(minor),
                ..
            } => tilde_xy(major, minor),
            Comparator {
                op: Op::Tilde,
                major,
                ..
            } => tilde_x(major),

            Comparator {
                op: Op::Caret,
                major: 0,
                minor: Some(0),
                patch: Some(patch),
                pre,
            } => caret_00z(patch, pre),
            Comparator {
                op: Op::Caret,
                major: 0,
                minor: Some(minor),
                patch: Some(patch),
                pre,
            } => caret_0yz(minor, patch, pre),
            Comparator {
                op: Op::Caret,
                major,
                minor: Some(minor),
                patch: Some(patch),
                pre,
            } => caret_xyz(major, minor, patch, pre),
            Comparator {
                op: Op::Caret,
                major: 0,
                minor: Some(0),
                ..
            } => caret_00(),
            Comparator {
                op: Op::Caret,
                major,
                minor: Some(minor),
                ..
            } => caret_xy(major, minor),
            Comparator {
                op: Op::Caret,
                major,
                ..
            } => caret_x(major),

            Comparator {
                op: Op::Wildcard,
                major,
                minor: Some(minor),
                ..
            } => wildcard_xy(major, minor),
            Comparator {
                op: Op::Wildcard,
                major,
                ..
            } => wildcard_x(major),

            _ => unimplemented!(),
        };
        range = range.intersection(&next);
    }
    range
}

fn exact_xyz(major: u64, minor: u64, patch: u64, pre: Prerelease) -> Range<VersionWrapper> {
    Range::singleton(semver::Version {
        major,
        minor,
        patch,
        pre,
        build: Default::default(),
    })
}
fn exact_xy(major: u64, minor: u64) -> Range<VersionWrapper> {
    greater_eq_xyz(major, minor, 0, Prerelease::EMPTY).intersection(&less_xyz(
        major,
        minor + 1,
        0,
        Prerelease::EMPTY,
    ))
}
fn exact_x(major: u64) -> Range<VersionWrapper> {
    greater_eq_xyz(major, 0, 0, Prerelease::EMPTY).intersection(&less_xyz(
        major + 1,
        0,
        0,
        Prerelease::EMPTY,
    ))
}

fn greater_xyz(major: u64, minor: u64, patch: u64, pre: Prerelease) -> Range<VersionWrapper> {
    Range::strictly_higher_than(semver::Version {
        major,
        minor,
        patch,
        pre,
        build: Default::default(),
    })
}
fn greater_xy(major: u64, minor: u64) -> Range<VersionWrapper> {
    greater_eq_xyz(major, minor + 1, 0, Prerelease::EMPTY)
}
fn greater_x(major: u64) -> Range<VersionWrapper> {
    greater_eq_xyz(major + 1, 0, 0, Prerelease::EMPTY)
}

fn greater_eq_xyz(major: u64, minor: u64, patch: u64, pre: Prerelease) -> Range<VersionWrapper> {
    Range::higher_than(semver::Version {
        major,
        minor,
        patch,
        pre,
        build: Default::default(),
    })
}
fn greater_eq_xy(major: u64, minor: u64) -> Range<VersionWrapper> {
    greater_eq_xyz(major, minor, 0, Prerelease::EMPTY)
}
fn greater_eq_x(major: u64) -> Range<VersionWrapper> {
    greater_eq_xyz(major, 0, 0, Prerelease::EMPTY)
}

fn less_xyz(major: u64, minor: u64, patch: u64, pre: Prerelease) -> Range<VersionWrapper> {
    Range::strictly_lower_than(semver::Version {
        major,
        minor,
        patch,
        pre,
        build: Default::default(),
    })
}
fn less_xy(major: u64, minor: u64) -> Range<VersionWrapper> {
    less_xyz(major, minor, 0, Prerelease::EMPTY)
}
fn less_x(major: u64) -> Range<VersionWrapper> {
    less_xyz(major, 0, 0, Prerelease::EMPTY)
}

fn less_eq_xyz(major: u64, minor: u64, patch: u64, pre: Prerelease) -> Range<VersionWrapper> {
    less_xyz(major, minor, patch, pre.clone()).union(&exact_xyz(major, minor, patch, pre))
}
fn less_eq_xy(major: u64, minor: u64) -> Range<VersionWrapper> {
    less_xyz(major, minor + 1, 0, Prerelease::EMPTY)
}
fn less_eq_x(major: u64) -> Range<VersionWrapper> {
    less_xyz(major + 1, 0, 0, Prerelease::EMPTY)
}

fn tilde_xyz(major: u64, minor: u64, patch: u64, pre: Prerelease) -> Range<VersionWrapper> {
    greater_eq_xyz(major, minor, patch, pre).intersection(&less_xyz(
        major,
        minor + 1,
        0,
        Prerelease::EMPTY,
    ))
}
fn tilde_xy(major: u64, minor: u64) -> Range<VersionWrapper> {
    exact_xy(major, minor)
}
fn tilde_x(major: u64) -> Range<VersionWrapper> {
    exact_x(major)
}

fn caret_00z(patch: u64, pre: Prerelease) -> Range<VersionWrapper> {
    exact_xyz(0, 0, patch, pre)
}
fn caret_0yz(minor: u64, patch: u64, pre: Prerelease) -> Range<VersionWrapper> {
    greater_eq_xyz(0, minor, patch, pre).intersection(&less_xyz(0, minor + 1, 0, Prerelease::EMPTY))
}
fn caret_xyz(major: u64, minor: u64, patch: u64, pre: Prerelease) -> Range<VersionWrapper> {
    greater_eq_xyz(major, minor, patch, pre).intersection(&less_xyz(
        major + 1,
        0,
        0,
        Prerelease::EMPTY,
    ))
}
fn caret_00() -> Range<VersionWrapper> {
    exact_xy(0, 0)
}
fn caret_xy(major: u64, minor: u64) -> Range<VersionWrapper> {
    caret_xyz(major, minor, 0, Prerelease::EMPTY)
}
fn caret_x(major: u64) -> Range<VersionWrapper> {
    exact_x(major)
}

fn wildcard_xy(major: u64, minor: u64) -> Range<VersionWrapper> {
    exact_xy(major, minor)
}
fn wildcard_x(major: u64) -> Range<VersionWrapper> {
    exact_x(major)
}

macro_rules! impl_traits {
    ($($t:ty => $tt:ty),*) => {
        $(
            impl fmt::Debug for $t {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::Debug::fmt(&self.0, f)
                }
            }
            impl fmt::Display for $t {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    fmt::Display::fmt(&self.0, f)
                }
            }
            impl From<$t> for $tt {
                fn from(v: $t) -> Self {
                    v.0
                }
            }
            impl From<$tt> for $t {
                fn from(v: $tt) -> Self {
                    Self(v)
                }
            }
            impl PartialEq<$tt> for $t {
                fn eq(&self, other: &$tt) -> bool {
                    self.0.eq(other)
                }
            }
        )*
    };
}
impl_traits!(VersionWrapper => semver::Version);

#[cfg(test)]
mod tests {
    use super::*;

    fn range_contains(req: &str, version: &str) -> bool {
        let range = req_to_range(VersionReq::parse(req).unwrap());
        range.contains(&VersionWrapper(semver::Version::parse(version).unwrap()))
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
}
