use std::fs;
use std::path::Path;
use std::process::Command;

use git_rusk::cli::{GitignoreLang, InitArgs};
use git_rusk::commands::init;
use git_rusk::config::Config;

fn init_in_tempdir(gitignore: GitignoreLang) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let args = InitArgs {
        path: tmp.path().to_path_buf(),
        gitignore,
    };
    init::run(&args, &Config::default()).expect("init::run should succeed");
    tmp
}

fn git_output(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"));
    assert!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn init_creates_repo() {
    let tmp = init_in_tempdir(GitignoreLang::None);
    assert!(tmp.path().join(".git").exists());
}

#[test]
fn init_idempotent_rerun() {
    let tmp = tempfile::tempdir().unwrap();
    let args = InitArgs {
        path: tmp.path().to_path_buf(),
        gitignore: GitignoreLang::None,
    };
    init::run(&args, &Config::default()).unwrap();
    init::run(&args, &Config::default()).unwrap();
    assert!(tmp.path().join(".git").exists());
}

#[test]
fn renames_master_to_main() {
    let tmp = init_in_tempdir(GitignoreLang::None);
    let branch = git_output(tmp.path(), &["symbolic-ref", "--short", "HEAD"]);
    assert_eq!(branch, "development");
    let ref_check = Command::new("git")
        .arg("-C")
        .arg(tmp.path())
        .args(["show-ref", "--verify", "--quiet", "refs/heads/main"])
        .output()
        .unwrap();
    assert!(
        ref_check.status.success(),
        "refs/heads/main should exist after init"
    );
}

#[test]
fn skip_rename_if_already_main() {
    let tmp = init_in_tempdir(GitignoreLang::None);
    let args = InitArgs {
        path: tmp.path().to_path_buf(),
        gitignore: GitignoreLang::None,
    };
    init::run(&args, &Config::default()).expect("second init should not error");
    let ref_check = Command::new("git")
        .arg("-C")
        .arg(tmp.path())
        .args(["show-ref", "--verify", "--quiet", "refs/heads/main"])
        .output()
        .unwrap();
    assert!(ref_check.status.success());
}

#[test]
fn creates_initial_commit() {
    let tmp = init_in_tempdir(GitignoreLang::None);
    let count = git_output(tmp.path(), &["rev-list", "--count", "HEAD"]);
    assert_eq!(count, "1");
}

#[test]
fn no_duplicate_commit() {
    let tmp = tempfile::tempdir().unwrap();
    let args = InitArgs {
        path: tmp.path().to_path_buf(),
        gitignore: GitignoreLang::None,
    };
    init::run(&args, &Config::default()).unwrap();
    init::run(&args, &Config::default()).unwrap();
    let count = git_output(tmp.path(), &["rev-list", "--count", "HEAD"]);
    assert_eq!(count, "1");
}

#[test]
fn development_branch_exists() {
    let tmp = init_in_tempdir(GitignoreLang::None);
    let check = Command::new("git")
        .arg("-C")
        .arg(tmp.path())
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            "refs/heads/development",
        ])
        .output()
        .unwrap();
    assert!(check.status.success());
}

#[test]
fn release_branch_exists() {
    let tmp = init_in_tempdir(GitignoreLang::None);
    let check = Command::new("git")
        .arg("-C")
        .arg(tmp.path())
        .args(["show-ref", "--verify", "--quiet", "refs/heads/release"])
        .output()
        .unwrap();
    assert!(check.status.success());
}

#[test]
fn checkout_development() {
    let tmp = init_in_tempdir(GitignoreLang::None);
    let branch = git_output(tmp.path(), &["symbolic-ref", "--short", "HEAD"]);
    assert_eq!(branch, "development");
}

#[test]
fn readme_generated() {
    let tmp = init_in_tempdir(GitignoreLang::None);
    let readme_path = tmp.path().join("README.md");
    assert!(readme_path.exists(), "README.md should exist after init");
    let content = fs::read_to_string(&readme_path).unwrap();
    let dir_name = tmp
        .path()
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    assert!(
        content.contains(dir_name),
        "README should contain directory name '{dir_name}', got: {content}"
    );
}

