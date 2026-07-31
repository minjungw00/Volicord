use serde_json::Value;
use volicord_types::schema::GuaranteeDisclosure;

pub(crate) const COOPERATIVE_DECISION_DISCLOSURE_TEXT: &str = "Does not prove: OS sandboxing, network isolation, malware defense, full write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review completion";

pub(crate) fn cooperative_host_decision_disclosure_json() -> Value {
    serde_json::to_value(GuaranteeDisclosure::cooperative_host_decision())
        .expect("guarantee disclosure should serialize")
}
