<!-- generated-by: gsd-doc-writer -->

# Testing

git-rusk uses Rust's built-in `cargo test` framework. There is no separate test
runner or assertion crate beyond the dev-dependencies listed in `Cargo.toml`. Tests
are split into **unit tests** (inline `#[cfg(test)] mod tests` blocks inside each
`src/*.rs` module) and **integration tests** (one file per concern in `tests/`).

## Test framework and setup

| Aspect | Value |
| --- | --- |
| Framework | Rust built-in (`#[test]` macro, `cargo test` runner) |
| Config location | `Cargo.toml` `[dev-dependencies]` |
| Unit test location | Inline `#[cfg(test)] mod tests { ... }` at the bottom of each `src/*.rs` |
| Integration test location | `tests/*.rs` (one binary per file) |
| External system required | `git` must be on `PATH` (tests shell out via `std::process::Command`) |

**Dev-dependencies** (from `Cargo.toml`):

| Crate | Version | Purpose |
| --- | --- | --- |
| `tempfile` | 3 | Per-test temp directories and config-file fixtures |
| `assert_cmd` | 2 | Invoke the `git-rusk` binary as a subprocess and assert on exit code / stdout / stderr |
| `predicates` | 3 | Composable matchers used with `assert_cmd` (e.g. `predicates::str::contains`) |
| `serial_test` | 3 | `#[serial]` attribute for tests that mutate global process state (`XDG_CONFIG_HOME`) |

No global setup step is required — running `cargo test` from the repository root
is sufficient. Every test that touches the filesystem creates its own
`tempfile::tempdir()` and cleans up on exit, so the suite is hermetic and safe to
run from any working directory.

## Running tests

All commands run from the project root.

```bash
# Full suite: unit + integration (the canonical "did I break anything?" command)
cargo test

# Unit tests only — fast feedback loop during library work
cargo test --lib

# Integration tests only — the five binaries under tests/
cargo test --tests

# A single integration test file (filename without .rs)
cargo test --test init_tests
cargo test --test cli_tests
cargo test --test totp_tests
cargo test --test validate_commit
cargo test --test verify_cli

# A single unit test by module path
cargo test --lib commit_validator::tests
cargo test --lib totp::tests::test_skew

# Filter by test-name substring across the whole suite
cargo test init_creates_repo

# Show println! output from passing tests (hidden by default)
cargo test -- --nocapture

# Concurrent test threads (default = N CPUs); pin to 1 for deterministic ordering
cargo test -- --test-threads=1
```

The full verification gate used during development also includes clippy and
format checks (these are not run by `cargo test`):

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo build --release
```

There is **no watch mode** configured (no `cargo-watch` dependency). Re-run
`cargo test` manually after changes.

## Writing new tests

### Where to put the test

| Behavior under test | Location | Pattern |
| --- | --- | --- |
| A function or pure module logic in `src/<module>.rs` | Inline at the bottom of that file | `#[cfg(test)] mod tests { use super::*; ... }` |
| The `git-rusk` binary's CLI surface (subcommands, flags, exit codes, stdout) | `tests/cli_tests.rs` | `assert_cmd::Command::cargo_bin("git-rusk")` |
| End-to-end `init` / hook behavior driving the library | `tests/init_tests.rs` | Call `git_rusk::commands::init::run(...)` then assert on the tempdir with `git -C` |
| TOTP secret lifecycle (touches global `XDG_CONFIG_HOME`) | `tests/totp_tests.rs` | `#[serial]` + `std::env::set_var("XDG_CONFIG_HOME", tmp.path())` |
| `commit_validator::validate` over commit-message fixtures | `tests/validate_commit.rs` | Build a `CommitConfig` fixture, call `validate(msg, &cfg)` |
| Structural CLI correctness (clap definition invariants) | `tests/verify_cli.rs` | `Cli::command().debug_assert()` |

### File naming convention

Integration test files live directly in `tests/` (not in subdirectories). The
project uses two naming styles — both are acceptable:

