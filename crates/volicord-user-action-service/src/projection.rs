use crate::{
    authority::user_action_from_record,
    error::{UserActionInvariantError, UserActionServiceError},
    model::{
        PendingUserAction, PendingUserActionFacts, UserActionResolutionAvailability,
        UserActionResolutionFacts, UserActionResolutionFactsBody,
    },
};
use volicord_store::core_pipeline::{ProjectStateHeader, StoredUserActionRecordSet};
use volicord_types::{
    ids::{ProjectId, TaskId},
    schema::{
        StateRecordRef, UserActionRequest, UserActionRequestBody, UserActionResolution,
        UserActionResolutionBody,
    },
    values::{StateRecordKind, UtcTimestamp},
};

pub fn pending_user_action_facts_from_records(
    project_id: ProjectId,
    task_id: TaskId,
    project_state: ProjectStateHeader,
    observed_at: UtcTimestamp,
    records: Vec<StoredUserActionRecordSet>,
) -> Result<PendingUserActionFacts, UserActionServiceError> {
    let actions = records
        .iter()
        .map(|record| {
            let request = user_action_from_record(record, project_state.state_version)?;
            Ok(PendingUserAction {
                request_ref: StateRecordRef::new(
                    StateRecordKind::UserActionRequest,
                    request.user_action_request_id.as_str(),
                    request.project_id.clone(),
                    Some(request.task_id.clone()),
                    Some(project_state.state_version),
                ),
                request,
                resolution_availability: UserActionResolutionAvailability::from_status(
                    record.status(),
                ),
            })
        })
        .collect::<Result<Vec<_>, UserActionServiceError>>()?;
    Ok(PendingUserActionFacts {
        project_id,
        task_id,
        observed_state_version: project_state.state_version,
        observed_at,
        actions,
    })
}

pub fn user_action_resolution_facts(
    request: &UserActionRequest,
    resolution: &UserActionResolution,
) -> Result<UserActionResolutionFacts, UserActionServiceError> {
    let resolution_summary = match &resolution.body {
        UserActionResolutionBody::Choice {
            selected_option_id,
            machine_action,
            resolution_outcome,
            ..
        } => {
            let UserActionRequestBody::Choice(choice) = &request.body else {
                return Err(invalid_resolution_projection());
            };
            let selected_option_label = choice
                .options
                .iter()
                .find(|option| option.option_id == *selected_option_id)
                .map(|option| option.label.clone())
                .ok_or_else(invalid_resolution_projection)?;
            UserActionResolutionFactsBody::Choice {
                selected_option_id: selected_option_id.clone(),
                selected_option_label,
                machine_action: *machine_action,
                resolution_outcome: *resolution_outcome,
            }
        }
        UserActionResolutionBody::EvidenceObservation { observation } => {
            if !matches!(request.body, UserActionRequestBody::EvidenceObservation(_)) {
                return Err(invalid_resolution_projection());
            }
            UserActionResolutionFactsBody::EvidenceObservation {
                target: observation.target.clone(),
                artifact_refs: observation.output_artifact_refs.clone(),
                relevance_status: observation.relevance_status,
            }
        }
    };
    Ok(UserActionResolutionFacts {
        user_action_resolution_id: resolution.user_action_resolution_id.clone(),
        user_action_request_id: resolution.user_action_request_id.clone(),
        action_kind: resolution.action_kind,
        channel_kind: resolution.channel_kind,
        resolved_at: resolution.resolved_at.clone(),
        resolution: resolution_summary,
    })
}

fn invalid_resolution_projection() -> UserActionServiceError {
    UserActionServiceError::Invariant(UserActionInvariantError::ActionFactsMismatch)
}
