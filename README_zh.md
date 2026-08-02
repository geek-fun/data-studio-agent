<div align="center">

# data-studio-agent

**驱动 [dockit](https://github.com/geek-fun/dockit) 和 [sqlkit](https://github.com/geek-fun/sqlkit) 的共享 Rust Agent 框架 —— 完整的 AI Agent 循环：Provider 适配、流式输出、工具调用、上下文压缩、会话管理。**

**一个 Crate。两个应用。可插拔的存储与事件。**

[![Release](https://img.shields.io/github/v/release/geek-fun/data-studio-agent?color=orange&label=release&logo=github)](https://github.com/geek-fun/data-studio-agent/releases)
[![Downloads](https://img.shields.io/github/downloads/geek-fun/data-studio-agent/total?color=orange&logo=docusign)](https://github.com/geek-fun/data-studio-agent/releases)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg&logo=apache)](LICENSE)
[![Stars](https://img.shields.io/github/stars/geek-fun/data-studio-agent&logo=github)](https://github.com/geek-fun/data-studio-agent/stargazers)
[![CI](https://github.com/geek-fun/data-studio-agent/actions/workflows/ci.yml/badge.svg)](https://github.com/geek-fun/data-studio-agent/actions/workflows/ci.yml)

<p>
  <img src="https://img.shields.io/badge/Rust-000000&logo=rust&logoColor=white"/>
  <img src="https://img.shields.io/badge/TypeScript-3178C6&logo=typescript&logoColor=white"/>
  <img src="https://img.shields.io/badge/MCP-000000&logo=modelcontextprotocol&logoColor=white"/>
</p>

[dockit](https://github.com/geek-fun/dockit) · [sqlkit](https://github.com/geek-fun/sqlkit) · [@geek-fun/data-studio-mcp](https://www.npmjs.com/package/@geek-fun/data-studio-mcp) · [Releases](https://github.com/geek-fun/data-studio-agent/releases)

[English](README.md) · 简体中文

</div>

---

从 [dockit](https://github.com/geek-fun/dockit) 和 [sqlkit](https://github.com/geek-fun/sqlkit) 中提取的共享 Rust Agent 框架。一个 Crate 提供完整的 AI Agent 循环 —— Provider 适配、流式输出、工具调用、上下文压缩、会话管理 —— 对可插拔的存储和事件完全泛化。

本仓库还托管 [@geek-fun/data-studio-mcp](packages/data-studio-mcp/README.md)，一个 MCP 服务器，将 dockit 和 sqlkit 的能力暴露给 AI 编程助手（Claude Code、Cursor 等）。

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

```
┌──────────────────────────────────┐
│  Tauri 应用 (dockit / sqlkit)    │
│  ┌────────────┐ ┌──────────────┐ │
│  │ adapters   │ │ capabilities │ │
│  │ (Tauri     │ │ (工具实现)   │ │
│  │  commands) │ │              │ │
│  └─────┬──────┘ └──────┬───────┘ │
│        │                │         │
├────────┼────────────────┼─────────┤
│        ▼                ▼         │
│  ┌─────────────────────────────┐  │
│  │    data-studio-agent        │  │
│  │  ┌───────────────────────┐  │  │
│  │  │  loop_runner          │  │  │
│  │  │  compact              │  │  │
│  │  │  conversation         │  │  │
│  │  ├───────────────────────┤  │  │
│  │  │  traits               │  │  │
│  │  │  (SessionStore,       │  │  │
│  │  │   EventEmitter)       │  │  │
│  │  ├───────────────────────┤  │  │
│  │  │  formatters, counter, │  │  │
│  │  │  registry, harness    │  │  │
│  │  └───────────────────────┘  │  │
│  │  ┌───────────────────────┐  │  │
│  │  │  storage (feature)    │  │  │
│  │  │  SqliteSessionStore   │  │  │
│  │  └───────────────────────┘  │  │
│  └─────────────────────────────┘  │
└──────────────────────────────────┘
```

## 快速开始

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
