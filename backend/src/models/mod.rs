pub mod config;
pub mod table;
pub mod task;

pub use config::*;
pub use table::*;
pub use task::*;

use serde::Serialize;

/// 泛型分页响应结构，用于所有需要分页的API端点
#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, limit: i64, offset: i64) -> Self {
        Self {
            data,
            total,
            limit,
            offset,
        }
    }
}
