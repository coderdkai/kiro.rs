-- 凭据表
CREATE TABLE IF NOT EXISTS credentials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    access_token TEXT,
    refresh_token TEXT,
    profile_arn TEXT,
    expires_at TEXT,
    auth_method TEXT,
    client_id TEXT,
    client_secret TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    region TEXT,
    auth_region TEXT,
    api_region TEXT,
    machine_id TEXT,
    email TEXT,
    subscription_title TEXT,
    proxy_url TEXT,
    proxy_username TEXT,
    proxy_password TEXT,
    disabled BOOLEAN NOT NULL DEFAULT 0,
    kiro_api_key TEXT,
    endpoint TEXT,
    password TEXT,
    web_access_token TEXT,
    web_session_token TEXT,
    web_user_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 部分唯一索引（仅约束非 NULL 值）
CREATE UNIQUE INDEX IF NOT EXISTS idx_credentials_refresh_token
    ON credentials(refresh_token) WHERE refresh_token IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_credentials_kiro_api_key
    ON credentials(kiro_api_key) WHERE kiro_api_key IS NOT NULL;

-- 统计表
CREATE TABLE IF NOT EXISTS credential_stats (
    credential_id INTEGER PRIMARY KEY,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    refresh_failure_count INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT,
    last_success_at TEXT,
    last_failure_at TEXT,
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE CASCADE
);

-- 余额缓存表
CREATE TABLE IF NOT EXISTS balance_cache (
    credential_id INTEGER PRIMARY KEY,
    subscription_title TEXT,
    current_usage REAL NOT NULL,
    usage_limit REAL NOT NULL,
    remaining REAL NOT NULL,
    usage_percentage REAL NOT NULL,
    next_reset_at TEXT,
    cached_at TEXT NOT NULL,
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE CASCADE
);

-- 审计日志表
CREATE TABLE IF NOT EXISTS audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    credential_id INTEGER,
    event_type TEXT NOT NULL,
    event_data TEXT,
    result TEXT NOT NULL,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE SET NULL
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_credentials_priority ON credentials(priority, disabled);
CREATE INDEX IF NOT EXISTS idx_credentials_disabled ON credentials(disabled);
CREATE INDEX IF NOT EXISTS idx_audit_logs_credential_id ON audit_logs(credential_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_audit_logs_event_type ON audit_logs(event_type);
