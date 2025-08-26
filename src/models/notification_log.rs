use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use crate::models::{NotificationStatus, NotificationAction, generate_id};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct NotificationLog {
    pub id: String,
    pub product_id: String,
    pub notification_type: String,
    pub status: NotificationStatus,
    pub action: Option<NotificationAction>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub actioned_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewNotificationLog {
    pub product_id: String,
    pub notification_type: String,
    pub status: NotificationStatus,
    pub error: Option<String>,
}

impl NotificationLog {
    pub fn new(new_log: NewNotificationLog) -> Self {
        Self {
            id: generate_id(),
            product_id: new_log.product_id,
            notification_type: new_log.notification_type,
            status: new_log.status,
            action: None,
            error: new_log.error,
            timestamp: Utc::now(),
            actioned_at: None,
        }
    }
    
    pub fn mark_actioned(&mut self, action: NotificationAction) {
        self.status = NotificationStatus::Actioned;
        self.action = Some(action);
        self.actioned_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_log_creation() {
        let new_log = NewNotificationLog {
            product_id: "product123".to_string(),
            notification_type: "email".to_string(),
            status: NotificationStatus::Sent,
            error: None,
        };
        
        let log = NotificationLog::new(new_log);
        
        assert_eq!(log.product_id, "product123");
        assert_eq!(log.notification_type, "email");
        assert_eq!(log.status, NotificationStatus::Sent);
        assert!(log.action.is_none());
        assert!(log.actioned_at.is_none());
    }

    #[test]
    fn test_mark_actioned() {
        let new_log = NewNotificationLog {
            product_id: "product123".to_string(),
            notification_type: "discord".to_string(),
            status: NotificationStatus::Sent,
            error: None,
        };
        
        let mut log = NotificationLog::new(new_log);
        log.mark_actioned(NotificationAction::Dismissed);
        
        assert_eq!(log.status, NotificationStatus::Actioned);
        assert_eq!(log.action, Some(NotificationAction::Dismissed));
        assert!(log.actioned_at.is_some());
    }
}