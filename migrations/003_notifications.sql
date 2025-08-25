-- Add notification rate limiting
ALTER TABLE notification_configs ADD COLUMN rate_limit_hours INTEGER DEFAULT 24;
ALTER TABLE notification_configs ADD COLUMN last_sent DATETIME;

-- Add webhook URLs for action handling
ALTER TABLE notification_logs ADD COLUMN webhook_id TEXT;
ALTER TABLE notification_logs ADD COLUMN action_token TEXT;

-- Create webhook tokens table for secure action handling
CREATE TABLE action_tokens (
    token TEXT PRIMARY KEY,
    notification_log_id TEXT NOT NULL,
    action_type TEXT NOT NULL CHECK (action_type IN ('dismiss', 'false_positive', 'purchased')),
    expires_at DATETIME NOT NULL,
    used_at DATETIME,
    
    FOREIGN KEY (notification_log_id) REFERENCES notification_logs(id) ON DELETE CASCADE
);

CREATE INDEX idx_action_tokens_expires ON action_tokens(expires_at);
CREATE INDEX idx_action_tokens_used ON action_tokens(used_at);