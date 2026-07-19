use serde_json::Value;

pub(in crate::connection_command) const PERSISTED_CONNECTION_METADATA_CORRUPT_REASON: &str =
    "persisted_connection_metadata_corrupt";

pub(in crate::connection_command) fn decode_persisted_object(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text)
        .ok()
        .filter(Value::is_object)
}
