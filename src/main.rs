#![feature(once_cell)]

use color_eyre::Result;

pub mod models;
pub mod network;
pub mod repository;

fn main() -> Result<()> {
    color_eyre::install()?;
    println!("Hello, world!");

    Ok(())
}
