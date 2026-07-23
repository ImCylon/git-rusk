use anyhow::Result;

use crate::cli::HookName;
use crate::config::Config;

pub fn run(name: HookName, _args: &[String], _config: &Config) -> Result<()> {
    eprintln!("hook {:?}: not yet implemented (Phase 5/6)", name);
    Ok(())
}
