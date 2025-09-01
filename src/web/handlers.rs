use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde_json::Value;

use crate::{
    models::Product,
    ProductRequest, ProductUpdate, ProductCheckResult, ProductStats,
    JobInfo, SchedulerStats,
};
use super::{AppState, PaginationParams, FilterParams, ApiResponse, AppError, PaginatedResponse};

// Product management handlers
pub async fn list_products(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filter): Query<FilterParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<Product>>>, AppError> {
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20);
    
    // Validate pagination parameters
    if page < 1 {
        return Err(AppError::bad_request("Page must be greater than 0"));
    }
    if !(1..100).contains(&per_page) {
        return Err(AppError::bad_request("Per page must be between 1 and 100"));
    }
    
    tracing::info!(
        "Listing products with pagination: page={}, per_page={}, filter={:?}",
        page, per_page, filter
    );

    match state.product_manager.list_products(page, per_page, &filter).await {
        Ok((products, total)) => {
            let paginated = PaginatedResponse::new(products, page, per_page, total);
            Ok(Json(ApiResponse::success(paginated)))
        }
        Err(e) => {
            tracing::error!("Failed to list products: {}", e);
            Err(AppError::internal("Failed to retrieve products"))
        }
    }
}

pub async fn create_product(
    State(state): State<AppState>,
    Json(request): Json<ProductRequest>,
) -> Result<Json<ApiResponse<Product>>, AppError> {
    // Validate the request
    if request.name.trim().is_empty() {
        return Err(AppError::bad_request("Product name is required"));
    }
    
    if request.sources.is_empty() {
        return Err(AppError::bad_request("At least one source URL is required"));
    }
    
    // Validate URLs
    for source in &request.sources {
        if url::Url::parse(&source.url).is_err() {
            return Err(AppError::bad_request(format!("Invalid URL: {}", source.url)));
        }
    }
    
    // Validate cron expression if provided
    if let Some(ref interval) = request.check_interval {
        if !interval.trim().is_empty() {
            // Simple cron validation - in a real implementation use a proper cron library
            let parts: Vec<&str> = interval.split_whitespace().collect();
            if parts.len() != 5 {
                return Err(AppError::bad_request("Invalid cron expression: must have 5 fields"));
            }
        }
    }

    match state.product_manager.create_product(request).await {
        Ok(product) => {
            tracing::info!("Created product: {} ({})", product.name, product.id);
            Ok(Json(ApiResponse::success(product)))
        }
        Err(e) => {
            tracing::error!("Failed to create product: {}", e);
            match e.to_string().as_str() {
                s if s.contains("duplicate") || s.contains("unique constraint") => {
                    Err(AppError::conflict("Product with this name already exists"))
                }
                _ => Err(AppError::internal("Failed to create product"))
            }
        }
    }
}

pub async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Product>>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    match state.product_manager.get_product(&id).await {
        Ok(Some(product)) => {
            tracing::debug!("Retrieved product: {} ({})", product.name, product.id);
            Ok(Json(ApiResponse::success(product)))
        }
        Ok(None) => {
            tracing::warn!("Product not found: {}", id);
            Err(AppError::not_found("Product"))
        }
        Err(e) => {
            tracing::error!("Failed to get product {}: {}", id, e);
            Err(AppError::internal("Failed to retrieve product"))
        }
    }
}

