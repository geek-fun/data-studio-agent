use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use rand::Rng;
use serde_json::{json, Value};

use crate::{
    chat_formatter::{ChatFormatter, LlmMessage, OpenAIChatFormatter},
    common::http_client::create_http_client,
    model_registry::{apply_overrides, resolve_spec, usable_window, ModelSpec, TokenizerFamily},
    provider_adapter::{build_headers, get_base_url},
    token_counter::{count_chat_messages, count_tools_tokens, estimate_stored_message},
    traits::{EventEmitter, SessionStore, StoredMessage},
};

// ---------------------------------------------------------------------------
// Type alias for backward compat / explicit re-export
// ---------------------------------------------------------------------------

/// Alias kept so callers that referenced `loop_runner_support::StoredMessage`
/// can migrate incrementally. Identical to `traits::StoredMessage`.
pub type LrsStoredMessage = StoredMessage;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Safety buffer scales with context window so large models get more room
/// and small models (Ollama 8K default) don't compact immediately.
/// ≥ 200K → 8,000 | ≥ 32K → 4,000 | < 32K → 2,000
pub fn safety_buffer_for_window(context_window: usize) -> usize {
    if context_window >= 200_000 {
        8_000
    } else if context_window >= 32_000 {
        4_000
    } else {
        2_000
    }
}

/// Compaction ratio: 85% for large windows (≥100K) — less aggressive
/// summarization when there's plenty of room. 75% otherwise.
pub fn compact_ratio_for_window(context_window: usize) -> f64 {
    if context_window >= 100_000 {
        0.85
    } else {
        0.75
    }
}
pub const KEEP_LAST_PAIRS: usize = 4;
#[allow(dead_code)]
pub const MAX_CONSECUTIVE_FAILURES: u32 = 3;

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct CompactDecision {
    pub capacity:       usize,
    pub trigger_at:     usize,
    pub should_compact: bool,
}

#[derive(Debug, Clone)]
pub struct CompactionInfo {
    pub trigger:             String,
    pub pre_tokens:          usize,
    pub post_tokens:         usize,
    pub removed_count:       usize,
    pub fallback_keep_pairs: Option<usize>,
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ---------------------------------------------------------------------------
// LLM HTTP helpers (retry logic lifted from loop_runner_support)
// ---------------------------------------------------------------------------

const RETRY_DELAYS_MS: &[u64] = &[1_000, 3_000, 8_000];
const RETRY_JITTER_MS: u64 = 250;
const RETRYABLE_ERROR_TYPES: &[&str] =
    &["rate_limit_exceeded", "insufficient_quota", "service_unavailable", "overloaded_error"];

fn classify_error(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    v.get("error").and_then(|e| e.get("type")).and_then(|t| t.as_str()).map(|s| s.to_string())
}

fn is_retryable(err_type: &str) -> bool {
    RETRYABLE_ERROR_TYPES.contains(&err_type)
}

async fn jittered_sleep_ms(base_ms: u64) {
    let jitter = rand::thread_rng().gen_range(-(RETRY_JITTER_MS as i64)..=RETRY_JITTER_MS as i64);
    let delay = (base_ms as i64 + jitter).max(0) as u64;
    tokio::time::sleep(Duration::from_millis(delay)).await;
}

/// POST to /chat/completions with retry logic.  Used by `summarize_with_llm`
/// and the agent loop's compaction pipeline.
pub async fn post_chat_completions_compact(
    http_client: &reqwest::Client,
    base_url: &str,
    headers: reqwest::header::HeaderMap,
    body: Value,
) -> Result<reqwest::Response, String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let mut last_err = String::from("LLM request failed");
    for (attempt, &delay) in RETRY_DELAYS_MS.iter().enumerate() {
        let resp = http_client.post(&url).headers(headers.clone()).json(&body).send().await;
        match resp {
            Ok(r) => {
                if r.status().is_success() {
                    return Ok(r);
                }
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                let err_type = classify_error(&text).unwrap_or_default();
                let retryable =
                    status.as_u16() == 429 || status.as_u16() == 503 || is_retryable(&err_type);
                last_err = format!("LLM HTTP {}: {}", status, text);
                if !retryable || attempt >= RETRY_DELAYS_MS.len() {
                    return Err(last_err);
                }
            },
            Err(e) => {
                last_err = format!("LLM request error: {}", e);
                if attempt >= RETRY_DELAYS_MS.len() {
                    return Err(last_err);
                }
            },
        }
        jittered_sleep_ms(delay).await;
    }
    Err(last_err)
}

