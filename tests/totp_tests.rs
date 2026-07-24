use std::os::unix::fs::PermissionsExt;

use git_rusk::cli::{GitignoreLang, InitArgs, TotpAction, TotpArgs};
use git_rusk::commands;
use git_rusk::config::{Config, TotpConfig};
use git_rusk::totp;

use serial_test::serial;

/// A 160-bit Base32 secret (RFC 6238 test key) valid for totp-rs (>=128 bit).
const SECRET_A: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
/// A distinct 160-bit Base32 secret for reset/rotation tests.
const SECRET_B: &str = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP";

fn init_args(secret: Option<&str>, force: bool) -> TotpArgs {
    TotpArgs {
        action: TotpAction::Init {
            force,
            secret: secret.map(String::from),
        },
    }
}

fn reset_args(secret: Option<&str>, force: bool) -> TotpArgs {
    TotpArgs {
        action: TotpAction::Reset {
            force,
            secret: secret.map(String::from),
        },
    }
}

/// Verify `build_totp` computes skew purely from `TotpConfig`, not hard-coded.
///
/// Skew = backward_tolerance_secs / step_seconds. Goes through the real
/// config-driven `build_totp()` path rather than constructing TOTP directly.
#[test]
fn skew_computation_from_config() {
    let code = totp::build_totp(SECRET_A, &TotpConfig::default())
        .unwrap()
        .generate(0);

    let totp_default = totp::build_totp(SECRET_A, &TotpConfig::default()).unwrap();
    assert!(
        totp_default.check(&code, 120),
        "skew=4 (tolerance=120) accepts 120s backward"
    );
    assert!(
        !totp_default.check(&code, 150),
        "skew=4 (tolerance=120) rejects 150s backward"
    );

    let cfg60 = TotpConfig {
        backward_tolerance_secs: 60,
        ..Default::default()
    };
    let totp60 = totp::build_totp(SECRET_A, &cfg60).unwrap();
    assert!(
        totp60.check(&code, 60),
        "skew=2 (tolerance=60) accepts 60s backward"
    );
    assert!(
        !totp60.check(&code, 90),
        "skew=2 (tolerance=60) rejects 90s backward"
    );

    let cfg30 = TotpConfig {
        backward_tolerance_secs: 30,
        ..Default::default()
    };
    let totp30 = totp::build_totp(SECRET_A, &cfg30).unwrap();
    assert!(
        totp30.check(&code, 30),
        "skew=1 (tolerance=30) accepts 30s backward"
    );
    assert!(
        !totp30.check(&code, 60),
        "skew=1 (tolerance=30) rejects 60s backward"
    );
}

/// TOTP-07: global secret file is created with 0o600 and load rejects insecure perms.
#[test]
#[serial]
fn secret_file_permissions() {
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());

    totp::save_secret(SECRET_A).unwrap();
    let path = totp::secret_file_path();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o7777, 0o600, "secret file must be exactly 0o600");

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    let result = totp::load_secret();
    assert!(
        result.is_err(),
        "load_secret must reject insecure (0o644) permissions"
    );

    std::env::remove_var("XDG_CONFIG_HOME");
}

/// TOTP-08: `git-rusk init` never creates or touches the global TOTP secret.
#[test]
#[serial]
fn init_does_not_touch_secret() {
    let secret_tmp = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", secret_tmp.path());

    let repo_tmp = tempfile::tempdir().unwrap();
    let args = InitArgs {
        path: repo_tmp.path().to_path_buf(),
        gitignore: GitignoreLang::None,
    };
    commands::init::run(&args, &Config::default()).unwrap();

    let secret_path = totp::secret_file_path();
    assert!(
        !secret_path.exists(),
        "git-rusk init must never create or touch the global TOTP secret"
    );

    std::env::remove_var("XDG_CONFIG_HOME");
}

/// TOTP-09: `totp init` generates and persists the global secret.
#[test]
#[serial]
fn totp_init_creates_secret() {
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());

    let result = commands::totp::dispatch(&init_args(Some(SECRET_A), false));
    assert!(
        result.is_ok(),
        "totp init should succeed on a clean machine"
    );

    let path = totp::secret_file_path();
    assert!(
        path.exists(),
        "totp init must create the global secret file"
    );

    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o7777, 0o600);

    assert_eq!(totp::load_secret().unwrap(), SECRET_A);

    std::env::remove_var("XDG_CONFIG_HOME");
}

/// TOTP-09: `totp init` refuses to overwrite an existing secret without --force.
#[test]
#[serial]
fn totp_init_refuses_existing() {
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());

    commands::totp::dispatch(&init_args(Some(SECRET_A), false)).unwrap();

    let result = commands::totp::dispatch(&init_args(Some(SECRET_B), false));
    assert!(
        result.is_err(),
        "totp init must refuse to overwrite without --force"
    );
    let msg = result.err().unwrap().to_string();
    assert!(
        msg.contains("already exists"),
        "error should mention 'already exists', got: {msg}"
    );

    assert_eq!(totp::load_secret().unwrap(), SECRET_A);

    std::env::remove_var("XDG_CONFIG_HOME");
}

/// TOTP-10: `totp reset --force` replaces the global secret.
#[test]
#[serial]
fn totp_reset_overwrites() {
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());

    commands::totp::dispatch(&init_args(Some(SECRET_A), false)).unwrap();
    commands::totp::dispatch(&reset_args(Some(SECRET_B), true)).unwrap();

    let saved = totp::load_secret().unwrap();
    assert_eq!(saved, SECRET_B, "totp reset must replace the secret");
    assert_ne!(saved, SECRET_A);

    std::env::remove_var("XDG_CONFIG_HOME");
}

/// TOTP-10: `totp reset` without --force is a safe no-op.
#[test]
#[serial]
fn totp_reset_without_force_is_noop() {
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());

    commands::totp::dispatch(&init_args(Some(SECRET_A), false)).unwrap();
    let result = commands::totp::dispatch(&reset_args(Some(SECRET_B), false));
    assert!(result.is_ok(), "reset without --force should exit cleanly");

    assert_eq!(
        totp::load_secret().unwrap(),
        SECRET_A,
        "secret must be unchanged without --force"
    );

    std::env::remove_var("XDG_CONFIG_HOME");
}

/// TOTP-10: codes generated before a reset are rejected afterwards.
#[test]
#[serial]
fn old_codes_rejected_after_reset() {
    let tmp = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", tmp.path());

    commands::totp::dispatch(&init_args(Some(SECRET_A), false)).unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_code = totp::build_totp(SECRET_A, &TotpConfig::default())
        .unwrap()
        .generate(now);

    commands::totp::dispatch(&reset_args(Some(SECRET_B), true)).unwrap();

    let result = totp::verify_code(&old_code, &TotpConfig::default());
    assert!(result.is_ok());
    assert!(
        !result.unwrap(),
        "old TOTP code must be rejected after reset rotates the secret"
    );

    std::env::remove_var("XDG_CONFIG_HOME");
}
