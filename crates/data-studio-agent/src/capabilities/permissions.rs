use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::types::RiskLevel;

/// Result of a policy decision for a capability invocation.
///
/// `Ask` is informational this iteration: both the MCP bridge and the built-in
/// agent loop execute on `Ask` (confirmation UX is client-driven). Only `Deny`
/// short-circuits execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicyAction {
    Allow,
    Ask,
    Deny,
}

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
    /// Optional action-level allowlist for this connection.
    /// When set, only capabilities whose risk maps to a listed action pass;
    /// `read_only` remains the coarse legacy gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_actions: Option<Vec<McpAction>>,
}

/// Action categories a connection-level override can allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpAction {
    /// Safe capabilities (queries, metadata)
    Read,
    /// Elevated capabilities (insert, update, create)
    Write,
    /// Destructive capabilities (delete, drop, truncate)
    Delete,
}

impl McpAction {
    pub fn from_risk(risk: RiskLevel) -> McpAction {
        match risk {
            RiskLevel::Safe => McpAction::Read,
            RiskLevel::Elevated => McpAction::Write,
            RiskLevel::Destructive => McpAction::Delete,
        }
    }
}

/// MCP permission policy, stored per app and read by the bridge on every request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpPolicy {
    pub mode: McpPermissionMode,
    /// Empty = all connections allowed; non-empty = only listed connection IDs
    pub allowed_connection_ids: Vec<String>,
    /// Connection-ID-keyed overrides that supersede the global mode
    pub connection_overrides: HashMap<String, ConnectionMcpOverride>,
    /// When true, Destructive capabilities are exposed and gated by mode
    /// (only FullAccess allows them). When false, Destructive is always
    /// hidden/rejected regardless of mode.
    #[serde(default = "default_confirm_destructive")]
    pub confirm_destructive: bool,
}

fn default_confirm_destructive() -> bool {
    true
}

impl Default for McpPolicy {
    fn default() -> Self {
        Self {
            mode: McpPermissionMode::DataReadWrite,
            allowed_connection_ids: Vec::new(),
            connection_overrides: HashMap::new(),
            confirm_destructive: true,
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

    /// Whether the connection override allows the risk level's action.
    /// Falls back to `read_only` when no action allowlist is set.
    fn connection_allows_action(&self, connection_id: &str, risk: RiskLevel) -> bool {
        let Some(ov) = self.connection_overrides.get(connection_id) else {
            return true;
        };
        match &ov.allowed_actions {
            Some(actions) => actions.contains(&McpAction::from_risk(risk)),
            None => !(ov.read_only && !matches!(risk, RiskLevel::Safe)),
        }
    }

    /// Whether a capability may run for the given connection.
    /// Compat: `allows()` == `decide() != Deny` (Ask counts as allowed).
    pub fn allows(&self, risk_level: RiskLevel, connection_id: Option<&str>) -> bool {
        !matches!(self.decide(risk_level, connection_id), PolicyAction::Deny)
    }

    /// Decide the policy action for a capability and connection.
    pub fn decide(&self, risk_level: RiskLevel, connection_id: Option<&str>) -> PolicyAction {
        if let Some(id) = connection_id {
            if !self.is_connection_allowed(id) {
                return PolicyAction::Deny;
            }
            if !self.connection_allows_action(id, risk_level) {
                return PolicyAction::Deny;
            }
        }
        if !self.confirm_destructive && matches!(risk_level, RiskLevel::Destructive) {
            return PolicyAction::Deny;
        }
        if !self.mode.allows(risk_level) {
            return PolicyAction::Deny;
        }
        if matches!(risk_level, RiskLevel::Destructive) && self.confirm_destructive {
            PolicyAction::Ask
        } else {
            PolicyAction::Allow
        }
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
    fn test_default_policy_is_data_read_write_with_all_connections() {
        let policy = McpPolicy::default();
        assert_eq!(policy.mode, McpPermissionMode::DataReadWrite);
        assert!(policy.allowed_connection_ids.is_empty());
        assert!(policy.is_connection_allowed("any-id"));
        assert!(!policy.is_connection_read_only("any-id"));
        // Destructive confirmations are on by default (issue #10)
        assert!(policy.confirm_destructive);
    }

    #[test]
    fn test_confirm_destructive_false_blocks_destructive_even_in_full_access() {
        let policy = McpPolicy {
            mode: McpPermissionMode::FullAccess,
            confirm_destructive: false,
            ..McpPolicy::default()
        };
        // confirm_destructive=false hides Destructive from FullAccess too
        assert!(!policy.allows(RiskLevel::Destructive, None));
        // Safe/Elevated unaffected
        assert!(policy.allows(RiskLevel::Safe, None));
        assert!(policy.allows(RiskLevel::Elevated, None));
    }

    #[test]
    fn test_confirm_destructive_true_allows_destructive_only_in_full_access() {
        let policy = McpPolicy {
            mode: McpPermissionMode::FullAccess,
            confirm_destructive: true,
            ..McpPolicy::default()
        };
        assert!(policy.allows(RiskLevel::Destructive, None));

        let readonly_policy = McpPolicy {
            mode: McpPermissionMode::DataReadWrite,
            confirm_destructive: true,
            ..McpPolicy::default()
        };
        // Mode gate still applies — DataReadWrite blocks Destructive regardless
        assert!(!readonly_policy.allows(RiskLevel::Destructive, None));
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
                ConnectionMcpOverride { read_only: true, allowed_actions: None },
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
                ConnectionMcpOverride { read_only: true, allowed_actions: None },
            )]),
            confirm_destructive: false,
        };
        let v = serde_json::to_value(&policy).unwrap();
        assert_eq!(v["mode"], "DataReadWrite");
        assert_eq!(v["allowed_connection_ids"][0], "conn-1");
        assert_eq!(v["connection_overrides"]["conn-1"]["read_only"], true);
        assert_eq!(v["confirm_destructive"], false);

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

