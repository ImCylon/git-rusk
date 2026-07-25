use crate::branch_protection;
use crate::config::Config;
use crate::error::GitHookError;
use crate::git_ops;
use crate::totp;

pub fn run(
    _prev_head: String,
    _new_head: String,
    branch_switch: u8,
    config: &Config,
) -> Result<(), GitHookError> {
    if branch_switch != 1 {
        return Ok(());
    }

    let current_branch = git_ops::get_current_branch()?;

    if current_branch == config.branches.default_branch {
        return Ok(());
    }

    if config.totp.require_for_branch_switch && !branch_protection::is_allowed_branch(&current_branch, &config.branches.allowed) {
        totp::verify_from_env(&config.totp)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_skips_totp_when_disabled() {
        let config = Config::default();
        let result = run("abc123".to_string(), "def456".to_string(), 1, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_verifies_totp_when_enabled() {
        let mut config = Config::default();
        config.totp.require_for_branch_switch = true;
        config.branches.default_branch = "main".to_string();
        config.branches.allowed = vec!["main".to_string()];
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_checkout_totp");
        std::env::set_var("TOTP_CODE", "123456");
        let result = run("abc123".to_string(), "def456".to_string(), 1, &config);
        assert!(result.is_err());
        std::env::remove_var("TOTP_CODE");
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    fn test_run_skips_totp_for_file_checkout() {
        let mut config = Config::default();
        config.totp.require_for_branch_switch = true;
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_checkout_totp");
        std::env::set_var("TOTP_CODE", "123456");
        let result = run("abc123".to_string(), "def456".to_string(), 0, &config);
        assert!(result.is_ok());
        std::env::remove_var("TOTP_CODE");
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    fn test_run_skips_totp_for_default_branch() {
        let mut config = Config::default();
        config.totp.require_for_branch_switch = true;
        config.branches.default_branch = "main".to_string();

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        crate::git_ops::init_repo(path).unwrap();
        crate::git_ops::ensure_initial_commit(path).unwrap();
        crate::git_ops::ensure_branch(path, "main").unwrap();
        crate::git_ops::checkout(path, "main").unwrap();
        std::env::set_current_dir(path).unwrap();

        let result = run("abc123".to_string(), "def456".to_string(), 1, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_verifies_totp_for_protected_branch() {
        let mut config = Config::default();
        config.totp.require_for_branch_switch = true;
        config.branches.allowed = vec!["development".to_string()];

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        crate::git_ops::init_repo(path).unwrap();
        crate::git_ops::ensure_initial_commit(path).unwrap();
        crate::git_ops::ensure_branch(path, "main").unwrap();
        crate::git_ops::checkout(path, "main").unwrap();
        std::env::set_current_dir(path).unwrap();

        std::env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_checkout_totp");
        std::env::set_var("TOTP_CODE", "123456");
        let result = run("abc123".to_string(), "def456".to_string(), 1, &config);
        assert!(result.is_err());
        std::env::remove_var("TOTP_CODE");
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
