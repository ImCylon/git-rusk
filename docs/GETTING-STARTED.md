<!-- generated-by: gsd-doc-writer -->

# Getting Started

This guide takes you from a fresh checkout to enforced, conventional-commit-safe git hooks in under five minutes. git-rusk is a single Rust binary that wraps `git` with branch protection, commit-message validation, and optional TOTP human verification.

## Prerequisites

| Requirement | Version | How to check |
|-------------|---------|--------------|
| **Rust toolchain** | `>= 1.74` (MSRV) | `rustc --version` |
| **git** (system binary) | any recent version | `git --version` |
| **(Optional) Nix** with flakes | NixOS or nix-installed Linux | `nix --version` |
| **(Optional) direnv** | any version | `direnv --version` |

Details:

- The **MSRV of 1.74** is declared in `Cargo.toml` (`rust-version = "1.74"`). Any stable Rust toolchain at or above that version works. The NixOS flake pins `rust-bin.stable.latest.default`, which always satisfies this.
- **`git` must be on your `PATH`.** git-rusk shells out to the system `git` binary for every repository operation (`src/git_ops.rs`); there is no libgit2 dependency. Without `git`, init and all hooks fail with `Git is not installed or not on PATH`.
- **Nix and direnv are entirely optional.** They just give you a reproducible dev shell. Plain `cargo` works without them.

## Installation

### Option A — NixOS flake (recommended on NixOS)

Flakes must be enabled on your Nix installation.

```bash
# Install into your user profile
nix profile install .

# …or run once without installing
nix run . -- --help
```

### Option B — Cargo (any platform with Rust)

```bash
# Build and install the binary from this repo
cargo install --path .

# …or build in place (binary lands at target/release/git-rusk)
cargo build --release
```

### Option C — Dev shell for hacking on git-rusk itself

```bash
# With Nix + direnv (auto-loads the flake environment on cd)
direnv allow

# …or enter the shell manually
nix develop
```

The dev shell provides the Rust toolchain (`flake.nix` → `devShells.default`). No system-level Rust install is required inside it.

## First Run

The shortest path to enforced hooks in an existing repository:

```bash
# 1. Enter any git repository (or create a new project)
cd my-repo   # or: git-rusk init my-app --gitignore rust

# 2. Initialize the repo model + write .git-rusk.toml
git-rusk init

# 3. Install the .git/hooks/ wrapper scripts that delegate to git-rusk
git-rusk install-hooks

# 4. Commit as usual — hooks now enforce the rules
git commit -m "feat(auth): add login endpoint

Description: Implement the user login endpoint with session handling."
```

If step 4 succeeds with no errors, your setup is working. The `pre-commit` hook checked the branch against the allow list, the `commit-msg` hook validated your message format, and the `post-commit` hook ran without auto-returning you to the default branch.

To confirm the hooks are installed, list them:

```bash
ls -1 .git/hooks/
# pre-commit
# commit-msg
# post-checkout
# post-commit
```

Each is a thin shell wrapper that executes `git-rusk hook <name> "$@"`.

## Common Setup Issues

### 1. `Error: Git is not installed or not on PATH`

git-rusk could not find the `git` binary. Install git via your package manager (`nix-env -iA nixpkgs.git`, `apt install git`, etc.) and ensure the `git` executable is on your `PATH`.

### 2. Commit rejected with `CommitBlockedOnProtectedBranch`

You tried to commit on a branch that is not on the configured allow list (by default only `development` is allowed; `main` and `release` are protected). Switch to an allowed branch first:

```bash
git checkout development
```

This is the intended behavior — the tool exists to stop commits to protected branches.

### 3. Commit message validation failures

By default every commit must match `type(scope): description` **with a mandatory scope** and a body that begins with `Description:`. The validator reports all problems at once. Common mistakes:

- Missing scope: `feat: add login` → use `feat(auth): add login`.
- Missing `Description:` body: add a body line starting with `Description:`.
- Body too short: the body must be at least `min_body_length` characters (default `10`) after the prefix.
- Disallowed type/scope: only the configured `types` (default `feat`, `fix`, `docs`, `refactor`, `chore`, `test`, `style`) and `scopes` (default `"all"`) are accepted.

See [CONFIGURATION.md](CONFIGURATION.md) to customize the allowed types, scopes, and minimum body length.

### 4. `TOTP_CODE environment variable is not set`

You enabled TOTP verification (`require_for_commit = true` or `require_for_branch_switch = true` in `.git-rusk.toml`) but have not provided a code. First generate the global secret once per machine:

```bash
git-rusk totp init     # prints the Base32 secret + otpauth:// URI for your authenticator app
```

Then provide a current code at commit time:

```bash
TOTP_CODE=123456 git commit -m "fix(core): handle empty input

Description: Guard against empty input before processing."
```

If you did not mean to enable TOTP, leave both `[totp]` toggles at their default of `false`.

### 5. `TOTP secret file … has insecure permissions`

The global secret file (at `$XDG_CONFIG_HOME/git-rusk/totp-secret`, falling back to `$HOME/.config/git-rusk/totp-secret`) must be mode `0600`. Fix it with:

```bash
chmod 600 "$HOME/.config/git-rusk/totp-secret"
```

### 6. Nix: `error: experimental Nix feature 'flakes' is disabled`

The flake requires flakes to be enabled. Add `--extra-experimental-features 'nix-command flakes'` to the command, or enable flakes permanently in your Nix configuration.

## Next Steps

- **[CONFIGURATION.md](CONFIGURATION.md)** — every key in `.git-rusk.toml`, the full environment-variable reference, and the defaults table. Read this to customize branch lists, commit types/scopes, and TOTP behavior.
- **[ARCHITECTURE.md](ARCHITECTURE.md)** — how the CLI dispatches to hook handlers, the commit/checkout data flows, and the key abstractions in `src/`.
- **[README.md](../README.md)** — the command quick-reference and usage examples for `totp init|show|reset`, `install-hooks --force`, and `--default-branch`.
