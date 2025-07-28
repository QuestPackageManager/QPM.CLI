#![feature(try_find)]
#![feature(iterator_try_collect)]
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

// #[cfg(test)]
// mod tests;

fn print_completions<G: Generator>(generator: G, cmd: &mut clap::Command) {
    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut io::stdout(),
    );
}

/// Suggests the location where to pipe the auto-generated completion script
/// based on the shell type.
fn suggest_completion_location(shell: Shell) {
    eprintln!("To add this to your shell, you may use the following command:");

    let file_name = shell.file_name("qpm");

    // powershell is unique so
    // we make it its own suggestion
    if shell == Shell::PowerShell {
        eprintln!("\tqpm --generate {shell} | Set-Content \"$HOME\\qpm_autocomplete.ps1\"");
        eprintln!(
            "\t'if (Test-Path \"$HOME\\qpm_autocomplete.ps1\") {{ . \"$HOME\\qpm_autocomplete.ps1\" }}' | Add-Content -Path $PROFILE"
        );
    } else {
        let loc = match shell {
            Shell::Bash => format!("/etc/bash_completion.d/{file_name}"),
            Shell::Elvish => format!("~/.elvish/lib/completions/{file_name}"),
            Shell::Fish => format!("~/.config/fish/completions/{file_name}"),
            Shell::Zsh => format!("~/.zsh/completions/{file_name}"),
            _ => todo!(),
        };

        eprintln!("\tqpm --generate {shell} > {loc}")
    }
}

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