pub async fn update_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(update): Json<ProductUpdate>,
) -> Result<Json<ApiResponse<Product>>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    // Validate the update request
    if let Some(ref name) = update.name {
        if name.trim().is_empty() {
            return Err(AppError::bad_request("Product name cannot be empty"));
        }
    }

    // Validate URLs if sources are being updated
    if let Some(ref sources) = update.sources {
        if sources.is_empty() {
            return Err(AppError::bad_request("At least one source URL is required"));
        }
        
        for source in sources {
            if url::Url::parse(&source.url).is_err() {
                return Err(AppError::bad_request(format!("Invalid URL: {}", source.url)));
            }
        }
    }

    // Validate cron expression if being updated
    if let Some(ref interval) = update.check_interval {
        if !interval.trim().is_empty() {
            // Simple cron validation - in a real implementation use a proper cron library
            let parts: Vec<&str> = interval.split_whitespace().collect();
            if parts.len() != 5 {
                return Err(AppError::bad_request("Invalid cron expression: must have 5 fields"));
            }
        }
    }

    match state.product_manager.update_product(&id, update).await {
        Ok(product) => {
            tracing::info!("Updated product: {} ({})", product.name, product.id);
            Ok(Json(ApiResponse::success(product)))
        }
        Err(e) => {
            tracing::error!("Failed to update product {}: {}", id, e);
            match e.to_string().as_str() {
                s if s.contains("not found") => Err(AppError::not_found("Product")),
                s if s.contains("duplicate") || s.contains("unique constraint") => {
                    Err(AppError::conflict("Product with this name already exists"))
                }
                _ => Err(AppError::internal("Failed to update product"))
            }
        }
    }
}

pub async fn delete_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    // First check if product exists to provide better error message
    match state.product_manager.get_product(&id).await {
        Ok(Some(_)) => {
            // Product exists, proceed with deletion
            match state.product_manager.delete_product(&id).await {
                Ok(()) => {
                    tracing::info!("Deleted product: {}", id);
                    Ok(Json(ApiResponse::success(())))
                }
                Err(e) => {
                    tracing::error!("Failed to delete product {}: {}", id, e);
                    Err(AppError::internal("Failed to delete product"))
                }
            }
        }
        Ok(None) => {
            tracing::warn!("Attempted to delete non-existent product: {}", id);
            Err(AppError::not_found("Product"))
        }
        Err(e) => {
            tracing::error!("Failed to check product existence for deletion {}: {}", id, e);
            Err(AppError::internal("Failed to delete product"))
        }
    }
}

pub async fn check_product_now(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ProductCheckResult>>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    // First get the product to check
    let product = match state.product_manager.get_product(&id).await {
        Ok(Some(product)) => product,
        Ok(None) => {
            tracing::warn!("Attempted to check non-existent product: {}", id);
            return Err(AppError::not_found("Product"));
        }
        Err(e) => {
            tracing::error!("Failed to get product for checking {}: {}", id, e);
            return Err(AppError::internal("Failed to retrieve product for checking"));
        }
    };

    // Check if product is active
    if !product.is_active {
        return Err(AppError::bad_request("Cannot check inactive product"));
    }

    match state.product_manager.check_product(&product).await {
        Ok(result) => {
            tracing::info!("Checked product {} ({}) immediately: {} changes detected", 
                         product.name, id, result.changes_detected);
            Ok(Json(ApiResponse::success(result)))
        }
        Err(e) => {
            tracing::error!("Failed to check product {} ({}): {}", product.name, id, e);
            Err(AppError::internal("Failed to check product"))
        }
    }
}

pub async fn get_product_stats(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ProductStats>>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    // First verify product exists
    match state.product_manager.get_product(&id).await {
        Ok(Some(_)) => {
            // Product exists, get stats
            match state.product_manager.get_product_stats(&id).await {
                Ok(stats) => {
                    tracing::debug!("Retrieved stats for product: {}", id);
                    Ok(Json(ApiResponse::success(stats)))
                }
                Err(e) => {
                    tracing::error!("Failed to get product stats for {}: {}", id, e);
                    Err(AppError::internal("Failed to retrieve product statistics"))
                }
            }
        }
        Ok(None) => {
            tracing::warn!("Attempted to get stats for non-existent product: {}", id);
            Err(AppError::not_found("Product"))
        }
        Err(e) => {
            tracing::error!("Failed to verify product existence for stats {}: {}", id, e);
            Err(AppError::internal("Failed to retrieve product statistics"))
        }
    }
}

