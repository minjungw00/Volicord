"""Single Phase 8 qualification policy, independent of technical gate execution."""
import json
import evidence_purpose
import resource_observer
from collections import Counter
from pathlib import Path
import secrets

import evaluation_runs
import machine_findings as machine
import qualitative_review as review
import review_operations as operations

REVISION = "replacement-qualification-12"
COVERAGE_CRITERION = "campaign/campaign_interaction/interaction_coverage_adequacy"
MULTI_WORK_CRITERION = "journey-volicord/live_viewer/en/multiple_work_comprehension"

def multi_work_criteria():
    import harness
    locales = harness.load_definition()["qualitative_review_contract"]["live_viewer_locales"]
    return [f"journey-volicord/live_viewer/{locale}/multiple_work_comprehension"
        for locale in locales]

# Direct human/user observations cannot be inferred from an agent's artifact review.
HUMAN_CRITERIA = {"live_viewer/*", "interaction/decision_comprehension_when_applicable"}
TOPOLOGY = {
    "repository_journeys": 3,
    "work_items": 5,
    "resume_pairs": 3,
    "fresh_sessions": 8,
    "work_distribution": {"volicord": 3, "small-python": 1, "polyglot-medium": 1},
    "resume_repository_classes": ["polyglot-medium", "small-python", "volicord"],
}
EXPECTED_WORKS = {
    ("volicord", "A", "journey-volicord-work-a", True),
    ("volicord", "B", "journey-volicord-work-b", False),
    ("volicord", "C", "journey-volicord-work-c", False),
    ("small-python", "A", "journey-small-python-work-a", True),
    ("polyglot-medium", "A", "journey-polyglot-medium-work-a", True),
}
EXPECTED_JOURNEYS = {
    ("volicord", "journey-volicord", ("journey-volicord-work-a", "journey-volicord-work-b", "journey-volicord-work-c")),
    ("small-python", "journey-small-python", ("journey-small-python-work-a",)),
    ("polyglot-medium", "journey-polyglot-medium", ("journey-polyglot-medium-work-a",)),
}
STRUCTURAL_RULES = {
    "journey_project_identity", "journey_work_identity", "journey_work_history",
    "journey_relation_consistency", "journey_resume_continuity", "journey_isolation",
}


def contract():
    return {"revision": REVISION, "human_required": sorted(HUMAN_CRITERIA),
        "human_rationale": "Current Viewer comprehension, accessibility and input/paint experience, plus original user Decision comprehension, require direct human observation. Historical artifact fidelity remains separate.",
        "agent_permitted": "All other rubric criteria with required evidence surfaces and valid references.",
        "conflicts": "A human assessment must explicitly resolve the conflicting review run IDs.",
        "insufficient": "Unresolved; high-impact authority/context recovery and interaction-coverage insufficiency escalates to human.",
        "hard": "Integrity uncertainty and confirmed hard violations cannot be waived by any review or approval.",
        "technical": "Independently verified exact-candidate gate capsule/archive; no technical rerun.",
        "approval": "Explicit operator authorization bound to a complete qualification run and exact input hashes.",
        "interaction_coverage": {"criterion": COVERAGE_CRITERION,
            "required": True, "independent_agent_semantic_review_required": True,
            "satisfied": "may_qualify", "insufficient_evidence": "unresolved", "violated": "blocked",
            "not_observed_allowed": False, "count_thresholds": False},
        "campaign_topology": TOPOLOGY,
        "cli_scope": {"repository_classes": 3, "criteria_per_class": 7, "required_assessments": 21}}


def identity():
    return {"revision": REVISION, "sha256": machine.digest({"contract": contract(),
        "evaluation_policy": evaluation_runs.policy_identity(),
        "implementation_sha256": operations.digest(Path(__file__).read_bytes())})}


def human_required(spec):
    return review.human_only(spec)


