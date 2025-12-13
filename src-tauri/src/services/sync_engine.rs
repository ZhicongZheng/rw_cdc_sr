use crate::db::{ConfigRepository, TaskRepository};
use crate::generators::{RisingWaveDDLGenerator, StarRocksDDLGenerator};
use crate::models::{DatabaseConfig, SyncRequest, SyncTask, TaskStatus};
use crate::services::{ConnectionService, MetadataService};
use crate::utils::error::Result;
use sqlx::{MySqlPool, PgPool, SqlitePool};

/// 同步引擎
pub struct SyncEngine {
    app_db: SqlitePool,
}

impl SyncEngine {
    pub fn new(app_db: SqlitePool) -> Self {
        Self { app_db }
    }

    /// 同步单个表
    pub async fn sync_table(&self, request: SyncRequest) -> Result<i64> {
        tracing::info!(
            "Starting sync for table: {}.{}",
            request.mysql_database,
            request.mysql_table
        );

        // 获取数据库配置
        let config_repo = ConfigRepository::new(&self.app_db);
        let mysql_config = config_repo.find_by_id(request.mysql_config_id).await?;
        let rw_config = config_repo.find_by_id(request.rw_config_id).await?;
        let sr_config = config_repo.find_by_id(request.sr_config_id).await?;

        // 创建任务记录
        let task_repo = TaskRepository::new(&self.app_db);
        let task = SyncTask {
            id: 0,
            task_name: format!("Sync {}.{}", request.mysql_database, request.mysql_table),
            mysql_config_id: request.mysql_config_id,
            rw_config_id: request.rw_config_id,
            sr_config_id: request.sr_config_id,
            mysql_database: request.mysql_database.clone(),
            mysql_table: request.mysql_table.clone(),
            target_database: request.target_database.clone(),
            target_table: request.target_table.clone(),
            status: TaskStatus::Running,
            started_at: chrono::Utc::now(),
            completed_at: None,
            error_message: None,
            options: serde_json::to_string(&request.options)?,
        };

        let task_id = task_repo.create(&task).await?;

        // 异步执行同步任务
        let app_db_clone = self.app_db.clone();
        tokio::spawn(async move {
            tracing::info!("Executing sync task ID: {}", task_id);
            let result = Self::execute_sync(
                app_db_clone.clone(),
                task_id,
                mysql_config,
                rw_config,
                sr_config,
                request,
            )
                .await;

            let task_repo = TaskRepository::new(&app_db_clone);
            match result {
                Ok(_) => {
                    let _ = task_repo
                        .update_status(task_id, TaskStatus::Completed, None)
                        .await;
                    let _ = task_repo
                        .add_log(task_id, "info", "Sync completed successfully")
                        .await;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    tracing::error!("Sync task {} failed: {}", task_id, error_msg);
                    let _ = task_repo
                        .update_status(task_id, TaskStatus::Failed, Some(error_msg.clone()))
                        .await;
                    let _ = task_repo
                        .add_log(task_id, "error", &format!("Sync failed: {}", error_msg))
                        .await;
                }
            }
        });

        Ok(task_id)
    }

    /// 执行同步任务
    async fn execute_sync(
        app_db: SqlitePool,
        task_id: i64,
        mysql_config: DatabaseConfig,
        rw_config: DatabaseConfig,
        sr_config: DatabaseConfig,
        request: SyncRequest,
    ) -> Result<()> {
        let task_repo = TaskRepository::new(&app_db);

        // Step 1: 获取 MySQL 表结构
        task_repo
            .add_log(task_id, "info", "Fetching MySQL table schema...")
            .await?;

        let schema = MetadataService::get_mysql_table_schema(
            &mysql_config,
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

        // Step 2: 连接到 RisingWave
        task_repo
            .add_log(task_id, "info", "Connecting to RisingWave...")
            .await?;

        let rw_opts = ConnectionService::build_postgres_options_from_config(&rw_config);
        let rw_pool = PgPool::connect_with(rw_opts).await.map_err(|e| {
            tracing::error!("Failed to connect to RisingWave: {}", e);
            e
        })?;
        tracing::info!("Successfully connected to RisingWave");

        // Step 3: 创建 ods schema（如果不存在）
        task_repo
            .add_log(task_id, "info", "Creating ods schema in RisingWave...")
            .await?;

        let schema_ddl =
            RisingWaveDDLGenerator::generate_create_schema_ddl(&request.mysql_database);
        tracing::debug!("Schema DDL: {}", schema_ddl);
        sqlx::query(&schema_ddl)
            .execute(&rw_pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create ods schema: {}", e);
                e
            })?;
        tracing::info!("Ensured ods schema exists");

        // Step 4: 创建 SECRET（如果不存在）
        task_repo
            .add_log(task_id, "info", "Creating secret for MySQL password...")
            .await?;

        let secret_ddl = RisingWaveDDLGenerator::generate_secret_ddl(&mysql_config)?;
        tracing::debug!("Secret DDL: {}", secret_ddl);
        sqlx::query(&secret_ddl)
            .execute(&rw_pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create secret: {}", e);
                e
            })?;
        tracing::info!("Ensured secret exists");

        // Step 5: 如果需要，删除现有的 RisingWave 对象
        if request.options.recreate_rw_source {
            task_repo
                .add_log(task_id, "info", "Dropping existing RisingWave objects...")
                .await?;

            Self::drop_risingwave_objects(&rw_pool, &request.mysql_database, &request.mysql_table)
                .await?;
        }

