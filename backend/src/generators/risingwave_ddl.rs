use crate::models::{DatabaseConfig, SyncRequest, TableSchema};
use crate::utils::error::{AppError, Result};
use rand::Rng;

/// RisingWave DDL 生成器
pub struct RisingWaveDDLGenerator;

impl RisingWaveDDLGenerator {
    /// 生成创建 ods schema 的语句
    pub fn generate_create_schema_ddl(mysql_database: &String) -> String {
        let schema_name = Self::get_rw_schema_name(mysql_database);
        format!("CREATE SCHEMA IF NOT EXISTS {};", schema_name)
    }

    pub fn get_rw_schema_name(mysql_database: &str) -> String {
        format!("ods_{}", mysql_database)
    }

    /// 生成创建 SECRET 的语句（用于存储 MySQL 密码）
    pub fn generate_secret_ddl(mysql_config: &DatabaseConfig) -> Result<String> {
        // Secret 名称: {config_name}_pwd (移除特殊字符，转为小写)
        let secret_name = Self::get_secret_name(mysql_config);

        let ddl = format!(
            r#"CREATE SECRET IF NOT EXISTS {} WITH ( backend = 'meta' ) AS '{}';"#,
            secret_name,
            mysql_config.password.replace('\'', "''") // 转义单引号
        );

        Ok(ddl)
    }

    /// 获取 secret 名称 (移除特殊字符，转为小写)
    pub fn get_secret_name(mysql_config: &DatabaseConfig) -> String {
        format!(
            "{}_pwd",
            mysql_config
                .name
                .to_lowercase()
                .replace(' ', "_")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        )
    }

    /// 生成数据库级别的 CDC Source 创建语句
    /// 一个 Source 对应整个 MySQL 数据库，而不是单个表
    pub fn generate_source_ddl(
        mysql_config: &DatabaseConfig,
        mysql_database: &str,
    ) -> Result<String> {
        // 生成随机 server.id (避免冲突)
        let server_id: u32 = rand::thread_rng().gen_range(5000..9999);

        // Source 命名: ods.ods_{mysql_database}
        let source_name = Self::get_source_name(mysql_database);
        let secret_name = Self::get_secret_name(mysql_config);

        let ddl = format!(
            r#"CREATE SOURCE IF NOT EXISTS {} WITH (
              connector = 'mysql-cdc',
              hostname = '{}',
              port = '{}',
              username = '{}',
              password = secret {},
              database.name = '{}',
              server.id = '{}',
              auto.schema.change = 'true'
            );"#,
            source_name,
            mysql_config.host,
            mysql_config.port,
            mysql_config.username,
            secret_name,
            mysql_database,
            server_id
        );