// ---------------------------------------------------------------------------
// Evaluation / threshold
// ---------------------------------------------------------------------------

pub fn compact_trigger_threshold(spec: &ModelSpec) -> usize {
    let capacity = usable_window(spec);
    let ratio = compact_ratio_for_window(spec.context_window);
    let buffer = safety_buffer_for_window(spec.context_window);
    let by_ratio = ((capacity as f64) * ratio) as usize;
    let by_buffer = capacity.saturating_sub(buffer);
    by_ratio.min(by_buffer).max(1)
}

/// Evaluate whether the message list has crossed the compaction trigger
/// threshold for the given model spec.
pub fn evaluate(messages: &[StoredMessage], spec: &ModelSpec) -> CompactDecision {
    let used: usize =
        messages.iter().map(|m| estimate_stored_message(&m.role, &m.content, spec)).sum();
    let capacity = usable_window(spec);
    let trigger_at = compact_trigger_threshold(spec);
    CompactDecision { capacity, trigger_at, should_compact: used >= trigger_at }
}

/// Estimate total tokens for the given message list including system prompt
/// and tool definitions, using the model's tokenizer.
pub fn count_projected_tokens(
    messages: &[StoredMessage],
    system_prompt: Option<&str>,
    tools: Option<&Value>,
    spec: &ModelSpec,
) -> usize {
    let mut chat_msgs: Vec<Value> = Vec::new();
    if let Some(sp) = system_prompt {
        if !sp.is_empty() {
            chat_msgs.push(json!({"role": "system", "content": sp}));
        }
    }
    for m in messages {
        chat_msgs.push(json!({"role": m.role, "content": m.content}));
    }
    let msg_tokens = count_chat_messages(&chat_msgs, spec);
    let tool_tokens = tools.map(|t| count_tools_tokens(t, spec)).unwrap_or(0);
    msg_tokens + tool_tokens
}

pub fn resolve_model_spec(settings: &Value) -> ModelSpec {
    let provider = settings.get("provider").and_then(|v| v.as_str()).unwrap_or("OPEN_AI");
    let model = settings.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4o-mini");
    let override_window =
        settings.get("contextWindowOverride").and_then(|v| v.as_u64()).map(|n| n as usize);
    apply_overrides(resolve_spec(provider, model), override_window)
}

// ---------------------------------------------------------------------------
// Tokenizer family cache (per-session)
// ---------------------------------------------------------------------------

/// Tokenizer family cache, keyed by session_id. Once a session has been
/// resolved, subsequent calls force the same TokenizerFamily even if the
/// caller's settings (provider/model) drift mid-session. This prevents
/// context-usage percentage from jumping when the model picker is changed
/// or when settings are partially reloaded, which previously produced the
/// observed 62% -> 60% regression mid-loop.
static SESSION_TOKENIZER_CACHE: OnceLock<Mutex<HashMap<String, TokenizerFamily>>> = OnceLock::new();

pub fn resolve_model_spec_for_session(session_id: &str, settings: &Value) -> ModelSpec {
    let mut spec = resolve_model_spec(settings);
    let cache = SESSION_TOKENIZER_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().expect("tokenizer cache poisoned");
    match map.get(session_id) {
        Some(locked) => {
            spec.tokenizer = *locked;
        },
        None => {
            map.insert(session_id.to_string(), spec.tokenizer);
        },
    }
    spec
}

// ---------------------------------------------------------------------------
// Split-point helpers
// ---------------------------------------------------------------------------

fn assistant_has_tool_calls(message: &StoredMessage) -> bool {
    message.role == "assistant"
        && serde_json::from_str::<Value>(&message.content)
            .ok()
            .and_then(|v| v.get("tool_calls").cloned())
            .and_then(|tc| tc.as_array().map(|a| !a.is_empty()))
            .unwrap_or(false)
}

