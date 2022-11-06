#![feature(custom_test_frameworks)]
#![feature(once_cell)]
#![feature(entry_insert)]
#![feature(try_find)]
#![feature(iterator_try_collect)]
#![feature(let_chains)]
#![feature(is_some_and)]

use clap::Parser;
use color_eyre::Result;
use commands::Command;

pub mod models;
pub mod network;
pub mod repository;
pub mod utils;
pub mod terminal;
pub mod resolver;
pub mod commands;

#[cfg(test)]
pub mod tests;

#[cfg(test)]
pub mod benches;

fn main() -> Result<()> {
    color_eyre::install()?;
    commands::MainCommand::parse().execute()?;


    Ok(())
}
