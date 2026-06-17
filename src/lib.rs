pub mod capabilities;
pub mod chat_formatter;
pub mod common;
pub mod compact;
pub mod conversation;
pub mod harness;
pub mod loop_runner;
pub mod loop_runner_support;
pub mod model_registry;
pub mod provider_adapter;
pub mod token_counter;
pub mod tool_executor;
pub mod tools;
pub mod traits;

#[cfg(feature = "sqlite-storage")]
pub mod storage;

// Re-exports for convenience
pub use chat_formatter::ChatFormatter;
pub use provider_adapter::{build_headers, get_base_url, map_to_api_compatibility};
pub use tool_executor::{ToolEnvelope, ToolExecutor, ToolResultMetadata};
