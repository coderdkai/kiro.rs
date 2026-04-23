//! 数据库连接池管理

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;
use tracing::info;

/// 初始化数据库连接池
///
/// # 参数
/// - `database_path`: 数据库文件路径
///
/// # 返回
/// - `SqlitePool`: 数据库连接池
pub async fn init_pool(database_path: &Path) -> Result<SqlitePool> {
    // 确保父目录存在
    if let Some(parent) = database_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("创建数据库目录失败")?;
    }

    let database_url = format!("sqlite:{}", database_path.display());
    info!("初始化数据库连接池: {}", database_url);

    // 配置连接选项
    let connect_options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal) // 使用 WAL 模式提高并发性能
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal) // 平衡性能和安全性
        .busy_timeout(std::time::Duration::from_secs(30)); // 锁等待超时

    // 创建连接池
    let pool = SqlitePoolOptions::new()
        .max_connections(5) // SQLite 写入串行化，限制连接数
        .connect_with(connect_options)
        .await
        .context("创建数据库连接池失败")?;

    info!("数据库连接池初始化成功");
    Ok(pool)
}

/// 运行数据库迁移
///
/// # 参数
/// - `pool`: 数据库连接池
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    info!("开始执行数据库迁移");

    // 读取并执行 Schema
    let schema_sql = include_str!("../../../migrations/001_initial_schema.sql");

    sqlx::raw_sql(schema_sql)
        .execute(pool)
        .await
        .context("执行数据库迁移失败")?;

    info!("数据库迁移完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_pool() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = init_pool(&db_path).await.unwrap();
        assert!(db_path.exists());

        // 测试连接
        let result: i64 = sqlx::query_scalar("SELECT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_run_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = init_pool(&db_path).await.unwrap();
        run_migrations(&pool).await.unwrap();

        // 验证表是否创建
        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('credentials', 'credential_stats', 'balance_cache', 'audit_logs')"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(table_count, 4);
    }
}
