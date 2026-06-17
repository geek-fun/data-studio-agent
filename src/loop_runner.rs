// ---------------------------------------------------------------------------
// loop_runner — generic ReAct agent loop
//
// Generic over SessionStore and EventEmitter traits instead of Tauri.
// Connection resolution is handled by the caller (pre-resolved HashMap).
// All persistence goes through the SessionStore trait.
// ---------------------------------------------------------------------------

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;

use crate::chat_formatter::{
    AnthropicChatFormatter, ChatFormatter, LlmMessage, LlmToolCall, OpenAIChatFormatter,
};
use crate::common::http_client::create_http_client;
use crate::compact::{
    count_projected_tokens, evaluate, resolve_model_spec_for_session, run_compact_manual,
};
use crate::conversation::prepare_for_llm;
use crate::loop_runner_support::new_id;
use crate::provider_adapter::{build_headers, get_base_url};
use crate::token_counter::count_chat_messages;
use crate::tool_executor::{ToolEnvelope, ToolExecutor};
use crate::tools::get_tool_required_params;
use crate::traits::{CancelMap, ConfirmMap, EventEmitter, SessionStore, StoredMessage};
use futures::future::{select, Either};
use futures::pin_mut;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use serde_json::{json, Value};
use tokio::sync::oneshot;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_MAX_ITERATIONS: usize = 200;
const DEFAULT_WALL_CLOCK_BUDGET_SECS: u64 = 30 * 60;
const DEFAULT_TOKEN_BUDGET: usize = 20_000_000;
const CONFIRM_TIMEOUT_SECS: u64 = 300;
const RETRY_DELAYS_MS: &[u64] = &[1_000, 3_000, 8_000];
const RETRY_JITTER_MS: u64 = 250;
const RETRYABLE_ERROR_TYPES: &[&str] =
    &["rate_limit_exceeded", "service_unavailable", "overloaded_error"];
const FATAL_ERROR_TYPES: &[&str] =
    &["insufficient_quota", "invalid_request_error", "authentication_error"];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn jittered_sleep_ms(base_ms: u64) {
    let jitter = rand::Rng::gen_range(
        &mut rand::thread_rng(),
        -(RETRY_JITTER_MS as i64)..=RETRY_JITTER_MS as i64,
    );
    let delay = (base_ms as i64 + jitter).max(0) as u64;
    tokio::time::sleep(Duration::from_millis(delay)).await;
}

fn settings_get_str<'a>(settings: &'a Value, key: &str) -> Option<&'a str> {
    settings.get(key).and_then(|v| v.as_str())
}

fn classify_error(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    v.get("error").and_then(|e| e.get("type")).and_then(|t| t.as_str()).map(|s| s.to_string())
}

fn is_retryable(err_type: &str) -> bool {
    RETRYABLE_ERROR_TYPES.contains(&err_type)
}

fn is_fatal(err_type: &str) -> bool {
    FATAL_ERROR_TYPES.contains(&err_type)
}

// ---------------------------------------------------------------------------
// ConfirmGuard — RAII cleanup for confirm_map entries
// ---------------------------------------------------------------------------

struct ConfirmGuard {
    confirm_map: ConfirmMap,
    tool_call_id: String,
}

impl ConfirmGuard {
    fn new(confirm_map: ConfirmMap, tool_call_id: String) -> Self {
        Self { confirm_map, tool_call_id }
    }
}

impl Drop for ConfirmGuard {
    fn drop(&mut self) {
        if let Ok(mut cm) = self.confirm_map.lock() {
            cm.remove(&self.tool_call_id);
        }
    }
}

// ---------------------------------------------------------------------------
// Message Projection Layer
// ---------------------------------------------------------------------------

