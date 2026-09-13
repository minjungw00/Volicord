"""Append-only machine evaluation storage; historical input is never upgraded."""
import json
from pathlib import Path

import machine_findings as machine


def policy_identity():
    import qualitative_review
    import harness
    policy = {"machine_version": machine.POLICY_VERSION,
        "machine_policy_sha256": harness.sha256(Path(machine.__file__)),
        "rubric": qualitative_review.rubric(harness.load_definition())}
    return {"revision": "evidence-evaluation-1", "sha256": machine.digest(policy)}


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


def load(path):
    from review_operations import bounded_read, digest
    if path.name != "evaluation.json" or path.parent.with_name(path.parent.name + ".publication-lock").exists():
        raise ValueError("evaluation is not a completed immutable run")
    data = bounded_read(path)
    value = json.loads(data)
    machine.validate_run(value)
    receipt = json.loads(bounded_read(path.with_name("receipt.json")))
    if receipt != {"kind": "dogfood_evaluation_receipt", "run_id": value["run_id"],
        "evaluation_sha256": digest(data)}:
        raise ValueError("evaluation publication hash/identity changed")
    return value
