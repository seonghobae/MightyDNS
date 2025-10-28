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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::NotFound("Resource not found".to_string());
        assert_eq!(err.to_string(), "Not found: Resource not found");
        
        let err = Error::Unauthorized;
        assert_eq!(err.to_string(), "Unauthorized");
        
        let err = Error::Forbidden;
        assert_eq!(err.to_string(), "Forbidden");
        
        let err = Error::InvalidInput("Bad data".to_string());
        assert_eq!(err.to_string(), "Invalid input: Bad data");
        
        let err = Error::Internal("Server error".to_string());
        assert_eq!(err.to_string(), "Internal error: Server error");
    }

    #[test]
    fn test_error_variants() {
        let errors = vec![
            Error::NotFound("test".to_string()),
            Error::Unauthorized,
            Error::Forbidden,
            Error::InvalidInput("test".to_string()),
            Error::LimitExceeded("test".to_string()),
            Error::Internal("test".to_string()),
        ];
        
        for error in errors {
            // All errors should have non-empty display strings
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn test_serialization_error_conversion() {
        let json_error = serde_json::from_str::<i32>("invalid json").unwrap_err();
        let error: Error = json_error.into();
        
        match error {
            Error::Serialization(_) => (),
            _ => panic!("Expected Serialization error"),
        }
    }

    #[test]
    fn test_nats_error_conversion() {
        let nats_err = Error::Nats("Connection failed".to_string());
        assert!(nats_err.to_string().contains("NATS error"));
        assert!(nats_err.to_string().contains("Connection failed"));
    }

    #[test]
    fn test_result_type() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }
        
        fn returns_err() -> Result<i32> {
            Err(Error::Internal("Test error".to_string()))
        }
        
        assert!(returns_ok().is_ok());
        assert!(returns_err().is_err());
    }

    #[test]
    fn test_error_messages_with_context() {
        let err = Error::NotFound("User with ID 123".to_string());
        assert!(err.to_string().contains("User with ID 123"));
        
        let err = Error::InvalidInput("Email must contain @".to_string());
        assert!(err.to_string().contains("Email must contain @"));
        
        let err = Error::LimitExceeded("Query quota exceeded".to_string());
        assert!(err.to_string().contains("Query quota exceeded"));
    }

    #[test]
    fn test_error_types_are_distinguishable() {
        let not_found = Error::NotFound("test".to_string());
        let unauthorized = Error::Unauthorized;
        let forbidden = Error::Forbidden;
        
        assert_ne!(not_found.to_string(), unauthorized.to_string());
        assert_ne!(unauthorized.to_string(), forbidden.to_string());
        assert_ne!(not_found.to_string(), forbidden.to_string());
    }
}