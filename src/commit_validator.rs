use std::sync::OnceLock;

use regex::Regex;
use thiserror::Error;

use crate::config::{AllowList, CommitConfig};

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ValidationError {
    #[error(
        "Invalid header format: expected 'type(scope): description'\n  \
         Received: {received}\n\n  \
         Format:\n  \
         type(scope): description\n\n  \
         Example:\n  \
         feat(auth): add user login endpoint\n\n  \
         Allowed types: {allowed_types}"
    )]
    InvalidHeader {
        received: String,
        allowed_types: String,
    },

    #[error(
        "Missing mandatory scope\n  \
         Commit messages must include a scope in parentheses: type(scope): description\n\n  \
         Example:\n  \
         feat(auth): add user login endpoint\n\n  \
         Note: scope is MANDATORY in this project (unlike the standard spec where it is optional)"
    )]
    MissingScope,

    #[error(
        "Invalid commit type: '{received}'\n  \
         Allowed types: {allowed_types}\n\n  \
         Example:\n  \
         {example_type}(scope): description"
    )]
    InvalidType {
        received: String,
        allowed_types: String,
        example_type: String,
    },

    #[error(
        "Invalid scope: '{received}'\n  \
         Allowed scopes: {allowed_scopes}\n\n  \
         Example:\n  \
         type({example_scope}): description"
    )]
    InvalidScope {
        received: String,
        allowed_scopes: String,
        example_scope: String,
    },

    #[error(
        "Missing mandatory body\n  \
         Every commit must have a body starting with 'Description:'\n  \
         (minimum {minimum} characters of description)\n\n  \
         Example:\n  \
         feat(auth): add user login endpoint\n\n  \
         Description: Implements the user login endpoint with\n  \
         email and password validation."
    )]
    MissingBody { minimum: usize },

    #[error(
        "Body must start with 'Description:'\n  \
         The first line of the body must begin with 'Description:'\n\n  \
         Example:\n  \
         feat(auth): add user login endpoint\n\n  \
         Description: Implements the user login endpoint."
    )]
    BodyMissingDescriptionPrefix,

    #[error(
        "Body description too short: {actual} characters (minimum: {minimum})\n  \
         Provide more detail after 'Description:'\n\n  \
         Example:\n  \
         Description: Implements the user login endpoint with\n  \
         email and password validation and session management."
    )]
    BodyTooShort { actual: usize, minimum: usize },
}

static HEADER_RE: OnceLock<Regex> = OnceLock::new();

fn header_regex() -> &'static Regex {
    HEADER_RE.get_or_init(|| {
        Regex::new(r"^(?P<type>[a-zA-Z0-9_-]+)\((?P<scope>[^)]+)\)(?P<breaking>!)?: (?P<desc>.+)$")
            .expect("header regex must compile")
    })
}

#[allow(dead_code)]
struct HeaderParts<'a> {
    r#type: &'a str,
    scope: &'a str,
    breaking_change: bool,
    description: &'a str,
}

fn parse_header(line: &str) -> Option<HeaderParts<'_>> {
    let caps = header_regex().captures(line)?;
    Some(HeaderParts {
        r#type: caps.name("type")?.as_str(),
        scope: caps.name("scope")?.as_str(),
        breaking_change: caps.name("breaking").is_some(),
        description: caps.name("desc")?.as_str(),
    })
}

/// Checks if a commit message is auto-generated and should be exempt from validation.
fn is_exempt(message: &str) -> bool {
    let first_line = message.lines().next().unwrap_or("");
    first_line.starts_with("Merge ")
        || first_line.starts_with("Revert \"")
        || first_line.starts_with("fixup! ")
        || first_line.starts_with("squash! ")
        || first_line.starts_with("amend! ")
}

fn split_message(message: &str) -> (&str, Option<&str>) {
    let message = message.trim_end();
    if let Some(idx) = message.find("\n\n") {
        let header = &message[..idx];
        let body = &message[idx + 2..];
        if body.is_empty() {
            (header, None)
        } else {
            (header, Some(body))
        }
    } else {
        let header = message.lines().next().unwrap_or("");
        (header, None)
    }
}