// Scheduler handlers
pub async fn schedule_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    // Get the actual product to schedule
    let product = match state.product_manager.get_product(&id).await {
        Ok(Some(product)) => product,
        Ok(None) => {
            tracing::warn!("Attempted to schedule non-existent product: {}", id);
            return Err(AppError::not_found("Product"));
        }
        Err(e) => {
            tracing::error!("Failed to get product for scheduling {}: {}", id, e);
            return Err(AppError::internal("Failed to retrieve product for scheduling"));
        }
    };

    // Check if product is active
    if !product.is_active {
        return Err(AppError::bad_request("Cannot schedule inactive product"));
    }

    // Check if product has a check interval
    if product.check_interval.is_empty() {
        return Err(AppError::bad_request("Product must have a check interval to be scheduled"));
    }

    let scheduler = state.scheduler.lock().await;
    match scheduler.schedule_product(&product).await {
        Ok(()) => {
            tracing::info!("Scheduled product: {} ({})", product.name, id);
            Ok(Json(ApiResponse::success(())))
        }
        Err(e) => {
            tracing::error!("Failed to schedule product {} ({}): {}", product.name, id, e);
            Err(AppError::internal("Failed to schedule product"))
        }
    }
}

pub async fn unschedule_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    let scheduler = state.scheduler.lock().await;
    match scheduler.unschedule_product(&id).await {
        Ok(()) => {
            tracing::info!("Unscheduled product: {}", id);
            Ok(Json(ApiResponse::success(())))
        }
        Err(e) => {
            tracing::error!("Failed to unschedule product {}: {}", id, e);
            Err(AppError::internal("Failed to unschedule product"))
        }
    }
}

pub async fn get_scheduler_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SchedulerStats>>, AppError> {
    let scheduler = state.scheduler.lock().await;
    let stats = scheduler.get_stats().await;
    tracing::debug!("Retrieved scheduler stats: {} active jobs", stats.active_jobs);
    Ok(Json(ApiResponse::success(stats)))
}

pub async fn list_jobs(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<JobInfo>>>, AppError> {
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20);
    
    // Validate pagination parameters
    if page < 1 {
        return Err(AppError::bad_request("Page must be greater than 0"));
    }
    if !(1..100).contains(&per_page) {
        return Err(AppError::bad_request("Per page must be between 1 and 100"));
    }

    let scheduler = state.scheduler.lock().await;
    let all_jobs = scheduler.list_jobs().await;
    
    // Apply pagination
    let offset = ((page - 1) * per_page) as usize;
    let jobs = all_jobs.clone().into_iter().skip(offset).take(per_page as usize).collect::<Vec<_>>();
    let total = all_jobs.len() as u32;
    
    let paginated = PaginatedResponse::new(jobs, page, per_page, total);
    Ok(Json(ApiResponse::success(paginated)))
}

pub async fn get_job_info(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
) -> Result<Json<ApiResponse<JobInfo>>, AppError> {
    if product_id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    let scheduler = state.scheduler.lock().await;
    match scheduler.get_job_info(&product_id).await {
        Some(job_info) => {
            tracing::debug!("Retrieved job info for product: {}", product_id);
            Ok(Json(ApiResponse::success(job_info)))
        }
        None => {
            tracing::warn!("Job not found for product: {}", product_id);
            Err(AppError::not_found("Job"))
        }
    }
}

pub async fn pause_job(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if product_id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    let scheduler = state.scheduler.lock().await;
    match scheduler.pause_job(&product_id).await {
        Ok(()) => {
            tracing::info!("Paused job for product: {}", product_id);
            Ok(Json(ApiResponse::success(())))
        }
        Err(e) => {
            tracing::error!("Failed to pause job for product {}: {}", product_id, e);
            Err(AppError::internal("Failed to pause job"))
        }
    }
}

pub async fn resume_job(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if product_id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    let scheduler = state.scheduler.lock().await;
    match scheduler.resume_job(&product_id).await {
        Ok(()) => {
            tracing::info!("Resumed job for product: {}", product_id);
            Ok(Json(ApiResponse::success(())))
        }
        Err(e) => {
            tracing::error!("Failed to resume job for product {}: {}", product_id, e);
            Err(AppError::internal("Failed to resume job"))
        }
    }
}

