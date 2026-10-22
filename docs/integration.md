# Integration Guide

How to add the shared agent library to a new (or existing) Tauri application.

## Prerequisites

- Rust edition 2021
- Tauri v2

## Step 1: Add Dependency

```toml
# Production — replace with a full commit SHA containing the API used below
[dependencies]
data-studio-agent = { git = "https://github.com/geek-fun/data-studio-agent.git", rev = "<full-commit-sha>" }
```

During local development (crate lives at `crates/data-studio-agent` in the repo):

```toml
data-studio-agent = { path = "../../data-studio-agent/crates/data-studio-agent" }
```

## Step 2: Initialize the Database

In your `lib.rs` setup closure:

```rust
use data_studio_agent::storage;

.setup(|app| {
    let app_data_dir = app.path().app_data_dir()?;
    let db_path = app_data_dir.join("agent.sqlite");
    let agent_db = storage::db::open(&db_path)?;
    storage::db::migrate(&agent_db)?;

    // Recover stuck sessions from previous crash
    {
        let conn = agent_db.0.lock().map_err(|e| e.to_string())?;
        storage::db::recover_stuck_sessions(&conn)?;
    }
    app.manage(agent_db);
    Ok(())
})
```

## Step 3: Implement ToolExecutor

The minimal legacy adapter below implements `execute` only. It remains supported, but its string errors are treated as retryable failures. For changing permissions or source scope, also implement the host hooks shown next.

```rust
use async_trait::async_trait;
use data_studio_agent::tool_executor::{ToolEnvelope, ToolExecutor, ToolResultMetadata};
use serde_json::Value;

pub struct MyToolExecutor;

#[async_trait]
impl ToolExecutor for MyToolExecutor {
    async fn execute(
        &self,
        tool_name: &str,
        arguments: &Value,
        connection_config: &Value,
    ) -> Result<ToolEnvelope, String> {
        let start = std::time::Instant::now();
        let conn_opt = if connection_config.is_null() { None } else { Some(connection_config.clone()) };

        let raw = crate::capabilities::registry::invoke_capability_inner(
            tool_name, arguments.clone(), conn_opt,
        ).await?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let max_chars = 32768;
        let (full_result, truncated) = if raw.chars().count() <= max_chars {
            (raw.clone(), false)
        } else {
            (raw.chars().take(max_chars).collect::<String>(), true)
        };
        let (summary, _) = if full_result.chars().count() <= 4096 {
            (full_result.clone(), false)
        } else {
            (full_result.chars().take(4096).collect::<String>(), true)
        };

        Ok(ToolEnvelope {
            summary,
            full_result,
            metadata: ToolResultMetadata {
                tool_name: tool_name.to_string(),
                duration_ms,
                truncated,
            },
        })
    }
}
```

### Hosts with authorization and confirmation

The [complete, compilable host adapter](../crates/data-studio-agent/examples/host_executor.rs) defines `HostAccess` as application-provided inspection and dispatch. `inspect` returns an error for denied access, or a boolean indicating whether the allowed operation requires confirmation. `dispatch` uses the application's final execution gate. This adapter intentionally transforms the original config:

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

Register it in setup:

```rust
use std::sync::Arc;
let executor: Arc<dyn data_studio_agent::ToolExecutor> = Arc::new(MyToolExecutor);
app.manage(executor);
```

## Step 4: Create Adapter Module

Create `src/agent_adapters.rs` with Tauri command wrappers:

```rust
use std::collections::HashMap;
use std::sync::Arc;

use data_studio_agent as lib;
use data_studio_agent::traits::{CancelMap, ConfirmMap, EventEmitter};
use data_studio_agent::storage;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};

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
    let db_state: State<storage::db::AgentDb> = app.state::<storage::db::AgentDb>();
    let store = storage::session_store::SqliteSessionStore::new(db_state.inner().clone());
    let emitter = TauriEmitter(app.clone());

    let confirm_state: State<ConfirmMap> = app.state::<ConfirmMap>();
    let confirm_map = confirm_state.inner().clone();
    let cancel_state: State<CancelMap> = app.state::<CancelMap>();
    let cancel_map = cancel_state.inner().clone();
    let executor_state: State<Arc<dyn lib::ToolExecutor>> = app.state::<Arc<dyn lib::ToolExecutor>>();
    let executor = executor_state.inner().clone();

    let connections: HashMap<String, Value> = settings
        .get("connections")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let fallback = settings.get("connectionConfig").cloned().unwrap_or(Value::Null);

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

// Also register these thin wrappers:
// - cancel_agent_loop → lib::loop_runner::cancel_agent_loop()
// - confirm_tool_call → lib::loop_runner::confirm_tool_call()
// - compact_agent_session → lib::loop_runner::compact_agent_session()
// - get_agent_context_usage → lib::loop_runner::get_agent_context_usage()
// - get_tool_full_result → direct SQLite query (or via store)
// - run_agent_step → lib::harness::run_agent_step()
// - validate_llm_config → lib::harness::validate_llm_config()
// - list_llm_models → lib::harness::list_llm_models()
// - get_all_tools → lib::tools::get_all_tools()
```

## Step 5: Register Commands

In your `lib.rs`:

```rust
pub mod agent_adapters;

// In setup:
let confirm_map: lib::traits::ConfirmMap = Arc::new(Mutex::new(HashMap::new()));
let cancel_map: lib::traits::CancelMap = Arc::new(Mutex::new(HashMap::new()));
app.manage(confirm_map);
app.manage(cancel_map);

// In invoke_handler:
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

## Step 6: Clean Up

Remove any old duplicated agent code:

```bash
# Delete files now provided by the lib
rm -rf src-tauri/src/agent/chat_formatter/
rm src-tauri/src/agent/compact.rs
rm src-tauri/src/agent/config.rs
rm src-tauri/src/agent/conversation.rs
rm src-tauri/src/agent/harness.rs
rm src-tauri/src/agent/loop_runner.rs
rm src-tauri/src/agent/loop_runner_support.rs
rm src-tauri/src/agent/model_registry.rs
rm src-tauri/src/agent/provider_adapter.rs
rm src-tauri/src/agent/token_counter.rs
rm src-tauri/src/agent/tool_executor.rs
rm src-tauri/src/agent/tools.rs

# Keep these app-specific files:
# src-tauri/src/agent/executor.rs   — your ToolExecutor impl
# src-tauri/src/agent/session_store.rs — your session CRUD commands
# src-tauri/src/agent/query_history.rs — your query history commands
```

Remove stale dependencies from `Cargo.toml` (no longer needed, provided by the lib):

```toml
# Remove or comment out:
# async-openai = "..."
# tiktoken-rs = "..."
```

## Custom SessionStore

If you prefer a different storage backend, implement the `SessionStore` trait directly:

```rust
use async_trait::async_trait;
use data_studio_agent::traits::{SessionStore, StoredMessage};
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

pub struct MyCustomStore {
    // your connection pool, in-memory store, etc.
}

#[async_trait]
impl SessionStore for MyCustomStore {
    async fn load_active_history(&self, session_id: &str) -> Result<Vec<(String, String, String)>, String> {
        // Your implementation
        todo!()
    }
    // ... implement remaining 8 methods
}
```

Then use it in your adapter instead of `SqliteSessionStore`:

```rust
let store = MyCustomStore::new(my_db_pool.clone());
lib::loop_runner::run_agent_loop(..., &store, ...).await
```
