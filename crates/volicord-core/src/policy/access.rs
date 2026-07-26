use serde_json::{Map, Value};
use volicord_store::core_pipeline::ProjectStateHeader;
use volicord_types::canonical::{canonical_git_object_id, is_canonical_sha256_digest};
use volicord_types::schema::{ToolEnvelope, ToolError};
use volicord_types::values::{
    ActorSource, ErrorCode, OperationCategory, ACTOR_ASSURANCE_AGENT_CONNECTION_COOPERATIVE,
};

use crate::pipeline::{
    tool_error, InvocationAuthority, InvocationContext, MethodPolicy, VerifiedInvocationContext,
};

const ACTOR_ASSURANCE_LOCAL_USER_CHANNEL: &str = "local_user_channel";
const ACTOR_ASSURANCE_SYSTEM: &str = "system";

pub(crate) fn derive_verified_invocation(
    project_state: &ProjectStateHeader,
    envelope: &ToolEnvelope,
    invocation: &InvocationContext,
    policy: &MethodPolicy,
) -> Result<VerifiedInvocationContext, ToolError> {
    if envelope.project_id != invocation.project_id {
        return Err(invocation_context_mismatch_error("envelope.project_id"));
    }
    if project_state.project_id != invocation.project_id.as_str() {
        return Err(invocation_context_mismatch_error(
            "project_state.project_id",
        ));
    }
    if invocation.operation_category != policy.operation_category {
        return Err(operation_category_mismatch_error(
            policy.operation_category,
            invocation.operation_category,
        ));
    }
    let actor_source = invocation.actor_source();
    validate_actor_source(&actor_source, policy.operation_category)?;
    let (verification_basis, session_id) = verified_binding_basis(invocation)?;
    let mut git_workspace_context = invocation.git_workspace_context.clone();
    if let Some(workspace) = git_workspace_context.as_mut() {
        validate_git_workspace_context(workspace)?;
    }

    Ok(VerifiedInvocationContext {
        project_id: invocation.project_id.clone(),
        actor_source: actor_source.clone(),
        operation_category: invocation.operation_category,
        verification_basis,
        assurance_level: actor_assurance_level(&actor_source).to_owned(),
        session_id,
        git_workspace_context,
    })
}

fn verified_binding_basis(
    invocation: &InvocationContext,
) -> Result<(String, Option<String>), ToolError> {
    match invocation.authority() {
        InvocationAuthority::AgentConnection(validated) => {
            if validated.project_id() != &invocation.project_id {
                return Err(invocation_context_mismatch_error(
                    "invocation.validated_agent_session",
                ));
            }
            Ok((
                validated.verification_basis(),
                Some(validated.project_session_id().as_str().to_owned()),
            ))
        }
        InvocationAuthority::LocalUser { channel } => Ok((
            channel.verification_basis().to_owned(),
            invocation.session_id.clone(),
        )),
    }
}

fn validate_git_workspace_context(
    workspace: &mut crate::pipeline::GitWorkspaceContext,
) -> Result<(), ToolError> {
    if workspace.git_common_dir.trim().is_empty()
        || !std::path::Path::new(&workspace.git_common_dir).is_absolute()
    {
        return Err(invocation_context_mismatch_error(
            "invocation.git_workspace_context.git_common_dir",
        ));
    }
    if !is_canonical_sha256_digest(&workspace.worktree_id) {
        return Err(invocation_context_mismatch_error(
            "invocation.git_workspace_context.worktree_id",
        ));
    }
    if workspace.branch_ref.as_ref().is_some_and(|reference| {
        !reference.starts_with("refs/")
            || reference.contains(['\0', '\n', '\r'])
            || reference.trim() != reference
    }) {
        return Err(invocation_context_mismatch_error(
            "invocation.git_workspace_context.branch_ref",
        ));
    }
    if let Some(head_sha) = workspace.head_sha.as_mut() {
        *head_sha = canonical_git_object_id(head_sha).map_err(|_| {
            invocation_context_mismatch_error("invocation.git_workspace_context.head_sha")
        })?;
    }
    if !is_canonical_sha256_digest(&workspace.workspace_fingerprint) {
        return Err(invocation_context_mismatch_error(
            "invocation.git_workspace_context.workspace_fingerprint",
        ));
    }
    Ok(())
}

fn validate_actor_source(
    actor_source: &ActorSource,
    operation_category: OperationCategory,
) -> Result<(), ToolError> {
    match (operation_category, actor_source) {
        (OperationCategory::Read, ActorSource::AgentConnection(connection_id))
            if !connection_id.as_str().trim().is_empty() =>
        {
            Ok(())
        }
        (OperationCategory::Read, ActorSource::LocalUser) => Ok(()),
        (OperationCategory::AgentWorkflow, ActorSource::AgentConnection(connection_id))
            if !connection_id.as_str().trim().is_empty() =>
        {
            Ok(())
        }
        (OperationCategory::UserOnly, ActorSource::LocalUser) => Ok(()),
        (OperationCategory::AdminLocal, ActorSource::LocalUser) => Ok(()),
        (OperationCategory::LocalRecovery, ActorSource::LocalUser) => Ok(()),
        _ => Err(actor_source_mismatch_error(
            "invocation.actor_source",
            operation_category,
            actor_source,
        )),
    }
}

