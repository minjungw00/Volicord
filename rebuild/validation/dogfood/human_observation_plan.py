"""Bounded changed-display preparation; supplies no experience or qualification."""
import hashlib
import json
from pathlib import Path
import subprocess

import qualitative_review as review
import review_operations as operations

ROOT = Path(__file__).resolve().parents[3]
FIXTURE = Path(__file__).with_name("fixtures") / "human-observation-surfaces.json"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def plan(candidate, executables, changed_paths, inventory):
    """Pure preparation over authored affected surfaces and actual changed paths."""
    review.require(candidate != inventory["diagnostic_candidate"],
        "changed candidate preparation cannot relabel the diagnostic candidate")
    blocks = []
    for item in inventory["observation_blocks"]:
        changed = sorted(set(item["affected_paths"]) & set(changed_paths))
        if changed:
            blocks.append({**item, "actually_changed_paths": changed,
                "locales": ["en", "ko"], "human_evidence": "unobserved"})
    review.require(blocks, "no affected Viewer experience found")
    return {"kind": "dogfood_human_observation_preparation", "schema_version": 2,
        "candidate_head": candidate, "executables": executables,
        "diagnostic_candidate": inventory["diagnostic_candidate"],
        "historical_evidence_use": "diagnostic_and_regression_input_only",
        "question_count_available": False,
        "capture_before_questions": True,
        "observation_blocks": blocks,
        "locale_reuse": "SAME AS ENGLISH only after actual Korean context and personal inspection",
        "operator_mapping": "derive formal fields from clear answers; clarify ambiguity, contradiction or missing experience only",
        "unreported": "not_reported; never none or success",
        "separate_evidence": inventory["separate_evidence"],
        "unaffected_historical_claims": inventory["unaffected_historical_claims"],
        "replacement_qualification": "not_run", "phase_9_ready": False}


def prepare(bin_dir, output):
    review.require(not git("status", "--porcelain"), "preparation requires the clean final candidate")
    candidate = git("rev-parse", "HEAD")
    inventory = json.loads(FIXTURE.read_bytes())
    changed = git("diff", "--name-only", inventory["diagnostic_candidate"], candidate).splitlines()
    executables = {name: digest(bin_dir / name)
        for name in ("volicord", "volicord-mcp", "volicord-viewer")}
    value = plan(candidate, executables, changed, inventory)
    value["inputs"] = {str(p.relative_to(ROOT)): digest(p)
        for p in (Path(__file__), FIXTURE, ROOT / "rebuild/docs/design/qualitative-review.md")}
    review.require(git("rev-parse", "HEAD") == candidate and not git("status", "--porcelain")
        and executables == {name: digest(bin_dir / name) for name in executables},
        "candidate or executable changed during observation preparation")
    operations.publish_directory(output, {"preparation.json": operations.encoded(value)})
    return value


def require_contexts(value, contexts, candidate, viewer_sha256):
    """Required changed views must exist before the person is questioned."""
    review.require(value["candidate_head"] == candidate
        and value["executables"]["volicord-viewer"] == viewer_sha256,
        "observation plan candidate/executable mismatch")
    inventory = json.loads(FIXTURE.read_bytes())
    changed = git("diff", "--name-only", inventory["diagnostic_candidate"], candidate).splitlines()
    expected = plan(candidate, value["executables"], changed, inventory)
    expected["inputs"] = {str(p.relative_to(ROOT)): digest(p)
        for p in (Path(__file__), FIXTURE, ROOT / "rebuild/docs/design/qualitative-review.md")}
    review.require(value == expected, "changed-surface plan was altered or belongs to another contract")
    return block_readiness(value["observation_blocks"], contexts)


def block_readiness(blocks, contexts):
    """Absence is a local evidence gap, never proof of inapplicability."""
    result = {}
    for locale in ("en", "ko"):
        displays = [c["context"] for c in contexts[locale]]
        result[locale] = []
        for block in blocks:
            missing = [surface for surface in block["required_views"]
                if not any(all(c["view"].get(k) == v for k, v in surface.items()) for c in displays)]
            reasons = ["required_view_missing"] if missing else []
            if block["id"] == "multi-work" and len({c["selected_work"] for c in displays
                    if c["view"].get("view") == "work" and c["selected_work"] is not None}) < 2:
                reasons.append("two_distinct_works_missing")
            if block["id"] == "work" and not any(c["view"].get("view") == "work"
                    and c["selected_work"] is not None for c in displays):
                reasons.append("selected_work_missing")
            if block["id"] == "decision" and not any(c["view"].get("view") == "decisions"
                    and c["selected_decision"] is not None for c in displays):
                reasons.append("selected_decision_missing_applicability_unresolved")
            result[locale].append({"id": block["id"], "claims": block["claims"],
                "state": "insufficient_evidence" if reasons else "ready", "reasons": reasons,
                "missing_views": missing})
    return result


def validate_scope(scope, contexts, candidate, viewer_sha256):
    """Copied packages recompute readiness without the original Campaign or Git."""
    review.require(isinstance(scope, dict) and set(scope) == {"plan", "readiness"},
        "invalid block applicability scope")
    inventory = json.loads(FIXTURE.read_bytes())
    value = scope["plan"]
    if value is None:
        blocks = inventory["observation_blocks"]
    else:
        review.require(value.get("schema_version") == 2 and value.get("candidate_head") == candidate
            and value.get("executables", {}).get("volicord-viewer") == viewer_sha256,
            "observation scope candidate/executable mismatch")
        blocks = value["observation_blocks"]
        authored = {b["id"]: b for b in inventory["observation_blocks"]}
        review.require(blocks and len({b["id"] for b in blocks}) == len(blocks), "invalid observation block inventory")
        for block in blocks:
            review.require(block["id"] in authored and
                {k: block[k] for k in authored[block["id"]]} == authored[block["id"]],
                "observation block differs from authored contract")
        review.require(value["inputs"] == {str(p.relative_to(ROOT)): digest(p)
            for p in (Path(__file__), FIXTURE, ROOT / "rebuild/docs/design/qualitative-review.md")},
            "observation scope contract identity mismatch")
    if value is not None:
        changed = {path for block in blocks for path in block["actually_changed_paths"]}
        expected = plan(candidate, value["executables"], changed, inventory)
        expected["inputs"] = value["inputs"]
        review.require(value == expected, "copied changed-surface plan differs from authored routing")
    review.require(scope["readiness"] == block_readiness(blocks, contexts), "block applicability/context mismatch")
    return blocks


def prepared_claims(scope, locale, contexts):
    displays = [c["context"] for c in contexts]
    claims = {claim for b in scope["readiness"][locale] if b["state"] == "ready" for claim in b["claims"]}
    if len({c["selected_work"] for c in displays
            if c["view"].get("view") == "work" and c["selected_work"] is not None}) < 2:
        claims.discard("multiple_work_comprehension")
    return sorted(claims)


def require_claim_context(scope, contexts, locale, claim, state):
    """Shared/grouped questions cannot waive an individual claim's subject context."""
    if state in {"satisfied", "violated"}:
        local = contexts[locale] if isinstance(contexts, dict) else contexts
        review.require(claim in prepared_claims(scope, locale, local),
            "mapped claim lacks its prepared block context")


def main():
    import argparse
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--bin-dir", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    value = prepare(args.bin_dir.resolve(), args.output.absolute())
    print(json.dumps({"candidate_head": value["candidate_head"], "output": str(args.output),
        "human_evidence": "unobserved", "replacement_qualification": "not_run"}, indent=2))


if __name__ == "__main__":
    main()