        // Step 6: 创建数据库级别的 CDC Source（如果不存在）
        task_repo
            .add_log(
                task_id,
                "info",
                &format!(
                    "Creating RisingWave CDC source for database {}...",
                    request.mysql_database
                ),
            )
            .await?;

        let source_ddl =
            RisingWaveDDLGenerator::generate_source_ddl(&mysql_config, &request.mysql_database)?;
        tracing::debug!("Source DDL: {}", source_ddl);
        sqlx::query(&source_ddl)
            .execute(&rw_pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create RisingWave source: {}", e);
                e
            })?;
        tracing::info!(
            "Successfully created RisingWave CDC source for database {}",
            request.mysql_database
        );

        // Step 7: 在 RisingWave 中创建 Table（使用简化语法）
        task_repo
            .add_log(
                task_id,
                "info",
                &format!("Creating RisingWave table ods.{}...", request.mysql_table),
            )
            .await?;

        let table_ddl = RisingWaveDDLGenerator::generate_table_ddl(
            &request.mysql_database,
            &request.mysql_table,
        )?;
        tracing::debug!("Table DDL: {}", table_ddl);
        sqlx::query(&table_ddl)
            .execute(&rw_pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create RisingWave table: {}", e);
                e
            })?;
        tracing::info!(
            "Successfully created RisingWave table ods.{}",
            request.mysql_table
        );

        // Step 8: 连接到 StarRocks
        task_repo
            .add_log(task_id, "info", "Connecting to StarRocks...")
            .await?;

        let sr_opts = ConnectionService::build_mysql_options_from_config(&sr_config);
        let sr_pool = MySqlPool::connect_with(sr_opts).await.map_err(|e| {
            tracing::error!("Failed to connect to StarRocks: {}", e);
            e
        })?;
        tracing::info!("Successfully connected to StarRocks");

        // Step 9: 创建 StarRocks 数据库（如果不存在）
        let create_db_ddl =
            StarRocksDDLGenerator::generate_create_database_ddl(&request.target_database);
        tracing::debug!("Create database DDL: {}", create_db_ddl);
        sqlx::query(&create_db_ddl)
            .execute(&sr_pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create StarRocks database: {}", e);
                e
            })?;
        tracing::info!(
            "Ensured StarRocks database exists: {}",
            request.target_database
        );

        // Step 10: 处理 StarRocks 表
        if request.options.recreate_sr_table {
            task_repo
                .add_log(task_id, "info", "Dropping existing StarRocks table...")
                .await?;

            let drop_table_ddl = StarRocksDDLGenerator::generate_drop_table_ddl(
                &request.target_database,
                &request.target_table,
            );
            sqlx::query(&drop_table_ddl).execute(&sr_pool).await?;
        } else if request.options.truncate_sr_table {
            task_repo
                .add_log(task_id, "info", "Truncating StarRocks table...")
                .await?;

            let truncate_ddl = StarRocksDDLGenerator::generate_truncate_table_ddl(
                &request.target_database,
                &request.target_table,
            );
            sqlx::query(&truncate_ddl).execute(&sr_pool).await?;
        }

        // Step 11: 在 StarRocks 中创建表
        task_repo
            .add_log(task_id, "info", "Creating StarRocks table...")
            .await?;

        let sr_table_ddl = StarRocksDDLGenerator::generate_table_ddl(
            &schema,
            &request.target_database,
            &request.target_table,
        )?;
        tracing::debug!("StarRocks Table DDL: {}", sr_table_ddl);
        sqlx::query(&sr_table_ddl)
            .execute(&sr_pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create StarRocks table: {}", e);
                e
            })?;
        tracing::info!(
            "Successfully created StarRocks table: {}.{}",
            request.target_database,
            request.target_table
        );

        // Step 12: 在 RisingWave 中创建 Sink 到 StarRocks
        task_repo
            .add_log(task_id, "info", "Creating RisingWave sink to StarRocks...")
            .await?;

        let sink_ddl = RisingWaveDDLGenerator::generate_sink_ddl(
            &sr_config,
            &request.mysql_table,
            &schema,
            &request.target_database,
            &request.target_table,
        )?;
        tracing::debug!("Sink DDL: {}", sink_ddl);
        sqlx::query(&sink_ddl)
            .execute(&rw_pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create RisingWave sink: {}", e);
                e
            })?;
        tracing::info!("Successfully created RisingWave sink to StarRocks");

        task_repo
            .add_log(
                task_id,
                "info",
                &format!(
                    "Successfully synced {}.{} to {}.{}",
                    request.mysql_database,
                    request.mysql_table,
                    request.target_database,
                    request.target_table
                ),
            )
            .await?;

        rw_pool.close().await;
        sr_pool.close().await;

        Ok(())
    }

    /// 删除 RisingWave 对象
    async fn drop_risingwave_objects(
        pool: &PgPool,
        mysql_database: &str,
        mysql_table: &str,
    ) -> Result<()> {
        // 先删除 Sink
        let drop_sink = RisingWaveDDLGenerator::generate_drop_sink_ddl(mysql_table);
        tracing::debug!("Drop sink DDL: {}", drop_sink);
        let _ = sqlx::query(&drop_sink).execute(pool).await; // 忽略错误

        // 再删除 Table
        let drop_table = RisingWaveDDLGenerator::generate_drop_table_ddl(mysql_table);
        tracing::debug!("Drop table DDL: {}", drop_table);
        let _ = sqlx::query(&drop_table).execute(pool).await; // 忽略错误

        // 注意：不删除 Source，因为 Source 是数据库级别的，可能被其他表使用
        tracing::info!(
            "Note: Database-level source ods.ods_{} is retained for reuse",
            mysql_database
        );

        Ok(())
    }
}
