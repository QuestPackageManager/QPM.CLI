use std::{collections::HashMap, fs::File, path::PathBuf};

use color_eyre::eyre::Result;
use qpm_package::models::{
    extra::PackageTripletCompileOptions, package::DependencyId, shared_package::SharedPackageConfig,
};
use schemars::JsonSchema;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    models::package::SharedPackageConfigExtensions,
    repository::{Repository, local::FileRepository},
};

use super::schemas::{SchemaLinks, WithSchema};

#[derive(Serialize, JsonSchema, Deserialize, Debug, Clone)]
pub struct ToolchainData {
    /// Compile options
    pub compile_options: PackageTripletCompileOptions,

    /// Path to the extern directory
    pub extern_dir: PathBuf,

    pub libs_dir: PathBuf,
    pub include_dir: PathBuf,
    pub shared_dir: PathBuf,

    pub build_out: PathBuf,
    pub triplet_out: PathBuf,

    pub package_id: String,
    pub package_version: Version,
    pub restored_triplet: String,

    pub linked_binaries: HashMap<DependencyId, Vec<PathBuf>>,
}

pub fn write_toolchain_file(
    shared_config: &SharedPackageConfig,
    repo: &impl Repository,
    toolchain_path: &std::path::PathBuf,
) -> Result<()> {
    let extern_dir = &shared_config.config.dependencies_directory.display();
    let compile_options = shared_config
        .get_restored_triplet()
        .restored_dependencies
        .iter()
        .filter_map(|(dep_id, dep_triplet)| {
            let dep_config = repo
                .get_package(dep_id, &dep_triplet.restored_version)
                .ok()??;
            let dep_triplet_config = dep_config
                .triplets
                .get_merged_triplet(&dep_triplet.restored_triplet)?
                .into_owned();

            let package_id = &dep_config.id;
            // Prepend the extern dir and package id to the include paths
            let prepend_path = |dir: &String| format!("{extern_dir}/includes/{package_id}/{dir}");

            let mut compile_options = dep_triplet_config.compile_options?;

            // prepend path
            compile_options.include_paths = compile_options
                .include_paths
                .map(|v| v.iter().map(prepend_path).collect());
            compile_options.system_includes = compile_options
                .system_includes
                .map(|v| v.iter().map(prepend_path).collect());

            Some(compile_options)
        })
        .fold(PackageTripletCompileOptions::default(), |acc, x| {
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

            PackageTripletCompileOptions {
                c_flags: Some(c_flags),
                cpp_flags: Some(cpp_flags),
                include_paths: Some(include_paths),
                system_includes: Some(system_includes),
            }
        });

    let linked_binaries =
        shared_config
            .get_restored_triplet()
            .restored_dependencies
            .iter()
            .filter_map(|(dep_id, dep_triplet)| {
                let dep_config = repo
                    .get_package(dep_id, &dep_triplet.restored_version)
                    .unwrap_or_else(|e| {
                        panic!(
                            "Failed to get package config for package '{}' version '{}': {}",
                            dep_id, dep_triplet.restored_version, e
                        )
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "Package config not found for package '{}' version '{}'",
                            dep_id, dep_triplet.restored_version
                        )
                    });
                let dep_triplet_config = dep_config
                .triplets
                .get_merged_triplet(&dep_triplet.restored_triplet)
                .unwrap_or_else(|| panic!(
                    "Failed to get triplet settings for package '{}' version '{}' triplet '{}'",
                    dep_id, dep_triplet.restored_version, dep_triplet.restored_triplet
                ));

                let out_binaries = dep_triplet_config.out_binaries.clone()?;
                Some((dep_id.clone(), out_binaries))
            })
            .collect();

    let package_id = shared_config.config.id.clone();
    let package_version = shared_config.config.version.clone();
    let restored_triplet = shared_config.restored_triplet.clone();

    let extern_dir = shared_config.config.dependencies_directory.clone();
    let libs_dir = FileRepository::libs_dir(&extern_dir);
    let include_dir = FileRepository::headers_path(&extern_dir);
    let shared_dir = shared_config.config.shared_directory.clone();
    let build_out = FileRepository::build_path(&extern_dir);
    let triplet_out = build_out.join(&shared_config.restored_triplet.0);

    let toolchain = ToolchainData {
        compile_options,
        extern_dir,
        libs_dir,
        include_dir,
        shared_dir,

        build_out,
        triplet_out,

        linked_binaries,

        package_id: package_id.0,
        restored_triplet: restored_triplet.0,
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
