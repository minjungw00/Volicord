#!/usr/bin/env python3
"""Map V08 requirements to the maintained clean journey and Rust oracles."""

from __future__ import annotations

import json
from pathlib import Path
import shlex
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[3]
DIRECTORY = ROOT / "rebuild/validation/linux-codex-integration"
FIXTURE = DIRECTORY / "fixtures/v08-matrix.json"
MANIFEST = ROOT / "rebuild/validation/shared/fixture-manifest.json"
HARNESS = DIRECTORY / "harness.py"
REPORT = DIRECTORY / "report.md"
PHASE_SUMMARY = ROOT / "rebuild/validation/phase-7-summary.md"
EXPECTED_GROUPS = {
    "clean_linux_install",
    "codex_and_host",
    "guarded_and_viewer",
    "failure_cleanup_and_exclusion",
}
EXPECTED_COMMITS = {
    "viewer": (
        "a6355a9edf5a587a17ad93eeb8357d1de977ba54",
        "feat: add local project viewer",
    ),
    "host_and_install": (
        "85c876033c35acb5ad95eee3dec223fc91213f50",
        "feat: add Linux Codex host integration",
    ),
    "current_host_source": (
        "bec6424ee0e7a7f378f2fc799bb58e201cc0c00f",
        "fix: preserve current-host Source observer",
    ),
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
        unavailable = " (environment-dependent Codex evidence unavailable)" if result.returncode == 77 else ""
        raise RuntimeError(
            f"command failed with exit {result.returncode}{unavailable}: {shlex.join(arguments)}"
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
    require(fixture.get("schema_version") == 1, "V08 fixture schema_version changed")
    require(fixture.get("validation_id") == "V08", "fixture is not V08 evidence")
    require(set(fixture.get("groups", {})) == EXPECTED_GROUPS, "V08 evidence groups changed")
    require(
        fixture.get("production_commits")
        == {key: value[0] for key, value in EXPECTED_COMMITS.items()},
        "V08 Production commit identities changed",
    )

    for role, (commit, expected_subject) in EXPECTED_COMMITS.items():
        subject = run(["git", "show", "-s", "--format=%s", commit], capture=True).stdout.strip()
        require(subject == expected_subject, f"{role} Production commit subject changed")

    changed_since_production = run(
        ["git", "diff", "--name-only", EXPECTED_COMMITS["current_host_source"][0]], capture=True
    ).stdout.splitlines()
    production_drift = [
        path
        for path in changed_since_production
        if (
            path.startswith("rebuild/crates/")
            and path != "rebuild/crates/volicord-host/tests/mcp.rs"
        )
        or path in {
            "rebuild/Cargo.toml",
            "rebuild/Cargo.lock",
            "rebuild/install.sh",
            "rebuild/docs/linux-codex-setup.md",
        }
    ]
    require(not production_drift, f"V08 contains Production drift: {production_drift}")

    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    v08_entries = [
        entry for entry in manifest.get("fixtures", []) if entry.get("validation_id") == "V08"
    ]
    require(
        [entry.get("id") for entry in v08_entries] == ["v08-linux-codex-integration"],
        "V08 fixture-manifest entry is missing or ambiguous",
    )
    require(REPORT.is_file(), "maintained V08 report is missing")
    require(PHASE_SUMMARY.is_file(), "maintained Phase 7 summary is missing")
    report_text = REPORT.read_text(encoding="utf-8")
    phase_summary = PHASE_SUMMARY.read_text(encoding="utf-8")
    for validation_id in ("V06", "V07", "V08", "V10"):
        require(
            f"| {validation_id} | passed |" in phase_summary,
            f"Phase 7 summary does not record {validation_id} as passed",
        )
    require(
        "No accepted Q1–Q13 Decision revisit trigger is active" in phase_summary,
        "Phase 7 summary hides the accepted-Decision revisit-trigger status",
    )
    require("V11" in report_text and "not" in report_text, "V08 report hides the V11 exclusion")
    normalized_report = " ".join(report_text.split())
    require(
        "final aggregate has not yet been run" in normalized_report,
        "V08 report must not pre-claim the final aggregate",
    )

    mappings: list[tuple[str, str, str, str]] = []
    for group, values in fixture["groups"].items():
        require(isinstance(values, list) and values, f"empty V08 group: {group}")
        for mapping in values:
            require(
                isinstance(mapping, list)
                and len(mapping) == 4
                and all(isinstance(value, str) and value for value in mapping),
                f"invalid V08 mapping in {group}: {mapping!r}",
            )
            mappings.append(tuple(mapping))
    requirement_ids = [mapping[0] for mapping in mappings]
    require(len(requirement_ids) == len(set(requirement_ids)), "duplicate V08 requirement")
    require(len(mappings) >= 35, "V08 matrix no longer covers the complete integration boundary")

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
    prohibited_dependencies = {
        "volicord-core",
        "volicord-store",
        "volicord-types",
        "volicord-user-action-service",
        "volicord-mcp-protocol",
        "volicord-mcp-server",
    }
    for package in metadata["packages"]:
        require(
            Path(package["manifest_path"]).resolve().is_relative_to(rebuild_root),
            f"workspace package is outside rebuild: {package['name']}",
        )
        dependencies = {dependency["name"] for dependency in package["dependencies"]}
        require(
            not dependencies & prohibited_dependencies,
            f"legacy dependency entered {package['name']}: {dependencies & prohibited_dependencies}",
        )

    installer_text = (ROOT / "rebuild/install.sh").read_text(encoding="utf-8")
    for forbidden in ("VOLICORD_HOME", ".volicord", "migrate", "import", "backup"):
        require(forbidden not in installer_text, f"installer contains active legacy path token: {forbidden}")
    host_text = (ROOT / "rebuild/crates/volicord-host/src/mcp.rs").read_text(encoding="utf-8")
    for forbidden in ("volicord_mcp_protocol", "write_ticket", "final_acceptance", '"intake"'):
        require(forbidden not in host_text, f"host contains legacy MCP surface: {forbidden}")

    target_mappings: dict[tuple[str, str], set[str]] = {}
    for _, evidence, target, oracle in mappings:
        if evidence != "rust":
            continue
        package, test_target = target.split("/", maxsplit=1)
        target_mappings.setdefault((package, test_target), set()).add(oracle)
    discovered = 0
    for (package, target), expected_tests in sorted(target_mappings.items()):
        command = production_command(package, target)
        catalog = run([*command, "--", "--list"], capture=True).stdout
        missing = sorted(test for test in expected_tests if f"{test}: test" not in catalog)
        require(not missing, f"mapped Rust tests are missing: {package}/{target}: {missing}")
        discovered += sum(line.endswith(": test") for line in catalog.splitlines())
        run(command)

    run([str(HARNESS)])
    print(
        json.dumps(
            {
                "group_count": len(fixture["groups"]),
                "mapped_requirements": len(mappings),
                "production_targets": len(target_mappings),
                "discovered_tests": discovered,
                "real_codex_journey": "passed",
                "status": "passed",
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"V08 assertions failed: {error}", file=sys.stderr)
        raise SystemExit(1)
