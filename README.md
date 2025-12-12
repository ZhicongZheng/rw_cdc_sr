# RisingWave CDC to StarRocks 同步工具

一个基于 Tauri + Rust 的数据同步工具，通过 RisingWave 实现 MySQL 到 StarRocks 的一键数据同步。

## 项目概述

一个给开发者使用的小工具，核心功能是可以通过 RisingWave, 一键同步 MySQL 的数据到 StarRocks。

## 项目状态

**🎉 项目已完成！前后端全栈实现 100%**

### ✅ 后端功能（Rust + Tauri）
- ✅ 完整的数据模型和类型系统
- ✅ MySQL/RisingWave/StarRocks 连接服务
- ✅ 表结构元数据读取和分析
- ✅ 智能 SQL DDL 生成器
- ✅ 异步同步引擎和任务管理
- ✅ SQLite 配置和日志持久化
- ✅ AES-256 密码加密存储
- ✅ 16 个 Tauri Command API 接口

### ✅ 前端功能（React + TypeScript + Ant Design）
- ✅ 现代化的 UI 界面设计
- ✅ 三个核心页面（连接配置、数据同步、任务管理）
- ✅ 完整的 API 集成
- ✅ 实时任务状态和进度显示
- ✅ 友好的用户交互体验

## 快速开始

### 环境要求
- Rust 1.75+ (支持 2024 edition)
- Node.js 18+
- npm/pnpm/yarn

### 安装依赖

```bash
# 安装前端依赖
npm install
```

**注意**: Rust 依赖会在首次运行 Tauri 时自动安装

### 开发模式

```bash
# 启动 Tauri 开发服务器（会同时启动前后端）
npm run tauri:dev
```

### 构建应用

```bash
# 构建 Tauri 应用（会自动构建前端）
npm run tauri:build
```

构建产物位置：
- **macOS**: `src-tauri/target/release/bundle/macos/`
- **Windows**: `src-tauri/target/release/bundle/msi/`
- **Linux**: `src-tauri/target/release/bundle/appimage/`

## 功能特性

### 1️⃣ 连接管理
- 支持 MySQL、RisingWave、StarRocks 连接配置
- 连接测试功能
- 密码加密存储
- 连接配置持久化

### 2️⃣ 数据同步
- 可视化表选择
- 支持单表/批量同步
- 自定义目标数据库和表名
- 灵活的同步选项：
  - 重建 RisingWave Source
  - 重建 StarRocks 表
  - 清空数据

### 3️⃣ 任务管理
- 实时任务状态查看
- 详细的执行日志
- 任务进度跟踪
- 任务历史记录

## 技术栈

### 后端
- **Rust 2024** + Tauri 2.0
- **SQLx 0.8** (MySQL, PostgreSQL, SQLite)
- **Tokio** (异步运行时)
- **AES-GCM** (密码加密)

### 前端
- **React 18** + TypeScript 5
- **Ant Design 5** (UI 组件库)
- **Vite** (构建工具)
- **React Router** (路由)

## 项目结构

```
rw_cdc_sr/
├── 📄 配置文件
│   ├── package.json            # 前端依赖
│   ├── tsconfig.json           # TypeScript 配置
│   ├── vite.config.ts          # Vite 配置
│   └── index.html              # HTML 入口
│
├── 📚 文档
│   ├── README.md               # 项目说明
│   ├── DESIGN.md               # 详细设计文档
│   └── USER_GUIDE.md           # 使用指南
│
├── ⚛️ 前端代码 (src/)
│   ├── main.tsx                # 前端入口
│   ├── types/                  # TypeScript 类型
│   ├── services/               # API 调用服务
│   │   └── api.ts
│   ├── components/             # React 组件
│   │   └── MainLayout.tsx
│   ├── pages/                  # 页面组件
│   │   ├── ConnectionConfig.tsx
│   │   ├── TableSelection.tsx
│   │   └── TaskManagement.tsx
│   └── styles/                 # 样式文件
│       └── global.css
│
└── 🦀 后端代码 (src-tauri/)
    ├── Cargo.toml              # Rust 依赖
    ├── build.rs                # 构建脚本
    ├── tauri.conf.json         # Tauri 配置
    └── src/                    # Rust 源码
        ├── main.rs             # 后端入口
        ├── lib.rs              # 库入口
        ├── models/             # 数据模型 (3 文件)
        ├── utils/              # 工具函数 (4 文件)
        ├── db/                 # 数据库层 (3 文件)
        ├── services/           # 业务逻辑 (4 文件)
        ├── generators/         # SQL 生成器 (3 文件)
        └── commands/           # Tauri 命令 (5 文件)
```

## 文档

- 📖 [设计文档](./DESIGN.md) - 详细的架构和实现设计
- 📘 [使用指南](./USER_GUIDE.md) - 完整的使用说明

## 核心功能演示

### 连接配置页面
- 添加和管理数据库连接
- 测试连接可用性
- 加密存储敏感信息

### 数据同步页面
- 三步式向导流程
- 可视化表选择
- 灵活的同步选项配置

### 任务管理页面
- 任务列表和状态筛选
- 详细的任务执行日志
- 实时进度显示

## 许可证

MIT

---

**开发完成时间**: 2025-12-12
**作者**: Claude Code
**技术栈**: Rust + Tauri + React + TypeScript + Ant Design
