use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::types::RiskLevel;

/// Permission modes for MCP bridge access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpPermissionMode {
    /// Read-only queries and metadata — only Safe capabilities
    ReadOnly,
    /// Read + write (insert, update, create) — Safe + Elevated
    DataReadWrite,
    /// Everything including destructive ops — Safe + Elevated + Destructive
    FullAccess,
}

impl McpPermissionMode {
    pub fn allows(&self, risk_level: RiskLevel) -> bool {
        match self {
            McpPermissionMode::ReadOnly => matches!(risk_level, RiskLevel::Safe),
            McpPermissionMode::DataReadWrite => {
                matches!(risk_level, RiskLevel::Safe | RiskLevel::Elevated)
            },
            McpPermissionMode::FullAccess => true,
        }
    }
}

/// Per-connection override that takes precedence over the global mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionMcpOverride {
    /// Read-only for MCP: blocks Elevated/Destructive regardless of global mode
    pub read_only: bool,
}

/// MCP permission policy, stored per app and read by the bridge on every request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpPolicy {
    pub mode: McpPermissionMode,
    /// Empty = all connections allowed; non-empty = only listed connection IDs
    pub allowed_connection_ids: Vec<String>,
    /// Connection-ID-keyed overrides that supersede the global mode
    pub connection_overrides: HashMap<String, ConnectionMcpOverride>,
}

impl Default for McpPolicy {
    fn default() -> Self {
        Self {
            mode: McpPermissionMode::ReadOnly,
            allowed_connection_ids: Vec::new(),
            connection_overrides: HashMap::new(),
        }
    }
}

impl McpPolicy {
    pub fn is_connection_allowed(&self, connection_id: &str) -> bool {
        self.allowed_connection_ids.is_empty()
            || self.allowed_connection_ids.iter().any(|id| id == connection_id)
    }

    pub fn is_connection_read_only(&self, connection_id: &str) -> bool {
        self.connection_overrides.get(connection_id).is_some_and(|o| o.read_only)
    }

    /// Whether a capability may run for the given connection.
    pub fn allows(&self, risk_level: RiskLevel, connection_id: Option<&str>) -> bool {
        if let Some(id) = connection_id {
            if !self.is_connection_allowed(id) {
                return false;
            }
            if self.is_connection_read_only(id) && !matches!(risk_level, RiskLevel::Safe) {
                return false;
            }
        }
        self.mode.allows(risk_level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mode_allows_risk_levels() {
        assert!(McpPermissionMode::ReadOnly.allows(RiskLevel::Safe));
        assert!(!McpPermissionMode::ReadOnly.allows(RiskLevel::Elevated));
        assert!(!McpPermissionMode::ReadOnly.allows(RiskLevel::Destructive));

        assert!(McpPermissionMode::DataReadWrite.allows(RiskLevel::Safe));
        assert!(McpPermissionMode::DataReadWrite.allows(RiskLevel::Elevated));
        assert!(!McpPermissionMode::DataReadWrite.allows(RiskLevel::Destructive));

        assert!(McpPermissionMode::FullAccess.allows(RiskLevel::Safe));
        assert!(McpPermissionMode::FullAccess.allows(RiskLevel::Elevated));
        assert!(McpPermissionMode::FullAccess.allows(RiskLevel::Destructive));
    }

    #[test]
    fn test_default_policy_is_read_only_with_all_connections() {
        let policy = McpPolicy::default();
        assert_eq!(policy.mode, McpPermissionMode::ReadOnly);
        assert!(policy.allowed_connection_ids.is_empty());
        assert!(policy.is_connection_allowed("any-id"));
        assert!(!policy.is_connection_read_only("any-id"));
    }

    #[test]
    fn test_allowlist_restricts_connections() {
        let policy =
            McpPolicy { allowed_connection_ids: vec!["conn-1".into()], ..McpPolicy::default() };
        assert!(policy.is_connection_allowed("conn-1"));
        assert!(!policy.is_connection_allowed("conn-2"));
        assert!(!policy.allows(RiskLevel::Safe, Some("conn-2")));
    }

    #[test]
    fn test_read_only_override_blocks_elevated_but_allows_safe() {
        let policy = McpPolicy {
            mode: McpPermissionMode::FullAccess,
            connection_overrides: HashMap::from([(
                "prod-es".into(),
                ConnectionMcpOverride { read_only: true },
            )]),
            ..McpPolicy::default()
        };
        assert!(policy.allows(RiskLevel::Destructive, Some("staging-es")));
        assert!(!policy.allows(RiskLevel::Destructive, Some("prod-es")));
        assert!(!policy.allows(RiskLevel::Elevated, Some("prod-es")));
        assert!(policy.allows(RiskLevel::Safe, Some("prod-es")));
    }

    #[test]
    fn test_serde_roundtrip() {
        let policy = McpPolicy {
            mode: McpPermissionMode::DataReadWrite,
            allowed_connection_ids: vec!["conn-1".into()],
            connection_overrides: HashMap::from([(
                "conn-1".into(),
                ConnectionMcpOverride { read_only: true },
            )]),
        };
        let v = serde_json::to_value(&policy).unwrap();
        assert_eq!(v["mode"], "DataReadWrite");
        assert_eq!(v["allowed_connection_ids"][0], "conn-1");
        assert_eq!(v["connection_overrides"]["conn-1"]["read_only"], true);

        let back: McpPolicy = serde_json::from_value(v).unwrap();
        assert_eq!(back, policy);
    }

    #[test]
    fn test_mode_serde_names() {
        assert_eq!(serde_json::to_value(McpPermissionMode::ReadOnly).unwrap(), json!("ReadOnly"));
        assert_eq!(
            serde_json::to_value(McpPermissionMode::DataReadWrite).unwrap(),
            json!("DataReadWrite")
        );
        assert_eq!(
            serde_json::to_value(McpPermissionMode::FullAccess).unwrap(),
            json!("FullAccess")
        );
    }
}
