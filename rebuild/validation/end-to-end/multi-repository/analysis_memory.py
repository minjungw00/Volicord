#!/usr/bin/env python3
"""Focused local reproduction for large Analysis read/use/write memory."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
LOCAL_ROOT = ROOT / "rebuild/.local/analysis-memory"
sys.path.insert(0, str(HERE))

import harness as v11  # noqa: E402


def run(argv: list[str], *, cwd: Path, env: dict[str, str]) -> str:
    completed = subprocess.run(
        argv,
        cwd=cwd,
        env=env,
        stdin=subprocess.DEVNULL,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"focused Analysis memory setup failed with exit {completed.returncode}"
        )
    return completed.stdout


def cli_json(binary: Path, cwd: Path, env: dict[str, str], *arguments: str) -> dict:
    output = run([str(binary), "--json", *arguments], cwd=cwd, env=env)
    value = json.loads(output)
    if not isinstance(value, dict):
        raise RuntimeError("focused Analysis memory setup returned non-object JSON")
    return value


def atomic_alternative(source_id: str, alternative_id: str, summary: str) -> dict:
    return {
        "alternative_id": alternative_id,
        "summary": summary,
        "technical_consequences": ["The internal representation changes"],
        "material_decomposition": {
            "state": "materially_atomic",
            "rationale": "The bounded diagnostic Source fixes public behavior.",
            "residual_fork_closure": {
                "interaction_comparisons": [],
                "fixed_outcome": "Preserve the current Repository Intelligence contract",
                "credible_implementations": [
                    "Ordered internal representation",
                    "Indexed internal representation",
                ],
                "remaining_material_outcomes": [],
                "source_basis": [source_id],
            },
        },
    }


def latest_manifest(runtime: Path, project_id: str) -> dict:
    paths = list((runtime / "derived/analysis" / project_id).glob("*.json"))
    if not paths:
        raise RuntimeError("focused Analysis memory workload produced no manifest")
    values = [json.loads(path.read_text(encoding="utf-8")) for path in paths]
    return max(
        values,
        key=lambda value: (
            value.get("generated_at_unix_micros", 0),
            value.get("identity", ""),
        ),
    )


def storage_totals(runtime: Path, project_id: str) -> tuple[int, int]:
    logical = 0
    physical = 0
    for path in (runtime / "derived/analysis" / project_id).rglob("*"):
        if not path.is_file():
            continue
        stat = path.stat()
        logical += stat.st_size
        physical += getattr(stat, "st_blocks", 0) * 512
    return logical, physical


def analysis_contract(manifest: dict, analysis: dict) -> dict:
    capability_states: dict[str, int] = {}
    for report in analysis.get("capability_reports", []):
        state = str(report.get("state", "missing"))
        capability_states[state] = capability_states.get(state, 0) + 1
    value = {
        "inventory_entry_count": manifest.get("inventory_entry_count"),
        "entity_count": manifest.get("entity_count"),
        "relation_count": manifest.get("relation_count"),
        "scalar_count": manifest.get("scalar_count"),
        "capability_state_counts": dict(sorted(capability_states.items())),
        "diagnostic_count": len(analysis.get("diagnostics", [])),
        "diagnostics_omitted_count": analysis.get("diagnostics_omitted_count"),
    }
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return {**value, "fingerprint_sha256": hashlib.sha256(encoded).hexdigest()}


def workload(output: Path, repository_revision: str | None) -> dict:
    if not output.is_relative_to(LOCAL_ROOT):
        raise ValueError("diagnostic output must remain under rebuild/.local/analysis-memory")
    output.mkdir(parents=True, exist_ok=False)
    repository = output / "repository"
    prefix = output / "prefix"
    runtime = output / "runtime"
    diagnostic_home = output / "home"
    diagnostic_home.mkdir()
    env = os.environ.copy()
    original_home = Path.home()
    env.update({
        "HOME": str(diagnostic_home),
        "CARGO_HOME": env.get("CARGO_HOME", str(original_home / ".cargo")),
        "RUSTUP_HOME": env.get("RUSTUP_HOME", str(original_home / ".rustup")),
        "VOLICORD_RUNTIME_DIR": str(runtime),
        "PYTHONDONTWRITEBYTECODE": "1",
    })
    run(
        ["git", "clone", "--local", "--no-hardlinks", str(ROOT), str(repository)],
        cwd=ROOT,
        env=env,
    )
    if repository_revision:
        run(
            ["git", "checkout", "--detach", repository_revision],
            cwd=repository,
            env=env,
        )
    analyzed_revision = run(
        ["git", "rev-parse", "HEAD"], cwd=repository, env=env
    ).strip()
    run(
        [
            str(ROOT / "rebuild/install.sh"),
            "--prefix",
            str(prefix),
            "--runtime-dir",
            str(runtime),
        ],
        cwd=ROOT,
        env=env,
    )
    cli = prefix / "bin/volicord"
    mcp = prefix / "bin/volicord-mcp"
    initialized = cli_json(cli, repository, env, "init", "Analysis memory diagnostic")
    project_id = initialized.get("project_id")
    if not isinstance(project_id, str):
        raise RuntimeError("focused Analysis memory Project initialization failed")
    cli_json(cli, repository, env, "analyze")

    collector = v11.performance_module.Collector()
    collector.enabled = True
    v11.PERFORMANCE = collector
    started = time.monotonic_ns()
    host = v11.Mcp(mcp, env)
    try:
        host.initialize()
        goal, goal_ok = host.tool(
            "context_record",
            {
                "project_id": project_id,
                "user_turn": "Exercise the maintained Analysis memory path",
                "role": "goal",
                "statement": "Exercise the maintained Analysis memory path",
            },
        )
        analysis, analysis_ok = host.tool(
            "repository_analyze",
            {"project_id": project_id, "excluded_paths": []},
        )
        source_ids = v11.candidate_repository_source_basis(analysis)
        if not goal_ok or not analysis_ok or not source_ids:
            raise RuntimeError("focused Analysis memory prerequisites failed")
        source_id = source_ids[0]
        choice_id = "analysis-memory-representation"
        choices = [{
            "choice_id": choice_id,
            "summary": "Represent bounded internal analysis state",
            "affected_scope": ["repository-intelligence"],
            "alternatives": [
                atomic_alternative(source_id, "ordered", "Use ordered records"),
                atomic_alternative(source_id, "indexed", "Use an internal index"),
            ],
            "technical_consequences": [
                "The alternatives change private organization only"
            ],
            "source_ids": [source_id],
            "effect_categories": ["implementation_internal"],
            "relationship": {"state": "independent"},
            "evidence_state": "sufficient",
        }]
        discovery, discovery_ok = host.tool(
            "engineering_choice_discovery",
            {
                "project_id": project_id,
                "goal_context_id": goal.get("context_item_id"),
                "baseline_analysis_snapshot_id": analysis.get("analysis_snapshot_id"),
                "source_operation": "focused Analysis memory diagnostic",
                "summary": "Exercise the current baseline Analysis consumer",
                "choices": choices,
                "interaction_review": v11.outside_interactions(choices, [source_id]),
                "material_boundary_review": v11.material_boundary_review(
                    choices, [source_id]
                ),
            },
        )
        if not discovery_ok or not discovery:
            raise RuntimeError("focused Engineering Choice Discovery failed")
        review, review_ok = host.tool(
            "materiality_review",
            {
                "action": "record",
                "project_id": project_id,
                "engineering_choice_discovery_candidate_id": discovery.get(
                    "discovery_candidate_id"
                ),
                "rationale": "Both representations preserve the current product contract.",
                "behavioral_context_basis": {
                    "context_item_ids": [],
                    "completeness_rationale": (
                        "No behaviorally consequential non-Goal Context is used."
                    ),
                },
                "learning_participation": {"state": "inactive"},
                "judgments": [{
                    "choice_id": choice_id,
                    "disposition": "agent_owned_implementation_choice",
                    "materially_varying_outcomes": [
                        "private Analysis state organization"
                    ],
                    "contains_user_owned_outcome": False,
                    "user_owned_outcomes": [],
                    "ownership_rationale": (
                        "Both alternatives preserve observable Repository Intelligence behavior."
                    ),
                    "discretion_counterfactuals": [{
                        "choice_id": choice_id,
                        "alternative_id": alternative,
                        "externally_observable": False,
                        "observation_rationale": (
                            "Only private organization changes under the same output contract."
                        ),
                        "source_id": source_id,
                        "source_supported_boundary": (
                            "The current repository Source fixes the observable analysis basis."
                        ),
                    } for alternative in ("ordered", "indexed")],
                    "bounded_implementation_discretion_rationale": (
                        "The alternatives stay inside the settled analysis boundary."
                    ),
                    "ownership_source_ids": [source_id],
                    "alternative_accounting": v11.alternative_accounting(
                        choice_id, ["ordered", "indexed"], source_id
                    ),
                    "basis_summary": "The representation is bounded implementation detail.",
                    "authority_counterfactual": (
                        "The alternatives do not change a user-owned product outcome."
                    ),
                    "learning_authority": {"state": "inactive"},
                    "learning_value": {
                        "state": "routine",
                        "rationale": "The diagnostic does not create a learning interaction.",
                    },
                }],
            },
        )
        if not review_ok or not review:
            raise RuntimeError("focused Materiality Review failed")
        host.tool("project_health", {"project_id": project_id})
    finally:
        cleanup = host.close()
    if cleanup.get("exit_code") != 0:
        raise RuntimeError("focused MCP process did not exit cleanly")

    duration_ms = round((time.monotonic_ns() - started) / 1_000_000, 3)
    manifest = latest_manifest(runtime, project_id)
    logical, physical = storage_totals(runtime, project_id)
    diagnostics = collector.diagnostics()
    (output / "memory-calls.json").write_text(
        json.dumps(diagnostics, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    materiality_calls = [
        call for call in diagnostics["calls"]
        if call["operation"] == "materiality_review"
    ]
    if len(materiality_calls) != 1:
        raise RuntimeError("focused workload did not observe exactly one Materiality Review")
    summary = {
        "schema_version": 1,
        "status": "passed",
        "repository_revision": analyzed_revision,
        "duration_ms": duration_ms,
        "process_count": len(diagnostics["processes"]),
        "call_count": len(diagnostics["calls"]),
        "peak_rss_bytes": max(
            call["process_peak_rss_bytes"] or 0 for call in diagnostics["calls"]
        ),
        "first_band_crossings": diagnostics["first_band_crossings"],
        "materiality_review": materiality_calls[0],
        "analysis_contract": analysis_contract(manifest, analysis),
        "analysis_storage_logical_bytes": logical,
        "analysis_storage_physical_bytes": physical,
        "analysis_snapshot_count": len(
            list((runtime / "derived/analysis" / project_id).glob("*.json"))
        ),
        "evidence_policy": {
            "rpc_arguments_retained": False,
            "rpc_responses_retained": False,
            "source_bodies_retained": False,
            "process_arguments_retained": False,
        },
    }
    (output / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir")
    parser.add_argument("--repository-revision")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    LOCAL_ROOT.mkdir(parents=True, exist_ok=True)
    output = (
        Path(args.output_dir).resolve()
        if args.output_dir
        else Path(tempfile.mkdtemp(prefix="run-", dir=LOCAL_ROOT))
    )
    if not args.output_dir:
        output.rmdir()
    summary = workload(output, args.repository_revision)
    print(json.dumps({
        "status": summary["status"],
        "output": str(output),
        "peak_rss_bytes": summary["peak_rss_bytes"],
        "repository_revision": summary["repository_revision"],
        "materiality_review": summary["materiality_review"],
        "analysis_contract": summary["analysis_contract"],
        "analysis_storage_logical_bytes": summary["analysis_storage_logical_bytes"],
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
