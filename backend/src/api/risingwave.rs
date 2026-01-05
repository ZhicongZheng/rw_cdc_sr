use axum::{
    Json,
    extract::{State, Query},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};

use super::connection::AppError;
use crate::db::ConfigRepository;

#[derive(Deserialize)]
pub struct RwObjectQuery {
    pub config_id: i64,
    pub schema: Option<String>,
}

#[derive(Deserialize)]
pub struct DeleteObjectRequest {
    pub config_id: i64,
    pub schema: String,
    pub name: String,
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
    pub connector: String
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
) -> Result<Json<Vec<RwSource>>, AppError> {
    let rw_pool = get_rw_pool(&pool, params.config_id).await?;
    let schema = params.schema.unwrap_or_else(|| "public".to_string());

    let sources: Vec<RwSource> = sqlx::query(
        "SELECT s.id, s.name, sch.name as schema_name, s.owner, s.connector, s.columns::text as columns_text
         FROM rw_catalog.rw_sources s
         JOIN rw_catalog.rw_schemas sch ON s.schema_id = sch.id
         WHERE sch.name = $1
         ORDER BY s.name"
    )
    .bind(&schema)
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
        }
    })
    .collect();

    Ok(Json(sources))
}

/// 列出 tables
pub async fn list_tables(
    State(pool): State<sqlx::MySqlPool>,
    Query(params): Query<RwObjectQuery>,
) -> Result<Json<Vec<RwTable>>, AppError> {
    let rw_pool = get_rw_pool(&pool, params.config_id).await?;
    let schema = params.schema.unwrap_or_else(|| "public".to_string());

    let tables: Vec<RwTable> = sqlx::query(
        "SELECT t.id, t.name, sch.name as schema_name, t.owner, t.definition
         FROM rw_catalog.rw_tables t
         JOIN rw_catalog.rw_schemas sch ON t.schema_id = sch.id
         WHERE sch.name = $1
         ORDER BY t.name"
    )
    .bind(&schema)
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

    Ok(Json(tables))
}

/// 列出 materialized views
pub async fn list_materialized_views(
    State(pool): State<sqlx::MySqlPool>,
    Query(params): Query<RwObjectQuery>,
) -> Result<Json<Vec<RwMaterializedView>>, AppError> {
    let rw_pool = get_rw_pool(&pool, params.config_id).await?;
    let schema = params.schema.unwrap_or_else(|| "public".to_string());

    let mvs: Vec<RwMaterializedView> = sqlx::query(
        "SELECT mv.id, mv.name, sch.name as schema_name, mv.owner, mv.definition
         FROM rw_catalog.rw_materialized_views mv
         JOIN rw_catalog.rw_schemas sch ON mv.schema_id = sch.id
         WHERE sch.name = $1
         ORDER BY mv.name"
    )
    .bind(&schema)
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

    Ok(Json(mvs))
}

/// 列出 sinks
pub async fn list_sinks(
    State(pool): State<sqlx::MySqlPool>,
    Query(params): Query<RwObjectQuery>,
) -> Result<Json<Vec<RwSink>>, AppError> {
    let rw_pool = get_rw_pool(&pool, params.config_id).await?;
    let schema = params.schema.unwrap_or_else(|| "public".to_string());

    let sinks: Vec<RwSink> = sqlx::query(
        "SELECT s.id, s.name, sch.name as schema_name, s.owner, s.connector
         FROM rw_catalog.rw_sinks s
         JOIN rw_catalog.rw_schemas sch ON s.schema_id = sch.id
         WHERE sch.name = $1
         ORDER BY s.name"
    )
    .bind(&schema)
    .fetch_all(&rw_pool)
    .await?
    .iter()
    .map(|row| RwSink {
        id: row.get("id"),
        name: row.get("name"),
        schema_name: row.get("schema_name"),
        owner: row.get("owner"),
        connector: row.get("connector"),
        
    })
    .collect();

    Ok(Json(sinks))
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
