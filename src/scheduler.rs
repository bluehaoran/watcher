use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::config::SchedulerConfig;
use crate::models::Product;
use crate::product_manager::{ProductManager, ProductCheckResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    pub id: Uuid,
    pub product_id: String,
    pub cron_expression: String,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub run_count: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    Active,
    Paused,
    Disabled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStats {
    pub total_jobs: usize,
    pub active_jobs: usize,
    pub paused_jobs: usize,
    pub running_jobs: usize,
    pub completed_runs: u64,
    pub failed_runs: u64,
    pub average_run_time_ms: f64,
    pub uptime_seconds: u64,
}

pub struct ProductScheduler {
    scheduler: JobScheduler,
    product_manager: Arc<ProductManager>,
    jobs: Arc<RwLock<HashMap<String, JobInfo>>>, // product_id -> JobInfo
    running_jobs: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>, // product_id -> handle
    config: SchedulerConfig,
    start_time: DateTime<Utc>,
}

impl ProductScheduler {
    pub async fn new(product_manager: Arc<ProductManager>, config: SchedulerConfig) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        
        Ok(Self {
            scheduler,
            product_manager,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            running_jobs: Arc::new(Mutex::new(HashMap::new())),
            config,
            start_time: Utc::now(),
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        self.scheduler.start().await?;
        tracing::info!("Product scheduler started");
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        // Cancel all running jobs
        let mut running_jobs = self.running_jobs.lock().await;
        for (product_id, handle) in running_jobs.drain() {
            handle.abort();
            tracing::debug!("Cancelled running job for product: {}", product_id);
        }

        // Shutdown the scheduler
        self.scheduler.shutdown().await?;
        tracing::info!("Product scheduler shutdown");
        Ok(())
    }

    /// Schedule a product for regular checking
    pub async fn schedule_product(&self, product: &Product) -> Result<()> {
        if !product.is_active || product.is_paused {
            return Err(anyhow::anyhow!("Cannot schedule inactive or paused product"));
        }

        // Remove existing job if any
        self.unschedule_product(&product.id).await?;

        // Create job info
        let job_info = JobInfo {
            id: Uuid::new_v4(),
            product_id: product.id.clone(),
            cron_expression: product.check_interval.clone(),
            status: JobStatus::Active,
            created_at: Utc::now(),
            last_run: None,
            next_run: None,
            run_count: 0,
            success_count: 0,
            error_count: 0,
            last_error: None,
        };

        // Create the cron job
        let product_manager = Arc::clone(&self.product_manager);
        let jobs = Arc::clone(&self.jobs);
        let running_jobs = Arc::clone(&self.running_jobs);
        let product_id_for_job = product.id.clone();
        let product_clone = product.clone();

        let job = Job::new_async(product.check_interval.as_str(), move |_uuid, _l| {
            let product_manager = Arc::clone(&product_manager);
            let jobs = Arc::clone(&jobs);
            let running_jobs = Arc::clone(&running_jobs);
            let product_id_inner = product_id_for_job.clone();
            let product = product_clone.clone();

            Box::pin(async move {
                let product_id_for_spawn = product_id_inner.clone();
                let job_handle = tokio::spawn(async move {
                    Self::execute_product_check(
                        product_manager,
                        jobs,
                        product_id_for_spawn,
                        &product,
                    ).await;
                });

                // Store the job handle
                {
                    let mut running_jobs = running_jobs.lock().await;
                    let product_id_store = product_id_inner.clone();
                    running_jobs.insert(product_id_store, job_handle);
                }

                // Wait for completion and remove handle
                let product_id_cleanup = product_id_inner;
                tokio::spawn(async move {
                    // Give the job handle a chance to complete
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    let mut running_jobs = running_jobs.lock().await;
                    running_jobs.remove(&product_id_cleanup);
                });
            })
        })?;

        // Add job to scheduler
        self.scheduler.add(job).await?;

        // Store job info
        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(product.id.clone(), job_info);
        }

        tracing::info!("Scheduled product {} with interval: {}", product.id, product.check_interval);
        Ok(())
    }

    /// Unschedule a product
    pub async fn unschedule_product(&self, product_id: &str) -> Result<()> {
        // Cancel running job if any
        {
            let mut running_jobs = self.running_jobs.lock().await;
            if let Some(handle) = running_jobs.remove(product_id) {
                handle.abort();
                tracing::debug!("Cancelled running job for product: {}", product_id);
            }
        }

        // Remove from jobs tracking
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job_info) = jobs.remove(product_id) {
                // Note: tokio-cron-scheduler doesn't provide easy removal by product_id
                // In a full implementation, we'd need to track job UUIDs for removal
                tracing::info!("Unscheduled product {} (job: {})", product_id, job_info.id);
            }
        }

        Ok(())
    }

    /// Update schedule for a product
    pub async fn reschedule_product(&self, product: &Product) -> Result<()> {
        self.unschedule_product(&product.id).await?;
        
        if product.is_active && !product.is_paused {
            self.schedule_product(product).await?;
        }
        
        Ok(())
    }

    /// Get job information for a product
    pub async fn get_job_info(&self, product_id: &str) -> Option<JobInfo> {
        let jobs = self.jobs.read().await;
        jobs.get(product_id).cloned()
    }

    /// Get all scheduled jobs
    pub async fn list_jobs(&self) -> Vec<JobInfo> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Get scheduler statistics
    pub async fn get_stats(&self) -> SchedulerStats {
        let jobs = self.jobs.read().await;
        let running_jobs = self.running_jobs.lock().await;

        let total_jobs = jobs.len();
        let active_jobs = jobs.values().filter(|j| j.status == JobStatus::Active).count();
        let paused_jobs = jobs.values().filter(|j| j.status == JobStatus::Paused).count();
        let running_jobs_count = running_jobs.len();

        let _total_runs: u64 = jobs.values().map(|j| j.run_count).sum();
        let successful_runs: u64 = jobs.values().map(|j| j.success_count).sum();
        let failed_runs: u64 = jobs.values().map(|j| j.error_count).sum();

        let uptime = Utc::now().signed_duration_since(self.start_time);

        SchedulerStats {
            total_jobs,
            active_jobs,
            paused_jobs,
            running_jobs: running_jobs_count,
            completed_runs: successful_runs,
            failed_runs,
            average_run_time_ms: 0.0, // Would be calculated from actual run times
            uptime_seconds: uptime.num_seconds().max(0) as u64,
        }
    }

    /// Check if a product job is currently running
    pub async fn is_job_running(&self, product_id: &str) -> bool {
        let running_jobs = self.running_jobs.lock().await;
        running_jobs.contains_key(product_id)
    }

    /// Pause a scheduled job
    pub async fn pause_job(&self, product_id: &str) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        if let Some(job_info) = jobs.get_mut(product_id) {
            job_info.status = JobStatus::Paused;
            tracing::info!("Paused job for product: {}", product_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Job not found for product: {}", product_id))
        }
    }

    /// Resume a paused job
    pub async fn resume_job(&self, product_id: &str) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        if let Some(job_info) = jobs.get_mut(product_id) {
            job_info.status = JobStatus::Active;
            tracing::info!("Resumed job for product: {}", product_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Job not found for product: {}", product_id))
        }
    }

    /// Execute a product check immediately (outside of schedule)
    pub async fn run_job_now(&self, product_id: &str, product: &Product) -> Result<ProductCheckResult> {
        if self.is_job_running(product_id).await {
            return Err(anyhow::anyhow!("Job is already running for product: {}", product_id));
        }

        tracing::info!("Running immediate check for product: {}", product_id);
        
        let result = self.product_manager.check_product(product).await?;
        
        // Update job statistics
        self.update_job_stats(product_id, &result, None).await;
        
        Ok(result)
    }

    async fn execute_product_check(
        product_manager: Arc<ProductManager>,
        jobs: Arc<RwLock<HashMap<String, JobInfo>>>,
        product_id: String,
        product: &Product,
    ) {
        let start_time = tokio::time::Instant::now();
        
        tracing::debug!("Starting scheduled check for product: {}", product_id);
        
        let result = match product_manager.check_product(product).await {
            Ok(result) => {
                tracing::info!("Completed check for product {}: {} changes detected", 
                              product_id, result.changes_detected);
                result
            }
            Err(e) => {
                tracing::error!("Failed to check product {}: {}", product_id, e);
                // Create error result
                ProductCheckResult {
                    product_id: product_id.clone(),
                    success: false,
                    sources_checked: 0,
                    sources_succeeded: 0,
                    changes_detected: 0,
                    notifications_sent: 0,
                    source_results: vec![],
                    price_comparison: None,
                    error: Some(e.to_string()),
                    total_time_ms: start_time.elapsed().as_millis() as u64,
                }
            }
        };

        // Update job statistics
        Self::update_job_stats_internal(jobs, &product_id, &result, Some(start_time.elapsed())).await;
    }

    async fn update_job_stats(&self, product_id: &str, result: &ProductCheckResult, duration: Option<tokio::time::Duration>) {
        Self::update_job_stats_internal(Arc::clone(&self.jobs), product_id, result, duration).await;
    }

    async fn update_job_stats_internal(
        jobs: Arc<RwLock<HashMap<String, JobInfo>>>,
        product_id: &str,
        result: &ProductCheckResult,
        _duration: Option<tokio::time::Duration>,
    ) {
        let mut jobs = jobs.write().await;
        if let Some(job_info) = jobs.get_mut(product_id) {
            job_info.last_run = Some(Utc::now());
            job_info.run_count += 1;
            
            if result.success {
                job_info.success_count += 1;
                job_info.last_error = None;
                if job_info.status == JobStatus::Error {
                    job_info.status = JobStatus::Active;
                }
            } else {
                job_info.error_count += 1;
                job_info.last_error = result.error.clone();
                job_info.status = JobStatus::Error;
            }
        }
    }

    /// Validate a cron expression
    pub fn validate_cron_expression(expression: &str) -> bool {
        // Basic cron validation - 5 parts (minute hour day month weekday)
        let parts: Vec<&str> = expression.split_whitespace().collect();
        if parts.len() != 5 {
            return false;
        }

        // Each part should be valid
        for part in parts {
            if part.is_empty() {
                return false;
            }
            // Allow numbers, ranges, lists, wildcards, and steps
            if !part.chars().all(|c| c.is_ascii_digit() || c == '*' || c == '-' || c == ',' || c == '/') {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScraperConfig;
    use crate::element_finder::ElementFinder;
    use crate::models::{NewProduct, TrackerType, NotifyOn};
    use crate::plugins::manager::PluginManager;
    use crate::scraper::WebScraper;
    use crate::AppConfig;

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

    async fn create_test_scheduler() -> Option<ProductScheduler> {
        let config = get_test_config();
        
        let scraper = match WebScraper::new(config.scraper.clone()) {
            Ok(s) => s,
            Err(_) => return None, // Skip test if Chrome not available
        };

        let element_finder = ElementFinder::new(scraper.clone());
        let plugin_manager = PluginManager::new();
        let product_manager = Arc::new(ProductManager::new(
            scraper, 
            element_finder, 
            plugin_manager, 
            config.clone()
        ));

        match ProductScheduler::new(product_manager, config.scheduler).await {
            Ok(scheduler) => Some(scheduler),
            Err(_) => None,
        }
    }

    fn create_test_product() -> Product {
        Product::new(NewProduct {
            name: "Test Product".to_string(),
            description: Some("Test product for scheduler".to_string()),
            tracker_type: TrackerType::Price,
            notify_on: Some(NotifyOn::Decrease),
            threshold_type: None,
            threshold_value: Some(10.0),
            check_interval: Some("*/5 * * * *".to_string()), // Every 5 minutes
        })
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        if let Some(mut scheduler) = create_test_scheduler().await {
            assert!(scheduler.start().await.is_ok());
            assert!(scheduler.shutdown().await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_product_scheduling() {
        if let Some(mut scheduler) = create_test_scheduler().await {
            let product = create_test_product();
            
            // Start scheduler
            scheduler.start().await.unwrap();
            
            // Schedule product
            let result = scheduler.schedule_product(&product).await;
            assert!(result.is_ok());
            
            // Check job was created
            let job_info = scheduler.get_job_info(&product.id).await;
            assert!(job_info.is_some());
            
            let job = job_info.unwrap();
            assert_eq!(job.product_id, product.id);
            assert_eq!(job.status, JobStatus::Active);
            
            scheduler.shutdown().await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_product_unscheduling() {
        if let Some(mut scheduler) = create_test_scheduler().await {
            let product = create_test_product();
            
            scheduler.start().await.unwrap();
            scheduler.schedule_product(&product).await.unwrap();
            
            // Verify job exists
            assert!(scheduler.get_job_info(&product.id).await.is_some());
            
            // Unschedule
            let result = scheduler.unschedule_product(&product.id).await;
            assert!(result.is_ok());
            
            // Verify job is gone
            assert!(scheduler.get_job_info(&product.id).await.is_none());
            
            scheduler.shutdown().await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_job_pause_resume() {
        if let Some(mut scheduler) = create_test_scheduler().await {
            let product = create_test_product();
            
            scheduler.start().await.unwrap();
            scheduler.schedule_product(&product).await.unwrap();
            
            // Pause job
            scheduler.pause_job(&product.id).await.unwrap();
            let job_info = scheduler.get_job_info(&product.id).await.unwrap();
            assert_eq!(job_info.status, JobStatus::Paused);
            
            // Resume job
            scheduler.resume_job(&product.id).await.unwrap();
            let job_info = scheduler.get_job_info(&product.id).await.unwrap();
            assert_eq!(job_info.status, JobStatus::Active);
            
            scheduler.shutdown().await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_scheduler_stats() {
        if let Some(mut scheduler) = create_test_scheduler().await {
            let product = create_test_product();
            
            scheduler.start().await.unwrap();
            scheduler.schedule_product(&product).await.unwrap();
            
            let stats = scheduler.get_stats().await;
            assert_eq!(stats.total_jobs, 1);
            assert_eq!(stats.active_jobs, 1);
            assert_eq!(stats.paused_jobs, 0);
            
            scheduler.shutdown().await.unwrap();
        }
    }

    #[test]
    fn test_cron_validation() {
        // Valid cron expressions
        assert!(ProductScheduler::validate_cron_expression("0 0 * * *"));
        assert!(ProductScheduler::validate_cron_expression("*/15 * * * *"));
        assert!(ProductScheduler::validate_cron_expression("0 9-17 * * 1-5"));
        assert!(ProductScheduler::validate_cron_expression("30 2 * * 0"));

        // Invalid cron expressions
        assert!(!ProductScheduler::validate_cron_expression("invalid"));
        assert!(!ProductScheduler::validate_cron_expression("0 0 * *")); // Too few parts
        assert!(!ProductScheduler::validate_cron_expression("0 0 * * * *")); // Too many parts
        assert!(!ProductScheduler::validate_cron_expression("")); // Empty
    }

    #[test]
    fn test_job_info_creation() {
        let job_info = JobInfo {
            id: Uuid::new_v4(),
            product_id: "test_product".to_string(),
            cron_expression: "0 0 * * *".to_string(),
            status: JobStatus::Active,
            created_at: Utc::now(),
            last_run: None,
            next_run: None,
            run_count: 0,
            success_count: 0,
            error_count: 0,
            last_error: None,
        };

        assert_eq!(job_info.product_id, "test_product");
        assert_eq!(job_info.status, JobStatus::Active);
        assert_eq!(job_info.run_count, 0);
    }

    #[test]
    fn test_scheduler_stats_creation() {
        let stats = SchedulerStats {
            total_jobs: 5,
            active_jobs: 3,
            paused_jobs: 2,
            running_jobs: 1,
            completed_runs: 150,
            failed_runs: 5,
            average_run_time_ms: 2500.0,
            uptime_seconds: 3600,
        };

        assert_eq!(stats.total_jobs, 5);
        assert_eq!(stats.active_jobs, 3);
        assert_eq!(stats.completed_runs, 150);
        assert_eq!(stats.uptime_seconds, 3600);
    }

    #[tokio::test]
    async fn test_inactive_product_scheduling() {
        if let Some(mut scheduler) = create_test_scheduler().await {
            let mut product = create_test_product();
            product.is_active = false;
            
            scheduler.start().await.unwrap();
            
            let result = scheduler.schedule_product(&product).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("inactive"));
            
            scheduler.shutdown().await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_job_running_check() {
        if let Some(mut scheduler) = create_test_scheduler().await {
            let product = create_test_product();
            
            scheduler.start().await.unwrap();
            scheduler.schedule_product(&product).await.unwrap();
            
            // Initially not running
            assert!(!scheduler.is_job_running(&product.id).await);
            
            scheduler.shutdown().await.unwrap();
        }
    }
}