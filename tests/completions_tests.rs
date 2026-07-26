use assert_cmd::Command;
use predicates::str::contains;

fn bin() -> Command {
    Command::cargo_bin("git-rusk").unwrap()
}

#[test]
fn completions_bash_emits_script() {
    bin()
        .arg("completions")
        .arg("bash")
        .assert()
        .success()
        .stdout(contains("_git-rusk"));
}

#[test]
fn completions_zsh_emits_script() {
    bin()
        .arg("completions")
        .arg("zsh")
        .assert()
        .success()
        .stdout(contains("#compdef"));
}

#[test]
fn completions_fish_emits_script() {
    bin()
        .arg("completions")
        .arg("fish")
        .assert()
        .success()
        .stdout(contains("fish"));
}
