import React, { useState, useEffect } from "react";
import {
  Card,
  Select,
  Tabs,
  Table,
  message,
  Spin,
  Typography,
  Space,
  Tag,
  Button,
  Modal,
  Popconfirm,
  Form,
  Input,
} from "antd";
import { EyeOutlined, CopyOutlined, DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import type { ColumnsType } from "antd/es/table";
import * as api from "../services/api";
import type {
  DatabaseConfig,
  RwSchema,
  RwSource,
  RwTable,
  RwMaterializedView,
  RwSink,
} from "../types";

const { Title, Paragraph } = Typography;
const { TabPane } = Tabs;

// 防抖 Hook：延迟更新值
function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      clearTimeout(handler);
    };
  }, [value, delay]);

  return debouncedValue;
}

// Tab 状态接口
interface TabState<T> {
  data: T[];
  total: number;
  currentPage: number;
  pageSize: number;
  search: string;
  loading: boolean;
}

const RisingWaveManager: React.FC = () => {
  const [rwConnections, setRwConnections] = useState<DatabaseConfig[]>([]);
  const [selectedRwId, setSelectedRwId] = useState<number | null>(null);
  const [schemas, setSchemas] = useState<RwSchema[]>([]);
  const [selectedSchema, setSelectedSchema] = useState<string>("public");

  // Tab state management
  const [sourcesState, setSourcesState] = useState<TabState<RwSource>>({
    data: [],
    total: 0,
    currentPage: 1,
    pageSize: 20,
    search: "",
    loading: false,
  });

  const [tablesState, setTablesState] = useState<TabState<RwTable>>({
    data: [],
    total: 0,
    currentPage: 1,
    pageSize: 20,
    search: "",
    loading: false,
  });

  const [mvsState, setMvsState] = useState<TabState<RwMaterializedView>>({
    data: [],
    total: 0,
    currentPage: 1,
    pageSize: 20,
    search: "",
    loading: false,
  });

  const [sinksState, setSinksState] = useState<TabState<RwSink>>({
    data: [],
    total: 0,
    currentPage: 1,
    pageSize: 20,
    search: "",
    loading: false,
  });

  // Modal states for viewing SQL definitions
  const [sqlModalVisible, setSqlModalVisible] = useState(false);
  const [sqlModalContent, setSqlModalContent] = useState("");
  const [sqlModalTitle, setSqlModalTitle] = useState("");

  // Row selection states
  const [selectedSourceKeys, setSelectedSourceKeys] = useState<React.Key[]>([]);
  const [selectedTableKeys, setSelectedTableKeys] = useState<React.Key[]>([]);
  const [selectedMvKeys, setSelectedMvKeys] = useState<React.Key[]>([]);
  const [selectedSinkKeys, setSelectedSinkKeys] = useState<React.Key[]>([]);

  // Create Sink modal states
  const [createSinkModalVisible, setCreateSinkModalVisible] = useState(false);
  const [createSinkLoading, setCreateSinkLoading] = useState(false);
  const [srConnections, setSrConnections] = useState<DatabaseConfig[]>([]);
  const [sinkForm] = Form.useForm();
  const [currentSinkSource, setCurrentSinkSource] = useState<{
    name: string;
    type: 'table' | 'materialized_view';
  } | null>(null);

  // Load RisingWave connections on mount
  useEffect(() => {
    loadConnections();
  }, []);

  // Load schemas when connection changes
  useEffect(() => {
    if (selectedRwId) {
      loadSchemas();
    }
  }, [selectedRwId]);

  // Load objects when schema changes - reset all states
  useEffect(() => {
    if (selectedRwId && selectedSchema) {
      resetAllTabStates();
       }
  }, [selectedRwId, selectedSchema]);

  // Debounced search values for each tab
  const debouncedSourcesSearch = useDebounce(sourcesState.search, 500);
  const debouncedTablesSearch = useDebounce(tablesState.search, 500);
  const debouncedMvsSearch = useDebounce(mvsState.search, 500);
  const debouncedSinksSearch = useDebounce(sinksState.search, 500);

  // Load sources when its state changes (using debounced search)
  useEffect(() => {
    if (selectedRwId && selectedSchema) {
      loadSources();
    }
  }, [selectedRwId, selectedSchema, sourcesState.currentPage, sourcesState.pageSize, debouncedSourcesSearch]);

  // Load tables when its state changes (using debounced search)
  useEffect(() => {
    if (selectedRwId && selectedSchema) {
      loadTables();
    }
  }, [selectedRwId, selectedSchema, tablesState.currentPage, tablesState.pageSize, debouncedTablesSearch]);

  // Load materialized views when its state changes (using debounced search)
  useEffect(() => {
    if (selectedRwId && selectedSchema) {
      loadMaterializedViews();
    }
  }, [selectedRwId, selectedSchema, mvsState.currentPage, mvsState.pageSize, debouncedMvsSearch]);

  // Load sinks when its state changes (using debounced search)
  useEffect(() => {
    if (selectedRwId && selectedSchema) {
      loadSinks();
    }
  }, [selectedRwId, selectedSchema, sinksState.currentPage, sinksState.pageSize, debouncedSinksSearch]);

  const loadConnections = async () => {
    try {
      const conns = await api.getAllConnections();
      const rwConns = conns.filter((c) => c.db_type === "risingwave");
      const srConns = conns.filter((c) => c.db_type === "starrocks");

      setRwConnections(rwConns);
      setSrConnections(srConns);

      if (rwConns.length > 0) {
        setSelectedRwId(rwConns[0].id);
      }
    } catch (error) {
      message.error("加载连接失败: " + error);
    }
  };

  const loadSchemas = async () => {
    if (!selectedRwId) return;

    try {
      const schemaList = await api.listRwSchemas(selectedRwId);
      setSchemas(schemaList);
      if (schemaList.length > 0) {
        const publicSchema = schemaList.find((s) => s.schema_name === "public");
        setSelectedSchema(publicSchema ? "public" : schemaList[0].schema_name);
      }
    } catch (error) {
      message.error("加载 Schema 失败: " + error);
    }
  };

  const resetAllTabStates = () => {
    setSourcesState(prev => ({
      ...prev,
      currentPage: 1,
      search: "",
    }));
    setTablesState(prev => ({
      ...prev,
      currentPage: 1,
      search: "",
    }));
    setMvsState(prev => ({
      ...prev,
      currentPage: 1,
      search: "",
    }));
    setSinksState(prev => ({
      ...prev,
      currentPage: 1,
      search: "",
    }));
  };

  const loadSources = async () => {
    if (!selectedRwId || !selectedSchema) return;

    setSourcesState(prev => ({ ...prev, loading: true }));
    try {
      const offset = (sourcesState.currentPage - 1) * sourcesState.pageSize;
      const response = await api.listRwSources(
        selectedRwId,
        selectedSchema,
        sourcesState.search || undefined,
        sourcesState.pageSize,
        offset
      );

      setSourcesState(prev => ({
        ...prev,
        data: response.data,
        total: response.total,
        loading: false,
      }));
    } catch (error) {
      message.error("加载 Sources 失败: " + error);
      setSourcesState(prev => ({ ...prev, loading: false }));
    }
  };

  const loadTables = async () => {
    if (!selectedRwId || !selectedSchema) return;

    setTablesState(prev => ({ ...prev, loading: true }));
    try {
      const offset = (tablesState.currentPage - 1) * tablesState.pageSize;
      const response = await api.listRwTables(
        selectedRwId,
        selectedSchema,
        tablesState.search || undefined,
        tablesState.pageSize,
        offset
      );

      setTablesState(prev => ({
        ...prev,
        data: response.data,
        total: response.total,
        loading: false,
      }));
    } catch (error) {
      message.error("加载 Tables 失败: " + error);
      setTablesState(prev => ({ ...prev, loading: false }));
    }
  };

  const loadMaterializedViews = async () => {
    if (!selectedRwId || !selectedSchema) return;

    setMvsState(prev => ({ ...prev, loading: true }));
    try {
      const offset = (mvsState.currentPage - 1) * mvsState.pageSize;
      const response = await api.listRwMaterializedViews(
        selectedRwId,
        selectedSchema,
        mvsState.search || undefined,
        mvsState.pageSize,
        offset
      );

      setMvsState(prev => ({
        ...prev,
        data: response.data,
        total: response.total,
        loading: false,
      }));
    } catch (error) {
      message.error("加载 Materialized Views 失败: " + error);
      setMvsState(prev => ({ ...prev, loading: false }));
    }
  };

  const loadSinks = async () => {
    if (!selectedRwId || !selectedSchema) return;

    setSinksState(prev => ({ ...prev, loading: true }));
    try {
      const offset = (sinksState.currentPage - 1) * sinksState.pageSize;
      const response = await api.listRwSinks(
        selectedRwId,
        selectedSchema,
        sinksState.search || undefined,
        sinksState.pageSize,
        offset
      );

      setSinksState(prev => ({
        ...prev,
        data: response.data,
        total: response.total,
        loading: false,
      }));
    } catch (error) {
      message.error("加载 Sinks 失败: " + error);
      setSinksState(prev => ({ ...prev, loading: false }));
    }
  };

  // Show SQL definition in modal
  const showSqlModal = (title: string, sql: string) => {
    setSqlModalTitle(title);
    setSqlModalContent(sql);
    setSqlModalVisible(true);
  };

  // Copy SQL to clipboard
  const copySql = (sql: string) => {
    navigator.clipboard.writeText(sql).then(
      () => {
        message.success("已复制到剪贴板");
      },
      () => {
        message.error("复制失败");
      }
    );
  };

  // Delete handlers
  const handleDeleteSource = async (name: string, schemaName: string) => {
    if (!selectedRwId) return;
    try {
      await api.deleteRwSource(selectedRwId, schemaName, name);
      message.success(`已删除 Source: ${name}`);
      loadSources();
    } catch (error) {
      message.error("删除失败: " + error);
    }
  };

  const handleDeleteTable = async (name: string, schemaName: string) => {
    if (!selectedRwId) return;
    try {
      await api.deleteRwTable(selectedRwId, schemaName, name);
      message.success(`已删除 Table: ${name}`);
      loadTables();
    } catch (error) {
      message.error("删除失败: " + error);
    }
  };

  const handleDeleteMaterializedView = async (name: string, schemaName: string) => {
    if (!selectedRwId) return;
    try {
      await api.deleteRwMaterializedView(selectedRwId, schemaName, name);
      message.success(`已删除 Materialized View: ${name}`);
      loadMaterializedViews();
    } catch (error) {
      message.error("删除失败: " + error);
    }
  };

  const handleDeleteSink = async (name: string, schemaName: string) => {
    if (!selectedRwId) return;
    try {
      await api.deleteRwSink(selectedRwId, schemaName, name);
      message.success(`已删除 Sink: ${name}`);
      loadSinks();
    } catch (error) {
      message.error("删除失败: " + error);
    }
  };

  // Batch delete handlers
  const handleBatchDelete = async (
    objectType: 'source' | 'table' | 'materialized_view' | 'sink',
    selectedKeys: React.Key[],
    objects: Array<{ id: number; name: string; schema_name: string }>,
    clearSelection: () => void
  ) => {
    if (!selectedRwId || !selectedSchema || selectedKeys.length === 0) return;

    const selectedObjects = objects.filter(obj => selectedKeys.includes(obj.id));
    const names = selectedObjects.map(obj => obj.name);

    try {
      const result = await api.batchDeleteRwObjects(
        selectedRwId,
        selectedSchema,
        objectType,
        names
      );

      if (result.success) {
        message.success(`成功删除 ${result.deleted_count} 个对象`);
      } else {
        message.warning(
          `删除完成：成功 ${result.deleted_count} 个，失败 ${result.failed.length} 个`
        );
      }

      clearSelection();
      // Reload the respective tab
      if (objectType === 'source') loadSources();
      else if (objectType === 'table') loadTables();
      else if (objectType === 'materialized_view') loadMaterializedViews();
      else if (objectType === 'sink') loadSinks();
    } catch (error) {
      message.error("批量删除失败: " + error);
    }
  };

  // Show Create Sink modal
  const showCreateSinkModal = (objectName: string, objectType: 'table' | 'materialized_view') => {
    setCurrentSinkSource({ name: objectName, type: objectType });
    setCreateSinkModalVisible(true);

    // 预填充表单：默认选择第一个 StarRocks 连接
    if (srConnections.length > 0) {
      const firstSr = srConnections[0];
      sinkForm.setFieldsValue({
        sr_config_id: firstSr.id,
        target_database: firstSr.database_name || '',
        target_table: objectName, // 默认使用相同的表名
      });
    }
  };

  // Handle Create Sink
  const handleCreateSink = async () => {
    if (!selectedRwId || !selectedSchema || !currentSinkSource) return;

    try {
      const values = await sinkForm.validateFields();
      setCreateSinkLoading(true);

      const request: api.CreateSinkRequest = {
        rw_config_id: selectedRwId,
        sr_config_id: values.sr_config_id,
        schema: selectedSchema,
        source_object: currentSinkSource.name,
        source_type: currentSinkSource.type,
        target_database: values.target_database,
        target_table: values.target_table,
      };

      const result = await api.createRwSink(request);
      message.success(result.message || 'Sink 创建成功');

      setCreateSinkModalVisible(false);
      sinkForm.resetFields();
      setCurrentSinkSource(null);
      loadSinks(); // 刷新 Sinks 列表
    } catch (error) {
      message.error("创建 Sink 失败: " + error);
    } finally {
      setCreateSinkLoading(false);
    }
  };

  // Table columns definitions
  const sourceColumns: ColumnsType<RwSource> = [
    {
      title: "ID",
      dataIndex: "id",
      key: "id",
      width: 80,
    },
    {
      title: "名称",
      dataIndex: "name",
      key: "name",
    },
    {
      title: "Schema",
      dataIndex: "schema_name",
      key: "schema_name",
    },
    {
      title: "所有者",
      dataIndex: "owner",
      key: "owner",
    },
    {
      title: "连接器",
      dataIndex: "connector",
      key: "connector",
      render: (connector: string) => <Tag color="blue">{connector}</Tag>,
    },
    {
      title: "列",
      dataIndex: "columns",
      key: "columns",
      render: (columns: string[]) => columns.join(", "),
    },
    {
      title: "操作",
      key: "actions",
      width: 150,
      render: (_: any, record: RwSource) => (
        <Space>
          {record.definition && (
            <Button
              type="link"
              size="small"
              icon={<EyeOutlined />}
              onClick={() => showSqlModal(`Source: ${record.name}`, record.definition!)}
            >
              查看
            </Button>
          )}
          <Popconfirm
            title="确认删除"
            description={`确定要删除 Source "${record.name}" 吗？`}
            onConfirm={() => handleDeleteSource(record.name, record.schema_name)}
            okText="确定"
            cancelText="取消"
          >
            <Button type="link" size="small" danger icon={<DeleteOutlined />}>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const tableColumns: ColumnsType<RwTable> = [
    {
      title: "ID",
      dataIndex: "id",
      key: "id",
      width: 80,
    },
    {
      title: "名称",
      dataIndex: "name",
      key: "name",
    },
    {
      title: "Schema",
      dataIndex: "schema_name",
      key: "schema_name",
    },
    {
      title: "所有者",
      dataIndex: "owner",
      key: "owner",
    },
    {
      title: "操作",
      key: "actions",
      width: 220,
      render: (_: any, record: RwTable) => (
        <Space>
          {record.definition && (
            <Button
              type="link"
              size="small"
              icon={<EyeOutlined />}
              onClick={() => showSqlModal(`Table: ${record.name}`, record.definition!)}
            >
              查看
            </Button>
          )}
          <Button
            type="link"
            size="small"
            icon={<PlusOutlined />}
            onClick={() => showCreateSinkModal(record.name, 'table')}
          >
            创建 Sink
          </Button>
          <Popconfirm
            title="确认删除"
            description={`确定要删除 Table "${record.name}" 吗？`}
            onConfirm={() => handleDeleteTable(record.name, record.schema_name)}
            okText="确定"
            cancelText="取消"
          >
            <Button type="link" size="small" danger icon={<DeleteOutlined />}>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const mvColumns: ColumnsType<RwMaterializedView> = [
    {
      title: "ID",
      dataIndex: "id",
      key: "id",
      width: 80,
    },
    {
      title: "名称",
      dataIndex: "name",
      key: "name",
    },
    {
      title: "Schema",
      dataIndex: "schema_name",
      key: "schema_name",
    },
    {
      title: "所有者",
      dataIndex: "owner",
      key: "owner",
    },
    {
      title: "操作",
      key: "actions",
      width: 220,
      render: (_: any, record: RwMaterializedView) => (
        <Space>
          {record.definition && (
            <Button
              type="link"
              size="small"
              icon={<EyeOutlined />}
              onClick={() =>
                showSqlModal(`Materialized View: ${record.name}`, record.definition!)
              }
            >
              查看
            </Button>
          )}
          <Button
            type="link"
            size="small"
            icon={<PlusOutlined />}
            onClick={() => showCreateSinkModal(record.name, 'materialized_view')}
          >
            创建 Sink
          </Button>
          <Popconfirm
            title="确认删除"
            description={`确定要删除 Materialized View "${record.name}" 吗？`}
            onConfirm={() => handleDeleteMaterializedView(record.name, record.schema_name)}
            okText="确定"
            cancelText="取消"
          >
            <Button type="link" size="small" danger icon={<DeleteOutlined />}>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const sinkColumns: ColumnsType<RwSink> = [
    {
      title: "ID",
      dataIndex: "id",
      key: "id",
      width: 80,
    },
    {
      title: "名称",
      dataIndex: "name",
      key: "name",
    },
    {
      title: "Schema",
      dataIndex: "schema_name",
      key: "schema_name",
    },
    {
      title: "所有者",
      dataIndex: "owner",
      key: "owner",
    },
    {
      title: "连接器",
      dataIndex: "connector",
      key: "connector",
      render: (connector: string) => <Tag color="green">{connector}</Tag>,
    },
    {
      title: "目标表",
      dataIndex: "target_table",
      key: "target_table",
    },
    {
      title: "操作",
      key: "actions",
      width: 150,
      render: (_: any, record: RwSink) => (
        <Space>
          {record.definition && (
            <Button
              type="link"
              size="small"
              icon={<EyeOutlined />}
              onClick={() => showSqlModal(`Sink: ${record.name}`, record.definition!)}
            >
              查看
            </Button>
          )}
          <Popconfirm
            title="确认删除"
            description={`确定要删除 Sink "${record.name}" 吗？`}
            onConfirm={() => handleDeleteSink(record.name, record.schema_name)}
            okText="确定"
            cancelText="取消"
          >
            <Button type="link" size="small" danger icon={<DeleteOutlined />}>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div style={{ padding: 24 }}>
      <Title level={2}>RisingWave Object Manager</Title>

      <Card style={{ marginBottom: 24 }}>
        <Space size="large">
          <div>
            <label style={{ marginRight: 8 }}>RisingWave 连接:</label>
            <Select
              style={{ width: 300 }}
              value={selectedRwId}
              onChange={setSelectedRwId}
              placeholder="选择 RisingWave 连接"
            >
              {rwConnections.map((conn) => (
                <Select.Option key={conn.id} value={conn.id}>
                  {conn.name} ({conn.host}:{conn.port})
                </Select.Option>
              ))}
            </Select>
          </div>

          <div>
            <label style={{ marginRight: 8 }}>Schema:</label>
            <Select
              style={{ width: 200 }}
              value={selectedSchema}
              onChange={setSelectedSchema}
              placeholder="选择 Schema"
              disabled={!selectedRwId}
            >
              {schemas.map((schema) => (
                <Select.Option
                  key={schema.schema_name}
                  value={schema.schema_name}
                >
                  {schema.schema_name}
                </Select.Option>
              ))}
            </Select>
          </div>
        </Space>
      </Card>

      <Card>
        <Tabs defaultActiveKey="sources">
          <TabPane tab={`Sources (${sourcesState.total})`} key="sources">
            <Spin spinning={sourcesState.loading}>
              <Space direction="vertical" style={{ width: "100%", marginBottom: 16 }}>
                <Input.Search
                  placeholder="搜索对象名称..."
                  value={sourcesState.search}
                  onChange={(e) => setSourcesState(prev => ({
                    ...prev,
                    search: e.target.value,
                    currentPage: 1
                  }))}
                  onSearch={() => setSourcesState(prev => ({ ...prev, currentPage: 1 }))}
                  allowClear
                  style={{ width: 300 }}
                />
              </Space>

              {selectedSourceKeys.length > 0 && (
                <div style={{ marginBottom: 16 }}>
                  <Popconfirm
                    title="批量删除确认"
                    description={`确定要删除选中的 ${selectedSourceKeys.length} 个 Source 吗？`}
                    onConfirm={() => handleBatchDelete('source', selectedSourceKeys, sourcesState.data, () => setSelectedSourceKeys([]))}
                    okText="确定"
                    cancelText="取消"
                  >
                    <Button type="primary" danger>
                      批量删除 ({selectedSourceKeys.length})
                    </Button>
                  </Popconfirm>
                </div>
              )}

              <Table
                columns={sourceColumns}
                dataSource={sourcesState.data}
                rowKey="id"
                pagination={{
                  current: sourcesState.currentPage,
                  pageSize: sourcesState.pageSize,
                  total: sourcesState.total,
                  showSizeChanger: true,
                  showTotal: (total) => `共 ${total} 条`,
                  pageSizeOptions: ['10', '20', '50', '100'],
                  onChange: (page, size) => {
                    setSourcesState(prev => ({
                      ...prev,
                      currentPage: page,
                      pageSize: size,
                    }));
                  },
                }}
                rowSelection={{
                  selectedRowKeys: selectedSourceKeys,
                  onChange: setSelectedSourceKeys,
                }}
              />
            </Spin>
          </TabPane>

          <TabPane tab={`Tables (${tablesState.total})`} key="tables">
            <Spin spinning={tablesState.loading}>
              <Space direction="vertical" style={{ width: "100%", marginBottom: 16 }}>
                <Input.Search
                  placeholder="搜索对象名称..."
                  value={tablesState.search}
                  onChange={(e) => setTablesState(prev => ({
                    ...prev,
                    search: e.target.value,
                    currentPage: 1
                  }))}
                  onSearch={() => setTablesState(prev => ({ ...prev, currentPage: 1 }))}
                  allowClear
                  style={{ width: 300 }}
                />
              </Space>

              {selectedTableKeys.length > 0 && (
                <div style={{ marginBottom: 16 }}>
                  <Popconfirm
                    title="批量删除确认"
                    description={`确定要删除选中的 ${selectedTableKeys.length} 个 Table 吗？`}
                    onConfirm={() => handleBatchDelete('table', selectedTableKeys, tablesState.data, () => setSelectedTableKeys([]))}
                    okText="确定"
                    cancelText="取消"
                  >
                    <Button type="primary" danger>
                      批量删除 ({selectedTableKeys.length})
                    </Button>
                  </Popconfirm>
                </div>
              )}

              <Table
                columns={tableColumns}
                dataSource={tablesState.data}
                rowKey="id"
                pagination={{
                  current: tablesState.currentPage,
                  pageSize: tablesState.pageSize,
                  total: tablesState.total,
                  showSizeChanger: true,
                  showTotal: (total) => `共 ${total} 条`,
                  pageSizeOptions: ['10', '20', '50', '100'],
                  onChange: (page, size) => {
                    setTablesState(prev => ({
                      ...prev,
                      currentPage: page,
                      pageSize: size,
                    }));
                  },
                }}
                rowSelection={{
                  selectedRowKeys: selectedTableKeys,
                  onChange: setSelectedTableKeys,
                }}
              />
            </Spin>
          </TabPane>

          <TabPane
            tab={`Materialized Views (${mvsState.total})`}
            key="mvs"
          >
            <Spin spinning={mvsState.loading}>
              <Space direction="vertical" style={{ width: "100%", marginBottom: 16 }}>
                <Input.Search
                  placeholder="搜索对象名称..."
                  value={mvsState.search}
                  onChange={(e) => setMvsState(prev => ({
                    ...prev,
                    search: e.target.value,
                    currentPage: 1
                  }))}
                  onSearch={() => setMvsState(prev => ({ ...prev, currentPage: 1 }))}
                  allowClear
                  style={{ width: 300 }}
                />
              </Space>

              {selectedMvKeys.length > 0 && (
                <div style={{ marginBottom: 16 }}>
                  <Popconfirm
                    title="批量删除确认"
                    description={`确定要删除选中的 ${selectedMvKeys.length} 个 Materialized View 吗？`}
                    onConfirm={() => handleBatchDelete('materialized_view', selectedMvKeys, mvsState.data, () => setSelectedMvKeys([]))}
                    okText="确定"
                    cancelText="取消"
                  >
                    <Button type="primary" danger>
                      批量删除 ({selectedMvKeys.length})
                    </Button>
                  </Popconfirm>
                </div>
              )}

              <Table
                columns={mvColumns}
                dataSource={mvsState.data}
                rowKey="id"
                pagination={{
                  current: mvsState.currentPage,
                  pageSize: mvsState.pageSize,
                  total: mvsState.total,
                  showSizeChanger: true,
                  showTotal: (total) => `共 ${total} 条`,
                  pageSizeOptions: ['10', '20', '50', '100'],
                  onChange: (page, size) => {
                    setMvsState(prev => ({
                      ...prev,
                      currentPage: page,
                      pageSize: size,
                    }));
                  },
                }}
                rowSelection={{
                  selectedRowKeys: selectedMvKeys,
                  onChange: setSelectedMvKeys,
                }}
              />
            </Spin>
          </TabPane>

          <TabPane tab={`Sinks (${sinksState.total})`} key="sinks">
            <Spin spinning={sinksState.loading}>
              <Space direction="vertical" style={{ width: "100%", marginBottom: 16 }}>
                <Input.Search
                  placeholder="搜索对象名称..."
                  value={sinksState.search}
                  onChange={(e) => setSinksState(prev => ({
                    ...prev,
                    search: e.target.value,
                    currentPage: 1
                  }))}
                  onSearch={() => setSinksState(prev => ({ ...prev, currentPage: 1 }))}
                  allowClear
                  style={{ width: 300 }}
                />
              </Space>

              {selectedSinkKeys.length > 0 && (
                <div style={{ marginBottom: 16 }}>
                  <Popconfirm
                    title="批量删除确认"
                    description={`确定要删除选中的 ${selectedSinkKeys.length} 个 Sink 吗？`}
                    onConfirm={() => handleBatchDelete('sink', selectedSinkKeys, sinksState.data, () => setSelectedSinkKeys([]))}
                    okText="确定"
                    cancelText="取消"
                  >
                    <Button type="primary" danger>
                      批量删除 ({selectedSinkKeys.length})
                    </Button>
                  </Popconfirm>
                </div>
              )}

              <Table
                columns={sinkColumns}
                dataSource={sinksState.data}
                rowKey="id"
                pagination={{
                  current: sinksState.currentPage,
                  pageSize: sinksState.pageSize,
                  total: sinksState.total,
                  showSizeChanger: true,
                  showTotal: (total) => `共 ${total} 条`,
                  pageSizeOptions: ['10', '20', '50', '100'],
                  onChange: (page, size) => {
                    setSinksState(prev => ({
                      ...prev,
                      currentPage: page,
                      pageSize: size,
                    }));
                  },
                }}
                rowSelection={{
                  selectedRowKeys: selectedSinkKeys,
                  onChange: setSelectedSinkKeys,
                }}
              />
            </Spin>
          </TabPane>
        </Tabs>
      </Card>

      {/* SQL Definition Modal */}
      <Modal
        title={sqlModalTitle}
        open={sqlModalVisible}
        onCancel={() => setSqlModalVisible(false)}
        width={800}
        footer={[
          <Button
            key="copy"
            icon={<CopyOutlined />}
            onClick={() => copySql(sqlModalContent)}
          >
            复制 SQL
          </Button>,
          <Button
            key="close"
            type="primary"
            onClick={() => setSqlModalVisible(false)}
          >
            关闭
          </Button>,
        ]}
      >
        <Paragraph
          code
          copyable
          style={{
            background: "#f5f5f5",
            padding: "12px",
            borderRadius: "4px",
            maxHeight: "500px",
            overflow: "auto",
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
          }}
        >
          {sqlModalContent}
        </Paragraph>
      </Modal>

      {/* Create Sink Modal */}
      <Modal
        title={`创建 Sink 到 StarRocks - ${currentSinkSource?.type === 'table' ? '表' : '物化视图'}: ${currentSinkSource?.name}`}
        open={createSinkModalVisible}
        onCancel={() => {
          setCreateSinkModalVisible(false);
          sinkForm.resetFields();
          setCurrentSinkSource(null);
        }}
        onOk={handleCreateSink}
        confirmLoading={createSinkLoading}
        width={600}
      >
        <Form
          form={sinkForm}
          layout="vertical"
          style={{ marginTop: 24 }}
        >
          <Form.Item
            label="StarRocks 连接"
            name="sr_config_id"
            rules={[{ required: true, message: "请选择 StarRocks 连接" }]}
          >
            <Select placeholder="选择 StarRocks 连接">
              {srConnections.map((conn) => (
                <Select.Option key={conn.id} value={conn.id}>
                  {conn.name} ({conn.host}:{conn.port})
                </Select.Option>
              ))}
            </Select>
          </Form.Item>

          <Form.Item
            label="目标数据库"
            name="target_database"
            rules={[{ required: true, message: "请输入目标数据库名称" }]}
          >
            <Input placeholder="输入目标数据库名称" />
          </Form.Item>

          <Form.Item
            label="目标表名"
            name="target_table"
            rules={[{ required: true, message: "请输入目标表名称" }]}
          >
            <Input placeholder="输入目标表名称" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default RisingWaveManager;
