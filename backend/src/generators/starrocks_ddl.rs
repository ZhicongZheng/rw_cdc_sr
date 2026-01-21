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
        // 确定主键列
        let pk_columns = if !schema.primary_keys.is_empty() {
            schema.primary_keys.clone()
        } else {
            // StarRocks 需要主键，如果没有主键，使用第一列
            if !schema.columns.is_empty() {
                vec![schema.columns[0].name.clone()]
            } else {
                return Err(crate::utils::error::AppError::SqlGeneration(
                    "Table has no columns".to_string(),
                ));
            }
        };

        let mut column_defs = Vec::new();
        let mut non_pk_columns = Vec::new();

        // 先处理主键列，放到最前面
        for pk_col_name in &pk_columns {
            if let Some(col) = schema.columns.iter().find(|c| &c.name == pk_col_name) {
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
        }

        // 再处理非主键列
        for col in &schema.columns {
            if !pk_columns.contains(&col.name) {
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

                non_pk_columns.push(format!(
                    "  `{}` {}{}{}",
                    col.name, sr_type, nullable, comment
                ));
            }
        }

        // 合并主键列和非主键列
        column_defs.extend(non_pk_columns);

        // 构建主键
        let primary_key = format!("PRIMARY KEY({})", pk_columns.join(", "));

        // 确定 DISTRIBUTED BY HASH 的列
        let hash_column = pk_columns[0].clone();

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
        assert_eq!(ddl, "DROP TABLE IF EXISTS `test_db`.`users`;");
    }

    #[test]
    fn test_primary_key_columns_first() {
        // 测试主键字段是否在最前面
        let schema = TableSchema {
            database: "test_db".to_string(),
            table_name: "orders".to_string(),
            columns: vec![
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
                Column {
                    name: "order_id".to_string(),
                    data_type: "BIGINT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    comment: Some("Order ID".to_string()),
                    character_maximum_length: None,
                    numeric_precision: Some(20),
                    numeric_scale: Some(0),
                },
                Column {
                    name: "user_id".to_string(),
                    data_type: "BIGINT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    comment: None,
                    character_maximum_length: None,
                    numeric_precision: Some(20),
                    numeric_scale: Some(0),
                },
                Column {
                    name: "amount".to_string(),
                    data_type: "DECIMAL(10,2)".to_string(),
                    is_nullable: false,
                    default_value: None,
                    comment: None,
                    character_maximum_length: None,
                    numeric_precision: Some(10),
                    numeric_scale: Some(2),
                },
            ],
            primary_keys: vec!["order_id".to_string(), "user_id".to_string()],
            indexes: vec![],
        };

        let ddl = StarRocksDDLGenerator::generate_table_ddl(&schema, "target_db", "orders").unwrap();

        // 验证主键字段在最前面
        let lines: Vec<&str> = ddl.lines().collect();
        // 第一个字段应该是 order_id
        assert!(lines.iter().any(|line| line.contains("`order_id` BIGINT NOT NULL COMMENT 'Order ID'")));
        // 第二个字段应该是 user_id
        assert!(lines.iter().any(|line| line.contains("`user_id` BIGINT NOT NULL")));

        // 验证 DDL 中字段的顺序
        let order_id_pos = ddl.find("`order_id`").unwrap();
        let user_id_pos = ddl.find("`user_id`").unwrap();
        let created_at_pos = ddl.find("`created_at`").unwrap();
        let amount_pos = ddl.find("`amount`").unwrap();

        // 主键字段应该在非主键字段之前
        assert!(order_id_pos < created_at_pos);
        assert!(order_id_pos < amount_pos);
        assert!(user_id_pos < created_at_pos);
        assert!(user_id_pos < amount_pos);
    }

    #[test]
    fn test_mysql_tinyint_to_starrocks_tinyint() {
        // 测试 MySQL TINYINT 正确映射到 StarRocks TINYINT
        let schema = TableSchema {
            database: "test_db".to_string(),
            table_name: "users".to_string(),
            columns: vec![
                Column {
                    name: "id".to_string(),
                    data_type: "INT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    comment: None,
                    character_maximum_length: None,
                    numeric_precision: Some(10),
                    numeric_scale: Some(0),
                },
                Column {
                    name: "active".to_string(),
                    data_type: "TINYINT".to_string(),
                    is_nullable: false,
                    default_value: None,
                    comment: None,
                    character_maximum_length: None,
                    numeric_precision: Some(3),
                    numeric_scale: Some(0),
                },
            ],
            primary_keys: vec!["id".to_string()],
            indexes: vec![],
        };

        let ddl = StarRocksDDLGenerator::generate_table_ddl(&schema, "target_db", "users").unwrap();

        // 验证 TINYINT 类型被正确映射
        assert!(ddl.contains("`active` TINYINT NOT NULL"));
    }
}
