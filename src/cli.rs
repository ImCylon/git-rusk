use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "git-rusk",
    version,
    about = "Git hook manager with branch protection + TOTP"
)]
pub struct Cli {
    /// Path to a .git-rusk.toml config file (default: auto-discover in CWD)
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Override the default protected branch name (default: "main")
    #[arg(long, global = true, value_name = "NAME")]
    pub default_branch: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a git repo with hooks and optional .gitignore
    #[command(name = "init")]
    Init(InitArgs),

    /// (Re)write the .git/hooks/ wrapper scripts
    #[command(name = "install-hooks")]
    InstallHooks,

    /// Run a git hook by name (called by the installed wrapper scripts)
    #[command(name = "hook")]
    Hook {
        name: HookName,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}

#[derive(Args)]
pub struct InitArgs {
    /// Repository path to initialize (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Language template for .gitignore (e.g. "rust", "python")
    #[arg(short, long)]
    pub gitignore: Option<String>,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum HookName {
    /// pre-commit hook — validates commits before they are created
    PreCommit,

    /// commit-msg hook — validates the commit message format
    CommitMsg,

    /// post-checkout hook — runs after branch switching
    PostCheckout,
}
