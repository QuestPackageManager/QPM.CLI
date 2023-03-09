#![feature(once_cell)]
#![feature(entry_insert)]
#![feature(try_find)]
#![feature(iterator_try_collect)]
#![feature(let_chains)]
#![feature(is_some_and)]
#![feature(option_result_contains)]
#![feature(exit_status_error)]

use clap::Parser;
use color_eyre::Result;
use commands::Command;

pub mod commands;
pub mod models;
pub mod network;
pub mod repository;
pub mod resolver;
pub mod terminal;
pub mod utils;

#[cfg(benchmark)]
mod benchmark;

#[cfg(test)]
mod tests;


#[cfg(all(feature = "reqwest", feature = "ureq"))]
compile_error!("feature \"reqwest\" and feature \"ureq\" cannot be enabled at the same time");

#[cfg(not(any(feature = "reqwest", feature = "ureq")))]
compile_error!("feature \"reqwest\" or feature \"ureq\" must be enabled, though not both simultaneously");


fn main() -> Result<()> {
    color_eyre::config::HookBuilder::default()
        .panic_section(concat!(
            "version ",
            env!("CARGO_PKG_VERSION"),
            " consider reporting the bug on github ",
            env!("CARGO_PKG_REPOSITORY"),
            "/issues/new"
        ))
        .install()?;
    commands::MainCommand::parse().execute()?;

    Ok(())
}
