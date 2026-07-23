use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub branches: BranchConfig,
    pub commit: CommitConfig,
    pub totp: TotpConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            branches: BranchConfig::default(),
            commit: CommitConfig::default(),
            totp: TotpConfig::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct BranchConfig {
    pub allowed: Vec<String>,
    pub protected: Vec<String>,
    pub default_branch: String,
}

impl Default for BranchConfig {
    fn default() -> Self {
        BranchConfig {
            allowed: vec!["development".to_string()],
            protected: vec!["main".to_string(), "release".to_string()],
            default_branch: "development".to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct CommitConfig {
    pub types: AllowList,
    pub scopes: AllowList,
    pub min_body_length: usize,
}

impl Default for CommitConfig {
    fn default() -> Self {
        CommitConfig {
            types: AllowList::Only(vec![
                "feat".to_string(),
                "fix".to_string(),
                "docs".to_string(),
                "refactor".to_string(),
                "chore".to_string(),
                "test".to_string(),
                "style".to_string(),
            ]),
            scopes: AllowList::All,
            min_body_length: 10,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct TotpConfig {
    pub require_for_commit: bool,
    pub require_for_branch_switch: bool,
    pub step_seconds: u32,
    pub backward_tolerance_secs: u32,
}

impl Default for TotpConfig {
    fn default() -> Self {
        TotpConfig {
            require_for_commit: false,
            require_for_branch_switch: false,
            step_seconds: 30,
            backward_tolerance_secs: 120,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum AllowList {
    All,
    Only(Vec<String>),
}

impl AllowList {
    pub fn allows(&self, value: &str) -> bool {
        match self {
            AllowList::All => true,
            AllowList::Only(list) => list.iter().any(|v| v == value),
        }
    }
}

impl Config {
    pub fn load(_config_path: Option<&Path>) -> Result<Self> {
        Ok(Config::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_types_all() {
        let toml_str = r#"types = "all""#;
        let config: CommitConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.types, AllowList::All);
    }

    #[test]
    fn test_deserialize_types_list() {
        let toml_str = r#"types = ["feat", "fix"]"#;
        let config: CommitConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.types,
            AllowList::Only(vec!["feat".to_string(), "fix".to_string()])
        );
    }

    #[test]
    fn test_deserialize_types_invalid_string() {
        let toml_str = r#"types = "maybe""#;
        let result: Result<CommitConfig, toml::de::Error> = toml::from_str(toml_str);
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("expected") || msg.contains("all"),
            "error message should contain 'expected' or 'all', got: {msg}"
        );
    }

    #[test]
    fn test_deserialize_scopes_all() {
        let toml_str = r#"scopes = "all""#;
        let config: CommitConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scopes, AllowList::All);
    }

    #[test]
    fn test_deserialize_scopes_empty_list() {
        let toml_str = r#"scopes = []"#;
        let config: CommitConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scopes, AllowList::Only(vec![]));
    }

    #[test]
    fn test_allows_all_variant() {
        let list = AllowList::All;
        assert!(list.allows("anything"));
        assert!(list.allows("feat"));
        assert!(list.allows(""));
    }

    #[test]
    fn test_allows_only_variant() {
        let list = AllowList::Only(vec!["feat".to_string()]);
        assert!(list.allows("feat"));
        assert!(!list.allows("fix"));
    }

    #[test]
    fn test_allows_empty_only_variant() {
        let list = AllowList::Only(vec![]);
        assert!(!list.allows("feat"));
    }
}
