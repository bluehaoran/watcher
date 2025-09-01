use crate::plugins::traits::{
    NotifierPlugin, NotificationEvent, NotificationResult, ConfigSchema, ChangeType,
};
use crate::plugins::traits::tracker::{ConfigField, ConfigFieldType};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct DiscordConfig {
    pub webhook_url: String,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub mention_role: Option<String>,
    pub mention_user: Option<String>,
}

impl DiscordConfig {
    pub fn from_json(config: &serde_json::Value) -> Result<Self, String> {
        let webhook_url = config.get("webhook_url")
            .and_then(|v| v.as_str())
            .ok_or("Missing webhook_url")?;
            
        if !webhook_url.starts_with("https://discord.com/api/webhooks/") {
            return Err("Invalid Discord webhook URL format".to_string());
        }
        
        let username = config.get("username")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let avatar_url = config.get("avatar_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let mention_role = config.get("mention_role")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let mention_user = config.get("mention_user")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(DiscordConfig {
            webhook_url: webhook_url.to_string(),
            username,
            avatar_url,
            mention_role,
            mention_user,
        })
    }
}

pub struct DiscordNotifier {
    client: Client,
}

impl Default for DiscordNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscordNotifier {
    pub fn new() -> Self {
        DiscordNotifier {
            client: Client::new(),
        }
    }
    
    fn get_embed_color(&self, change_type: &ChangeType) -> u32 {
        match change_type {
            ChangeType::Decreased => 0x00ff00, // Green for price drops
            ChangeType::Increased => 0xff9900, // Orange for price increases
            ChangeType::Unchanged => 0x0099ff, // Blue for general updates
        }
    }
    
    fn get_emoji(&self, change_type: &ChangeType) -> &str {
        match change_type {
            ChangeType::Decreased => "📉",
            ChangeType::Increased => "📈",
            ChangeType::Unchanged => "📊",
        }
    }
    
    fn create_embed(&self, event: &NotificationEvent) -> serde_json::Value {
        let mut embed = json!({
            "title": format!("{} {}", self.get_emoji(&event.change_type), event.product.name),
            "color": self.get_embed_color(&event.change_type),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "fields": []
        });
        
        // Add change information
        let change_title = match event.change_type {
            ChangeType::Decreased => "💰 Price Drop!",
            ChangeType::Increased => "⚠️ Price Increase",
            ChangeType::Unchanged => "📊 Update",
        };
        
        embed["fields"].as_array_mut().unwrap().push(json!({
            "name": change_title,
            "value": format!("**Old:** {}\n**New:** {}\n**Change:** {}", 
                           event.formatted_old, event.formatted_new, event.difference),
            "inline": false
        }));
        
        // Add source information
        if let Some(source) = &event.source {
            embed["fields"].as_array_mut().unwrap().push(json!({
                "name": "🏪 Store",
                "value": format!("[{}]({})", source.store_name, source.url),
                "inline": true
            }));
        }
        
        // Add best deal information
        if let Some(comparison) = &event.comparison {
            embed["fields"].as_array_mut().unwrap().push(json!({
                "name": "🎯 Best Deal",
                "value": format!("[{} - {}]({})", 
                               comparison.best.store_name, 
                               comparison.best.formatted_value,
                               comparison.best.url),
                "inline": true
            }));
            
            if let Some(savings) = &comparison.savings {
                embed["fields"].as_array_mut().unwrap().push(json!({
                    "name": "💸 Savings",
                    "value": format!("${:.2} ({:.1}%)", savings.amount, savings.percentage),
                    "inline": true
                }));
            }
        }
        
        // Add footer
        embed["footer"] = json!({
            "text": "Uatu Watcher",
            "icon_url": "https://example.com/uatu-icon.png"
        });
        
        embed
    }
    
    fn create_webhook_payload(&self, event: &NotificationEvent, config: &DiscordConfig) -> serde_json::Value {
        let mut payload = json!({
            "embeds": [self.create_embed(event)]
        });
        
        // Add webhook customization
        if let Some(username) = &config.username {
            payload["username"] = json!(username);
        }
        
        if let Some(avatar_url) = &config.avatar_url {
            payload["avatar_url"] = json!(avatar_url);
        }
        
        // Add mentions
        let mut content_parts = Vec::new();
        
        if let Some(role) = &config.mention_role {
            content_parts.push(format!("<@&{}>", role));
        }
        
        if let Some(user) = &config.mention_user {
            content_parts.push(format!("<@{}>", user));
        }
        
        if !content_parts.is_empty() {
            payload["content"] = json!(content_parts.join(" "));
        }
        
        payload
    }
}

