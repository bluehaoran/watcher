use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use crate::models::generate_id;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct PriceHistory {
    pub id: String,
    pub source_id: String,
    pub value_json: String,
    pub text: String,
    pub timestamp: DateTime<Utc>,
}

impl PriceHistory {
    pub fn new(source_id: String, value: serde_json::Value, text: String) -> Result<Self, String> {
        let value_json = serde_json::to_string(&value)
            .map_err(|e| format!("Failed to serialize value: {}", e))?;
            
        Ok(Self {
            id: generate_id(),
            source_id,
            value_json,
            text,
            timestamp: Utc::now(),
        })
    }
    
    pub fn get_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.value_json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_price_history_creation() {
        let value = json!({"amount": "19.99", "currency": "USD"});
        let history = PriceHistory::new("source123".to_string(), value.clone(), "$19.99".to_string()).unwrap();
        
        assert_eq!(history.source_id, "source123");
        assert_eq!(history.text, "$19.99");
        assert_eq!(history.get_value().unwrap(), value);
    }
}