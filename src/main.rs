use std::process::ExitCode;

use clap::Parser;

use git_hook::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match git_hook::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::from(1)
        }
    }
}
