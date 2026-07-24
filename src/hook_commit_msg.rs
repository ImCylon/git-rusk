use crate::config::Config;
use crate::commit_validator;
use crate::error::GitHookError;
use std::fs;
use std::path::PathBuf;

pub fn run(msg_file: PathBuf, config: &Config) -> Result<(), GitHookError> {
    let msg = fs::read_to_string(&msg_file)
        .map_err(|e| GitHookError::HookMessageFileReadFailed {
            path: msg_file.clone(),
            source: e,
        })?;

    commit_validator::validate(&msg, &config.commit).map_err(|errors| {
        GitHookError::CommitValidationFailed {
            errors: errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "),
        }
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_accepts_valid_commit_message() {
        let config = Config::default();
        let msg_file = PathBuf::from("/tmp/test_valid_commit_msg.txt");
        fs::write(&msg_file, "feat(test): add feature\n\nDescription: Implement the test feature").unwrap();
        let result = run(msg_file.clone(), &config);
        assert!(result.is_ok());
        fs::remove_file(msg_file).ok();
    }

    #[test]
    fn test_run_rejects_invalid_header() {
        let config = Config::default();
        let msg_file = PathBuf::from("/tmp/test_invalid_header.txt");
        fs::write(&msg_file, "invalid message").unwrap();
        let result = run(msg_file.clone(), &config);
        assert!(result.is_err());
        fs::remove_file(msg_file).ok();
    }

    #[test]
    fn test_run_handles_missing_file() {
        let config = Config::default();
        let msg_file = PathBuf::from("/tmp/nonexistent_commit_msg.txt");
        let result = run(msg_file, &config);
        assert!(result.is_err());
    }
}