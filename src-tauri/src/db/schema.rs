/// 数据库配置表的 CREATE TABLE 语句
pub const CREATE_DATABASE_CONFIGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS database_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    db_type TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    database_name TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
"#;

/// 同步任务表的 CREATE TABLE 语句
pub const CREATE_SYNC_TASKS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS sync_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_name TEXT NOT NULL,
    mysql_config_id INTEGER NOT NULL,
    rw_config_id INTEGER NOT NULL,
    sr_config_id INTEGER NOT NULL,
    mysql_database TEXT NOT NULL,
    mysql_table TEXT NOT NULL,
    target_database TEXT NOT NULL,
    target_table TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    completed_at TEXT,
    error_message TEXT,
    options TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (mysql_config_id) REFERENCES database_configs(id) ON DELETE CASCADE,
    FOREIGN KEY (rw_config_id) REFERENCES database_configs(id) ON DELETE CASCADE,
    FOREIGN KEY (sr_config_id) REFERENCES database_configs(id) ON DELETE CASCADE
);
"#;

/// 任务执行日志表的 CREATE TABLE 语句
pub const CREATE_TASK_LOGS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS task_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    log_level TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    FOREIGN KEY (task_id) REFERENCES sync_tasks(id) ON DELETE CASCADE
);
"#;
