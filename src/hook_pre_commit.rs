use crate::branch_protection;
use crate::config::Config;
use crate::error::GitHookError;
use crate::git_ops;
use crate::totp;

pub fn run(config: &Config) -> Result<(), GitHookError> {
    let current_branch = git_ops::get_current_branch()?;

    if current_branch == "HEAD" {
        return Ok(());
    }

    if !branch_protection::is_allowed_branch(&current_branch, &config.branches.allowed) {
        return Err(GitHookError::CommitBlockedOnProtectedBranch {
            branch: current_branch,
            allowed: config.branches.allowed.join(", "),
        });
    }

    if config.totp.require_for_commit {
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
        let result = run(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_verifies_totp_when_enabled() {
        let mut config = Config::default();
        config.totp.require_for_commit = true;
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/test_pre_commit_totp");
        std::env::set_var("TOTP_CODE", "123456");
        let result = run(&config);
        assert!(result.is_err());
        std::env::remove_var("TOTP_CODE");
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    fn test_run_blocks_commit_on_protected_branch() {
        let mut config = Config::default();
        config.branches.allowed = vec!["development".to_string()];
        config.totp.require_for_commit = false;

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        crate::git_ops::init_repo(path).unwrap();
        crate::git_ops::ensure_initial_commit(path).unwrap();
        crate::git_ops::ensure_branch(path, "main").unwrap();
        crate::git_ops::checkout(path, "main").unwrap();
        std::env::set_current_dir(path).unwrap();

        let result = run(&config);
        assert!(result.is_err());
        match result {
            Err(GitHookError::CommitBlockedOnProtectedBranch { branch, .. }) => {
                assert_eq!(branch, "main");
            }
            _ => panic!("Expected CommitBlockedOnProtectedBranch error"),
        }
    }

    #[test]
    fn test_run_allows_commit_on_allowed_branch() {
        let mut config = Config::default();
        config.branches.allowed = vec!["main".to_string()];
        config.totp.require_for_commit = false;

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        crate::git_ops::init_repo(path).unwrap();
        crate::git_ops::ensure_initial_commit(path).unwrap();
        crate::git_ops::ensure_branch(path, "main").unwrap();
        crate::git_ops::checkout(path, "main").unwrap();
        std::env::set_current_dir(path).unwrap();

        let result = run(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_skips_branch_check_for_detached_head() {
        let mut config = Config::default();
        config.totp.require_for_commit = false;

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        crate::git_ops::init_repo(path).unwrap();
        crate::git_ops::ensure_initial_commit(path).unwrap();

        std::env::set_current_dir(path).unwrap();
        let output = std::process::Command::new("git")
            .args(["checkout", "--detach"])
            .output()
            .unwrap();
        assert!(output.status.success());

        let result = run(&config);
        assert!(result.is_ok());
    }
}
