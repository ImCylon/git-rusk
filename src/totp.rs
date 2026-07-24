use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use totp_rs::{Algorithm, Secret, TOTP};

use crate::config::TotpConfig;
use crate::error::GitHookError;

/// Resolve the global TOTP secret file path.
///
/// Checks `$XDG_CONFIG_HOME` first, then falls back to `$HOME/.config`.
/// The secret always lives at `<config_dir>/git-rusk/totp-secret`.
pub fn secret_file_path() -> PathBuf {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|_| PathBuf::from(".config"));
    config_dir.join("git-rusk").join("totp-secret")
}

/// Read the global secret, verifying file permissions are exactly 0o600.
///
/// Returns the trimmed Base32-encoded secret string.
pub fn load_secret() -> Result<String, GitHookError> {
    let path = secret_file_path();

    if !path.exists() {
        return Err(GitHookError::TotpSecretNotFound {
            path: path.display().to_string(),
        });
    }

    let metadata = std::fs::metadata(&path).map_err(|source| GitHookError::TotpSecretRead {
        path: path.display().to_string(),
        source,
    })?;

    let mode = metadata.permissions().mode();
    if mode & 0o777 != 0o600 {
        return Err(GitHookError::TotpSecretInsecurePerms {
            path: path.display().to_string(),
            mode: format!("{:o}", mode & 0o777),
        });
    }

    let secret = std::fs::read_to_string(&path).map_err(|source| GitHookError::TotpSecretRead {
        path: path.display().to_string(),
        source,
    })?;

    Ok(secret.trim().to_string())
}

/// Write the Base32 secret to the global file with chmod 0o600.
///
/// Creates parent directories if they do not exist.
pub fn save_secret(base32_secret: &str) -> Result<(), GitHookError> {
    let path = secret_file_path();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| GitHookError::TotpSecretWrite {
            path: parent.display().to_string(),
            source,
        })?;
    }

    std::fs::write(&path, base32_secret).map_err(|source| GitHookError::TotpSecretWrite {
        path: path.display().to_string(),
        source,
    })?;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).map_err(|source| {
        GitHookError::TotpSecretWrite {
            path: path.display().to_string(),
            source,
        }
    })?;

    Ok(())
}

/// Build a `TOTP` verifier from a Base32 secret and config.
///
/// The skew is computed from `backward_tolerance_secs / step_seconds`.
/// totp-rs skew is symmetric (accepts N steps backward AND forward).
/// The returned `TOTP` instance auto-zeroizes its secret field on Drop
/// when the `zeroize` feature is enabled.
pub fn build_totp(base32_secret: &str, config: &TotpConfig) -> Result<TOTP, GitHookError> {
    let secret = Secret::Encoded(base32_secret.to_string());
    let secret_bytes = secret
        .to_bytes()
        .map_err(|e| GitHookError::TotpSecretInvalid {
            message: e.to_string(),
        })?;

    let skew = (config.backward_tolerance_secs / config.step_seconds) as u8;

    TOTP::new(
        Algorithm::SHA1,
        6,
        skew,
        config.step_seconds as u64,
        secret_bytes,
        Some("git-rusk".to_string()),
        "git-rusk".to_string(),
    )
    .map_err(|e| GitHookError::TotpConstruction {
        message: e.to_string(),
    })
}

/// Verify a TOTP code against the global secret using system time.
///
/// Returns `Ok(true)` if valid, `Ok(false)` if invalid, `Err` on system errors.
pub fn verify_code(code: &str, config: &TotpConfig) -> Result<bool, GitHookError> {
    let base32_secret = load_secret()?;
    let totp = build_totp(&base32_secret, config)?;
    totp.check_current(code)
        .map_err(|e| GitHookError::TotpSystemTime {
            message: e.to_string(),
        })
}

/// Verify a TOTP code from the `TOTP_CODE` environment variable.
///
/// Returns `Err(TotpCodeNotSet)` when the variable is missing.
pub fn verify_from_env(config: &TotpConfig) -> Result<bool, GitHookError> {
    let code = std::env::var("TOTP_CODE").map_err(|_| GitHookError::TotpCodeNotSet)?;
    verify_code(&code, config)
}

/// Display data for the `totp init` / `totp reset` commands.
pub struct SecretDisplay {
    /// The Base32-encoded secret string.
    pub base32_secret: String,
    /// The otpauth:// URI for QR code generation.
    pub otpauth_url: String,
}

/// Generate a new CSPRNG 160-bit secret, save it globally, and return display data.
pub fn generate_and_save_secret() -> Result<SecretDisplay, GitHookError> {
    let secret = Secret::generate_secret();
    let encoded = secret.to_encoded();
    let base32 = match &encoded {
        Secret::Encoded(s) => s.clone(),
        _ => unreachable!("to_encoded always returns Encoded"),
    };

    save_secret(&base32)?;

    let totp = build_totp(&base32, &TotpConfig::default())?;

    Ok(SecretDisplay {
        base32_secret: base32,
        otpauth_url: totp.get_url(),
    })
}

