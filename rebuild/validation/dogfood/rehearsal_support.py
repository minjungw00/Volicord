"""Source-bound support input builders for actual Product workflow operations.

Extracted bounded schema construction, without any Final/V11 execution owner.
These authored assertions demonstrate transport and validation, not semantic truth.
"""
from typing import Any


def realization(plan):
    """Authored structural input; the Product alone supplies plan/receipt/readback."""
    import explanation_evidence as e
    questions = e.WORK_QUESTIONS if plan['subject']['kind'] == 'work' else e.DECISION_QUESTIONS
    return {'format_kind': 'volicord_explanation', 'format_version': 1,
        'plan_fingerprint': plan['fingerprint'], 'language': plan['requested_language'],
        'generator': {'host': 'self_authored_test_support', 'session': 'rehearsal-script',
            'agent': None, 'model': None},
        'paragraphs': [{'question': q,
            'text': '직접 작성한 구조 검사 입력입니다.' if plan['requested_language'] == 'ko' else 'Self-authored structural support input.',
            'evidence_keys': [v['key'] for v in plan['evidence']]} for q in sorted(questions)]}


def product_explanation(root, binary, runtime, project, work, logs, phase):
    """Fresh public CLI generation for session-time Recall; no campaign seams."""
    import json
    import campaign as c
    import explanation_evidence as e
    argv = [binary, '--runtime', runtime, '--project', project, '--json']
    prepared = json.loads(logs.run([*argv, 'work', 'explain', 'prepare', '--work', work, '--language', 'en']))
    plan = prepared['plan']
    response = realization(plan)
    path = root / (phase + '-authored-response.json')
    c.write_json(path, response)
    receipt = json.loads(logs.run([*argv, 'work', 'explain', 'record', '--work', work,
        '--language', 'en', '--input', path]))
    return plan, response, receipt


def correction(call, project, work, revision, scenario):
    """MCP creates a distinct Source from this explicitly authored authorization."""
    receipt = call('canonical_mutate', {'project_id': project, 'action': 'correct_context',
        'record_id': work, 'expected_revision': revision,
        'corrected_text': scenario['corrected_statement'], 'user_turn': scenario['authorization']})
    if receipt.get('revision') != revision + 1 or receipt.get('identity') != work:
        raise ValueError('Product did not complete the requested correction')
    return receipt


def goal_evidence(plan):
    return next(e for e in plan['evidence'] if e['key'] == 'goal')


def check_sources(before, after, receipt):
    old, new = goal_evidence(before), goal_evidence(after)
    if (old['identity'] != new['identity'] or new['revision'] != old['revision'] + 1
            or old['sources'] != new['sources'] or not old['sources']
            or receipt['user_response_source_id'] in new['sources']):
        raise ValueError('correction confused Goal Sources with authorization Source')


ENGINEERING_EFFECT_CATEGORIES = (
    "public_api_shape_or_semantics",
    "compatibility",
    "failure_or_error_semantics",
    "persistence_or_lifetime",
    "privacy_or_disclosure",
    "security",
    "user_visible_behavior_or_default",
    "performance_or_resource_behavior",
    "concurrency_or_operability",
    "maintenance_or_support",
    "implementation_internal",
)

def coupled_artifact_review(paths: list[str]) -> dict[str, Any]:
    categories = (
        "implementation",
        "focused_tests",
        "public_or_internal_documentation",
        "changelog_or_release_notes",
        "schema_snapshot_or_generated_artifact",
        "other_repository_owned_artifact",
    )
    return {
        "assessments": [
            {
                "category": category,
                "disposition": (
                    {"state": "included", "repository_paths": paths}
                    if category == "implementation"
                    else {"state": "no_coupled_artifact"}
                ),
                "basis_summary": "Self-authored support repository inspection accounts for this artifact category.",
            }
            for category in categories
        ],
        "materiality_closure": {"state": "no_new_material_outcome", "commitments": [{"commitment_id": "private-fixture", "description": "Private fixture change preserves the entire current reviewed material outcome graph", "repository_paths": paths, "temporal_effect": {"state": "no_temporal_change", "outcome_id": "fixture-temporal_and_lifetime", "result_id": "unchanged", "rationale": "This fixture preserves temporal results without choosing timestamp or lifetime behavior."}, "outcome_binding": {"state": "private_equivalent", "equivalence_rationale": "The fixture introduces no new material result; all server-bound current dimensions and interactions remain unchanged"}}], "rationale": (
            "The bounded Self-authored support artifact introduces no new material outcome beyond the current dimensions."
        )},
    }

