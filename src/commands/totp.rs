use anyhow::Result;

use crate::cli::{TotpAction, TotpArgs};

pub fn dispatch(args: &TotpArgs) -> Result<()> {
    match &args.action {
        TotpAction::Init { force } => run_init(*force),
        TotpAction::Show => run_show(),
        TotpAction::Reset { force } => run_reset(*force),
    }
}

fn run_init(_force: bool) -> Result<()> {
    eprintln!("not yet implemented (Plan 04-02)");
    Ok(())
}

fn run_show() -> Result<()> {
    eprintln!("not yet implemented (Plan 04-02)");
    Ok(())
}

fn run_reset(_force: bool) -> Result<()> {
    eprintln!("not yet implemented (Plan 04-02)");
    Ok(())
}
