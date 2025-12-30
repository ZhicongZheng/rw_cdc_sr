# 多阶段构建 Dockerfile
# Stage 1: 前端构建
FROM node:20-alpine AS frontend-builder

WORKDIR /app/frontend

# 复制前端代码
COPY frontend/package*.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# Stage 2: Rust 后端构建（嵌入前端静态文件）
FROM rust:1.91.1-slim AS backend-builder

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录为 backend 目录以匹配本地结构
WORKDIR /build/backend

# 从前端构建复制静态文件到正确位置（与本地结构一致）
COPY --from=frontend-builder /app/frontend/dist /build/frontend/dist

# 复制后端代码
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src

# 构建应用（会自动嵌入 ../frontend/dist）
RUN cargo build --release

# Stage 3: 运行时镜像
FROM debian:stable-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 从构建阶段复制二进制文件（包含嵌入的前端）
COPY --from=backend-builder /build/backend/target/release/rw_cdc_sr /app/rw_cdc_sr

# 暴露端口
EXPOSE 3000

# 设置环境变量
ENV RUST_LOG=info
ENV PORT=3000

# 启动应用
CMD ["/app/rw_cdc_sr"]
