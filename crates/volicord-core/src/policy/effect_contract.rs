use std::{collections::BTreeSet, path::Path};

use volicord_types::schema::ChangeUnitEffectContract;
use volicord_types::values::ChangeUnitEffectKind;

use volicord_types::product_path::{normalize_product_paths, path_is_within, ProductPathError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectContractValidationError {
    ConflictingEffect(ChangeUnitEffectKind),
    EmptyText(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EffectContractViolation {
    FileWriteForbidden,
    FileWriteNotAllowed,
    PathNotAllowed,
}

pub(crate) fn validate_effect_contract(
    contract: &ChangeUnitEffectContract,
) -> Result<(), EffectContractValidationError> {
    for effect in &contract.allowed_effects {
        if contract.forbidden_effects.contains(effect) {
            return Err(EffectContractValidationError::ConflictingEffect(*effect));
        }
    }

    for (field, values) in [
        ("allowed_paths", &contract.allowed_paths),
        ("expected_outputs", &contract.expected_outputs),
        ("invariants", &contract.invariants),
        ("evidence_expectations", &contract.evidence_expectations),
        (
            "sensitive_action_expectations",
            &contract.sensitive_action_expectations,
        ),
    ] {
        if values.iter().any(|value| value.trim().is_empty()) {
            return Err(EffectContractValidationError::EmptyText(field));
        }
    }

    Ok(())
}

pub(crate) fn validate_effect_contract_paths(
    repo_root: &Path,
    contract: &ChangeUnitEffectContract,
) -> Result<(), ProductPathError> {
    if contract.allowed_paths.is_empty() {
        return Ok(());
    }
    normalize_product_paths(repo_root, &contract.allowed_paths).map(|_| ())
}

pub(crate) fn product_write_violations(
    repo_root: &Path,
    contract: &ChangeUnitEffectContract,
    product_file_write_intended: bool,
    intended_paths: &[String],
) -> Result<Vec<EffectContractViolation>, ProductPathError> {
    if !product_file_write_intended {
        return Ok(Vec::new());
    }

    let mut violations = Vec::new();
    if contract
        .forbidden_effects
        .contains(&ChangeUnitEffectKind::ProductFileWrite)
    {
        violations.push(EffectContractViolation::FileWriteForbidden);
    }
    if !contract.allowed_effects.is_empty()
        && !contract
            .allowed_effects
            .contains(&ChangeUnitEffectKind::ProductFileWrite)
    {
        violations.push(EffectContractViolation::FileWriteNotAllowed);
    }
    if !contract.allowed_paths.is_empty() && !intended_paths.is_empty() {
        let allowed_paths = normalize_product_paths(repo_root, &contract.allowed_paths)?;
        if intended_paths.iter().any(|path| {
            allowed_paths
                .iter()
                .all(|allowed_path| !path_is_within(path, allowed_path))
        }) {
            violations.push(EffectContractViolation::PathNotAllowed);
        }
    }

    Ok(dedupe_violations(violations))
}

fn dedupe_violations(violations: Vec<EffectContractViolation>) -> Vec<EffectContractViolation> {
    let mut seen = BTreeSet::new();
    violations
        .into_iter()
        .filter(|violation| seen.insert(*violation))
        .collect()
}
