use crate::identity::UserActionOrigin;
use volicord_types::ids::{IdempotencyKey, UnrecordedChangeId};
use volicord_types::schema::PersistedUserActionRequestMetadata;
use volicord_types::values::MethodName;

#[test]
fn direct_request_identity_is_stable_and_empty_of_origin_metadata() {
    let origin = UserActionOrigin::DirectRequest;
    let first = origin
        .persistence_identity(&IdempotencyKey::new("idem-direct"))
        .expect("identity must construct");
    let second = origin
        .persistence_identity(&IdempotencyKey::new("idem-direct"))
        .expect("identity must construct deterministically");

    assert_eq!(first.source_method, MethodName::RequestUserAction);
    assert_eq!(first.source_idempotency_key, "idem-direct");
    assert!(matches!(
        first.metadata,
        PersistedUserActionRequestMetadata::DirectRequest(_)
    ));
    assert_eq!(first.source_method, second.source_method);
    assert_eq!(first.source_idempotency_key, second.source_idempotency_key);
    assert_eq!(first.metadata, second.metadata);
}

#[test]
fn reconciliation_identity_preserves_the_deduplication_origin() {
    let identity = UserActionOrigin::Reconciliation {
        unrecorded_change_id: UnrecordedChangeId::new("change-observed"),
    }
    .persistence_identity(&IdempotencyKey::new("idem-reconcile"))
    .expect("identity must construct");

    assert_eq!(identity.source_method, MethodName::ReconcileChanges);
    assert_eq!(identity.source_idempotency_key, "idem-reconcile");
    let PersistedUserActionRequestMetadata::Reconciliation(metadata) = identity.metadata else {
        panic!("reconciliation identity must preserve reconciliation metadata")
    };
    assert_eq!(metadata.created_by, MethodName::ReconcileChanges);
    assert_eq!(metadata.unrecorded_change_id.as_str(), "change-observed");
}
