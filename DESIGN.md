# RisingWave CDC to StarRocks 同步工具设计文档

## 项目概述

一个基于 Tauri + Rust 的桌面/Web 应用，帮助开发者通过 RisingWave 一键同步 MySQL 数据到 StarRocks。

**核心价值**: 简化 MySQL → RisingWave → StarRocks 的数据同步流程，提供友好的可视化界面。

## 技术栈

### 后端
- **Rust 2024 Edition**: 最新的 Rust 版本
- **Tauri 2.x**: 最新的 Tauri 框架（支持桌面和 Web）
- **SQLx 0.8.x**: 异步 SQL 工具包（支持 MySQL, PostgreSQL, SQLite）
- **Tokio**: 异步运行时
- **Serde**: 序列化/反序列化
- **Anyhow/Thiserror**: 错误处理

### 前端
- **React 18.x**: 最新的 React
- **TypeScript 5.x**: 类型安全
- **Ant Design 5.x**: UI 组件库
- **Vite**: 构建工具
- **TanStack Query**: 数据获取和状态管理

### 数据库
- **SQLite**: 本地配置和任务记录存储

## 架构设计

### 整体架构

```
┌─────────────────────────────────────────────────┐
│                  前端 (React)                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐      │
│  │ 连接配置 │  │ 表选择   │  │ 任务管理 │      │
│  └──────────┘  └──────────┘  └──────────┘      │
└─────────────────────┬───────────────────────────┘
                      │ Tauri Commands
┌─────────────────────▼───────────────────────────┐
│              Rust 后端 (Tauri)                   │
│  ┌──────────────────────────────────────────┐  │
│  │         Command Layer (API)              │  │
│  └──────────────┬───────────────────────────┘  │
│  ┌──────────────▼───────────────────────────┐  │
│  │        Service Layer (业务逻辑)          │  │
│  │  - ConnectionService                     │  │
│  │  - MetadataService                       │  │
│  │  - SyncEngine                            │  │
│  │  - TaskManager                           │  │
│  └──────────────┬───────────────────────────┘  │
│  ┌──────────────▼───────────────────────────┐  │
│  │      Database Layer (SQLite)             │  │
│  └──────────────────────────────────────────┘  │
└──────┬──────────┬──────────┬──────────────────┘
       │          │          │
   ┌───▼──┐  ┌───▼──┐  ┌────▼─────┐
   │ MySQL│  │ RW   │  │StarRocks │
   └──────┘  └──────┘  └──────────┘
```

### 目录结构

```
rw_cdc_sr/
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # 入口文件
│   │   ├── commands/       # Tauri 命令层
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs   # 连接测试命令
│   │   │   ├── metadata.rs     # 元数据查询命令
│   │   │   ├── sync.rs         # 同步操作命令
│   │   │   └── task.rs         # 任务管理命令
│   │   ├── services/       # 业务逻辑层
│   │   │   ├── mod.rs
│   │   │   ├── connection_service.rs  # 数据库连接管理
│   │   │   ├── metadata_service.rs    # 元数据服务
│   │   │   ├── sync_engine.rs         # 同步引擎
│   │   │   └── task_manager.rs        # 任务管理器
│   │   ├── models/         # 数据模型
│   │   │   ├── mod.rs
│   │   │   ├── config.rs       # 配置模型
│   │   │   ├── table.rs        # 表结构模型
│   │   │   └── task.rs         # 任务模型
│   │   ├── db/             # 数据库层
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs       # 数据库表结构
│   │   │   └── repository.rs   # 数据访问层
│   │   ├── generators/     # SQL 生成器
│   │   │   ├── mod.rs
│   │   │   ├── risingwave_ddl.rs   # RisingWave DDL 生成
│   │   │   └── starrocks_ddl.rs    # StarRocks DDL 生成
│   │   └── utils/          # 工具函数
│   │       ├── mod.rs
│   │       ├── error.rs        # 错误定义
│   │       └── type_mapper.rs  # 类型映射
│   ├── Cargo.toml
│   └── tauri.conf.json     # Tauri 配置
├── src/                    # React 前端
│   ├── pages/
│   │   ├── ConnectionConfig.tsx    # 连接配置页
│   │   ├── TableSelection.tsx      # 表选择页
│   │   └── TaskManagement.tsx      # 任务管理页
│   ├── components/         # 通用组件
│   ├── services/           # API 调用
│   ├── hooks/              # 自定义 Hooks
│   └── types/              # TypeScript 类型
├── DESIGN.md               # 本设计文档
└── README.md               # 使用说明
```

## 核心功能模块

### 1. 连接管理模块

**功能**:
- MySQL 连接配置和测试
- RisingWave 连接配置和测试
- StarRocks 连接配置和测试
- 连接信息持久化存储

**数据模型**:
```rust
pub struct DatabaseConfig {
    pub id: i64,
    pub name: String,           // 配置名称
    pub db_type: DbType,        // MySQL/RisingWave/StarRocks
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,       // 加密存储
    pub database: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum DbType {
    MySQL,
    RisingWave,
    StarRocks,
}
```

