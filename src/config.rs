use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Config {
    pub branches: BranchConfig,
    pub commit: CommitConfig,
    pub totp: TotpConfig,
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

#[derive(Serialize, Clone, Debug, PartialEq)]
pub enum AllowList {
    All,
    Only(Vec<String>),
}

impl<'de> serde::Deserialize<'de> for AllowList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_allow_list(deserializer)
    }
}

enum StringOrVec {
    Str(String),
    Vec(Vec<String>),
}

impl<'de> serde::Deserialize<'de> for StringOrVec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = StringOrVec;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a string or array of strings")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(StringOrVec::Str(v.to_string()))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut list = Vec::new();
                while let Some(item) = seq.next_element()? {
                    list.push(item);
                }
                Ok(StringOrVec::Vec(list))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

fn deserialize_allow_list<'de, D>(deserializer: D) -> Result<AllowList, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::Str(s) if s == "all" => Ok(AllowList::All),
        StringOrVec::Str(s) => Err(serde::de::Error::custom(format!(
            "expected \"all\" or an array of strings, got string \"{}\"",
            s
        ))),
        StringOrVec::Vec(v) => Ok(AllowList::Only(v)),
    }
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
    pub fn load(config_path: Option<&Path>) -> Result<Self> {
        match config_path {
            Some(path) => Self::load_from_path(path),
            None => {
                let cwd_path = Path::new(".git-rusk.toml");
                if cwd_path.exists() {
                    Self::load_from_path(cwd_path)
                } else {
                    Ok(Config::default())
                }
            }
        }
    }

    fn load_from_path(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path).map_err(|source| {
            crate::error::GitHookError::ConfigFileRead {
                path: path.display().to_string(),
                source,
            }
        })?;
        toml::from_str::<Config>(&contents).map_err(|e| {
            crate::error::GitHookError::ConfigFileParse {
                path: path.display().to_string(),
                message: e.to_string(),
            }
            .into()
        })
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

    #[test]
    fn test_deserialize_full_config() {
        let toml_str = r#"
[branches]
allowed = ["dev"]
protected = ["main"]
default_branch = "dev"

[commit]
types = ["feat"]
scopes = "all"
min_body_length = 20

[totp]
require_for_commit = true
require_for_branch_switch = false
step_seconds = 30
backward_tolerance_secs = 60
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.branches.allowed, vec!["dev".to_string()]);
        assert_eq!(config.branches.protected, vec!["main".to_string()]);
        assert_eq!(config.branches.default_branch, "dev");
        assert_eq!(
            config.commit.types,
            AllowList::Only(vec!["feat".to_string()])
        );
        assert_eq!(config.commit.scopes, AllowList::All);
        assert_eq!(config.commit.min_body_length, 20);
        assert!(config.totp.require_for_commit);
        assert!(!config.totp.require_for_branch_switch);
        assert_eq!(config.totp.step_seconds, 30);
        assert_eq!(config.totp.backward_tolerance_secs, 60);
    }

    #[test]
    fn test_deserialize_empty_config() {
        let config: Config = toml::from_str("").unwrap();
        let default = Config::default();
        assert_eq!(config.branches.allowed, default.branches.allowed);
        assert_eq!(config.branches.protected, default.branches.protected);
        assert_eq!(
            config.branches.default_branch,
            default.branches.default_branch
        );
        assert_eq!(config.commit.types, default.commit.types);
        assert_eq!(config.commit.scopes, default.commit.scopes);
        assert_eq!(
            config.commit.min_body_length,
            default.commit.min_body_length
        );
        assert_eq!(
            config.totp.require_for_commit,
            default.totp.require_for_commit
        );
        assert_eq!(
            config.totp.require_for_branch_switch,
            default.totp.require_for_branch_switch
        );
        assert_eq!(config.totp.step_seconds, default.totp.step_seconds);
        assert_eq!(
            config.totp.backward_tolerance_secs,
            default.totp.backward_tolerance_secs
        );
    }

    #[test]
    fn test_deserialize_partial_branches_only() {
        let toml_str = r#"
[branches]
allowed = ["dev"]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.branches.allowed, vec!["dev".to_string()]);
        assert_eq!(config.branches.protected, BranchConfig::default().protected);
        assert_eq!(
            config.branches.default_branch,
            BranchConfig::default().default_branch
        );
        assert_eq!(config.commit.types, CommitConfig::default().types);
        assert_eq!(config.commit.scopes, CommitConfig::default().scopes);
        assert_eq!(
            config.commit.min_body_length,
            CommitConfig::default().min_body_length
        );
        assert_eq!(config.totp.step_seconds, TotpConfig::default().step_seconds);
    }

    #[test]
    fn test_deserialize_partial_commit_only() {
        let toml_str = r#"
[commit]
types = ["feat"]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.branches.allowed, BranchConfig::default().allowed);
        assert_eq!(
            config.commit.types,
            AllowList::Only(vec!["feat".to_string()])
        );
        assert_eq!(config.commit.scopes, CommitConfig::default().scopes);
        assert_eq!(config.totp.step_seconds, TotpConfig::default().step_seconds);
    }

    #[test]
    fn test_deserialize_partial_totp_only() {
        let toml_str = r#"
[totp]
require_for_commit = true
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.branches.allowed, BranchConfig::default().allowed);
        assert_eq!(config.commit.types, CommitConfig::default().types);
        assert!(config.totp.require_for_commit);
        assert_eq!(
            config.totp.backward_tolerance_secs,
            TotpConfig::default().backward_tolerance_secs
        );
    }

    #[test]
    fn test_branch_config_default_allowed() {
        let bc = BranchConfig::default();
        assert_eq!(bc.allowed, vec!["development".to_string()]);
    }

    #[test]
    fn test_branch_config_default_protected() {
        let bc = BranchConfig::default();
        assert_eq!(
            bc.protected,
            vec!["main".to_string(), "release".to_string()]
        );
    }

    #[test]
    fn test_branch_config_default_branch() {
        let bc = BranchConfig::default();
        assert_eq!(bc.default_branch, "development");
    }

    #[test]
    fn test_commit_config_default_types() {
        let cc = CommitConfig::default();
        match &cc.types {
            AllowList::Only(types) => {
                assert_eq!(types.len(), 7);
                assert!(types.contains(&"feat".to_string()));
                assert!(types.contains(&"fix".to_string()));
                assert!(types.contains(&"docs".to_string()));
                assert!(types.contains(&"refactor".to_string()));
                assert!(types.contains(&"chore".to_string()));
                assert!(types.contains(&"test".to_string()));
                assert!(types.contains(&"style".to_string()));
            }
            other => panic!("expected AllowList::Only, got {:?}", other),
        }
    }

    #[test]
    fn test_commit_config_default_scopes() {
        let cc = CommitConfig::default();
        assert_eq!(cc.scopes, AllowList::All);
    }

    #[test]
    fn test_totp_config_default_step_seconds() {
        let tc = TotpConfig::default();
        assert_eq!(tc.step_seconds, 30);
    }

    #[test]
    fn test_totp_config_default_backward_tolerance() {
        let tc = TotpConfig::default();
        assert_eq!(tc.backward_tolerance_secs, 120);
    }

    #[test]
    fn test_load_valid_full_toml() {
        let toml_content = r#"
[branches]
allowed = ["dev"]
protected = ["main"]
default_branch = "dev"

[commit]
types = ["feat"]
scopes = "all"
min_body_length = 20

[totp]
require_for_commit = true
require_for_branch_switch = false
step_seconds = 30
backward_tolerance_secs = 60
"#;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), toml_content).unwrap();
        let config = Config::load(Some(tmp.path())).unwrap();
        assert_eq!(config.branches.allowed, vec!["dev".to_string()]);
        assert_eq!(config.branches.protected, vec!["main".to_string()]);
        assert_eq!(config.branches.default_branch, "dev");
        assert_eq!(
            config.commit.types,
            AllowList::Only(vec!["feat".to_string()])
        );
        assert_eq!(config.commit.scopes, AllowList::All);
        assert_eq!(config.commit.min_body_length, 20);
        assert!(config.totp.require_for_commit);
        assert_eq!(config.totp.backward_tolerance_secs, 60);
    }

    #[test]
    fn test_load_none_returns_default() {
        let config = Config::load(None).unwrap();
        let default = Config::default();
        assert_eq!(config.branches.allowed, default.branches.allowed);
        assert_eq!(
            config.branches.default_branch,
            default.branches.default_branch
        );
        assert_eq!(
            config.commit.min_body_length,
            default.commit.min_body_length
        );
        assert_eq!(config.totp.step_seconds, default.totp.step_seconds);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let path = std::path::Path::new("/nonexistent/path/.git-rusk.toml");
        let result = Config::load(Some(path));
        assert!(result.is_err());
        let msg = format!("{:#}", result.err().unwrap());
        assert!(
            msg.contains("/nonexistent/path/.git-rusk.toml"),
            "error message should contain the file path, got: {msg}"
        );
    }

    #[test]
    fn test_load_invalid_toml() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "invalid = = toml").unwrap();
        let result = Config::load(Some(tmp.path()));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_partial_toml() {
        let toml_content = r#"
[branches]
allowed = ["dev"]
"#;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), toml_content).unwrap();
        let config = Config::load(Some(tmp.path())).unwrap();
        assert_eq!(config.branches.allowed, vec!["dev".to_string()]);
        assert_eq!(config.branches.protected, BranchConfig::default().protected);
        assert_eq!(
            config.branches.default_branch,
            BranchConfig::default().default_branch
        );
        assert_eq!(config.commit.types, CommitConfig::default().types);
        assert_eq!(config.commit.scopes, CommitConfig::default().scopes);
        assert_eq!(config.totp.step_seconds, TotpConfig::default().step_seconds);
    }

    #[test]
    fn test_load_empty_toml() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "").unwrap();
        let config = Config::load(Some(tmp.path())).unwrap();
        let default = Config::default();
        assert_eq!(config.branches.allowed, default.branches.allowed);
        assert_eq!(
            config.branches.default_branch,
            default.branches.default_branch
        );
        assert_eq!(
            config.commit.min_body_length,
            default.commit.min_body_length
        );
        assert_eq!(config.totp.step_seconds, default.totp.step_seconds);
    }

    #[test]
    fn test_load_none_cwd_auto_discovery() {
        let original_dir = std::env::current_dir().unwrap();
        let tmp_dir = tempfile::tempdir().unwrap();
        let config_path = tmp_dir.path().join(".git-rusk.toml");
        let toml_content = r#"
[branches]
allowed = ["feature"]
default_branch = "feature"
"#;
        std::fs::write(&config_path, toml_content).unwrap();
        std::env::set_current_dir(tmp_dir.path()).unwrap();
        let config = Config::load(None).unwrap();
        assert_eq!(config.branches.allowed, vec!["feature".to_string()]);
        assert_eq!(config.branches.default_branch, "feature");
        std::env::set_current_dir(&original_dir).unwrap();
    }

    #[test]
    fn test_serialize_allow_list_all() {
        let cc = CommitConfig {
            types: AllowList::All,
            ..Default::default()
        };
        let toml_str = toml::to_string(&cc).unwrap();
        assert!(
            toml_str.contains("types = \"all\""),
            "expected types = \"all\", got: {toml_str}"
        );
        assert!(
            !toml_str.contains("\"All\""),
            "should not contain '\"All\"': {toml_str}"
        );
    }

    #[test]
    fn test_serialize_allow_list_only() {
        let cc = CommitConfig {
            types: AllowList::Only(vec!["feat".to_string(), "fix".to_string()]),
            ..Default::default()
        };
        let toml_str = toml::to_string(&cc).unwrap();
        assert!(
            toml_str.contains("types = [\"feat\", \"fix\"]"),
            "expected types = [\"feat\", \"fix\"], got: {toml_str}"
        );
    }

    #[test]
    fn test_serialize_allow_list_empty_only() {
        let cc = CommitConfig {
            types: AllowList::Only(vec![]),
            ..Default::default()
        };
        let toml_str = toml::to_string(&cc).unwrap();
        assert!(
            toml_str.contains("types = []"),
            "expected types = [], got: {toml_str}"
        );
    }

    #[test]
    fn test_config_default_round_trip() {
        let original = Config::default();
        let serialized = toml::to_string_pretty(&original).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.branches.allowed, original.branches.allowed);
        assert_eq!(
            deserialized.branches.protected,
            original.branches.protected
        );
        assert_eq!(
            deserialized.branches.default_branch,
            original.branches.default_branch
        );
        assert_eq!(deserialized.commit.types, original.commit.types);
        assert_eq!(deserialized.commit.scopes, original.commit.scopes);
        assert_eq!(
            deserialized.commit.min_body_length,
            original.commit.min_body_length
        );
        assert_eq!(
            deserialized.totp.require_for_commit,
            original.totp.require_for_commit
        );
        assert_eq!(
            deserialized.totp.require_for_branch_switch,
            original.totp.require_for_branch_switch
        );
        assert_eq!(deserialized.totp.step_seconds, original.totp.step_seconds);
        assert_eq!(
            deserialized.totp.backward_tolerance_secs,
            original.totp.backward_tolerance_secs
        );
    }

    #[test]
    fn test_commit_config_types_all_round_trip() {
        let cc = CommitConfig {
            types: AllowList::All,
            ..Default::default()
        };
        let serialized = toml::to_string_pretty(&cc).unwrap();
        let deserialized: CommitConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.types, AllowList::All);
    }

    #[test]
    fn test_commit_config_types_only_round_trip() {
        let cc = CommitConfig {
            types: AllowList::Only(vec!["custom".to_string()]),
            ..Default::default()
        };
        let serialized = toml::to_string_pretty(&cc).unwrap();
        let deserialized: CommitConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(
            deserialized.types,
            AllowList::Only(vec!["custom".to_string()])
        );
    }

    #[test]
    fn test_config_default_serialized_contains_types() {
        let config = Config::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        assert!(
            serialized.contains("types ="),
            "serialized config should contain 'types =': {serialized}"
        );
    }
}
