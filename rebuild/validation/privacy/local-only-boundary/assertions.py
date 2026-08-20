#!/usr/bin/env python3
"""Map V07 privacy requirements to maintained Production Rust behavior."""

from __future__ import annotations

import json
from pathlib import Path
import shlex
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[4]
FIXTURE_ROOT = ROOT / "rebuild/validation/privacy/local-only-boundary/fixtures"
FIXTURE = FIXTURE_ROOT / "v07-matrix.json"
MANIFEST = ROOT / "rebuild/validation/shared/fixture-manifest.json"
PRODUCTION_BASELINE_SUBJECTS = (
    "fix: enforce private serialized runtime access",
    "fix: make canonical forgetting recoverable across local stores",
    "feat: add forgetting repair operations",
    "fix: preserve Candidate dependency failures in projections",
    "fix: bound generated documents and normalize HTML language metadata",
)
EXPECTED_GROUPS = {
    "authority_and_local_only",
    "opt_in_transmission_and_revoke",
    "candidate_and_forgetting",
    "retention_deletion_and_correction",
    "degradation_portability_and_purity",
}
EXPECTED_SOURCE_ROLES = {
    "provider_scope_filter_and_deletion",
    "local_structural_and_ecosystem",
    "candidate_inquiry_checkpoint_recall",
    "provider_independent_projections_and_documents",
}
EXPECTED_PROVIDER_FILES = {
    "src/lib.rs",
    "src/vendor/generated.rs",
    "docs/readme.md",
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


def production_command(package: str, target: str) -> list[str]:
    return [
        "cargo",
        "test",
        "--manifest-path",
        "rebuild/Cargo.toml",
        "-p",
        package,
        "--test",
        target,
        "--all-features",
    ]


def main() -> int:
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    require(fixture.get("schema_version") == 1, "V07 fixture schema_version changed")
    require(fixture.get("validation_id") == "V07", "fixture is not V07 evidence")
    require(
        tuple(fixture.get("production_baseline_subjects", [])) == PRODUCTION_BASELINE_SUBJECTS,
        "V07 Production baseline subjects changed",
    )
    groups = fixture.get("groups")
    require(isinstance(groups, dict), "V07 fixture groups must be an object")
    require(set(groups) == EXPECTED_GROUPS, "V07 evidence groups changed without review")

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    entries = manifest.get("fixtures", [])
    by_id = {entry.get("id"): entry for entry in entries}
    v07_entries = [entry for entry in entries if entry.get("validation_id") == "V07"]
    require(
        [entry.get("id") for entry in v07_entries] == ["v07-privacy-boundary"],
        "V07 manifest entry is missing or ambiguous",
    )
    sources = fixture.get("fixture_sources")
    require(isinstance(sources, list), "V07 fixture_sources must be a list")
    require(
        {source.get("role") for source in sources} == EXPECTED_SOURCE_ROLES,
        "V07 sources no longer cover provider, local, Candidate, and projection roles",
    )
    for source in sources:
        fixture_id = source.get("fixture_id")
        require(fixture_id in by_id, f"V07 references an unknown fixture: {fixture_id}")
        require(
            isinstance(source.get("coverage"), list) and source["coverage"],
            f"V07 fixture source has no coverage: {fixture_id}",
        )

    provider_root = FIXTURE_ROOT / "provider-scope"
    provider_files = {
        path.relative_to(provider_root).as_posix()
        for path in provider_root.rglob("*")
        if path.is_file()
    }
    require(provider_files == EXPECTED_PROVIDER_FILES, "V07 provider-scope files changed")
    visible = (provider_root / "src/lib.rs").read_text(encoding="utf-8")
    excluded = (provider_root / "src/vendor/generated.rs").read_text(encoding="utf-8")
    outside = (provider_root / "docs/readme.md").read_text(encoding="utf-8")
    require("TOKEN=V07-FAKE-FILTER-MARKER" in visible, "secret-like filter marker missing")
    require("generated vendor fixture" in excluded, "vendor exclusion fixture missing")
    require("outside the V07 `src`" in outside, "outside-request fixture missing")

    mappings: list[tuple[str, str, str, str]] = []
    for group, group_mappings in groups.items():
        require(isinstance(group_mappings, list) and group_mappings, f"empty V07 group: {group}")
        for mapping in group_mappings:
            require(
                isinstance(mapping, list)
                and len(mapping) == 4
                and all(isinstance(value, str) and value for value in mapping),
                f"invalid V07 mapping in {group}: {mapping!r}",
            )
            mappings.append(tuple(mapping))
    requirement_ids = [requirement for requirement, _, _, _ in mappings]
    require(len(requirement_ids) == len(set(requirement_ids)), "duplicate V07 requirement")
    require(len(mappings) >= 40, "V07 matrix no longer covers the full privacy boundary")
    require(
        "volicord-privacy" in {package for _, package, _, _ in mappings},
        "V07 no longer maps to the Production privacy owner",
    )

    owner_tests = fixture.get("owner_tests")
    require(isinstance(owner_tests, list) and owner_tests, "V07 owner tests are not classified")
    for mapping in owner_tests:
        require(
            isinstance(mapping, list)
            and len(mapping) == 4
            and all(isinstance(value, str) and value for value in mapping),
            f"invalid V07 owner-test mapping: {mapping!r}",
        )
    require(
        not ({mapping[0] for mapping in owner_tests} & set(requirement_ids)),
        "V07 owner tests were counted as product acceptance requirements",
    )
    mappings.extend(tuple(mapping) for mapping in owner_tests)

    current_subjects = set(
        run(["git", "log", "--format=%s"], capture=True).stdout.splitlines()
    )
    require(
        set(PRODUCTION_BASELINE_SUBJECTS) <= current_subjects,
        "V07 Production-fix baseline is not present in current history",
    )

    metadata = json.loads(
        run(
            [
                "cargo",
                "metadata",
                "--manifest-path",
                "rebuild/Cargo.toml",
                "--no-deps",
                "--format-version",
                "1",
            ],
            capture=True,
        ).stdout
    )
    rebuild_root = (ROOT / "rebuild").resolve()
    legacy_names = {
        "volicord-core",
        "volicord-store",
        "volicord-types",
        "volicord-user-action-service",
    }
    for package in metadata["packages"]:
        require(
            Path(package["manifest_path"]).resolve().is_relative_to(rebuild_root),
            f"workspace package is outside rebuild: {package['name']}",
        )
        dependencies = {dependency["name"] for dependency in package["dependencies"]}
        require(
            not (dependencies & legacy_names),
            f"legacy dependency entered {package['name']}: {dependencies & legacy_names}",
        )

    target_mappings: dict[tuple[str, str], set[str]] = {}
    for _, package, target, test in mappings:
        target_mappings.setdefault((package, target), set()).add(test)
    discovered = 0
    for (package, target), expected_tests in sorted(target_mappings.items()):
        command = production_command(package, target)
        catalog = run([*command, "--", "--list"], capture=True).stdout
        missing = sorted(test for test in expected_tests if f"{test}: test" not in catalog)
        require(not missing, f"mapped Production Rust tests are missing: {package}/{target}: {missing}")
        discovered += sum(line.endswith(": test") for line in catalog.splitlines())
        run(command)

    print(
        json.dumps(
            {
                "fixture_sources": len(sources),
                "group_count": len(groups),
                "mapped_requirements": len(requirement_ids),
                "classified_owner_tests": len(owner_tests),
                "production_targets": len(target_mappings),
                "discovered_tests": discovered,
                "provider_fixture_files": len(provider_files),
                "status": "passed",
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