/// Build the LLM message list from stored DB rows + optional system prompt.
/// Filters, reshapes, and handles orphan tool_call_ids for provider compat.
pub fn build_llm_messages(
    messages: &[(String, String, String)],
    system_prompt: Option<&str>,
) -> Vec<LlmMessage> {
    let mut out: Vec<LlmMessage> = Vec::new();
    if let Some(sys) = system_prompt {
        if !sys.trim().is_empty() {
            out.push(LlmMessage {
                role: "system".into(),
                text_content: sys.to_string(),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            });
        }
    }
    // Track tool_call_ids announced by the most recent assistant message.
    let mut pending_tool_call_ids: HashSet<String> = HashSet::new();

    for (_id, role, content) in messages {
        if role == "assistant" && content.starts_with("LLM HTTP ") {
            continue;
        }
        if role == "tool" {
            if let Ok(v) = serde_json::from_str::<Value>(content) {
                let tool_call_id = v.get("tool_call_id").and_then(|x| x.as_str()).unwrap_or("");
                let inner = v.get("content").and_then(|x| x.as_str()).unwrap_or("");
                if tool_call_id.is_empty() || !pending_tool_call_ids.remove(tool_call_id) {
                    continue;
                }
                out.push(LlmMessage {
                    role: "tool".into(),
                    text_content: inner.to_string(),
                    tool_calls: None,
                    tool_call_id: Some(tool_call_id.to_string()),
                    thinking: None,
                });
            }
        } else if role == "assistant" {
            if !pending_tool_call_ids.is_empty()
                && out.last().map(|m| m.role.as_str()) == Some("assistant")
            {
                out.pop();
            }
            pending_tool_call_ids.clear();
            if let Ok(v) = serde_json::from_str::<Value>(content) {
                if v.is_object() && (v.get("tool_calls").is_some() || v.get("content").is_some()) {
                    let text_content =
                        v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                    let tool_calls = v.get("tool_calls").and_then(|tc| {
                        tc.as_array().map(|arr| {
                            arr.iter()
                                .map(|call| {
                                    let id = call
                                        .get("id")
                                        .and_then(|x| x.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let name = call
                                        .get("function")
                                        .and_then(|f| f.get("name"))
                                        .and_then(|x| x.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let args = call
                                        .get("function")
                                        .and_then(|f| f.get("arguments"))
                                        .and_then(|x| x.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    if !id.is_empty() {
                                        pending_tool_call_ids.insert(id.clone());
                                    }
                                    LlmToolCall { id, name, arguments: args }
                                })
                                .collect()
                        })
                    });
                    let thinking =
                        v.get("thinking").and_then(|t| t.as_str()).map(|s| s.to_string());
                    out.push(LlmMessage {
                        role: "assistant".into(),
                        text_content,
                        tool_calls,
                        tool_call_id: None,
                        thinking,
                    });
                    continue;
                }
            }
            out.push(LlmMessage {
                role: "assistant".into(),
                text_content: content.clone(),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            });
        } else {
            // Non-assistant/non-tool row: drop orphan assistant
            if !pending_tool_call_ids.is_empty()
                && out.last().map(|m| m.role.as_str()) == Some("assistant")
            {
                out.pop();
            }
            pending_tool_call_ids.clear();
            if role == "system" {
                if let Ok(v) = serde_json::from_str::<Value>(content) {
                    if v.get("_compact_boundary").and_then(|x| x.as_bool()).unwrap_or(false) {
                        let summary = v.get("summary").and_then(|x| x.as_str()).unwrap_or_default();
                        out.push(LlmMessage {
                            role: "system".into(),
                            text_content: summary.to_string(),
                            tool_calls: None,
                            tool_call_id: None,
                            thinking: None,
                        });
                        continue;
                    }
                }
            }
            out.push(LlmMessage {
                role: role.clone(),
                text_content: content.clone(),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            });
        }
    }
    if !pending_tool_call_ids.is_empty() && out.last().map(|m| m.role.as_str()) == Some("assistant")
    {
        out.pop();
    }
    out
}

/// Convert LlmMessage list to OpenAI-format JSON values for token counting.
pub fn llm_messages_to_values(messages: &[LlmMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|msg| match msg.role.as_str() {
            "tool" => json!({
                "role": "tool",
                "tool_call_id": msg.tool_call_id,
                "content": msg.text_content
            }),
            "assistant" => {
                let mut m = json!({"role": "assistant", "content": msg.text_content});
                if let Some(ref thinking) = msg.thinking {
                    if !thinking.is_empty() {
                        m["reasoning_content"] = Value::String(thinking.clone());
                    }
                }
                if let Some(ref calls) = msg.tool_calls {
                    let tc: Vec<Value> = calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments
                                }
                            })
                        })
                        .collect();
                    m["tool_calls"] = Value::Array(tc);
                }
                m
            },
            _ => json!({"role": msg.role, "content": msg.text_content}),
        })
        .collect()
}

/// Public wrapper that projects stored messages the same way as
/// build_llm_messages. Used by count_projected_tokens for accurate
/// token estimation that matches what the LLM actually receives.
pub fn project_messages(messages: &[StoredMessage], system_prompt: Option<&str>) -> Vec<Value> {
    let tuples: Vec<(String, String, String)> =
        messages.iter().map(|m| (m.id.clone(), m.role.clone(), m.content.clone())).collect();
    let llm_msgs = build_llm_messages(&tuples, system_prompt);
    llm_messages_to_values(&llm_msgs)
}

// ---------------------------------------------------------------------------
// Streaming
// ---------------------------------------------------------------------------

#[derive(Default)]
struct StreamAccumulator {
    content: String,
    thinking: String,
    tool_calls: Vec<AccTool>,
    finish_reason: String,
}

#[derive(Default, Clone)]
struct AccTool {
    id: String,
    name: String,
    arguments: String,
}

