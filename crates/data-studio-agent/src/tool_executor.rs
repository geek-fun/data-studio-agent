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
#[derive(Debug, Clone)]
pub struct ToolCallPreparation {
    pub connection_config: Value,
    pub risk_level: Option<RiskLevel>,
    pub confirmation_required: Option<bool>,
}

/// Execution outcomes that must not be treated as transient transport failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolExecutionError {
    Failed(String),
    Denied(String),
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
    /// Called before confirmation. Hosts can reject the request or attach trusted context.
    /// Execution must revalidate access if authorization can change while awaiting confirmation.
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

    /// Hosts enforce current authorization here on every attempt. Older executors
    /// retain their existing string-error behavior through the default adapter.
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
