-- Performance indexes
CREATE INDEX idx_products_active ON products(is_active, is_paused);
CREATE INDEX idx_products_next_check ON products(next_check) WHERE is_active = TRUE;
CREATE INDEX idx_products_tracker_type ON products(tracker_type);

CREATE INDEX idx_sources_product_id ON sources(product_id);
CREATE INDEX idx_sources_active ON sources(is_active);
CREATE INDEX idx_sources_last_checked ON sources(last_checked);
CREATE INDEX idx_sources_error_count ON sources(error_count);

CREATE INDEX idx_price_comparisons_product_timestamp ON price_comparisons(product_id, timestamp);
CREATE INDEX idx_price_history_source_timestamp ON price_history(source_id, timestamp);
CREATE INDEX idx_notification_logs_product_timestamp ON notification_logs(product_id, timestamp);

CREATE INDEX idx_false_positives_source_id ON false_positives(source_id);
CREATE INDEX idx_notification_configs_product_type ON notification_configs(product_id, notifier_type);