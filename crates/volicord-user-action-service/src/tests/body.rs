use super::validated_choice_intent;
use crate::body::construct_canonical_body;
use crate::model::UserActionBodyFacts;
use volicord_types::schema::UserActionRequestBody;
use volicord_types::values::{JudgmentResolutionOutcome, UserActionOptionAction};

#[test]
fn body_construction_closes_and_serializes_the_canonical_choice() {
    let constructed = construct_canonical_body(
        validated_choice_intent(),
        UserActionBodyFacts::Choice {
            close_basis_revision: None,
            result_refs: Vec::new(),
            residual_risk_ids: Vec::new(),
        },
        None,
    )
    .expect("valid choice must construct");

    let UserActionRequestBody::Choice(body) = &constructed.body else {
        panic!("choice intent must produce a choice body")
    };
    assert_eq!(body.question, "Choose the current product direction.");
    assert_eq!(body.options.len(), 2);
    assert!(body.options.iter().all(|option| {
        option.machine_action == UserActionOptionAction::Accept
            && option.resolution_outcome == JudgmentResolutionOutcome::Accepted
    }));
    let serialized =
        serde_json::to_value(&constructed.body).expect("canonical body must serialize");
    assert_eq!(serialized["action_type"], "choice");
    assert_eq!(
        serialized["question"],
        "Choose the current product direction."
    );
}

#[test]
fn body_construction_rejects_mismatched_action_facts() {
    let error = construct_canonical_body(
        validated_choice_intent(),
        UserActionBodyFacts::EvidenceObservation {
            artifact_candidates: Vec::new(),
        },
        None,
    )
    .expect_err("mismatched body facts must reject");

    assert_eq!(error.field(), "action.action_type");
}
