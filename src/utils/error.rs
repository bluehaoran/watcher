use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parsing error: {message}")]
    Parse { message: String },
    
    #[error("Plugin error: {plugin_type}: {message}")]
    Plugin { plugin_type: String, message: String },
    
    #[error("Plugin error: {0}")]
    PluginError(String),
    
    #[error("Scraping error: {0}")]
    Scraping(String),
    
    #[error("Element not found: {selector}")]
    ElementNotFound { selector: String },
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Not found: {resource}")]
    NotFound { resource: String },
    
    #[error("Internal error: {0}")]
    Internal(String),
}

// Implement conversion from validation errors
impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::Validation(format!("{}", err))
    }
}

// AppError can be converted to anyhow::Error via Display implementation

// Result type alias for convenience
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));
    }

    #[test] 
    fn test_plugin_error() {
        let err = AppError::Plugin {
            plugin_type: "price".to_string(),
            message: "failed to parse price".to_string(),
        };
        assert_eq!(err.to_string(), "Plugin error: price: failed to parse price");
    }
    
    #[test]
    fn test_element_not_found_error() {
        let err = AppError::ElementNotFound {
            selector: ".price".to_string(),
        };
        assert_eq!(err.to_string(), "Element not found: .price");
    }
}