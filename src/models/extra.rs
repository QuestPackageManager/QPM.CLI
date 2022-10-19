use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalPackageMetadata {
    /// Copy a dependency from a location that is local to this root path instead of from a remote url
    /// Technically just a dependency field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,

    /// By default if empty, true
    /// If false, this mod dependency will NOT be included in the generated mod.json
    /// Technically just a dependency field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_qmod: Option<bool>,

    /// Whether or not the package is header only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers_only: Option<bool>,

    /// Whether or not the package is statically linked
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_linking: Option<bool>,

    /// Whether to use the release or debug .so for linking
    /// Technically just a dependency field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_release: Option<bool>,

    /// the link to the so file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub so_link: Option<String>,

    /// the link to the debug .so file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_so_link: Option<String>,

    /// the overridden so file name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_so_name: Option<String>,

    /// the link to the qmod
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_link: Option<String>,

    /// Branch name of a Github repo. Only used when a valid github url is provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_name: Option<String>,

    /// Specify any additional files to be downloaded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_files: Option<Vec<String>>,

    /// Whether or not the dependency is private and should be used in restore
    /// Technically just a dependency field
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename(serialize = "private", deserialize = "private")
    )]
    pub is_private: Option<bool>,

    /// Additional Compile options to be used with this package
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_options: Option<CompileOptions>,

    /// Sub folder to use from the downloaded repo / zip, so one repo can contain multiple packages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_folder: Option<String>,
}

/// - compileOptions (QPM.Commands.SupportedPropertiesCommand+CompileOptionsProperty): Additional options for compilation and edits to compilation related files. - Supported in: package
/// Type: QPM.Commands.SupportedPropertiesCommand+CompileOptionsProperty
/// - includePaths - OPTIONAL (System.String[]): Additional include paths to add, relative to the extern directory.
/// - systemIncludes - OPTIONAL (System.String[]): Additional system include paths to add, relative to the extern directory.
/// - cppFeatures - OPTIONAL (System.String[]): Additional C++ features to add.
/// - cppFlags - OPTIONAL (System.String[]): Additional C++ flags to add.
/// - cFlags - OPTIONAL (System.String[]): Additional C flags to add.
#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompileOptions {
    /// Additional include paths to add, relative to the extern directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_paths: Option<Vec<String>>,

    /// Additional system include paths to add, relative to the extern directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_includes: Option<Vec<String>>,

    /// Additional C++ features to add.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpp_features: Option<Vec<String>>,

    /// Additional C++ flags to add.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpp_flags: Option<Vec<String>>,

    /// Additional C flags to add.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c_flags: Option<Vec<String>>,
}
