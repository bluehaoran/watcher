// pub mod config;
// pub mod core;
// pub mod models;
pub mod plugins;
pub mod utils;
// pub mod web;

// Re-export commonly used types
// pub use config::AppConfig;
pub use utils::error::AppError;

pub type Result<T> = std::result::Result<T, AppError>;