fn is_type_allowed(types: &AllowList, captured: &str) -> bool {
    match types {
        AllowList::All => true,
        AllowList::Only(list) => list.iter().any(|t| t.eq_ignore_ascii_case(captured)),
    }
}

fn is_scope_allowed(scopes: &AllowList, captured: &str) -> bool {
    match scopes {
        AllowList::All => true,
        AllowList::Only(list) => list.iter().any(|s| s.eq_ignore_ascii_case(captured)),
    }
}

fn format_allowed_types(types: &AllowList) -> String {
    match types {
        AllowList::All => "all types accepted".to_string(),
        AllowList::Only(list) => list.join(", "),
    }
}

fn format_allowed_scopes(scopes: &AllowList) -> String {
    match scopes {
        AllowList::All => "all scopes accepted".to_string(),
        AllowList::Only(list) => list.join(", "),
    }
}

fn first_type(types: &AllowList) -> String {
    match types {
        AllowList::All => "feat".to_string(),
        AllowList::Only(list) => list.first().cloned().unwrap_or_else(|| "feat".to_string()),
    }
}

fn first_scope(scopes: &AllowList) -> String {
    match scopes {
        AllowList::All => "auth".to_string(),
        AllowList::Only(list) => list.first().cloned().unwrap_or_else(|| "auth".to_string()),
    }
}

const FOOTER_TOKENS: &[&str] = &[
    "BREAKING CHANGE:",
    "BREAKING-CHANGE:",
    "Fixes #",
    "Refs #",
    "Co-authored-by:",
];

fn extract_body_content(body: &str) -> String {
    let mut content_lines = Vec::new();
    for line in body.lines() {
        if FOOTER_TOKENS.iter().any(|token| line.starts_with(token)) {
            break;
        }
        content_lines.push(line);
    }
    content_lines.join("\n")
}

fn validate_body(body: &str, min_length: usize) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let content = extract_body_content(body);
    let trimmed = content.trim();

    if trimmed.is_empty() {
        errors.push(ValidationError::MissingBody {
            minimum: min_length,
        });
        return errors;
    }

    if !trimmed.starts_with("Description:") {
        errors.push(ValidationError::BodyMissingDescriptionPrefix);
    }

    let description_content = trimmed
        .strip_prefix("Description:")
        .unwrap_or(trimmed)
        .trim();
    let count = description_content.chars().count();
    if count < min_length {
        errors.push(ValidationError::BodyTooShort {
            actual: count,
            minimum: min_length,
        });
    }

    errors
}

fn has_type_without_scope(header: &str) -> bool {
    if let Some(idx) = header.find(": ") {
        let before_colon = &header[..idx];
        !before_colon.contains('(')
            && !before_colon.is_empty()
            && before_colon
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    } else {
        false
    }
}

