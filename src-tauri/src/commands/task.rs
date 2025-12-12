use crate::db::TaskRepository;
use crate::models::{SyncTask, TaskHistoryQuery, TaskLog};
use sqlx::SqlitePool;
use tauri::State;

#[tauri::command]
pub async fn get_task_history(
    db: State<'_, SqlitePool>,
    query: TaskHistoryQuery,
) -> std::result::Result<Vec<SyncTask>, String> {
    let repo = TaskRepository::new(&db);
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    repo.find_history(query.status, limit, offset)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_task_detail(
    db: State<'_, SqlitePool>,
    task_id: i64,
) -> std::result::Result<SyncTask, String> {
    let repo = TaskRepository::new(&db);
    repo.find_by_id(task_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_task_logs(
    db: State<'_, SqlitePool>,
    task_id: i64,
) -> std::result::Result<Vec<TaskLog>, String> {
    let repo = TaskRepository::new(&db);
    repo.get_logs(task_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_task(
    db: State<'_, SqlitePool>,
    task_id: i64,
) -> std::result::Result<(), String> {
    use crate::models::TaskStatus;

    let repo = TaskRepository::new(&db);
    repo.update_status(task_id, TaskStatus::Cancelled, Some("Cancelled by user".to_string()))
        .await
        .map_err(|e| e.to_string())
}
