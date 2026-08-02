# data-studio-agent

Shared Rust agent framework extracted from [dockit](https://github.com/geek-fun/dockit) and [sqlkit](https://github.com/geek-fun/sqlkit). A single crate that provides a complete AI agent loop — provider adapters, streaming, tool calling, context compaction, conversation management — generic over pluggable storage and eventing.

## Install

```toml
[dependencies]
data-studio-agent = { git = "https://github.com/geekfun/data-studio-agent", tag = "v0.1.4" }
```

Opt out of SQLite if you provide your own `SessionStore`:

```toml
data-studio-agent = { git = "https://github.com/geekfun/data-studio-agent", tag = "v0.1.4", default-features = false }
```

## Project structure

```
crates/data-studio-agent/
├── Cargo.toml              # single crate, feature-gated
└── src/
    ├── lib.rs
    ├── traits.rs           # SessionStore, EventEmitter
    ├── chat_formatter/     # OpenAI + Anthropic
    ├── provider_adapter.rs
    ├── model_registry.rs
    ├── token_counter.rs
    ├── tool_executor.rs    # ToolExecutor trait
    ├── loop_runner.rs      # ReAct agent loop
    ├── compact.rs          # Context compaction
    ├── conversation.rs     # Message lifecycle
    ├── harness.rs          # Single-step LLM calls
    ├── tools.rs            # Tool resolution
    ├── loop_runner_support.rs
    ├── capabilities/       # CapabilityRegistry
    ├── common/             # HTTP client, formatting
    └── storage/            # #[cfg(feature = "sqlite-storage")]
        ├── mod.rs
        ├── db.rs           # AgentDb, schema migration
        └── session_store.rs
```

Related docs: [architecture](../../docs/architecture.md) · [integration](../../docs/integration.md)

## Design

Two traits decouple the agent loop from any framework:

| Trait | Role | App provides |
|-------|------|-------------|
| `SessionStore` | Persist messages, tool calls, sessions | `SqliteSessionStore` (built-in) or custom impl |
| `EventEmitter` | Stream deltas, status, errors | `TauriEmitter(AppHandle)` or any impl |

The loop itself knows nothing about Tauri, SQLite, or any specific tool — it's pure async Rust generic over these traits.

```
┌──────────────────────────────────┐
│  Tauri app (dockit / sqlkit)     │
│  ┌────────────┐ ┌──────────────┐ │
│  │ adapters   │ │ capabilities │ │
│  │ (Tauri     │ │ (tool impls) │ │
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

## Quick start

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
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all -- --check
```

## License

Apache 2.0 — see [LICENSE](../../LICENSE).
