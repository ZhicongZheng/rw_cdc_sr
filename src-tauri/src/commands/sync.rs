use crate::models::{BatchSyncRequest, SyncOptions, SyncProgress, SyncRequest};
use crate::services::SyncEngine;
use sqlx::SqlitePool;
use tauri::State;

#[tauri::command]
pub async fn sync_single_table(
    db: State<'_, SqlitePool>,
    request: SyncRequest,
) -> std::result::Result<i64, String> {
    let engine = SyncEngine::new(db.inner().clone());
    engine.sync_table(request).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_multiple_tables(
    db: State<'_, SqlitePool>,
    request: BatchSyncRequest,
) -> std::result::Result<Vec<i64>, String> {
    let engine = SyncEngine::new(db.inner().clone());
    let mut task_ids = Vec::new();

    for table_info in request.tables {
        let sync_request = SyncRequest {
            mysql_config_id: request.mysql_config_id,
            rw_config_id: request.rw_config_id,
            sr_config_id: request.sr_config_id,
            mysql_database: table_info.mysql_database,
            mysql_table: table_info.mysql_table,
            target_database: table_info.target_database,
            target_table: table_info.target_table,
            options: request.options.clone(),
        };

        let task_id = engine
            .sync_table(sync_request)
            .await
            .map_err(|e| e.to_string())?;
        task_ids.push(task_id);
    }

    Ok(task_ids)
}

#[tauri::command]
pub async fn retry_sync_task(
    db: State<'_, SqlitePool>,
    task_id: i64,
) -> std::result::Result<i64, String> {
    use crate::db::TaskRepository;

    let repo = TaskRepository::new(&db);
    let task = repo.find_by_id(task_id).await.map_err(|e| e.to_string())?;

    // 解析 options JSON
    let options: SyncOptions = serde_json::from_str(&task.options)
        .map_err(|e| format!("Failed to parse task options: {}", e))?;

    // 创建新的同步请求
    let request = SyncRequest {
        mysql_config_id: task.mysql_config_id,
        rw_config_id: task.rw_config_id,
        sr_config_id: task.sr_config_id,
        mysql_database: task.mysql_database,
        mysql_table: task.mysql_table,
        target_database: task.target_database,
        target_table: task.target_table,
        options,
    };

    // 执行同步
    let engine = SyncEngine::new(db.inner().clone());
    engine.sync_table(request).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_sync_progress(
    db: State<'_, SqlitePool>,
    task_id: i64,
) -> std::result::Result<SyncProgress, String> {
    use crate::db::TaskRepository;

    let repo = TaskRepository::new(&db);
    let task = repo.find_by_id(task_id).await.map_err(|e| e.to_string())?;
    let logs = repo.get_logs(task_id).await.map_err(|e| e.to_string())?;

    let log_messages: Vec<String> = logs.iter().map(|log| log.message.clone()).collect();

    Ok(SyncProgress {
        task_id,
        status: task.status,
        current_step: log_messages.last().cloned().unwrap_or_default(),
        total_steps: 10, // 固定步骤数
        current_step_index: log_messages.len(),
        logs: log_messages,
    })
}
