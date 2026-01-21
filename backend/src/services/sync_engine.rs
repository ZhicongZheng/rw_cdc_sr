use crate::db::{ConfigRepository, TaskRepository};
use crate::generators::{RisingWaveDDLGenerator, StarRocksDDLGenerator};
use crate::models::{DatabaseConfig, SyncRequest, SyncTask, TaskStatus};
use crate::services::{ConnectionService, MetadataService};
use crate::utils::error::Result;
use mysql_async::prelude::*;
use sqlx::{MySqlPool, PgPool};

/// 同步引擎
pub struct SyncEngine {
    app_db: MySqlPool,
}

impl SyncEngine {
    pub fn new(app_db: MySqlPool) -> Self {
        Self { app_db }
    }

    /// 同步单个表
    /// 内部调用 sync_multiple_tables，单表同步是批量同步的特例
    pub async fn sync_table(&self, request: SyncRequest) -> Result<i64> {
        tracing::info!(
            "Starting sync for single table: {}.{}",
            request.mysql_database,
            request.mysql_table
        );

        // 单表同步就是只有一个表的批量同步
        self.sync_multiple_tables(vec![request]).await
    }

    /// 同步多个表（批量同步）
    /// 创建一个批量任务，顺序处理多个表
    pub async fn sync_multiple_tables(&self, requests: Vec<SyncRequest>) -> Result<i64> {
        if requests.is_empty() {
            return Err(crate::utils::error::AppError::Validation(
                "No tables to sync".to_string(),
            ));
        }

        tracing::info!("Starting batch sync for {} tables", requests.len());

        // 验证所有请求使用相同的配置
        let first_request = &requests[0];
        for req in &requests {
            if req.mysql_config_id != first_request.mysql_config_id
                || req.rw_config_id != first_request.rw_config_id
                || req.sr_config_id != first_request.sr_config_id
            {
                return Err(crate::utils::error::AppError::Validation(
                    "All tables must use the same database configurations".to_string(),
                ));
            }
        }

        // 获取数据库配置
        let config_repo = ConfigRepository::new(&self.app_db);
        let mysql_config = config_repo.find_by_id(first_request.mysql_config_id).await?;
        let rw_config = config_repo.find_by_id(first_request.rw_config_id).await?;
        let sr_config = config_repo.find_by_id(first_request.sr_config_id).await?;

        // 创建批量任务记录
        let task_repo = TaskRepository::new(&self.app_db);

        // 根据表数量生成任务名称
        let (task_name, mysql_table_display, target_table_display) = if requests.len() == 1 {
            let req = &requests[0];
            (
                format!("Sync {}.{}", req.mysql_database, req.mysql_table),
                req.mysql_table.clone(),
                req.target_table.clone(),
            )
        } else {
            let table_list = requests
                .iter()
                .map(|r| format!("{}.{}", r.mysql_database, r.mysql_table))
                .collect::<Vec<_>>()
                .join(", ");
            (
                format!("Batch Sync {} tables", requests.len()),
                format!("[Batch: {}]", table_list),
                format!("[Batch: {} tables]", requests.len()),
            )
        };

        let task = SyncTask {
            id: 0,
            task_name,
            mysql_config_id: first_request.mysql_config_id,
            rw_config_id: first_request.rw_config_id,
            sr_config_id: first_request.sr_config_id,
            mysql_database: first_request.mysql_database.clone(),
            mysql_table: mysql_table_display,
            target_database: first_request.target_database.clone(),
            target_table: target_table_display,
            status: TaskStatus::Running,
            started_at: chrono::Utc::now(),
            completed_at: None,
            error_message: None,
            options: serde_json::to_string(&first_request.options)?,
        };

        let task_id = task_repo.create(&task).await?;

        // 异步执行批量同步任务
        let app_db_clone = self.app_db.clone();
        tokio::spawn(async move {
            tracing::info!("Executing batch sync task ID: {}", task_id);
            let result = Self::execute_batch_sync(
                app_db_clone.clone(),
                task_id,
                mysql_config,
                rw_config,
                sr_config,
                requests,
            )
            .await;

            let task_repo = TaskRepository::new(&app_db_clone);
            match result {
                Ok(_) => {
                    let _ = task_repo
                        .update_status(task_id, TaskStatus::Completed, None)
                        .await;
                    let _ = task_repo
                        .add_log(task_id, "info", "Batch sync completed successfully")
                        .await;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    tracing::error!("Batch sync task {} failed: {}", task_id, error_msg);
                    let _ = task_repo
                        .update_status(task_id, TaskStatus::Failed, Some(error_msg.clone()))
                        .await;
                    let _ = task_repo
                        .add_log(task_id, "error", &format!("Batch sync failed: {}", error_msg))
                        .await;
                }
            }
        });

        Ok(task_id)
    }

