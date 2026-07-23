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
            if !body_text.trim().starts_with("Description:") {
                errors.push(ValidationError::BodyMissingDescriptionPrefix);
            }
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
    fn test_invalid_header_rejected() {
        let msg = "bad header";
        let result = validate(msg, &test_config());
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidHeader { .. })));
    }
}