async fn stream_chat<E: EventEmitter>(
    http_client: &reqwest::Client,
    base_url: &str,
    headers: reqwest::header::HeaderMap,
    body: Value,
    session_id: &str,
    emitter: &E,
    formatter: &dyn ChatFormatter,
) -> Result<StreamAccumulator, String> {
    let url = format!("{}{}", base_url.trim_end_matches('/'), formatter.chat_path());
    let mut last_err = String::from("Stream failed");

    for (attempt, &delay) in RETRY_DELAYS_MS.iter().enumerate() {
        let resp = http_client.post(&url).headers(headers.clone()).json(&body).send().await;
        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("LLM request error: {}", e);
                if attempt >= RETRY_DELAYS_MS.len() {
                    return Err(last_err);
                }
                jittered_sleep_ms(delay).await;
                continue;
            },
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let err_type = classify_error(&text).unwrap_or_default();
            let retryable = !is_fatal(&err_type)
                && (status.as_u16() == 429 || status.as_u16() == 503 || is_retryable(&err_type));
            last_err = format!("LLM HTTP {}: {}", status, text);
            if !retryable || attempt >= RETRY_DELAYS_MS.len() {
                return Err(last_err);
            }
            jittered_sleep_ms(delay).await;
            continue;
        }

        let mut acc = StreamAccumulator::default();
        let mut buf = String::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| format!("Stream chunk error: {}", e))?;
            let s = String::from_utf8_lossy(&bytes);
            buf.push_str(&s);

            while let Some(pos) = buf.find("\n\n") {
                let event_block = buf[..pos].to_string();
                buf.drain(..pos + 2);

                for line in event_block.lines() {
                    let line = line.trim();
                    if !line.starts_with("data:") {
                        continue;
                    }
                    let data = line[5..].trim();
                    if data == "[DONE]" {
                        return Ok(acc);
                    }
                    let delta = match formatter.parse_chunk(data) {
                        Ok(d) => d,
                        Err(_) => continue,
                    };
                    if !delta.content_delta.is_empty() {
                        acc.content.push_str(&delta.content_delta);
                        emitter.emit(
                            "agent-loop-delta",
                            json!({
                                "session_id": session_id,
                                "content": delta.content_delta,
                            }),
                        );
                    }
                    if !delta.thinking_delta.is_empty() {
                        acc.thinking.push_str(&delta.thinking_delta);
                        emitter.emit(
                            "agent-loop-thinking-delta",
                            json!({
                                "session_id": session_id,
                                "content": delta.thinking_delta,
                            }),
                        );
                    }
                    for tcd in &delta.tool_call_deltas {
                        let idx = tcd.index;
                        while acc.tool_calls.len() <= idx {
                            acc.tool_calls.push(AccTool::default());
                        }
                        let entry = &mut acc.tool_calls[idx];
                        if !tcd.id.is_empty() {
                            entry.id = tcd.id.clone();
                        }
                        if !tcd.name.is_empty() {
                            entry.name = tcd.name.clone();
                        }
                        if !tcd.arguments_delta.is_empty() {
                            entry.arguments.push_str(&tcd.arguments_delta);
                        }
                    }
                    if let Some(ref reason) = delta.finish_reason {
                        acc.finish_reason = reason.clone();
                    }
                }
            }
        }
        return Ok(acc);
    }

    Err(last_err)
}

// ---------------------------------------------------------------------------
// emit_loop_stopped helper
// ---------------------------------------------------------------------------

fn emit_loop_stopped<E: EventEmitter>(emitter: &E, session_id: &str, reason: &str, message: &str) {
    emitter.emit(
        "agent-loop-stopped",
        json!({
            "session_id": session_id,
            "reason": reason,
            "message": message,
        }),
    );
    emitter.emit("agent-loop-done", json!({"session_id": session_id}));
}

// ---------------------------------------------------------------------------
// Emit context usage helper
// ---------------------------------------------------------------------------

