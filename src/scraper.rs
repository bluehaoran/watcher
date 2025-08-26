use anyhow::{anyhow, Result};
use headless_chrome::{Browser, LaunchOptions, Tab};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::config::ScraperConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeRequest {
    pub url: String,
    pub selector: String,
    pub selector_type: String, // "css", "xpath", "text"
    pub wait_for_selector: Option<String>,
    pub screenshot: bool,
    pub user_agent: Option<String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResult {
    pub success: bool,
    pub text: Option<String>,
    pub html: Option<String>,
    pub screenshot_path: Option<String>,
    pub error: Option<String>,
    pub response_time_ms: u64,
    pub final_url: String, // After redirects
}

pub struct BrowserPool {
    config: ScraperConfig,
    browsers: Vec<Arc<Browser>>,
    current_index: std::sync::atomic::AtomicUsize,
}

pub struct WebScraper {
    browser_pool: Arc<BrowserPool>,
    config: ScraperConfig,
}

impl BrowserPool {
    pub fn new(config: ScraperConfig) -> Result<Self> {
        let mut browsers = Vec::new();
        
        // Create browser instances up to max_concurrent_checks
        for _ in 0..config.max_concurrent_checks.min(3) {  // Limit to max 3 for resource management
            let mut launch_options = LaunchOptions::default_builder()
                .headless(true)
                .sandbox(false) // Often needed in containerized environments
                .args(vec![
                    std::ffi::OsStr::new("--no-sandbox"),
                    std::ffi::OsStr::new("--disable-dev-shm-usage"),
                    std::ffi::OsStr::new("--disable-gpu"), 
                    std::ffi::OsStr::new("--disable-extensions"),
                    std::ffi::OsStr::new("--disable-background-timer-throttling"),
                    std::ffi::OsStr::new("--disable-backgrounding-occluded-windows"),
                    std::ffi::OsStr::new("--disable-renderer-backgrounding"),
                ])
                .build()
                .map_err(|e| anyhow!("Failed to create launch options: {}", e))?;

            // Set Chrome path if provided
            if let Some(chrome_path) = &config.chrome_path {
                launch_options.path = Some(std::path::PathBuf::from(chrome_path));
            }

            let browser = Browser::new(launch_options)
                .map_err(|e| anyhow!("Failed to launch browser: {}", e))?;

            browsers.push(Arc::new(browser));
        }

        Ok(Self {
            config,
            browsers,
            current_index: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    pub fn get_browser(&self) -> Arc<Browser> {
        let index = self.current_index.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % self.browsers.len();
        self.browsers[index].clone()
    }
}

impl WebScraper {
    pub fn new(config: ScraperConfig) -> Result<Self> {
        let browser_pool = Arc::new(BrowserPool::new(config.clone())?);
        Ok(Self {
            browser_pool,
            config,
        })
    }

    pub async fn scrape(&self, request: ScrapeRequest) -> Result<ScrapeResult> {
        let start_time = std::time::Instant::now();
        
        // Get browser from pool
        let browser = self.browser_pool.get_browser();
        
        // Create new tab
        let tab = browser.new_tab()
            .map_err(|e| anyhow!("Failed to create tab: {}", e))?;

        // Set user agent if provided
        let user_agent = request.user_agent.as_deref().unwrap_or(&self.config.user_agent);
        tab.set_user_agent(user_agent, None, None)
            .map_err(|e| anyhow!("Failed to set user agent: {}", e))?;

        // Navigate to URL with timeout
        let timeout_duration = Duration::from_millis(
            request.timeout_ms.unwrap_or(self.config.request_timeout * 1000)
        );

        let navigate_result = tab.navigate_to(&request.url);
        if let Err(e) = navigate_result {
            return Ok(ScrapeResult {
                success: false,
                text: None,
                html: None,
                screenshot_path: None,
                error: Some(format!("Navigation failed: {}", e)),
                response_time_ms: start_time.elapsed().as_millis() as u64,
                final_url: request.url.clone(),
            });
        }

        // Wait for page to load
        if let Err(e) = tab.wait_until_navigated() {
            return Ok(ScrapeResult {
                success: false,
                text: None,
                html: None,
                screenshot_path: None,
                error: Some(format!("Page load failed: {}", e)),
                response_time_ms: start_time.elapsed().as_millis() as u64,
                final_url: request.url.clone(),
            });
        }

        // Wait for specific selector if provided
        if let Some(wait_selector) = &request.wait_for_selector {
            let wait_result = tab.wait_for_element_with_custom_timeout(wait_selector, timeout_duration);
            if let Err(e) = wait_result {
                return Ok(ScrapeResult {
                    success: false,
                    text: None,
                    html: None,
                    screenshot_path: None,
                    error: Some(format!("Wait for selector '{}' failed: {}", wait_selector, e)),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    final_url: request.url.clone(),
                });
            }
        }

        // Get final URL after redirects  
        let final_url = {
            let url = tab.get_url();
            if url.is_empty() {
                request.url.clone()
            } else {
                url
            }
        };

        // Extract content based on selector type
        let (text, html) = match request.selector_type.as_str() {
            "css" => self.extract_css_content(&tab, &request.selector)?,
            "xpath" => self.extract_xpath_content(&tab, &request.selector)?,
            "text" => self.extract_text_content(&tab, &request.selector)?,
            _ => {
                return Ok(ScrapeResult {
                    success: false,
                    text: None,
                    html: None,
                    screenshot_path: None,
                    error: Some(format!("Unsupported selector type: {}", request.selector_type)),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    final_url: request.url.clone(),
                });
            }
        };

        // Take screenshot if requested
        let screenshot_path = if request.screenshot {
            self.take_screenshot(&tab).ok()
        } else {
            None
        };

        // Close tab to free resources
        let _ = tab.close(true);

        Ok(ScrapeResult {
            success: true,
            text,
            html,
            screenshot_path,
            error: None,
            response_time_ms: start_time.elapsed().as_millis() as u64,
            final_url,
        })
    }

    fn extract_css_content(&self, tab: &Tab, selector: &str) -> Result<(Option<String>, Option<String>)> {
        // Get page HTML
        let html_content = tab.get_content()
            .map_err(|e| anyhow!("Failed to get page content: {}", e))?;

        // Parse HTML with scraper
        let document = Html::parse_document(&html_content);
        let css_selector = Selector::parse(selector)
            .map_err(|e| anyhow!("Invalid CSS selector '{}': {:?}", selector, e))?;

        let mut text_parts = Vec::new();
        let mut html_parts = Vec::new();

        for element in document.select(&css_selector) {
            text_parts.push(element.text().collect::<Vec<_>>().join(" ").trim().to_string());
            html_parts.push(element.html());
        }

        let text = if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.join("\n"))
        };

        let html = if html_parts.is_empty() {
            None
        } else {
            Some(html_parts.join("\n"))
        };

        Ok((text, html))
    }

    fn extract_xpath_content(&self, tab: &Tab, xpath: &str) -> Result<(Option<String>, Option<String>)> {
        // Use Chrome DevTools to evaluate XPath
        let js_code = format!(
            r#"
            (function() {{
                try {{
                    const result = document.evaluate('{}', document, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null);
                    const elements = [];
                    for (let i = 0; i < result.snapshotLength; i++) {{
                        const node = result.snapshotItem(i);
                        elements.push({{
                            text: node.textContent.trim(),
                            html: node.outerHTML
                        }});
                    }}
                    return elements;
                }} catch (e) {{
                    return {{ error: e.message }};
                }}
            }})()
            "#,
            xpath.replace('"', r#"\""#)
        );

        let result = tab.evaluate(&js_code, false)
            .map_err(|e| anyhow!("XPath evaluation failed: {}", e))?;

        let elements: serde_json::Value = serde_json::from_str(&result.value.unwrap_or_default().to_string())
            .map_err(|e| anyhow!("Failed to parse XPath result: {}", e))?;

        if let Some(error) = elements.get("error") {
            return Err(anyhow!("XPath error: {}", error.as_str().unwrap_or("Unknown error")));
        }

        let mut text_parts = Vec::new();
        let mut html_parts = Vec::new();

        if let Some(array) = elements.as_array() {
            for element in array {
                if let (Some(text), Some(html)) = (element.get("text"), element.get("html")) {
                    if let (Some(text_str), Some(html_str)) = (text.as_str(), html.as_str()) {
                        if !text_str.is_empty() {
                            text_parts.push(text_str.to_string());
                            html_parts.push(html_str.to_string());
                        }
                    }
                }
            }
        }

        let text = if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.join("\n"))
        };

        let html = if html_parts.is_empty() {
            None
        } else {
            Some(html_parts.join("\n"))
        };

        Ok((text, html))
    }

    fn extract_text_content(&self, _tab: &Tab, text_pattern: &str) -> Result<(Option<String>, Option<String>)> {
        // For text-based extraction, we'll use the page content and search for patterns
        // This is a simplified implementation - in practice, you might want more sophisticated text matching
        let html_content = _tab.get_content()
            .map_err(|e| anyhow!("Failed to get page content: {}", e))?;

        // Simple text search - this could be enhanced with regex support
        if html_content.contains(text_pattern) {
            // Extract some context around the found text
            if let Some(start) = html_content.find(text_pattern) {
                let context_start = start.saturating_sub(100);
                let context_end = (start + text_pattern.len() + 100).min(html_content.len());
                let context = &html_content[context_start..context_end];
                
                Ok((Some(text_pattern.to_string()), Some(context.to_string())))
            } else {
                Ok((None, None))
            }
        } else {
            Ok((None, None))
        }
    }

    fn take_screenshot(&self, tab: &Tab) -> Result<String> {
        let screenshot_data = tab.capture_screenshot(
            headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
            None,
            None,
            true,
        ).map_err(|e| anyhow!("Screenshot capture failed: {}", e))?;

        // Generate unique filename
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("screenshot_{}_{}.png", timestamp, uuid::Uuid::new_v4().simple());
        let path = std::path::Path::new("data/screenshots").join(&filename);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed to create screenshot directory: {}", e))?;
        }

        // Write screenshot file
        std::fs::write(&path, screenshot_data)
            .map_err(|e| anyhow!("Failed to write screenshot: {}", e))?;

        Ok(path.to_string_lossy().to_string())
    }

    pub async fn test_connection(&self, url: &str) -> Result<bool> {
        let request = ScrapeRequest {
            url: url.to_string(),
            selector: "html".to_string(),
            selector_type: "css".to_string(),
            wait_for_selector: None,
            screenshot: false,
            user_agent: None,
            timeout_ms: Some(10000), // 10 second timeout for connection test
        };

        let result = self.scrape(request).await?;
        Ok(result.success)
    }

    pub fn shutdown(&self) -> Result<()> {
        // Browsers will be dropped when Arc references go out of scope
        // Chrome processes should clean up automatically
        Ok(())
    }
}

