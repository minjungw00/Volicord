"""Single Phase 8 qualification policy, independent of technical gate execution."""
import json
from collections import Counter
from pathlib import Path
import secrets

import evaluation_runs
import machine_findings as machine
import qualitative_review as review
import review_operations as operations

REVISION = "replacement-qualification-1"
# Direct human/user observations cannot be inferred from an agent's artifact review.
HUMAN_CRITERIA = {"live_viewer/*", "interaction/decision_comprehension_when_applicable"}


def contract():
    return {"revision": REVISION, "human_required": sorted(HUMAN_CRITERIA),
        "human_rationale": "Live accessibility and the user's Decision comprehension require direct human observation.",
        "agent_permitted": "All other rubric criteria with required evidence surfaces and valid references.",
        "conflicts": "A human assessment must explicitly resolve the conflicting review run IDs.",
        "insufficient": "Unresolved; high-impact authority/context recovery insufficiency escalates to human.",
        "hard": "Integrity uncertainty and confirmed hard violations cannot be waived by any review or approval.",
        "technical": "Independently verified exact-candidate gate capsule/archive; no technical rerun.",
        "approval": "Explicit operator authorization bound to a complete qualification run and exact input hashes.",
        "cycles": 8, "fresh_sessions": 16}


def identity():
    return {"revision": REVISION, "sha256": machine.digest({"contract": contract(),
        "evaluation_policy": evaluation_runs.policy_identity(),
        "implementation_sha256": operations.digest(Path(__file__).read_bytes())})}


def human_required(spec):
    return spec["group"] == "live_viewer" or (spec["group"] == "interaction"
        and spec["name"] == "decision_comprehension_when_applicable")


def finding_id(cycle, number):
    return f"{cycle['repository_class']}-{cycle['cycle']}/{cycle['findings'][number]['check']}"


def combine(evaluation, specs, reviews, technical, *, evidence_validity="valid"):
    """Inputs have been identity/hash validated by the file boundary below.

    Every required criterion remains explicit, including those with no review.
    A machine observation is never erased by semantic adjudication.
    """
    findings = {finding_id(c, n): f for c in evaluation["cycles"] for n, f in enumerate(c["findings"])}
    hard = sorted(k for k, f in findings.items() if f["disposition"] == "hard_blocking")
    criteria = {s["criterion_id"]: s for s in specs}
    assessments = {}
    for value in reviews:
        run = value["reviewer"]["run_id"]
        for a in value["assessments"] + [v["finding"] for v in value["additional_outcomes"]]:
            if a["criterion_id"] not in criteria:
                criteria[a["criterion_id"]] = {"criterion_id": a["criterion_id"], "group": "authority", "name": "additional"}
            assessments.setdefault(a["criterion_id"], []).append((value, a))
    resolved, unresolved, escalated, violated = [], [], [], []
    accepted = {}
    for cid, spec in sorted(criteria.items()):
        entries = [(r, a) for r, a in assessments.get(cid, []) if a["assessment"] != "not_reviewed"]
        humans = [(r, a) for r, a in entries if r["reviewer"]["kind"] == "human"]
        decisive = [(r, a) for r, a in entries if a["assessment"] in {"satisfied", "not_applicable", "violated"}]
        conflict = len({a["assessment"] == "violated" for _, a in decisive}) > 1
        impact_gap = spec["group"] in {"authority", "context_recovery"} and any(a["assessment"] == "insufficient_evidence" for _, a in entries)
        inapplicable_comprehension = (spec["group"] == "interaction" and spec["name"] == "decision_comprehension_when_applicable"
            and bool(entries) and all(a["assessment"] == "not_applicable" for _, a in entries))
        requires_human = (human_required(spec) and not inapplicable_comprehension) or conflict or impact_gap
        eligible = humans if requires_human else entries
        if conflict or impact_gap:
            eligible = [(r, a) for r, a in humans if
                {v["reviewer"]["run_id"] for v, _ in entries if v is not r}
                <= set(r.get("resolves_review_runs", {}).get(cid, []))]
        states = {a["assessment"] for _, a in eligible}
        if "violated" in states:
            violated.append(cid)
        elif states & {"satisfied", "not_applicable"}:
            resolved.append(cid)
            accepted[cid] = [(r, a) for r, a in eligible if a["assessment"] in {"satisfied", "not_applicable"}]
        else:
            unresolved.append(cid)
            if requires_human:
                escalated.append(cid)
    unresolved_findings = []
    for fid, finding in findings.items():
        if finding["disposition"] != "qualitative_review_required":
            continue
        # Only the policy-owned semantic group can resolve a finding; no unrelated
        # positive criterion may launder a negative observation.
        groups = machine.review_groups(finding["check"])
        addressed = any(spec["group"] in groups and any(
            (r["binding"].get("machine_evaluation") or {}).get("run_id") == evaluation["run_id"]
            and any(rel["finding_id"] == fid and rel["relationship"] in
                {"clarifies_indeterminate", "probable_false_positive"} for rel in a["machine_relationships"])
            for r, a in accepted.get(cid, [])) for cid, spec in criteria.items())
        if not addressed:
            unresolved_findings.append(fid)
    complete = not unresolved and not unresolved_findings and not violated
    blocked = evidence_validity != "valid" or bool(hard) or technical["state"] in {"failed", "candidate_mismatch", "invalid"} or bool(violated)
    status = "blocked" if blocked else "qualified" if complete and technical["state"] == "passed" else "unresolved"
    return {"evidence_validity": evidence_validity, "technical_gate": technical,
        "machine_summary": {"counts": dict(sorted(Counter(f["disposition"] for f in findings.values()).items())),
            "hard_findings": hard, "unresolved_findings": sorted(unresolved_findings)},
        "qualitative_review": {"state": "complete" if complete else "incomplete", "resolved_criteria": resolved,
            "violated_criteria": violated, "unresolved_criteria": unresolved, "human_escalations": escalated},
        "operator_approval": {"state": "not_provided"}, "replacement_qualification": status,
        "replacement_pass_candidate": status == "qualified", "phase_9_ready": False}


