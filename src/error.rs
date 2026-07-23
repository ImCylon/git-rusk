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
            | GitHookError::TemplateNotFound(_) => 1,
        }
    }
}
