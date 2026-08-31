# V11 — End-to-end multi-repository journey

## Status

Passed. The official V11 gate completed all 54 required steps for exact-final
production/test candidate HEAD `6031641c46cf014a754442dcee3137caf265882e`.
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
- Q14 Engineering Choice Discovery and explicit learning participation.

## Input repositories and revisions

| Class | Validated identity | Capsule-recorded outcome |
| --- | --- | --- |
| Volicord reconstruction repository | `validated_candidate_head = 6031641c46cf014a754442dcee3137caf265882e` | authenticated Codex target `volicord`: status `passed`, classification `passed` |
| Small Python application | fixture `v01-python` (V01), SHA-256 `7feb9a79db3c37b10399171c615294286531cb12e0265263df2e6ec5d50c5867` | authenticated Codex target `small-python`: status `passed`, classification `passed` |
| Medium polyglot repository | fixture `v11-polyglot-medium` (V11), SHA-256 `7cb34ff3435dfd91a55f261e27ca407bfef7f3654aa8d8dac5c90eaa245edafb` | authenticated Codex target `polyglot-medium`: status `passed`, classification `passed` |

The capsule's dependency and required-fixture identities are:

| Capsule field | Path or identity | Status | SHA-256 |
| --- | --- | --- | --- |
| `cargo_lock` | `rebuild/Cargo.lock` | `cargo_lock.status = available` | `cargo_lock.sha256 = a8b13e8e14761867d62a682dba972ae762507976f53531a5350ba20dc19192ab` |
| `workspace_manifest` | `rebuild/Cargo.toml` | `workspace_manifest.status = available` | `workspace_manifest.sha256 = 4b3a0552a71547385b4246ea9b3ec5103581a436ef2b14650014b78efb28a670` |
| `fixture_manifest` | `rebuild/validation/shared/fixture-manifest.json` | `fixture_manifest.status = available` | `fixture_manifest.sha256 = e52edec239cb9a5370e4013ae0f42f3021484cc5bd5fc6aadb651a6cb9370164` |

- `fixture.id = v01-python`; `fixture.content_sha256 = 7feb9a79db3c37b10399171c615294286531cb12e0265263df2e6ec5d50c5867`.
- `fixture.id = v11-polyglot-medium`; `fixture.content_sha256 = 7cb34ff3435dfd91a55f261e27ca407bfef7f3654aa8d8dac5c90eaa245edafb`.
- `fixture.id = repository-intelligence-realistic-v1`; `fixture.content_sha256 = df249af327700395d99953374bc9837b7435eb94a562dd6f5fd231ff25b47d36`.
- `fixture.id = v12-current-codex-mcp-completion`; `fixture.content_sha256 = 3a1a7b175cffdeb9943f46595fb8c151024b68bb548d86d6bb29e32d129729c0`.
- `fixture.id = v07-background-provider-bounded-rust`; `fixture.content_sha256 = 13bd3a5d20d64636b24c5298b671e988662cb6d327c4649411ed0903d31ce97c`.

The exact-final validated candidate, capsule candidate, dependency snapshot,
required identity, pre-final observed candidate, and documentation-session
starting HEAD all identify the same commit.

## Environment and tool versions

Admission status was `eligible`. Immediately before exact final, the gate
observed a clean worktree with zero dirty entries and confirmed that HEAD was
unchanged at `6031641c46cf014a754442dcee3137caf265882e`.

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
| `final_command.cargo_metadata.outcome`; `final_command.cargo_metadata.exit_code`; `final_command.cargo_metadata.termination`; `final_command.cargo_metadata.spawn_error`; `final_command.cargo_metadata.duration_ms` | `succeeded`; `0`; `null`; `false`; `11.389` |
| `final_command.cargo_fmt` | `cargo fmt --manifest-path rebuild/Cargo.toml --all -- --check` |
| `final_command.cargo_fmt.outcome`; `final_command.cargo_fmt.exit_code`; `final_command.cargo_fmt.termination`; `final_command.cargo_fmt.spawn_error`; `final_command.cargo_fmt.duration_ms` | `succeeded`; `0`; `null`; `false`; `669.946` |
| `final_command.cargo_clippy` | `cargo clippy --manifest-path rebuild/Cargo.toml --workspace --all-targets --all-features -- -D warnings` |
| `final_command.cargo_clippy.outcome`; `final_command.cargo_clippy.exit_code`; `final_command.cargo_clippy.termination`; `final_command.cargo_clippy.spawn_error`; `final_command.cargo_clippy.duration_ms` | `succeeded`; `0`; `null`; `false`; `10225.389` |
| `final_command.cargo_test` | `cargo test --manifest-path rebuild/Cargo.toml --workspace --all-targets --all-features` |
| `final_command.cargo_test.outcome`; `final_command.cargo_test.exit_code`; `final_command.cargo_test.termination`; `final_command.cargo_test.spawn_error`; `final_command.cargo_test.duration_ms` | `succeeded`; `0`; `null`; `false`; `43352.971` |

