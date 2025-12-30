use axum::{extract::State, Json};
use sqlx::MySqlPool;

use crate::db::ConfigRepository;
use crate::models::TableSchema;
use crate::services::MetadataService;
use serde::{Deserialize};

use super::connection::AppError;

#[derive(Deserialize)]
pub struct ListDatabasesRequest {
    pub config_id: i64,
}

#[derive(Deserialize)]
pub struct ListTablesRequest {
    pub config_id: i64,
    pub database: String,
}

#[derive(Deserialize)]
pub struct GetSchemaRequest {
    pub config_id: i64,
    pub database: String,
    pub table: String,
}

/// 获取 MySQL 数据库列表
pub async fn list_databases(
    State(pool): State<MySqlPool>,
    Json(request): Json<ListDatabasesRequest>,
) -> Result<Json<Vec<String>>, AppError> {
    let repo = ConfigRepository::new(&pool);
    let config = repo.find_by_id(request.config_id).await?;

    let databases = MetadataService::list_mysql_databases(&config).await?;
    Ok(Json(databases))
}

/// 获取数据库表列表
pub async fn list_tables(
    State(pool): State<MySqlPool>,
    Json(request): Json<ListTablesRequest>,
) -> Result<Json<Vec<String>>, AppError> {
    let repo = ConfigRepository::new(&pool);
    let config = repo.find_by_id(request.config_id).await?;

    let tables = MetadataService::list_mysql_tables(&config, &request.database).await?;
    Ok(Json(tables))
}

/// 获取表结构
pub async fn get_schema(
    State(pool): State<MySqlPool>,
    Json(request): Json<GetSchemaRequest>,
) -> Result<Json<TableSchema>, AppError> {
    let repo = ConfigRepository::new(&pool);
    let config = repo.find_by_id(request.config_id).await?;

    let schema = MetadataService::get_mysql_table_schema(
        &config,
        &request.database,
        &request.table,
    ).await?;

    Ok(Json(schema))
}