async fn emit_context_usage<S: SessionStore, E: EventEmitter>(
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
    let system_prompt = settings_get_str(settings, "systemPrompt");
    let tools = settings.get("tools");
    let used = count_projected_tokens(&messages, system_prompt, tools, &spec);
    let should_compact = used >= decision.trigger_at;
    emitter.emit(
        "agent-context-usage",
        json!({
            "session_id": session_id,
            "used_tokens": used,
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
// Inline append (no background spawn — store is &S, not &Arc<S>)
// ---------------------------------------------------------------------------

/// Write a message and emit the updated context-usage event.
/// Does NOT spawn a background compaction task (use `conversation::prepare_for_llm`
/// before each LLM call when compaction is needed).
async fn inline_append<S: SessionStore, E: EventEmitter>(
    store: &S,
    emitter: &E,
    settings: &Value,
    id: &str,
    session_id: &str,
    role: &str,
    content: &str,
) -> Result<(), String> {
    store.write_message(id, session_id, role, content).await?;
    emit_context_usage(store, emitter, session_id, settings).await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Main Loop — Public Face
// ---------------------------------------------------------------------------

/// Run the agent loop for a session. Generic over SessionStore + EventEmitter.
/// Connections are expected to be pre-resolved by the caller.
pub async fn run_agent_loop<S: SessionStore, E: EventEmitter>(
    session_id: &str,
    user_message: &str,
    settings: &Value,
    store: &S,
    emitter: &E,
    tool_executor: &dyn ToolExecutor,
    connections: HashMap<String, Value>,
    fallback_connection_config: Value,
    confirm_map: &ConfirmMap,
    cancel_map: &CancelMap,
    is_parallel_ok: &dyn Fn(&str) -> bool,
) -> Result<(), String> {
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    {
        let mut cm = match cancel_map.lock() {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };
        if cm.contains_key(session_id) {
            return Err(format!("session already running: {}", session_id));
        }
        cm.insert(session_id.to_string(), cancel_tx);
    }

    let result = run_agent_loop_inner(
        session_id,
        user_message,
        settings,
        store,
        emitter,
        tool_executor,
        connections,
        fallback_connection_config,
        confirm_map,
        cancel_rx,
        is_parallel_ok,
    )
    .await;

    let _ = store.update_session_status(session_id, "idle").await;

    {
        if let Ok(mut cm) = cancel_map.lock() {
            cm.remove(session_id);
        }
    }

    if let Err(ref e) = result {
        emitter.emit("agent-loop-error", json!({"session_id": session_id, "error": e}));
    }
    result
}

// ---------------------------------------------------------------------------
// PreparedToolCall — collects confirmed tools for phased execution
// ---------------------------------------------------------------------------

struct PreparedToolCall {
    tool_call_id: String,
    #[allow(dead_code)]
    assistant_message_id: String,
    tool_name: String,
    arguments: Value,
    resolved_config: Value,
    parallel_ok: bool,
}

// ---------------------------------------------------------------------------
// Main Loop — Inner
// ---------------------------------------------------------------------------

async fn run_agent_loop_inner<S: SessionStore, E: EventEmitter>(
    session_id: &str,
    user_message: &str,
    settings: &Value,
    store: &S,
    emitter: &E,
    tool_executor: &dyn ToolExecutor,
    connections: HashMap<String, Value>,
    fallback_connection_config: Value,
    confirm_map: &ConfirmMap,
    mut cancel_rx: oneshot::Receiver<()>,
    is_parallel_ok: &dyn Fn(&str) -> bool,
) -> Result<(), String> {
    store.update_session_status(session_id, "running").await?;

    let user_id = new_id();
    inline_append(store, emitter, settings, &user_id, session_id, "user", user_message).await?;

    let system_prompt = settings_get_str(settings, "systemPrompt").map(|s| s.to_string());

    let base_url = get_base_url(settings);
    let headers = build_headers(settings)?;
    let http_proxy = settings_get_str(settings, "httpProxy").map(|s| s.to_string());
    let proxy_mode = settings_get_str(settings, "proxyMode").unwrap_or("system");
    let http_client = create_http_client(proxy_mode, http_proxy, None, None);

    let allowed_tools: HashSet<String> = settings
        .get("tools")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    t.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str())
                })
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let mut recent_tool_signatures: VecDeque<String> = VecDeque::with_capacity(4);

    let max_iterations = settings
        .get("maxIterations")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_MAX_ITERATIONS);
    let wall_clock_budget_secs = settings
        .get("wallClockBudgetMin")
        .and_then(|v| v.as_u64())
        .map(|n| n.saturating_mul(60))
        .unwrap_or(DEFAULT_WALL_CLOCK_BUDGET_SECS);
    let token_budget = settings
        .get("tokenBudget")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_TOKEN_BUDGET);
    let loop_started_at = std::time::Instant::now();

    // Create formatter based on apiCompatibility setting
    let openai_formatter = OpenAIChatFormatter;
    let anthropic_formatter = AnthropicChatFormatter;
    let api_compat = settings_get_str(settings, "apiCompatibility").unwrap_or("openai-compatible");
    let formatter: &dyn ChatFormatter = match api_compat {
        "anthropic" => &anthropic_formatter,
        _ => &openai_formatter,
    };
    let model = settings_get_str(settings, "model").unwrap_or("gpt-4o-mini");

    let mut cumulative_input_tokens: usize = 0;
    let mut iter_count: usize = 0;
    // Tracks consecutive argument-parse failures per tool name.
    let mut consecutive_parse_failures: HashMap<String, usize> = HashMap::new();

    loop {
        if iter_count >= max_iterations {
            emit_loop_stopped(
                emitter,
                session_id,
                "iteration_cap",
                &format!(
                    "Agent paused after {} iterations (configured cap). The task may need more work — reply 'continue' or raise the cap in settings.",
                    iter_count
                ),
            );
            return Ok(());
        }
        let elapsed_secs = loop_started_at.elapsed().as_secs();
        if elapsed_secs >= wall_clock_budget_secs {
            emit_loop_stopped(
                emitter,
                session_id,
                "wall_clock_budget",
                &format!(
                    "Agent paused after {}m wall-clock budget. Reply 'continue' to keep going or raise the budget in settings.",
                    elapsed_secs / 60
                ),
            );
            return Ok(());
        }
        iter_count += 1;
        // Progress heartbeat
        emitter.emit(
            "agent-loop-iteration",
            json!({
                "session_id": session_id,
                "iter_count": iter_count,
                "max_iterations": max_iterations,
            }),
        );

        // Prepare: run compaction if needed (uses store directly)
        prepare_for_llm(store, emitter, settings, session_id).await?;

        let history = store.load_active_history(session_id).await?;
        let chat_msgs = build_llm_messages(&history, system_prompt.as_deref());
        let spec = resolve_model_spec_for_session(session_id, settings);
        let chat_msgs_values = llm_messages_to_values(&chat_msgs);
        cumulative_input_tokens =
            cumulative_input_tokens.saturating_add(count_chat_messages(&chat_msgs_values, &spec));
        if cumulative_input_tokens >= token_budget {
            emit_loop_stopped(
                emitter,
                session_id,
                "token_budget",
                &format!(
                    "Agent paused after consuming {} input tokens (configured budget {}). Reply 'continue' to keep going or raise the budget in settings.",
                    cumulative_input_tokens, token_budget
                ),
            );
            return Ok(());
        }
        let raw_tools = settings.get("tools");
        let body =
            formatter.build_request(model, system_prompt.as_deref(), &chat_msgs, raw_tools, true);
        // Emit waiting-for-LLM event
        emitter.emit(
            "agent-loop-waiting-llm",
            json!({
                "session_id": session_id,
                "iter_count": iter_count,
            }),
        );

        // Stream chat with cancellation support
        let acc = {
            let stream_fut = stream_chat(
                &http_client,
                &base_url,
                headers.clone(),
                body,
                session_id,
                emitter,
                formatter,
            );
            let cancel_fut = &mut cancel_rx;
            pin_mut!(stream_fut);
            pin_mut!(cancel_fut);
            match select(stream_fut, cancel_fut).await {
                Either::Left((result, _)) => match result {
                    Ok(a) => a,
                    Err(e) => {
                        let _ = inline_append(
                            store,
                            emitter,
                            settings,
                            &new_id(),
                            session_id,
                            "assistant",
                            &e,
                        )
                        .await;
                        let err_type = classify_error(&e).unwrap_or_default();
                        if is_fatal(&err_type) || e.starts_with("LLM HTTP 4") {
                            emitter.emit(
                                "agent-loop-error",
                                json!({"session_id": session_id, "error": e}),
                            );
                        } else {
                            emit_loop_stopped(emitter, session_id, "llm_error", &e);
                        }
                        return Ok(());
                    },
                },
                Either::Right((_, _)) => {
                    return Err("cancelled".to_string());
                },
            }
        };

        let assistant_message_id = new_id();

        if acc.tool_calls.is_empty() {
            let payload = if acc.thinking.is_empty() {
                acc.content.clone()
            } else {
                json!({
                    "content": acc.content,
                    "thinking": acc.thinking,
                })
                .to_string()
            };
            inline_append(
                store,
                emitter,
                settings,
                &assistant_message_id,
                session_id,
                "assistant",
                &payload,
            )
            .await?;
            emitter.emit(
                "agent-loop-step-done",
                json!({"session_id": session_id, "message_id": assistant_message_id}),
            );
            emitter.emit("agent-loop-done", json!({"session_id": session_id}));
            return Ok(());
        }

        // Runaway-loop guard: if the LLM emits the same tool-call set 3x, stop.
        let iter_signature: String = {
            let mut sigs: Vec<String> =
                acc.tool_calls.iter().map(|t| format!("{}:{}", t.name, t.arguments)).collect();
            sigs.sort();
            sigs.join("|")
        };
        recent_tool_signatures.push_back(iter_signature.clone());
        if recent_tool_signatures.len() > 3 {
            recent_tool_signatures.pop_front();
        }
        if recent_tool_signatures.len() == 3
            && recent_tool_signatures.iter().all(|s| s == &iter_signature)
        {
            let stuck_msg = "Agent stopped: detected the same tool call repeating across 3 iterations with no progress. Try rephrasing your request or check the tool's previous results.";
            inline_append(
                store,
                emitter,
                settings,
                &assistant_message_id,
                session_id,
                "assistant",
                stuck_msg,
            )
            .await?;
            emitter.emit("agent-loop-error", json!({"session_id": session_id, "error": stuck_msg}));
            return Ok(());
        }

        let resolved_tool_ids: Vec<String> = acc
            .tool_calls
            .iter()
            .map(|t| if t.id.is_empty() { new_id() } else { t.id.clone() })
            .collect();

        let tool_calls_json: Vec<Value> = acc
            .tool_calls
            .iter()
            .zip(resolved_tool_ids.iter())
            .map(|(t, resolved_id)| {
                json!({
                    "id": resolved_id,
                    "type": "function",
                    "function": {"name": t.name, "arguments": t.arguments}
                })
            })
            .collect();
        let assistant_payload = json!({
            "content": if acc.content.is_empty() { Value::Null } else { Value::String(acc.content.clone()) },
            "thinking": if acc.thinking.is_empty() { Value::Null } else { Value::String(acc.thinking.clone()) },
            "tool_calls": tool_calls_json,
        });
        inline_append(
            store,
            emitter,
            settings,
            &assistant_message_id,
            session_id,
            "assistant",
            &assistant_payload.to_string(),
        )
        .await?;
        emitter.emit(
            "agent-loop-step-done",
            json!({"session_id": session_id, "message_id": assistant_message_id}),
        );

        let mut prepared: Vec<PreparedToolCall> = Vec::new();
        for (tc, tool_call_id) in acc.tool_calls.iter().zip(resolved_tool_ids.iter()) {
            let tool_call_id: String = tool_call_id.clone();
            let tool_name: String = tc.name.clone();
            let arguments_value: Value = match serde_json::from_str(&tc.arguments) {
                Ok(v) => {
                    consecutive_parse_failures.remove(&tool_name);
                    v
                },
                Err(e) => {
                    store
                        .insert_tool_call(
                            &tool_call_id,
                            &assistant_message_id,
                            session_id,
                            &tool_name,
                            &tc.arguments,
                            "failed",
                        )
                        .await?;
                    let required = get_tool_required_params(&tool_name)
                        .map(|p| format!(" Required parameters: {}.", p))
                        .unwrap_or_default();
                    let err_msg = json!({
                        "tool_call_id": tool_call_id,
                        "name": tool_name,
                        "content": format!("Invalid arguments for '{}': {}.{}", tool_name, e, required),
                    });
                    inline_append(
                        store,
                        emitter,
                        settings,
                        &new_id(),
                        session_id,
                        "tool",
                        &err_msg.to_string(),
                    )
                    .await?;

                    let fail_count =
                        consecutive_parse_failures.entry(tool_name.clone()).or_insert(0usize);
                    *fail_count += 1;
                    if *fail_count >= 2 {
                        let param_hint = get_tool_required_params(&tool_name).unwrap_or_default();
                        let stuck_msg = if param_hint.is_empty() {
                            format!(
                                "Model keeps generating invalid tool calls for '{}'. \
                                 Try rephrasing your request or switch to a model with better \
                                 function-calling support.",
                                tool_name,
                            )
                        } else {
                            format!(
                                "Model keeps generating invalid tool calls for '{}'. \
                                 The tool requires valid JSON arguments with fields: {}. \
                                 Try rephrasing your request to include all required parameters, \
                                 or switch to a model with better function-calling support.",
                                tool_name, param_hint,
                            )
                        };
                        inline_append(
                            store,
                            emitter,
                            settings,
                            &new_id(),
                            session_id,
                            "assistant",
                            &stuck_msg,
                        )
                        .await?;
                        emitter.emit(
                            "agent-loop-error",
                            json!({"session_id": session_id, "error": stuck_msg}),
                        );
                        return Ok(());
                    }
                    continue;
                },
            };

            store
                .insert_tool_call(
                    &tool_call_id,
                    &assistant_message_id,
                    session_id,
                    &tool_name,
                    &tc.arguments,
                    "pending",
                )
                .await?;

            if allowed_tools.is_empty() || !allowed_tools.contains(&tc.name) {
                store.update_tool_call_status(&tool_call_id, "failed").await?;
                let err_content = format!("Tool '{}' is not allowed in this session.", tool_name);
                let deny_msg = json!({
                    "tool_call_id": tool_call_id,
                    "name": tool_name,
                    "content": err_content,
                });
                inline_append(
                    store,
                    emitter,
                    settings,
                    &new_id(),
                    session_id,
                    "tool",
                    &deny_msg.to_string(),
                )
                .await?;
                emitter.emit(
                    "agent-loop-tool-result",
                    json!({
                        "session_id": session_id,
                        "tool_call_id": tool_call_id,
                        "error": true,
                        "envelope": { "summary": err_content },
                    }),
                );
                continue;
            }

            // Resolve connection config from the pre-resolved connections map
            let resolved_config = match arguments_value
                .get("connection_id")
                .and_then(|v| v.as_str())
            {
                Some(conn_id) => match connections.get(conn_id) {
                    Some(cfg) => cfg.clone(),
                    None => {
                        store.update_tool_call_status(&tool_call_id, "failed").await?;
                        let err_content = format!(
                            "Unknown connection_id '{}' for tool '{}'.",
                            conn_id, tool_name
                        );
                        let err_msg = json!({
                            "tool_call_id": tool_call_id,
                            "name": tool_name,
                            "content": err_content,
                        });
                        inline_append(
                            store,
                            emitter,
                            settings,
                            &new_id(),
                            session_id,
                            "tool",
                            &err_msg.to_string(),
                        )
                        .await?;
                        emitter.emit(
                            "agent-loop-tool-result",
                            json!({
                                "session_id": session_id,
                                "tool_call_id": tool_call_id,
                                "error": true,
                                "envelope": { "summary": err_content },
                            }),
                        );
                        continue;
                    },
                },
                None => {
                    if !connections.is_empty() {
                        store.update_tool_call_status(&tool_call_id, "failed").await?;
                        let err_content = format!(
                            "Tool '{}' requires a connection_id argument. Available connections: {}.",
                            tool_name,
                            connections.keys().cloned().collect::<Vec<_>>().join(", ")
                        );
                        let err_msg = json!({
                            "tool_call_id": tool_call_id,
                            "name": tool_name,
                            "content": err_content,
                        });
                        inline_append(
                            store,
                            emitter,
                            settings,
                            &new_id(),
                            session_id,
                            "tool",
                            &err_msg.to_string(),
                        )
                        .await?;
                        emitter.emit(
                            "agent-loop-tool-result",
                            json!({
                                "session_id": session_id,
                                "tool_call_id": tool_call_id,
                                "error": true,
                                "envelope": { "summary": err_content },
                            }),
                        );
                        continue;
                    }
                    fallback_connection_config.clone()
                },
            };

            // Frontend confirmation gate
            let (confirm_tx, confirm_rx) = oneshot::channel::<bool>();
            {
                let mut cm = match confirm_map.lock() {
                    Ok(c) => c,
                    Err(e) => return Err(e.to_string()),
                };
                cm.insert(tool_call_id.clone(), confirm_tx);
            }
            let _guard = ConfirmGuard::new(confirm_map.clone(), tool_call_id.clone());

            emitter.emit(
                "agent-loop-tool-call",
                json!({
                    "session_id": session_id,
                    "tool_call_id": tool_call_id,
                    "tool_name": tool_name,
                    "arguments": arguments_value,
                }),
            );

            let confirm_future =
                tokio::time::timeout(Duration::from_secs(CONFIRM_TIMEOUT_SECS), confirm_rx);

            let allowed = {
                let cf = confirm_future;
                let cancel_fut = &mut cancel_rx;
                pin_mut!(cf);
                pin_mut!(cancel_fut);
                match select(cf, cancel_fut).await {
                    Either::Left((result, _)) => match result {
                        Ok(Ok(v)) => v,
                        Ok(Err(_)) => false,
                        Err(_) => {
                            return Err(format!("tool confirmation timeout: {}", tool_call_id));
                        },
                    },
                    Either::Right((_, _)) => {
                        return Err("cancelled".to_string());
                    },
                }
            };

            if !allowed {
                store.update_tool_call_status(&tool_call_id, "denied").await?;
                let tool_deny_msg = json!({
                    "tool_call_id": tool_call_id,
                    "name": tool_name,
                    "content": format!("Tool call '{}' was denied by the user. Try an alternative approach.", tool_name),
                });
                inline_append(
                    store,
                    emitter,
                    settings,
                    &new_id(),
                    session_id,
                    "tool",
                    &tool_deny_msg.to_string(),
                )
                .await?;
                continue;
            }

            store.update_tool_call_status(&tool_call_id, "approved").await?;

            let parallel_ok = is_parallel_ok(&tool_name);
            prepared.push(PreparedToolCall {
                tool_call_id: tool_call_id.clone(),
                assistant_message_id: assistant_message_id.clone(),
                tool_name: tool_name.clone(),
                arguments: arguments_value.clone(),
                resolved_config,
                parallel_ok,
            });
        }

        if prepared.is_empty() {
            return Ok(());
        }
        execute_phase2_3(
            session_id,
            &assistant_message_id,
            prepared,
            store,
            emitter,
            tool_executor,
            &mut cancel_rx,
        )
        .await?;
    }
}

