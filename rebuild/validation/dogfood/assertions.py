#!/usr/bin/env python3
"""Self-test the Phase 8 dogfood evaluation support boundary."""

from __future__ import annotations

import ast
import copy
import json
import re
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
HARNESS = HERE / "harness.py"
CAMPAIGN = HERE / "campaign.py"
CODEX_EVENTS = HERE / "codex_events.py"
DEFINITION = HERE / "evaluation.json"
MACHINE_FINDINGS = HERE / "machine_findings.py"
REVIEW_OPERATIONS = HERE / "review_operations.py"
QUALITATIVE_REVIEW = HERE / "qualitative_review.py"
QUALIFICATION_POLICY = HERE / "qualification_policy.py"
CURRENT_MCP_FIXTURE = HERE / "fixtures/current-codex-mcp-completion.jsonl"
HOST_MCP = ROOT / "rebuild/crates/volicord-host/src/mcp.rs"
OPERATIONS = ROOT / "rebuild/crates/volicord-operations/src/operations.rs"
REVIEWER_SAFE_CONTRACTS = (
    ROOT / "rebuild/docs/design/validation-plan.md",
    ROOT / "rebuild/docs/design/cutover-plan.md",
    ROOT / "rebuild/validation/README.md",
    ROOT / "rebuild/validation/phase-8-summary.md",
    ROOT / "rebuild/validation/dogfood/report.md",
)
PUBLIC_CAMPAIGN_CONTRACTS = (
    ROOT / "rebuild/docs/design/cutover-plan.md",
    ROOT / "rebuild/validation/README.md",
    ROOT / "rebuild/validation/phase-8-summary.md",
)
PUBLIC_CAMPAIGN_CONTRACT_START = "<!-- phase8-public-campaign-contract:start -->"
PUBLIC_CAMPAIGN_CONTRACT_END = "<!-- phase8-public-campaign-contract:end -->"
ACTIVE_OPERATIONS_START = "<!-- phase8-active-operations:start -->"
ACTIVE_OPERATIONS_END = "<!-- phase8-active-operations:end -->"
ACTIVE_OPERATION_CONTRACTS = (
    ROOT / "rebuild/validation/README.md",
    ROOT / "rebuild/validation/phase-8-summary.md",
    ROOT / "rebuild/validation/dogfood/report.md",
)


def require_semantic_clauses(
    value: object,
    contract_name: str,
    required_term_groups: tuple[tuple[str, ...], ...],
) -> None:
    """Require each semantic relationship in at least one maintained clause."""

    if not isinstance(value, list) or not all(
        isinstance(clause, str) and clause.strip() for clause in value
    ):
        raise AssertionError(f"{contract_name} must be a non-empty clause list")
    normalized = [clause.casefold() for clause in value]
    for terms in required_term_groups:
        if not any(all(term.casefold() in clause for term in terms) for clause in normalized):
            raise AssertionError(
                f"{contract_name} is missing semantic relationship: {', '.join(terms)}"
            )


def require_semantic_terms(
    value: object, contract_name: str, required_terms: tuple[str, ...]
) -> None:
    """Require semantic terms without freezing the complete contract wording."""

    if not isinstance(value, str) or not value.strip():
        raise AssertionError(f"{contract_name} must be a non-empty string")
    normalized = value.casefold()
    missing = [term for term in required_terms if term.casefold() not in normalized]
    if missing:
        raise AssertionError(
            f"{contract_name} is missing semantic terms: {', '.join(missing)}"
        )


def validate_behavior_specific_work_intake_contract(
    contract: object, behavior_classes: object
) -> None:
    """Check durable work-intake semantics without copying the definition subtree."""

    if not isinstance(contract, dict):
        raise AssertionError("Phase 8 behavior-specific work-intake contract is malformed")
    if not isinstance(behavior_classes, list) or not all(
        isinstance(value, str) for value in behavior_classes
    ):
        raise AssertionError("Phase 8 behavior classes are malformed")

    required_fields = {
        "explicit_user_owned_decision",
        "hidden_user_owned_decision_additional",
        "non_user_owned_classes",
        "materiality_correlation",
        "behavior_class_exact_disposition_oracle",
        "all_behavior_classes_require_inquiry",
    }
    missing_fields = required_fields - set(contract)
    if missing_fields:
        raise AssertionError(
            "Phase 8 behavior-specific work-intake contract is missing fields: "
            + ", ".join(sorted(missing_fields))
        )

    require_semantic_clauses(
        contract["explicit_user_owned_decision"],
        "explicit user-owned work intake",
        (
            ("pre-work", "unresolved", "user-owned", "Materiality Review"),
            (
                "ready-to-ask or fully researched",
                "material Question Candidate",
                "presented Question",
            ),
            ("exact current-host response", "current Question revision"),
            (
                "canonical Decision",
                "executable-scope-required Materiality revision",
                "typed inspect readiness",
                "before affected work",
            ),
        ),
    )
    require_semantic_clauses(
        contract["hidden_user_owned_decision_additional"],
        "hidden user-owned work intake",
        (
            (
                "successful meaningful repository investigation",
                "after baseline",
                "before Engineering Choice Discovery",
            ),
            (
                "repository research attachment",
                "ready-to-ask transition",
                "before Question promotion",
            ),
            (
                "Decision",
                "executable-scope-required Materiality revision",
                "typed inspect readiness",
                "before affected work",
            ),
        ),
    )

    non_user_contracts = contract["non_user_owned_classes"]
    if not isinstance(non_user_contracts, dict):
        raise AssertionError("Phase 8 non-user-owned work-intake contracts are malformed")
    expected_non_user_classes = set(behavior_classes) - {
        "explicit_user_owned_decision",
        "hidden_user_owned_decision",
    }
    if set(non_user_contracts) != expected_non_user_classes:
        raise AssertionError(
            "Phase 8 non-user-owned work-intake coverage does not match behavior classes"
        )

    require_semantic_terms(
        non_user_contracts["research_or_no_question"],
        "research/no-question work intake",
        (
            "repository/environment fact",
            "settled authority",
            "resolved research",
            "another maintained non-user-owned ready basis",
            "no manufactured Candidate, Question, or Decision",
        ),
    )
    require_semantic_terms(
        non_user_contracts["delegated_implementation_choice"],
        "delegated implementation work intake",
        ("current Goal delegation", "no manufactured Candidate, Question, or Decision"),
    )
    require_semantic_terms(
        non_user_contracts["exploratory_uncertainty"],
        "exploratory work intake",
        (
            "discovery-owned research or prototype requirements",
            "remain blocking",
            "same-review revision",
            "matching bounded evidence completion",
            "typed inspect readiness",
            "precedes affected work",
            "no manufactured Candidate, Question, or Decision",
        ),
    )
    require_semantic_terms(
        non_user_contracts["learning_deliberation"],
        "Learning Deliberation work intake",
        (
            "legal current-host Learning Deliberation state transitions",
            "valid reconsideration",
            "repeated response/feedback rounds",
            "typed executable-scope inspect readiness",
            "before affected work",
            "no canonical Decision",
        ),
    )
    require_semantic_terms(
        non_user_contracts["learning_routine_control"],
        "routine learning work intake",
        (
            "active participation",
            "routine value",
            "truthful maintained non-user-owned ready basis",
            "no Learning Deliberation, Candidate, Question, or Decision",
        ),
    )
    require_semantic_terms(
        contract["materiality_correlation"],
        "Materiality correlation",
        (
            "Goal Context",
            "baseline Analysis Snapshot",
            "Engineering Choice Discovery identity",
            "review_candidate_id",
            "dimension_id",
            "ordered review_revision",
        ),
    )
    if contract.get("initial_concern_is_rebuttable") is not True or contract.get("semantic_authority_requires_bounded_evidence_review") is not True:
        raise AssertionError("Phase 8 authority obligations require rebuttable evidence-backed semantic review")
    if contract["behavior_class_exact_disposition_oracle"] is not False:
        raise AssertionError("Phase 8 exact behavior-disposition oracle must remain disabled")
    if contract["all_behavior_classes_require_inquiry"] is not False:
        raise AssertionError("Phase 8 must not require Inquiry for every behavior class")