def finding_id(scope, number):
    return f"{scope.get('work_slot_id') or scope['journey_id']}/{scope['findings'][number]['check']}"


def validate_topology(evaluation):
    works = evaluation.get("works")
    journeys = evaluation.get("journeys")
    review.require(isinstance(works, list) and isinstance(journeys, list),
        "qualification requires Work- and journey-scoped evaluation")
    actual_works = {
        (item.get("repository_class"), item.get("work"), item.get("work_slot_id"), item.get("resume_pair"))
        for item in works
    }
    actual_journeys = {
        (item.get("repository_class"), item.get("journey_id"), tuple(item.get("work_slot_ids", [])))
        for item in journeys
    }
    review.require(len(works) == 5 and actual_works == EXPECTED_WORKS,
        "qualification Work topology changed")
    review.require(len(journeys) == 3 and actual_journeys == EXPECTED_JOURNEYS,
        "qualification repository-journey topology changed")
    review.require(evaluation.get("coverage") == TOPOLOGY,
        "qualification session/resume coverage changed")
    return dict(TOPOLOGY)


def _criterion_state(result, required_ids):
    """A required exact set passes only when every member is resolved."""
    qualitative = result["qualitative_review"]
    required = set(required_ids)
    review.require(required, "summary requires at least one exact criterion")
    if required & set(qualitative["violated_criteria"]):
        return "violated"
    if required <= set(qualitative["resolved_criteria"]):
        return "satisfied"
    return "unresolved"


def browser_criteria():
    import harness
    locales = harness.load_definition()["qualitative_review_contract"]["live_viewer_locales"]
    return [f"journey-volicord/live_viewer/{locale}/browser_input_and_paint_responsiveness"
        for locale in locales]


def structural_state(checks):
    statuses = set(checks.values())
    if statuses == {"confirmed_pass"}:
        return "passed"
    if "confirmed_violation" in statuses:
        return "violated"
    return "unresolved"


def naturalistic_summary(result, evaluation, memory=None):
    resource = memory if memory is not None else {
        "kind": "dogfood_naturalistic_mcp_memory_evidence",
        "schema_version": 1,
        "status": "not_provided",
    }
    volicord = next(item for item in evaluation["journeys"]
        if item["journey_id"] == "journey-volicord")
    checks = {finding["check"]: finding["status"] for finding in volicord["findings"]
        if finding["check"] in STRUCTURAL_RULES}
    review.require(set(checks) == STRUCTURAL_RULES,
        "Volicord journey omitted structural continuity findings")
    return {
        "multi_work_structural_continuity": {
            "state": structural_state(checks),
            "evidence_class": "immutable_main_campaign_journey_structure",
            "journey_id": "journey-volicord",
            "checks": dict(sorted(checks.items())),
            "deterministic_fixture": "supporting_regression_only",
        },
        "multi_work_viewer_comprehension": {
            "state": _criterion_state(result,
                multi_work_criteria()),
            "evidence_class": "direct_human_live_viewer_observation",
            "criterion_ids": multi_work_criteria(),
        },
        "live_browser_input_and_paint": {
            "state": _criterion_state(result, browser_criteria()),
            "snapshot_export_proxy_may_substitute": False,
        },
        "naturalistic_resource": resource,
    }


# An absent naturalistic opportunity is recorded without claiming a pass. These
# criterion names describe optional events; core Work quality and direct human
# observations still require a judgment or remain incomplete.
OPTIONAL_OPPORTUNITY_CRITERIA = review.NOT_OBSERVED_OPPORTUNITIES


def nonblocking_not_observed(spec):
    return (spec["group"] == "interaction" and spec["name"] in OPTIONAL_OPPORTUNITY_CRITERIA
        and not (spec.get("workload_intent") == "learning_collaborative"
            and spec["name"] in review.LEARNING_OPPORTUNITIES))


