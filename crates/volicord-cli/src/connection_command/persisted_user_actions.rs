use serde_json::{json, Value};

use crate::host_integration::UserAction;
#[cfg(test)]
use crate::host_integration::UserActionKind;
use volicord_types::PERSISTED_USER_ACTIONS_CORRUPT_REASON;

pub(in crate::connection_command) const PERSISTED_VERIFICATION_REPORT_CORRUPT_REASON: &str =
    "persisted_verification_report_corrupt";
pub(in crate::connection_command) const PERSISTED_CONNECTION_METADATA_CORRUPT_REASON: &str =
    "persisted_connection_metadata_corrupt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::connection_command) enum PersistedUserActions {
    Current(Vec<UserAction>),
    Corrupt,
}

impl PersistedUserActions {
    pub(in crate::connection_command) fn actions(&self) -> Option<&[UserAction]> {
        match self {
            Self::Current(actions) => Some(actions),
            Self::Corrupt => None,
        }
    }

    pub(in crate::connection_command) fn is_corrupt(&self) -> bool {
        matches!(self, Self::Corrupt)
    }

    /// Verification is the repair boundary: it recomputes current typed actions and replaces
    /// corrupt persisted input instead of making an authority decision from it.
    pub(in crate::connection_command) fn actions_for_verification_repair(&self) -> &[UserAction] {
        self.actions().unwrap_or(&[])
    }

    pub(in crate::connection_command) fn state_json(&self) -> Value {
        match self {
            Self::Current(actions) => json!({
                "status": "current",
                "reason": Value::Null,
                "action_count": actions.len(),
            }),
            Self::Corrupt => json!({
                "status": "degraded",
                "reason": PERSISTED_USER_ACTIONS_CORRUPT_REASON,
                "repair": "connection_verify_regenerates_current_typed_values",
            }),
        }
    }
}

pub(in crate::connection_command) fn decode_persisted_user_actions(
    text: &str,
) -> PersistedUserActions {
    match serde_json::from_str::<Vec<UserAction>>(text) {
        Ok(actions) => PersistedUserActions::Current(actions),
        Err(_) => PersistedUserActions::Corrupt,
    }
}

pub(in crate::connection_command) fn persisted_user_actions_check_json(
    state: &PersistedUserActions,
) -> Value {
    if state.is_corrupt() {
        json!({
            "id": "persisted_user_actions",
            "status": "degraded",
            "summary": "stored UserAction values could not be decoded as the current typed contract",
            "details": state.state_json(),
        })
    } else {
        json!({
            "id": "persisted_user_actions",
            "status": "passed",
            "summary": "stored UserAction values match the current typed contract",
            "details": state.state_json(),
        })
    }
}

pub(in crate::connection_command) fn decode_persisted_object(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text)
        .ok()
        .filter(Value::is_object)
}

pub(in crate::connection_command) fn persisted_object_state_json(
    text: &str,
    corrupt_reason: &'static str,
    repair: &'static str,
) -> Value {
    if decode_persisted_object(text).is_some() {
        json!({
            "status": "current",
            "reason": Value::Null,
        })
    } else {
        json!({
            "status": "degraded",
            "reason": corrupt_reason,
            "repair": repair,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_decode_distinguishes_valid_empty_and_nonempty_arrays() {
        assert_eq!(
            decode_persisted_user_actions("[]"),
            PersistedUserActions::Current(Vec::new())
        );
        let decoded = decode_persisted_user_actions(
            r#"[{"kind":"reload_required","message":"reload Codex"}]"#,
        );
        assert_eq!(
            decoded.actions(),
            Some(
                [UserAction::new(
                    UserActionKind::ReloadRequired,
                    "reload Codex"
                )]
                .as_slice()
            )
        );
    }

    #[test]
    fn strict_decode_degrades_every_persisted_damage_shape() {
        for text in [
            "[",
            r#"{"kind":"reload_required"}"#,
            r#"["not-an-object"]"#,
            r#"[{"message":"missing kind"}]"#,
            r#"[{"kind":"removed_variant","message":"unknown"}]"#,
            r#"[{"kind":"reload_required","message":"reload","extra":true}]"#,
            r#"[{"kind":"reload_required","message":""}]"#,
        ] {
            let decoded = decode_persisted_user_actions(text);
            assert!(decoded.is_corrupt(), "fixture should degrade: {text}");
            assert_eq!(
                decoded.state_json()["reason"],
                PERSISTED_USER_ACTIONS_CORRUPT_REASON
            );
            assert!(decoded.actions().is_none());
        }
    }

    #[test]
    fn verification_repair_replaces_corrupt_input_with_current_typed_values() {
        let corrupt =
            decode_persisted_user_actions(r#"[{"kind":"removed_variant","message":"unknown"}]"#);
        assert!(corrupt.actions_for_verification_repair().is_empty());

        let repaired_actions = vec![UserAction::new(
            UserActionKind::ManagedHostStartupNotObserved,
            "start Codex",
        )];
        let repaired_json = serde_json::to_string(&repaired_actions).expect("typed actions encode");
        let repaired = decode_persisted_user_actions(&repaired_json);

        assert_eq!(repaired.actions(), Some(repaired_actions.as_slice()));
        assert_eq!(repaired.state_json()["status"], "current");
    }
}
