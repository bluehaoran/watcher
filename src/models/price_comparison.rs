use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use crate::models::generate_id;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct PriceComparison {
    pub id: String,
    pub product_id: String,
    
    // Comparison data (JSON serialized)
    pub sources_json: String, // Array of {sourceId, value, storeName}
    pub best_source_id: String,
    pub best_value_json: String,
    pub worst_value_json: Option<String>,
    pub avg_value_json: Option<String>,
    
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceComparison {
    pub source_id: String,
    pub store_name: String,
    pub value: serde_json::Value,
    pub formatted_value: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPriceComparison {
    pub product_id: String,
    pub sources: Vec<SourceComparison>,
}

impl PriceComparison {
    pub fn new(new_comparison: NewPriceComparison) -> Result<Self, String> {
        if new_comparison.sources.is_empty() {
            return Err("Cannot create price comparison with no sources".to_string());
        }
        
        // Find best (lowest) price
        let best_source = new_comparison.sources
            .iter()
            .min_by(|a, b| Self::compare_values(&a.value, &b.value))
            .ok_or("Failed to find best price")?;
            
        // Find worst (highest) price if multiple sources
        let worst_source = if new_comparison.sources.len() > 1 {
            new_comparison.sources
                .iter()
                .max_by(|a, b| Self::compare_values(&a.value, &b.value))
        } else {
            None
        };
        
        // Calculate average if multiple sources
        let avg_value = if new_comparison.sources.len() > 1 {
            Self::calculate_average(&new_comparison.sources)
        } else {
            None
        };
        
        Ok(Self {
            id: generate_id(),
            product_id: new_comparison.product_id,
            sources_json: serde_json::to_string(&new_comparison.sources)
                .map_err(|e| format!("Failed to serialize sources: {}", e))?,
            best_source_id: best_source.source_id.clone(),
            best_value_json: serde_json::to_string(&best_source.value)
                .map_err(|e| format!("Failed to serialize best value: {}", e))?,
            worst_value_json: worst_source
                .map(|s| serde_json::to_string(&s.value))
                .transpose()
                .map_err(|e| format!("Failed to serialize worst value: {}", e))?,
            avg_value_json: avg_value
                .map(|v| serde_json::to_string(&v))
                .transpose()
                .map_err(|e| format!("Failed to serialize average value: {}", e))?,
            timestamp: Utc::now(),
        })
    }
    
    pub fn get_sources(&self) -> Result<Vec<SourceComparison>, serde_json::Error> {
        serde_json::from_str(&self.sources_json)
    }
    
    pub fn get_best_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.best_value_json)
    }
    
    pub fn get_worst_value(&self) -> Result<Option<serde_json::Value>, serde_json::Error> {
        match &self.worst_value_json {
            Some(json_str) => serde_json::from_str(json_str).map(Some),
            None => Ok(None),
        }
    }
    
    pub fn get_avg_value(&self) -> Result<Option<serde_json::Value>, serde_json::Error> {
        match &self.avg_value_json {
            Some(json_str) => serde_json::from_str(json_str).map(Some),
            None => Ok(None),
        }
    }
    
    pub fn get_savings(&self) -> Option<f64> {
        if let (Ok(Some(worst)), Ok(best)) = (self.get_worst_value(), self.get_best_value()) {
            let worst_amount = Self::extract_amount(&worst)?;
            let best_amount = Self::extract_amount(&best)?;
            Some(worst_amount - best_amount)
        } else {
            None
        }
    }
    
    pub fn get_savings_percentage(&self) -> Option<f64> {
        if let Some(savings) = self.get_savings() {
            if let Ok(Some(worst)) = self.get_worst_value() {
                let worst_amount = Self::extract_amount(&worst)?;
                if worst_amount > 0.0 {
                    return Some((savings / worst_amount) * 100.0);
                }
            }
        }
        None
    }
    
    fn compare_values(a: &serde_json::Value, b: &serde_json::Value) -> std::cmp::Ordering {
        let amount_a = Self::extract_amount(a).unwrap_or(f64::MAX);
        let amount_b = Self::extract_amount(b).unwrap_or(f64::MAX);
        amount_a.partial_cmp(&amount_b).unwrap_or(std::cmp::Ordering::Equal)
    }
    
    fn extract_amount(value: &serde_json::Value) -> Option<f64> {
        match value {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::Object(obj) => {
                // For price objects like {"amount": "19.99", "currency": "USD"}
                if let Some(amount_str) = obj.get("amount").and_then(|v| v.as_str()) {
                    amount_str.parse().ok()
                } else if let Some(amount_num) = obj.get("amount").and_then(|v| v.as_f64()) {
                    Some(amount_num)
                } else { obj.get("number").and_then(|v| v.as_f64()) }
            },
            serde_json::Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }
    
    fn calculate_average(sources: &[SourceComparison]) -> Option<serde_json::Value> {
        let amounts: Vec<f64> = sources
            .iter()
            .filter_map(|s| Self::extract_amount(&s.value))
            .collect();
            
        if amounts.is_empty() {
            return None;
        }
        
        let sum: f64 = amounts.iter().sum();
        let avg = sum / amounts.len() as f64;
        
        // Try to maintain the same structure as the source values
        if let Some(first_source) = sources.first() {
            match &first_source.value {
                serde_json::Value::Object(obj) if obj.contains_key("amount") => {
                    // For price objects, maintain currency info
                    let currency = obj.get("currency").and_then(|v| v.as_str()).unwrap_or("USD");
                    Some(serde_json::json!({
                        "amount": format!("{:.2}", avg),
                        "currency": currency
                    }))
                },
                serde_json::Value::Object(obj) if obj.contains_key("number") => {
                    // For number objects
                    Some(serde_json::json!({
                        "number": avg
                    }))
                },
                _ => Some(serde_json::Value::Number(
                    serde_json::Number::from_f64(avg).unwrap_or_else(|| serde_json::Number::from(0))
                )),
            }
        } else {
            Some(serde_json::Value::Number(
                serde_json::Number::from_f64(avg).unwrap_or_else(|| serde_json::Number::from(0))
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_sources() -> Vec<SourceComparison> {
        vec![
            SourceComparison {
                source_id: "source1".to_string(),
                store_name: "Store A".to_string(),
                value: json!({"amount": "19.99", "currency": "USD"}),
                formatted_value: "$19.99 USD".to_string(),
                url: "https://storea.com/product".to_string(),
            },
            SourceComparison {
                source_id: "source2".to_string(),
                store_name: "Store B".to_string(),
                value: json!({"amount": "24.99", "currency": "USD"}),
                formatted_value: "$24.99 USD".to_string(),
                url: "https://storeb.com/product".to_string(),
            },
            SourceComparison {
                source_id: "source3".to_string(),
                store_name: "Store C".to_string(),
                value: json!({"amount": "17.99", "currency": "USD"}),
                formatted_value: "$17.99 USD".to_string(),
                url: "https://storec.com/product".to_string(),
            },
        ]
    }

    #[test]
    fn test_price_comparison_creation() {
        let sources = create_test_sources();
        let new_comparison = NewPriceComparison {
            product_id: "test_product".to_string(),
            sources,
        };
        
        let comparison = PriceComparison::new(new_comparison).unwrap();
        
        assert_eq!(comparison.product_id, "test_product");
        assert_eq!(comparison.best_source_id, "source3"); // Store C has lowest price ($17.99)
        assert!(comparison.worst_value_json.is_some());
        assert!(comparison.avg_value_json.is_some());
        assert_eq!(comparison.id.len(), 32);
    }

    #[test]
    fn test_price_comparison_empty_sources() {
        let new_comparison = NewPriceComparison {
            product_id: "test_product".to_string(),
            sources: vec![],
        };
        
        let result = PriceComparison::new(new_comparison);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no sources"));
    }

    #[test]
    fn test_price_comparison_single_source() {
        let sources = vec![create_test_sources().into_iter().next().unwrap()];
        let new_comparison = NewPriceComparison {
            product_id: "test_product".to_string(),
            sources,
        };
        
        let comparison = PriceComparison::new(new_comparison).unwrap();
        
        assert_eq!(comparison.best_source_id, "source1");
        assert!(comparison.worst_value_json.is_none()); // No worst value with single source
        assert!(comparison.avg_value_json.is_none()); // No average with single source
    }

    #[test]
    fn test_get_sources() {
        let sources = create_test_sources();
        let new_comparison = NewPriceComparison {
            product_id: "test_product".to_string(),
            sources: sources.clone(),
        };
        
        let comparison = PriceComparison::new(new_comparison).unwrap();
        let retrieved_sources = comparison.get_sources().unwrap();
        
        assert_eq!(retrieved_sources.len(), 3);
        assert_eq!(retrieved_sources[0].source_id, "source1");
        assert_eq!(retrieved_sources[1].source_id, "source2");
        assert_eq!(retrieved_sources[2].source_id, "source3");
    }

    #[test]
    fn test_get_best_value() {
        let sources = create_test_sources();
        let new_comparison = NewPriceComparison {
            product_id: "test_product".to_string(),
            sources,
        };
        
        let comparison = PriceComparison::new(new_comparison).unwrap();
        let best_value = comparison.get_best_value().unwrap();
        
        // Should be Store C's value ($17.99)
        assert_eq!(best_value["amount"], "17.99");
        assert_eq!(best_value["currency"], "USD");
    }

    #[test]
    fn test_get_worst_value() {
        let sources = create_test_sources();
        let new_comparison = NewPriceComparison {
            product_id: "test_product".to_string(),
            sources,
        };
        
        let comparison = PriceComparison::new(new_comparison).unwrap();
        let worst_value = comparison.get_worst_value().unwrap().unwrap();
        
        // Should be Store B's value ($24.99)
        assert_eq!(worst_value["amount"], "24.99");
        assert_eq!(worst_value["currency"], "USD");
    }

    #[test]
    fn test_get_avg_value() {
        let sources = create_test_sources();
        let new_comparison = NewPriceComparison {
            product_id: "test_product".to_string(),
            sources,
        };
        
        let comparison = PriceComparison::new(new_comparison).unwrap();
        let avg_value = comparison.get_avg_value().unwrap().unwrap();
        
        // Average of 19.99, 24.99, 17.99 = 20.99
        assert_eq!(avg_value["amount"], "20.99");
        assert_eq!(avg_value["currency"], "USD");
    }

    #[test]
    fn test_get_savings() {
        let sources = create_test_sources();
        let new_comparison = NewPriceComparison {
            product_id: "test_product".to_string(),
            sources,
        };
        
        let comparison = PriceComparison::new(new_comparison).unwrap();
        let savings = comparison.get_savings().unwrap();
        
        // Savings = worst (24.99) - best (17.99) = 7.00
        assert!((savings - 7.0).abs() < 0.01);
    }

    #[test]
    fn test_get_savings_percentage() {
        let sources = create_test_sources();
        let new_comparison = NewPriceComparison {
            product_id: "test_product".to_string(),
            sources,
        };
        
        let comparison = PriceComparison::new(new_comparison).unwrap();
        let savings_pct = comparison.get_savings_percentage().unwrap();
        
        // Savings percentage = (7.00 / 24.99) * 100 ≈ 28.01%
        assert!((savings_pct - 28.01).abs() < 0.1);
    }

    #[test]
    fn test_extract_amount_price_object() {
        let value = json!({"amount": "19.99", "currency": "USD"});
        assert_eq!(PriceComparison::extract_amount(&value), Some(19.99));
    }

    #[test]
    fn test_extract_amount_number_object() {
        let value = json!({"number": 42.5});
        assert_eq!(PriceComparison::extract_amount(&value), Some(42.5));
    }

    #[test]
    fn test_extract_amount_simple_number() {
        let value = json!(123.45);
        assert_eq!(PriceComparison::extract_amount(&value), Some(123.45));
    }

    #[test]
    fn test_extract_amount_string() {
        let value = json!("99.99");
        assert_eq!(PriceComparison::extract_amount(&value), Some(99.99));
    }

    #[test]
    fn test_extract_amount_invalid() {
        let value = json!("not a number");
        assert_eq!(PriceComparison::extract_amount(&value), None);
    }

    #[test]
    fn test_compare_values() {
        let value_a = json!({"amount": "19.99", "currency": "USD"});
        let value_b = json!({"amount": "24.99", "currency": "USD"});
        
        assert_eq!(PriceComparison::compare_values(&value_a, &value_b), std::cmp::Ordering::Less);
        assert_eq!(PriceComparison::compare_values(&value_b, &value_a), std::cmp::Ordering::Greater);
        assert_eq!(PriceComparison::compare_values(&value_a, &value_a), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_calculate_average_price_objects() {
        let sources = vec![
            SourceComparison {
                source_id: "s1".to_string(),
                store_name: "Store A".to_string(),
                value: json!({"amount": "10.00", "currency": "USD"}),
                formatted_value: "$10.00".to_string(),
                url: "https://a.com".to_string(),
            },
            SourceComparison {
                source_id: "s2".to_string(),
                store_name: "Store B".to_string(),
                value: json!({"amount": "20.00", "currency": "USD"}),
                formatted_value: "$20.00".to_string(),
                url: "https://b.com".to_string(),
            },
        ];
        
        let avg = PriceComparison::calculate_average(&sources).unwrap();
        assert_eq!(avg["amount"], "15.00");
        assert_eq!(avg["currency"], "USD");
    }

    #[test]
    fn test_calculate_average_number_objects() {
        let sources = vec![
            SourceComparison {
                source_id: "s1".to_string(),
                store_name: "Store A".to_string(),
                value: json!({"number": 10.0}),
                formatted_value: "10".to_string(),
                url: "https://a.com".to_string(),
            },
            SourceComparison {
                source_id: "s2".to_string(),
                store_name: "Store B".to_string(),
                value: json!({"number": 20.0}),
                formatted_value: "20".to_string(),
                url: "https://b.com".to_string(),
            },
        ];
        
        let avg = PriceComparison::calculate_average(&sources).unwrap();
        assert_eq!(avg["number"], 15.0);
    }

    #[test]
    fn test_serialization() {
        let sources = create_test_sources();
        let new_comparison = NewPriceComparison {
            product_id: "test_product".to_string(),
            sources,
        };
        
        let comparison = PriceComparison::new(new_comparison).unwrap();
        
        let serialized = serde_json::to_string(&comparison).unwrap();
        let deserialized: PriceComparison = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(comparison, deserialized);
    }
}