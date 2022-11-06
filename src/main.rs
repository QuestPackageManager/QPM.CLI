use clap::Parser;
use color_eyre::Result;
use qpm_cli::commands::Command;

fn main() -> Result<()> {
    color_eyre::install()?;
    qpm_cli::commands::MainCommand::parse().execute()?;


    Ok(())
}
