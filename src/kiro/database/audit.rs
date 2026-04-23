//! 审计日志数据库操作

use anyhow::{Context, Result};
use sqlx::SqlitePool;

/// 审计日志事件类型
#[derive(Debug, Clone, Copy)]
pub enum EventType {
    Add,
    Delete,
    Disable,
    Enable,
    PriorityChange,
    TokenRefresh,
    ApiCall,
}

impl EventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::Add => "add",
            EventType::Delete => "delete",
            EventType::Disable => "disable",
            EventType::Enable => "enable",
            EventType::PriorityChange => "priority_change",
            EventType::TokenRefresh => "token_refresh",
            EventType::ApiCall => "api_call",
        }
    }
}

/// 记录审计日志
pub async fn log(
    pool: &SqlitePool,
    credential_id: Option<u64>,
    event_type: EventType,
    event_data: Option<&str>,
    result: bool,
    error_message: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (credential_id, event_type, event_data, result, error_message)
        VALUES (?, ?, ?, ?, ?)
        "#
    )
    .bind(credential_id.map(|id| id as i64))
    .bind(event_type.as_str())
    .bind(event_data)
    .bind(if result { "success" } else { "failure" })
    .bind(error_message)
    .execute(pool)
    .await
    .context("记录审计日志失败")?;

    Ok(())
}

/// 审计日志行
#[derive(Debug, sqlx::FromRow)]
pub struct AuditLogRow {
    pub id: i64,
    pub credential_id: Option<i64>,
    pub event_type: String,
    pub event_data: Option<String>,
    pub result: String,
    pub error_message: Option<String>,
    pub created_at: String,
}

/// 获取凭据的审计日志
pub async fn get_by_credential(
    pool: &SqlitePool,
    credential_id: u64,
    limit: i64,
) -> Result<Vec<AuditLogRow>> {
    let rows = sqlx::query_as::<_, AuditLogRow>(
        "SELECT * FROM audit_logs WHERE credential_id = ? ORDER BY created_at DESC LIMIT ?"
    )
    .bind(credential_id as i64)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("查询审计日志失败")?;

    Ok(rows)
}

/// 获取最近的审计日志
pub async fn get_recent(pool: &SqlitePool, limit: i64) -> Result<Vec<AuditLogRow>> {
    let rows = sqlx::query_as::<_, AuditLogRow>(
        "SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT ?"
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("查询审计日志失败")?;

    Ok(rows)
}

/// 清理旧日志（保留最近 N 天）
pub async fn clean_old(pool: &SqlitePool, days: i64) -> Result<u64> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
    let cutoff_str = cutoff.to_rfc3339();

    let result = sqlx::query("DELETE FROM audit_logs WHERE created_at < ?")
        .bind(&cutoff_str)
        .execute(pool)
        .await
        .context("清理旧日志失败")?;

    Ok(result.rows_affected())
}
