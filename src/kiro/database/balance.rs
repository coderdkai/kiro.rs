//! 余额缓存数据库操作

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;

/// 余额缓存行
#[derive(Debug, sqlx::FromRow)]
pub struct BalanceCacheRow {
    pub credential_id: i64,
    pub subscription_title: Option<String>,
    pub current_usage: f64,
    pub usage_limit: f64,
    pub remaining: f64,
    pub usage_percentage: f64,
    pub next_reset_at: Option<String>,
    pub cached_at: String,
}

/// 获取余额缓存
pub async fn get(pool: &SqlitePool, credential_id: u64) -> Result<Option<BalanceCacheRow>> {
    let row = sqlx::query_as::<_, BalanceCacheRow>(
        "SELECT * FROM balance_cache WHERE credential_id = ?"
    )
    .bind(credential_id as i64)
    .fetch_optional(pool)
    .await
    .context("查询余额缓存失败")?;

    Ok(row)
}

/// 保存或更新余额缓存
pub async fn upsert(
    pool: &SqlitePool,
    credential_id: u64,
    subscription_title: Option<&str>,
    current_usage: f64,
    usage_limit: f64,
    remaining: f64,
    usage_percentage: f64,
    next_reset_at: Option<&str>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO balance_cache (
            credential_id, subscription_title, current_usage, usage_limit,
            remaining, usage_percentage, next_reset_at, cached_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(credential_id) DO UPDATE SET
            subscription_title = excluded.subscription_title,
            current_usage = excluded.current_usage,
            usage_limit = excluded.usage_limit,
            remaining = excluded.remaining,
            usage_percentage = excluded.usage_percentage,
            next_reset_at = excluded.next_reset_at,
            cached_at = excluded.cached_at
        "#
    )
    .bind(credential_id as i64)
    .bind(subscription_title)
    .bind(current_usage)
    .bind(usage_limit)
    .bind(remaining)
    .bind(usage_percentage)
    .bind(next_reset_at)
    .bind(&now)
    .execute(pool)
    .await
    .context("保存余额缓存失败")?;

    Ok(())
}

/// 删除余额缓存
pub async fn delete(pool: &SqlitePool, credential_id: u64) -> Result<()> {
    sqlx::query("DELETE FROM balance_cache WHERE credential_id = ?")
        .bind(credential_id as i64)
        .execute(pool)
        .await
        .context("删除余额缓存失败")?;

    Ok(())
}

/// 清理过期缓存（超过 TTL）
pub async fn clean_expired(pool: &SqlitePool, ttl_seconds: i64) -> Result<u64> {
    let cutoff = Utc::now() - chrono::Duration::seconds(ttl_seconds);
    let cutoff_str = cutoff.to_rfc3339();

    let result = sqlx::query("DELETE FROM balance_cache WHERE cached_at < ?")
        .bind(&cutoff_str)
        .execute(pool)
        .await
        .context("清理过期缓存失败")?;

    Ok(result.rows_affected())
}