def combine(evaluation, specs, reviews, technical, *, evidence_validity="valid",
            naturalistic_resource=None):
    """Inputs have been identity/hash validated by the file boundary below.

    Every required criterion remains explicit, including those with no review.
    A machine observation is never erased by semantic adjudication.
    """
    purpose = evidence_purpose.validate(evaluation["evidence_purpose"])
    topology = validate_topology(evaluation)
    scopes = [*evaluation["works"], *evaluation["journeys"]]
    findings = {finding_id(scope, n): f for scope in scopes for n, f in enumerate(scope["findings"])}
    hard = sorted(k for k, f in findings.items() if f["disposition"] == "hard_blocking")
    criteria = {s["criterion_id"]: s for s in specs}
    review.require(COVERAGE_CRITERION in criteria, "qualification requires interaction coverage assessment")
    assessments = {}
    for value in reviews:
        run = value["reviewer"]["run_id"]
        for a in value["assessments"] + [v["finding"] for v in value["additional_outcomes"]]:
            if a["criterion_id"] not in criteria:
                criteria[a["criterion_id"]] = {"criterion_id": a["criterion_id"], "group": "authority", "name": "additional"}
            assessments.setdefault(a["criterion_id"], []).append((value, a))
    resolved, unresolved, escalated, violated, not_observed = [], [], [], [], []
    accepted = {}
    for cid, spec in sorted(criteria.items()):
        entries = [(r, a) for r, a in assessments.get(cid, []) if a["assessment"] != "not_reviewed"]
        humans = [(r, a) for r, a in entries if r["reviewer"]["kind"] == "human"]
        decisive = [(r, a) for r, a in entries if a["assessment"] in {"satisfied", "not_applicable", "violated"}]
        conflict = (len({a["assessment"] == "violated" for _, a in decisive}) > 1
            or ("not_observed" in {a["assessment"] for _, a in entries}
                and bool(decisive)))
        impact_gap = spec["group"] in review.HIGH_IMPACT_INSUFFICIENCY_GROUPS and any(a["assessment"] == "insufficient_evidence" for _, a in entries)
        inapplicable_comprehension = (spec["group"] == "interaction" and spec["name"] == "decision_comprehension_when_applicable"
            and bool(entries) and all(a["assessment"] == "not_applicable" for _, a in entries))
        requires_human = (human_required(spec) and not inapplicable_comprehension) or conflict or impact_gap
        eligible = humans if requires_human else entries
        if conflict or impact_gap:
            eligible = [(r, a) for r, a in humans if
                {v["reviewer"]["run_id"] for v, _ in entries if v is not r}
                <= set(r.get("resolves_review_runs", {}).get(cid, []))]
        if spec["name"] == "interaction_coverage_adequacy":
            # A direct human review can resolve conflict, but cannot replace the
            # separately recorded independent agent semantic inspection.
            if not any(r["reviewer"]["kind"] == "agent" and a["assessment"] in {"satisfied", "violated", "insufficient_evidence"}
                       for r, a in entries):
                eligible = [(r, a) for r, a in eligible if a["assessment"] == "violated"]
        states = {a["assessment"] for _, a in eligible}
        if "violated" in states:
            violated.append(cid)
        elif states & {"satisfied", "not_applicable"}:
            resolved.append(cid)
            accepted[cid] = [(r, a) for r, a in eligible if a["assessment"] in {"satisfied", "not_applicable"}]
        elif "not_observed" in states and nonblocking_not_observed(spec) and not requires_human:
            not_observed.append(cid)
        else:
            unresolved.append(cid)
            if requires_human:
                escalated.append(cid)
    # Semantic machine findings are inspectable support. Recorded qualitative
    # judgments own their interpretation; only deterministic hard findings block.
    review_support_findings = sorted(fid for fid, finding in findings.items()
        if finding["disposition"] == "qualitative_review_required")
    unresolved_findings = []
    complete = not unresolved and not unresolved_findings and not violated
    blocked = evidence_validity != "valid" or bool(hard) or technical["state"] in {"failed", "candidate_mismatch", "invalid"} or bool(violated)
    status = "blocked" if blocked else "qualified" if complete and technical["state"] == "passed" and purpose == evidence_purpose.NATURALISTIC else "unresolved"
    result = {"evidence_purpose": purpose, "evidence_validity": evidence_validity, "campaign_topology": topology,
        "technical_gate": technical,
        "machine_summary": {"counts": dict(sorted(Counter(f["disposition"] for f in findings.values()).items())),
            "hard_findings": hard, "review_support_findings": review_support_findings,
            "unresolved_findings": unresolved_findings},
        "qualitative_review": {"state": "complete" if complete else "incomplete", "resolved_criteria": resolved,
            "violated_criteria": violated, "not_observed_criteria": not_observed, "unresolved_criteria": unresolved, "human_escalations": escalated},
        "operator_approval": {"state": "not_provided"}, "replacement_qualification": status,
        "replacement_pass_candidate": status == "qualified", "phase_9_ready": False}
    result["naturalistic_evidence"] = naturalistic_summary(result, evaluation, naturalistic_resource)
    return result


