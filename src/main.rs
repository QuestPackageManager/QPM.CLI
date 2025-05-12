#![feature(try_find)]
#![feature(iterator_try_collect)]
#![feature(let_chains)]
#![feature(exit_status_error)]
#![feature(if_let_guard)]
#![feature(path_add_extension)]

use std::io;

use clap::{CommandFactory, Parser};
use clap_complete::{Generator, Shell, generate};
use color_eyre::Result;
use commands::Command;

#[cfg(feature = "cli")]
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
    let command_result = commands::Opt::parse();

    if let Some(generator) = command_result.generator {
        let mut cmd = commands::Opt::command();
        eprintln!("Generating completion file for {generator:?}...");
        print_completions(generator, &mut cmd);
        suggest_completion_location(generator);
    }
    if let Some(command) = command_result.command {
        command.execute()?;
    }

    Ok(())
}
