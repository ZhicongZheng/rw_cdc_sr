use crate::db::ConfigRepository;
use crate::models::{
    ConnectionTestResult, CreateConnectionRequest, DatabaseConfig, TestConnectionRequest,
};
use crate::services::ConnectionService;
use sqlx::SqlitePool;
use tauri::State;

#[tauri::command]
pub async fn test_mysql_connection(
    request: TestConnectionRequest,
) -> std::result::Result<ConnectionTestResult, String> {
    ConnectionService::test_mysql(&request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_risingwave_connection(
    request: TestConnectionRequest,
) -> std::result::Result<ConnectionTestResult, String> {
    ConnectionService::test_risingwave(&request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_starrocks_connection(
    request: TestConnectionRequest,
) -> std::result::Result<ConnectionTestResult, String> {
    ConnectionService::test_starrocks(&request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_connection_config(
    db: State<'_, SqlitePool>,
    request: CreateConnectionRequest,
) -> std::result::Result<i64, String> {
    let repo = ConfigRepository::new(&db);
    repo.save(request).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_all_connections(
    db: State<'_, SqlitePool>,
) -> std::result::Result<Vec<DatabaseConfig>, String> {
    let repo = ConfigRepository::new(&db);
    repo.find_all().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_connection(
    db: State<'_, SqlitePool>,
    id: i64,
) -> std::result::Result<(), String> {
    let repo = ConfigRepository::new(&db);
    repo.delete(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_connection_config(
    db: State<'_, SqlitePool>,
    id: i64,
    request: CreateConnectionRequest,
) -> std::result::Result<(), String> {
    let repo = ConfigRepository::new(&db);
    repo.update(id, request).await.map_err(|e| e.to_string())
}
