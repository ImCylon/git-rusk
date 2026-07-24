use anyhow::Result;
use std::fs;
use std::path::Path;

const HOOKS: &[&str] = &["pre-commit", "commit-msg", "post-checkout"];
const WRAPPER_TEMPLATE: &str = "#!/bin/sh\nexec git-rusk hook {hook_name} \"$@\"\n";

pub fn run(force: bool) -> Result<()> {
    let git_dir = Path::new(".git");
    if !git_dir.exists() {
        return Err(anyhow::anyhow!(
            crate::error::GitHookError::NotAGitRepository
        ));
    }

    let hooks_dir = git_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create hooks directory: {}", e))?;

    for hook_name in HOOKS {
        install_hook(&hooks_dir, hook_name, force)?;
    }

    Ok(())
}

fn install_hook(hooks_dir: &Path, hook_name: &str, force: bool) -> Result<()> {
    let hook_path = hooks_dir.join(hook_name);

    if hook_path.exists() {
        if !force {
            if !contains_git_rusk(&hook_path)? {
                return Err(anyhow::anyhow!(
                    crate::error::GitHookError::HookOverwriteRefused {
                        hook_name: hook_name.to_string(),
                        reason: "file exists and is not a git-rusk hook".to_string(),
                    }
                ));
            }
        } else {
            if is_symlink(&hook_path) {
                return Err(anyhow::anyhow!(crate::error::GitHookError::HookIsSymlink {
                    hook_name: hook_name.to_string(),
                }));
            }
        }
    }

    let wrapper_content = WRAPPER_TEMPLATE.replace("{hook_name}", hook_name);
    fs::write(&hook_path, &wrapper_content).map_err(|e| {
        anyhow::anyhow!(crate::error::GitHookError::HookWriteFailed {
            hook_name: hook_name.to_string(),
            source: e,
        })
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.is_symlink())
        .unwrap_or(false)
}

fn contains_git_rusk(path: &Path) -> Result<bool> {
    let content = fs::read_to_string(path)?;
    Ok(content.contains("git-rusk hook"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_symlink() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let regular_file = tmp_dir.path().join("regular");
        fs::write(&regular_file, "test").unwrap();

        #[cfg(unix)]
        {
            let symlink = tmp_dir.path().join("link");
            std::os::unix::fs::symlink(&regular_file, &symlink).unwrap();
            assert!(is_symlink(&symlink));
            assert!(!is_symlink(&regular_file));
        }

        #[cfg(not(unix))]
        {
            assert!(!is_symlink(&regular_file));
        }
    }

    #[test]
    fn test_contains_git_rusk() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("hook");

        fs::write(&path, "#!/bin/sh\nexec git-rusk hook pre-commit").unwrap();
        assert!(contains_git_rusk(&path).unwrap());

        fs::write(&path, "#!/bin/sh\nexec some-other-tool").unwrap();
        assert!(!contains_git_rusk(&path).unwrap());
    }

    #[test]
    fn test_install_hook_creates_file() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let hooks_dir = tmp_dir.path().join("hooks");
        fs::create_dir_all(&hooks_dir).unwrap();

        install_hook(&hooks_dir, "pre-commit", false).unwrap();

        let hook_path = hooks_dir.join("pre-commit");
        assert!(hook_path.exists());
        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(content.contains("exec git-rusk hook pre-commit"));
    }

    #[test]
    fn test_install_hook_refuses_overwrite() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let hooks_dir = tmp_dir.path().join("hooks");
        fs::create_dir_all(&hooks_dir).unwrap();
        let hook_path = hooks_dir.join("pre-commit");

        fs::write(&hook_path, "#!/bin/sh\necho 'custom hook'").unwrap();

        let result = install_hook(&hooks_dir, "pre-commit", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_install_hook_allows_self_replacement() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let hooks_dir = tmp_dir.path().join("hooks");
        fs::create_dir_all(&hooks_dir).unwrap();
        let hook_path = hooks_dir.join("pre-commit");

        fs::write(&hook_path, "#!/bin/sh\nexec git-rusk hook pre-commit").unwrap();

        let result = install_hook(&hooks_dir, "pre-commit", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_install_hook_overwrites_with_force() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let hooks_dir = tmp_dir.path().join("hooks");
        fs::create_dir_all(&hooks_dir).unwrap();
        let hook_path = hooks_dir.join("pre-commit");

        fs::write(&hook_path, "#!/bin/sh\necho 'custom hook'").unwrap();

        let result = install_hook(&hooks_dir, "pre-commit", true);
        assert!(result.is_ok());
        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(content.contains("exec git-rusk hook pre-commit"));
    }

    #[test]
    fn test_install_hook_refuses_symlink() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let hooks_dir = tmp_dir.path().join("hooks");
        fs::create_dir_all(&hooks_dir).unwrap();
        let hook_path = hooks_dir.join("pre-commit");

        #[cfg(unix)]
        {
            let target = tmp_dir.path().join("target");
            fs::write(&target, "test").unwrap();
            std::os::unix::fs::symlink(&target, &hook_path).unwrap();

            let result = install_hook(&hooks_dir, "pre-commit", true);
            assert!(result.is_err());
        }
    }
}