// System handlers
pub async fn system_info(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let info = serde_json::json!({
        "service": "uatu-watcher",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "uptime": "calculated from start time", // In a real implementation, calculate actual uptime
        "config": {
            "server": {
                "host": state.config.server.host,
                "port": state.config.server.port,
                "base_url": state.config.server.base_url
            },
            "scheduler": {
                "max_running_jobs": state.config.scheduler.max_running_jobs,
                "default_interval": state.config.scheduler.default_interval,
                "job_timeout": state.config.scheduler.job_timeout
            },
            "scraper": {
                "max_concurrent_checks": state.config.scraper.max_concurrent_checks,
                "retry_attempts": state.config.scraper.retry_attempts,
                "request_timeout": state.config.scraper.request_timeout
            }
        }
    });

    tracing::debug!("System info requested");
    Ok(Json(ApiResponse::success(info)))
}

pub async fn system_metrics(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    // In a full implementation, this would collect real metrics from system monitors
    let metrics = serde_json::json!({
        "timestamp": chrono::Utc::now(),
        "memory_usage": {
            "current_mb": "estimated", // In reality, would use system memory info
            "peak_mb": "estimated"
        },
        "requests": {
            "total": "counter would go here",
            "per_minute": "rate would go here",
            "errors_per_minute": "error rate would go here"
        },
        "products": {
            "total": 0, // Would query database
            "active": 0,
            "scheduled": 0
        },
        "checks": {
            "successful_24h": 0,
            "failed_24h": 0,
            "average_duration_ms": 0,
            "changes_detected_24h": 0
        },
        "scheduler": {
            "jobs_running": 0,
            "jobs_pending": 0,
            "jobs_failed": 0
        },
        "system": {
            "cpu_usage_percent": "estimated",
            "disk_usage_percent": "estimated",
            "uptime_seconds": "calculated"
        }
    });

    tracing::debug!("System metrics requested");
    Ok(Json(ApiResponse::success(metrics)))
}

// Additional product management endpoints

/// Bulk operation to check multiple products
pub async fn check_multiple_products(
    State(state): State<AppState>,
    Json(request): Json<Vec<String>>,
) -> Result<Json<ApiResponse<Vec<ProductCheckResult>>>, AppError> {
    if request.is_empty() {
        return Err(AppError::bad_request("At least one product ID is required"));
    }

    if request.len() > 50 {
        return Err(AppError::bad_request("Cannot check more than 50 products at once"));
    }

    let mut results = Vec::new();
    
    for product_id in &request {
        if product_id.trim().is_empty() {
            continue;
        }

        // Get the product
        match state.product_manager.get_product(product_id).await {
            Ok(Some(product)) => {
                if product.is_active {
                    match state.product_manager.check_product(&product).await {
                        Ok(result) => results.push(result),
                        Err(e) => {
                            tracing::error!("Failed to check product {}: {}", product_id, e);
                            // Create error result
                            results.push(ProductCheckResult {
                                product_id: product_id.clone(),
                                success: false,
                                sources_checked: 0,
                                sources_succeeded: 0,
                                changes_detected: 0,
                                notifications_sent: 0,
                                source_results: Vec::new(),
                                price_comparison: None,
                                error: Some(e.to_string()),
                                total_time_ms: 0,
                            });
                        }
                    }
                } else {
                    tracing::warn!("Skipping inactive product: {}", product_id);
                }
            }
            Ok(None) => {
                tracing::warn!("Product not found: {}", product_id);
            }
            Err(e) => {
                tracing::error!("Failed to get product {}: {}", product_id, e);
            }
        }
    }

    tracing::info!("Bulk check completed for {} products, {} results", request.len(), results.len());
    Ok(Json(ApiResponse::success(results)))
}

