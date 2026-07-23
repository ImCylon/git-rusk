use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "git-hook", version, about = "Git hook manager with branch protection + TOTP")]
pub struct Cli {
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(name = "init")]
    Init(InitArgs),

    #[command(name = "install-hooks")]
    InstallHooks,

    #[command(name = "hook")]
    Hook {
        name: HookName,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}

#[derive(Args)]
pub struct InitArgs {
    #[arg(default_value = ".")]
    pub path: PathBuf,

    #[arg(short, long)]
    pub gitignore: Option<String>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum HookName {
    PreCommit,
    CommitMsg,
    PostCheckout,
}
