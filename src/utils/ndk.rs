use std::path::PathBuf;

use itertools::Itertools;
use qpm_package::models::{package::PackageConfig, triplet::PackageTriplet};
use semver::Version;

use crate::models::config::get_combine_config;

/// Resolves the NDK version based on the package configuration.
pub fn resolve_ndk_version(triplet: &PackageTriplet) -> Option<PathBuf> {
    let ndk_requirement = triplet.ndk.as_ref()?;

    let ndk_installed_path_opt = get_combine_config()
        .get_ndk_installed()
        .into_iter()
        .flatten()
        .sorted_by(|a, b| a.file_name().cmp(b.file_name()))
        .rev() // descending
        .find(|s| {
            Version::parse(s.file_name().to_str().unwrap())
                .is_ok_and(|version| ndk_requirement.matches(&version))
        });

    Some(ndk_installed_path_opt?.into_path())
}
