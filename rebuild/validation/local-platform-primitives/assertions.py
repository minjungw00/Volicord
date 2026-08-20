#!/usr/bin/env python3
"""Assert maintained V10 fixtures, evidence coverage, and candidate conclusions."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True


ROOT = Path(__file__).resolve().parents[3]
DIRECTORY = Path(__file__).resolve().parent
SCENARIOS = DIRECTORY / "fixtures" / "scenarios.json"
REPORT = DIRECTORY / "report.md"
CLASSIFICATIONS = {
    "adopt_as_new_primitive",
    "reimplement_from_behavior",
    "reference_only",
    "reject",
}


def load_probe():
    specification = importlib.util.spec_from_file_location("v10_probe", DIRECTORY / "probe.py")
    if specification is None or specification.loader is None:
        raise AssertionError("cannot load V10 probe")
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


def main() -> int:
    fixture = json.loads(SCENARIOS.read_text(encoding="utf-8"))
    assert fixture["schema_version"] == 1
    assert fixture["validation_id"] == "V10"
    assert fixture["platform"] == "linux"
    candidates = fixture["candidates"]
    assert len(candidates) == 10
    identifiers = {candidate["id"] for candidate in candidates}
    assert len(identifiers) == len(candidates)
    assert {candidate["classification"] for candidate in candidates} <= CLASSIFICATIONS
    for candidate in candidates:
        assert (ROOT / candidate["source"]).exists(), candidate["source"]

    process_manifest = (ROOT / "crates/volicord-platform-process/Cargo.toml").read_text(encoding="utf-8")
    test_process_manifest = (ROOT / "crates/volicord-test-process/Cargo.toml").read_text(encoding="utf-8")
    filesystem_manifest = (ROOT / "crates/volicord-platform-fs/Cargo.toml").read_text(encoding="utf-8")
    assert 'path = "../volicord-' not in process_manifest
    assert "volicord-platform-process" in test_process_manifest
    assert "volicord-types" in filesystem_manifest
    assert "volicord-platform-process" in filesystem_manifest

    probe = load_probe()
    result = probe.full_probe()
    assert result["platform"] == "linux"
    assert result["process"]["exit_code"] == 23
    assert result["process"]["timeout_triggered"] is True
    assert result["process"]["termination_requested"] is True
    assert result["process"]["descendant_termination_observed"] is True
    assert result["process"]["readiness_protocol"] == "dedicated inherited pipe observed before timeout"
    repeated_process_runs = [result["process"]]
    repeated_process_runs.extend(probe.process_probe() for _ in range(11))
    assert len(repeated_process_runs) == 12
    assert all(run["timeout_triggered"] is True for run in repeated_process_runs)
    assert all(run["termination_requested"] is True for run in repeated_process_runs)
    assert all(run["descendant_termination_observed"] is True for run in repeated_process_runs)
    assert len({run["complete_stdout_sha256"] for run in repeated_process_runs}) == 1
    assert len({run["complete_stderr_sha256"] for run in repeated_process_runs}) == 1
    result["bounded_process_repetition_count"] = len(repeated_process_runs)
    assert result["filesystem"]["symlink_escape_rejected"] is True
    assert result["repository"]["linked_dirty"]["dirty"] == "true"
    assert result["storage"]["uncommitted_rows_after_crash"] == 0
    assert result["storage"]["integrity_check"] == "ok"

    probe_source = (DIRECTORY / "probe.py").read_text(encoding="utf-8")
    assert "timeout.stdout" not in probe_source
    production_tests = (ROOT / "rebuild/crates/volicord-local-platform/tests/filesystem.rs").read_text(
        encoding="utf-8"
    )
    for test_name in (
        "private_runtime_paths_repair_owned_modes_and_reject_symlinks",
        "private_runtime_creation_rejects_a_read_only_parent",
        "mutation_lock_is_released_after_holder_process_termination",
        "publication_rejects_symlink_and_read_only_faults_without_false_success",
    ):
        assert f"fn {test_name}" in production_tests, test_name
    production_filesystem = (ROOT / "rebuild/crates/volicord-local-platform/src/filesystem.rs").read_text(
        encoding="utf-8"
    )
    assert "fn parent_sync_failure_reports_published_namespace_effect" in production_filesystem

    report = REPORT.read_text(encoding="utf-8")
    for candidate in candidates:
        assert candidate["id"] in report
        assert candidate["classification"] in report
    for term in (
        "complete stdout",
        "complete stderr",
        "timeout",
        "cancellation",
        "child-tree",
        "readiness",
        "repeated",
        "read-only",
        "permission",
        "process death",
        "parent-sync",
        "symlink",
        "worktree",
        "clone",
        "dirty",
        "fingerprint",
        "atomic",
        "transaction",
        "schema",
        "repair",
        "Decision revisit trigger: not triggered",
    ):
        assert term in report, term
    print(json.dumps(result, indent=2, sort_keys=True))
    print(f"V10 assertions passed for {len(candidates)} candidate classifications")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
