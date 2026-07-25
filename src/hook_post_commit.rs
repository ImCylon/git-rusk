use std::path::Path;

use crate::branch_protection;
use crate::config::Config;
use crate::error::GitHookError;
use crate::git_ops;

pub fn run(config: &Config) -> Result<(), GitHookError> {
    let current_branch = git_ops::get_current_branch()?;

    if branch_protection::is_allowed_branch(&current_branch, &config.branches.allowed) {
        return Ok(());
    }

    let default_branch = &config.branches.default_branch;
    if current_branch == *default_branch {
        return Ok(());
    }

    if let Err(e) = git_ops::checkout(Path::new("."), default_branch) {
        eprintln!("Warning: auto-return to {} failed: {}", default_branch, e);
        eprintln!("You are still on protected branch: {}", current_branch);
        return Err(GitHookError::AutoReturnFailed(std::io::Error::new(
            std::io::ErrorKind::Other,
            e,
        )));
    }

    eprintln!(
        "Auto-returned to {} after commit on protected branch",
        default_branch
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    struct EnvGuard {
        old_value: Option<String>,
        var_name: String,
    }

    impl EnvGuard {
        fn new(var_name: &str) -> Self {
            Self {
                old_value: env::var(var_name).ok(),
                var_name: var_name.to_string(),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.old_value.as_ref() {
                Some(v) => env::set_var(&self.var_name, v),
                None => env::remove_var(&self.var_name),
            }
        }
    }

    fn create_test_repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        git_ops::init_repo(path).unwrap();
        git_ops::ensure_initial_commit(path).unwrap();
        tmp
    }

    fn setup_test_config(allowed: Vec<String>, default_branch: String) -> Config {
        let mut config = Config::default();
        config.branches.allowed = allowed;
        config.branches.default_branch = default_branch;
        config
    }

    #[test]
    fn test_run_does_nothing_on_allowed_branch() {
        let config = setup_test_config(vec!["feature".to_string()], "main".to_string());
        let _guard = EnvGuard::new("XDG_CONFIG_HOME");
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_commit_allowed");

        let tmp = create_test_repo();
        let path = tmp.path();
        git_ops::ensure_branch(path, "feature").unwrap();
        git_ops::checkout(path, "feature").unwrap();

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(path).unwrap();
        let result = run(&config);
        env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());
    }

    #[test]
    fn test_run_does_nothing_on_default_branch() {
        let config = setup_test_config(vec!["main".to_string()], "development".to_string());
        let _guard = EnvGuard::new("XDG_CONFIG_HOME");
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_commit_default");

        let tmp = create_test_repo();
        let path = tmp.path();
        git_ops::ensure_branch(path, "development").unwrap();
        git_ops::checkout(path, "development").unwrap();

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(path).unwrap();
        let result = run(&config);
        env::set_current_dir(original_dir).unwrap();
        drop(tmp);

        assert!(result.is_ok());
    }

    #[test]
    fn test_run_auto_checkouts_to_default_branch() {
        let config = setup_test_config(vec!["development".to_string()], "development".to_string());
        let _guard = EnvGuard::new("XDG_CONFIG_HOME");
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_commit_auto_return");

        let tmp = create_test_repo();
        let path = tmp.path();
        git_ops::ensure_branch(path, "development").unwrap();
        git_ops::checkout(path, "master").unwrap();

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(path).unwrap();
        let result = run(&config);
        assert!(result.is_ok());

        let current = git_ops::current_branch(path).unwrap();
        assert_eq!(current, "development");

        env::set_current_dir(original_dir).ok();
        drop(tmp);
    }

    #[test]
    fn test_run_handles_checkout_failure_gracefully() {
        let config = setup_test_config(vec!["development".to_string()], "nonexistent".to_string());
        let _guard = EnvGuard::new("XDG_CONFIG_HOME");
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_commit_failure");

        let tmp = create_test_repo();
        let path = tmp.path();
        git_ops::ensure_branch(path, "development").unwrap();
        git_ops::checkout(path, "master").unwrap();

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(path).unwrap();
        let result = run(&config);
        assert!(result.is_err());
        match result {
            Err(GitHookError::AutoReturnFailed(_)) => (),
            _ => panic!("Expected AutoReturnFailed error"),
        }

        env::set_current_dir(original_dir).ok();
        drop(tmp);
    }
}