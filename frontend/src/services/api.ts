// RW CDC SR - HTTP API Client
// 将所有 Tauri invoke 调用改为标准 HTTP fetch

import type {
  CreateConnectionRequest,
  TestConnectionRequest,
  ConnectionTestResult,
  DatabaseConfig,
  TableSchema,
  SyncRequest,
  SyncProgress,
  SyncTask,
  TaskHistoryQuery,
  TaskLog,
  PaginatedTasksResponse,
} from '../types';

// API 基础 URL（生产环境为空，开发环境通过 Vite 代理）
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || '';

/**
 * 通用 API fetch 函数
 */
async function apiFetch<T>(
  endpoint: string,
  options?: RequestInit
): Promise<T> {
  try {
    const response = await fetch(`${API_BASE_URL}${endpoint}`, {
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
      ...options,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({
        error: `HTTP ${response.status}: ${response.statusText}`,
      }));
      throw new Error(error.error || 'Request failed');
    }

    if (response.status === 204) {
      return null as T;
    }

    return response.json();
  } catch (err) {
    console.error('API Error:', err);
    throw err;
  }
}

// ============ 连接管理 ============

export const testMysqlConnection = async (
  request: TestConnectionRequest
): Promise<ConnectionTestResult> => {
  return apiFetch<ConnectionTestResult>('/api/connections/test/mysql', {
    method: 'POST',
    body: JSON.stringify(request),
  });
};

export const testRisingwaveConnection = async (
  request: TestConnectionRequest
): Promise<ConnectionTestResult> => {
  return apiFetch<ConnectionTestResult>('/api/connections/test/risingwave', {
    method: 'POST',
    body: JSON.stringify(request),
  });
};

export const testStarrocksConnection = async (
  request: TestConnectionRequest
): Promise<ConnectionTestResult> => {
  return apiFetch<ConnectionTestResult>('/api/connections/test/starrocks', {
    method: 'POST',
    body: JSON.stringify(request),
  });
};

export const saveConnectionConfig = async (
  request: CreateConnectionRequest
): Promise<number> => {
  const result = await apiFetch<{ id: number }>('/api/connections', {
    method: 'POST',
    body: JSON.stringify(request),
  });
  return result.id;
};

export const getAllConnections = async (): Promise<DatabaseConfig[]> => {
  return apiFetch<DatabaseConfig[]>('/api/connections');
};

export const deleteConnection = async (id: number): Promise<void> => {
  await apiFetch<{ success: boolean }>(`/api/connections/${id}`, {
    method: 'DELETE',
  });
};

export const updateConnectionConfig = async (
  id: number,
  request: CreateConnectionRequest
): Promise<void> => {
  await apiFetch<{ success: boolean }>(`/api/connections/${id}`, {
    method: 'PUT',
    body: JSON.stringify(request),
  });
};

// ============ 元数据 ============

export const listMysqlDatabases = async (configId: number): Promise<string[]> => {
  return apiFetch<string[]>('/api/metadata/databases', {
    method: 'POST',
    body: JSON.stringify({ config_id: configId }),
  });
};

export const listMysqlTables = async (
  configId: number,
  database: string
): Promise<string[]> => {
  return apiFetch<string[]>('/api/metadata/tables', {
    method: 'POST',
    body: JSON.stringify({
      config_id: configId,
      database,
    }),
  });
};

export const getTableSchema = async (
  configId: number,
  database: string,
  table: string
): Promise<TableSchema> => {
  return apiFetch<TableSchema>('/api/metadata/schema', {
    method: 'POST',
    body: JSON.stringify({
      config_id: configId,
      database,
      table,
    }),
  });
};

// ============ 同步 ============

export const syncSingleTable = async (request: SyncRequest): Promise<number> => {
  const result = await apiFetch<{ task_id: number }>('/api/sync/single', {
    method: 'POST',
    body: JSON.stringify(request),
  });
  return result.task_id;
};

export const syncMultipleTables = async (
  request: SyncRequest[]
): Promise<number[]> => {
  const result = await apiFetch<{ task_ids: number[] }>('/api/sync/multiple', {
    method: 'POST',
    body: JSON.stringify(request),
  });
  return result.task_ids;
};

export const getSyncProgress = async (taskId: number): Promise<SyncProgress> => {
  const task = await apiFetch<SyncTask>(`/api/sync/progress/${taskId}`);
  return task as unknown as SyncProgress;
};

export const retrySyncTask = async (taskId: number): Promise<number> => {
  const result = await apiFetch<{ task_id: number }>(`/api/sync/retry/${taskId}`, {
    method: 'POST',
  });
  return result.task_id;
};

// ============ 任务管理 ============

export const getTaskHistory = async (
  query: TaskHistoryQuery
): Promise<PaginatedTasksResponse> => {
  const params = new URLSearchParams();

  if (query.status) params.append('status', query.status);
  if (query.limit) params.append('limit', query.limit.toString());
  if (query.offset) params.append('offset', query.offset.toString());

  const queryString = params.toString();
  return apiFetch<PaginatedTasksResponse>(
    `/api/tasks/history${queryString ? `?${queryString}` : ''}`
  );
};

export const getTaskDetail = async (taskId: number): Promise<SyncTask> => {
  return apiFetch<SyncTask>(`/api/tasks/${taskId}`);
};

export const getTaskLogs = async (taskId: number): Promise<TaskLog[]> => {
  return apiFetch<TaskLog[]>(`/api/tasks/${taskId}/logs`);
};

export const cancelTask = async (taskId: number): Promise<void> => {
  await apiFetch<{ success: boolean }>(`/api/tasks/${taskId}/cancel`, {
    method: 'POST',
  });
};
