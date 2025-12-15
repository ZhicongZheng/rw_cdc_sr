use crate::models::{Column, DatabaseConfig, TableSchema};
use crate::services::ConnectionService;
use crate::utils::error::Result;
use sqlx::{MySqlPool, Row};

/// 元数据服务
pub struct MetadataService;

impl MetadataService {
    /// 获取 MySQL 数据库列表
    pub async fn list_mysql_databases(config: &DatabaseConfig) -> Result<Vec<String>> {
        tracing::info!("Listing MySQL databases for config: {}", config.name);
        let opts = ConnectionService::build_mysql_options_from_config(config);
        let pool = MySqlPool::connect_with(opts).await.map_err(|e| {
            tracing::error!("Failed to connect to MySQL: {}", e);
            e
        })?;

        let databases: Vec<String> = sqlx::query_scalar(
            "SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA
             WHERE SCHEMA_NAME NOT IN ('information_schema', 'mysql', 'performance_schema', 'sys')
             ORDER BY SCHEMA_NAME",
        )
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch database list: {}", e);
            e
        })?;

        tracing::info!("Found {} databases", databases.len());
        pool.close().await;

        Ok(databases)
    }

    /// 获取 MySQL 数据库的表列表
    pub async fn list_mysql_tables(config: &DatabaseConfig, database: &str) -> Result<Vec<String>> {
        tracing::info!("Listing tables in database: {}", database);
        let opts = ConnectionService::build_mysql_options_from_config(config);
        let pool = MySqlPool::connect_with(opts).await.map_err(|e| {
            tracing::error!("Failed to connect to MySQL: {}", e);
            e
        })?;

        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES
             WHERE TABLE_SCHEMA = ? AND TABLE_TYPE = 'BASE TABLE'
             ORDER BY TABLE_NAME",
        )
        .bind(database)
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to fetch table list from database {}: {}",
                database,
                e
            );
            e
        })?;

        tracing::info!("Found {} tables in database {}", tables.len(), database);
        pool.close().await;

        Ok(tables)
    }

    /// 获取 MySQL 表结构
    pub async fn get_mysql_table_schema(
        config: &DatabaseConfig,
        database: &str,
        table: &str,
    ) -> Result<TableSchema> {
        tracing::info!("Fetching schema for table: {}.{}", database, table);
        let opts = ConnectionService::build_mysql_options_from_config(config);
        let pool = MySqlPool::connect_with(opts).await.map_err(|e| {
            tracing::error!("Failed to connect to MySQL: {}", e);
            e
        })?;

        // 获取列信息
        let columns = Self::get_columns(&pool, database, table)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get columns for {}.{}: {}", database, table, e);
                e
            })?;

        // 获取主键信息
        let primary_keys = Self::get_primary_keys(&pool, database, table)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to get primary keys for {}.{}: {}",
                    database,
                    table,
                    e
                );
                e
            })?;

        // 获取索引信息
        // let indexes = Self::get_indexes(&pool, database, table)
        //     .await
        //     .map_err(|e| {
        //         tracing::error!("Failed to get indexes for {}.{}: {}", database, table, e);
        //         e
        //     })?;
        let indexes = vec![];

        tracing::info!(
            "Successfully fetched schema for {}.{}: {} columns, {} primary keys, {} indexes",
            database,
            table,
            columns.len(),
            primary_keys.len(),
            indexes.len()
        );

        pool.close().await;

        Ok(TableSchema {
            database: database.to_string(),
            table_name: table.to_string(),
            columns,
            primary_keys,
            indexes,
        })
    }

    /// 获取列信息
    async fn get_columns(pool: &MySqlPool, database: &str, table: &str) -> Result<Vec<Column>> {
        tracing::debug!("Fetching columns for {}.{}", database, table);
        let rows = sqlx::query(
            r#"
            SELECT
                COLUMN_NAME,
                DATA_TYPE,
                IS_NULLABLE,
                CHARACTER_MAXIMUM_LENGTH,
                NUMERIC_PRECISION,
                NUMERIC_SCALE,
                CAST(COLUMN_TYPE AS CHAR) AS COLUMN_TYPE
            FROM INFORMATION_SCHEMA.COLUMNS
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
            ORDER BY ORDINAL_POSITION
            "#,
        )
        .bind(database)
        .bind(table)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to query columns for {}.{}: {}", database, table, e);
            e
        })?;

        let mut columns = Vec::new();
        for row in rows {
            let column_type: String = row.try_get("COLUMN_TYPE").map_err(|e| {
                tracing::error!("Failed to decode COLUMN_TYPE: {}", e);
                e
            })?;

            let column_name: String = row.try_get("COLUMN_NAME")?;
            tracing::debug!("Processing column: {} (type: {})", column_name, column_type);

            columns.push(Column {
                name: column_name,
                data_type: column_type,
                is_nullable: row.try_get::<String, _>("IS_NULLABLE")? == "YES",
                default_value: Some(String::from("")),
                comment: Some(String::from("")), // INFORMATION_SCHEMA.COLUMNS 不包含注释字段
                character_maximum_length: row.try_get("CHARACTER_MAXIMUM_LENGTH").ok(),
                numeric_precision: row.try_get("NUMERIC_PRECISION").ok(),
                numeric_scale: row.try_get("NUMERIC_SCALE").ok(),
            });
        }

        tracing::debug!(
            "Fetched {} columns for {}.{}",
            columns.len(),
            database,
            table
        );
        Ok(columns)
    }

    /// 获取主键信息
    async fn get_primary_keys(
        pool: &MySqlPool,
        database: &str,
        table: &str,
    ) -> Result<Vec<String>> {
        let primary_keys: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT COLUMN_NAME
            FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? AND CONSTRAINT_NAME = 'PRIMARY'
            ORDER BY ORDINAL_POSITION
            "#,
        )
        .bind(database)
        .bind(table)
        .fetch_all(pool)
        .await?;

        Ok(primary_keys)
    }
}
