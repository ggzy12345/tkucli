use thiserror::Error;

pub type TkucliResult<T> = Result<T, TkucliError>;

#[derive(Debug, Error)]
pub enum TkucliError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Invalid argument '{name}': {reason}")]
    InvalidArgument { name: String, reason: String },

    #[error("Missing required argument: {0}")]
    MissingArgument(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Handler error: {0}")]
    Handler(#[from] anyhow::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(String),

    #[error("Auth error: {0}")]
    Auth(String),

    #[error("Aborted by user")]
    Aborted,
}
