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

    if config.totp.require_for_branch_switch
        && !branch_protection::is_allowed_branch(
            &current_branch,
            &config.branches.allowed,
        )
    {
        if let Err(e) = totp::verify_from_env(&config.totp) {
            let _ = std::process::Command::new("git")
                .args(["checkout", &config.branches.default_branch])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            return Err(e);
        }
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
        config.totp.require_for_branch_switch = false;

        let tmp = fixture_on_branch("main");
        let _dir = DirGuard::enter(tmp.path());

        let result = run("abc123".to_string(), "def456".to_string(), 1, &config);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_run_verifies_totp_when_enabled() {
        let mut config = Config::default();
        config.totp.require_for_branch_switch = true;
        config.branches.default_branch = "main".to_string();
        config.branches.allowed = vec!["main".to_string()];

        // Current branch is protected (not default, not allowed) → TOTP required.
        let tmp = fixture_on_branch("release");
        let _dir = DirGuard::enter(tmp.path());
        let _xdg = EnvGuard::new("XDG_CONFIG_HOME");
        let _code = EnvGuard::new("TOTP_CODE");
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_checkout_totp");
        env::set_var("TOTP_CODE", "123456");

        let result = run("abc123".to_string(), "def456".to_string(), 1, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_skips_totp_for_file_checkout() {
        let mut config = Config::default();
        config.totp.require_for_branch_switch = true;
        // branch_switch == 0 → early return, no git calls
        let result = run("abc123".to_string(), "def456".to_string(), 0, &config);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_run_skips_totp_for_default_branch() {
        let mut config = Config::default();
        config.totp.require_for_branch_switch = true;
        config.branches.default_branch = "main".to_string();

        let tmp = fixture_on_branch("main");
        let _dir = DirGuard::enter(tmp.path());

        let result = run("abc123".to_string(), "def456".to_string(), 1, &config);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_run_verifies_totp_for_protected_branch() {
        let mut config = Config::default();
        config.totp.require_for_branch_switch = true;
        config.branches.default_branch = "development".to_string();
        config.branches.allowed = vec!["development".to_string()];

        let tmp = fixture_on_branch("main");
        let _dir = DirGuard::enter(tmp.path());
        let _xdg = EnvGuard::new("XDG_CONFIG_HOME");
        let _code = EnvGuard::new("TOTP_CODE");
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_post_checkout_totp");
        env::set_var("TOTP_CODE", "123456");

        let result = run("abc123".to_string(), "def456".to_string(), 1, &config);
        assert!(result.is_err());
    }
}
