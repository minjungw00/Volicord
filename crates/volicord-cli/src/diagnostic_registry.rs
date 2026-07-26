//! Current machine-readable diagnostic code registry.

use std::collections::BTreeSet;

use volicord_platform_fs::PlatformDiagnosticKind;
use volicord_store::operational_diagnostics::{RuntimeHomeDiagnostic, StoreDiagnostic};
use volicord_types::guard_outcome::GuardHookDiagnosticCode;

use crate::operational_diagnostics::OperationalDiagnostic;

/// Returns the deterministic union of current typed diagnostic registries.
pub fn current_diagnostic_codes() -> BTreeSet<String> {
    let mut codes = OperationalDiagnostic::ALL
        .into_iter()
        .map(|diagnostic| diagnostic.code().to_owned())
        .collect::<BTreeSet<_>>();
    codes.extend(
        RuntimeHomeDiagnostic::ALL
            .into_iter()
            .map(|diagnostic| diagnostic.code().to_owned()),
    );
    codes.extend(
        StoreDiagnostic::ALL
            .into_iter()
            .map(|diagnostic| diagnostic.code().to_owned()),
    );
    codes.extend(
        PlatformDiagnosticKind::ALL
            .into_iter()
            .map(|diagnostic| diagnostic.code().to_owned()),
    );
    codes.extend(
        GuardHookDiagnosticCode::ALL
            .into_iter()
            .map(|diagnostic| diagnostic.as_str().to_owned()),
    );
    codes.extend(volicord_mcp::diagnostic_codes());
    codes.extend(crate::connection_command::current_connection_diagnostic_codes());
    codes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_owner_derived_and_deduplicated() {
        let codes = current_diagnostic_codes();

        assert!(codes.contains("guard.policy.denied"));
        assert!(codes.contains("mcp.protocol.unsupported_version"));
        assert!(codes.contains("process.initialize.timeout"));
        assert!(codes.contains("store.sqlite.busy"));
    }
}