def verify_technical(candidate, capsule_path, archive_path, *, candidate_artifacts=None):
    """Read-only reuse of the maintained gate's independent verifier and capsule contract."""
    if capsule_path is None and archive_path is None:
        return {"state": "not_provided"}
    review.require(capsule_path is not None and archive_path is not None, "technical evidence requires capsule and archive")
    import runpy
    import tarfile
    import campaign
    verifier = runpy.run_path(str(campaign.ROOT / "rebuild/scripts/verify-validation-archive"))
    capsule_checker = runpy.run_path(str(campaign.ROOT / "rebuild/scripts/check-validation-report"))
    verification = verifier["verify"](archive_path, candidate)
    data = operations.bounded_read(capsule_path)
    capsule = json.loads(data)
    review.require(not capsule_checker["capsule_contract_errors"](capsule), "candidate gate capsule contract failed")
    review.require(capsule.get("validated_candidate_head") == candidate, "technical gate Product candidate mismatch")
    evidence = capsule["evidence_archive"]
    review.require(evidence["sha256"] == verification["archive_sha256"]
        and evidence["size_bytes"] == verification["archive_size_bytes"]
        and evidence["member_count"] == verification["member_count"]
        and evidence["verification_status"] == "passed", "technical capsule/archive binding mismatch")
    with tarfile.open(archive_path) as archive:
        prior = json.load(archive.extractfile("validation-evidence/capsule.json"))
    # The final capsule differs from its archived pre-verification snapshot only
    # by archive completion and, when ready, one successful publication check.
    gate = runpy.run_path(str(campaign.ROOT / "rebuild/validation/end-to-end/multi-repository/gate.py"))
    expected = gate["complete_evidence_archive"](prior,
        {"path": archive_path, "candidate_head": candidate, "sha256": verification["archive_sha256"],
         "size_bytes": verification["archive_size_bytes"], "member_count": verification["member_count"]}, verification)
    if expected["phase_8_ready"] is True:
        expected["candidate_continuity_checks"].append({"boundary": "archive_publication",
            **gate["candidate_continuity_check"](candidate, candidate, 0, [])})
    # Compare the entire structure, including the unchanged continuity prefix.
    # JSON encoding also distinguishes booleans from numerically equal values.
    review.require(operations.encoded(capsule) == operations.encoded(expected),
        "final capsule differs from verified archive completion/publication")
    if candidate_artifacts is not None and capsule["phase_8_ready"]:
        review.require({name: binding["sha256"] for name, binding in candidate_artifacts.items()}
            == capsule["dogfood_rehearsal"]["result"]["executables"],
            "technical gate candidate executable mismatch")
    return {"rehearsal": {key: capsule["dogfood_rehearsal"][key]
            for key in ("contract", "status", "result_sha256")},
        "state": "passed" if capsule["phase_8_ready"] else "failed", "candidate_head": candidate,
        "capsule_sha256": operations.digest(data), "archive_sha256": verification["archive_sha256"],
        "verification": "maintained_independent_archive_and_capsule_contract", "execution": "reused"}


