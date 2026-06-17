# Architecture

## Layered Design

```
┌─────────────────────────────────────────────────────────────┐
│  Tauri App (dockit / sqlkit / your-app)                     │
│  ┌──────────────┐  ┌────────────────┐  ┌─────────────────┐  │
│  │ agent_adapters│  │  capabilities  │  │  session_store  │  │
│  │ (Tauri cmds)  │  │  (tool impls)  │  │  (CRUD wrappers)│  │
│  └──────┬───────┘  └───────┬────────┘  └────────┬────────┘  │
│         │                  │                     │           │
├─────────┼──────────────────┼─────────────────────┼───────────┤
│         ▼                  ▼                     ▼           │
│  ┌──────────────────────────────────────────────────────┐   │
│  │          data-studio-agent-lib                        │   │
│  │  ┌─────────────┐ ┌──────────────┐ ┌───────────────┐  │   │
│  │  │ loop_runner │ │   compact    │ │ conversation  │  │   │
│  │  │ (ReAct loop)│ │ (summarize)  │ │ (lifecycle)   │  │   │
│  │  └──────┬──────┘ └──────┬───────┘ └───────┬───────┘  │   │
│  │         │               │                  │          │   │
│  │  ┌──────┴───────────────┴──────────────────┴───────┐  │   │
│  │  │           traits (SessionStore + EventEmitter)   │  │   │
│  │  └────────────────────────┬────────────────────────┘  │   │
│  │  ┌────────────┬───────────┼────────────┬───────────┐  │   │
│  │  │formatter   │provider   │model/token │capability │  │   │
│  │  │(OpenAI/    │adapter    │counter     │registry   │  │   │
│  │  │ Anthropic) │(endpoints)│(tiktoken)  │(tools)    │  │   │
│  │  └────────────┴───────────┴────────────┴───────────┘  │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │          data-studio-agent-storage-sqlite             │   │
│  │  ┌──────────────┐  ┌───────────────────────┐         │   │
│  │  │   AgentDb    │  │  SqliteSessionStore   │         │   │
│  │  │ (open/migrate)│  │ (impl SessionStore)   │         │   │
│  │  └──────────────┘  └───────────────────────┘         │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Trait-Based Abstraction

### SessionStore

Replaces all direct `db.0.lock()` / raw SQL calls. Each app provides an implementation — typically `SqliteSessionStore` from `storage-sqlite`, but any backend works (Postgres, in-memory, mock for tests).

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn load_active_history(&self, session_id: &str) -> Result<Vec<(String, String, String)>, String>;
    async fn write_message(&self, id: &str, session_id: &str, role: &str, content: &str) -> Result<(), String>;
    async fn update_session_status(&self, session_id: &str, status: &str) -> Result<(), String>;
    async fn insert_tool_call(&self, id: &str, message_id: &str, session_id: &str, tool_name: &str, arguments: &str, status: &str) -> Result<(), String>;
    async fn update_tool_call_status(&self, id: &str, status: &str) -> Result<(), String>;
    async fn insert_tool_result(&self, tool_call_id: &str, full_result: &str) -> Result<String, String>;
    async fn load_messages_for_compact(&self, session_id: &str) -> Result<Vec<StoredMessage>, String>;
    async fn load_all_messages(&self, session_id: &str) -> Result<Vec<StoredMessage>, String>;
    fn compact_lock(&self, session_id: &str) -> Arc<AsyncMutex<()>>;
}
```

### EventEmitter

Replaces all `app.emit()` Tauri calls. The Tauri app provides a thin wrapper:

```rust
struct TauriEmitter(AppHandle);

impl EventEmitter for TauriEmitter {
    fn emit(&self, event: &str, payload: Value) {
        let _ = self.0.emit(event, payload);
    }
}
```

## Event Flow

During an agent loop iteration, these events are emitted (via EventEmitter):

| Event | When |
|-------|------|
| `agent-loop-delta` | Streaming content arrives from LLM |
| `agent-loop-thinking-delta` | Reasoning/thinking content arrives |
| `agent-loop-iteration` | Heartbeat before each loop iteration |
| `agent-loop-waiting-llm` | Just before sending request to LLM |
| `agent-loop-tool-call` | Tool call needs user confirmation |
| `agent-loop-tool-result` | Tool execution completed |
| `agent-loop-step-done` | Assistant message written |
| `agent-loop-summary-injected` | Compaction completed |
| `agent-loop-compacting` | Compaction phase start/end |
| `agent-loop-warning` | Non-fatal warning |
| `agent-loop-error` | Fatal error, loop stopped |
| `agent-loop-stopped` | Budget exceeded (iteration/time/tokens) |
| `agent-loop-done` | Loop completed successfully |
| `agent-context-usage` | Context usage snapshot |

## Compaction Strategy

The compaction system uses a two-pass approach:

1. **Backward pass**: Walk backward from `proposed_split` to find a safe boundary that doesn't orphan tool messages or sever assistant `tool_calls → tool` pairs.
2. **Forward fallback**: If backward collapses to zero (tool-heavy history), walk forward from the keep_pairs target.
3. **Keep-pairs strategy**: Try 4 pairs → 2 pairs → 1 pair before falling back to forward search.
4. **Lock serialization**: All compaction paths (in-loop, background, manual) serialize through a single per-session `AsyncMutex`.

## Database Schema

The canonical schema in `storage-sqlite`:

```sql
CREATE TABLE agent_sessions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'idle',
    sources TEXT NOT NULL DEFAULT '[]',
    permissions_mode TEXT NOT NULL DEFAULT 'Ask',
    model_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE agent_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE agent_tool_calls (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES agent_messages(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    arguments TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at INTEGER NOT NULL
);

CREATE TABLE tool_result_store (
    id TEXT PRIMARY KEY,
    tool_call_id TEXT NOT NULL REFERENCES agent_tool_calls(id) ON DELETE CASCADE,
    full_result TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE confirmation_rules (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL,
    action TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(session_id, tool_name)
);

CREATE TABLE attached_sources (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    -- ... additional source metadata columns
);
```

Each app uses its own SQLite file — no shared data between apps.
