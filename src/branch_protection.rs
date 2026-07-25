pub fn is_allowed_branch(branch: &str, allowed: &[String]) -> bool {
    allowed.iter().any(|pattern| matches_pattern(branch, pattern))
}

fn matches_pattern(branch: &str, pattern: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/*") {
        match_branch_prefix(branch, prefix)
    } else {
        branch == pattern
    }
}

fn match_branch_prefix(branch: &str, prefix: &str) -> bool {
    let parts: Vec<&str> = branch.split('/').collect();
    parts.len() == 2 && parts[0] == prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern_exact() {
        assert!(matches_pattern("development", "development"));
        assert!(!matches_pattern("development", "main"));
    }

    #[test]
    fn test_matches_pattern_wildcard() {
        assert!(matches_pattern("hotfix/123", "hotfix/*"));
        assert!(!matches_pattern("hotfix/subdir/123", "hotfix/*"));
    }

    #[test]
    fn test_match_branch_prefix() {
        assert!(match_branch_prefix("hotfix/123", "hotfix"));
        assert!(!match_branch_prefix("hotfix/123/extra", "hotfix"));
        assert!(!match_branch_prefix("hotfix-subdir/123", "hotfix"));
    }

    #[test]
    fn test_is_allowed_branch_exact() {
        assert!(is_allowed_branch("development", &["development".to_string()]));
        assert!(!is_allowed_branch("main", &["development".to_string()]));
    }

    #[test]
    fn test_is_allowed_branch_wildcard() {
        assert!(is_allowed_branch("hotfix/123", &["hotfix/*".to_string()]));
        assert!(!is_allowed_branch("hotfix/subdir/123", &["hotfix/*".to_string()]));
    }

    #[test]
    fn test_is_allowed_branch_empty_list() {
        assert!(!is_allowed_branch("development", &[]));
    }
}