#[test]
fn gitignore_rust() {
    let tmp = init_in_tempdir(GitignoreLang::Rust);
    let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert!(
        content.contains("/target"),
        "rust .gitignore should contain '/target', got: {content}"
    );
}

#[test]
fn gitignore_python() {
    let tmp = init_in_tempdir(GitignoreLang::Python);
    let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert!(
        content.contains("__pycache__"),
        "python .gitignore should contain '__pycache__', got: {content}"
    );
}

#[test]
fn gitignore_node() {
    let tmp = init_in_tempdir(GitignoreLang::Node);
    let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert!(
        content.contains("node_modules"),
        "node .gitignore should contain 'node_modules', got: {content}"
    );
}

#[test]
fn gitignore_none() {
    let tmp = init_in_tempdir(GitignoreLang::None);
    assert!(
        !tmp.path().join(".gitignore").exists(),
        ".gitignore should NOT exist when gitignore=none"
    );
}

#[test]
fn config_generated() {
    let tmp = init_in_tempdir(GitignoreLang::None);
    let config_path = tmp.path().join(".git-rusk.toml");
    assert!(config_path.exists(), ".git-rusk.toml should exist after init");
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[branches]"), "config should have [branches] section");
    assert!(content.contains("[commit]"), "config should have [commit] section");
}

#[test]
fn config_roundtrips() {
    let tmp = init_in_tempdir(GitignoreLang::None);
    let config_path = tmp.path().join(".git-rusk.toml");
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("types"),
        "generated config should contain 'types'"
    );
    let loaded = Config::load(Some(&config_path)).expect("Config::load should succeed");
    assert_eq!(
        loaded.branches.default_branch, "development",
        "loaded config should have default_branch=development"
    );
}

#[test]
fn preserves_readme_on_rerun() {
    let tmp = tempfile::tempdir().unwrap();
    let args = InitArgs {
        path: tmp.path().to_path_buf(),
        gitignore: GitignoreLang::None,
    };
    init::run(&args, &Config::default()).unwrap();

    let readme_path = tmp.path().join("README.md");
    let original = fs::read_to_string(&readme_path).unwrap();
    let modified = format!("{original}\n# custom edit\n");
    fs::write(&readme_path, &modified).unwrap();

    init::run(&args, &Config::default()).unwrap();

    let after = fs::read_to_string(&readme_path).unwrap();
    assert!(
        after.contains("# custom edit"),
        "README modification should be preserved on re-run, got: {after}"
    );
}

#[test]
fn preserves_branches_on_rerun() {
    let tmp = tempfile::tempdir().unwrap();
    let args = InitArgs {
        path: tmp.path().to_path_buf(),
        gitignore: GitignoreLang::None,
    };
    init::run(&args, &Config::default()).unwrap();
    init::run(&args, &Config::default()).unwrap();

    for branch in &["main", "development", "release"] {
        let ref_path = format!("refs/heads/{branch}");
        let check = Command::new("git")
            .arg("-C")
            .arg(tmp.path())
            .args(["show-ref", "--verify", "--quiet", &ref_path])
            .output()
            .unwrap();
        assert!(
            check.status.success(),
            "branch {branch} should still exist after re-run"
        );
    }

    let count = git_output(tmp.path(), &["rev-list", "--count", "HEAD"]);
    assert_eq!(count, "1", "no extra commits after re-run");
}

#[test]
fn fresh_repo_unborn_head() {
    let parent = tempfile::tempdir().unwrap();
    let target = parent.path().join("fresh-project");
    assert!(!target.exists());

    let args = InitArgs {
        path: target.clone(),
        gitignore: GitignoreLang::None,
    };
    init::run(&args, &Config::default()).expect("init on fresh directory should succeed");

    assert!(target.join(".git").exists());
}

#[test]
fn init_with_path() {
    let parent = tempfile::tempdir().unwrap();
    let target = parent.path().join("explicit-subdir");
    assert!(!target.exists());

    let args = InitArgs {
        path: target.clone(),
        gitignore: GitignoreLang::None,
    };
    init::run(&args, &Config::default()).unwrap();

    assert!(target.exists(), "target directory should be created");
    assert!(target.join(".git").exists(), ".git should exist in target");
}
