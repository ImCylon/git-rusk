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
    use serial_test::serial;
    use std::env;
    use std::path::{Path, PathBuf};

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

    fn fixture_on_branch(branch: &str) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        git_ops::init_repo(path).unwrap();
        git_ops::ensure_initial_commit(path).unwrap();
        git_ops::ensure_branch(path, branch).unwrap();
        git_ops::checkout(path, branch).unwrap();
        tmp
    }

    #[test]
    #[serial]
    fn test_run_skips_totp_when_disabled() {
        let mut config = Config::default();
        config.totp.require_for_commit = false;
        config.branches.allowed = vec!["development".to_string()];

        let tmp = fixture_on_branch("development");
        let _dir = DirGuard::enter(tmp.path());

        let result = run(&config);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_run_verifies_totp_when_enabled() {
        let mut config = Config::default();
        config.totp.require_for_commit = true;
        config.branches.allowed = vec!["main".to_string()];

        let tmp = fixture_on_branch("main");
        let _dir = DirGuard::enter(tmp.path());
        let _xdg = EnvGuard::new("XDG_CONFIG_HOME");
        let _totp = EnvGuard::new("TOTP_CODE");
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_pre_commit_totp");
        env::set_var("TOTP_CODE", "123456");

        let result = run(&config);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_run_blocks_commit_on_protected_branch() {
        let mut config = Config::default();
        config.branches.allowed = vec!["development".to_string()];
        config.totp.require_for_commit = false;

        let tmp = fixture_on_branch("main");
        let _dir = DirGuard::enter(tmp.path());

        let result = run(&config);
        assert!(result.is_err());
        match result {
            Err(GitHookError::CommitBlockedOnProtectedBranch { branch, .. }) => {
                assert_eq!(branch, "main");
            }
            other => panic!("Expected CommitBlockedOnProtectedBranch, got {other:?}"),
        }
    }

    #[test]
    #[serial]
    fn test_run_allows_commit_on_allowed_branch() {
        let mut config = Config::default();
        config.branches.allowed = vec!["main".to_string()];
        config.totp.require_for_commit = false;

        let tmp = fixture_on_branch("main");
        let _dir = DirGuard::enter(tmp.path());

        let result = run(&config);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_run_skips_branch_check_for_detached_head() {
        let mut config = Config::default();
        config.totp.require_for_commit = false;

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        git_ops::init_repo(path).unwrap();
        git_ops::ensure_initial_commit(path).unwrap();
        // Path-based detach — avoids racing other tests' CWD before we enter.
        std::process::Command::new("git")
            .args(["-C", path.to_str().unwrap(), "checkout", "--detach"])
            .output()
            .unwrap();

        let _dir = DirGuard::enter(path);
        let result = run(&config);
        assert!(result.is_ok());
    }
}
