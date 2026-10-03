"""Durable, self-contained discovery lineage for immutable Dogfood results."""
from __future__ import annotations

import json
from pathlib import Path
import re

import evaluation_runs
import machine_findings as machine
import qualification_policy
import qualitative_review as review
import review_operations as operations


SCHEMA_VERSION = 2


def _binding(data):
    return {"bytes": len(data), "sha256": operations.digest(data)}


def _relative_files(root, names):
    result = {}
    for name in names:
        path = operations.safe_path(root, name)
        result[name] = operations.bounded_read(path)
    return result


def _evaluation_specs(evaluation):
    """Reconstruct current naturalistic scope without campaign/source access.

    Naturalistic selection has no preassigned authority obligations; reviewers
    declare additional actual outcomes in their recorded reviews. All other
    criterion-generating fields are already preserved in the machine run.
    """
    import harness
    qualification_policy.validate_topology(evaluation)
    index = {
        "samples": [{"sample_id": work["work_slot_id"],
            "resume_pair": work["resume_pair"], "workload_intent": work["workload_intent"],
            "authority_obligations": []} for work in evaluation["works"]],
        "journey_samples": [{"sample_id": journey["journey_id"]}
            for journey in evaluation["journeys"]],
        "cli_samples": [{"sample_id": kind} for kind in harness.CLASSES],
        "live_viewer_sample": "journey-volicord",
    }
    return review.criterion_specs(index, review.rubric(harness.load_definition()))


