use anyhow::Result;

use crate::config::Config;
use crate::error::GitHookError;
use crate::totp;

pub fn run(config: &Config) -> Result<()> {
    if config.totp.require_for_commit {
        let valid = totp::verify_from_env(&config.totp).map_err(|e| {
            if matches!(e, GitHookError::TotpCodeNotSet) {
                GitHookError::TotpCodeNotSet
            } else {
                e
            }
        })?;

        if !valid {
            return Err(GitHookError::TotpSystemTime {
                message: "Invalid TOTP code".to_string(),
            }
            .into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, TotpConfig};
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_run_requires_totp_when_enabled() {
        let config = Config {
            totp: TotpConfig {
                require_for_commit: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());

        crate::totp::save_secret("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ").unwrap();
        std::env::remove_var("TOTP_CODE");

        let result = run(&config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().downcast::<GitHookError>().unwrap(),
            GitHookError::TotpCodeNotSet
        ));

        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[serial]
    fn test_run_skips_totp_when_disabled() {
        let config = Config {
            totp: TotpConfig {
                require_for_commit: false,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = run(&config);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_run_rejects_invalid_totp_code() {
        let config = Config {
            totp: TotpConfig {
                require_for_commit: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());

        crate::totp::save_secret("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ").unwrap();
        std::env::set_var("TOTP_CODE", "000000");

        let result = run(&config);
        assert!(result.is_err());

        std::env::remove_var("TOTP_CODE");
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