def qualify(root, evaluation_path, output, *, candidate, review_roots=(), capsule_path=None, archive_path=None):
    import campaign
    manifest = campaign.load_evidence_set(root)
    evaluation = evaluation_runs.load(evaluation_path)
    purpose = evidence_purpose.require_same(manifest, evaluation)
    evidence_hash = campaign.harness.sha256(root / "evidence-set.json")
    review.require(candidate == manifest["candidate_head"] == evaluation["candidate_head"], "Product candidate mismatch")
    review.require(evaluation["evidence_set"] == {"path": "evidence-set.json", "sha256": evidence_hash}, "evaluation evidence mismatch")
    _, index, _ = operations.select_evidence(root, manifest, evaluation, include_raw=False)
    specs = review.criterion_specs(index, review.rubric(campaign.harness.load_definition()))
    reviews, references = [], []
    for path in review_roots:
        prep, sha, _ = operations.load_package(path)
        files = operations.recorded_files(path, prep, sha)
        review.require(files, "qualification consumes only immutable recorded reviews")
        value = json.loads(files["recorded/review.json"])
        binding = value["binding"]
        evidence_purpose.require_same(manifest, binding)
        review.require(binding["candidate_head"] == candidate and binding["evidence_set"] == {"sha256": evidence_hash}, "review candidate/evidence mismatch")
        if binding["machine_evaluation"] is not None:
            review.require(binding["machine_evaluation"] == {"run_id": evaluation["run_id"],
                "sha256": campaign.harness.sha256(evaluation_path),
                "recorded_policy": evaluation["policy"],
                "policy_verification": "recorded_identity_not_current_equivalence"}, "review binds a different machine run")
        review.require(review.criterion_specs(prep["index"], prep["rubric"]) == specs, "review criterion coverage differs from evidence")
        reviews.append(value)
        references.append({"run_id": value["reviewer"]["run_id"], "kind": value["reviewer"]["kind"],
            "review_sha256": operations.digest(files["recorded/review.json"]), "preparation_sha256": sha})
    review.require(len({r["run_id"] for r in references}) == len(references), "duplicate review run")
    consumed_ids = {r["run_id"] for r in references}
    for value in reviews:
        review.require(all(set(ids) <= consumed_ids for ids in value["resolves_review_runs"].values()),
            "human resolution references an unconsumed review run")
    technical = verify_technical(candidate, capsule_path, archive_path,
        candidate_artifacts=manifest["candidate_artifacts"])
    result = {"kind": "phase8_dogfood_result", "schema_version": 3, "candidate_head": candidate,
        "evidence_set": evaluation["evidence_set"], "evaluator_revision": campaign.harness.git_head(campaign.ROOT),
        "policy": identity(), "evaluation_run": {"run_id": evaluation["run_id"], "sha256": campaign.harness.sha256(evaluation_path)},
        "qualitative_review_runs": sorted(references, key=lambda v: v["run_id"]), "run_nonce": secrets.token_hex(16),
        **combine(evaluation, specs, reviews, technical,
            naturalistic_resource=manifest.get("naturalistic_memory_evidence"))}
    result["run_id"] = machine.digest(result)
    validate_result(result)
    campaign.load_evidence_set(root)
    if output is not None:
        review.require(not output.resolve().is_relative_to(root.resolve()), "qualification output must be outside campaign")
        inputs = {"campaign_root": str(root.resolve()), "evaluation": str(evaluation_path.resolve()),
            "candidate": candidate, "review_roots": [str(p.resolve()) for p in review_roots],
            "capsule": str(capsule_path.resolve()) if capsule_path else None,
            "archive": str(archive_path.resolve()) if archive_path else None}
        operations.publish_directory(output, {"qualification.json": operations.encoded(result),
            "inputs.json": operations.encoded(inputs)})
    return result