def publish(campaign_root, evaluation_path, review_roots, qualification_path,
            output=None, approval_path=None):
    """Copy exact immutable results into a discoverable sibling package."""
    import campaign

    campaign_root = campaign_root.resolve()
    manifest = campaign.load_evidence_set(campaign_root)
    evidence_data = operations.bounded_read(campaign_root / "evidence-set.json")
    evidence_sha = operations.digest(evidence_data)
    evaluation_path = evaluation_path.resolve()
    evaluation_data = operations.bounded_read(evaluation_path)
    evaluation = evaluation_runs.load(evaluation_path)
    review.require(evaluation["candidate_head"] == manifest["candidate_head"]
        and evaluation["evidence_set"] == {"path": "evidence-set.json", "sha256": evidence_sha},
        "evaluation does not bind the campaign candidate/evidence set")

    qualification_path = qualification_path.resolve()
    qualification_data = operations.bounded_read(qualification_path)
    qualification = qualification_policy.verify_qualification(qualification_path)
    review.require(qualification["candidate_head"] == manifest["candidate_head"]
        and qualification["evidence_set"] == evaluation["evidence_set"]
        and qualification["evaluation_run"] == {"run_id": evaluation["run_id"],
            "sha256": operations.digest(evaluation_data)},
        "qualification does not bind the selected campaign/evaluation")

    files = {
        "source/evidence-set.json": evidence_data,
        "evaluation/evaluation.json": evaluation_data,
        "evaluation/receipt.json": operations.bounded_read(evaluation_path.with_name("receipt.json")),
        "qualification/qualification.json": qualification_data,
    }
    reviews, observed_references = [], []
    for review_root in review_roots:
        review_root = review_root.resolve()
        preparation, preparation_sha, package = operations.load_package(review_root)
        recorded = operations.recorded_files(review_root, preparation, preparation_sha)
        review.require(recorded, "result lineage accepts only immutable recorded reviews")
        value = json.loads(recorded["recorded/review.json"])
        run_id = value["reviewer"]["run_id"]
        prefix = f"reviews/{run_id}"
        package_names = {*package["artifacts"], "package.json"}
        for name, data in _relative_files(review_root, package_names).items():
            files[f"{prefix}/{name}"] = data
        for name, data in recorded.items():
            files[f"{prefix}/{name}"] = data
        reference = {"run_id": run_id, "kind": value["reviewer"]["kind"],
            "review_sha256": operations.digest(recorded["recorded/review.json"]),
            "preparation_sha256": preparation_sha}
        observed_references.append(reference)
        reviews.append({**reference, "root": prefix,
            "review_path": f"{prefix}/recorded/review.json",
            "receipt_path": f"{prefix}/recorded/receipt.json",
            "package_id": package["package_id"]})
    observed_references.sort(key=lambda item: item["run_id"])
    review.require(observed_references == qualification["qualitative_review_runs"],
        "published review set differs from qualification inputs")

    approval = None
    if approval_path is not None:
        approval_path = approval_path.resolve()
        approval_data = operations.bounded_read(approval_path)
        approval_value = qualification_policy.verify_approval(approval_path, qualification_path)
        files["approval/approval.json"] = approval_data
        files["approval/qualification.json"] = operations.bounded_read(
            approval_path.with_name("qualification.json"))
        approval = {"run_id": approval_value["run_id"], "path": "approval/approval.json",
            "sha256": operations.digest(approval_data),
            "qualification_run_id": approval_value["qualification_run_id"],
            "operator_approval": approval_value["operator_approval"]}

    index_without_id = {"kind": "dogfood_result_lineage", "schema_version": SCHEMA_VERSION,
        "candidate_head": manifest["candidate_head"],
        "campaign": {"campaign_id": manifest["campaign_id"],
            "evidence_set_path": "source/evidence-set.json", "evidence_set_sha256": evidence_sha},
        "evaluation": {"run_id": evaluation["run_id"], "path": "evaluation/evaluation.json",
            "sha256": operations.digest(evaluation_data), "receipt_path": "evaluation/receipt.json",
            "evaluator_revision": evaluation["evaluator_revision"], "policy": evaluation["policy"]},
        "qualitative_reviews": sorted(reviews, key=lambda item: item["run_id"]),
        "qualification": {"run_id": qualification["run_id"],
            "path": "qualification/qualification.json", "sha256": operations.digest(qualification_data),
            "policy": qualification["policy"],
            "replacement_qualification": qualification["replacement_qualification"],
            "phase_9_ready": qualification["phase_9_ready"]},
        "approval": approval,
        "discovery": {"default_root": "<campaign-parent>/results/<qualification-run-id>",
            "external_staging_paths_required": False}}
    lineage_id = machine.digest(index_without_id)
    index = {**index_without_id, "lineage_id": lineage_id}
    index_data = operations.encoded(index)
    inventory = {name: _binding(data) for name, data in sorted(files.items())}
    receipt = {"kind": "dogfood_result_lineage_receipt", "schema_version": SCHEMA_VERSION,
        "lineage_id": lineage_id, "index_sha256": operations.digest(index_data),
        "artifacts": inventory}
    files.update({"index.json": index_data, "receipt.json": operations.encoded(receipt)})
    destination = (output.absolute() if output is not None else
        campaign_root.parent / "results" / qualification["run_id"])
    review.require(not destination.resolve().is_relative_to(campaign_root),
        "result lineage must not mutate the immutable campaign")

    # Recheck every source identity immediately before create-only publication.
    campaign.load_evidence_set(campaign_root)
    review.require(operations.bounded_read(campaign_root / "evidence-set.json") == evidence_data
        and operations.bounded_read(evaluation_path) == evaluation_data
        and operations.bounded_read(qualification_path) == qualification_data,
        "result source changed during lineage publication")
    operations.publish_directory(destination, files)
    return {"state": "published", "lineage_root": str(destination),
        "lineage_id": lineage_id, "candidate_head": manifest["candidate_head"],
        "qualification_run_id": qualification["run_id"],
        "approval_run_id": approval["run_id"] if approval else None}


