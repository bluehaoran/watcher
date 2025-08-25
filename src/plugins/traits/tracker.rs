use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    pub success: bool,
    pub value: serde_json::Value,
    pub normalized: String,
    pub confidence: f32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub changed: bool,
    pub change_type: ChangeType,
    pub difference: serde_json::Value,
    pub percent_change: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeType {
    Increased,
    Decreased,
    Unchanged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementMatch {
    pub element: String,      // CSS selector
    pub text: String,
    pub html: String,
    pub context: String,      // Surrounding HTML
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub name: String,
    pub field_type: ConfigFieldType,
    pub label: String,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub options: Option<Vec<ConfigOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigFieldType {
    Text,
    Number,
    Select,
    Checkbox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSchema {
    pub fields: Vec<ConfigField>,
}

/// Trait for implementing value trackers (price, version, number, etc.)
#[async_trait]
pub trait TrackerPlugin: Send + Sync {
    /// Plugin metadata
    fn name(&self) -> &str;
    fn plugin_type(&self) -> &str;
    fn description(&self) -> &str;
    
    /// Core functionality
    fn parse(&self, text: &str) -> ParseResult;
    fn format(&self, value: &serde_json::Value) -> String;
    fn compare(&self, old_value: &serde_json::Value, new_value: &serde_json::Value) -> ComparisonResult;
    
    /// Element matching and ranking
    fn get_search_variations(&self, input: &str) -> Vec<String>;
    fn rank_matches(&self, input: &str, matches: &[ElementMatch]) -> Vec<ElementMatch>;
    
    /// Configuration
    fn get_config_schema(&self) -> ConfigSchema;
    fn validate_config(&self, config: &serde_json::Value) -> bool;
    
    /// Plugin lifecycle
    async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}