def alternative_accounting(
    choice_id: str,
    alternative_ids: list[str],
    source_id: str,
    *,
    resolution_decision_id: str | None = None,
) -> list[dict[str, Any]]:
    accounts: list[dict[str, Any]] = []
    for index, alternative_id in enumerate(alternative_ids):
        account: dict[str, Any] = {
            "choice_id": choice_id,
            "alternative_id": alternative_id,
            "status": "unresolved",
            "rationale": "This discovered alternative remains credible on the current authority basis.",
            "source_ids": [source_id],
        }
        if resolution_decision_id:
            account.update(
                {
                    "status": (
                        "selected"
                        if index == 0
                        else "eliminated_by_applicable_decision"
                    ),
                    "rationale": (
                        "The current-host Decision selects this alternative."
                        if index == 0
                        else "The current-host Decision excludes this alternative."
                    ),
                }
            )
            if index != 0:
                account["decision_id"] = resolution_decision_id
        accounts.append(account)
    return accounts

def outside_interactions(choices, source_ids):
    """Isolated qualification choices do not alter these interactions."""
    return [{"axis": axis, "outcomes": [{
        "outcome_id": "fixture-" + axis,
        "scenario": "The bounded fixture leaves existing " + axis + " behavior unchanged",
        "credible_outcomes": [{"result_id": "unchanged", "description": "Existing interaction result is preserved"}],
        "affected_choice_ids": [], "source_basis": source_ids,
        "conclusion": {"result_id": "unchanged", "state": "no_independent_fork", "basis": "outside_affected_scope",
            "rationale": "The maintained fixture Source limits this isolated authority test; these interaction results are unchanged"},
    }]} for axis in ("reference_basis", "composition_and_precedence", "multi_item_effects", "failure_and_recovery", "temporal_and_lifetime")]

def material_boundary_review(
    choices: list[dict[str, Any]], source_ids: list[str]
) -> list[dict[str, Any]]:
    return [
        {
            "effect_category": category,
            "reviewed_outcomes": [f"Fixture observable behavior within {category}"],
            "conclusion": (
                {
                    "state": "represented_by_choices",
                    "choice_ids": [
                        choice["choice_id"]
                        for choice in choices
                        if category in choice["effect_categories"]
                    ],
                }
                if any(category in choice["effect_categories"] for choice in choices)
                else {
                    "state": "no_independent_fork",
                    "basis": "outside_affected_scope",
                    "rationale": f"The Self-authored support source basis exposes no independent {category} fork.",
                }
            ),
            "source_ids": source_ids,
        }
        for category in ENGINEERING_EFFECT_CATEGORIES
    ]

