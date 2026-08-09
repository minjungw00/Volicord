#!/usr/bin/env python3
"""Disposable V05 inquiry frontier using committed V03 experiment storage."""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
from pathlib import Path
import signal
import sys
from typing import Any, Iterable


sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[4]
V03_PROTOTYPE = ROOT / "rebuild/validation/canonical-context/portability/prototype.py"
V03_SPEC = importlib.util.spec_from_file_location("v03_experimental_storage", V03_PROTOTYPE)
if V03_SPEC is None or V03_SPEC.loader is None:
    raise RuntimeError("committed V03 experimental storage could not be loaded")
v03 = importlib.util.module_from_spec(V03_SPEC)
sys.modules[V03_SPEC.name] = v03
V03_SPEC.loader.exec_module(v03)


TERMINAL_OUTCOMES = {
    "answered",
    "delegated",
    "resolved_by_research",
    "requires_prototype",
    "deferred",
    "out_of_scope",
    "superseded",
}
USER_OUTCOMES = TERMINAL_OUTCOMES - {"superseded"}
PREREQUISITE_SATISFIED = {"answered", "delegated", "resolved_by_research", "requires_prototype"}


def load_fixture(path: Path) -> dict[str, Any]:
    fixture = v03.read_json(path)
    if fixture.get("schema_version") != 1:
        raise ValueError("unsupported inquiry fixture schema")
    identifiers = [item["id"] for item in fixture["questions"]]
    if len(identifiers) != len(set(identifiers)):
        raise ValueError("duplicate Question identity")
    return fixture


class DeterministicFactSource:
    def __init__(self, path: Path):
        self.path = path
        self.data = v03.read_json(path)
        if self.data.get("schema_version") != 1:
            raise ValueError("unsupported fact source schema")

    @property
    def source_id(self) -> str:
        return self.data["source_id"]

    def resolve(self, key: str) -> Any:
        if key not in self.data["facts"]:
            raise KeyError(key)
        return self.data["facts"][key]


