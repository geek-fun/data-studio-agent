use async_trait::async_trait;
use serde_json::Value;

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

/// Application-provided trait for executing tool calls.
/// Each app implements this trait, wiring its own capability registry.
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(
        &self,
        tool_name: &str,
        arguments: &Value,
        connection_config: &Value,
    ) -> Result<ToolEnvelope, String>;
}
