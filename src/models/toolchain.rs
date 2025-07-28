use std::{fs::File, path::PathBuf};

use color_eyre::eyre::Result;
use qpm_package::models::{
    extra::PackageTripletCompileOptions, shared_package::SharedPackageConfig,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{models::package::SharedPackageConfigExtensions, repository::Repository};

use super::schemas::{SchemaLinks, WithSchema};

#[derive(Serialize, JsonSchema, Deserialize, Debug, Default, Clone)]
pub struct ToolchainData {
    /// Compile options
    pub compile_options: PackageTripletCompileOptions,

    /// Path to the extern directory
    pub extern_dir: PathBuf,
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
                .get_triplet_settings(&dep_triplet.restored_triplet)?;

            let package_id = &dep_config.id;
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

    let toolchain = ToolchainData {
        compile_options,
        extern_dir: shared_config.config.dependencies_directory.clone(),
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
