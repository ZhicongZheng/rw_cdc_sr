pub mod repository;
pub mod schema;

use crate::utils::error::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::PathBuf;

pub use repository::*;

/// 初始化数据库
pub async fn init_database() -> Result<SqlitePool> {
    // 获取应用数据目录
    let data_dir = get_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("rw_cdc_sr.db");

    // 确保数据库文件可以被创建（如果不存在的话）
    if !db_path.exists() {
        std::fs::File::create(&db_path)?;
    }

    let db_url = format!("sqlite://{}", db_path.display());

    tracing::info!("Initializing database at: {}", db_url);

    // 创建连接池
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // 运行迁移
    run_migrations(&pool).await?;

    Ok(pool)
}

/// 运行数据库迁移
async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    tracing::info!("Running database migrations");

    // 创建数据库配置表
    sqlx::query(schema::CREATE_DATABASE_CONFIGS_TABLE)
        .execute(pool)
        .await?;

    // 创建同步任务表
    sqlx::query(schema::CREATE_SYNC_TASKS_TABLE)
        .execute(pool)
        .await?;

    // 创建任务日志表
    sqlx::query(schema::CREATE_TASK_LOGS_TABLE)
        .execute(pool)
        .await?;

    tracing::info!("Database migrations completed");

    Ok(())
}

/// 获取应用数据目录
fn get_data_dir() -> Result<PathBuf> {
    let data_dir = if cfg!(target_os = "macos") {
        dirs::data_local_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Data directory not found"))?
            .join("com.rwcdcsr.app")
    } else if cfg!(target_os = "windows") {
        dirs::data_local_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Data directory not found"))?
            .join("RwCdcSr")
    } else {
        dirs::data_local_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Data directory not found"))?
            .join("rw_cdc_sr")
    };

    Ok(data_dir)
}

// 添加 dirs crate 到 Cargo.toml
// dirs = "5.0"
