use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::cli::InitArgs;
use crate::config::Config;
use crate::git_ops;

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

pub fn run(args: &InitArgs, config: &Config) -> Result<()> {
    let target = resolve_target_path(&args.path)?;
    let _project_name = project_name_from_path(&target);

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

    if was_new {
        println!("Initialized git-rusk repository in {}", target.display());
    } else {
        println!("Updated git-rusk repository in {}", target.display());
    }

    Ok(())
}