/// Validates a commit message against conventional commit rules.
///
/// Returns `Ok(())` if the message is valid or exempt (merge, revert, fixup!, squash!, amend!).
/// Returns `Err(Vec<ValidationError>)` with ALL validation errors found,
/// so AI agents can fix all issues in a single retry.
///
/// Scope is mandatory in this project, diverging from the Conventional Commits spec
/// where scope is optional.
pub fn validate(message: &str, config: &CommitConfig) -> Result<(), Vec<ValidationError>> {
    let normalized = message.replace("\r\n", "\n");

    if is_exempt(&normalized) {
        return Ok(());
    }

    let mut errors = Vec::new();
    let (header, body) = split_message(&normalized);

    match parse_header(header) {
        Some(parts) => {
            if !is_type_allowed(&config.types, parts.r#type) {
                errors.push(ValidationError::InvalidType {
                    received: parts.r#type.to_string(),
                    allowed_types: format_allowed_types(&config.types),
                    example_type: first_type(&config.types),
                });
            }
            if !is_scope_allowed(&config.scopes, parts.scope) {
                errors.push(ValidationError::InvalidScope {
                    received: parts.scope.to_string(),
                    allowed_scopes: format_allowed_scopes(&config.scopes),
                    example_scope: first_scope(&config.scopes),
                });
            }
        }
        None => {
            if has_type_without_scope(header) {
                errors.push(ValidationError::MissingScope);
            } else {
                errors.push(ValidationError::InvalidHeader {
                    received: header.to_string(),
                    allowed_types: format_allowed_types(&config.types),
                });
            }
        }
    }

    match body {
        None => {
            errors.push(ValidationError::MissingBody {
                minimum: config.min_body_length,
            });
        }
        Some(body_text) => {
            errors.extend(validate_body(body_text, config.min_body_length));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AllowList, CommitConfig};

    fn test_config() -> CommitConfig {
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
    fn test_valid_commit_passes() {
        let msg = "feat(auth): add login\n\nDescription: Adds login screen.";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_valid_header_with_all_types_passes() {
        let config = CommitConfig {
            types: AllowList::All,
            ..test_config()
        };
        let msg = "customtype(api): test\n\nDescription: Tests custom type.";
        assert!(validate(msg, &config).is_ok());
    }

    #[test]
    fn test_invalid_type_rejected() {
        let msg = "unknown(scope): desc\n\nDescription: Some description.";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidType { .. })));
    }

    #[test]
    fn test_invalid_scope_rejected() {
        let config = CommitConfig {
            scopes: AllowList::Only(vec!["auth".into()]),
            ..test_config()
        };
        let msg = "feat(unknown): desc\n\nDescription: Some description.";
        let result = validate(msg, &config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidScope { .. })));
    }

    #[test]
    fn test_missing_scope_rejected() {
        let msg = "feat: add login\n\nDescription: Adds login screen.";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::MissingScope)));
    }

    #[test]
    fn test_invalid_header_rejected() {
        let msg = "bad header";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidHeader { .. })));
    }

    #[test]
    fn test_header_without_colon_space_rejected() {
        let msg = "feat(auth) add login";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidHeader { .. })));
    }

    #[test]
    fn test_breaking_change_marker_accepted() {
        let msg = "feat(auth)!: redesign login\n\nDescription: Redesigns login.";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_type_case_insensitive_uppercase() {
        let config = CommitConfig {
            types: AllowList::Only(vec!["feat".into()]),
            scopes: AllowList::Only(vec!["auth".into()]),
            min_body_length: 10,
        };
        let msg = "FEAT(auth): test\n\nDescription: Tests the feature.";
        assert!(validate(msg, &config).is_ok());
    }

    #[test]
    fn test_type_and_scope_case_insensitive_mixed() {
        let config = CommitConfig {
            types: AllowList::Only(vec!["feat".into()]),
            scopes: AllowList::Only(vec!["auth".into()]),
            min_body_length: 10,
        };
        let msg = "Feat(Auth): test\n\nDescription: Tests the feature.";
        assert!(validate(msg, &config).is_ok());
    }

    #[test]
    fn test_scope_case_insensitive_uppercase() {
        let config = CommitConfig {
            types: AllowList::Only(vec!["feat".into()]),
            scopes: AllowList::Only(vec!["auth".into()]),
            min_body_length: 10,
        };
        let msg = "feat(AUTH): test\n\nDescription: Tests the feature.";
        assert!(validate(msg, &config).is_ok());
    }

    #[test]
    fn test_merge_branch_exempt() {
        let msg = "Merge branch 'feature' into development";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_merge_pull_request_exempt() {
        let msg = "Merge pull request #123 from user/branch";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_revert_commit_exempt() {
        let msg = "Revert \"feat(auth): add login\"";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_fixup_commit_exempt() {
        let msg = "fixup! feat(auth): add login";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_squash_commit_exempt() {
        let msg = "squash! feat(auth): add login";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_amend_commit_exempt() {
        let msg = "amend! feat(auth): add login";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_non_exempt_merge_prefix_not_bypassed() {
        let msg = "Mergeable: thing\n\nDescription: Something happened.";
        let result = validate(msg, &test_config());
        assert!(
            result.is_err(),
            "Mergeable (without space after Merge) should NOT be exempt"
        );
    }

    #[test]
    fn test_missing_body_rejected() {
        let msg = "feat(auth): test";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::MissingBody { .. })));
    }

    #[test]
    fn test_body_without_description_prefix_rejected() {
        let msg = "feat(auth): add login\n\nThis is a body without prefix.";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e == &ValidationError::BodyMissingDescriptionPrefix));
    }

    #[test]
    fn test_multiple_errors_returned_at_once() {
        let config = CommitConfig {
            types: AllowList::Only(vec!["feat".into()]),
            scopes: AllowList::Only(vec!["auth".into()]),
            min_body_length: 10,
        };
        let msg = "badtype(badscope): desc\n\nDescription: Some valid body.";
        let result = validate(msg, &config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.len() >= 2,
            "expected at least 2 errors, got {}: {:?}",
            errors.len(),
            errors
        );
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidType { .. })));
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidScope { .. })));
    }

    #[test]
    fn test_body_valid_with_min_length() {
        let msg = "feat(auth): add login\n\nDescription: Adds login screen with validation.";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_body_too_short_error_fields() {
        let msg = "feat(auth): add login\n\nDescription: Short.";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        let too_short = errors
            .iter()
            .find(|e| matches!(e, ValidationError::BodyTooShort { .. }));
        assert!(too_short.is_some(), "should have BodyTooShort error");
        if let Some(ValidationError::BodyTooShort { actual, minimum }) = too_short {
            assert_eq!(*actual, 6, "actual length should be 6 for 'Short.'");
            assert_eq!(*minimum, 10);
        }
    }

    #[test]
    fn test_missing_body_error_carries_minimum() {
        let msg = "feat(auth): add login";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::MissingBody { minimum: 10 })));
    }

    #[test]
    fn test_body_without_description_prefix_error() {
        let msg = "feat(auth): add login\n\nThis has no prefix.";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e == &ValidationError::BodyMissingDescriptionPrefix),
            "should have BodyMissingDescriptionPrefix"
        );
    }

    #[test]
    fn test_multiline_body_counted_in_full() {
        let msg = "feat(auth): add login\n\nDescription: First line of description\nthat continues to a second line\nand a third.";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_body_with_footer_passes() {
        let msg = "feat(auth): add login\n\nDescription: Valid description text.\n\nFixes #123";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_footer_only_body_rejected() {
        let msg = "feat(auth): add login\n\nFixes #123\nCo-authored-by: Jane <jane@example.com>";
        let result = validate(msg, &test_config());
        assert!(result.is_err(), "footer-only body should be rejected");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ValidationError::MissingBody { .. } | ValidationError::BodyMissingDescriptionPrefix
            )),
            "should have MissingBody or BodyMissingDescriptionPrefix"
        );
    }

    #[test]
    fn test_description_prefix_case_sensitive() {
        let msg = "feat(auth): add login\n\ndescription: lowercase d";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e == &ValidationError::BodyMissingDescriptionPrefix),
            "lowercase 'description:' should be rejected"
        );
    }

    #[test]
    fn test_body_length_counts_chars_not_bytes() {
        let config = CommitConfig {
            min_body_length: 10,
            ..test_config()
        };
        let msg = "feat(auth): add login\n\nDescription: héllo";
        let result = validate(msg, &config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        if let Some(ValidationError::BodyTooShort { actual, minimum }) = errors
            .iter()
            .find(|e| matches!(e, ValidationError::BodyTooShort { .. }))
        {
            assert_eq!(*actual, 5, "héllo is 5 chars not 6 bytes");
            assert_eq!(*minimum, 10);
        } else {
            panic!("expected BodyTooShort error");
        }
    }

    #[test]
    fn test_footer_fixes_accepted() {
        let msg = "feat(auth): add login\n\nDescription: Adds login screen.\n\nFixes #123";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_footer_refs_accepted() {
        let msg = "feat(auth): add login\n\nDescription: Adds login screen.\n\nRefs #456";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_footer_co_authored_by_accepted() {
        let msg = "feat(auth): add login\n\nDescription: Adds login screen.\n\nCo-authored-by: Jane <jane@example.com>";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_footer_breaking_change_accepted() {
        let msg = "feat(auth)!: redesign\n\nDescription: Redesigns login flow.\n\nBREAKING CHANGE: old API removed.";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_footer_breaking_change_hyphen_synonym() {
        let msg = "feat(auth)!: redesign\n\nDescription: Redesigns login flow.\n\nBREAKING-CHANGE: old API removed.";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_footer_multiple_accepted() {
        let msg = "feat(auth): add login\n\nDescription: Adds login screen.\n\nFixes #123\nRefs #456\nCo-authored-by: Jane <jane@example.com>";
        assert!(validate(msg, &test_config()).is_ok());
    }

    #[test]
    fn test_footer_unknown_accepted() {
        let msg = "feat(auth): add login\n\nDescription: Adds login screen.\n\nReviewed-by: Bob <bob@example.com>";
        assert!(
            validate(msg, &test_config()).is_ok(),
            "unknown footers should be accepted, not rejected"
        );
    }

    #[test]
    fn test_footer_text_not_counted_toward_body_length() {
        let msg = "feat(auth): add login\n\nDescription: Short.\n\nFixes #123";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        let too_short = errors
            .iter()
            .find(|e| matches!(e, ValidationError::BodyTooShort { .. }));
        assert!(
            too_short.is_some(),
            "footer text should not satisfy min_body_length"
        );
        if let Some(ValidationError::BodyTooShort { actual, .. }) = too_short {
            assert_eq!(*actual, 6, "'Short.' is 6 chars; Fixes #123 excluded");
        }
    }

    #[test]
    fn test_footer_only_body_with_no_description_rejected() {
        let msg = "feat(auth): add login\n\nFixes #123\nCo-authored-by: Jane <jane@example.com>";
        let result = validate(msg, &test_config());
        assert!(
            result.is_err(),
            "footer-only body with no Description: must be rejected"
        );
    }

    #[test]
    fn test_error_invalid_header_contains_format_reference() {
        let msg = "bad header";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let error = result
            .unwrap_err()
            .into_iter()
            .find(|e| matches!(e, ValidationError::InvalidHeader { .. }))
            .expect("should have InvalidHeader error");
        let msg = error.to_string();
        assert!(
            msg.contains("type(scope): description"),
            "InvalidHeader should contain format reference"
        );
        assert!(
            msg.contains("Example:"),
            "InvalidHeader should contain Example section"
        );
        assert!(
            msg.contains("feat(auth)"),
            "InvalidHeader should contain a copy-pasteable example"
        );
        assert!(
            msg.contains("Allowed types"),
            "InvalidHeader should list allowed types"
        );
    }

    #[test]
    fn test_error_missing_scope_contains_format_reference() {
        let msg = "feat: add login\n\nDescription: Adds login screen.";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let error = result
            .unwrap_err()
            .into_iter()
            .find(|e| matches!(e, ValidationError::MissingScope))
            .expect("should have MissingScope error");
        let msg = error.to_string();
        assert!(
            msg.contains("type(scope): description"),
            "MissingScope should contain format reference"
        );
        assert!(
            msg.contains("scope is MANDATORY"),
            "MissingScope should explain scope is mandatory"
        );
    }

    #[test]
    fn test_error_invalid_type_contains_received_and_allowed_and_example() {
        let msg = "unknown(scope): desc\n\nDescription: Some description.";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let error = result
            .unwrap_err()
            .into_iter()
            .find(|e| matches!(e, ValidationError::InvalidType { .. }))
            .expect("should have InvalidType error");
        let msg = error.to_string();
        assert!(
            msg.contains("unknown"),
            "InvalidType should contain received type"
        );
        assert!(
            msg.contains("Allowed types"),
            "InvalidType should list allowed types"
        );
        assert!(
            msg.contains("Example:"),
            "InvalidType should contain Example section"
        );
        assert!(
            msg.contains("feat"),
            "InvalidType example should use a valid type from config"
        );
    }

    #[test]
    fn test_error_invalid_scope_contains_received_and_allowed_and_example() {
        let config = CommitConfig {
            scopes: AllowList::Only(vec!["auth".into(), "api".into()]),
            ..test_config()
        };
        let msg = "feat(unknown): desc\n\nDescription: Some description.";
        let result = validate(msg, &config);
        assert!(result.is_err());
        let error = result
            .unwrap_err()
            .into_iter()
            .find(|e| matches!(e, ValidationError::InvalidScope { .. }))
            .expect("should have InvalidScope error");
        let msg = error.to_string();
        assert!(
            msg.contains("unknown"),
            "InvalidScope should contain received scope"
        );
        assert!(
            msg.contains("Allowed scopes"),
            "InvalidScope should list allowed scopes"
        );
        assert!(
            msg.contains("auth") || msg.contains("api"),
            "InvalidScope example should use a valid scope from config"
        );
    }

    #[test]
    fn test_error_missing_body_contains_description_and_minimum() {
        let msg = "feat(auth): test";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let error = result
            .unwrap_err()
            .into_iter()
            .find(|e| matches!(e, ValidationError::MissingBody { .. }))
            .expect("should have MissingBody error");
        let msg = error.to_string();
        assert!(
            msg.contains("Description:"),
            "MissingBody should mention Description: prefix"
        );
        assert!(
            msg.contains("10"),
            "MissingBody should contain the minimum body length number"
        );
    }

    #[test]
    fn test_error_body_missing_description_prefix_contains_example_body() {
        let msg = "feat(auth): add login\n\nThis has no prefix.";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let error = result
            .unwrap_err()
            .into_iter()
            .find(|e| matches!(e, ValidationError::BodyMissingDescriptionPrefix))
            .expect("should have BodyMissingDescriptionPrefix error");
        let msg = error.to_string();
        assert!(
            msg.contains("Description:"),
            "BodyMissingDescriptionPrefix should mention Description:"
        );
        assert!(
            msg.contains("Example:"),
            "BodyMissingDescriptionPrefix should contain Example section"
        );
    }

    #[test]
    fn test_error_body_too_short_contains_actual_and_minimum() {
        let msg = "feat(auth): add login\n\nDescription: Short.";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let error = result
            .unwrap_err()
            .into_iter()
            .find(|e| matches!(e, ValidationError::BodyTooShort { .. }))
            .expect("should have BodyTooShort error");
        let msg = error.to_string();
        assert!(
            msg.contains("6"),
            "BodyTooShort should contain actual character count"
        );
        assert!(
            msg.contains("10"),
            "BodyTooShort should contain minimum required"
        );
    }

    #[test]
    fn test_multiple_errors_combined_contains_format_examples() {
        let config = CommitConfig {
            types: AllowList::Only(vec!["feat".into()]),
            scopes: AllowList::Only(vec!["auth".into()]),
            min_body_length: 20,
        };
        let msg = "badtype(badscope): desc\n\nDescription: Short.";
        let result = validate(msg, &config);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        let combined: String = errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            combined.contains("badtype"),
            "combined errors should mention received type"
        );
        assert!(
            combined.contains("Allowed types"),
            "combined errors should include allowed types list"
        );
        assert!(
            combined.contains("badscope"),
            "combined errors should mention received scope"
        );
        assert!(
            combined.contains("Allowed scopes"),
            "combined errors should include allowed scopes list"
        );
        assert!(
            combined.contains("Example:"),
            "combined errors should include format examples"
        );
    }

    #[test]
    fn test_error_messages_no_internal_debug_strings() {
        let configs = vec![
            ("bad header", test_config()),
            (
                "feat: no scope\n\nDescription: Has body text.",
                test_config(),
            ),
            (
                "unknown(scope): desc\n\nDescription: Some description.",
                test_config(),
            ),
            (
                "feat(badscope): desc\n\nDescription: Some description.",
                CommitConfig {
                    scopes: AllowList::Only(vec!["auth".into()]),
                    ..test_config()
                },
            ),
            ("feat(auth): test", test_config()),
            ("feat(auth): add login\n\nNo prefix here.", test_config()),
            (
                "feat(auth): add login\n\nDescription: Short.",
                test_config(),
            ),
        ];

        for (msg, config) in configs {
            if let Err(errors) = validate(msg, &config) {
                for error in &errors {
                    let display = error.to_string();
                    assert!(
                        !display.contains("todo!"),
                        "error message must not contain 'todo!': {display}"
                    );
                    assert!(
                        !display.contains("panicked"),
                        "error message must not contain 'panicked': {display}"
                    );
                    assert!(
                        !display.contains("unwrap"),
                        "error message must not contain 'unwrap': {display}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_error_messages_no_raw_file_paths() {
        let msg = "bad header";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let combined: String = result
            .unwrap_err()
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !combined.contains(".rs:"),
            "error messages must not contain raw file paths"
        );
        assert!(
            !combined.contains("src/"),
            "error messages must not contain source directory paths"
        );
        assert!(
            !combined.contains("at line"),
            "error messages must not contain stack trace references"
        );
    }
}
