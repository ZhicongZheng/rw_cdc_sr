use axum::{extract::{Path, State}, Json};
use sqlx::MySqlPool;
use serde_json::json;

use crate::models::{SyncRequest, SyncTask};
use crate::services::SyncEngine;
use super::connection::AppError;

/// 同步单个表
pub async fn sync_single_table(
    State(pool): State<MySqlPool>,
    Json(request): Json<SyncRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engine = SyncEngine::new(pool);
    let task_id = engine.sync_table(request).await?;
    Ok(Json(json!({ "task_id": task_id })))
}

/// 同步多个表
pub async fn sync_multiple_tables(
    State(pool): State<MySqlPool>,
    Json(requests): Json<Vec<SyncRequest>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engine = SyncEngine::new(pool);
    let mut task_ids = Vec::new();

    for request in requests {
        let task_id = engine.sync_table(request).await?;
        task_ids.push(task_id);
    }

    Ok(Json(json!({ "task_ids": task_ids })))
}

/// 获取同步进度
pub async fn get_progress(
    State(pool): State<MySqlPool>,
    Path(id): Path<i64>,
) -> Result<Json<SyncTask>, AppError> {
    use crate::db::TaskRepository;
    let repo = TaskRepository::new(&pool);
    let task = repo.find_by_id(id).await?;
    Ok(Json(task))
}

/// 重试失败的任务
pub async fn retry_task(
    State(pool): State<MySqlPool>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    use crate::db::TaskRepository;
    let repo = TaskRepository::new(&pool);
    let task = repo.find_by_id(id).await?;

    // 创建新的同步请求
    let request = SyncRequest {
        mysql_config_id: task.mysql_config_id,
        rw_config_id: task.rw_config_id,
        sr_config_id: task.sr_config_id,
        mysql_database: task.mysql_database,
        mysql_table: task.mysql_table,
        target_database: task.target_database,
        target_table: task.target_table,
        options: serde_json::from_str(&task.options).unwrap_or_default(),
    };

    let engine = SyncEngine::new(pool);
    let new_task_id = engine.sync_table(request).await?;

    Ok(Json(json!({ "task_id": new_task_id })))
}
