use anyhow::Result;
use scraper::{Html, Selector, ElementRef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::scraper::{WebScraper, ScrapeRequest};
use crate::models::SelectorType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementSelector {
    pub selector_type: SelectorType,
    pub selector: String,
    pub attributes: Vec<String>, // Additional attributes to extract
    pub validate_text: Option<String>, // Expected text pattern for validation
    pub wait_selector: Option<String>, // Element to wait for before extraction
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementMatch {
    pub selector: ElementSelector,
    pub text: String,
    pub html: String,
    pub attributes: HashMap<String, String>,
    pub position: usize, // Position in the list of matches
    pub confidence: f64, // Confidence score 0.0-1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementFinderRequest {
    pub url: String,
    pub target_text: Option<String>, // Text we're looking for
    pub target_type: String, // "price", "version", "number", "text"
    pub context_hints: Vec<String>, // Additional context like "price", "cost", "total"
    pub exclude_selectors: Vec<String>, // Selectors to avoid
    pub max_matches: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementFinderResult {
    pub success: bool,
    pub matches: Vec<ElementMatch>,
    pub suggestions: Vec<ElementSelector>,
    pub error: Option<String>,
    pub analysis_time_ms: u64,
}

pub struct ElementFinder {
    scraper: WebScraper,
}

impl ElementFinder {
    pub fn new(scraper: WebScraper) -> Self {
        Self { scraper }
    }

    pub async fn find_elements(&self, request: ElementFinderRequest) -> Result<ElementFinderResult> {
        let start_time = std::time::Instant::now();
        
        // First scrape the page to get HTML content
        let scrape_request = ScrapeRequest {
            url: request.url.clone(),
            selector: "html".to_string(),
            selector_type: "css".to_string(),
            wait_for_selector: None,
            screenshot: false,
            user_agent: None,
            timeout_ms: None,
        };

        let scrape_result = self.scraper.scrape(scrape_request).await?;

        if !scrape_result.success {
            return Ok(ElementFinderResult {
                success: false,
                matches: Vec::new(),
                suggestions: Vec::new(),
                error: scrape_result.error,
                analysis_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        let html_content = scrape_result.html.unwrap_or_default();
        let document = Html::parse_document(&html_content);

        // Find potential elements based on target type and context
        let matches = self.analyze_elements(&document, &request)?;

        // Generate suggestions based on analysis
        let suggestions = self.generate_suggestions(&matches, &request);

        Ok(ElementFinderResult {
            success: true,
            matches,
            suggestions,
            error: None,
            analysis_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    fn analyze_elements(&self, document: &Html, request: &ElementFinderRequest) -> Result<Vec<ElementMatch>> {
        let mut matches = Vec::new();

        // Define selectors based on target type
        let candidate_selectors = self.get_candidate_selectors(&request.target_type);

        for selector_str in candidate_selectors {
            if request.exclude_selectors.contains(&selector_str) {
                continue;
            }

            if let Ok(selector) = Selector::parse(&selector_str) {
                let elements: Vec<ElementRef> = document.select(&selector).collect();
                
                for (index, element) in elements.iter().enumerate() {
                    if matches.len() >= request.max_matches {
                        break;
                    }

                    let text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    let html = element.html();

                    // Skip empty elements
                    if text.is_empty() {
                        continue;
                    }

                    // Calculate confidence score
                    let confidence = self.calculate_confidence(&text, &html, request);

                    // Filter by minimum confidence
                    if confidence < 0.1 {
                        continue;
                    }

                    // Extract attributes
                    let mut attributes = HashMap::new();
                    for attr_name in &["class", "id", "data-price", "data-value", "title", "aria-label"] {
                        if let Some(value) = element.value().attr(attr_name) {
                            attributes.insert(attr_name.to_string(), value.to_string());
                        }
                    }

                    let element_match = ElementMatch {
                        selector: ElementSelector {
                            selector_type: SelectorType::Css,
                            selector: selector_str.clone(),
                            attributes: attributes.keys().cloned().collect(),
                            validate_text: request.target_text.clone(),
                            wait_selector: None,
                        },
                        text,
                        html,
                        attributes,
                        position: index,
                        confidence,
                    };

                    matches.push(element_match);
                }
            }
        }

        // Sort by confidence score (highest first)
        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        // Limit results
        matches.truncate(request.max_matches);

        Ok(matches)
    }

    fn get_candidate_selectors(&self, target_type: &str) -> Vec<String> {
        match target_type {
            "price" => vec![
                // Common price selectors
                ".price".to_string(),
                ".cost".to_string(),
                ".amount".to_string(),
                ".total".to_string(),
                ".value".to_string(),
                "[data-price]".to_string(),
                "[data-cost]".to_string(),
                "[data-amount]".to_string(),
                "span:contains('$')".to_string(),
                ".price-current".to_string(),
                ".price-now".to_string(),
                ".sale-price".to_string(),
                ".regular-price".to_string(),
                ".current-price".to_string(),
                // Generic selectors that might contain prices
                "span".to_string(),
                "div".to_string(),
                "p".to_string(),
                "strong".to_string(),
                "b".to_string(),
            ],
            "version" => vec![
                // Version-specific selectors
                ".version".to_string(),
                ".ver".to_string(),
                "[data-version]".to_string(),
                ".release".to_string(),
                ".build".to_string(),
                ".tag".to_string(),
                "code".to_string(),
                ".version-number".to_string(),
                ".version-info".to_string(),
                // Generic selectors
                "span".to_string(),
                "div".to_string(),
                "p".to_string(),
            ],
            "number" => vec![
                // Number/count selectors
                ".count".to_string(),
                ".number".to_string(),
                ".quantity".to_string(),
                ".qty".to_string(),
                ".stock".to_string(),
                "[data-count]".to_string(),
                "[data-number]".to_string(),
                "[data-quantity]".to_string(),
                // Generic selectors
                "span".to_string(),
                "div".to_string(),
                "p".to_string(),
                "strong".to_string(),
            ],
            _ => vec![
                // Generic text selectors
                "span".to_string(),
                "div".to_string(),
                "p".to_string(),
                "h1".to_string(),
                "h2".to_string(),
                "h3".to_string(),
                "strong".to_string(),
                "b".to_string(),
            ],
        }
    }

    fn calculate_confidence(&self, text: &str, html: &str, request: &ElementFinderRequest) -> f64 {
        let mut confidence = 0.0;

        // Base confidence for having non-empty text
        if !text.trim().is_empty() {
            confidence += 0.2;
        }

        // Check for target text match if provided
        if let Some(target) = &request.target_text {
            if text.contains(target) {
                confidence += 0.4;
            } else if text.to_lowercase().contains(&target.to_lowercase()) {
                confidence += 0.3;
            }
        }

        // Check for context hints in text and HTML
        for hint in &request.context_hints {
            let hint_lower = hint.to_lowercase();
            if text.to_lowercase().contains(&hint_lower) {
                confidence += 0.1;
            }
            if html.to_lowercase().contains(&hint_lower) {
                confidence += 0.05;
            }
        }

        // Type-specific confidence boosters
        match request.target_type.as_str() {
            "price" => {
                confidence += self.calculate_price_confidence(text, html);
            }
            "version" => {
                confidence += self.calculate_version_confidence(text, html);
            }
            "number" => {
                confidence += self.calculate_number_confidence(text, html);
            }
            _ => {}
        }

        // Penalize very long text (likely not the target)
        if text.len() > 200 {
            confidence *= 0.5;
        }

        // Cap confidence at 1.0
        confidence.min(1.0)
    }

    fn calculate_price_confidence(&self, text: &str, html: &str) -> f64 {
        let mut confidence = 0.0;

        // Currency symbols
        if text.contains('$') || text.contains('€') || text.contains('£') || text.contains('¥') {
            confidence += 0.3;
        }

        // Price patterns
        if regex::Regex::new(r"\$\d+\.?\d*").unwrap().is_match(text) {
            confidence += 0.4;
        }

        // Price-related class names in HTML
        let price_classes = ["price", "cost", "amount", "total", "value", "money"];
        for class in &price_classes {
            if html.to_lowercase().contains(class) {
                confidence += 0.1;
                break;
            }
        }

        confidence
    }

    fn calculate_version_confidence(&self, text: &str, _html: &str) -> f64 {
        let mut confidence = 0.0;

        // Version patterns (semantic versioning, etc.)
        let version_patterns = [
            r"v?\d+\.\d+\.\d+",      // v1.2.3 or 1.2.3
            r"v?\d+\.\d+",           // v1.2 or 1.2
            r"\d+\.\d+\.\d+\.\d+",   // 1.2.3.4
        ];

        for pattern in &version_patterns {
            if regex::Regex::new(pattern).unwrap().is_match(text) {
                confidence += 0.4;
                break;
            }
        }

        // Version keywords
        let version_keywords = ["version", "ver", "v", "release", "build"];
        for keyword in &version_keywords {
            if text.to_lowercase().contains(keyword) {
                confidence += 0.1;
            }
        }

        confidence
    }

    fn calculate_number_confidence(&self, text: &str, _html: &str) -> f64 {
        let mut confidence = 0.0;

        // Simple number pattern
        if regex::Regex::new(r"^\d+$").unwrap().is_match(text.trim()) {
            confidence += 0.3;
        }

        // Number with units
        if regex::Regex::new(r"\d+\s*(items?|pcs?|units?)").unwrap().is_match(&text.to_lowercase()) {
            confidence += 0.2;
        }

        confidence
    }

    fn generate_suggestions(&self, matches: &[ElementMatch], request: &ElementFinderRequest) -> Vec<ElementSelector> {
        let mut suggestions = Vec::new();

        // Generate more specific selectors for top matches
        for (index, element_match) in matches.iter().take(3).enumerate() {
            // Create more specific CSS selectors based on attributes
            if let Some(class) = element_match.attributes.get("class") {
                let class_selector = format!(".{}", class.split_whitespace().next().unwrap_or(""));
                if !suggestions.iter().any(|s: &ElementSelector| s.selector == class_selector) {
                    suggestions.push(ElementSelector {
                        selector_type: SelectorType::Css,
                        selector: class_selector,
                        attributes: vec!["text".to_string()],
                        validate_text: request.target_text.clone(),
                        wait_selector: None,
                    });
                }
            }

            if let Some(id) = element_match.attributes.get("id") {
                let id_selector = format!("#{}", id);
                suggestions.push(ElementSelector {
                    selector_type: SelectorType::Css,
                    selector: id_selector,
                    attributes: vec!["text".to_string()],
                    validate_text: request.target_text.clone(),
                    wait_selector: None,
                });
            }

            // Suggest nth-child selectors for positional matching
            if index == 0 && element_match.position > 0 {
                let nth_selector = format!("{}:nth-child({})", 
                    element_match.selector.selector, 
                    element_match.position + 1
                );
                suggestions.push(ElementSelector {
                    selector_type: SelectorType::Css,
                    selector: nth_selector,
                    attributes: vec!["text".to_string()],
                    validate_text: request.target_text.clone(),
                    wait_selector: None,
                });
            }
        }

        // Remove duplicates and limit suggestions
        suggestions.sort_by(|a, b| a.selector.cmp(&b.selector));
        suggestions.dedup_by(|a, b| a.selector == b.selector);
        suggestions.truncate(5);

        suggestions
    }

    pub async fn validate_selector(&self, url: &str, selector: &ElementSelector) -> Result<bool> {
        let scrape_request = ScrapeRequest {
            url: url.to_string(),
            selector: selector.selector.clone(),
            selector_type: match selector.selector_type {
                SelectorType::Css => "css".to_string(),
                SelectorType::Xpath => "xpath".to_string(),
                SelectorType::Text => "text".to_string(),
            },
            wait_for_selector: selector.wait_selector.clone(),
            screenshot: false,
            user_agent: None,
            timeout_ms: Some(10000),
        };

        let result = self.scraper.scrape(scrape_request).await?;
        
        if !result.success {
            return Ok(false);
        }

        // Check if we got the expected text
        if let (Some(expected), Some(actual)) = (&selector.validate_text, &result.text) {
            Ok(actual.contains(expected))
        } else {
            Ok(result.text.is_some() && !result.text.unwrap().trim().is_empty())
        }
    }

    pub fn optimize_selector(&self, selector: &str, document: &Html) -> Result<String> {
        // Try to create a more specific/reliable selector
        if let Ok(css_selector) = Selector::parse(selector) {
            let elements: Vec<ElementRef> = document.select(&css_selector).collect();
            
            if elements.len() == 1 {
                // Already unique, return as-is
                return Ok(selector.to_string());
            }

            if elements.len() > 1 {
                // Try to make it more specific by adding attributes
                if let Some(first_element) = elements.first() {
                    // Try ID first (most specific)
                    if let Some(id) = first_element.value().attr("id") {
                        return Ok(format!("#{}", id));
                    }

                    // Try class + tag combination
                    if let Some(class) = first_element.value().attr("class") {
                        let tag_name = first_element.value().name();
                        let first_class = class.split_whitespace().next().unwrap_or("");
                        if !first_class.is_empty() {
                            let specific_selector = format!("{}.{}", tag_name, first_class);
                            
                            // Test if this is unique - clone to avoid lifetime issues
                            match Selector::parse(&specific_selector.clone()) {
                                Ok(test_selector) => {
                                    let test_elements: Vec<ElementRef> = document.select(&test_selector).collect();
                                    if test_elements.len() == 1 {
                                        return Ok(specific_selector);
                                    }
                                },
                                Err(_) => {} // Invalid selector, continue
                            }
                        }
                    }

                    // Fallback to nth-child
                    let parent = first_element.parent().and_then(|p| ElementRef::wrap(p));
                    if let Some(parent_element) = parent {
                        let siblings: Vec<ElementRef> = parent_element.children()
                            .filter_map(|node| ElementRef::wrap(node))
                            .filter(|el| el.value().name() == first_element.value().name())
                            .collect();
                        
                        if let Some(position) = siblings.iter().position(|el| 
                            std::ptr::eq(el.value(), first_element.value())
                        ) {
                            return Ok(format!("{}:nth-child({})", first_element.value().name(), position + 1));
                        }
                    }
                }
            }
        }

        // Return original selector if no optimization possible
        Ok(selector.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ScraperConfig;

    fn get_test_scraper_config() -> ScraperConfig {
        ScraperConfig {
            max_concurrent_checks: 1,
            retry_attempts: 1,
            retry_delay_ms: 1000,
            request_timeout: 5,
            user_agent: "TestAgent/1.0".to_string(),
            chrome_path: None,
        }
    }

    fn create_test_finder() -> ElementFinder {
        // Create a mock web scraper for testing - in real environment this would require Chrome
        let config = get_test_scraper_config();
        match WebScraper::new(config) {
            Ok(scraper) => ElementFinder::new(scraper),
            Err(_) => {
                // In test environments without Chrome, we still want to test the pure functions
                // We'll skip tests that require the actual scraper
                panic!("Test environment requires Chrome for full integration tests");
            }
        }
    }

    #[test]
    fn test_element_selector_creation() {
        let selector = ElementSelector {
            selector_type: SelectorType::Css,
            selector: ".price".to_string(),
            attributes: vec!["text".to_string(), "data-price".to_string()],
            validate_text: Some("$19.99".to_string()),
            wait_selector: Some(".price-container".to_string()),
        };

        assert_eq!(selector.selector, ".price");
        assert_eq!(selector.attributes.len(), 2);
        assert_eq!(selector.validate_text, Some("$19.99".to_string()));
    }

    #[test]
    fn test_element_finder_request() {
        let request = ElementFinderRequest {
            url: "https://example.com/product".to_string(),
            target_text: Some("$19.99".to_string()),
            target_type: "price".to_string(),
            context_hints: vec!["price".to_string(), "cost".to_string()],
            exclude_selectors: vec![".advertisement".to_string()],
            max_matches: 5,
        };

        assert_eq!(request.target_type, "price");
        assert_eq!(request.context_hints.len(), 2);
        assert_eq!(request.max_matches, 5);
    }

    #[test]
    fn test_candidate_selectors_generation() {
        // Test the pure function without requiring Chrome
        let config = get_test_scraper_config();
        if let Ok(scraper) = WebScraper::new(config) {
            let finder = ElementFinder::new(scraper);
            
            let price_selectors = finder.get_candidate_selectors("price");
            assert!(price_selectors.contains(&".price".to_string()));
            assert!(price_selectors.contains(&"[data-price]".to_string()));
            assert!(price_selectors.len() > 5);

            let version_selectors = finder.get_candidate_selectors("version");
            assert!(version_selectors.contains(&".version".to_string()));
            assert!(version_selectors.contains(&"code".to_string()));

            let number_selectors = finder.get_candidate_selectors("number");
            assert!(number_selectors.contains(&".count".to_string()));
            assert!(number_selectors.contains(&".quantity".to_string()));
        } else {
            // Skip test in environments without Chrome
            println!("Skipping test - Chrome not available in test environment");
        }
    }

    #[test]
    fn test_confidence_calculation() {
        let config = get_test_scraper_config();
        if let Ok(scraper) = WebScraper::new(config) {
            let finder = ElementFinder::new(scraper);
            
            let request = ElementFinderRequest {
                url: "https://example.com".to_string(),
                target_text: Some("$19.99".to_string()),
                target_type: "price".to_string(),
                context_hints: vec!["price".to_string()],
                exclude_selectors: vec![],
                max_matches: 10,
            };

            // Test exact text match
            let confidence1 = finder.calculate_confidence("$19.99", r#"<span class="price">$19.99</span>"#, &request);
            assert!(confidence1 > 0.5);

            // Test partial match
            let confidence2 = finder.calculate_confidence("Price: $19.99", r#"<div>Price: $19.99</div>"#, &request);
            assert!(confidence2 > 0.3);

            // Test no match
            let confidence3 = finder.calculate_confidence("Hello world", r#"<p>Hello world</p>"#, &request);
            assert!(confidence3 < 0.5);
        } else {
            println!("Skipping test - Chrome not available in test environment");
        }
    }

    #[test]
    fn test_price_confidence_calculation() {
        let config = get_test_scraper_config();
        if let Ok(scraper) = WebScraper::new(config) {
            let finder = ElementFinder::new(scraper);

        // Test currency symbols
        let confidence1 = finder.calculate_price_confidence("$19.99", r#"<span class="price">$19.99</span>"#);
        assert!(confidence1 > 0.5);

        // Test price pattern without symbol
        let confidence2 = finder.calculate_price_confidence("19.99", r#"<span class="amount">19.99</span>"#);
        assert!(confidence2 > 0.0);

        // Test non-price text
        let confidence3 = finder.calculate_price_confidence("Hello world", r#"<p>Hello world</p>"#);
        assert_eq!(confidence3, 0.0);
        } else {
            println!("Skipping test - Chrome not available in test environment");
        }
    }

    #[test]
    fn test_version_confidence_calculation() {
        let config = get_test_scraper_config();
        if let Ok(scraper) = WebScraper::new(config) {
            let finder = ElementFinder::new(scraper);

        // Test semantic version
        let confidence1 = finder.calculate_version_confidence("v1.2.3", "");
        assert!(confidence1 > 0.3);

        // Test simple version
        let confidence2 = finder.calculate_version_confidence("2.1", "");
        assert!(confidence2 > 0.3);

        // Test version with keyword
        let confidence3 = finder.calculate_version_confidence("Version 1.0", "");
        assert!(confidence3 > 0.4);

        // Test non-version text
        let confidence4 = finder.calculate_version_confidence("Hello world", "");
        assert_eq!(confidence4, 0.0);
        } else {
            println!("Skipping test - Chrome not available in test environment");
        }
    }

    #[test]
    fn test_number_confidence_calculation() {
        let config = get_test_scraper_config();
        if let Ok(scraper) = WebScraper::new(config) {
            let finder = ElementFinder::new(scraper);

        // Test simple number
        let confidence1 = finder.calculate_number_confidence("123", "");
        assert!(confidence1 > 0.2);

        // Test number with units
        let confidence2 = finder.calculate_number_confidence("50 items", "");
        assert!(confidence2 > 0.4);

        // Test non-number text
        let confidence3 = finder.calculate_number_confidence("Hello world", "");
        assert_eq!(confidence3, 0.0);
        } else {
            println!("Skipping test - Chrome not available in test environment");
        }
    }

    #[test]
    fn test_optimize_selector() {
        let config = get_test_scraper_config();
        if let Ok(scraper) = WebScraper::new(config) {
            let finder = ElementFinder::new(scraper);
        
        let html = r#"
            <html>
                <body>
                    <div id="unique-price" class="price">$19.99</div>
                    <span class="price">$29.99</span>
                    <span class="price">$39.99</span>
                </body>
            </html>
        "#;

        let document = Html::parse_document(html);

        // Test optimization for ID selector
        let optimized1 = finder.optimize_selector("div", &document).unwrap();
        assert_eq!(optimized1, "#unique-price");

        // Test optimization for class selector
        let optimized2 = finder.optimize_selector(".price", &document).unwrap();
        // Should return more specific selector since .price matches multiple elements
        assert_ne!(optimized2, ".price");
        } else {
            println!("Skipping test - Chrome not available in test environment");
        }
    }

    #[tokio::test]
    async fn test_element_match_structure() {
        let element_match = ElementMatch {
            selector: ElementSelector {
                selector_type: SelectorType::Css,
                selector: ".price".to_string(),
                attributes: vec!["text".to_string()],
                validate_text: Some("$19.99".to_string()),
                wait_selector: None,
            },
            text: "$19.99".to_string(),
            html: r#"<span class="price">$19.99</span>"#.to_string(),
            attributes: {
                let mut attrs = HashMap::new();
                attrs.insert("class".to_string(), "price".to_string());
                attrs
            },
            position: 0,
            confidence: 0.8,
        };

        assert_eq!(element_match.text, "$19.99");
        assert_eq!(element_match.confidence, 0.8);
        assert_eq!(element_match.attributes.get("class"), Some(&"price".to_string()));
    }
}