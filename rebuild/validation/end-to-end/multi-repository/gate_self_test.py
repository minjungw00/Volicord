#!/usr/bin/env python3
"""Focused regression checks for validation admission and gate orchestration."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import copy
import shutil
import sys
import tempfile
from typing import Any, Sequence


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
GATE_PATH = HERE / "gate.py"
HARNESS_PATH = HERE / "harness.py"
sys.dont_write_bytecode = True


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"could not load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


gate = load_module("volicord_gate_self_test_target", GATE_PATH)
harness = load_module("volicord_harness_self_test_target", HARNESS_PATH)
HEAD = gate.git_output("rev-parse", "HEAD").stdout.strip()
FINAL_COMMANDS = (("synthetic-final-command", "--exact"),)
SECRET_SENTINELS = (
    "auth.json contents must not be projected",
    "credential-value-must-not-be-projected",
    "reusable-credential-fingerprint-must-not-be-projected",
    "raw-repository-source-body-must-not-be-projected",
    "full-command-log-must-not-be-projected",
    "raw-provider-payload-must-not-be-projected",
)


def passed(name: str, **details: Any) -> dict[str, Any]:
    return gate.check(name, "passed", f"{name} passed", **details)


def admission_overrides() -> dict[str, dict[str, Any]]:
    return {
        "candidate_identity_and_clean_worktree": passed(
            "candidate_identity_and_clean_worktree", candidate_head=HEAD, dirty_entry_count=0
        ),
        "validation_runner_self_check": passed("validation_runner_self_check"),
        "v11_harness_self_check": passed("v11_harness_self_check"),
        **{
            name: passed(name)
            for name in (
                "architecture_contracts",
                "architecture_contracts_self_test",
                "repository_intelligence_realistic_qualification",
                "dogfood_harness_self_test",
                "dogfood_campaign_self_test",
                "provider_qualification_self_test",
            )
        },
        "required_fixture_identities": passed(
            "required_fixture_identities",
            fixtures=[
                {"id": "v01-python", "validation_id": "V01", "content_sha256": "a" * 64},
                {"id": "v11-polyglot-medium", "validation_id": "V11", "content_sha256": "b" * 64},
                {"id": "repository-intelligence-realistic-v1", "validation_id": "RI-REALISTIC", "content_sha256": "f" * 64},
                {"id": "v12-current-codex-mcp-completion", "validation_id": "V12", "content_sha256": "1" * 64},
                {"id": "v07-background-provider-bounded-rust", "validation_id": "V07", "content_sha256": "2" * 64},
            ],
        ),
        "fixture_manifest_integrity": passed("fixture_manifest_integrity"),
        "required_local_executables": passed("required_local_executables"),
        "filesystem_and_runtime_home": passed("filesystem_and_runtime_home"),
        "bounded_local_resource_estimate": passed("bounded_local_resource_estimate"),
        "local_loopback": passed("local_loopback", requires_escalation=False),
        "codex_authentication_material": passed(
            "codex_authentication_material",
            available=True,
            auth_json_contents=SECRET_SENTINELS[0],
            credential_value=SECRET_SENTINELS[1],
            credential_fingerprint=SECRET_SENTINELS[2],
        ),
    }


def unused_command_runner(_directory: Path, _argv: Sequence[str]) -> dict[str, Any]:
    raise AssertionError("overridden admission unexpectedly invoked a command")


def synthetic_environment_evidence() -> dict[str, Any]:
    return {
        "platform": {
            "operating_system": "Linux",
            "release": "6.12.0-fixture",
            "platform_version": "#1 fixture kernel",
            "machine": "x86_64",
            "architecture": "64bit",
        },
        "python_runtime": {
            "implementation": "CPython",
            "version": "3.13.0",
            "executable_basename": "python3",
        },
        "tools": {
            name: {
                "argv": [name if name != "python" else "python3", "--version"],
                "status": "available",
                "version": f"{name} synthetic-version",
                "exit_code": 0,
            }
            for name in ("python", "git", "cargo", "rustc", "codex")
        },
    }


def synthetic_dependency_evidence() -> dict[str, Any]:
    return {
        "candidate_head": HEAD,
        "cargo_lock": {
            "path": "rebuild/Cargo.lock", "status": "available", "sha256": "c" * 64,
        },
        "workspace_manifest": {
            "path": "rebuild/Cargo.toml", "status": "available", "sha256": "d" * 64,
        },
        "fixture_manifest": {
            "path": "rebuild/validation/shared/fixture-manifest.json",
            "status": "available", "sha256": "e" * 64,
        },
    }


def admission(
    root: Path,
    *,
    authorization: str | None = None,
    provider_authorization: str | None = None,
    provider_model: str | None = None,
    loopback_failed: bool = False,
) -> dict[str, Any]:
    overrides = admission_overrides()
    if loopback_failed:
        overrides["local_loopback"] = gate.check(
            "local_loopback",
            "environment_blocked",
            "synthetic loopback restriction requires escalation",
            requires_escalation=True,
        )
    return gate.evaluate_admission(
        authorization_assertion=authorization,
        provider_authorization_assertion=provider_authorization,
        provider_model=provider_model,
        external_network="available",
        artifact_root=root,
        command_runner=unused_command_runner,
        runner_path=ROOT / "rebuild/scripts/validate",
        overrides=overrides,
        environment_evidence=synthetic_environment_evidence(),
        dependency_evidence=synthetic_dependency_evidence(),
    )


class Owners:
    def __init__(
        self,
        root: Path,
        *,
        final_passes: bool = True,
        preflight_passes: bool = True,
        v11_passes: bool = True,
        provider_passes: bool = True,
        revisit_assessment: dict[str, Any] | None = None,
    ):
        self.root = root
        self.final_passes = final_passes
        self.preflight_passes = preflight_passes
        self.v11_passes = v11_passes
        self.provider_passes = provider_passes
        self.revisit_assessment = revisit_assessment
        self.counts = {"final": 0, "provider": 0, "preflight": 0, "v11": 0, "audit": 0}
        self.final_path: Path | None = None
        self.preflight_path: Path | None = None
        self.v11_result: dict[str, Any] | None = None

    def final(self) -> tuple[dict[str, Any], Path]:
        self.counts["final"] += 1
        directory = self.root / f"new-final-{self.counts['final']}"
        directory.mkdir(parents=True)
        failed = not self.final_passes
        summary = {
            "outcome": "failed" if failed else "succeeded",
            "command_count": len(FINAL_COMMANDS),
            "failure_count": 1 if failed else 0,
            "commands": [
                {
                    "argv": list(command),
                    "outcome": "failed" if failed else "succeeded",
                    "exit_code": 9 if failed else 0,
                    "termination": None,
                    "duration_ms": 1.0,
                    "stdout": SECRET_SENTINELS[4],
                }
                for command in FINAL_COMMANDS
            ],
        }
        self.final_path = directory / "summary.json"
        self.final_path.write_text(json.dumps(summary), encoding="utf-8")
        return summary, self.final_path

    def preflight(self, candidate_head: str, final_path: Path) -> tuple[dict[str, Any], dict[str, Any]]:
        self.counts["preflight"] += 1
        if candidate_head != HEAD:
            raise AssertionError("preflight received the wrong candidate HEAD")
        self.preflight_path = final_path
        return (
            {"status": "passed" if self.preflight_passes else "failed"},
            {"exit_code": 0 if self.preflight_passes else 1},
        )

    def provider(self, candidate_head: str) -> tuple[dict[str, Any] | None, dict[str, Any], Path]:
        self.counts["provider"] += 1
        path = self.root / "provider-evidence.json"
        evidence = {
            "schema_version": 1,
            "qualification_id": "background-provider-openai-codex-v1",
            "production_head": candidate_head,
            "authorization": {
                "assertion_id": gate.PROVIDER_AUTHORIZATION_ASSERTION,
                "distinct_from_v11": True,
                "supplied_by": "current_invocation",
            },
            "provider": {
                "identity": "openai-codex",
                "model": "synthetic-model",
                "transport": "authenticated installed Codex CLI",
                "provider_side_deletion": "unsupported_by_adapter",
            },
            "fixture": {
                "id": "background-provider-bounded-rust-v1",
                "source_count": 1,
                "source_locator": "src/lib.rs",
                "original_bytes": 632,
                "transmitted_bytes": 632,
                "content_sha256": "3" * 64,
                "maintained_bytes": 632,
            },
            "success": {
                "guarded_outcome": "dispatched_and_completed",
                "provider_request_outcome": "completed",
                "transmission_outcome": "transmitted",
                "repository_snapshot": "4" * 64,
                "analysis_snapshot": "5" * 64,
                "semantic_annotation_count": 3,
                "annotation_provenance_complete": True,
            },
            "degradation": {
                "trigger": "configured executable unavailable",
                "guarded_confirmation_consumed": True,
                "provider_request_outcome": "provider_unavailable",
                "transmission_outcome": "not_transmitted",
                "local_canonical_continuity": True,
            },
            "retained_evidence": {
                "source_body": False,
                "provider_response_body": False,
                "credential": False,
            },
        }
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(evidence), encoding="utf-8")
        return (evidence if self.provider_passes else None), {
            "exit_code": 0 if self.provider_passes else 1
        }, path

    def v11(
        self,
        candidate_head: str,
        final_path: Path,
        output_directory: Path,
    ) -> tuple[dict[str, Any], dict[str, Any]]:
        self.counts["v11"] += 1
        if candidate_head != HEAD or final_path != self.final_path:
            raise AssertionError("V11 did not receive the same-session final artifact")
        output_directory.mkdir()
        status = "passed" if self.v11_passes else "failed"
        repositories = []
        for target in ("volicord", "small-python", "polyglot-medium"):
            steps = {
                name: harness.step(status, "synthetic gate fixture")
                for name in harness.REQUIRED_STEPS
            }
            steps["codex_mcp_connection"]["evidence"] = {
                "authenticated": {
                    "status": status,
                    "summary": "synthetic-secret-must-not-be-copied",
                }
            }
            repositories.append({"class": target, "steps": steps})
        result = harness.make_v11_result(
            validated_production_head=candidate_head,
            final_gate_artifact=str(final_path),
            duration_ms=1.0,
            repositories=repositories,
            revisit_assessment=self.revisit_assessment or harness.read_decision_revisit_assessment(),
        )
        result["raw_repository_source_body"] = SECRET_SENTINELS[3]
        result["raw_provider_payload"] = SECRET_SENTINELS[5]
        self.v11_result = result
        (output_directory / "result.json").write_text(json.dumps(result), encoding="utf-8")
        return result, {"exit_code": 0 if result["status"] == "passed" else 1}

    def audit(self, _output_directory: Path) -> tuple[dict[str, Any], dict[str, Any]]:
        self.counts["audit"] += 1
        return {
            "kind": "v11_credential_retention_audit",
            "status": "passed",
            "auth_named_file_count": 0,
            "credential_content_match_count": 0,
            "scan_error_count": 0,
            "credential_fingerprint": SECRET_SENTINELS[2],
        }, {"exit_code": 0}


def run_orchestration(root: Path, admitted: dict[str, Any], owners: Owners) -> tuple[dict[str, Any], dict[str, int]]:
    root.mkdir(parents=True, exist_ok=True)
    return gate.orchestrate(
        gate_directory=root,
        admission=admitted,
        final_commands=FINAL_COMMANDS,
        final_owner=owners.final,
        provider_owner=owners.provider,
        preflight_owner=owners.preflight,
        v11_owner=owners.v11,
        audit_owner=owners.audit,
        pre_final_check_owner=lambda candidate_head: gate.check(
            "pre_final_candidate_identity_and_clean_worktree",
            "passed",
            "synthetic candidate remained clean",
            expected_candidate_head=candidate_head,
            observed_candidate_head=candidate_head,
            head_unchanged=True,
            dirty_entry_count=0,
            dirty_entries=[],
        ),
    )


def gate_consumed_result_contract(result: dict[str, Any]) -> dict[str, Any]:
    return {
        "result_fields": sorted(
            key for key in result
            if key in {
                "status", "phase_8_ready", "counts", "repositories",
                "active_decision_revisit_triggers",
                "decision_revisit_trigger_assessment",
                "decision_revisit_trigger_source",
            }
        ),
        "assessment_source_fields": sorted(result["decision_revisit_trigger_source"]),
        "targets": [repository["class"] for repository in result["repositories"]],
        "authenticated_evidence_fields": [
            sorted(
                repository["steps"]["codex_mcp_connection"]["evidence"]["authenticated"]
            )
            for repository in result["repositories"]
        ],
    }


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="volicord-gate-self-test-") as directory:
        root = Path(directory)

        loopback_blocked = admission(
            root / "loopback",
            authorization=gate.AUTHORIZATION_ASSERTION,
            provider_authorization=gate.PROVIDER_AUTHORIZATION_ASSERTION,
            provider_model="synthetic-model",
            loopback_failed=True,
        )
        owners = Owners(root / "loopback-owners")
        capsule, counts = run_orchestration(root / "loopback-gate", loopback_blocked, owners)
        assert capsule["blocking_classification"] == "environment_blocked"
        assert counts["final"] == 0 and counts["official_v11"] == 0
        assert owners.counts == {"final": 0, "provider": 0, "preflight": 0, "v11": 0, "audit": 0}

        authorization_blocked = admission(root / "authorization")
        owners = Owners(root / "authorization-owners")
        capsule, counts = run_orchestration(root / "authorization-gate", authorization_blocked, owners)
        assert authorization_blocked["blocking_classification"] == "authorization_blocked"
        assert capsule["blocking_classification"] == "authorization_blocked"
        assert counts["final"] == 0 and counts["official_v11"] == 0
        assert capsule["final_aggregate"] == {
            "status": "not_run", "command_count": 0, "failure_count": 0, "commands": [],
        }
        assert set(capsule["gate_configuration"]["same_session_artifact_flow"].values()) == {False}
        assert any(
            value["status"] == "authorization_blocked"
            for value in capsule["admission_checks"]
        )

        v11_only = admission(
            root / "v11-only",
            authorization=gate.AUTHORIZATION_ASSERTION,
            provider_model="synthetic-model",
        )
        assert v11_only["blocking_classification"] == "authorization_blocked"
        assert any(
            check["name"] == "operator_provider_source_transmission_authorization"
            and check["status"] == "authorization_blocked"
            for check in v11_only["checks"]
        )

        admitted = admission(
            root / "admitted",
            authorization=gate.AUTHORIZATION_ASSERTION,
            provider_authorization=gate.PROVIDER_AUTHORIZATION_ASSERTION,
            provider_model="synthetic-model",
        )
        assert admitted["eligible"] is True
        owners = Owners(root / "success-owners")
        old = root / "older-unrelated-final.json"
        old.write_text('{"outcome":"succeeded","failure_count":0}\n', encoding="utf-8")
        capsule, counts = run_orchestration(root / "success-gate", admitted, owners)
        assert counts == {"final": 1, "provider_live_qualification": 1, "preflight": 1, "official_v11": 1, "credential_audit": 1}
        assert owners.counts == {"final": 1, "provider": 1, "preflight": 1, "v11": 1, "audit": 1}
        assert owners.preflight_path == owners.final_path and owners.preflight_path != old
        assert capsule["phase_8_ready"] is True
        pending_capsule = gate.stage_evidence_archive(capsule)
        assert pending_capsule["phase_8_ready"] is False
        assert pending_capsule["blocking_classification"] == "evidence_archive_pending"
        assert pending_capsule["evidence_archive"]["status"] == "pending"
        archive_failed = gate.fail_evidence_archive(pending_capsule, "verification")
        assert archive_failed["phase_8_ready"] is False
        assert archive_failed["blocking_classification"] == (
            "evidence_archive_verification_failed"
        )
        assert archive_failed["evidence_archive"]["status"] == "verification_failed"
        archive_identity = {
            "candidate_head": HEAD,
            "path": str(root / "synthetic-evidence.tar.gz"),
            "sha256": "9" * 64,
            "size_bytes": 512,
            "member_count": 8,
        }
        archive_verified = gate.complete_evidence_archive(
            pending_capsule,
            archive_identity,
            {
                "status": "passed",
                "candidate_head": HEAD,
                "archive_sha256": "9" * 64,
            },
        )
        assert archive_verified["phase_8_ready"] is True
        assert archive_verified["blocking_classification"] is None
        assert archive_verified["evidence_archive"]["status"] == "verified"
        assert capsule["decision_revisit_trigger_assessment"] == "reported_by_official_v11"
        assert capsule["active_decision_revisit_triggers"] == []
        assert capsule["decision_revisit_trigger_source"]["path"] == (
            "rebuild/docs/design/open-decisions.md"
        )
        assert owners.v11_result is not None
        real_builder_result = harness.make_v11_result(
            validated_production_head=HEAD,
            final_gate_artifact=str(owners.final_path),
            duration_ms=2.0,
            repositories=copy.deepcopy(owners.v11_result["repositories"]),
            revisit_assessment=harness.read_decision_revisit_assessment(),
        )
        assert gate_consumed_result_contract(owners.v11_result) == (
            gate_consumed_result_contract(real_builder_result)
        )
        assert gate_consumed_result_contract(owners.v11_result)["result_fields"] == sorted({
            "status", "phase_8_ready", "counts", "repositories",
            "active_decision_revisit_triggers",
            "decision_revisit_trigger_assessment",
            "decision_revisit_trigger_source",
        })
        encoded = json.dumps(capsule, sort_keys=True)
        assert "synthetic-secret-must-not-be-copied" not in encoded
        assert capsule["credential_retention_audit"]["status"] == "passed"
        required_capsule_keys = {
            "validated_candidate_head", "final_aggregate", "final_summary_sha256",
            "official_v11", "required_identities", "credential_retention_audit",
            "authenticated_codex_outcomes", "active_decision_revisit_triggers",
            "decision_revisit_trigger_assessment", "decision_revisit_trigger_source",
            "admission_checks", "pre_final_candidate_check", "execution_environment", "dependency_snapshot",
            "gate_configuration", "phase_8_ready",
            "live_provider_qualification",
            "evidence_archive",
        }
        assert required_capsule_keys <= capsule.keys()
        assert capsule["pre_final_candidate_check"]["status"] == "passed"
        assert capsule["execution_environment"]["platform"]["operating_system"] == "Linux"
        assert set(capsule["execution_environment"]["tools"]) == {
            "python", "git", "cargo", "rustc", "codex",
        }
        assert all(
            value["status"] == "available" and value["version"]
            for value in capsule["execution_environment"]["tools"].values()
        )
        assert capsule["dependency_snapshot"] == synthetic_dependency_evidence()
        assert capsule["final_aggregate"]["commands"][0]["argv"] == list(FINAL_COMMANDS[0])
        configuration = capsule["gate_configuration"]
        assert configuration["argv"] == [
            "rebuild/scripts/validate", "gate", "--external-network", "available",
            "--authorize-external-transmission", gate.AUTHORIZATION_ASSERTION,
            "--authorize-provider-source-transmission", gate.PROVIDER_AUTHORIZATION_ASSERTION,
            "--provider-model", "synthetic-model",
        ]
        assert configuration["technical_external_network_assertion"] == "available"
        assert configuration["authorization_assertion_id"] == gate.AUTHORIZATION_ASSERTION
        assert configuration["provider_authorization_assertion_id"] == gate.PROVIDER_AUTHORIZATION_ASSERTION
        assert configuration["external_transmission"] == gate.EXTERNAL_TRANSMISSION
        assert configuration["provider_external_transmission"] == gate.PROVIDER_EXTERNAL_TRANSMISSION
        assert set(configuration["same_session_artifact_flow"].values()) == {True}
        for sentinel in SECRET_SENTINELS:
            assert sentinel not in encoded
        assert len(encoded.encode("utf-8")) < 32_000
        shutil.rmtree(owners.final_path.parent)
        shutil.rmtree(root / "success-gate" / "official-v11")
        assert required_capsule_keys <= capsule.keys() and capsule["phase_8_ready"] is True

        owners = Owners(root / "final-failure-owners", final_passes=False)
        capsule, counts = run_orchestration(root / "final-failure-gate", admitted, owners)
        assert capsule["blocking_classification"] == "final_failed"
        assert counts["final"] == 1 and counts["official_v11"] == 0
        assert owners.counts["final"] == 1 and owners.counts["v11"] == 0
        assert capsule["final_aggregate"]["status"] == "failed"
        assert capsule["final_summary_sha256"]
        assert capsule["official_v11"]["status"] == "not_run"
        assert capsule["authenticated_codex_outcomes"] == []
        assert capsule["phase_8_ready"] is False

        owners = Owners(root / "provider-failure-owners", provider_passes=False)
        capsule, counts = run_orchestration(root / "provider-failure-gate", admitted, owners)
        assert capsule["blocking_classification"] == "provider_live_qualification_failed"
        assert counts == {
            "final": 1,
            "provider_live_qualification": 1,
            "preflight": 0,
            "official_v11": 0,
            "credential_audit": 0,
        }
        assert owners.counts == {
            "final": 1, "provider": 1, "preflight": 0, "v11": 0, "audit": 0,
        }
        assert capsule["live_provider_qualification"]["status"] == "failed"
        assert capsule["official_v11"]["status"] == "not_run"
        assert capsule["phase_8_ready"] is False

        active_assessment = copy.deepcopy(harness.read_decision_revisit_assessment())
        active_assessment["active_decision_revisit_triggers"] = ["Q3"]
        owners = Owners(root / "active-trigger-owners", revisit_assessment=active_assessment)
        capsule, counts = run_orchestration(root / "active-trigger-gate", admitted, owners)
        assert counts["official_v11"] == 1
        assert capsule["blocking_classification"] == "v11_failed"
        assert capsule["active_decision_revisit_triggers"] == ["Q3"]
        assert capsule["decision_revisit_trigger_assessment"] == "reported_by_official_v11"
        assert capsule["phase_8_ready"] is False

        owners = Owners(
            root / "unassessable-trigger-owners",
            revisit_assessment=harness.failed_decision_revisit_assessment(),
        )
        capsule, counts = run_orchestration(
            root / "unassessable-trigger-gate", admitted, owners
        )
        assert counts["official_v11"] == 1
        assert capsule["blocking_classification"] == "v11_failed"
        assert capsule["active_decision_revisit_triggers"] is None
        assert capsule["decision_revisit_trigger_assessment"] == (
            "official_v11_assessment_failed"
        )
        assert capsule["phase_8_ready"] is False

        owners = Owners(root / "preflight-failure-owners", preflight_passes=False)
        capsule, counts = run_orchestration(root / "preflight-failure-gate", admitted, owners)
        assert capsule["blocking_classification"] == "v11_preflight_failed"
        assert counts["final"] == 1 and counts["preflight"] == 1
        assert counts["official_v11"] == 0
        assert capsule["final_aggregate"]["status"] == "succeeded"
        flow = capsule["gate_configuration"]["same_session_artifact_flow"]
        assert flow["final_artifact_produced_by_gate"] is True
        assert flow["v11_preflight_consumed_same_gate_final_artifact"] is True
        assert flow["official_v11_consumed_same_gate_final_artifact"] is False
        assert capsule["official_v11"]["status"] == "not_run"
        assert capsule["phase_8_ready"] is False

        owners = Owners(root / "v11-failure-owners", v11_passes=False)
        capsule, counts = run_orchestration(root / "v11-failure-gate", admitted, owners)
        assert capsule["blocking_classification"] == "v11_failed"
        assert counts["final"] == 1 and counts["official_v11"] == 1
        assert owners.counts["final"] == 1 and owners.counts["v11"] == 1
        assert capsule["official_v11"]["status"] == "failed"
        assert capsule["official_v11"]["result_sha256"]
        assert len(capsule["authenticated_codex_outcomes"]) == 3
        assert capsule["phase_8_ready"] is False

        harness.assert_required_step_policy_regressions()
        harness.assert_credential_retention_audit()
        unavailable = gate.bounded_version_probe(("volicord-version-probe-does-not-exist", "--version"))
        assert unavailable["status"] == "unavailable" and unavailable["version"] is None

    print(json.dumps({
        "status": "passed",
        "scenarios": 16,
        "real_synthetic_result_contract_parity": "passed",
        "real_final_invocations": 0,
        "official_v11_invocations": 0,
    }, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
