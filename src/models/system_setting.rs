use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct SystemSetting {
    pub key: String,
    pub value_json: String,
}

impl SystemSetting {
    pub fn new(key: String, value: serde_json::Value) -> Result<Self, String> {
        let value_json = serde_json::to_string(&value)
            .map_err(|e| format!("Failed to serialize value: {}", e))?;
            
        Ok(Self {
            key,
            value_json,
        })
    }
    
    pub fn get_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.value_json)
    }
    
    pub fn update_value(&mut self, value: serde_json::Value) -> Result<(), String> {
        self.value_json = serde_json::to_string(&value)
            .map_err(|e| format!("Failed to serialize value: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_system_setting_creation() {
        let value = json!({"max_concurrent_scrapes": 5, "default_timeout": 30});
        let setting = SystemSetting::new("scraper_config".to_string(), value.clone()).unwrap();
        
        assert_eq!(setting.key, "scraper_config");
        assert_eq!(setting.get_value().unwrap(), value);
    }

    #[test]
    fn test_update_value() {
        let initial_value = json!({"enabled": true});
        let mut setting = SystemSetting::new("feature_flag".to_string(), initial_value).unwrap();
        
        let new_value = json!({"enabled": false, "reason": "maintenance"});
        setting.update_value(new_value.clone()).unwrap();
        
        assert_eq!(setting.get_value().unwrap(), new_value);
    }
}