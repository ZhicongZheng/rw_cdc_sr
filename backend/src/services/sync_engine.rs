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
        app_db: MySqlPool,
        task_id: i64,
        mysql_config: DatabaseConfig,
        rw_config: DatabaseConfig,
        sr_config: DatabaseConfig,
        request: SyncRequest,
    ) -> Result<()> {
        let task_repo = TaskRepository::new(&app_db);

        // 第一步：从 MySQL 获取表结构
        let schema = Self::fetch_mysql_schema(
            &task_repo,
            task_id,
            &mysql_config,
            &request,
        ).await?;

        // 第二步：设置 RisingWave（schema、secret、source、table、sink）
        let rw_pool = Self::setup_risingwave(
            &task_repo,
            task_id,
            &mysql_config,
            &rw_config,
            &request
        ).await?;

        // 第三步：设置 StarRocks（数据库、表）
        let sr_conn = Self::setup_starrocks(
            &task_repo,
            task_id,
            &sr_config,
            &request,
            &schema,
        ).await?;

        // 第四步：创建 Sink 到 StarRocks
        Self::sink_to_starrocks(
            &task_repo,
            task_id,
            &sr_config,
            &request,
            &schema,
            &rw_pool,
        ).await?;

        // 完成日志
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

        // 关闭连接
        rw_pool.close().await;
        let _ = sr_conn.disconnect().await;

        Ok(())
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

    /// 第二步：设置 RisingWave（schema、secret、source、table、sink）
    async fn setup_risingwave(
        task_repo: &TaskRepository<'_>,
        task_id: i64,
        mysql_config: &DatabaseConfig,
        rw_config: &DatabaseConfig,
        request: &SyncRequest
    ) -> Result<PgPool> {
        // 连接到 RisingWave
        task_repo
            .add_log(task_id, "info", "Connecting to RisingWave...")
            .await?;
        tracing::info!("Connecting to RisingWave...");

        let rw_opts = ConnectionService::build_postgres_options_from_config(rw_config);
        let rw_pool = PgPool::connect_with(rw_opts).await.map_err(|e| {
            tracing::error!("Failed to connect to RisingWave: {}", e);
            e
        })?;
        tracing::info!("Successfully connected to RisingWave");

        // 创建 ods schema
        task_repo
            .add_log(task_id, "info", "Creating ods schema in RisingWave...")
            .await?;

        let schema_ddl = RisingWaveDDLGenerator::generate_create_schema_ddl(&request.mysql_database);
        tracing::debug!("Schema DDL: {}", schema_ddl);
        sqlx::query(&schema_ddl).execute(&rw_pool).await.map_err(|e| {
            tracing::error!("Failed to create ods schema: {}", e);
            e
        })?;
        tracing::info!("Ensured ods schema exists");

        // 创建 SECRET
        task_repo
            .add_log(task_id, "info", "Creating secret for MySQL password...")
            .await?;

        let secret_ddl = RisingWaveDDLGenerator::generate_secret_ddl(mysql_config)?;
        tracing::debug!("Secret DDL: {}", secret_ddl);
        sqlx::query(&secret_ddl).execute(&rw_pool).await.map_err(|e| {
            tracing::error!("Failed to create secret: {}", e);
            e
        })?;
        tracing::info!("Ensured secret exists");

        // 如果需要，删除现有对象
        if request.options.recreate_rw_source {
            task_repo
                .add_log(task_id, "info", "Dropping existing RisingWave objects...")
                .await?;

            Self::drop_risingwave_objects(&rw_pool, &request.mysql_database, &request.mysql_table).await?;
        }

        // 创建数据库级别的 CDC Source
        task_repo
            .add_log(
                task_id,
                "info",
                &format!("Creating RisingWave CDC source for database {}...", request.mysql_database),
            )
            .await?;

        let source_ddl = RisingWaveDDLGenerator::generate_source_ddl(mysql_config, &request.mysql_database)?;
        tracing::debug!("Source DDL: {}", source_ddl);
        sqlx::query(&source_ddl).execute(&rw_pool).await.map_err(|e| {
            tracing::error!("Failed to create RisingWave source: {}", e);
            e
        })?;
        tracing::info!("Successfully created RisingWave CDC source for database {}", request.mysql_database);

        // 创建 Table
        task_repo
            .add_log(
                task_id,
                "info",
                &format!("Creating RisingWave table ods.{}...", request.mysql_table),
            )
            .await?;

        let table_ddl = RisingWaveDDLGenerator::generate_table_ddl(&request.mysql_database, &request.mysql_table)?;
        tracing::debug!("Table DDL: {}", table_ddl);
        sqlx::query(&table_ddl).execute(&rw_pool).await.map_err(|e| {
            tracing::error!("Failed to create RisingWave table: {}", e);
            e
        })?;
        tracing::info!("Successfully created RisingWave table ods.{}", request.mysql_table);

        Ok(rw_pool)
    }

    /// 第三步：设置 StarRocks（数据库、表）
    async fn setup_starrocks(
        task_repo: &TaskRepository<'_>,
        task_id: i64,
        sr_config: &DatabaseConfig,
        request: &SyncRequest,
        schema: &crate::models::TableSchema,
    ) -> Result<mysql_async::Conn> {
        // 连接到 StarRocks
        task_repo
            .add_log(task_id, "info", "Connecting to StarRocks...")
            .await?;
        tracing::info!("Connecting to StarRocks...");

        let mut sr_opts_builder = mysql_async::OptsBuilder::default()
            .ip_or_hostname(&sr_config.host)
            .tcp_port(sr_config.port)
            .user(Some(&sr_config.username))
            .pass(Some(&sr_config.password))
            .prefer_socket(false); // 禁用 socket，只使用 TCP

        if let Some(db) = &sr_config.database_name {
            sr_opts_builder = sr_opts_builder.db_name(Some(db));
        }

        let sr_opts = mysql_async::Opts::from(sr_opts_builder);
        let mut sr_conn = mysql_async::Conn::new(sr_opts).await.map_err(|e| {
            tracing::error!("Failed to connect to StarRocks: {}", e);
            crate::utils::error::AppError::Connection(format!("StarRocks connection failed: {}", e))
        })?;
        tracing::info!("Successfully connected to StarRocks");

        // 创建数据库
        let create_db_ddl = StarRocksDDLGenerator::generate_create_database_ddl(&request.target_database);
        tracing::debug!("Create database DDL: {}", create_db_ddl);
        sr_conn.query_drop(&create_db_ddl).await.map_err(|e| {
            tracing::error!("Failed to create StarRocks database: {}", e);
            crate::utils::error::AppError::Unknown(format!("Failed to create database: {}", e))
        })?;
        tracing::info!("Ensured StarRocks database exists: {}", request.target_database);

        // 处理表（删除或清空）
        if request.options.recreate_sr_table {
            task_repo
                .add_log(task_id, "info", "Dropping existing StarRocks table...")
                .await?;

            let drop_table_ddl = StarRocksDDLGenerator::generate_drop_table_ddl(
                &request.target_database,
                &request.target_table,
            );
            tracing::debug!("Drop table DDL: {}", drop_table_ddl);
            sr_conn.query_drop(&drop_table_ddl).await.map_err(|e| {
                tracing::error!("Failed to drop StarRocks table: {}", e);
                crate::utils::error::AppError::Unknown(format!("Failed to drop table: {}", e))
            })?;
        } else if request.options.truncate_sr_table {
            // 先检查表是否存在
            let check_table_sql = format!(
                "SELECT 1 FROM information_schema.tables WHERE table_schema = '{}' AND table_name = '{}' LIMIT 1",
                request.target_database,
                request.target_table
            );
            tracing::debug!("Check table existence SQL: {}", check_table_sql);

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
                tracing::debug!("Truncate table DDL: {}", truncate_ddl);
                sr_conn.query_drop(&truncate_ddl).await.map_err(|e| {
                    tracing::error!("Failed to truncate StarRocks table: {}", e);
                    crate::utils::error::AppError::Unknown(format!("Failed to truncate table: {}", e))
                })?;
                tracing::info!("Successfully truncated table: {}.{}", request.target_database, request.target_table);
            } else {
                task_repo
                    .add_log(task_id, "info", "Table does not exist, skipping truncate...")
                    .await?;
                tracing::info!("Table {}.{} does not exist, skipping truncate", request.target_database, request.target_table);
            }
        }

        // 创建表
        task_repo
            .add_log(task_id, "info", "Creating StarRocks table...")
            .await?;

        let sr_table_ddl = StarRocksDDLGenerator::generate_table_ddl(
            schema,
            &request.target_database,
            &request.target_table,
        )?;
        tracing::debug!("StarRocks Table DDL: {}", sr_table_ddl);
        sr_conn.query_drop(&sr_table_ddl).await.map_err(|e| {
            tracing::error!("Failed to create StarRocks table: {}", e);
            crate::utils::error::AppError::Unknown(format!("Failed to create table: {}", e))
        })?;
        tracing::info!(
            "Successfully created StarRocks table: {}.{}",
            request.target_database,
            request.target_table
        );

        Ok(sr_conn)
    }

    async fn sink_to_starrocks(
        task_repo: &TaskRepository<'_>, 
        task_id: i64, 
        sr_config: &DatabaseConfig, 
        request: &SyncRequest, 
        schema: &crate::models::TableSchema,
        rw_pool: &PgPool,
    ) -> Result<()> {

        task_repo
            .add_log(task_id, "info", "Creating RisingWave sink to StarRocks...")
            .await?;

        let sink_ddl = RisingWaveDDLGenerator::generate_sink_ddl(
            sr_config,
            &request,
            schema
        )?;
        tracing::debug!("Sink DDL: {}", sink_ddl);
        sqlx::query(&sink_ddl).execute(rw_pool).await.map_err(|e| {
            tracing::error!("Failed to create RisingWave sink: {}", e);
            e
        })?;
        tracing::info!("Successfully created RisingWave sink to StarRocks");

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