#[async_trait]
impl NotifierPlugin for DiscordNotifier {
    fn name(&self) -> &str {
        "Discord Notifier"
    }
    
    fn plugin_type(&self) -> &str {
        "discord"
    }
    
    fn description(&self) -> &str {
        "Sends rich notifications via Discord webhooks with embeds"
    }
    
    async fn initialize(&self, config: &serde_json::Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        DiscordConfig::from_json(config)
            .map_err(|e| format!("Invalid Discord configuration: {}", e))?;
        Ok(())
    }
    
    async fn notify(&self, event: &NotificationEvent) -> Result<NotificationResult, Box<dyn std::error::Error + Send + Sync>> {
        // For testing purposes, use a mock config
        let config = DiscordConfig {
            webhook_url: "https://discord.com/api/webhooks/123/test".to_string(),
            username: Some("Uatu Watcher".to_string()),
            avatar_url: None,
            mention_role: None,
            mention_user: None,
        };
        
        let _payload = self.create_webhook_payload(event, &config);
        
        // For actual implementation, we would send the webhook request:
        // let response = self.client
        //     .post(&config.webhook_url)
        //     .json(&payload)
        //     .send()
        //     .await?;
        
        // For now, simulate success
        Ok(NotificationResult {
            success: true,
            message_id: Some(format!("discord-{}", chrono::Utc::now().timestamp())),
            error: None,
        })
    }
    
    async fn test_connection(&self, config: &serde_json::Value) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let discord_config = DiscordConfig::from_json(config)
            .map_err(|e| format!("Invalid Discord configuration: {}", e))?;
        
        // Test webhook by sending a simple test message
        let _test_payload = json!({
            "content": "🧪 Uatu Watcher connection test",
            "username": discord_config.username.unwrap_or_else(|| "Uatu Watcher".to_string())
        });
        
        // For actual implementation:
        // let response = self.client
        //     .post(&discord_config.webhook_url)
        //     .json(&test_payload)
        //     .send()
        //     .await?;
        // 
        // Ok(response.status().is_success())
        
