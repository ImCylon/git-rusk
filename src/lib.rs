pub mod cli;
pub mod commands;
pub mod config;
pub mod error;

use anyhow::Result;
use cli::Cli;

pub fn run(cli: Cli) -> Result<()> {
    let config = resolve_config(&cli)?;

    match &cli.command {
        cli::Command::Init(args) => commands::init::run(args, &config),
        cli::Command::InstallHooks => commands::install_hooks::run(&config),
        cli::Command::Hook { name, args } => commands::hook::run(*name, args, &config),
    }
}

fn resolve_config(cli: &Cli) -> Result<config::Config> {
    let mut config = config::Config::load(cli.config.as_deref())?;
    if let Some(ref branch) = cli.default_branch {
        config.branches.default_branch = branch.clone();
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_run_config_none_no_error() {
        let cli = Cli::parse_from(["git-hook", "init"]);
        let result = run(cli);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_config_valid_path() {
        let toml_content = r#"
[branches]
allowed = ["dev"]
default_branch = "dev"
"#;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), toml_content).unwrap();
        let cli = Cli::parse_from(["git-hook", "--config", tmp.path().to_str().unwrap(), "init"]);
        let result = run(cli);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_config_nonexistent_path_errors() {
        let cli = Cli::parse_from(["git-hook", "--config", "/nonexistent/path.toml", "init"]);
        let result = run(cli);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_config_default_branch_override() {
        let cli = Cli::parse_from(["git-hook", "--default-branch", "feature", "init"]);
        let config = resolve_config(&cli).unwrap();
        assert_eq!(config.branches.default_branch, "feature");
    }

    #[test]
    fn test_resolve_config_no_override_uses_default() {
        let cli = Cli::parse_from(["git-hook", "init"]);
        let config = resolve_config(&cli).unwrap();
        assert_eq!(config.branches.default_branch, "development");
    }

    #[test]
    fn test_resolve_config_default_branch_overrides_toml() {
        let toml_content = r#"
[branches]
default_branch = "development"
"#;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), toml_content).unwrap();
        let config_path = tmp.path().to_str().unwrap();
        let cli = Cli::parse_from([
            "git-hook",
            "--config",
            config_path,
            "--default-branch",
            "feature",
            "init",
        ]);
        let config = resolve_config(&cli).unwrap();
        assert_eq!(config.branches.default_branch, "feature");
    }

    #[test]
    fn test_resolve_config_toml_value_without_override() {
        let toml_content = r#"
[branches]
default_branch = "staging"
"#;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), toml_content).unwrap();
        let config_path = tmp.path().to_str().unwrap();
        let cli = Cli::parse_from(["git-hook", "--config", config_path, "init"]);
        let config = resolve_config(&cli).unwrap();
        assert_eq!(config.branches.default_branch, "staging");
    }
}
