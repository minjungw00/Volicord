//! Closed diagnostic action values persisted with managed host setup state.

use std::{error::Error, fmt};

use schemars::JsonSchema;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

/// Machine-readable reason for damaged persisted host-setup action data.
pub const PERSISTED_USER_ACTIONS_CORRUPT_REASON: &str = "persisted_user_actions_corrupt";

/// Maximum UTF-8 byte length of one host-setup action message.
pub const MAX_HOST_SETUP_USER_ACTION_MESSAGE_BYTES: usize = 4_096;

/// Closed host-setup guidance kinds stored on an Agent Connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HostSetupUserActionKind {
    HostTrustRequired,
    ProjectApprovalRequired,
    ReloadRequired,
    ManagedHostStartupNotObserved,
    ManagedHostToolsListNotObserved,
    ActiveToolExposureUnconfirmed,
    ManagedHostStorageDegraded,
}

/// One validated diagnostic host-setup action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct HostSetupUserAction {
    pub kind: HostSetupUserActionKind,
    pub message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HostSetupUserActionWire {
    kind: HostSetupUserActionKind,
    message: String,
}

impl<'de> Deserialize<'de> for HostSetupUserAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = HostSetupUserActionWire::deserialize(deserializer)?;
        Self::try_new(wire.kind, wire.message).map_err(D::Error::custom)
    }
}

impl HostSetupUserAction {
    /// Constructs one action from trusted adapter-owned text.
    pub fn new(kind: HostSetupUserActionKind, message: impl Into<String>) -> Self {
        Self::try_new(kind, message)
            .expect("adapter-owned host-setup action messages must satisfy the closed contract")
    }

    /// Validates and constructs one action without normalizing its message.
    pub fn try_new(
        kind: HostSetupUserActionKind,
        message: impl Into<String>,
    ) -> Result<Self, HostSetupUserActionError> {
        let message = message.into();
        if message.is_empty()
            || message.len() > MAX_HOST_SETUP_USER_ACTION_MESSAGE_BYTES
            || message.as_bytes().contains(&0)
        {
            return Err(HostSetupUserActionError);
        }
        Ok(Self { kind, message })
    }
}

/// Validation failure for one host-setup action payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostSetupUserActionError;

impl fmt::Display for HostSetupUserActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("host-setup action message is empty, oversized, or contains NUL")
    }
}

impl Error for HostSetupUserActionError {}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn strict_array_decode_distinguishes_empty_and_current_values() {
        let empty: Vec<HostSetupUserAction> = serde_json::from_str("[]").unwrap();
        assert!(empty.is_empty());
        let current: Vec<HostSetupUserAction> = serde_json::from_value(json!([{
            "kind": "reload_required",
            "message": "reload Codex"
        }]))
        .unwrap();
        assert_eq!(current[0].kind, HostSetupUserActionKind::ReloadRequired);
    }

    #[test]
    fn strict_decode_rejects_every_damaged_shape() {
        for value in [
            json!({"kind": "reload_required", "message": "reload"}),
            json!([{"kind": "removed", "message": "reload"}]),
            json!([{"message": "reload"}]),
            json!([{"kind": "reload_required"}]),
            json!([{"kind": "reload_required", "message": "", "extra": true}]),
            json!([{"kind": "reload_required", "message": ""}]),
        ] {
            assert!(serde_json::from_value::<Vec<HostSetupUserAction>>(value).is_err());
        }
    }
}
