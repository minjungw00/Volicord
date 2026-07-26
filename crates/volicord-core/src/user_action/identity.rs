use crate::pipeline::{CoreResult, CoreService};
use serde::Serialize;
use volicord_types::ids::{DurableIdKind, IdempotencyKey, UnrecordedChangeId, UserActionRequestId};
use volicord_types::values::MethodName;

/// Current Core operation that owns a newly constructed UserAction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UserActionOrigin {
    DirectRequest,
    Reconciliation {
        unrecorded_change_id: UnrecordedChangeId,
    },
}

/// Stable source identity used by Store deduplication for one UserAction request.
pub(super) struct UserActionPersistenceIdentity {
    pub(super) source_method: MethodName,
    pub(super) source_idempotency_key: String,
    pub(super) metadata_json: String,
}

impl UserActionOrigin {
    fn source_method(&self) -> MethodName {
        match self {
            Self::DirectRequest => MethodName::RequestUserAction,
            Self::Reconciliation { .. } => MethodName::ReconcileChanges,
        }
    }

    fn metadata_json(&self) -> CoreResult<String> {
        #[derive(Serialize)]
        #[serde(deny_unknown_fields)]
        struct EmptyMetadata {}

        #[derive(Serialize)]
        #[serde(deny_unknown_fields)]
        struct ReconciliationMetadata<'a> {
            created_by: &'a str,
            unrecorded_change_id: &'a UnrecordedChangeId,
        }

        match self {
            Self::DirectRequest => serde_json::to_string(&EmptyMetadata {}).map_err(Into::into),
            Self::Reconciliation {
                unrecorded_change_id,
            } => serde_json::to_string(&ReconciliationMetadata {
                created_by: MethodName::ReconcileChanges.as_str(),
                unrecorded_change_id,
            })
            .map_err(Into::into),
        }
    }

    pub(super) fn persistence_identity(
        &self,
        source_idempotency_key: &IdempotencyKey,
    ) -> CoreResult<UserActionPersistenceIdentity> {
        Ok(UserActionPersistenceIdentity {
            source_method: self.source_method(),
            source_idempotency_key: source_idempotency_key.as_str().to_owned(),
            metadata_json: self.metadata_json()?,
        })
    }
}

/// Allocates one stable request identity against a focused existence query.
pub(super) fn allocate_user_action_request_id(
    service: &CoreService,
    mut exists: impl FnMut(&str) -> CoreResult<bool>,
) -> CoreResult<UserActionRequestId> {
    service
        .allocate_generated_id(DurableIdKind::UserActionRequest, |candidate| {
            exists(candidate)
        })
        .map(UserActionRequestId::new)
}
