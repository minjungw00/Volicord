use serde_json::{json, Map, Value};

/// Includes both MCP content representations. 768 KiB of headroom below 1 MiB.
pub const HOST_READ_RESULT_BYTE_BUDGET: usize = 256 * 1024;
/// Compact JSON is repeated in text and structuredContent. Even maximally escaped
/// text takes at most twice its original JSON bytes, leaving envelope headroom.
pub const HOST_READ_STRUCTURED_BYTE_BUDGET: usize = 80 * 1024;

/// Bound a read projection at complete field/item boundaries. The caller supplies
/// an authoritative inspection basis separately; this is never a write decoder.
/// Object identities and state precede prose and repeated detail. Arrays retain a
/// stable prefix, followed by an explicit suffix omission (including exact count).
pub fn bounded_read_section(value: Value, budget: usize) -> Value {
    if value.to_string().len() <= budget {
        return value;
    }
    let original_bytes = value.to_string().len();
    match value {
        Value::Array(items) => {
            let total = items.len();
            let mut selected = Vec::new();
            let mut used = 2;
            // Reserve the suffix report, including an optional stable identity.
            let available = budget.saturating_sub(384);
            for item in &items {
                let remaining = available.saturating_sub(used);
                if remaining < 512 {
                    break;
                }
                let item = bounded_read_section(item.clone(), remaining.min(8 * 1024));
                let bytes = item.to_string().len() + 1;
                if used + bytes > available {
                    break;
                }
                used += bytes;
                selected.push(item);
            }
            if selected.len() < total {
                let first = &items[selected.len()];
                let identity = ["identity", "candidate_id", "choice_id", "analysis_snapshot"]
                    .iter()
                    .find_map(|key| first.get(key))
                    .filter(|id| id.to_string().len() <= 128);
                selected.push(json!({"transport_omission":{
                    "reason":"serialized_byte_budget", "omitted_count":total-selected.len(),
                    "first_omitted_identity":identity,
                    "basis":"same parent identity, field and stable input order; inspect the authoritative record",
                }}));
            }
            Value::Array(selected)
        }
        Value::Object(object) if budget >= 1024 => {
            let mut entries = object.into_iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| (field_priority(key), key.clone()));
            let mut result = Map::new();
            let mut omitted = 0;
            for (key, value) in entries {
                let used = Value::Object(result.clone()).to_string().len();
                let available = budget.saturating_sub(used + key.len() + 384);
                if available < 512 {
                    omitted += 1;
                    continue;
                }
                result.insert(key, bounded_read_section(value, available.min(8 * 1024)));
            }
            if omitted > 0 {
                result.insert(
                    "transport_omission".into(),
                    json!({
                        "reason":"serialized_byte_budget", "omitted_field_count":omitted,
                        "basis":"inspect the authoritative record at this identity",
                    }),
                );
            }
            Value::Object(result)
        }
        _ => json!({"transport_omission":{
            "reason":"serialized_byte_budget", "exact_json_bytes":original_bytes,
            "basis":"inspect the complete field on the authoritative parent record",
        }}),
    }
}

fn field_priority(key: &str) -> u8 {
    if matches!(
        key,
        "project_id"
            | "transport_budget"
            | "checkpoint"
            | "goal_basis"
            | "goals"
            | "behaviorally_relevant_context"
            | "decisions"
            | "open_questions"
            | "next_step"
    ) {
        return 0;
    }
    if key == "identity"
        || key.ends_with("_id")
        || key.ends_with("_ids")
        || matches!(
            key,
            "revision"
                | "state"
                | "kind"
                | "role"
                | "work_state"
                | "blocks_ordinary_work"
                | "required_next_action"
                | "disposition"
                | "user_review"
                | "user_acceptance"
                | "stage"
                | "availability"
                | "analysis_snapshot"
                | "repository_snapshot"
                | "freshness"
                | "language"
                | "capability"
        )
    {
        0
    } else if matches!(
        key,
        "goal"
            | "statement"
            | "choice"
            | "rationale"
            | "next_step"
            | "reason"
            | "verification"
            | "unresolved_requirements"
            | "state_change"
    ) {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_history_preserves_current_resume_and_has_an_exact_wire_bound() {
        let paths = (0..24_000)
            .map(|i| json!({"path":format!("packages/service-{i}/src/요청_처리.rs")}))
            .collect::<Vec<_>>();
        let current = json!({
            "project_id":"project", "goals":["Resume the local service"],
            "goal_basis":[{"identity":"goal","statement":"Resume the local service","source_ids":["user"]}],
            "behaviorally_relevant_context":[{"identity":"constraint","role":"constraint","statement":"Keep state local","source_ids":["user"]}],
            "decisions":[{"identity":"decision","state":"current","choice":"local","rationale":"privacy","source_basis":["answer"]}],
            "checkpoint":{"identity":"checkpoint","revision":4,"work_state":"paused","goal":"goal","next_step":"Run focused checks","verification":[{"state":"passed","source_id":"command"}],"user_review":{"state":"pending"},"user_acceptance":{"state":"not_requested"}},
            "snapshots":[{"analysis_snapshot":"analysis","repository_snapshot":"repository","freshness":{"state":"stale"},"capabilities":[{"capability":"structural","language":"Python","state":"partial","coverage":{"included":paths,"failed":[{"path":"broken.py"}]}}]}],
        });
        assert!(current.to_string().len() > 1024 * 1024);
        let bounded = bounded_read_section(current.clone(), 56 * 1024);
        for key in [
            "goals",
            "goal_basis",
            "behaviorally_relevant_context",
            "decisions",
            "checkpoint",
        ] {
            assert_eq!(bounded[key], current[key], "lost current {key}");
        }
        assert_eq!(bounded["snapshots"][0]["analysis_snapshot"], "analysis");
        assert_eq!(bounded, bounded_read_section(current, 56 * 1024));
        assert!(bounded.to_string().contains("omitted_count"));
        let wire = json!({"content":[{"type":"text","text":bounded.to_string()}],"structuredContent":bounded,"isError":false});
        let bytes = serde_json::to_vec(&wire).unwrap();
        assert!(bytes.len() <= HOST_READ_RESULT_BYTE_BUDGET);
        let decoded: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(decoded["content"][0]["text"].as_str().unwrap()).unwrap(),
            decoded["structuredContent"]
        );
    }

    #[test]
    fn whole_oversized_fields_and_escaped_unicode_never_break_json_or_the_budget() {
        for size in [1024, 2048, 8192, 80 * 1024] {
            let input = json!({"identity":"stable","state":"blocked","description":"\"\\\n한글".repeat(100_000),"history":vec!["large"; 5000]});
            let output = bounded_read_section(input, size);
            assert!(
                output.to_string().len() <= size,
                "{size}: {}",
                output.to_string().len()
            );
            assert_eq!(output["identity"], "stable");
            assert_eq!(output["state"], "blocked");
            assert!(output["description"].get("transport_omission").is_some());
        }
    }
}
