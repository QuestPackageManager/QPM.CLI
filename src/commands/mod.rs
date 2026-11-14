use crate::context::QpmContext;
use clap::{Parser, Subcommand};
use clap_complete::Shell;
use color_eyre::Result;

pub mod build;
pub mod cache;
pub mod clear;
pub mod collapse;
pub mod config;
pub mod dependency;
pub mod doctor;
pub mod download;
pub mod genschema;
pub mod install;
pub mod list;
#[cfg(feature = "migrate")]
pub mod migrate;
pub mod ndk;
pub mod package;
pub mod publish;
pub mod qmod;
pub mod qpkg;
pub mod restore;
pub mod scripts;
pub mod version;

#[cfg(feature = "templatr")]
pub mod templatr;

pub trait Command {
    fn execute(self, ctx: &QpmContext) -> Result<()>;
}

#[derive(Parser)]
#[command(name = "qpm", bin_name = "qpm", version, about, long_about)]
#[command(arg_required_else_help = true)]
pub struct Opt {
    // If provided, outputs the completion file for given shell
    #[arg(long = "generate", value_enum)]
    pub generator: Option<Shell>,

    #[command(subcommand)]
    pub command: Option<MainCommand>,

    /// Don't use the network: only already-cached packages are available
    #[arg(long, global = true, default_value = "false")]
    pub offline: bool,
}

#[derive(Subcommand)]
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
    Ndk(ndk::Ndk),

    /// Migrate a legacy qpm.json (QPM v1) into a qpm2.json (QPM v2) package config
    #[cfg(feature = "migrate")]
    Migrate(migrate::MigrateCommand),

    #[command(about = "Shorthand for qpm dependency add")]
    Add(dependency::DependencyOperationAddArgs),

    #[command(alias = "s")]
    Scripts(scripts::ScriptsCommand),

    #[cfg(feature = "templatr")]
    Templatr(templatr::TemplatrCommand),

    #[cfg(feature = "quest_emu")]
    Emu {
        yes: bool,
        #[command(subcommand)]
        main_command: quest_emu::commands::MainCommand,
    },

    Version(version::VersionCommand),

    #[command(hide = true)]
    GenSchema(genschema::GenSchemaCommand),

    /// QPKG control
    #[command(name = "qpkg", about = "QPKG control")]
    QPkg(qpkg::QPkgCommand),

    Build(build::BuildCommand),
}

impl Command for MainCommand {
    fn execute(self, ctx: &QpmContext) -> Result<()> {
        match self {
            MainCommand::Restore(r) => r.execute(ctx),
            MainCommand::Cache(c) => c.execute(ctx),
            MainCommand::Clear(c) => c.execute(ctx),
            MainCommand::Collapse(c) => c.execute(ctx),
            MainCommand::Config(c) => c.execute(ctx),
            MainCommand::Dependency(c) => c.execute(ctx),
            MainCommand::Package(c) => c.execute(ctx),
            MainCommand::List(c) => c.execute(ctx),
            MainCommand::Publish(c) => c.execute(ctx),
            MainCommand::Qmod(c) => c.execute(ctx),
            MainCommand::Install(c) => c.execute(ctx),
            MainCommand::Doctor(c) => c.execute(ctx),
            MainCommand::Download(c) => c.execute(ctx),
            MainCommand::Ndk(n) => n.execute(ctx),

            #[cfg(feature = "migrate")]
            MainCommand::Migrate(c) => c.execute(ctx),
            MainCommand::Add(add) => add.execute(ctx),
            MainCommand::Scripts(s) => s.execute(ctx),
            MainCommand::Version(v) => v.execute(ctx),
            MainCommand::GenSchema(g) => g.execute(ctx),
            MainCommand::QPkg(q) => q.execute(ctx),
            MainCommand::Build(build_command) => build_command.execute(ctx),

            #[cfg(feature = "templatr")]
            MainCommand::Templatr(c) => c.execute(ctx),

            #[cfg(feature = "quest_emu")]
            MainCommand::Emu { yes, main_command } => quest_emu::commands::Command::execute(
                main_command,
                &quest_emu::commands::GlobalContext { yes },
            ),
        }
    }
}