def assert_behavior_specific_work_intake_regressions(
    contract: dict[str, object], behavior_classes: list[str]
) -> None:
    """Prove current semantics reject each recently superseded oracle assumption."""

    def expect_rejected(name: str, mutation: object) -> None:
        candidate = copy.deepcopy(contract)
        if not callable(mutation):
            raise AssertionError(f"invalid behavior-contract regression mutation: {name}")
        mutation(candidate)
        try:
            validate_behavior_specific_work_intake_contract(candidate, behavior_classes)
        except AssertionError:
            return
        raise AssertionError(f"obsolete behavior-contract assumption passed: {name}")

    expect_rejected(
        "missing stable Materiality correlation",
        lambda value: value.pop("materiality_correlation"),
    )
    expect_rejected(
        "explicit Question must be ready without research",
        lambda value: value["explicit_user_owned_decision"].__setitem__(
            1, "ready-to-ask material Question Candidate and presented Question"
        ),
    )
    expect_rejected(
        "research/no-question requires one exact disposition",
        lambda value: value["non_user_owned_classes"].__setitem__(
            "research_or_no_question",
            "repository_or_environment_fact with no Candidate, Question, or Decision",
        ),
    )
    expect_rejected(
        "exploratory work must be ready at initial record",
        lambda value: value["non_user_owned_classes"].__setitem__(
            "exploratory_uncertainty",
            "evidence-backed exploratory disposition with no manufactured Candidate, Question, or Decision",
        ),
    )
    expect_rejected(
        "Learning Deliberation is one fixed response round",
        lambda value: value["non_user_owned_classes"].__setitem__(
            "learning_deliberation",
            "ordered current-host Learning Deliberation to terminal ready-for-work with no canonical Decision",
        ),
    )
    expect_rejected(
        "exact behavior-disposition oracle re-enabled",
        lambda value: value.__setitem__("behavior_class_exact_disposition_oracle", True),
    )
    expect_rejected(
        "initial concern used as an oracle",
        lambda value: value.__setitem__("initial_concern_is_rebuttable", False),
    )
    expect_rejected(
        "machine lifecycle fabricated semantic authority",
        lambda value: value.__setitem__("semantic_authority_requires_bounded_evidence_review", False),
    )
    expect_rejected(
        "Inquiry universally required",
        lambda value: value.__setitem__("all_behavior_classes_require_inquiry", True),
    )


def validate_evaluation_consumer_integration(
    harness_source: str, campaign_source: str
) -> None:
    """Keep direct consumers routed through the authoritative definition/helper."""

    for marker in (
        'DEFINITION = HERE / "evaluation.json"',
        'value = json.loads(DEFINITION.read_text(encoding="utf-8"))',
        'evidence.get("behavior_specific_work_intake_contract")',
    ):
        if marker not in harness_source:
            raise AssertionError(
                f"Dogfood harness no longer consumes evaluation.json through {marker}"
            )
    for marker in (
        "MATERIALITY_OBLIGATIONS = harness.MATERIALITY_OBLIGATIONS",
        "WORK_SLOTS_BY_REPOSITORY = harness.WORK_SLOTS_BY_REPOSITORY",
        "definition = harness.load_definition()",
    ):
        if marker not in campaign_source:
            raise AssertionError(
                f"Dogfood campaign no longer shares authoritative evaluation helpers through {marker}"
            )


def expected_public_campaign_contract(definition: dict[str, object]) -> dict[str, str]:
    topology = definition["campaign_topology"]
    real_session = definition["real_session_evidence"]
    profile = definition["qualification_profile_contract"]
    if not all(
        isinstance(value, dict)
        for value in (topology, real_session, profile)
    ):
        raise AssertionError("Phase 8 public campaign definition is malformed")
    batch = real_session["batch_campaign_contract"]
    if not isinstance(batch, dict):
        raise AssertionError("Phase 8 public batch definition is malformed")
    work_slots = ", ".join(
        f"{repository_class}={'/'.join(labels)}"
        for repository_class, labels in topology["work_slots_by_repository"].items()
    )
    return {
        "repository_journeys": str(topology["journey_count"]),
        "work_items": str(topology["work_count"]),
        "work_slots_by_repository": work_slots,
        "fresh_resume_pairs": str(topology["resume_pair_count"]),
        "fresh_sessions": str(real_session["full_replacement_session_count"]),
        "provisional_reviews_before_reveal": str(
            profile["reveal_requires_provisional_count"]
        ),
        "sealed_descriptors_and_reviews": str(topology["work_count"]),
        "complete_batch_raw_rollouts": str(batch["required_raw_rollout_count"]),
    }


def validate_public_campaign_contract(
    text: str, source: str, expected: dict[str, str]
) -> None:
    blocks = re.findall(
        re.escape(PUBLIC_CAMPAIGN_CONTRACT_START)
        + r"(.*?)"
        + re.escape(PUBLIC_CAMPAIGN_CONTRACT_END),
        text,
        flags=re.DOTALL,
    )
    if len(blocks) != 1:
        raise AssertionError(
            f"maintained public campaign contract must contain one bounded block: {source}"
        )
    actual: dict[str, str] = {}
    for match in re.finditer(
        r"^\|\s*`([^`]+)`\s*\|\s*`([^`]*)`\s*\|\s*$",
        blocks[0],
        flags=re.MULTILINE,
    ):
        key, value = match.groups()
        if key in actual:
            raise AssertionError(
                f"maintained public campaign contract repeats {key}: {source}"
            )
        actual[key] = value
    if actual != expected:
        mismatches = {
            key: {"expected": expected.get(key), "actual": actual.get(key)}
            for key in sorted(set(expected) | set(actual))
            if expected.get(key) != actual.get(key)
        }
        raise AssertionError(
            f"maintained public campaign contract drifted from evaluation.json: "
            f"{source}: {mismatches}"
        )


def render_public_campaign_contract(values: dict[str, str]) -> str:
    rows = "\n".join(f"| `{key}` | `{value}` |" for key, value in values.items())
    return (
        f"{PUBLIC_CAMPAIGN_CONTRACT_START}\n"
        "| Public campaign field | Current requirement |\n"
        "| --- | --- |\n"
        f"{rows}\n"
        f"{PUBLIC_CAMPAIGN_CONTRACT_END}\n"
    )


def assert_public_campaign_contract_regressions(expected: dict[str, str]) -> None:
    stale = dict(expected)
    stale.update(
        {
            "repository_journeys": "4",
            "work_items": "6",
            "fresh_sessions": "9",
            "provisional_reviews_before_reveal": "6",
            "sealed_descriptors_and_reviews": "6",
            "complete_batch_raw_rollouts": "9",
        }
    )
    try:
        validate_public_campaign_contract(
            render_public_campaign_contract(stale),
            "deliberately-stale-6-12-sample",
            expected,
        )
    except AssertionError as error:
        if "drifted from evaluation.json" not in str(error):
            raise
    else:
        raise AssertionError("deliberately stale 6/12 public campaign sample passed")
    historical_context = (
        "Historical note: a superseded campaign used six cycles and twelve sessions.\n\n"
        + render_public_campaign_contract(expected)
    )
    validate_public_campaign_contract(
        historical_context, "historical-prose-control", expected
    )


def active_operations(text: str, source: str) -> str:
    if text.count(ACTIVE_OPERATIONS_START) != 1 or text.count(ACTIVE_OPERATIONS_END) != 1:
        raise AssertionError(f"{source} must contain one active-operation boundary")
    start = text.index(ACTIVE_OPERATIONS_START) + len(ACTIVE_OPERATIONS_START)
    end = text.index(ACTIVE_OPERATIONS_END, start)
    return text[start:end]


def validate_active_operations(text: str, source: str) -> None:
    active = active_operations(text, source)
    retired = {
        "seal-cycle": r"\bseal-cycle\b",
        "activate-cycle": r"\bactivate-cycle\b",
        "eight provisional reviews": r"\b(?:all\s+)?(?:eight|8)\s+provisional\s+reviews\b",
        "provisional_count = 8": r"provisional_count\s*=\s*8",
        "sixteen raw rollouts": r"\b(?:all\s+)?(?:sixteen|16)\s+(?:raw\s+)?rollouts\b",
        "sixteen collect-batch files": r"\bexactly\s+(?:sixteen|16)\s+files\b",
        "3/3/2 cycle distribution": r"\b3\s*/\s*3\s*/\s*2\b",
        "mandatory work+resume": r"\b(?:every|each|all)\s+Work.{0,80}\b(?:work\s*\+\s*resume|resume\s+session)\b",
    }
    for label, pattern in retired.items():
        if re.search(pattern, active, flags=re.IGNORECASE | re.DOTALL):
            raise AssertionError(f"{source} active operations contain retired {label}")
    required = (
        "five",
        "eight",
        "seal-work",
        "collect-batch",
        "current clean",
        "exact-candidate",
    )
    require_semantic_terms(active, f"{source} active operations", required)


