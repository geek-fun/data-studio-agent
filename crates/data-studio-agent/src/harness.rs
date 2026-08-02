// ---------------------------------------------------------------------------
// LLM-harness — single-step agent execution, config validation, model listing.
// ---------------------------------------------------------------------------
//
// This module provides non-Tauri, library-level wrappers around the async-openai
// and raw-HTTP (Anthropic) streaming APIs.  Callers supply plain parameters
// instead of Tauri state / window handles, and receive aggregated results.
// Streaming deltas are accumulated and returned in a single JSON `Value`.

use std::time::Duration;

use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionMessageToolCall, ChatCompletionRequestMessage, ChatCompletionTools,
        CreateChatCompletionRequestArgs, FinishReason,
    },
    Client,
};
use futures::StreamExt;
use serde_json::{json, Value};

use crate::{common::http_client::create_http_client, provider_adapter};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Redact the API key from error messages so secrets never leak into logs or
/// the UI.  Only replaces when the key is non-empty and ≥ 8 characters so
/// short placeholder keys don't trigger false positives.
fn sanitize_error(msg: String, api_key: &str) -> String {
    if api_key.is_empty() || api_key.len() < 8 {
        return msg;
    }
    msg.replace(api_key, "[REDACTED]")
}

/// Build a minimal settings Value from provider / base-URL / api-key, suitable
/// for passing to `provider_adapter::get_base_url` and `build_headers`.
fn make_settings(provider: &str, base_url: Option<String>, api_key: &str) -> Value {
    json!({
        "apiCompatibility": provider_adapter::map_to_api_compatibility(provider),
        "baseUrl": base_url,
        "apiKey": api_key,
    })
}

// ---------------------------------------------------------------------------
// Single-step agent execution
// ---------------------------------------------------------------------------

/// Send a single (non-loop) LLM request and collect the streaming response.
///
/// Supports both **OpenAI-compatible** providers (via `async-openai`) and
/// **Anthropic** (via raw HTTP SSE parsing).
///
/// Returns a JSON object with keys:
/// - `"finishReason"` — `"stop"`, `"tool_calls"`, `"length"`, or `"content_filter"`
/// - `"content"` — the full text content accumulated during streaming
/// - `"toolCalls"` — array of `{ id, name, arguments }` objects
pub async fn run_agent_step(
    provider: String,
    model: String,
    messages: Vec<Value>,
    tools: Vec<Value>,
    http_proxy: Option<String>,
    proxy_mode: Option<String>,
    api_key: String,
    base_url: Option<String>,
) -> Result<Value, String> {
    let settings = make_settings(&provider, base_url, &api_key);
    let normalized_base_url = provider_adapter::get_base_url(&settings);

    // -----------------------------------------------------------------------
    // Anthropic streaming path — raw HTTP request with SSE parsing
    // -----------------------------------------------------------------------
    let api_compat = provider_adapter::map_to_api_compatibility(&provider);
    if api_compat == "anthropic" {
        return run_anthropic_stream(
            &model,
            &messages,
            &http_proxy,
            &proxy_mode,
            &api_key,
            &normalized_base_url,
        )
        .await;
    }

    // -----------------------------------------------------------------------
    // OpenAI-compatible path via async-openai
    // -----------------------------------------------------------------------
    run_openai_stream(
        &model,
        messages,
        tools,
        &http_proxy,
        &proxy_mode,
        &api_key,
        &normalized_base_url,
    )
    .await
}