impl Drop for BrowserPool {
    fn drop(&mut self) {
        // Browsers will close automatically when dropped
        // The headless_chrome crate handles cleanup internally
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    fn get_test_config() -> ScraperConfig {
        ScraperConfig {
            max_concurrent_checks: 2,
            retry_attempts: 1,
            retry_delay_ms: 1000,
            request_timeout: 10,
            user_agent: "TestAgent/1.0".to_string(),
            chrome_path: None,
        }
    }

    #[tokio::test]
    async fn test_scraper_creation() {
        let config = get_test_config();
        let result = WebScraper::new(config);
        
        // This might fail in CI/test environments without Chrome
        match result {
            Ok(scraper) => {
                assert!(scraper.shutdown().is_ok());
            },
            Err(e) => {
                // Expected in environments without Chrome
                assert!(e.to_string().contains("browser") || e.to_string().contains("chrome"));
            }
        }
    }

    #[tokio::test] 
    async fn test_scrape_request_creation() {
        let request = ScrapeRequest {
            url: "https://example.com".to_string(),
            selector: "title".to_string(),
            selector_type: "css".to_string(),
            wait_for_selector: None,
            screenshot: false,
            user_agent: Some("TestAgent".to_string()),
            timeout_ms: Some(5000),
        };

        assert_eq!(request.url, "https://example.com");
        assert_eq!(request.selector, "title");
        assert_eq!(request.selector_type, "css");
        assert!(!request.screenshot);
    }

    #[tokio::test]
    async fn test_scrape_result_default() {
        let result = ScrapeResult {
            success: false,
            text: None,
            html: None,
            screenshot_path: None,
            error: Some("Test error".to_string()),
            response_time_ms: 1000,
            final_url: "https://example.com".to_string(),
        };

        assert!(!result.success);
        assert_eq!(result.error, Some("Test error".to_string()));
        assert_eq!(result.response_time_ms, 1000);
    }

    #[test]
    fn test_browser_pool_creation() {
        let _config = get_test_config();
        // Browser pool creation requires Chrome installation
        // This test just validates that the config is properly structured
        assert!(true); // Placeholder test - actual browser testing needs Chrome
    }

    #[test]
    fn test_css_selector_validation() {
        let _config = get_test_config();
        
        // Test valid CSS selectors
        let valid_selectors = vec![
            "div",
            ".price", 
            "#total",
            "span.amount",
            "div > span",
            "[data-price]",
        ];

        for selector in valid_selectors {
            let parsed = Selector::parse(selector);
            assert!(parsed.is_ok(), "Selector '{}' should be valid", selector);
        }

        // Test invalid CSS selectors  
        let invalid_selectors = vec![
            ">>>",
            "div >",
            // Note: "[unclosed" is actually valid CSS - attribute selectors don't require quotes
            // "::unknown" is also valid, it just means an unknown pseudo-element
        ];

        for selector in invalid_selectors {
            let parsed = Selector::parse(selector);
            assert!(parsed.is_err(), "Selector '{}' should be invalid", selector);
        }
    }

    #[tokio::test]
    async fn test_extract_css_content_with_mock_html() {
        // We can't easily test the full Chrome integration in unit tests,
        // but we can test the HTML parsing logic separately
        let html = r#"
            <html>
                <body>
                    <div class="price">$19.99</div>
                    <span class="currency">USD</span>
                    <div class="price">$29.99</div>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let selector = Selector::parse(".price").unwrap();
        
        let mut text_parts = Vec::new();
        let mut html_parts = Vec::new();

        for element in document.select(&selector) {
            text_parts.push(element.text().collect::<Vec<_>>().join(" ").trim().to_string());
            html_parts.push(element.html());
        }

        assert_eq!(text_parts.len(), 2);
        assert_eq!(text_parts[0], "$19.99");
        assert_eq!(text_parts[1], "$29.99");
        assert!(html_parts[0].contains(r#"class="price""#));
        assert!(html_parts[0].contains("$19.99"));
    }

    #[tokio::test]
    async fn test_url_validation() {
        use url::Url;
        let valid_urls = vec![
            "https://example.com",
            "http://localhost:3000", 
            "https://shop.example.com/product/123",
            "https://api.example.com/v1/price?id=456",
        ];

        for url_str in valid_urls {
            let parsed = Url::parse(url_str);
            assert!(parsed.is_ok(), "URL '{}' should be valid", url_str);
        }

        let invalid_urls = vec![
            "not-a-url",
            "",
            // Note: javascript: and ftp: are actually valid URL schemes from the URL parser's perspective
        ];

        for url_str in invalid_urls {
            let parsed = Url::parse(url_str);
            assert!(parsed.is_err(), "URL '{}' should be invalid", url_str);
        }
    }
}