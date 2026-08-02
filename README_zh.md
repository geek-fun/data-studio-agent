<div align="center">

# data-studio-agent

**把你的 AI 编程助手变成数据库助手 —— 用自然语言查询、探索和理解你的数据库。**

**本地优先。数据永不离开你的电脑。开源开放。**

[![Release](https://img.shields.io/github/v/release/geek-fun/data-studio-agent?color=orange&label=release&logo=github)](https://github.com/geek-fun/data-studio-agent/releases)
[![Downloads](https://img.shields.io/github/downloads/geek-fun/data-studio-agent/total?color=orange&logo=docusign)](https://github.com/geek-fun/data-studio-agent/releases)
[![npm](https://img.shields.io/npm/dt/@geek-fun/data-studio-mcp?color=orange&logo=npm)](https://www.npmjs.com/package/@geek-fun/data-studio-mcp)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg&logo=apache)](LICENSE)
[![Stars](https://img.shields.io/github/stars/geek-fun/data-studio-agent&logo=github)](https://github.com/geek-fun/data-studio-agent/stargazers)
[![CI](https://github.com/geek-fun/data-studio-agent/actions/workflows/ci.yml/badge.svg)](https://github.com/geek-fun/data-studio-agent/actions/workflows/ci.yml)

<p>
  <img src="https://img.shields.io/badge/SQL-PostgreSQL%20%7C%20MySQL%20%7C%20SQL%20Server%20%7C%20SQLite-336791"/>
  <img src="https://img.shields.io/badge/NoSQL-Elasticsearch%20%7C%20OpenSearch%20%7C%20MongoDB%20%7C%20DynamoDB-47A248"/>
  <img src="https://img.shields.io/badge/MCP-000000&logo=modelcontextprotocol&logoColor=white"/>
</p>

[npm](https://www.npmjs.com/package/@geek-fun/data-studio-mcp) · [dockit](https://github.com/geek-fun/dockit) · [sqlkit](https://github.com/geek-fun/sqlkit) · [Releases](https://github.com/geek-fun/data-studio-agent/releases)

[English](README.md) · 简体中文

</div>

---

本仓库是 **Data Studio MCP Server** 的家园 —— 一个统一的 [Model Context Protocol](https://modelcontextprotocol.io/) 服务器，通过 [dockit](https://github.com/geek-fun/dockit) 和 [sqlkit](https://github.com/geek-fun/sqlkit) 桌面应用，让 AI 编程助手（Claude Code、Cursor、Windsurf、OpenCode、Codex）直接访问你的数据库。

- **SQL**（通过 sqlkit）：PostgreSQL、MySQL、SQL Server、SQLite
- **NoSQL**（通过 dockit）：Elasticsearch、OpenSearch、MongoDB、DynamoDB

## 快速开始

### 1. 环境要求

安装并启动 [dockit](https://github.com/geek-fun/dockit) 和/或 [sqlkit](https://github.com/geek-fun/sqlkit)，添加数据库连接，确认 **设置 → MCP Bridge → 自动启动** 已开启（默认开启）。两个应用都装可获得完整的 SQL + NoSQL 工具集。

### 2. 安装 MCP 服务器

```bash
npm install -g @geek-fun/data-studio-mcp
```

或不安装直接运行：

```bash
npx -y @geek-fun/data-studio-mcp
```

### 3. 添加到你的 AI 工具

**Claude Code** — 在 MCP 配置中添加：

```json
{
  "mcpServers": {
    "data-studio": {
      "command": "npx",
      "args": ["-y", "@geek-fun/data-studio-mcp"]
    }
  }
}
```

**Cursor / Windsurf / OpenCode / 其他 MCP 客户端** — 注册一个 stdio 服务器：

| 设置 | 值 |
|---|---|
| 类型 (Type) | stdio |
| 命令 (Command) | `npx` |
| 参数 (Args) | `-y @geek-fun/data-studio-mcp` |

### 4. 开始提问

直接用自然语言即可 —— AI 助手会替你查询数据库：

- "列出我 PostgreSQL 数据库中的所有表"
- "从 Elasticsearch 的 `orders*` 索引中找出最近 10 条订单"
- "在 MongoDB 中查找所有年龄大于 30 的用户"
- "执行这条查询并解释结果"

AI 助手可以查看表结构、运行查询、探索你的数据 — 并且它会展示执行的每一条查询。

## 工具命名

所有工具遵循 `data_studio__{backend}__{action}` 命名规则：

| 前缀 | 后端 | 示例 |
|---|---|---|
| `data_studio__sql_*` | sqlkit | `data_studio__sql_execute`、`data_studio__sql_list_tables` |
| `data_studio__es_*` | dockit | `data_studio__es_search`、`data_studio__es_list_indices` |
| `data_studio__mongo_*` | dockit | `data_studio__mongo_find`、`data_studio__mongo_insert` |
| `data_studio__dynamo_*` | dockit | `data_studio__dynamo_query`、`data_studio__dynamo_list_tables` |

## 安全性

- Bridge 仅绑定 `127.0.0.1` — 其他机器无法访问
- **破坏性操作和写入操作会被 Bridge 拒绝** — 通过 MCP 服务器只能使用只读能力
- 凭证永远不会暴露给 AI 助手；所有连接都在桌面应用内部解析

## 工作原理

```
AI 编程助手 (Claude Code / Cursor / OpenCode ...)
    |
    | MCP stdio 协议
    v
@geek-fun/data-studio-mcp   ← npm 包（纯 TypeScript）
    |
    | HTTP（仅本机）
    +----------------+----------------+
    v                v                |
dockit:9120    sqlkit:9121            |
(NoSQL bridge)  (SQL bridge)          |
    |                |                |
    v                v                |
Elasticsearch    PostgreSQL           |
MongoDB          MySQL                |
DynamoDB         SQL Server           |
OpenSearch       SQLite               |
```

MCP 服务器是一个轻量的路由层。所有数据库驱动、SSH 隧道和连接管理都在桌面应用中，由应用暴露一个本机 HTTP Bridge（仅 `127.0.0.1`）。MCP 服务器通过各应用的端口文件自动发现正在运行的后端。

---

# 给开发者：data-studio-agent Rust 框架

在底层，本仓库还包含驱动 [dockit](https://github.com/geek-fun/dockit) 和 [sqlkit](https://github.com/geek-fun/sqlkit) 内置 AI 助手的共享 Rust Agent 框架 —— 一个 Crate 提供完整的 AI Agent 循环：Provider 适配、流式输出、工具调用、上下文压缩、会话管理，对可插拔的存储和事件完全泛化。

## 安装

```toml
[dependencies]
data-studio-agent = { git = "https://github.com/geek-fun/data-studio-agent", tag = "v0.1.4" }
```

如果使用自定义 `SessionStore`，可关闭 SQLite：

```toml
data-studio-agent = { git = "https://github.com/geek-fun/data-studio-agent", tag = "v0.1.4", default-features = false }
```

## 项目结构

```
data-studio-agent/
├── Cargo.toml              # 单一 crate，feature 控制
├── src/
│   ├── lib.rs
│   ├── traits.rs           # SessionStore, EventEmitter
│   ├── chat_formatter/     # OpenAI + Anthropic
│   ├── provider_adapter.rs
│   ├── model_registry.rs
│   ├── token_counter.rs
│   ├── tool_executor.rs    # ToolExecutor trait
│   ├── loop_runner.rs      # ReAct Agent 循环
│   ├── compact.rs          # 上下文压缩
│   ├── conversation.rs     # 消息生命周期
│   ├── harness.rs          # 单步 LLM 调用
│   ├── tools.rs            # 工具解析
│   ├── loop_runner_support.rs
│   ├── capabilities/       # CapabilityRegistry
│   ├── common/             # HTTP 客户端、格式化
│   └── storage/            # #[cfg(feature = "sqlite-storage")]
│       ├── mod.rs
│       ├── db.rs           # AgentDb, schema migration
│       └── session_store.rs
├── packages/
│   └── data-studio-mcp/    # MCP 服务器 (TypeScript)
├── docs/
│   ├── architecture.md
│   └── integration.md
├── .github/workflows/
│   ├── ci.yml
│   ├── mcp-ci.yml
│   ├── release.yml
│   └── release-mcp.yml
├── rustfmt.toml
├── clippy.toml
└── README.md
```

## 设计

两个 trait 将 Agent 循环与任何框架解耦：

| Trait | 职责 | 应用提供 |
|-------|------|-------------|
| `SessionStore` | 持久化消息、工具调用、会话 | `SqliteSessionStore`（内置）或自定义实现 |
| `EventEmitter` | 流式增量、状态、错误 | `TauriEmitter(AppHandle)` 或任意实现 |

循环本身不依赖 Tauri、SQLite 或任何具体工具 —— 它是基于这些 trait 泛化的纯异步 Rust 代码。

## 快速开始（Rust 集成）

### 1. 初始化数据库

```rust
use data_studio_agent::storage::{self, session_store::SqliteSessionStore};

let db_path = app.path().app_data_dir()?.join("agent.sqlite");
let agent_db = storage::db::open(&db_path)?;
storage::db::migrate(&agent_db)?;
app.manage(agent_db);
```

### 2. 封装 Tauri 命令

创建 `agent_adapters.rs` 薄封装。每个命令提取 Tauri 状态、构建 `TauriEmitter`，并委托给库：

```rust
use data_studio_agent as lib;
use data_studio_agent::traits::{CancelMap, ConfirmMap, EventEmitter};
use data_studio_agent::storage::{self, session_store::SqliteSessionStore};

struct TauriEmitter(AppHandle);
impl EventEmitter for TauriEmitter {
    fn emit(&self, event: &str, payload: Value) {
        let _ = self.0.emit(event, payload);
    }
}

#[tauri::command]
pub async fn run_agent_loop(
    session_id: String, user_message: String,
    settings: Value, app: AppHandle,
) -> Result<(), String> {
    let db = app.state::<storage::db::AgentDb>();
    let store = SqliteSessionStore::new(db.inner().clone());
    let emitter = TauriEmitter(app.clone());
    let confirm_map = app.state::<ConfirmMap>().inner().clone();
    let cancel_map = app.state::<CancelMap>().inner().clone();
    let executor = app.state::<Arc<dyn lib::ToolExecutor>>().inner().clone();

    lib::loop_runner::run_agent_loop(
        &session_id, &user_message, &settings,
        &store, &emitter, executor.as_ref(),
        connections, fallback,
        &confirm_map, &cancel_map,
    ).await
}
```

### 3. 注册命令

```rust
.invoke_handler(tauri::generate_handler![
    agent_adapters::run_agent_loop,
    agent_adapters::cancel_agent_loop,
    agent_adapters::confirm_tool_call,
    agent_adapters::compact_agent_session,
    agent_adapters::get_agent_context_usage,
    agent_adapters::get_tool_full_result,
    agent_adapters::run_agent_step,
    agent_adapters::validate_llm_config,
    agent_adapters::list_llm_models,
    agent_adapters::get_all_tools,
])
```

### 4. 实现 ToolExecutor

```rust
use data_studio_agent::tool_executor::{ToolEnvelope, ToolExecutor, ToolResultMetadata};

pub struct MyToolExecutor;

#[async_trait]
impl ToolExecutor for MyToolExecutor {
    async fn execute(
        &self, tool_name: &str, arguments: &Value, connection_config: &Value,
    ) -> Result<ToolEnvelope, String> {
        let start = std::time::Instant::now();
        let raw = crate::capabilities::registry::invoke_capability_inner(
            tool_name, arguments.clone(), as_opt(connection_config),
        ).await?;
        let duration_ms = start.elapsed().as_millis() as u64;
        Ok(ToolEnvelope {
            summary: truncate(&raw, 4096),
            full_result: truncate(&raw, 32768),
            metadata: ToolResultMetadata { tool_name: tool_name.into(), duration_ms, truncated: raw.len() > 32768 },
        })
    }
}
```

## 支持的 Provider

- **OpenAI**（GPT-4o、GPT-4.1、o1/o3）— `/v1/chat/completions`
- **Anthropic**（Claude 3.5/4）— `/v1/messages`
- **Ollama** / **LM Studio** — 本地模型
- **OpenRouter** / **DeepSeek** / 任意 OpenAI 兼容端点

## 能力特性

| 特性 | 说明 |
|--------|---------|
| **ReAct 循环** | 工具调用 + 重试 + 指数退避 |
| **确认门控** | 按工具 Allow/Deny，基于 oneshot channel |
| **失控保护** | 同一工具连续重复 3 次即停止 |
| **上下文压缩** | 上下文将满时自动摘要，安全分割点 |
| **Token 预算** | 200 次迭代、30 分钟墙钟、2000 万 token |
| **压缩锁** | 所有压缩路径共享每会话互斥锁 |
| **流式输出** | 基于各 Provider 的 SSE 解析 |
| **SQLite 持久化** | 规范化 schema，按应用数据隔离 |

## 构建

```bash
cargo build
cargo test
cargo clippy --all-features -- -D warnings
cargo fmt --all -- --check
```

## 许可证

Apache 2.0 — 见 [LICENSE](LICENSE)。
