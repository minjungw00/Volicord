use serde_json::{Map, Value};
use volicord_store::core_pipeline::ProjectStateHeader;
use volicord_types::{
    ActorSource, ErrorCode, OperationCategory, ToolEnvelope, ToolError,
    ACTOR_ASSURANCE_AGENT_CONNECTION_COOPERATIVE,
};

use crate::pipeline::{tool_error, InvocationContext, MethodPolicy, VerifiedInvocationContext};

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
    validate_actor_source(&invocation.actor_source, policy.operation_category)?;
    if invocation.invocation_binding_basis.trim().is_empty() {
        return Err(invocation_context_mismatch_error(
            "invocation.invocation_binding_basis",
        ));
    }
    if let Some(workspace) = invocation.git_workspace_context.as_ref() {
        validate_git_workspace_context(workspace)?;
    }

    Ok(VerifiedInvocationContext {
        project_id: invocation.project_id.clone(),
        actor_source: invocation.actor_source.clone(),
        operation_category: invocation.operation_category,
        verification_basis: invocation.invocation_binding_basis.trim().to_owned(),
        assurance_level: actor_assurance_level(&invocation.actor_source).to_owned(),
        session_id: invocation.session_id.clone(),
        host_elicitation_available: invocation.host_elicitation_available,
        local_web_consent_available: invocation.local_web_consent_available,
        git_workspace_context: invocation.git_workspace_context.clone(),
    })
}

fn validate_git_workspace_context(
    workspace: &crate::pipeline::GitWorkspaceContext,
) -> Result<(), ToolError> {
    let sha256_coordinate = |value: &str| {
        value.strip_prefix("sha256:").is_some_and(|digest| {
            digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    };
    if workspace.git_common_dir.trim().is_empty()
        || !std::path::Path::new(&workspace.git_common_dir).is_absolute()
    {
        return Err(invocation_context_mismatch_error(
            "invocation.git_workspace_context.git_common_dir",
        ));
    }
    if !sha256_coordinate(&workspace.worktree_id) {
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
    if workspace.head_sha.as_ref().is_some_and(|sha| {
        !matches!(sha.len(), 40 | 64) || !sha.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        return Err(invocation_context_mismatch_error(
            "invocation.git_workspace_context.head_sha",
        ));
    }
    if !sha256_coordinate(&workspace.workspace_fingerprint) {
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