def validate_result(value):
    import re
    review.require(value.get("kind") == "phase8_dogfood_result" and value.get("schema_version") == 3
        and value.get("policy") == identity(), "invalid qualification policy/schema")
    for field, size in (("candidate_head", 40), ("evaluator_revision", 40), ("run_nonce", 32)):
        review.require(re.fullmatch(f"[0-9a-f]{{{size}}}", str(value.get(field, ""))), "invalid qualification identity")
    review.require(value.get("run_id") == machine.digest({k: v for k, v in value.items() if k != "run_id"}), "qualification run hash changed")
    q, m, t = value["qualitative_review"], value["machine_summary"], value["technical_gate"]
    review.require(COVERAGE_CRITERION in set(q["resolved_criteria"] + q["unresolved_criteria"] + q["violated_criteria"])
        and COVERAGE_CRITERION not in q["not_observed_criteria"], "required interaction coverage was omitted or unobserved")
    purpose = evidence_purpose.validate(value.get("evidence_purpose"))
    naturalistic = value.get("naturalistic_evidence")
    structural = naturalistic.get("multi_work_structural_continuity") if isinstance(naturalistic, dict) else None
    comprehension = naturalistic.get("multi_work_viewer_comprehension") if isinstance(naturalistic, dict) else None
    review.require(value.get("campaign_topology") == TOPOLOGY
        and isinstance(naturalistic, dict)
        and set(naturalistic) == {"multi_work_structural_continuity", "multi_work_viewer_comprehension",
            "live_browser_input_and_paint", "naturalistic_resource"}
        and isinstance(structural, dict) and structural.get("evidence_class")
            == "immutable_main_campaign_journey_structure"
        and structural.get("journey_id") == "journey-volicord"
        and set(structural.get("checks", {})) == STRUCTURAL_RULES
        and structural.get("state") == structural_state(structural["checks"])
        and structural.get("deterministic_fixture") == "supporting_regression_only"
        and isinstance(comprehension, dict)
        and comprehension.get("state")
            == _criterion_state(value, multi_work_criteria())
        and comprehension.get("evidence_class") == "direct_human_live_viewer_observation"
        and comprehension.get("criterion_ids")
            == multi_work_criteria()
        and naturalistic["live_browser_input_and_paint"]["state"]
            == _criterion_state(value, browser_criteria())
        and naturalistic["live_browser_input_and_paint"]["snapshot_export_proxy_may_substitute"] is False
        and naturalistic["naturalistic_resource"].get("status")
            in resource_observer.STATUSES | {"not_provided"},
        "naturalistic evidence scope or qualification relationship changed")
    if naturalistic["naturalistic_resource"].get("status") != "not_provided":
        resource_observer.validate(naturalistic["naturalistic_resource"])
    complete = not (q["unresolved_criteria"] or q["violated_criteria"] or q["human_escalations"] or m["unresolved_findings"])
    review.require("campaign_control_coverage" not in value
        and isinstance(q.get("not_observed_criteria"), list)
        and not set(q["not_observed_criteria"]) & (set(q["resolved_criteria"]) | set(q["violated_criteria"]) | set(q["unresolved_criteria"])),
        "qualitative not-observed state is not distinct")
    blocked = value["evidence_validity"] != "valid" or bool(m["hard_findings"]) or bool(q["violated_criteria"]) or t["state"] in {"failed", "invalid", "candidate_mismatch"}
    expected = "blocked" if blocked else "qualified" if complete and t["state"] == "passed" and purpose == evidence_purpose.NATURALISTIC else "unresolved"
    review.require(value["replacement_qualification"] == expected and value["replacement_pass_candidate"] is (expected == "qualified")
        and q["state"] == ("complete" if complete else "incomplete"), "qualification state contradicts mandatory evidence")
    if t["state"] == "passed":
        import rehearsal_contract
        proof = t.get("rehearsal")
        review.require(isinstance(proof, dict) and set(proof) == {"contract", "status", "result_sha256"}
            and proof["contract"] == rehearsal_contract.CONTRACT and proof["status"] == "passed"
            and re.fullmatch(r"[0-9a-f]{64}", str(proof["result_sha256"])),
            "technical qualification requires retained passed rehearsal identity")
        review.require(t["candidate_head"] == value["candidate_head"], "technical gate candidate mismatch")
    approval = value["operator_approval"]
    review.require(approval == {"state": "not_provided"} and value["phase_9_ready"] is False,
        "qualification alone cannot grant operator approval")


