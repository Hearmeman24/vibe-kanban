-- Create webhooks table for subscription storage
CREATE TABLE webhooks (
    id            BLOB PRIMARY KEY,
    project_id    BLOB NOT NULL,
    url           TEXT NOT NULL CHECK(url != ''),
    secret        TEXT NOT NULL CHECK(secret != ''),  -- For HMAC signing
    events        TEXT NOT NULL CHECK(events != ''),  -- JSON array: ["task.created", "task.updated", ...]
    is_active     INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Index for efficient lookup by project_id
CREATE INDEX idx_webhooks_project_id ON webhooks(project_id);

-- Index for finding active webhooks
CREATE INDEX idx_webhooks_active ON webhooks(is_active) WHERE is_active = 1;

-- Create webhook_deliveries table for delivery attempts tracking
CREATE TABLE webhook_deliveries (
    id            BLOB PRIMARY KEY,
    webhook_id    BLOB NOT NULL,
    event_type    TEXT NOT NULL CHECK(event_type != ''),
    payload       TEXT NOT NULL,  -- JSON payload
    status        TEXT NOT NULL DEFAULT 'pending'
                     CHECK (status IN ('pending', 'success', 'failed', 'retrying')),
    attempts      INTEGER NOT NULL DEFAULT 0,
    last_error    TEXT,
    next_retry_at TEXT,           -- For exponential backoff
    created_at    TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    delivered_at  TEXT,
    FOREIGN KEY (webhook_id) REFERENCES webhooks(id) ON DELETE CASCADE
);

-- Index for efficient lookup by webhook_id
CREATE INDEX idx_webhook_deliveries_webhook_id ON webhook_deliveries(webhook_id);

-- Index for finding pending/retrying deliveries (for retry queue processing)
CREATE INDEX idx_webhook_deliveries_pending ON webhook_deliveries(status, next_retry_at)
    WHERE status IN ('pending', 'retrying');

-- Index for finding deliveries by status
CREATE INDEX idx_webhook_deliveries_status ON webhook_deliveries(status);