- `gate_configuration.argv = rebuild/scripts/validate gate --external-network available --authorize-external-transmission v11-openai-codex-project-health-three-targets --authorize-provider-source-transmission openai-codex-background-semantic-bounded-rust-v1 --provider-model gpt-5.6-sol`
- `gate_configuration.argv_status = complete`
- `technical_external_network_assertion = available`
- `authorization_assertion_id = v11-openai-codex-project-health-three-targets`
- `provider_authorization_assertion_id = openai-codex-background-semantic-bounded-rust-v1`
- `provider_model = gpt-5.6-sol`
- `final_artifact_produced_by_gate = true`
- `v11_preflight_consumed_same_gate_final_artifact = true`
- `official_v11_consumed_same_gate_final_artifact = true`
- `final_aggregate.status = succeeded`; `final_aggregate.command_count = 4`;
  `final_aggregate.failure_count = 0`.

The exact-final summary SHA-256 is
`final_summary_sha256 = ada27af45ba7dee5606e0a8e2fcc0b305617d6ce2a1fd5a05f950c75f62e5cc4`.
Final and official V11 were run by the immediately preceding gate session and
are not rerun for this documentation conclusion.

## Observed results

Official V11 reported `official_v11.status = passed`,
`official_v11.required_step_count = 54`, and
`official_v11.phase_8_ready = true`. Its result SHA-256 is
`official_v11.result_sha256 = 015d50dd0201992d1944c2413863b55319602c23b7462e56b9ed70f82d3b661e`.
The final-validated and V11-validated candidate HEAD is
`6031641c46cf014a754442dcee3137caf265882e`.

The separately authorized live production-provider qualification reported
`live_provider_qualification.status = passed` and
`live_provider_qualification.evidence_sha256 = 31adc8b010babed6ef40b10ebfb716377f13a4987be84aeb815ed137e0a68576`.
It used provider `openai-codex`, model `gpt-5.6-sol`, and the authenticated
installed Codex CLI transport. The bounded source was the single 632-byte
`src/lib.rs` from fixture `background-provider-bounded-rust-v1`, content
SHA-256 `13bd3a5d20d64636b24c5298b671e988662cb6d327c4649411ed0903d31ce97c`.
The successful request was `transmitted` and `completed`, produced three
semantic annotations with complete provenance, and recorded repository
snapshot `52c5535f21c646e7d55a5db6b792894f3a26d10aa4c1e3a7be736b58f18650ba`
and analysis snapshot `cbb487254201c41e1e8bfc9a4312827b180ecfee20ffc63e4a099a91f78d8f44`.
The separate unavailable-provider probe recorded `provider_unavailable` and
`not_transmitted` while preserving Guarded-confirmation consumption and local
canonical continuity. Provider-side deletion remains
`unsupported_by_adapter`; retained source body, provider response body, and
credential state are all `false`.

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

The capsule records exact-final command durations of 11.389 ms, 669.946 ms,
10225.389 ms, and 43352.971 ms. It does not project official V11 per-step
timings, output sizes, or peak-resource measurements, so this conclusion makes
no broader performance claim.

## Privacy and external transmission

The bounded transmission configuration was
`external_transmission.required = true`, with
`external_transmission.destination = OpenAI Codex service used by the installed Codex CLI`,
`external_transmission.purpose = three authenticated turns that select each repository-scoped Volicord project_health MCP tool`,
and
`external_transmission.source_scope = bounded V11 prompt, Project identity, and project_health tool result; no intended repository source body`.
The target records are `external_transmission.scope = volicord`,
`external_transmission.scope = small-python`, and
`external_transmission.scope = polyglot-medium`; the bounded assertion is
`external_transmission.authorization_assertion = v11-openai-codex-project-health-three-targets`.

The independent provider transmission recorded
`provider_external_transmission.destination = OpenAI Codex service used by the installed Codex CLI`,
`provider_external_transmission.purpose = qualify the production background semantic provider against one bounded maintained Rust source`, and
`provider_external_transmission.source_scope = the maintained bounded-rust fixture's src/lib.rs source body, at most 4096 bytes`.
Its sole
`provider_external_transmission.scope = rebuild/validation/privacy/background-provider-qualification/fixtures/bounded-rust/src/lib.rs`,
and its authorization was
`provider_external_transmission.authorization_assertion = openai-codex-background-semantic-bounded-rust-v1`.

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
| Live production-provider qualification | `live_provider_qualification.status = passed`: bounded success and unavailable-provider degradation retained truthful outcomes |
| Three-repository official V11 | `passed`: 54 of 54 required steps passed |
| Authenticated Codex targets | `passed`: `volicord`, `small-python`, and `polyglot-medium` |
| Credential-retention audit | `passed`: all three recorded counts are zero |
| Blocking classification | `blocking_classification = null` |
| Phase 8 entry | `phase_8_ready = true` |
| Sanitized evidence archive | `evidence_archive.status = verified`; `evidence_archive.verification_status = passed`; `evidence_archive.prerequisites_passed = true` |

