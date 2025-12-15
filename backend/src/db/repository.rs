use crate::models::{
    CreateConnectionRequest, DatabaseConfig, DbType, SyncTask, TaskLog, TaskStatus,
};
use crate::utils::crypto;
use crate::utils::error::{AppError, Result};
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;

/// 数据库配置仓库
pub struct ConfigRepository<'a> {
    pool: &'a MySqlPool,
}

impl<'a> ConfigRepository<'a> {
    pub fn new(pool: &'a MySqlPool) -> Self {
        Self { pool }
    }

    /// 保存数据库配置
    pub async fn save(&self, req: CreateConnectionRequest) -> Result<i64> {
        // 加密密码
        let encrypted_password = crypto::encrypt(&req.password)?;

        let result = sqlx::query(
            r#"
            INSERT INTO database_configs (name, db_type, host, port, username, password, database_name)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&req.name)
        .bind(req.db_type.as_str())
        .bind(&req.host)
        .bind(req.port as i32)
        .bind(&req.username)
        .bind(&encrypted_password)
        .bind(&req.database_name)
        .execute(self.pool)
        .await?;

        Ok(result.last_insert_id() as i64)
    }

    /// 获取所有数据库配置
    pub async fn find_all(&self) -> Result<Vec<DatabaseConfig>> {
        let configs: Vec<_> = sqlx::query_as::<_, ConfigRow>(
            "SELECT id, name, db_type, host, port, username, password, database_name, created_at, updated_at FROM database_configs ORDER BY created_at DESC",
        )
        .fetch_all(self.pool)
        .await?;

        configs.into_iter().map(|row| row.try_into()).collect()
    }

    /// 根据 ID 获取配置
    pub async fn find_by_id(&self, id: i64) -> Result<DatabaseConfig> {
        let row = sqlx::query_as::<_, ConfigRow>(
            "SELECT id, name, db_type, host, port, username, password, database_name, created_at, updated_at FROM database_configs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Config with id {} not found", id)))?;

        row.try_into()
    }

    /// 根据类型获取配置
    pub async fn find_by_type(&self, db_type: DbType) -> Result<Vec<DatabaseConfig>> {
        let configs: Vec<_> = sqlx::query_as::<_, ConfigRow>(
            "SELECT id, name, db_type, host, port, username, password, database_name, created_at, updated_at FROM database_configs WHERE db_type = ?",
        )
        .bind(db_type.as_str())
        .fetch_all(self.pool)
        .await?;

        configs.into_iter().map(|row| row.try_into()).collect()
    }

    /// 删除配置
    pub async fn delete(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM database_configs WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// 更新配置
    pub async fn update(&self, id: i64, req: CreateConnectionRequest) -> Result<()> {
        let encrypted_password = crypto::encrypt(&req.password)?;

        sqlx::query(
            r#"
            UPDATE database_configs
            SET name = ?, db_type = ?, host = ?, port = ?, username = ?, password = ?, database_name = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(&req.name)
        .bind(req.db_type.as_str())
        .bind(&req.host)
        .bind(req.port as i32)
        .bind(&req.username)
        .bind(&encrypted_password)
        .bind(&req.database_name)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(())
    }
}

/// 任务仓库
pub struct TaskRepository<'a> {
    pool: &'a MySqlPool,
}

impl<'a> TaskRepository<'a> {
    pub fn new(pool: &'a MySqlPool) -> Self {
        Self { pool }
    }

    /// 创建任务
    pub async fn create(&self, task: &SyncTask) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO sync_tasks (
                task_name, mysql_config_id, rw_config_id, sr_config_id,
                mysql_database, mysql_table, target_database, target_table,
                status, options
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&task.task_name)
        .bind(task.mysql_config_id)
        .bind(task.rw_config_id)
        .bind(task.sr_config_id)
        .bind(&task.mysql_database)
        .bind(&task.mysql_table)
        .bind(&task.target_database)
        .bind(&task.target_table)
        .bind(task.status.as_str())
        .bind(&task.options)
        .execute(self.pool)
        .await?;

        Ok(result.last_insert_id() as i64)
    }

    /// 更新任务状态
    pub async fn update_status(
        &self,
        task_id: i64,
        status: TaskStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        let completed_at = if status == TaskStatus::Completed || status == TaskStatus::Failed {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE sync_tasks
            SET status = ?, error_message = ?, completed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(&error_message)
        .bind(&completed_at)
        .bind(task_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// 获取任务详情
    pub async fn find_by_id(&self, task_id: i64) -> Result<SyncTask> {
        sqlx::query_as::<_, SyncTask>(
            "SELECT id, task_name, mysql_config_id, rw_config_id, sr_config_id, mysql_database, mysql_table, target_database, target_table, status, started_at, completed_at, error_message, options FROM sync_tasks WHERE id = ?",
        )
        .bind(task_id)
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Task with id {} not found", task_id)))
    }

    /// 获取任务历史（分页）
    pub async fn find_history(
        &self,
        status: Option<TaskStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SyncTask>> {
        let tasks = if let Some(status) = status {
            sqlx::query_as::<_, SyncTask>(
                "SELECT id, task_name, mysql_config_id, rw_config_id, sr_config_id, mysql_database, mysql_table, target_database, target_table, status, started_at, completed_at, error_message, options FROM sync_tasks WHERE status = ? ORDER BY started_at DESC LIMIT ? OFFSET ?",
            )
            .bind(status.as_str())
            .bind(limit)
            .bind(offset)
            .fetch_all(self.pool)
            .await?
        } else {
            sqlx::query_as::<_, SyncTask>(
                "SELECT id, task_name, mysql_config_id, rw_config_id, sr_config_id, mysql_database, mysql_table, target_database, target_table, status, started_at, completed_at, error_message, options FROM sync_tasks ORDER BY started_at DESC LIMIT ? OFFSET ?",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(self.pool)
            .await?
        };

        Ok(tasks)
    }

    /// 添加任务日志
    pub async fn add_log(&self, task_id: i64, level: &str, message: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO task_logs (task_id, log_level, message)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(task_id)
        .bind(level)
        .bind(message)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// 获取任务日志
    pub async fn get_logs(&self, task_id: i64) -> Result<Vec<TaskLog>> {
        let logs = sqlx::query_as::<_, TaskLog>(
            "SELECT id, task_id, log_level, message, created_at FROM task_logs WHERE task_id = ? ORDER BY created_at ASC",
        )
        .bind(task_id)
        .fetch_all(self.pool)
        .await?;

        Ok(logs)
    }
}

// 辅助结构用于从数据库读取配置
#[derive(sqlx::FromRow)]
struct ConfigRow {
    id: i64,
    name: String,
    db_type: String,
    host: String,
    port: i32,
    username: String,
    password: String,
    database_name: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<ConfigRow> for DatabaseConfig {
    type Error = AppError;

    fn try_from(row: ConfigRow) -> Result<Self> {
        let db_type = DbType::from_str(&row.db_type)
            .ok_or_else(|| AppError::Config(format!("Invalid db_type: {}", row.db_type)))?;

        // 解密密码
        let password = crypto::decrypt(&row.password)?;

        Ok(DatabaseConfig {
            id: row.id,
            name: row.name,
            db_type,
            host: row.host,
            port: row.port as u16,
            username: row.username,
            password,
            database_name: row.database_name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}
