use git_hook::commit_validator::{validate, ValidationError};
use git_hook::config::{AllowList, CommitConfig};

fn default_config() -> CommitConfig {
    CommitConfig {
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
    }
}

#[test]
fn test_complete_valid_commit_with_footers() {
    let msg = "feat(auth): add user login endpoint\n\nDescription: Implements the user login endpoint with email\nand password validation. Returns JWT token on success.\n\nFixes #123\nCo-authored-by: Jane Doe <jane@example.com>";
    assert!(validate(msg, &default_config()).is_ok());
}

#[test]
fn test_minimal_valid_commit() {
    let msg = "fix(parser): handle empty input gracefully\n\nDescription: Fixes crash when parser receives empty string.";
    assert!(validate(msg, &default_config()).is_ok());
}

#[test]
fn test_breaking_change_commit() {
    let msg = "feat(api)!: redesign authentication flow\n\nDescription: Replaces session-based auth with JWT tokens.\n\nBREAKING CHANGE: All API endpoints now require Bearer token.\nThe old session cookie mechanism is removed.";
    assert!(validate(msg, &default_config()).is_ok());
}

#[test]
fn test_merge_commit_bypass() {
    let msg = "Merge branch 'feature/auth-system' into development\n\nDescription: Merges auth feature branch.";
    assert!(validate(msg, &default_config()).is_ok());
}

#[test]
fn test_multiple_validation_errors_collected() {
    let config = CommitConfig {
        types: AllowList::Only(vec!["feat".into()]),
        scopes: AllowList::Only(vec!["auth".into()]),
        min_body_length: 50,
    };
    let msg = "badtype(badscope): desc\n\nDescription: Too short.";
    let result = validate(msg, &config);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(
        errors.len() >= 3,
        "expected at least 3 errors (invalid type, invalid scope, body too short), got {}",
        errors.len()
    );
    assert!(errors
        .iter()
        .any(|e| matches!(e, ValidationError::InvalidType { .. })));
    assert!(errors
        .iter()
        .any(|e| matches!(e, ValidationError::InvalidScope { .. })));
    assert!(errors
        .iter()
        .any(|e| matches!(e, ValidationError::BodyTooShort { .. })));
}

#[test]
fn test_config_restrictive_scopes() {
    let config = CommitConfig {
        scopes: AllowList::Only(vec!["auth".into(), "api".into()]),
        ..default_config()
    };

    let valid_msg = "feat(auth): add login\n\nDescription: Adds login screen.";
    assert!(validate(valid_msg, &config).is_ok());

    let invalid_msg = "feat(database): add migration\n\nDescription: Adds migration.";
    let result = validate(invalid_msg, &config);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors
        .iter()
        .any(|e| matches!(e, ValidationError::InvalidScope { .. })));
}

#[test]
fn test_config_all_types_accepts_any_type() {
    let config = CommitConfig {
        types: AllowList::All,
        ..default_config()
    };

    let msg = "customtype(api): test\n\nDescription: Tests custom type.";
    assert!(validate(msg, &config).is_ok());
}

#[test]
fn test_case_insensitive_full_pipeline() {
    let msg = "FEAT(AUTH): add login\n\nDescription: Adds login screen.";
    assert!(
        validate(msg, &default_config()).is_ok(),
        "uppercase FEAT(AUTH) should pass when config allows feat/auth"
    );
}

#[test]
fn test_crlf_line_endings_normalized() {
    let msg = "feat(auth): add login\r\n\r\nDescription: Adds login screen.";
    let result = validate(msg, &default_config());
    assert!(
        result.is_ok(),
        "CRLF line endings should be normalized and pass validation"
    );
}

#[test]
fn test_error_message_readability_for_cold_reader() {
    let msg = "bad header";
    let result = validate(msg, &default_config());
    assert!(result.is_err());
    let error_string = result
        .unwrap_err()
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        error_string.contains("type(scope): description"),
        "error output should contain the format reference"
    );
    assert!(
        error_string.contains("Allowed types"),
        "error output should list allowed types"
    );
}
