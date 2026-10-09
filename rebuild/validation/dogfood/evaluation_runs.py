"""Append-only machine evaluation storage; historical input is never upgraded."""
import json
from pathlib import Path

import machine_findings as machine


def policy_identity():
    import qualitative_review
    import harness
    policy = {"collection_runs_sha256": harness.sha256(Path(__file__).with_name("collection_runs.py")), "purpose_sha256": harness.sha256(Path(__file__).with_name("evidence_purpose.py")), "machine_version": machine.POLICY_VERSION, "authority": machine.POLICY,
        "machine_policy_sha256": harness.sha256(Path(machine.__file__)),
        "interaction_diagnostics_sha256": harness.sha256(Path(__file__).with_name("interaction_diagnostics.py")),
        "answer_projection_sha256": harness.sha256(Path(__file__).with_name("answer_projection.py")),
        "review_captures_sha256": harness.sha256(Path(__file__).with_name("review_captures.py")),
        "review_explanations_sha256": harness.sha256(Path(__file__).with_name("review_explanations.py")),
        "review_operations_sha256": harness.sha256(Path(__file__).with_name("review_operations.py")),
        "explanation_evidence_sha256": harness.sha256(Path(__file__).with_name("explanation_evidence.py")),
        "codex_events_sha256": harness.sha256(Path(__file__).with_name("codex_events.py")),
        "answer_observations_sha256": harness.sha256(Path(__file__).with_name("answer_observations.py")),
        "recorded_action_evidence_sha256": harness.sha256(harness.ROOT / "rebuild/validation/shared/recorded_action_evidence.py"),
        "workload_intents_sha256": harness.sha256(Path(__file__).with_name("workload_intents.py")),
        "rubric": qualitative_review.rubric(harness.load_definition())}
    return {"revision": "evidence-evaluation-16", "sha256": machine.digest(policy)}


def historical_reference(path, candidate, evidence):
    from review_operations import bounded_read, digest
    data = bounded_read(path)
    value = json.loads(data)
    # Historical schemas/policies are opaque comparison inputs, never qualification.
    if (value.get("candidate_head") != candidate or value.get("evidence_set") != evidence
        or value.get("run_id") != machine.digest({k: v for k, v in value.items() if k != "run_id"})):
        raise ValueError("historical evaluation cannot rebind candidate or evidence")
    return {"run_id": value["run_id"], "sha256": digest(data)}


def publish(destination, result):
    from review_operations import publish_directory, encoded, digest
    machine.validate_run(result)
    data = encoded(result)
    receipt = {"kind": "dogfood_evaluation_receipt", "run_id": result["run_id"],
        "evaluation_sha256": digest(data)}
    publish_directory(destination, {"evaluation.json": data, "receipt.json": encoded(receipt)})
    return destination.absolute() / "evaluation.json"


def load(path, *, for_review=False):
    """Review verifies the recorded policy; qualification requires current policy.

    Both use the same current machine schema, finding semantics and publication
    integrity checks. Reading for review cannot attest current-policy equivalence.
    """
    from review_operations import bounded_read, digest
    if path.name != "evaluation.json" or path.parent.with_name(path.parent.name + ".publication-lock").exists():
        raise ValueError("evaluation is not a completed immutable run")
    data = bounded_read(path)
    value = json.loads(data)
    machine.validate_run(value, require_current_policy=not for_review)
    receipt = json.loads(bounded_read(path.with_name("receipt.json")))
    if receipt != {"kind": "dogfood_evaluation_receipt", "run_id": value["run_id"],
        "evaluation_sha256": digest(data)}:
        raise ValueError("evaluation publication hash/identity changed")
    return value