        Ok(ddl)
    }

    pub fn get_source_name(mysql_database: &str) -> String {
        let schema_name = Self::get_rw_schema_name(mysql_database);
        format!("{}.ods_{}_source", schema_name, mysql_database)
    }

    /// 生成 Table 创建语句（从 CDC Source，使用简化语法）
    /// 使用 (*) 自动推断所有列，支持 auto.schema.change
    pub fn generate_table_ddl(mysql_database: &str, mysql_table: &str) -> Result<String> {
        let table_name = Self::get_rw_table_name(mysql_database, mysql_table);
        let source_name = Self::get_source_name(mysql_database);

        // 使用 (*) 语法自动推断所有列
        let ddl = format!(
            r#"CREATE TABLE IF NOT EXISTS {} (*) FROM {} TABLE '{}.{}';"#,
            table_name, source_name, mysql_database, mysql_table
        );

        Ok(ddl)
    }

    pub fn get_rw_table_name(mysql_database: &str, mysql_table: &str) -> String {
        format!(
            "{}.{}",
            Self::get_rw_schema_name(mysql_database),
            mysql_table
        )
    }

    /// 生成 Sink 到 StarRocks 的语句
    pub fn generate_sink_ddl(
        sr_config: &DatabaseConfig,
        request: &SyncRequest,
        schema: &TableSchema,
    ) -> Result<String> {
        // RisingWave table: ods.{mysql_table}
        let rw_schema_name = Self::get_rw_schema_name(&request.mysql_database);
        let rw_table_name = Self::get_rw_table_name(&request.mysql_database, &request.mysql_table);

        // Sink 命名: ods.{table}_to_sr_sink
        let sink_name = format!("{}.{}_to_sr_sink", rw_schema_name, &request.mysql_table);

        // 检查是否有主键
        if schema.primary_keys.is_empty() {
            return Err(AppError::SqlGeneration(format!(
                "Table {} has no primary key, cannot create upsert sink",
                &request.mysql_table
            )));
        }

        // 检查是否有需要类型转换的列
        let mut needs_type_conversion = false;
        let mut select_columns = Vec::new();

        for col in &schema.columns {
            let col_type_upper = col.data_type.to_uppercase();
            let base_type = col_type_upper.split('(').next().unwrap_or(&col_type_upper);

            // MySQL TIMESTAMP/DATETIME -> RisingWave TIMESTAMPTZ -> StarRocks DATETIME
            // 需要转换为 TIMESTAMP（不带时区）
            if base_type == "TIMESTAMP" || base_type == "DATETIME" {
                needs_type_conversion = true;
                select_columns.push(format!("{}::TIMESTAMP as {}", col.name, col.name));
            } else {
                select_columns.push(col.name.clone());
            }
        }

        let ddl = if needs_type_conversion {
            // 使用 SELECT 语句进行类型转换
            format!(
                r#"CREATE SINK IF NOT EXISTS {} AS
                   SELECT
                   {}
                   FROM {}
                   WITH (
                   connector = 'starrocks',
                   starrocks.host = '{}',
                   starrocks.mysqlport = '{}',
                   starrocks.httpport = '8030',
                   starrocks.user = '{}',
                   starrocks.password = '{}',
                   starrocks.database = '{}',
                   starrocks.table = '{}',
                   type = 'upsert',
                   primary_key = '{}'
                   );"#,
                sink_name,
                select_columns.join(",\n  "),
                rw_table_name,
                sr_config.host,
                sr_config.port,
                sr_config.username,
                sr_config.password,
                &request.target_database,
                &request.target_table,
                schema.primary_keys.join(",")
            )
        } else {
            // 不需要类型转换，直接从表创建 sink
            format!(
                r#"CREATE SINK IF NOT EXISTS {} FROM {}
                   WITH (
                   connector = 'starrocks',
                   starrocks.host = '{}',
                   starrocks.mysqlport = '{}',
                   starrocks.httpport = '8030',
                   starrocks.user = '{}',
                   starrocks.password = '{}',
                   starrocks.database = '{}',
                   starrocks.table = '{}',
                   type = 'upsert',
                   primary_key = '{}'
                   );"#,
                sink_name,
                rw_table_name,
                sr_config.host,
                sr_config.port,
                sr_config.username,
                sr_config.password,
                &request.target_database,
                &request.target_table,
                schema.primary_keys.join(",")
            )
        };

        Ok(ddl)
    }


    /// 生成删除 Table 的语句
    pub fn generate_drop_table_ddl(mysql_table: &str) -> String {
        let table_name = format!("ods.{}", mysql_table);
        format!("DROP TABLE IF EXISTS {} CASCADE;", table_name)
    }

    /// 生成删除 Sink 的语句
    pub fn generate_drop_sink_ddl(mysql_table: &str) -> String {
        let sink_name = format!("ods.{}_to_sr_sink", mysql_table);
        format!("DROP SINK IF EXISTS {};", sink_name)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DbType;

    #[test]
    fn test_generate_schema_ddl() {
        let ddl = RisingWaveDDLGenerator::generate_create_schema_ddl(&"test".to_string());
        assert_eq!(ddl, "CREATE SCHEMA IF NOT EXISTS ods_test;");
    }

    #[test]
    fn test_generate_secret_ddl() {
        let config = DatabaseConfig {
            id: 1,
            name: "Production MySQL".to_string(),
            db_type: DbType::MySQL,
            host: "localhost".to_string(),
            port: 3306,
            username: "root".to_string(),
            password: "my'password".to_string(),
            database_name: Some("test_db".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let ddl = RisingWaveDDLGenerator::generate_secret_ddl(&config).unwrap();
        assert!(ddl.contains("CREATE SECRET IF NOT EXISTS production_mysql_pwd"));
        assert!(ddl.contains("my''password")); // 单引号被转义
    }

    #[test]
    fn test_generate_source_ddl() {
        let config = DatabaseConfig {
            id: 1,
            name: "test".to_string(),
            db_type: DbType::MySQL,
            host: "localhost".to_string(),
            port: 3306,
            username: "root".to_string(),
            password: "password".to_string(),
            database_name: Some("test_db".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let ddl = RisingWaveDDLGenerator::generate_source_ddl(&config, "test").unwrap();
        assert!(ddl.contains("CREATE SOURCE IF NOT EXISTS ods_test_source"));
        assert!(ddl.contains("connector = 'mysql-cdc'"));
        assert!(ddl.contains("auto.schema.change = 'true'"));
        assert!(ddl.contains("password = secret test_pwd"));
    }

    #[test]
    fn test_generate_table_ddl() {
        let ddl = RisingWaveDDLGenerator::generate_table_ddl("test", "users").unwrap();
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS ods_test.users (*)"));
        assert!(ddl.contains("FROM ods_test_source TABLE 'test.users'"));
    }
}
