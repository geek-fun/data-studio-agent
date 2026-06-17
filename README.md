# data-studio-agent

Shared Rust agent framework for database studio applications. Extracted from [dockit](https://github.com/geekfun/dockit) and [sqlkit](https://github.com/geekfun/sqlkit).

## Overview

`data-studio-agent-lib` provides a complete AI agent loop — LLM provider adapters, chat formatters, streaming, tool calling, context compaction, conversation management — generic over pluggable storage and eventing backends. Apps integrate by implementing two traits and wiring a few Tauri commands.

## Workspace Structure

```
data-studio-agent/
├── Cargo.toml                     # [workspace] root
├── crates/
│   ├── agent-lib/                 # Core agent framework
│   │   └── src/
│   │       ├── traits.rs          # SessionStore, EventEmitter, StoredMessage
│   │       ├── chat_formatter/    # ChatFormatter trait + OpenAI + Anthropic
│   │       ├── provider_adapter.rs
│   │       ├── model_registry.rs
│   │       ├── token_counter.rs
│   │       ├── tool_executor.rs   # ToolExecutor trait + ToolEnvelope
│   │       ├── loop_runner.rs     # Generic ReAct agent loop
│   │       ├── compact.rs         # Context compaction
│   │       ├── conversation.rs    # Message lifecycle
│   │       ├── harness.rs         # Single-step LLM call
│   │       ├── tools.rs           # Tool resolution
│   │       ├── capabilities/      # Capability registry
│   │       └── common/            # HTTP client, formatting
│   │
│   └── storage-sqlite/            # SQLite SessionStore implementation
│       └── src/
│           ├── db.rs              # AgentDb, open(), migrate()
│           └── session_store.rs   # SqliteSessionStore
│
├── docs/                          # Architecture and integration guides
├── LICENSE                        # Apache 2.0
└── README.md
```

## Quick Start

### 1. Add as Dependency

In your Tauri app's `Cargo.toml`:

```toml
[dependencies]
data-studio-agent-lib = { path = "../data-studio-agent/crates/agent-lib" }
data-studio-agent-storage-sqlite = { path = "../data-studio-agent/crates/storage-sqlite" }
```

### 2. Initialize the Database

```rust
use data_studio_agent_storage_sqlite as storage;

let app_data_dir = app.path().app_data_dir()?;
let db_path = app_data_dir.join("agent.sqlite");
let agent_db = storage::db::open(&db_path)?;
storage::db::migrate(&agent_db)?;
app.manage(agent_db);
```

### 3. Wire Tauri Commands

Create an `agent_adapters.rs` module with thin wrappers:

```rust
use data_studio_agent_lib as lib;
use data_studio_agent_lib::traits::{CancelMap, ConfirmMap, EventEmitter};
use data_studio_agent_storage_sqlite as storage;

struct TauriEmitter(AppHandle);

impl EventEmitter for TauriEmitter {
    fn emit(&self, event: &str, payload: Value) {
        let _ = self.0.emit(event, payload);
    }
}

#[tauri::command]
pub async fn run_agent_loop(
    session_id: String,
    user_message: String,
    settings: Value,
    app: AppHandle,
) -> Result<(), String> {
    let db = app.state::<storage::db::AgentDb>();
    let store = storage::session_store::SqliteSessionStore::new(db.inner().clone());
    let emitter = TauriEmitter(app.clone());
    // ... wire confirm_map, cancel_map, tool_executor from Tauri state ...

    lib::loop_runner::run_agent_loop(
        &session_id, &user_message, &settings,
        &store, &emitter, executor.as_ref(),
        connections, fallback,
        &confirm_map, &cancel_map,
    ).await
}
```

### 4. Register Commands

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

### 5. Implement ToolExecutor

```rust
use data_studio_agent_lib::tool_executor::{ToolEnvelope, ToolExecutor, ToolResultMetadata};

pub struct MyToolExecutor;

#[async_trait]
impl ToolExecutor for MyToolExecutor {
    async fn execute(
        &self,
        tool_name: &str,
        arguments: &Value,
        connection_config: &Value,
    ) -> Result<ToolEnvelope, String> {
        // Delegate to your capability registry
        let raw = my_capability_registry::invoke(tool_name, arguments, connection_config).await?;
        Ok(ToolEnvelope {
            summary: truncate(&raw, 4096),
            full_result: truncate(&raw, 32768),
            metadata: ToolResultMetadata {
                tool_name: tool_name.to_string(),
                duration_ms: 0,
                truncated: raw.len() > 32768,
            },
        })
    }
}
```

## Key Abstractions

| Trait | Purpose | Call sites replaced |
|-------|---------|-------------------|
| `SessionStore` | Load/save messages, tool calls, sessions, compact boundaries | ~15 `db.0.lock()` calls |
| `EventEmitter` | Streaming deltas, status events, errors | ~22 `app.emit()` calls |
| `ToolExecutor` | Execute tool calls via app's capability registry | 1 `execute()` call per tool |

## Supported LLM Providers

- **OpenAI** (GPT-4o, GPT-4.1, o1/o3, etc.) — `/v1/chat/completions`
- **Anthropic** (Claude 3.5/4, Sonnet, Opus) — `/v1/messages`
- **Ollama** / **LM Studio** (local models) — `/api/chat`, `/api/tags`
- **OpenRouter** / **DeepSeek** / any OpenAI-compatible endpoint

## Features

- **ReAct agent loop** — tool calling with retry, confirmation gating, runaway guard
- **Context compaction** — automatic summarization when context fills, safe split points
- **Streaming SSE** — real-time content + thinking + tool call deltas via EventEmitter
- **Token budgets** — iteration cap (200), wall-clock budget (30min), token budget (20M)
- **Rate limit handling** — exponential backoff with jitter, retryable error classification
- **SQLite persistence** — canonical schema in `storage-sqlite`, per-app data isolation

## Build

```bash
cargo build --workspace
cargo test --workspace
```

## License

Apache 2.0 — see [LICENSE](LICENSE).
