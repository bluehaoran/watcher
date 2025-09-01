// Integration tests for Uatu Watcher
// These tests verify that all components work together correctly

pub mod api_tests;
pub mod product_lifecycle_tests;
pub mod scheduler_tests;
pub mod web_interface_tests;

use std::sync::Arc;
use uatu_watcher::{
    AppConfig,
    config::{DatabaseConfig, ServerConfig, SecurityConfig, SchedulerConfig, ScraperConfig, NotificationsConfig, SmtpConfig, DiscordConfig, ScreenshotConfig, MetricsConfig, PerformanceConfig},
    element_finder::ElementFinder,
    plugins::manager::PluginManager,
    product_manager::ProductManager,
    scheduler::ProductScheduler,
    scraper::WebScraper,
    web::{AppState, create_router},
};
use axum::{
    body::Body,
    http::{Request, Method},
};
use tower::{Service, ServiceExt};
use sqlx::{Pool, Sqlite, SqlitePool};

/// Test configuration for integration tests
pub fn get_test_config() -> AppConfig {
    AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0, // Use random port for testing
            base_url: "http://localhost".to_string(),
            request_timeout: 30,
            shutdown_timeout: 5,
        },
        database: DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 2,
            min_connections: 1,
            acquire_timeout: 10,
        },
        security: SecurityConfig {
            secret_key: "test-secret-key-32-characters-min".to_string(),
            jwt_expiry: 3600,
            rate_limit_requests: 1000,
            rate_limit_window: 60,
        },
        scraper: ScraperConfig {
            max_concurrent_checks: 1,
            retry_attempts: 1,
            retry_delay_ms: 100,
            request_timeout: 10,
            user_agent: "UatuWatcher-Test/1.0".to_string(),
            chrome_path: None,
        },
        scheduler: SchedulerConfig {
            default_interval: "0 */5 * * *".to_string(), // Every 5 minutes for testing
            max_running_jobs: 2,
            job_timeout: 60,
        },
        notifications: NotificationsConfig {
            smtp: SmtpConfig {
                host: "localhost".to_string(),
                port: 587,
                username: None,
                password: None,
                from_address: None,
                from_name: "Uatu Watcher Test".to_string(),
                use_tls: false,
            },
            discord: DiscordConfig {
                webhook_url: None,
                username: "Uatu Test Bot".to_string(),
                avatar_url: None,
            },
        },
        screenshots: ScreenshotConfig {
            enabled: false,
            quality: 80,
            max_size_mb: 5,
            retention_days: 7,
        },
        metrics: MetricsConfig {
            enabled: false,
            port: 9090,
            endpoint: "/metrics".to_string(),
        },
        performance: PerformanceConfig {
            thread_pool_size: 1,
            memory_limit_mb: 128,
            enable_compression: false,
        },
    }
}

/// Create a test database pool
pub async fn create_test_db() -> anyhow::Result<Pool<Sqlite>> {
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    Ok(pool)
}

/// Create test app state with all components initialized
pub async fn create_test_app_state() -> anyhow::Result<AppState> {
    let config = get_test_config();
    let _pool = create_test_db().await?;
    
    // Initialize scraper
    let scraper = WebScraper::new(config.scraper.clone())?;
    
    // Initialize other components
    let element_finder = ElementFinder::new(scraper.clone());
    let plugin_manager = PluginManager::new();
    let product_manager = Arc::new(ProductManager::new(
        scraper, 
        element_finder, 
        plugin_manager, 
        config.clone()
    ));

    let scheduler = ProductScheduler::new(
        Arc::clone(&product_manager), 
        config.scheduler.clone()
    ).await?;

    Ok(AppState {
        product_manager,
        scheduler: Arc::new(tokio::sync::Mutex::new(scheduler)),
        config,
    })
}

/// Helper to make HTTP requests to the test app
pub async fn make_request(
    app: &mut impl Service<Request<Body>, Response = axum::response::Response, Error = std::convert::Infallible>,
    method: Method,
    uri: &str,
    body: Option<String>,
) -> anyhow::Result<axum::response::Response> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri);

    if let Some(body_content) = body {
        request = request.header("content-type", "application/json");
    }

    let request = request.body(
        body.unwrap_or_default().into()
    )?;

    let response = app.call(request).await?;
    Ok(response)
}

/// Helper to create a test product request
pub fn create_test_product_request() -> serde_json::Value {
    serde_json::json!({
        "name": "Test Product",
        "description": "A product for integration testing",
        "tracker_type": "price",
        "sources": [{
            "url": "https://httpbin.org/json",
            "store_name": "Test Store",
            "selector": null
        }],
        "check_interval": "0 * * * *",
        "is_active": true,
        "threshold_value": 10.0,
        "threshold_type": "absolute",
        "notify_on": ["decreased"]
    })
}

/// Helper to wait for async operations
pub async fn wait_for_condition<F, Fut>(mut condition: F, timeout_seconds: u64) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_seconds);
    
    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    
    false
}