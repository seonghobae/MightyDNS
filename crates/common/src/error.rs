use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("NATS error: {0}")]
    Nats(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Limit exceeded: {0}")]
    LimitExceeded(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<async_nats::Error> for Error {
    fn from(err: async_nats::Error) -> Self {
        Error::Nats(err.to_string())
    }
}
