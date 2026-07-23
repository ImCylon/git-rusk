use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;

fn bin() -> Command {
    Command::cargo_bin("git-rusk").unwrap()
}

#[test]
fn help_shows_init_subcommand() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("init"));
}

#[test]
fn help_shows_install_hooks_subcommand() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("install-hooks"));
}

#[test]
fn help_shows_hook_subcommand() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("hook"));
}

#[test]
fn version_shows_binary_name() {
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("git-rusk"));
}

#[test]
fn init_with_defaults_exits_zero() {
    bin().arg("init").assert().success();
}

#[test]
fn install_hooks_with_defaults_exits_zero() {
    bin().arg("install-hooks").assert().success();
}

#[test]
fn init_with_valid_config_exits_zero() {
    let toml_content = r#"
[branches]
allowed = ["development"]
protected = ["main", "release"]
default_branch = "development"

[commit]
types = ["feat", "fix", "docs", "refactor", "chore", "test", "style"]
scopes = "all"
min_body_length = 10

[totp]
require_for_commit = false
require_for_branch_switch = false
step_seconds = 30
backward_tolerance_secs = 120
"#;
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), toml_content).unwrap();

    bin()
        .arg("--config")
        .arg(tmp.path())
        .arg("init")
        .assert()
        .success();
}

#[test]
fn config_nonexistent_errors() {
    bin()
        .arg("--config")
        .arg("/nonexistent/path/to/config.toml")
        .arg("init")
        .assert()
        .failure();
}

#[test]
fn config_invalid_toml_errors() {
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "invalid = = = broken").unwrap();

    bin()
        .arg("--config")
        .arg(tmp.path())
        .arg("init")
        .assert()
        .failure();
}

#[test]
fn help_contains_description_text() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Git hook manager"));
}

#[test]
fn default_branch_override_exits_zero() {
    bin()
        .arg("--default-branch")
        .arg("feature")
        .arg("init")
        .assert()
        .success();
}

#[test]
fn default_branch_cli_overrides_toml_exits_zero() {
    let toml_content = r#"
[branches]
default_branch = "dev"
"#;
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), toml_content).unwrap();

    bin()
        .arg("--config")
        .arg(tmp.path())
        .arg("--default-branch")
        .arg("feature")
        .arg("init")
        .assert()
        .success();
}