fn actor_assurance_level(actor_source: &ActorSource) -> &'static str {
    match actor_source {
        ActorSource::AgentConnection(_) => ACTOR_ASSURANCE_AGENT_CONNECTION_COOPERATIVE,
        ActorSource::LocalUser => ACTOR_ASSURANCE_LOCAL_USER_CHANNEL,
        ActorSource::System => ACTOR_ASSURANCE_SYSTEM,
    }
}

pub(crate) fn invocation_context_mismatch_error(field: &'static str) -> ToolError {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    tool_error(
        ErrorCode::InvocationContextMismatch,
        "invocation context does not match Core preflight requirements",
        false,
        Some(details),
    )
}

fn operation_category_mismatch_error(
    required_operation_category: OperationCategory,
    actual_operation_category: OperationCategory,
) -> ToolError {
    let mut details = Map::new();
    details.insert(
        "field".to_owned(),
        Value::String("invocation.operation_category".to_owned()),
    );
    details.insert(
        "required_operation_category".to_owned(),
        Value::String(required_operation_category.as_str().to_owned()),
    );
    details.insert(
        "actual_operation_category".to_owned(),
        Value::String(actual_operation_category.as_str().to_owned()),
    );
    tool_error(
        ErrorCode::InvocationContextMismatch,
        "invocation operation_category does not match the method operation category",
        false,
        Some(details),
    )
}

fn actor_source_mismatch_error(
    field: &'static str,
    operation_category: OperationCategory,
    actor_source: &ActorSource,
) -> ToolError {
    let mut details = Map::new();
    details.insert("field".to_owned(), Value::String(field.to_owned()));
    details.insert(
        "operation_category".to_owned(),
        Value::String(operation_category.as_str().to_owned()),
    );
    details.insert(
        "actor_source".to_owned(),
        Value::String(actor_source.to_canonical_string()),
    );
    tool_error(
        ErrorCode::InvocationContextMismatch,
        "actor_source is not allowed for the method operation category",
        false,
        Some(details),
    )
}

#[cfg(test)]
mod tests {
    use super::{validate_git_workspace_context, verified_binding_basis};
    use crate::pipeline::{GitWorkspaceContext, InvocationContext};
    use volicord_types::ids::ProjectId;
    use volicord_types::values::{OperationCategory, UserActionChannelKind};

    fn workspace_context(head_sha: &str) -> GitWorkspaceContext {
        GitWorkspaceContext {
            git_common_dir: "/tmp/volicord-git-object-id/.git".to_owned(),
            worktree_id: format!("sha256:{}", "1".repeat(64)),
            branch_ref: Some("refs/heads/test".to_owned()),
            head_sha: Some(head_sha.to_owned()),
            workspace_fingerprint: format!("sha256:{}", "2".repeat(64)),
        }
    }

    #[test]
    fn workspace_head_sha_uses_the_shared_git_object_id_canonicalizer() {
        let mut context = workspace_context(&"A".repeat(40));

        validate_git_workspace_context(&mut context).expect("uppercase Git OID should be valid");

        assert_eq!(context.head_sha, Some("a".repeat(40)));
    }

    #[test]
    fn workspace_head_sha_rejects_intermediate_length() {
        let mut context = workspace_context(&"a".repeat(63));

        assert!(validate_git_workspace_context(&mut context).is_err());
    }

    #[test]
    fn workspace_digest_coordinates_require_canonical_lowercase() {
        let mut context = workspace_context(&"a".repeat(40));
        context.worktree_id = format!("sha256:{}", "A".repeat(64));
        assert!(validate_git_workspace_context(&mut context).is_err());

        let mut context = workspace_context(&"a".repeat(40));
        context.workspace_fingerprint = format!("sha256:{}", "F".repeat(64));
        assert!(validate_git_workspace_context(&mut context).is_err());
    }

    #[test]
    fn local_user_authority_derives_a_controlled_semantic_basis() {
        let invocation = InvocationContext::local_user(
            ProjectId::new("project-a"),
            OperationCategory::Read,
            UserActionChannelKind::Cli,
        );

        let (basis, session_id) =
            verified_binding_basis(&invocation).expect("typed local authority is valid");
        assert_eq!(basis, UserActionChannelKind::Cli.verification_basis());
        assert_eq!(session_id, None);
    }

    #[test]
    fn validated_session_supplies_the_current_operational_identity() {
        let validated =
            crate::agent_session::validated_agent_session_for_test("connection-a", "project-a");
        let invocation = InvocationContext::agent_connection(OperationCategory::Read, validated);

        let (basis, session_id) = verified_binding_basis(&invocation)
            .expect("validated fixture session must authorize the invocation");
        assert!(basis.starts_with(
            "connection:connection-a/session:agent_test_project_session/revision:sha256:"
        ));
        assert_eq!(session_id.as_deref(), Some("agent_test_project_session"));
    }
}
