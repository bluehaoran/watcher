use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use crate::models::{SelectorType, generate_id};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct Source {
    pub id: String,
    pub product_id: String,
    
    // Source information
    pub url: String,
    pub store_name: Option<String>,
    pub title: String,
    
    // Selector information
    pub selector: String,
    pub selector_type: SelectorType,
    
    // Values (JSON serialized for complex types)
    pub original_value_json: Option<String>,
    pub current_value_json: Option<String>,
    pub original_text: Option<String>,
    pub current_text: Option<String>,
    
    // Source-specific status
    pub is_active: bool,
    pub last_checked: Option<DateTime<Utc>>,
    pub error_count: i32,
    pub last_error: Option<String>,
    
    // Metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSource {
    pub product_id: String,
    pub url: String,
    pub store_name: Option<String>,
    pub title: String,
    pub selector: String,
    pub selector_type: Option<SelectorType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSource {
    pub store_name: Option<String>,
    pub title: Option<String>,
    pub selector: Option<String>,
    pub selector_type: Option<SelectorType>,
    pub is_active: Option<bool>,
}

impl Source {
    pub fn new(new_source: NewSource) -> Self {
        let now = Utc::now();
        Self {
            id: generate_id(),
            product_id: new_source.product_id,
            url: new_source.url,
            store_name: new_source.store_name,
            title: new_source.title,
            selector: new_source.selector,
            selector_type: new_source.selector_type.unwrap_or(SelectorType::Css),
            original_value_json: None,
            current_value_json: None,
            original_text: None,
            current_text: None,
            is_active: true,
            last_checked: None,
            error_count: 0,
            last_error: None,
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn update(&mut self, update: UpdateSource) {
        if let Some(store_name) = update.store_name {
            self.store_name = Some(store_name);
        }
        if let Some(title) = update.title {
            self.title = title;
        }
        if let Some(selector) = update.selector {
            self.selector = selector;
        }
        if let Some(selector_type) = update.selector_type {
            self.selector_type = selector_type;
        }
        if let Some(is_active) = update.is_active {
            self.is_active = is_active;
        }
        
        self.updated_at = Utc::now();
    }
    
    pub fn update_value(&mut self, text: String, value_json: String) {
        self.current_text = Some(text.clone());
        self.current_value_json = Some(value_json.clone());
        
        // Set original values if this is the first time
        if self.original_text.is_none() {
            self.original_text = Some(text);
            self.original_value_json = Some(value_json);
        }
        
        self.last_checked = Some(Utc::now());
        self.clear_error();
    }
    
    pub fn record_error(&mut self, error: String) {
        self.error_count += 1;
        self.last_error = Some(error);
        self.last_checked = Some(Utc::now());
        self.updated_at = Utc::now();
    }
    
    pub fn clear_error(&mut self) {
        self.error_count = 0;
        self.last_error = None;
    }
    
    pub fn has_changed(&self) -> bool {
        match (&self.original_value_json, &self.current_value_json) {
            (Some(original), Some(current)) => original != current,
            _ => false,
        }
    }
    
    pub fn should_retry(&self) -> bool {
        self.is_active && self.error_count < 5
    }
    
    pub fn get_current_value(&self) -> Option<serde_json::Value> {
        self.current_value_json
            .as_ref()
            .and_then(|json_str| serde_json::from_str(json_str).ok())
    }
    
    pub fn get_original_value(&self) -> Option<serde_json::Value> {
        self.original_value_json
            .as_ref()
            .and_then(|json_str| serde_json::from_str(json_str).ok())
    }
    
    pub fn get_store_display_name(&self) -> String {
        self.store_name
            .clone()
            .unwrap_or_else(|| {
                // Extract domain from URL as fallback
                if let Ok(url) = url::Url::parse(&self.url) {
                    url.host_str().unwrap_or("Unknown Store").to_string()
                } else {
                    "Unknown Store".to_string()
                }
            })
    }
    
    pub fn is_healthy(&self) -> bool {
        self.is_active && self.error_count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_source() -> NewSource {
        NewSource {
            product_id: "test_product_id".to_string(),
            url: "https://example.com/product/123".to_string(),
            store_name: Some("Example Store".to_string()),
            title: "Test Product Page".to_string(),
            selector: ".price".to_string(),
            selector_type: Some(SelectorType::Css),
        }
    }

    #[test]
    fn test_source_creation() {
        let new_source = create_test_source();
        let source = Source::new(new_source);
        
        assert_eq!(source.product_id, "test_product_id");
        assert_eq!(source.url, "https://example.com/product/123");
        assert_eq!(source.store_name, Some("Example Store".to_string()));
        assert_eq!(source.title, "Test Product Page");
        assert_eq!(source.selector, ".price");
        assert_eq!(source.selector_type, SelectorType::Css);
        assert!(source.is_active);
        assert_eq!(source.error_count, 0);
        assert!(source.last_error.is_none());
        assert_eq!(source.id.len(), 32);
    }

    #[test]
    fn test_source_creation_with_defaults() {
        let new_source = NewSource {
            product_id: "test_product_id".to_string(),
            url: "https://example.com/product/123".to_string(),
            store_name: None,
            title: "Test Product".to_string(),
            selector: "//span[@class='price']".to_string(),
            selector_type: None, // Should default to CSS
        };
        
        let source = Source::new(new_source);
        
        assert!(source.store_name.is_none());
        assert_eq!(source.selector_type, SelectorType::Css);
    }

    #[test]
    fn test_source_update() {
        let new_source = create_test_source();
        let mut source = Source::new(new_source);
        let original_updated_at = source.updated_at;
        
        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        let update = UpdateSource {
            store_name: Some("Updated Store".to_string()),
            title: Some("Updated Title".to_string()),
            selector: Some("#new-price".to_string()),
            selector_type: Some(SelectorType::Xpath),
            is_active: Some(false),
        };
        
        source.update(update);
        
        assert_eq!(source.store_name, Some("Updated Store".to_string()));
        assert_eq!(source.title, "Updated Title");
        assert_eq!(source.selector, "#new-price");
        assert_eq!(source.selector_type, SelectorType::Xpath);
        assert!(!source.is_active);
        assert!(source.updated_at > original_updated_at);
    }

    #[test]
    fn test_update_value_first_time() {
        let new_source = create_test_source();
        let mut source = Source::new(new_source);
        
        let text = "$19.99".to_string();
        let value_json = json!({"amount": "19.99", "currency": "USD"}).to_string();
        
        source.update_value(text.clone(), value_json.clone());
        
        assert_eq!(source.current_text, Some(text.clone()));
        assert_eq!(source.current_value_json, Some(value_json.clone()));
        assert_eq!(source.original_text, Some(text));
        assert_eq!(source.original_value_json, Some(value_json));
        assert!(source.last_checked.is_some());
        assert_eq!(source.error_count, 0);
        assert!(source.last_error.is_none());
    }

    #[test]
    fn test_update_value_subsequent() {
        let new_source = create_test_source();
        let mut source = Source::new(new_source);
        
        // First value
        let original_text = "$19.99".to_string();
        let original_value_json = json!({"amount": "19.99", "currency": "USD"}).to_string();
        source.update_value(original_text.clone(), original_value_json.clone());
        
        // Second value
        let new_text = "$15.99".to_string();
        let new_value_json = json!({"amount": "15.99", "currency": "USD"}).to_string();
        source.update_value(new_text.clone(), new_value_json.clone());
        
        // Original values should remain unchanged
        assert_eq!(source.original_text, Some(original_text));
        assert_eq!(source.original_value_json, Some(original_value_json));
        
        // Current values should be updated
        assert_eq!(source.current_text, Some(new_text));
        assert_eq!(source.current_value_json, Some(new_value_json));
    }

    #[test]
    fn test_record_error() {
        let new_source = create_test_source();
        let mut source = Source::new(new_source);
        
        assert_eq!(source.error_count, 0);
        assert!(source.last_error.is_none());
        
        source.record_error("Network timeout".to_string());
        
        assert_eq!(source.error_count, 1);
        assert_eq!(source.last_error, Some("Network timeout".to_string()));
        assert!(source.last_checked.is_some());
        
        // Record another error
        source.record_error("Selector not found".to_string());
        
        assert_eq!(source.error_count, 2);
        assert_eq!(source.last_error, Some("Selector not found".to_string()));
    }

    #[test]
    fn test_clear_error() {
        let new_source = create_test_source();
        let mut source = Source::new(new_source);
        
        source.record_error("Some error".to_string());
        assert_eq!(source.error_count, 1);
        assert!(source.last_error.is_some());
        
        source.clear_error();
        
        assert_eq!(source.error_count, 0);
        assert!(source.last_error.is_none());
    }

    #[test]
    fn test_has_changed() {
        let new_source = create_test_source();
        let mut source = Source::new(new_source);
        
        // No change when no values set
        assert!(!source.has_changed());
        
        // No change when only current value set
        source.current_value_json = Some(json!({"amount": "19.99"}).to_string());
        assert!(!source.has_changed());
        
        // No change when values are the same
        let value_json = json!({"amount": "19.99"}).to_string();
        source.original_value_json = Some(value_json.clone());
        source.current_value_json = Some(value_json);
        assert!(!source.has_changed());
        
        // Change detected when values differ
        source.current_value_json = Some(json!({"amount": "15.99"}).to_string());
        assert!(source.has_changed());
    }

    #[test]
    fn test_should_retry() {
        let new_source = create_test_source();
        let mut source = Source::new(new_source);
        
        // Should retry when active and error count low
        assert!(source.should_retry());
        
        // Should not retry when inactive
        source.is_active = false;
        assert!(!source.should_retry());
        
        // Should not retry when error count too high
        source.is_active = true;
        source.error_count = 5;
        assert!(!source.should_retry());
        
        // Should retry again when error count reduced
        source.error_count = 4;
        assert!(source.should_retry());
    }

    #[test]
    fn test_get_current_value() {
        let new_source = create_test_source();
        let mut source = Source::new(new_source);
        
        // No value when not set
        assert!(source.get_current_value().is_none());
        
        // Valid JSON value
        let value = json!({"amount": "19.99", "currency": "USD"});
        source.current_value_json = Some(value.to_string());
        assert_eq!(source.get_current_value(), Some(value));
        
        // Invalid JSON should return None
        source.current_value_json = Some("{invalid json".to_string());
        assert!(source.get_current_value().is_none());
    }

    #[test]
    fn test_get_original_value() {
        let new_source = create_test_source();
        let mut source = Source::new(new_source);
        
        // No value when not set
        assert!(source.get_original_value().is_none());
        
        // Valid JSON value
        let value = json!({"amount": "19.99", "currency": "USD"});
        source.original_value_json = Some(value.to_string());
        assert_eq!(source.get_original_value(), Some(value));
    }

    #[test]
    fn test_get_store_display_name() {
        let mut new_source = create_test_source();
        let mut source = Source::new(new_source.clone());
        
        // Should return store name when provided
        assert_eq!(source.get_store_display_name(), "Example Store");
        
        // Should extract from URL when no store name
        new_source.store_name = None;
        source = Source::new(new_source);
        assert_eq!(source.get_store_display_name(), "example.com");
        
        // Should handle invalid URLs
        source.url = "not a valid url".to_string();
        assert_eq!(source.get_store_display_name(), "Unknown Store");
    }

    #[test]
    fn test_is_healthy() {
        let new_source = create_test_source();
        let mut source = Source::new(new_source);
        
        // Healthy when active and no errors
        assert!(source.is_healthy());
        
        // Not healthy when inactive
        source.is_active = false;
        assert!(!source.is_healthy());
        
        // Not healthy with errors
        source.is_active = true;
        source.error_count = 1;
        assert!(!source.is_healthy());
        
        // Healthy again when errors cleared
        source.error_count = 0;
        assert!(source.is_healthy());
    }

    #[test]
    fn test_serialization() {
        let new_source = create_test_source();
        let source = Source::new(new_source);
        
        let serialized = serde_json::to_string(&source).unwrap();
        let deserialized: Source = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(source, deserialized);
    }
}