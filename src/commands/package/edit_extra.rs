use clap::{Args, Subcommand};
use qpm_package::models::{package::PackageConfig, shared_package::SharedPackageConfig};

use crate::{
    commands::Command,
    models::package::PackageConfigExtensions,
    repository::{self},
    utils::toggle::Toggle,
};

#[derive(Args, Debug, Clone)]

pub struct EditExtraArgs {

    /// Provide a link to the mod
    #[clap(long = "modLink")]
    pub mod_link: Option<String>,

    /// Additional options for compilation and edits to compilation related files.
    #[clap(subcommand)]
    pub compile_options: Option<EditExtraOptions>,

    #[clap(long, default_value = "false")]
    offline: bool,
}

#[derive(Subcommand, Debug, Clone)]

pub enum EditExtraOptions {
    /// Additional options for compilation and edits to compilation related files.
    CompileOptions(CompileOptionsEditArgs),
}

#[derive(Args, Debug, Clone)]

pub struct CompileOptionsEditArgs {
    /// Additional include paths to add, relative to the extern directory. Prefix with a '-' to remove that entry
    #[clap(long = "includePaths")]
    pub include_paths: Option<String>,
    /// Additional system include paths to add, relative to the extern directory. Prefix with a '-' to remove that entry
    #[clap(long = "systemIncludes")]
    pub system_includes: Option<String>,
    /// Additional C++ features to add. Prefix with a '-' to remove that entry
    #[clap(long = "cppFeatures")]
    pub cpp_features: Option<String>,
    /// Additional C++ flags to add. Prefix with a '-' to remove that entry
    #[clap(long = "cppFlags")]
    pub cpp_flags: Option<String>,
    /// Additional C flags to add. Prefix with a '-' to remove that entry
    #[clap(long = "cFlags")]
    pub c_flags: Option<String>,
}

impl Command for EditExtraArgs {
    fn execute(self) -> color_eyre::Result<()> {
        let mut package = PackageConfig::read(".")?;
        let mut any_changed = false;
       
        if let Some(mod_link) = self.mod_link {
            package_edit_extra_mod_link(&mut package, mod_link);
            any_changed = true;
        }


        if any_changed {
            package.write(".")?;
            let mut shared_package = SharedPackageConfig::read(".")?;
            shared_package.config = package;
            shared_package.write(".")?;
        }
        Ok(())
    }
}



pub fn package_edit_extra_mod_link(package: &mut PackageConfig, mod_link: String) {
    println!("Setting mod_link: {mod_link:#?}");
    package.additional_data.mod_link = Some(mod_link);
}

