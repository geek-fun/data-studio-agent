use async_trait::async_trait;
use data_studio_agent::{ToolCallPreparation, ToolEnvelope, ToolExecutionError, ToolExecutor};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Implement this boundary in the application with its current policy and source scope.
#[async_trait]
pub trait HostAccess: Send + Sync {
    /// Return `Err` for denied access, or whether the allowed call needs confirmation.
    /// Read current app policy and source scope; do not execute the operation.
    fn inspect(&self, tool: &str, args: &Value, connection: &Value) -> Result<bool, String>;

    /// Resolve credentials and dispatch using the application's execution gate.
    /// If resolution awaits, revalidate authorization and confirmation requirements
    /// at dispatch. Use the typed outcomes before executing if either changed.
    async fn dispatch(
        &self,
        tool: &str,
        args: &Value,
        connection: &Value,
        presented_confirmation: bool,
    ) -> Result<ToolEnvelope, ToolExecutionError>;
}

pub struct HostExecutor<H> {
    pub host: H,
}

#[async_trait]
impl<H: HostAccess> ToolExecutor for HostExecutor<H> {
    fn prepare(
        &self,
        tool: &str,
        args: &Value,
        original: &Value,
        _connections: &HashMap<String, Value>,
    ) -> Result<ToolCallPreparation, String> {
        let requires_confirmation = self.host.inspect(tool, args, original)?;
        Ok(ToolCallPreparation {
            connection_config: json!({
                "connection": original,
                "presented_confirmation": requires_confirmation,
            }),
            risk_level: None,
            confirmation_required: Some(requires_confirmation),
        })
    }

    async fn execute_checked(
        &self,
        tool: &str,
        args: &Value,
        prepared: &Value,
    ) -> Result<ToolEnvelope, ToolExecutionError> {
        let original = prepared
            .get("connection")
            .ok_or_else(|| ToolExecutionError::Denied("Missing host preparation context".into()))?;
        let requires_confirmation =
            self.host.inspect(tool, args, original).map_err(ToolExecutionError::Denied)?;
        let presented = prepared["presented_confirmation"].as_bool().unwrap_or(false);
        if requires_confirmation && !presented {
            return Err(ToolExecutionError::ConfirmationRequired);
        }
        self.host.dispatch(tool, args, original, presented).await
    }

    async fn execute(
        &self,
        tool: &str,
        args: &Value,
        prepared: &Value,
    ) -> Result<ToolEnvelope, String> {
        self.execute_checked(tool, args, prepared).await.map_err(|error| error.to_string())
    }
}

fn main() {}
