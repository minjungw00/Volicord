#!/usr/bin/env python3
"""Fetch or inspect revision-pinned public qualification inputs in ignored state."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
MANIFEST = HERE / "external-repositories.json"
DEFAULT_STATE = ROOT / "rebuild/.local/repository-intelligence/external-corpus"


def run(arguments: list[str], *, capture: bool = False) -> str:
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
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(arguments)}")
    return result.stdout.strip() if capture else ""


def load_manifest() -> dict[str, object]:
    return json.loads(MANIFEST.read_text(encoding="utf-8"))


def checkout_path(state: Path, repository: dict[str, object]) -> Path:
    return state / str(repository["id"]) / "checkout"


def fetch(state: Path, manifest: dict[str, object]) -> None:
    state.mkdir(parents=True, exist_ok=True)
    for repository in manifest["repositories"]:
        checkout = checkout_path(state, repository)
        if not checkout.exists():
            checkout.parent.mkdir(parents=True, exist_ok=True)
            run(
                [
                    "git",
                    "clone",
                    "--filter=blob:none",
                    "--no-checkout",
                    str(repository["origin"]),
                    str(checkout),
                ]
            )
        run(["git", "-C", str(checkout), "sparse-checkout", "init", "--no-cone"])
        run(
            [
                "git",
                "-C",
                str(checkout),
                "sparse-checkout",
                "set",
                "--no-cone",
                *[str(path) for path in repository["bounded_input"]],
            ]
        )
        current = run(["git", "-C", str(checkout), "rev-parse", "HEAD"], capture=True)
        if current != repository["revision"]:
            run(
                [
                    "git",
                    "-C",
                    str(checkout),
                    "fetch",
                    "--depth=1",
                    "origin",
                    str(repository["revision"]),
                ]
            )
        run(
            [
                "git",
                "-C",
                str(checkout),
                "checkout",
                "--detach",
                str(repository["revision"]),
            ]
        )
        identity = {
            "schema_version": 1,
            "id": repository["id"],
            "origin": repository["origin"],
            "revision": repository["revision"],
            "license": repository["license"],
            "bounded_input": repository["bounded_input"],
            "manifest_sha256": hashlib.sha256(MANIFEST.read_bytes()).hexdigest(),
        }
        (checkout.parent / "input-identity.json").write_text(
            json.dumps(identity, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )


def inspect(state: Path, manifest: dict[str, object]) -> dict[str, object]:
    results: list[dict[str, object]] = []
    for repository in manifest["repositories"]:
        checkout = checkout_path(state, repository)
        if not checkout.is_dir():
            results.append(
                {
                    "id": repository["id"],
                    "status": "environment_blocked",
                    "reason": "pinned checkout is absent from ignored validation state",
                }
            )
            continue
        try:
            revision = run(["git", "-C", str(checkout), "rev-parse", "HEAD"], capture=True)
            origin = run(
                ["git", "-C", str(checkout), "remote", "get-url", "origin"], capture=True
            )
        except RuntimeError as error:
            results.append(
                {"id": repository["id"], "status": "failed", "reason": str(error)}
            )
            continue
        missing_licenses = [
            path for path in repository["license_files"] if not (checkout / path).is_file()
        ]
        mismatch = []
        if revision != repository["revision"]:
            mismatch.append("revision")
        if origin.rstrip("/") != str(repository["origin"]).rstrip("/"):
            mismatch.append("origin")
        if missing_licenses:
            mismatch.append("license_files")
        results.append(
            {
                "id": repository["id"],
                "status": "passed" if not mismatch else "failed",
                "origin": origin,
                "revision": revision,
                "license": repository["license"],
                "license_files": repository["license_files"],
                "missing_license_files": missing_licenses,
                "bounded_input": repository["bounded_input"],
                "mismatch": mismatch,
            }
        )
    statuses = {result["status"] for result in results}
    overall = (
        "failed"
        if "failed" in statuses
        else "environment_blocked"
        if "environment_blocked" in statuses
        else "passed"
    )
    return {
        "schema_version": 1,
        "kind": "repository_intelligence_external_corpus_status",
        "status": overall,
        "state_root": str(state.relative_to(ROOT)) if state.is_relative_to(ROOT) else str(state),
        "repositories": results,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=("fetch", "status"))
    parser.add_argument("--state", type=Path, default=DEFAULT_STATE)
    arguments = parser.parse_args()
    manifest = load_manifest()
    if arguments.command == "fetch":
        fetch(arguments.state, manifest)
    result = inspect(arguments.state, manifest)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 1 if result["status"] == "failed" else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, ValueError) as error:
        print(f"external corpus orchestration failed: {error}", file=sys.stderr)
        raise SystemExit(1)
