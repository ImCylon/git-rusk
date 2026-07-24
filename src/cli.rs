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
    InstallHooks {
        /// Force overwrite existing hooks (not symlinks)
        #[arg(long)]
        force: bool,
    },

    /// Run a git hook by name (called by the installed wrapper scripts)
    #[command(name = "hook")]
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },

    /// Manage the global TOTP secret
    #[command(name = "totp")]
    Totp(TotpArgs),
}

/// Arguments for the `totp` subcommand.
#[derive(Args)]
pub struct TotpArgs {
    #[command(subcommand)]
    pub action: TotpAction,
}

/// Actions available under the `totp` subcommand.
#[derive(Subcommand)]
pub enum TotpAction {
    /// Generate and save a new global TOTP secret
    #[command(name = "init")]
    Init {
        /// Overwrite existing secret without confirmation
        #[arg(long)]
        force: bool,

        /// Manually set a Base32 secret instead of generating one
        #[arg(long, value_name = "BASE32")]
        secret: Option<String>,
    },

    /// Display the current TOTP secret and otpauth URI
    #[command(name = "show")]
    Show,

    /// Rotate the global TOTP secret
    #[command(name = "reset")]
    Reset {
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,

        /// Manually set a Base32 secret instead of generating one
        #[arg(long, value_name = "BASE32")]
        secret: Option<String>,
    },
}

/// Actions available under the `hook` subcommand.
#[derive(Subcommand)]
pub enum HookAction {
    /// pre-commit hook — validates TOTP before commit (if enabled)
    PreCommit,

    /// commit-msg hook — validates the commit message format
    CommitMsg {
        /// Path to the commit message file
        msg_file: std::path::PathBuf,
    },

    /// post-checkout hook — validates TOTP after branch switch (if enabled)
    PostCheckout,
}

#[derive(Args)]
pub struct InitArgs {
    /// Repository path to initialize (default: current directory)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Language template for .gitignore
    #[arg(short, long, value_enum, default_value_t = GitignoreLang::None)]
    pub gitignore: GitignoreLang,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum GitignoreLang {
    /// Rust .gitignore template
    #[value(name = "rust")]
    Rust,

    /// Python .gitignore template
    #[value(name = "python")]
    Python,

    /// Node.js .gitignore template
    #[value(name = "node")]
    Node,

    /// Skip .gitignore generation
    #[value(name = "none")]
    None,
}

impl GitignoreLang {
    pub fn as_str(&self) -> &'static str {
        match self {
            GitignoreLang::Rust => "rust",
            GitignoreLang::Python => "python",
            GitignoreLang::Node => "node",
            GitignoreLang::None => "none",
        }
    }
}
