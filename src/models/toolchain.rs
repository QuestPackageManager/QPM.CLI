use std::{collections::HashMap, fs::File, path::PathBuf};

use color_eyre::eyre::{Context, ContextCompat, Result};
use itertools::Itertools;
use qpm_package::models::{
    extra::PackageCompileOptions, package::DependencyId, shared_package::SharedPackageConfig,
};
use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::repository::{Repository, file::FileRepository};

use super::schemas::{SchemaLinks, WithSchema};

#[derive(Serialize, JsonSchema, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainData {
    /// Compile options
    pub compile_options: PackageCompileOptions,

    /// Path to the extern directory
    pub extern_dir: PathBuf,

    pub libs_dir: PathBuf,
    pub include_dir: PathBuf,
    pub shared_dir: PathBuf,

    pub build_out: PathBuf,

    pub package_id: String,
    pub package_version: Version,

    pub linked_binaries: HashMap<DependencyId, Vec<PathBuf>>,
}

pub fn write_toolchain_file(
    shared_config: &SharedPackageConfig,
    repo: &impl Repository,
    toolchain_path: &std::path::PathBuf,
) -> Result<()> {
    let file_repo = FileRepository::read_global_cache()?;
    let extern_dir = shared_config.config.dependencies_directory.clone();
    let compile_options = shared_config
        .restored_dependencies
        .iter()
        .map(|(dep_id, dep_info)| -> Result<Option<PackageCompileOptions>> {
            let Some(dep_config) = repo.get_package(dep_id, &dep_info.restored_version)? else {
                return Ok(None);
            };
            let dep_config = dep_config.config;

            let package_id = dep_config.id.clone();
            // Prepend the extern dir and package id to the include paths
            let prepend_path = |dir: &String| {
                extern_dir
                    .join("includes")
                    .join(&package_id.0)
                    .join(dir)
                    .to_string_lossy()
                    .to_string()
            };

            let Some(mut compile_options) = dep_config.compile_options else {
                return Ok(None);
            };

            // prepend path
            compile_options.include_paths = compile_options
                .include_paths
                .map(|v| v.iter().map(prepend_path).collect());
            compile_options.system_includes = compile_options
                .system_includes
                .map(|v| v.iter().map(prepend_path).collect());

            Ok(Some(compile_options))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .fold(PackageCompileOptions::default(), |acc, x| {
            let c_flags: Vec<String> = acc
                .c_flags
                .unwrap_or_default()
                .into_iter()
                .chain(x.c_flags.unwrap_or_default())
                .collect();
            let cpp_flags: Vec<String> = acc
                .cpp_flags
                .unwrap_or_default()
                .into_iter()
                .chain(x.cpp_flags.unwrap_or_default())
                .collect();
            let include_paths: Vec<String> = acc
                .include_paths
                .unwrap_or_default()
                .into_iter()
                .chain(x.include_paths.unwrap_or_default())
                .collect();

            let system_includes: Vec<String> = acc
                .system_includes
                .unwrap_or_default()
                .into_iter()
                .chain(x.system_includes.unwrap_or_default())
                .collect();

            PackageCompileOptions {
                c_flags: Some(c_flags),
                cpp_flags: Some(cpp_flags),
                include_paths: Some(include_paths),
                system_includes: Some(system_includes),
            }
        });

    let extern_binaries = FileRepository::libs_dir(&extern_dir);

    let linked_binaries = shared_config
        .restored_dependencies
        .iter()
        .map(|(dep_id, dep_info)| -> Result<(DependencyId, Vec<PathBuf>)> {
            let dep_config = repo
                .get_package(dep_id, &dep_info.restored_version)
                .with_context(|| {
                    format!(
                        "Failed to get package config for package '{}' version '{}'",
                        dep_id, dep_info.restored_version
                    )
                })?
                .with_context(|| {
                    format!(
                        "Package config not found for package '{}' version '{}'",
                        dep_id, dep_info.restored_version
                    )
                })?;
            let collect_files_of_package = file_repo.collect_files_of_package(&dep_config.config)?;

            let binaries = collect_files_of_package
                .binaries
                .into_iter()
                .map(|bin| extern_binaries.join(bin.file_name().unwrap()))
                .collect_vec();

            Ok((dep_id.clone(), binaries))
        })
        .collect::<Result<_>>()?;

    let package_id = shared_config.config.id.clone();
    let package_version = shared_config.config.version.clone();

    let libs_dir = FileRepository::libs_dir(&extern_dir);
    let include_dir = FileRepository::headers_path(&extern_dir);
    let shared_dir = shared_config.config.shared_directory.clone();
    let build_out = FileRepository::build_path(&extern_dir);

    let toolchain = ToolchainData {
        compile_options,
        extern_dir,
        libs_dir,
        include_dir,
        shared_dir,

        build_out,

        linked_binaries,

        package_id: package_id.0,
        package_version,
    };
    let file = File::create(toolchain_path)?;
    serde_json::to_writer_pretty(
        file,
        &WithSchema {
            schema: SchemaLinks::TOOLCHAIN_DATA,
            value: toolchain,
        },
    )?;
    Ok(())
}
