//! 凭据数据库操作

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::SqlitePool;

use crate::kiro::model::credentials::KiroCredentials;

/// 数据库凭据行
#[derive(Debug, sqlx::FromRow)]
pub struct CredentialRow {
    pub id: i64,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub profile_arn: Option<String>,
    pub expires_at: Option<String>,
    pub auth_method: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub priority: i64,
    pub region: Option<String>,
    pub auth_region: Option<String>,
    pub api_region: Option<String>,
    pub machine_id: Option<String>,
    pub email: Option<String>,
    pub subscription_title: Option<String>,
    pub proxy_url: Option<String>,
    pub proxy_username: Option<String>,
    pub proxy_password: Option<String>,
    pub disabled: bool,
    pub kiro_api_key: Option<String>,
    pub endpoint: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<CredentialRow> for KiroCredentials {
    fn from(row: CredentialRow) -> Self {
        KiroCredentials {
            id: Some(row.id as u64),
            access_token: row.access_token,
            refresh_token: row.refresh_token,
            profile_arn: row.profile_arn,
            expires_at: row.expires_at,
            auth_method: row.auth_method,
            client_id: row.client_id,
            client_secret: row.client_secret,
            priority: row.priority as u32,
            region: row.region,
            auth_region: row.auth_region,
            api_region: row.api_region,
            machine_id: row.machine_id,
            email: row.email,
            subscription_title: row.subscription_title,
            proxy_url: row.proxy_url,
            proxy_username: row.proxy_username,
            proxy_password: row.proxy_password,
            disabled: row.disabled,
            kiro_api_key: row.kiro_api_key,
            endpoint: row.endpoint,
        }
    }
}

/// 获取所有凭据
pub async fn get_all(pool: &SqlitePool) -> Result<Vec<KiroCredentials>> {
    let rows = sqlx::query_as::<_, CredentialRow>(
        "SELECT * FROM credentials ORDER BY priority ASC, id ASC"
    )
    .fetch_all(pool)
    .await
    .context("查询凭据失败")?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// 根据 ID 获取凭据
pub async fn get_by_id(pool: &SqlitePool, id: u64) -> Result<Option<KiroCredentials>> {
    let row = sqlx::query_as::<_, CredentialRow>(
        "SELECT * FROM credentials WHERE id = ?"
    )
    .bind(id as i64)
    .fetch_optional(pool)
    .await
    .context("查询凭据失败")?;

    Ok(row.map(Into::into))
}

/// 插入凭据
pub async fn insert(pool: &SqlitePool, cred: &KiroCredentials) -> Result<u64> {
    let result = sqlx::query(
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
    .execute(pool)
    .await
    .context("插入凭据失败")?;

    Ok(result.last_insert_rowid() as u64)
}

/// 更新凭据
pub async fn update(pool: &SqlitePool, id: u64, cred: &KiroCredentials) -> Result<()> {
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE credentials SET
            access_token = ?, refresh_token = ?, profile_arn = ?, expires_at = ?,
            auth_method = ?, client_id = ?, client_secret = ?, priority = ?,
            region = ?, auth_region = ?, api_region = ?, machine_id = ?,
            email = ?, subscription_title = ?, proxy_url = ?, proxy_username = ?,
            proxy_password = ?, disabled = ?, kiro_api_key = ?, endpoint = ?,
            updated_at = ?
        WHERE id = ?
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
    .bind(&now)
    .bind(id as i64)
    .execute(pool)
    .await
    .context("更新凭据失败")?;

    Ok(())
}

/// 删除凭据
pub async fn delete(pool: &SqlitePool, id: u64) -> Result<()> {
    sqlx::query("DELETE FROM credentials WHERE id = ?")
        .bind(id as i64)
        .execute(pool)
        .await
        .context("删除凭据失败")?;

    Ok(())
}

/// 更新凭据的 disabled 状态
pub async fn update_disabled(pool: &SqlitePool, id: u64, disabled: bool) -> Result<()> {
    let now = Utc::now().to_rfc3339();

    sqlx::query("UPDATE credentials SET disabled = ?, updated_at = ? WHERE id = ?")
        .bind(disabled)
        .bind(&now)
        .bind(id as i64)
        .execute(pool)
        .await
        .context("更新凭据状态失败")?;

    Ok(())
}

/// 更新凭据优先级
pub async fn update_priority(pool: &SqlitePool, id: u64, priority: u32) -> Result<()> {
    let now = Utc::now().to_rfc3339();

    sqlx::query("UPDATE credentials SET priority = ?, updated_at = ? WHERE id = ?")
        .bind(priority as i64)
        .bind(&now)
        .bind(id as i64)
        .execute(pool)
        .await
        .context("更新凭据优先级失败")?;

    Ok(())
}