fn is_safe_boundary(messages: &[StoredMessage], split: usize) -> bool {
    if split == 0 || split > messages.len() {
        return false;
    }
    let curr_role = messages.get(split).map(|m| m.role.as_str()).unwrap_or("");
    let prev = &messages[split - 1];
    curr_role != "tool" && !assistant_has_tool_calls(prev)
}

/// Find a safe split point that preserves the (assistant+tool_calls, tool...)
/// pairing invariant. Walks backward from `proposed_split` while the boundary
/// would orphan a tool message or sever a tool_calls -> tool group.
pub fn safe_split_index(messages: &[StoredMessage], proposed_split: usize) -> usize {
    let mut split = proposed_split.min(messages.len());
    while split > 0 && !is_safe_boundary(messages, split) {
        split -= 1;
    }
    split
}

/// Forward fallback for tool-heavy histories where backward scan collapses
/// to zero. Starting at `proposed_split`, walks forward until finding a
/// boundary that does not split assistant tool_calls from following tool
/// messages.
pub fn safe_split_index_forward(messages: &[StoredMessage], proposed_split: usize) -> usize {
    let start = proposed_split.min(messages.len());
    (start..=messages.len()).find(|split| is_safe_boundary(messages, *split)).unwrap_or(0)
}

/// Compute a target split that keeps the last N user/assistant pairs intact.
pub fn target_split_keeping_pairs(messages: &[StoredMessage], keep_pairs: usize) -> usize {
    let mut pairs_seen = 0usize;
    for (idx, m) in messages.iter().enumerate().rev() {
        if m.role == "user" {
            pairs_seen += 1;
            if pairs_seen >= keep_pairs {
                return idx;
            }
        }
    }
    0
}

// ---------------------------------------------------------------------------
// LLM summarization
// ---------------------------------------------------------------------------

pub const COMPACT_SYSTEM_PROMPT: &str =
    "Summarize this conversation so it can continue without the full history.

Output exactly this Markdown structure, keeping all sections even if empty:

