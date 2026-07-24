use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::cli::InitArgs;
use crate::config::Config;
use crate::git_ops;
use crate::templates;

fn resolve_target_path(path: &Path) -> Result<PathBuf> {
    let canonical = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    if !canonical.exists() {
        std::fs::create_dir_all(&canonical)
            .with_context(|| format!("Failed to create directory: {}", canonical.display()))?;
    }

    Ok(canonical)
}

fn project_name_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-project")
        .to_string()
}

/// Write `content` to `path` only if the file does not already exist.
///
/// Returns `Ok(true)` if the file was created, `Ok(false)` if it was skipped
/// (already exists). This is the idempotency guard for all init file generation.
fn write_if_missing(path: &Path, content: &str) -> Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write: {}", path.display()))?;
    Ok(true)
}

pub fn run(args: &InitArgs, config: &Config) -> Result<()> {
    let target = resolve_target_path(&args.path)?;
    let project_name = project_name_from_path(&target);

    let was_new = !git_ops::is_git_repo(&target);

    git_ops::init_repo(&target)?;

    git_ops::ensure_main_branch(&target)?;

    git_ops::ensure_initial_commit(&target)?;

    for branch_name in &config.branches.allowed {
        git_ops::ensure_branch(&target, branch_name)?;
    }

    for branch_name in &config.branches.protected {
        git_ops::ensure_branch(&target, branch_name)?;
    }

    git_ops::checkout(&target, &config.branches.default_branch)?;

    let readme_content = templates::render_readme(&project_name)?;
    write_if_missing(&target.join("README.md"), &readme_content)?;

    let lang = args.gitignore.as_str();
    if let Some(gitignore_content) = templates::render_gitignore(lang)? {
        write_if_missing(&target.join(".gitignore"), &gitignore_content)?;
    }

    let toml_str = toml::to_string_pretty(config).context("Failed to serialize config to TOML")?;
    write_if_missing(&target.join(".git-rusk.toml"), &toml_str)?;

    if was_new {
        println!("Initialized git-rusk repository in {}", target.display());
    } else {
        println!("Updated git-rusk repository in {}", target.display());
    }

    Ok(())
}
