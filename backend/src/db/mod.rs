pub mod repository;
pub mod schema;

use crate::utils::error::Result;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};

pub use repository::*;

/// 初始化数据库
pub async fn init_database() -> Result<MySqlPool> {
    // 从环境变量读取数据库连接 URL
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:123456@localhost:3306/rw_cdc_sr".to_string());

    tracing::info!("Initializing database at: {}", mask_password(&database_url));

    // 创建连接池
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    // 运行迁移
    run_migrations(&pool).await?;

    Ok(pool)
}

/// 运行数据库迁移
async fn run_migrations(pool: &MySqlPool) -> Result<()> {
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

/// 隐藏密码用于日志输出
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let mut masked = url.to_string();
            masked.replace_range(colon_pos + 1..at_pos, "****");
            return masked;
        }
    }
    url.to_string()
}
