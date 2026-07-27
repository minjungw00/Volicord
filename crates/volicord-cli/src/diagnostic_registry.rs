//! Current machine-readable diagnostic code registry.

use std::collections::{BTreeMap, BTreeSet};

use volicord_platform_fs::PlatformDiagnosticKind;
use volicord_store::operational_diagnostics::{RuntimeHomeDiagnostic, StoreDiagnostic};
use volicord_types::guard_outcome::GuardHookDiagnosticCode;

use crate::operational_diagnostics::OperationalDiagnostic;

/// One stable semantic diagnostic contract from the typed registry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DiagnosticContractDescriptor {
    pub id: String,
    pub codes: BTreeSet<String>,
    pub related_contracts: Vec<String>,
}

/// Returns exact diagnostic contracts grouped by their typed code namespace.
pub fn current_diagnostic_contract_descriptors() -> Vec<DiagnosticContractDescriptor> {
    let mut contracts = BTreeMap::<String, BTreeSet<String>>::new();
    for code in current_diagnostic_codes() {
        let namespace = code
            .split_once('.')
            .map_or(code.as_str(), |(namespace, _)| namespace);
        contracts
            .entry(format!("diagnostic.{namespace}"))
            .or_default()
            .insert(code);
    }
    contracts
        .into_iter()
        .map(|(id, codes)| DiagnosticContractDescriptor {
            id,
            codes,
            related_contracts: Vec::new(),
        })
        .collect()
}

fn current_diagnostic_codes() -> BTreeSet<String> {
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
        let descriptors = current_diagnostic_contract_descriptors();
        let codes = descriptors
            .iter()
            .flat_map(|descriptor| descriptor.codes.iter())
            .map(String::as_str)
            .collect::<BTreeSet<_>>();

        assert!(codes.contains("guard.policy.denied"));
        assert!(codes.contains("mcp.protocol.unsupported_version"));
        assert!(codes.contains("process.initialize.timeout"));
        assert!(codes.contains("store.sqlite.busy"));
        assert!(descriptors
            .iter()
            .any(|descriptor| descriptor.id == "diagnostic.platform"));
    }
}
