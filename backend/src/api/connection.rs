use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use sqlx::MySqlPool;

use crate::db::ConfigRepository;
use crate::models::{
    ConnectionTestResult, CreateConnectionRequest, DatabaseConfig, TestConnectionRequest,
};
use crate::services::ConnectionService;

/// 测试 MySQL 连接
pub async fn test_mysql(
    Json(request): Json<TestConnectionRequest>,
) -> Result<Json<ConnectionTestResult>, AppError> {
    let result = ConnectionService::test_mysql(&request).await?;
    Ok(Json(result))
}

/// 测试 RisingWave 连接
pub async fn test_risingwave(
    Json(request): Json<TestConnectionRequest>,
) -> Result<Json<ConnectionTestResult>, AppError> {
    let result = ConnectionService::test_risingwave(&request).await?;
    Ok(Json(result))
}

/// 测试 StarRocks 连接
pub async fn test_starrocks(
    Json(request): Json<TestConnectionRequest>,
) -> Result<Json<ConnectionTestResult>, AppError> {
    let result = ConnectionService::test_starrocks(&request).await?;
    Ok(Json(result))
}

/// 保存连接配置
pub async fn save_connection(
    State(pool): State<MySqlPool>,
    Json(request): Json<CreateConnectionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = ConfigRepository::new(&pool);
    let id = repo.save(request).await?;
    Ok(Json(json!({ "id": id })))
}

/// 获取所有连接配置
pub async fn list_connections(
    State(pool): State<MySqlPool>,
) -> Result<Json<Vec<DatabaseConfig>>, AppError> {
    let repo = ConfigRepository::new(&pool);
    let configs = repo.find_all().await?;
    Ok(Json(configs))
}

/// 更新连接配置
pub async fn update_connection(
    State(pool): State<MySqlPool>,
    Path(id): Path<i64>,
    Json(request): Json<CreateConnectionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = ConfigRepository::new(&pool);
    repo.update(id, request).await?;
    Ok(Json(json!({ "success": true })))
}

/// 删除连接配置
pub async fn delete_connection(
    State(pool): State<MySqlPool>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let repo = ConfigRepository::new(&pool);
    repo.delete(id).await?;
    Ok(Json(json!({ "success": true })))
}

/// Axum 错误处理
pub struct AppError(crate::utils::error::AppError);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string());

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<crate::utils::error::AppError>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
