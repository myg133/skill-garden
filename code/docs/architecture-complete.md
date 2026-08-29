# Enterprise Skills Platform - Complete Architecture

```mermaid
flowchart TB
    %% ==================== EXTERNAL LAYER ====================
    subgraph External["外部"]
        Agent_Fin["Agent: Finance<br/>(部门群)"]
        Agent_HR["Agent: HR<br/>(部门群)"]
        Agent_Ops["Agent: Ops<br/>(公司层)"]
    end

    %% ==================== INSTALL LAYER ====================
    subgraph Install["安装层"]
        Install_MD["agent-install.md<br/>(固定URL)"]
        
        Install_MD -->|下载| Core_Skills["openspace-core-skills<br/>(打包文件)"]
        Install_MD -->|配置| MCP_Config["MCP Config<br/>(platform_url + JWT)"]
    end

    %% ==================== PLATFORM CORE ====================
    subgraph Platform["Enterprise Skills Platform"]
        direction TB
        
        %% Auth
        subgraph Auth["身份与授权"]
            JWT_System["JWT Token System<br/>- Agent ID + Secret<br/>- Roles[]<br/>- Scope[]"]
            Virtual_Pools["虚拟组池<br/>- dept_finance<br/>- dept_hr<br/>- skill_admin<br/>- ..."]
        end
        
        %% MCP Server
        subgraph MCP["MCP Server (Glue Layer)"]
            direction LR
            
            MCP_Tools["Tools<br/>├── list_skills()<br/>├── get_skill_repo_url()<br/>├── read_skill_file()<br/>├── write_skill_file()<br/>├── commit_skill()<br/>├── execute_skill()<br/>├── submit_pr()<br/>└── review_pr()"]
            
            MCP_Resources["Resources<br/>└── skill://{id}<br/>&nbsp;&nbsp;&nbsp;&nbsp;├── prompt_context<br/>&nbsp;&nbsp;&nbsp;&nbsp;├── metadata<br/>&nbsp;&nbsp;&nbsp;&nbsp;└── refs/"]
            
            MCP_Auth["Auth<br/>- JWT Validation<br/>- Agent ID Injection"]
        end
        
        %% Skill Store
        subgraph Skill_Store["Skill Store (PostgreSQL)"]
            Skill_Meta["Skill Metadata<br/>├── name<br/>├── description<br/>├── tags<br/>├── version<br/>├── execution_policy<br/>│&nbsp;&nbsp;&nbsp;&nbsp;├── mode: platform/agent<br/>│&nbsp;&nbsp;&nbsp;&nbsp;└── allowed_tools<br/>└── created_by"]
            
            Permission_Model["Permission Model<br/>┌───────┬────────┬────────┐<br/>│&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;│ 公开 &nbsp;&nbsp;│ 私有 &nbsp;&nbsp;│<br/>├───────┼────────┼────────┤<br/>│ 编辑 │ 共享编辑��� 专属编辑│<br/>├───────┼────────┼────────┤<br/>│ 只读 │ 共享只读│ 白名单 &nbsp;│<br/>└───────┴────────┴────────┘"]
            
            Agent_Registry["Agent Registry<br/>├── agent_id<br/>├── agent_secret<br/>├── roles[]<br/>└── status"]
            
            Audit_Log["审计日志<br/>├── 操作类型<br/>├── agent_id<br/>├── skill_id<br/>└── timestamp"]
        end
        
        %% Git Layer
        subgraph Git_Layer["Git Layer (Gitea)"]
            direction LR
            
            GitRepos["Git Repos (Bare)<br/>/gitroot/<br/>├── skill-payroll-calc.git<br/>├── skill-invoice-match.git<br/>├── skill-report-gen.git<br/>└── ...<br/><br/>每个 Skill 独立仓库"]
            
            GitHooks["Hooks<br/>├── pre-receive<br/>│&nbsp;&nbsp;&nbsp;&nbsp;└── 验证 MCP Server 调用<br/>└── post-receive<br/>&nbsp;&nbsp;&nbsp;&nbsp;└── 触发通知/CI"]
        end
        
        %% Core Skills
        subgraph Core_Skills_Store["Core Skills Store"]
            Core_Skills_Pkg["openspace-core-skills<br/>├── skill_discover.md<br/>├── skill_execute.md<br/>├── skill_submit_pr.md<br/>├── skill_review.md<br/>└── skill_self_check.md<br/><br/>平台提供的元技能"]
        end
    end

    %% ==================== CORE SKILLS ON AGENT ====================
    subgraph Agent_Core["Agent 端"]
        direction LR
        
        Installed_Core["已安装的 openspace-core-skills<br/>让 Agent 具备：<br/>• 发现 Skill<br/>• 执行 & 报告结果<br/>• 提交 PR<br/>• 参与 Review"]
        
        Agent_Workspace["Agent Workspace<br/>本地 Skill 工作目录"]
    end

    %% ==================== CONNECTIONS ====================
    
    %% Install Flow
    Agent_Fin -->|读取| Install_MD
    Install_MD -->|安装| Core_Skills
    Install_MD -->|配置| MCP_Config
    Core_Skills -->|部署到| Installed_Core
    
    %% Agent to Platform
    Agent_Fin <-->|"MCP (JWT Auth)"| MCP_Tools
    Agent_HR <-->|"MCP (JWT Auth)"| MCP_Tools
    Agent_Ops <-->|"MCP (JWT Auth)"| MCP_Tools
    
    %% MCP to Store
    MCP_Tools -->|查询| Skill_Meta
    MCP_Tools -->|验证| JWT_System
    MCP_Tools -->|检查| Virtual_Pools
    MCP_Tools -->|记录| Audit_Log
    
    %% MCP to Git
    MCP_Tools -->|代理 Git 操作| GitRepos
    GitRepos -->|触发| GitHooks
    
    %% Core Skills分发
    Core_Skills_Pkg -->|Agent 主动拉取| Core_Skills
    
    %% Permission Model
    Virtual_Pools -.->|控制| Permission_Model
    JWT_System -.->|基于| Permission_Model

    %% ==================== SKILL LIFECYCLE ====================
    subgraph Lifecycle["Skill 生命周期"]
        direction TB
        
        LC1["发现需求"]
        LC2["创建 Skill<br/>(Agent 或 Human)"]
        LC3["提交 PR<br/>(Git Fork + Commit)"]
        LC4["Review<br/>(Human / Agent Hook)"]
        LC5["合并发布<br/>(版本 + 权限)"]
        LC6["执行 & 调用"]
        LC7["淘汰 / 归档"]
        
        LC1 --> LC2 --> LC3 --> LC4 --> LC5 --> LC6 --> LC7
    end

    %% ==================== LEGEND ====================
    subgraph Legend["图例"]
        direction LR
        L1["─── 数据流"]
        L2["-.-. 控制流/依赖"]
    end

    %% ==================== STYLES ====================
    style External fill:#f9f,stroke:#333,stroke-width:2px
    style Install fill:#bbf,stroke:#333,stroke-width:1px
    style Platform fill:#dfd,stroke:#333,stroke-width:3px
    style Auth fill:#ffc,stroke:#333,stroke-width:1px
    style MCP fill:#ffe,stroke:#333,stroke-width:2px
    style Skill_Store fill:#eef,stroke:#333,stroke-width:1px
    style Git_Layer fill:#fef,stroke:#333,stroke-width:1px
    style Core_Skills_Store fill:#ffe,stroke:#333,stroke-width:1px
    style Agent_Core fill:#ffe,stroke:#333,stroke-width:1px
    style Lifecycle fill:#eee,stroke:#333,stroke-width:1px
```