def approve(qualification_path, output, *, operator, statement):
    """Explicit operator action, never invoked by evaluation or reviewer recording."""
    data = operations.bounded_read(qualification_path)
    value = json.loads(data)
    review.require(verify_qualification(qualification_path) == value, "qualification changed during approval")
    review.require(value["replacement_qualification"] == "qualified", "operator approval cannot replace missing evidence or required review")
    review.require(review.authority.bounded_text(operator) and statement == "approve-phase-9", "explicit operator authorization is required")
    inputs = json.loads(operations.bounded_read(qualification_path.with_name("inputs.json")))
    review.require(not output.resolve().is_relative_to(Path(inputs["campaign_root"]).resolve()), "approval must remain outside immutable campaign")
    result = approval_value(value, data, operator, statement)
    review.require(operations.bounded_read(qualification_path) == data, "qualification changed before approval publication")
    operations.publish_directory(output, {"approval.json": operations.encoded(result), "qualification.json": data})
    return result


def approval_value(value, data, operator, statement):
    evidence_purpose.require_measured(value)
    result = {**value, "kind": "dogfood_operator_approval", "schema_version": 1, "policy": identity(),
        "candidate_head": value["candidate_head"], "evidence_set": value["evidence_set"],
        "qualification_run_id": value["run_id"], "qualification_sha256": operations.digest(data),
        "operator": {"identity": operator, "identity_state": "self_reported"}, "action": statement,
        "operator_approval": {"state": "approved"}, "replacement_qualification": "qualified", "phase_9_ready": True}
    result.pop("run_id", None)
    result["run_id"] = machine.digest(result)
    return result


def verify_qualification(path):
    value = json.loads(operations.bounded_read(path))
    validate_result(value)
    inputs = json.loads(operations.bounded_read(path.with_name("inputs.json")))
    recomputed = qualify(Path(inputs["campaign_root"]), Path(inputs["evaluation"]), None,
        candidate=inputs["candidate"], review_roots=[Path(p) for p in inputs["review_roots"]],
        capsule_path=Path(inputs["capsule"]) if inputs["capsule"] else None,
        archive_path=Path(inputs["archive"]) if inputs["archive"] else None)
    for key in ("run_id", "run_nonce", "evaluator_revision"):
        recomputed[key] = value[key]
    review.require(value == recomputed, "qualification differs from verified evidence and recorded reviews")
    return value


def verify_approval(path, qualification_path):
    value = json.loads(operations.bounded_read(path))
    qualified = verify_qualification(qualification_path)
    data = operations.bounded_read(qualification_path)
    review.require(qualified["replacement_qualification"] == "qualified", "approval cannot replace qualification")
    operator = value.get("operator", {}).get("identity")
    review.require(review.authority.bounded_text(operator), "approval requires explicit operator identity")
    review.require(value == approval_value(qualified, data, operator, "approve-phase-9"), "approval state/hash or qualification binding changed")
    review.require(operations.bounded_read(path.with_name("qualification.json")) == data, "approval's preserved qualification changed")
    return value
