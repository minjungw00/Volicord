#!/usr/bin/env python3
"""Black-box parity checks for maintained validation argv and current Clap."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import tempfile


REMOVED_FORMS = (
    ("project", "init", "Old"),
    ("health", "PROJECT"),
    ("recall", "PROJECT"),
    ("portable", "export", "PROJECT", "bundle.json"),
    ("documents", "export", "PROJECT", "handoff-resume", "markdown", "out.md", "en"),
    ("repair", "PROJECT", "derived-analysis"),
    ("reindex", "PROJECT"),
    ("codex", "enable", "/repository"),
)


def run(binary: Path, arguments: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(binary), *arguments],
        cwd=cwd,
        text=True,
        capture_output=True,
        check=False,
    )


def require_non_usage(result: subprocess.CompletedProcess[str], label: str) -> None:
    if result.returncode == 2:
        raise AssertionError(f"current parser rejected maintained {label}: {result.stderr}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    arguments = parser.parse_args()
    binary = arguments.binary.resolve()
    if not binary.is_file():
        raise AssertionError(f"current CLI binary is missing: {binary}")
    with tempfile.TemporaryDirectory(prefix="volicord-current-cli-parity-") as directory:
        root = Path(directory)
        runtime = root / "runtime"
        repository = root / "repository"
        repository.mkdir()
        initialized = run(
            binary,
            ["--runtime", str(runtime), "--repository", str(repository), "--json", "init", "Parity"],
            repository,
        )
        if initialized.returncode != 0:
            raise AssertionError(f"parity fixture initialization failed: {initialized.stderr}")
        project_id = json.loads(initialized.stdout)["project_id"]
        maintained = (
            ("codex enable", ["--runtime", str(runtime), "--repository", str(repository), "codex", "enable"]),
            ("context export", ["--runtime", str(runtime), "--repository", str(repository), "context", "export", "--output", str(root / "bundle.json")]),
            ("document export", ["--runtime", str(runtime), "--repository", str(repository), "document", "export", "handoff-resume", "--format", "markdown", "--output", str(root / "handoff.md"), "--language", "en"]),
            ("doctor check", ["--runtime", str(runtime), "--repository", str(repository), "doctor", "check"]),
            ("doctor repair", ["--runtime", str(runtime), "--repository", str(repository), "doctor", "repair"]),
            ("doctor reindex", ["--runtime", str(runtime), "--repository", str(repository), "doctor", "reindex"]),
            ("advanced source", ["--runtime", str(runtime), "--repository", str(repository), "advanced", "records", "source", "--host", "parity", "--session", "current-cli", "--text", "parser parity"]),
            ("explicit Project recall", ["--runtime", str(runtime), "--project", project_id, "recall"]),
        )
        for label, argv in maintained:
            require_non_usage(run(binary, argv, repository), label)
        for removed in REMOVED_FORMS:
            result = run(binary, ["--runtime", str(runtime), *removed], repository)
            if result.returncode != 2:
                raise AssertionError(
                    f"removed CLI form did not fail as usage: {removed!r}: "
                    f"exit={result.returncode} stdout={result.stdout!r} stderr={result.stderr!r}"
                )
    print(json.dumps({"status": "passed", "maintained_shapes": len(maintained), "removed_shapes": len(REMOVED_FORMS)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
