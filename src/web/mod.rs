use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::Level;

use crate::{
    AppConfig, ProductManager, ProductScheduler,
};

pub mod handlers;
pub mod middleware;
pub mod responses;

pub use handlers::{
    // Product handlers
    list_products, create_product, get_product, update_product, delete_product,
    check_product_now, get_product_stats, get_product_health, toggle_product_status,
    check_multiple_products,
    // Scheduler handlers  
    schedule_product, unschedule_product, get_scheduler_stats, list_jobs,
    get_job_info, pause_job, resume_job,
    // System handlers
    system_info, system_metrics,
    // Page handlers
    setup_page, dashboard_page, products_page, scheduler_page,
};
pub use responses::*;

#[derive(Clone)]
pub struct AppState {
    pub product_manager: Arc<ProductManager>,
    pub scheduler: Arc<tokio::sync::Mutex<ProductScheduler>>,
    pub config: AppConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(20),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilterParams {
    pub active: Option<bool>,
    pub tracker_type: Option<String>,
    pub search: Option<String>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        
        // API routes
        .nest("/api/v1", api_routes())
        
        // Dashboard routes (will serve HTML)
        .route("/", get(dashboard_page))
        .route("/dashboard", get(dashboard_page))
        .route("/setup", get(setup_page))
        .route("/products", get(products_page))
        .route("/products/new", get(new_product_page))
        .route("/products/:id", get(product_detail_page))
        .route("/products/:id/edit", get(edit_product_page))
        .route("/scheduler", get(scheduler_page))
        .route("/settings", get(settings_page))
        
        // Static assets (placeholder for now)
        .route("/static/*file", get(serve_static))
        
        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                    .on_response(tower_http::trace::DefaultOnResponse::new().level(Level::INFO)))
                .layer(CompressionLayer::new())
                .layer(CorsLayer::permissive())
        )
        .with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        // Product management
        .route("/products", get(list_products).post(create_product))
        .route("/products/:id", get(get_product).put(update_product).delete(delete_product))
        .route("/products/:id/check", post(check_product_now))
        .route("/products/:id/stats", get(get_product_stats))
        .route("/products/:id/health", get(get_product_health))
        .route("/products/:id/toggle", post(toggle_product_status))
        .route("/products/:id/schedule", post(schedule_product).delete(unschedule_product))
        
        // Bulk operations
        .route("/products/check/bulk", post(check_multiple_products))
        
        // Scheduler management
        .route("/scheduler/stats", get(get_scheduler_stats))
        .route("/scheduler/jobs", get(list_jobs))
        .route("/scheduler/jobs/:product_id", get(get_job_info))
        .route("/scheduler/jobs/:product_id/pause", post(pause_job))
        .route("/scheduler/jobs/:product_id/resume", post(resume_job))
        
        // System information
        .route("/system/info", get(system_info))
        .route("/system/metrics", get(system_metrics))
}

// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "version": env!("CARGO_PKG_VERSION"),
        "service": "uatu-watcher"
    }))
}

