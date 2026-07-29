use volicord_store::core_pipeline::ProjectStateHeader;
use volicord_types::schema::ToolEnvelope;

use crate::{
    artifact::ArtifactPolicyError, error_boundary::store::store_error_plan,
    method_execution::PlanError, method_rejection::validation_rejected,
    pipeline::CorePipelineError,
};

pub(crate) fn artifact_policy_plan_error(
    envelope: &ToolEnvelope,
    project_state: &ProjectStateHeader,
    error: ArtifactPolicyError,
) -> PlanError {
    match error {
        ArtifactPolicyError::Core(CorePipelineError::Store(error)) => {
            store_error_plan(envelope, project_state, error)
        }
        ArtifactPolicyError::Core(error) => PlanError::Core(error),
        ArtifactPolicyError::Validation { field, message } => {
            match validation_rejected(
                envelope.dry_run,
                Some(project_state.state_version),
                field,
                message,
            ) {
                Ok(response) => PlanError::Response(Box::new(response)),
                Err(error) => PlanError::Core(error),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::method_execution::PlanError;
    use serde_json::json;
    use volicord_types::values::UtcTimestamp;

    #[test]
    fn artifact_validation_maps_only_at_the_public_response_boundary() {
        let envelope: ToolEnvelope = serde_json::from_value(json!({
            "project_id": "project_artifact_boundary",
            "task_id": null,
            "request_id": "request_artifact_boundary",
            "idempotency_key": null,
            "expected_state_version": null,
            "dry_run": false,
            "locale": null
        }))
        .expect("valid envelope");
        let project_state = ProjectStateHeader {
            project_id: "project_artifact_boundary".to_owned(),
            state_version: 9,
            active_task_id: None,
            updated_at: UtcTimestamp::parse("2026-07-29T00:00:00Z").expect("timestamp"),
        };

        let error = artifact_policy_plan_error(
            &envelope,
            &project_state,
            ArtifactPolicyError::Validation {
                field: "source_refs",
                message: "source refs are invalid",
            },
        );

        let PlanError::Response(response) = error else {
            panic!("validation should project a response");
        };
        assert_eq!(
            response.response_value["errors"][0]["code"],
            "VALIDATION_FAILED"
        );
        assert_eq!(
            response.response_value["errors"][0]["details"]["field"],
            "source_refs"
        );
    }
}
