use std::{error::Error, fmt};

use serde_json::Value;
use volicord_types::{
    AuthorityNextActor, AuthorityReceipt, ChangeUnitId, CloseState, EffectKind, NextActionSummary,
    OperationCategory, ProjectId, ResponseKind, StateRecordKind, StateRecordRef, StatusCloseState,
    StatusResult, TaskId,
};

/// Adapter-supplied coordinates that a fresh authority status must confirm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityStatusExpectation {
    project_id: ProjectId,
    task_id: TaskId,
    expected_state_version: Option<u64>,
    expected_change_unit_id: Option<Option<ChangeUnitId>>,
}

impl AuthorityStatusExpectation {
    /// Creates an expectation for one selected Project and Task.
    pub fn new(project_id: ProjectId, task_id: TaskId) -> Self {
        Self {
            project_id,
            task_id,
            expected_state_version: None,
            expected_change_unit_id: None,
        }
    }

    /// Requires the status refresh to observe this already-captured state version.
    pub fn with_state_version(mut self, state_version: u64) -> Self {
        self.expected_state_version = Some(state_version);
        self
    }

    /// Requires the status refresh to confirm this already-captured current Change Unit.
    ///
    /// Passing `None` requires the Task to have no current Change Unit. Omitting this
    /// expectation leaves the freshly read status projection as the source of that fact.
    pub fn with_current_change_unit(mut self, change_unit_id: Option<ChangeUnitId>) -> Self {
        self.expected_change_unit_id = Some(change_unit_id);
        self
    }
}

/// A fully validated fresh status projection and its canonical authority receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedAuthorityStatus {
    status: StatusResult,
}

impl ValidatedAuthorityStatus {
    /// Borrows the validated status result.
    pub fn status(&self) -> &StatusResult {
        &self.status
    }

    /// Borrows the canonical receipt validated against the status result.
    pub fn authority_receipt(&self) -> &AuthorityReceipt {
        self.status
            .authority_receipt
            .as_ref()
            .expect("validated authority status always contains a receipt")
    }

    /// Borrows the current status next actions validated against the receipt.
    pub fn next_actions(&self) -> &[NextActionSummary] {
        &self.status.next_actions
    }

    /// Consumes the wrapper and returns the validated status result.
    pub fn into_status(self) -> StatusResult {
        self.status
    }

    /// Consumes the wrapper into the canonical receipt and current next actions.
    pub fn into_authority_projection(mut self) -> (AuthorityReceipt, Vec<NextActionSummary>) {
        let receipt = self
            .status
            .authority_receipt
            .take()
            .expect("validated authority status always contains a receipt");
        (receipt, self.status.next_actions)
    }
}

/// Closed failure classifications for validating a fresh authority status.
///
/// Variants intentionally carry no response body, parsing source, or private error text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityStatusValidationError {
    MalformedStatus,
    IneligibleStatus,
    MissingProjection,
    CoordinateMismatch,
    FreshnessMismatch,
    ProjectionMismatch,
}

impl fmt::Display for AuthorityStatusValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MalformedStatus => "authority status is malformed",
            Self::IneligibleStatus => "authority status is not an eligible read-only result",
            Self::MissingProjection => "authority status is missing a required projection",
            Self::CoordinateMismatch => "authority status coordinates do not match",
            Self::FreshnessMismatch => "authority status freshness does not match",
            Self::ProjectionMismatch => "authority status projections do not match",
        })
    }
}

impl Error for AuthorityStatusValidationError {}

