pub mod cli;
pub mod commands;
pub mod config;
pub mod error;

use anyhow::Result;
use cli::Cli;

pub fn run(cli: Cli) -> Result<()> {
    let config = config::Config::load(cli.config.as_deref())?;

    match &cli.command {
        cli::Command::Init(args) => commands::init::run(args, &config),
        cli::Command::InstallHooks => commands::install_hooks::run(&config),
        cli::Command::Hook { name, args } => commands::hook::run(*name, args, &config),
    }
}
