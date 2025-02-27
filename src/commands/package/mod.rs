use clap::{Args, Subcommand};

use super::Command;

pub(crate) mod create;
pub(crate) mod edit;
pub(crate) mod edit_extra;
pub(crate) mod format;

#[derive(Args, Debug, Clone)]

pub struct PackageCommand {
    #[clap(subcommand)]
    pub op: PackageOperation,
}

#[derive(Subcommand, Debug, Clone)]

pub enum PackageOperation {
    /// Create a package
    Create(create::PackageOperationCreateArgs),
    /// Edit various properties of the package
    Edit(edit::EditArgs),
    /// Edit extra supported properties of the package
    EditExtra(edit_extra::EditExtraArgs),
    /// Re-serialize the qpm.json file
    Format(format::FormatArgs),
}

impl Command for PackageCommand {
    fn execute(self) -> color_eyre::Result<()> {
        match self.op {
            PackageOperation::Create(c) => c.execute(),
            PackageOperation::Edit(e) => e.execute(),
            PackageOperation::EditExtra(ee) => ee.execute(),
            PackageOperation::Format(f) => f.execute(),
        }
    }
}

/*  Check if all these are supported here:
    - branchName (System.String): Branch name of a Github repo. Only used when a valid github url is provided - Supported in: package, dependency
    - headersOnly (System.Boolean): Specify that this package is headers only and does not contain a .so or .a file - Supported in: package
    - staticLinking (System.Boolean): Specify that this package is static linking - Supported in: package
    - soLink (System.String): Specify the download link for a release .so or .a file - Supported in: package
    - debugSoLink (System.String): Specify the download link for a debug .so or .a files (usually from the obj folder) - Supported in: package
    - extraFiles (System.String[]): Specify any additional files to be downloaded - Supported in: package, dependency
    - overrideSoName (System.String): Override the downloaded .so or .a filename with this name instead. - Supported in: package
    - subfolder (System.String): Subfolder for this particular package in the provided repository, relative to root of the repo. - Supported in: package
    - compileOptions (QPM.Commands.SupportedPropertiesCommand+CompileOptionsProperty): Additional options for compilation and edits to compilation related files. - Supported in: package
        Type: QPM.Commands.SupportedPropertiesCommand+CompileOptionsProperty
        - includePaths - OPTIONAL (System.String[]): Additional include paths to add, relative to the extern directory.
        - systemIncludes - OPTIONAL (System.String[]): Additional system include paths to add, relative to the extern directory.
        - cppFeatures - OPTIONAL (System.String[]): Additional C++ features to add.
        - cppFlags - OPTIONAL (System.String[]): Additional C++ flags to add.
        - cFlags - OPTIONAL (System.String[]): Additional C flags to add.

    - localPath (System.String): Copy a dependency from a location that is local to this root path instead of from a remote url - Supported in: dependency
    - useRelease (System.Boolean): Specify if a dependency should download a release .so or .a file. Defaults to false - Supported in: dependency

    NOTE: Styles are not used by anybody, deprecate!
    - styles (QPM.Commands.SupportedPropertiesCommand+StyleProperty[]): Provide various download links of differing styles. Styles are appended to module names. - Supported in: package
    - style (System.String): Specify the style to use. - Supported in: dependency
    Type: QPM.Commands.SupportedPropertiesCommand+StyleProperty
      - name - REQUIRED (System.String): The name of the style.
      - soLink - OPTIONAL (System.String): The release downloadable so link for this style. Must exist if being used as release.
      - debugSoLink - OPTIONAL (System.String): The debug downloadable so link for this style. Must exist if being used as debug.

*/