/// Validates one freshly read `volicord.status` result as canonical authority.
///
/// This boundary accepts only a non-dry-run read-only result for the expected
/// Project and Task. It verifies the receipt against the same status result's
/// state version, Task reference, scope revision, current Change Unit, evidence
/// gate, close state, complete close blockers, and next action.
pub fn validate_authority_status(
    response: &Value,
    expectation: &AuthorityStatusExpectation,
) -> Result<ValidatedAuthorityStatus, AuthorityStatusValidationError> {
    let status = serde_json::from_value::<StatusResult>(response.clone())
        .map_err(|_| AuthorityStatusValidationError::MalformedStatus)?;
    if status.base.response_kind != ResponseKind::Result
        || status.base.effect_kind != EffectKind::ReadOnly
        || status.base.dry_run
    {
        return Err(AuthorityStatusValidationError::IneligibleStatus);
    }

    let state_version = status
        .base
        .state_version
        .ok_or(AuthorityStatusValidationError::MissingProjection)?;
    let receipt = status
        .authority_receipt
        .as_ref()
        .ok_or(AuthorityStatusValidationError::MissingProjection)?;
    let active_task = status
        .active_task
        .as_ref()
        .ok_or(AuthorityStatusValidationError::MissingProjection)?;
    let active_task_ref = active_task
        .task_ref
        .as_ref()
        .ok_or(AuthorityStatusValidationError::MissingProjection)?;
    let status_evidence_gate = status
        .evidence_gate
        .as_ref()
        .ok_or(AuthorityStatusValidationError::MissingProjection)?;
    let status_close_state = status
        .close_state
        .ok_or(AuthorityStatusValidationError::MissingProjection)?;
    let status_close_blockers = status
        .close_blockers
        .as_ref()
        .ok_or(AuthorityStatusValidationError::MissingProjection)?;
    let completion_claim_allowed = status_close_blockers.is_empty()
        && matches!(
            status_close_state,
            StatusCloseState::Ready | StatusCloseState::Closed
        );
    let missing_agent_owner_method = status
        .next_actions
        .iter()
        .chain(
            status_close_blockers
                .iter()
                .flat_map(|blocker| blocker.next_actions.iter()),
        )
        .any(|action| {
            action
                .allowed_operation_categories
                .contains(&OperationCategory::AgentWorkflow)
                && action.owner_method.is_none()
        });

    if receipt.project_id != expectation.project_id
        || receipt.task_ref.record_kind != StateRecordKind::Task
        || receipt.task_ref.project_id != expectation.project_id
        || receipt.task_ref.record_id.as_str() != expectation.task_id.as_str()
        || receipt.task_ref.task_id.as_ref() != Some(&expectation.task_id)
        || active_task.project_id != expectation.project_id
        || active_task_ref != &receipt.task_ref
    {
        return Err(AuthorityStatusValidationError::CoordinateMismatch);
    }

    if receipt.state_version != state_version
        || receipt.task_ref.produced_at_state_version.as_ref() != Some(&state_version)
        || active_task.state_version != state_version
        || expectation
            .expected_state_version
            .is_some_and(|expected| expected != state_version)
    {
        return Err(AuthorityStatusValidationError::FreshnessMismatch);
    }

    if active_task.scope_revision != receipt.scope_revision
        || !change_unit_projection_matches(
            active_task.active_change_unit_ref.as_ref(),
            receipt.change_unit_ref.as_ref(),
            &expectation.project_id,
            &expectation.task_id,
            state_version,
        )
        || !expected_change_unit_matches(
            expectation.expected_change_unit_id.as_ref(),
            receipt.change_unit_ref.as_ref(),
        )
        || active_task
            .evidence_gate
            .as_ref()
            .and_then(|gate| gate.as_ref())
            != receipt.evidence_gate.as_ref()
        || status_evidence_gate.as_ref() != receipt.evidence_gate.as_ref()
        || !close_state_matches(active_task.close_state, receipt.close_state)
        || status_close_state != receipt.close_state
        || active_task.close_blockers.as_deref() != Some(receipt.close_blockers.as_slice())
        || status_close_blockers != &receipt.close_blockers
        || status.next_actions.first() != receipt.next_action.as_ref()
        || authority_next_actor(receipt.next_action.as_ref()) != receipt.next_actor
        || receipt.completion_claim_allowed != completion_claim_allowed
        || missing_agent_owner_method
    {
        return Err(AuthorityStatusValidationError::ProjectionMismatch);
    }

    Ok(ValidatedAuthorityStatus { status })
}

fn change_unit_projection_matches(
    active: Option<&StateRecordRef>,
    receipt: Option<&StateRecordRef>,
    project_id: &ProjectId,
    task_id: &TaskId,
    state_version: u64,
) -> bool {
    match (active, receipt) {
        (None, None) => true,
        (Some(active), Some(receipt)) => {
            active.record_kind == StateRecordKind::ChangeUnit
                && receipt.record_kind == StateRecordKind::ChangeUnit
                && active.record_id == receipt.record_id
                && active.project_id == *project_id
                && receipt.project_id == *project_id
                && active.task_id.as_ref() == Some(task_id)
                && receipt.task_id.as_ref() == Some(task_id)
                && active
                    .produced_at_state_version
                    .as_ref()
                    .is_some_and(|produced| *produced <= state_version)
                && receipt.produced_at_state_version.as_ref() == Some(&state_version)
        }
        (None, Some(_)) | (Some(_), None) => false,
    }
}

fn close_state_matches(active: Option<CloseState>, receipt: StatusCloseState) -> bool {
    matches!(
        (active, receipt),
        (Some(CloseState::Ready), StatusCloseState::Ready)
            | (Some(CloseState::Blocked), StatusCloseState::Blocked)
            | (Some(CloseState::Closed), StatusCloseState::Closed)
            | (Some(CloseState::Cancelled), StatusCloseState::Cancelled)
            | (Some(CloseState::Superseded), StatusCloseState::Superseded)
            | (None, StatusCloseState::None)
    )
}