    /// 第一步：从 MySQL 获取表结构
    async fn fetch_mysql_schema(
        task_repo: &TaskRepository<'_>,
        task_id: i64,
        mysql_config: &DatabaseConfig,
        request: &SyncRequest,
    ) -> Result<crate::models::TableSchema> {
        task_repo
            .add_log(task_id, "info", "Fetching MySQL table schema...")
            .await?;
        tracing::info!("Fetching MySQL table schema from db: {}, table {}", &request.mysql_database, &request.mysql_table);

        let schema = MetadataService::get_mysql_table_schema(
            mysql_config,
            &request.mysql_database,
            &request.mysql_table,
        )
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to fetch schema for {}.{}: {}",
                request.mysql_database,
                request.mysql_table,
                e
            );
            e
        })?;

        tracing::info!(
            "Fetched schema for {}.{}: {} columns, {} primary keys",
            request.mysql_database,
            request.mysql_table,
            schema.columns.len(),
            schema.primary_keys.len()
        );

        Ok(schema)
    }

    /// 删除 RisingWave 对象
    async fn drop_risingwave_objects(
        pool: &PgPool,
        request: &SyncRequest,
    ) -> Result<()> {
        // 先删除 Sink
        let drop_sink = RisingWaveDDLGenerator::generate_drop_sink_ddl(
            &request.target_database,
            &request.target_table
        );
        tracing::debug!("Drop sink DDL: {}", drop_sink);
        let _ = sqlx::query(&drop_sink).execute(pool).await; // 忽略错误

        // 再删除 Table
        let drop_table = RisingWaveDDLGenerator::generate_drop_table_ddl(
            &request.target_database,
            &request.target_table
        );
        tracing::debug!("Drop table DDL: {}", drop_table);
        let _ = sqlx::query(&drop_table).execute(pool).await; // 忽略错误

        // 注意：不删除 Source，因为 Source 是数据库级别的，可能被其他表使用
        tracing::info!(
            "Note: Database-level source {}.{}_source is retained for reuse",
            request.target_database,
            request.mysql_database
        );

        Ok(())
    }

    /// 执行批量同步任务
    /// 顺序处理多个表，共享 schema、secret、source 和 database
    async fn execute_batch_sync(
        app_db: MySqlPool,
        task_id: i64,
        mysql_config: DatabaseConfig,
        rw_config: DatabaseConfig,
        sr_config: DatabaseConfig,
        requests: Vec<SyncRequest>,
    ) -> Result<()> {
        let task_repo = TaskRepository::new(&app_db);

        task_repo
            .add_log(
                task_id,
                "info",
                &format!("Starting batch sync for {} tables", requests.len()),
            )
            .await?;

        // 连接到 RisingWave
        task_repo
            .add_log(task_id, "info", "Connecting to RisingWave...")
            .await?;

        let rw_opts = ConnectionService::build_postgres_options_from_config(&rw_config);
        let rw_pool = PgPool::connect_with(rw_opts).await.map_err(|e| {
            tracing::error!("Failed to connect to RisingWave: {}", e);
            e
        })?;

        // 连接到 StarRocks
        task_repo
            .add_log(task_id, "info", "Connecting to StarRocks...")
            .await?;

        let mut sr_opts_builder = mysql_async::OptsBuilder::default()
            .ip_or_hostname(&sr_config.host)
            .tcp_port(sr_config.port)
            .user(Some(&sr_config.username))
            .pass(Some(&sr_config.password))
            .prefer_socket(false);

        if let Some(db) = &sr_config.database_name {
            sr_opts_builder = sr_opts_builder.db_name(Some(db));
        }

        let sr_opts = mysql_async::Opts::from(sr_opts_builder);
        let mut sr_conn = mysql_async::Conn::new(sr_opts).await.map_err(|e| {
            tracing::error!("Failed to connect to StarRocks: {}", e);
            crate::utils::error::AppError::Connection(format!("StarRocks connection failed: {}", e))
        })?;

        // 收集所有需要的 schema、source 和 database
        let mut schemas_created = std::collections::HashSet::new();
        let mut secrets_created = std::collections::HashSet::new();
        let mut sources_created = std::collections::HashSet::new();
        let mut databases_created = std::collections::HashSet::new();

        // 顺序处理每个表
        let total_tables = requests.len();
        for (index, request) in requests.iter().enumerate() {
            task_repo
                .add_log(
                    task_id,
                    "info",
                    &format!(
                        "Processing table {}/{}: {}.{}",
                        index + 1,
                        total_tables,
                        request.mysql_database,
                        request.mysql_table
                    ),
                )
                .await?;

            // 第一步：获取表结构
            let schema = Self::fetch_mysql_schema(
                &task_repo,
                task_id,
                &mysql_config,
                &request,
            ).await?;

            // 第二步：设置 RisingWave（只创建一次共享资源）
            // 创建 schema（如果还没创建）
            if !schemas_created.contains(&request.target_database) {
                task_repo
                    .add_log(task_id, "info", &format!("Creating schema {} in RisingWave...", request.target_database))
                    .await?;

                let schema_ddl = RisingWaveDDLGenerator::generate_create_schema_ddl(&request.target_database);
                sqlx::query(&schema_ddl).execute(&rw_pool).await.map_err(|e| {
                    tracing::error!("Failed to create schema: {}", e);
                    e
                })?;
                schemas_created.insert(request.target_database.clone());
            }

            // 创建 MySQL SECRET（如果还没创建）
            if !secrets_created.contains(&request.target_database) {
                task_repo
                    .add_log(task_id, "info", "Creating secret for MySQL password...")
                    .await?;

                let secret_ddl = RisingWaveDDLGenerator::generate_secret_ddl(&mysql_config, &request.target_database)?;
                sqlx::query(&secret_ddl).execute(&rw_pool).await.map_err(|e| {
                    tracing::error!("Failed to create secret: {}", e);
                    e
                })?;
                secrets_created.insert(request.target_database.clone());
            }

            // 创建 CDC Source（如果还没创建）
            let source_key = format!("{}:{}", request.target_database, request.mysql_database);
            if !sources_created.contains(&source_key) {
                task_repo
                    .add_log(
                        task_id,
                        "info",
                        &format!("Creating RisingWave CDC source for database {}...", request.mysql_database),
                    )
                    .await?;

                let source_ddl = RisingWaveDDLGenerator::generate_source_ddl(
                    &mysql_config,
                    &request.mysql_database,
                    &request.target_database
                )?;
                sqlx::query(&source_ddl).execute(&rw_pool).await.map_err(|e| {
                    tracing::error!("Failed to create RisingWave source: {}", e);
                    e
                })?;
                sources_created.insert(source_key);
            }

            // 如果需要，删除现有对象
            if request.options.recreate_rw_source {
                task_repo
                    .add_log(task_id, "info", "Dropping existing RisingWave objects...")
                    .await?;

                Self::drop_risingwave_objects(&rw_pool, request).await?;
            }

            // 创建 Table
            task_repo
                .add_log(
                    task_id,
                    "info",
                    &format!("Creating RisingWave table {}.{}...", request.target_database, request.target_table),
                )
                .await?;

            let table_ddl = RisingWaveDDLGenerator::generate_table_ddl(
                &request.mysql_database,
                &request.mysql_table,
                &request.target_database,
                &request.target_table
            )?;
            sqlx::query(&table_ddl).execute(&rw_pool).await.map_err(|e| {
                tracing::error!("Failed to create RisingWave table: {}", e);
                e
            })?;

            // 第三步：设置 StarRocks
            // 创建数据库（如果还没创建）
            if !databases_created.contains(&request.target_database) {
                let create_db_ddl = StarRocksDDLGenerator::generate_create_database_ddl(&request.target_database);
                sr_conn.query_drop(&create_db_ddl).await.map_err(|e| {
                    tracing::error!("Failed to create StarRocks database: {}", e);
                    crate::utils::error::AppError::Unknown(format!("Failed to create database: {}", e))
                })?;
                databases_created.insert(request.target_database.clone());
            }

            // 处理表（删除或清空）
            if request.options.recreate_sr_table {
                task_repo
                    .add_log(task_id, "info", "Dropping existing StarRocks table...")
                    .await?;

                let drop_table_ddl = StarRocksDDLGenerator::generate_drop_table_ddl(
                    &request.target_database,
                    &request.target_table,
                );
                sr_conn.query_drop(&drop_table_ddl).await.map_err(|e| {
                    tracing::error!("Failed to drop StarRocks table: {}", e);
                    crate::utils::error::AppError::Unknown(format!("Failed to drop table: {}", e))
                })?;
            } else if request.options.truncate_sr_table {
                let check_table_sql = format!(
                    "SELECT 1 FROM information_schema.tables WHERE table_schema = '{}' AND table_name = '{}' LIMIT 1",
                    request.target_database,
                    request.target_table
                );

                let table_exists: Option<i32> = sr_conn.query_first(&check_table_sql).await.map_err(|e| {
                    tracing::error!("Failed to check if table exists: {}", e);
                    crate::utils::error::AppError::Unknown(format!("Failed to check table existence: {}", e))
                })?;

                if table_exists.is_some() {
                    task_repo
                        .add_log(task_id, "info", "Truncating StarRocks table...")
                        .await?;

                    let truncate_ddl = StarRocksDDLGenerator::generate_truncate_table_ddl(
                        &request.target_database,
                        &request.target_table,
                    );
                    sr_conn.query_drop(&truncate_ddl).await.map_err(|e| {
                        tracing::error!("Failed to truncate StarRocks table: {}", e);
                        crate::utils::error::AppError::Unknown(format!("Failed to truncate table: {}", e))
                    })?;
                }
            }

            // 创建表
            task_repo
                .add_log(task_id, "info", "Creating StarRocks table...")
                .await?;

            let sr_table_ddl = StarRocksDDLGenerator::generate_table_ddl(
                &schema,
                &request.target_database,
                &request.target_table,
            )?;
            sr_conn.query_drop(&sr_table_ddl).await.map_err(|e| {
                tracing::error!("Failed to create StarRocks table: {}", e);
                crate::utils::error::AppError::Unknown(format!("Failed to create table: {}", e))
            })?;

            // 第四步：创建 Sink 到 StarRocks
            // 创建 StarRocks SECRET（如果还没创建）
            let sr_secret_key = format!("sr_secret:{}", request.target_database);
            if !secrets_created.contains(&sr_secret_key) {
                task_repo
                    .add_log(task_id, "info", "Creating secret for StarRocks password...")
                    .await?;

                let sr_secret_ddl = RisingWaveDDLGenerator::generate_starrocks_secret_ddl(&sr_config, &request.target_database)?;
                sqlx::query(&sr_secret_ddl).execute(&rw_pool).await.map_err(|e| {
                    tracing::error!("Failed to create StarRocks secret: {}", e);
                    e
                })?;
                secrets_created.insert(sr_secret_key);
            }

            task_repo
                .add_log(task_id, "info", "Creating RisingWave sink to StarRocks...")
                .await?;

            let sink_ddl = RisingWaveDDLGenerator::generate_sink_ddl(
                &sr_config,
                &request,
                &schema
            )?;
            sqlx::query(&sink_ddl).execute(&rw_pool).await.map_err(|e| {
                tracing::error!("Failed to create RisingWave sink: {}", e);
                e
            })?;

            task_repo
                .add_log(
                    task_id,
                    "info",
                    &format!(
                        "Successfully synced {}.{} to {}.{} ({}/{})",
                        request.mysql_database,
                        request.mysql_table,
                        request.target_database,
                        request.target_table,
                        index + 1,
                        total_tables
                    ),
                )
                .await?;
        }

        // 完成日志
        task_repo
            .add_log(
                task_id,
                "info",
                &format!("Successfully completed batch sync for {} tables", total_tables),
            )
            .await?;

        // 关闭连接
        rw_pool.close().await;
        let _ = sr_conn.disconnect().await;

        Ok(())
    }
}
