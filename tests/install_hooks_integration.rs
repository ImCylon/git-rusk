use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use tempfile::TempDir;

use git_rusk::commands::install_hooks;

#[test]
fn test_integration_install_hooks_creates_executable_wrappers() {
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path();

    Command::new("git")
        .args(["init", repo_path.to_str().unwrap()])
        .output()
        .unwrap();

    std::env::set_current_dir(repo_path).unwrap();
    install_hooks::run(false).unwrap();

    for hook_name in &["pre-commit", "commit-msg", "post-checkout"] {
        let hook_path = repo_path.join(".git").join("hooks").join(hook_name);
        assert!(
            hook_path.exists(),
            "{} hook should exist at {:?}",
            hook_name,
            hook_path
        );

        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(
            content.contains("git-rusk hook"),
            "{} hook should contain 'git-rusk hook'",
            hook_name
        );
        assert!(
            content.contains(hook_name),
            "{} hook should contain its name",
            hook_name
        );

        let metadata = fs::metadata(&hook_path).unwrap();
        let mode = metadata.permissions().mode();
        assert_eq!(
            mode & 0o111,
            0o111,
            "{} hook should be executable",
            hook_name
        );
    }
}

#[test]
fn test_integration_idempotent_install() {
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path();

    Command::new("git")
        .args(["init", repo_path.to_str().unwrap()])
        .output()
        .unwrap();

    std::env::set_current_dir(repo_path).unwrap();
    install_hooks::run(false).unwrap();

    let result = install_hooks::run(false);
    assert!(
        result.is_ok(),
        "Second install should succeed (idempotent): {:?}",
        result
    );
}

#[test]
fn test_integration_refuses_non_git_rusk_hook_without_force() {
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path();

    Command::new("git")
        .args(["init", repo_path.to_str().unwrap()])
        .output()
        .unwrap();

    let hooks_dir = repo_path.join(".git/hooks");
    fs::create_dir_all(&hooks_dir).unwrap();
    fs::write(
        hooks_dir.join("pre-commit"),
        "#!/bin/sh\necho 'custom hook'",
    )
    .unwrap();

    std::env::set_current_dir(repo_path).unwrap();
    let result = install_hooks::run(false);
    assert!(result.is_err());
    let err = result
        .unwrap_err()
        .downcast::<git_rusk::error::GitHookError>()
        .unwrap();
    assert!(
        matches!(
            err,
            git_rusk::error::GitHookError::HookOverwriteRefused { .. }
        ),
        "Should return HookOverwriteRefused error"
    );
}

#[test]
fn test_integration_force_overwrites_custom_hook() {
    let tmp_dir = TempDir::new().unwrap();
    let repo_path = tmp_dir.path();

    Command::new("git")
        .args(["init", repo_path.to_str().unwrap()])
        .output()
        .unwrap();

    let hooks_dir = repo_path.join(".git/hooks");
    fs::create_dir_all(&hooks_dir).unwrap();
    fs::write(hooks_dir.join("pre-commit"), "#!/bin/sh\necho 'old hook'").unwrap();

    std::env::set_current_dir(repo_path).unwrap();
    install_hooks::run(true).unwrap();

    let hook_path = hooks_dir.join("pre-commit");
    let content = fs::read_to_string(&hook_path).unwrap();
    assert!(content.contains("git-rusk hook pre-commit"));
    assert!(!content.contains("old hook"));
}
