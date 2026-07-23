use anyhow::Result;

use crate::cli::InitArgs;
use crate::config::Config;

pub fn run(_args: &InitArgs, _config: &Config) -> Result<()> {
    eprintln!("init: not yet implemented (Phase 3)");
    Ok(())
}
