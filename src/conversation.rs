// ---------------------------------------------------------------------------
// Conversation management — message persistence, context-usage tracking,
// and background compaction orchestration.
// ---------------------------------------------------------------------------
//
// All functions are generic over the **SessionStore** (persistence) and
// **EventEmitter** (UI / streaming notifications) traits so that any
// application — Tauri, CLI, test harness — can provide its own backend.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{json, Value};
use tokio::sync::Mutex as AsyncMutex;

use crate::compact::{
    count_projected_tokens, evaluate, resolve_model_spec_for_session, run_compact_with_events,
};
use crate::traits::{EventEmitter, SessionStore, StoredMessage};

// ---------------------------------------------------------------------------
// Per-session compaction lock registry (global)
// ---------------------------------------------------------------------------

/// Ensures only one compaction runs per session at a time, regardless of
/// whether it was triggered by `append()` (background) or `prepare_for_llm()`
/// (foreground).
static SESSION_COMPACT_LOCKS: OnceLock<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>> =
    OnceLock::new();

/// Sessions that already have a background compaction queued or running.
/// Used by `append()` to avoid stacking N redundant background tasks while
/// one is already in flight.
static SESSION_COMPACT_INFLIGHT: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn inflight_set() -> &'static Mutex<HashSet<String>> {
    SESSION_COMPACT_INFLIGHT.get_or_init(|| Mutex::new(HashSet::new()))
}

fn try_acquire_inflight(session_id: &str) -> bool {
    let mut set = inflight_set().lock().expect("compact inflight set poisoned");
    set.insert(session_id.to_string())
}

fn release_inflight(session_id: &str) {
    let mut set = inflight_set().lock().expect("compact inflight set poisoned");
    set.remove(session_id);
}

/// Return or create the per-session compaction mutex.
///
/// Only one compaction runs per session at a time — this mutex serializes
/// both background (from `append`) and foreground (from `prepare_for_llm`)
/// compaction attempts.
pub fn lock_for(session_id: &str) -> Arc<AsyncMutex<()>> {
    let map_mu = SESSION_COMPACT_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map_mu.lock().expect("compact lock map poisoned");
    map.entry(session_id.to_string()).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone()
}

// ---------------------------------------------------------------------------
// Inflight guard (RAII)
// ---------------------------------------------------------------------------

struct InflightGuard {
    session_id: String,
}

impl InflightGuard {
    fn new(session_id: String) -> Self {
        InflightGuard { session_id }
    }
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        release_inflight(&self.session_id);
    }
}

// ---------------------------------------------------------------------------
// Settings helpers
// ---------------------------------------------------------------------------

fn auto_compact_enabled(settings: &Value) -> bool {
    settings.get("autoCompact").and_then(|v| v.as_bool()).unwrap_or(true)
}

fn is_tool_heavy_compaction_warning(message: &str) -> bool {
    message.starts_with("Context compaction failed: history has too many consecutive tool calls")
}

// ---------------------------------------------------------------------------
// Context-usage emission
// ---------------------------------------------------------------------------

/// Load messages visible in the current compaction window, compute token
/// usage against the model's context window, and emit an
/// `"agent-context-usage"` event so the UI can display the progress bar.
async fn emit_usage<S: SessionStore, E: EventEmitter>(
    store: &S,
    emitter: &E,
    session_id: &str,
    settings: &Value,
) {
    let Ok(messages) = store.load_messages_for_compact(session_id).await else {
        return;
    };
    let spec = resolve_model_spec_for_session(session_id, settings);
    let decision = evaluate(&messages, &spec);
    let system_prompt = settings.get("systemPrompt").and_then(|v| v.as_str());
    let tools = settings.get("tools");
    let used_tokens = count_projected_tokens(&messages, system_prompt, tools, &spec);
    let should_compact = used_tokens >= decision.trigger_at;
    emitter.emit(
        "agent-context-usage",
        json!({
            "session_id": session_id,
            "used_tokens": used_tokens,
            "capacity": decision.capacity,
            "context_window": spec.context_window,
            "output_reserve": spec.output_reserve,
            "trigger_at": decision.trigger_at,
            "should_compact": should_compact,
            "model": spec.model_id,
        }),
    );
}

// ---------------------------------------------------------------------------
// Compaction-boundary detection
// ---------------------------------------------------------------------------

fn is_compact_boundary(msg: &StoredMessage) -> bool {
    if msg.role != "system" {
        return false;
    }
    serde_json::from_str::<Value>(&msg.content)
        .ok()
        .and_then(|v| v.get("_compact_boundary").and_then(|b| b.as_bool()))
        .unwrap_or(false)
}

fn has_compactable_content_since_boundary(messages: &[StoredMessage]) -> bool {
    let last_boundary_idx = messages.iter().rposition(is_compact_boundary);
    let start = last_boundary_idx.map(|i| i + 1).unwrap_or(0);
    messages[start..].iter().any(|m| matches!(m.role.as_str(), "user" | "assistant" | "tool"))
}

// ---------------------------------------------------------------------------
// Public API — compaction checks
// ---------------------------------------------------------------------------

/// Check whether the session's message history has crossed the compaction
/// trigger threshold for the configured model.
///
/// Returns `false` if the session has no compactable content since the last
/// boundary, or if the store call fails.
pub async fn needs_compact<S: SessionStore>(store: &S, session_id: &str, settings: &Value) -> bool {
    let Ok(messages) = store.load_messages_for_compact(session_id).await else {
        return false;
    };
    if !has_compactable_content_since_boundary(&messages) {
        return false;
    }
    let spec = resolve_model_spec_for_session(session_id, settings);
    evaluate(&messages, &spec).should_compact
}

