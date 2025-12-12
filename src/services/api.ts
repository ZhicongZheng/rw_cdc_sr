import { invoke } from '@tauri-apps/api/core';
import type {
  CreateConnectionRequest,
  TestConnectionRequest,
  ConnectionTestResult,
  DatabaseConfig,
  TableSchema,
  SyncRequest,
  BatchSyncRequest,
  SyncProgress,
  SyncTask,
  TaskHistoryQuery,
  TaskLog,
} from '../types';

// ============ 连接管理 ============

export const testMysqlConnection = async (
  request: TestConnectionRequest
): Promise<ConnectionTestResult> => {
  return invoke('test_mysql_connection', { request });
};

export const testRisingwaveConnection = async (
  request: TestConnectionRequest
): Promise<ConnectionTestResult> => {
  return invoke('test_risingwave_connection', { request });
};

export const testStarrocksConnection = async (
  request: TestConnectionRequest
): Promise<ConnectionTestResult> => {
  return invoke('test_starrocks_connection', { request });
};

export const saveConnectionConfig = async (
  request: CreateConnectionRequest
): Promise<number> => {
  return invoke('save_connection_config', { request });
};

export const getAllConnections = async (): Promise<DatabaseConfig[]> => {
  return invoke('get_all_connections');
};

export const deleteConnection = async (id: number): Promise<void> => {
  return invoke('delete_connection', { id });
};

export const updateConnectionConfig = async (
  id: number,
  request: CreateConnectionRequest
): Promise<void> => {
  return invoke('update_connection_config', { id, request });
};

// ============ 元数据 ============

export const listMysqlDatabases = async (configId: number): Promise<string[]> => {
  return invoke('list_mysql_databases', { configId });
};

export const listMysqlTables = async (
  configId: number,
  database: string
): Promise<string[]> => {
  return invoke('list_mysql_tables', { configId, database });
};

export const getTableSchema = async (
  configId: number,
  database: string,
  table: string
): Promise<TableSchema> => {
  return invoke('get_table_schema', { configId, database, table });
};

// ============ 同步 ============

export const syncSingleTable = async (request: SyncRequest): Promise<number> => {
  return invoke('sync_single_table', { request });
};

export const syncMultipleTables = async (
  request: BatchSyncRequest
): Promise<number[]> => {
  return invoke('sync_multiple_tables', { request });
};

export const getSyncProgress = async (taskId: number): Promise<SyncProgress> => {
  return invoke('get_sync_progress', { taskId });
};

export const retrySyncTask = async (taskId: number): Promise<number> => {
  return invoke('retry_sync_task', { taskId });
};

// ============ 任务管理 ============

export const getTaskHistory = async (
  query: TaskHistoryQuery
): Promise<SyncTask[]> => {
  return invoke('get_task_history', { query });
};

export const getTaskDetail = async (taskId: number): Promise<SyncTask> => {
  return invoke('get_task_detail', { taskId });
};

export const getTaskLogs = async (taskId: number): Promise<TaskLog[]> => {
  return invoke('get_task_logs', { taskId });
};

export const cancelTask = async (taskId: number): Promise<void> => {
  return invoke('cancel_task', { taskId });
};
