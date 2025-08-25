use crate::plugins::traits::{
    TrackerPlugin, ParseResult, ComparisonResult, ElementMatch, ConfigSchema, ChangeType,
};
use crate::plugins::traits::tracker::{ConfigField, ConfigFieldType, ConfigOption};
use async_trait::async_trait;
use regex::Regex;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct WebsiteContext {
    pub url: String,
    pub lang: Option<String>,         // HTML lang attribute
    pub locale: Option<String>,       // Detected locale
    pub country_code: Option<String>, // Country code from various sources
}

pub struct PriceTracker {
    price_regex: Regex,
    currency_symbols: HashMap<String, String>,
    default_currency: String,
    locale_currency_map: HashMap<String, String>,
}

impl PriceTracker {
    pub fn new() -> Self {
        Self::with_default_currency("AUD")
    }
    
    pub fn with_default_currency(default_currency: &str) -> Self {
        let mut currency_symbols = HashMap::new();
        
        // Set $ symbol based on default currency, but allow explicit overrides
        if default_currency == "USD" {
            currency_symbols.insert("$".to_string(), "USD".to_string());
        } else {
            currency_symbols.insert("$".to_string(), default_currency.to_string());
        }
        
        currency_symbols.insert("US$".to_string(), "USD".to_string());
        currency_symbols.insert("USD$".to_string(), "USD".to_string());
        currency_symbols.insert("£".to_string(), "GBP".to_string());
        currency_symbols.insert("€".to_string(), "EUR".to_string());
        currency_symbols.insert("¥".to_string(), "JPY".to_string());
        currency_symbols.insert("₹".to_string(), "INR".to_string());
        
        // Build locale to currency mapping
        let mut locale_currency_map = HashMap::new();
        locale_currency_map.insert("en-AU".to_string(), "AUD".to_string());
        locale_currency_map.insert("en-US".to_string(), "USD".to_string());
        locale_currency_map.insert("en-GB".to_string(), "GBP".to_string());
        locale_currency_map.insert("en-CA".to_string(), "CAD".to_string());
        locale_currency_map.insert("fr-FR".to_string(), "EUR".to_string());
        locale_currency_map.insert("de-DE".to_string(), "EUR".to_string());
        locale_currency_map.insert("es-ES".to_string(), "EUR".to_string());
        locale_currency_map.insert("it-IT".to_string(), "EUR".to_string());
        locale_currency_map.insert("ja-JP".to_string(), "JPY".to_string());
        locale_currency_map.insert("ko-KR".to_string(), "KRW".to_string());
        locale_currency_map.insert("zh-CN".to_string(), "CNY".to_string());
        locale_currency_map.insert("hi-IN".to_string(), "INR".to_string());
        
        // Add country code mappings
        locale_currency_map.insert("AU".to_string(), "AUD".to_string());
        locale_currency_map.insert("US".to_string(), "USD".to_string());
        locale_currency_map.insert("GB".to_string(), "GBP".to_string());
        locale_currency_map.insert("UK".to_string(), "GBP".to_string());
        locale_currency_map.insert("CA".to_string(), "CAD".to_string());
        locale_currency_map.insert("FR".to_string(), "EUR".to_string());
        locale_currency_map.insert("DE".to_string(), "EUR".to_string());
        locale_currency_map.insert("ES".to_string(), "EUR".to_string());
        locale_currency_map.insert("IT".to_string(), "EUR".to_string());
        locale_currency_map.insert("JP".to_string(), "JPY".to_string());
        locale_currency_map.insert("KR".to_string(), "KRW".to_string());
        locale_currency_map.insert("CN".to_string(), "CNY".to_string());
        locale_currency_map.insert("IN".to_string(), "INR".to_string());
        
        PriceTracker {
            price_regex: Regex::new(r"[\$£€¥₹]?\s*(\d{1,3}(?:,\d{3})*(?:\.\d{2})?|\d+(?:\.\d{2})?)").unwrap(),
            currency_symbols,
            default_currency: default_currency.to_string(),
            locale_currency_map,
        }
    }
    
    fn extract_price(&self, text: &str) -> Option<(Decimal, String)> {
        self.extract_price_with_context(text, &self.default_currency, None)
    }
    
    #[allow(dead_code)]
    fn extract_price_with_default(&self, text: &str, default_currency: &str) -> Option<(Decimal, String)> {
        self.extract_price_with_context(text, default_currency, None)
    }
    