- `*_tests.rs` for grouped suites: `cli_tests.rs`, `init_tests.rs`, `totp_tests.rs`
- `<topic>.rs` for single-concern files: `validate_commit.rs`, `verify_cli.rs`

Each `.rs` file in `tests/` compiles to its own test binary, so prefer adding a
new file over bloating an existing one when the concern is unrelated.

### Test helpers and fixtures

There are no shared helper modules — each file defines its own small helpers
inline. Reusable patterns to copy:

- **Temp directory isolation** — every filesystem-touching test starts with
  `let tmp = tempfile::tempdir().unwrap();` and passes `tmp.path()` into the
  code under test. The directory is deleted when `tmp` is dropped.
- **Config-file fixtures** — write a TOML string to a `tempfile::NamedTempFile`
  and pass its path to `--config` (see `cli_tests.rs::init_with_valid_config_exits_zero`).
- **Driving `git` for assertions** — use `std::process::Command::new("git").arg("-C").arg(dir)...`;
  the `git_output` helper in `init_tests.rs` is a good template.
- **Global-state isolation** — tests that set process-wide env vars (notably
  `XDG_CONFIG_HOME` for TOTP secret paths) must be marked `#[serial]` from the
  `serial_test` crate, and must remove the var at the end of the test. See
  `totp_tests.rs::secret_file_permissions` for the canonical pattern.
- **Commit-message fixtures** — long raw strings (`r#"..."#`) hold realistic
  conventional-commit messages; see `validate_commit.rs` for examples including
  BREAKING CHANGE footers and CRLF normalization.

### Conventions to follow

- Test functions are named `fn <describes_behavior>(...)` in snake_case — the
  project mixes `test_`-prefixed and unprefixed names; match the surrounding file.
- Assert with a custom failure message: `assert!(condition, "expected ..., got: {value}")`
  so failures are self-explanatory.
- Never mutate the real repository working copy. Always operate inside a
  `tempfile::tempdir()`.
- Tests that touch `~/.config`-backed global state (TOTP secret) must redirect
  via `XDG_CONFIG_HOME` and clean up the env var on exit.

## Coverage requirements

**No coverage threshold is configured.** There is no `tarpaulin.toml`,
`.config/nextest.toml`, `llvm-cov` config, or `coverage` section in
`Cargo.toml`. The project does not currently measure or gate on line/branch/
function coverage in any automated way.

The internal development target referenced in phase plans is **>80% coverage for
new modules**, but this is a manual reviewer expectation, not an enforced gate.
If you want to measure coverage locally, install a tool ad hoc, for example:

```bash
# Option A — cargo-llvm-cov (recommended for Rust)
cargo install cargo-llvm-cov
cargo llvm-cov --html                    # writes target/llvm-cov/html/index.html
cargo llvm-cov --summary-only            # text summary only

# Option B — tarpaulin
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

<!-- VERIFY: Coverage tooling is not pinned in the project; pick the version your local toolchain recommends. -->

## CI integration

**No CI pipeline is configured.** The repository has no `.github/workflows/`
directory and no other CI config (no `.gitlab-ci.yml`, no `circleci/`, no
` Drone`). Tests, clippy, and formatting are run manually by contributors before
committing.

The `flake.nix` defines a NixOS build via `crane` (`craneLib.buildPackage`)
that compiles the binary but does **not** wire `cargo test` into a Nix check
phase. Running the test suite through Nix would require adding a
`checks.<system>.git-rusk` attribute — that does not exist today.

<!-- VERIFY: If CI is added later (e.g. a GitHub Actions workflow), update this section with the workflow filename, trigger, and the exact test command it runs. -->

### Recommended pre-push verification

Until CI exists, contributors should run this gate locally before pushing:

```bash
cargo test && cargo clippy -- -D warnings && cargo fmt --check && cargo build --release
```

This matches the verification sequence used throughout the project's development
history and is the closest thing to a "CI check" currently in place.
