#!/usr/bin/env python3
"""Run maintained realistic Repository Intelligence qualification assertions."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
QUALIFICATION = HERE / "qualification.json"
EXTERNAL = HERE / "external_corpus.py"
EXTERNAL_STATE = ROOT / "rebuild/.local/repository-intelligence/external-corpus"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def run(arguments: list[str], *, environment: dict[str, str] | None = None) -> None:
    print(f"$ {' '.join(arguments)}", flush=True)
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        env=environment,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(arguments)}")


def main() -> int:
    qualification = json.loads(QUALIFICATION.read_text(encoding="utf-8"))
    languages = qualification["tier_1"]["languages"]
    require(
        [item["language"] for item in languages]
        == ["java", "python", "javascript", "typescript", "c", "cpp", "rust"],
        "Tier 1 language matrix must contain the seven official structural languages",
    )
    for item in languages:
        fixture = HERE / item["path"]
        require(fixture.is_dir(), f"missing realistic fixture: {item['language']}")
        source_files = [path for path in fixture.rglob("*") if path.is_file()]
        require(len(source_files) >= 5, f"{item['language']} fixture is not multi-file")
        require(item["difficult_constructs"], f"{item['language']} has no difficult cases")

    cargo = [
        "cargo",
        "test",
        "--manifest-path",
        "rebuild/Cargo.toml",
        "-p",
        "volicord-repository-intelligence",
    ]
    run([*cargo, "--test", "realistic_qualification"])
    run(
        [
            *cargo,
            "--lib",
            "structural::tests::injected_adapter_failure_is_bounded_to_one_language",
            "--",
            "--exact",
        ]
    )

    with tempfile.TemporaryDirectory(prefix="volicord-external-corpus-absent-") as absent:
        absent_status = subprocess.run(
            [sys.executable, str(EXTERNAL), "status", "--state", absent],
            cwd=ROOT,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
        )
        require(absent_status.returncode == 0, "absent external corpus status failed")
        absent_result = json.loads(absent_status.stdout)
        require(
            absent_result["status"] == "environment_blocked"
            and all(
                item["status"] == "environment_blocked"
                for item in absent_result["repositories"]
            ),
            "absent external corpus was not explicitly environment_blocked",
        )

    status = subprocess.run(
        [sys.executable, str(EXTERNAL), "status", "--state", str(EXTERNAL_STATE)],
        cwd=ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
    )
    require(status.returncode == 0, "external corpus status failed")
    external = json.loads(status.stdout)
    if external["status"] == "passed":
        environment = os.environ.copy()
        environment["VOLICORD_EXTERNAL_CORPUS_ROOT"] = str(EXTERNAL_STATE)
        run(
            [
                *cargo,
                "--test",
                "realistic_qualification",
                "external_repositories_when_provided_are_bounded_and_honest",
                "--",
                "--exact",
            ],
            environment=environment,
        )

    print(
        json.dumps(
            {
                "schema_version": 1,
                "kind": "repository_intelligence_realistic_qualification",
                "local_status": "passed",
                "tier_1_language_count": len(languages),
                "semantic_ecosystem_count": len(
                    qualification["tier_1"]["semantic_gold"]["ecosystems"]
                ),
                "polyglot_boundary_kinds": qualification["tier_1"]["polyglot"][
                    "boundary_kinds"
                ],
                "external_status": external["status"],
                "external_repositories": external["repositories"],
                "status": (
                    "passed"
                    if external["status"] == "passed"
                    else "passed_with_external_environment_blocked"
                ),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, RuntimeError, ValueError) as error:
        print(f"realistic qualification failed: {error}", file=sys.stderr)
        raise SystemExit(1)
