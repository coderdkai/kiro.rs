//! 凭据统计数据库操作

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;

/// 统计数据行
#[derive(Debug, sqlx::FromRow)]
pub struct StatsRow {
    pub credential_id: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub refresh_failure_count: i64,
    pub last_used_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
}

/// 获取凭据统计
pub async fn get(pool: &SqlitePool, credential_id: u64) -> Result<Option<StatsRow>> {
    let row = sqlx::query_as::<_, StatsRow>(
        "SELECT * FROM credential_stats WHERE credential_id = ?"
    )
    .bind(credential_id as i64)
    .fetch_optional(pool)
    .await
    .context("查询统计数据失败")?;

    Ok(row)
}

/// 初始化凭据统计（如果不存在）
pub async fn init(pool: &SqlitePool, credential_id: u64) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO credential_stats (credential_id) VALUES (?)"
    )
    .bind(credential_id as i64)
    .execute(pool)
    .await
    .context("初始化统计数据失败")?;

    Ok(())
}

/// 记录成功调用
pub async fn record_success(pool: &SqlitePool, credential_id: u64) -> Result<()> {
    let now = Utc::now().to_rfc3339();

    // 先确保记录存在
    init(pool, credential_id).await?;

    sqlx::query(
        r#"
        UPDATE credential_stats SET
            success_count = success_count + 1,
            last_used_at = ?,
            last_success_at = ?
        WHERE credential_id = ?
        "#
    )
    .bind(&now)
    .bind(&now)
    .bind(credential_id as i64)
    .execute(pool)
    .await
    .context("记录成功调用失败")?;

    Ok(())
}

/// 记录失败调用
pub async fn record_failure(pool: &SqlitePool, credential_id: u64) -> Result<()> {
    let now = Utc::now().to_rfc3339();

    // 先确保记录存在
    init(pool, credential_id).await?;

    sqlx::query(
        r#"
        UPDATE credential_stats SET
            failure_count = failure_count + 1,
            last_used_at = ?,
            last_failure_at = ?
        WHERE credential_id = ?
        "#
    )
    .bind(&now)
    .bind(&now)
    .bind(credential_id as i64)
    .execute(pool)
    .await
    .context("记录失败调用失败")?;

    Ok(())
}

/// 记录刷新失败
pub async fn record_refresh_failure(pool: &SqlitePool, credential_id: u64) -> Result<()> {
    // 先确保记录存在
    init(pool, credential_id).await?;

    sqlx::query(
        "UPDATE credential_stats SET refresh_failure_count = refresh_failure_count + 1 WHERE credential_id = ?"
    )
    .bind(credential_id as i64)
    .execute(pool)
    .await
    .context("记录刷新失败失败")?;

    Ok(())
}

/// 重置失败计数
pub async fn reset_failure_counts(pool: &SqlitePool, credential_id: u64) -> Result<()> {
    sqlx::query(
        "UPDATE credential_stats SET failure_count = 0, refresh_failure_count = 0 WHERE credential_id = ?"
    )
    .bind(credential_id as i64)
    .execute(pool)
    .await
    .context("重置失败计数失败")?;

    Ok(())
}
