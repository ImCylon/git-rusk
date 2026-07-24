use crate::config::Config;
use crate::error::GitHookError;
use crate::totp;

pub fn run(config: &Config) -> Result<(), GitHookError> {
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
}