### 2. 元数据服务模块

**功能**:
- 获取 MySQL 数据库列表
- 获取指定数据库的表列表
- 获取表结构（字段名、类型、约束等）
- 类型映射服务

**核心接口**:
```rust
pub trait MetadataService {
    async fn list_databases(&self, config: &DatabaseConfig) -> Result<Vec<String>>;
    async fn list_tables(&self, config: &DatabaseConfig, db: &str) -> Result<Vec<String>>;
    async fn get_table_schema(&self, config: &DatabaseConfig, db: &str, table: &str)
        -> Result<TableSchema>;
}
```

**TableSchema 模型**:
```rust
pub struct TableSchema {
    pub database: String,
    pub table_name: String,
    pub columns: Vec<Column>,
    pub primary_keys: Vec<String>,
    pub indexes: Vec<Index>,
}

pub struct Column {
    pub name: String,
    pub data_type: String,      // MySQL 类型
    pub is_nullable: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
}
```

### 3. SQL 生成器模块

#### 3.1 RisingWave CDC Source DDL 生成器

参考文档: https://docs.risingwave.com/ingestion/sources/mysql/mysql-cdc

**生成示例**:
```sql
CREATE SOURCE mysql_source WITH (
  connector = 'mysql-cdc',
  hostname = '127.0.0.1',
  port = '3306',
  username = 'root',
  password = 'password',
  database.name = 'mydb',
  server.id = '5454',
  table.name = 'orders'
);

CREATE TABLE orders_rw (
  order_id INT PRIMARY KEY,
  user_id INT,
  product_id INT,
  amount DECIMAL(10, 2),
  created_at TIMESTAMP
) FROM mysql_source TABLE 'orders';
```

**核心逻辑**:
1. 类型映射: MySQL → RisingWave (PostgreSQL)
2. 主键处理
3. 自动生成 server.id (避免冲突)

#### 3.2 StarRocks Sink DDL 生成器

参考文档: https://docs.risingwave.com/integrations/destinations/starrocks

**生成示例**:
```sql
-- 在 StarRocks 中创建表
CREATE TABLE orders_sr (
  order_id INT NOT NULL,
  user_id INT,
  product_id INT,
  amount DECIMAL(10, 2),
  created_at DATETIME
) ENGINE=OLAP
PRIMARY KEY(order_id)
DISTRIBUTED BY HASH(order_id) BUCKETS 10;

-- 在 RisingWave 中创建 Sink
CREATE SINK orders_sink FROM orders_rw
WITH (
  connector = 'starrocks',
  starrocks.host = '127.0.0.1',
  starrocks.mysqlport = '9030',
  starrocks.httpport = '8030',
  starrocks.user = 'root',
  starrocks.password = 'password',
  starrocks.database = 'demo',
  starrocks.table = 'orders_sr'
);
```

### 4. 同步引擎模块

**核心流程**:
```rust
pub struct SyncEngine {
    mysql_config: DatabaseConfig,
    rw_config: DatabaseConfig,
    sr_config: DatabaseConfig,
}

impl SyncEngine {
    pub async fn sync_table(&self, sync_request: SyncRequest) -> Result<SyncResult> {
        // 1. 获取 MySQL 表结构
        let schema = self.get_mysql_schema(&sync_request).await?;

        // 2. 在 RisingWave 中创建 CDC Source 和 Table
        let rw_ddl = self.generate_risingwave_ddl(&schema)?;
        self.execute_on_risingwave(&rw_ddl).await?;

        // 3. 在 StarRocks 中创建表
        let sr_table_ddl = self.generate_starrocks_table_ddl(&schema)?;
        self.execute_on_starrocks(&sr_table_ddl).await?;

        // 4. 在 RisingWave 中创建 Sink 到 StarRocks
        let sr_sink_ddl = self.generate_starrocks_sink_ddl(&schema)?;
        self.execute_on_risingwave(&sr_sink_ddl).await?;

        Ok(SyncResult::success())
    }
}
```

**SyncRequest 模型**:
```rust
pub struct SyncRequest {
    pub mysql_database: String,
    pub mysql_table: String,
    pub target_database: String,
    pub target_table: String,
    pub options: SyncOptions,
}

pub struct SyncOptions {
    pub recreate_rw_source: bool,   // 是否重建 RisingWave Source
    pub recreate_sr_table: bool,    // 是否重建 StarRocks 表
    pub truncate_sr_table: bool,    // 是否清空 StarRocks 表
}
```

### 5. 任务管理模块

**功能**:
- 记录每次同步任务
- 追踪任务状态
- 批量任务队列管理
- 任务历史查询

**数据模型**:
```rust
pub struct SyncTask {
    pub id: i64,
    pub task_name: String,
    pub mysql_table: String,
    pub status: TaskStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_by: String,
}

pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}
```

## 数据库设计 (SQLite)

### 表结构

