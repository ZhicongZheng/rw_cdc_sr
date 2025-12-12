# RisingWave CDC to StarRocks 使用指南

## 快速开始

### 1. 安装依赖

```bash
# 安装 Rust 依赖
cargo build

# 安装前端依赖
npm install
# 或
pnpm install
```

### 2. 开发模式运行

```bash
# 方式 1: 使用 Tauri CLI（推荐）
npm run dev

# 方式 2: 分别启动前后端
# 终端 1: 启动前端开发服务器
npm run dev

# 终端 2: 启动 Tauri
cargo tauri dev
```

### 3. 构建生产版本

```bash
# 构建前端
npm run build

# 构建 Tauri 应用
cargo tauri build
```

构建完成后，可执行文件位于：
- macOS: `target/release/bundle/macos/`
- Windows: `target/release/bundle/msi/`
- Linux: `target/release/bundle/appimage/` 或 `target/release/bundle/deb/`

## 使用流程

### 步骤 1: 配置数据库连接

1. 进入"连接配置"页面
2. 点击"新建连接"按钮
3. 填写数据库连接信息：
   - **MySQL**: 源数据库配置
   - **RisingWave**: 流处理引擎配置
   - **StarRocks**: 目标分析数据库配置
4. 点击"测试连接"验证配置
5. 点击"保存"保存连接配置

#### 默认端口参考

- MySQL: 3306
- RisingWave: 4566
- StarRocks: 9030

### 步骤 2: 选择表并同步

1. 进入"数据同步"页面

#### 2.1 选择源表
- 选择 MySQL 连接
- 选择数据库
- 选择要同步的表（支持多选）
- 配置目标数据库和表名

#### 2.2 选择目标
- 选择 RisingWave 连接
- 选择 StarRocks 连接

#### 2.3 配置同步选项
- **重建 RisingWave Source**: 删除并重新创建 CDC Source 和 Table
- **重建 StarRocks 表**: 删除并重新创建目标表
- **清空 StarRocks 表**: 仅清空表数据，保留表结构

#### 2.4 开始同步
- 点击"开始同步"按钮
- 系统将自动创建同步任务

### 步骤 3: 查看任务状态

1. 进入"任务管理"页面
2. 查看所有同步任务的状态
3. 点击"详情"查看任务详细信息和日志
4. 对于运行中的任务，可以点击"取消"停止任务

#### 任务状态说明

- 🟦 **等待中**: 任务已创建，等待执行
- 🔵 **执行中**: 任务正在执行
- 🟢 **已完成**: 任务执行成功
- 🔴 **失败**: 任务执行失败
- 🟠 **已取消**: 任务被用户取消

## 同步原理

### 数据流向

```
MySQL (源数据)
    ↓ [Binlog CDC]
RisingWave CDC Source
    ↓ [实时流处理]
RisingWave Table
    ↓ [Sink Connector]
StarRocks Table (目标数据)
```

### 自动执行的操作

工具会自动执行以下操作：

1. **读取 MySQL 表结构**
   - 获取列定义、数据类型
   - 获取主键信息
   - 获取索引信息

2. **在 RisingWave 中创建 CDC Source**
   ```sql
   CREATE SOURCE mysql_source WITH (
     connector = 'mysql-cdc',
     hostname = '...',
     port = '...',
     database.name = '...',
     table.name = '...'
   );
   ```

3. **在 RisingWave 中创建 Table**
   ```sql
   CREATE TABLE target_table (
     -- 自动映射的列定义
   ) FROM mysql_source TABLE 'source_table';
   ```

4. **在 StarRocks 中创建表**
   ```sql
   CREATE TABLE target_table (
     -- 自动映射的列定义
   ) ENGINE=OLAP
   PRIMARY KEY(...)
   DISTRIBUTED BY HASH(...);
   ```

5. **在 RisingWave 中创建 Sink**
   ```sql
   CREATE SINK starrocks_sink FROM target_table
   WITH (
     connector = 'starrocks',
     starrocks.host = '...',
     starrocks.table = '...'
   );
   ```

## 类型映射规则

### MySQL → RisingWave

| MySQL      | RisingWave        |
|------------|-------------------|
| TINYINT    | SMALLINT          |
| INT        | INTEGER           |
| BIGINT     | BIGINT            |
| FLOAT      | REAL              |
| DOUBLE     | DOUBLE PRECISION  |
| DECIMAL    | DECIMAL           |
| VARCHAR    | VARCHAR           |
| TEXT       | TEXT              |
| DATE       | DATE              |
| DATETIME   | TIMESTAMP         |
| JSON       | JSONB             |

### RisingWave → StarRocks

| RisingWave        | StarRocks  |
|-------------------|------------|
| SMALLINT          | SMALLINT   |
| INTEGER           | INT        |
| BIGINT            | BIGINT     |
| REAL              | FLOAT      |
| DOUBLE PRECISION  | DOUBLE     |
| DECIMAL           | DECIMAL    |
| VARCHAR           | VARCHAR    |
| TEXT              | STRING     |
| DATE              | DATE       |
| TIMESTAMP         | DATETIME   |
| JSONB             | JSON       |

## 常见问题

### Q: 密码如何存储？
A: 所有数据库密码使用 AES-256-GCM 加密存储在本地 SQLite 数据库中。

### Q: 支持哪些操作系统？
A: 支持 macOS、Windows 和 Linux。

### Q: 可以同时同步多个表吗？
A: 可以，在"数据同步"页面选择多个表即可批量同步。

### Q: 同步失败怎么办？
A: 在"任务管理"页面查看失败任务的详细错误信息和日志，根据错误提示进行排查。

### Q: 如何重新同步数据？
A: 在配置同步选项时，勾选"重建 StarRocks 表"或"清空 StarRocks 表"选项。

### Q: RisingWave 需要什么权限？
A: 需要创建 Source、Table 和 Sink 的权限。

### Q: MySQL 需要开启什么配置？
A: 需要开启 binlog，并设置 `binlog_format=ROW`。

## 数据库配置要求

### MySQL 配置

```ini
[mysqld]
# 开启 binlog
log-bin=mysql-bin
binlog_format=ROW
binlog_row_image=FULL
```

### RisingWave 配置

需要确保 RisingWave 可以访问 MySQL 服务器。

### StarRocks 配置

确保 StarRocks FE 和 BE 正常运行。

## 故障排查

### 连接测试失败

1. 检查网络连通性
2. 检查防火墙设置
3. 验证用户名和密码
4. 确认端口号正确

### 同步任务失败

1. 查看任务日志，了解详细错误信息
2. 检查 MySQL binlog 是否开启
3. 检查 RisingWave 是否正常运行
4. 检查 StarRocks 是否有足够的资源

### 类型映射错误

如果遇到不支持的数据类型，请查阅设计文档中的类型映射表，或联系开发者。

## 技术支持

如有问题，请提交 GitHub Issue。