def record_support_checkpoint(call, project_id, goal, baseline, repository, label, learning_request=None, learning_context=None):
    goal_id = goal["context_item_id"]
    baseline_id = baseline["analysis_snapshot_id"]
    repository_source = baseline["repository_source_id"]
    marker = "support-marker-" + label.lower() + ".txt"
    choice_id = f"private-continuity-{label.lower()}"
    alternatives = ("inline", "helper")
    choice = {
        "choice_id": choice_id,
        "summary": f"Choose private {label} Work marker organization",
        "affected_scope": [marker],
        "alternatives": [{
            "alternative_id": alternative,
            "summary": f"Keep the {label} marker in a private {alternative} form",
            "technical_consequences": ["The bounded marker preserves public Work behavior"],
            "material_decomposition": {
                "state": "materially_atomic",
                "rationale": "The maintained fixture fixes public behavior and leaves no subordinate product fork.",
                "residual_fork_closure": {
                    "interaction_comparisons": [],
                    "fixed_outcome": "The Work marker preserves the same public behavior",
                    "credible_implementations": [
                        "Direct private implementation", "Private helper implementation"],
                    "remaining_material_outcomes": [],
                    "source_basis": [repository_source],
                },
            },
        } for alternative in alternatives],
        "technical_consequences": ["Private organization varies without changing public Work behavior"],
        "source_ids": [repository_source],
        "effect_categories": ["implementation_internal"],
        "relationship": {"state": "independent"},
        "evidence_state": "sufficient",
    }
    discovery = call("engineering_choice_discovery", {
        "project_id": project_id,
        "goal_context_id": goal_id,
        "baseline_analysis_snapshot_id": baseline_id,
        "source_operation": f"Self-authored support {label} Work private-marker inspection",
        "summary": choice["summary"],
        "choices": [choice],
        "interaction_review": outside_interactions([choice], [repository_source]),
        "material_boundary_review": material_boundary_review([choice], [repository_source]),
    })
    review = call("materiality_review", {
        "action": "record",
        "project_id": project_id,
        "engineering_choice_discovery_candidate_id": discovery["discovery_candidate_id"],
        "rationale": "The exact current Goal and source leave only private marker organization open.",
        "behavioral_context_basis": {
            "context_item_ids": [learning_context["context_item_id"]] if learning_context else [],
            "completeness_rationale": "The bounded authored task Learning is included when requested; the private support marker adds no other behavior context.",
        },
        "learning_participation": ({"state": "active", "user_turn_source_id": learning_context["source_id"], "verbatim_statement": learning_request} if learning_request else {"state": "inactive"}),
        "judgments": [{
            "choice_id": choice_id,
            "disposition": "agent_owned_implementation_choice",
            "materially_varying_outcomes": ["Private marker organization"],
            "contains_user_owned_outcome": False,
            "user_owned_outcomes": [],
            "ownership_rationale": "Both private arrangements preserve the same public Work result.",
            "discretion_counterfactuals": [{
                "choice_id": choice_id, "alternative_id": alternative,
                "externally_observable": False,
                "observation_rationale": "Public Work identity and history remain the same.",
                "source_id": repository_source,
                "source_supported_boundary": "The maintained fixture bounds this private marker.",
            } for alternative in alternatives],
            "bounded_implementation_discretion_rationale": "The change is limited to a private marker.",
            "ownership_source_ids": [repository_source],
            "alternative_accounting": alternative_accounting(
                choice_id, list(alternatives), repository_source),
            "basis_summary": "Only private marker organization varies.",
            "authority_counterfactual": "Neither alternative changes a user-owned outcome.",
            "learning_authority": {"state": "assessed", "independent_user_authority": False,
                "rationale": "The authored marker preserves public outcomes independently of the learning request.",
                "source_ids": [repository_source]},
            "learning_value": {"state": "routine", "rationale": "The authored private marker adds no significant learning fork."},
        }],
    })
    ready = call("materiality_review", {
        "action": "inspect",
        "project_id": project_id,
        "review_candidate_id": review["review_candidate_id"],
        "goal_context_id": goal_id,
        "baseline_analysis_snapshot_id": baseline_id,
        "paths": [marker], "components": [], "work_contexts": [],
        "met_revisit_triggers": [],
        "coupled_artifact_review": coupled_artifact_review([marker]),
    })
    if (ready.get("workflow", {}).get("stage") != "ready_for_work"
            or ready["workflow"].get("blocks_ordinary_work") is not False):
        raise ValueError(f"Work {label} materiality did not resolve")
    (repository / marker).write_text(f"{label} Work continuity marker\n", encoding="utf-8")
    checkpoint_args = {
        "verification_basis": {"state": "ordinary_change"},
        "project_id": project_id,
        "goal_context_id": goal_id,
        "baseline_analysis_snapshot_id": baseline_id,
        "kind": "handoff", "work_state": "in_progress" if label == "C" else "paused",
        "work_contexts": [], "verification": [{"state": "not_run"}],
        "next_step": f"Resume {label} Work", "handoff_to": "next Codex session",
    }
    return call("checkpoint_record", {**checkpoint_args, "applied_decision_ids": [], "state_change": "Wrote an authored private support marker; requested implementation remains unverified."})
