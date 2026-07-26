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
        return Err(GitHookError::AutoReturnFailed(std::io::Error::other(e)));
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
    use serial_test::serial;
    use std::env;
    use std::path::PathBuf;

    /// Restores CWD on drop so parallel/serial tests cannot leave a poisoned cwd.
    struct DirGuard {
        original: PathBuf,
    }

    impl DirGuard {
        fn enter(path: &Path) -> Self {
            let original = env::current_dir()
                .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
            env::set_current_dir(path).expect("set_current_dir");
            Self { original }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            if env::set_current_dir(&self.original).is_err() {
                let _ = env::set_current_dir(env!("CARGO_MANIFEST_DIR"));
            }
        }
    }

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

    /// Isolated repo with an explicit default branch and a protected working branch.
    ///
    /// Avoids hardcoding `master`/`main` — `git init` default differs across systems.
    fn create_fixture_repo(default_branch: &str, working_branch: &str) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        git_ops::init_repo(path).unwrap();
        git_ops::ensure_initial_commit(path).unwrap();
        git_ops::ensure_branch(path, default_branch).unwrap();
        git_ops::ensure_branch(path, working_branch).unwrap();
        git_ops::checkout(path, working_branch).unwrap();
        tmp
    }

    fn setup_test_config(allowed: Vec<String>, default_branch: String) -> Config {
        let mut config = Config::default();
        config.branches.allowed = allowed;
        config.branches.default_branch = default_branch;
        config
    }

    #[test]
    #[serial]
    fn test_run_does_nothing_on_allowed_branch() {
        let config = setup_test_config(vec!["feature".to_string()], "main".to_string());
        let _env = EnvGuard::new("XDG_CONFIG_HOME");
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_commit_allowed");

        let tmp = create_fixture_repo("main", "feature");
        let _dir = DirGuard::enter(tmp.path());

        let result = run(&config);
        assert!(result.is_ok());
        assert_eq!(git_ops::get_current_branch().unwrap(), "feature");
    }

    #[test]
    #[serial]
    fn test_run_does_nothing_on_default_branch() {
        // Allowed list does not include development — but current branch IS the
        // default branch, so auto-return must still be a no-op.
        let config = setup_test_config(vec!["main".to_string()], "development".to_string());
        let _env = EnvGuard::new("XDG_CONFIG_HOME");
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_commit_default");

        let tmp = create_fixture_repo("main", "development");
        let _dir = DirGuard::enter(tmp.path());

        let result = run(&config);
        assert!(result.is_ok());
        assert_eq!(git_ops::get_current_branch().unwrap(), "development");
    }

    #[test]
    #[serial]
    fn test_run_auto_checkouts_to_default_branch() {
        let config = setup_test_config(vec!["development".to_string()], "development".to_string());
        let _env = EnvGuard::new("XDG_CONFIG_HOME");
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_commit_auto_return");

        // Working branch is protected (not in allowed); default exists as a real ref.
        let tmp = create_fixture_repo("development", "protected");
        let path = tmp.path();
        let _dir = DirGuard::enter(path);

        assert_eq!(git_ops::get_current_branch().unwrap(), "protected");
        let result = run(&config);
        assert!(result.is_ok(), "auto-return should succeed: {result:?}");
        assert_eq!(git_ops::get_current_branch().unwrap(), "development");
    }

    #[test]
    #[serial]
    fn test_run_handles_checkout_failure_gracefully() {
        let config = setup_test_config(vec!["development".to_string()], "nonexistent".to_string());
        let _env = EnvGuard::new("XDG_CONFIG_HOME");
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_commit_failure");

        let tmp = create_fixture_repo("development", "protected");
        let _dir = DirGuard::enter(tmp.path());

        let result = run(&config);
        assert!(result.is_err());
        match result {
            Err(GitHookError::AutoReturnFailed(_)) => (),
            other => panic!("Expected AutoReturnFailed error, got {other:?}"),
        }
        // Still on the protected branch after failed auto-return
        assert_eq!(git_ops::get_current_branch().unwrap(), "protected");
    }
}