The verified sanitized evidence archive records
`evidence_archive.candidate_head = 6031641c46cf014a754442dcee3137caf265882e`,
`evidence_archive.filename = validation-evidence-6031641c46cf.tar.gz`,
`evidence_archive.sha256 = bed36fa89e82a60e198dd3dc3ef5b8864e9ce27332a8a60461f230a4f74adc0e`,
`evidence_archive.size_bytes = 14510`, and
`evidence_archive.member_count = 9`.

All admission and target outcomes from the capsule are retained below:

- `admission_check.candidate_identity_and_clean_worktree.status = passed`
- `admission_check.validation_runner_self_check.status = passed`
- `admission_check.v11_harness_self_check.status = passed`
- `admission_check.architecture_contracts.status = passed`
- `admission_check.architecture_contracts_self_test.status = passed`
- `admission_check.repository_intelligence_realistic_qualification.status = passed`
- `admission_check.dogfood_harness_self_test.status = passed`
- `admission_check.dogfood_campaign_self_test.status = passed`
- `admission_check.provider_qualification_self_test.status = passed`
- `admission_check.required_fixture_identities.status = passed`
- `admission_check.fixture_manifest_integrity.status = passed`
- `admission_check.required_local_executables.status = passed`
- `admission_check.filesystem_and_runtime_home.status = passed`
- `admission_check.bounded_local_resource_estimate.status = passed`
- `admission_check.local_loopback.status = passed`
- `admission_check.codex_authentication_material.status = passed`
- `admission_check.external_network_capability.status = passed`
- `admission_check.authenticated_v11_external_transmission.status = passed`
- `admission_check.production_provider_external_transmission.status = passed`
- `admission_check.operator_external_transmission_authorization.status = passed`
- `admission_check.operator_provider_source_transmission_authorization.status = passed`
- `admission_check.provider_qualification_model.status = passed`
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
repeated-use and replacement-gate plan from a separate clean worktree whose
actual Git `HEAD` is exactly the sealed production/test candidate
`6031641c46cf014a754442dcee3137caf265882e`. The later documentation-only
commit must not be used for qualification by supplying the sealed commit only
as a harness candidate argument. Use the maintained
`rebuild/scripts/dogfood-campaign` helper for campaign setup, routine evidence
collection, bundle export, bounded Runtime summaries, descriptor and manifest
completion, and review packaging. Fresh-resume sessions must resolve the
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
- `decision_revisit_trigger_source.content_sha256 = 9264438238e011e329de35d140cb5ed533dde7b48b8522df5c8cda864b3ef736`
- `decision_revisit_trigger_source.assessed_decision_count = 15`
- `decision_revisit_trigger_source.assessed_decision_ids = ["Q1","Q2","Q3","Q4","Q5","Q6","Q7","Q8-A","Q8-B","Q9","Q10","Q11","Q12","Q13","Q14"]`

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
  `ada27af45ba7dee5606e0a8e2fcc0b305617d6ce2a1fd5a05f950c75f62e5cc4`.
- Official V11 result SHA-256:
  `015d50dd0201992d1944c2413863b55319602c23b7462e56b9ed70f82d3b661e`.
- Live production-provider evidence SHA-256:
  `31adc8b010babed6ef40b10ebfb716377f13a4987be84aeb815ed137e0a68576`.
- Sanitized capsule SHA-256:
  `a657b8c517a3135c8b64ce5b75ae98ef66a0e5f43573ef2c092e590e3eba7695`.
- Independently verified sanitized evidence archive SHA-256:
  `bed36fa89e82a60e198dd3dc3ef5b8864e9ce27332a8a60461f230a4f74adc0e`.
- Maintained inputs: this report, `harness.py`, fixtures
  `v11-polyglot-medium`, `v01-python`,
  `repository-intelligence-realistic-v1`,
  `v12-current-codex-mcp-completion`,
  `v07-background-provider-bounded-rust`, and
  `rebuild/validation/shared/fixture-manifest.json`.

Raw logs and local validation artifacts remain ignored and are not durable
cross-session dependencies for this conclusion.
