use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitHookError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Failed to read config file {path}: {source}")]
    ConfigFileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse config file {path}: {message}")]
    ConfigFileParse { path: String, message: String },

    #[error("Invalid configuration value: {0}")]
    InvalidConfig(String),

    #[error("Git operation failed: {0}")]
    GitOperation(String),

    #[error("Git is not installed or not on PATH")]
    GitNotFound,

    #[error("Failed to write file {path}: {source}")]
    FileWrite {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("TOTP secret not found at {path}. Run 'git-rusk totp init' to create one.")]
    TotpSecretNotFound { path: String },

    #[error("Failed to read TOTP secret from {path}: {source}")]
    TotpSecretRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write TOTP secret to {path}: {source}")]
    TotpSecretWrite {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("TOTP secret file {path} has insecure permissions: {mode} (expected 600)")]
    TotpSecretInsecurePerms { path: String, mode: String },

    #[error("TOTP secret is invalid: {message}")]
    TotpSecretInvalid { message: String },

    #[error("TOTP_CODE environment variable is not set")]
    TotpCodeNotSet,

    #[error("TOTP construction failed: {message}")]
    TotpConstruction { message: String },

    #[error("System time error during TOTP verification: {message}")]
    TotpSystemTime { message: String },

    #[error("TOTP secret already exists. Use --force to overwrite.")]
    TotpSecretAlreadyExists,

    #[error("Failed to read hook message file {path}: {source}")]
    HookMessageFileReadFailed {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Commit validation failed: {errors}")]
    CommitValidationFailed { errors: String },
}

impl GitHookError {
    pub fn exit_code(&self) -> u8 {
        match self {
            GitHookError::Config(_)
            | GitHookError::ConfigFileRead { .. }
            | GitHookError::ConfigFileParse { .. }
            | GitHookError::InvalidConfig(_)
            | GitHookError::GitOperation(_)
            | GitHookError::GitNotFound
            | GitHookError::FileWrite { .. }
            | GitHookError::TemplateNotFound(_)
            | GitHookError::TotpSecretNotFound { .. }
            |             GitHookError::TotpSecretRead { .. }
            | GitHookError::TotpSecretWrite { .. }
            | GitHookError::TotpSecretInsecurePerms { .. }
            | GitHookError::TotpSecretInvalid { .. }
            | GitHookError::TotpCodeNotSet
            | GitHookError::TotpConstruction { .. }
            | GitHookError::TotpSystemTime { .. }
            | GitHookError::TotpSecretAlreadyExists
            | GitHookError::HookMessageFileReadFailed { .. }
            | GitHookError::CommitValidationFailed { .. } => 1,
        }
    }
}