def assert_active_operations_regressions() -> None:
    current = active_operations(
        ACTIVE_OPERATION_CONTRACTS[0].read_text(encoding="utf-8"), "current-control"
    )
    for retired in (
        "Run seal-cycle after all eight provisional reviews.",
        "Require provisional_count = 8.",
        "Pass exactly sixteen files to collect-batch.",
    ):
        mutated = f"{ACTIVE_OPERATIONS_START}\n{current}\n{retired}\n{ACTIVE_OPERATIONS_END}"
        try:
            validate_active_operations(mutated, "controlled-retired-mutation")
        except AssertionError as error:
            if "retired" not in str(error):
                raise
        else:
            raise AssertionError(f"active procedure accepted retired mutation: {retired}")
    historical = (
        "Historical evidence: seal-cycle used provisional_count = 8 and exactly sixteen files.\n"
        f"{ACTIVE_OPERATIONS_START}\n{current}\n{ACTIVE_OPERATIONS_END}"
    )
    validate_active_operations(historical, "historical-prose-control")


def validate_corrected_evidence_contracts(definition, campaign_source):
    state = definition.get("repository_state_attestation_contract", {})
    if (state.get("candidate_state") != "clean_and_exact_head_bound"
        or state.get("target_state") != "observed_committed_or_naturally_dirty_without_mutation"
        or state.get("target_git_mutation_allowed") is not False
        or state.get("dirty_state_is_product_failure") is not False
        or state.get("actor_attribution_or_verification_success_implied") is not False
        or state.get("analysis_or_checkpoint_replaced") is not False
        or state.get("publication_verification") != "reobserve_exact_state_before_and_during_atomic_publication"
        or set(state.get("artifacts", [])) != {"repository-state.json", "staged.patch", "unstaged.patch"}):
        raise AssertionError("naturalistic repository-state attestation boundary changed")
    reconciliation = definition.get("reconciliation_contract", {})
    if (reconciliation.get("external_descriptor_input_allowed") is not False
        or reconciliation.get("seal_requires_exact_validated_draft") is not True
        or reconciliation.get("draft_inventory_bound") is not False
        or reconciliation.get("validation_inventory_bound_after_seal") is not True
        or reconciliation.get("visibility") != "steward_private"
        or reconciliation.get("provisional_bytes_immutable") is not True):
        raise AssertionError("campaign-owned private reconciliation boundary changed")
    import harness
    contract = harness.provisional_review_contract()
    if (contract["protocol"]["sequence"] != ["blind_discovery", "independent_blind_critique",
            "blind_adjudication", "final_provisional", "reveal"]
        or contract["protocol"]["adjudication"]["automatic_union"] is not False
        or "adjudication" not in contract["required_fields"]):
        raise AssertionError("blind completeness protocol contract changed")
    if ("assessments" not in contract["required_fields"]
        or contract["assessments"]["maximum"] != 32
        or contract["assessments"]["summary_classification"] != "summary_only_never_independent_coverage"):
        raise AssertionError("independent multi-dimension blind coverage contract changed")
    import inspect, campaign
    for operation in ("validate_discovery", "record_discovery", "prepare_critique",
                      "validate_critique", "record_critique", "prepare_adjudication",
                      "validate_final_lineage"):
        if not hasattr(campaign, operation):
            raise AssertionError("blind completeness campaign operation missing")
    if tuple(inspect.signature(campaign.seal_work).parameters) != ("root", "kind", "work"):
        raise AssertionError("seal-work admits an external descriptor path")
    tree = ast.parse(campaign_source)
    chronology = next(node for node in tree.body if isinstance(node, ast.FunctionDef)
        and node.name == "verify_journey_revision_chronology")
    if "harness.git_clean" in ast.unparse(chronology):
        raise AssertionError("target journey naturalistic dirty state is rejected")
    for operation in ("collect_batch", "publish_batch"):
        node = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == operation)
        if "verify_final_repository_states_for_publication" not in ast.unparse(node):
            raise AssertionError("publication bypasses attested target state verification")


def assert_corrected_evidence_contract_mutations(definition, campaign_source):
    for owner, field, bad in (("repository_state_attestation_contract", "target_git_mutation_allowed", True),
        ("repository_state_attestation_contract", "dirty_state_is_product_failure", True),
        ("repository_state_attestation_contract", "candidate_state", "dirty_allowed"),
        ("reconciliation_contract", "external_descriptor_input_allowed", True),
        ("reconciliation_contract", "seal_requires_exact_validated_draft", False)):
        proposed = copy.deepcopy(definition)
        proposed[owner][field] = bad
        try:
            validate_corrected_evidence_contracts(proposed, campaign_source)
        except AssertionError:
            pass
        else:
            raise AssertionError(f"controlled invariant mutation passed: {owner}.{field}")
    weakened = campaign_source.replace("integrity_check(\"project_binding\", verify_final_repository_states_for_publication, stage)", "None")
    try:
        validate_corrected_evidence_contracts(definition, weakened)
    except AssertionError as error:
        if "publication bypasses" not in str(error):
            raise
    else:
        raise AssertionError("controlled publication verification removal passed")