// ---------------------------------------------------------------------------
// Phase 2+3: Parallel execution + result processing
// ---------------------------------------------------------------------------

struct ToolExecutionResult {
    index: usize,
    tool_call_id: String,
    tool_name: String,
    result: Result<ToolEnvelope, String>,
    cancelled: bool,
}

async fn execute_phase2_3<S: SessionStore, E: EventEmitter>(
    session_id: &str,
    _assistant_message_id: &str,
    prepared: Vec<PreparedToolCall>,
    store: &S,
    emitter: &E,
    tool_executor: &dyn ToolExecutor,
    cancel_rx: &mut oneshot::Receiver<()>,
) -> Result<(), String> {
    // --- Phase 2: Partition into execution groups & execute ---
    let mut all_results: Vec<ToolExecutionResult> = Vec::new();
    let mut global_cancelled = false;
    let mut pos = 0;

    while pos < prepared.len() {
        let group_start = pos;
        let is_parallel = prepared[pos].parallel_ok;
        while pos < prepared.len() && prepared[pos].parallel_ok == is_parallel {
            pos += 1;
        }
        let group_end = pos;

        if is_parallel && (group_end - group_start) > 1 {
            // Parallel batch via FuturesUnordered
            let mut futs = FuturesUnordered::new();
            #[allow(clippy::needless_range_loop)]
            for j in group_start..group_end {
                let tool = &prepared[j];
                let index = j;
                let tool_call_id = tool.tool_call_id.clone();
                let tool_name = tool.tool_name.clone();
                let arguments = tool.arguments.clone();
                let resolved_config = tool.resolved_config.clone();

                futs.push(async move {
                    let result =
                        tool_executor.execute(&tool_name, &arguments, &resolved_config).await;
                    ToolExecutionResult { index, tool_call_id, tool_name, result, cancelled: false }
                });
            }

            let mut batch_completed: Vec<ToolExecutionResult> = Vec::new();
            loop {
                tokio::select! {
                    biased;
                    _ = &mut *cancel_rx => {
                        global_cancelled = true;
                        break;
                    }
                    result = futs.next() => {
                        match result {
                            Some(r) => batch_completed.push(r),
                            None => break,
                        }
                    }
                }
            }

            let completed_indices: HashSet<usize> =
                batch_completed.iter().map(|r| r.index).collect();
            #[allow(clippy::needless_range_loop)]
            for j in group_start..group_end {
                if !completed_indices.contains(&j) {
                    let tool = &prepared[j];
                    all_results.push(ToolExecutionResult {
                        index: j,
                        tool_call_id: tool.tool_call_id.clone(),
                        tool_name: tool.tool_name.clone(),
                        result: Err("cancelled".to_string()),
                        cancelled: true,
                    });
                }
            }
            all_results.extend(batch_completed);
        } else {
            // Sequential: single tool
            let tool = &prepared[group_start];
            let index = group_start;
            let tool_call_id = tool.tool_call_id.clone();
            let tool_name = tool.tool_name.clone();

            let tr = tokio::select! {
                biased;
                _ = &mut *cancel_rx => {
                    ToolExecutionResult {
                        index,
                        tool_call_id,
                        tool_name,
                        result: Err("cancelled".to_string()),
                        cancelled: true,
                    }
                }
                r = tool_executor.execute(&tool.tool_name, &tool.arguments, &tool.resolved_config) => {
                    match r {
                        Ok(envelope) => ToolExecutionResult {
                            index,
                            tool_call_id,
                            tool_name,
                            result: Ok(envelope),
                            cancelled: false,
                        },
                        Err(e) => ToolExecutionResult {
                            index,
                            tool_call_id,
                            tool_name,
                            result: Err(e),
                            cancelled: false,
                        },
                    }
                }
            };

            if tr.cancelled {
                global_cancelled = true;
            }
            all_results.push(tr);
        }

        if global_cancelled {
            break;
        }
    }

    // Mark any remaining unprocessed tools as cancelled
    if global_cancelled {
        let processed: HashSet<usize> = all_results.iter().map(|r| r.index).collect();
        for (j, tool) in prepared.iter().enumerate() {
            if !processed.contains(&j) {
                all_results.push(ToolExecutionResult {
                    index: j,
                    tool_call_id: tool.tool_call_id.clone(),
                    tool_name: tool.tool_name.clone(),
                    result: Err("cancelled".to_string()),
                    cancelled: true,
                });
            }
        }
    }

    // --- Phase 3: Process results in original order ---
    all_results.sort_by_key(|r| r.index);

    for result in &all_results {
        if result.cancelled {
            store.update_tool_call_status(&result.tool_call_id, "failed").await?;
            let cancel_msg = json!({
                "tool_call_id": result.tool_call_id,
                "name": result.tool_name,
                "content": "Tool call was cancelled.",
            });
            store.write_message(&new_id(), session_id, "tool", &cancel_msg.to_string()).await?;
        } else {
            match &result.result {
                Ok(envelope) => {
                    store.insert_tool_result(&result.tool_call_id, &envelope.full_result).await?;
                    store.update_tool_call_status(&result.tool_call_id, "completed").await?;
                    emitter.emit(
                        "agent-loop-tool-result",
                        json!({
                            "session_id": session_id,
                            "tool_call_id": result.tool_call_id,
                            "envelope": envelope,
                        }),
                    );
                    let tool_msg = json!({
                        "tool_call_id": result.tool_call_id,
                        "name": result.tool_name,
                        "content": envelope.summary,
                    });
                    store
                        .write_message(&new_id(), session_id, "tool", &tool_msg.to_string())
                        .await?;
                },
                Err(e) => {
                    store.update_tool_call_status(&result.tool_call_id, "failed").await?;
                    emitter.emit(
                        "agent-loop-tool-result",
                        json!({
                            "session_id": session_id,
                            "tool_call_id": result.tool_call_id,
                            "error": true,
                            "envelope": { "summary": e },
                        }),
                    );
                    let err_msg = json!({
                        "tool_call_id": result.tool_call_id,
                        "name": result.tool_name,
                        "content": e,
                    });
                    store
                        .write_message(&new_id(), session_id, "tool", &err_msg.to_string())
                        .await?;
                },
            }
        }
    }

    if global_cancelled {
        Err("cancelled".to_string())
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Cancel helper
// ---------------------------------------------------------------------------

/// Cancel a running agent loop by sending the cancellation signal.
pub fn cancel_agent_loop(session_id: &str, cancel_map: &CancelMap) -> Result<(), String> {
    let tx_opt = {
        let mut cm = match cancel_map.lock() {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };
        cm.remove(session_id)
    };
    if let Some(tx) = tx_opt {
        let _ = tx.send(());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Confirm helper
// ---------------------------------------------------------------------------

/// Confirm or deny a pending tool call.
pub fn confirm_tool_call(
    tool_call_id: &str,
    allowed: bool,
    confirm_map: &ConfirmMap,
) -> Result<(), String> {
    let tx_opt = {
        let mut cm = match confirm_map.lock() {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };
        cm.remove(tool_call_id)
    };
    if let Some(tx) = tx_opt {
        let _ = tx.send(allowed);
        Ok(())
    } else {
        Err(format!("no pending confirmation for {}", tool_call_id))
    }
}

// ---------------------------------------------------------------------------
// Get tool full result
// ---------------------------------------------------------------------------

/// Retrieve the full result for a completed tool call.
/// Apps that need this should implement a custom query in their SessionStore.
pub async fn get_tool_full_result<S: SessionStore>(
    _tool_call_id: &str,
    _store: &S,
) -> Result<String, String> {
    Err("get_tool_full_result: not implemented in generic SessionStore; \
         implement a custom query in your app"
        .to_string())
}

// ---------------------------------------------------------------------------
// Manual compaction
// ---------------------------------------------------------------------------

/// Manually trigger compaction for a session. Returns context usage info.
pub async fn compact_agent_session<S: SessionStore, E: EventEmitter>(
    session_id: &str,
    settings: &Value,
    store: &S,
    emitter: &E,
) -> Result<Value, String> {
    let lock = store.compact_lock(session_id);
    let _guard = lock.lock().await;
    let outcome = run_compact_manual(session_id, settings, store, emitter).await?;
    if let Some(info) = outcome {
        emitter.emit(
            "agent-loop-summary-injected",
            json!({
                "session_id": session_id,
                "trigger": info.trigger,
                "pre_tokens": info.pre_tokens,
                "post_tokens": info.post_tokens,
                "removed_count": info.removed_count,
                "fallback_keep_pairs": info.fallback_keep_pairs,
            }),
        );
    }
    emit_context_usage(store, emitter, session_id, settings).await;
    let messages = store.load_messages_for_compact(session_id).await?;
    let spec = resolve_model_spec_for_session(session_id, settings);
    let decision = evaluate(&messages, &spec);
    let system_prompt = settings_get_str(settings, "systemPrompt");
    let tools = settings.get("tools");
    let used = count_projected_tokens(&messages, system_prompt, tools, &spec);
    let should_compact = used >= decision.trigger_at;
    Ok(json!({
        "session_id": session_id,
        "used_tokens": used,
        "capacity": decision.capacity,
        "context_window": spec.context_window,
        "output_reserve": spec.output_reserve,
        "trigger_at": decision.trigger_at,
        "should_compact": should_compact,
        "model": spec.model_id,
    }))
}

// ---------------------------------------------------------------------------
// Context usage query
// ---------------------------------------------------------------------------

/// Get the current context usage for a session without running compaction.
pub async fn get_agent_context_usage<S: SessionStore>(
    session_id: &str,
    settings: &Value,
    store: &S,
) -> Result<Value, String> {
    let messages = store.load_messages_for_compact(session_id).await?;
    let spec = resolve_model_spec_for_session(session_id, settings);
    let decision = evaluate(&messages, &spec);
    let system_prompt = settings_get_str(settings, "systemPrompt");
    let tools = settings.get("tools");
    let used = count_projected_tokens(&messages, system_prompt, tools, &spec);
    let should_compact = used >= decision.trigger_at;
    Ok(json!({
        "session_id": session_id,
        "used_tokens": used,
        "capacity": decision.capacity,
        "context_window": spec.context_window,
        "output_reserve": spec.output_reserve,
        "trigger_at": decision.trigger_at,
        "should_compact": should_compact,
        "model": spec.model_id,
    }))
}
