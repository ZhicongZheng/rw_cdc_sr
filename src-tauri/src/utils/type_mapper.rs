use crate::utils::error::{AppError, Result};

/// MySQL 类型到 RisingWave (PostgreSQL) 类型的映射
pub struct TypeMapper;

impl TypeMapper {
    /// 将 MySQL 类型映射到 RisingWave (PostgreSQL) 类型
    pub fn mysql_to_risingwave(mysql_type: &str) -> Result<String> {
        let mysql_type_upper = mysql_type.to_uppercase();
        let base_type = mysql_type_upper.split('(').next().unwrap_or(&mysql_type_upper);

        let rw_type = match base_type {
            // 整数类型
            "TINYINT" => "SMALLINT",
            "SMALLINT" => "SMALLINT",
            "MEDIUMINT" => "INTEGER",
            "INT" | "INTEGER" => "INTEGER",
            "BIGINT" => "BIGINT",

            // 浮点类型
            "FLOAT" => "REAL",
            "DOUBLE" | "DOUBLE PRECISION" => "DOUBLE PRECISION",
            "DECIMAL" | "NUMERIC" => {
                // 保留精度和小数位数
                if mysql_type.contains('(') {
                    return Ok(mysql_type.to_uppercase().replace("DECIMAL", "DECIMAL"));
                }
                "DECIMAL"
            }

            // 字符串类型
            "CHAR" => {
                if mysql_type.contains('(') {
                    return Ok(mysql_type.to_uppercase());
                }
                "CHAR"
            }
            "VARCHAR" => {
                if mysql_type.contains('(') {
                    return Ok(mysql_type.to_uppercase());
                }
                "VARCHAR"
            }
            "TEXT" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" => "TEXT",

            // 二进制类型
            "BINARY" | "VARBINARY" | "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" => "BYTEA",

            // 日期时间类型
            "DATE" => "DATE",
            "TIME" => "TIME",
            "DATETIME" | "TIMESTAMP" => "TIMESTAMP",
            "YEAR" => "SMALLINT",

            // JSON 类型
            "JSON" => "JSONB",

            // 其他类型
            "BOOLEAN" | "BOOL" => "BOOLEAN",
            "ENUM" => "VARCHAR(255)", // ENUM 转换为 VARCHAR
            "SET" => "TEXT",           // SET 转换为 TEXT

            _ => {
                return Err(AppError::TypeMapping(format!(
                    "Unsupported MySQL type: {}",
                    mysql_type
                )))
            }
        };

        Ok(rw_type.to_string())
    }

    /// 将 RisingWave (PostgreSQL) 类型映射到 StarRocks 类型
    pub fn risingwave_to_starrocks(rw_type: &str) -> Result<String> {
        let rw_type_upper = rw_type.to_uppercase();
        let base_type = rw_type_upper.split('(').next().unwrap_or(&rw_type_upper);

        let sr_type = match base_type {
            // 整数类型
            "SMALLINT" => "SMALLINT",
            "INTEGER" | "INT" => "INT",
            "BIGINT" => "BIGINT",

            // 浮点类型
            "REAL" | "FLOAT4" => "FLOAT",
            "DOUBLE PRECISION" | "FLOAT8" => "DOUBLE",
            "DECIMAL" | "NUMERIC" => {
                if rw_type.contains('(') {
                    return Ok(rw_type.to_uppercase());
                }
                "DECIMAL"
            }

            // 字符串类型
            "CHAR" => {
                if rw_type.contains('(') {
                    return Ok(rw_type.to_uppercase());
                }
                "CHAR"
            }
            "VARCHAR" => {
                if rw_type.contains('(') {
                    return Ok(rw_type.to_uppercase());
                }
                "VARCHAR"
            }
            "TEXT" => "STRING",

            // 二进制类型
            "BYTEA" => "VARBINARY",

            // 日期时间类型
            "DATE" => "DATE",
            "TIME" => "TIME",
            "TIMESTAMP" | "TIMESTAMPTZ" => "DATETIME",

            // JSON 类型
            "JSON" | "JSONB" => "JSON",

            // 布尔类型
            "BOOLEAN" | "BOOL" => "BOOLEAN",

            _ => {
                return Err(AppError::TypeMapping(format!(
                    "Unsupported RisingWave type: {}",
                    rw_type
                )))
            }
        };

        Ok(sr_type.to_string())
    }

    /// 直接从 MySQL 类型映射到 StarRocks 类型
    pub fn mysql_to_starrocks(mysql_type: &str) -> Result<String> {
        let rw_type = Self::mysql_to_risingwave(mysql_type)?;
        Self::risingwave_to_starrocks(&rw_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mysql_to_risingwave() {
        assert_eq!(
            TypeMapper::mysql_to_risingwave("INT").unwrap(),
            "INTEGER"
        );
        assert_eq!(
            TypeMapper::mysql_to_risingwave("VARCHAR(255)").unwrap(),
            "VARCHAR(255)"
        );
        assert_eq!(
            TypeMapper::mysql_to_risingwave("DATETIME").unwrap(),
            "TIMESTAMP"
        );
        assert_eq!(
            TypeMapper::mysql_to_risingwave("JSON").unwrap(),
            "JSONB"
        );
    }

    #[test]
    fn test_risingwave_to_starrocks() {
        assert_eq!(
            TypeMapper::risingwave_to_starrocks("INTEGER").unwrap(),
            "INT"
        );
        assert_eq!(
            TypeMapper::risingwave_to_starrocks("VARCHAR(255)").unwrap(),
            "VARCHAR(255)"
        );
        assert_eq!(
            TypeMapper::risingwave_to_starrocks("TIMESTAMP").unwrap(),
            "DATETIME"
        );
        assert_eq!(
            TypeMapper::risingwave_to_starrocks("TEXT").unwrap(),
            "STRING"
        );
    }

    #[test]
    fn test_mysql_to_starrocks() {
        assert_eq!(
            TypeMapper::mysql_to_starrocks("INT").unwrap(),
            "INT"
        );
        assert_eq!(
            TypeMapper::mysql_to_starrocks("TEXT").unwrap(),
            "STRING"
        );
    }
}
