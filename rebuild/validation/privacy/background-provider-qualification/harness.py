#!/usr/bin/env python3
"""Admission-gated qualification for the production background provider."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
from typing import Callable, Sequence


ROOT = Path(__file__).resolve().parents[4]
FIXTURE = Path(__file__).resolve().parent / "fixtures" / "bounded-rust"
AUTHORIZATION_ASSERTION = "openai-codex-background-semantic-bounded-rust-v1"
V11_AUTHORIZATION_ASSERTION = "v11-openai-codex-project-health-three-targets"
AUTHORIZATION_ENV = "VOLICORD_PROVIDER_QUALIFICATION_AUTHORIZATION"
MODEL_ENV = "VOLICORD_PROVIDER_QUALIFICATION_MODEL"
HEAD_ENV = "VOLICORD_PROVIDER_QUALIFICATION_PRODUCTION_HEAD"
EVIDENCE_PREFIX = "VOLICORD_PROVIDER_QUALIFICATION_EVIDENCE="
MAX_SOURCE_BYTES = 4 * 1024


class AuthorizationBlocked(RuntimeError):
    pass


def admit(authorization: str | None) -> None:
    if authorization != AUTHORIZATION_ASSERTION:
        raise AuthorizationBlocked(
            "authorization_blocked: pass --authorize-source-transmission with the exact "
            "background-provider assertion"
        )


def fixture_fingerprint() -> tuple[str, int, int]:
    files = sorted(path for path in FIXTURE.rglob("*") if path.is_file())
    if [path.relative_to(FIXTURE).as_posix() for path in files] != ["src/lib.rs"]:
        raise RuntimeError("fixture must contain exactly the maintained src/lib.rs Source")
    digest = hashlib.sha256()
    total = 0
    for path in files:
        relative = path.relative_to(FIXTURE).as_posix().encode("utf-8")
        content = path.read_bytes()
        total += len(content)
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    if total > MAX_SOURCE_BYTES:
        raise RuntimeError(f"fixture exceeds the {MAX_SOURCE_BYTES}-byte qualification bound")
    return digest.hexdigest(), len(files), total


def cargo_command() -> list[str]:
    return [
        "cargo",
        "test",
        "--manifest-path",
        str(ROOT / "rebuild" / "Cargo.toml"),
        "-p",
        "volicord-operations",
        "--test",
        "provider_live_qualification",
        "live_provider_qualification",
        "--",
        "--ignored",
        "--exact",
        "--nocapture",
        "--test-threads=1",
    ]


def extract_evidence(output: str) -> dict[str, object]:
    evidence_lines = [
        line.split(EVIDENCE_PREFIX, maxsplit=1)[1]
        for line in output.splitlines()
        if EVIDENCE_PREFIX in line
    ]
    if len(evidence_lines) != 1:
        raise RuntimeError("live qualification produced no unique sanitized evidence projection")
    return json.loads(evidence_lines[0])


def audit_evidence(evidence: dict[str, object]) -> None:
    expected = {
        "": {
            "schema_version",
            "qualification_id",
            "production_head",
            "authorization",
            "provider",
            "fixture",
            "success",
            "degradation",
            "retained_evidence",
        },
        "authorization": {"assertion_id", "distinct_from_v11", "supplied_by"},
        "provider": {"identity", "model", "transport", "provider_side_deletion"},
        "fixture": {
            "id",
            "source_count",
            "source_locator",
            "original_bytes",
            "transmitted_bytes",
            "content_sha256",
            "maintained_bytes",
        },
        "success": {
            "guarded_outcome",
            "provider_request_outcome",
            "transmission_outcome",
            "repository_snapshot",
            "analysis_snapshot",
            "semantic_annotation_count",
            "annotation_provenance_complete",
        },
        "degradation": {
            "trigger",
            "guarded_confirmation_consumed",
            "provider_request_outcome",
            "transmission_outcome",
            "local_canonical_continuity",
        },
        "retained_evidence": {"source_body", "provider_response_body", "credential"},
    }
    if set(evidence) != expected[""]:
        raise RuntimeError("sanitized evidence has an unexpected top-level shape")
    for section in expected.keys() - {""}:
        value = evidence.get(section)
        if not isinstance(value, dict) or set(value) != expected[section]:
            raise RuntimeError(f"sanitized evidence section {section} has an unexpected shape")
    retained = evidence["retained_evidence"]
    if not isinstance(retained, dict) or any(value is not False for value in retained.values()):
        raise RuntimeError("sanitized evidence claims retention of forbidden raw material")
    fixture = evidence["fixture"]
    authorization = evidence["authorization"]
    provider = evidence["provider"]
    if (
        not isinstance(fixture, dict)
        or fixture.get("source_locator") != "src/lib.rs"
        or not isinstance(authorization, dict)
        or authorization.get("assertion_id") != AUTHORIZATION_ASSERTION
        or not isinstance(provider, dict)
        or provider.get("identity") != "openai-codex"
    ):
        raise RuntimeError("sanitized evidence does not match the bounded qualification identity")


def execute_live(
    authorization: str | None,
    model: str,
    runner: Callable[..., subprocess.CompletedProcess[str]] = subprocess.run,
) -> dict[str, object]:
    admit(authorization)
    fixture_hash, source_count, source_bytes = fixture_fingerprint()
    if not model.strip():
        raise RuntimeError("--model must name the exact provider model")
    production_head = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    environment = os.environ.copy()
    environment[AUTHORIZATION_ENV] = AUTHORIZATION_ASSERTION
    environment[MODEL_ENV] = model
    environment[HEAD_ENV] = production_head
    completed = runner(
        cargo_command(),
        cwd=ROOT,
        env=environment,
        check=False,
        capture_output=True,
        text=True,
    )
    combined = f"{completed.stdout}\n{completed.stderr}"
    if completed.returncode != 0:
        diagnostic_lines = [
            line.strip()
            for line in combined.splitlines()
            if line.strip().startswith("Error:")
            or line.strip().endswith("... FAILED")
            or line.strip().startswith("test result:")
        ]
        diagnostic = " | ".join(diagnostic_lines[-4:]) or "no sanitized test diagnostic"
        raise RuntimeError(
            "live qualification failed; inspect the focused validation runner artifact "
            f"(cargo exit {completed.returncode}; {diagnostic})"
        )
    evidence = extract_evidence(combined)
    evidence["fixture"]["content_sha256"] = fixture_hash
    evidence["fixture"]["source_count"] = source_count
    evidence["fixture"]["maintained_bytes"] = source_bytes
    audit_evidence(evidence)
    return evidence


def self_test() -> None:
    fixture_fingerprint()
    projected = extract_evidence(
        f"test live_provider_qualification ... {EVIDENCE_PREFIX}"
        '{"schema_version":1,"fixture":{}}\nok'
    )
    if projected["schema_version"] != 1:
        raise AssertionError("sanitized evidence projection was not parsed")
    maintained_evaluation = Path(__file__).resolve().parent / "evaluation.json"
    if maintained_evaluation.is_file():
        audit_evidence(json.loads(maintained_evaluation.read_text(encoding="utf-8")))
    calls: list[Sequence[str]] = []

    def forbidden_runner(command: Sequence[str], **_: object) -> subprocess.CompletedProcess[str]:
        calls.append(command)
        return subprocess.CompletedProcess(command, 99, "", "must not run")

    for assertion in (None, "", V11_AUTHORIZATION_ASSERTION, "operator-approved"):
        try:
            execute_live(assertion, "self-test-model", forbidden_runner)
        except AuthorizationBlocked:
            pass
        else:
            raise AssertionError(f"non-exact assertion was admitted: {assertion!r}")
    if calls:
        raise AssertionError("authorization refusal invoked the live runner")
    print("background-provider qualification self-test passed: blocked assertions made zero live invocations")


def parse_args(arguments: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--self-test", action="store_true")
    mode.add_argument("--live", action="store_true")
    parser.add_argument("--authorize-source-transmission")
    parser.add_argument("--model")
    parser.add_argument("--evidence-output", type=Path)
    return parser.parse_args(arguments)


def main(arguments: list[str]) -> int:
    args = parse_args(arguments)
    if args.self_test:
        self_test()
        return 0
    if args.model is None:
        raise RuntimeError("--live requires --model")
    evidence = execute_live(args.authorize_source_transmission, args.model)
    encoded = json.dumps(evidence, indent=2, sort_keys=True) + "\n"
    if args.evidence_output is not None:
        args.evidence_output.parent.mkdir(parents=True, exist_ok=True)
        args.evidence_output.write_text(encoded, encoding="utf-8")
    print(encoded, end="")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main(sys.argv[1:]))
    except AuthorizationBlocked as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(3)
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"qualification_failed: {error}", file=sys.stderr)
        raise SystemExit(1)
