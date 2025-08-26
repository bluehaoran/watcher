use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use crate::models::generate_id;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct NotificationConfig {
    pub id: String,
    pub product_id: String,
    pub notifier_type: String, // "email", "discord", etc.
    pub config_json: String,   // Plugin-specific configuration
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewNotificationConfig {
    pub product_id: String,
    pub notifier_type: String,
    pub config: serde_json::Value,
    pub is_enabled: Option<bool>,
}

impl NotificationConfig {
    pub fn new(new_config: NewNotificationConfig) -> Result<Self, String> {
        let config_json = serde_json::to_string(&new_config.config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
            
        Ok(Self {
            id: generate_id(),
            product_id: new_config.product_id,
            notifier_type: new_config.notifier_type,
            config_json,
            is_enabled: new_config.is_enabled.unwrap_or(true),
            created_at: Utc::now(),
        })
    }
    
    pub fn get_config(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.config_json)
    }
    
    pub fn update_config(&mut self, config: serde_json::Value) -> Result<(), String> {
        self.config_json = serde_json::to_string(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_notification_config_creation() {
        let config = json!({
            "webhook_url": "https://discord.com/api/webhooks/123/abc",
            "username": "Uatu Watcher"
        });
        
        let new_config = NewNotificationConfig {
            product_id: "product123".to_string(),
            notifier_type: "discord".to_string(),
            config: config.clone(),
            is_enabled: Some(true),
        };
        
        let notification_config = NotificationConfig::new(new_config).unwrap();
        
        assert_eq!(notification_config.product_id, "product123");
        assert_eq!(notification_config.notifier_type, "discord");
        assert!(notification_config.is_enabled);
        assert_eq!(notification_config.get_config().unwrap(), config);
    }

    #[test]
    fn test_update_config() {
        let initial_config = json!({"webhook_url": "https://example.com"});
        let new_config = NewNotificationConfig {
            product_id: "product123".to_string(),
            notifier_type: "discord".to_string(),
            config: initial_config,
            is_enabled: None,
        };
        
        let mut notification_config = NotificationConfig::new(new_config).unwrap();
        
        let updated_config = json!({"webhook_url": "https://updated.com", "username": "New Bot"});
        notification_config.update_config(updated_config.clone()).unwrap();
        
        assert_eq!(notification_config.get_config().unwrap(), updated_config);
    }
}