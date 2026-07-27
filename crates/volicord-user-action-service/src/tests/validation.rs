use super::validation_input;
use crate::validation::validate_user_action;
use volicord_types::values::{UserActionRequiredFor, UtcTimestamp};

#[test]
fn validation_accepts_a_current_compatible_semantic_intent() {
    let validated =
        validate_user_action(validation_input()).expect("valid semantic intent must pass");

    assert_eq!(validated.task_id.as_str(), "task-test");
    assert_eq!(
        validated
            .coordinate_change_unit_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("change-test")
    );
    assert_eq!(
        validated.required_for,
        vec![UserActionRequiredFor::CloseComplete]
    );
}

#[test]
fn validation_rejects_empty_duplicate_and_incompatible_operation_targets() {
    let mut empty = validation_input();
    empty.intent.required_for.clear();
    assert_eq!(
        validate_user_action(empty)
            .expect_err("empty operation targets must reject")
            .field(),
        "required_for"
    );

    let mut duplicate = validation_input();
    duplicate.intent.required_for = vec![
        UserActionRequiredFor::RecordRun,
        UserActionRequiredFor::RecordRun,
    ];
    assert_eq!(
        validate_user_action(duplicate)
            .expect_err("duplicate operation targets must reject")
            .field(),
        "required_for"
    );

    let mut incompatible = validation_input();
    incompatible.intent.required_for = vec![UserActionRequiredFor::CloseCancel];
    assert_eq!(
        validate_user_action(incompatible)
            .expect_err("incompatible operation target must reject")
            .field(),
        "required_for"
    );
}

#[test]
fn validation_rejects_non_current_coordinates_before_body_construction() {
    let mut input = validation_input();
    input.intent.change_unit_id = Some(volicord_types::ids::ChangeUnitId::new("change-other"));
    input.requested_change_unit_exists = false;

    let error =
        validate_user_action(input).expect_err("unknown Change Unit coordinate must reject");

    assert_eq!(error.field(), "change_unit_id");
}

#[test]
fn validation_uses_a_half_open_expiration_boundary() {
    let operation_time =
        UtcTimestamp::parse("2026-07-27T00:00:00Z").expect("test timestamp must parse");

    let mut at_boundary = validation_input();
    at_boundary.intent.expires_at = Some(operation_time.clone()).into();
    assert_eq!(
        validate_user_action(at_boundary)
            .expect_err("expiration at operation time must reject")
            .field(),
        "expires_at"
    );

    let mut after_boundary = validation_input();
    after_boundary.intent.expires_at =
        Some(UtcTimestamp::parse("2026-07-27T00:00:00.000000001Z").unwrap()).into();
    assert!(validate_user_action(after_boundary).is_ok());
}
