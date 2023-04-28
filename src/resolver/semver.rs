use std::fmt;

use pubgrub::range::Range;
use semver::{Comparator, Op, Prerelease, VersionReq};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VersionWrapper(pub semver::Version);

pub fn req_to_range(req: VersionReq) -> Range<VersionWrapper> {
    let mut range = Range::any();
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
    Range::exact(semver::Version {
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
    greater_eq_xyz(major, minor, patch, pre.clone())
        .intersection(&exact_xyz(major, minor, patch, pre).negate())
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

impl pubgrub::version::Version for VersionWrapper {
    fn lowest() -> Self {
        Self(semver::Version::new(0, 0, 0))
    }

    fn bump(&self) -> Self {
        let mut v = self.0.clone();
        v.patch += 1;
        Self(v)
    }
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
