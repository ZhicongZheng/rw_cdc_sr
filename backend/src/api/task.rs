use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use serde_json::json;
use sqlx::MySqlPool;

use super::connection::AppError;
use crate::db::TaskRepository;
use crate::models::{SyncTask, TaskLog, TaskStatus};

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// 获取任务历史
pub async fn get_history(
    State(pool): State<MySqlPool>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<Vec<SyncTask>>, AppError> {
    let repo = TaskRepository::new(&pool);

    // Map optional status string into an optional TaskStatus, discarding infallible errors
    let status = params.status.and_then(|s| TaskStatus::try_from(s).ok());
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    let tasks = repo.find_history(status, limit, offset).await?;
    Ok(Json(tasks))
}

/// 获取任务详情
pub async fn get_detail(
    State(pool): State<MySqlPool>,
    Path(id): Path<i64>,
) -> Result<Json<SyncTask>, AppError> {
    let repo = TaskRepository::new(&pool);
    let task = repo.find_by_id(id).await?;
    Ok(Json(task))
}

/// 获取任务日志
pub async fn get_logs(
    State(pool): State<MySqlPool>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<TaskLog>>, AppError> {
    let repo = TaskRepository::new(&pool);
    let logs = repo.get_logs(id).await?;
    Ok(Json(logs))
}

/// 取消任务
pub async fn cancel_task(
    State(pool): State<MySqlPool>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: 实现任务取消逻辑
    // 目前只是将状态标记为 Failed
    let repo = TaskRepository::new(&pool);
    repo.update_status(
        id,
        TaskStatus::Failed,
        Some("Cancelled by user".to_string()),
    )
    .await?;

    Ok(Json(json!({ "success": true })))
}
