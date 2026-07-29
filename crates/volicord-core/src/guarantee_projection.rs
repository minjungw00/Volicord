use crate::pipeline::VerifiedInvocationContext;
use crate::record_refs::state_ref;
use volicord_types::schema::{GuaranteeDisplay, ProjectEnforcementProfile, StateRecordRef};
use volicord_types::values::{ActorSource, StateRecordKind};

pub(crate) fn guarantee_display(
    profile: &ProjectEnforcementProfile,
    verified_invocation: &VerifiedInvocationContext,
    state_version: u64,
) -> GuaranteeDisplay {
    GuaranteeDisplay {
        level: profile.guarantee_level,
        basis: format!(
            "Project enforcement profile `{}` is active for actor source `{}` operation category `{}` verified by `{}`; enabled mechanisms: none; no stronger enforcement is active.",
            profile.profile_id,
            verified_invocation.actor_source,
            verified_invocation.operation_category.as_str(),
            verified_invocation.verification_basis
        ),
        capability_refs: vec![invocation_binding_ref(verified_invocation, state_version)],
    }
}

fn invocation_binding_ref(
    verified_invocation: &VerifiedInvocationContext,
    state_version: u64,
) -> StateRecordRef {
    match &verified_invocation.actor_source {
        ActorSource::AgentConnection(connection_id) => state_ref(
            StateRecordKind::AgentConnection,
            connection_id.as_str(),
            &verified_invocation.project_id,
            None,
            Some(state_version),
        ),
        ActorSource::LocalUser | ActorSource::System => state_ref(
            StateRecordKind::ProjectState,
            verified_invocation
                .actor_source
                .to_canonical_string()
                .as_str(),
            &verified_invocation.project_id,
            None,
            Some(state_version),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_types::ids::ProjectId;
    use volicord_types::schema::baseline_project_enforcement_profile;
    use volicord_types::values::{GuaranteeLevel, OperationCategory, StateRecordKind};

    #[test]
    fn guarantee_projection_uses_typed_profile_and_invocation_facts() {
        let invocation = VerifiedInvocationContext {
            project_id: ProjectId::new("project_guarantee"),
            actor_source: ActorSource::System,
            operation_category: OperationCategory::Read,
            verification_basis: "typed_test_basis".to_owned(),
            assurance_level: "typed_test_assurance".to_owned(),
            session_id: None,
            git_workspace_context: None,
        };

        let display = guarantee_display(&baseline_project_enforcement_profile(), &invocation, 8);

        assert_eq!(display.level, GuaranteeLevel::Cooperative);
        assert_eq!(display.capability_refs.len(), 1);
        assert_eq!(
            display.capability_refs[0].record_kind,
            StateRecordKind::ProjectState
        );
        assert_eq!(
            display.capability_refs[0]
                .produced_at_state_version
                .as_ref(),
            Some(&8)
        );
    }
}