```sql
-- 数据库配置表
CREATE TABLE database_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    db_type TEXT NOT NULL,  -- 'mysql', 'risingwave', 'starrocks'
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    username TEXT NOT NULL,
    password TEXT NOT NULL,  -- 加密存储
    database_name TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 同步任务表
CREATE TABLE sync_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_name TEXT NOT NULL,
    mysql_config_id INTEGER NOT NULL,
    rw_config_id INTEGER NOT NULL,
    sr_config_id INTEGER NOT NULL,
    mysql_database TEXT NOT NULL,
    mysql_table TEXT NOT NULL,
    target_database TEXT NOT NULL,
    target_table TEXT NOT NULL,
    status TEXT NOT NULL,  -- 'pending', 'running', 'completed', 'failed'
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    error_message TEXT,
    options TEXT,  -- JSON: SyncOptions
    FOREIGN KEY (mysql_config_id) REFERENCES database_configs(id),
    FOREIGN KEY (rw_config_id) REFERENCES database_configs(id),
    FOREIGN KEY (sr_config_id) REFERENCES database_configs(id)
);

-- 任务执行日志表
CREATE TABLE task_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    log_level TEXT NOT NULL,  -- 'info', 'warn', 'error'
    message TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (task_id) REFERENCES sync_tasks(id)
);
```

## 类型映射规则

### MySQL → RisingWave (PostgreSQL)

| MySQL 类型 | RisingWave 类型 | 说明 |
|-----------|----------------|------|
| TINYINT | SMALLINT | |
| SMALLINT | SMALLINT | |
| MEDIUMINT | INTEGER | |
| INT | INTEGER | |
| BIGINT | BIGINT | |
| FLOAT | REAL | |
| DOUBLE | DOUBLE PRECISION | |
| DECIMAL(p,s) | DECIMAL(p,s) | |
| CHAR(n) | CHAR(n) | |
| VARCHAR(n) | VARCHAR(n) | |
| TEXT | TEXT | |
| DATE | DATE | |
| DATETIME | TIMESTAMP | |
| TIMESTAMP | TIMESTAMP | |
| JSON | JSONB | |
| BINARY | BYTEA | |

### RisingWave → StarRocks

| RisingWave 类型 | StarRocks 类型 | 说明 |
|----------------|---------------|------|
| SMALLINT | SMALLINT | |
| INTEGER | INT | |
| BIGINT | BIGINT | |
| REAL | FLOAT | |
| DOUBLE PRECISION | DOUBLE | |
| DECIMAL(p,s) | DECIMAL(p,s) | |
| CHAR(n) | CHAR(n) | |
| VARCHAR(n) | VARCHAR(n) | |
| TEXT | STRING | |
| DATE | DATE | |
| TIMESTAMP | DATETIME | |
| JSONB | JSON | |

## 错误处理策略

1. **连接错误**: 友好提示，建议检查配置
2. **权限错误**: 提示需要的权限
3. **表已存在**: 根据 recreate 选项决定是否重建
4. **同步中断**: 记录错误日志，支持重试
5. **类型不兼容**: 提供详细的类型映射错误信息

## 安全考虑

1. **密码加密**: 使用 AES-256 加密存储数据库密码
2. **连接安全**: 支持 SSL/TLS 连接
3. **权限最小化**: 建议用户创建专用账号
4. **SQL 注入防护**: 使用参数化查询

## 性能优化

1. **连接池**: 复用数据库连接
2. **批量操作**: 支持批量选择多个表
3. **异步处理**: 所有 I/O 操作使用 async/await
4. **进度反馈**: 实时显示同步进度

## 后续扩展

1. **数据校验**: 同步后数据一致性校验
2. **增量更新**: 支持定时增量同步
3. **监控告警**: 同步失败告警
4. **多数据源**: 支持更多数据源类型
5. **权限管理**: 多用户支持

## 开发计划

### Phase 1: 基础框架 (完成度: 0%)
- [ ] Cargo.toml 依赖配置
- [ ] Tauri 项目初始化
- [ ] SQLite 数据库初始化
- [ ] 前端基础框架

### Phase 2: 连接管理 (完成度: 0%)
- [ ] 数据库连接服务
- [ ] 配置管理和持久化
- [ ] 连接测试功能

### Phase 3: 元数据服务 (完成度: 0%)
- [ ] MySQL 元数据读取
- [ ] 类型映射实现

### Phase 4: SQL 生成器 (完成度: 0%)
- [ ] RisingWave DDL 生成器
- [ ] StarRocks DDL 生成器

### Phase 5: 同步引擎 (完成度: 0%)
- [ ] 单表同步流程
- [ ] 批量同步和任务队列

### Phase 6: 前端界面 (完成度: 0%)
- [ ] 连接配置页面
- [ ] 表选择页面
- [ ] 任务管理页面

### Phase 7: 完善和优化 (完成度: 0%)
- [ ] 错误处理
- [ ] 日志系统
- [ ] 测试和文档
