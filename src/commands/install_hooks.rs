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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_symlink_returns_false_for_regular_file() {
        let tmp_dir = TempDir::new().unwrap();
        let file_path = tmp_dir.path().join("test_file.txt");
        fs::write(&file_path, "content").unwrap();
        assert!(!is_symlink(&file_path));
    }

    #[test]
    fn test_is_symlink_returns_true_for_symlink() {
        let tmp_dir = TempDir::new().unwrap();
        let target = tmp_dir.path().join("target.txt");
        fs::write(&target, "content").unwrap();
        let link_path = tmp_dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link_path).unwrap();
        assert!(is_symlink(&link_path));
    }

    #[test]
    fn test_contains_git_rusk_returns_true_when_present() {
        let tmp_dir = TempDir::new().unwrap();
        let file_path = tmp_dir.path().join("hook.sh");
        fs::write(&file_path, "#!/bin/sh\nexec git-rusk hook pre-commit \"$@\"")
            .unwrap();
        assert!(contains_git_rusk(&file_path).unwrap());
    }

    #[test]
    fn test_contains_git_rusk_returns_false_when_absent() {
        let tmp_dir = TempDir::new().unwrap();
        let file_path = tmp_dir.path().join("hook.sh");
        fs::write(&file_path, "#!/bin/sh\necho 'custom hook'").unwrap();
        assert!(!contains_git_rusk(&file_path).unwrap());
    }

    #[test]
    fn test_run_not_in_git_repo_errors() {
        let tmp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(tmp_dir.path()).unwrap();
        let result = run(false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().downcast::<GitHookError>(),
            Ok(GitHookError::NotAGitRepository)
        ));
    }

    #[test]
    fn test_run_creates_missing_hooks_directory() {
        let tmp_dir = TempDir::new().unwrap();
        let hooks_dir = tmp_dir.path().join(".git/hooks");
        assert!(!hooks_dir.exists());

        std::env::set_current_dir(tmp_dir.path()).unwrap();
        fs::create_dir_all(tmp_dir.path().join(".git")).unwrap();

        let result = run(false);
        assert!(result.is_ok());
        assert!(hooks_dir.exists());
    }

    #[test]
    fn test_run_refuses_non_git_rusk_hook_without_force() {
        let tmp_dir = TempDir::new().unwrap();
        let hooks_dir = tmp_dir.path().join(".git/hooks");
        fs::create_dir_all(&hooks_dir).unwrap();
        fs::write(hooks_dir.join("pre-commit"), "#!/bin/sh\necho 'custom'")
            .unwrap();

        std::env::set_current_dir(tmp_dir.path()).unwrap();
        let result = run(false);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().downcast::<GitHookError>(),
            Ok(GitHookError::HookOverwriteRefused { .. })
        ));
    }

    #[test]
    fn test_run_allows_self_replacement_without_force() {
        let tmp_dir = TempDir::new().unwrap();
        let hooks_dir = tmp_dir.path().join(".git/hooks");
        fs::create_dir_all(&hooks_dir).unwrap();
        fs::write(
            hooks_dir.join("pre-commit"),
            "#!/bin/sh\nexec git-rusk hook pre-commit \"$@\"",
        )
        .unwrap();

        std::env::set_current_dir(tmp_dir.path()).unwrap();
        let result = run(false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_overwrites_with_force() {
        let tmp_dir = TempDir::new().unwrap();
        let hooks_dir = tmp_dir.path().join(".git/hooks");
        fs::create_dir_all(&hooks_dir).unwrap();
        fs::write(hooks_dir.join("pre-commit"), "#!/bin/sh\necho 'old'")
            .unwrap();

        std::env::set_current_dir(tmp_dir.path()).unwrap();
        let result = run(true);
        assert!(result.is_ok());

        let content = fs::read_to_string(hooks_dir.join("pre-commit")).unwrap();
        assert!(content.contains("git-rusk hook pre-commit"));
    }

    #[test]
    fn test_run_refuses_symlink_with_force() {
        let tmp_dir = TempDir::new().unwrap();
        let hooks_dir = tmp_dir.path().join(".git/hooks");
        fs::create_dir_all(&hooks_dir).unwrap();
        let target = tmp_dir.path().join("target");
        fs::write(&target, "content").unwrap();
        let symlink = hooks_dir.join("pre-commit");
        std::os::unix::fs::symlink(&target, &symlink).unwrap();

        std::env::set_current_dir(tmp_dir.path()).unwrap();
        let result = run(true);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().downcast::<GitHookError>(),
            Ok(GitHookError::HookIsSymlink { .. })
        ));
    }

    #[test]
    fn test_run_sets_executable_permissions() {
        let tmp_dir = TempDir::new().unwrap();
        fs::create_dir_all(tmp_dir.path().join(".git")).unwrap();

        std::env::set_current_dir(tmp_dir.path()).unwrap();
        run(true).unwrap();

        let hook_path = tmp_dir.path().join(".git/hooks/pre-commit");
        let metadata = fs::metadata(&hook_path).unwrap();
        let mode = metadata.permissions().mode();
        assert_eq!(mode & 0o755, 0o755);
    }

    #[test]
    fn test_run_idempotent() {
        let tmp_dir = TempDir::new().unwrap();
        fs::create_dir_all(tmp_dir.path().join(".git")).unwrap();

        std::env::set_current_dir(tmp_dir.path()).unwrap();
        run(false).unwrap();
        let result = run(false);
        assert!(result.is_ok());
    }
}