/// Anthropic SSE streaming path (raw HTTP, no SDK dependency).
async fn run_anthropic_stream(
    model: &str,
    messages: &[Value],
    http_proxy: &Option<String>,
    proxy_mode: &Option<String>,
    api_key: &str,
    normalized_base_url: &str,
) -> Result<Value, String> {
    let http_client = create_http_client(
        proxy_mode.as_deref().unwrap_or("system"),
        http_proxy.clone(),
        None,
        None,
    );

    let anthropic_url = format!("{}{}", normalized_base_url.trim_end_matches('/'), "/messages");

    // Build Anthropic request body — filter system messages out of the
    // messages array (Anthropic API rejects role="system") and use the
    // first one as the top-level "system" parameter.
    let system_parts: Vec<&str> = messages
        .iter()
        .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
        .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
        .collect();
    let system_text = system_parts.join("\n");
    let mut filtered_messages = messages.to_vec();
    filtered_messages.retain(|m| m.get("role").and_then(|r| r.as_str()) != Some("system"));

    let mut request_body = json!({
        "model": model,
        "messages": filtered_messages,
        "stream": true,
        "max_tokens": 4096,
    });
    if !system_text.is_empty() {
        request_body["system"] = Value::String(system_text);
    }

    let response = http_client
        .post(&anthropic_url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Anthropic request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Anthropic HTTP {}: {}", status, text));
    }

    // Read streaming response — accumulate content, ignore tool calls for now
    let mut stream = response.bytes_stream();
    let mut buf = String::new();
    let mut full_content = String::new();

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("Stream error: {}", e))?;
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
                    break;
                }

                let v: Value = serde_json::from_str(data).unwrap_or(json!({}));
                if v.get("type").and_then(|t| t.as_str()) == Some("content_block_delta") {
                    if let Some(text) =
                        v.get("delta").and_then(|d| d.get("text")).and_then(|t| t.as_str())
                    {
                        full_content.push_str(text);
                    }
                }
            }
        }
    }

    Ok(json!({
        "finishReason": "stop",
        "content": full_content,
        "toolCalls": []
    }))
}

/// OpenAI-compatible streaming path via the `async-openai` crate.
async fn run_openai_stream(
    model: &str,
    messages: Vec<Value>,
    tools: Vec<Value>,
    http_proxy: &Option<String>,
    proxy_mode: &Option<String>,
    api_key: &str,
    normalized_base_url: &str,
) -> Result<Value, String> {
    let config = OpenAIConfig::new().with_api_key(api_key).with_api_base(normalized_base_url);
    let http_client = create_http_client(
        proxy_mode.as_deref().unwrap_or("system"),
        http_proxy.clone(),
        None,
        None,
    );
    let client = Client::with_config(config).with_http_client(http_client);

    let msgs: Vec<ChatCompletionRequestMessage> = serde_json::from_value(Value::Array(messages))
        .map_err(|e| format!("Failed to parse messages: {}", e))?;

    let tool_defs: Vec<ChatCompletionTools> = serde_json::from_value(Value::Array(tools))
        .map_err(|e| format!("Failed to parse tools: {}", e))?;

    let mut builder = CreateChatCompletionRequestArgs::default();
    builder.model(model).stream(true).messages(msgs);

    if !tool_defs.is_empty() {
        builder.tools(tool_defs);
    }

    let request = builder.build().map_err(|e| e.to_string())?;

    let mut stream = client
        .chat()
        .create_stream(request)
        .await
        .map_err(|e| format!("Failed to create stream: {}", e))?;

    let mut tool_calls: Vec<ChatCompletionMessageToolCall> = Vec::new();
    let mut finish_reason_str = String::from("stop");
    let mut full_content = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                if let Some(choice) = response.choices.first() {
                    if let Some(ref content) = choice.delta.content {
                        full_content.push_str(content);
                    }

                    if let Some(ref chunks) = choice.delta.tool_calls {
                        for chunk in chunks {
                            let index = chunk.index as usize;

                            while tool_calls.len() <= index {
                                tool_calls.push(ChatCompletionMessageToolCall {
                                    id: String::new(),
                                    function: Default::default(),
                                });
                            }

                            let tc = &mut tool_calls[index];
                            if let Some(ref id) = chunk.id {
                                tc.id.clone_from(id);
                            }
                            if let Some(ref func) = chunk.function {
                                if let Some(ref name) = func.name {
                                    tc.function.name.clone_from(name);
                                }
                                if let Some(ref args) = func.arguments {
                                    tc.function.arguments.push_str(args);
                                }
                            }
                        }
                    }

                    if let Some(ref reason) = choice.finish_reason {
                        finish_reason_str = match reason {
                            FinishReason::Stop => "stop".to_string(),
                            FinishReason::ToolCalls => "tool_calls".to_string(),
                            FinishReason::Length => "length".to_string(),
                            FinishReason::ContentFilter => "content_filter".to_string(),
                            _ => format!("{:?}", reason).to_lowercase(),
                        };
                    }
                }
            },
            Err(e) => {
                return Err(sanitize_error(format!("Stream error: {}", e), api_key));
            },
        }
    }

    let tool_calls_json: Vec<Value> = tool_calls
        .iter()
        .map(|tc| {
            json!({
                "id": tc.id,
                "name": tc.function.name,
                "arguments": tc.function.arguments
            })
        })
        .collect();

    Ok(json!({
        "finishReason": finish_reason_str,
        "content": full_content,
        "toolCalls": tool_calls_json
    }))
}

