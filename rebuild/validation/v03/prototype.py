#!/usr/bin/env python3
"""Disposable V03 canonical-context persistence and bundle experiment."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import sqlite3
import sys
import tempfile
from typing import Any, Iterable
import uuid


BUNDLE_FORMAT = "volicord-context-bundle"
BUNDLE_FORMAT_VERSION = 1
CANONICAL_SCHEMA = "canonical-context"
CANONICAL_SCHEMA_VERSION = 1
RECORD_KINDS = {"source", "question", "decision", "context_item", "checkpoint"}
AUTHOR_KINDS = {"user", "agent", "observed"}
REVIEW_STATES = {"active", "contradicted", "review_due", "superseded"}


def canonical_bytes(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def atomic_write(path: Path, content: bytes, stop_before_publish: bool = False) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(content)
            output.flush()
            os.fsync(output.fileno())
        if stop_before_publish:
            os.kill(os.getpid(), signal.SIGKILL)
        os.replace(temporary, path)
        fsync_directory(path.parent)
    finally:
        temporary.unlink(missing_ok=True)


def validate_record(record: dict[str, Any]) -> None:
    required = {"id", "kind", "body", "author_kind", "source_id", "created_at"}
    missing = required - record.keys()
    if missing:
        raise ValueError(f"record missing fields: {', '.join(sorted(missing))}")
    if record["kind"] not in RECORD_KINDS:
        raise ValueError(f"unsupported record kind: {record['kind']}")
    if record["author_kind"] not in AUTHOR_KINDS:
        raise ValueError(f"unsupported author kind: {record['author_kind']}")
    if record["kind"] == "decision" and record["author_kind"] == "user" and not record["source_id"]:
        raise ValueError("user decisions require an exact user-turn source")


def initial_bundle(project: dict[str, Any]) -> dict[str, Any]:
    return {
        "format": BUNDLE_FORMAT,
        "format_version": BUNDLE_FORMAT_VERSION,
        "schema": CANONICAL_SCHEMA,
        "schema_version": CANONICAL_SCHEMA_VERSION,
        "project": project,
        "records": [],
        "tombstones": [],
    }


def new_project_identity(name: str, created_at: str) -> dict[str, str]:
    return {"id": f"project-{uuid.uuid4()}", "name": name, "created_at": created_at}


def validate_bundle(bundle: dict[str, Any]) -> None:
    expected = (BUNDLE_FORMAT, BUNDLE_FORMAT_VERSION, CANONICAL_SCHEMA, CANONICAL_SCHEMA_VERSION)
    actual = (bundle.get("format"), bundle.get("format_version"), bundle.get("schema"), bundle.get("schema_version"))
    if actual != expected:
        raise ValueError(f"unsupported bundle envelope: {actual!r}")
    if not isinstance(bundle.get("project", {}).get("id"), str):
        raise ValueError("bundle project identity is missing")
    for record in bundle.get("records", []):
        validate_record(record)


class SqliteCanonicalStore:
    """Transactional experiment candidate; not a production schema."""

    def __init__(self, path: Path):
        self.path = path
        path.parent.mkdir(parents=True, exist_ok=True)
        self.connection = sqlite3.connect(path)
        self.connection.row_factory = sqlite3.Row
        self.connection.execute("PRAGMA foreign_keys=ON")
        self.connection.execute("PRAGMA journal_mode=WAL")
        self.connection.execute("PRAGMA synchronous=FULL")
        self.connection.execute("PRAGMA secure_delete=ON")
        self._create_schema()

    def _create_schema(self) -> None:
        self.connection.executescript(
            """
            CREATE TABLE IF NOT EXISTS experiment_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS project (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS clone_binding (
                project_id TEXT NOT NULL,
                clone_path TEXT NOT NULL,
                bound_at TEXT NOT NULL,
                PRIMARY KEY (project_id, clone_path),
                FOREIGN KEY (project_id) REFERENCES project(id)
            );
            CREATE TABLE IF NOT EXISTS canonical_record (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                revision INTEGER NOT NULL,
                body_json TEXT NOT NULL,
                author_kind TEXT NOT NULL,
                source_id TEXT,
                review_state TEXT NOT NULL,
                supersedes TEXT,
                superseded_by TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (project_id) REFERENCES project(id)
            );
            CREATE TABLE IF NOT EXISTS record_revision (
                record_id TEXT NOT NULL,
                revision INTEGER NOT NULL,
                body_json TEXT NOT NULL,
                corrected_at TEXT NOT NULL,
                PRIMARY KEY (record_id, revision),
                FOREIGN KEY (record_id) REFERENCES canonical_record(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS tombstone (
                record_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                record_kind TEXT NOT NULL,
                deleted_at TEXT NOT NULL
            );
            """
        )
        metadata = {
            "schema": CANONICAL_SCHEMA,
            "schema_version": str(CANONICAL_SCHEMA_VERSION),
            "experiment": "V03",
        }
        with self.connection:
            self.connection.executemany(
                "INSERT OR IGNORE INTO experiment_meta(key, value) VALUES (?, ?)", metadata.items()
            )

    def close(self) -> None:
        self.connection.close()

    def create_project(self, project: dict[str, Any], clone_path: str) -> None:
        with self.connection:
            self.connection.execute(
                "INSERT INTO project(id, name, created_at) VALUES (?, ?, ?)",
                (project["id"], project["name"], project["created_at"]),
            )
            self.connection.execute(
                "INSERT INTO clone_binding(project_id, clone_path, bound_at) VALUES (?, ?, ?)",
                (project["id"], clone_path, project["created_at"]),
            )

    def bind_clone(self, project_id: str, clone_path: str, bound_at: str) -> None:
        with self.connection:
            self.connection.execute(
                "INSERT OR REPLACE INTO clone_binding(project_id, clone_path, bound_at) VALUES (?, ?, ?)",
                (project_id, clone_path, bound_at),
            )

    def clone_bindings(self, project_id: str) -> list[str]:
        rows = self.connection.execute(
            "SELECT clone_path FROM clone_binding WHERE project_id=? ORDER BY clone_path", (project_id,)
        )
        return [row["clone_path"] for row in rows]

    def add_record(self, project_id: str, record: dict[str, Any]) -> None:
        validate_record(record)
        body_json = canonical_bytes(record["body"]).decode("utf-8").rstrip("\n")
        with self.connection:
            self.connection.execute(
                """INSERT INTO canonical_record(
                    id, project_id, kind, revision, body_json, author_kind, source_id,
                    review_state, supersedes, superseded_by, created_at, updated_at
                ) VALUES (?, ?, ?, 1, ?, ?, ?, 'active', NULL, NULL, ?, ?)""",
                (
                    record["id"], project_id, record["kind"], body_json,
                    record["author_kind"], record["source_id"], record["created_at"], record["created_at"],
                ),
            )
            self.connection.execute(
                "INSERT INTO record_revision(record_id, revision, body_json, corrected_at) VALUES (?, 1, ?, ?)",
                (record["id"], body_json, record["created_at"]),
            )

    def get_record(self, record_id: str) -> dict[str, Any] | None:
        row = self.connection.execute("SELECT * FROM canonical_record WHERE id=?", (record_id,)).fetchone()
        return self._row_record(row) if row else None

    def revision_history(self, record_id: str) -> list[dict[str, Any]]:
        rows = self.connection.execute(
            "SELECT revision, body_json, corrected_at FROM record_revision WHERE record_id=? ORDER BY revision",
            (record_id,),
        )
        return [
            {"revision": row["revision"], "body": json.loads(row["body_json"]), "corrected_at": row["corrected_at"]}
            for row in rows
        ]

    def _row_record(self, row: sqlite3.Row) -> dict[str, Any]:
        return {
            "id": row["id"],
            "kind": row["kind"],
            "revision": row["revision"],
            "body": json.loads(row["body_json"]),
            "author_kind": row["author_kind"],
            "source_id": row["source_id"],
            "review_state": row["review_state"],
            "supersedes": row["supersedes"],
            "superseded_by": row["superseded_by"],
            "created_at": row["created_at"],
            "updated_at": row["updated_at"],
        }

    def revise(self, record_id: str, body: dict[str, Any], corrected_at: str) -> None:
        body_json = canonical_bytes(body).decode("utf-8").rstrip("\n")
        with self.connection:
            row = self.connection.execute(
                "SELECT revision FROM canonical_record WHERE id=?", (record_id,)
            ).fetchone()
            if row is None:
                raise KeyError(record_id)
            revision = row["revision"] + 1
            self.connection.execute(
                "UPDATE canonical_record SET revision=?, body_json=?, updated_at=? WHERE id=?",
                (revision, body_json, corrected_at, record_id),
            )
            self.connection.execute(
                "INSERT INTO record_revision(record_id, revision, body_json, corrected_at) VALUES (?, ?, ?, ?)",
                (record_id, revision, body_json, corrected_at),
            )

    def set_review_state(self, record_id: str, state: str, updated_at: str) -> None:
        if state not in REVIEW_STATES:
            raise ValueError(state)
        with self.connection:
            cursor = self.connection.execute(
                "UPDATE canonical_record SET review_state=?, updated_at=? WHERE id=?", (state, updated_at, record_id)
            )
            if cursor.rowcount != 1:
                raise KeyError(record_id)

    def supersede(self, project_id: str, old_id: str, new_record: dict[str, Any]) -> None:
        if new_record["kind"] != "decision":
            raise ValueError("semantic supersession requires a new decision")
        validate_record(new_record)
        body_json = canonical_bytes(new_record["body"]).decode("utf-8").rstrip("\n")
        with self.connection:
            old = self.connection.execute(
                "SELECT kind FROM canonical_record WHERE id=?", (old_id,)
            ).fetchone()
            if old is None or old["kind"] != "decision":
                raise ValueError("superseded record must be a decision")
            self.connection.execute(
                "UPDATE canonical_record SET review_state='superseded', superseded_by=?, updated_at=? WHERE id=?",
                (new_record["id"], new_record["created_at"], old_id),
            )
            self.connection.execute(
                """INSERT INTO canonical_record(
                    id, project_id, kind, revision, body_json, author_kind, source_id,
                    review_state, supersedes, superseded_by, created_at, updated_at
                ) VALUES (?, ?, 'decision', 1, ?, ?, ?, 'active', ?, NULL, ?, ?)""",
                (
                    new_record["id"], project_id, body_json, new_record["author_kind"],
                    new_record["source_id"], old_id, new_record["created_at"], new_record["created_at"],
                ),
            )
            self.connection.execute(
                "INSERT INTO record_revision(record_id, revision, body_json, corrected_at) VALUES (?, 1, ?, ?)",
                (new_record["id"], body_json, new_record["created_at"]),
            )

    def bundle(self) -> dict[str, Any]:
        project = self.connection.execute("SELECT id, name, created_at FROM project").fetchone()
        if project is None:
            raise ValueError("project not initialized")
        records = []
        for row in self.connection.execute("SELECT * FROM canonical_record ORDER BY id"):
            record = self._row_record(row)
            record["revisions"] = self.revision_history(record["id"])
            records.append(record)
        tombstones = [dict(row) for row in self.connection.execute("SELECT * FROM tombstone ORDER BY record_id")]
        bundle = initial_bundle(dict(project))
        bundle["records"] = records
        bundle["tombstones"] = tombstones
        return bundle

    def export(self, path: Path) -> bytes:
        content = canonical_bytes(self.bundle())
        atomic_write(path, content)
        return content

    def import_bundle(self, bundle_path: Path) -> str:
        bundle = read_json(bundle_path)
        validate_bundle(bundle)
        project = bundle["project"]
        with self.connection:
            self.connection.execute(
                "INSERT INTO project(id, name, created_at) VALUES (?, ?, ?)",
                (project["id"], project["name"], project["created_at"]),
            )
            for record in bundle["records"]:
                body_json = canonical_bytes(record["body"]).decode("utf-8").rstrip("\n")
                self.connection.execute(
                    """INSERT INTO canonical_record(
                        id, project_id, kind, revision, body_json, author_kind, source_id,
                        review_state, supersedes, superseded_by, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                    (
                        record["id"], project["id"], record["kind"], record["revision"], body_json,
                        record["author_kind"], record["source_id"], record["review_state"],
                        record["supersedes"], record["superseded_by"], record["created_at"], record["updated_at"],
                    ),
                )
                revisions = record.get("revisions") or [
                    {"revision": record["revision"], "body": record["body"], "corrected_at": record["updated_at"]}
                ]
                for revision in revisions:
                    revision_body = canonical_bytes(revision["body"]).decode("utf-8").rstrip("\n")
                    self.connection.execute(
                        "INSERT INTO record_revision(record_id, revision, body_json, corrected_at) VALUES (?, ?, ?, ?)",
                        (record["id"], revision["revision"], revision_body, revision["corrected_at"]),
                    )
            for tombstone in bundle["tombstones"]:
                self.connection.execute(
                    "INSERT INTO tombstone(record_id, project_id, record_kind, deleted_at) VALUES (?, ?, ?, ?)",
                    (tombstone["record_id"], tombstone["project_id"], tombstone["record_kind"], tombstone["deleted_at"]),
                )
        return project["id"]

    def delete_derived(self, derived_path: Path) -> None:
        if derived_path.exists():
            shutil.rmtree(derived_path)

    def forget(
        self,
        record_id: str,
        deleted_at: str,
        derived_path: Path,
        managed_bundles: Iterable[Path],
        recoverable_logs: Iterable[Path] = (),
    ) -> None:
        with self.connection:
            row = self.connection.execute(
                "SELECT project_id, kind FROM canonical_record WHERE id=?", (record_id,)
            ).fetchone()
            if row is None:
                raise KeyError(record_id)
            self.connection.execute("DELETE FROM canonical_record WHERE id=?", (record_id,))
            self.connection.execute(
                "INSERT INTO tombstone(record_id, project_id, record_kind, deleted_at) VALUES (?, ?, ?, ?)",
                (record_id, row["project_id"], row["kind"], deleted_at),
            )
        self.delete_derived(derived_path)
        for log_path in recoverable_logs:
            log_path.unlink(missing_ok=True)
        self.connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        self.connection.execute("VACUUM")
        self.connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        for bundle_path in managed_bundles:
            self.export(bundle_path)


class AtomicJsonCanonicalStore:
    """Portable canonical snapshot candidate using fsync plus atomic rename."""

    def __init__(self, path: Path):
        self.path = path

    def initialize(self, project: dict[str, Any]) -> None:
        atomic_write(self.path, canonical_bytes(initial_bundle(project)))

    def load(self) -> dict[str, Any]:
        bundle = read_json(self.path)
        validate_bundle(bundle)
        return bundle

    def add_record(self, record: dict[str, Any], stop_before_publish: bool = False) -> None:
        validate_record(record)
        bundle = self.load()
        normalized = {
            **record,
            "revision": 1,
            "review_state": "active",
            "supersedes": None,
            "superseded_by": None,
            "updated_at": record["created_at"],
        }
        bundle["records"].append(normalized)
        bundle["records"].sort(key=lambda item: item["id"])
        atomic_write(self.path, canonical_bytes(bundle), stop_before_publish=stop_before_publish)


def crash_sqlite(path: Path, fixture: Path, phase: str) -> None:
    scenario = read_json(fixture)
    store = SqliteCanonicalStore(path)
    project_id = scenario["project"]["id"]
    record = {
        "id": f"context-crash-{phase}",
        "kind": "context_item",
        "body": {"context_kind": "fact", "text": phase},
        "author_kind": "agent",
        "source_id": "source-repository",
        "created_at": "2026-08-08T20:31:00Z",
    }
    body_json = canonical_bytes(record["body"]).decode("utf-8").rstrip("\n")
    store.connection.execute("BEGIN IMMEDIATE")
    store.connection.execute(
        """INSERT INTO canonical_record(
            id, project_id, kind, revision, body_json, author_kind, source_id,
            review_state, supersedes, superseded_by, created_at, updated_at
        ) VALUES (?, ?, ?, 1, ?, ?, ?, 'active', NULL, NULL, ?, ?)""",
        (
            record["id"], project_id, record["kind"], body_json, record["author_kind"],
            record["source_id"], record["created_at"], record["created_at"],
        ),
    )
    store.connection.execute(
        "INSERT INTO record_revision(record_id, revision, body_json, corrected_at) VALUES (?, 1, ?, ?)",
        (record["id"], body_json, record["created_at"]),
    )
    if phase == "after-commit":
        store.connection.commit()
    os.kill(os.getpid(), signal.SIGKILL)


def crash_json(path: Path, fixture: Path) -> None:
    scenario = read_json(fixture)
    store = AtomicJsonCanonicalStore(path)
    record = dict(scenario["sensitive_record"])
    record["id"] = "context-json-uncommitted"
    record["body"] = {"context_kind": "fact", "text": "uncommitted JSON candidate write"}
    store.add_record(record, stop_before_publish=True)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    sqlite_parser = subparsers.add_parser("crash-sqlite")
    sqlite_parser.add_argument("--db", type=Path, required=True)
    sqlite_parser.add_argument("--fixture", type=Path, required=True)
    sqlite_parser.add_argument("--phase", choices=("before-commit", "after-commit"), required=True)
    json_parser = subparsers.add_parser("crash-json")
    json_parser.add_argument("--store", type=Path, required=True)
    json_parser.add_argument("--fixture", type=Path, required=True)
    arguments = parser.parse_args(argv)
    if arguments.command == "crash-sqlite":
        crash_sqlite(arguments.db, arguments.fixture, arguments.phase)
    else:
        crash_json(arguments.store, arguments.fixture)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
