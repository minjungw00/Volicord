use serde_json::json;
use volicord_core::validate_host_verification_receipt;
use volicord_types::{
    CurrentHostReceiptContext, FailureCategory, HostVerificationReceipt, PlatformEnvironment,
    PlatformReleaseCoordinate, UtcTimestamp,
};

const RAW_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const RAW_B: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const PREFIXED_A: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const PREFIXED_B: &str = "sha256:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

fn receipt() -> HostVerificationReceipt {
    serde_json::from_value(json!({
        "contract_id": "volicord.host-verification-receipt",
        "project_id": "project-a",
        "connection_id": "connection-a",
        "host_kind": "codex",
        "integration_profile": "record",
        "platform_environment": "linux",
        "platform_release_coordinate": { "kind": "native" },
        "required_capabilities": [
            "managed_stdio_mcp",
            "personal_managed_binding",
            "record_workflow",
            "shared_managed_binding"
        ],
        "verified_capabilities": [
            "managed_stdio_mcp",
            "personal_managed_binding",
            "record_workflow",
            "shared_managed_binding"
        ],
        "binding_digest": PREFIXED_A,
        "generated_artifacts_digest": PREFIXED_A,
        "executable_digest": RAW_A,
        "policy_digest": PREFIXED_A,
        "verifier_build_digest": RAW_A,
        "observed_at": "2026-07-17T01:00:00Z",
        "expires_at": "2026-07-17T01:05:00Z",
        "result": "verified"
    }))
    .unwrap()
}

fn current(receipt: &HostVerificationReceipt) -> CurrentHostReceiptContext {
    CurrentHostReceiptContext {
        project_id: receipt.project_id.clone(),
        connection_id: receipt.connection_id.clone(),
        host_kind: receipt.host_kind,
        integration_profile: receipt.integration_profile,
        platform_environment: receipt.platform_environment,
        platform_release_coordinate: receipt.platform_release_coordinate.clone(),
        required_capabilities: receipt.required_capabilities.clone(),
        binding_digest: receipt.binding_digest.clone(),
        generated_artifacts_digest: receipt.generated_artifacts_digest.clone(),
        executable_digest: receipt.executable_digest.clone(),
        policy_digest: receipt.policy_digest.clone(),
        verifier_build_digest: receipt.verifier_build_digest.clone(),
    }
}

fn timestamp(value: &str) -> UtcTimestamp {
    UtcTimestamp::parse(value).unwrap()
}

#[test]
fn exact_current_receipt_is_valid_through_the_half_open_time_window() {
    let receipt = receipt();
    let current = current(&receipt);

    for now in ["2026-07-17T01:00:00Z", "2026-07-17T01:04:59Z"] {
        let validated =
            validate_host_verification_receipt(receipt.clone(), &current, &timestamp(now))
                .expect("matching receipt must validate inside its freshness window");
        assert_eq!(validated.receipt(), &receipt);
    }
}

#[test]
fn receipt_freshness_rejects_future_and_expired_observations() {
    let receipt = receipt();
    let current = current(&receipt);
    let future = validate_host_verification_receipt(
        receipt.clone(),
        &current,
        &timestamp("2026-07-17T00:59:59Z"),
    )
    .unwrap_err();
    assert_eq!(future.category(), FailureCategory::Rejected);
    assert_eq!(future.reason(), "host_receipt_not_yet_current");

    for now in ["2026-07-17T01:05:00Z", "2026-07-17T02:00:00Z"] {
        let expired =
            validate_host_verification_receipt(receipt.clone(), &current, &timestamp(now))
                .unwrap_err();
        assert_eq!(expired.category(), FailureCategory::Rejected);
        assert_eq!(expired.reason(), "host_receipt_expired");
    }
}

