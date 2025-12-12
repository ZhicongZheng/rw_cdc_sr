use crate::models::TableSchema;
use crate::utils::error::Result;
use crate::utils::type_mapper::TypeMapper;

/// StarRocks DDL 生成器
pub struct StarRocksDDLGenerator;

impl StarRocksDDLGenerator {
    /// 生成 StarRocks 表创建语句
    pub fn generate_table_ddl(
        schema: &TableSchema,
        target_database: &str,
        target_table: &str,
    ) -> Result<String> {
        let mut column_defs = Vec::new();

        for col in &schema.columns {
            // 将 MySQL 类型转换为 StarRocks 类型
            let sr_type = TypeMapper::mysql_to_starrocks(&col.data_type)?;
            let nullable = if col.is_nullable {
                " NULL"
            } else {
                " NOT NULL"
            };

            let comment = if let Some(ref comment) = col.comment {
                format!(" COMMENT '{}'", comment.replace('\'', "''"))
            } else {
                String::new()
            };

            column_defs.push(format!(
                "  `{}` {}{}{}",
                col.name, sr_type, nullable, comment
            ));
        }

        // 构建主键
        let primary_key = if !schema.primary_keys.is_empty() {
            format!("PRIMARY KEY({})", schema.primary_keys.join(", "))
        } else {
            // StarRocks 需要主键，如果没有主键，使用第一列
            if !schema.columns.is_empty() {
                format!("PRIMARY KEY({})", schema.columns[0].name)
            } else {
                return Err(crate::utils::error::AppError::SqlGeneration(
                    "Table has no columns".to_string(),
                ));
            }
        };

        // 确定 DISTRIBUTED BY HASH 的列
        let hash_column = if !schema.primary_keys.is_empty() {
            schema.primary_keys[0].clone()
        } else if !schema.columns.is_empty() {
            schema.columns[0].name.clone()
        } else {
            return Err(crate::utils::error::AppError::SqlGeneration(
                "Cannot determine hash column".to_string(),
            ));
        };

        let ddl = format!(
            r#"CREATE TABLE IF NOT EXISTS `{}`.`{}` (
{}
) ENGINE=OLAP
{}
DISTRIBUTED BY HASH({}) BUCKETS 10
PROPERTIES (
  "replication_num" = "1",
  "in_memory" = "false",
  "storage_format" = "DEFAULT"
);"#,
            target_database,
            target_table,
            column_defs.join(",\n"),
            primary_key,
            hash_column
        );

        Ok(ddl)
    }

    /// 生成删除表的语句
    pub fn generate_drop_table_ddl(database: &str, table: &str) -> String {
        format!("DROP TABLE IF EXISTS `{}`.`{}`;", database, table)
    }

    /// 生成清空表数据的语句
    pub fn generate_truncate_table_ddl(database: &str, table: &str) -> String {
        format!("TRUNCATE TABLE `{}`.`{}`;", database, table)
    }

    /// 检查表是否存在的查询
    pub fn generate_check_table_exists_query(database: &str, table: &str) -> String {
        format!(
            "SELECT COUNT(*) as count FROM information_schema.tables WHERE table_schema = '{}' AND table_name = '{}'",
            database, table
        )
    }

    /// 生成创建数据库的语句
    pub fn generate_create_database_ddl(database: &str) -> String {
        format!("CREATE DATABASE IF NOT EXISTS `{}`;", database)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Column;

    fn create_test_schema() -> TableSchema {
        TableSchema {
            database: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                Column {
                    name: "id".to_string(),
                    data_type: "INT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    comment: Some("User ID".to_string()),
                    character_maximum_length: None,
                    numeric_precision: Some(10),
                    numeric_scale: Some(0),
                },
                Column {
                    name: "name".to_string(),
                    data_type: "VARCHAR(255)".to_string(),
                    is_nullable: true,
                    default_value: None,
                    comment: Some("User name".to_string()),
                    character_maximum_length: Some(255),
                    numeric_precision: None,
                    numeric_scale: None,
                },
                Column {
                    name: "created_at".to_string(),
                    data_type: "DATETIME".to_string(),
                    is_nullable: false,
                    default_value: None,
                    comment: None,
                    character_maximum_length: None,
                    numeric_precision: None,
                    numeric_scale: None,
                },
            ],
            primary_keys: vec!["id".to_string()],
            indexes: vec![],
        }
    }

    #[test]
    fn test_generate_table_ddl() {
        let schema = create_test_schema();
        let ddl =
            StarRocksDDLGenerator::generate_table_ddl(&schema, "target_db", "users_sr").unwrap();

        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS `target_db`.`users_sr`"));
        assert!(ddl.contains("`id` INT NOT NULL COMMENT 'User ID'"));
        assert!(ddl.contains("`name` VARCHAR(255) NULL COMMENT 'User name'"));
        assert!(ddl.contains("`created_at` DATETIME NOT NULL"));
        assert!(ddl.contains("PRIMARY KEY(id)"));
        assert!(ddl.contains("DISTRIBUTED BY HASH(id)"));
    }

    #[test]
    fn test_generate_drop_table_ddl() {
        let ddl = StarRocksDDLGenerator::generate_drop_table_ddl("test_db", "users");
        assert_eq!(ddl, "DROP TABLE IF NOT EXISTS `test_db`.`users`;");
    }
}
