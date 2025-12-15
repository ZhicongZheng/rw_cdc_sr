# RW CDC SR - K8s 部署指南

## 前提条件

- Kubernetes 集群（版本 >= 1.20）
- kubectl 已配置
- Nginx Ingress Controller（可选，用于外部访问）
- cert-manager（可选，用于 HTTPS 证书）

## 快速部署

### 1. 创建命名空间

```bash
kubectl create namespace rw-cdc-sr
```

### 2. 部署 PostgreSQL（如果集群中没有）

```bash
kubectl apply -f k8s/deployment.yaml
```

### 3. 等待 PostgreSQL 就绪

```bash
kubectl wait --for=condition=ready pod -l app=postgres -n rw-cdc-sr --timeout=300s
```

### 4. 部署应用

修改 `k8s/deployment.yaml` 中的以下配置：

- 镜像地址：`your-registry/rw-cdc-sr:latest`
- Ingress 域名：`rw-cdc.yourdomain.com`
- PostgreSQL 密码（生产环境建议使用 Sealed Secrets）

然后部署：

```bash
kubectl apply -f k8s/deployment.yaml -n rw-cdc-sr
```

### 5. 验证部署

```bash
# 查看 Pod 状态
kubectl get pods -n rw-cdc-sr

# 查看服务
kubectl get svc -n rw-cdc-sr

# 查看 Ingress
kubectl get ingress -n rw-cdc-sr
```

### 6. 访问应用

- 通过 Ingress: `https://rw-cdc.yourdomain.com`
- 通过 Port Forward (开发): `kubectl port-forward svc/rw-cdc-sr 3000:80 -n rw-cdc-sr`

## 环境变量配置

### 必需环境变量

- `DATABASE_URL`: PostgreSQL 连接字符串
- `PORT`: 应用监听端口（默认 3000）

### 可选环境变量

- `RUST_LOG`: 日志级别（默认 info）

## 连接到 K8s 内部服务

应用部署后，可以使用 K8s DNS 访问其他服务：

```yaml
# 示例：连接配置
MySQL: mysql.default.svc.cluster.local:3306
RisingWave: risingwave-frontend.default.svc.cluster.local:4566
StarRocks FE: starrocks-fe.default.svc.cluster.local:9030
StarRocks HTTP: starrocks-fe.default.svc.cluster.local:8030
```

## 生产环境建议

### 1. 使用外部 PostgreSQL

不使用 k8s/deployment.yaml 中的 PostgreSQL 部署，改用：
- 云数据库（AWS RDS, GCP Cloud SQL, Azure Database）
- 或独立的 PostgreSQL StatefulSet

### 2. 密钥管理

使用 Sealed Secrets 或云提供商的密钥管理服务：

```bash
# 使用 Sealed Secrets
kubeseal --controller-namespace=kube-system < postgres-secret.yaml > sealed-secret.yaml
kubectl apply -f sealed-secret.yaml
```

### 3. 资源限制

根据实际负载调整 resources 配置：

```yaml
resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
  limits:
    memory: "1Gi"
    cpu: "1000m"
```

### 4. 水平扩展

```bash
kubectl scale deployment/rw-cdc-sr --replicas=5 -n rw-cdc-sr
```

### 5. 监控和日志

- 集成 Prometheus + Grafana
- 使用 EFK/ELK Stack 收集日志
- 配置告警规则

## 故障排查

### 查看日志

```bash
kubectl logs -f deployment/rw-cdc-sr -n rw-cdc-sr
```

### 进入容器调试

```bash
kubectl exec -it deployment/rw-cdc-sr -n rw-cdc-sr -- /bin/bash
```

### 检查数据库连接

```bash
kubectl exec -it deployment/postgres -n rw-cdc-sr -- psql -U postgres -d rw_cdc_sr -c "SELECT * FROM database_configs;"
```

## 卸载

```bash
kubectl delete -f k8s/deployment.yaml -n rw-cdc-sr
kubectl delete namespace rw-cdc-sr
```
