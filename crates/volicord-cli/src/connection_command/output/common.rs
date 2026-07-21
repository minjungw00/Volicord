pub(super) const COOPERATIVE_ASSURANCE_LIMIT: &str = "Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.";
pub(super) const DIAGNOSTIC_CAUSE_LIMIT: &str =
    "Diagnostic cause traversal is bounded to 32 edges and 128 findings.";
pub(super) const DIAGNOSTIC_FACT_LIMIT: &str = "Diagnostic fact strings are bounded to 1024 bytes, collections to 32 items, and sensitive fields remain redacted.";

pub(in crate::connection_command) fn cooperative_assurance_limits() -> Vec<String> {
    vec![
        DIAGNOSTIC_CAUSE_LIMIT.to_owned(),
        DIAGNOSTIC_FACT_LIMIT.to_owned(),
        COOPERATIVE_ASSURANCE_LIMIT.to_owned(),
    ]
}
