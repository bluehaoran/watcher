use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use crate::models::generate_id;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct FalsePositive {
    pub id: String,
    pub source_id: String,
    pub detected_text: String,
    pub detected_value_json: String,
    pub actual_text: Option<String>,
    pub html_context: String,
    pub screenshot_path: Option<String>,
    pub notes: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFalsePositive {
    pub source_id: String,
    pub detected_text: String,
    pub detected_value: serde_json::Value,
    pub actual_text: Option<String>,
    pub html_context: String,
    pub screenshot_path: Option<String>,
    pub notes: Option<String>,
}

impl FalsePositive {
    pub fn new(new_fp: NewFalsePositive) -> Result<Self, String> {
        let detected_value_json = serde_json::to_string(&new_fp.detected_value)
            .map_err(|e| format!("Failed to serialize detected value: {}", e))?;
            
        Ok(Self {
            id: generate_id(),
            source_id: new_fp.source_id,
            detected_text: new_fp.detected_text,
            detected_value_json,
            actual_text: new_fp.actual_text,
            html_context: new_fp.html_context,
            screenshot_path: new_fp.screenshot_path,
            notes: new_fp.notes,
            timestamp: Utc::now(),
        })
    }
    
    pub fn get_detected_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.detected_value_json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_false_positive_creation() {
        let new_fp = NewFalsePositive {
            source_id: "source123".to_string(),
            detected_text: "$99.99".to_string(),
            detected_value: json!({"amount": "99.99", "currency": "USD"}),
            actual_text: Some("$19.99".to_string()),
            html_context: "<div class='price'>$99.99</div>".to_string(),
            screenshot_path: Some("/screenshots/fp1.png".to_string()),
            notes: Some("Detected old price in cache".to_string()),
        };
        
        let fp = FalsePositive::new(new_fp).unwrap();
        
        assert_eq!(fp.source_id, "source123");
        assert_eq!(fp.detected_text, "$99.99");
        assert_eq!(fp.actual_text, Some("$19.99".to_string()));
    }
}