use crate::plugins::traits::{
    NotifierPlugin, NotificationEvent, NotificationResult, ConfigSchema, ChangeType,
};
use crate::plugins::traits::tracker::{ConfigField, ConfigFieldType};
use async_trait::async_trait;
use lettre::message::{header, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    pub to_email: String,
    pub use_tls: bool,
}

impl EmailConfig {
    pub fn from_json(config: &serde_json::Value) -> Result<Self, String> {
        let smtp_server = config.get("smtp_server")
            .and_then(|v| v.as_str())
            .ok_or("Missing smtp_server")?;
        let smtp_port = config.get("smtp_port")
            .and_then(|v| v.as_u64())
            .map(|v| v as u16)
            .unwrap_or(587);
        let username = config.get("username")
            .and_then(|v| v.as_str())
            .ok_or("Missing username")?;
        let password = config.get("password")
            .and_then(|v| v.as_str())
            .ok_or("Missing password")?;
        let from_email = config.get("from_email")
            .and_then(|v| v.as_str())
            .ok_or("Missing from_email")?;
        let from_name = config.get("from_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Uatu Watcher");
        let to_email = config.get("to_email")
            .and_then(|v| v.as_str())
            .ok_or("Missing to_email")?;
        let use_tls = config.get("use_tls")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        Ok(EmailConfig {
            smtp_server: smtp_server.to_string(),
            smtp_port,
            username: username.to_string(),
            password: password.to_string(),
            from_email: from_email.to_string(),
            from_name: from_name.to_string(),
            to_email: to_email.to_string(),
            use_tls,
        })
    }
}

pub struct EmailNotifier {
    config: Option<EmailConfig>,
}

impl Default for EmailNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl EmailNotifier {
    pub fn new() -> Self {
        EmailNotifier { config: None }
    }
    
    fn format_subject(&self, event: &NotificationEvent) -> String {
        match event.change_type {
            ChangeType::Decreased => format!("🔔 Price Drop: {} - {}", event.product.name, event.formatted_new),
            ChangeType::Increased => format!("📈 Price Increase: {} - {}", event.product.name, event.formatted_new),
            ChangeType::Unchanged => format!("📊 Update: {}", event.product.name),
        }
    }
    
