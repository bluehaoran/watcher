use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub mod product;
pub mod source;
pub mod price_comparison;
pub mod notification_config;
pub mod price_history;
pub mod false_positive;
pub mod notification_log;
pub mod system_setting;

// Re-exports for convenience
pub use product::*;
pub use source::*;
pub use price_comparison::*;
pub use notification_config::*;
pub use price_history::*;
pub use false_positive::*;
pub use notification_log::*;
pub use system_setting::*;

// Common enums used across models
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT")]
pub enum TrackerType {
    #[sqlx(rename = "price")]
    Price,
    #[sqlx(rename = "version")]
    Version,
    #[sqlx(rename = "number")]
    Number,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT")]
pub enum NotifyOn {
    #[sqlx(rename = "any_change")]
    AnyChange,
    #[sqlx(rename = "decrease")]
    Decrease,
    #[sqlx(rename = "increase")]
    Increase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT")]
pub enum ThresholdType {
    #[sqlx(rename = "absolute")]
    Absolute,
    #[sqlx(rename = "relative")]
    Relative,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT")]
pub enum SelectorType {
    #[sqlx(rename = "css")]
    Css,
    #[sqlx(rename = "xpath")]
    Xpath,
    #[sqlx(rename = "text")]
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT")]
pub enum NotificationStatus {
    #[sqlx(rename = "sent")]
    Sent,
    #[sqlx(rename = "failed")]
    Failed,
    #[sqlx(rename = "actioned")]
    Actioned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT")]
pub enum NotificationAction {
    #[sqlx(rename = "dismissed")]
    Dismissed,
    #[sqlx(rename = "false_positive")]
    FalsePositive,
    #[sqlx(rename = "purchased")]
    Purchased,
}

// Helper function to generate UUIDs in the format expected by the database
pub fn generate_id() -> String {
    Uuid::new_v4().simple().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_type_serialization() {
        assert_eq!(
            serde_json::to_string(&TrackerType::Price).unwrap(),
            "\"price\""
        );
        assert_eq!(
            serde_json::to_string(&TrackerType::Version).unwrap(),
            "\"version\""
        );
        assert_eq!(
            serde_json::to_string(&TrackerType::Number).unwrap(),
            "\"number\""
        );
    }

    #[test]
    fn test_tracker_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<TrackerType>("\"price\"").unwrap(),
            TrackerType::Price
        );
        assert_eq!(
            serde_json::from_str::<TrackerType>("\"version\"").unwrap(),
            TrackerType::Version
        );
        assert_eq!(
            serde_json::from_str::<TrackerType>("\"number\"").unwrap(),
            TrackerType::Number
        );
    }

    #[test]
    fn test_notify_on_values() {
        let values = vec![NotifyOn::AnyChange, NotifyOn::Decrease, NotifyOn::Increase];
        for value in values {
            let serialized = serde_json::to_string(&value).unwrap();
            let deserialized: NotifyOn = serde_json::from_str(&serialized).unwrap();
            assert_eq!(value, deserialized);
        }
    }

    #[test]
    fn test_threshold_type_values() {
        let values = vec![ThresholdType::Absolute, ThresholdType::Relative];
        for value in values {
            let serialized = serde_json::to_string(&value).unwrap();
            let deserialized: ThresholdType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(value, deserialized);
        }
    }

    #[test]
    fn test_selector_type_values() {
        let values = vec![SelectorType::Css, SelectorType::Xpath];
        for value in values {
            let serialized = serde_json::to_string(&value).unwrap();
            let deserialized: SelectorType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(value, deserialized);
        }
    }

    #[test]
    fn test_notification_status_values() {
        let values = vec![
            NotificationStatus::Sent,
            NotificationStatus::Failed,
            NotificationStatus::Actioned,
        ];
        for value in values {
            let serialized = serde_json::to_string(&value).unwrap();
            let deserialized: NotificationStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(value, deserialized);
        }
    }

    #[test]
    fn test_notification_action_values() {
        let values = vec![
            NotificationAction::Dismissed,
            NotificationAction::FalsePositive,
            NotificationAction::Purchased,
        ];
        for value in values {
            let serialized = serde_json::to_string(&value).unwrap();
            let deserialized: NotificationAction = serde_json::from_str(&serialized).unwrap();
            assert_eq!(value, deserialized);
        }
    }

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 32); // UUID simple format is 32 chars
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
    }
}