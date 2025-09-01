use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use crate::models::{TrackerType, NotifyOn, ThresholdType, generate_id};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tracker_type: TrackerType,
    
    // Notification rules
    pub notify_on: NotifyOn,
    pub threshold_type: Option<ThresholdType>,
    pub threshold_value: Option<f64>,
    
    // Schedule
    pub check_interval: String, // Cron expression
    pub last_checked: Option<DateTime<Utc>>,
    pub next_check: Option<DateTime<Utc>>,
    
    // Status
    pub is_active: bool,
    pub is_paused: bool,
    
    // Best deal tracking
    pub best_source_id: Option<String>,
    pub best_value_json: Option<String>,
    
    // Metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProduct {
    pub name: String,
    pub description: Option<String>,
    pub tracker_type: TrackerType,
    pub notify_on: Option<NotifyOn>,
    pub threshold_type: Option<ThresholdType>,
    pub threshold_value: Option<f64>,
    pub check_interval: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProduct {
    pub name: Option<String>,
    pub description: Option<String>,
    pub notify_on: Option<NotifyOn>,
    pub threshold_type: Option<ThresholdType>,
    pub threshold_value: Option<f64>,
    pub check_interval: Option<String>,
    pub is_active: Option<bool>,
    pub is_paused: Option<bool>,
}

