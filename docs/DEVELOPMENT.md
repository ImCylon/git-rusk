<!-- generated-by: gsd-doc-writer -->

# Development

How to set up a local working copy of **git-rusk**, build and test it, and the
conventions to follow when contributing changes. The project is a single-binary
Rust CLI (edition 2021, MSRV 1.74) with first-class NixOS flake support.

## Local setup

git-rusk has two supported local-setup paths. Pick one.

### Option A — NixOS flake (recommended)

The repository ships a `flake.nix` that pins the Rust toolchain via
[rust-overlay](https://github.com/oxalica/rust-overlay) and provides a dev shell
with everything needed to build, test, and run rust-analyzer.

```bash
# 1. Clone
git clone <repo-url> git-rusk
cd git-rusk

# 2. Enter the dev shell (flakes must be enabled on your Nix installation)
nix develop
```

If you use [direnv](https://direnv.net/), the included `.envrc` contains
`use flake`, so the dev shell activates automatically on `cd`:

```bash
direnv allow    # run once, then every cd into the repo loads the toolchain
```

### Option B — Manual Rust toolchain

If you are not on NixOS, install the stable Rust toolchain yourself. The project
requires **Rust >= 1.74** (set via `rust-version` in `Cargo.toml`):

```bash
# 1. Clone
git clone <repo-url> git-rusk
cd git-rusk

# 2. Install Rust >= 1.74 (https://rustup.rs)
rustup toolchain install stable
rustup default stable

# 3. Build dependencies and the binary
cargo build
```

### System dependency

git-rusk shells out to the system `git` binary for repository operations (it
does **not** link libgit2). Ensure `git` is installed and on your `PATH`:

```bash
git --version    # any recent git release works
```

### Verify the setup

Whichever path you chose, confirm the binary builds and tests pass:

```bash
cargo test
```

See [GETTING-STARTED.md](../docs/GETTING-STARTED.md) for end-user install and
first-run instructions.

## Build commands

There is no `package.json` script layer or `Makefile`; all developer tasks are
plain `cargo` invocations. The commands you will use day to day:

| Command | Description |
| --- | --- |
| `cargo build` | Compile the binary in debug mode (output: `target/debug/git-rusk`). |
| `cargo build --release` | Compile an optimized release build (output: `target/release/git-rusk`). |
| `cargo run -- <args>` | Build (debug) and run the binary, e.g. `cargo run -- init --gitignore rust`. |
| `cargo test` | Run the full test suite — unit tests in `src/` and integration tests in `tests/`. |
| `cargo test <name>` | Run only tests whose name matches `<name>` (e.g. `cargo test totp`). |
| `cargo fmt` | Format all source files with rustfmt. |
| `cargo fmt --check` | Fail (non-zero exit) if any file is not formatted — run before committing. |
| `cargo clippy` | Run the clippy linter. |
| `cargo clippy -- -D warnings` | Treat all clippy warnings as errors (strict mode used in commits). |
| `cargo doc --no-deps --open` | Generate and open the rustdoc HTML for this crate. |

### Release profile

`Cargo.toml` defines an aggressive release profile for the distributed binary:

```toml
[profile.release]
opt-level = 3
lto = true
strip = true
codegen-units = 1
```

This favors a small, optimized binary over compile time. It only affects
`cargo build --release` / `cargo install`; day-to-day `cargo build` (debug) is
unaffected.

### NixOS build

The flake also exposes the release build as a package and runnable app, useful
for reproducing the exact published artifact:

```bash
nix build .#default              # build into ./result/
nix run .#default -- init        # run the flake-built binary
```

## Code style

git-rusk follows standard Rust style enforced by two tools, both run from the
repo root with no extra configuration.

### rustfmt (formatting)

- **Tool:** `rustfmt` (invoked via `cargo fmt`)
- **Config file:** none — the project uses rustfmt's default style (no
  `rustfmt.toml` or `.rustfmt.toml` in the repo).
- **Run:** `cargo fmt` to format, `cargo fmt --check` to verify without writing.
- Commit history shows formatting is applied deliberately (e.g.
  `style(03-03): apply cargo fmt`), so always run `cargo fmt` before committing.

### Clippy (linting)

- **Tool:** `clippy` (invoked via `cargo clippy`)
- **Config file:** none — default warning set (no `clippy.toml`).
- **Run:** `cargo clippy`. New code should be clippy-clean; historical commits
  such as `style(05-02): fix clippy warning` show warnings are fixed as they
  appear. For strict checking use `cargo clippy -- -D warnings`.

There is no `.editorconfig` and no other formatter/linter configured. Beyond
rustfmt + clippy, follow the idioms already present in `src/` (thiserror-based
error enums in `error.rs`, `anyhow::Result` at the binary boundary in `lib.rs`,
`std::process::Command` for git calls in `git_ops.rs`).

## Branch conventions

The repository uses a small set of long-lived branches; there is no documented
feature-branch naming scheme.

| Branch | Role |
| --- | --- |
| `development` | **Default working branch.** Day-to-day commits land here. This is the branch currently checked out by default after cloning. |
| `main` | Protected release branch (mirrors git-rusk's own branch-protection model). |
| `release` | Protected release branch. |
| `dev` | Legacy/alternate working branch. |

Feature work is tracked as numbered phases (see the commit history) and is
committed directly to `development`; no `feat/*` or `feature/*` branch-naming
convention is enforced or documented. If you are preparing a change, branch from
`development`.

### Commit message format

Commits follow **Conventional Commits**, scoped by the phase/plan the work
belongs to. The established pattern (visible throughout `git log`) is:

```
<type>(<phase>-<plan>): <description>
<type>(<phase>): <description>      # for cross-plan or docs commits
```

Where:

- `<type>` — one of `feat`, `fix`, `docs`, `test`, `style`, `chore`, `refactor`,
  `plan`, `research` (the project-specific `plan` and `research` types are used
  alongside the standard set).
- `<phase>-<plan>` — the phase and plan identifiers the change belongs to, e.g.
  `06-01`, `05-02`. For phase-wide work (research, roadmap updates) use just the
  phase number, e.g. `docs(06)`.
- `<description>` — lowercase imperative summary.

Examples from the history:

```
feat(06-01): add branch protection to pre-commit hook
test(05-02): add integration tests for install_hooks
style(05-02): fix clippy warning
docs(06): review and correct Phase 6 plans
chore: update STATE and ROADMAP for Phase 5 complete
```

## PR process

There is no `CONTRIBUTING.md`, no `.github/PULL_REQUEST_TEMPLATE.md`, and no CI
pipeline configured in this repository, so the PR process is lightweight. When
preparing a change for review:

- Branch from `development` and open the PR back against `development`.
- Keep commits focused and use the Conventional Commits format described above
  (type + phase/plan scope).
- Ensure `cargo fmt --check`, `cargo clippy`, and `cargo test` all pass locally
  before requesting review — there is no CI to catch these for you.
- Add or update tests under `tests/` (integration) and inline `#[cfg(test)]`
  modules in `src/` (unit) for any new behavior. The existing suite uses
  `assert_cmd` + `tempfile` + `predicates` for CLI/integration tests and plain
  `#[test]` functions for unit tests.
- Describe the phase/plan the change relates to in the PR body so reviewers have
  the same context the commit scopes convey.

> **Note:** No formal review checklist or branch-protection rule is enforced on
> the remote today. Treat the items above as the de facto contribution contract.
