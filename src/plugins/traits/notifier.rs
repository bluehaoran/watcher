use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::tracker::{ConfigSchema, ChangeType}; // Re-use from tracker module

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvent {
    pub product: ProductInfo,
    pub source: Option<SourceInfo>,
    pub comparison: Option<ComparisonInfo>,
    pub change_type: ChangeType,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub formatted_old: String,
    pub formatted_new: String,
    pub difference: String,
    pub threshold: Option<ThresholdInfo>,
    pub action_urls: ActionUrls,
    pub screenshot: Option<String>, // Base64 or URL
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductInfo {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub id: Uuid,
    pub url: String,
    pub store_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonInfo {
    pub best: BestDealInfo,
    pub all_sources: Vec<SourceComparisonInfo>,
    pub savings: Option<SavingsInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestDealInfo {
    pub source_id: Uuid,
    pub store_name: String,
    pub value: serde_json::Value,
    pub formatted_value: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceComparisonInfo {
    pub source_id: Uuid,
    pub store_name: String,
    pub value: serde_json::Value,
    pub formatted_value: String,
    pub url: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavingsInfo {
    pub amount: f64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdInfo {
    pub threshold_type: ThresholdType,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThresholdType {
    Absolute,
    Relative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionUrls {
    pub dismiss: String,
    pub false_positive: String,
    pub purchased: String,
    pub view_product: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResult {
    pub success: bool,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

/// Trait for implementing notification methods (email, Discord, etc.)
#[async_trait]
pub trait NotifierPlugin: Send + Sync {
    /// Plugin metadata
    fn name(&self) -> &str;
    fn plugin_type(&self) -> &str;
    fn description(&self) -> &str;
    
    /// Core functionality
    async fn initialize(&self, config: &serde_json::Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn notify(&self, event: &NotificationEvent) -> Result<NotificationResult, Box<dyn std::error::Error + Send + Sync>>;
    async fn test_connection(&self, config: &serde_json::Value) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
    
    /// Configuration
    fn get_config_schema(&self) -> ConfigSchema;
    fn validate_config(&self, config: &serde_json::Value) -> bool;
    
    /// Plugin lifecycle
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}