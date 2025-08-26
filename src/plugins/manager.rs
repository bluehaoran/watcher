// use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::traits::{TrackerPlugin, NotifierPlugin};
use super::trackers::{PriceTracker, VersionTracker, NumberTracker};
use super::notifiers::{EmailNotifier, DiscordNotifier};
use crate::utils::error::AppError;

pub type TrackerPluginBox = Box<dyn TrackerPlugin>;
pub type NotifierPluginBox = Box<dyn NotifierPlugin>;

#[derive(Clone)]
pub struct PluginManager {
    trackers: Arc<RwLock<HashMap<String, TrackerPluginBox>>>,
    notifiers: Arc<RwLock<HashMap<String, NotifierPluginBox>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            trackers: Arc::new(RwLock::new(HashMap::new())),
            notifiers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a tracker plugin
    pub async fn register_tracker(&self, plugin: TrackerPluginBox) -> Result<(), AppError> {
        let plugin_type = plugin.plugin_type().to_string();
        plugin.initialize().await
            .map_err(|e| AppError::PluginError(format!("Failed to initialize tracker {}: {}", plugin_type, e)))?;
        
        let mut trackers = self.trackers.write().await;
        trackers.insert(plugin_type, plugin);
        Ok(())
    }
    
    /// Register a notifier plugin
    pub async fn register_notifier(&self, plugin: NotifierPluginBox) -> Result<(), AppError> {
        let plugin_type = plugin.plugin_type().to_string();
        
        let mut notifiers = self.notifiers.write().await;
        notifiers.insert(plugin_type, plugin);
        Ok(())
    }
    
    /// Check if a tracker plugin exists
    pub async fn has_tracker(&self, plugin_type: &str) -> bool {
        let trackers = self.trackers.read().await;
        trackers.contains_key(plugin_type)
    }
    
    /// Check if a notifier plugin exists
    pub async fn has_notifier(&self, plugin_type: &str) -> bool {
        let notifiers = self.notifiers.read().await;
        notifiers.contains_key(plugin_type)
    }
    
    /// List all available tracker types
    pub async fn list_tracker_types(&self) -> Vec<String> {
        let trackers = self.trackers.read().await;
        trackers.keys().cloned().collect()
    }
    
    /// List all available notifier types
    pub async fn list_notifier_types(&self) -> Vec<String> {
        let notifiers = self.notifiers.read().await;
        notifiers.keys().cloned().collect()
    }
    
    /// Initialize all plugins with default implementations
    pub async fn initialize_default_plugins(&self) -> Result<(), AppError> {
        // Register tracker plugins
        self.register_tracker(Box::new(PriceTracker::new())).await?;
        self.register_tracker(Box::new(VersionTracker::new())).await?;
        self.register_tracker(Box::new(NumberTracker::new())).await?;
        
        // Register notifier plugins
        self.register_notifier(Box::new(EmailNotifier::new())).await?;
        self.register_notifier(Box::new(DiscordNotifier::new())).await?;
        
        Ok(())
    }
    
    /// Parse text using a tracker plugin
    pub async fn parse_value_with_tracker(&self, plugin_type: &str, text: &str) -> Result<serde_json::Value, AppError> {
        let trackers = self.trackers.read().await;
        if let Some(tracker) = trackers.get(plugin_type) {
            let parse_result = tracker.as_ref().parse(text);
            if parse_result.success {
                Ok(parse_result.value)
            } else {
                Err(AppError::PluginError(format!("Tracker {} failed to parse text", plugin_type)))
            }
        } else {
            Err(AppError::PluginError(format!("Tracker plugin '{}' not found", plugin_type)))
        }
    }
    
    /// Send notification using a notifier plugin
    pub async fn send_notification(&self, plugin_type: &str, event: &super::traits::notifier::NotificationEvent) -> Result<super::traits::notifier::NotificationResult, AppError> {
        let notifiers = self.notifiers.read().await;
        if let Some(notifier) = notifiers.get(plugin_type) {
            match notifier.as_ref().notify(event).await {
                Ok(result) => Ok(result),
                Err(e) => Err(AppError::PluginError(format!("Notifier {} failed: {}", plugin_type, e)))
            }
        } else {
            Err(AppError::PluginError(format!("Notifier plugin '{}' not found", plugin_type)))
        }
    }
    
    /// Shutdown all plugins
    pub async fn shutdown(&self) -> Result<(), AppError> {
        // Shutdown all trackers
        let mut trackers = self.trackers.write().await;
        for (_, plugin) in trackers.drain() {
            if let Err(e) = plugin.shutdown().await {
                tracing::warn!("Error shutting down tracker plugin: {}", e);
            }
        }
        
        // Shutdown all notifiers
        let mut notifiers = self.notifiers.write().await;
        for (_, plugin) in notifiers.drain() {
            if let Err(e) = plugin.shutdown().await {
                tracing::warn!("Error shutting down notifier plugin: {}", e);
            }
        }
        
        Ok(())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        assert!(manager.list_tracker_types().await.is_empty());
        assert!(manager.list_notifier_types().await.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_initialization() {
        let manager = PluginManager::new();
        let result = manager.initialize_default_plugins().await;
        assert!(result.is_ok());
        
        // Should have registered tracker plugins
        let tracker_types = manager.list_tracker_types().await;
        assert!(tracker_types.contains(&"price".to_string()));
        assert!(tracker_types.contains(&"version".to_string()));
        assert!(tracker_types.contains(&"number".to_string()));
        
        // Should have registered notifier plugins
        let notifier_types = manager.list_notifier_types().await;
        assert!(notifier_types.contains(&"email".to_string()));
        assert!(notifier_types.contains(&"discord".to_string()));
    }

    #[tokio::test] 
    async fn test_plugin_exists_check() {
        let manager = PluginManager::new();
        manager.initialize_default_plugins().await.unwrap();
        
        assert!(manager.has_tracker("price").await);
        assert!(manager.has_tracker("version").await);
        assert!(manager.has_tracker("number").await);
        assert!(!manager.has_tracker("nonexistent").await);
        
        assert!(manager.has_notifier("email").await);
        assert!(manager.has_notifier("discord").await);
        assert!(!manager.has_notifier("nonexistent").await);
    }
}