def main() -> int:
    source = HARNESS.read_text(encoding="utf-8")
    campaign_source = CAMPAIGN.read_text(encoding="utf-8")
    machine_source = MACHINE_FINDINGS.read_text(encoding="utf-8")
    review_source = REVIEW_OPERATIONS.read_text(encoding="utf-8")
    qualitative_source = QUALITATIVE_REVIEW.read_text(encoding="utf-8")
    qualification_source = QUALIFICATION_POLICY.read_text(encoding="utf-8")
    event_source = CODEX_EVENTS.read_text(encoding="utf-8")
    host_source = HOST_MCP.read_text(encoding="utf-8")
    compact_host_source = re.sub(r"\s+", "", host_source)
    operations_source = OPERATIONS.read_text(encoding="utf-8")
    definition = DEFINITION.read_text(encoding="utf-8")
    definition_value = json.loads(definition)
    validate_corrected_evidence_contracts(definition_value, campaign_source)
    assert_corrected_evidence_contract_mutations(definition_value, campaign_source)
    for path in ACTIVE_OPERATION_CONTRACTS:
        validate_active_operations(path.read_text(encoding="utf-8"), str(path))
    assert_active_operations_regressions()
    validate_evaluation_consumer_integration(source, campaign_source)
    for stale_consumer, consumer_source in {
        "machine evaluation": machine_source,
        "review selection": review_source,
        "qualification": qualification_source,
    }.items():
        if '["cycles"]' in consumer_source:
            raise AssertionError(
                f"{stale_consumer} still consumes the predecessor cycle aggregate"
            )
    if "qualification_behavior_multiset" in definition_value:
        raise AssertionError("reviewer-safe evaluation definition exposes the behavior histogram")
    profile_contract = definition_value.get("qualification_profile_contract")
    if not isinstance(profile_contract, dict) or any(
        "behavior" in key or "hidden" in key or "repository" in key
        for key in profile_contract
    ):
        raise AssertionError("reviewer-safe evaluation definition exposes profile composition")
    old_profile_patterns = (
        r"one\s+explicit.{0,80}two\s+hidden",
        r"exactly\s+one\s+explicit",
        r"exactly\s+two\s+hidden",
        r"hidden\s+(?:assignments|cycles).{0,80}(?:span|across)\s+two\s+repository",
        r"hidden_user_owned_decision.{0,80}정확히\s*두\s*번",
        r"hidden\s*두\s*sample",
        r"나머지\s*behavior는\s*각각\s*한\s*번",
    )
    for path in REVIEWER_SAFE_CONTRACTS:
        text = path.read_text(encoding="utf-8")
        if "qualification_behavior_multiset" in text or any(
            re.search(pattern, text, flags=re.IGNORECASE | re.DOTALL)
            for pattern in old_profile_patterns
        ):
            raise AssertionError(
                f"reviewer-safe maintained contract exposes the realized behavior profile: {path}"
            )
    tree = ast.parse(source)
    for node in ast.walk(tree):
        if (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Name)
            and node.func.id == "unique_call"
            and len(node.args) >= 2
            and isinstance(node.args[1], ast.Constant)
            and node.args[1].value == "repository_analyze"
        ):
            raise AssertionError(
                "Dogfood qualification still assumes one successful repository analysis"
            )
    if "exactly one pre-work repository analysis baseline" in campaign_source:
        raise AssertionError("campaign resume inspection still requires one analysis call")
    imports = {
        alias.name
        for node in ast.walk(tree)
        if isinstance(node, ast.Import)
        for alias in node.names
    }
    if "requests" in imports or "psutil" in imports:
        raise AssertionError("Phase 8 harness unexpectedly added a process/network framework dependency")
    if "rebuild/scripts/validate final" in source:
        raise AssertionError("Phase 8 harness may not invoke direct final validation")
    if "rebuild/scripts/validate gate" in source:
        raise AssertionError("Phase 8 harness may not own the final gate")
    import qualification_policy
    if json.loads(DEFINITION.read_text())["qualification_policy"] != qualification_policy.contract():
        raise AssertionError("qualification policy definition drift")
    policy_source = (HERE / "qualification_policy.py").read_text()
    if "verify-validation-archive" not in policy_source or "verify_technical" not in policy_source:
        raise AssertionError("Phase 8 qualification must reuse the candidate technical gate")
    if "def run_evaluation(" in source or 'subparsers.add_parser("run")' in source:
        raise AssertionError("superseded technical/automated qualification path remains")
    if "real_session_evidence" not in source or "REAL_SESSION_CHECKS" not in source:
        raise AssertionError("Phase 8 harness no longer requires real-session evidence")
    for marker in (
        "load_codex_capture",
        "load_canonical_bundle",
        "custom_tool_call",
        "custom_tool_call_output",
        "patch_apply_end",
        "FileChange",
        "mcp__volicord",
        "mcp_tool_call_end",
        "context_record",
        "baseline_analysis_snapshot_id",
        "materiality_review",
        "engineering_choice_discovery",
        "learning_deliberation",
        "pre_write_materiality_work_authority",
        "submit_question_from_materiality",
        "resume_materiality_work_authority",
        "checkpoint_verifications",
        "current_host_user_turn",
        "work_user_task",
        "fresh_resume_user_task",
        "evaluation_basis",
        "behavior_class",
        "behavior_review",
        "blind_first_review_errors",
        "phase8_blind_review_preparation",
        "phase8_provisional_behavior_review",
        "classification_comparison",
        "fact_authority_agreement",
        "counterfactual_review",
        "fully_satisfies_without_user_owned_outcome",
        "unavoidable_user_owned_outcome",
        "research_or_no_question",
        "delegated_implementation_choice",
        "indexed_materiality_dimensions",
        "resolved_user_owned_dimensions_valid",
        "dimension_correlation",
        "explicit_delegation",
        "exploratory_uncertainty",
        "campaign_preparation_independent_reviewer",
        "terminal_checkpoint_call",
        "verified_state_continuation",
        "repository_scoped_activation_observed",
        "operator_environment_invalid",
        "project_resolve",
        "repository_bound_project_resolution",
        "inspect-work",
        "dogfood_work_observation",
        "naturalistic_prompt_integrity",
        "task_goal_basis",
        "inquiry_behavior_basis",
        "continuation_basis",
        "check-descriptors",
        "batch_campaign_contract",
    ):
        if marker not in source and marker not in event_source:
            raise AssertionError(f"Phase 8 content normalizer is missing {marker}")
    if "evaluate_work_authority(" in host_source:
        raise AssertionError("the MCP host fabricates work-authority policy")
    for marker in (
        "evaluate_work_authority(",
        "workflow_from_authority(",
        "workflow_for_review_candidate(",
        "workflow_for_question_candidate(",
        "workflow_for_work_basis(",
    ):
        if marker not in operations_source:
            raise AssertionError(
                f"production Operations no longer derives workflow through {marker}"
            )
    for delegation in (
        ".operations.record_materiality_review(",
        ".operations.workflow_for_review_candidate(",
        ".operations.workflow_for_question_candidate(",
        ".operations.workflow_for_work_basis(",
    ):
        if delegation not in compact_host_source:
            raise AssertionError(
                f"the MCP host no longer delegates workflow derivation through {delegation}"
            )
    if "verified_capture" in source or "first_repository_inspection_sequence" in source:
        raise AssertionError("the declaration-only real-session path remains active")
    for obsolete in (
        "def observation_template(",
        "def validate_observation_object(",
        "permitted_accessibility_observations",
        '"observation_schema"',
    ):
        if obsolete in source or obsolete in definition:
            raise AssertionError(f"obsolete per-cycle review compatibility remains: {obsolete}")
    if 'payload_type == "function_call"' in event_source or "eval(" in event_source:
        raise AssertionError("the obsolete decoder or JavaScript evaluation path remains active")
    if 'cli_version ==' in source or 'cli_version ==' in event_source:
        raise AssertionError("Phase 8 may not dispatch capture parsing by numeric Codex version")
    for obsolete in (
        "PHASE8_OBJECTIVE_PREFIX",
        "MAX_PHASE8_OBJECTIVE_BYTES",
        "phase8_objective_from_turns",
        "Phase8Objective",
        "normalized_resume_change_scope",
        "next_step_reserves_change",
    ):
        if obsolete in source or obsolete in event_source:
            raise AssertionError(f"obsolete scripted Phase 8 mechanism remains active: {obsolete}")
    if "--authorize-codex-transmission" in source:
        raise AssertionError("the superseded project-health-only Phase 8 assertion remains")
    if '"codex_transmission"' in definition or "project-health-six-real-repository-cycles" in definition:
        raise AssertionError("the superseded Phase 8 transmission contract remains")
    if "verify_repository_normalized_codex_rollout_and_canonical_bundle" not in definition:
        raise AssertionError("Phase 8 definition does not select content-normalized evidence")
    real_session = definition_value["real_session_evidence"]
    materiality_contract = real_session.get("materiality_review_contract")
    materiality_invariants = {
        "input_collection_field": "judgments",
        "judgment_identity_field": "choice_id",
        "array_order_is_authoritative": False,
        "mixed_dispositions_allowed": True,
        "engineering_choice_discovery_required": True,
        "interaction_review_axes": ["reference_basis", "composition_and_precedence", "multi_item_effects", "failure_and_recovery"],
        "interaction_review_requires_current_outcome_and_source_closure": True,
        "question_count_is_fixed": False,
        "all_unresolved_user_owned_dimensions_block_work": True,
        "resolved_user_owned_decision_correlation": "per_dimension_id",
        "late_scope_binding_can_certify_earlier_work": False,
        "write_chronology_basis": (
            "each repository path's first supported meaningful Codex file-change event"
        ),
        "event_authority_selection": (
            "latest valid matching production Materiality scope binding completed "
            "before the write event"
        ),
        "prospective_scope_expansion_supported": True,
        "unsupported_write_chronology": "indeterminate_non_passing",
        "learning_deliberation_is_canonical_decision": False,
    }
    if (
        not isinstance(materiality_contract, dict)
        or any(
            materiality_contract.get(key) != value
            for key, value in materiality_invariants.items()
        )
        or materiality_contract.get("executable_scope_fields")
        != ["paths", "components", "work_contexts"]
        or materiality_contract.get("coupled_artifact_review")
        != {
            "categories": [
                "implementation",
                "focused_tests",
                "public_or_internal_documentation",
                "changelog_or_release_notes",
                "schema_snapshot_or_generated_artifact",
                "other_repository_owned_artifact",
            ],
            "exactly_once": True,
            "included_paths_exactly_account_for_executable_paths": True,
            "no_coupled_artifact_requires_basis": True,
            "late_discovery_is_prospective_only": True,
            "new_material_outcome_requires_materiality_reevaluation": True,
            "materiality_closure_states": ["no_new_material_outcome", "new_material_outcome"],
            "no_new_outcome_binding": ["review_candidate_id", "review_revision", "engineering_choice_discovery_candidate_id", "source_ids", "exact_planned_scope_and_artifact_assessments"],
            "commitment_binding_states": ["reviewed_choice", "reviewed_interaction", "private_equivalent"],
            "unmapped_commitment_becomes_new_material_outcome": True,
            "new_outcome_revokes_scope_until_current_rediscovery": True,
            "repository_root_convenience_scope_allowed": False,
        }
        or materiality_contract.get("pre_work_readiness_sequence")
        != [
            "record_or_revise_returns_executable_scope_required",
            "inspect_binds_typed_executable_scope",
            "inspect_returns_ready_for_work",
        ]
    ):
        raise AssertionError("Phase 8 Materiality Review authority contract changed")
    if (
        real_session.get("session_roles_by_work")
        != {"resumed_work": ["start", "resume"], "non_resumed_work": ["start"]}
        or real_session.get("full_replacement_session_count") != 8
        or definition_value.get("campaign_topology")
        != {
            "journeys": ["volicord", "small-python", "polyglot-medium"],
            "work_slots_by_repository": {
                "volicord": ["A", "B", "C"],
                "small-python": ["A"],
                "polyglot-medium": ["A"],
            },
            "resume_work_slot_by_repository": {
                "volicord": "A", "small-python": "A", "polyglot-medium": "A"
            },
            "journey_count": 3,
            "work_count": 5,
            "resume_pair_count": 3,
            "session_count": 8,
        }
        or definition_value.get("qualification_profile_contract")
        != {
            "visibility": "evaluator_steward_private_until_all_provisionals_recorded",
            "reveal_requires_provisional_count": 5,
            "validation_phase": "post_reveal_before_sealing",
            "reviewer_safe_profile_disclosure": False,
        }
        or tuple(definition_value.get("materiality_obligations", [])) != (
            "explicit_user_owned_decision",
            "hidden_user_owned_decision",
            "research_or_no_question",
            "repository_or_environment_fact",
            "delegated_implementation_choice",
            "exploratory_uncertainty",
            "learning_deliberation",
            "learning_routine_control",
        )
        or len(definition_value.get("repository_classes", {})) != 3
    ):
        raise AssertionError("Phase 8 journey/Work/session topology changed")
    import machine_findings
    expected_works = {
        (repository_class, label)
        for repository_class, labels in definition_value["campaign_topology"]["work_slots_by_repository"].items()
        for label in labels
    }
    if (
        machine_findings.EXPECTED_WORKS != expected_works
        or machine_findings.EXPECTED_JOURNEYS
            != set(definition_value["campaign_topology"]["journeys"])
        or 'value.get("schema_version") != 3' not in machine_source
        or 'evaluation["works"]' not in review_source
        or 'evaluation["journeys"]' not in review_source
    ):
        raise AssertionError("journey-based machine/review consumer schema drifted")
    public_campaign_contract = expected_public_campaign_contract(definition_value)
    for path in PUBLIC_CAMPAIGN_CONTRACTS:
        validate_public_campaign_contract(
            path.read_text(encoding="utf-8"), str(path), public_campaign_contract
        )
    assert_public_campaign_contract_regressions(public_campaign_contract)
    if "def build_review_package(" in campaign_source or "prepare-human-review" in campaign_source:
        raise AssertionError("superseded campaign/human-only review publication path remains")
    small_rules = definition_value["repository_classes"]["small-python"]
    polyglot_rules = definition_value["repository_classes"]["polyglot-medium"]
    if (
        small_rules.get("minimum_files", 0) < 8
        or small_rules.get("production_source_files_required", 0) < 3
        or small_rules.get("test_files_required", 0) < 2
        or small_rules.get("configuration_required") is not True
        or small_rules.get("trivial_arithmetic_or_example_disallowed") is not True
        or polyglot_rules.get("component_boundary_required") is not True
        or polyglot_rules.get("cross_language_config_api_or_process_work_required") is not True
    ):
        raise AssertionError("Phase 8 repository suitability contracts are incomplete")
    resources = definition_value.get("resource_qualification", {})
    if (
        resources.get("supported_operating_system") != "Linux"
        or resources.get("peak_memory_mechanism")
        != "linux_procfs_process_tree_rss_sampling"
        or resources.get("repeated_resource_repetition_count", 0) < 3
        or resources.get("universal_product_ceiling_applied") is not False
        or resources.get("naturalistic_mcp_memory", {}).get(
            "harness_descendant_measurement_may_substitute") is not False
        or resources.get("naturalistic_mcp_memory", {}).get(
            "unsupported_state_is_explicit") is not True
        or resources.get("naturalistic_mcp_memory", {}).get(
            "unsupported_state_qualifies_as_measured_pass") is not False
    ):
        raise AssertionError("Phase 8 resource qualification definition is incomplete")
    accessibility = definition_value.get("accessibility_machine_contract", {})
    if (
        "meaningful_visible_text" not in accessibility.get("button_names", [])
        or "aria_labelledby" not in accessibility.get("visible_form_control_names", [])
        or accessibility.get("manual_observation_may_override_deterministic_failure")
        is not False
    ):
        raise AssertionError("Phase 8 accessible-name qualification definition is incomplete")
    transport_identity = definition_value.get("real_session_evidence", {}).get(
        "codex_user_turn_transport_identity", {}
    )
    user_turn_normalization = definition_value.get("real_session_evidence", {}).get(
        "codex_user_turn_normalization", {}
    )
    goal_provenance = definition_value.get("real_session_evidence", {}).get(
        "current_host_goal_provenance", {}
    )
    mcp_completion = definition_value.get("real_session_evidence", {}).get(
        "mcp_completion_contract", {}
    )
    behavior_specific_work_intake = definition_value.get(
        "real_session_evidence", {}
    ).get("behavior_specific_work_intake_contract", {})
    evidence_transport = definition_value.get("real_session_evidence", {}).get(
        "evidence_transport_attribution", {}
    )
    interaction_diagnostics = definition_value.get("real_session_evidence", {}).get(
        "interaction_cost_diagnostics", {}
    )
    if user_turn_normalization != {
        "accepted_representations": [
            "event_msg.user_message with active turn and client_id",
            "event_msg.item_completed with matching thread_id, outer turn_id, and UserMessage item id/client_id",
        ],
        "current_text_content": (
            "one or more ordered text segments concatenated without a separator"
        ),
        "unsupported_current_content": (
            "malformed, empty, oversized, non-text, or identity-ambiguous UserMessage fails closed"
        ),
        "deduplication_identity": ["turn_id", "client_id"],
        "current_item_identity_must_agree": True,
        "same_identity_same_text_count": 1,
        "same_identity_conflicting_text": "capture_rejected",
        "generic_response_item_role_user_is_user_turn": False,
        "numeric_cli_version_dispatch": False,
    }:
        raise AssertionError("Phase 8 Codex user-turn normalization contract changed")
    if transport_identity != {
        "captured_text_allowance": (
            "CRLF-to-LF normalization, removal of only terminal CR/LF characters, "
            "and uniquely reversible raw-only backslash insertion before exact "
            "Markdown-escapable ASCII punctuation"
        ),
        "markdown_escapable_ascii_punctuation": "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~",
        "comparison_direction": "frozen_descriptor_task_to_raw_captured_first_user_turn",
        "ambiguous_alignment": "rejected",
        "bounded_success_diagnostic": [
            "transport_equivalence_used",
            "ignored_escape_count",
            "ignored_escape_normalized_raw_utf8_offsets",
            "ignored_escape_offsets_truncated",
            "normalized_comparison_sha256",
        ],
        "descriptor_task_mutated": False,
        "raw_capture_mutated": False,
        "evidence_sha256_mutated": False,
        "other_whitespace_normalized": False,
    }:
        raise AssertionError("Phase 8 Codex user-turn transport identity contract changed")
    if (
        not isinstance(goal_provenance, dict)
        or goal_provenance.get("mcp_raw_host_turn_authentication") != "unavailable"
        or goal_provenance.get("context_record_user_turn_content")
        != "caller_supplied_not_host_authenticated"
        or goal_provenance.get("dogfood_raw_consistency")
        != "maintained_transport_equivalent_context_record_user_turn_to_captured_first_user_turn"
        or goal_provenance.get("semantic_statement_relation")
        != "verbatim_containment_without_raw_source_rewrite"
        or goal_provenance.get("bounded_decomposition_supported") is not True
        or goal_provenance.get("authoritative_goal_resolution")
        != (
            "Checkpoint and applicable Materiality identities through baseline to the "
            "exact canonical Goal Context and current-host Source"
        )
        or goal_provenance.get("unused_or_superseded_goal_records")
        != "bounded diagnostic only"
        or goal_provenance.get("non_goal_context_roles_may_share_first_turn") is not True
    ):
        raise AssertionError("Phase 8 current-host Goal provenance contract changed")
    if mcp_completion != {
        "accepted_representations": [
            "event_msg.mcp_tool_call_end",
            "event_msg.item_completed.McpToolCall",
        ],
        "server": "volicord",
        "success": (
            "legacy result.Ok.isError false with object structuredContent, or current "
            "completed status with isError false and either object structuredContent or "
            "one bounded serialized CallToolResult content envelope carrying the same object result"
        ),
        "failure": (
            "tool/application error, validation error, malformed/incomplete result, unsupported "
            "status, status/result mismatch, result.Err, or correlated wrapper mismatch cannot qualify"
        ),
        "semantic_identity": ["server", "turn_id", "item_or_call_id"],
        "deduplication": (
            "equivalent legacy/current evidence with common transport identity yields one semantic ToolCall"
        ),
        "identity_conflict": (
            "material argument, status, error, result, operation, or turn disagreement rejects the capture"
        ),
        "structured_result_sources": [
            "structuredContent",
            "one exact serialized CallToolResult text envelope containing one exact structured JSON text object",
        ],
        "dual_result_representation": (
            "equivalent objects are accepted and conflicting objects reject the capture"
        ),
        "failed_calls_retained_for_diagnostics": True,
        "numeric_cli_version_dispatch": False,
    }:
        raise AssertionError("Phase 8 Codex MCP completion normalization contract changed")
    if definition_value["real_session_evidence"].get("file_change_contract") != {
        "accepted_representations": [
            "event_msg.patch_apply_end",
            "event_msg.item_completed.FileChange",
        ],
        "semantic_identity": ["turn_id", "item_or_call_id"],
        "current_success": (
            "matching thread/turn identity, completed status, bounded typed changes, and string stdout/stderr"
        ),
        "path_scope": (
            "bounded repository-relative paths after exact cwd relativization; external absolute paths and generated paths do not become work observations"
        ),
        "deduplication": (
            "equivalent old/current representations with common transport identity yield one PathObservation"
        ),
        "identity_conflict": (
            "path, change type, body, or move-target disagreement rejects the capture"
        ),
        "malformed_current_success": "evidence_transport_indeterminate",
        "prose_or_command_filename_inference": False,
        "numeric_cli_version_dispatch": False,
    }:
        raise AssertionError("Phase 8 Codex FileChange normalization contract changed")
    if definition_value["real_session_evidence"].get(
        "question_lifecycle_stages"
    ) != [
        "candidate_created_from_materiality",
        "required_research_complete",
        "candidate_ready",
        "promoted",
        "current_revision_available",
        "inquiry_frontier_presented",
        "exact_presentation_receipt_bound",
        "current_host_response_matched",
        "decision_recorded",
        "post_decision_materiality_resolved_before_affected_work",
    ] or definition_value["real_session_evidence"].get(
        "question_stage_failure_attribution"
    ) != (
        "a missing required stage fails the overall lifecycle without erasing "
        "independently supported later response or Decision authenticity"
    ):
        raise AssertionError("Phase 8 Question lifecycle stage contract changed")
    command_forwarding = definition_value["real_session_evidence"].get(
        "command_forwarding_contract", {}
    )
    if (
        command_forwarding.get("additional_structured_metadata")
        != (
            "allowed when required fields remain valid and projected fields come "
            "from the same statically bound result"
        )
        or command_forwarding.get("execution_identity_conflict") != "rejected"
    ):
        raise AssertionError("Phase 8 terminal command evidence contract changed")
    behavior_classes = definition_value.get("materiality_obligations")
    validate_behavior_specific_work_intake_contract(
        behavior_specific_work_intake, behavior_classes
    )
    assert_behavior_specific_work_intake_regressions(
        behavior_specific_work_intake, behavior_classes
    )
    if evidence_transport != {
        "states": ["complete", "indeterminate"],
        "indeterminate_causes": [
            "malformed_mcp_completion",
            "malformed_file_change",
            "unsupported_mcp_completion_status",
            "mcp_completion_status_mismatch",
        ],
        "actual_tool_or_application_failure_is_transport_indeterminate": False,
        "required_operation_indeterminate_classification": "evidence_transport_failure",
        "required_operation_indeterminate_outcome": "evidence_failed",
        "actual_missing_required_operation_classification": "semantic_work_observation",
        "unknown_means_pass": False,
    }:
        raise AssertionError("Phase 8 evidence transport attribution contract changed")
    if interaction_diagnostics != {
        "fields": [
            "tool_call_count",
            "engineering_choice_discovery_call_count",
            "materiality_review_call_count",
            "unsuccessful_or_rejected_call_count",
            "ready_for_work_observed",
            "calls_before_ready_for_work",
        ],
        "work_and_resume_separate": True,
        "numeric_pass_threshold": None,
    }:
        raise AssertionError("Phase 8 interaction-cost diagnostic contract changed")
    for marker in (
        "control_has_accessible_name",
        "aria-labelledby",
        "aria-label",
        "hidden_control_count",
        "unlabeled_control_count",
        "LinuxProcessTreePeakRss",
        "linux_process_tree_procfs_unavailability",
        "repeated_resource_rehearsal",
        "rehearsal_destination_preexisting",
        "failed_document_export_created_unowned_destination",
        "unexplained_cumulative_growth_observed",
        "universal_product_ceiling_applied",
        "codex_user_turn_transport_identity_matches",
    ):
        if marker not in source:
            raise AssertionError(f"Phase 8 qualification support is missing {marker}")
    for forwarding_requirement in (
        "commands used only for incidental inspection",
        "numeric exit_code from the same captured command result",
        '"complete_result_evidence"',
        '"correlated_split_evidence"',
        '"output_only_outcome": "unknown"',
        '"uncorrelated_or_synthesized_status_outcome": "unknown"',
        "the first captured user turn matches the descriptor plain work_user_task under the directional fail-closed transport-equivalence contract",
        "does not disclose Recall",
        "resolves the repository-bound existing Project through project_resolve before Recall",
        "a fresh resume session invokes Recall after project_resolve",
        "record a typed Materiality Review bound to the exact Goal and pre-work Analysis Snapshot before the first affected ordinary write, then review every maintained coupled-artifact category and evaluate each path's first supported write against the latest valid executable-scope binding that existed before that write",
        "correlate every unresolved review dimension through its Question Candidate",
        "for change continuation, recompute Materiality Review/work authority from the fresh baseline before continued ordinary work",
        "event_msg.mcp_tool_call_end",
        "event_msg.item_completed",
        "McpToolCall",
        "permit one or more successful work Checkpoints",
        "latest terminal Checkpoint candidate after the last meaningful repository change",
        "change continuation produces a repository change correlated to the strongest available typed executable or descriptor scope",
        "verified-state continuation requires a recalled completed Checkpoint",
    ):
        if forwarding_requirement not in definition:
            raise AssertionError(
                f"Phase 8 definition is missing forwarding requirement {forwarding_requirement}"
            )
    if "MCP_WRAPPER" not in event_source or "normalize_mcp_completion" not in event_source:
        raise AssertionError("Phase 8 MCP completion normalization boundary is missing")
    if "correlated_split" not in event_source or "custom_correlated_command_result" not in event_source:
        raise AssertionError("Phase 8 correlated command-result normalization boundary is missing")
    if "parsed.tool_name.startswith" in event_source:
        raise AssertionError("custom wrapper output remains an MCP semantic source")
    for linkage in (
        "descriptor_plain_work_user_task",
        "first_work_session_user_task_turn_transport_identity_match",
        "evaluated_repository_revision",
        "context_record_exact_user_turn_source",
        "canonical_goal_identity_and_statement",
        "checkpoint_goal_context_identity",
        "fresh_session_recall_same_goal_identity_and_materially_consistent_statement",
    ):
        if linkage not in definition:
            raise AssertionError(f"Phase 8 definition is missing plain-task Goal linkage {linkage}")
    for basis_field in (
        "repository_facts",
        "accepted_contract_constraints",
        "delegated_boundaries",
        "possible_material_concerns",
        "consequences",
        "facts_not_for_user",
        "current_relevance",
    ):
        if basis_field not in definition:
            raise AssertionError(f"Phase 8 definition is missing evaluation-basis field {basis_field}")
    behavior_review = real_session.get("behavior_review", {})
    agreement = behavior_review.get("fact_authority_agreement", {})
    comparison = behavior_review.get("classification_comparison", {})
    blind_first = behavior_review.get("blind_first_review", {})
    counterfactual = behavior_review.get("material_user_owned_counterfactual_review", {})
    if (
        behavior_review.get("required_independent_review_fields")
        != [
            "status",
            "reviewer_role",
            "basis",
            "review_preparation",
            "provisional_review",
            "classification_comparison",
            "fact_authority_agreement",
            "counterfactual_review",
        ]
        or blind_first.get("evaluator_material_visible_before_provisional_fix") is not False
        or blind_first.get("logical_identity_visible_before_provisional_fix") is not False
        or blind_first.get("reviewer_order") != "opaque_review_slot_id"
        or blind_first.get("preparation_immutable_and_inventory_bound") is not True
        or blind_first.get("draft_artifact_kind")
        != "phase8_blind_discovery"
        or blind_first.get("draft_path")
        != "reviewer/drafts/<review_slot_id>.json"
        or blind_first.get("draft_ownership")
        != "reviewer_owned_mutable_work_product_before_recording"
        or blind_first.get("draft_mutable_before_recording") is not True
        or blind_first.get("draft_inventory_bound_before_recording") is not False
        or blind_first.get("recording_operation") != "record-provisional-review"
        or blind_first.get("recording_identity")
        != "candidate_and_opaque_review_slot"
        or blind_first.get("recording_transition")
        != "critique_recorded_to_provisional_recorded"
        or blind_first.get("protocol_sequence") != ["blind_discovery", "independent_blind_critique",
            "blind_adjudication", "final_provisional", "reveal"]
        or blind_first.get("discovery_recording_operation") != "record-discovery"
        or blind_first.get("critique_recording_operation") != "record-critique"
        or blind_first.get("adjudication_preparation_operation") != "prepare-adjudication"
        or blind_first.get("final_preflight_operation") != "validate-provisional-review"
        or blind_first.get("automatic_critic_union") is not False
        or blind_first.get("recording_success_exit_code") != 0
        or blind_first.get("recording_reads_evaluator_descriptor") is not False
        or blind_first.get("recording_compares_evaluator_classification_or_materiality")
        is not False
        or blind_first.get("recording_failure_atomic") is not True
        or blind_first.get("recording_preserves_exact_accepted_input_bytes") is not True
        or blind_first.get("recorded_destination")
        != "reviewer/provisional/<review_slot_id>.json"
        or blind_first.get("recorded_independent_of_later_draft_mutation") is not True
        or blind_first.get("sealed_provisional_immutable_and_inventory_bound") is not True
        or blind_first.get("all_provisionals_required_before_any_reveal") is not True
        or blind_first.get("qualification_profile_reveal_operation")
        != "reveal-qualification-profile"
        or blind_first.get("evaluator_reveal_operation") != "seal-work"
        or blind_first.get("required_provisional_count_before_reveal") != 5
        or blind_first.get("sealing_accepts_provisional_payload") is not False
        or blind_first.get("preparation_fields")
        != [
            "review_slot_id",
            "candidate_head",
            "repository_revision",
            "reviewer_repository_path",
            "work_user_task",
            "fresh_resume_user_task",
            "work_scope",
            "owner_document_locations",
            "provisional_review_contract",
            "preflight",
        ]
        or blind_first.get("reviewer_contract_path")
        != "reviewer/provisional-review-contract.json"
        or blind_first.get("reviewer_contract_integrity")
        != "sha256_bound_to_each_preparation"
        or blind_first.get("preflight_operation") != "validate-discovery"
        or blind_first.get("preflight_mutates_campaign") is not False
        or blind_first.get("preflight_validation_semantics")
        != "shared_with_record-discovery"
        or blind_first.get("preflight_reads_evaluator_or_steward_truth") is not False
        or blind_first.get("preflight_rejects_inventory_bound_campaign_artifact")
        is not True
        or agreement.get("sealing_blocked_status") != "unresolved_conflict"
        or set(agreement.get("accepted_statuses", []))
        != {"agreed", "resolved_from_evidence"}
        or comparison.get("sealing_blocked_status") != "unresolved_conflict"
        or set(comparison.get("accepted_statuses", []))
        != {"agreed", "resolved_from_evidence"}
        or comparison.get("mechanical_disagreement_fields")
        != [
            "applicability",
            "classification",
            "materiality_conclusion",
            "material_outcome_unavoidable",
            "operator_prompt_disclosure",
        ]
        or comparison.get("applicability_resolution")
        != "per_fixed_negative_scope_evaluator_correct_reviewer_correct_or_unresolved_conflict"
        or comparison.get("provisional_classification_basis")
        != "sorted_unique_immutable_positive_assessment_classifications_not_scalar_summary"
        or comparison.get("provisional_artifact_rewritten") is not False
        or counterfactual.get("applicability")
        != "required_for_material_user_owned_decision"
        or counterfactual.get("accepted_conclusion") != "unavoidable_user_owned_outcome"
        or counterfactual.get("rejecting_task_satisfaction")
        != "fully_satisfies_without_user_owned_outcome"
        or counterfactual.get("question_wording_prescribed") is not False
        or counterfactual.get("alternatives_prescribed") is not False
        or counterfactual.get("user_selection_prescribed") is not False
        or behavior_review.get("non_user_decision_counterfactual_applicability")
        != "not_required_for_behavior_class"
    ):
        raise AssertionError("Phase 8 independent counterfactual-review contract is incomplete")
    opaque_slots = real_session.get("opaque_slot_contract", {})
    if (
        opaque_slots.get("identity_generation")
        != "campaign_time_cryptographic_random_128_bit_token"
        or opaque_slots.get("derived_from_repository_or_work") is not False
        or opaque_slots.get("physical_workspace_layout")
        != "journeys/<journey_id>/{repository,runtime}"
        or opaque_slots.get("reviewer_workspace_layout")
        != "reviewer/workspaces/<review_slot_id>/repository"
        or opaque_slots.get("private_mapping_integrity")
        != "campaign_bound_sha256_and_evidence_inventory"
        or opaque_slots.get("numeric_compatibility_branch") is not False
    ):
        raise AssertionError("Phase 8 opaque review-slot contract is incomplete")
    for marker in (
        "secrets.token_hex(16)",
        "phase8_dogfood_opaque_slot_mapping",
        "opaque_review_slot_id",
        "reviewer/workspaces",
        'slot_artifact_path(root, "reviewer", "drafts"',
        "record-provisional-review",
        "validate-provisional-review",
        "provisional-review-contract.json",
        "provisional_recorded",
        "reveal-qualification-profile",
        "all five provisional reviews",
        "qualification_profile_state",
    ):
        if marker not in campaign_source:
            raise AssertionError(f"Dogfood campaign helper is missing opaque-slot boundary {marker}")
    for stale_public_identity in (
        'root / "cycles"',
        'f"{cycle_key(kind, cycle)}.json"',
        'f"## {kind} — cycle {cycle}',
        'f"# Generated document review: {kind} cycle {cycle}',
    ):
        if stale_public_identity in campaign_source:
            raise AssertionError(
                f"Dogfood reviewer/operator path retains fixed-cycle identity: {stale_public_identity}"
            )
    blocker_contract = real_session.get("work_observation_contract", {})
    if blocker_contract != {
        "subcommand": "inspect-work",
        "result_kind": "dogfood_work_observation",
        "observation_only": True,
        "campaign_complete": False,
        "replacement_pass_candidate": False,
        "phase_9_ready": False,
        "later_evidence_status": "not_run",
        "missing_activation_outcome": "operator_environment_invalid",
        "indeterminate_required_evidence_outcome": "review_required",
        "actual_missing_required_operation_outcome": "review_required",
        "mixed_failure_checks_preserved": True,
        "failure_attribution_domains": [
            "environment", "product_integration",
            "evidence",
            "behavior_contract",
            "validation_internal",
        ],
        "failure_attribution_basis_visibility": "bounded_evaluator_safe_identifier",
    }:
        raise AssertionError("Phase 8 failure-only work-blocker contract is incomplete")
    batch_contract = real_session.get("batch_campaign_contract", {})
    from machine_findings import Status, Disposition, BEHAVIOR_RULES
    machine = batch_contract.get("machine_evaluation", {})
    if (batch_contract.get("semantic_evaluation_during_collection") is not False
        or batch_contract.get("evidence_set_manifest") != "evidence-set.json"
        or machine.get("operation") != "evaluate"
        or machine.get("technical_aggregate_input") != "required_immutable_machine_evaluation_no_semantic_rerun"
        or machine.get("qualification_state") != "not_run"
        or machine.get("hard_integrity_review_override") is not False
        or set(machine.get("statuses", [])) != set(Status)
        or set(machine.get("dispositions", [])) != set(Disposition)):
        raise AssertionError("Phase 8 evidence/finding lifecycle is incomplete")
    import harness
    if not set(harness.REAL_SESSION_CHECKS) <= BEHAVIOR_RULES:
        raise AssertionError("machine check missing finite disposition policy")
    candidate_guard = batch_contract.get("candidate_mutation_guard", {})
    if (
        batch_contract.get("operation") != "collect-batch"
        or batch_contract.get("required_raw_rollout_count") != 8
        or batch_contract.get("global_mapping_precedes_campaign_mutation") is not True
        or batch_contract.get("terminal_work_failure_repaired_by_resume") is not False
        or candidate_guard
        != {
            "required_state": (
                "campaign candidate equals current HEAD and qualifying worktree is clean"
            ),
            "operations": [
                "prepare-review",
                "record-provisional-review",
                "reveal-qualification-profile",
                "prepare-reconciliation",
                "validate-reconciliation",
                "seal-work",
                "activate-journey",
                "activate-all",
                "collect-batch",
                "finalize-manifest",
            ],
            "rejection_precedes_mutation": True,
            "superseded_recovery_exception": False,
            "read_only_predecessor_inspection_allowed": True,
        }
        or "read_only_static_viewer_snapshot"
        not in batch_contract.get("automatic_journey_final_evidence", [])
    ):
        raise AssertionError("Phase 8 batch campaign contract is incomplete")
    qualitative_contract = definition_value.get("qualitative_review_contract", {})
    import review_operations
    if qualitative_contract.get("workflow") != review_operations.workflow_contract():
        raise AssertionError("qualitative reviewer workflow/privacy bounds drifted")
    import qualitative_review
    if (
        "live_viewer_criteria" in qualitative_contract
        or "long_lived_project" in qualitative_contract.get("common_criteria", {})
        or "long_lived_project_observation" in qualitative_source
        or qualitative_review.rubric(definition_value).get("criteria")
        != qualitative_contract.get("common_criteria")
        or "browser_input_and_paint_responsiveness"
        not in qualitative_contract.get("common_criteria", {}).get("live_viewer", [])
    ):
        raise AssertionError("qualitative criterion inventory drifted")
    from review_operations_self_test import run_workflow_tests
    run_workflow_tests()
    import authority_obligations
    if qualitative_contract.get("authority_obligation_contract") != authority_obligations.assessment_contract():
        raise AssertionError("maintained authority obligation schema drifted from the human-review consumer")
    required_authority_regressions = {
        "different_wording_same_authority_obligation", "stronger_repository_contract_disproves_concern",
        "unrelated_question_plus_silent_target_commitment", "prototype_defer_without_production_commitment", "trivial_question_ceremony",
        "interaction_unrelated_decisions", "interaction_silent_durability", "interaction_explicit_contract",
        "interaction_accepted_prior_outcome", "interaction_exact_durability_delegation", "interaction_different_question_decomposition",
        "interaction_late_resolution", "interaction_avoided", "interaction_deferred", "interaction_scratch_prototype",
        "qualitative_review_exposes_interaction_and_planned_commitment_identities",
    }
    behavior_criteria = qualitative_contract.get("interaction_behavior_criterion_contracts", {})
    material_grounding = qualitative_contract.get("material_completeness_grounding", {})
    if (
        qualitative_contract.get("artifact_kind") != "dogfood_qualitative_review"
        or qualitative_contract.get("machine_accessibility_may_be_overridden") is not False
        or qualitative_contract.get("sampling_algorithm")
        != "work_scoped_with_journey_final_projection_and_repository_class_cli_scope"
        or qualitative_contract.get("every_work_review_surfaces")
        != ["interaction", "authority"]
        or qualitative_contract.get("resumed_work_review_surfaces")
        != ["context_recovery"]
        or qualitative_contract.get("resumed_work_sample_count") != 3
        or qualitative_contract.get("journey_final_sample_count") != 3
        or qualitative_contract.get("journey_final_review_surfaces")
        != [
            "generated_documents",
            "viewer_snapshot",
            "viewer_navigation_machine",
            "repository_intelligence",
        ]
        or qualitative_contract.get("repository_class_review_surfaces") != ["cli_usability"]
        or qualitative_contract.get("cli_repository_classes") != list(harness.CLASSES)
        or qualitative_contract.get("cli_criteria_per_repository_class") != 7
        or qualitative_contract.get("cli_assessment_count") != 21
        or set(qualitative_contract.get("live_viewer_locales", [])) != {"en", "ko"}
        or set(qualitative_contract.get("interaction_behavior_criteria", []))
        != {
            "explicit_material_handling_quality",
            "hidden_material_discovery_quality",
            "unnecessary_interruption",
            "learning_fork_value",
            "learning_alternatives_and_tradeoffs",
            "pre_response_recommendation_anchoring",
            "post_response_feedback_quality",
            "learning_implementation_fidelity",
            "routine_detail_omission",
            "proportional_learning_cost",
        }
        or set(behavior_criteria) != set(qualitative_contract.get("interaction_behavior_criteria", []))
        or behavior_criteria.get("explicit_material_handling_quality", {}).get("applies_to")
        != ["explicit_user_owned_decision"]
        or behavior_criteria.get("hidden_material_discovery_quality", {}).get("applies_to")
        != ["hidden_user_owned_decision"]
        or set(behavior_criteria.get("unnecessary_interruption", {}).get("applies_to", []))
        != {
            "research_or_no_question",
            "delegated_implementation_choice",
            "exploratory_uncertainty",
            "learning_routine_control",
        }
        or any(
            contract.get("exact_evaluator_wording_required") is not False
            or contract.get("exact_evaluator_alternatives_required") is not False
            or contract.get("exact_expected_user_answer_required") is not False
            or contract.get("semantically_equivalent_decomposition_allowed") is not True
            or not isinstance(contract.get("review_prompt"), str)
            or not contract.get("review_prompt", "").strip()
            for criterion, contract in behavior_criteria.items()
        )
        or any(
            "independently material" not in behavior_criteria[criterion]["review_prompt"]
            for criterion in (
                "explicit_material_handling_quality",
                "hidden_material_discovery_quality",
            )
        )
        or material_grounding.get("available_only_after_naturalistic_execution") is not True
        or material_grounding.get("operator_task_or_work_resume_session_visibility") is not False
        or material_grounding.get("possible_material_concerns_are_exhaustive") is not False
    ):
        raise AssertionError("Phase 8 campaign-level human-review contract is incomplete")
    import qualification_policy
    if (
        definition_value.get("qualification_policy") != qualification_policy.contract()
        or qualification_policy.TOPOLOGY
        != {
            "repository_journeys": 3,
            "work_items": 5,
            "resume_pairs": 3,
            "fresh_sessions": 8,
            "work_distribution": {
                "volicord": 3,
                "small-python": 1,
                "polyglot-medium": 1,
            },
            "resume_repository_classes": [
                "polyglot-medium", "small-python", "volicord"
            ],
        }
        or "long_lived_project" in qualification_source
        or 'evaluation["works"]' not in qualification_source
        or 'evaluation["journeys"]' not in qualification_source
        or '"schema_version": 3' not in qualification_source
    ):
        raise AssertionError("journey-based replacement qualification contract drifted")
    if "rehearse_target(" in source:
        raise AssertionError("naturalistic qualification must reuse gate evidence without rerunning V11")
    fixture_source = CURRENT_MCP_FIXTURE.read_text(encoding="utf-8")
    for marker in ("text(JSON.stringify(x))", '"type":"mcp_tool_call_end"', '"server":"volicord"'):
        if marker not in fixture_source:
            raise AssertionError(f"current Codex MCP completion fixture is missing {marker}")
    result = subprocess.run(
        [sys.executable, "-B", str(HARNESS), "self-test"],
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        text=True,
    )
    # Preserve the nested result and check its actual completed regression IDs.
    print(result.stdout, end="")
    if result.returncode != 0:
        raise RuntimeError(f"Phase 8 harness self-test failed with exit {result.returncode}")
    completed = json.loads(result.stdout)
    if not required_authority_regressions <= completed.get('authority_obligation_regressions', {}).keys():
        raise AssertionError("required authority obligation regression scenarios are missing")
    print("phase 8 dogfood assertions passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
