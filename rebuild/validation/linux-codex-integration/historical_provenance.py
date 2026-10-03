#!/usr/bin/env python3
"""Inspect preserved Phase 7 provenance only; never current conformance."""
from assertions import ROOT, FIXTURE, MANIFEST, REPORT, PHASE_SUMMARY, CODEX_PROBE, require, run
import json
import sys

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
    "viewer_http": (
        "55271418ea9f7b621a31250bf086194e7ac92dfd",
        "fix: make local viewer interactions live",
    ),
    "mcp_schemas": (
        "ecef64e1a3516f4a1aa2ceaaebcc8b84f8b60183",
        "fix: publish exact MCP tool schemas",
    ),
    "analysis_recovery": (
        "369402c6065232b4ef0a0534340b1b2a447436ad",
        "feat: add derived analysis repair and reindex",
    ),
    "fresh_repository_sources": (
        "5c20f53a1aa7c0cf64767a3c10e54c0b719f5d6a",
        "fix: bind rebuilds to fresh repository sources",
    ),
    "viewer_request_trust": (
        "3b48545bd9e2a224d6feb75ae1c743d1af31f4cf",
        "fix: authenticate local viewer mutations",
    ),
}


def main():
    fixture = json.loads(FIXTURE.read_text())
    require(fixture['production_commits'] == {k: v[0] for k, v in EXPECTED_COMMITS.items()},
            'historical provenance changed')
    for role, (commit, expected_subject) in EXPECTED_COMMITS.items():
        subject = run(["git", "show", "-s", "--format=%s", commit], capture=True).stdout.strip()
        require(subject == expected_subject, f"{role} Production commit subject changed")

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
    require(CODEX_PROBE.is_file(), "maintained authenticated Codex probe is missing")
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
    require(
        "authenticated Codex product-tool probe passed" in normalized_report,
        "V08 report does not preserve the observed model-driven product-tool result",
    )
    require(
        "byte-identical after both operations" not in normalized_report
        and "portable canonical bytes remain identical" not in normalized_report,
        "V08 report still requires whole-bundle equality after a repository observation",
    )
    require(
        "request-authenticity" in normalized_report
        and "repository Source" in normalized_report,
        "V08 report omits a corrected provenance or viewer-trust boundary",
    )

    print(json.dumps({'status': 'passed', 'scope': 'historical_provenance_only',
                      'current_product_result': 'not_run'}))
    return 0

if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, RuntimeError, ValueError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1)
