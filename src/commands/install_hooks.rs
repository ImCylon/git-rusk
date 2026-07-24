use anyhow::Result;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::error::GitHookError;

pub fn run(force: bool) -> Result<()> {
    if !Path::new(".git").exists() {
        return Err(GitHookError::NotAGitRepository.into());
    }

    let hooks = ["pre-commit", "commit-msg", "post-checkout"];

    for hook_name in &hooks {
        install_single_hook(hook_name, force)?;
    }

    Ok(())
}

fn install_single_hook(hook_name: &str, force: bool) -> Result<()> {
    let hooks_dir = Path::new(".git/hooks");

    if !hooks_dir.exists() {
        fs::create_dir_all(hooks_dir).map_err(|e| GitHookError::HookWriteFailed {
            hook_name: hook_name.to_string(),
            source: e,
        })?;
    }

    let hook_path = hooks_dir.join(hook_name);

    if hook_path.exists() {
        if !force {
            if !contains_git_rusk(&hook_path)? {
                return Err(GitHookError::HookOverwriteRefused {
                    hook_name: hook_name.to_string(),
                    reason: format!(
                        "existing hook does not contain 'git-rusk hook'. Use --force to overwrite."
                    ),
                }
                .into());
            }
        } else {
            if is_symlink(&hook_path) {
                return Err(GitHookError::HookIsSymlink {
                    hook_name: hook_name.to_string(),
                }
                .into());
            }
        }
    }

    let wrapper_script = format!(
        "#!/bin/sh\nexec git-rusk hook {} \"$@\"\n",
        hook_name
    );

    fs::write(&hook_path, wrapper_script).map_err(|e| GitHookError::HookWriteFailed {
        hook_name: hook_name.to_string(),
        source: e,
    })?;

    let mut perms = fs::metadata(&hook_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&hook_path, perms).map_err(|e| GitHookError::HookWriteFailed {
        hook_name: hook_name.to_string(),
        source: e,
    })?;

    eprintln!("Installed .git/hooks/{}", hook_name);

    Ok(())
}

fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata().map(|m| m.is_symlink()).unwrap_or(false)
}

fn contains_git_rusk(path: &Path) -> Result<bool> {
    let content = fs::read_to_string(path)?;
    Ok(content.contains("git-rusk hook"))
}
