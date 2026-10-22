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
        Ok(ToolCallPreparation { connection_config: connection_config.clone(), risk_level: None })
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
