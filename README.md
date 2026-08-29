# BA 工作区

需求管理、迭代计划、Agent 调度、验证审批。

## 目录结构

```
BA/
├── README.md                      # 本文件
├── demands/                       # 需求详情
│   ├── REQ-001-xxx/
│   │   ├── demand.md             # 需求描述
│   │   ├── acceptance.md         # 验收标准
│   │   ├── design-summary.md     # 设计概要
│   │   ├── status.md             # 当前状态
│   │   └── test-cases/          # QA 生成的测试用例
│   └── _template/
├── backlog/
│   ├── inbox/                     # 未梳理的原始想法
│   └── refined/                   # 已梳理待排期
├── sprint/
│   ├── current.md                 # 当前迭代计划
│   └── retrospective.md           # 迭代回顾
├── decisions/                      # 架构决策记录 (ADR)
│   └── README.md
└── dispatch/
    ├── rules.md                   # 调度规则
    ├── registry.md                # Agent 注册表
    ├── verification-queue.md      # 待验证队列
    └── cleanup-log.md             # Worktree 清理日志
```

## 需求状态流转

```
草稿 → 已评审 → 已就绪 → 进行中 → 待验证 → 已验证 → 已完成
                            ↓                ↓
                         进行中 → 已取消    待验证 → 已退回 → 进行中
```

## 调度规则

详见 `dispatch/rules.md`。

## 工作流程

1. **接收需求** → 放入 `backlog/inbox/`
2. **梳理需求** → 移到 `backlog/refined/`，写 `demand.md` + `acceptance.md`
3. **评审需求** → 更新状态为"已评审"
4. **分配开发** → 创建 `feature-REQ-xxx/` worktree，更新状态为"已就绪"
5. **开发完成** → 进入验证队列 `dispatch/verification-queue.md`
6. **验证通过** → 更新状态为"已完成"

## 启动巡检

BA Agent 每次启动时执行 workspace 巡检：
- 检查 `feature-*` / `hotfix-*` worktree 状态
- 清理已合并但未删除的 worktree
- 记录到 `dispatch/cleanup-log.md`
