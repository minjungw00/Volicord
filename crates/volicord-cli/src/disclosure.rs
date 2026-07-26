use serde_json::Value;
use volicord_types::schema::GuaranteeDisclosure;

pub(crate) const DOES_NOT_PROVE_LABEL: &str = "Does not prove";

pub(crate) const AUTHORITY_RECORD_NON_GUARANTEE_TEXT: &str = "correctness, test sufficiency, QA or human review completion, deployment success, risk-free outcome, OS write permission, or that a product-file write occurred";

pub(crate) const USER_CHANNEL_NON_GUARANTEE_TEXT: &str = "approval, close readiness, correctness, test sufficiency, human review completion, or that listing resolved a user action";

pub(crate) const DIAGNOSTIC_OBSERVATION_NON_GUARANTEE_TEXT: &str = "OS sandboxing, network isolation, malware defense, full write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review completion";

pub(crate) const COOPERATIVE_DECISION_DISCLOSURE_TEXT: &str = "Does not prove: OS sandboxing, network isolation, malware defense, full write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review completion";

pub(crate) fn does_not_prove_line(non_guarantees: &str) -> String {
    format!("{DOES_NOT_PROVE_LABEL}: {non_guarantees}\n")
}

pub(crate) fn cooperative_host_decision_disclosure_json() -> Value {
    serde_json::to_value(GuaranteeDisclosure::cooperative_host_decision())
        .expect("guarantee disclosure should serialize")
}
