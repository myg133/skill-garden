# Deploy 工作区

部署配置、Helm Chart、环境配置、发布管理。

## 目录结构

```
Deploy/
├── README.md                      # 本文件
├── apps/                          # 应用 Helm Chart
│   └── aion-hive/
│       ├── Chart.yaml
│       ├── templates/
│       ├── values.yaml
│       └── environments/
│           ├── .env.staging
│           └── .env.production
├── environments/                  # 环境级别共享配置
│   ├── staging/
│   └── production/
├── releases/                      # 发布快照（自动生成）
│   └── v{x.y.z}/
├── scripts/
│   ├── deploy.sh                  # 部署脚本
│   ├── rollback.sh                # 回滚脚本
│   └── healthcheck.sh             # 健康检查
└── .deploy/                      # 私有工作目录
```

## 核心原则

**Deploy 分支只做"部署配置"，不做"构建"。**

```
CI 的职责：                      Deploy 的职责：
代码 checkout → 构建镜像           helm chart → k8s manifests
→ 打镜像 tag → 推镜像仓库         → 环境配置 → rollout
```

## 镜像 Tag 策略

| 环境 | Tag 来源 |
|------|---------|
| staging | `develop` 分支最新 commit SHA |
| production | Git Tag (`v{x.y.z}`) |

## 环境配置

使用 **目录 + .env 文件**混合管理：

```bash
apps/aion-hive/
├── values.yaml                     # 公共默认值
└── environments/
    ├── .env.staging               # staging 环境差异变量
    └── .env.production            # production 环境差异变量
```

## 部署流程

```bash
# 1. 确认部署目标
# 2. 更新对应环境的 .env 文件中的 IMAGE_TAG
# 3. git commit + push → 触发 CD
# 4. 监控 CD 流水线
# 5. 执行健康检查
# 6. 记录发布到 releases/
```

## 回滚流程

```bash
# 1. 确认要回滚的版本
# 2. git revert 上一个部署 commit
# 3. 调整 .env 中的镜像 tag 为旧版本
# 4. git commit + push → 触发 CD 回滚
# 5. 确认回滚成功
# 6. 记录回滚原因到 releases/
```
