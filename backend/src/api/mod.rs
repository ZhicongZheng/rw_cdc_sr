pub mod connection;
pub mod metadata;
pub mod sync;
pub mod task;
pub mod risingwave;
pub mod webhook;

use axum::{
    routing::{get, post, delete, put},
    Router, Json,
};
use serde_json::json;
use sqlx::MySqlPool;
use tower_http::cors::CorsLayer;

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "service": "rw-cdc-sr"
    }))
}

/// 创建 API 路由
pub fn create_router(pool: MySqlPool) -> Router {
    Router::new()
        // Health check
        .route("/api/health", get(health_check))

        // 连接管理路由
        .route("/api/connections/test/mysql", post(connection::test_mysql))
        .route("/api/connections/test/risingwave", post(connection::test_risingwave))
        .route("/api/connections/test/starrocks", post(connection::test_starrocks))
        .route("/api/connections", post(connection::save_connection))
        .route("/api/connections", get(connection::list_connections))
        .route("/api/connections/:id", put(connection::update_connection))
        .route("/api/connections/:id", delete(connection::delete_connection))

        // 元数据路由
        .route("/api/metadata/databases", post(metadata::list_databases))
        .route("/api/metadata/tables", post(metadata::list_tables))
        .route("/api/metadata/schema", post(metadata::get_schema))

        // 同步任务路由
        .route("/api/sync/single", post(sync::sync_single_table))
        .route("/api/sync/multiple", post(sync::sync_multiple_tables))
        .route("/api/sync/progress/:id", get(sync::get_progress))
        .route("/api/sync/retry/:id", post(sync::retry_task))

        // 任务管理路由
        .route("/api/tasks/history", get(task::get_history))
        .route("/api/tasks/:id", get(task::get_detail))
        .route("/api/tasks/:id/logs", get(task::get_logs))
        .route("/api/tasks/:id/cancel", post(task::cancel_task))

        // RisingWave 对象管理路由
        .route("/api/risingwave/schemas", get(risingwave::list_schemas))
        .route("/api/risingwave/sources", get(risingwave::list_sources))
        .route("/api/risingwave/tables", get(risingwave::list_tables))
        .route("/api/risingwave/materialized_views", get(risingwave::list_materialized_views))
        .route("/api/risingwave/sinks", get(risingwave::list_sinks))
        .route("/api/risingwave/sources/delete", post(risingwave::delete_source))
        .route("/api/risingwave/tables/delete", post(risingwave::delete_table))
        .route("/api/risingwave/materialized_views/delete", post(risingwave::delete_materialized_view))
        .route("/api/risingwave/sinks/delete", post(risingwave::delete_sink))
        .route("/api/risingwave/objects/batch_delete", post(risingwave::batch_delete_objects))

        // Webhook 路由 - 用于接收 Alertmanager 告警
        .route("/api/webhook/alertmanager", post(webhook::receive_alertmanager_webhook))
        .route("/api/webhook/health", get(webhook::webhook_health))

        // CORS 配置
        .layer(CorsLayer::permissive())

        // 共享状态
        .with_state(pool)
}
