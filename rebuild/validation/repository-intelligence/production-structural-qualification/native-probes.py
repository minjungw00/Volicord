#!/usr/bin/env python3
"""Reproducible local probes for language-native structural candidates."""

from __future__ import annotations

import ast
import json
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[4]
FIXTURES = ROOT / "rebuild/validation/repository-intelligence/polyglot-structural/fixtures"


def executable(name: str) -> dict[str, object]:
    path = shutil.which(name)
    return {"available": path is not None, "path": path}


def command(argv: list[str], cwd: Path | None = None) -> dict[str, object]:
    completed = subprocess.run(
        argv,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
    )
    return {
        "argv": argv,
        "exit_code": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
    }


def python_ast_probe() -> dict[str, object]:
    path = FIXTURES / "python/src/greeter/core.py"
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=path.as_posix())
    declarations = []
    for node in ast.walk(tree):
        if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            declarations.append(
                {
                    "kind": type(node).__name__,
                    "name": node.name,
                    "start_line": node.lineno,
                    "end_line": node.end_lineno,
                    "start_column_utf8_bytes": node.col_offset,
                    "end_column_utf8_bytes": node.end_col_offset,
                }
            )
    return {
        "machine_readable_tree": True,
        "coordinate_convention": "one-based lines, zero-based UTF-8 byte columns",
        "declarations": sorted(declarations, key=lambda item: (item["start_line"], item["name"])),
    }


def main() -> None:
    tools = {name: executable(name) for name in ("python3", "gcc", "g++", "rustc", "javac", "node", "tsc")}
    probes: dict[str, object] = {"python_ast": python_ast_probe()}
    if tools["gcc"]["available"]:
        probes["gcc_syntax"] = command(
            ["gcc", "-std=c11", "-Iinclude", "-fsyntax-only", "src/greeter.c", "tests/test_greeter.c"],
            FIXTURES / "c",
        )
    if tools["g++"]["available"]:
        probes["gxx_syntax"] = command(
            ["g++", "-std=c++17", "-Iinclude", "-fsyntax-only", "src/greeter.cpp", "tests/test_greeter.cpp"],
            FIXTURES / "cpp",
        )
    if tools["rustc"]["available"]:
        with tempfile.TemporaryDirectory(prefix="volicord-structural-probe-") as directory:
            probes["rustc_metadata"] = command(
                [
                    "rustc",
                    "--edition=2021",
                    "--crate-type=lib",
                    "--emit=metadata",
                    "-o",
                    str(Path(directory) / "greeter.rmeta"),
                    "crates/greeter/src/lib.rs",
                ],
                FIXTURES / "rust",
            )
    print(
        json.dumps(
            {
                "format": "volicord.native_structural_candidate_probes.v1",
                "platform": platform.platform(),
                "tools": tools,
                "probes": probes,
                "external_source_transmission": False,
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
