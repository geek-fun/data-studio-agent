<div align="center">

# data-studio-agent

**Turn your AI coding agent into a database assistant — query, explore, and understand your databases in plain language.**

**Local-first. Your data never leaves your machine. Open source.**

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

English · [简体中文](README_zh.md)

</div>

---

This repository is home to the **Data Studio MCP Server** — a unified [Model Context Protocol](https://modelcontextprotocol.io/) server that gives AI coding agents (Claude Code, Cursor, Windsurf, OpenCode, Codex) direct access to your databases through the [dockit](https://github.com/geek-fun/dockit) and [sqlkit](https://github.com/geek-fun/sqlkit) desktop apps.

- **SQL** (via sqlkit): PostgreSQL, MySQL, SQL Server, SQLite
- **NoSQL** (via dockit): Elasticsearch, OpenSearch, MongoDB, DynamoDB

## Quick start

### 1. Prerequisites

Install and launch [dockit](https://github.com/geek-fun/dockit) and/or [sqlkit](https://github.com/geek-fun/sqlkit), add a database connection, and make sure **Settings → MCP Bridge → Auto-start** is enabled (default). Install both apps for the full SQL + NoSQL tool set.

### 2. Install the MCP server

```bash
npm install -g @geek-fun/data-studio-mcp
```

Or run without installing:

```bash
npx -y @geek-fun/data-studio-mcp
```

### 3. Add it to your AI tool

**Claude Code** — add to your MCP config:

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

**Cursor / Windsurf / OpenCode / other MCP clients** — register a stdio server:

| Setting | Value |
|---|---|
| Type | stdio |
| Command | `npx` |
| Args | `-y @geek-fun/data-studio-mcp` |

### 4. Start asking

Just use plain language — the agent queries your databases for you:

- "List all tables in my PostgreSQL database"
- "Show me the last 10 orders from the Elasticsearch index `orders*`"
- "Find all users older than 30 in MongoDB"
- "Run this query and explain the results"

The agent reads schemas, runs queries, and explores your data — and shows you every query it executes.

## Tool naming

All tools follow the `data_studio__{backend}__{action}` convention:

| Prefix | Backend | Examples |
|---|---|---|
| `data_studio__sql_*` | sqlkit | `data_studio__sql_execute`, `data_studio__sql_list_tables` |
| `data_studio__es_*` | dockit | `data_studio__es_search`, `data_studio__es_list_indices` |
| `data_studio__mongo_*` | dockit | `data_studio__mongo_find`, `data_studio__mongo_insert` |
| `data_studio__dynamo_*` | dockit | `data_studio__dynamo_query`, `data_studio__dynamo_list_tables` |

## Safety

- The bridge binds to `127.0.0.1` only — unreachable from other machines
- **Destructive and elevated operations are rejected by the bridge** — only read-safe capabilities are exposed through the MCP server
- Credentials are never exposed to the agent; all connections are resolved inside the desktop apps

## How it works

```
code agent (Claude Code / Cursor / OpenCode ...)
    |
    | MCP stdio protocol
    v
@geek-fun/data-studio-mcp   ← npm package (pure TypeScript)
    |
    | HTTP (localhost)
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

The MCP server is a thin routing layer. All database drivers, SSH tunnels, and connection management live in the desktop apps, which expose a local HTTP bridge (`127.0.0.1` only). The MCP server auto-discovers running backends via each app's port file.

---

# For developers: the data-studio-agent Rust framework

Under the hood, this repository also contains the shared Rust agent framework that powers the built-in AI assistants in [dockit](https://github.com/geek-fun/dockit) and [sqlkit](https://github.com/geek-fun/sqlkit) — a single crate providing a complete AI agent loop: provider adapters, streaming, tool calling, context compaction, conversation management, generic over pluggable storage and eventing.

## Install

```toml
[dependencies]
data-studio-agent = { git = "https://github.com/geek-fun/data-studio-agent", tag = "v0.1.4" }
```

Opt out of SQLite if you provide your own `SessionStore`:

```toml
data-studio-agent = { git = "https://github.com/geek-fun/data-studio-agent", tag = "v0.1.4", default-features = false }
```

## Project structure

```
data-studio-agent/
├── Cargo.toml              # single crate, feature-gated
├── src/
│   ├── lib.rs
│   ├── traits.rs           # SessionStore, EventEmitter
│   ├── chat_formatter/     # OpenAI + Anthropic
│   ├── provider_adapter.rs
│   ├── model_registry.rs
│   ├── token_counter.rs
│   ├── tool_executor.rs    # ToolExecutor trait
│   ├── loop_runner.rs      # ReAct agent loop
│   ├── compact.rs          # Context compaction
│   ├── conversation.rs     # Message lifecycle
│   ├── harness.rs          # Single-step LLM calls
│   ├── tools.rs            # Tool resolution
│   ├── loop_runner_support.rs
│   ├── capabilities/       # CapabilityRegistry
│   ├── common/             # HTTP client, formatting
│   └── storage/            # #[cfg(feature = "sqlite-storage")]
│       ├── mod.rs
│       ├── db.rs           # AgentDb, schema migration
│       └── session_store.rs
├── packages/
│   └── data-studio-mcp/    # MCP server (TypeScript)
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

## Design

Two traits decouple the agent loop from any framework:

| Trait | Role | App provides |
|-------|------|-------------|
| `SessionStore` | Persist messages, tool calls, sessions | `SqliteSessionStore` (built-in) or custom impl |
| `EventEmitter` | Stream deltas, status, errors | `TauriEmitter(AppHandle)` or any impl |

The loop itself knows nothing about Tauri, SQLite, or any specific tool — it's pure async Rust generic over these traits.

## Quick start (Rust integration)

### 1. Initialize the database

```rust
use data_studio_agent::storage::{self, session_store::SqliteSessionStore};

let db_path = app.path().app_data_dir()?.join("agent.sqlite");
let agent_db = storage::db::open(&db_path)?;
storage::db::migrate(&agent_db)?;
app.manage(agent_db);
```

### 2. Wire Tauri commands

Create `agent_adapters.rs` with thin wrappers. Each command extracts Tauri state, builds a `TauriEmitter`, and delegates to the lib:

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

### 3. Register commands

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

### 4. Implement ToolExecutor

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

## Supported providers

- **OpenAI** (GPT-4o, GPT-4.1, o1/o3) — `/v1/chat/completions`
- **Anthropic** (Claude 3.5/4) — `/v1/messages`
- **Ollama** / **LM Studio** — local models
- **OpenRouter** / **DeepSeek** / any OpenAI-compatible endpoint

## Capabilities

| Feature | Detail |
|---------|--------|
| **ReAct loop** | Tool calling with retry + exponential backoff |
| **Confirmation gating** | Per-tool Allow/Deny via oneshot channels |
| **Runaway guard** | Stops if same tool call repeats 3× consecutively |
| **Context compaction** | Auto-summarizes when context fills, safe split points |
| **Token budgets** | 200 iterations, 30min wall clock, 20M tokens |
| **Compaction locking** | Single per-session mutex for all compaction paths |
| **Streaming** | SSE parsing via provider-specific formatters |
| **SQLite persistence** | Canonical schema, per-app data isolation |

## Build

```bash
cargo build
cargo test
cargo clippy --all-features -- -D warnings
cargo fmt --all -- --check
```

## License

Apache 2.0 — see [LICENSE](LICENSE).
