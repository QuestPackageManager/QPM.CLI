use clap::Parser;
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
    fn execute(self) -> Result<()>;
}

#[derive(Parser)]
pub enum MainCommand {
    Restore(restore::RestoreCommand),
    /// Cache control
    Cache(cache::CacheCommand),
    /// Clear all resolved dependencies by clearing the lock file
    Clear(clear::ClearCommand),
    /// Collect and collapse dependencies and print them to console
    Collapse(collapse::CollapseCommand),
    /// Config control
    Config(config::ConfigCommand),
    /// Dependency control
    Dependency(dependency::DependencyCommand),
    /// Package control
    Package(package::PackageCommand),
    /// List all properties that are currently supported by QPM
    List(list::ListCommand),
    /// Publish package
    Publish(publish::PublishCommand),
    /// Restore and resolve all dependencies from the package
    /// Qmod control
    Qmod(qmod::QmodCommand),
    /// Install to local repository
    Install(install::InstallCommand),
    /// Checks if your quest modding workspace is ready
    Doctor(doctor::DoctorCommand),
    Download(download::Download),
}

impl Command for MainCommand {
    fn execute(self) -> Result<()> {
        match self {
            MainCommand::Restore(r) => r.execute(),
            MainCommand::Cache(c) => c.execute(),
            MainCommand::Clear(c) => c.execute(),
            MainCommand::Collapse(c) => c.execute(),
            MainCommand::Config(c) => c.execute(),
            MainCommand::Dependency(c) => c.execute(),
            MainCommand::Package(c) => c.execute(),
            MainCommand::List(c) => c.execute(),
            MainCommand::Publish(c) => c.execute(),
            MainCommand::Qmod(c) => c.execute(),
            MainCommand::Install(c) => c.execute(),
            MainCommand::Doctor(c) => c.execute(),
            MainCommand::Download(c) => c.execute(),
        }
    }
}