---

## 组件说明

### 外部层
| 组件 | 说明 |
|------|------|
| Agent: Finance | 财务部门 Agent 群 |
| Agent: HR | 人力资源 Agent 群 |
| Agent: Ops | 公司级运营 Agent 群 |

### 安装层
| 组件 | 说明 |
|------|------|
| agent-install.md | 固定 URL 的安装手册，Agent 自行解析安装 |
| openspace-core-skills | 平台提供的元技能包 |
| MCP Config | Agent 连接配置（platform_url + JWT） |

### 平台核心
| 组件 | 说明 |
|------|------|
| JWT Token System | Agent 身份认证，支持角色和权限范围 |
| 虚拟组池 | dept_finance, dept_hr, skill_admin 等抽象角色 |
| MCP Server | **胶水层**，代理所有 Git 操作，统一权限控制 |
| Skill Store | 元数据、权限模型、Agent 注册、审计日志 |
| Git Layer | 每个 Skill 独立仓库，MCP 代理操作 |
| Core Skills Store | 平台元技能包 |

### Skill 生命周期
```
发现需求 → 创建 → PR提交 → Review → 合并发布 → 执行 → 淘汰归档
```

---

## Skill 目录协议

基于 [Agent Skills Spec](https://agentskills.io/specification) 标准。

### 目录结构

每个 Skill 是一个独立 Git 仓库，遵循以下目录结构：

```
skill-name/
├── SKILL.md           # 必须：元数据 + 指令
├── scripts/           # 可选：可执行脚本
│   ├── run           # 入口脚本（无扩展名）
│   └── lib/          # 辅助脚本
├── references/        # 可选：文档（REFERENCE.md 等）
├── assets/           # 可选：模板、图片等静态资源
└── ...               # 可扩展
```

### SKILL.md 格式

```markdown
---
name: browse
description: 使用无头浏览器进行网页浏览和交互测试。
license: MIT
compatibility: Requires Python 3.10+ and playwright
metadata:
  author: example-org
  version: "1.0"
allowed-tools: Bash(git:*) Read
---

# Browse Skill

## 简介

...
```

### Frontmatter 字段

| 字段 | 必填 | 说明 |
|------|------|------|
| `name` | 是 | 1-64字符，小写字母、数字、连字符 |
| `description` | 是 | 描述技能用途和使用场景，1-1024字符 |
| `license` | 否 | 许可证名称 |
| `compatibility` | 否 | 环境要求，如 Python 3.10+ |
| `metadata` | 否 | 自定义键值对 |
| `allowed-tools` | 否 | 预批准的工具（实验性） |

### 渐进式加载（Progressive Disclosure）

```
启动时加载   → name + description (~100 tokens)
激活时加载   → SKILL.md 全文 (< 5000 tokens 推荐)
按需加载     → scripts/、references/、assets/
```

### 版本管理

使用 Git 管理版本：
- 每个 Skill 一个独立仓库
- `git tag v1.0.0` 标记版本
- 版本号遵循 SemVer

---

## 数据流示例

### 1. Agent 发现 Skill
```
Agent → MCP.list_skills(JWT) → MCP验证JWT → 查询Skill_Meta → 
过滤权限 → 返回有权限的Skill列表
```

### 2. Agent 改进 Skill（PR流程）
```
Agent → MCP.read_skill_file(skill_id) 
    → MCP代理Gitea读取 → 返回文件内容

Agent本地编辑 → MCP.write_skill_file(skill_id, path, content)
    → MCP代理Gitea写入 → 创建commit

Agent → MCP.submit_pr(skill_id, message)
    → MCP调用Gitea PR API → 创建PR → 进入Review
```

### 3. Skill 执行
```
Agent → MCP.execute_skill(skill_id, params)
    → 根据execution_policy决定:
       ├── platform模式: 平台沙箱执行CLI → 返回结果
       └── agent模式: 返回Skill定义 → Agent本地执行
```

---

## 权限模型详解

```
权限表达式：
  access: ["dept_finance", "dept_hr"]   // 这些组可读
  edit:  ["dept_finance"]                // 只有Finance可编辑
  admin: ["skill_admin"]                 // 管理员权限

Agent JWT Payload：
  {
    "agent_id": "agent_finance_001",
    "roles": ["dept_finance", "skill_contributor"],
    "scope": ["skill:read", "skill:write", "skill:execute"],
    "exp": "2026-04-21T16:00:00Z"
  }
```

---

## 部署视图

```mermaid
flowchart LR
    subgraph K8s["Kubernetes Cluster"]
        MCP_Pod["MCP Server Pod"]
        DB_Pod["PostgreSQL Pod"]
        Cache_Pod["Redis Pod"]
    end
    
    subgraph Gitea_Server["Gitea Server"]
        Gitea_App["Gitea App"]
        Git_Data["/gitroot/<br/>Skill Repos"]
    end
    
    subgraph Internal_Network["内部网络"]
        Agent_Container["Agent Container"]
    end
    
    Agent_Container -->|MCP| MCP_Pod
    MCP_Pod -->|SQL| DB_Pod
    MCP_Pod -->|Cache| Cache_Pod
    MCP_Pod -->|Git API| Gitea_App
    Gitea_App -->|Storage| Git_Data
    
    style K8s fill:#dfd,stroke:#333
    style Gitea_Server fill:#ffe,stroke:#333
    style Internal_Network fill:#eef,stroke:#333
```
