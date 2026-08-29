# Deploy 工作区

部署配置、Helm Chart、环境配置、发布管理。

> **职责划分**: CI 在 `code/` 管理镜像构建，Deploy 只负责 CD 部署。

## 目录结构

```
Deploy/
├── docker/                         # Docker Compose 配置
│   ├── docker-compose.yml          # 本地开发
│   ├── docker-compose.prod.yml      # 生产高可用
│   └── .env.example                # 环境变量示例
├── apps/
│   └── aion-hive/                  # Helm Chart
│       ├── Chart.yaml
│       ├── values.yaml              # 默认配置
│       ├── templates/              # K8s 资源模板
│       │   ├── deployment.yaml
│       │   ├── service.yaml
│       │   ├── ingress.yaml
│       │   ├── secret.yaml
│       │   ├── configmap.yaml
│       │   ├── pvc.yaml
│       │   ├── serviceaccount.yaml
│       │   ├── hpa.yaml
│       │   ├── pdb.yaml
│       │   ├── _helpers.tpl
│       │   └── NOTES.txt
│       └── environments/
│           ├── values.staging.yaml
│           └── values.production.yaml
├── k8s/                           # 原始 K8s Manifests
│   ├── namespace.yaml
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── ingress.yaml
│   ├── secret.yaml
│   ├── pvc.yaml
│   ├── serviceaccount.yaml
│   ├── hpa.yaml
│   └── pdb.yaml
├── scripts/                       # 部署脚本
│   ├── deploy-helm.sh             # Helm 部署
│   ├── deploy-k8s.sh              # K8s 直接部署
│   ├── deploy.sh                  # Docker Compose 部署
│   ├── rollback.sh
│   └── healthcheck.sh
└── environments/                   # 环境级别配置
    ├── staging/
    │   └── .env
    └── production/
        └── .env
```

## 核心原则

**Deploy 分支只做"部署配置"，不做"构建"。**

```
CI 的职责 (code/):              Deploy 的职责 (Deploy/):
代码 checkout → 构建镜像            helm chart → k8s manifests
→ 打镜像 tag → 推镜像仓库         → 环境配置 → rollout
```

## 镜像管理

镜像由 CI 在 `code/` 构建并推送到镜像仓库，默认镜像:

```
docker.io/aionhive/aion-hive:<tag>
```

可通过环境变量覆盖:
- `REGISTRY`: 镜像仓库 (默认: docker.io)
- `IMAGE_PREFIX`: 镜像前缀 (默认: aionhive)
- `IMAGE_TAG`: 镜像标签 (默认: latest)

## 快速开始

### 1. Docker Compose (本地开发)

```bash
cd Deploy/docker

# 复制环境变量文件
cp .env.example .env
# 编辑 .env 配置

# 启动服务
docker-compose up -d

# 查看日志
docker-compose logs -f app

# 停止服务
docker-compose down
```

### 2. Helm 部署 (Kubernetes)

```bash
cd Deploy

# 创建 Secret (必需)
kubectl create secret generic aionhive-secret -n aionhive \
  --from-literal=database-url="postgres://user:pass@host:5432/aionhive" \
  --from-literal=jwt-secret="your-jwt-secret-at-least-32-chars" \
  --from-literal=api-key-salt="your-api-key-salt"

# 使用脚本部署
./scripts/deploy-helm.sh install production v0.3.0

# 或者手动部署
cd apps/aion-hive
helm upgrade --install aionhive . \
  --namespace aionhive \
  --create-namespace \
  --values environments/values.production.yaml \
  --set image.tag=v0.3.0 \
  --set secrets.databaseUrl="..." \
  --set secrets.jwtSecret="..." \
  --set secrets.apiKeySalt="..."
```

### 3. K8s 直接部署 (不使用 Helm)

```bash
cd Deploy/k8s

# 编辑 secret.yaml 中的密码

# 一键部署
kubectl apply -f namespace.yaml
kubectl apply -f .

# 查看状态
./scripts/deploy-k8s.sh status
```

## 环境配置

### 必需环境变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `DATABASE_URL` | PostgreSQL 连接 | `postgres://user:pass@host:5432/aionhive` |
| `AION_HIVE_JWT_SECRET` | JWT 签名密钥 (≥32字符) | `change-me-in-production` |
| `API_KEY_SALT` | API Key 加密盐 | `change-me-in-production` |

### 可选环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `AION_HIVE_HTTP_PORT` | HTTP 端口 | `8080` |
| `AION_HIVE_DATA_DIR` | 数据目录 | `/data` |
| `AION_HIVE_PUBLIC_URL` | 公开 URL | `http://localhost:8080` |
| `RUST_LOG` | 日志级别 | `warn` |
| `GITLAB_URL` | GitLab 地址 | - |
| `GITLAB_TOKEN` | GitLab Token | - |
| `GITLAB_GROUP` | GitLab 组 | - |
| `GIT_PROXY_API_BASE` | Git 代理 API | - |
| `AION_HIVE_EVAL_WEBHOOK_URLS` | 评估 Webhook URLs | - |
| `AION_HIVE_CLI_ENCRYPTION_KEY` | CLI 加密密钥 (64 字符 hex) | - |

## 部署流程

### CI/CD 集成

```yaml
# GitHub Actions 示例 (CI 在 code/ 构建镜像)
- name: Deploy to Kubernetes
  env:
    KUBECONFIG: ${{ secrets.KUBE_CONFIG }}
  run: |
    ./Deploy/scripts/deploy-helm.sh upgrade production ${{ github.ref_name }}
```

### 镜像 Tag 策略

| 环境 | Tag 来源 |
|------|---------|
| staging | `develop` 分支最新 commit SHA |
| production | Git Tag (`v{x.y.z}`) |

## 回滚流程

```bash
# Helm 回滚
./scripts/deploy-helm.sh rollback 1

# K8s 直接回滚
./scripts/deploy-k8s.sh rollback 2

# Docker Compose 回滚
docker-compose -f docker-compose.prod.yml pull
docker-compose -f docker-compose.prod.yml up -d
```

## 监控和健康检查

```bash
# 健康检查
curl http://localhost:8080/health

# 查看 Pod 状态
kubectl get pods -n aionhive

# 查看日志
kubectl logs -n aionhive -l app.kubernetes.io/name=aionhive --tail=100

# Port-forward 调试
kubectl port-forward -n aionhive svc/aionhive 8080:80
```

## 存储配置

| 存储 | 大小 | 说明 |
|------|------|------|
| 数据卷 | 10Gi | 配置文件、索引 |
| Skills 卷 | 5Gi | Skills 内容 |

生产环境建议使用 `fast-ssd` StorageClass。

## 高可用配置

- **副本数**: 生产默认 3
- **PDB**: 最多 1 个 Pod 不可用
- **HPA**: CPU 70% 或 Memory 80% 时扩容
- **健康检查**: Liveness + Readiness

## 故障排除

```bash
# Pod 无法启动
kubectl describe pod <pod-name> -n aionhive
kubectl logs <pod-name> -n aionhive

# PVC 挂载失败
kubectl describe pvc -n aionhive

# Secret 不存在
kubectl get secret aionhive-secret -n aionhive

# Ingress 配置错误
kubectl describe ingress aionhive -n aionhive
```
