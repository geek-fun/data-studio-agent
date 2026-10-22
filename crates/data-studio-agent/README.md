# data-studio-agent

Shared Rust agent framework extracted from [dockit](https://github.com/geek-fun/dockit) and [sqlkit](https://github.com/geek-fun/sqlkit). A single crate that provides a complete AI agent loop — provider adapters, streaming, tool calling, context compaction, conversation management — generic over pluggable storage and eventing.

## Install

```toml
[dependencies]
data-studio-agent = { git = "https://github.com/geek-fun/data-studio-agent.git", rev = "<full-commit-sha>" }
```

Replace `<full-commit-sha>` with a revision containing the host hooks documented below. Opt out of SQLite if you provide your own `SessionStore`:

```toml
data-studio-agent = { git = "https://github.com/geek-fun/data-studio-agent.git", rev = "<full-commit-sha>", default-features = false }
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

    let is_parallel_ok = |name: &str| {
        lib::capabilities::registry::registry().get(name)
            .map(|cap| cap.parallel_ok).unwrap_or(false)
    };
    // Authorization is enforced by the host hooks and final execution gate.
    let legacy_deny_hook = |_: &str, _: Option<&str>| false;
    lib::loop_runner::run_agent_loop(
        &session_id, &user_message, &settings,
        &store, &emitter, executor.as_ref(),
        connections, fallback,
        &confirm_map, &cancel_map,
        &is_parallel_ok, &legacy_deny_hook,
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

Implement both `prepare` and `execute_checked` when authorization can change while a call waits for approval. The existing `execute` method remains required for compatibility; delegate it to `execute_checked` so direct use also revalidates access.

The [compilable adapter example](examples/host_executor.rs) leaves policy and dispatch in the application through `HostAccess`. Inspection returns `Err` for denial or whether confirmation is required. Its implementation is:

```rust
#[async_trait]
impl<H: HostAccess> ToolExecutor for HostExecutor<H> {
    fn prepare(
        &self,
        tool: &str,
        args: &Value,
        original: &Value,
        _connections: &HashMap<String, Value>,
    ) -> Result<ToolCallPreparation, String> {
        let requires_confirmation = self.host.inspect(tool, args, original)?;
        Ok(ToolCallPreparation {
            connection_config: json!({
                "connection": original,
                "presented_confirmation": requires_confirmation,
            }),
            risk_level: None,
            confirmation_required: Some(requires_confirmation),
        })
    }

    async fn execute_checked(
        &self,
        tool: &str,
        args: &Value,
        prepared: &Value,
    ) -> Result<ToolEnvelope, ToolExecutionError> {
        let original = prepared
            .get("connection")
            .ok_or_else(|| ToolExecutionError::Denied("Missing host preparation context".into()))?;
        let requires_confirmation =
            self.host.inspect(tool, args, original).map_err(ToolExecutionError::Denied)?;
        let presented = prepared["presented_confirmation"].as_bool().unwrap_or(false);
        if requires_confirmation && !presented {
            return Err(ToolExecutionError::ConfirmationRequired);
        }
        self.host.dispatch(tool, args, original, presented).await
    }

    async fn execute(
        &self,
        tool: &str,
        args: &Value,
        prepared: &Value,
    ) -> Result<ToolEnvelope, String> {
        self.execute_checked(tool, args, prepared).await.map_err(|error| error.to_string())
    }
}
```

The host owns authorization; the runtime owns confirmation and retry orchestration:

| Outcome | Runtime behavior |
|---------|------------------|
| `prepare` returns `Err` | Persist denial; do not emit a confirmation event or execute |
| `Denied(message)` | Stop immediately; no retries or further confirmation |
| `Failed(message)` | Retry execution up to three failed attempts (two retries) |
| `ConfirmationRequired` | Re-run `prepare` with the original resolved config, emit a new tool-call event, and await approval |

Each tool call permits at most three confirmation renewals, independently of transport retries. A fourth renewal request terminates with a failure, without another prompt or transport retry.

Every `prepare` receives the original input, so its transformation need not be idempotent. `execute_checked` receives the latest prepared context only after approval and must recheck current permissions and source scope on every attempt. Capture the confirmation requirements presented to the user in that context; they describe what was presented, not proof of approval or an authorization grant. Return `ConfirmationRequired` only if the current allowed operation needs stricter confirmation than that captured context. Never map authorization errors through the default `From<String>` conversion: that produces retryable `Failed`; use `Denied` explicitly.

The client must honor host-provided `confirmation_required` metadata and reply through the existing confirmation API. An automatic approval reply remains subject to current host authorization. Do not hold application/database locks while waiting for approval.

Use a session-bound executor when authorization depends on conversation attachments. A globally registered executor is appropriate only when it carries no mutable per-session scope.

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
