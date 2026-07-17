use serde_json::{Map, Value};
use volicord_store::core_pipeline::ProjectStateHeader;
use volicord_types::{
    canonical_git_object_id, is_canonical_sha256_digest, ActorSource, ErrorCode, OperationCategory,
    ToolEnvelope, ToolError, ACTOR_ASSURANCE_AGENT_CONNECTION_COOPERATIVE,
    VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
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
    let verification_basis = verified_binding_basis(invocation)?;
    let mut git_workspace_context = invocation.git_workspace_context.clone();
    if let Some(workspace) = git_workspace_context.as_mut() {
        validate_git_workspace_context(workspace)?;
    }

    Ok(VerifiedInvocationContext {
        project_id: invocation.project_id.clone(),
        actor_source: invocation.actor_source.clone(),
        operation_category: invocation.operation_category,
        verification_basis,
        assurance_level: actor_assurance_level(&invocation.actor_source).to_owned(),
        session_id: invocation.session_id.clone(),
        git_workspace_context,
    })
}

fn verified_binding_basis(invocation: &InvocationContext) -> Result<String, ToolError> {
    match (&invocation.actor_source, &invocation.validated_host_receipt) {
        (ActorSource::AgentConnection(connection_id), Some(validated)) => {
            let receipt = validated.receipt();
            if invocation.invocation_binding_basis
                != VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING
                || receipt.project_id != invocation.project_id
                || receipt.connection_id.as_str() != connection_id.as_str()
            {
                return Err(invocation_context_mismatch_error(
                    "invocation.validated_host_receipt",
                ));
            }
            Ok(receipt.binding_digest.clone())
        }
        (ActorSource::AgentConnection(_), None) => Err(invocation_context_mismatch_error(
            "invocation.validated_host_receipt",
        )),
        (_, Some(_)) => Err(invocation_context_mismatch_error(
            "invocation.validated_host_receipt",
        )),
        (_, None) => Ok(invocation.invocation_binding_basis.trim().to_owned()),
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
    use crate::{
        pipeline::{GitWorkspaceContext, InvocationContext},
        validate_host_verification_receipt,
    };
    use volicord_types::{
        AgentConnectionId, CodexCapability, CurrentHostReceiptContext, HostKind,
        HostVerificationReceipt, HostVerificationResult, IntegrationProfile, PlatformEnvironment,
        PlatformReleaseCoordinate, ProjectId, UtcTimestamp, HOST_VERIFICATION_RECEIPT_CONTRACT_ID,
        VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
    };

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
    fn static_managed_stdio_label_cannot_authorize_without_a_typed_receipt() {
        let invocation = InvocationContext::new(
            ProjectId::new("project-a"),
            volicord_types::ActorSource::agent_connection("connection-a"),
            volicord_types::OperationCategory::Read,
            VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
        );

        let error = verified_binding_basis(&invocation).unwrap_err();
        assert_eq!(
            error.code,
            volicord_types::ErrorCode::InvocationContextMismatch
        );
        assert_eq!(
            error
                .details
                .and_then(|details| details.get("field").cloned()),
            Some(serde_json::Value::String(
                "invocation.validated_host_receipt".to_owned()
            ))
        );
    }

    #[test]
    fn alternate_agent_connection_label_cannot_bypass_a_typed_receipt() {
        let invocation = InvocationContext::new(
            ProjectId::new("project-a"),
            volicord_types::ActorSource::agent_connection("connection-a"),
            volicord_types::OperationCategory::Read,
            "nonstatic-caller-controlled-label",
        );

        let error = verified_binding_basis(&invocation).unwrap_err();
        assert_eq!(
            error.code,
            volicord_types::ErrorCode::InvocationContextMismatch
        );
        assert_eq!(
            error
                .details
                .and_then(|details| details.get("field").cloned()),
            Some(serde_json::Value::String(
                "invocation.validated_host_receipt".to_owned()
            ))
        );
    }

    #[test]
    fn validated_managed_receipt_supplies_the_exact_binding_identity() {
        const RAW: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        const PREFIXED: &str =
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let capabilities = vec![
            CodexCapability::ManagedStdioMcp,
            CodexCapability::PersonalManagedBinding,
            CodexCapability::RecordWorkflow,
            CodexCapability::SharedManagedBinding,
        ];
        let receipt = HostVerificationReceipt {
            contract_id: HOST_VERIFICATION_RECEIPT_CONTRACT_ID.to_owned(),
            project_id: ProjectId::new("project-a"),
            connection_id: AgentConnectionId::new("connection-a"),
            host_kind: HostKind::Codex,
            integration_profile: IntegrationProfile::Record,
            platform_environment: PlatformEnvironment::Linux,
            platform_release_coordinate: PlatformReleaseCoordinate::Native,
            required_capabilities: capabilities.clone(),
            verified_capabilities: capabilities.clone(),
            binding_digest: PREFIXED.to_owned(),
            generated_artifacts_digest: PREFIXED.to_owned(),
            executable_digest: RAW.to_owned(),
            policy_digest: PREFIXED.to_owned(),
            verifier_build_digest: RAW.to_owned(),
            observed_at: UtcTimestamp::parse("2026-07-17T01:00:00Z")
                .expect("fixture observation timestamp is valid"),
            expires_at: UtcTimestamp::parse("2026-07-17T01:05:00Z")
                .expect("fixture expiry timestamp is valid"),
            result: HostVerificationResult::Verified,
        };
        let current = CurrentHostReceiptContext {
            project_id: receipt.project_id.clone(),
            connection_id: receipt.connection_id.clone(),
            host_kind: receipt.host_kind,
            integration_profile: receipt.integration_profile,
            platform_environment: receipt.platform_environment,
            platform_release_coordinate: receipt.platform_release_coordinate.clone(),
            required_capabilities: capabilities,
            binding_digest: receipt.binding_digest.clone(),
            generated_artifacts_digest: receipt.generated_artifacts_digest.clone(),
            executable_digest: receipt.executable_digest.clone(),
            policy_digest: receipt.policy_digest.clone(),
            verifier_build_digest: receipt.verifier_build_digest.clone(),
        };
        let now = UtcTimestamp::parse("2026-07-17T01:01:00Z")
            .expect("fixture current timestamp is valid");
        let validated = validate_host_verification_receipt(receipt, &current, &now)
            .expect("matching fixture receipt must validate");
        let invocation = InvocationContext::new(
            ProjectId::new("project-a"),
            volicord_types::ActorSource::agent_connection("connection-a"),
            volicord_types::OperationCategory::Read,
            VERIFICATION_BASIS_MCP_STDIO_CONNECTION_BINDING,
        )
        .with_validated_host_receipt(validated);

        assert_eq!(
            verified_binding_basis(&invocation)
                .expect("validated fixture receipt must authorize the binding"),
            PREFIXED
        );
    }
}
