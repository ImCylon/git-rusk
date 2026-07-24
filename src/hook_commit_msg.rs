use anyhow::Result;
use std::path::PathBuf;

use crate::commit_validator;
use crate::config::Config;
use crate::error::GitHookError;

pub fn run(msg_file: PathBuf, config: &Config) -> Result<()> {
    let message = std::fs::read_to_string(&msg_file).map_err(|source| GitHookError::FileWrite {
        path: msg_file.display().to_string(),
        source,
    })?;

    commit_validator::validate(&message, &config.commit).map_err(|errors| {
        let error_msg = errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        GitHookError::GitOperation(format!("Commit message validation failed:\n{}", error_msg))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AllowList, CommitConfig};

    fn test_config() -> Config {
        Config {
            commit: CommitConfig {
                types: AllowList::Only(vec![
                    "feat".into(),
                    "fix".into(),
                    "docs".into(),
                    "refactor".into(),
                    "chore".into(),
                    "test".into(),
                    "style".into(),
                ]),
                scopes: AllowList::All,
                min_body_length: 10,
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_run_rejects_invalid_header() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "bad header").unwrap();
        let result = run(tmp.path().to_path_buf(), &test_config());
        assert!(result.is_err());
    }

    #[test]
    fn test_run_accepts_valid_commit_message() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "feat(auth): add login\n\nDescription: Adds login screen.",
        )
        .unwrap();
        let result = run(tmp.path().to_path_buf(), &test_config());
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_handles_missing_file_gracefully() {
        let path = PathBuf::from("/nonexistent/COMMIT_EDITMSG");
        let result = run(path, &test_config());
        assert!(result.is_err());
        let err = result.unwrap_err().downcast::<GitHookError>().unwrap();
        assert!(matches!(err, GitHookError::FileWrite { .. }));
    }
}
