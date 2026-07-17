//! Core validation for typed managed-host verification evidence.

use std::{error::Error, fmt};

use volicord_types::{
    CurrentHostReceiptContext, FailureCategory, HostVerificationReceipt, UtcTimestamp,
};

/// Receipt that Core has compared with one complete current Store context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedHostVerificationReceipt {
    receipt: HostVerificationReceipt,
}

impl ValidatedHostVerificationReceipt {
    /// Returns the immutable receipt that passed every current-context check.
    pub const fn receipt(&self) -> &HostVerificationReceipt {
        &self.receipt
    }

    /// Consumes the validation wrapper and returns the immutable receipt.
    pub fn into_receipt(self) -> HostVerificationReceipt {
        self.receipt
    }
}

/// Machine-readable failure from Core receipt validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostReceiptValidationError {
    category: FailureCategory,
    reason: &'static str,
}

impl HostReceiptValidationError {
    const fn new(category: FailureCategory, reason: &'static str) -> Self {
        Self { category, reason }
    }

    /// Returns the product-wide failure category.
    pub const fn category(self) -> FailureCategory {
        self.category
    }

    /// Returns the stable domain reason.
    pub const fn reason(self) -> &'static str {
        self.reason
    }
}

impl fmt::Display for HostReceiptValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason)
    }
}

impl Error for HostReceiptValidationError {}

/// Validates a typed receipt against complete current Store and adapter facts.
///
/// Core performs identity and freshness comparisons only. Host configuration,
/// filesystem inspection, process inspection, and artifact discovery remain
/// outside this boundary.
pub fn validate_host_verification_receipt(
    receipt: HostVerificationReceipt,
    current: &CurrentHostReceiptContext,
    current_time: &UtcTimestamp,
) -> Result<ValidatedHostVerificationReceipt, HostReceiptValidationError> {
    receipt.validate_shape().map_err(|_| {
        HostReceiptValidationError::new(FailureCategory::Rejected, "host_receipt_invalid")
    })?;
    current.validate().map_err(|_| {
        HostReceiptValidationError::new(
            FailureCategory::Corrupt,
            "current_host_receipt_context_corrupt",
        )
    })?;

    require(
        receipt.project_id == current.project_id,
        "host_receipt_project_mismatch",
    )?;
    require(
        receipt.connection_id == current.connection_id,
        "host_receipt_connection_mismatch",
    )?;
    require(
        receipt.host_kind == current.host_kind,
        "host_receipt_kind_mismatch",
    )?;
    require(
        receipt.integration_profile == current.integration_profile,
        "host_receipt_profile_mismatch",
    )?;
    require(
        receipt.platform_environment == current.platform_environment,
        "host_receipt_platform_mismatch",
    )?;
    require(
        receipt.platform_release_coordinate == current.platform_release_coordinate,
        "host_receipt_platform_release_coordinate_mismatch",
    )?;
    require(
        receipt.required_capabilities == current.required_capabilities
            && receipt.verified_capabilities == current.required_capabilities,
        "host_receipt_capabilities_mismatch",
    )?;
    require(
        receipt.binding_digest == current.binding_digest,
        "host_receipt_binding_stale",
    )?;
    require(
        receipt.generated_artifacts_digest == current.generated_artifacts_digest,
        "host_receipt_generated_artifacts_stale",
    )?;
    require(
        receipt.executable_digest == current.executable_digest,
        "host_receipt_executable_stale",
    )?;
    require(
        receipt.policy_digest == current.policy_digest,
        "host_receipt_policy_stale",
    )?;
    require(
        receipt.verifier_build_digest == current.verifier_build_digest,
        "host_receipt_verifier_build_stale",
    )?;
    require(
        receipt.observed_at <= *current_time,
        "host_receipt_not_yet_current",
    )?;
    require(*current_time < receipt.expires_at, "host_receipt_expired")?;

    Ok(ValidatedHostVerificationReceipt { receipt })
}

fn require(condition: bool, reason: &'static str) -> Result<(), HostReceiptValidationError> {
    if condition {
        Ok(())
    } else {
        Err(HostReceiptValidationError::new(
            FailureCategory::Rejected,
            reason,
        ))
    }
}
