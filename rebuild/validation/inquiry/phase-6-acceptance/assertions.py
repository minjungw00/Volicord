#!/usr/bin/env python3
"""Map V09/Phase 6 requirements to Production Rust tests and execute them."""

from __future__ import annotations

import json
from pathlib import Path
import shlex
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[4]
FIXTURE = ROOT / "rebuild/validation/inquiry/phase-6-acceptance/fixtures/phase-6-matrix.json"
MANIFEST = ROOT / "rebuild/validation/shared/fixture-manifest.json"

PACKAGE_COMMANDS = {
    "volicord-inquiry": [
        "cargo", "test", "--manifest-path", "rebuild/Cargo.toml",
        "-p", "volicord-inquiry", "--all-targets", "--all-features",
    ],
    "volicord-projections": [
        "cargo", "test", "--manifest-path", "rebuild/Cargo.toml",
        "-p", "volicord-projections", "--all-targets", "--all-features",
    ],
    "volicord-context": [
        "cargo", "test", "--manifest-path", "rebuild/Cargo.toml",
        "-p", "volicord-context", "--all-features",
        "--test", "inquiry", "--test", "context_checkpoint", "--test", "lifecycle",
        "--test", "canonical_read", "--test", "process_reopen",
    ],
}

EXPECTED_GROUPS = {
    "candidate_lifecycle",
    "promotion_and_frontier",
    "response_and_decision",
    "checkpoint",
    "recall_and_inspection",
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
    require(fixture.get("schema_version") == 1, "V09 fixture schema_version changed")
    require(fixture.get("validation_id") == "V09", "fixture is not V09 evidence")
    groups = fixture.get("groups")
    require(isinstance(groups, dict), "V09 fixture groups must be an object")
    require(set(groups) == EXPECTED_GROUPS, "V09 evidence groups changed without review")

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    entries = [item for item in manifest.get("fixtures", []) if item.get("validation_id") == "V09"]
    require(
        [item.get("id") for item in entries] == ["v09-phase-6-matrix"],
        "V09 manifest entry is missing or ambiguous",
    )

    requirements: list[tuple[str, str, str]] = []
    for group, mappings in groups.items():
        require(isinstance(mappings, list) and mappings, f"V09 group is empty: {group}")
        for mapping in mappings:
            require(
                isinstance(mapping, list)
                and len(mapping) == 3
                and all(isinstance(value, str) and value for value in mapping),
                f"invalid V09 mapping in {group}: {mapping!r}",
            )
            requirements.append(tuple(mapping))
    identities = [requirement for requirement, _, _ in requirements]
    require(len(identities) == len(set(identities)), "duplicate V09 requirement identity")
    require(len(requirements) >= 75, "V09 matrix no longer covers the complete Phase 6 surface")
    require(
        {package for _, package, _ in requirements} == set(PACKAGE_COMMANDS),
        "V09 must retain all three Production Rust semantic owners",
    )

    catalogs: dict[str, str] = {}
    production_tests: dict[str, int] = {}
    for package, command in PACKAGE_COMMANDS.items():
        catalog = run([*command, "--", "--list"], capture=True).stdout
        catalogs[package] = catalog
        production_tests[package] = sum(
            line.endswith(": test") for line in catalog.splitlines()
        )
    missing = sorted(
        f"{package}:{test}"
        for _, package, test in requirements
        if f"{test}: test" not in catalogs[package]
    )
    require(not missing, f"mapped Production Rust tests are missing: {missing}")

    for command in PACKAGE_COMMANDS.values():
        run(command)

    print(
        json.dumps(
            {
                "group_count": len(groups),
                "mapped_requirements": len(requirements),
                "production_tests": production_tests,
                "status": "passed",
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
