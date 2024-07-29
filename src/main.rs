#![feature(entry_insert)]
#![feature(try_find)]
#![feature(iterator_try_collect)]
#![feature(let_chains)]
#![feature(exit_status_error)]
#![feature(if_let_guard)]

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


#[cfg(all(feature = "gitoxide", feature = "libgit2"))]
compile_error!("feature \"gitoxide\" and feature \"libgit2\" cannot be enabled at the same time");

fn main() -> Result<()> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Trace)
        .parse_default_env()
        .init();
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