// ---------------------------------------------------------------------------
// Public API — message append
// ---------------------------------------------------------------------------

/// Persist a message, emit the updated context-usage snapshot, and (when
/// `autoCompact` is enabled and no assistant tool-calls are pending) spawn a
/// background compaction task.
///
/// The function returns immediately — it does **not** wait for the LLM
/// summarization call.  Callers that need the next LLM payload to reflect the
/// post-compacted state **must** call `prepare_for_llm` before building the
/// request body.
pub async fn append<S: SessionStore + 'static, E: EventEmitter + 'static>(
    store: &Arc<S>,
    emitter: &Arc<E>,
    settings: &Value,
    session_id: &str,
    id: &str,
    role: &str,
    content: &str,
) -> Result<(), String> {
    // 1. Persist the message (the store implementation handles updated_at).
    store.write_message(id, session_id, role, content).await?;

    // 2. Emit an up-to-date context-usage event.
    emit_usage(store.as_ref(), emitter.as_ref(), session_id, settings).await;

    // 3. Do NOT trigger background compaction immediately after an assistant
    //    message that contains tool_calls: the tool results have not been
    //    written yet, so compaction would see a dangling assistant+tool_calls
    //    with no following tool messages and produce a payload that OpenAI
    //    rejects with HTTP 400.  The `prepare_for_llm` gate runs before every
    //    LLM call and is the correct place to compact in that scenario.
    let has_pending_tool_calls = role == "assistant"
        && serde_json::from_str::<Value>(content)
            .ok()
            .and_then(|v| v.get("tool_calls").and_then(|tc| tc.as_array()).map(|a| !a.is_empty()))
            .unwrap_or(false);

    if !has_pending_tool_calls && auto_compact_enabled(settings) && try_acquire_inflight(session_id)
    {
        spawn_background_compact(
            store.clone(),
            emitter.clone(),
            settings.clone(),
            session_id.to_string(),
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Public API — pre-LLM gate
// ---------------------------------------------------------------------------

/// Blocking gate called by the loop right before sending a request to the LLM.
///
/// If a background compaction is in flight (or queued via the mutex), this
/// awaits it so the next LLM payload reflects the post-compact state.  Then
/// runs one final compaction synchronously if `needs_compact` is still true.
///
/// When compaction runs, `compact.rs` emits `agent-loop-compacting` phase
/// start/end around the summarize LLM call.
pub async fn prepare_for_llm<S: SessionStore, E: EventEmitter>(
    store: &S,
    emitter: &E,
    settings: &Value,
    session_id: &str,
) -> Result<(), String> {
    if !auto_compact_enabled(settings) {
        emit_usage(store, emitter, session_id, settings).await;
        return Ok(());
    }

    let lock = lock_for(session_id);
    let _guard = lock.lock().await;

    if needs_compact(store, session_id, settings).await {
        match run_compact_with_events(session_id, settings, store, emitter).await {
            Ok(Some(info)) => {
                let mut summary_payload = json!({
                    "session_id": session_id,
                    "trigger": info.trigger,
                    "pre_tokens": info.pre_tokens,
                    "post_tokens": info.post_tokens,
                    "removed_count": info.removed_count,
                });
                if let Some(fallback_keep_pairs) = info.fallback_keep_pairs {
                    summary_payload["fallback_keep_pairs"] = json!(fallback_keep_pairs);
                }
                emitter.emit("agent-loop-summary-injected", summary_payload);
            },
            Ok(None) => {},
            Err(compact_err) => {
                if !is_tool_heavy_compaction_warning(&compact_err) {
                    emitter.emit(
                        "agent-loop-warning",
                        json!({
                            "session_id": session_id,
                            "warning": compact_err,
                        }),
                    );
                }
            },
        }
    }

    emit_usage(store, emitter, session_id, settings).await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Background compaction spawn
// ---------------------------------------------------------------------------

/// Fire-and-forget background compaction.  Acquires the per-session mutex so
/// it serializes with `prepare_for_llm`.  Errors are swallowed (emitted as
/// warnings) because the caller already returned.  Progress for long summarize
/// calls is emitted via `agent-loop-compacting` start/end from `compact.rs`.
fn spawn_background_compact<S: SessionStore + 'static, E: EventEmitter + 'static>(
    store: Arc<S>,
    emitter: Arc<E>,
    settings: Value,
    session_id: String,
) {
    tokio::spawn(async move {
        let _inflight = InflightGuard::new(session_id.clone());
        let lock = lock_for(&session_id);
        let _guard = lock.lock().await;

        let should_run = needs_compact(&*store, &session_id, &settings).await;
        if !should_run {
            return;
        }

        let result = run_compact_with_events(&session_id, &settings, &*store, &*emitter).await;

        match result {
            Ok(Some(info)) => {
                let mut summary_payload = json!({
                    "session_id": session_id,
                    "trigger": info.trigger,
                    "pre_tokens": info.pre_tokens,
                    "post_tokens": info.post_tokens,
                    "removed_count": info.removed_count,
                });
                if let Some(fallback_keep_pairs) = info.fallback_keep_pairs {
                    summary_payload["fallback_keep_pairs"] = json!(fallback_keep_pairs);
                }
                emitter.emit("agent-loop-summary-injected", summary_payload);
                emit_usage(&*store, &*emitter, &session_id, &settings).await;
            },
            Ok(None) => {},
            Err(compact_err) => {
                if !is_tool_heavy_compaction_warning(&compact_err) {
                    emitter.emit(
                        "agent-loop-warning",
                        json!({
                            "session_id": session_id,
                            "warning": compact_err,
                        }),
                    );
                }
            },
        }
    });
}