/// Save a user-provided Base32 secret globally and return display data.
pub fn save_and_display_secret(base32_secret: &str) -> Result<SecretDisplay, GitHookError> {
    let totp = build_totp(base32_secret, &TotpConfig::default())?;
    save_secret(base32_secret)?;
    Ok(SecretDisplay {
        base32_secret: base32_secret.to_string(),
        otpauth_url: totp.get_url(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::os::unix::fs::PermissionsExt;

    const RFC_SECRET: &[u8] = b"12345678901234567890";

    fn make_totp(skew: u8, digits: usize) -> TOTP {
        TOTP::new(
            Algorithm::SHA1,
            digits,
            skew,
            30,
            RFC_SECRET.to_vec(),
            Some("test".to_string()),
            "test".to_string(),
        )
        .unwrap()
    }

    #[test]
    fn test_rfc6238_sha1_8digit_vectors() {
        let totp = make_totp(0, 8);
        assert_eq!(totp.generate(59), "94287082");
        assert_eq!(totp.generate(1111111109), "07081804");
        assert_eq!(totp.generate(1111111111), "14050471");
        assert_eq!(totp.generate(1234567890), "89005924");
        assert_eq!(totp.generate(2000000000), "69279037");
        assert_eq!(totp.generate(20000000000), "65353130");
    }

    #[test]
    fn test_rfc6238_sha1_6digit_vectors() {
        let totp = make_totp(0, 6);
        assert_eq!(totp.generate(59), "287082");
        assert_eq!(totp.generate(1111111109), "081804");
        assert_eq!(totp.generate(1111111111), "050471");
    }

    #[test]
    fn test_skew_accepts_120s_backward() {
        let totp = make_totp(4, 6);
        let code = totp.generate(0);
        assert!(totp.check(&code, 120));
    }

    #[test]
    fn test_skew_rejects_150s_backward() {
        let totp = make_totp(4, 6);
        let code = totp.generate(0);
        assert!(!totp.check(&code, 150));
    }

    #[test]
    fn test_check_valid_code_at_current_time() {
        let totp = make_totp(0, 6);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let code = totp.generate(now);
        assert!(totp.check(&code, now));
    }

    #[test]
    fn test_check_invalid_code_returns_false() {
        let totp = make_totp(0, 6);
        assert!(!totp.check("000000", 1234567890));
    }

    #[test]
    fn test_build_totp_default_skew_is_4() {
        let result = build_totp("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ", &TotpConfig::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_totp_custom_skew_from_config() {
        let config = TotpConfig {
            backward_tolerance_secs: 60,
            ..Default::default()
        };
        let totp = build_totp("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ", &config).unwrap();
        let code = totp.generate(0);
        assert!(totp.check(&code, 60));
        assert!(!totp.check(&code, 90));
    }

    #[test]
    #[serial]
    fn test_secret_file_path_with_xdg_config_home() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg_path = tmp.path().to_str().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", xdg_path);
        let path = secret_file_path();
        assert_eq!(
            path,
            std::path::PathBuf::from(format!("{}/git-rusk/totp-secret", xdg_path))
        );
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[serial]
    fn test_secret_file_path_falls_back_to_home() {
        std::env::remove_var("XDG_CONFIG_HOME");
        let tmp = tempfile::tempdir().unwrap();
        let home_path = tmp.path().to_str().unwrap();
        std::env::set_var("HOME", home_path);
        let path = secret_file_path();
        assert_eq!(
            path,
            std::path::PathBuf::from(format!("{}/.config/git-rusk/totp-secret", home_path))
        );
        std::env::remove_var("HOME");
    }

    #[test]
    #[serial]
    fn test_save_secret_creates_file_with_0600_perms() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        save_secret("JBSWY3DPEHPK3PXP").unwrap();
        let path = secret_file_path();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o7777, 0o600);
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[serial]
    fn test_save_secret_creates_parent_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        let parent = tmp.path().join("git-rusk");
        assert!(!parent.exists());
        save_secret("JBSWY3DPEHPK3PXP").unwrap();
        assert!(parent.exists());
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[serial]
    fn test_load_secret_missing_file_returns_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        let result = load_secret();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GitHookError::TotpSecretNotFound { .. }
        ));
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[serial]
    fn test_load_secret_rejects_0644_perms() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        let path = secret_file_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "JBSWY3DPEHPK3PXP").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let result = load_secret();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GitHookError::TotpSecretInsecurePerms { .. }
        ));
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[serial]
    fn test_load_secret_rejects_0640_perms() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        let path = secret_file_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "JBSWY3DPEHPK3PXP").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        let result = load_secret();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GitHookError::TotpSecretInsecurePerms { .. }
        ));
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[serial]
    fn test_load_secret_0600_returns_trimmed_content() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        let path = secret_file_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "  JBSWY3DPEHPK3PXP\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let result = load_secret();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "JBSWY3DPEHPK3PXP");
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[serial]
    fn test_save_load_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        save_secret("JBSWY3DPEHPK3PXP").unwrap();
        let loaded = load_secret().unwrap();
        assert_eq!(loaded, "JBSWY3DPEHPK3PXP");
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[serial]
    fn test_verify_from_env_unset_returns_code_not_set() {
        std::env::remove_var("TOTP_CODE");
        let result = verify_from_env(&TotpConfig::default());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GitHookError::TotpCodeNotSet));
    }

    #[test]
    #[serial]
    fn test_verify_code_wrong_code_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        save_secret("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ").unwrap();
        let result = verify_code("000000", &TotpConfig::default());
        assert!(result.is_ok());
        assert!(!result.unwrap());
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
