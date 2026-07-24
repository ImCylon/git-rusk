use crate::error::GitHookError;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

fn git(path: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute git {:?}", args))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_success(path: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Returns true if `path` contains a `.git` directory or file.
pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

/// Initialize a git repository at `path` if one does not already exist.
///
/// `git init` is inherently idempotent, but we check first to produce
/// clearer user-facing output.
pub fn init_repo(path: &Path) -> Result<()> {
    if is_git_repo(path) {
        return Ok(());
    }
    git(path, &["init"])?;
    Ok(())
}

/// Returns the symbolic-ref branch name HEAD currently points to.
///
/// Works on unborn HEAD (returns the branch name even with zero commits).
pub fn current_branch(path: &Path) -> Result<String> {
    git(path, &["symbolic-ref", "--short", "HEAD"])
}

/// Returns `true` if HEAD is born (at least one commit exists).
pub fn has_commits(path: &Path) -> bool {
    git_success(path, &["rev-parse", "--verify", "HEAD"])
}

/// Ensure HEAD points to `main`.
///
/// - If already `main`, does nothing.
/// - If on `master` with unborn HEAD: uses `git symbolic-ref` (safe rename).
/// - If on `master` with commits: uses `git branch -m` (rename in place).
/// - If on any other branch, leaves it unchanged.
pub fn ensure_main_branch(path: &Path) -> Result<()> {
    let current = current_branch(path)?;

    if current == "main" {
        return Ok(());
    }

    if current == "master" {
        if has_commits(path) {
            git(path, &["branch", "-m", "main"])?;
        } else {
            git(path, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
        }
    }

    Ok(())
}

/// Create an initial empty commit if the repo has no commits yet.
///
/// Skips if HEAD is already born (prevents duplicate commits on re-run).
/// Sets local `user.name`/`user.email` as fallback if missing.
/// Uses `--no-gpg-sign` to bypass systems with `commit.gpgsign=true`.
pub fn ensure_initial_commit(path: &Path) -> Result<()> {
    if has_commits(path) {
        return Ok(());
    }

    if !git_success(path, &["config", "user.name"]) {
        git(path, &["config", "user.name", "git-rusk"])?;
    }
    if !git_success(path, &["config", "user.email"]) {
        git(path, &["config", "user.email", "git-rusk@local"])?;
    }

    git(
        path,
        &[
            "commit",
            "--allow-empty",
            "--no-verify",
            "--no-gpg-sign",
            "-m",
            "Initial commit",
        ],
    )?;
    Ok(())
}

/// Create branch `name` if it does not already exist.
///
/// Must only be called AFTER [`ensure_initial_commit`] — git cannot
/// create branches on an unborn HEAD.
pub fn ensure_branch(path: &Path, name: &str) -> Result<()> {
    let ref_path = format!("refs/heads/{name}");
    if git_success(path, &["show-ref", "--verify", "--quiet", &ref_path]) {
        return Ok(());
    }
    git(path, &["branch", name])?;
    Ok(())
}

/// Checkout branch `name`. Must be called after [`ensure_branch`] for the target.
pub fn checkout(path: &Path, name: &str) -> Result<()> {
    git(path, &["checkout", name])?;
    Ok(())
}

/// Returns the current branch name using `git rev-parse --abbrev-ref HEAD`.
///
/// Returns "HEAD" for detached HEAD state.
pub fn get_current_branch() -> Result<String, GitHookError> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| GitHookError::GitOperation(format!("Failed to execute git rev-parse: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitHookError::GitOperation(format!(
            "git rev-parse --abbrev-ref HEAD failed: {}",
            stderr.trim()
        )));
    }

    let branch = String::from_utf8(output.stdout)
        .map_err(|e| GitHookError::GitOperation(format!("Failed to parse git output: {}", e)))?;
    Ok(branch.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_creates_git_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        assert!(!is_git_repo(path));
        init_repo(path).unwrap();
        assert!(is_git_repo(path));
    }

    #[test]
    fn test_ensure_initial_commit_creates_exactly_one() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        init_repo(path).unwrap();
        ensure_initial_commit(path).unwrap();
        let count = git(path, &["rev-list", "--count", "HEAD"]).unwrap();
        assert_eq!(count, "1");

        ensure_initial_commit(path).unwrap();
        let count2 = git(path, &["rev-list", "--count", "HEAD"]).unwrap();
        assert_eq!(count2, "1");
    }

    #[test]
    fn test_ensure_branch_and_checkout() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        init_repo(path).unwrap();
        ensure_initial_commit(path).unwrap();
        ensure_branch(path, "test-branch").unwrap();
        checkout(path, "test-branch").unwrap();
        assert_eq!(current_branch(path).unwrap(), "test-branch");
    }

    #[test]
    fn test_get_current_branch() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        init_repo(path).unwrap();
        ensure_initial_commit(path).unwrap();
        ensure_branch(path, "feature/test").unwrap();
        checkout(path, "feature/test").unwrap();
        assert_eq!(get_current_branch().unwrap(), "feature/test");
    }

    #[test]
    fn test_get_current_branch_detached_head() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path();
        init_repo(path).unwrap();
        ensure_initial_commit(path).unwrap();

        std::env::set_current_dir(path).unwrap();
        let output = Command::new("git")
            .args(["checkout", "--detach"])
            .output()
            .unwrap();
        assert!(output.status.success(), "git checkout --detach failed");

        assert_eq!(get_current_branch().unwrap(), "HEAD");
    }
}
