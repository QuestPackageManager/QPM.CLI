//! The CLI's color scheme. Color output through these methods rather than calling
//! `owo_colors` colors (`.red()`, `.bright_yellow()`, ...) directly, so the same kind of
//! value always gets the same color, and colors turn off when the stream isn't a terminal
//! or `NO_COLOR` is set (`CLICOLOR_FORCE` forces them on).

use std::fmt::Display;

use owo_colors::{OwoColorize, Stream};

macro_rules! scheme {
    ($($(#[$doc:meta])* $name:ident => $stream:ident, |$s:ident| $style:expr;)*) => {
        pub trait QPMColor: OwoColorize + Display {
            $(
                $(#[$doc])*
                #[inline(always)]
                fn $name(&self) -> impl Display + '_ {
                    self.if_supports_color(Stream::$stream, |$s| $style)
                }
            )*
        }
    };
}

scheme! {
    /// A file being downloaded
    download_file_name_color => Stdout, |s| s.cyan();
    /// A file or directory path
    file_path_color => Stdout, |s| s.yellow();
    /// A package id
    dependency_id_color => Stdout, |s| s.blue();
    /// A concrete package version
    version_id_color => Stdout, |s| s.blue();
    /// A version range a dependency asks for
    dependency_version_color => Stdout, |s| s.purple();
    /// A second version range, shown next to `dependency_version_color`
    alternate_dependency_version_color => Stdout, |s| s.yellow();
    /// A target triplet
    triplet_id_color => Stdout, |s| s.green();
    /// A command or flag the user can run
    command_color => Stdout, |s| s.bold();
    /// A URL
    url_color => Stdout, |s| s.cyan();
    /// A config value or count
    value_color => Stdout, |s| s.bright_yellow();
    /// Something that went wrong. Goes to stderr when used in errors.
    error_color => Stderr, |s| s.bright_red();
    /// Something that went right
    success_color => Stdout, |s| s.green();
}

impl<D: OwoColorize + Display> QPMColor for D {}
