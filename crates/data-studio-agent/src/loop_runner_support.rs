// ---------------------------------------------------------------------------
// Loop-runner support utilities — shared by the agent loop and compaction.
// ---------------------------------------------------------------------------
//
// This module provides lightweight helpers (time, ID generation, LLM retry)
// that are used across multiple loop-related modules.

/// Re-export the compact LLM-call helper that lives in `compact.rs` to avoid
/// duplicating the retry logic.
pub use crate::compact::post_chat_completions_compact;
/// Re-export the structured message type used by compaction and the projection layer.
pub use crate::traits::StoredMessage;

// ---------------------------------------------------------------------------
// Time / ID helpers
// ---------------------------------------------------------------------------

/// Current wall-clock time as a UTC millisecond timestamp.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Generate a new v4 UUID string.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