    /// Extract price with website context for currency detection
    pub fn extract_price_with_context(&self, text: &str, default_currency: &str, website_context: Option<&WebsiteContext>) -> Option<(Decimal, String)> {
        if let Some(captures) = self.price_regex.captures(text) {
            let price_str = captures.get(1)?.as_str().replace(',', "");
            if let Ok(price) = Decimal::from_str(&price_str) {
                // Try to extract currency in order of priority:
                // 1. Explicit currency symbol in text
                // 2. Website locale/language detection
                // 3. Default currency
                let currency = self.extract_currency(text)
                    .or_else(|| self.infer_currency_from_website(website_context))
                    .unwrap_or_else(|| default_currency.to_string());
                return Some((price, currency));
            }
        }
        None
    }
    
    /// Infer currency from website context (URL, locale, lang attributes)
    fn infer_currency_from_website(&self, context: Option<&WebsiteContext>) -> Option<String> {
        let context = context?;
        
        // Check explicit currency in URL path
        if let Some(currency) = self.extract_currency_from_url(&context.url) {
            return Some(currency);
        }
        
        // Check HTML lang attribute
        if let Some(ref lang) = context.lang {
            if let Some(currency) = self.locale_currency_map.get(lang) {
                return Some(currency.clone());
            }
        }
        
        // Check domain TLD
        if let Some(currency) = self.infer_currency_from_domain(&context.url) {
            return Some(currency);
        }
        
        None
    }
    
    /// Extract currency from URL patterns (e.g., /en-au/, /us/, amazon.com.au)
    fn extract_currency_from_url(&self, url: &str) -> Option<String> {
        let url_lower = url.to_lowercase();
        
        // Check for locale patterns in path
        let locale_patterns = [
            (r"/en-au/|/au/|/australia/", "AUD"),
            (r"/en-us/|/us/|/usa/", "USD"),
            (r"/en-gb/|/gb/|/uk/", "GBP"),
            (r"/en-ca/|/ca/|/canada/", "CAD"),
            (r"/de/|/germany/", "EUR"),
            (r"/fr/|/france/", "EUR"),
            (r"/es/|/spain/", "EUR"),
            (r"/it/|/italy/", "EUR"),
            (r"/jp/|/japan/", "JPY"),
            (r"/kr/|/korea/", "KRW"),
            (r"/cn/|/china/", "CNY"),
            (r"/in/|/india/", "INR"),
        ];
        
        for (pattern, currency) in &locale_patterns {
            let regex = Regex::new(pattern).ok()?;
            if regex.is_match(&url_lower) {
                return Some(currency.to_string());
            }
        }
        
        None
    }
    
    /// Infer currency from domain TLD
    fn infer_currency_from_domain(&self, url: &str) -> Option<String> {
        let tld_patterns = [
            (r"\.com\.au|\.au$", "AUD"),
            (r"\.com|\.us$", "USD"),  // Default .com to USD
            (r"\.co\.uk|\.uk$", "GBP"),
            (r"\.ca$", "CAD"),
            (r"\.de$", "EUR"),
            (r"\.fr$", "EUR"),
            (r"\.es$", "EUR"),
            (r"\.it$", "EUR"),
            (r"\.jp$", "JPY"),
            (r"\.kr$", "KRW"),
            (r"\.cn$", "CNY"),
            (r"\.in$", "INR"),
        ];
        
        for (pattern, currency) in &tld_patterns {
            let regex = Regex::new(pattern).ok()?;
            if regex.is_match(url) {
                return Some(currency.to_string());
            }
        }
        
        None
    }
    
    fn extract_currency(&self, text: &str) -> Option<String> {
        // Check longer currency symbols first (US$, USD$ before $)
        let mut symbols: Vec<_> = self.currency_symbols.iter().collect();
        symbols.sort_by(|a, b| b.0.len().cmp(&a.0.len())); // Sort by length descending
        
        for (symbol, code) in symbols {
            if text.contains(symbol) {
                return Some(code.clone());
            }
        }
        None
    }
}

#[async_trait]
impl TrackerPlugin for PriceTracker {
    fn name(&self) -> &str {
        "Price Tracker"
    }
    
    fn plugin_type(&self) -> &str {
        "price"
    }
    
    fn description(&self) -> &str {
        "Tracks price changes on web pages with multi-currency support"
    }
    
    fn parse(&self, text: &str) -> ParseResult {
        if let Some((price, currency)) = self.extract_price(text) {
            let value = json!({
                "amount": price.to_string(),
                "currency": currency
            });
            
            ParseResult {
                success: true,
                value,
                normalized: format!("{} {}", price, currency),
                confidence: 0.9,
                metadata: [("currency".to_string(), currency)].into(),
            }
        } else {
            ParseResult {
                success: false,
                value: json!(null),
                normalized: "".to_string(),
                confidence: 0.0,
                metadata: HashMap::new(),
            }
        }
    }
    
