use serde_json::{json, Value};

pub(in crate::connection_command) const PERSISTED_CONNECTION_METADATA_CORRUPT_REASON: &str =
    "persisted_connection_metadata_corrupt";

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