/// Toggle product active status
pub async fn toggle_product_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Product>>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    // Get current product
    let product = match state.product_manager.get_product(&id).await {
        Ok(Some(product)) => product,
        Ok(None) => return Err(AppError::not_found("Product")),
        Err(e) => {
            tracing::error!("Failed to get product for toggle {}: {}", id, e);
            return Err(AppError::internal("Failed to retrieve product"));
        }
    };

    // Create update to toggle status
    let update = ProductUpdate {
        name: None,
        description: None,
        sources: None,
        check_interval: None,
        is_active: Some(!product.is_active),
        threshold_value: None,
        threshold_type: None,
        notify_on: None,
    };

    match state.product_manager.update_product(&id, update).await {
        Ok(updated_product) => {
            tracing::info!("Toggled product {} status to {}", id, updated_product.is_active);
            Ok(Json(ApiResponse::success(updated_product)))
        }
        Err(e) => {
            tracing::error!("Failed to toggle product {} status: {}", id, e);
            Err(AppError::internal("Failed to toggle product status"))
        }
    }
}

/// Get product health check
pub async fn get_product_health(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    if id.trim().is_empty() {
        return Err(AppError::bad_request("Product ID is required"));
    }

    // Verify product exists
    let product = match state.product_manager.get_product(&id).await {
        Ok(Some(product)) => product,
        Ok(None) => return Err(AppError::not_found("Product")),
        Err(e) => {
            tracing::error!("Failed to get product for health check {}: {}", id, e);
            return Err(AppError::internal("Failed to retrieve product"));
        }
    };

    // Get product stats for health calculation
    let stats = match state.product_manager.get_product_stats(&id).await {
        Ok(stats) => stats,
        Err(e) => {
            tracing::error!("Failed to get product stats for health {}: {}", id, e);
            return Err(AppError::internal("Failed to retrieve product statistics"));
        }
    };

    let health = serde_json::json!({
        "product_id": id,
        "product_name": product.name,
        "is_active": product.is_active,
        "health_score": stats.health_score,
        "status": if stats.health_score > 0.8 { "healthy" } 
                else if stats.health_score > 0.5 { "degraded" } 
                else { "unhealthy" },
        "last_check": stats.last_check,
        "last_change": stats.last_change,
        "success_rate": if stats.total_checks > 0 { 
            stats.successful_checks as f64 / stats.total_checks as f64 
        } else { 
            0.0 
        },
        "total_checks": stats.total_checks,
        "failed_checks": stats.failed_checks,
        "average_response_time_ms": stats.average_response_time_ms,
        "issues": []  // In a real implementation, this would contain specific issues
    });

    tracing::debug!("Retrieved health check for product: {}", id);
    Ok(Json(ApiResponse::success(health)))
}

// Page handlers for web UI

use axum::response::Html;

/// GET /setup - Setup wizard page
pub async fn setup_page() -> Result<Html<String>, AppError> {
    let setup_template = include_str!("../../templates/setup.html");
    Ok(Html(setup_template.to_string()))
}

/// GET /dashboard - Dashboard page  
pub async fn dashboard_page() -> Result<Html<String>, AppError> {
    let dashboard_template = include_str!("../../templates/dashboard.html");
    Ok(Html(dashboard_template.to_string()))
}

/// GET /products - Products management page
pub async fn products_page() -> Result<Html<String>, AppError> {
    let template = r#"
    {% extends "base.html" %}
    {% block title %}Products - Uatu Watcher{% endblock %}
    {% block content %}
    <div class="page-header">
        <h1 class="page-title">Products</h1>
        <p class="page-description">Manage your tracked products and sources</p>
    </div>
    {% endblock %}
    "#;
    Ok(Html(template.to_string()))
}

/// GET /scheduler - Scheduler management page
pub async fn scheduler_page() -> Result<Html<String>, AppError> {
    let template = r#"
    {% extends "base.html" %}
    {% block title %}Scheduler - Uatu Watcher{% endblock %}
    {% block content %}
    <div class="page-header">
        <h1 class="page-title">Scheduler</h1>
        <p class="page-description">Manage automated checking schedules</p>
    </div>
    {% endblock %}
    "#;
    Ok(Html(template.to_string()))
}