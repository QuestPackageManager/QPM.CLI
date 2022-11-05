use clap::{Parser, Args};
use color_eyre::Result;

pub mod cache;
pub mod clear;
pub mod collapse;
pub mod config;
pub mod dependency;
pub mod doctor;
pub mod download;
pub mod install;
pub mod list;
pub mod package;
pub mod publish;
pub mod qmod;
pub mod restore;

pub trait Command {
    fn execute(&self) -> Result<()>;
}

#[derive(Parser)]
pub enum MainCommand {
    Restore(restore::RestoreCommand),
}

impl Command for MainCommand {
    fn execute(&self) -> Result<()> {
        match self {
            MainCommand::Restore(r) => r.execute(),
        }
    }
}
