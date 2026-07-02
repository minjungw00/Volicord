use serde_json::Value;
use volicord_types::GuaranteeDisclosure;

pub(crate) const COOPERATIVE_DECISION_DISCLOSURE_TEXT: &str = "disclosure: cooperative host decision only; not OS sandboxing, network isolation, malware defense, full write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review replacement";

pub(crate) const DETECTIVE_OBSERVATION_DISCLOSURE_TEXT: &str = "disclosure: diagnostic observations only; not OS sandboxing, network isolation, malware defense, full write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review replacement";

pub(crate) fn cooperative_host_decision_disclosure_json() -> Value {
    disclosure_json(GuaranteeDisclosure::cooperative_host_decision())
}

pub(crate) fn detective_observation_disclosure_json() -> Value {
    disclosure_json(GuaranteeDisclosure::detective_observation())
}

fn disclosure_json(disclosure: GuaranteeDisclosure) -> Value {
    serde_json::to_value(disclosure).expect("guarantee disclosure should serialize")
}
