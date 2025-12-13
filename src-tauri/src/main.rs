// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod generators;
mod models;
mod services;
mod utils;

use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rw_cdc_sr=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 初始化数据库
    let db = db::init_database()
        .await
        .expect("Failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            // 连接管理相关命令
            commands::connection::test_mysql_connection,
            commands::connection::test_risingwave_connection,
            commands::connection::test_starrocks_connection,
            commands::connection::save_connection_config,
            commands::connection::update_connection_config,
            commands::connection::get_all_connections,
            commands::connection::delete_connection,
            // 元数据相关命令
            commands::metadata::list_mysql_databases,
            commands::metadata::list_mysql_tables,
            commands::metadata::get_table_schema,
            // 同步任务相关命令
            commands::sync::sync_single_table,
            commands::sync::sync_multiple_tables,
            commands::sync::get_sync_progress,
            commands::sync::retry_sync_task,
            // 任务管理相关命令
            commands::task::get_task_history,
            commands::task::get_task_detail,
            commands::task::get_task_logs,
            commands::task::cancel_task,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
