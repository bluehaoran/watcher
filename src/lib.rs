pub mod config;
// pub mod core;
pub mod models;
pub mod plugins;
pub mod scraper;
pub mod utils;
// pub mod web;

// Re-export commonly used types
pub use config::AppConfig;
pub use models::*;
pub use scraper::{WebScraper, ScrapeRequest, ScrapeResult};
pub use utils::error::AppError;

pub type Result<T> = std::result::Result<T, AppError>;