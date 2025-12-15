import React, { useState, useEffect } from 'react';
import {
  Card,
  Button,
  Table,
  Modal,
  Form,
  Input,
  Select,
  InputNumber,
  message,
  Space,
  Tag,
  Popconfirm,
} from 'antd';
import { PlusOutlined, DeleteOutlined, CheckCircleOutlined, EditOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import type {
  DatabaseConfig,
  CreateConnectionRequest,
  TestConnectionRequest,
  DbType,
} from '../types';
import * as api from '../services/api';

const ConnectionConfig: React.FC = () => {
  const [connections, setConnections] = useState<DatabaseConfig[]>([]);
  const [loading, setLoading] = useState(false);
  const [modalVisible, setModalVisible] = useState(false);
  const [testLoading, setTestLoading] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [form] = Form.useForm();

  // 加载连接列表
  const loadConnections = async () => {
    setLoading(true);
    try {
      const data = await api.getAllConnections();
      setConnections(data);
    } catch (error) {
      message.error('加载连接配置失败: ' + error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadConnections();
  }, []);

  // 打开新建对话框
  const handleOpenCreate = () => {
    setEditingId(null);
    form.resetFields();
    setModalVisible(true);
  };

  // 打开编辑对话框
  const handleOpenEdit = (record: DatabaseConfig) => {
    setEditingId(record.id);
    form.setFieldsValue({
      name: record.name,
      db_type: record.db_type,
      host: record.host,
      port: record.port,
      username: record.username,
      password: record.password,
      database_name: record.database_name,
    });
    setModalVisible(true);
  };

  // 测试连接
  const handleTestConnection = async () => {
    try {
      await form.validateFields();
      const values = form.getFieldsValue();
      setTestLoading(true);

      const request: TestConnectionRequest = {
        db_type: values.db_type,
        host: values.host,
        port: values.port,
        username: values.username,
        password: values.password,
        database_name: values.database_name,
      };

      let result;
      if (values.db_type === 'mysql') {
        result = await api.testMysqlConnection(request);
      } else if (values.db_type === 'risingwave') {
        result = await api.testRisingwaveConnection(request);
      } else {
        result = await api.testStarrocksConnection(request);
      }

      if (result.success) {
        message.success('连接测试成功！');
      } else {
        message.error('连接测试失败: ' + (result.error || result.message));
      }
    } catch (error: any) {
      if (error.errorFields) {
        message.warning('请填写完整的连接信息');
      } else {
        message.error('连接测试失败: ' + error);
      }
    } finally {
      setTestLoading(false);
    }
  };

  // 保存连接
  const handleSaveConnection = async () => {
    try {
      const values = await form.validateFields();
      setLoading(true);

      const request: CreateConnectionRequest = {
        name: values.name,
        db_type: values.db_type,
        host: values.host,
        port: values.port,
        username: values.username,
        password: values.password,
        database_name: values.database_name,
      };

      if (editingId) {
        // 编辑模式
        await api.updateConnectionConfig(editingId, request);
        message.success('更新成功！');
      } else {
        // 新建模式
        await api.saveConnectionConfig(request);
        message.success('保存成功！');
      }

      setModalVisible(false);
      form.resetFields();
      setEditingId(null);
      loadConnections();
    } catch (error: any) {
      if (error.errorFields) {
        message.warning('请填写完整的连接信息');
      } else {
        message.error((editingId ? '更新' : '保存') + '失败: ' + error);
      }
    } finally {
      setLoading(false);
    }
  };

  // 删除连接
  const handleDeleteConnection = async (id: number) => {
    try {
      await api.deleteConnection(id);
      message.success('删除成功！');
      loadConnections();
    } catch (error) {
      message.error('删除失败: ' + error);
    }
  };

  // 获取数据库类型标签颜色
  const getDbTypeColor = (type: DbType): string => {
    switch (type) {
      case 'mysql':
        return 'blue';
      case 'risingwave':
        return 'green';
      case 'starrocks':
        return 'orange';
      default:
        return 'default';
    }
  };

  // 表格列定义
  const columns: ColumnsType<DatabaseConfig> = [
    {
      title: '名称',
      dataIndex: 'name',
      key: 'name',
    },
    {
      title: '类型',
      dataIndex: 'db_type',
      key: 'db_type',
      render: (type: DbType) => (
        <Tag color={getDbTypeColor(type)}>{type.toUpperCase()}</Tag>
      ),
    },
    {
      title: '主机',
      dataIndex: 'host',
      key: 'host',
    },
    {
      title: '端口',
      dataIndex: 'port',
      key: 'port',
    },
    {
      title: '用户名',
      dataIndex: 'username',
      key: 'username',
    },
    {
      title: '数据库',
      dataIndex: 'database_name',
      key: 'database_name',
      render: (name?: string) => name || '-',
    },
    {
      title: '操作',
      key: 'action',
      render: (_, record) => (
        <Space>
          <Button
            type="link"
            icon={<EditOutlined />}
            onClick={() => handleOpenEdit(record)}
          >
            编辑
          </Button>
          <Popconfirm
            title="确定要删除此连接吗？"
            onConfirm={() => handleDeleteConnection(record.id)}
            okText="确定"
            cancelText="取消"
          >
            <Button type="link" danger icon={<DeleteOutlined />}>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  // 获取默认端口
  const getDefaultPort = (dbType: DbType): number => {
    switch (dbType) {
      case 'mysql':
        return 3306;
      case 'risingwave':
        return 4566;
      case 'starrocks':
        return 9030;
      default:
        return 3306;
    }
  };

  return (
    <div>
      <Card
        title="数据库连接配置"
        extra={
          <Button
            type="primary"
            icon={<PlusOutlined />}
            onClick={handleOpenCreate}
          >
            新建连接
          </Button>
        }
      >
        <Table
          columns={columns}
          dataSource={connections}
          rowKey="id"
          loading={loading}
          pagination={{ pageSize: 10 }}
        />
      </Card>

      <Modal
        title={editingId ? "编辑数据库连接" : "新建数据库连接"}
        open={modalVisible}
        onCancel={() => {
          setModalVisible(false);
          form.resetFields();
          setEditingId(null);
        }}
        footer={[
          <Button
            key="cancel"
            onClick={() => {
              setModalVisible(false);
              form.resetFields();
              setEditingId(null);
            }}
          >
            取消
          </Button>,
          <Button
            key="test"
            icon={<CheckCircleOutlined />}
            loading={testLoading}
            onClick={handleTestConnection}
          >
            测试连接
          </Button>,
          <Button
            key="save"
            type="primary"
            loading={loading}
            onClick={handleSaveConnection}
          >
            {editingId ? "更新" : "保存"}
          </Button>,
        ]}
        width={600}
      >
        <Form
          form={form}
          layout="vertical"
          initialValues={{
            db_type: 'mysql',
            port: 3306,
          }}
        >
          <Form.Item
            label="连接名称"
            name="name"
            rules={[{ required: true, message: '请输入连接名称' }]}
          >
            <Input placeholder="例如：生产环境 MySQL" />
          </Form.Item>

          <Form.Item
            label="数据库类型"
            name="db_type"
            rules={[{ required: true, message: '请选择数据库类型' }]}
          >
            <Select
              onChange={(value: DbType) => {
                form.setFieldValue('port', getDefaultPort(value));
              }}
            >
              <Select.Option value="mysql">MySQL</Select.Option>
              <Select.Option value="risingwave">RisingWave</Select.Option>
              <Select.Option value="starrocks">StarRocks</Select.Option>
            </Select>
          </Form.Item>

          <Space.Compact style={{ width: '100%' }}>
            <Form.Item
              label="主机地址"
              name="host"
              rules={[{ required: true, message: '请输入主机地址' }]}
              style={{ flex: 1, marginBottom: 0 }}
            >
              <Input placeholder="localhost 或 IP 地址" />
            </Form.Item>

            <Form.Item
              label="端口"
              name="port"
              rules={[{ required: true, message: '请输入端口' }]}
              style={{ width: 120, marginBottom: 0 }}
            >
              <InputNumber min={1} max={65535} style={{ width: '100%' }} />
            </Form.Item>
          </Space.Compact>

          <Form.Item
            label="用户名"
            name="username"
            rules={[{ required: true, message: '请输入用户名' }]}
            style={{ marginTop: 24 }}
          >
            <Input placeholder="数据库用户名" />
          </Form.Item>

          <Form.Item
            label="密码"
            name="password"
            rules={[{ required: true, message: '请输入密码' }]}
          >
            <Input.Password placeholder="数据库密码" />
          </Form.Item>

          <Form.Item label="数据库名称（可选）" name="database_name">
            <Input placeholder="默认数据库名称" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default ConnectionConfig;
