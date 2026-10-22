use crate::capabilities::types::RiskLevel;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Metadata about a tool execution result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolResultMetadata {
    pub tool_name: String,
    pub duration_ms: u64,
    pub truncated: bool,
}

/// The result envelope from a tool execution — summary for the LLM,
/// full result for storage/UI display.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolEnvelope {
    pub summary: String,
    pub full_result: String,
    pub metadata: ToolResultMetadata,
}

/// Trusted execution context and confirmation metadata supplied by the host application.
///
/// The runtime uses this context only after receiving approval. Preparation metadata
/// describes the requirements presented for approval; it is not proof of approval
/// and must never override a later authorization denial.
#[derive(Debug, Clone)]
pub struct ToolCallPreparation {
    /// Context passed to execution after approval. Hosts may transform the original
    /// connection config and capture the confirmation requirements in this value.
    pub connection_config: Value,
    pub risk_level: Option<RiskLevel>,
    /// Host confirmation requirement; `None` preserves the client's existing rules.
    /// This flag is not an authorization grant or a record of user approval.
    pub confirmation_required: Option<bool>,
}

/// Host execution outcomes that control retry and confirmation handling.
///
/// Converting a `String` with `From` or `?` creates [`Self::Failed`]. Authorization
/// failures must explicitly use [`Self::Denied`] to avoid transport retries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolExecutionError {
    /// Retryable transport/execution failure: at most three failed attempts
    /// (two retries). The runtime also returns this when a retry or renewal limit
    /// is exhausted; that final result is terminal for the tool call.
    Failed(String),
    /// Terminal refusal: no execution retry and no further confirmation request.
    Denied(String),
    /// Pause execution, call `prepare` again with the original resolved config,
    /// emit a new tool-call event, and await approval before another attempt.
    ///
    /// The next prepared config must capture the newly presented confirmation
    /// requirements so the host can recognize them on the next execution check.
    /// Preparation itself does not approve the call. The runtime allows at most
    /// three renewals per tool call, independently of transport retry attempts.
    ConfirmationRequired,
}

impl std::fmt::Display for ToolExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed(message) | Self::Denied(message) => f.write_str(message),
            Self::ConfirmationRequired => f.write_str("Confirmation requirements changed"),
        }
    }
}

impl From<String> for ToolExecutionError {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

/// Application-provided trait for executing tool calls.
/// Each app implements this trait, wiring its own capability registry.
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Inspect a request before confirmation, without executing it.
    ///
    /// `Err` is a terminal denial: the runtime persists it without emitting a
    /// confirmation event or invoking execution. `Ok` supplies the context and
    /// requirements the runtime will present for approval.
    ///
    /// Every call, including confirmation renewals, receives the original resolved
    /// `connection_config`, never a previous preparation's output. Hosts may
    /// transform it without requiring that transformation to be idempotent.
    /// Re-read current policy and capture applicable confirmation requirements in
    /// the returned config, rather than treating preparation as authorization.
    fn prepare(
        &self,
        _tool_name: &str,
        _arguments: &Value,
        connection_config: &Value,
        _connections: &HashMap<String, Value>,
    ) -> Result<ToolCallPreparation, String> {
        Ok(ToolCallPreparation {
            connection_config: connection_config.clone(),
            risk_level: None,
            confirmation_required: None,
        })
    }

    /// Called on every execution attempt, including transport retries and attempts
    /// after renewed approval, with the latest approved preparation's context.
    ///
    /// Hosts must revalidate current authorization and source scope here before
    /// dispatch. Return `Denied` for revoked/forbidden access. If access is allowed
    /// but current confirmation requirements exceed those captured in the prepared
    /// context, return `ConfirmationRequired` without executing the operation.
    /// Once the new context has been approved, do not request the same renewal
    /// again unless the requirements changed again.
    ///
    /// The default delegates to `execute` and maps every string error to retryable
    /// `Failed`. Hosts enforcing permissions should override this method and use
    /// explicit error variants; approval never bypasses authorization.
    async fn execute_checked(
        &self,
        tool_name: &str,
        arguments: &Value,
        connection_config: &Value,
    ) -> Result<ToolEnvelope, ToolExecutionError> {
        self.execute(tool_name, arguments, connection_config).await.map_err(Into::into)
    }

    async fn execute(
        &self,
        tool_name: &str,
        arguments: &Value,
        connection_config: &Value,
    ) -> Result<ToolEnvelope, String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct ExistingExecutor;
    #[async_trait]
    impl ToolExecutor for ExistingExecutor {
        async fn execute(&self, _: &str, _: &Value, _: &Value) -> Result<ToolEnvelope, String> {
            unreachable!("preparation must not execute the request")
        }
    }

    #[test]
    fn existing_executors_keep_their_context_without_implementing_the_new_hook() {
        let config = json!({"resource":"host-owned","permissions":{"arbitrary":"opaque"}});
        let prepared = ExistingExecutor
            .prepare("unregistered-host-tool", &json!({}), &config, &HashMap::new())
            .unwrap();
        assert_eq!(prepared.connection_config, config);
        assert_eq!(prepared.risk_level, None);
    }
}
