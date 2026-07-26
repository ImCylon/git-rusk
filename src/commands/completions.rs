use std::io;

use clap::CommandFactory;
use clap_complete::Shell;

use crate::cli::Cli;

/// Print a shell completion script for `git-rusk` to stdout.
///
/// The `bin_name` is the constant literal `"git-rusk"` (matches the package
/// name = binary name). Shells key completion on `argv[0]`, so a mismatched
/// name would produce dead scripts. Do NOT use `cmd.get_name()` — clap's
/// internal name could differ from the binary filename.
pub fn run(shell: Shell) {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "git-rusk", &mut io::stdout());
}
