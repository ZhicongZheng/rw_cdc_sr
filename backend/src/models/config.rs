use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 数据库类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DbType {
    #[serde(rename = "mysql")]
    MySQL,
    #[serde(rename = "risingwave")]
    RisingWave,
    #[serde(rename = "starrocks")]
    StarRocks,
}

// 实现 String 到 DbType 的转换（用于 SQLx）
impl TryFrom<String> for DbType {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "mysql" => Ok(DbType::MySQL),
            "risingwave" => Ok(DbType::RisingWave),
            "starrocks" => Ok(DbType::StarRocks),
            _ => Err(format!("Unknown db_type: {}", s)),
        }
    }
}

impl DbType {
    pub fn as_str(&self) -> &str {
        match self {
            DbType::MySQL => "mysql",
            DbType::RisingWave => "risingwave",
            DbType::StarRocks => "starrocks",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mysql" => Some(DbType::MySQL),
            "risingwave" => Some(DbType::RisingWave),
            "starrocks" => Some(DbType::StarRocks),
            _ => None,
        }
    }
}

/// 数据库连接配置
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DatabaseConfig {
    pub id: i64,
    pub name: String,
    #[sqlx(try_from = "String")]
    pub db_type: DbType,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String, // 加密存储
    pub database_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建数据库配置的请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConnectionRequest {
    pub name: String,
    pub db_type: DbType,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database_name: Option<String>,
}

/// 连接测试请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConnectionRequest {
    pub db_type: DbType,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database_name: Option<String>,
}

/// 连接测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub message: String,
    pub error: Option<String>,
}

impl ConnectionTestResult {
    pub fn success() -> Self {
        Self {
            success: true,
            message: "Connection successful".to_string(),
            error: None,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            message: "Connection failed".to_string(),
            error: Some(error),
        }
    }
}
