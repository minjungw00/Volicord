#!/usr/bin/env python3
"""Reproducible assertions for V05 inquiry frontier and session resume."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time


sys.dont_write_bytecode = True

from prototype import InquiryEngine, TERMINAL_OUTCOMES, v03


ROOT = Path(__file__).resolve().parents[4]
FIXTURE = ROOT / "rebuild/validation/inquiry/frontier-resume/fixtures/inquiry-scenario.json"
FACTS = ROOT / "rebuild/validation/inquiry/frontier-resume/fixtures/environment-facts.json"
PROTOTYPE = ROOT / "rebuild/validation/inquiry/frontier-resume/prototype.py"
V03_PROTOTYPE = ROOT / "rebuild/validation/canonical-context/portability/prototype.py"
ARTIFACT_PARENT = ROOT / "rebuild/.local/v05"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def response(frontier: dict[str, object], answer: str, outcome: str, turn: str, second: int) -> dict[str, object]:
    return {
        "question_id": frontier["question_id"],
        "expected_revision": frontier["question_revision"],
        "response": answer,
        "outcome": outcome,
        "user_turn_id": turn,
        "at": f"2026-08-08T20:45:{second:02d}Z",
    }


def run_helper(command: str, db: Path, output: Path) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        (
            sys.executable,
            str(PROTOTYPE),
            command,
            "--db",
            str(db),
            "--fixture",
            str(FIXTURE),
            "--facts",
            str(FACTS),
            "--output",
            str(output),
        ),
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def main() -> int:
    started = time.monotonic_ns()
    ARTIFACT_PARENT.mkdir(parents=True, exist_ok=True)
    artifact_root = Path(tempfile.mkdtemp(prefix="assertions-", dir=ARTIFACT_PARENT))
    database = artifact_root / "inquiry.sqlite3"
    engine = InquiryEngine.initialize(database, FIXTURE, FACTS)

    initial = engine.frontier()
    initial_ids = [item["question_id"] for item in initial]
    expected_initial = [
        "question-user-value",
        "question-unknown-recovery",
        "question-platform-scope",
        "question-future-sync",
        "question-enterprise-auth",
    ]
    require(initial_ids == expected_initial, "initial independent frontier order")
    require("question-language-fact" not in initial_ids, "deterministic fact must not be asked")
    fact_question = engine.question("question-language-fact")
    require(fact_question["body"]["status"] == "resolved_by_research", "fact resolved before user frontier")
    fact_record = engine.store.get_record("context-fact-question-language-fact")
    require(fact_record["body"]["value"] == "Rust", "explicit fact source value")
    require(fact_record["source_id"] == "source-deterministic-environment", "fact source provenance")
    require(initial == engine.frontier(), "initial frontier deterministic")

    by_id = {item["question_id"]: item for item in initial}
    value = by_id["question-user-value"]
    try:
        engine.respond(
            value["question_id"],
            value["question_revision"] + 1,
            "local-first",
            "answered",
            "user-turn-stale",
            "2026-08-08T20:44:59Z",
        )
    except ValueError as error:
        require("revision mismatch" in str(error), "stale response rejection reason")
    else:
        raise AssertionError("stale Question revision response was accepted")
    require(engine.store.get_record("source-user-turn-stale") is None, "rejected response must not persist")

    round_one = [
        response(by_id["question-user-value"], "local-first", "answered", "user-turn-101", 1),
        response(
            by_id["question-unknown-recovery"],
            "I do not know; research the workflow.",
            "resolved_by_research",
            "user-turn-102",
            2,
        ),
        response(by_id["question-platform-scope"], "single-owner", "answered", "user-turn-103", 3),
        response(by_id["question-future-sync"], "defer until local portability is proven", "deferred", "user-turn-104", 4),
        response(by_id["question-enterprise-auth"], "exclude from this product scope", "out_of_scope", "user-turn-105", 5),
    ]
    engine.respond_round(round_one)
    engine.close()

    after_round_one = InquiryEngine.resume(database, FIXTURE, FACTS)
    next_frontier = after_round_one.frontier()
    next_ids = [item["question_id"] for item in next_frontier]
    require(next_ids == ["question-storage-implementation", "question-resume-ux"], "dependency frontier after round one")
    require(after_round_one.question("question-team-permissions")["body"]["status"] == "superseded", "upstream supersession")
    require(after_round_one.question("question-user-value-rephrased")["body"]["status"] == "superseded", "rephrased answer suppression")
    require("question-user-value-rephrased" not in next_ids, "answered question must not repeat by rephrasing")

    user_decision = after_round_one.store.get_record("decision-question-user-value")
    value_question = after_round_one.question("question-user-value")
    user_response = after_round_one.store.get_record("context-response-user-turn-101")
    require(value_question["body"]["recommendation"] == "local-first", "agent recommendation preserved on Question")
    require(user_decision["body"]["choice"] == "local-first", "user choice preserved on Decision")
    require("recommendation" not in user_decision["body"], "recommendation separate from user choice")
    require(
        user_response["body"]["question_id"] == "question-user-value"
        and user_response["body"]["question_revision"] == value["question_revision"],
        "user response exact Question identity and revision",
    )

    paused_ids = after_round_one.pause("checkpoint-inquiry-pause", "2026-08-08T20:46:00Z")
    require(paused_ids == next_ids, "pause captures current frontier")
    checkpoint = after_round_one.store.get_record("checkpoint-inquiry-pause")
    require(checkpoint["body"] == {"state": "paused", "open_frontier": next_ids}, "pause checkpoint persisted")
    expected_frontier_bytes = v03.canonical_bytes(next_frontier)
    after_round_one.close()

    killed_output = artifact_root / "frontier-before-kill.json"
    killed = run_helper("frontier-and-kill", database, killed_output)
    require(killed.returncode < 0, "frontier helper must terminate by signal")
    require(killed_output.read_bytes() == expected_frontier_bytes, "frontier captured before termination")
    resumed_output = artifact_root / "frontier-after-restart.json"
    resumed = run_helper("frontier", database, resumed_output)
    require(resumed.returncode == 0, "frontier restart helper")
    require(resumed_output.read_bytes() == expected_frontier_bytes, "same open frontier after process restart")

    resumed_engine = InquiryEngine.resume(database, FIXTURE, FACTS)
    resumed_frontier = resumed_engine.frontier()
    require(v03.canonical_bytes(resumed_frontier) == expected_frontier_bytes, "in-process resume agrees")
    resumed_by_id = {item["question_id"]: item for item in resumed_frontier}
    round_two = [
        response(
            resumed_by_id["question-storage-implementation"],
            "delegate transactional details to the agent",
            "delegated",
            "user-turn-106",
            6,
        ),
        response(
            resumed_by_id["question-resume-ux"],
            "prototype the brief and timeline; I cannot know from text alone",
            "requires_prototype",
            "user-turn-107",
            7,
        ),
    ]
    resumed_engine.respond_round(round_two)
    resumed_engine.close()

    final_engine = InquiryEngine.resume(database, FIXTURE, FACTS)
    statuses = final_engine.statuses()
    require(final_engine.frontier() == [], "no frontier after every material branch terminates")
    require(final_engine.complete(), "all fixture Questions terminal")
    require(set(statuses.values()) == TERMINAL_OUTCOMES, "all accepted terminal outcomes exercised")
    require(len(statuses) == 10, "fixture terminates without a fixed question-count shortcut")
    unknown_response = final_engine.store.get_record("context-response-user-turn-102")
    require(unknown_response["body"]["outcome"] == "resolved_by_research", "do-not-know becomes research")
    delegated_decision = final_engine.store.get_record("decision-question-storage-implementation")
    require(delegated_decision["body"]["outcome"] == "delegated", "delegation preserved")
    final_engine.close()

    source_text = PROTOTYPE.read_text(encoding="utf-8")
    require(str(V03_PROTOTYPE.relative_to(ROOT)) in source_text, "V03 dependency must remain visibly experimental")
    require("max_questions" not in source_text and "question_limit" not in source_text, "fixed question-count limit present")

    summary = {
        "schema_version": 1,
        "status": "passed",
        "initial_frontier": initial_ids,
        "resumed_frontier": next_ids,
        "terminal_statuses": statuses,
        "terminal_outcomes": sorted(set(statuses.values())),
        "question_count": len(statuses),
        "process_termination_returncode": killed.returncode,
        "database_bytes": database.stat().st_size,
        "frontier_sha256": sha256(resumed_output),
        "duration_ms": round((time.monotonic_ns() - started) / 1_000_000, 3),
        "artifact_root": str(artifact_root),
    }
    (artifact_root / "summary.json").write_bytes(v03.canonical_bytes(summary))
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