def verify_technical(candidate, capsule_path, archive_path):
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
    # by the gate-owned archive completion transition.
    gate = runpy.run_path(str(campaign.ROOT / "rebuild/validation/end-to-end/multi-repository/gate.py"))
    expected = gate["complete_evidence_archive"](prior,
        {"path": evidence["filename"], "candidate_head": candidate, "sha256": verification["archive_sha256"],
         "size_bytes": verification["archive_size_bytes"], "member_count": verification["member_count"]}, verification)
    review.require(capsule == expected, "final capsule differs from verified archive completion")
    return {"state": "passed" if capsule["phase_8_ready"] else "failed", "candidate_head": candidate,
        "capsule_sha256": operations.digest(data), "archive_sha256": verification["archive_sha256"],
        "verification": "maintained_independent_archive_and_capsule_contract", "execution": "reused"}


def qualify(root, evaluation_path, output, *, candidate, review_roots=(), capsule_path=None, archive_path=None):
    import campaign
    manifest = campaign.load_evidence_set(root)
    evaluation = evaluation_runs.load(evaluation_path)
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
        review.require(binding["candidate_head"] == candidate and binding["evidence_set"] == {"sha256": evidence_hash}, "review candidate/evidence mismatch")
        if binding["machine_evaluation"] is not None:
            review.require(binding["machine_evaluation"] == {"run_id": evaluation["run_id"],
                "sha256": campaign.harness.sha256(evaluation_path)}, "review binds a different machine run")
        review.require(review.criterion_specs(prep["index"], prep["rubric"]) == specs, "review criterion coverage differs from evidence")
        reviews.append(value)
        references.append({"run_id": value["reviewer"]["run_id"], "kind": value["reviewer"]["kind"],
            "review_sha256": operations.digest(files["recorded/review.json"]), "preparation_sha256": sha})
    review.require(len({r["run_id"] for r in references}) == len(references), "duplicate review run")
    consumed_ids = {r["run_id"] for r in references}
    for value in reviews:
        review.require(all(set(ids) <= consumed_ids for ids in value["resolves_review_runs"].values()),
            "human resolution references an unconsumed review run")
    technical = verify_technical(candidate, capsule_path, archive_path)
    result = {"kind": "phase8_dogfood_result", "schema_version": 2, "candidate_head": candidate,
        "evidence_set": evaluation["evidence_set"], "evaluator_revision": campaign.harness.git_head(campaign.ROOT),
        "policy": identity(), "evaluation_run": {"run_id": evaluation["run_id"], "sha256": campaign.harness.sha256(evaluation_path)},
        "qualitative_review_runs": sorted(references, key=lambda v: v["run_id"]), "run_nonce": secrets.token_hex(16),
        **combine(evaluation, specs, reviews, technical)}
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
    review.require(value.get("kind") == "phase8_dogfood_result" and value.get("schema_version") == 2
        and value.get("policy") == identity(), "invalid qualification policy/schema")
    for field, size in (("candidate_head", 40), ("evaluator_revision", 40), ("run_nonce", 32)):
        review.require(re.fullmatch(f"[0-9a-f]{{{size}}}", str(value.get(field, ""))), "invalid qualification identity")
    review.require(value.get("run_id") == machine.digest({k: v for k, v in value.items() if k != "run_id"}), "qualification run hash changed")
    q, m, t = value["qualitative_review"], value["machine_summary"], value["technical_gate"]
    complete = not (q["unresolved_criteria"] or q["violated_criteria"] or q["human_escalations"] or m["unresolved_findings"])
    blocked = value["evidence_validity"] != "valid" or bool(m["hard_findings"]) or bool(q["violated_criteria"]) or t["state"] in {"failed", "invalid", "candidate_mismatch"}
    expected = "blocked" if blocked else "qualified" if complete and t["state"] == "passed" else "unresolved"
    review.require(value["replacement_qualification"] == expected and value["replacement_pass_candidate"] is (expected == "qualified")
        and q["state"] == ("complete" if complete else "incomplete"), "qualification state contradicts mandatory evidence")
    if t["state"] == "passed":
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
