use crate::error::UserActionServiceError;
use volicord_types::ids::{IdempotencyKey, UnrecordedChangeId};
use volicord_types::schema::{
    PersistedUserActionDirectRequestMetadata, PersistedUserActionReconciliationMetadata,
    PersistedUserActionRequestMetadata,
};
use volicord_types::values::MethodName;

/// Current Core operation that owns a newly constructed UserAction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserActionOrigin {
    DirectRequest,
    Reconciliation {
        unrecorded_change_id: UnrecordedChangeId,
    },
}

/// Stable source identity used by Store deduplication for one UserAction request.
pub(super) struct UserActionPersistenceIdentity {
    pub(super) source_method: MethodName,
    pub(super) source_idempotency_key: String,
    pub(super) metadata: PersistedUserActionRequestMetadata,
}

impl UserActionOrigin {
    fn source_method(&self) -> MethodName {
        match self {
            Self::DirectRequest => MethodName::RequestUserAction,
            Self::Reconciliation { .. } => MethodName::ReconcileChanges,
        }
    }

    fn metadata(&self) -> PersistedUserActionRequestMetadata {
        match self {
            Self::DirectRequest => PersistedUserActionRequestMetadata::DirectRequest(
                PersistedUserActionDirectRequestMetadata {},
            ),
            Self::Reconciliation {
                unrecorded_change_id,
            } => PersistedUserActionRequestMetadata::Reconciliation(
                PersistedUserActionReconciliationMetadata {
                    created_by: MethodName::ReconcileChanges,
                    unrecorded_change_id: unrecorded_change_id.clone(),
                },
            ),
        }
    }

    pub(super) fn persistence_identity(
        &self,
        source_idempotency_key: &IdempotencyKey,
    ) -> Result<UserActionPersistenceIdentity, UserActionServiceError> {
        Ok(UserActionPersistenceIdentity {
            source_method: self.source_method(),
            source_idempotency_key: source_idempotency_key.as_str().to_owned(),
            metadata: self.metadata(),
        })
    }
}
