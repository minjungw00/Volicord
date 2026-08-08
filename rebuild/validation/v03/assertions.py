#!/usr/bin/env python3
"""Reproducible assertions for V03 canonical persistence and portability."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True

from prototype import AtomicJsonCanonicalStore, SqliteCanonicalStore, canonical_bytes, new_project_identity, read_json


ROOT = Path(__file__).resolve().parents[3]
FIXTURE = ROOT / "rebuild/validation/fixtures/v03/canonical-scenario.json"
PROTOTYPE = ROOT / "rebuild/validation/v03/prototype.py"
ARTIFACT_PARENT = ROOT / "rebuild/.local/v03"
SECRET = b"SENSITIVE-V03-7f43-private-alias"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def files_containing(root: Path, needle: bytes) -> list[str]:
    matches: list[str] = []
    for path in sorted(item for item in root.rglob("*") if item.is_file()):
        if needle in path.read_bytes():
            matches.append(path.relative_to(root).as_posix())
    return matches


def kill_helper(*arguments: str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        (sys.executable, str(PROTOTYPE), *arguments),
        cwd=ROOT,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def main() -> int:
    started = time.monotonic_ns()
    scenario = read_json(FIXTURE)
    generated_one = new_project_identity("Path-independent fixture", "2026-08-08T20:29:58Z")
    generated_two = new_project_identity("Path-independent fixture", "2026-08-08T20:29:58Z")
    require(generated_one["id"] != generated_two["id"], "Project identity generated at initialization")
    require("clones" not in generated_one["id"], "Project identity must not encode a path")
    ARTIFACT_PARENT.mkdir(parents=True, exist_ok=True)
    artifact_root = Path(tempfile.mkdtemp(prefix="assertions-", dir=ARTIFACT_PARENT))
    runtime = artifact_root / "runtime"
    runtime.mkdir()
    database = runtime / "canonical.sqlite3"
    derived = runtime / "derived"
    bundles = runtime / "managed-bundles"
    bundles.mkdir()
    first_bundle = bundles / "context-one.json"
    second_bundle = bundles / "context-two.json"

    store = SqliteCanonicalStore(database)
    store.create_project(scenario["project"], scenario["initial_clone"])
    for record in scenario["records"]:
        store.add_record(scenario["project"]["id"], record)
    store.add_record(scenario["project"]["id"], scenario["sensitive_record"])

    kinds = {store.get_record(record["id"])["kind"] for record in scenario["records"]}
    require(kinds == {"source", "question", "decision", "context_item", "checkpoint"}, "all canonical kinds")
    user_decision = store.get_record("decision-storage")
    checkpoint = store.get_record("checkpoint-1")
    require(user_decision["author_kind"] == "user" and user_decision["source_id"] == "source-user-turn-17", "user provenance")
    require(checkpoint["author_kind"] == "agent" and checkpoint["source_id"] == "source-repository", "agent provenance")
    invalid_decision = dict(scenario["records"][2])
    invalid_decision["id"] = "decision-without-user-turn"
    invalid_decision["source_id"] = None
    try:
        store.add_record(scenario["project"]["id"], invalid_decision)
    except ValueError:
        pass
    else:
        raise AssertionError("user decision without exact source was accepted")

    store.revise(
        "context-goal",
        {"context_kind": "goal", "text": "Resume with the same Project context."},
        "2026-08-08T20:32:00Z",
    )
    require(store.get_record("context-goal")["revision"] == 2, "correction increments revision")
    store.set_review_state("context-fact", "contradicted", "2026-08-08T20:32:01Z")
    store.set_review_state("question-storage", "review_due", "2026-08-08T20:32:02Z")
    replacement = {
        "id": "decision-storage-2",
        "kind": "decision",
        "body": {"choice": "SQLite experiment candidate", "question_id": "question-storage", "rationale": "Observed atomic recovery."},
        "author_kind": "user",
        "source_id": "source-user-turn-19",
        "created_at": "2026-08-08T20:32:03Z",
    }
    store.supersede(scenario["project"]["id"], "decision-storage", replacement)
    require(store.get_record("decision-storage")["superseded_by"] == replacement["id"], "old decision linked")
    require(store.get_record(replacement["id"])["supersedes"] == "decision-storage", "new decision linked")

    first = store.export(first_bundle)
    second = store.export(second_bundle)
    require(first == second, "repeated export must be byte-identical")
    envelope = json.loads(first)
    require(envelope["format"] == "volicord-context-bundle" and envelope["format_version"] == 1, "explicit format")
    require(envelope["schema"] == "canonical-context" and envelope["schema_version"] == 1, "explicit schema")
    require("clone_path" not in first.decode("utf-8"), "local clone path excluded from portable bundle")

    store.close()
    restarted = SqliteCanonicalStore(database)
    require(restarted.bundle()["project"]["id"] == scenario["project"]["id"], "stable identity after restart")
    require(restarted.get_record("context-goal")["revision"] == 2, "revision survives restart")

    imported_db = runtime / "imported.sqlite3"
    imported = SqliteCanonicalStore(imported_db)
    imported_id = imported.import_bundle(first_bundle)
    imported.bind_clone(imported_id, scenario["another_clone"], "2026-08-08T20:33:00Z")
    require(imported_id == scenario["project"]["id"], "identity survives import")
    require(imported.clone_bindings(imported_id) == [scenario["another_clone"]], "explicit another-path binding")
    require(imported.get_record("source-repository")["body"]["locator"] == "src/lib.rs", "source locator remains portable")
    require(len(imported.revision_history("context-goal")) == 2, "revision history survives import")
    imported.forget(
        "context-sensitive",
        "2026-08-08T20:33:01Z",
        runtime / "imported-derived",
        (),
    )
    imported.close()

    before = kill_helper("crash-sqlite", "--db", str(database), "--fixture", str(FIXTURE), "--phase", "before-commit")
    require(before.returncode < 0, "before-commit helper must be hard-terminated")
    crash_recovered = SqliteCanonicalStore(database)
    require(crash_recovered.get_record("context-crash-before-commit") is None, "uncommitted record must not survive")
    crash_recovered.close()
    after = kill_helper("crash-sqlite", "--db", str(database), "--fixture", str(FIXTURE), "--phase", "after-commit")
    require(after.returncode < 0, "after-commit helper must be hard-terminated")
    crash_recovered = SqliteCanonicalStore(database)
    require(crash_recovered.get_record("context-crash-after-commit") is not None, "committed record must survive")

    derived.mkdir()
    (derived / "full-text.index").write_bytes(b"prefix:" + SECRET + b":suffix")
    canonical_count = len(crash_recovered.bundle()["records"])
    crash_recovered.delete_derived(derived)
    require(not derived.exists(), "derived directory completely deleted")
    require(len(crash_recovered.bundle()["records"]) == canonical_count, "canonical records survive derived deletion")
    derived.mkdir()
    (derived / "rebuilt.index").write_bytes(SECRET)
    recoverable_log = runtime / "recoverable.log"
    recoverable_log.write_bytes(b"bounded observation:" + SECRET)
    crash_recovered.forget(
        "context-sensitive",
        "2026-08-08T20:34:00Z",
        derived,
        (first_bundle, second_bundle),
        (recoverable_log,),
    )
    require(crash_recovered.get_record("context-sensitive") is None, "sensitive record deleted")
    tombstone = next(item for item in crash_recovered.bundle()["tombstones"] if item["record_id"] == "context-sensitive")
    require(set(tombstone) == {"record_id", "project_id", "record_kind", "deleted_at"}, "minimal tombstone")
    canonical_count_after_forget = len(crash_recovered.bundle()["records"])
    crash_recovered.close()

    json_path = runtime / "canonical-snapshot.json"
    json_store = AtomicJsonCanonicalStore(json_path)
    json_store.initialize(scenario["project"])
    json_store.add_record(scenario["records"][0])
    json_before = json_path.read_bytes()
    json_restart = AtomicJsonCanonicalStore(json_path)
    require(json_restart.load()["records"][0]["id"] == "source-repository", "JSON snapshot restart")
    json_crash = kill_helper("crash-json", "--store", str(json_path), "--fixture", str(FIXTURE))
    require(json_crash.returncode < 0, "JSON helper must be hard-terminated")
    require(json_path.read_bytes() == json_before, "atomic JSON preserves last published snapshot")
    require(not any(item["id"] == "context-json-uncommitted" for item in json_restart.load()["records"]), "JSON uncommitted change absent")

    residue = files_containing(runtime, SECRET)
    require(not residue, f"sensitive bytes remain in managed runtime artifacts: {residue}")

    legacy_tokens = ("VOLICORD_HOME", "Runtime Home", "UserAction", "Write Ticket", "dual-read", "legacy schema")
    maintained_source = PROTOTYPE.read_text(encoding="utf-8") + FIXTURE.read_text(encoding="utf-8")
    require(not any(token in maintained_source for token in legacy_tokens), "legacy runtime dependency token present")

    summary = {
        "schema_version": 1,
        "status": "passed",
        "project_id": scenario["project"]["id"],
        "canonical_record_count_after_forget": canonical_count_after_forget,
        "sqlite_bundle_sha256": sha256(first_bundle),
        "sqlite_bundle_bytes": first_bundle.stat().st_size,
        "sqlite_database_bytes": database.stat().st_size,
        "json_snapshot_bytes": json_path.stat().st_size,
        "hard_termination_returncodes": [before.returncode, after.returncode, json_crash.returncode],
        "sensitive_residue_files": residue,
        "duration_ms": round((time.monotonic_ns() - started) / 1_000_000, 3),
        "artifact_root": str(artifact_root),
    }
    summary_path = artifact_root / "summary.json"
    summary_path.write_bytes(canonical_bytes(summary))
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
