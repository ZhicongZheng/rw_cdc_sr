use crate::models::{DatabaseConfig, SyncRequest, TableSchema};
use crate::utils::error::{AppError, Result};
use rand::Rng;

/// RisingWave DDL 生成器
pub struct RisingWaveDDLGenerator;

impl RisingWaveDDLGenerator {
    /// 生成创建 schema 的语句
    /// 使用 target_database 作为 schema 名称
    pub fn generate_create_schema_ddl(target_database: &str) -> String {
        format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", target_database)
    }

    /// 生成创建 SECRET 的语句（用于存储 MySQL 密码）
    /// 使用 target_database.mysql_pwd 作为 secret 名称
    pub fn generate_secret_ddl(mysql_config: &DatabaseConfig, target_database: &str) -> Result<String> {
        let secret_name = Self::get_secret_name(target_database);

        let ddl = format!(
            r#"CREATE SECRET IF NOT EXISTS {} WITH ( backend = 'meta' ) AS '{}';"#,
            secret_name,
            mysql_config.password.replace('\'', "''") // 转义单引号
        );

        Ok(ddl)
    }

    /// 获取 secret 名称: {target_database}.mysql_pwd
    pub fn get_secret_name(target_database: &str) -> String {
        format!("\"{}\".mysql_pwd", target_database)
    }

    /// 生成创建 StarRocks SECRET 的语句（用于存储 StarRocks 密码）
    /// 使用 target_database.starrocks_pwd 作为 secret 名称
    pub fn generate_starrocks_secret_ddl(sr_config: &DatabaseConfig, target_database: &str) -> Result<String> {
        let secret_name = Self::get_starrocks_secret_name(target_database);

        let ddl = format!(
            r#"CREATE SECRET IF NOT EXISTS {} WITH ( backend = 'meta' ) AS '{}';"#,
            secret_name,
            sr_config.password.replace('\'', "''") // 转义单引号
        );

        Ok(ddl)
    }

    /// 获取 StarRocks secret 名称: {target_database}.starrocks_pwd
    pub fn get_starrocks_secret_name(target_database: &str) -> String {
        format!("\"{}\".starrocks_pwd", target_database)
    }

    /// 生成数据库级别的 CDC Source 创建语句
    /// 一个 Source 对应整个 MySQL 数据库，而不是单个表
    /// Source 命名: {target_database}.{mysql_database}_source
    pub fn generate_source_ddl(
        mysql_config: &DatabaseConfig,
        mysql_database: &str,
        target_database: &str,
    ) -> Result<String> {
        // 生成随机 server.id (避免冲突)
        let server_id: u32 = rand::thread_rng().gen_range(5000..9999);

        // Source 命名: {target_database}.{mysql_database}_source
        let source_name = Self::get_source_name(mysql_database, target_database);
        let secret_name = Self::get_secret_name(target_database);

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

    pub fn get_source_name(mysql_database: &str, target_database: &str) -> String {
        format!("\"{}\".{}_source", target_database, mysql_database)
    }

    /// 生成 Table 创建语句（从 CDC Source，使用简化语法）
    /// 使用 (*) 自动推断所有列，支持 auto.schema.change
    /// Table 命名: {target_database}.{target_table}
    pub fn generate_table_ddl(
        mysql_database: &str,
        mysql_table: &str,
        target_database: &str,
        target_table: &str,
    ) -> Result<String> {
        let table_name = Self::get_rw_table_name(target_database, target_table);
        let source_name = Self::get_source_name(mysql_database, target_database);

        // 使用 (*) 语法自动推断所有列
        let ddl = format!(
            r#"CREATE TABLE IF NOT EXISTS {} (*) FROM {} TABLE '{}.{}';"#,
            table_name, source_name, mysql_database, mysql_table
        );

        Ok(ddl)
    }

    pub fn get_rw_table_name(target_database: &str, target_table: &str) -> String {
        format!("\"{}\".{}", target_database, target_table)
    }

    /// 生成 Sink 到 StarRocks 的语句
    /// Sink 命名: {target_database}.{target_table}_to_sr_sink
    pub fn generate_sink_ddl(
        sr_config: &DatabaseConfig,
        request: &SyncRequest,
        schema: &TableSchema,
    ) -> Result<String> {
        // RisingWave table: {target_database}.{target_table}
        let rw_table_name = Self::get_rw_table_name(&request.target_database, &request.target_table);

        // Sink 命名: {target_database}.{target_table}_to_sr_sink
        let sink_name = format!("\"{}\".{}_to_sr_sink", &request.target_database, &request.target_table);

        // 获取 StarRocks secret 名称
        let sr_secret_name = Self::get_starrocks_secret_name(&request.target_database);

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
            }
            // MySQL TINYINT -> RisingWave SMALLINT -> StarRocks TINYINT
            // 需要转换回 TINYINT，使用 CAST 到 INT2（即 SMALLINT，但在 StarRocks 中会被识别为 TINYINT）
            else if base_type == "TINYINT" {
                needs_type_conversion = true;
                // 使用 CAST 确保类型正确
                select_columns.push(format!("CAST({} AS SMALLINT) as {}", col.name, col.name));
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
                   starrocks.password = secret {},
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
                sr_secret_name,
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
                   starrocks.password = secret {},
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
                sr_secret_name,
                &request.target_database,
                &request.target_table,
                schema.primary_keys.join(",")
            )
        };

        Ok(ddl)
    }


    /// 生成删除 Table 的语句
    pub fn generate_drop_table_ddl(target_database: &str, target_table: &str) -> String {
        let table_name = format!("\"{}\".{}", target_database, target_table);
        format!("DROP TABLE IF EXISTS {} CASCADE;", table_name)
    }

    /// 生成删除 Sink 的语句
    pub fn generate_drop_sink_ddl(target_database: &str, target_table: &str) -> String {
        let sink_name = format!("\"{}\".{}_to_sr_sink", target_database, target_table);
        format!("DROP SINK IF EXISTS {};", sink_name)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DbType;

    #[test]
    fn test_generate_schema_ddl() {
        let ddl = RisingWaveDDLGenerator::generate_create_schema_ddl("ods_apn");
        assert_eq!(ddl, "CREATE SCHEMA IF NOT EXISTS \"ods_apn\";");
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

        let ddl = RisingWaveDDLGenerator::generate_secret_ddl(&config, "ods_apn").unwrap();
        assert!(ddl.contains("CREATE SECRET IF NOT EXISTS \"ods_apn\".mysql_pwd"));
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

        let ddl = RisingWaveDDLGenerator::generate_source_ddl(&config, "apnv3", "ods_apn").unwrap();
        assert!(ddl.contains("CREATE SOURCE IF NOT EXISTS \"ods_apn\".apnv3_source"));
        assert!(ddl.contains("connector = 'mysql-cdc'"));
        assert!(ddl.contains("auto.schema.change = 'true'"));
        assert!(ddl.contains("password = secret \"ods_apn\".mysql_pwd"));
    }

    #[test]
    fn test_generate_table_ddl() {
        let ddl = RisingWaveDDLGenerator::generate_table_ddl("apnv3", "invoice_activity", "ods_apn", "invoice_activity").unwrap();
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS \"ods_apn\".invoice_activity (*)"));
        assert!(ddl.contains("FROM \"ods_apn\".apnv3_source TABLE 'apnv3.invoice_activity'"));
    }
}
