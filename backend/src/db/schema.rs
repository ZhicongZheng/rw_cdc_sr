/// 数据库配置表的 CREATE TABLE 语句 (MySQL 8)
pub const CREATE_DATABASE_CONFIGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS database_configs (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    db_type VARCHAR(50) NOT NULL,
    host VARCHAR(255) NOT NULL,
    port INT NOT NULL,
    username VARCHAR(255) NOT NULL,
    password TEXT NOT NULL,
    database_name VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
"#;

/// 同步任务表的 CREATE TABLE 语句 (MySQL 8)
pub const CREATE_SYNC_TASKS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS sync_tasks (
    id INT AUTO_INCREMENT PRIMARY KEY,
    task_name VARCHAR(500) NOT NULL,
    mysql_config_id INT NOT NULL,
    rw_config_id INT NOT NULL,
    sr_config_id INT NOT NULL,
    mysql_database VARCHAR(255) NOT NULL,
    mysql_table VARCHAR(255) NOT NULL,
    target_database VARCHAR(255) NOT NULL,
    target_table VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    started_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP NULL,
    error_message TEXT,
    options TEXT NOT NULL DEFAULT ('{}'),
    FOREIGN KEY (mysql_config_id) REFERENCES database_configs(id) ON DELETE CASCADE,
    FOREIGN KEY (rw_config_id) REFERENCES database_configs(id) ON DELETE CASCADE,
    FOREIGN KEY (sr_config_id) REFERENCES database_configs(id) ON DELETE CASCADE,
    INDEX idx_status (status),
    INDEX idx_started_at (started_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
"#;

/// 任务执行日志表的 CREATE TABLE 语句 (MySQL 8)
pub const CREATE_TASK_LOGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS task_logs (
    id INT AUTO_INCREMENT PRIMARY KEY,
    task_id INT NOT NULL,
    log_level VARCHAR(50) NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES sync_tasks(id) ON DELETE CASCADE,
    INDEX idx_task_id (task_id),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
"#;