// Dashboard route (will return HTML)
async fn dashboard(State(_state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Uatu Watcher - Dashboard</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        nav ul { list-style: none; padding: 0; }
        nav li { display: inline; margin-right: 20px; }
        nav a { text-decoration: none; color: #007bff; }
        nav a:hover { text-decoration: underline; }
        #stats { margin-top: 20px; padding: 20px; background: #f8f9fa; border-radius: 5px; }
    </style>
</head>
<body>
    <h1>Uatu Watcher Dashboard</h1>
    <p>Welcome to the Uatu Watcher price tracking system.</p>
    <nav>
        <ul>
            <li><a href="/products">Products</a></li>
            <li><a href="/scheduler">Scheduler</a></li>
            <li><a href="/settings">Settings</a></li>
            <li><a href="/api/v1/system/info">API Info</a></li>
        </ul>
    </nav>
    <div id="stats">
        <p>Loading system information...</p>
    </div>
    <script>
        fetch('/api/v1/system/info')
            .then(response => response.json())
            .then(data => {
                document.getElementById('stats').innerHTML = 
                    '<h3>System Information</h3>' +
                    '<p><strong>Version:</strong> ' + data.version + '</p>' +
                    '<p><strong>Uptime:</strong> ' + data.uptime + '</p>' +
                    '<p><strong>Status:</strong> ' + data.status + '</p>';
            })
            .catch(error => {
                document.getElementById('stats').innerHTML = 
                    '<p>Error loading system information: ' + error + '</p>';
            });
    </script>
</body>
</html>"#.to_string();
    
    Ok(Html(html))
}

// Product pages (placeholders)

async fn new_product_page() -> Html<&'static str> {
    Html(r#"
    <html>
    <head><title>New Product - Uatu Watcher</title></head>
    <body>
        <h1>New Product</h1>
        <p>Product creation form will be implemented here.</p>
        <p><a href="/products">← Back to Products</a></p>
    </body>
    </html>
    "#)
}

async fn product_detail_page(Path(id): Path<String>) -> Html<String> {
    Html(format!(r#"
    <html>
    <head><title>Product {} - Uatu Watcher</title></head>
    <body>
        <h1>Product Details</h1>
        <p>Details for product ID: {}</p>
        <p><a href="/products">← Back to Products</a></p>
    </body>
    </html>
    "#, id, id))
}

async fn edit_product_page(Path(id): Path<String>) -> Html<String> {
    Html(format!(r#"
    <html>
    <head><title>Edit Product {} - Uatu Watcher</title></head>
    <body>
        <h1>Edit Product</h1>
        <p>Edit form for product ID: {}</p>
        <p><a href="/products/{}">← Back to Product</a></p>
    </body>
    </html>
    "#, id, id, id))
}


async fn settings_page() -> Html<&'static str> {
    Html(r#"
    <html>
    <head><title>Settings - Uatu Watcher</title></head>
    <body>
        <h1>Settings</h1>
        <p>System settings interface will be implemented here.</p>
        <p><a href="/">← Back to Dashboard</a></p>
    </body>
    </html>
    "#)
}

use axum::response::{Response, IntoResponse};
use axum::http::header;

async fn serve_static(Path(file): Path<String>) -> Result<Response, StatusCode> {
    match file.as_str() {
        "css/style.css" => {
            let css_content = include_str!("../../static/css/style.css");
            Ok((
                [(header::CONTENT_TYPE, "text/css")],
                css_content
            ).into_response())
        },
        "js/app.js" => {
            let js_content = include_str!("../../static/js/app.js");
            Ok((
                [(header::CONTENT_TYPE, "application/javascript")],
                js_content
            ).into_response())
        },
        _ => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn serve(config: AppConfig, state: AppState) -> anyhow::Result<()> {
    let app = create_router(state);
    
    let listener = tokio::net::TcpListener::bind(
        format!("{}:{}", config.server.host, config.server.port)
    ).await?;
    
    tracing::info!(
        "Server starting on {}:{}", 
        config.server.host, 
        config.server.port
    );
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use crate::{
        element_finder::ElementFinder,
        plugins::manager::PluginManager,
        scraper::WebScraper,
        config::ScraperConfig,
    };

    async fn create_test_app_state() -> Option<AppState> {
        let config = get_test_config();
        
        let scraper = match WebScraper::new(config.scraper.clone()) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let element_finder = ElementFinder::new(scraper.clone());
        let plugin_manager = PluginManager::new();
        let product_manager = Arc::new(ProductManager::new(
            scraper, 
            element_finder, 
            plugin_manager, 
            config.clone()
        ));

        let scheduler = match ProductScheduler::new(
            Arc::clone(&product_manager), 
            config.scheduler.clone()
        ).await {
            Ok(s) => Arc::new(tokio::sync::Mutex::new(s)),
            Err(_) => return None,
        };

        Some(AppState {
            product_manager,
            scheduler,
            config,
        })
    }

    fn get_test_config() -> AppConfig {
        AppConfig {
            server: crate::config::ServerConfig {
                host: "localhost".to_string(),
                port: 3000,
                base_url: "http://localhost:3000".to_string(),
                request_timeout: 30,
                shutdown_timeout: 10,
            },
            database: crate::config::DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                max_connections: 5,
                min_connections: 1,
                acquire_timeout: 30,
            },
            security: crate::config::SecurityConfig {
                secret_key: "test-key-32-characters-minimum".to_string(),
                jwt_expiry: 3600,
                rate_limit_requests: 100,
                rate_limit_window: 60,
            },
            scraper: ScraperConfig {
                max_concurrent_checks: 2,
                retry_attempts: 1,
                retry_delay_ms: 1000,
                request_timeout: 10,
                user_agent: "TestAgent/1.0".to_string(),
                chrome_path: None,
            },
            scheduler: crate::config::SchedulerConfig {
                default_interval: "0 0 * * *".to_string(),
                max_running_jobs: 5,
                job_timeout: 300,
            },
            notifications: crate::config::NotificationsConfig {
                smtp: crate::config::SmtpConfig {
                    host: "localhost".to_string(),
                    port: 587,
                    username: None,
                    password: None,
                    from_address: None,
                    from_name: "Test".to_string(),
                    use_tls: false,
                },
                discord: crate::config::DiscordConfig {
                    webhook_url: None,
                    username: "Test".to_string(),
                    avatar_url: None,
                },
            },
            screenshots: crate::config::ScreenshotConfig {
                enabled: false,
                quality: 80,
                max_size_mb: 5,
                retention_days: 30,
            },
            metrics: crate::config::MetricsConfig {
                enabled: false,
                port: 9001,
                endpoint: "/metrics".to_string(),
            },
            performance: crate::config::PerformanceConfig {
                thread_pool_size: 2,
                memory_limit_mb: 256,
                enable_compression: true,
            },
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        if let Some(state) = create_test_app_state().await {
            let app = create_router(state);

            let response = app
                .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn test_dashboard_route() {
        if let Some(state) = create_test_app_state().await {
            let app = create_router(state);

            let response = app
                .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn test_products_route() {
        if let Some(state) = create_test_app_state().await {
            let app = create_router(state);

            let response = app
                .oneshot(Request::builder().uri("/products").body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn test_scheduler_route() {
        if let Some(state) = create_test_app_state().await {
            let app = create_router(state);

            let response = app
                .oneshot(Request::builder().uri("/scheduler").body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn test_static_css_route() {
        if let Some(state) = create_test_app_state().await {
            let app = create_router(state);

            let response = app
                .oneshot(Request::builder().uri("/static/style.css").body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn test_nonexistent_static_route() {
        if let Some(state) = create_test_app_state().await {
            let app = create_router(state);

            let response = app
                .oneshot(Request::builder().uri("/static/nonexistent.js").body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }
}