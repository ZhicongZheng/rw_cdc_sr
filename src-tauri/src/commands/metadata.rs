use crate::db::ConfigRepository;
use crate::models::TableSchema;
use crate::services::MetadataService;
use sqlx::SqlitePool;
use tauri::State;

#[tauri::command]
pub async fn list_mysql_databases(
    db: State<'_, SqlitePool>,
    config_id: i64,
) -> std::result::Result<Vec<String>, String> {
    let repo = ConfigRepository::new(&db);
    let config = repo.find_by_id(config_id).await.map_err(|e| e.to_string())?;

    MetadataService::list_mysql_databases(&config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mysql_tables(
    db: State<'_, SqlitePool>,
    config_id: i64,
    database: String,
) -> std::result::Result<Vec<String>, String> {
    let repo = ConfigRepository::new(&db);
    let config = repo.find_by_id(config_id).await.map_err(|e| e.to_string())?;

    MetadataService::list_mysql_tables(&config, &database)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_table_schema(
    db: State<'_, SqlitePool>,
    config_id: i64,
    database: String,
    table: String,
) -> std::result::Result<TableSchema, String> {
    let repo = ConfigRepository::new(&db);
    let config = repo.find_by_id(config_id).await.map_err(|e| e.to_string())?;

    MetadataService::get_mysql_table_schema(&config, &database, &table)
        .await
        .map_err(|e| e.to_string())
}
