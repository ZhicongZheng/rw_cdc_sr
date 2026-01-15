// 数据库类型
export type DbType = 'mysql' | 'risingwave' | 'starrocks';

// 数据库配置
export interface DatabaseConfig {
  id: number;
  name: string;
  db_type: DbType;
  host: string;
  port: number;
  username: string;
  password: string;
  database_name?: string;
  created_at: string;
  updated_at: string;
}

// 创建连接请求
export interface CreateConnectionRequest {
  name: string;
  db_type: DbType;
  host: string;
  port: number;
  username: string;
  password: string;
  database_name?: string;
}

// 测试连接请求
export interface TestConnectionRequest {
  db_type: DbType;
  host: string;
  port: number;
  username: string;
  password: string;
  database_name?: string;
}

// 连接测试结果
export interface ConnectionTestResult {
  success: boolean;
  message: string;
  error?: string;
}

// 表列信息
export interface Column {
  name: string;
  data_type: string;
  is_nullable: boolean;
  default_value?: string;
  comment?: string;
  character_maximum_length?: number;
  numeric_precision?: number;
  numeric_scale?: number;
}

// 表索引信息
export interface Index {
  index_name: string;
  column_name: string;
  is_unique: boolean;
  seq_in_index: number;
}

// 表结构
export interface TableSchema {
  database: string;
  table_name: string;
  columns: Column[];
  primary_keys: string[];
  indexes: Index[];
}

// 任务状态
export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

// 同步选项
export interface SyncOptions {
  recreate_rw_source: boolean;
  recreate_sr_table: boolean;
  truncate_sr_table: boolean;
}

// 同步请求
export interface SyncRequest {
  mysql_config_id: number;
  rw_config_id: number;
  sr_config_id: number;
  mysql_database: string;
  mysql_table: string;
  target_database: string;
  target_table: string;
  options: SyncOptions;
}

// 批量同步请求
export interface BatchSyncRequest {
  mysql_config_id: number;
  rw_config_id: number;
  sr_config_id: number;
  tables: TableSyncInfo[];
  options: SyncOptions;
}

export interface TableSyncInfo {
  mysql_database: string;
  mysql_table: string;
  target_database: string;
  target_table: string;
}

// 同步任务
export interface SyncTask {
  id: number;
  task_name: string;
  mysql_config_id: number;
  rw_config_id: number;
  sr_config_id: number;
  mysql_database: string;
  mysql_table: string;
  target_database: string;
  target_table: string;
  status: TaskStatus;
  started_at: string;
  completed_at?: string;
  error_message?: string;
  options: string;
}

// 任务日志
export interface TaskLog {
  id: number;
  task_id: number;
  log_level: string;
  message: string;
  created_at: string;
}

// 同步进度
export interface SyncProgress {
  task_id: number;
  status: TaskStatus;
  current_step: string;
  total_steps: number;
  current_step_index: number;
  logs: string[];
}

// 任务历史查询
export interface TaskHistoryQuery {
  status?: TaskStatus;
  limit?: number;
  offset?: number;
}

// 分页任务响应
export interface PaginatedTasksResponse {
  tasks: SyncTask[];
  total: number;
  limit: number;
  offset: number;
}

// RisingWave 对象
export interface RwSchema {
  schema_name: string;
}

export interface RwSource {
  id: number;
  name: string;
  schema_name: string;
  owner: number;
  connector: string;
  columns: string[];
  definition?: string;
}

export interface RwTable {
  id: number;
  name: string;
  schema_name: string;
  owner: number;
  definition?: string;
}

export interface RwMaterializedView {
  id: number;
  name: string;
  schema_name: string;
  owner: number;
  definition?: string;
}

export interface RwSink {
  id: number;
  name: string;
  schema_name: string;
  owner: number;
  connector: string;
  target_table?: string;
  definition?: string;
}
