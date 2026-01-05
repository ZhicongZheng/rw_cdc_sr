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
} from "antd";
import { EyeOutlined, CopyOutlined, DeleteOutlined } from "@ant-design/icons";
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

const RisingWaveManager: React.FC = () => {
  const [rwConnections, setRwConnections] = useState<DatabaseConfig[]>([]);
  const [selectedRwId, setSelectedRwId] = useState<number | null>(null);
  const [schemas, setSchemas] = useState<RwSchema[]>([]);
  const [selectedSchema, setSelectedSchema] = useState<string>("public");
  const [loading, setLoading] = useState(false);

  // Tab data states
  const [sources, setSources] = useState<RwSource[]>([]);
  const [tables, setTables] = useState<RwTable[]>([]);
  const [materializedViews, setMaterializedViews] = useState<
    RwMaterializedView[]
  >([]);
  const [sinks, setSinks] = useState<RwSink[]>([]);

  // Modal states for viewing SQL definitions
  const [sqlModalVisible, setSqlModalVisible] = useState(false);
  const [sqlModalContent, setSqlModalContent] = useState("");
  const [sqlModalTitle, setSqlModalTitle] = useState("");

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

  // Load objects when schema changes
  useEffect(() => {
    if (selectedRwId && selectedSchema) {
      loadAllObjects();
    }
  }, [selectedRwId, selectedSchema]);

  const loadConnections = async () => {
    try {
      const conns = await api.getAllConnections();
      const rwConns = conns.filter((c) => c.db_type === "risingwave");
      setRwConnections(rwConns);
      if (rwConns.length > 0) {
        setSelectedRwId(rwConns[0].id);
      }
    } catch (error) {
      message.error("加载连接失败: " + error);
    }
  };

  const loadSchemas = async () => {
    if (!selectedRwId) return;

    setLoading(true);
    try {
      const schemaList = await api.listRwSchemas(selectedRwId);
      setSchemas(schemaList);
      if (schemaList.length > 0) {
        const publicSchema = schemaList.find((s) => s.schema_name === "public");
        setSelectedSchema(publicSchema ? "public" : schemaList[0].schema_name);
      }
    } catch (error) {
      message.error("加载 Schema 失败: " + error);
    } finally {
      setLoading(false);
    }
  };

  const loadAllObjects = async () => {
    if (!selectedRwId || !selectedSchema) return;

    setLoading(true);
    try {
      const [sourcesData, tablesData, mvsData, sinksData] = await Promise.all([
        api.listRwSources(selectedRwId, selectedSchema),
        api.listRwTables(selectedRwId, selectedSchema),
        api.listRwMaterializedViews(selectedRwId, selectedSchema),
        api.listRwSinks(selectedRwId, selectedSchema),
      ]);

      setSources(sourcesData);
      setTables(tablesData);
      setMaterializedViews(mvsData);
      setSinks(sinksData);
    } catch (error) {
      message.error("加载对象失败: " + error);
    } finally {
      setLoading(false);
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
      loadAllObjects();
    } catch (error) {
      message.error("删除失败: " + error);
    }
  };

  const handleDeleteTable = async (name: string, schemaName: string) => {
    if (!selectedRwId) return;
    try {
      await api.deleteRwTable(selectedRwId, schemaName, name);
      message.success(`已删除 Table: ${name}`);
      loadAllObjects();
    } catch (error) {
      message.error("删除失败: " + error);
    }
  };

  const handleDeleteMaterializedView = async (name: string, schemaName: string) => {
    if (!selectedRwId) return;
    try {
      await api.deleteRwMaterializedView(selectedRwId, schemaName, name);
      message.success(`已删除 Materialized View: ${name}`);
      loadAllObjects();
    } catch (error) {
      message.error("删除失败: " + error);
    }
  };

  const handleDeleteSink = async (name: string, schemaName: string) => {
    if (!selectedRwId) return;
    try {
      await api.deleteRwSink(selectedRwId, schemaName, name);
      message.success(`已删除 Sink: ${name}`);
      loadAllObjects();
    } catch (error) {
      message.error("删除失败: " + error);
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
      width: 120,
      render: (_: any, record: RwSource) => (
        <Space>
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
      width: 150,
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
      width: 150,
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
      width: 120,
      render: (_: any, record: RwSink) => (
        <Space>
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

      <Spin spinning={loading}>
        <Card>
          <Tabs defaultActiveKey="sources">
            <TabPane tab={`Sources (${sources.length})`} key="sources">
              <Table
                columns={sourceColumns}
                dataSource={sources}
                rowKey="id"
                pagination={{ pageSize: 20 }}
              />
            </TabPane>

            <TabPane tab={`Tables (${tables.length})`} key="tables">
              <Table
                columns={tableColumns}
                dataSource={tables}
                rowKey="id"
                pagination={{ pageSize: 20 }}
              />
            </TabPane>

            <TabPane
              tab={`Materialized Views (${materializedViews.length})`}
              key="mvs"
            >
              <Table
                columns={mvColumns}
                dataSource={materializedViews}
                rowKey="id"
                pagination={{ pageSize: 20 }}
              />
            </TabPane>

            <TabPane tab={`Sinks (${sinks.length})`} key="sinks">
              <Table
                columns={sinkColumns}
                dataSource={sinks}
                rowKey="id"
                pagination={{ pageSize: 20 }}
              />
            </TabPane>
          </Tabs>
        </Card>
      </Spin>

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
    </div>
  );
};

export default RisingWaveManager;