impl Product {
    pub fn new(new_product: NewProduct) -> Self {
        let now = Utc::now();
        Self {
            id: generate_id(),
            name: new_product.name,
            description: new_product.description,
            tracker_type: new_product.tracker_type,
            notify_on: new_product.notify_on.unwrap_or(NotifyOn::AnyChange),
            threshold_type: new_product.threshold_type,
            threshold_value: new_product.threshold_value,
            check_interval: new_product.check_interval.unwrap_or_else(|| "0 0 * * *".to_string()),
            last_checked: None,
            next_check: None,
            is_active: true,
            is_paused: false,
            best_source_id: None,
            best_value_json: None,
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn update(&mut self, update: UpdateProduct) {
        if let Some(name) = update.name {
            self.name = name;
        }
        if let Some(description) = update.description {
            self.description = Some(description);
        }
        if let Some(notify_on) = update.notify_on {
            self.notify_on = notify_on;
        }
        if let Some(threshold_type) = update.threshold_type {
            self.threshold_type = Some(threshold_type);
        }
        if let Some(threshold_value) = update.threshold_value {
            self.threshold_value = Some(threshold_value);
        }
        if let Some(check_interval) = update.check_interval {
            self.check_interval = check_interval;
        }
        if let Some(is_active) = update.is_active {
            self.is_active = is_active;
        }
        if let Some(is_paused) = update.is_paused {
            self.is_paused = is_paused;
        }
        
        self.updated_at = Utc::now();
    }
    
    pub fn is_ready_for_check(&self) -> bool {
        if !self.is_active || self.is_paused {
            return false;
        }
        
        match self.next_check {
            Some(next_check) => Utc::now() >= next_check,
            None => true, // Never checked before
        }
    }
    
    pub fn should_notify(&self, old_value: Option<&serde_json::Value>, new_value: &serde_json::Value) -> bool {
        if old_value.is_none() {
            return false; // Don't notify on first check
        }
        
        let old_val = old_value.unwrap();
        if old_val == new_value {
            return false; // No change
        }
        
        match self.notify_on {
            NotifyOn::AnyChange => true,
            NotifyOn::Decrease => self.is_decrease(old_val, new_value),
            NotifyOn::Increase => self.is_increase(old_val, new_value),
        }
    }
    
    fn is_decrease(&self, old_value: &serde_json::Value, new_value: &serde_json::Value) -> bool {
        match self.tracker_type {
            TrackerType::Price | TrackerType::Number => {
                let old_num = self.extract_number(old_value).unwrap_or(0.0);
                let new_num = self.extract_number(new_value).unwrap_or(0.0);
                new_num < old_num
            },
            TrackerType::Version => false, // Version decreases are rare and usually not desired
        }
    }
    
    fn is_increase(&self, old_value: &serde_json::Value, new_value: &serde_json::Value) -> bool {
        match self.tracker_type {
            TrackerType::Price | TrackerType::Number => {
                let old_num = self.extract_number(old_value).unwrap_or(0.0);
                let new_num = self.extract_number(new_value).unwrap_or(0.0);
                new_num > old_num
            },
            TrackerType::Version => {
                // For versions, any change is considered an increase (new version)
                old_value != new_value
            }
        }
    }
    
    fn extract_number(&self, value: &serde_json::Value) -> Option<f64> {
        match value {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => {
                // Try to parse decimal string for prices
                if let Some(amount_obj) = value.as_object() {
                    if let Some(amount_str) = amount_obj.get("amount").and_then(|v| v.as_str()) {
                        return amount_str.parse().ok();
                    }
                }
                s.parse().ok()
            },
            serde_json::Value::Object(obj) => {
                // For structured values like prices, extract the numeric component
                if let Some(amount) = obj.get("amount").and_then(|v| v.as_str()) {
                    amount.parse().ok()
                } else { obj.get("number").and_then(|v| v.as_f64()) }
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_product() -> NewProduct {
        NewProduct {
            name: "Test Product".to_string(),
            description: Some("A test product for tracking".to_string()),
            tracker_type: TrackerType::Price,
            notify_on: Some(NotifyOn::Decrease),
            threshold_type: Some(ThresholdType::Relative),
            threshold_value: Some(10.0),
            check_interval: Some("0 */6 * * *".to_string()), // Every 6 hours
        }
    }

    #[test]
    fn test_product_creation() {
        let new_product = create_test_product();
        let product = Product::new(new_product);
        
        assert_eq!(product.name, "Test Product");
        assert_eq!(product.description, Some("A test product for tracking".to_string()));
        assert_eq!(product.tracker_type, TrackerType::Price);
        assert_eq!(product.notify_on, NotifyOn::Decrease);
        assert_eq!(product.threshold_type, Some(ThresholdType::Relative));
        assert_eq!(product.threshold_value, Some(10.0));
        assert_eq!(product.check_interval, "0 */6 * * *");
        assert!(product.is_active);
        assert!(!product.is_paused);
        assert!(product.last_checked.is_none());
        assert!(product.next_check.is_none());
        assert_eq!(product.id.len(), 32);
    }

    #[test]
    fn test_product_creation_with_defaults() {
        let new_product = NewProduct {
            name: "Simple Product".to_string(),
            description: None,
            tracker_type: TrackerType::Version,
            notify_on: None,
            threshold_type: None,
            threshold_value: None,
            check_interval: None,
        };
        
        let product = Product::new(new_product);
        
        assert_eq!(product.notify_on, NotifyOn::AnyChange);
        assert_eq!(product.check_interval, "0 0 * * *"); // Daily default
        assert!(product.threshold_type.is_none());
        assert!(product.threshold_value.is_none());
    }

    #[test]
    fn test_product_update() {
        let new_product = create_test_product();
        let mut product = Product::new(new_product);
        let original_updated_at = product.updated_at;
        
        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        let update = UpdateProduct {
            name: Some("Updated Product".to_string()),
            description: Some("Updated description".to_string()),
            notify_on: Some(NotifyOn::AnyChange),
            threshold_type: Some(ThresholdType::Absolute),
            threshold_value: Some(5.0),
            check_interval: Some("0 */2 * * *".to_string()),
            is_active: Some(false),
            is_paused: Some(true),
        };
        
        product.update(update);
        
        assert_eq!(product.name, "Updated Product");
        assert_eq!(product.description, Some("Updated description".to_string()));
        assert_eq!(product.notify_on, NotifyOn::AnyChange);
        assert_eq!(product.threshold_type, Some(ThresholdType::Absolute));
        assert_eq!(product.threshold_value, Some(5.0));
        assert_eq!(product.check_interval, "0 */2 * * *");
        assert!(!product.is_active);
        assert!(product.is_paused);
        assert!(product.updated_at > original_updated_at);
    }

    #[test]
    fn test_product_partial_update() {
        let new_product = create_test_product();
        let mut product = Product::new(new_product);
        let original_name = product.name.clone();
        
        let update = UpdateProduct {
            name: None,
            description: Some("Only description updated".to_string()),
            notify_on: None,
            threshold_type: None,
            threshold_value: None,
            check_interval: None,
            is_active: None,
            is_paused: None,
        };
        
        product.update(update);
        
        assert_eq!(product.name, original_name); // Unchanged
        assert_eq!(product.description, Some("Only description updated".to_string()));
    }

    #[test]
    fn test_is_ready_for_check() {
        let new_product = create_test_product();
        let mut product = Product::new(new_product);
        
        // Active product with no previous check should be ready
        assert!(product.is_ready_for_check());
        
        // Inactive product should not be ready
        product.is_active = false;
        assert!(!product.is_ready_for_check());
        
        // Paused product should not be ready
        product.is_active = true;
        product.is_paused = true;
        assert!(!product.is_ready_for_check());
        
        // Product with future next_check should not be ready
        product.is_paused = false;
        product.next_check = Some(Utc::now() + chrono::Duration::hours(1));
        assert!(!product.is_ready_for_check());
        
        // Product with past next_check should be ready
        product.next_check = Some(Utc::now() - chrono::Duration::hours(1));
        assert!(product.is_ready_for_check());
    }

    #[test]
    fn test_should_notify_price_decrease() {
        let new_product = NewProduct {
            name: "Price Product".to_string(),
            description: None,
            tracker_type: TrackerType::Price,
            notify_on: Some(NotifyOn::Decrease),
            threshold_type: None,
            threshold_value: None,
            check_interval: None,
        };
        let product = Product::new(new_product);
        
        let old_value = json!({"amount": "100.00", "currency": "USD"});
        let new_value_decrease = json!({"amount": "90.00", "currency": "USD"});
        let new_value_increase = json!({"amount": "110.00", "currency": "USD"});
        let new_value_same = json!({"amount": "100.00", "currency": "USD"});
        
        // Should notify on decrease
        assert!(product.should_notify(Some(&old_value), &new_value_decrease));
        
        // Should not notify on increase
        assert!(!product.should_notify(Some(&old_value), &new_value_increase));
        
        // Should not notify when same
        assert!(!product.should_notify(Some(&old_value), &new_value_same));
        
        // Should not notify on first check (no old value)
        assert!(!product.should_notify(None, &new_value_decrease));
    }

    #[test]
    fn test_should_notify_any_change() {
        let new_product = NewProduct {
            name: "Any Change Product".to_string(),
            description: None,
            tracker_type: TrackerType::Version,
            notify_on: Some(NotifyOn::AnyChange),
            threshold_type: None,
            threshold_value: None,
            check_interval: None,
        };
        let product = Product::new(new_product);
        
        let old_value = json!({"version": "1.0.0"});
        let new_value = json!({"version": "1.1.0"});
        let same_value = json!({"version": "1.0.0"});
        
        // Should notify on any change
        assert!(product.should_notify(Some(&old_value), &new_value));
        
        // Should not notify when same
        assert!(!product.should_notify(Some(&old_value), &same_value));
    }

    #[test]
    fn test_extract_number_from_price() {
        let new_product = create_test_product();
        let product = Product::new(new_product);
        
        let price_value = json!({"amount": "123.45", "currency": "USD"});
        assert_eq!(product.extract_number(&price_value), Some(123.45));
        
        let number_value = json!({"number": 67.89});
        assert_eq!(product.extract_number(&number_value), Some(67.89));
        
        let simple_number = json!(42.0);
        assert_eq!(product.extract_number(&simple_number), Some(42.0));
        
        let string_number = json!("99.99");
        assert_eq!(product.extract_number(&string_number), Some(99.99));
        
        let invalid_value = json!("not a number");
        assert_eq!(product.extract_number(&invalid_value), None);
    }

    #[test]
    fn test_version_increase_detection() {
        let new_product = NewProduct {
            name: "Version Product".to_string(),
            description: None,
            tracker_type: TrackerType::Version,
            notify_on: Some(NotifyOn::Increase),
            threshold_type: None,
            threshold_value: None,
            check_interval: None,
        };
        let product = Product::new(new_product);
        
        let old_version = json!({"version": "1.0.0"});
        let new_version = json!({"version": "1.1.0"});
        let same_version = json!({"version": "1.0.0"});
        
        // Should notify on version change (treated as increase)
        assert!(product.should_notify(Some(&old_version), &new_version));
        
        // Should not notify when same
        assert!(!product.should_notify(Some(&old_version), &same_version));
    }

    #[test]
    fn test_serialization() {
        let new_product = create_test_product();
        let product = Product::new(new_product);
        
        let serialized = serde_json::to_string(&product).unwrap();
        let deserialized: Product = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(product, deserialized);
    }
}