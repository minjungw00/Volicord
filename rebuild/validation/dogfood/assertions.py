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
    """Assert the one active Naturalistic contract and retained support engine."""
    import campaign
    import harness
    import machine_findings
    import qualitative_review
    import qualification_policy
    import review_operations

    definition = harness.load_definition()
    topology = definition["campaign_topology"]
    if (topology["journey_count"], topology["work_count"],
            topology["resume_pair_count"], topology["session_count"]) != (3, 5, 3, 8):
        raise AssertionError("current Naturalistic topology changed")
    if any(key in definition for key in (
            "qualification_profile_contract", "materiality_obligations",
            "materiality_obligation_minimums", "reconciliation_contract")):
        raise AssertionError("active definition retains a semantic admission profile")
    if any(key in definition["real_session_evidence"] for key in (
            "bounded_evaluation_basis", "behavior_review",
            "opaque_slot_contract", "behavior_specific_work_intake_contract")):
        raise AssertionError("active definition retains pre-execution semantic obligations")
    commands = set(campaign.parser()._subparsers._group_actions[0].choices)
    obsolete = {"prepare-review", "validate-discovery", "record-discovery",
        "prepare-critique", "validate-critique", "record-critique",
        "prepare-adjudication", "validate-provisional-review",
        "record-provisional-review", "reveal-qualification-profile",
        "prepare-reconciliation", "validate-reconciliation",
        "inspect-reconciliation", "seal-work"}
    if commands & obsolete or not {"prepare", "activate-all", "collect-batch",
            "evaluate", "prepare-qualitative-review", "qualify",
            "approve-phase-9"} <= commands:
        raise AssertionError("current campaign CLI exposes a superseded semantic gate")
    prepare = campaign.parser()._subparsers._group_actions[0].choices["prepare"]
    if not any("--tasks" in action.option_strings and action.required
            for action in prepare._actions):
        raise AssertionError("preparation does not require frozen task input")
    if qualification_policy.contract() != definition["qualification_policy"]:
        raise AssertionError("qualification policy differs from active definition")
    if "campaign_control_coverage" in qualification_policy.contract():
        raise AssertionError("blind control coverage is still a qualification condition")
    if qualitative_review.STATES != definition["qualitative_review_contract"]["states"]:
        raise AssertionError("qualitative assessment states differ from the current definition")
    if len(set(qualitative_review.STATES)) != 6 or "not_observed" not in qualitative_review.STATES:
        raise AssertionError("not_observed is not a distinct review state")
    machine_findings.validate_policy()
    if machine_findings.disposition("raw_hash", "confirmed_violation") != machine_findings.Disposition.HARD:
        raise AssertionError("raw hash tampering lost hard authority")
    if machine_findings.disposition("learning_deliberation_order", "confirmed_violation") == machine_findings.Disposition.HARD:
        raise AssertionError("semantic Learning observation became a hard machine gate")
    if review_operations.workflow_contract()["qualification_authority"] is not False:
        raise AssertionError("review recording gained qualification authority")
    result = subprocess.run([sys.executable, "-B", str(HARNESS), "self-test"],
        cwd=ROOT, check=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    print(result.stdout, end="")
    if result.returncode != 0:
        raise RuntimeError(f"Phase 8 harness self-test failed with exit {result.returncode}: {result.stderr[-2000:]}")
    print("phase 8 dogfood assertions passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
