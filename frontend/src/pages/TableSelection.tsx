import React, { useState, useEffect } from "react";
import {
  Card,
  Steps,
  Select,
  Button,
  Table,
  Checkbox, Input,
  message,
  Space, Alert
} from "antd";
import { SyncOutlined, ArrowRightOutlined } from "@ant-design/icons";
import type { ColumnsType } from "antd/es/table";
import type {
  DatabaseConfig, SyncRequest,
  BatchSyncRequest,
  SyncOptions,
  TableSyncInfo
} from "../types";
import * as api from "../services/api";

interface SelectedTable {
  database: string;
  table: string;
  targetDatabase: string;
  targetTable: string;
}

const TableSelection: React.FC = () => {
  const [currentStep, setCurrentStep] = useState(0);
  const [mysqlConnections, setMysqlConnections] = useState<DatabaseConfig[]>(
    []
  );
  const [rwConnections, setRwConnections] = useState<DatabaseConfig[]>([]);
  const [srConnections, setSrConnections] = useState<DatabaseConfig[]>([]);

  const [selectedMysqlId, setSelectedMysqlId] = useState<number>();
  const [selectedRwId, setSelectedRwId] = useState<number>();
  const [selectedSrId, setSelectedSrId] = useState<number>();

  const [databases, setDatabases] = useState<string[]>([]);
  const [selectedDatabase, setSelectedDatabase] = useState<string>();
  const [tables, setTables] = useState<string[]>([]);
  const [selectedTables, setSelectedTables] = useState<SelectedTable[]>([]);

  const [syncOptions, setSyncOptions] = useState<SyncOptions>({
    recreate_rw_source: false,
    recreate_sr_table: false,
    truncate_sr_table: false,
  });

  const [syncing, setSyncing] = useState(false);

  // 加载连接列表
  useEffect(() => {
    loadConnections();
  }, []);

  const loadConnections = async () => {
    try {
      const data = await api.getAllConnections();
      setMysqlConnections(data.filter((c) => c.db_type === "mysql"));
      setRwConnections(data.filter((c) => c.db_type === "risingwave"));
      setSrConnections(data.filter((c) => c.db_type === "starrocks"));
    } catch (error) {
      message.error("加载连接配置失败: " + error);
    }
  };

  // 加载数据库列表
  const loadDatabases = async (configId: number) => {
    try {
      const dbs = await api.listMysqlDatabases(configId);
      setDatabases(dbs);
      setSelectedDatabase(undefined);
      setTables([]);
    } catch (error) {
      message.error("加载数据库列表失败: " + error);
    }
  };

  // 加载表列表
  const loadTables = async (configId: number, database: string) => {
    try {
      const tbls = await api.listMysqlTables(configId, database);
      setTables(tbls);
    } catch (error) {
      message.error("加载表列表失败: " + error);
    }
  };

  // 步骤1：选择 MySQL 连接和数据库
  const handleStep1Next = () => {
    if (!selectedMysqlId || !selectedDatabase || selectedTables.length === 0) {
      message.warning("请选择 MySQL 连接、数据库和至少一个表");
      return;
    }
    setCurrentStep(1);
  };

  // 步骤2：选择 RisingWave 和 StarRocks 连接
  const handleStep2Next = () => {
    if (!selectedRwId || !selectedSrId) {
      message.warning("请选择 RisingWave 和 StarRocks 连接");
      return;
    }
    setCurrentStep(2);
  };

  // 执行同步
  const handleSync = async () => {
    if (!selectedMysqlId || !selectedRwId || !selectedSrId) {
      message.error("请完成所有配置");
      return;
    }

    setSyncing(true);
    try {
      if (selectedTables.length === 1) {
        // 单表同步
        const table = selectedTables[0];
        const request: SyncRequest = {
          mysql_config_id: selectedMysqlId,
          rw_config_id: selectedRwId,
          sr_config_id: selectedSrId,
          mysql_database: table.database,
          mysql_table: table.table,
          target_database: table.targetDatabase,
          target_table: table.targetTable,
          options: syncOptions,
        };

        const taskId = await api.syncSingleTable(request);
        message.success(`同步任务已创建！任务 ID: ${taskId}`);
      } else {
        // 批量同步
        const tableInfos: TableSyncInfo[] = selectedTables.map((t) => ({
          mysql_database: t.database,
          mysql_table: t.table,
          target_database: t.targetDatabase,
          target_table: t.targetTable,
        }));

        const request: BatchSyncRequest = {
          mysql_config_id: selectedMysqlId,
          rw_config_id: selectedRwId,
          sr_config_id: selectedSrId,
          tables: tableInfos,
          options: syncOptions,
        };

        const taskIds = await api.syncMultipleTables(request);
        message.success(`批量同步任务已创建！共 ${taskIds.length} 个任务`);
      }

      // 重置表单
      setCurrentStep(0);
      setSelectedTables([]);
    } catch (error) {
      message.error("创建同步任务失败: " + error);
    } finally {
      setSyncing(false);
    }
  };

  // 表格选择
  const tableColumns: ColumnsType<string> = [
    {
      title: "表名",
      dataIndex: "table",
      key: "table",
      render: (_, table) => table,
    },
    {
      title: "操作",
      key: "action",
      render: (_, table) => {
        const isSelected = selectedTables.some(
          (t) => t.table === table && t.database === selectedDatabase
        );

        return (
          <Button
            type={isSelected ? "default" : "primary"}
            size="small"
            onClick={() => {
              if (isSelected) {
                setSelectedTables(
                  selectedTables.filter(
                    (t) =>
                      !(t.table === table && t.database === selectedDatabase)
                  )
                );
              } else {
                setSelectedTables([
                  ...selectedTables,
                  {
                    database: selectedDatabase!,
                    table,
                    targetDatabase: selectedDatabase!,
                    targetTable: table,
                  },
                ]);
              }
            }}
          >
            {isSelected ? "取消选择" : "选择"}
          </Button>
        );
      },
    },
  ];

  // 已选表格列
  const selectedTableColumns: ColumnsType<SelectedTable> = [
    {
      title: "MySQL 数据库",
      dataIndex: "database",
      key: "database",
    },
    {
      title: "MySQL 表名",
      dataIndex: "table",
      key: "table",
    },
    {
      title: <ArrowRightOutlined />,
      key: "arrow",
      width: 50,
      align: "center",
    },
    {
      title: "StarRocks 数据库",
      dataIndex: "targetDatabase",
      key: "targetDatabase",
      render: (_, record, index) => (
        <Input
          value={record.targetDatabase}
          onChange={(e) => {
            const newTables = [...selectedTables];
            newTables[index].targetDatabase = e.target.value;
            setSelectedTables(newTables);
          }}
        />
      ),
    },
    {
      title: "StarRocks 表名",
      dataIndex: "targetTable",
      key: "targetTable",
      render: (_, record, index) => (
        <Input
          value={record.targetTable}
          onChange={(e) => {
            const newTables = [...selectedTables];
            newTables[index].targetTable = e.target.value;
            setSelectedTables(newTables);
          }}
        />
      ),
    },
    {
      title: "操作",
      key: "action",
      render: (_, record) => (
        <Button
          type="link"
          danger
          onClick={() => {
            setSelectedTables(
              selectedTables.filter(
                (t) =>
                  !(t.table === record.table && t.database === record.database)
              )
            );
          }}
        >
          删除
        </Button>
      ),
    },
  ];

  return (
    <div>
      <Card>
        <Steps
          current={currentStep}
          items={[
            { title: "选择源表" },
            { title: "选择目标" },
            { title: "配置并同步" },
          ]}
          style={{ marginBottom: 32 }}
        />

        {/* 步骤 1: 选择 MySQL 连接和表 */}
        {currentStep === 0 && (
          <Space direction="vertical" style={{ width: "100%" }} size="large">
            <Card title="选择 MySQL 连接" size="small">
              <Select
                style={{ width: "100%" }}
                placeholder="请选择 MySQL 连接"
                value={selectedMysqlId}
                onChange={(value) => {
                  setSelectedMysqlId(value);
                  loadDatabases(value);
                }}
              >
                {mysqlConnections.map((conn) => (
                  <Select.Option key={conn.id} value={conn.id}>
                    {conn.name} ({conn.host}:{conn.port})
                  </Select.Option>
                ))}
              </Select>
            </Card>

            {selectedMysqlId && (
              <Card title="选择数据库" size="small">
                <Select
                  style={{ width: "100%" }}
                  placeholder="请选择数据库"
                  value={selectedDatabase}
                  onChange={(value) => {
                    setSelectedDatabase(value);
                    loadTables(selectedMysqlId, value);
                  }}
                >
                  {databases.map((db) => (
                    <Select.Option key={db} value={db}>
                      {db}
                    </Select.Option>
                  ))}
                </Select>
              </Card>
            )}

            {selectedDatabase && (
              <Card title="选择表" size="small">
                <Table
                  columns={tableColumns}
                  dataSource={tables}
                  rowKey={(table) => table}
                  pagination={{ pageSize: 10 }}
                  size="small"
                />
              </Card>
            )}

            {selectedTables.length > 0 && (
              <Card title="已选择的表" size="small">
                <Alert
                  message={`已选择 ${selectedTables.length} 个表`}
                  type="info"
                  style={{ marginBottom: 16 }}
                />
                <Table
                  columns={selectedTableColumns}
                  dataSource={selectedTables}
                  rowKey={(t) => `${t.database}.${t.table}`}
                  pagination={false}
                  size="small"
                />
              </Card>
            )}

            <div style={{ textAlign: "right" }}>
              <Button type="primary" onClick={handleStep1Next}>
                下一步
              </Button>
            </div>
          </Space>
        )}

        {/* 步骤 2: 选择 RisingWave 和 StarRocks 连接 */}
        {currentStep === 1 && (
          <Space direction="vertical" style={{ width: "100%" }} size="large">
            <Card title="选择 RisingWave 连接" size="small">
              <Select
                style={{ width: "100%" }}
                placeholder="请选择 RisingWave 连接"
                value={selectedRwId}
                onChange={setSelectedRwId}
              >
                {rwConnections.map((conn) => (
                  <Select.Option key={conn.id} value={conn.id}>
                    {conn.name} ({conn.host}:{conn.port})
                  </Select.Option>
                ))}
              </Select>
            </Card>

            <Card title="选择 StarRocks 连接" size="small">
              <Select
                style={{ width: "100%" }}
                placeholder="请选择 StarRocks 连接"
                value={selectedSrId}
                onChange={setSelectedSrId}
              >
                {srConnections.map((conn) => (
                  <Select.Option key={conn.id} value={conn.id}>
                    {conn.name} ({conn.host}:{conn.port})
                  </Select.Option>
                ))}
              </Select>
            </Card>

            <div style={{ textAlign: "right" }}>
              <Space>
                <Button onClick={() => setCurrentStep(0)}>上一步</Button>
                <Button type="primary" onClick={handleStep2Next}>
                  下一步
                </Button>
              </Space>
            </div>
          </Space>
        )}

        {/* 步骤 3: 配置同步选项并执行 */}
        {currentStep === 2 && (
          <Space direction="vertical" style={{ width: "100%" }} size="large">
            <Card title="同步选项" size="small">
              <Space direction="vertical">
                <Checkbox
                  checked={syncOptions.recreate_rw_source}
                  onChange={(e) =>
                    setSyncOptions({
                      ...syncOptions,
                      recreate_rw_source: e.target.checked,
                    })
                  }
                >
                  重建 RisingWave Source 和 Table（删除并重新创建）
                </Checkbox>
                <Checkbox
                  checked={syncOptions.recreate_sr_table}
                  onChange={(e) =>
                    setSyncOptions({
                      ...syncOptions,
                      recreate_sr_table: e.target.checked,
                    })
                  }
                >
                  重建 StarRocks 表（删除并重新创建）
                </Checkbox>
                <Checkbox
                  checked={syncOptions.truncate_sr_table}
                  disabled={syncOptions.recreate_sr_table}
                  onChange={(e) =>
                    setSyncOptions({
                      ...syncOptions,
                      truncate_sr_table: e.target.checked,
                    })
                  }
                >
                  清空 StarRocks 表数据（仅清空数据）
                </Checkbox>
              </Space>
            </Card>

            <Card title="同步预览" size="small">
              <Alert
                message="同步流程"
                description={
                  <div>
                    <p>
                      MySQL ({selectedDatabase}) → RisingWave (CDC) → StarRocks
                    </p>
                    <p>共 {selectedTables.length} 个表将被同步</p>
                  </div>
                }
                type="info"
              />
            </Card>

            <div style={{ textAlign: "right" }}>
              <Space>
                <Button onClick={() => setCurrentStep(1)}>上一步</Button>
                <Button
                  type="primary"
                  icon={<SyncOutlined />}
                  loading={syncing}
                  onClick={handleSync}
                >
                  开始同步
                </Button>
              </Space>
            </div>
          </Space>
        )}
      </Card>
    </div>
  );
};

export default TableSelection;
