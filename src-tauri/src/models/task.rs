use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

// 实现 String 到 TaskStatus 的转换（用于 SQLx）
impl TryFrom<String> for TaskStatus {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "pending" => Ok(TaskStatus::Pending),
            "running" => Ok(TaskStatus::Running),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            "cancelled" => Ok(TaskStatus::Cancelled),
            _ => Err(format!("Unknown task status: {}", s)),
        }
    }
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
        }
    }
}

/// 同步选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOptions {
    /// 是否重建 RisingWave Source
    pub recreate_rw_source: bool,
    /// 是否重建 StarRocks 表
    pub recreate_sr_table: bool,
    /// 是否清空 StarRocks 表数据
    pub truncate_sr_table: bool,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            recreate_rw_source: false,
            recreate_sr_table: false,
            truncate_sr_table: false,
        }
    }
}

/// 同步请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub mysql_config_id: i64,
    pub rw_config_id: i64,
    pub sr_config_id: i64,
    pub mysql_database: String,
    pub mysql_table: String,
    pub target_database: String,
    pub target_table: String,
    pub options: SyncOptions,
}

/// 批量同步请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSyncRequest {
    pub mysql_config_id: i64,
    pub rw_config_id: i64,
    pub sr_config_id: i64,
    pub tables: Vec<TableSyncInfo>,
    pub options: SyncOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSyncInfo {
    pub mysql_database: String,
    pub mysql_table: String,
    pub target_database: String,
    pub target_table: String,
}

/// 同步任务
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncTask {
    pub id: i64,
    pub task_name: String,
    pub mysql_config_id: i64,
    pub rw_config_id: i64,
    pub sr_config_id: i64,
    pub mysql_database: String,
    pub mysql_table: String,
    pub target_database: String,
    pub target_table: String,
    #[sqlx(try_from = "String")]
    pub status: TaskStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub options: String, // JSON serialized SyncOptions
}

/// 任务日志
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TaskLog {
    pub id: i64,
    pub task_id: i64,
    pub log_level: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

/// 同步进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProgress {
    pub task_id: i64,
    pub status: TaskStatus,
    pub current_step: String,
    pub total_steps: usize,
    pub current_step_index: usize,
    pub logs: Vec<String>,
}

/// 任务历史查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistoryQuery {
    pub status: Option<TaskStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
