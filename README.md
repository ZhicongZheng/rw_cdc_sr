# RW CDC SR - MySQL to StarRocks CDC Sync via RisingWave

一个用于将 MySQL 数据通过 RisingWave CDC 实时同步到 StarRocks 的 Web 应用。

## 🏗️ 项目结构

```
rw_cdc_sr/
├── frontend/              # React + TypeScript 前端
│   ├── src/
│   │   ├── services/      # API 调用层
│   │   ├── pages/         # 页面组件
│   │   ├── components/    # 通用组件
│   │   └── types/         # TypeScript 类型定义
│   ├── package.json
│   └── vite.config.ts
│
├── backend/               # Rust + Axum 后端
│   ├── src/
│   │   ├── api/           # HTTP API handlers
│   │   ├── db/            # MySQL 数据层
│   │   ├── services/      # 业务逻辑
│   │   ├── generators/    # DDL 生成器
│   │   ├── models/        # 数据模型
│   │   └── main.rs        # 主入口（嵌入前端静态文件）
│   └── Cargo.toml
│
├── k8s/                   # Kubernetes 部署清单
│   ├── deployment.yaml
│   └── README.md
│
├── Dockerfile             # 多阶段构建配置
├── docker-compose.yml     # 本地开发环境
└── README.md              # 本文档
```

## ✨ 核心特性

- **单二进制部署**：前端静态文件嵌入到 Rust 二进制中
- **完整 Web 应用**：前后端一体化，无需分离部署
- **MySQL 8 元数据存储**：支持集群部署，利用现有 K8s MySQL 实例
- **K8s 原生**：直接使用 Service DNS 访问数据库服务
- **RESTful API**：标准 HTTP API，易于集成

## 🚀 快速开始

### 本地开发

#### 1. 前端开发

```bash
cd frontend
npm install
npm run dev  # 启动 Vite 开发服务器（http://localhost:5173）
```

#### 2. 后端开发

```bash
# 启动 MySQL 8
docker run -d \
  --name mysql \
  -e MYSQL_ROOT_PASSWORD=password \
  -e MYSQL_DATABASE=rw_cdc_sr \
  -e MYSQL_USER=rw_user \
  -e MYSQL_PASSWORD=password \
  -p 3306:3306 \
  mysql:8.0

# 运行后端（需要先构建前端）
cd backend
export DATABASE_URL="mysql://rw_user:password@localhost:3306/rw_cdc_sr"
cargo run
```

访问 http://localhost:3000

### 使用 Docker Compose

```bash
# 启动所有服务（MySQL 8 + 应用）
docker-compose up -d

# 查看日志
docker-compose logs -f app

# 停止服务
docker-compose down
```

## 📦 构建

### 构建 Docker 镜像

```bash
# 构建镜像（自动构建前后端并打包为单个二进制）
docker build -t rw-cdc-sr:latest .

# 运行容器
docker run -d \
  --name rw-cdc-sr \
  -p 3000:3000 \
  -e DATABASE_URL="mysql://user:password@host:3306/db" \
  rw-cdc-sr:latest
```

### 本地构建二进制

```bash
# 1. 构建前端
cd frontend
npm install
npm run build

# 2. 构建后端（会自动嵌入 frontend/dist）
cd ../backend
cargo build --release

# 生成的二进制文件：
# backend/target/release/rw_cdc_sr
```

### 运行二进制

```bash
export DATABASE_URL="mysql://rw_user:password@localhost:3306/rw_cdc_sr"
export PORT=3000
export RUST_LOG=info

./backend/target/release/rw_cdc_sr
```

## ☸️ Kubernetes 部署

详见 [k8s/README.md](k8s/README.md)

```bash
# 部署应用
kubectl apply -f k8s/deployment.yaml

# 访问应用
kubectl port-forward svc/rw-cdc-sr 3000:80
```

## 🔧 环境变量

### 必需

- `DATABASE_URL`: MySQL 连接字符串（用于元数据存储）
  ```
  mysql://username:password@hostname:port/database
  ```

### 可选

- `PORT`: HTTP 服务器端口（默认：3000）
- `RUST_LOG`: 日志级别（默认：info）
  ```
  RUST_LOG=debug,rw_cdc_sr=debug
  ```

## 📡 API 端点

所有 API 在 `/api` 路径下：

### 健康检查
- `GET /api/health` - 健康检查

### 连接管理
- `POST /api/connections/test/mysql` - 测试 MySQL 连接
- `POST /api/connections/test/risingwave` - 测试 RisingWave 连接
- `POST /api/connections/test/starrocks` - 测试 StarRocks 连接
- `GET /api/connections` - 获取所有连接
- `POST /api/connections` - 创建连接
- `PUT /api/connections/:id` - 更新连接
- `DELETE /api/connections/:id` - 删除连接

### 元数据
- `POST /api/metadata/databases` - 列出数据库
- `POST /api/metadata/tables` - 列出表
- `POST /api/metadata/schema` - 获取表结构

### 同步任务
- `POST /api/sync/single` - 同步单个表
- `POST /api/sync/multiple` - 同步多个表
- `GET /api/sync/progress/:id` - 获取同步进度
- `POST /api/sync/retry/:id` - 重试任务

### 任务管理
- `GET /api/tasks/history` - 任务历史
- `GET /api/tasks/:id` - 任务详情
- `GET /api/tasks/:id/logs` - 任务日志
- `POST /api/tasks/:id/cancel` - 取消任务

## 📚 技术栈

**前端**:
- React 18 + TypeScript
- Ant Design 5
- Vite 5
- React Router 6

**后端**:
- Rust 1.75+
- Axum 0.7 (Web 框架)
- SQLx 0.8 (MySQL 元数据存储 + PostgreSQL 连接 RisingWave)
- mysql_async 0.34 (StarRocks 兼容性)
- rust-embed (静态文件嵌入)

**部署**:
- Docker
- Kubernetes
- MySQL 8 (元数据存储)

## 📖 文档

- [前端 API 迁移指南](FRONTEND_MIGRATION.md)
- [完整迁移总结](MIGRATION_SUMMARY.md)
- [K8s 部署指南](k8s/README.md)

## 📄 License

MIT
