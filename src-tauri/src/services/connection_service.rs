use crate::models::{ConnectionTestResult, DatabaseConfig, DbType, TestConnectionRequest};
use crate::utils::error::{AppError, Result};
use sqlx::{
    mysql::{MySqlConnectOptions, MySqlSslMode},
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    Connection, MySqlConnection,
};
use mysql_async::prelude::*;

/// 连接服务
pub struct ConnectionService;

impl ConnectionService {
    /// 测试 MySQL 连接
    pub async fn test_mysql(req: &TestConnectionRequest) -> Result<ConnectionTestResult> {
        if req.db_type != DbType::MySQL {
            return Err(AppError::InvalidInput(
                "Expected MySQL connection type".to_string(),
            ));
        }

        tracing::info!("Testing MySQL connection to {}:{}", req.host, req.port);
        let opts = Self::build_mysql_options(req);

        match MySqlConnection::connect_with(&opts).await {
            Ok(mut conn) => {
                tracing::debug!("MySQL connection established, testing query...");
                // 测试简单查询
                sqlx::query("SELECT 1")
                    .execute(&mut conn)
                    .await
                    .map_err(|e| {
                        tracing::error!("MySQL query test failed: {}", e);
                        AppError::Connection(format!("Query test failed: {}", e))
                    })?;
                tracing::info!("MySQL connection test successful");
                Ok(ConnectionTestResult::success())
            }
            Err(e) => {
                tracing::error!("MySQL connection failed: {}", e);
                Ok(ConnectionTestResult::failure(format!(
                    "MySQL connection failed: {}",
                    e
                )))
            }
        }
    }

    /// 测试 RisingWave (PostgreSQL) 连接
    pub async fn test_risingwave(req: &TestConnectionRequest) -> Result<ConnectionTestResult> {
        if req.db_type != DbType::RisingWave {
            return Err(AppError::InvalidInput(
                "Expected RisingWave connection type".to_string(),
            ));
        }

        tracing::info!("Testing RisingWave connection to {}:{}", req.host, req.port);
        let opts = Self::build_postgres_options(req);

        match PgPoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
        {
            Ok(pool) => {
                tracing::debug!("RisingWave connection established, testing query...");
                // 测试简单查询
                sqlx::query("SELECT 1")
                    .execute(&pool)
                    .await
                    .map_err(|e| {
                        tracing::error!("RisingWave query test failed: {}", e);
                        AppError::Connection(format!("Query test failed: {}", e))
                    })?;
                tracing::info!("RisingWave connection test successful");
                Ok(ConnectionTestResult::success())
            }
            Err(e) => {
                tracing::error!("RisingWave connection failed: {}", e);
                Ok(ConnectionTestResult::failure(format!(
                    "RisingWave connection failed: {}",
                    e
                )))
            }
        }
    }

    /// 测试 StarRocks 连接 (使用 MySQL 协议)
    pub async fn test_starrocks(req: &TestConnectionRequest) -> Result<ConnectionTestResult> {
        if req.db_type != DbType::StarRocks {
            return Err(AppError::InvalidInput(
                "Expected StarRocks connection type".to_string(),
            ));
        }

        tracing::info!("Testing StarRocks connection to {}:{}", req.host, req.port);
        // 使用 OptsBuilder 并禁用 socket 连接（StarRocks 不支持 @@socket 变量）
        let mut opts_builder = mysql_async::OptsBuilder::default()
            .ip_or_hostname(&req.host)
            .tcp_port(req.port)
            .user(Some(&req.username))
            .pass(Some(&req.password))
            .prefer_socket(false);  // 关键：禁用 socket，只使用 TCP

        if let Some(db) = &req.database_name {
            opts_builder = opts_builder.db_name(Some(db));
        }

        let opts = mysql_async::Opts::from(opts_builder);

        match mysql_async::Conn::new(opts).await {
            Ok(mut conn) => {
                tracing::debug!("StarRocks connection established, testing query...");
                // 测试简单查询
                match conn.query_drop("SELECT 1").await {
                    Ok(_) => {
                        let _ = conn.disconnect().await;
                        tracing::info!("StarRocks connection test successful");
                        Ok(ConnectionTestResult::success())
                    }
                    Err(e) => {
                        let _ = conn.disconnect().await;
                        tracing::error!("StarRocks query test failed: {}", e);
                        Ok(ConnectionTestResult::failure(format!(
                            "StarRocks query failed: {}",
                            e
                        )))
                    }
                }
            }
            Err(e) => {
                tracing::error!("StarRocks connection failed: {}", e);
                Ok(ConnectionTestResult::failure(format!(
                    "StarRocks connection failed: {}",
                    e
                )))
            }
        }
    }

    /// 构建 MySQL 连接选项（避免密码特殊字符问题）
    fn build_mysql_options(req: &TestConnectionRequest) -> MySqlConnectOptions {
        let mut opts = MySqlConnectOptions::new()
            .host(&req.host)
            .port(req.port)
            .username(&req.username)
            .password(&req.password);

        // StarRocks 使用 Disabled SSL，普通 MySQL 使用 Preferred
        if req.db_type == DbType::StarRocks {
            opts = opts.ssl_mode(MySqlSslMode::Disabled);
        } else {
            opts = opts.ssl_mode(MySqlSslMode::Preferred);
        }

        if let Some(db) = &req.database_name {
            opts = opts.database(db);
        }

        opts
    }

    /// 构建 PostgreSQL 连接选项（避免密码特殊字符问题）
    fn build_postgres_options(req: &TestConnectionRequest) -> PgConnectOptions {
        let database = req.database_name.as_deref().unwrap_or("dev");

        PgConnectOptions::new()
            .host(&req.host)
            .port(req.port)
            .username(&req.username)
            .password(&req.password)
            .database(database)
            .ssl_mode(PgSslMode::Prefer)
    }

    /// 从 DatabaseConfig 构建 MySQL 连接选项
    pub fn build_mysql_options_from_config(config: &DatabaseConfig) -> MySqlConnectOptions {
        let mut opts = MySqlConnectOptions::new()
            .host(&config.host)
            .port(config.port)
            .username(&config.username)
            .password(&config.password);

        // 根据数据库类型设置 SSL 模式和字符集
        if config.db_type == DbType::StarRocks {
            opts = opts
                .ssl_mode(MySqlSslMode::Disabled)
                .charset("utf8mb4")
                .collation("utf8mb4_general_ci");
        } else {
            opts = opts.ssl_mode(MySqlSslMode::Preferred);
        }

        if let Some(db) = &config.database_name {
            opts = opts.database(db);
        }

        opts
    }

    /// 从 DatabaseConfig 构建 PostgreSQL 连接选项
    pub fn build_postgres_options_from_config(config: &DatabaseConfig) -> PgConnectOptions {
        let database = config.database_name.as_deref().unwrap_or("dev");

        PgConnectOptions::new()
            .host(&config.host)
            .port(config.port)
            .username(&config.username)
            .password(&config.password)
            .database(database)
            .ssl_mode(PgSslMode::Prefer)
    }
}