    #[test]
    fn test_decide_full_matrix() {
        // mode × risk × confirm_destructive → expected PolicyAction
        let cases: Vec<(McpPermissionMode, RiskLevel, bool, PolicyAction)> = vec![
            (McpPermissionMode::ReadOnly, RiskLevel::Safe, true, PolicyAction::Allow),
            (McpPermissionMode::ReadOnly, RiskLevel::Safe, false, PolicyAction::Allow),
            (McpPermissionMode::ReadOnly, RiskLevel::Elevated, true, PolicyAction::Deny),
            (McpPermissionMode::ReadOnly, RiskLevel::Elevated, false, PolicyAction::Deny),
            (McpPermissionMode::ReadOnly, RiskLevel::Destructive, true, PolicyAction::Deny),
            (McpPermissionMode::ReadOnly, RiskLevel::Destructive, false, PolicyAction::Deny),
            (McpPermissionMode::DataReadWrite, RiskLevel::Safe, true, PolicyAction::Allow),
            (McpPermissionMode::DataReadWrite, RiskLevel::Safe, false, PolicyAction::Allow),
            (McpPermissionMode::DataReadWrite, RiskLevel::Elevated, true, PolicyAction::Allow),
            (McpPermissionMode::DataReadWrite, RiskLevel::Elevated, false, PolicyAction::Allow),
            (McpPermissionMode::DataReadWrite, RiskLevel::Destructive, true, PolicyAction::Deny),
            (McpPermissionMode::DataReadWrite, RiskLevel::Destructive, false, PolicyAction::Deny),
            (McpPermissionMode::FullAccess, RiskLevel::Safe, true, PolicyAction::Allow),
            (McpPermissionMode::FullAccess, RiskLevel::Safe, false, PolicyAction::Allow),
            (McpPermissionMode::FullAccess, RiskLevel::Elevated, true, PolicyAction::Allow),
            (McpPermissionMode::FullAccess, RiskLevel::Elevated, false, PolicyAction::Allow),
            // NEW semantic: confirm_destructive=true + Destructive + FullAccess → Ask
            (McpPermissionMode::FullAccess, RiskLevel::Destructive, true, PolicyAction::Ask),
            (McpPermissionMode::FullAccess, RiskLevel::Destructive, false, PolicyAction::Deny),
        ];

        for (mode, risk, confirm, expected) in cases {
            let policy = McpPolicy { mode, confirm_destructive: confirm, ..McpPolicy::default() };
            let decided = policy.decide(risk, None);
            assert_eq!(
                decided, expected,
                "decide({mode:?}, {risk:?}, confirm_destructive={confirm}) expected {expected:?} got {decided:?}"
            );
            // Compat invariant: allows() == (decide() != Deny)
            assert_eq!(
                policy.allows(risk, None),
                !matches!(decided, PolicyAction::Deny),
                "allows() must equal (decide() != Deny) for {mode:?}/{risk:?}/{confirm}"
            );
        }
    }