class InquiryEngine:
    """Deterministic frontier experiment with no question-count limit."""

    def __init__(self, store: Any, fixture_path: Path, fact_path: Path):
        self.store = store
        self.fixture_path = fixture_path
        self.fixture = load_fixture(fixture_path)
        self.fact_source = DeterministicFactSource(fact_path)
        self.project_id = self.fixture["project"]["id"]
        self.spec_by_id = {item["id"]: item for item in self.fixture["questions"]}

    @classmethod
    def initialize(cls, db_path: Path, fixture_path: Path, fact_path: Path) -> "InquiryEngine":
        fixture = load_fixture(fixture_path)
        store = v03.SqliteCanonicalStore(db_path)
        store.create_project(fixture["project"], fixture["clone_path"])
        fact_data = v03.read_json(fact_path)
        store.add_record(
            fixture["project"]["id"],
            {
                "id": fact_data["source_id"],
                "kind": "source",
                "body": {
                    "source_kind": "deterministic_fixture",
                    "locator": fact_path.name,
                    "source_revision": fact_data["source_revision"],
                },
                "author_kind": "observed",
                "source_id": None,
                "created_at": "2026-08-08T20:40:01Z",
            },
        )
        for index, spec in enumerate(fixture["questions"], start=1):
            body = {
                "text": spec["text"],
                "semantic_key": spec["semantic_key"],
                "category": spec["category"],
                "why_now": spec["why_now"],
                "options": spec["options"],
                "recommendation": spec["recommendation"],
                "prerequisites": spec["prerequisites"],
                "order": spec["order"],
                "status": "open",
            }
            if "fact_key" in spec:
                body["fact_key"] = spec["fact_key"]
            if "supersede_when" in spec:
                body["supersede_when"] = spec["supersede_when"]
            store.add_record(
                fixture["project"]["id"],
                {
                    "id": spec["id"],
                    "kind": "question",
                    "body": body,
                    "author_kind": "agent",
                    "source_id": fact_data["source_id"] if spec["category"] == "fact" else None,
                    "created_at": f"2026-08-08T20:41:{index:02d}Z",
                },
            )
        return cls(store, fixture_path, fact_path)

    @classmethod
    def resume(cls, db_path: Path, fixture_path: Path, fact_path: Path) -> "InquiryEngine":
        return cls(v03.SqliteCanonicalStore(db_path), fixture_path, fact_path)

    def close(self) -> None:
        self.store.close()

    def question(self, question_id: str) -> dict[str, Any]:
        record = self.store.get_record(question_id)
        if record is None or record["kind"] != "question":
            raise KeyError(question_id)
        return record

    def _revise_question(self, question_id: str, updates: dict[str, Any], at: str) -> None:
        record = self.question(question_id)
        body = dict(record["body"])
        body.update(updates)
        self.store.revise(question_id, body, at)

    def _decision_choice(self, question_id: str) -> Any | None:
        for record in self.store.bundle()["records"]:
            if record["kind"] != "decision":
                continue
            if record["body"].get("question_id") == question_id:
                return record["body"].get("choice")
        return None

    def resolve_deterministic_facts(self) -> None:
        for spec in self.fixture["questions"]:
            if spec["category"] != "fact":
                continue
            record = self.question(spec["id"])
            if record["body"]["status"] != "open":
                continue
            value = self.fact_source.resolve(spec["fact_key"])
            context_id = f"context-fact-{spec['id']}"
            self.store.add_record(
                self.project_id,
                {
                    "id": context_id,
                    "kind": "context_item",
                    "body": {
                        "context_kind": "fact",
                        "fact_key": spec["fact_key"],
                        "value": value,
                        "question_id": spec["id"],
                        "question_revision": record["revision"],
                    },
                    "author_kind": "observed",
                    "source_id": self.fact_source.source_id,
                    "created_at": "2026-08-08T20:42:00Z",
                },
            )
            self._revise_question(
                spec["id"],
                {
                    "status": "resolved_by_research",
                    "resolution_record_id": context_id,
                    "response_question_revision": record["revision"],
                },
                "2026-08-08T20:42:01Z",
            )

    def _apply_upstream_supersession(self) -> None:
        for spec in self.fixture["questions"]:
            rule = spec.get("supersede_when")
            if not rule:
                continue
            record = self.question(spec["id"])
            if record["body"]["status"] != "open":
                continue
            if self._decision_choice(rule["question_id"]) == rule["choice_equals"]:
                self._revise_question(
                    spec["id"],
                    {"status": "superseded", "superseded_by_question_id": rule["question_id"]},
                    "2026-08-08T20:43:00Z",
                )

    def _suppress_rephrased_answers(self) -> None:
        answered_keys = {
            record["body"]["semantic_key"]
            for record in (self.question(spec["id"]) for spec in self.fixture["questions"])
            if record["body"]["status"] == "answered"
        }
        for spec in self.fixture["questions"]:
            record = self.question(spec["id"])
            if record["body"]["status"] == "open" and spec["semantic_key"] in answered_keys:
                self._revise_question(
                    spec["id"],
                    {"status": "superseded", "supersession_reason": "answered_semantic_key"},
                    "2026-08-08T20:43:01Z",
                )

    def frontier(self) -> list[dict[str, Any]]:
        self.resolve_deterministic_facts()
        self._apply_upstream_supersession()
        self._suppress_rephrased_answers()
        results: list[dict[str, Any]] = []
        for spec in self.fixture["questions"]:
            record = self.question(spec["id"])
            if record["body"]["status"] != "open":
                continue
            prerequisites = [self.question(item)["body"]["status"] for item in spec["prerequisites"]]
            if not all(status in PREREQUISITE_SATISFIED for status in prerequisites):
                continue
            results.append(
                {
                    "question_id": record["id"],
                    "question_revision": record["revision"],
                    "order": spec["order"],
                    "text": record["body"]["text"],
                    "why_now": record["body"]["why_now"],
                    "options": record["body"]["options"],
                    "recommendation": record["body"]["recommendation"],
                }
            )
        return sorted(results, key=lambda item: (item["order"], item["question_id"]))

    def respond(
        self,
        question_id: str,
        expected_revision: int,
        response: str,
        outcome: str,
        user_turn_id: str,
        at: str,
    ) -> None:
        if outcome not in USER_OUTCOMES:
            raise ValueError(f"unsupported user outcome: {outcome}")
        record = self.question(question_id)
        if record["body"]["status"] != "open":
            raise ValueError("Question is not open")
        if record["revision"] != expected_revision:
            raise ValueError("Question revision mismatch")
        source_id = f"source-{user_turn_id}"
        response_id = f"context-response-{user_turn_id}"
        self.store.add_record(
            self.project_id,
            {
                "id": source_id,
                "kind": "source",
                "body": {"source_kind": "user_turn", "turn_id": user_turn_id},
                "author_kind": "user",
                "source_id": None,
                "created_at": at,
            },
        )
        self.store.add_record(
            self.project_id,
            {
                "id": response_id,
                "kind": "context_item",
                "body": {
                    "context_kind": "inquiry_response",
                    "question_id": question_id,
                    "question_revision": expected_revision,
                    "response": response,
                    "outcome": outcome,
                },
                "author_kind": "user",
                "source_id": source_id,
                "created_at": at,
            },
        )
        if outcome in {"answered", "delegated"}:
            self.store.add_record(
                self.project_id,
                {
                    "id": f"decision-{question_id}",
                    "kind": "decision",
                    "body": {
                        "question_id": question_id,
                        "question_revision": expected_revision,
                        "choice": response,
                        "outcome": outcome,
                    },
                    "author_kind": "user",
                    "source_id": source_id,
                    "created_at": at,
                },
            )
        self._revise_question(
            question_id,
            {
                "status": outcome,
                "response_record_id": response_id,
                "response_question_revision": expected_revision,
            },
            at,
        )

    def respond_round(self, responses: Iterable[dict[str, Any]]) -> None:
        for response in responses:
            self.respond(**response)

    def pause(self, checkpoint_id: str, at: str) -> list[str]:
        open_ids = [item["question_id"] for item in self.frontier()]
        self.store.add_record(
            self.project_id,
            {
                "id": checkpoint_id,
                "kind": "checkpoint",
                "body": {"state": "paused", "open_frontier": open_ids},
                "author_kind": "agent",
                "source_id": self.fact_source.source_id,
                "created_at": at,
            },
        )
        return open_ids

    def statuses(self) -> dict[str, str]:
        return {spec["id"]: self.question(spec["id"])["body"]["status"] for spec in self.fixture["questions"]}

    def complete(self) -> bool:
        return all(status in TERMINAL_OUTCOMES for status in self.statuses().values())


def write_frontier(path: Path, frontier: list[dict[str, Any]]) -> None:
    v03.atomic_write(path, v03.canonical_bytes(frontier))


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("frontier", "frontier-and-kill"))
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--facts", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args(argv)
    engine = InquiryEngine.resume(arguments.db, arguments.fixture, arguments.facts)
    frontier = engine.frontier()
    write_frontier(arguments.output, frontier)
    engine.close()
    if arguments.command == "frontier-and-kill":
        os.kill(os.getpid(), signal.SIGKILL)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
