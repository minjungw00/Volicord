# V11 — End-to-end multi-repository journey

## Status

Passed. The official V11 gate completed all 54 required steps for exact-final
production/test candidate HEAD `972a0af7436091a822f689668a64e0f03195bb59`.
All 54 steps passed, no blocking classification was reported, and
`phase_8_ready = true`. Phase 8 may begin; this result does not claim that
Phase 8 itself has passed.

## Goal

Rehearse one installed Volicord journey against the Volicord reconstruction
repository, a small single-language application, and a medium documented
polyglot repository. Use current CLI and MCP boundaries for repository-bound
Project resolution, Candidate, Inquiry, Guarded provider, canonical, portable,
document, and recovery semantics without validation-only substitutes, while
retaining no reusable Codex authentication material in V11 evidence.

## Accepted decisions being validated

- Q1 staged Inquiry and terminal material Question handling.
- Q2 polyglot capability and honest per-language degradation.
- Q3 local-first operation, Project opt-in, and background-provider isolation.
- Q4 installed CLI, MCP, and user-inspectable logical surfaces.
- Q5 four source-grounded document types in Markdown and self-contained HTML.
- Q6 portable Project identity, explicit clone binding, divergence, and conflict.
- Q7 correction, semantic supersession, and privacy-prioritized deletion.
- Q8-A Linux/Codex installation and connection; Q8-B fresh-service legacy exclusion.
- Q9 bounded read-only Recall; Q10 Candidate collection and promotion boundaries.
- Q11 source-grounded Checkpoint; Q12 exact Guarded confirmation and effect behavior.
- Q13 Decision applicability, reuse, and evidence-driven re-questioning.

## Input repositories and revisions

| Class | Validated identity | Capsule-recorded outcome |
| --- | --- | --- |
| Volicord reconstruction repository | `validated_candidate_head = 972a0af7436091a822f689668a64e0f03195bb59` | authenticated Codex target `volicord`: status `passed`, classification `passed` |
| Small Python application | fixture `v01-python` (V01), SHA-256 `7feb9a79db3c37b10399171c615294286531cb12e0265263df2e6ec5d50c5867` | authenticated Codex target `small-python`: status `passed`, classification `passed` |
| Medium polyglot repository | fixture `v11-polyglot-medium` (V11), SHA-256 `7cb34ff3435dfd91a55f261e27ca407bfef7f3654aa8d8dac5c90eaa245edafb` | authenticated Codex target `polyglot-medium`: status `passed`, classification `passed` |

The capsule's dependency and required-fixture identities are:

| Capsule field | Path or identity | Status | SHA-256 |
| --- | --- | --- | --- |
| `cargo_lock` | `rebuild/Cargo.lock` | `cargo_lock.status = available` | `cargo_lock.sha256 = 4c162c8e223870db156e252407fa03c41d817c79595cc1755b9a37860ca45a35` |
| `workspace_manifest` | `rebuild/Cargo.toml` | `workspace_manifest.status = available` | `workspace_manifest.sha256 = 4b3a0552a71547385b4246ea9b3ec5103581a436ef2b14650014b78efb28a670` |
| `fixture_manifest` | `rebuild/validation/shared/fixture-manifest.json` | `fixture_manifest.status = available` | `fixture_manifest.sha256 = c8f535601e49ba0ee4999b6f29ed3e5ff45a4de53a842deabf03d3fab82140d9` |

- `fixture.id = v01-python`; `fixture.content_sha256 = 7feb9a79db3c37b10399171c615294286531cb12e0265263df2e6ec5d50c5867`.
- `fixture.id = v11-polyglot-medium`; `fixture.content_sha256 = 7cb34ff3435dfd91a55f261e27ca407bfef7f3654aa8d8dac5c90eaa245edafb`.

The exact-final validated candidate, capsule candidate, dependency snapshot,
required identity, pre-final observed candidate, and documentation-session
starting HEAD all identify the same commit.

## Environment and tool versions