fn expected_change_unit_matches(
    expected: Option<&Option<ChangeUnitId>>,
    receipt: Option<&StateRecordRef>,
) -> bool {
    match expected {
        None => true,
        Some(None) => receipt.is_none(),
        Some(Some(expected)) => {
            receipt.is_some_and(|receipt| receipt.record_id.as_str() == expected.as_str())
        }
    }
}

fn authority_next_actor(action: Option<&NextActionSummary>) -> AuthorityNextActor {
    let Some(action) = action else {
        return AuthorityNextActor::None;
    };
    if action
        .allowed_operation_categories
        .contains(&OperationCategory::UserOnly)
    {
        AuthorityNextActor::User
    } else if action
        .allowed_operation_categories
        .contains(&OperationCategory::AgentWorkflow)
    {
        AuthorityNextActor::Agent
    } else {
        AuthorityNextActor::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_test_support::{core_fixtures::CoreFixture, TestRuntimeHomeMutation};
    use volicord_types::{ActorSource, OperationCategory};

    use crate::{CoreService, InvocationContext};

    type StatusMutation = (&'static str, fn(&mut Value));

    fn current_status(
        prefix: &str,
    ) -> Result<(CoreFixture, TaskId, Value, AuthorityStatusExpectation), Box<dyn Error>> {
        let fixture = CoreFixture::new(prefix)?;
        let admission = TestRuntimeHomeMutation::acquire(fixture.runtime_home_path())?;
        let context = admission.context()?;
        let service = CoreService::for_mutation(&context);
        let intake = service.intake(
            &context,
            fixture.intake_request(
                "req_authority_status_intake",
                "idem_authority_status",
                false,
                Some(0),
            ),
            agent_invocation(&fixture, OperationCategory::AgentWorkflow),
        )?;
        let task_id = intake
            .resolved_task_id
            .clone()
            .expect("intake resolves a Task");
        let status = service.status(
            fixture.status_request("req_authority_status_refresh", Some(task_id.as_str())),
            agent_invocation(&fixture, OperationCategory::Read),
        )?;
        let state_version = status.response_value["base"]["state_version"]
            .as_u64()
            .expect("status has a state version");
        let change_unit_id = status.response_value["authority_receipt"]["change_unit_ref"]
            ["record_id"]
            .as_str()
            .map(ChangeUnitId::new);
        let expectation =
            AuthorityStatusExpectation::new(ProjectId::new(fixture.project_id()), task_id.clone())
                .with_state_version(state_version)
                .with_current_change_unit(change_unit_id);
        Ok((fixture, task_id, status.response_value, expectation))
    }

    fn agent_invocation(
        fixture: &CoreFixture,
        operation_category: OperationCategory,
    ) -> InvocationContext {
        InvocationContext::new(
            ProjectId::new(fixture.project_id()),
            ActorSource::agent_connection(fixture.connection_id()),
            operation_category,
            "",
        )
        .with_validated_agent_session(
            crate::agent_session::validated_agent_session_for_test(
                fixture.connection_id(),
                fixture.project_id(),
            ),
        )
    }

    #[test]
    fn validates_fresh_status_into_a_typed_authority_projection() -> Result<(), Box<dyn Error>> {
        let (_fixture, task_id, response, expectation) = current_status("authority-status-valid")?;

        let validated = validate_authority_status(&response, &expectation)?;

        assert_eq!(
            validated.authority_receipt().task_ref.record_id.as_str(),
            task_id.as_str()
        );
        assert_eq!(
            validated.authority_receipt().next_action.as_ref(),
            validated.next_actions().first()
        );
        Ok(())
    }

    #[test]
    fn rejects_each_stale_or_divergent_authority_coordinate() -> Result<(), Box<dyn Error>> {
        let (_fixture, _task_id, response, expectation) =
            current_status("authority-status-divergence")?;
        let mutations: &[StatusMutation] = &[
            ("response kind", |value| {
                value["base"]["response_kind"] = serde_json::json!("dry_run")
            }),
            ("effect kind", |value| {
                value["base"]["effect_kind"] = serde_json::json!("no_effect")
            }),
            ("dry run", |value| {
                value["base"]["dry_run"] = serde_json::json!(true)
            }),
            ("base state version", |value| {
                value["base"]["state_version"] = serde_json::json!(999)
            }),
            ("missing state version", |value| {
                value["base"]["state_version"] = Value::Null
            }),
            ("missing receipt", |value| {
                value["authority_receipt"] = Value::Null
            }),
            ("missing active task", |value| {
                value["active_task"] = Value::Null
            }),
            ("receipt project", |value| {
                value["authority_receipt"]["project_id"] = serde_json::json!("other_project")
            }),
            ("receipt state version", |value| {
                value["authority_receipt"]["state_version"] = serde_json::json!(999)
            }),
            ("task kind", |value| {
                value["authority_receipt"]["task_ref"]["record_kind"] = serde_json::json!("run")
            }),
            ("task identity", |value| {
                value["authority_receipt"]["task_ref"]["record_id"] =
                    serde_json::json!("other_task")
            }),
            ("task project", |value| {
                value["authority_receipt"]["task_ref"]["project_id"] =
                    serde_json::json!("other_project")
            }),
            ("task scope identity", |value| {
                value["authority_receipt"]["task_ref"]["task_id"] = serde_json::json!("other_task")
            }),
            ("task version", |value| {
                value["authority_receipt"]["task_ref"]["produced_at_state_version"] =
                    serde_json::json!(999)
            }),
            ("scope revision", |value| {
                value["authority_receipt"]["scope_revision"] = serde_json::json!(999)
            }),
            ("active task project", |value| {
                value["active_task"]["project_id"] = serde_json::json!("other_project")
            }),
            ("active task state version", |value| {
                value["active_task"]["state_version"] = serde_json::json!(999)
            }),
            ("active task reference", |value| {
                value["active_task"]["task_ref"]["record_id"] = serde_json::json!("other_task")
            }),
            ("change unit identity", |value| {
                value["authority_receipt"]["change_unit_ref"]["record_id"] =
                    serde_json::json!("other_change_unit")
            }),
            ("change unit version", |value| {
                value["authority_receipt"]["change_unit_ref"]["produced_at_state_version"] =
                    serde_json::json!(999)
            }),
            ("active change unit version", |value| {
                value["active_task"]["active_change_unit_ref"]["produced_at_state_version"] =
                    serde_json::json!(999)
            }),
            ("change unit kind", |value| {
                value["authority_receipt"]["change_unit_ref"]["record_kind"] =
                    serde_json::json!("run")
            }),
            ("change unit project", |value| {
                value["authority_receipt"]["change_unit_ref"]["project_id"] =
                    serde_json::json!("other_project")
            }),
            ("change unit task", |value| {
                value["authority_receipt"]["change_unit_ref"]["task_id"] =
                    serde_json::json!("other_task")
            }),
            ("evidence gate", |value| {
                value["authority_receipt"]["evidence_gate"]["state"] = serde_json::json!("met")
            }),
            ("active task evidence gate", |value| {
                value["active_task"]["evidence_gate"]["state"] = serde_json::json!("met")
            }),
            ("missing evidence projection", |value| {
                value["evidence_gate"] = Value::Null
            }),
            ("close state", |value| {
                value["authority_receipt"]["close_state"] = serde_json::json!("ready")
            }),
            ("active task close state", |value| {
                value["active_task"]["close_state"] = serde_json::json!("ready")
            }),
            ("close blockers", |value| {
                value["authority_receipt"]["close_blockers"] = serde_json::json!([])
            }),
            ("status close blockers", |value| {
                value["close_blockers"] = serde_json::json!([])
            }),
            ("active task close blockers", |value| {
                value["active_task"]["close_blockers"] = serde_json::json!([])
            }),
            ("next action", |value| {
                value["authority_receipt"]["next_action"] = Value::Null
            }),
            ("status next actions", |value| {
                value["next_actions"] = serde_json::json!([])
            }),
            ("next actor", |value| {
                value["authority_receipt"]["next_actor"] = serde_json::json!("none")
            }),
            ("completion claim", |value| {
                value["authority_receipt"]["completion_claim_allowed"] = serde_json::json!(true)
            }),
            ("agent action owner method", |value| {
                value["authority_receipt"]["next_action"]["owner_method"] = Value::Null;
                value["next_actions"][0]["owner_method"] = Value::Null;
            }),
        ];

        for (name, mutate) in mutations {
            let mut candidate = response.clone();
            mutate(&mut candidate);
            assert!(
                validate_authority_status(&candidate, &expectation).is_err(),
                "{name} divergence must be rejected"
            );
        }
        Ok(())
    }

    #[test]
    fn rejects_guard_context_freshness_without_exposing_response_text() -> Result<(), Box<dyn Error>>
    {
        let (fixture, task_id, mut response, _) = current_status("authority-status-private-body")?;
        let private_text = "private-owner-response-body";
        response["private_error_body"] = Value::String(private_text.to_owned());
        let expectation =
            AuthorityStatusExpectation::new(ProjectId::new(fixture.project_id()), task_id)
                .with_state_version(999)
                .with_current_change_unit(Some(ChangeUnitId::new("other_change_unit")));

        let error = validate_authority_status(&response, &expectation)
            .expect_err("stale guard coordinates must fail");
        assert!(!error.to_string().contains(private_text));
        assert!(!format!("{error:?}").contains(private_text));
        Ok(())
    }
}
