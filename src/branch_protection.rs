use crate::error::GitHookError;

/// Returns true if `branch` matches any pattern in `allowed`.
///
/// Supports wildcard patterns (e.g., "hotfix/*" matches "hotfix/123").
/// Wildcard matching is 1-level depth only.
pub fn is_allowed_branch(branch: &str, allowed: &[String]) -> bool {
    allowed.iter().any(|pattern| matches_pattern(branch, pattern))
}

/// Returns true if `branch` matches `pattern`.
///
/// Supports wildcard patterns (e.g., "hotfix/*").
/// Wildcard matching is 1-level depth only: "hotfix/*" matches "hotfix/123" but NOT "hotfix/subdir/123".
fn matches_pattern(branch: &str, pattern: &str) -> bool {
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        match_branch_prefix(branch, prefix)
    } else {
        branch == pattern
    }
}

/// Returns true if `branch` has `prefix` as its first component.
///
/// Example: match_branch_prefix("hotfix/123", "hotfix") == true
/// Example: match_branch_prefix("hotfix/subdir/123", "hotfix") == false (1-level depth)
fn match_branch_prefix(branch: &str, prefix: &str) -> bool {
    let parts: Vec<&str> = branch.split('/').collect();
    parts.len() >= 2 && parts[0] == prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern_exact_match() {
        assert!(matches_pattern("development", "development"));
        assert!(!matches_pattern("development", "main"));
    }

    #[test]
    fn test_matches_pattern_wildcard() {
        assert!(matches_pattern("hotfix/123", "hotfix/*"));
        assert!(matches_pattern("hotfix/abc", "hotfix/*"));
        assert!(!matches_pattern("hotfix", "hotfix/*"));
    }

    #[test]
    fn test_matches_pattern_wildcard_rejects_multi_depth() {
        assert!(!matches_pattern("hotfix-subdir/123", "hotfix/*"));
        assert!(!matches_pattern("hotfix/subdir/branch", "hotfix/*"));
    }

    #[test]
    fn test_is_allowed_branch_empty_list() {
        assert!(!is_allowed_branch("development", &[]));
        assert!(!is_allowed_branch("main", &[]));
    }

    #[test]
    fn test_is_allowed_branch_single_exact() {
        assert!(is_allowed_branch("development", &vec!["development".to_string()]));
        assert!(!is_allowed_branch("main", &vec!["development".to_string()]));
    }

    #[test]
    fn test_is_allowed_branch_multiple_patterns() {
        let allowed = vec![
            "development".to_string(),
            "hotfix/*".to_string(),
            "main".to_string(),
        ];
        assert!(is_allowed_branch("development", &allowed));
        assert!(is_allowed_branch("hotfix/123", &allowed));
        assert!(is_allowed_branch("hotfix/abc", &allowed));
        assert!(is_allowed_branch("main", &allowed));
        assert!(!is_allowed_branch("feature/test", &allowed));
        assert!(!is_allowed_branch("hotfix/subdir/123", &allowed));
    }

    #[test]
    fn test_match_branch_prefix() {
        assert!(match_branch_prefix("hotfix/123", "hotfix"));
        assert!(match_branch_prefix("hotfix/abc-def", "hotfix"));
        assert!(!match_branch_prefix("hotfix", "hotfix"));
        assert!(!match_branch_prefix("hotfix/subdir/branch", "hotfix"));
        assert!(!match_branch_prefix("not-hotfix/123", "hotfix"));
    }
}