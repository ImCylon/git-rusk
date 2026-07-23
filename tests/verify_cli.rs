use clap::CommandFactory;

use git_hook::cli::Cli;

#[test]
fn verify_cli() {
    Cli::command().debug_assert();
}

#[test]
fn verify_subcommands() {
    let cmd = Cli::command();
    let subcommands: Vec<&str> = cmd
        .get_subcommands()
        .map(|s| s.get_name())
        .collect();

    assert!(
        subcommands.contains(&"init"),
        "missing 'init' subcommand: {:?}",
        subcommands
    );
    assert!(
        subcommands.contains(&"install-hooks"),
        "missing 'install-hooks' subcommand: {:?}",
        subcommands
    );
    assert!(
        subcommands.contains(&"hook"),
        "missing 'hook' subcommand: {:?}",
        subcommands
    );
}
