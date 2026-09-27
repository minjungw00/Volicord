#!/usr/bin/env python3
"""Bounded cross-boundary regressions for the Phase 8 remediation sequence."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import time


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def command(identity: str, coverage: list[str], argv: list[str], cwd: Path) -> dict:
    started = time.monotonic_ns()
    completed = subprocess.run(
        argv,
        cwd=cwd,
        env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
        stdin=subprocess.DEVNULL,
        check=False,
    )
    return {
        "id": identity,
        "coverage": coverage,
        "argv": argv,
        "exit_code": completed.returncode,
        "duration_ms": round((time.monotonic_ns() - started) / 1_000_000, 3),
    }


def main() -> int:
    # Campaign owns repository state, blind dimensions, reconciliation,
    # evidence controls and long-lived Project classes. Assertions/harness own
    # frontier, review workflow and qualification classes. Final owns every
    # selected Rust test formerly repeated here with identical configuration.
    definitions = [
        ("dogfood-machine-findings",
         ["finite_machine_authority", "confirmed_failure_attribution",
          "immutable_evaluation_and_durable_result_lineage"],
         [sys.executable, "-B", "-m", "unittest", "-v",
          "machine_findings_self_test.MachineFindingTests"], HERE),
    ]
    results = [command(*definition) for definition in definitions]
    missing = sorted({item for result in results for item in result["coverage"]
                      if result["exit_code"] != 0})
    outcome = {
        "kind": "phase8_remediation_integration_regressions",
        "schema_version": 2,
        "status": "passed" if not missing else "failed",
        "checks": results,
        "failed_coverage": missing,
    }
    print(json.dumps(outcome, indent=2, sort_keys=True))
    return 0 if not missing else 1


if __name__ == "__main__":
    raise SystemExit(main())
