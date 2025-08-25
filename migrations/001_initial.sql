-- Products table: Core product information
CREATE TABLE products (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    name TEXT NOT NULL,
    description TEXT,
    tracker_type TEXT NOT NULL CHECK (tracker_type IN ('price', 'version', 'number')),
    
    -- Notification rules (apply to all sources)
    notify_on TEXT NOT NULL DEFAULT 'any_change' 
        CHECK (notify_on IN ('any_change', 'decrease', 'increase')),
    threshold_type TEXT CHECK (threshold_type IN ('absolute', 'relative')),
    threshold_value REAL,
    
    -- Schedule
    check_interval TEXT NOT NULL DEFAULT '0 0 * * *', -- Cron expression
    last_checked DATETIME,
    next_check DATETIME,
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_paused BOOLEAN NOT NULL DEFAULT FALSE,
    
    -- Best deal tracking
    best_source_id TEXT,
    best_value_json TEXT, -- JSON serialized value
    
    -- Metadata
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Sources table: URLs where products can be tracked
CREATE TABLE sources (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL,
    
    -- Source information
    url TEXT NOT NULL,
    store_name TEXT,
    title TEXT NOT NULL,
    
    -- Selector information
    selector TEXT NOT NULL,
    selector_type TEXT NOT NULL DEFAULT 'css' CHECK (selector_type IN ('css', 'xpath')),
    
    -- Values (JSON serialized for complex types)
    original_value_json TEXT, -- Parsed original value
    current_value_json TEXT,  -- Parsed current value
    original_text TEXT,       -- Raw original text
    current_text TEXT,        -- Raw current text
    
    -- Source-specific status
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_checked DATETIME,
    error_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    
    -- Metadata
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE,
    UNIQUE(product_id, url)
);

-- Price comparisons across sources
CREATE TABLE price_comparisons (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL,
    
    -- Comparison data (JSON serialized)
    sources_json TEXT NOT NULL, -- Array of {sourceId, value, storeName}
    best_source_id TEXT NOT NULL,
    best_value_json TEXT NOT NULL,
    worst_value_json TEXT,
    avg_value_json TEXT,
    
    timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE
);

-- Notification configurations
CREATE TABLE notification_configs (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL,
    notifier_type TEXT NOT NULL,
    config_json TEXT NOT NULL, -- Plugin-specific configuration
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE
);

-- Price history for tracking changes over time
CREATE TABLE price_history (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    source_id TEXT NOT NULL,
    value_json TEXT NOT NULL,
    text TEXT NOT NULL,
    timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (source_id) REFERENCES sources(id) ON DELETE CASCADE
);

-- False positives for debugging and learning
CREATE TABLE false_positives (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    source_id TEXT NOT NULL,
    detected_text TEXT NOT NULL,
    detected_value_json TEXT NOT NULL,
    actual_text TEXT,
    html_context TEXT NOT NULL,
    screenshot_path TEXT,
    notes TEXT,
    timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (source_id) REFERENCES sources(id) ON DELETE CASCADE
);

-- Notification logs for tracking sent notifications
CREATE TABLE notification_logs (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    product_id TEXT NOT NULL,
    notification_type TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('sent', 'failed', 'actioned')),
    action TEXT CHECK (action IN ('dismissed', 'false_positive', 'purchased')),
    error TEXT,
    timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    actioned_at DATETIME,
    
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE
);

-- System settings (key-value pairs)
CREATE TABLE system_settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL
);

-- Triggers for updated_at timestamps
CREATE TRIGGER update_products_updated_at
    AFTER UPDATE ON products
    FOR EACH ROW
    BEGIN
        UPDATE products SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
    END;

CREATE TRIGGER update_sources_updated_at
    AFTER UPDATE ON sources
    FOR EACH ROW
    BEGIN
        UPDATE sources SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
    END;