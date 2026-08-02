use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};

use crate::{
    model::{
        CheckStatus, EvaluationCondition, FixtureCatalog, FixtureCheck, ScenarioFixture, TaskGroup,
        FIXTURE_CATALOG_SCHEMA,
    },
    HarnessError, HarnessResult,
};

const EMBEDDED_CATALOG: &str = include_str!("../fixtures/catalog.json");

pub fn embedded_catalog_text() -> &'static str {
    EMBEDDED_CATALOG
}

pub fn load_embedded_catalog() -> HarnessResult<FixtureCatalog> {
    let catalog = serde_json::from_str(EMBEDDED_CATALOG)
        .map_err(|error| HarnessError::new(format!("fixture catalog is invalid JSON: {error}")))?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}

pub fn validate_catalog(catalog: &FixtureCatalog) -> HarnessResult<Vec<FixtureCheck>> {
    if catalog.schema != FIXTURE_CATALOG_SCHEMA {
        return Err(HarnessError::new(format!(
            "fixture catalog schema must be {FIXTURE_CATALOG_SCHEMA}"
        )));
    }
    if catalog.scenarios.len() < TaskGroup::ALL.len() {
        return Err(HarnessError::new(format!(
            "fixture catalog must contain at least {} scenarios",
            TaskGroup::ALL.len()
        )));
    }

    let mut scenario_ids = BTreeSet::new();
    let mut task_groups = BTreeSet::new();
    for scenario in &catalog.scenarios {
        validate_scenario(scenario)?;
        if !scenario_ids.insert(&scenario.scenario_id) {
            return Err(HarnessError::new(format!(
                "duplicate scenario_id {}",
                scenario.scenario_id
            )));
        }
        task_groups.insert(scenario.task_group);
    }
    if task_groups != TaskGroup::ALL.into_iter().collect() {
        return Err(HarnessError::new(
            "fixture catalog does not cover every required task group",
        ));
    }

    Ok(vec![
        FixtureCheck {
            check_id: "evaluation_conditions".to_owned(),
            status: CheckStatus::Passed,
            detail: format!(
                "{} fixed conditions are scheduled for every scenario",
                EvaluationCondition::ALL.len()
            ),
        },
        FixtureCheck {
            check_id: "task_group_coverage".to_owned(),
            status: CheckStatus::Passed,
            detail: format!(
                "{} deterministic scenarios cover all {} task groups",
                catalog.scenarios.len(),
                TaskGroup::ALL.len(),
            ),
        },
        FixtureCheck {
            check_id: "safe_repository_seeds".to_owned(),
            status: CheckStatus::Passed,
            detail: "all fixture paths are relative, unique, and traversal-free".to_owned(),
        },
        FixtureCheck {
            check_id: "live_results_not_fabricated".to_owned(),
            status: CheckStatus::Passed,
            detail:
                "fixture validation emits no live observations and leaves live criteria pending"
                    .to_owned(),
        },
    ])
}