    fn format_html_body(&self, event: &NotificationEvent) -> String {
        let mut html = String::new();
        
        html.push_str(&format!(r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: #f0f0f0; padding: 15px; border-radius: 5px; }}
        .product {{ font-size: 18px; font-weight: bold; }}
        .change {{ margin: 15px 0; padding: 10px; border-radius: 5px; }}
        .price-drop {{ background: #e8f5e8; border-left: 4px solid #4CAF50; }}
        .price-increase {{ background: #fff3cd; border-left: 4px solid #ff9800; }}
        .comparison {{ margin: 15px 0; }}
        .source {{ margin: 5px 0; padding: 10px; background: #f9f9f9; border-radius: 3px; }}
        .actions {{ margin: 20px 0; }}
        .button {{ display: inline-block; padding: 8px 15px; margin: 5px; text-decoration: none; border-radius: 3px; }}
        .primary {{ background: #007cba; color: white; }}
        .secondary {{ background: #6c757d; color: white; }}
    </style>
</head>
<body>
    <div class="header">
        <div class="product">{}</div>
    </div>
"#, event.product.name));

        // Change information
        let change_class = match event.change_type {
            ChangeType::Decreased => "price-drop",
            ChangeType::Increased => "price-increase", 
            ChangeType::Unchanged => "price-drop", // Default styling
        };
        
        html.push_str(&format!(r#"
    <div class="change {}">
        <strong>Change:</strong> {} → {}<br>
        <strong>Difference:</strong> {}
    </div>
"#, change_class, event.formatted_old, event.formatted_new, event.difference));

        // Source information
        if let Some(source) = &event.source {
            html.push_str(&format!(r#"
    <div class="source">
        <strong>Store:</strong> {}<br>
        <strong>URL:</strong> <a href="{}">{}</a>
    </div>
"#, source.store_name, source.url, source.url));
        }

        // Comparison information
        if let Some(comparison) = &event.comparison {
            html.push_str(r#"<div class="comparison"><h3>Price Comparison</h3>"#);
            html.push_str(&format!(r#"
    <div class="source">
        <strong>Best Deal:</strong> {} - {} 
        <a href="{}" class="button primary">View Deal</a>
    </div>
"#, comparison.best.store_name, comparison.best.formatted_value, comparison.best.url));
            
            html.push_str("</div>");
        }

        // Action buttons
        html.push_str(&format!(r#"
    <div class="actions">
        <a href="{}" class="button primary">View Product</a>
        <a href="{}" class="button secondary">Mark as Purchased</a>
        <a href="{}" class="button secondary">Dismiss</a>
    </div>
"#, event.action_urls.view_product, event.action_urls.purchased, event.action_urls.dismiss));

        html.push_str(r#"
</body>
</html>
"#);

        html
    }
    
    fn format_text_body(&self, event: &NotificationEvent) -> String {
        let mut text = String::new();
        
        text.push_str("🔔 UATU WATCHER ALERT\n\n");
        text.push_str(&format!("Product: {}\n", event.product.name));
        text.push_str(&format!("Change: {} → {}\n", event.formatted_old, event.formatted_new));
        text.push_str(&format!("Difference: {}\n\n", event.difference));
        
        if let Some(source) = &event.source {
            text.push_str(&format!("Store: {}\n", source.store_name));
            text.push_str(&format!("URL: {}\n\n", source.url));
        }
        
        if let Some(comparison) = &event.comparison {
            text.push_str("PRICE COMPARISON:\n");
            text.push_str(&format!("Best Deal: {} - {}\n", comparison.best.store_name, comparison.best.formatted_value));
            text.push_str(&format!("Best Deal URL: {}\n\n", comparison.best.url));
        }
        
        text.push_str("ACTIONS:\n");
        text.push_str(&format!("View Product: {}\n", event.action_urls.view_product));
        text.push_str(&format!("Mark Purchased: {}\n", event.action_urls.purchased));
        text.push_str(&format!("Dismiss: {}\n", event.action_urls.dismiss));
        
        text
    }
}

#[async_trait]
impl NotifierPlugin for EmailNotifier {
    fn name(&self) -> &str {
        "Email Notifier"
    }
    
    fn plugin_type(&self) -> &str {
        "email"
    }
    
    fn description(&self) -> &str {
        "Sends notifications via SMTP email with HTML formatting"
    }
    
    async fn initialize(&self, config: &serde_json::Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        EmailConfig::from_json(config)
            .map_err(|e| format!("Invalid email configuration: {}", e))?;
        // In a real implementation, we would store the config in self
        // For now, just validate it
        Ok(())
    }
    
    async fn notify(&self, event: &NotificationEvent) -> Result<NotificationResult, Box<dyn std::error::Error + Send + Sync>> {
        // For testing purposes, use a mock config
        // In real usage, this would come from initialize()
        let config = EmailConfig {
            smtp_server: "smtp.gmail.com".to_string(),
            smtp_port: 587,
            username: "test@example.com".to_string(),
            password: "password".to_string(),
            from_email: "test@example.com".to_string(),
            from_name: "Test Notifier".to_string(),
            to_email: "recipient@example.com".to_string(),
            use_tls: true,
        };
            
        let subject = self.format_subject(event);
        let html_body = self.format_html_body(event);
        let text_body = self.format_text_body(event);
        
        let email = Message::builder()
            .from(format!("{} <{}>", config.from_name, config.from_email).parse()?)
            .to(config.to_email.parse()?)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_PLAIN)
                            .body(text_body)
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(html_body)
                    )
            )?;

        // Create SMTP transport
        let credentials = Credentials::new(config.username.clone(), config.password.clone());
        
        let mailer = if config.use_tls {
            SmtpTransport::relay(&config.smtp_server)?
        } else {
            SmtpTransport::builder_dangerous(&config.smtp_server)
        }
        .port(config.smtp_port)
        .credentials(credentials)
        .build();

        // Send email
        match mailer.send(&email) {
            Ok(_response) => Ok(NotificationResult {
                success: true,
                message_id: Some(format!("email-{}", chrono::Utc::now().timestamp())),
                error: None,
            }),
            Err(e) => Ok(NotificationResult {
                success: false,
                message_id: None,
                error: Some(e.to_string()),
            }),
        }
    }
    
    async fn test_connection(&self, config: &serde_json::Value) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let email_config = EmailConfig::from_json(config)
            .map_err(|e| format!("Invalid email configuration: {}", e))?;

        let credentials = Credentials::new(email_config.username.clone(), email_config.password.clone());
        
        let mailer = if email_config.use_tls {
            SmtpTransport::relay(&email_config.smtp_server)?
        } else {
            SmtpTransport::builder_dangerous(&email_config.smtp_server)
        }
        .port(email_config.smtp_port)
        .credentials(credentials)
        .build();

        // Test connection by connecting and authenticating
        match mailer.test_connection() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    fn get_config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            fields: vec![
                ConfigField {
                    name: "smtp_server".to_string(),
                    field_type: ConfigFieldType::Text,
                    label: "SMTP Server".to_string(),
                    required: true,
                    default: None,
                    options: None,
                },
                ConfigField {
                    name: "smtp_port".to_string(),
                    field_type: ConfigFieldType::Number,
                    label: "SMTP Port".to_string(),
                    required: false,
                    default: Some(json!(587)),
                    options: None,
                },
                ConfigField {
                    name: "username".to_string(),
                    field_type: ConfigFieldType::Text,
                    label: "Username".to_string(),
                    required: true,
                    default: None,
                    options: None,
                },
                ConfigField {
                    name: "password".to_string(),
                    field_type: ConfigFieldType::Text,
                    label: "Password".to_string(),
                    required: true,
                    default: None,
                    options: None,
                },
                ConfigField {
                    name: "from_email".to_string(),
                    field_type: ConfigFieldType::Text,
                    label: "From Email".to_string(),
                    required: true,
                    default: None,
                    options: None,
                },
                ConfigField {
                    name: "from_name".to_string(),
                    field_type: ConfigFieldType::Text,
                    label: "From Name".to_string(),
                    required: false,
                    default: Some(json!("Uatu Watcher")),
                    options: None,
                },
                ConfigField {
                    name: "to_email".to_string(),
                    field_type: ConfigFieldType::Text,
                    label: "To Email".to_string(),
                    required: true,
                    default: None,
                    options: None,
                },
                ConfigField {
                    name: "use_tls".to_string(),
                    field_type: ConfigFieldType::Checkbox,
                    label: "Use TLS".to_string(),
                    required: false,
                    default: Some(json!(true)),
                    options: None,
                },
            ],
        }
    }
    
    fn validate_config(&self, config: &serde_json::Value) -> bool {
        EmailConfig::from_json(config).is_ok()
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
                name: "Test Product".to_string(),
            },
            source: Some(SourceInfo {
                id: Uuid::new_v4(),
                url: "https://example.com/product".to_string(),
                store_name: "Test Store".to_string(),
            }),
            comparison: Some(ComparisonInfo {
                best: BestDealInfo {
                    source_id: Uuid::new_v4(),
                    store_name: "Best Store".to_string(),
                    value: json!("19.99"),
                    formatted_value: "$19.99 AUD".to_string(),
                    url: "https://beststore.com/product".to_string(),
                },
                all_sources: vec![],
                savings: Some(SavingsInfo {
                    amount: 5.00,
                    percentage: 20.0,
                }),
            }),
            change_type: ChangeType::Decreased,
            old_value: json!("24.99"),
            new_value: json!("19.99"),
            formatted_old: "$24.99 AUD".to_string(),
            formatted_new: "$19.99 AUD".to_string(),
            difference: "-$5.00 (-20%)".to_string(),
            threshold: Some(ThresholdInfo {
                threshold_type: ThresholdType::Relative,
                value: 10.0,
            }),
            action_urls: ActionUrls {
                dismiss: "https://app.uatu.com/dismiss/123".to_string(),
                false_positive: "https://app.uatu.com/false-positive/123".to_string(),
                purchased: "https://app.uatu.com/purchased/123".to_string(),
                view_product: "https://app.uatu.com/product/123".to_string(),
            },
            screenshot: None,
        }
    }

    fn create_test_config() -> serde_json::Value {
        json!({
            "smtp_server": "smtp.gmail.com",
            "smtp_port": 587,
            "username": "test@gmail.com",
            "password": "test_password",
            "from_email": "test@gmail.com",
            "from_name": "Test Watcher",
            "to_email": "recipient@gmail.com",
            "use_tls": true
        })
    }

    #[test]
    fn test_email_notifier_metadata() {
        let notifier = EmailNotifier::new();
        
        assert_eq!(notifier.name(), "Email Notifier");
        assert_eq!(notifier.plugin_type(), "email");
        assert_eq!(notifier.description(), "Sends notifications via SMTP email with HTML formatting");
    }

    #[test]
    fn test_config_parsing_valid() {
        let config = create_test_config();
        let email_config = EmailConfig::from_json(&config);
        
        assert!(email_config.is_ok());
        let config = email_config.unwrap();
        assert_eq!(config.smtp_server, "smtp.gmail.com");
        assert_eq!(config.smtp_port, 587);
        assert_eq!(config.username, "test@gmail.com");
        assert_eq!(config.from_name, "Test Watcher");
        assert!(config.use_tls);
    }

    #[test]
    fn test_config_parsing_missing_required() {
        let config = json!({
            "smtp_server": "smtp.gmail.com"
            // Missing required fields
        });
        
        let email_config = EmailConfig::from_json(&config);
        assert!(email_config.is_err());
    }

    #[test]
    fn test_config_validation() {
        let notifier = EmailNotifier::new();
        let valid_config = create_test_config();
        let invalid_config = json!({
            "smtp_server": "smtp.gmail.com"
            // Missing required fields
        });
        
        assert!(notifier.validate_config(&valid_config));
        assert!(!notifier.validate_config(&invalid_config));
    }

    #[test]
    fn test_config_schema() {
        let notifier = EmailNotifier::new();
        let schema = notifier.get_config_schema();
        
        assert_eq!(schema.fields.len(), 8);
        
        // Check required fields
        let smtp_server_field = schema.fields.iter().find(|f| f.name == "smtp_server").unwrap();
        assert!(smtp_server_field.required);
        
        let username_field = schema.fields.iter().find(|f| f.name == "username").unwrap();
        assert!(username_field.required);
        
        // Check optional fields with defaults
        let smtp_port_field = schema.fields.iter().find(|f| f.name == "smtp_port").unwrap();
        assert!(!smtp_port_field.required);
        assert_eq!(smtp_port_field.default, Some(json!(587)));
        
        let from_name_field = schema.fields.iter().find(|f| f.name == "from_name").unwrap();
        assert_eq!(from_name_field.default, Some(json!("Uatu Watcher")));
    }

    #[test]
    fn test_subject_formatting() {
        let notifier = EmailNotifier::new();
        let mut event = create_test_event();
        
        // Test price decrease
        event.change_type = ChangeType::Decreased;
        let subject = notifier.format_subject(&event);
        assert!(subject.contains("🔔 Price Drop"));
        assert!(subject.contains("Test Product"));
        assert!(subject.contains("$19.99 AUD"));
        
        // Test price increase
        event.change_type = ChangeType::Increased;
        let subject = notifier.format_subject(&event);
        assert!(subject.contains("📈 Price Increase"));
    }

    #[test]
    fn test_html_body_formatting() {
        let notifier = EmailNotifier::new();
        let event = create_test_event();
        
        let html = notifier.format_html_body(&event);
        
        // Check essential content is present
        assert!(html.contains("Test Product"));
        assert!(html.contains("$24.99 AUD → $19.99 AUD"));
        assert!(html.contains("-$5.00 (-20%)"));
        assert!(html.contains("Test Store"));
        assert!(html.contains("https://example.com/product"));
        assert!(html.contains("Best Store"));
        assert!(html.contains("price-drop")); // CSS class for price decrease
        
        // Check action buttons
        assert!(html.contains("View Product"));
        assert!(html.contains("Mark as Purchased"));
        assert!(html.contains("Dismiss"));
    }

    #[test]
    fn test_text_body_formatting() {
        let notifier = EmailNotifier::new();
        let event = create_test_event();
        
        let text = notifier.format_text_body(&event);
        
        // Check essential content is present
        assert!(text.contains("UATU WATCHER ALERT"));
        assert!(text.contains("Test Product"));
        assert!(text.contains("$24.99 AUD → $19.99 AUD"));
        assert!(text.contains("-$5.00 (-20%)"));
        assert!(text.contains("Test Store"));
        assert!(text.contains("https://example.com/product"));
        assert!(text.contains("PRICE COMPARISON"));
        assert!(text.contains("Best Store"));
        assert!(text.contains("ACTIONS"));
    }

    #[tokio::test]
    async fn test_initialize_valid_config() {
        let notifier = EmailNotifier::new();
        let config = create_test_config();
        
        let result = notifier.initialize(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_initialize_invalid_config() {
        let notifier = EmailNotifier::new();
        let invalid_config = json!({
            "smtp_server": "smtp.gmail.com"
            // Missing required fields
        });
        
        let result = notifier.initialize(&invalid_config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_shutdown() {
        let notifier = EmailNotifier::new();
        let result = notifier.shutdown().await;
        assert!(result.is_ok());
    }
}