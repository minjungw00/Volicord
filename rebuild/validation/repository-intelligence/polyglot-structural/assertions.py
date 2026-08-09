#!/usr/bin/env python3
"""Deterministic assertions for the disposable V01 prototype."""

from __future__ import annotations

import json
from pathlib import Path
import shutil
import sys
import tempfile
from typing import Any

sys.dont_write_bytecode = True
import prototype


REPOSITORY_ROOT = Path(__file__).resolve().parents[4]
MANIFEST = REPOSITORY_ROOT / "rebuild" / "validation" / "shared" / "fixture-manifest.json"
LOCAL_ROOT = REPOSITORY_ROOT / "rebuild" / ".local" / "v01"
V01_FIXTURE_PREFIX = Path("rebuild/validation/repository-intelligence/polyglot-structural/fixtures")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def serialize(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def v01_fixture_id(fixture_manifest: dict[str, Any]) -> str:
    path_value = fixture_manifest.get("path")
    require(isinstance(path_value, str) and path_value, "V01 fixture path must be a non-empty string")
    path = Path(path_value)
    prefix_parts = V01_FIXTURE_PREFIX.parts
    require(
        not path.is_absolute()
        and path.parts[: len(prefix_parts)] == prefix_parts
        and len(path.parts) == len(prefix_parts) + 1,
        f"V01 fixture path is outside the V01 fixture root: {path_value}",
    )
    return path.name


def select_v01_manifest_fixtures(manifest: dict[str, Any]) -> tuple[dict[str, Any], ...]:
    fixtures = manifest.get("fixtures")
    require(isinstance(fixtures, list), "fixture manifest fixtures must be a list")
    selected = tuple(
        fixture
        for fixture in fixtures
        if isinstance(fixture, dict) and fixture.get("validation_id") == "V01"
    )
    require(selected, "fixture manifest contains no V01 fixture entries")

    manifest_ids: set[str] = set()
    graph_ids: set[str] = set()
    for fixture in selected:
        manifest_id = fixture.get("id")
        require(isinstance(manifest_id, str) and manifest_id, "V01 fixture id must be a non-empty string")
        require(manifest_id.startswith("v01-"), f"inconsistent V01 fixture manifest id: {manifest_id}")
        require(manifest_id not in manifest_ids, f"ambiguous V01 fixture manifest id: {manifest_id}")
        manifest_ids.add(manifest_id)

        graph_id = v01_fixture_id(fixture)
        require(graph_id not in graph_ids, f"ambiguous V01 graph fixture identity: {graph_id}")
        graph_ids.add(graph_id)
        for field in ("expected_entities", "expected_relations"):
            require(isinstance(fixture.get(field), list), f"{manifest_id}.{field} must be a list")
    return selected


def selected_fixture_map(
    graph: dict[str, Any], fixtures: tuple[dict[str, Any], ...]
) -> dict[str, dict[str, Any]]:
    graph_fixtures = graph.get("fixtures")
    require(isinstance(graph_fixtures, list), "V01 graph fixtures must be a list")
    by_fixture: dict[str, dict[str, Any]] = {}
    for fixture in graph_fixtures:
        fixture_id = fixture.get("fixture_id") if isinstance(fixture, dict) else None
        require(isinstance(fixture_id, str) and fixture_id, "V01 graph fixture_id must be a non-empty string")
        require(fixture_id not in by_fixture, f"ambiguous V01 produced graph fixture: {fixture_id}")
        by_fixture[fixture_id] = fixture
    expected_ids = {v01_fixture_id(fixture) for fixture in fixtures}
    missing = expected_ids - by_fixture.keys()
    require(not missing, f"V01 produced graph is missing selected fixtures: {sorted(missing)}")
    return {fixture_id: by_fixture[fixture_id] for fixture_id in expected_ids}


def fixture_map(graph: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {fixture["fixture_id"]: fixture for fixture in graph["fixtures"]}


def expected_entity_key(value: str) -> tuple[str, str, str, str, int]:
    require(isinstance(value, str), "V01 expected entity must be a string")
    parts = value.split("|")
    require(len(parts) == 5, f"invalid V01 expected entity: {value}")
    language, path, kind, name, start_line = parts
    require(start_line.isdigit(), f"invalid V01 expected entity start line: {value}")
    return language, path, kind, name, int(start_line)


def actual_entity_key(entity: dict[str, Any]) -> tuple[str, str, str, str, int]:
    return (
        entity["language"],
        entity["path"],
        entity["kind"],
        entity["name"],
        entity["range"]["start_line"],
    )


def declaration_scores(
    by_fixture: dict[str, dict[str, Any]], fixtures: tuple[dict[str, Any], ...]
) -> tuple[dict[str, dict[str, float | int]], set[tuple[str, str, str, str, int]], set[tuple[str, str, str, str, int]]]:
    scores: dict[str, dict[str, float | int]] = {}
    all_expected: set[tuple[str, str, str, str, int]] = set()
    all_actual: set[tuple[str, str, str, str, int]] = set()
    for fixture_manifest in fixtures:
        fixture_id = v01_fixture_id(fixture_manifest)
        fixture = by_fixture[fixture_id]
        expected = {expected_entity_key(item) for item in fixture_manifest["expected_entities"]}
        actual = {
            actual_entity_key(item)
            for item in fixture["entities"]
            if item["fact_kind"] == "parser_confirmed" and item["kind"] in prototype.DECLARATION_KINDS
        }
        true_positive = len(expected & actual)
        precision = true_positive / len(actual) if actual else 1.0 if not expected else 0.0
        recall = true_positive / len(expected) if expected else 1.0
        scores[fixture_id] = {
            "expected": len(expected),
            "actual": len(actual),
            "true_positive": true_positive,
            "precision": precision,
            "recall": recall,
        }
        all_expected.update((fixture_id, *item) for item in expected)
        all_actual.update((fixture_id, *item) for item in actual)
    return scores, all_expected, all_actual


def verify_ranges(
    by_fixture: dict[str, dict[str, Any]],
    fixtures: tuple[dict[str, Any], ...],
    fixture_root: Path,
) -> int:
    checked = 0
    for fixture_manifest in fixtures:
        fixture_id = v01_fixture_id(fixture_manifest)
        fixture = by_fixture[fixture_id]
        actual = {actual_entity_key(item): item for item in fixture["entities"]}
        for expectation in fixture_manifest["expected_entities"]:
            key = expected_entity_key(expectation)
            require(key in actual, f"missing expected entity for range check: {fixture_id} {key}")
            item = actual[key]
            source_path = fixture_root / fixture_id / item["path"]
            lines = source_path.read_text(encoding="utf-8").splitlines()
            source_range = item["range"]
            require(1 <= source_range["start_line"] <= source_range["end_line"] <= len(lines), f"invalid range: {fixture_id} {key}")
            selected = "\n".join(lines[source_range["start_line"] - 1 : source_range["end_line"]])
            require(item["name"] in selected, f"range does not contain declaration name: {fixture_id} {key}")
            checked += 1
    return checked


def verify_relations(
    by_fixture: dict[str, dict[str, Any]], fixtures: tuple[dict[str, Any], ...]
) -> int:
    checked = 0
    for fixture_manifest in fixtures:
        fixture_id = v01_fixture_id(fixture_manifest)
        actual = {
            (item["kind"], item["source"], item["target"])
            for item in by_fixture[fixture_id]["relations"]
        }
        for expected in fixture_manifest["expected_relations"]:
            require(isinstance(expected, str), f"V01 expected relation must be a string: {fixture_id}")
            relation_key = tuple(expected.split("|"))
            require(len(relation_key) == 3, f"invalid V01 expected relation: {fixture_id} {expected}")
            require(relation_key in actual, f"missing expected relation: {fixture_id} {relation_key}")
            checked += 1
    return checked


def require_assertion_failure(action: Any, expected_message: str) -> None:
    try:
        action()
    except AssertionError as error:
        require(expected_message in str(error), f"unexpected assertion failure: {error}")
    else:
        raise AssertionError(f"expected assertion failure containing: {expected_message}")


def verify_manifest_selection_regressions(
    manifest: dict[str, Any],
    fixtures: tuple[dict[str, Any], ...],
    graph: dict[str, Any],
) -> None:
    require(len(fixtures) == 9, f"expected all nine maintained V01 fixtures, selected {len(fixtures)}")
    other_validation_ids = {
        fixture.get("validation_id")
        for fixture in manifest["fixtures"]
        if isinstance(fixture, dict) and fixture.get("validation_id") != "V01"
    }
    require({"V03", "V05"} <= other_validation_ids, "shared manifest lacks V03 or V05 regression entries")

    future_manifest = {
        "fixtures": [
            *manifest["fixtures"],
            {"id": "future-suite", "validation_id": "V99", "path": "not-a-v01-fixture"},
        ]
    }
    require(
        select_v01_manifest_fixtures(future_manifest) == fixtures,
        "a future non-V01 fixture entered the V01 evaluation set",
    )
    require_assertion_failure(
        lambda: select_v01_manifest_fixtures({"fixtures": [fixture for fixture in manifest["fixtures"] if fixture.get("validation_id") != "V01"]}),
        "contains no V01 fixture entries",
    )

    duplicate_manifest = {"fixtures": [*manifest["fixtures"], dict(fixtures[0])]}
    require_assertion_failure(
        lambda: select_v01_manifest_fixtures(duplicate_manifest),
        "ambiguous V01 fixture manifest id",
    )
    inconsistent_id_fixture = dict(fixtures[0])
    inconsistent_id_fixture["id"] = "v03-owned-identity"
    inconsistent_id_manifest = {
        "fixtures": [inconsistent_id_fixture, *manifest["fixtures"][1:]]
    }
    require_assertion_failure(
        lambda: select_v01_manifest_fixtures(inconsistent_id_manifest),
        "inconsistent V01 fixture manifest id",
    )
    inconsistent_fixture = dict(fixtures[0])
    inconsistent_fixture["path"] = "rebuild/validation/canonical-context/portability/fixtures"
    inconsistent_manifest = {
        "fixtures": [inconsistent_fixture, *manifest["fixtures"][1:]]
    }
    require_assertion_failure(
        lambda: select_v01_manifest_fixtures(inconsistent_manifest),
        "outside the V01 fixture root",
    )

    missing_graph = dict(graph)
    missing_graph["fixtures"] = [
        fixture for fixture in graph["fixtures"] if fixture["fixture_id"] != v01_fixture_id(fixtures[0])
    ]
    require_assertion_failure(
        lambda: selected_fixture_map(missing_graph, fixtures),
        "missing selected fixtures",
    )


def main() -> int:
    LOCAL_ROOT.mkdir(parents=True, exist_ok=True)
    artifact_directory = Path(tempfile.mkdtemp(prefix="assertions-", dir=LOCAL_ROOT))
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    manifest_fixtures = select_v01_manifest_fixtures(manifest)
    fixture_root = prototype.DEFAULT_FIXTURE_ROOT

    native_graph_1, native_metrics_1 = prototype.analyze(fixture_root, python_mode="native")
    native_graph_2, native_metrics_2 = prototype.analyze(fixture_root, python_mode="native")
    require(serialize(native_graph_1) == serialize(native_graph_2), "native/hybrid serialization changed across repeated runs")
    common_graph_1, common_metrics_1 = prototype.analyze(fixture_root, python_mode="common")
    common_graph_2, _ = prototype.analyze(fixture_root, python_mode="common")
    require(serialize(common_graph_1) == serialize(common_graph_2), "common serialization changed across repeated runs")

    native_manifest_fixtures = selected_fixture_map(native_graph_1, manifest_fixtures)
    common_manifest_fixtures = selected_fixture_map(common_graph_1, manifest_fixtures)
    verify_manifest_selection_regressions(manifest, manifest_fixtures, native_graph_1)
    native_scores, native_expected, native_actual = declaration_scores(native_manifest_fixtures, manifest_fixtures)
    common_scores, common_expected, common_actual = declaration_scores(common_manifest_fixtures, manifest_fixtures)
    require(native_expected == native_actual, f"native declaration mismatch: missing={native_expected-native_actual}, extra={native_actual-native_expected}")
    require(common_expected == common_actual, f"common declaration mismatch: missing={common_expected-common_actual}, extra={common_actual-common_expected}")
    range_count = verify_ranges(native_manifest_fixtures, manifest_fixtures, fixture_root)
    relation_count = verify_relations(native_manifest_fixtures, manifest_fixtures)

    fixtures = fixture_map(native_graph_1)
    language_states = {
        capability["language"]: capability["structural"]
        for fixture in native_graph_1["fixtures"]
        for capability in fixture["capabilities"]
        if capability["language"] in prototype.GATE_LANGUAGES
    }
    require(set(language_states) == set(prototype.GATE_LANGUAGES), "not all seven gate languages were analyzed")
    require(all(state in {"available", "partial"} for state in language_states.values()), f"unexpected gate state: {language_states}")
    require(any(item["path"] == "src/broken.js" and item["structural_state"] == "partial" for item in fixtures["javascript"]["files"]), "malformed JavaScript was not partial")
    require(any(item["name"] == "stillVisible" for item in fixtures["javascript"]["entities"]), "partial parse lost recoverable declaration")
    go_capability = next(item for item in fixtures["out_of_set"]["capabilities"] if item["language"] == "go")
    require(go_capability == {"language": "go", "inventory": "available", "structural": "unavailable"}, "out-of-set capability is dishonest")
    for fixture_id in prototype.GATE_LANGUAGES:
        require(any(item["kind"] == "test" for item in fixtures[fixture_id]["entities"]), f"test detection missing for {fixture_id}")

    entity_kinds = {item["kind"] for fixture in native_graph_1["fixtures"] for item in fixture["entities"]}
    required_entity_kinds = {"repository", "package", "module", "namespace", "file", "class", "interface", "trait", "struct", "enum", "type", "function", "method", "field", "test", "configuration", "document"}
    require(required_entity_kinds <= entity_kinds, f"normalized entity kinds missing: {required_entity_kinds-entity_kinds}")
    relation_kinds = {item["kind"] for fixture in native_graph_1["fixtures"] for item in fixture["relations"]}
    required_relation_kinds = {"contains", "declares", "imports", "includes", "exports", "inherits", "implements", "calls_syntactically", "tests", "configures"}
    require(required_relation_kinds <= relation_kinds, f"normalized relation kinds missing: {required_relation_kinds-relation_kinds}")
    require(all(not fixture["interpretations"] for fixture in native_graph_1["fixtures"]), "prototype invented interpreted facts")

    failed_graph, failed_metrics = prototype.analyze(fixture_root, python_mode="native", fail_language="javascript")
    failed_fixtures = fixture_map(failed_graph)
    require(failed_fixtures["javascript"]["failures"], "injected analyzer failure was not recorded")
    for fixture_id, baseline_fixture in fixtures.items():
        if fixture_id != "javascript":
            require(baseline_fixture == failed_fixtures[fixture_id], f"failure changed unaffected fixture: {fixture_id}")

    incremental_root = artifact_directory / "incremental-fixtures"
    shutil.copytree(fixture_root, incremental_root)
    cache_path = artifact_directory / "incremental-cache.json"
    initial_graph, initial_metrics = prototype.analyze(incremental_root, python_mode="native", cache_path=cache_path)
    changed_path = incremental_root / "python" / "src" / "greeter" / "core.py"
    with changed_path.open("a", encoding="utf-8") as changed_file:
        changed_file.write("\n\ndef added_incrementally(value: str) -> str:\n    return value\n")
    changed_graph, changed_metrics = prototype.analyze(incremental_root, python_mode="native", cache_path=cache_path)
    require(changed_metrics["analyzed_file_count"] == 1, f"incremental rerun analyzed {changed_metrics['analyzed_file_count']} files")
    require(changed_metrics["reused_file_count"] == initial_metrics["analyzed_file_count"] - 1, "incremental rerun did not reuse every unaffected file")
    initial_fixtures = fixture_map(initial_graph)
    changed_fixtures = fixture_map(changed_graph)
    for fixture_id in initial_fixtures:
        if fixture_id != "python":
            require(initial_fixtures[fixture_id] == changed_fixtures[fixture_id], f"incremental change affected {fixture_id}")
    unchanged_python_ids = {
        item["id"] for item in initial_fixtures["python"]["entities"] if item["path"] != "src/greeter/core.py"
    }
    changed_python_ids = {
        item["id"] for item in changed_fixtures["python"]["entities"] if item["path"] != "src/greeter/core.py"
    }
    require(unchanged_python_ids == changed_python_ids, "unaffected Python entity identity changed")

    require(native_metrics_1["duration_ms"] > 0 and native_metrics_1["output_bytes"] > 0 and native_metrics_1["peak_rss_kib"] > 0, "resource metrics were not captured")
    require(native_metrics_1["entity_count"] == native_metrics_2["entity_count"], "repeat entity count changed")
    summary = {
        "schema_version": 1,
        "status": "passed",
        "artifact_directory": str(artifact_directory),
        "manifest_fixture_count": len(manifest_fixtures),
        "declaration_scores_native_hybrid": native_scores,
        "declaration_scores_common": common_scores,
        "source_ranges_checked": range_count,
        "expected_relations_checked": relation_count,
        "language_states": language_states,
        "deterministic_serialization": True,
        "stable_unaffected_identity": True,
        "malformed_source_partial": True,
        "failure_isolation": True,
        "incremental": changed_metrics,
        "native_hybrid_metrics": native_metrics_1,
        "common_metrics": common_metrics_1,
        "failed_analyzer_metrics": failed_metrics,
    }
    (artifact_directory / "summary.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (artifact_directory / "native-graph.json").write_bytes(serialize(native_graph_1))
    (artifact_directory / "common-graph.json").write_bytes(serialize(common_graph_1))
    (artifact_directory / "failed-javascript-graph.json").write_bytes(serialize(failed_graph))
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as error:
        print(f"V01 assertion failed: {error}", file=sys.stderr)
        raise SystemExit(1)
