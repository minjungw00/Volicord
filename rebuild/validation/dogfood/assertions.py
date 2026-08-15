#!/usr/bin/env python3
"""Self-test the Phase 8 dogfood evaluation support boundary."""

from __future__ import annotations

import ast
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
HARNESS = HERE / "harness.py"


def main() -> int:
    source = HARNESS.read_text(encoding="utf-8")
    tree = ast.parse(source)
    imports = {
        alias.name
        for node in ast.walk(tree)
        if isinstance(node, ast.Import)
        for alias in node.names
    }
    if "requests" in imports or "psutil" in imports:
        raise AssertionError("Phase 8 harness unexpectedly added a process/network framework dependency")
    if "rebuild/scripts/validate final" in source:
        raise AssertionError("Phase 8 harness may not invoke direct final validation")
    if "rebuild/scripts/validate gate" in source:
        raise AssertionError("Phase 8 harness may not own the final gate")
    if "rehearse_target" not in source or "V11_HARNESS" not in source:
        raise AssertionError("Phase 8 harness no longer reuses the maintained product journey")
    result = subprocess.run(
        [sys.executable, str(HARNESS), "self-test"],
        cwd=ROOT,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"Phase 8 harness self-test failed with exit {result.returncode}")
    print("phase 8 dogfood assertions passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
