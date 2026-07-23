use std::process::ExitCode;

use clap::Parser;

use git_rusk::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match git_rusk::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::from(1)
        }
    }
}
