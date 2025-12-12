use serde::{Deserialize, Serialize};

/// 数据库表的列信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
    pub character_maximum_length: Option<i64>,
    pub numeric_precision: Option<i64>,
    pub numeric_scale: Option<i64>,
}

/// 表索引信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub index_name: String,
    pub column_name: String,
    pub is_unique: bool,
    pub seq_in_index: i32,
}

/// 表结构信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub database: String,
    pub table_name: String,
    pub columns: Vec<Column>,
    pub primary_keys: Vec<String>,
    pub indexes: Vec<Index>,
}

/// 数据库列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseList {
    pub databases: Vec<String>,
}

/// 表列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableList {
    pub tables: Vec<String>,
}