    #[test]
    fn test_decide_connection_overrides() {
        // allowlist excludes a connection → Deny
        let allowlist =
            McpPolicy { allowed_connection_ids: vec!["conn-1".into()], ..McpPolicy::default() };
        assert_eq!(allowlist.decide(RiskLevel::Safe, Some("conn-2")), PolicyAction::Deny);
        assert_eq!(allowlist.decide(RiskLevel::Safe, Some("conn-1")), PolicyAction::Allow);

        // read-only override blocks Elevated for that connection
        let override_policy = McpPolicy {
            mode: McpPermissionMode::FullAccess,
            connection_overrides: HashMap::from([(
                "prod-es".into(),
                ConnectionMcpOverride { read_only: true, allowed_actions: None },
            )]),
            ..McpPolicy::default()
        };
        assert_eq!(
            override_policy.decide(RiskLevel::Elevated, Some("prod-es")),
            PolicyAction::Deny
        );
        assert_eq!(override_policy.decide(RiskLevel::Safe, Some("prod-es")), PolicyAction::Allow);
        assert_eq!(
            override_policy.decide(RiskLevel::Elevated, Some("staging")),
            PolicyAction::Allow
        );
    }

    #[test]
    fn test_policy_action_serde_names() {
        assert_eq!(serde_json::to_value(PolicyAction::Allow).unwrap(), json!("allow"));
        assert_eq!(serde_json::to_value(PolicyAction::Ask).unwrap(), json!("ask"));
        assert_eq!(serde_json::to_value(PolicyAction::Deny).unwrap(), json!("deny"));
    }

    #[test]
    fn test_connection_action_allowlist_blocks_risks() {
        // Read-only action list: only Safe capabilities pass
        let policy = McpPolicy {
            mode: McpPermissionMode::FullAccess,
            connection_overrides: HashMap::from([(
                "prod".into(),
                ConnectionMcpOverride {
                    read_only: false,
                    allowed_actions: Some(vec![McpAction::Read]),
                },
            )]),
            ..McpPolicy::default()
        };
        assert_eq!(policy.decide(RiskLevel::Safe, Some("prod")), PolicyAction::Allow);
        assert_eq!(policy.decide(RiskLevel::Elevated, Some("prod")), PolicyAction::Deny);
        assert_eq!(policy.decide(RiskLevel::Destructive, Some("prod")), PolicyAction::Deny);
    }

    #[test]
    fn test_connection_action_allowlist_read_write() {
        let policy = McpPolicy {
            mode: McpPermissionMode::FullAccess,
            connection_overrides: HashMap::from([(
                "prod".into(),
                ConnectionMcpOverride {
                    read_only: false,
                    allowed_actions: Some(vec![McpAction::Read, McpAction::Write]),
                },
            )]),
            ..McpPolicy::default()
        };
        assert_eq!(policy.decide(RiskLevel::Safe, Some("prod")), PolicyAction::Allow);
        assert_eq!(policy.decide(RiskLevel::Elevated, Some("prod")), PolicyAction::Allow);
        assert_eq!(policy.decide(RiskLevel::Destructive, Some("prod")), PolicyAction::Deny);
    }

    #[test]
    fn test_read_only_legacy_still_works_without_actions() {
        // Legacy read_only override (no allowed_actions) keeps old behavior
        let policy = McpPolicy {
            mode: McpPermissionMode::FullAccess,
            connection_overrides: HashMap::from([(
                "prod".into(),
                ConnectionMcpOverride { read_only: true, allowed_actions: None },
            )]),
            ..McpPolicy::default()
        };
        assert_eq!(policy.decide(RiskLevel::Safe, Some("prod")), PolicyAction::Allow);
        assert_eq!(policy.decide(RiskLevel::Elevated, Some("prod")), PolicyAction::Deny);
        assert!(policy.is_connection_read_only("prod"));
    }

    #[test]
    fn test_mcp_action_serde_names() {
        assert_eq!(serde_json::to_value(McpAction::Read).unwrap(), json!("read"));
        assert_eq!(serde_json::to_value(McpAction::Write).unwrap(), json!("write"));
        assert_eq!(serde_json::to_value(McpAction::Delete).unwrap(), json!("delete"));
        assert_eq!(McpAction::from_risk(RiskLevel::Safe), McpAction::Read);
        assert_eq!(McpAction::from_risk(RiskLevel::Elevated), McpAction::Write);
        assert_eq!(McpAction::from_risk(RiskLevel::Destructive), McpAction::Delete);
    }
}
