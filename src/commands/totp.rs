use anyhow::Result;

use crate::cli::{TotpAction, TotpArgs};
use crate::config::TotpConfig;
use crate::error::GitHookError;
use crate::totp;

pub fn dispatch(args: &TotpArgs) -> Result<()> {
    match &args.action {
        TotpAction::Init { force, secret } => run_init(*force, secret.as_deref()),
        TotpAction::Show => run_show(),
        TotpAction::Reset { force, secret } => run_reset(*force, secret.as_deref()),
    }
}

/// Generate and save a new global TOTP secret, then display it.
///
/// Refuses to overwrite an existing secret unless `force` is `true`.
fn run_init(force: bool, secret: Option<&str>) -> Result<()> {
    let path = totp::secret_file_path();

    if path.exists() && !force {
        return Err(GitHookError::TotpSecretAlreadyExists.into());
    }

    let display = if let Some(custom) = secret {
        totp::save_and_display_secret(custom)?
    } else {
        totp::generate_and_save_secret()?
    };

    println!("TOTP secret generated and saved to: {}", path.display());
    println!();
    println!("Add this to your authenticator app:");
    println!("  Base32 secret: {}", display.base32_secret);
    println!();
    println!("Or scan this otpauth URI with a QR generator:");
    println!("  {}", display.otpauth_url);
    println!();
    println!("This secret is shared across ALL repositories on this machine.");
    println!("You only need to do this ONCE.");

    Ok(())
}

/// Display the existing global TOTP secret and otpauth URI.
fn run_show() -> Result<()> {
    let base32_secret = totp::load_secret()?;
    let totp_instance = totp::build_totp(&base32_secret, &TotpConfig::default())?;

    println!("TOTP Secret: {}", base32_secret);
    println!("otpauth URI: {}", totp_instance.get_url());

    Ok(())
}

/// Rotate the global TOTP secret, invalidating all previous codes.
///
/// Without `--force`, prints a warning to stderr and exits cleanly
/// without performing any rotation.
fn run_reset(force: bool, secret: Option<&str>) -> Result<()> {
    if !force {
        eprintln!("WARNING: This will invalidate ALL TOTP codes across ALL repositories.");
        eprintln!("Use --force to confirm.");
        return Ok(());
    }

    let display = if let Some(custom) = secret {
        totp::save_and_display_secret(custom)?
    } else {
        totp::generate_and_save_secret()?
    };

    println!("TOTP secret rotated. Old codes are now invalid.");
    println!();
    println!("New Base32 secret: {}", display.base32_secret);
    println!("New otpauth URI: {}", display.otpauth_url);
    println!();
    println!("Update your authenticator app with the new secret.");

    Ok(())
}
