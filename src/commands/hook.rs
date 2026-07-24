use anyhow::Result;
use crate::cli::HookAction;
use crate::config::Config;
use crate::hook_commit_msg;
use crate::hook_post_checkout;
use crate::hook_pre_commit;

pub fn run(action: &HookAction, config: &Config) -> Result<()> {
    match action {
        HookAction::PreCommit => hook_pre_commit::run(config).map_err(|e| anyhow::anyhow!(e)),
        HookAction::CommitMsg { msg_file } => hook_commit_msg::run(msg_file.clone(), config).map_err(|e| anyhow::anyhow!(e)),
        HookAction::PostCheckout => hook_post_checkout::run(config).map_err(|e| anyhow::anyhow!(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::HookAction;

    #[test]
    fn test_run_dispatches_to_pre_commit() {
        let config = Config::default();
        let action = HookAction::PreCommit;
        let result = run(&action, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_dispatches_to_post_checkout() {
        let config = Config::default();
        let action = HookAction::PostCheckout;
        let result = run(&action, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_returns_correct_exit_code() {
        let config = Config::default();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "feat(auth): add login\n\nDescription: Adds login screen.",
        )
        .unwrap();
        let action = HookAction::CommitMsg {
            msg_file: tmp.path().to_path_buf(),
        };

        let result = run(&action, &config);
        assert!(result.is_ok());
    }
}