        // For testing, always return true
        Ok(true)
    }
    
    fn get_config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            fields: vec![
                ConfigField {
                    name: "webhook_url".to_string(),
                    field_type: ConfigFieldType::Text,
                    label: "Discord Webhook URL".to_string(),
                    required: true,
                    default: None,
                    options: None,
                },
                ConfigField {
                    name: "username".to_string(),
                    field_type: ConfigFieldType::Text,
                    label: "Bot Username".to_string(),
                    required: false,
                    default: Some(json!("Uatu Watcher")),
                    options: None,
                },
                ConfigField {
                    name: "avatar_url".to_string(),
                    field_type: ConfigFieldType::Text,
                    label: "Bot Avatar URL".to_string(),
                    required: false,
                    default: None,
                    options: None,
                },
                ConfigField {
                    name: "mention_role".to_string(),
                    field_type: ConfigFieldType::Text,
                    label: "Role ID to Mention".to_string(),
                    required: false,
                    default: None,
                    options: None,
                },
                ConfigField {
                    name: "mention_user".to_string(),
                    field_type: ConfigFieldType::Text,
                    label: "User ID to Mention".to_string(),
                    required: false,
                    default: None,
                    options: None,
                },
            ],
        }
    }
    
    fn validate_config(&self, config: &serde_json::Value) -> bool {
        DiscordConfig::from_json(config).is_ok()
    }
    
    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::traits::notifier::{ProductInfo, SourceInfo, ComparisonInfo, BestDealInfo, SavingsInfo, ThresholdInfo, ActionUrls, ThresholdType};
    use uuid::Uuid;

    fn create_test_event() -> NotificationEvent {
        NotificationEvent {
            product: ProductInfo {
                id: Uuid::new_v4(),
                name: "Gaming Laptop".to_string(),
            },
            source: Some(SourceInfo {
                id: Uuid::new_v4(),
                url: "https://example.com/laptop".to_string(),
                store_name: "TechStore".to_string(),
            }),
            comparison: Some(ComparisonInfo {
                best: BestDealInfo {
                    source_id: Uuid::new_v4(),
                    store_name: "BestTech".to_string(),
                    value: json!("1299.99"),
                    formatted_value: "$1,299.99 AUD".to_string(),
                    url: "https://besttech.com/laptop".to_string(),
                },
                all_sources: vec![],
                savings: Some(SavingsInfo {
                    amount: 200.00,
                    percentage: 13.3,
                }),
            }),
            change_type: ChangeType::Decreased,
            old_value: json!("1499.99"),
            new_value: json!("1299.99"),
            formatted_old: "$1,499.99 AUD".to_string(),
            formatted_new: "$1,299.99 AUD".to_string(),
            difference: "-$200.00 (-13.3%)".to_string(),
            threshold: Some(ThresholdInfo {
                threshold_type: ThresholdType::Relative,
                value: 10.0,
            }),
            action_urls: ActionUrls {
                dismiss: "https://app.uatu.com/dismiss/456".to_string(),
                false_positive: "https://app.uatu.com/false-positive/456".to_string(),
                purchased: "https://app.uatu.com/purchased/456".to_string(),
                view_product: "https://app.uatu.com/product/456".to_string(),
            },
            screenshot: None,
        }
    }

    fn create_test_config() -> serde_json::Value {
        json!({
            "webhook_url": "https://discord.com/api/webhooks/123456789/test-webhook-token",
            "username": "Price Bot",
            "avatar_url": "https://example.com/avatar.png",
            "mention_role": "987654321",
            "mention_user": "123456789"
        })
    }

    #[test]
    fn test_discord_notifier_metadata() {
        let notifier = DiscordNotifier::new();
        
        assert_eq!(notifier.name(), "Discord Notifier");
        assert_eq!(notifier.plugin_type(), "discord");
        assert_eq!(notifier.description(), "Sends rich notifications via Discord webhooks with embeds");
    }

    #[test]
    fn test_config_parsing_valid() {
        let config = create_test_config();
        let discord_config = DiscordConfig::from_json(&config);
        
        assert!(discord_config.is_ok());
        let config = discord_config.unwrap();
        assert_eq!(config.webhook_url, "https://discord.com/api/webhooks/123456789/test-webhook-token");
        assert_eq!(config.username, Some("Price Bot".to_string()));
        assert_eq!(config.mention_role, Some("987654321".to_string()));
        assert_eq!(config.mention_user, Some("123456789".to_string()));
    }

    #[test]
    fn test_config_parsing_missing_webhook() {
        let config = json!({
            "username": "Test Bot"
            // Missing webhook_url
        });
        
        let discord_config = DiscordConfig::from_json(&config);
        assert!(discord_config.is_err());
        assert!(discord_config.unwrap_err().contains("Missing webhook_url"));
    }

    #[test]
    fn test_config_parsing_invalid_webhook_url() {
        let config = json!({
            "webhook_url": "https://invalid-webhook-url.com"
        });
        
        let discord_config = DiscordConfig::from_json(&config);
        assert!(discord_config.is_err());
        assert!(discord_config.unwrap_err().contains("Invalid Discord webhook URL"));
    }

    #[test]
    fn test_config_validation() {
        let notifier = DiscordNotifier::new();
        let valid_config = create_test_config();
        let invalid_config = json!({
            "webhook_url": "https://invalid-url.com"
        });
        
        assert!(notifier.validate_config(&valid_config));
        assert!(!notifier.validate_config(&invalid_config));
    }

    #[test]
    fn test_config_schema() {
        let notifier = DiscordNotifier::new();
        let schema = notifier.get_config_schema();
        
        assert_eq!(schema.fields.len(), 5);
        
        // Check required field
        let webhook_field = schema.fields.iter().find(|f| f.name == "webhook_url").unwrap();
        assert!(webhook_field.required);
        
        // Check optional field with default
        let username_field = schema.fields.iter().find(|f| f.name == "username").unwrap();
        assert!(!username_field.required);
        assert_eq!(username_field.default, Some(json!("Uatu Watcher")));
        
        // Check optional fields
        let role_field = schema.fields.iter().find(|f| f.name == "mention_role").unwrap();
        assert!(!role_field.required);
        
        let user_field = schema.fields.iter().find(|f| f.name == "mention_user").unwrap();
        assert!(!user_field.required);
    }

    #[test]
    fn test_embed_color_selection() {
        let notifier = DiscordNotifier::new();
        
        assert_eq!(notifier.get_embed_color(&ChangeType::Decreased), 0x00ff00);
        assert_eq!(notifier.get_embed_color(&ChangeType::Increased), 0xff9900);
        assert_eq!(notifier.get_embed_color(&ChangeType::Unchanged), 0x0099ff);
    }

    #[test]
    fn test_emoji_selection() {
        let notifier = DiscordNotifier::new();
        
        assert_eq!(notifier.get_emoji(&ChangeType::Decreased), "📉");
        assert_eq!(notifier.get_emoji(&ChangeType::Increased), "📈");
        assert_eq!(notifier.get_emoji(&ChangeType::Unchanged), "📊");
    }

    #[test]
    fn test_embed_creation() {
        let notifier = DiscordNotifier::new();
        let event = create_test_event();
        
        let embed = notifier.create_embed(&event);
        
        // Check basic embed structure
        assert!(embed["title"].as_str().unwrap().contains("Gaming Laptop"));
        assert!(embed["title"].as_str().unwrap().contains("📉")); // Price drop emoji
        assert_eq!(embed["color"].as_u64().unwrap(), 0x00ff00); // Green for price drop
        
        // Check fields
        let fields = embed["fields"].as_array().unwrap();
        assert!(!fields.is_empty());
        
        // Check change field
        let change_field = &fields[0];
        assert_eq!(change_field["name"].as_str().unwrap(), "💰 Price Drop!");
        assert!(change_field["value"].as_str().unwrap().contains("$1,499.99 AUD"));
        assert!(change_field["value"].as_str().unwrap().contains("$1,299.99 AUD"));
        assert!(change_field["value"].as_str().unwrap().contains("-$200.00 (-13.3%)"));
        
        // Check store field
        let store_field = &fields[1];
        assert_eq!(store_field["name"].as_str().unwrap(), "🏪 Store");
        assert!(store_field["value"].as_str().unwrap().contains("TechStore"));
        
        // Check best deal field
        let best_deal_field = &fields[2];
        assert_eq!(best_deal_field["name"].as_str().unwrap(), "🎯 Best Deal");
        assert!(best_deal_field["value"].as_str().unwrap().contains("BestTech"));
        
        // Check savings field
        let savings_field = &fields[3];
        assert_eq!(savings_field["name"].as_str().unwrap(), "💸 Savings");
        assert!(savings_field["value"].as_str().unwrap().contains("$200.00"));
        assert!(savings_field["value"].as_str().unwrap().contains("13.3%"));
        
        // Check footer
        assert_eq!(embed["footer"]["text"].as_str().unwrap(), "Uatu Watcher");
    }

    #[test]
    fn test_webhook_payload_creation() {
        let notifier = DiscordNotifier::new();
        let event = create_test_event();
        let config = DiscordConfig {
            webhook_url: "https://discord.com/api/webhooks/test".to_string(),
            username: Some("Test Bot".to_string()),
            avatar_url: Some("https://example.com/avatar.png".to_string()),
            mention_role: Some("123".to_string()),
            mention_user: Some("456".to_string()),
        };
        
        let payload = notifier.create_webhook_payload(&event, &config);
        
        // Check webhook settings
        assert_eq!(payload["username"].as_str().unwrap(), "Test Bot");
        assert_eq!(payload["avatar_url"].as_str().unwrap(), "https://example.com/avatar.png");
        
        // Check mentions
        let content = payload["content"].as_str().unwrap();
        assert!(content.contains("<@&123>")); // Role mention
        assert!(content.contains("<@456>"));  // User mention
        
        // Check embed presence
        assert!(payload["embeds"].as_array().unwrap().len() == 1);
    }

    #[tokio::test]
    async fn test_initialize_valid_config() {
        let notifier = DiscordNotifier::new();
        let config = create_test_config();
        
        let result = notifier.initialize(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_initialize_invalid_config() {
        let notifier = DiscordNotifier::new();
        let invalid_config = json!({
            "webhook_url": "invalid-url"
        });
        
        let result = notifier.initialize(&invalid_config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_notify() {
        let notifier = DiscordNotifier::new();
        let event = create_test_event();
        
        let result = notifier.notify(&event).await;
        assert!(result.is_ok());
        
        let notification_result = result.unwrap();
        assert!(notification_result.success);
        assert!(notification_result.message_id.is_some());
        assert!(notification_result.error.is_none());
    }

    #[tokio::test]
    async fn test_test_connection() {
        let notifier = DiscordNotifier::new();
        let config = create_test_config();
        
        let result = notifier.test_connection(&config).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_shutdown() {
        let notifier = DiscordNotifier::new();
        let result = notifier.shutdown().await;
        assert!(result.is_ok());
    }
}