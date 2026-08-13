#!/usr/bin/env python3
"""Map V06 requirements to the Production projection/document Rust oracle."""

from __future__ import annotations

import json
from pathlib import Path
import shlex
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[4]
FIXTURE = ROOT / "rebuild/validation/projections/source-grounded-documents/fixtures/v06-matrix.json"
MANIFEST = ROOT / "rebuild/validation/shared/fixture-manifest.json"
PRODUCTION_SOURCES = (
    ROOT / "rebuild/crates/volicord-projections/src/project.rs",
    ROOT / "rebuild/crates/volicord-projections/src/documents.rs",
)
PRODUCTION_COMMAND = [
    "cargo", "test", "--manifest-path", "rebuild/Cargo.toml",
    "-p", "volicord-projections", "--test", "project_documents",
    "--all-features",
]
EXPECTED_GROUPS = {
    "project_read_surface",
    "generated_documents",
    "formats_and_language",
    "purity_and_publication",
}
EXPECTED_SOURCE_ROLES = {
    "single_language",
    "polyglot",
    "partial_or_failed_analysis",
    "canonical_and_resume",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def run(arguments: list[str], *, capture: bool = False) -> subprocess.CompletedProcess[str]:
    print(f"$ {shlex.join(arguments)}", flush=True)
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    if result.returncode != 0:
        if capture:
            print(result.stdout, end="", file=sys.stdout)
            print(result.stderr, end="", file=sys.stderr)
        raise RuntimeError(
            f"command failed with exit {result.returncode}: {shlex.join(arguments)}"
        )
    return result


def main() -> int:
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    require(fixture.get("schema_version") == 1, "V06 fixture schema_version changed")
    require(fixture.get("validation_id") == "V06", "fixture is not V06 evidence")
    groups = fixture.get("groups")
    require(isinstance(groups, dict), "V06 fixture groups must be an object")
    require(set(groups) == EXPECTED_GROUPS, "V06 evidence groups changed without review")

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    entries = manifest.get("fixtures", [])
    by_id = {entry.get("id"): entry for entry in entries}
    v06_entries = [entry for entry in entries if entry.get("validation_id") == "V06"]
    require(
        [entry.get("id") for entry in v06_entries] == ["v06-source-grounded-documents"],
        "V06 manifest entry is missing or ambiguous",
    )
    sources = fixture.get("fixture_sources")
    require(isinstance(sources, list), "V06 fixture_sources must be a list")
    require(
        {source.get("role") for source in sources} == EXPECTED_SOURCE_ROLES,
        "V06 input roles no longer cover single-language, polyglot, degraded, and canonical bases",
    )
    for source in sources:
        fixture_id = source.get("fixture_id")
        require(fixture_id in by_id, f"V06 references an unknown fixture: {fixture_id}")
        require(
            isinstance(source.get("coverage"), list) and source["coverage"],
            f"V06 fixture source has no coverage: {fixture_id}",
        )

    mappings: list[tuple[str, str, str, str]] = []
    for group, group_mappings in groups.items():
        require(isinstance(group_mappings, list) and group_mappings, f"empty V06 group: {group}")
        for mapping in group_mappings:
            require(
                isinstance(mapping, list)
                and len(mapping) == 4
                and all(isinstance(value, str) and value for value in mapping),
                f"invalid V06 mapping in {group}: {mapping!r}",
            )
            mappings.append(tuple(mapping))
    requirement_ids = [requirement for requirement, _, _, _ in mappings]
    require(len(requirement_ids) == len(set(requirement_ids)), "duplicate V06 requirement")
    require(len(mappings) >= 30, "V06 matrix no longer covers the complete read/document surface")
    require(
        {package for _, package, _, _ in mappings} == {"volicord-projections"},
        "V06 must map to the Production projection owner",
    )
    require(
        {target for _, _, target, _ in mappings} == {"project_documents"},
        "V06 mappings must use the maintained Production integration target",
    )

    catalog = run([*PRODUCTION_COMMAND, "--", "--list"], capture=True).stdout
    missing = sorted(
        f"{target}:{test}"
        for _, _, target, test in mappings
        if f"{test}: test" not in catalog
    )
    require(not missing, f"mapped Production Rust tests are missing: {missing}")

    for source_path in PRODUCTION_SOURCES:
        text = source_path.read_text(encoding="utf-8")
        require("std::fs" not in text, f"projection source owns filesystem I/O: {source_path}")
        require("File::create" not in text, f"projection source creates files: {source_path}")
        require("write_all(" not in text, f"projection source writes files: {source_path}")

    run(PRODUCTION_COMMAND)
    print(
        json.dumps(
            {
                "fixture_sources": len(sources),
                "group_count": len(groups),
                "mapped_requirements": len(mappings),
                "production_tests": sum(line.endswith(": test") for line in catalog.splitlines()),
                "status": "passed",
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
