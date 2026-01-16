use axum::{extract::State, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::MySqlPool;
use std::collections::HashMap;

/// Alertmanager Webhook 请求体
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertmanagerWebhook {
    pub version: Option<String>,
    pub group_key: Option<String>,
    pub truncated_alerts: Option<u32>,
    pub status: String, // "firing" or "resolved"
    pub receiver: Option<String>,
    pub group_labels: Option<HashMap<String, String>>,
    pub common_labels: Option<HashMap<String, String>>,
    pub common_annotations: Option<HashMap<String, String>>,
    pub external_url: Option<String>,
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub status: String,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub generator_url: Option<String>,
    pub fingerprint: Option<String>,
}

/// 飞书消息格式
#[derive(Debug, Serialize)]
pub struct LarkTextMessage {
    pub msg_type: String,
    pub content: LarkTextContent,
}

#[derive(Debug, Serialize)]
pub struct LarkTextContent {
    pub text: String,
}

/// 格式化告警消息为飞书文本格式
fn format_alert_for_lark(alert: &Alert, status: &str) -> LarkTextMessage {
    let labels = &alert.labels;
    let annotations = &alert.annotations;

    let alert_name = labels.get("alertname").map(|s| s.as_str()).unwrap_or("Unknown");
    let severity = labels.get("severity").map(|s| s.as_str()).unwrap_or("unknown");
    let component = labels.get("component").map(|s| s.as_str()).unwrap_or("unknown");
    let summary = annotations.get("summary").map(|s| s.as_str()).unwrap_or("无摘要");
    let description = annotations
        .get("description")
        .map(|s| s.as_str())
        .unwrap_or("无详细描述");
    let instance = labels.get("instance").map(|s| s.as_str()).unwrap_or("N/A");
    let job = labels.get("job").map(|s| s.as_str()).unwrap_or("N/A");

    // 状态前缀和颜色
    let prefix = if status == "firing" {
        if severity.to_lowercase() == "critical" {
            "🚨 [CRITICAL]"
        } else {
            "⚠️ [WARNING]"
        }
    } else {
        "✅ [RESOLVED]"
    };

    // 格式化时间
    let time_str = if let Ok(dt) = DateTime::parse_from_rfc3339(&alert.starts_at) {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        alert.starts_at.clone()
    };

    // 构建额外信息
    let extra_info = match component {
        "sink" => {
            let sink_name = labels.get("sink_name").map(|s| s.as_str()).unwrap_or("N/A");
            format!(
                "• Sink: {}\n• Instance: {}",
                sink_name, instance
            )
        }
        "source" => {
            let source_name = labels.get("source_name").map(|s| s.as_str()).unwrap_or("N/A");
            format!(
                "• Source: {}\n• Instance: {}",
                source_name, instance
            )
        }
        "compute" => {
            let executor = labels.get("executor_name").map(|s| s.as_str()).unwrap_or("N/A");
            let fragment = labels.get("fragment_id").map(|s| s.as_str()).unwrap_or("N/A");
            format!(
                "• Executor: {}\n• Fragment: {}\n• Instance: {}",
                executor, fragment, instance
            )
        }
        _ => format!("• Job: {}\n• Instance: {}", job, instance),
    };

    // 构建完整消息
    let message = format!(
        r#"{} {}

{}

详细信息:
{}
• Severity: {}
• Component: {}
• Time: {}
• Alert: {}"#,
        prefix,
        summary,
        description,
        extra_info,
        severity.to_uppercase(),
        component,
        time_str,
        alert_name
    );

    LarkTextMessage {
        msg_type: "text".to_string(),
        content: LarkTextContent { text: message },
    }
}

/// 发送消息到飞书
async fn send_to_lark(
    webhook_url: &str,
    message: &LarkTextMessage,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post(webhook_url)
        .json(message)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Failed to send to Lark: {}", error_text).into());
    }

    Ok(())
}

/// Webhook 端点 - 接收 Alertmanager 告警
pub async fn receive_alertmanager_webhook(
    State(_pool): State<MySqlPool>,
    Json(payload): Json<AlertmanagerWebhook>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!(
        "Received {} alert(s) with status: {}",
        payload.alerts.len(),
        payload.status
    );

    // 从环境变量获取飞书 webhook URL
    let lark_webhook_url = std::env::var("LARK_WEBHOOK_URL").unwrap_or_else(|_| {
        tracing::warn!("LARK_WEBHOOK_URL not set in environment");
        String::new()
    });

    if lark_webhook_url.is_empty() {
        tracing::warn!("Lark webhook URL not configured, skipping notification");
        return Ok(Json(json!({
            "status": "received",
            "count": payload.alerts.len(),
            "notification_sent": false,
            "reason": "webhook_url_not_configured"
        })));
    }

    let mut success_count = 0;
    let mut error_count = 0;

    // 处理每个告警
    for alert in &payload.alerts {
        // 转换为飞书格式
        let lark_message = format_alert_for_lark(alert, &payload.status);

        // 发送到飞书
        match send_to_lark(&lark_webhook_url, &lark_message).await {
            Ok(_) => {
                tracing::info!(
                    "Successfully sent alert {} to Lark",
                    alert.labels.get("alertname").unwrap_or(&"Unknown".to_string())
                );
                success_count += 1;
            }
            Err(e) => {
                tracing::error!("Failed to send alert to Lark: {}", e);
                error_count += 1;
            }
        }
    }

    Ok(Json(json!({
        "status": "received",
        "count": payload.alerts.len(),
        "notification_sent": true,
        "success_count": success_count,
        "error_count": error_count
    })))
}

/// 健康检查端点
pub async fn webhook_health() -> Json<serde_json::Value> {
    let lark_configured = std::env::var("LARK_WEBHOOK_URL").is_ok();

    Json(json!({
        "status": "healthy",
        "service": "alertmanager-webhook",
        "lark_configured": lark_configured
    }))
}
