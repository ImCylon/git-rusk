<!-- generated-by: gsd-doc-writer -->

# git-rusk

A single-binary Rust CLI that initializes git repositories with branch protection, conventional-commit enforcement, and optional TOTP-based human verification. It stops automated tools and AI coding agents from committing to protected branches (`main`, `release`) or triggering CI/CD pipelines without explicit human approval — while keeping every commit message compliant with a configurable Conventional Commits policy.

## Installation

git-rusk is a Rust binary with no runtime dependencies beyond the system `git`. Build it from source with Cargo, or install it via the NixOS flake.

```bash
# Option 1 — Cargo (build and install the binary from this repo)
cargo install --path .

# Option 2 — NixOS (flakes must be enabled)
nix profile install .     # install into your user profile
# or run once without installing:
nix run .
```

You can also just build and use the binary in place:

```bash
cargo build --release
# binary is at target/release/git-rusk
```

## Quick start

```bash
# 1. Inside an existing git repo (or pass a path to create a new project)
git-rusk init

# 2. Write the .git/hooks/ wrapper scripts that call git-rusk
git-rusk install-hooks

# 3. Commit as usual — hooks now enforce the rules
git commit -m "feat(auth): add login"
```

`git-rusk init` sets up the default branch model, creates a `.git-rusk.toml` config, and optionally a README and language-specific `.gitignore`. `git-rusk install-hooks` installs four git hooks (`pre-commit`, `commit-msg`, `post-checkout`, `post-commit`) that delegate to git-rusk.

## Usage examples

Initialize a new project with a Rust `.gitignore` template:

```bash
git-rusk init my-app --gitignore rust
# creates my-app/, runs `git init`, sets up branches, writes .git-rusk.toml
```

The default branch policy (from `.git-rusk.toml`) is:

| Setting | Default |
| --- | --- |
| Allowed branches (commits permitted) | `development` |
| Protected branches (commits blocked) | `main`, `release` |
| Default branch (auto-return target) | `development` |
| Allowed commit types | `feat`, `fix`, `docs`, `refactor`, `chore`, `test`, `style` |
| Allowed scopes | all |
| Minimum body length | 10 |

Every commit message must follow `type(scope): description` with a mandatory scope and a body beginning with `Description:`. For example:

```
feat(auth): add login endpoint

Description: Implement the user login endpoint with session handling.
```

Enable optional TOTP human verification (works with Google Authenticator, Authy, etc. — standard 30-second step):

```bash
# Generate the global secret (shared across all repos on this machine)
git-rusk totp init

# Then provide a current code via the TOTP_CODE env var when committing
TOTP_CODE=123456 git commit -m "fix(core): handle empty input"
```

Other useful commands:

```bash
git-rusk totp show                 # display current secret + otpauth URI
git-rusk totp reset --force        # rotate the secret (invalidates old codes)
git-rusk install-hooks --force     # overwrite existing (non-symlink) hooks
git-rusk --default-branch trunk init   # override the default branch for one run
```

## License

Licensed under the MIT license (see the `license` field in `Cargo.toml`).
