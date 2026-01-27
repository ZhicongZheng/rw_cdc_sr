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
  SyncOptions,
} from "../types";
import * as api from "../services/api";

interface SelectedTable {
  database: string;
  table: string;
  targetDatabase: string;
  targetTable: string;
}

const STORAGE_KEY = 'table_selection_batch_target_db';

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

  const [batchTargetDatabase, setBatchTargetDatabase] = useState<string>("");

  const [databases, setDatabases] = useState<string[]>([]);
  const [selectedDatabase, setSelectedDatabase] = useState<string>();
  const [tables, setTables] = useState<string[]>([]);
  const [tableSearchText, setTableSearchText] = useState<string>("");
  const [selectedTables, setSelectedTables] = useState<SelectedTable[]>([]);

  const [syncOptions, setSyncOptions] = useState<SyncOptions>({
    recreate_rw_source: false,
    recreate_sr_table: false,
    truncate_sr_table: false,
  });

  const [syncing, setSyncing] = useState(false);

  // 从 localStorage 恢复批量目标数据库
  useEffect(() => {
    const cachedBatchTargetDb = localStorage.getItem(STORAGE_KEY);
    if (cachedBatchTargetDb) {
      setBatchTargetDatabase(cachedBatchTargetDb);
    }
  }, []);

  // 保存批量目标数据库到 localStorage
  useEffect(() => {
    if (batchTargetDatabase) {
      localStorage.setItem(STORAGE_KEY, batchTargetDatabase);
    }
  }, [batchTargetDatabase]);

  // 加载连接列表
  useEffect(() => {
    loadConnections();
  }, []);

  const loadConnections = async () => {
    try {
      const data = await api.getAllConnections();
      const mysqlConns = data.filter((c) => c.db_type === "mysql");
      const rwConns = data.filter((c) => c.db_type === "risingwave");
      const srConns = data.filter((c) => c.db_type === "starrocks");

      setMysqlConnections(mysqlConns);
      setRwConnections(rwConns);
      setSrConnections(srConns);

      // 自动选择第一个 RisingWave 连接
      if (rwConns.length > 0) {
        setSelectedRwId(rwConns[0].id);
      }

      // 自动选择第一个 StarRocks 连接
      if (srConns.length > 0) {
        setSelectedSrId(srConns[0].id);
        // 如果有 StarRocks 连接，自动设置第一个连接的数据库名作为批量目标数据库默认值
        if (srConns[0].database_name) {
          setBatchTargetDatabase(srConns[0].database_name);
        }
      }
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
      setTableSearchText(""); // 清空搜索框
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
        const request =  selectedTables.map((table) => {
          return {
            mysql_config_id: selectedMysqlId,
            rw_config_id: selectedRwId,
            sr_config_id: selectedSrId,
            mysql_database: table.database,
            mysql_table: table.table,
            target_database: table.targetDatabase,
            target_table: table.targetTable,
            options: syncOptions,
          }
        });
        const taskId = await api.syncMultipleTables(request);
        message.success(`批量同步任务已创建！任务 ID: ${taskId}`);
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
                    targetDatabase: batchTargetDatabase || selectedDatabase!,
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
                <Input.Search
                  placeholder="搜索表名..."
                  value={tableSearchText}
                  onChange={(e) => setTableSearchText(e.target.value)}
                  onSearch={(value) => setTableSearchText(value)}
                  allowClear
                  style={{ marginBottom: 16 }}
                />
                <Table
                  columns={tableColumns}
                  dataSource={tables.filter((table) =>
                    table.toLowerCase().includes(tableSearchText.toLowerCase())
                  )}
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
                <div style={{ marginBottom: 16 }}>
                  <Space direction="vertical" style={{ width: "100%" }}>
                    <div>
                      <span style={{ marginRight: 8 }}>批量设置 StarRocks 目标数据库：</span>
                      <Input
                        style={{ width: 300 }}
                        placeholder="输入数据库名批量应用到所有表"
                        value={batchTargetDatabase}
                        onChange={(e) => {
                          const newDb = e.target.value;
                          setBatchTargetDatabase(newDb);
                          if (newDb) {
                            // 批量更新所有已选表的目标数据库
                            setSelectedTables((tables) =>
                              tables.map((t) => ({
                                ...t,
                                targetDatabase: newDb,
                              }))
                            );
                          }
                        }}
                      />
                      <span style={{ marginLeft: 8, color: "#999", fontSize: 12 }}>
                        （修改此处将统一应用到下方所有表）
                      </span>
                    </div>
                  </Space>
                </div>
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
            {selectedRwId && selectedSrId && (
              <Alert
                message="已自动选择连接"
                description="系统已自动选择第一个 RisingWave 和 StarRocks 连接，如需更改请在下方重新选择。"
                type="success"
                showIcon
                closable
              />
            )}

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
                onChange={(value) => {
                  setSelectedSrId(value);
                  // 只在还没有设置批量目标数据库时，才从连接配置中读取默认值
                  // 不要覆盖用户在步骤1已经设置好的值
                }}
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