Admission status was `eligible`. Immediately before exact final, the gate
observed a clean worktree with zero dirty entries and confirmed that HEAD was
unchanged at `972a0af7436091a822f689668a64e0f03195bb59`.

| Capsule field | Value |
| --- | --- |
| `platform.operating_system` | `Linux` |
| `platform.release` | `6.18.33.2-microsoft-standard-WSL2` |
| `platform.platform_version` | `#1 SMP PREEMPT_DYNAMIC Thu Jun 18 21:54:43 UTC 2026` |
| `platform.machine` | `x86_64` |
| `platform.architecture` | `64bit` |
| `python_runtime.implementation` | `CPython` |
| `python_runtime.version` | `3.12.3` |
| `python_runtime.executable_basename` | `python3` |
| `tools.python.status`; `tools.python.version` | `available`; `Python 3.12.3` |
| `tools.git.status`; `tools.git.version` | `available`; `git version 2.43.0` |
| `tools.cargo.status`; `tools.cargo.version` | `available`; `cargo 1.97.1 (c980f4866 2026-06-30)` |
| `tools.rustc.status`; `tools.rustc.version` | `available`; `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| `tools.codex.status`; `tools.codex.version` | `available`; `codex-cli 0.145.0` |

No authorization or environment blocker was reported:
`blocking_classification = null`.

## Candidate approaches

The maintained gate used the one-way lifecycle from admission through one
exact final and the same-session official V11. The later documentation-only
conclusion interprets the copied sanitized capsule and does not alter or extend
the sealed production/test candidate.

## Commands and configuration

The exact-final aggregate reported `status = succeeded`, four commands, and
zero failures. Its sealed command evidence is:

| Command evidence | Value |
| --- | --- |
| `final_command.cargo_metadata` | `cargo metadata --manifest-path rebuild/Cargo.toml --no-deps --format-version 1` |
| `final_command.cargo_metadata.outcome`; `final_command.cargo_metadata.exit_code`; `final_command.cargo_metadata.termination`; `final_command.cargo_metadata.spawn_error`; `final_command.cargo_metadata.duration_ms` | `succeeded`; `0`; `null`; `false`; `11.828` |
| `final_command.cargo_fmt` | `cargo fmt --manifest-path rebuild/Cargo.toml --all -- --check` |
| `final_command.cargo_fmt.outcome`; `final_command.cargo_fmt.exit_code`; `final_command.cargo_fmt.termination`; `final_command.cargo_fmt.spawn_error`; `final_command.cargo_fmt.duration_ms` | `succeeded`; `0`; `null`; `false`; `430.113` |
| `final_command.cargo_clippy` | `cargo clippy --manifest-path rebuild/Cargo.toml --workspace --all-targets --all-features` |
| `final_command.cargo_clippy.outcome`; `final_command.cargo_clippy.exit_code`; `final_command.cargo_clippy.termination`; `final_command.cargo_clippy.spawn_error`; `final_command.cargo_clippy.duration_ms` | `succeeded`; `0`; `null`; `false`; `7714.197` |
| `final_command.cargo_test` | `cargo test --manifest-path rebuild/Cargo.toml --workspace --all-targets --all-features` |
| `final_command.cargo_test.outcome`; `final_command.cargo_test.exit_code`; `final_command.cargo_test.termination`; `final_command.cargo_test.spawn_error`; `final_command.cargo_test.duration_ms` | `succeeded`; `0`; `null`; `false`; `14830.004` |

- `gate_configuration.argv = rebuild/scripts/validate gate --external-network available --authorize-external-transmission v11-openai-codex-project-health-three-targets`
- `gate_configuration.argv_status = complete`
- `technical_external_network_assertion = available`
- `authorization_assertion_id = v11-openai-codex-project-health-three-targets`
- `final_artifact_produced_by_gate = true`
- `v11_preflight_consumed_same_gate_final_artifact = true`
- `official_v11_consumed_same_gate_final_artifact = true`
- `final_aggregate.status = succeeded`; `final_aggregate.command_count = 4`;
  `final_aggregate.failure_count = 0`.

The exact-final summary SHA-256 is
`final_summary_sha256 = 077bb1d95971a00f693fd26ea5ac723d77fa36344aa8bc5c36ef3150c7f68cfd`.
Final and official V11 were run by the immediately preceding gate session and
are not rerun for this documentation conclusion.

## Observed results

Official V11 reported `official_v11.status = passed`,
`official_v11.required_step_count = 54`, and
`official_v11.phase_8_ready = true`. Its result SHA-256 is
`official_v11.result_sha256 = 3f9aa54167c6c129316730de43fa446e38a58ebff0c61246f2704ce363cd3c96`.
The final-validated and V11-validated candidate HEAD is
`972a0af7436091a822f689668a64e0f03195bb59`.

## Coverage and failures

| Official V11 status field | Count |
| --- | ---: |
| `official_v11.status_counts.passed` | 54 |
| `official_v11.status_counts.failed` | 0 |
| `official_v11.status_counts.partial` | 0 |
| `official_v11.status_counts.unsupported` | 0 |
| `official_v11.status_counts.skipped` | 0 |
| `official_v11.status_counts.environment_blocked` | 0 |
| **Required total** | **54** |

All three required authenticated Codex targets passed with `passed`
classification. The official result contains no failed, partial, unsupported,
skipped, or environment-blocked step.

## Performance and resource observations

The capsule records exact-final command durations of 11.828 ms, 430.113 ms,
7714.197 ms, and 14830.004 ms. It does not project official V11 per-step
timings, output sizes, or peak-resource measurements, so this conclusion makes
no broader performance claim.

## Privacy and external transmission

The bounded transmission configuration was
`external_transmission.required = true`, with
`external_transmission.destination = OpenAI Codex service used by the installed Codex CLI`,
`external_transmission.purpose = three authenticated turns that select the installed Volicord project_health MCP tool`,
and
`external_transmission.source_scope = bounded V11 prompt, Project identity, and project_health tool result; no intended repository source body`.
The target records are `external_transmission.scope = volicord`,
`external_transmission.scope = small-python`, and
`external_transmission.scope = polyglot-medium`; the bounded assertion is
`external_transmission.authorization_assertion = v11-openai-codex-project-health-three-targets`.

The credential-retention audit reported
`credential_retention_audit.status = passed`,
`credential_retention_audit.auth_named_file_count = 0`,
`credential_retention_audit.credential_content_match_count = 0`, and
`credential_retention_audit.scan_error_count = 0`. No credential content or
reusable secret fingerprint is recorded in this report.

## Acceptance results

| Acceptance area | Current conclusion |
| --- | --- |
| Admission | `admission_status = eligible` |
| Pre-final candidate identity and clean worktree | `passed`: unchanged candidate HEAD and zero dirty entries |
| Exact final | `succeeded`: four commands succeeded, zero failures |
| Three-repository official V11 | `passed`: 54 of 54 required steps passed |
| Authenticated Codex targets | `passed`: `volicord`, `small-python`, and `polyglot-medium` |
| Credential-retention audit | `passed`: all three recorded counts are zero |
| Blocking classification | `blocking_classification = null` |
| Phase 8 entry | `phase_8_ready = true` |

All admission and target outcomes from the capsule are retained below:

- `admission_check.candidate_identity_and_clean_worktree.status = passed`
- `admission_check.validation_runner_self_check.status = passed`
- `admission_check.v11_harness_self_check.status = passed`
- `admission_check.required_fixture_identities.status = passed`
- `admission_check.fixture_manifest_integrity.status = passed`
- `admission_check.required_local_executables.status = passed`
- `admission_check.filesystem_and_runtime_home.status = passed`
- `admission_check.bounded_local_resource_estimate.status = passed`
- `admission_check.local_loopback.status = passed`
- `admission_check.codex_authentication_material.status = passed`
- `admission_check.external_network_capability.status = passed`
- `admission_check.authenticated_v11_external_transmission.status = passed`
- `admission_check.operator_external_transmission_authorization.status = passed`
- `pre_final_candidate_check.status = passed`
- `authenticated_codex_outcomes.target = volicord`;
  `authenticated_codex_outcomes.volicord.status = passed`;
  `authenticated_codex_outcomes.volicord.classification = passed`.
- `authenticated_codex_outcomes.target = small-python`;
  `authenticated_codex_outcomes.small-python.status = passed`;
  `authenticated_codex_outcomes.small-python.classification = passed`.
- `authenticated_codex_outcomes.target = polyglot-medium`;
  `authenticated_codex_outcomes.polyglot-medium.status = passed`;
  `authenticated_codex_outcomes.polyglot-medium.classification = passed`.

The official V11 acceptance condition is satisfied. This opens Phase 8 entry;
it does not certify Phase 8 dogfood quality or completion.

## Known limits

- The sanitized capsule is a bounded projection and does not preserve raw
  target logs, provider payloads, source bodies, private prompt bodies, or
  per-step timing details.
- V11 qualifies the three maintained target journeys and required fixtures; it
  does not establish the broader repeated-dogfood conclusions owned by Phase 8.
- This documentation-only conclusion commit is not part of the exact-final
  production/test candidate and does not require another final aggregate.

## Recommended implementation choice

Begin a fresh Phase 8 naturalistic Dogfood campaign under the maintained
repeated-use and replacement-gate plan. Fresh-resume sessions must resolve the
repository-bound Project before Recall. Retain the accepted production,
provider, credential, and validation lifecycle boundaries while evaluating
Phase 8 quality.

## Rejected alternatives and reasons

None recorded. This maintained conclusion directly reflects the complete,
internally consistent sanitized gate capsule without reconstructing ignored raw
evidence or substituting a different candidate identity.

## Reusable primitive decision

`reference_only` for production. The V11 harness remains maintained external
validation orchestration and does not own product semantics. The self-authored
fixtures remain reusable validation inputs.

## Decision revisit trigger status

Official V11 owns this assessment. The capsule reports
`decision_revisit_trigger_assessment = reported_by_official_v11` and
`active_decision_revisit_triggers = []` from the bounded accepted Decision
register evidence:

- `decision_revisit_trigger_source.kind = accepted_decision_register`
- `decision_revisit_trigger_source.path = rebuild/docs/design/open-decisions.md`
- `decision_revisit_trigger_source.content_sha256 = 58e15ec852c3fd0d3f5dc3bd481ec01b79c4f90870baf82eee9b9f6e9bcf4a35`
- `decision_revisit_trigger_source.assessed_decision_count = 14`
- `decision_revisit_trigger_source.assessed_decision_ids = ["Q1","Q2","Q3","Q4","Q5","Q6","Q7","Q8-A","Q8-B","Q9","Q10","Q11","Q12","Q13"]`

Because the official gate passed and the official active-trigger list is
empty, the Phase 8 entry gate is ready. This documentation conclusion does not
independently reinterpret accepted Decision triggers.

## Follow-up work

Phase 8 may begin with repeated dogfood and replacement-gate evaluation. Phase
9 cutover remains separate and must not begin until its maintained gate is
satisfied.

## Artifacts

- Supplied sanitized evidence kind: `validation_handoff_capsule`.
- Exact-final summary SHA-256:
  `077bb1d95971a00f693fd26ea5ac723d77fa36344aa8bc5c36ef3150c7f68cfd`.
- Official V11 result SHA-256:
  `3f9aa54167c6c129316730de43fa446e38a58ebff0c61246f2704ce363cd3c96`.
- Maintained inputs: this report, `harness.py`, fixture
  `v11-polyglot-medium`, reused fixture `v01-python`, and
  `rebuild/validation/shared/fixture-manifest.json`.

Raw logs and local validation artifacts remain ignored and are not durable
cross-session dependencies for this conclusion.
