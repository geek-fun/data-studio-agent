use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

use crate::tool_executor::ToolExecutor;

// ---------------------------------------------------------------------------
// SessionStore — application-provided persistence layer
// ---------------------------------------------------------------------------

/// Application-provided trait for all session/message/tool persistence.
/// Each app implements this with its own SQLite (or other) backend.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Load messages visible in the current compaction window for the session.
    /// Returns (id, role, content) tuples ordered by created_at ASC.
    async fn load_active_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<(String, String, String)>, String>;

    /// Write a message to the session's message log.
    async fn write_message(
        &self,
        id: &str,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), String>;

    /// Update the session's status (e.g. "running", "idle").
    async fn update_session_status(&self, session_id: &str, status: &str) -> Result<(), String>;

    /// Insert a tool call record.
    async fn insert_tool_call(
        &self,
        id: &str,
        message_id: &str,
        session_id: &str,
        tool_name: &str,
        arguments: &str,
        status: &str,
    ) -> Result<(), String>;

    /// Update a tool call's status.
    async fn update_tool_call_status(&self, id: &str, status: &str) -> Result<(), String>;

    /// Insert tool execution result; returns the result row id.
    async fn insert_tool_result(
        &self,
        tool_call_id: &str,
        full_result: &str,
    ) -> Result<String, String>;

    /// Load messages from the last compaction boundary for compaction evaluation.
    async fn load_messages_for_compact(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredMessage>, String>;

    /// Load ALL messages for the session (ignoring compaction boundaries).
    async fn load_all_messages(&self, session_id: &str) -> Result<Vec<StoredMessage>, String>;

    /// Get the per-session compaction lock. Only one compaction runs per session at a time.
    fn compact_lock(&self, session_id: &str) -> Arc<AsyncMutex<()>>;
}

// ---------------------------------------------------------------------------
// EventEmitter — application-provided event/streaming notification layer
// ---------------------------------------------------------------------------

/// Application-provided trait for emitting structured events during
/// agent loop execution (streaming deltas, status changes, errors).
pub trait EventEmitter: Send + Sync {
    fn emit(&self, event: &str, payload: Value);
}

// ---------------------------------------------------------------------------
// Structured message used by compaction and the projection layer.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct StoredMessage {
    pub id: String,
    pub role: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// AgentContext — the full context needed to run the agent loop.
// Bundles all the trait dependencies into a single struct for convenience.
// ---------------------------------------------------------------------------

pub type ConfirmMap = Arc<std::sync::Mutex<HashMap<String, tokio::sync::oneshot::Sender<bool>>>>;
pub type CancelMap = Arc<std::sync::Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>>;

/// Bundled context for running the agent loop.
pub struct AgentContext<S: SessionStore, E: EventEmitter> {
    pub store: Arc<S>,
    pub emitter: Arc<E>,
    pub tool_executor: Arc<dyn ToolExecutor>,
    pub confirm_map: ConfirmMap,
}
