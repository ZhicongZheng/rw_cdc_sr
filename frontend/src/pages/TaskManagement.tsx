import React, { useState, useEffect } from 'react';
import {
  Card,
  Table,
  Tag,
  Button,
  Modal,
  Descriptions,
  Timeline,
  Select,
  Space,
  message,
  Tooltip,
  Progress,
} from 'antd';
import {
  ReloadOutlined,
  EyeOutlined,
  CloseCircleOutlined,
  CheckCircleOutlined,
  SyncOutlined,
  ExclamationCircleOutlined,
  MinusCircleOutlined,
  RedoOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import type { SyncTask, TaskStatus, TaskLog, SyncProgress } from '../types';
import * as api from '../services/api';

const TaskManagement: React.FC = () => {
  const [tasks, setTasks] = useState<SyncTask[]>([]);
  const [loading, setLoading] = useState(false);
  const [statusFilter, setStatusFilter] = useState<TaskStatus | undefined>();
  const [detailModalVisible, setDetailModalVisible] = useState(false);
  const [selectedTask, setSelectedTask] = useState<SyncTask | null>(null);
  const [taskLogs, setTaskLogs] = useState<TaskLog[]>([]);
  const [progress, setProgress] = useState<SyncProgress | null>(null);

  // 加载任务列表
  const loadTasks = async () => {
    setLoading(true);
    try {
      const data = await api.getTaskHistory({
        status: statusFilter,
        limit: 100,
        offset: 0,
      });
      setTasks(data);
    } catch (error) {
      message.error('加载任务列表失败: ' + error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadTasks();
    // 每5秒自动刷新
    const interval = setInterval(loadTasks, 5000);
    return () => clearInterval(interval);
  }, [statusFilter]);

  // 查看任务详情
  const handleViewDetail = async (task: SyncTask) => {
    setSelectedTask(task);
    setDetailModalVisible(true);

    try {
      const logs = await api.getTaskLogs(task.id);
      setTaskLogs(logs);

      if (task.status === 'running') {
        const prog = await api.getSyncProgress(task.id);
        setProgress(prog);
      }
    } catch (error) {
      message.error('加载任务详情失败: ' + error);
    }
  };

  // 取消任务
  const handleCancelTask = async (taskId: number) => {
    try {
      await api.cancelTask(taskId);
      message.success('任务已取消');
      loadTasks();
    } catch (error) {
      message.error('取消任务失败: ' + error);
    }
  };

  // 重试任务
  const handleRetryTask = async (taskId: number) => {
    try {
      const newTaskId = await api.retrySyncTask(taskId);
      message.success(`任务已重新提交，新任务 ID: ${newTaskId}`);
      loadTasks();
    } catch (error) {
      message.error('重试任务失败: ' + error);
    }
  };

  // 获取状态标签
  const getStatusTag = (status: TaskStatus) => {
    const statusConfig = {
      pending: { color: 'default', icon: <MinusCircleOutlined />, text: '等待中' },
      running: { color: 'blue', icon: <SyncOutlined spin />, text: '执行中' },
      completed: {
        color: 'success',
        icon: <CheckCircleOutlined />,
        text: '已完成',
      },
      failed: { color: 'error', icon: <CloseCircleOutlined />, text: '失败' },
      cancelled: {
        color: 'warning',
        icon: <ExclamationCircleOutlined />,
        text: '已取消',
      },
    };

    const config = statusConfig[status];
    return (
      <Tag color={config.color} icon={config.icon}>
        {config.text}
      </Tag>
    );
  };

  // 格式化时间
  const formatTime = (time?: string) => {
    if (!time) return '-';
    return new Date(time).toLocaleString('zh-CN');
  };

  // 计算耗时
  const calculateDuration = (start: string, end?: string) => {
    const startTime = new Date(start).getTime();
    const endTime = end ? new Date(end).getTime() : Date.now();
    const duration = Math.floor((endTime - startTime) / 1000);

    if (duration < 60) return `${duration}秒`;
    if (duration < 3600) return `${Math.floor(duration / 60)}分${duration % 60}秒`;
    return `${Math.floor(duration / 3600)}时${Math.floor((duration % 3600) / 60)}分`;
  };

  // 表格列定义
  const columns: ColumnsType<SyncTask> = [
    {
      title: 'ID',
      dataIndex: 'id',
      key: 'id',
      width: 80,
    },
    {
      title: '任务名称',
      dataIndex: 'task_name',
      key: 'task_name',
      ellipsis: true,
    },
    {
      title: '源表',
      key: 'source',
      render: (_, record) => (
        <Tooltip title={`${record.mysql_database}.${record.mysql_table}`}>
          <span>{record.mysql_database}.{record.mysql_table}</span>
        </Tooltip>
      ),
    },
    {
      title: '目标表',
      key: 'target',
      render: (_, record) => (
        <Tooltip title={`${record.target_database}.${record.target_table}`}>
          <span>{record.target_database}.{record.target_table}</span>
        </Tooltip>
      ),
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 100,
      render: (status: TaskStatus) => getStatusTag(status),
    },
    {
      title: '开始时间',
      dataIndex: 'started_at',
      key: 'started_at',
      width: 180,
      render: formatTime,
    },
    {
      title: '耗时',
      key: 'duration',
      width: 100,
      render: (_, record) =>
        calculateDuration(record.started_at, record.completed_at),
    },
    {
      title: '操作',
      key: 'action',
      width: 150,
      render: (_, record) => (
        <Space>
          <Button
            type="link"
            size="small"
            icon={<EyeOutlined />}
            onClick={() => handleViewDetail(record)}
          >
            详情
          </Button>
          {record.status === 'running' && (
            <Button
              type="link"
              size="small"
              danger
              onClick={() => handleCancelTask(record.id)}
            >
              取消
            </Button>
          )}
          {record.status === 'failed' && (
            <Button
              type="link"
              size="small"
              icon={<RedoOutlined />}
              onClick={() => handleRetryTask(record.id)}
            >
              重试
            </Button>
          )}
        </Space>
      ),
    },
  ];

  return (
    <div>
      <Card
        title="同步任务管理"
        extra={
          <Space>
            <Select
              style={{ width: 120 }}
              placeholder="任务状态"
              allowClear
              value={statusFilter}
              onChange={setStatusFilter}
            >
              <Select.Option value="pending">等待中</Select.Option>
              <Select.Option value="running">执行中</Select.Option>
              <Select.Option value="completed">已完成</Select.Option>
              <Select.Option value="failed">失败</Select.Option>
              <Select.Option value="cancelled">已取消</Select.Option>
            </Select>
            <Button icon={<ReloadOutlined />} onClick={loadTasks}>
              刷新
            </Button>
          </Space>
        }
      >
        <Table
          columns={columns}
          dataSource={tasks}
          rowKey="id"
          loading={loading}
          pagination={{ pageSize: 20 }}
        />
      </Card>

      {/* 任务详情 Modal */}
      <Modal
        title="任务详情"
        open={detailModalVisible}
        onCancel={() => {
          setDetailModalVisible(false);
          setSelectedTask(null);
          setTaskLogs([]);
          setProgress(null);
        }}
        footer={null}
        width={800}
      >
        {selectedTask && (
          <Space direction="vertical" style={{ width: '100%' }} size="large">
            <Descriptions bordered column={2} size="small">
              <Descriptions.Item label="任务 ID">{selectedTask.id}</Descriptions.Item>
              <Descriptions.Item label="状态">
                {getStatusTag(selectedTask.status)}
              </Descriptions.Item>
              <Descriptions.Item label="任务名称" span={2}>
                {selectedTask.task_name}
              </Descriptions.Item>
              <Descriptions.Item label="MySQL 表">
                {selectedTask.mysql_database}.{selectedTask.mysql_table}
              </Descriptions.Item>
              <Descriptions.Item label="StarRocks 表">
                {selectedTask.target_database}.{selectedTask.target_table}
              </Descriptions.Item>
              <Descriptions.Item label="开始时间">
                {formatTime(selectedTask.started_at)}
              </Descriptions.Item>
              <Descriptions.Item label="完成时间">
                {formatTime(selectedTask.completed_at)}
              </Descriptions.Item>
              <Descriptions.Item label="耗时" span={2}>
                {calculateDuration(
                  selectedTask.started_at,
                  selectedTask.completed_at
                )}
              </Descriptions.Item>
              {selectedTask.error_message && (
                <Descriptions.Item label="错误信息" span={2}>
                  <span style={{ color: 'red' }}>{selectedTask.error_message}</span>
                </Descriptions.Item>
              )}
            </Descriptions>

            {progress && selectedTask.status === 'running' && (
              <Card title="同步进度" size="small">
                <Progress
                  percent={Math.floor(
                    (progress.current_step_index / progress.total_steps) * 100
                  )}
                  status="active"
                />
                <p style={{ marginTop: 16 }}>
                  当前步骤: {progress.current_step}
                </p>
              </Card>
            )}

            <Card title="执行日志" size="small">
              {taskLogs.length > 0 ? (
                <Timeline
                  items={taskLogs.map((log) => ({
                    color:
                      log.log_level === 'error'
                        ? 'red'
                        : log.log_level === 'warn'
                        ? 'orange'
                        : 'blue',
                    children: (
                      <div>
                        <div style={{ fontSize: '12px', color: '#999' }}>
                          {formatTime(log.created_at)}
                        </div>
                        <div>{log.message}</div>
                      </div>
                    ),
                  }))}
                />
              ) : (
                <p style={{ textAlign: 'center', color: '#999' }}>暂无日志</p>
              )}
            </Card>
          </Space>
        )}
      </Modal>
    </div>
  );
};

export default TaskManagement;