## What We Were Doing
- [The user's goal and current task — one or two sentences]

## What We Found / Did
- [Key results, queries run, data discovered, decisions made]

## Next Steps
- [What to do next, or \"(none)\"]

## Critical Details
- [Exact names to preserve: connections, indexes, fields, query strings, error messages]

Rules:
- Bullets only, no prose.
- Preserve exact identifiers verbatim.
- Do not mention this summary process.";

pub async fn summarize_with_llm(
    messages_to_summarize: &[StoredMessage],
    settings: &Value,
) -> Result<String, String> {
    let chat_msgs: Vec<Value> = messages_to_summarize
        .iter()
        .map(|m| json!({"role": m.role, "content": m.content}))
        .collect();

    let formatter = OpenAIChatFormatter;
    let model = settings.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4o-mini");
    let user_msg_text = serde_json::to_string(&chat_msgs).unwrap_or_default();
    let llm_messages = vec![LlmMessage {
        role:         "user".into(),
        text_content: user_msg_text,
        tool_calls:   None,
        tool_call_id: None,
        thinking:     None,
    }];
    let body =
        formatter.build_request(model, Some(COMPACT_SYSTEM_PROMPT), &llm_messages, None, false);

    let base_url = get_base_url(settings);
    let headers = build_headers(settings)?;
    let http_proxy = settings.get("httpProxy").and_then(|v| v.as_str()).map(|s| s.to_string());
    let proxy_mode = settings.get("proxyMode").and_then(|v| v.as_str()).unwrap_or("system");
    let http_client = create_http_client(proxy_mode, http_proxy, None, None);

    let resp = post_chat_completions_compact(&http_client, &base_url, headers, body).await?;
    let payload: Value = resp.json().await.map_err(|e| e.to_string())?;
    let summary = payload
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if summary.is_empty() {
        return Err("LLM returned empty summary".to_string());
    }
    Ok(summary)
}

// ---------------------------------------------------------------------------
// Boundary payload
// ---------------------------------------------------------------------------

pub fn build_boundary_payload(
    summary: &str,
    pre_tokens: usize,
    post_tokens: usize,
    trigger: &str,
) -> String {
    let compacted_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    json!({
        "_compact_boundary": true,
        "trigger": trigger,
        "summary": summary,
        "pre_tokens": pre_tokens,
        "post_tokens": post_tokens,
        "compacted_at": compacted_at,
    })
    .to_string()
}

// ---------------------------------------------------------------------------
// Insert compact boundary (via SessionStore trait)
// ---------------------------------------------------------------------------

async fn insert_compact_boundary<S: SessionStore>(
    store: &S,
    session_id: &str,
    _removed_ids: &[String],
    boundary_payload: &str,
) -> Result<(), String> {
    store.write_message(&new_id(), session_id, "system", boundary_payload).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Inner compaction logic (generic over SessionStore + EventEmitter)
// ---------------------------------------------------------------------------

/// Auto-compaction split fallback strategy:
/// 1) Try backward-safe split with KEEP_LAST_PAIRS=4.
/// 2) If it collapses to zero, retry with keep_pairs 2, then 1.
/// 3) If still zero, do a forward walk from the keep_pairs=1 proposed split. Emits
///    `agent-loop-compacting` phase start/end around summarize_with_llm.
async fn run_compact_inner<S: SessionStore, E: EventEmitter>(
    session_id: &str,
    settings: &Value,
    store: &S,
    emitter: Option<&E>,
    trigger: &str,
    force: bool,
) -> Result<Option<CompactionInfo>, String> {
    // Manual compaction loads ALL session messages, ignoring existing
    // compaction boundaries, to compact the full conversation.
    // Auto compaction only loads from the last boundary onward.
    let messages = if force {
        store.load_all_messages(session_id).await?
    } else {
        store.load_messages_for_compact(session_id).await?
    };
    let spec = resolve_model_spec_for_session(session_id, settings);
    let decision = evaluate(&messages, &spec);
    if !force && !decision.should_compact {
        return Ok(None);
    }

    let keep_candidates: [usize; 3] = [KEEP_LAST_PAIRS, 2, 1];
    let split_result = keep_candidates.iter().find_map(|keep_pairs| {
        let proposed = target_split_keeping_pairs(&messages, *keep_pairs);
        let split = safe_split_index(&messages, proposed);
        (split > 0).then_some((split, *keep_pairs, proposed))
    });

    let (split, fallback_keep_pairs) = if let Some((split, keep_pairs, _)) = split_result {
        let fallback = (keep_pairs != KEEP_LAST_PAIRS).then_some(keep_pairs);
        (split, fallback)
    } else {
        let fallback_keep = 1usize;
        let fallback_proposed = target_split_keeping_pairs(&messages, fallback_keep);
        let forward_split = safe_split_index_forward(&messages, fallback_proposed);
        if forward_split == 0 {
            let warning_message = "Context compaction failed: history has too many consecutive tool calls — consider clearing the session or asking a more focused question";
            if let Some(emitter) = emitter {
                emitter.emit(
                    "agent-loop-warning",
                    json!({
                        "session_id": session_id,
                        "warning": warning_message,
                    }),
                );
            }
            return Err(warning_message.to_string());
        }
        (forward_split, Some(fallback_keep))
    };

    if split == 0 {
        return Err("compact: cannot find safe split".to_string());
    }

    let to_summarize = &messages[..split];
    let pre_tokens: usize =
        to_summarize.iter().map(|m| estimate_stored_message(&m.role, &m.content, &spec)).sum();
    let post_tokens: usize =
        messages[split..].iter().map(|m| estimate_stored_message(&m.role, &m.content, &spec)).sum();

    if let Some(emitter) = emitter {
        emitter.emit(
            "agent-loop-compacting",
            json!({
                "session_id": session_id,
                "phase": "start",
            }),
        );
    }
    let summary_result = summarize_with_llm(to_summarize, settings).await;
    if let Some(emitter) = emitter {
        emitter.emit(
            "agent-loop-compacting",
            json!({
                "session_id": session_id,
                "phase": "end",
            }),
        );
    }
    let summary = summary_result?;
    let payload = build_boundary_payload(&summary, pre_tokens, post_tokens, trigger);
    let ids_to_remove: Vec<String> = to_summarize.iter().map(|m| m.id.clone()).collect();
    let removed_count = ids_to_remove.len();
    insert_compact_boundary(store, session_id, &ids_to_remove, &payload).await?;

    Ok(Some(CompactionInfo {
        trigger: trigger.to_string(),
        pre_tokens,
        post_tokens,
        removed_count,
        fallback_keep_pairs,
    }))
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Auto-compaction triggered by crossing the threshold.  Emits compacting
/// start/end events around LLM summarization.  Returns `None` when the
/// threshold has not been crossed.
pub async fn run_compact_with_events<S: SessionStore, E: EventEmitter>(
    session_id: &str,
    settings: &Value,
    store: &S,
    emitter: &E,
) -> Result<Option<CompactionInfo>, String> {
    run_compact_inner(session_id, settings, store, Some(emitter), "auto", false).await
}

/// User-forced compaction: bypasses should_compact, tags boundary as "manual",
/// and emits compacting:start/end events.
pub async fn run_compact_manual<S: SessionStore, E: EventEmitter>(
    session_id: &str,
    settings: &Value,
    store: &S,
    emitter: &E,
) -> Result<Option<CompactionInfo>, String> {
    run_compact_inner(session_id, settings, store, Some(emitter), "manual", true).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── safety_buffer_for_window ──

    #[test]
    fn safety_buffer_below_32k() {
        assert_eq!(safety_buffer_for_window(1000), 2_000);
        assert_eq!(safety_buffer_for_window(8_192), 2_000);
        assert_eq!(safety_buffer_for_window(31_999), 2_000);
    }

    #[test]
    fn safety_buffer_32k_to_200k() {
        assert_eq!(safety_buffer_for_window(32_000), 4_000);
        assert_eq!(safety_buffer_for_window(128_000), 4_000);
        assert_eq!(safety_buffer_for_window(199_999), 4_000);
    }

    #[test]
    fn safety_buffer_200k_plus() {
        assert_eq!(safety_buffer_for_window(200_000), 8_000);
        assert_eq!(safety_buffer_for_window(1_000_000), 8_000);
    }

    // ── compact_ratio_for_window ──

    #[test]
    fn compact_ratio_below_100k() {
        assert_eq!(compact_ratio_for_window(1_000), 0.75);
        assert_eq!(compact_ratio_for_window(8_192), 0.75);
        assert_eq!(compact_ratio_for_window(99_999), 0.75);
    }

    #[test]
    fn compact_ratio_100k_plus() {
        assert_eq!(compact_ratio_for_window(100_000), 0.85);
        assert_eq!(compact_ratio_for_window(200_000), 0.85);
    }

    // ── compact_trigger_threshold with ModelSpec ──

    #[test]
    fn trigger_small_window_ollama_default() {
        let spec = ModelSpec {
            model_id:       "llama3".into(),
            context_window: 8_192,
            output_reserve: 2_048,
            tokenizer:      TokenizerFamily::Generic,
        };
        let trigger = compact_trigger_threshold(&spec);
        // capacity = 6144, buffer = 2000, ratio = 0.75
        // by_buffer (4144) wins over by_ratio (4608) → compact at ~4.1K
        assert_eq!(trigger, 4_144, "small window: buffer (2K) should win over ratio");
        assert!(trigger > 2_000, "must not trigger at near-zero tokens");
    }

    #[test]
    fn trigger_large_window_gpt4o() {
        let spec = ModelSpec {
            model_id:       "gpt-4o".into(),
            context_window: 128_000,
            output_reserve: 16_000,
            tokenizer:      TokenizerFamily::OpenAiO200k,
        };
        let trigger = compact_trigger_threshold(&spec);
        assert!(trigger > 80_000, "128K window should have generous trigger");
    }

    #[test]
    fn trigger_huge_window_gpt41() {
        let spec = ModelSpec {
            model_id:       "gpt-4.1".into(),
            context_window: 1_047_576,
            output_reserve: 32_000,
            tokenizer:      TokenizerFamily::OpenAiO200k,
        };
        let trigger = compact_trigger_threshold(&spec);
        let capacity = usable_window(&spec); // ~1,015,576
        let expected_ratio = ((capacity as f64) * 0.85) as usize;
        assert_eq!(trigger, expected_ratio, "huge window: ratio (0.85) should win");
    }
}
