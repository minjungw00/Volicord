pub(super) const COOPERATIVE_ASSURANCE_LIMIT: &str = "Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.";

pub(in crate::connection_command) fn cooperative_assurance_limits() -> Vec<String> {
    vec![COOPERATIVE_ASSURANCE_LIMIT.to_owned()]
}
