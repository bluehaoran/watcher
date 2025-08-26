pub mod config;
// pub mod core;
pub mod element_finder;
pub mod models;
pub mod plugins;
pub mod product_manager;
pub mod scheduler;
pub mod scraper;
pub mod utils;
pub mod web;

// Re-export commonly used types
pub use config::AppConfig;
pub use element_finder::{ElementFinder, ElementFinderRequest, ElementFinderResult, ElementSelector, ElementMatch};
pub use models::*;
pub use product_manager::{ProductManager, ProductRequest, ProductUpdate, ProductCheckResult, ProductStats};
pub use scheduler::{ProductScheduler, JobInfo, JobStatus, SchedulerStats};
pub use scraper::{WebScraper, ScrapeRequest, ScrapeResult};
pub use utils::error::AppError;

pub type Result<T> = std::result::Result<T, AppError>;