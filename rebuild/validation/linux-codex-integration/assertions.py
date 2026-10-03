#!/usr/bin/env python3
"""Map V08 requirements to the maintained clean journey and Rust oracles."""

from __future__ import annotations

import ast
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
CODEX_PROBE = DIRECTORY / "codex_probe.py"
REPORT = DIRECTORY / "report.md"
PHASE_SUMMARY = ROOT / "rebuild/validation/phase-7-summary.md"
EXPECTED_GROUPS = {
    "clean_linux_install",
    "codex_and_host",
    "viewer_http",
    "mcp_schema_client",
    "guarded_fallback",
    "analysis_recovery",
    "failure_cleanup_and_exclusion",
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
    # Historical report/commit provenance has a separate explicit entry point.
    # Current checks use the executable matrix and present Product invariants.
    lifecycle = {row[0]: row[3] for row in fixture["groups"]["clean_linux_install"]}
    require(lifecycle["V08-I07-uninstall-preserves"] ==
            "uninstall removes binaries while logical canonical context and local Project binding survive",
            "V08-I07 lost logical canonical continuity")
    require(lifecycle["V08-I08-reinstall-preserves"] ==
            "reinstall preserves canonical context by complete typed portable export equality while repository-derived Recall freshness truthfully changes after owned integration removal; fresh analysis adds only its repository Source and restores current freshness",
            "V08-I08 lost canonical-versus-derived continuity")
    harness_text = HARNESS.read_text(encoding="utf-8")
    require("reinstall_preserved_recall" not in harness_text,
            "V08 reintroduced whole-Recall persistence evidence")
    tree = ast.parse(harness_text)
    for node in ast.walk(tree):
        if isinstance(node, ast.Compare):
            names = {child.id for child in ast.walk(node) if isinstance(child, ast.Name)}
            require(not {"recall_before", "recall_after"} <= names,
                    "V08 reintroduced whole-Recall equality as canonical preservation")
    calls = {node.func.id for node in ast.walk(tree)
             if isinstance(node, ast.Call) and isinstance(node.func, ast.Name)}
    require({"assert_canonical_continuity", "assert_reinstall_recall",
             "exercise_reinstall_negative_checks", "assert_only_repository_observation_added"} <= calls,
            "V08 lost executable canonical, freshness or negative regression checks")

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
    require(len(mappings) >= 75, "V08 matrix no longer covers the corrected integration boundary")

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
    require("--setup-codex" not in installer_text, "installer retains the global Codex setup mode")
    require("codex mcp" not in installer_text, "installer retains global MCP registration")
    codex_source = (
        ROOT / "rebuild/crates/volicord-operations/src/codex.rs"
    ).read_text(encoding="utf-8")
    for required in (
        "mcp_servers",
        "SessionStart",
        "startup|resume|clear|compact",
        "required",
        "volicord-integration.json",
        "Codex integration conflict",
    ):
        require(required in codex_source, f"repository Codex integration is missing {required}")
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

    run(["cargo", "test", "--manifest-path", "rebuild/Cargo.toml", "-p",
         "volicord-operations", "--lib", "codex::tests", "--all-features"])
    run(["cargo", "test", "--manifest-path", "rebuild/Cargo.toml", "-p",
         "volicord-operations", "--test", "explanation_cli", "--all-features"])
    run([str(HARNESS)])
    run([sys.executable, str(ROOT / "rebuild/validation/shared/current_cli_parity.py"),
         "--binary", str(ROOT / "rebuild/target/debug/volicord")])
    print(
        json.dumps(
            {
                "group_count": len(fixture["groups"]),
                "mapped_requirements": len(mappings),
                "production_targets": len(target_mappings),
                "discovered_tests": discovered,
                "deterministic_v08_journey": "passed",
                "authenticated_codex_probe": "environment-dependent; run and inspect separately",
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
