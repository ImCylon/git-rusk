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
}

impl GitHookError {
    pub fn exit_code(&self) -> u8 {
        match self {
            GitHookError::Config(_)
            | GitHookError::ConfigFileRead { .. }
            | GitHookError::ConfigFileParse { .. }
            | GitHookError::InvalidConfig(_) => 1,
        }
    }
}