    fn format(&self, value: &serde_json::Value) -> String {
        if let Some(amount) = value.get("amount") {
            if let Some(currency) = value.get("currency") {
                let amount_str = amount.as_str().unwrap_or("0");
                return format!("{} {}", amount_str, currency.as_str().unwrap_or("USD"));
            }
        }
        "N/A".to_string()
    }
    
    fn compare(&self, old_value: &serde_json::Value, new_value: &serde_json::Value) -> ComparisonResult {
        let old_amount = old_value.get("amount").and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok()).unwrap_or_default();
        let new_amount = new_value.get("amount").and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok()).unwrap_or_default();
            
        let change_type = if new_amount > old_amount {
            ChangeType::Increased
        } else if new_amount < old_amount {
            ChangeType::Decreased
        } else {
            ChangeType::Unchanged
        };
        
        let difference = new_amount - old_amount;
        let percent_change = if !old_amount.is_zero() {
            Some((difference / old_amount * Decimal::from(100)).to_f64().unwrap_or(0.0))
        } else {
            None
        };
        
        ComparisonResult {
            changed: change_type != ChangeType::Unchanged,
            change_type,
            difference: json!(difference),
            percent_change,
        }
    }
    
    fn get_search_variations(&self, input: &str) -> Vec<String> {
        let mut variations = vec![input.to_string()];
        
        // Add variations with different currency symbols
        for symbol in ["$", "£", "€", "¥", "₹"] {
            variations.push(format!("{}{}", symbol, input));
            variations.push(format!("{} {}", symbol, input));
        }
        
        variations
    }
    
    fn rank_matches(&self, _input: &str, matches: &[ElementMatch]) -> Vec<ElementMatch> {
        let mut ranked = matches.to_vec();
        ranked.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }
    
    fn get_config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            fields: vec![
                ConfigField {
                    name: "default_currency".to_string(),
                    field_type: ConfigFieldType::Select,
                    label: "Default Currency".to_string(),
                    required: false,
                    default: Some(json!("AUD")),
                    options: Some(vec![
                        ConfigOption { value: "AUD".to_string(), label: "Australian Dollar (AUD)".to_string() },
                        ConfigOption { value: "USD".to_string(), label: "US Dollar (USD)".to_string() },
                        ConfigOption { value: "EUR".to_string(), label: "Euro (EUR)".to_string() },
                        ConfigOption { value: "GBP".to_string(), label: "British Pound (GBP)".to_string() },
                        ConfigOption { value: "JPY".to_string(), label: "Japanese Yen (JPY)".to_string() },
                        ConfigOption { value: "INR".to_string(), label: "Indian Rupee (INR)".to_string() },
                    ]),
                },
                ConfigField {
                    name: "auto_detect_currency".to_string(),
                    field_type: ConfigFieldType::Checkbox,
                    label: "Auto-detect Currency from Website".to_string(),
                    required: false,
                    default: Some(json!(true)),
                    options: None,
                },
            ],
        }
    }
    
    fn validate_config(&self, config: &serde_json::Value) -> bool {
        if let Some(currency) = config.get("default_currency") {
            return currency.is_string();
        }
        true
    }
    
    async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
    
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_parsing_aud_default() {
        let tracker = PriceTracker::new();
        let result = tracker.parse("$19.99");
        
        assert!(result.success);
        assert_eq!(result.value["amount"], "19.99");
        assert_eq!(result.value["currency"], "AUD");  // Now defaults to AUD
        assert_eq!(result.normalized, "19.99 AUD");
    }
    
    #[test]
    fn test_price_parsing_with_commas() {
        let tracker = PriceTracker::new();
        let result = tracker.parse("$1,299.99");
        
        assert!(result.success);
        assert_eq!(result.value["amount"], "1299.99");
        assert_eq!(result.value["currency"], "AUD");  // Now defaults to AUD
    }
    
    #[test]
    fn test_price_parsing_euro() {
        let tracker = PriceTracker::new();
        let result = tracker.parse("€50.00");
        
        assert!(result.success);
        assert_eq!(result.value["currency"], "EUR");
    }
    
    #[test]
    fn test_price_parsing_failure() {
        let tracker = PriceTracker::new();
        let result = tracker.parse("not a price");
        
        assert!(!result.success);
        assert_eq!(result.confidence, 0.0);
    }
    
    #[test]
    fn test_price_formatting() {
        let tracker = PriceTracker::new();
        let value = json!({
            "amount": "29.99",
            "currency": "AUD"
        });
        
        let formatted = tracker.format(&value);
        assert_eq!(formatted, "29.99 AUD");
    }
    
    #[test]
    fn test_price_comparison() {
        let tracker = PriceTracker::new();
        let old_price = json!({
            "amount": "20.00",
            "currency": "AUD"
        });
        let new_price = json!({
            "amount": "25.00", 
            "currency": "AUD"
        });
        
        let result = tracker.compare(&old_price, &new_price);
        assert!(result.changed);
        assert!(matches!(result.change_type, ChangeType::Increased));
        assert_eq!(result.percent_change, Some(25.0));
    }
    
    #[test]
    fn test_search_variations() {
        let tracker = PriceTracker::new();
        let variations = tracker.get_search_variations("29.99");
        
        assert!(variations.contains(&"29.99".to_string()));
        assert!(variations.contains(&"$29.99".to_string()));
        assert!(variations.contains(&"€29.99".to_string()));
    }
    
    #[test]
    fn test_config_validation() {
        let tracker = PriceTracker::new();
        let valid_config = json!({"default_currency": "EUR"});
        let invalid_config = json!({"default_currency": 123});
        
        assert!(tracker.validate_config(&valid_config));
        assert!(!tracker.validate_config(&invalid_config));
    }
    
    #[test]
    fn test_custom_default_currency() {
        let tracker = PriceTracker::with_default_currency("USD");
        let result = tracker.parse("$19.99");
        
        assert!(result.success);
        assert_eq!(result.value["currency"], "USD");
    }
    
    #[test]
    fn test_usd_explicit_symbol() {
        let tracker = PriceTracker::new();
        let result = tracker.parse("US$19.99");
        
        assert!(result.success);
        assert_eq!(result.value["currency"], "USD");
    }
    
    #[test]
    fn test_default_config_is_aud() {
        let tracker = PriceTracker::new();
        let schema = tracker.get_config_schema();
        let default_currency_field = &schema.fields[0];
        
        assert_eq!(default_currency_field.name, "default_currency");
        assert_eq!(default_currency_field.default, Some(json!("AUD")));
        assert!(matches!(default_currency_field.field_type, ConfigFieldType::Select));
        
        // Check auto-detect field exists
        let auto_detect_field = &schema.fields[1];
        assert_eq!(auto_detect_field.name, "auto_detect_currency");
        assert_eq!(auto_detect_field.default, Some(json!(true)));
    }
    
    #[test]
    fn test_currency_detection_from_url() {
        let tracker = PriceTracker::new();
        
        // Test Australian domains
        let context_au = WebsiteContext {
            url: "https://example.com.au/products/item".to_string(),
            lang: None,
            locale: None,
            country_code: None,
        };
        let result = tracker.extract_price_with_context("25.99", "USD", Some(&context_au));
        assert_eq!(result.unwrap().1, "AUD");
        
        // Test US domains
        let context_us = WebsiteContext {
            url: "https://example.com/en-us/products/item".to_string(),
            lang: None,
            locale: None,
            country_code: None,
        };
        let result = tracker.extract_price_with_context("25.99", "AUD", Some(&context_us));
        assert_eq!(result.unwrap().1, "USD");
        
        // Test UK domains
        let context_uk = WebsiteContext {
            url: "https://example.co.uk/products/item".to_string(),
            lang: None,
            locale: None,
            country_code: None,
        };
        let result = tracker.extract_price_with_context("25.99", "USD", Some(&context_uk));
        assert_eq!(result.unwrap().1, "GBP");
    }
    
    #[test]
    fn test_currency_detection_from_lang() {
        let tracker = PriceTracker::new();
        
        let context = WebsiteContext {
            url: "https://example.com/products/item".to_string(),
            lang: Some("en-AU".to_string()),
            locale: None,
            country_code: None,
        };
        
        let result = tracker.extract_price_with_context("25.99", "USD", Some(&context));
        assert_eq!(result.unwrap().1, "AUD");
    }
    
    #[test]
    fn test_currency_priority_explicit_symbol_wins() {
        let tracker = PriceTracker::new();
        
        // Even with AU context, explicit US$ should win
        let context = WebsiteContext {
            url: "https://example.com.au/products/item".to_string(),
            lang: Some("en-AU".to_string()),
            locale: None,
            country_code: None,
        };
        
        let result = tracker.extract_price_with_context("US$25.99", "EUR", Some(&context));
        assert_eq!(result.unwrap().1, "USD");
    }
    
    #[test]
    fn test_currency_fallback_to_default() {
        let tracker = PriceTracker::new();
        
        // No currency detection possible, should fall back to default
        let context = WebsiteContext {
            url: "https://unknown-domain.xyz/products/item".to_string(),
            lang: None,
            locale: None,
            country_code: None,
        };
        
        let result = tracker.extract_price_with_context("25.99", "EUR", Some(&context));
        assert_eq!(result.unwrap().1, "EUR");
    }
}