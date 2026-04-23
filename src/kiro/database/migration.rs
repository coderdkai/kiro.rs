//! JSON 数据迁移工具

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::path::Path;
use tracing::{info, warn};

use crate::kiro::model::credentials::KiroCredentials;
use super::credentials;

/// 从 JSON 文件迁移到数据库
///
/// # 参数
/// - `json_path`: JSON 文件路径
/// - `pool`: 数据库连接池
///
/// # 返回
/// - 迁移的凭据数量
pub async fn migrate_from_json(json_path: &Path, pool: &SqlitePool) -> Result<usize> {
    info!("开始从 JSON 迁移数据: {}", json_path.display());

    // 读取 JSON 文件
    let json_content = tokio::fs::read_to_string(json_path)
        .await
        .context("读取 JSON 文件失败")?;

    // 尝试解析为单个凭据或凭据数组
    let credentials: Vec<KiroCredentials> = if let Ok(single) = serde_json::from_str::<KiroCredentials>(&json_content) {
        vec![single]
    } else if let Ok(multiple) = serde_json::from_str::<Vec<KiroCredentials>>(&json_content) {
        multiple
    } else {
        anyhow::bail!("无法解析 JSON 文件格式");
    };

    if credentials.is_empty() {
        warn!("JSON 文件中没有凭据数据");
        return Ok(0);
    }

    info!("找到 {} 个凭据，开始迁移", credentials.len());

    let mut migrated_count = 0;

    // 开始事务
    let mut tx = pool.begin().await.context("开始事务失败")?;

    for cred in credentials {
        // 插入凭据
        let id = sqlx::query(
            r#"
            INSERT INTO credentials (
                access_token, refresh_token, profile_arn, expires_at, auth_method,
                client_id, client_secret, priority, region, auth_region, api_region,
                machine_id, email, subscription_title, proxy_url, proxy_username,
                proxy_password, disabled, kiro_api_key, endpoint
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&cred.access_token)
        .bind(&cred.refresh_token)
        .bind(&cred.profile_arn)
        .bind(&cred.expires_at)
        .bind(&cred.auth_method)
        .bind(&cred.client_id)
        .bind(&cred.client_secret)
        .bind(cred.priority as i64)
        .bind(&cred.region)
        .bind(&cred.auth_region)
        .bind(&cred.api_region)
        .bind(&cred.machine_id)
        .bind(&cred.email)
        .bind(&cred.subscription_title)
        .bind(&cred.proxy_url)
        .bind(&cred.proxy_username)
        .bind(&cred.proxy_password)
        .bind(cred.disabled)
        .bind(&cred.kiro_api_key)
        .bind(&cred.endpoint)
        .execute(&mut *tx)
        .await
        .context("插入凭据失败")?
        .last_insert_rowid();

        // 初始化统计数据
        sqlx::query("INSERT INTO credential_stats (credential_id) VALUES (?)")
            .bind(id)
            .execute(&mut *tx)
            .await
            .context("初始化统计数据失败")?;

        migrated_count += 1;
    }

    // 提交事务
    tx.commit().await.context("提交事务失败")?;

    info!("成功迁移 {} 个凭据", migrated_count);

    // 备份原 JSON 文件
    let backup_path = json_path.with_extension("json.backup");
    tokio::fs::copy(json_path, &backup_path)
        .await
        .context("备份 JSON 文件失败")?;

    info!("已备份原 JSON 文件到: {}", backup_path.display());

    Ok(migrated_count)
}

/// 导出数据库到 JSON 文件
///
/// # 参数
/// - `pool`: 数据库连接池
/// - `json_path`: 输出 JSON 文件路径
///
/// # 返回
/// - 导出的凭据数量
pub async fn export_to_json(pool: &SqlitePool, json_path: &Path) -> Result<usize> {
    info!("开始导出数据库到 JSON: {}", json_path.display());

    // 获取所有凭据
    let credentials = credentials::get_all(pool).await?;

    if credentials.is_empty() {
        warn!("数据库中没有凭据数据");
        return Ok(0);
    }

    // 序列化为 JSON
    let json_content = serde_json::to_string_pretty(&credentials)
        .context("序列化 JSON 失败")?;

    // 写入文件
    tokio::fs::write(json_path, json_content)
        .await
        .context("写入 JSON 文件失败")?;

    info!("成功导出 {} 个凭据到 JSON", credentials.len());

    Ok(credentials.len())
}

/// 检查是否需要迁移
///
/// # 参数
/// - `pool`: 数据库连接池
/// - `json_path`: JSON 文件路径
///
/// # 返回
/// - true: 需要迁移，false: 不需要迁移
pub async fn needs_migration(pool: &SqlitePool, json_path: &Path) -> Result<bool> {
    // 检查 JSON 文件是否存在
    if !json_path.exists() {
        return Ok(false);
    }

    // 检查数据库中是否已有凭据
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credentials")
        .fetch_one(pool)
        .await
        .context("查询凭据数量失败")?;

    // 如果数据库为空且 JSON 文件存在，则需要迁移
    Ok(count == 0)
}
