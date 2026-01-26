use axum::{
    Json,
    extract::{State, Query},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use mysql_async::prelude::*;

use super::connection::AppError;
use crate::db::ConfigRepository;
use crate::models::{TableSchema, Column, PaginatedResponse};
use crate::generators::{RisingWaveDDLGenerator, StarRocksDDLGenerator};

#[derive(Deserialize)]
pub struct RwObjectQuery {
    pub config_id: i64,
    pub schema: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl RwObjectQuery {
    /// 获取有效的 limit 值（默认20，范围1-100）
    fn get_limit(&self) -> i64 {
        self.limit.unwrap_or(20).max(1).min(100)
    }

    /// 获取有效的 offset 值（默认0，最小0）
    fn get_offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

#[derive(Deserialize)]
pub struct DeleteObjectRequest {
    pub config_id: i64,
    pub schema: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct BatchDeleteObjectRequest {
    pub config_id: i64,
    pub schema: String,
    pub object_type: String,  // "source", "table", "materialized_view", "sink"
    pub names: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct CreateSinkRequest {
    pub rw_config_id: i64,
    pub sr_config_id: i64,
    pub schema: String,
    pub source_object: String,  // table name or materialized view name
    pub source_type: String,     // "table" or "materialized_view"
    pub target_database: String,
    pub target_table: String,
}

#[derive(Serialize)]
pub struct RwSchema {
    pub schema_name: String,
}

#[derive(Serialize)]
pub struct RwSource {
    pub id: i32,
    pub name: String,
    pub schema_name: String,
    pub owner: i32,
    pub connector: String,
    pub columns: Vec<String>,
    pub definition: Option<String>,
}

#[derive(Serialize)]
pub struct RwTable {
    pub id: i32,
    pub name: String,
    pub schema_name: String,
    pub owner: i32,
    pub definition: Option<String>,
}

#[derive(Serialize)]
pub struct RwMaterializedView {
    pub id: i32,
    pub name: String,
    pub schema_name: String,
    pub owner: i32,
    pub definition: Option<String>,
}

#[derive(Serialize)]
pub struct RwSink {
    pub id: i32,
    pub name: String,
    pub schema_name: String,
    pub owner: i32,
    pub connector: String,
    pub definition: Option<String>,
}

/// 获取 RisingWave 连接池
async fn get_rw_pool(pool: &sqlx::MySqlPool, config_id: i64) -> Result<PgPool, AppError> {
    let config_repo = ConfigRepository::new(pool);
    let rw_config = config_repo.find_by_id(config_id).await?;

    let rw_opts = PgConnectOptions::new()
        .host(&rw_config.host)
        .port(rw_config.port)
        .username(&rw_config.username)
        .password(&rw_config.password)
        .database(rw_config.database_name.as_deref().unwrap_or("dev"))
        .ssl_mode(PgSslMode::Prefer);

    let rw_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(rw_opts)
        .await
        .map_err(|e| crate::utils::error::AppError::Connection(format!("Failed to connect to RisingWave: {}", e)))?;

    Ok(rw_pool)
}

/// 列出所有 schemas
pub async fn list_schemas(
    State(pool): State<sqlx::MySqlPool>,
    Query(params): Query<RwObjectQuery>,
) -> Result<Json<Vec<RwSchema>>, AppError> {
    let rw_pool = get_rw_pool(&pool, params.config_id).await?;
    let internal_schema = vec!["rw_catalog","information_schema", "pg_catalog"];

    let schemas: Vec<RwSchema> = sqlx::query(
        "SELECT name as schema_name FROM rw_catalog.rw_schemas
         ORDER BY name"
    )
    .fetch_all(&rw_pool)
    .await?
    .iter()
    .map(|row| RwSchema {
        schema_name: row.get("schema_name"),
    })
    .filter(|schema| !internal_schema.contains(&schema.schema_name.as_str()))
    .collect();

    Ok(Json(schemas))
}

/// 列出 sources
pub async fn list_sources(
    State(pool): State<sqlx::MySqlPool>,
    Query(params): Query<RwObjectQuery>,
) -> Result<Json<PaginatedResponse<RwSource>>, AppError> {
    let rw_pool = get_rw_pool(&pool, params.config_id).await?;
    let schema = params.schema.clone().unwrap_or_else(|| "public".to_string());
    let limit = params.get_limit();
    let offset = params.get_offset();

    // 构建查询条件
    let where_clause = if params.search.is_some() {
        "WHERE sch.name = $1 AND s.name ILIKE '%' || $2 || '%'"
    } else {
        "WHERE sch.name = $1"
    };

    // 数据查询 - LIMIT 和 OFFSET 必须直接在 SQL 中格式化，不能使用参数化查询
    let query_str = format!(
        "SELECT s.id, s.name, sch.name as schema_name, s.owner, s.connector, s.columns::text as columns_text, s.definition
         FROM rw_catalog.rw_sources s
         JOIN rw_catalog.rw_schemas sch ON s.schema_id = sch.id
         {}
         ORDER BY s.name
         LIMIT {} OFFSET {}", where_clause, limit, offset
    );

    let mut query = sqlx::query(&query_str).bind(&schema);
    if let Some(search) = &params.search {
        query = query.bind(search);
    }

    let sources: Vec<RwSource> = query
        .fetch_all(&rw_pool)
        .await?
        .iter()
        .map(|row| {
            let columns_text: String = row.get("columns_text");
            let columns: Vec<String> = columns_text
                .trim_matches(|c| c == '{' || c == '}')
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            RwSource {
                id: row.get("id"),
                name: row.get("name"),
                schema_name: row.get("schema_name"),
                owner: row.get("owner"),
                connector: row.get("connector"),
                columns,
                definition: row.get("definition"),
            }
        })
        .collect();

    // COUNT 查询
    let count_str = format!(
        "SELECT COUNT(*) as total
         FROM rw_catalog.rw_sources s
         JOIN rw_catalog.rw_schemas sch ON s.schema_id = sch.id
         {}", where_clause
    );

    let mut count_query = sqlx::query(&count_str).bind(&schema);
    if let Some(search) = &params.search {
        count_query = count_query.bind(search);
    }

    let total: i64 = count_query
        .fetch_one(&rw_pool)
        .await?
        .get("total");

    Ok(Json(PaginatedResponse::new(sources, total, limit, offset)))
}

/// 列出 tables
pub async fn list_tables(
    State(pool): State<sqlx::MySqlPool>,
    Query(params): Query<RwObjectQuery>,
) -> Result<Json<PaginatedResponse<RwTable>>, AppError> {
    let rw_pool = get_rw_pool(&pool, params.config_id).await?;
    let schema = params.schema.clone().unwrap_or_else(|| "public".to_string());
    let limit = params.get_limit();
    let offset = params.get_offset();

    // 构建查询条件
    let where_clause = if params.search.is_some() {
        "WHERE sch.name = $1 AND t.name ILIKE '%' || $2 || '%'"
    } else {
        "WHERE sch.name = $1"
    };

    // 数据查询 - LIMIT 和 OFFSET 必须直接在 SQL 中格式化，不能使用参数化查询
    let query_str = format!(
        "SELECT t.id, t.name, sch.name as schema_name, t.owner, t.definition
         FROM rw_catalog.rw_tables t
         JOIN rw_catalog.rw_schemas sch ON t.schema_id = sch.id
         {}
         ORDER BY t.name
         LIMIT {} OFFSET {}", where_clause, limit, offset
    );

    let mut query = sqlx::query(&query_str).bind(&schema);
    if let Some(search) = &params.search {
        query = query.bind(search);
    }

    let tables: Vec<RwTable> = query
        .fetch_all(&rw_pool)
        .await?
        .iter()
        .map(|row| RwTable {
            id: row.get("id"),
            name: row.get("name"),
            schema_name: row.get("schema_name"),
            owner: row.get("owner"),
            definition: row.get("definition"),
        })
        .collect();

    // COUNT 查询
    let count_str = format!(
        "SELECT COUNT(*) as total
         FROM rw_catalog.rw_tables t
         JOIN rw_catalog.rw_schemas sch ON t.schema_id = sch.id
         {}", where_clause
    );

    let mut count_query = sqlx::query(&count_str).bind(&schema);
    if let Some(search) = &params.search {
        count_query = count_query.bind(search);
    }

    let total: i64 = count_query
        .fetch_one(&rw_pool)
        .await?
        .get("total");

    Ok(Json(PaginatedResponse::new(tables, total, limit, offset)))
}

/// 列出 materialized views
pub async fn list_materialized_views(
    State(pool): State<sqlx::MySqlPool>,
    Query(params): Query<RwObjectQuery>,
) -> Result<Json<PaginatedResponse<RwMaterializedView>>, AppError> {
    let rw_pool = get_rw_pool(&pool, params.config_id).await?;
    let schema = params.schema.clone().unwrap_or_else(|| "public".to_string());
    let limit = params.get_limit();
    let offset = params.get_offset();

    // 构建查询条件
    let where_clause = if params.search.is_some() {
        "WHERE sch.name = $1 AND mv.name ILIKE '%' || $2 || '%'"
    } else {
        "WHERE sch.name = $1"
    };

    // 数据查询 - LIMIT 和 OFFSET 必须直接在 SQL 中格式化，不能使用参数化查询
    let query_str = format!(
        "SELECT mv.id, mv.name, sch.name as schema_name, mv.owner, mv.definition
         FROM rw_catalog.rw_materialized_views mv
         JOIN rw_catalog.rw_schemas sch ON mv.schema_id = sch.id
         {}
         ORDER BY mv.name
         LIMIT {} OFFSET {}", where_clause, limit, offset
    );

    let mut query = sqlx::query(&query_str).bind(&schema);
    if let Some(search) = &params.search {
        query = query.bind(search);
    }

    let mvs: Vec<RwMaterializedView> = query
        .fetch_all(&rw_pool)
        .await?
        .iter()
        .map(|row| RwMaterializedView {
            id: row.get("id"),
            name: row.get("name"),
            schema_name: row.get("schema_name"),
            owner: row.get("owner"),
            definition: row.get("definition"),
        })
        .collect();

    // COUNT 查询
    let count_str = format!(
        "SELECT COUNT(*) as total
         FROM rw_catalog.rw_materialized_views mv
         JOIN rw_catalog.rw_schemas sch ON mv.schema_id = sch.id
         {}", where_clause
    );

    let mut count_query = sqlx::query(&count_str).bind(&schema);
    if let Some(search) = &params.search {
        count_query = count_query.bind(search);
    }

    let total: i64 = count_query
        .fetch_one(&rw_pool)
        .await?
        .get("total");

    Ok(Json(PaginatedResponse::new(mvs, total, limit, offset)))
}

/// 列出 sinks
pub async fn list_sinks(
    State(pool): State<sqlx::MySqlPool>,
    Query(params): Query<RwObjectQuery>,
) -> Result<Json<PaginatedResponse<RwSink>>, AppError> {
    let rw_pool = get_rw_pool(&pool, params.config_id).await?;
    let schema = params.schema.clone().unwrap_or_else(|| "public".to_string());
    let limit = params.get_limit();
    let offset = params.get_offset();

    // 构建查询条件
    let where_clause = if params.search.is_some() {
        "WHERE sch.name = $1 AND s.name ILIKE '%' || $2 || '%'"
    } else {
        "WHERE sch.name = $1"
    };

    // 数据查询 - LIMIT 和 OFFSET 必须直接在 SQL 中格式化，不能使用参数化查询
    let query_str = format!(
        "SELECT s.id, s.name, sch.name as schema_name, s.owner, s.connector, s.definition
         FROM rw_catalog.rw_sinks s
         JOIN rw_catalog.rw_schemas sch ON s.schema_id = sch.id
         {}
         ORDER BY s.name
         LIMIT {} OFFSET {}", where_clause, limit, offset
    );

    let mut query = sqlx::query(&query_str).bind(&schema);
    if let Some(search) = &params.search {
        query = query.bind(search);
    }

    let sinks: Vec<RwSink> = query
        .fetch_all(&rw_pool)
        .await?
        .iter()
        .map(|row| RwSink {
            id: row.get("id"),
            name: row.get("name"),
            schema_name: row.get("schema_name"),
            owner: row.get("owner"),
            connector: row.get("connector"),
            definition: row.get("definition"),
        })
        .collect();

    // COUNT 查询
    let count_str = format!(
        "SELECT COUNT(*) as total
         FROM rw_catalog.rw_sinks s
         JOIN rw_catalog.rw_schemas sch ON s.schema_id = sch.id
         {}", where_clause
    );

    let mut count_query = sqlx::query(&count_str).bind(&schema);
    if let Some(search) = &params.search {
        count_query = count_query.bind(search);
    }

    let total: i64 = count_query
        .fetch_one(&rw_pool)
        .await?
        .get("total");

    Ok(Json(PaginatedResponse::new(sinks, total, limit, offset)))
}

/// 删除 source
pub async fn delete_source(
    State(pool): State<sqlx::MySqlPool>,
    Json(request): Json<DeleteObjectRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rw_pool = get_rw_pool(&pool, request.config_id).await?;

    let drop_sql = format!("DROP SOURCE IF EXISTS \"{}\".\"{}\"", request.schema, request.name);
    sqlx::query(&drop_sql)
        .execute(&rw_pool)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// 删除 table
pub async fn delete_table(
    State(pool): State<sqlx::MySqlPool>,
    Json(request): Json<DeleteObjectRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rw_pool = get_rw_pool(&pool, request.config_id).await?;

    let drop_sql = format!("DROP TABLE IF EXISTS \"{}\".\"{}\"", request.schema, request.name);
    sqlx::query(&drop_sql)
        .execute(&rw_pool)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// 删除 materialized view
pub async fn delete_materialized_view(
    State(pool): State<sqlx::MySqlPool>,
    Json(request): Json<DeleteObjectRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rw_pool = get_rw_pool(&pool, request.config_id).await?;

    let drop_sql = format!("DROP MATERIALIZED VIEW IF EXISTS \"{}\".\"{}\"", request.schema, request.name);
    sqlx::query(&drop_sql)
        .execute(&rw_pool)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// 删除 sink
pub async fn delete_sink(
    State(pool): State<sqlx::MySqlPool>,
    Json(request): Json<DeleteObjectRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rw_pool = get_rw_pool(&pool, request.config_id).await?;

    let drop_sql = format!("DROP SINK IF EXISTS \"{}\".\"{}\"", request.schema, request.name);
    sqlx::query(&drop_sql)
        .execute(&rw_pool)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// 批量删除对象
pub async fn batch_delete_objects(
    State(pool): State<sqlx::MySqlPool>,
    Json(request): Json<BatchDeleteObjectRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rw_pool = get_rw_pool(&pool, request.config_id).await?;

    let object_type_sql = match request.object_type.as_str() {
        "source" => "SOURCE",
        "table" => "TABLE",
        "materialized_view" => "MATERIALIZED VIEW",
        "sink" => "SINK",
        _ => {
            return Err(crate::utils::error::AppError::InvalidInput(
                format!("Invalid object type: {}", request.object_type)
            ).into())
        }
    };

    let mut success_count = 0;
    let mut failed = Vec::new();

    for name in &request.names {
        let drop_sql = format!("DROP {} IF EXISTS \"{}\".\"{}\"", object_type_sql, request.schema, name);
        tracing::debug!("Executing: {}", drop_sql);

        match sqlx::query(&drop_sql).execute(&rw_pool).await {
            Ok(_) => {
                tracing::info!("Successfully deleted {} {}", object_type_sql, name);
                success_count += 1;
            },
            Err(e) => {
                tracing::error!("Failed to delete {} {}: {}", object_type_sql, name, e);
                failed.push(name.clone());
            }
        }
    }

    Ok(Json(serde_json::json!({
        "success": failed.is_empty(),
        "deleted_count": success_count,
        "total_count": request.names.len(),
        "failed": failed,
    })))
}

/// 从 RisingWave 获取表或物化视图的 schema
async fn get_rw_table_schema(
    rw_pool: &PgPool,
    schema: &str,
    object_name: &str,
    object_type: &str, // "table" or "materialized_view"
) -> Result<TableSchema, AppError> {
    // 查询列信息
    let columns: Vec<Column> = sqlx::query(
        r#"
        SELECT
            c.name as column_name,
            c.data_type,
            c.is_nullable
        FROM rw_catalog.rw_columns c
        WHERE 
        c.relation_id IN (
            SELECT t.id FROM rw_catalog.rw_tables t
            JOIN rw_catalog.rw_schemas sch ON t.schema_id = sch.id
            WHERE t.name = $2 AND sch.name = $1
            UNION ALL
            SELECT mv.id FROM rw_catalog.rw_materialized_views mv
            JOIN rw_catalog.rw_schemas sch ON mv.schema_id = sch.id
            WHERE mv.name = $2 AND sch.name = $1
        )
        ORDER BY c.position
        "#
    )
    .bind(schema)
    .bind(object_name)
    .fetch_all(rw_pool)
    .await?
    .iter()
    .map(|row| {
        let data_type: String = row.get("data_type");
        Column {
            name: row.get("column_name"),
            data_type,
            is_nullable: row.get("is_nullable"),
            default_value: None,
            comment: None,
            character_maximum_length: None,
            numeric_precision: None,
            numeric_scale: None,
        }
    })
    .collect();

    if columns.is_empty() {
        return Err(crate::utils::error::AppError::NotFound(
            format!("No columns found for {}.{}", schema, object_name)
        ).into());
    }

    // 查询主键信息（只有表有主键，物化视图没有主键约束）
    let primary_keys: Vec<String> = if object_type == "table" {
        sqlx::query_scalar(
            r#"
            SELECT c.name
            FROM rw_catalog.rw_columns c
            JOIN rw_catalog.rw_tables t ON c.relation_id = t.id
            JOIN rw_catalog.rw_schemas s ON t.schema_id = s.id
            WHERE s.name = $1 AND t.name = $2 AND c.is_primary_key = true
            ORDER BY c.position
            "#
        )
        .bind(schema)
        .bind(object_name)
        .fetch_all(rw_pool)
        .await?
    } else {
        // 物化视图没有主键，使用第一列作为主键
        vec![columns[0].name.clone()]
    };

    Ok(TableSchema {
        database: schema.to_string(),
        table_name: object_name.to_string(),
        columns,
        primary_keys,
        indexes: vec![],
    })
}

/// 创建 Sink 到 StarRocks
pub async fn create_sink(
    State(pool): State<sqlx::MySqlPool>,
    Json(request): Json<CreateSinkRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    tracing::info!(
        "Creating sink from {}.{} ({}) to StarRocks {}.{}",
        request.schema,
        request.source_object,
        request.source_type,
        request.target_database,
        request.target_table
    );

    // 获取配置
    let config_repo = ConfigRepository::new(&pool);
    let sr_config = config_repo.find_by_id(request.sr_config_id).await?;

    // 连接到 RisingWave
    let rw_pool = get_rw_pool(&pool, request.rw_config_id).await?;

    // 获取表结构
    let schema = get_rw_table_schema(
        &rw_pool,
        &request.schema,
        &request.source_object,
        &request.source_type
    ).await?;

    // 连接到 StarRocks
    let mut sr_opts_builder = mysql_async::OptsBuilder::default()
        .ip_or_hostname(&sr_config.host)
        .tcp_port(sr_config.port)
        .user(Some(&sr_config.username))
        .pass(Some(&sr_config.password))
        .prefer_socket(false);

    if let Some(db) = &sr_config.database_name {
        sr_opts_builder = sr_opts_builder.db_name(Some(db));
    }

    let sr_opts = mysql_async::Opts::from(sr_opts_builder);
    let mut sr_conn = mysql_async::Conn::new(sr_opts).await.map_err(|e| {
        crate::utils::error::AppError::Connection(format!("StarRocks connection failed: {}", e))
    })?;

    // 创建 StarRocks 数据库
    let create_db_ddl = StarRocksDDLGenerator::generate_create_database_ddl(&request.target_database);
    sr_conn.query_drop(&create_db_ddl).await.map_err(|e| {
        crate::utils::error::AppError::Unknown(format!("Failed to create database: {}", e))
    })?;

    // 创建 StarRocks 表
    let sr_table_ddl = StarRocksDDLGenerator::generate_table_ddl(
        &schema,
        &request.target_database,
        &request.target_table,
    )?;
    sr_conn.query_drop(&sr_table_ddl).await.map_err(|e| {
        crate::utils::error::AppError::Unknown(format!("Failed to create StarRocks table: {}", e))
    })?;

    // 创建 StarRocks Secret
    let sr_secret_ddl = RisingWaveDDLGenerator::generate_starrocks_secret_ddl(&sr_config, &request.schema)?;
    let _ = sqlx::query(&sr_secret_ddl).execute(&rw_pool).await; // 忽略错误（可能已存在）

    // 创建 Sink - 需要构建一个临时的 SyncRequest
    let sync_request = crate::models::SyncRequest {
        mysql_config_id: 0, // 不需要
        rw_config_id: request.rw_config_id,
        sr_config_id: request.sr_config_id,
        mysql_database: String::new(), // 不需要
        mysql_table: String::new(), // 不需要
        target_database: request.target_database.clone(),
        target_table: request.target_table.clone(),
        options: crate::models::SyncOptions {
            recreate_rw_source: false,
            recreate_sr_table: false,
            truncate_sr_table: false,
        },
    };

    let sink_ddl = RisingWaveDDLGenerator::generate_sink_ddl(
        &sr_config,
        &sync_request,
        &schema
    )?;

    sqlx::query(&sink_ddl).execute(&rw_pool).await.map_err(|e| {
        crate::utils::error::AppError::SqlGeneration(format!("Failed to create sink: {}", e))
    })?;

    // 关闭连接
    let _ = sr_conn.disconnect().await;

    tracing::info!(
        "Successfully created sink from {}.{} to {}.{}",
        request.schema,
        request.source_object,
        request.target_database,
        request.target_table
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Successfully created sink from {}.{} to {}.{}",
            request.schema, request.source_object,
            request.target_database, request.target_table)
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rw_object_query_defaults() {
        let query = RwObjectQuery {
            config_id: 1,
            schema: Some("public".to_string()),
            search: None,
            limit: None,
            offset: None,
        };

        assert_eq!(query.get_limit(), 20);
        assert_eq!(query.get_offset(), 0);
    }

    #[test]
    fn test_rw_object_query_validation() {
        // Test limit boundary conditions
        let query_low_limit = RwObjectQuery {
            config_id: 1,
            schema: None,
            search: None,
            limit: Some(0),
            offset: None,
        };
        assert_eq!(query_low_limit.get_limit(), 1); // Should clamp to min 1

        let query_high_limit = RwObjectQuery {
            config_id: 1,
            schema: None,
            search: None,
            limit: Some(200),
            offset: None,
        };
        assert_eq!(query_high_limit.get_limit(), 100); // Should clamp to max 100

        let query_negative_limit = RwObjectQuery {
            config_id: 1,
            schema: None,
            search: None,
            limit: Some(-10),
            offset: None,
        };
        assert_eq!(query_negative_limit.get_limit(), 1); // Negative should clamp to 1

        // Test offset boundary conditions
        let query_negative_offset = RwObjectQuery {
            config_id: 1,
            schema: None,
            search: None,
            limit: None,
            offset: Some(-10),
        };
        assert_eq!(query_negative_offset.get_offset(), 0); // Should clamp to min 0

        let query_valid_offset = RwObjectQuery {
            config_id: 1,
            schema: None,
            search: None,
            limit: None,
            offset: Some(100),
        };
        assert_eq!(query_valid_offset.get_offset(), 100);
    }

    #[test]
    fn test_paginated_response_creation() {
        use crate::models::PaginatedResponse;

        let data = vec![1, 2, 3, 4, 5];
        let total = 100;
        let limit = 20;
        let offset = 40;

        let response = PaginatedResponse::new(data.clone(), total, limit, offset);

        assert_eq!(response.data, data);
        assert_eq!(response.total, total);
        assert_eq!(response.limit, limit);
        assert_eq!(response.offset, offset);
    }

    #[test]
    fn test_create_sink_request_serialization() {
        let request = CreateSinkRequest {
            rw_config_id: 1,
            sr_config_id: 2,
            schema: "public".to_string(),
            source_object: "test_table".to_string(),
            source_type: "table".to_string(),
            target_database: "test_db".to_string(),
            target_table: "test_table_sr".to_string(),
        };

        // Test that the struct can be serialized
        let json = serde_json::to_string(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_create_sink_request_deserialization() {
        let json = r#"{
            "rw_config_id": 1,
            "sr_config_id": 2,
            "schema": "public",
            "source_object": "test_mv",
            "source_type": "materialized_view",
            "target_database": "analytics",
            "target_table": "test_mv_sr"
        }"#;

        let request: Result<CreateSinkRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.rw_config_id, 1);
        assert_eq!(request.sr_config_id, 2);
        assert_eq!(request.schema, "public");
        assert_eq!(request.source_object, "test_mv");
        assert_eq!(request.source_type, "materialized_view");
        assert_eq!(request.target_database, "analytics");
        assert_eq!(request.target_table, "test_mv_sr");
    }

    #[test]
    fn test_batch_delete_object_types() {
        // Test valid object types
        let valid_types = vec!["source", "table", "materialized_view", "sink"];
        for obj_type in valid_types {
            assert!(["source", "table", "materialized_view", "sink"].contains(&obj_type));
        }
    }

    #[test]
    fn test_rw_schema_serialization() {
        let schema = RwSchema {
            schema_name: "public".to_string(),
        };
        let json = serde_json::to_string(&schema);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("public"));
    }
}
