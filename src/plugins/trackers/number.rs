use crate::plugins::traits::{
    TrackerPlugin, ParseResult, ComparisonResult, ElementMatch, ConfigSchema, ChangeType,
};
use crate::plugins::traits::tracker::{ConfigField, ConfigFieldType};
use async_trait::async_trait;
use regex::Regex;
use serde_json::json;
use std::collections::HashMap;
use std::cmp::Ordering;

pub struct NumberTracker {
    number_regex: Regex,
}

impl NumberTracker {
    pub fn new() -> Self {
        NumberTracker {
            // Match integers and decimals, with optional thousand separators
            number_regex: Regex::new(r"(\d{1,3}(?:,\d{3})*(?:\.\d+)?|\d+(?:\.\d+)?)").unwrap(),
        }
    }
    
    fn extract_number(&self, text: &str) -> Option<f64> {
        if let Some(captures) = self.number_regex.captures(text) {
            let number_str = captures.get(1)?.as_str().replace(',', "");
            number_str.parse::<f64>().ok()
        } else {
            None
        }
    }
}

#[async_trait]
impl TrackerPlugin for NumberTracker {
    fn name(&self) -> &str {
        "Number Tracker"
    }
    
    fn plugin_type(&self) -> &str {
        "number"
    }
    
    fn description(&self) -> &str {
        "Tracks numeric changes on web pages"
    }
    
    fn parse(&self, text: &str) -> ParseResult {
        if let Some(number) = self.extract_number(text) {
            let value = json!({
                "number": number,
                "formatted": if number.fract() == 0.0 {
                    format!("{}", number as i64)
                } else {
                    format!("{}", number)
                }
            });
            
            ParseResult {
                success: true,
                value,
                normalized: if number.fract() == 0.0 {
                    format!("{}", number as i64)
                } else {
                    format!("{}", number)
                },
                confidence: 0.85,
                metadata: HashMap::new(),
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
        if let Some(formatted) = value.get("formatted") {
            return formatted.as_str().unwrap_or("N/A").to_string();
        }
        if let Some(number) = value.get("number") {
            return number.to_string();
        }
        "N/A".to_string()
    }
    
    fn compare(&self, old_value: &serde_json::Value, new_value: &serde_json::Value) -> ComparisonResult {
        let old_number = old_value.get("number").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let new_number = new_value.get("number").and_then(|v| v.as_f64()).unwrap_or(0.0);
        
        let change_type = if new_number > old_number {
            ChangeType::Increased
        } else if new_number < old_number {
            ChangeType::Decreased
        } else {
            ChangeType::Unchanged
        };
        
        let difference = new_number - old_number;
        let percent_change = if old_number != 0.0 {
            Some(difference / old_number * 100.0)
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
        
        // Add variations with different formatting
        if let Ok(number) = input.parse::<f64>() {
            // Add thousand separator version (simplified)
            if number >= 1000.0 {
                let int_part = number as i64;
                variations.push(format!("{}", int_part));
            }
            
            // Add decimal versions
            variations.push(format!("{:.0}", number));
            variations.push(format!("{:.2}", number));
        }
        
        variations
    }
    
    fn rank_matches(&self, _input: &str, matches: &[ElementMatch]) -> Vec<ElementMatch> {
        let mut ranked = matches.to_vec();
        
        // Rank by confidence, then by whether text contains just a number
        ranked.sort_by(|a, b| {
            let a_score = a.confidence + if self.extract_number(&a.text).is_some() { 0.1 } else { 0.0 };
            let b_score = b.confidence + if self.extract_number(&b.text).is_some() { 0.1 } else { 0.0 };
            b_score.partial_cmp(&a_score).unwrap_or(Ordering::Equal)
        });
        
        ranked
    }
    
    fn get_config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            fields: vec![
                ConfigField {
                    name: "decimal_places".to_string(),
                    field_type: ConfigFieldType::Number,
                    label: "Decimal Places".to_string(),
                    required: false,
                    default: Some(json!(2)),
                    options: None,
                },
            ],
        }
    }
    
    fn validate_config(&self, config: &serde_json::Value) -> bool {
        if let Some(decimal_places) = config.get("decimal_places") {
            return decimal_places.is_number() && 
                   decimal_places.as_i64().unwrap_or(-1) >= 0 &&
                   decimal_places.as_i64().unwrap_or(-1) <= 10;
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
    fn test_number_parsing_integer() {
        let tracker = NumberTracker::new();
        let result = tracker.parse("42");
        
        assert!(result.success);
        assert_eq!(result.value["number"], 42.0);
        assert_eq!(result.value["formatted"], "42");
        assert_eq!(result.normalized, "42");
    }

    #[test]
    fn test_number_parsing_decimal() {
        let tracker = NumberTracker::new();
        let result = tracker.parse("42.5");
        
        assert!(result.success);
        assert_eq!(result.value["number"], 42.5);
        assert_eq!(result.value["formatted"], "42.5");
    }

    #[test]
    fn test_number_parsing_with_commas() {
        let tracker = NumberTracker::new();
        let result = tracker.parse("1,234,567");
        
        assert!(result.success);
        assert_eq!(result.value["number"], 1234567.0);
        assert_eq!(result.value["formatted"], "1234567");
    }

    #[test]
    fn test_number_parsing_with_text() {
        let tracker = NumberTracker::new();
        let result = tracker.parse("The count is 25 items");
        
        assert!(result.success);
        assert_eq!(result.value["number"], 25.0);
    }

    #[test]
    fn test_number_parsing_failure() {
        let tracker = NumberTracker::new();
        let result = tracker.parse("no numbers here");
        
        assert!(!result.success);
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn test_number_formatting() {
        let tracker = NumberTracker::new();
        let value = json!({
            "number": 123.45,
            "formatted": "123.45"
        });
        
        let formatted = tracker.format(&value);
        assert_eq!(formatted, "123.45");
    }

    #[test]
    fn test_number_comparison() {
        let tracker = NumberTracker::new();
        let old_number = json!({
            "number": 100.0
        });
        let new_number = json!({
            "number": 150.0
        });
        
        let result = tracker.compare(&old_number, &new_number);
        assert!(result.changed);
        assert!(matches!(result.change_type, ChangeType::Increased));
        assert_eq!(result.percent_change, Some(50.0));
    }

    #[test]
    fn test_search_variations() {
        let tracker = NumberTracker::new();
        let variations = tracker.get_search_variations("1234");
        
        assert!(variations.contains(&"1234".to_string()));
        assert!(variations.contains(&"1234.00".to_string()));
    }

    #[test]
    fn test_config_validation() {
        let tracker = NumberTracker::new();
        let valid_config = json!({"decimal_places": 2});
        let invalid_config = json!({"decimal_places": -1});
        
        assert!(tracker.validate_config(&valid_config));
        assert!(!tracker.validate_config(&invalid_config));
    }
}