#[test]
fn receipt_is_bound_to_current_project_connection_platform_and_capabilities() {
    let receipt = receipt();
    let now = timestamp("2026-07-17T01:02:00Z");

    let mut context = current(&receipt);
    context.project_id = "project-b".into();
    assert_eq!(
        validate_host_verification_receipt(receipt.clone(), &context, &now)
            .unwrap_err()
            .reason(),
        "host_receipt_project_mismatch"
    );

    let mut context = current(&receipt);
    context.connection_id = "connection-b".into();
    assert_eq!(
        validate_host_verification_receipt(receipt.clone(), &context, &now)
            .unwrap_err()
            .reason(),
        "host_receipt_connection_mismatch"
    );

    let mut context = current(&receipt);
    context.platform_environment = PlatformEnvironment::Wsl2;
    context.platform_release_coordinate = PlatformReleaseCoordinate::first_release_wsl2();
    assert_eq!(
        validate_host_verification_receipt(receipt.clone(), &context, &now)
            .unwrap_err()
            .reason(),
        "host_receipt_platform_mismatch"
    );

    let mut context = current(&receipt);
    context.platform_release_coordinate = PlatformReleaseCoordinate::first_release_wsl2();
    assert_eq!(
        validate_host_verification_receipt(receipt.clone(), &context, &now)
            .unwrap_err()
            .reason(),
        "current_host_receipt_context_corrupt"
    );

    let mut context = current(&receipt);
    context.required_capabilities.pop();
    let corrupt = validate_host_verification_receipt(receipt, &context, &now)
        .expect_err("corrupt current Store context must fail closed");
    assert_eq!(corrupt.category(), FailureCategory::Corrupt);
    assert_eq!(corrupt.reason(), "current_host_receipt_context_corrupt");
}

#[test]
fn every_current_store_identity_change_makes_the_receipt_stale() {
    type ContextMutation = fn(&mut CurrentHostReceiptContext);

    let receipt = receipt();
    let now = timestamp("2026-07-17T01:02:00Z");

    let cases: [(&str, ContextMutation); 5] = [
        ("host_receipt_binding_stale", |context| {
            context.binding_digest = PREFIXED_B.to_owned()
        }),
        ("host_receipt_generated_artifacts_stale", |context| {
            context.generated_artifacts_digest = PREFIXED_B.to_owned()
        }),
        ("host_receipt_executable_stale", |context| {
            context.executable_digest = RAW_B.to_owned()
        }),
        ("host_receipt_policy_stale", |context| {
            context.policy_digest = PREFIXED_B.to_owned()
        }),
        ("host_receipt_verifier_build_stale", |context| {
            context.verifier_build_digest = RAW_B.to_owned()
        }),
    ];

    for (expected_reason, mutate) in cases {
        let mut context = current(&receipt);
        mutate(&mut context);
        let error = validate_host_verification_receipt(receipt.clone(), &context, &now)
            .expect_err("mismatched receipt coordinate must be rejected");
        assert_eq!(error.category(), FailureCategory::Rejected);
        assert_eq!(error.reason(), expected_reason);
    }
}

#[test]
fn invalid_typed_receipt_and_corrupt_current_context_remain_distinct() {
    let mut invalid_receipt = receipt();
    let expected = current(&invalid_receipt);
    invalid_receipt.contract_id = "wrong".to_owned();
    let rejected = validate_host_verification_receipt(
        invalid_receipt,
        &expected,
        &timestamp("2026-07-17T01:02:00Z"),
    )
    .unwrap_err();
    assert_eq!(rejected.category(), FailureCategory::Rejected);
    assert_eq!(rejected.reason(), "host_receipt_invalid");

    let valid_receipt = receipt();
    let mut corrupt_context = current(&valid_receipt);
    corrupt_context.policy_digest = "not-a-digest".to_owned();
    let corrupt = validate_host_verification_receipt(
        valid_receipt,
        &corrupt_context,
        &timestamp("2026-07-17T01:02:00Z"),
    )
    .unwrap_err();
    assert_eq!(corrupt.category(), FailureCategory::Corrupt);
    assert_eq!(corrupt.reason(), "current_host_receipt_context_corrupt");
}
