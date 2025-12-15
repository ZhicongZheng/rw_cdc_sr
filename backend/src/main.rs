//! RisingWave CDC to StarRocks Sync Tool
//! Web API Server

mod api;
mod db;
mod generators;
mod models;
mod services;
mod utils;

use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    Router,
};
use rust_embed::RustEmbed;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// 嵌入前端静态文件
#[derive(RustEmbed)]
#[folder = "../frontend/dist"]
struct Assets;

/// 静态文件服务 handler
async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == "index.html" {
        return index_html();
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            Response::builder()
                .status(StatusCode::OK)
                .header(
                    header::CONTENT_TYPE,
                    HeaderValue::from_str(mime.as_ref()).unwrap(),
                )
                .body(Body::from(content.data))
                .unwrap()
        }
        None => {
            if path.contains('.') {
                return not_found();
            }

            // SPA fallback - 返回 index.html
            index_html()
        }
    }
}

fn index_html() -> Response {
    match Assets::get("index.html") {
        Some(content) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html"),
            )
            .body(Body::from(content.data))
            .unwrap(),
        None => not_found(),
    }
}

fn not_found() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("404 - Not Found"))
        .unwrap()
}

use axum::http::HeaderValue;

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rw_cdc_sr=debug,tower_http=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting RW CDC SR Web Server...");

    // 初始化数据库
    let db = db::init_database()
        .await
        .expect("Failed to initialize database");

    tracing::info!("Database initialized successfully");

    // 创建 API 路由
    let app = Router::new()
        .merge(api::create_router(db))
        // 静态文件服务（嵌入的前端）
        .fallback(static_handler)
        // 请求追踪
        .layer(TraceLayer::new_for_http());

    // 服务器监听地址
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("Server listening on {}", addr);
    tracing::info!("API available at http://{}/api", addr);

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
