use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::config::AppConfig;
use crate::element_finder::{ElementFinder, ElementFinderRequest, ElementSelector};
use crate::models::{Product, NewProduct, Source, NewSource, PriceComparison, TrackerType, NotifyOn, SelectorType};
use crate::plugins::manager::PluginManager;
use crate::scraper::{WebScraper, ScrapeRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRequest {
    pub name: String,
    pub description: Option<String>,
    pub urls: Vec<String>,
    pub tracker_type: TrackerType,
    pub notify_on: NotifyOn,
    pub check_interval: String, // Cron expression
    pub selector: Option<ElementSelector>,
    pub threshold_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub check_interval: Option<String>,
    pub is_active: Option<bool>,
    pub threshold_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceCheckResult {
    pub source_id: String,
    pub success: bool,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub text_changed: bool,
    pub value_changed: bool,
    pub should_notify: bool,
    pub error: Option<String>,
    pub response_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCheckResult {
    pub product_id: String,
    pub success: bool,
    pub sources_checked: usize,
    pub sources_succeeded: usize,
    pub changes_detected: usize,
    pub notifications_sent: usize,
    pub source_results: Vec<SourceCheckResult>,
    pub price_comparison: Option<PriceComparison>,
    pub error: Option<String>,
    pub total_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductStats {
    pub total_checks: u64,
    pub successful_checks: u64,
    pub failed_checks: u64,
    pub changes_detected: u64,
    pub notifications_sent: u64,
    pub average_response_time_ms: f64,
    pub last_check: Option<chrono::DateTime<chrono::Utc>>,
    pub last_change: Option<chrono::DateTime<chrono::Utc>>,
    pub health_score: f64, // 0.0 to 1.0
}

pub struct ProductManager {
    scraper: WebScraper,
    element_finder: ElementFinder,
    plugin_manager: PluginManager,
    config: AppConfig,
}

impl ProductManager {
    pub fn new(
        scraper: WebScraper,
        element_finder: ElementFinder,
        plugin_manager: PluginManager,
        config: AppConfig,
    ) -> Self {
        Self {
            scraper,
            element_finder,
            plugin_manager,
            config,
        }
    }

    pub async fn create_product(&self, request: ProductRequest) -> Result<Product> {
        // Validate URLs
        for url in &request.urls {
            if url::Url::parse(url).is_err() {
                return Err(anyhow::anyhow!("Invalid URL: {}", url));
            }
        }

        // Validate cron expression
        if !self.is_valid_cron(&request.check_interval) {
            return Err(anyhow::anyhow!("Invalid cron expression: {}", request.check_interval));
        }

        // Create product
        let product = Product::new(NewProduct {
            name: request.name,
            description: request.description,
            tracker_type: request.tracker_type,
            notify_on: Some(request.notify_on),
            threshold_type: None,
            threshold_value: request.threshold_value,
            check_interval: Some(request.check_interval),
        });

        // In a full implementation, we would create sources here
        // For now, we return the product and handle sources separately
        Ok(product)
    }

    pub async fn update_product(&self, product_id: &str, update: ProductUpdate) -> Result<Product> {
        // In a real implementation, this would load from database
        // For now, we'll create a mock product to demonstrate the update logic
        let mut product = Product::new(NewProduct {
            name: "Sample Product".to_string(),
            description: None,
            tracker_type: TrackerType::Price,
            notify_on: Some(NotifyOn::Decrease),
            threshold_type: None,
            threshold_value: None,
            check_interval: Some("0 0 * * *".to_string()),
        });
        product.id = product_id.to_string();

        // Apply updates
        if let Some(name) = update.name {
            product.name = name;
        }
        if let Some(description) = update.description {
            product.description = Some(description);
        }
        if let Some(interval) = update.check_interval {
            if !self.is_valid_cron(&interval) {
                return Err(anyhow::anyhow!("Invalid cron expression: {}", interval));
            }
            product.check_interval = interval;
        }
        if let Some(active) = update.is_active {
            product.is_active = active;
        }
        if let Some(threshold) = update.threshold_value {
            product.threshold_value = Some(threshold);
        }

        product.updated_at = chrono::Utc::now();
        Ok(product)
    }

    pub async fn check_product(&self, product: &Product) -> Result<ProductCheckResult> {
        let start_time = Instant::now();
        let mut source_results = Vec::new();
        let mut changes_detected = 0;
        let mut notifications_sent = 0;

        // In a real implementation, we would load sources from database
        // For now, create a mock source for testing
        let mock_sources = vec![Source::new(NewSource {
            product_id: product.id.clone(),
            url: "https://example.com".to_string(),
            store_name: None,
            title: "Test Source".to_string(),
            selector: ".price".to_string(),
            selector_type: Some(SelectorType::Css),
        })];
        
        // Check each source
        for source in &mock_sources {
            let result = self.check_source(source, product).await;
            
            if result.value_changed {
                changes_detected += 1;
            }
            
            if result.should_notify && product.is_active {
                // Send notifications (mock for now)
                notifications_sent += 1;
            }
            
            source_results.push(result);
        }

        // Calculate price comparison if this is a price tracker
        let price_comparison = if product.tracker_type == TrackerType::Price && source_results.len() > 1 {
            self.calculate_price_comparison(&source_results).await?
        } else {
            None
        };

        let total_time = start_time.elapsed().as_millis() as u64;
        let sources_succeeded = source_results.iter().filter(|r| r.success).count();

        Ok(ProductCheckResult {
            product_id: product.id.clone(),
            success: sources_succeeded > 0,
            sources_checked: source_results.len(),
            sources_succeeded,
            changes_detected,
            notifications_sent,
            source_results,
            price_comparison,
            error: None,
            total_time_ms: total_time,
        })
    }

    async fn check_source(&self, source: &Source, product: &Product) -> SourceCheckResult {
        let start_time = Instant::now();
        
        // Create scrape request
        let scrape_request = ScrapeRequest {
            url: source.url.clone(),
            selector: source.selector.clone(),
            selector_type: match source.selector_type {
                SelectorType::Css => "css".to_string(),
                SelectorType::Xpath => "xpath".to_string(), 
                SelectorType::Text => "text".to_string(),
            },
            wait_for_selector: None, // Source model doesn't have wait_selector field
            screenshot: false,
            user_agent: None,
            timeout_ms: Some(self.config.scraper.request_timeout * 1000),
        };

        // Perform scrape
        let scrape_result = match self.scraper.scrape(scrape_request).await {
            Ok(result) => result,
            Err(e) => {
                return SourceCheckResult {
                    source_id: source.id.clone(),
                    success: false,
                    old_value: None,
                    new_value: None,
                    text_changed: false,
                    value_changed: false,
                    should_notify: false,
                    error: Some(e.to_string()),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                };
            }
        };

        if !scrape_result.success {
            return SourceCheckResult {
                source_id: source.id.clone(),
                success: false,
                old_value: None,
                new_value: None,
                text_changed: false,
                value_changed: false,
                should_notify: false,
                error: scrape_result.error,
                response_time_ms: start_time.elapsed().as_millis() as u64,
            };
        }

        // Extract value using appropriate tracker plugin
        let new_text = scrape_result.text.unwrap_or_default();
        let new_value = match self.extract_value(&new_text, &product.tracker_type).await {
            Ok(value) => value,
            Err(e) => {
                return SourceCheckResult {
                    source_id: source.id.clone(),
                    success: false,
                    old_value: None,
                    new_value: None,
                    text_changed: false,
                    value_changed: false,
                    should_notify: false,
                    error: Some(format!("Value extraction failed: {}", e)),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                };
            }
        };

        // Compare with previous values
        let old_text = source.current_text.as_deref().unwrap_or("");
        let old_value = source.get_current_value();
        
        let text_changed = old_text != new_text;
        let value_changed = old_value.as_ref() != Some(&new_value);
        let should_notify = product.should_notify(old_value.as_ref(), &new_value);

        SourceCheckResult {
            source_id: source.id.clone(),
            success: true,
            old_value,
            new_value: Some(new_value),
            text_changed,
            value_changed,
            should_notify,
            error: None,
            response_time_ms: start_time.elapsed().as_millis() as u64,
        }
    }

    async fn extract_value(&self, text: &str, tracker_type: &TrackerType) -> Result<serde_json::Value> {
        match tracker_type {
            TrackerType::Price => {
                match self.plugin_manager.parse_value_with_tracker("price", text).await {
                    Ok(value) => Ok(value),
                    Err(e) => Err(anyhow::anyhow!("Price tracker failed: {}", e))
                }
            }
            TrackerType::Version => {
                match self.plugin_manager.parse_value_with_tracker("version", text).await {
                    Ok(value) => Ok(value),
                    Err(e) => Err(anyhow::anyhow!("Version tracker failed: {}", e))
                }
            }
            TrackerType::Number => {
                match self.plugin_manager.parse_value_with_tracker("number", text).await {
                    Ok(value) => Ok(value),
                    Err(e) => Err(anyhow::anyhow!("Number tracker failed: {}", e))
                }
            }
        }
    }

    async fn calculate_price_comparison(&self, source_results: &[SourceCheckResult]) -> Result<Option<PriceComparison>> {
        let sources: Vec<_> = source_results
            .iter()
            .filter_map(|result| {
                if result.success {
                    result.new_value.as_ref().map(|value| crate::models::SourceComparison {
                        source_id: result.source_id.clone(),
                        url: "".to_string(), // Would be populated from database
                        store_name: "".to_string(), // Would be populated from database
                        value: value.clone(),
                        formatted_value: "".to_string(), // Would be populated from value formatting
                    })
                } else {
                    None
                }
            })
            .collect();

        if sources.len() < 2 {
            return Ok(None);
        }

        // For now, return None since we need to check the PriceComparison API
        Ok(None)
    }

    async fn auto_detect_selector(&self, source: &Source, tracker_type: &TrackerType) -> Result<Option<ElementSelector>> {
        let request = ElementFinderRequest {
            url: source.url.clone(),
            target_text: None,
            target_type: match tracker_type {
                TrackerType::Price => "price".to_string(),
                TrackerType::Version => "version".to_string(),
                TrackerType::Number => "number".to_string(),
            },
            context_hints: self.get_context_hints(tracker_type),
            exclude_selectors: vec![],
            max_matches: 5,
        };

        let result = self.element_finder.find_elements(request).await?;
        
        if result.success && !result.matches.is_empty() {
            // Return the highest confidence match
            let best_match = &result.matches[0];
            Ok(Some(best_match.selector.clone()))
        } else {
            Ok(None)
        }
    }

    fn get_default_selector(&self, tracker_type: &TrackerType) -> String {
        match tracker_type {
            TrackerType::Price => ".price".to_string(),
            TrackerType::Version => ".version".to_string(),
            TrackerType::Number => ".count".to_string(),
        }
    }

    fn get_context_hints(&self, tracker_type: &TrackerType) -> Vec<String> {
        match tracker_type {
            TrackerType::Price => vec!["price".to_string(), "cost".to_string(), "amount".to_string()],
            TrackerType::Version => vec!["version".to_string(), "release".to_string(), "build".to_string()],
            TrackerType::Number => vec!["count".to_string(), "quantity".to_string(), "stock".to_string()],
        }
    }

    fn is_valid_cron(&self, cron_expr: &str) -> bool {
        // Basic cron validation - should have 5 parts (minute hour day month weekday)
        let parts: Vec<&str> = cron_expr.split_whitespace().collect();
        if parts.len() != 5 {
            return false;
        }

        // Each part should be valid
        for part in parts {
            if part.is_empty() {
                return false;
            }
            // Allow numbers, ranges, lists, and wildcards
            if !part.chars().all(|c| c.is_ascii_digit() || c == '*' || c == '-' || c == ',' || c == '/') {
                return false;
            }
        }

        true
    }

    pub async fn get_product_stats(&self, product_id: &str) -> Result<ProductStats> {
        // In a real implementation, this would query the database
        // For now, return mock stats
        Ok(ProductStats {
            total_checks: 150,
            successful_checks: 142,
            failed_checks: 8,
            changes_detected: 12,
            notifications_sent: 8,
            average_response_time_ms: 850.5,
            last_check: Some(chrono::Utc::now() - chrono::Duration::hours(2)),
            last_change: Some(chrono::Utc::now() - chrono::Duration::days(3)),
            health_score: 0.95,
        })
    }

    pub async fn validate_product_sources(&self, product: &Product) -> Result<Vec<(String, bool, Option<String>)>> {
        let mut results = Vec::new();
        
        // In a real implementation, we would load sources from database
        // For now, create a mock source for testing
        let mock_sources = vec![Source::new(NewSource {
            product_id: product.id.clone(),
            url: "https://example.com".to_string(),
            store_name: None,
            title: "Test Source".to_string(),
            selector: ".price".to_string(),
            selector_type: Some(SelectorType::Css),
        })];
        
        for source in &mock_sources {
            let is_valid = match self.element_finder.validate_selector(&source.url, &ElementSelector {
                selector_type: source.selector_type.clone(),
                selector: source.selector.clone(),
                attributes: vec!["text".to_string()],
                validate_text: None,
                wait_selector: None, // Source model doesn't have wait_selector field
            }).await {
                Ok(valid) => valid,
                Err(e) => {
                    results.push((source.id.clone(), false, Some(e.to_string())));
                    continue;
                }
            };

            results.push((source.id.clone(), is_valid, None));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScraperConfig;

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

    async fn create_test_manager() -> Option<ProductManager> {
        let config = get_test_config();
        
        let scraper = match WebScraper::new(config.scraper.clone()) {
            Ok(s) => s,
            Err(_) => return None, // Skip test if Chrome not available
        };

        let element_finder = ElementFinder::new(scraper.clone());
        let plugin_manager = PluginManager::new();
        
        Some(ProductManager::new(scraper, element_finder, plugin_manager, config))
    }

    #[test]
    fn test_product_request_creation() {
        let request = ProductRequest {
            name: "Test Product".to_string(),
            description: Some("Test description".to_string()),
            urls: vec!["https://example.com/product".to_string()],
            tracker_type: TrackerType::Price,
            notify_on: NotifyOn::Decrease,
            check_interval: "0 0 * * *".to_string(),
            selector: None,
            threshold_value: Some(10.0),
        };

        assert_eq!(request.name, "Test Product");
        assert_eq!(request.urls.len(), 1);
        assert_eq!(request.tracker_type, TrackerType::Price);
        assert_eq!(request.threshold_value, Some(10.0));
    }

    #[test]
    fn test_product_update_creation() {
        let update = ProductUpdate {
            name: Some("Updated Product".to_string()),
            description: Some("Updated description".to_string()),
            check_interval: Some("0 */6 * * *".to_string()),
            is_active: Some(false),
            threshold_value: Some(5.0),
        };

        assert_eq!(update.name, Some("Updated Product".to_string()));
        assert_eq!(update.is_active, Some(false));
        assert_eq!(update.threshold_value, Some(5.0));
    }

    #[tokio::test]
    async fn test_create_product_basic() {
        if let Some(manager) = create_test_manager().await {
            let request = ProductRequest {
                name: "Test Product".to_string(),
                description: None,
                urls: vec!["https://example.com/product".to_string()],
                tracker_type: TrackerType::Price,
                notify_on: NotifyOn::Decrease,
                check_interval: "0 0 * * *".to_string(),
                selector: None,
                threshold_value: None,
            };

            let result = manager.create_product(request).await;
            assert!(result.is_ok());
            
            let product = result.unwrap();
            assert_eq!(product.name, "Test Product");
            assert_eq!(product.tracker_type, TrackerType::Price);
        }
    }

    #[tokio::test]
    async fn test_create_product_invalid_url() {
        if let Some(manager) = create_test_manager().await {
            let request = ProductRequest {
                name: "Test Product".to_string(),
                description: None,
                urls: vec!["invalid-url".to_string()],
                tracker_type: TrackerType::Price,
                notify_on: NotifyOn::Decrease,
                check_interval: "0 0 * * *".to_string(),
                selector: None,
                threshold_value: None,
            };

            let result = manager.create_product(request).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Invalid URL"));
        }
    }

    #[tokio::test]
    async fn test_create_product_invalid_cron() {
        if let Some(manager) = create_test_manager().await {
            let request = ProductRequest {
                name: "Test Product".to_string(),
                description: None,
                urls: vec!["https://example.com".to_string()],
                tracker_type: TrackerType::Price,
                notify_on: NotifyOn::Decrease,
                check_interval: "invalid cron".to_string(),
                selector: None,
                threshold_value: None,
            };

            let result = manager.create_product(request).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Invalid cron expression"));
        }
    }

    #[tokio::test]
    async fn test_update_product() {
        if let Some(manager) = create_test_manager().await {
            let update = ProductUpdate {
                name: Some("Updated Name".to_string()),
                description: Some("Updated description".to_string()),
                check_interval: Some("0 */2 * * *".to_string()),
                is_active: Some(false),
                threshold_value: Some(15.0),
            };

            let result = manager.update_product("test-id", update).await;
            assert!(result.is_ok());

            let product = result.unwrap();
            assert_eq!(product.name, "Updated Name");
            assert_eq!(product.check_interval, "0 */2 * * *");
            assert!(!product.is_active);
            assert_eq!(product.threshold_value, Some(15.0));
        }
    }

    #[test]
    fn test_cron_validation() {
        let config = get_test_config();
        let scraper = match WebScraper::new(config.scraper.clone()) {
            Ok(s) => s,
            Err(_) => return, // Skip if Chrome not available
        };
        
        let element_finder = ElementFinder::new(scraper.clone());
        let plugin_manager = PluginManager::new();
        let manager = ProductManager::new(scraper, element_finder, plugin_manager, config);

        // Valid cron expressions
        assert!(manager.is_valid_cron("0 0 * * *"));
        assert!(manager.is_valid_cron("*/15 * * * *"));
        assert!(manager.is_valid_cron("0 9-17 * * 1-5"));

        // Invalid cron expressions
        assert!(!manager.is_valid_cron("invalid"));
        assert!(!manager.is_valid_cron("0 0 * *")); // Too few parts
        assert!(!manager.is_valid_cron("0 0 * * * *")); // Too many parts
    }

    #[test]
    fn test_get_default_selectors() {
        let config = get_test_config();
        let scraper = match WebScraper::new(config.scraper.clone()) {
            Ok(s) => s,
            Err(_) => return,
        };
        
        let element_finder = ElementFinder::new(scraper.clone());
        let plugin_manager = PluginManager::new();
        let manager = ProductManager::new(scraper, element_finder, plugin_manager, config);

        assert_eq!(manager.get_default_selector(&TrackerType::Price), ".price");
        assert_eq!(manager.get_default_selector(&TrackerType::Version), ".version");
        assert_eq!(manager.get_default_selector(&TrackerType::Number), ".count");
    }

    #[test]
    fn test_get_context_hints() {
        let config = get_test_config();
        let scraper = match WebScraper::new(config.scraper.clone()) {
            Ok(s) => s,
            Err(_) => return,
        };
        
        let element_finder = ElementFinder::new(scraper.clone());
        let plugin_manager = PluginManager::new();
        let manager = ProductManager::new(scraper, element_finder, plugin_manager, config);

        let price_hints = manager.get_context_hints(&TrackerType::Price);
        assert!(price_hints.contains(&"price".to_string()));
        assert!(price_hints.contains(&"cost".to_string()));

        let version_hints = manager.get_context_hints(&TrackerType::Version);
        assert!(version_hints.contains(&"version".to_string()));
        assert!(version_hints.contains(&"release".to_string()));

        let number_hints = manager.get_context_hints(&TrackerType::Number);
        assert!(number_hints.contains(&"count".to_string()));
        assert!(number_hints.contains(&"quantity".to_string()));
    }

    #[tokio::test]
    async fn test_get_product_stats() {
        if let Some(manager) = create_test_manager().await {
            let stats = manager.get_product_stats("test-id").await.unwrap();
            
            assert!(stats.total_checks > 0);
            assert!(stats.successful_checks > 0);
            assert!(stats.health_score >= 0.0 && stats.health_score <= 1.0);
            assert!(stats.last_check.is_some());
        }
    }

    #[test]
    fn test_source_check_result() {
        let result = SourceCheckResult {
            source_id: "source123".to_string(),
            success: true,
            old_value: Some(serde_json::json!({"amount": "19.99"})),
            new_value: Some(serde_json::json!({"amount": "17.99"})),
            text_changed: true,
            value_changed: true,
            should_notify: true,
            error: None,
            response_time_ms: 850,
        };

        assert_eq!(result.source_id, "source123");
        assert!(result.success);
        assert!(result.value_changed);
        assert!(result.should_notify);
        assert_eq!(result.response_time_ms, 850);
    }

    #[test]
    fn test_product_check_result() {
        let result = ProductCheckResult {
            product_id: "product123".to_string(),
            success: true,
            sources_checked: 3,
            sources_succeeded: 2,
            changes_detected: 1,
            notifications_sent: 1,
            source_results: vec![],
            price_comparison: None,
            error: None,
            total_time_ms: 2500,
        };

        assert_eq!(result.product_id, "product123");
        assert!(result.success);
        assert_eq!(result.sources_checked, 3);
        assert_eq!(result.changes_detected, 1);
        assert_eq!(result.total_time_ms, 2500);
    }
}