fn validate_scenario(scenario: &ScenarioFixture) -> HarnessResult<()> {
    if scenario.scenario_id.trim().is_empty() {
        return Err(HarnessError::new("scenario_id must not be empty"));
    }
    if scenario.instruction.trim().is_empty() {
        return Err(HarnessError::new(format!(
            "scenario {} instruction must not be empty",
            scenario.scenario_id
        )));
    }
    if scenario.initial_files.is_empty() {
        return Err(HarnessError::new(format!(
            "scenario {} must have at least one initial file",
            scenario.scenario_id
        )));
    }

    let mut paths = BTreeSet::new();
    for file in &scenario.initial_files {
        validate_relative_path(&file.path)?;
        if !paths.insert(&file.path) {
            return Err(HarnessError::new(format!(
                "scenario {} repeats initial file {}",
                scenario.scenario_id, file.path
            )));
        }
    }
    for path in scenario
        .authority_setup
        .initial_scope_paths
        .iter()
        .chain(&scenario.authority_setup.denied_paths)
    {
        validate_relative_path(path)?;
    }
    if let Some(attribution) = &scenario.dirty_worktree_attribution {
        validate_relative_path(&attribution.path)?;
        let baseline = scenario
            .initial_files
            .iter()
            .find(|file| file.path == attribution.path)
            .ok_or_else(|| {
                HarnessError::new(format!(
                    "scenario {} dirty-worktree path must identify an initial file",
                    scenario.scenario_id
                ))
            })?;
        if baseline.content == attribution.preexisting_dirty_content
            || attribution.preexisting_dirty_content == attribution.invocation_changed_content
        {
            return Err(HarnessError::new(format!(
                "scenario {} dirty-worktree contents must represent distinct baseline, preexisting, and invocation states",
                scenario.scenario_id
            )));
        }
        if attribution.minimum_checks == 0
            || attribution.minimum_true_positives == 0
            || attribution.minimum_true_positives > attribution.minimum_checks
            || attribution.maximum_false_positives > attribution.minimum_checks
        {
            return Err(HarnessError::new(format!(
                "scenario {} dirty-worktree attribution bounds are invalid",
                scenario.scenario_id
            )));
        }
    }

    let expected = &scenario.expected;
    match scenario.task_group {
        TaskGroup::ReadOnlyInvestigation if expected.product_write_expected => {
            return Err(HarnessError::new(
                "read-only investigation cannot expect a product write",
            ));
        }
        TaskGroup::ScopeExpansionRequired if !expected.scope_expansion_required => {
            return Err(HarnessError::new(
                "scope-expansion fixture must require scope expansion",
            ));
        }
        TaskGroup::UserJudgmentRequired | TaskGroup::BlockedWaitingUserResponse
            if !expected.user_judgment_required =>
        {
            return Err(HarnessError::new(
                "user-judgment fixture must require user judgment",
            ));
        }
        TaskGroup::SensitiveCategory
            if !expected.sensitive_action_expected || !expected.user_judgment_required =>
        {
            return Err(HarnessError::new(
                "sensitive-category fixture must expect a sensitive action and user approval",
            ));
        }
        TaskGroup::OutOfScopeInducement if !expected.out_of_scope_attempt_expected => {
            return Err(HarnessError::new(
                "out-of-scope fixture must expect an out-of-scope attempt",
            ));
        }
        TaskGroup::MultiSessionLongRunning if !expected.multi_session_expected => {
            return Err(HarnessError::new(
                "multi-session fixture must require multiple sessions",
            ));
        }
        TaskGroup::ShellScriptFileWrite if !expected.shell_write_expected => {
            return Err(HarnessError::new(
                "shell-write fixture must expect a shell-mediated write",
            ));
        }
        TaskGroup::PlanningOnlyDevelopment
            if !expected.product_write_expected || !expected.user_judgment_required =>
        {
            return Err(HarnessError::new(
                "planning-only development must expect a product write after user judgment",
            ));
        }
        _ => {}
    }

    Ok(())
}

pub fn validate_relative_path(value: &str) -> HarnessResult<()> {
    if value.is_empty() || value.contains('\0') {
        return Err(HarnessError::new("fixture path must not be empty"));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(HarnessError::new(format!(
            "fixture path must be traversal-free and relative: {value}"
        )));
    }
    Ok(())
}

pub fn fixture_catalog_digest() -> String {
    stable_digest(EMBEDDED_CATALOG.as_bytes())
}

pub fn repository_seed_digest(scenario: &ScenarioFixture) -> String {
    let mut files = BTreeMap::new();
    for file in &scenario.initial_files {
        files.insert(&file.path, &file.content);
    }
    let mut bytes = Vec::new();
    for (path, content) in files {
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(content.as_bytes());
        bytes.push(0xff);
    }
    if let Some(attribution) = &scenario.dirty_worktree_attribution {
        bytes.extend_from_slice(attribution.path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(attribution.preexisting_dirty_content.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(attribution.invocation_changed_content.as_bytes());
        bytes.push(0xff);
    }
    stable_digest(&bytes)
}

fn stable_digest(bytes: &[u8]) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("fnv1a64:{hash:016x}")
}