def verify(root):
    """Verify a copied package without consulting original campaign/staging paths."""
    root = root.resolve()
    index_data = operations.bounded_read(root / "index.json")
    index = json.loads(index_data)
    receipt = json.loads(operations.bounded_read(root / "receipt.json"))
    review.require(isinstance(index, dict) and index.get("kind") == "dogfood_result_lineage"
        and index.get("schema_version") == SCHEMA_VERSION
        and re.fullmatch(r"[0-9a-f]{64}", str(index.get("lineage_id", ""))),
        "invalid result lineage index")
    expected_id = machine.digest({key: value for key, value in index.items() if key != "lineage_id"})
    review.require(index["lineage_id"] == expected_id
        and receipt.get("kind") == "dogfood_result_lineage_receipt"
        and receipt.get("schema_version") == SCHEMA_VERSION
        and receipt.get("lineage_id") == expected_id
        and receipt.get("index_sha256") == operations.digest(index_data),
        "result lineage index/receipt identity changed")
    inventory = receipt.get("artifacts")
    review.require(isinstance(inventory, dict) and inventory, "result lineage inventory is empty")
    actual_names = {path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file()}
    review.require(actual_names == {*inventory, "index.json", "receipt.json"},
        "result lineage contains missing or unindexed files")
    for name, binding in inventory.items():
        data = operations.bounded_read(operations.safe_path(root, name))
        review.require(binding == _binding(data), "result lineage artifact changed")

    evidence_data = operations.bounded_read(operations.safe_path(
        root, index["campaign"]["evidence_set_path"]))
    review.require(operations.digest(evidence_data) == index["campaign"]["evidence_set_sha256"],
        "result lineage evidence-set binding changed")
    evidence_set = json.loads(evidence_data)
    review.require(evidence_set.get("kind") == "dogfood_evidence_set"
        and evidence_set.get("candidate_head") == index["candidate_head"],
        "result lineage evidence-set identity changed")
    evaluation_path = operations.safe_path(root, index["evaluation"]["path"])
    evaluation = evaluation_runs.load(evaluation_path)
    review.require(index["evaluation"] == {"run_id": evaluation["run_id"],
        "path": "evaluation/evaluation.json", "sha256": operations.digest(operations.bounded_read(evaluation_path)),
        "receipt_path": "evaluation/receipt.json", "evaluator_revision": evaluation["evaluator_revision"],
        "policy": evaluation["policy"]}, "result lineage evaluation binding changed")
    review.require(evaluation["candidate_head"] == index["candidate_head"]
        and evaluation["evidence_set"] == {"path": "evidence-set.json",
            "sha256": index["campaign"]["evidence_set_sha256"]}
        and evidence_set.get("campaign_id") == index["campaign"]["campaign_id"],
        "result lineage evaluation candidate/evidence changed")

    specs = _evaluation_specs(evaluation)
    review_refs, review_values = [], []
    for item in index["qualitative_reviews"]:
        review_root = operations.safe_path(root, item["root"] + "/package.json").parent
        preparation, sha, package = operations.load_package(review_root)
        import review_explanations
        import review_captures
        for entry in preparation['index']['evidence'].values():
            content = operations.bounded_read(operations.safe_path(review_root, entry['path']))
            if entry['surface'] == 'live_viewer_observation':
                import viewer_observation
                observed = json.loads(content)
                for display in observed['contexts']:
                    viewer_observation.for_manifest(evidence_set, display, entry['locale'])
            if entry['surface'] == review_explanations.SURFACE:
                lifecycle = review_explanations.validate(content)
                review_explanations.verify_manifest(lifecycle, evidence_set)
            if entry['surface'] in review_captures.CAPTURE_SURFACES:
                capture = review_captures.validate(content)
                origin = capture['origin']
                review.require(evidence_set['artifacts'].get(origin['path']) == {
                    'bytes': origin['raw_bytes'], 'sha256': origin['raw_sha256']},
                    'copied returned answer source binding changed')
                for returned in capture['records']:
                    if (returned['semantic_role'] != 'volicord_operation'
                            or returned['body']['state'] != 'retained'):
                        continue
                    meaning = returned['body']['value']['returned_meaning']
                    if meaning is None or returned['transport'] == 'mcp' and returned['outcome'] != 'succeeded':
                        continue
                    observed = [v for v in evidence_set['explanation_evidence']['measured_observations']
                        if v['raw_capture_sha256'] == origin['raw_sha256']
                        and v['sequence'] == returned['sequence'] and v['call_id'] == returned['call_id']
                        and v['transport'] == returned['transport']]
                    import campaign
                    meaning_sha = operations.digest(campaign.json_bytes(meaning))
                    review.require(observed and all(v['review_meaning_sha256'] == meaning_sha
                        and v['operation'] == returned['operation'] and v['turn_id'] == returned['turn_id']
                        and v['requested_language'] == returned['requested_language'] for v in observed),
                        'copied returned meaning differs from observed source index')
        recorded = operations.recorded_files(review_root, preparation, sha)
        review.require(recorded, "result lineage requires recorded review evidence")
        value = json.loads(recorded["recorded/review.json"])
        binding = value["binding"]
        review.require(binding["candidate_head"] == index["candidate_head"]
            and binding["evidence_set"] == {"sha256": index["campaign"]["evidence_set_sha256"]},
            "result lineage review candidate/evidence changed")
        if binding["machine_evaluation"] is not None:
            review.require(binding["machine_evaluation"] == {
                "run_id": evaluation["run_id"], "sha256": index["evaluation"]["sha256"],
                "recorded_policy": evaluation["policy"],
                "policy_verification": "recorded_identity_not_current_equivalence"},
                "result lineage review binds a different machine run")
        review.require(operations.encoded(sorted(
            review.criterion_specs(preparation["index"], preparation["rubric"]),
            key=lambda spec: spec["criterion_id"])) == operations.encoded(sorted(
                specs, key=lambda spec: spec["criterion_id"])),
            "result lineage review criterion coverage differs from evaluation")
        review_values.append(value)
        expected = {"run_id": value["reviewer"]["run_id"], "kind": value["reviewer"]["kind"],
            "review_sha256": operations.digest(recorded["recorded/review.json"]),
            "preparation_sha256": sha}
        review_refs.append(expected)
        review.require(item == {**expected, "root": item["root"],
            "review_path": item["root"] + "/recorded/review.json",
            "receipt_path": item["root"] + "/recorded/receipt.json",
            "package_id": package["package_id"]}, "result lineage review binding changed")
    consumed_ids = {item["run_id"] for item in review_refs}
    review.require(len(consumed_ids) == len(review_refs), "duplicate result lineage review run")
    for value in review_values:
        review.require(all(set(ids) <= consumed_ids for ids in value["resolves_review_runs"].values()),
            "result lineage human resolution references an unconsumed review run")

    qualification_path = operations.safe_path(root, index["qualification"]["path"])
    qualification_data = operations.bounded_read(qualification_path)
    qualification = json.loads(qualification_data)
    qualification_policy.validate_result(qualification)
    review.require(qualification["candidate_head"] == index["candidate_head"]
        and qualification["evidence_set"]["sha256"] == index["campaign"]["evidence_set_sha256"]
        and qualification["naturalistic_evidence"]["naturalistic_resource"]
            == evidence_set.get("naturalistic_memory_evidence")
        and evidence_set.get("live_evidence_obligations", {}).get("naturalistic_resource")
            == evidence_set.get("naturalistic_memory_evidence")
        and qualification["evaluation_run"] == {"run_id": evaluation["run_id"],
            "sha256": operations.digest(operations.bounded_read(evaluation_path))}
        and qualification["qualitative_review_runs"] == sorted(review_refs, key=lambda item: item["run_id"]),
        "result lineage qualification inputs changed")
    expected_qualification = {"run_id": qualification["run_id"],
        "path": "qualification/qualification.json", "sha256": operations.digest(qualification_data),
        "policy": qualification["policy"],
        "replacement_qualification": qualification["replacement_qualification"],
        "phase_9_ready": qualification["phase_9_ready"]}
    review.require(index["qualification"] == expected_qualification,
        "result lineage qualification binding changed")
    # Replay the derived semantics, retaining the separately verified technical
    # summary as input. This checks internal agreement, not review truth or
    # external authentication, and never exercises operator authorization.
    import resource_observer
    resource_observer.validate(evidence_set["naturalistic_memory_evidence"],
        evidence_set["candidate_artifacts"]["volicord-mcp"]["sha256"])
    replayed = qualification_policy.combine(evaluation, specs, review_values,
        qualification["technical_gate"], evidence_validity=qualification["evidence_validity"],
        naturalistic_resource=evidence_set.get("naturalistic_memory_evidence"))
    review.require(operations.encoded(replayed) == operations.encoded({
        key: qualification[key] for key in replayed}),
        "result lineage qualification contradicts evaluation and recorded reviews")

    approval = index["approval"]
    if approval is not None:
        approval_data = operations.bounded_read(operations.safe_path(root, approval["path"]))
        value = json.loads(approval_data)
        expected = qualification_policy.approval_value(qualification, qualification_data,
            value.get("operator", {}).get("identity"), "approve-phase-9")
        review.require(qualification["replacement_qualification"] == "qualified"
            and value.get("phase_9_ready") is True
            and value == expected and approval == {"run_id": value["run_id"],
            "path": "approval/approval.json", "sha256": operations.digest(approval_data),
            "qualification_run_id": value["qualification_run_id"],
            "operator_approval": value["operator_approval"]}
            and operations.bounded_read(operations.safe_path(root, "approval/qualification.json")) == qualification_data,
            "result lineage approval binding changed")
    return {"state": "verified", "lineage_id": index["lineage_id"],
        "candidate_head": index["candidate_head"], "evaluation_run_id": evaluation["run_id"],
        "review_run_ids": [item["run_id"] for item in review_refs],
        "qualification_run_id": qualification["run_id"],
        "approval_run_id": approval["run_id"] if approval else None,
        "external_staging_paths_used": False}