// ---------------------------------------------------------------------------
// LLM configuration validation
// ---------------------------------------------------------------------------

/// Validate that an LLM provider can be reached by querying the models
/// endpoint.  Returns `true` on success; returns an error string explaining
/// the HTTP failure.
pub async fn validate_llm_config(
    provider: String,
    api_key: String,
    model: String,
    http_proxy: Option<String>,
    proxy_mode: Option<String>,
    base_url: Option<String>,
) -> Result<bool, String> {
    let _ = model; // consumed for future provider-specific checks
    let http_client = create_http_client(
        proxy_mode.as_deref().unwrap_or("system"),
        http_proxy,
        None,
        Some(Duration::from_secs(30)),
    );
    let settings = make_settings(&provider, base_url, &api_key);
    let normalized_base_url = provider_adapter::get_base_url(&settings);
    let api_compatibility = provider_adapter::map_to_api_compatibility(&provider);

    // Local providers (Ollama) use native API for validation
    if api_compatibility == "local" {
        let url = provider_adapter::get_native_api_url("OLLAMA", &normalized_base_url, "api/tags");
        let response = http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Validation request failed: {}", e))?;
        let status = response.status();
        if status.is_success() {
            return Ok(true);
        }
        return Err(format!("HTTP {} — verify Ollama is running.", status.as_u16()));
    }

    // All openai-compatible and anthropic providers: validate via /v1/models
    let url = format!("{}/models", normalized_base_url);

    let request = http_client.get(&url).headers(provider_adapter::build_headers(&settings)?);

    let response = request.send().await.map_err(|e| format!("Validation request failed: {}", e))?;

    let status = response.status();
    if status.is_success() {
        Ok(true)
    } else {
        Err(format!("HTTP {} — verify your API key and provider settings.", status.as_u16()))
    }
}

// ---------------------------------------------------------------------------
// Model listing
// ---------------------------------------------------------------------------

/// List the available models from the configured provider.
///
/// Returns a vector of model ID strings.  The parsing logic is provider-aware
/// (handles OpenAI `data[].id`, Ollama `models[].name`, LM Studio `models[].key`).
pub async fn list_llm_models(
    provider: String,
    api_key: String,
    http_proxy: Option<String>,
    proxy_mode: Option<String>,
    base_url: Option<String>,
) -> Result<Vec<String>, String> {
    let http_client = create_http_client(
        proxy_mode.as_deref().unwrap_or("system"),
        http_proxy,
        None,
        Some(Duration::from_secs(60)),
    );
    let settings = make_settings(&provider, base_url, &api_key);
    let normalized_base_url = provider_adapter::get_base_url(&settings);
    let api_compatibility = provider_adapter::map_to_api_compatibility(&provider);

    let (url, requires_auth) = match api_compatibility {
        "local" => (
            provider_adapter::get_native_api_url("OLLAMA", &normalized_base_url, "api/tags"),
            false,
        ),
        _ => (format!("{}/models", normalized_base_url), !api_key.is_empty()),
    };

    let request = if requires_auth {
        http_client.get(&url).headers(provider_adapter::build_headers(&settings)?)
    } else {
        http_client.get(&url)
    };

    let response = request.send().await.map_err(|e| format!("Failed to list models: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to list models: HTTP {}", response.status()));
    }

    let payload: Value =
        response.json().await.map_err(|e| format!("Failed to parse models response: {}", e))?;

    Ok(provider_adapter::extract_model_ids(api_compatibility, &payload))
}
