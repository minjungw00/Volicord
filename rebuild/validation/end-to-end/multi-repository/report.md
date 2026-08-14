# V11 — End-to-end multi-repository journey

## Status

Passed. The official V11 gate completed all 54 required steps for exact-final
production/test candidate HEAD `21e7f8f77b8ecfc44be7a71b140533ad191bb683`.
All 54 steps passed, no blocking classification was reported, and
`phase_8_ready = true`. Phase 8 may begin; this result does not claim that
Phase 8 itself has passed.

## Goal

Rehearse one installed Volicord journey against the Volicord reconstruction
repository, a small single-language application, and a medium documented
polyglot repository. Use current CLI and MCP boundaries for Candidate,
Inquiry, Guarded provider, canonical, portable, document, and recovery
semantics without validation-only substitutes, while retaining no reusable
Codex authentication material in V11 evidence.

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
| Volicord reconstruction repository | candidate HEAD `21e7f8f77b8ecfc44be7a71b140533ad191bb683` | authenticated Codex target `volicord`: status `passed`, classification `passed` |
| Small Python application | fixture `v01-python` (V01), SHA-256 `7feb9a79db3c37b10399171c615294286531cb12e0265263df2e6ec5d50c5867` | authenticated Codex target `small-python`: status `passed`, classification `passed` |
| Medium polyglot repository | fixture `v11-polyglot-medium` (V11), SHA-256 `7cb34ff3435dfd91a55f261e27ca407bfef7f3654aa8d8dac5c90eaa245edafb` | authenticated Codex target `polyglot-medium`: status `passed`, classification `passed` |

The exact-final validated candidate, capsule candidate, pre-final observed
candidate, and current documentation-session starting HEAD are the same
commit. The two fixture content identities match the required identities in
the supplied sanitized capsule.

## Environment and tool versions

Admission status was `eligible`. Immediately before exact final, the gate
observed a clean worktree with zero dirty entries and confirmed that HEAD was
unchanged at `21e7f8f77b8ecfc44be7a71b140533ad191bb683`.

The sanitized capsule intentionally does not project tool versions or raw
environment details. No authorization or environment blocker was reported:
`blocking_classification = null`.

## Candidate approaches

The maintained gate used the one-way lifecycle from admission through one
exact final and the same-session official V11. The later documentation-only
conclusion interprets the copied sanitized capsule and does not alter or extend
the sealed production/test candidate.

## Commands and configuration

The exact-final aggregate reported `status = succeeded`, four commands, and
zero failures:

| Command | Outcome | Exit | Termination | Duration (ms) |
| --- | --- | ---: | --- | ---: |
| `cargo_metadata` | `succeeded` | 0 | `null` | 48.512 |
| `cargo_fmt` | `succeeded` | 0 | `null` | 524.758 |
| `cargo_clippy` | `succeeded` | 0 | `null` | 10452.019 |
| `cargo_test` | `succeeded` | 0 | `null` | 19565.197 |

The exact-final summary SHA-256 is
`61ce1dfce9f5a349a02c200779428afd3bed8f8085e8c935dac2f956659258de`.
Final and official V11 were run by the immediately preceding gate session and
are not rerun for this documentation conclusion.

## Observed results

Official V11 reported `status = passed`, `required_step_count = 54`, and
`phase_8_ready = true`. Its result SHA-256 is
`66d65c73fadb6b69437375b60642972f9f7c8661635fce7489ca3b8f8f2cbb22`.
The final-validated and V11-validated candidate HEAD is
`21e7f8f77b8ecfc44be7a71b140533ad191bb683`.

## Coverage and failures

| Official V11 status | Count |
| --- | ---: |
| `passed` | 54 |
| `failed` | 0 |
| `partial` | 0 |
| `unsupported` | 0 |
| `skipped` | 0 |
| `environment_blocked` | 0 |
| **Required total** | **54** |

All three required authenticated Codex targets passed with `passed`
classification. The official result contains no failed, partial, unsupported,
skipped, or environment-blocked step.

## Performance and resource observations

The capsule records exact-final command durations of 48.512 ms, 524.758 ms,
10452.019 ms, and 19565.197 ms. It does not project official V11 per-step
timings, output sizes, or peak-resource measurements, so this conclusion makes
no broader performance claim.

## Privacy and external transmission

The credential-retention audit reported `status = passed`,
`auth_named_file_count = 0`, `credential_content_match_count = 0`, and
`scan_error_count = 0`. No credential content or reusable secret fingerprint
is recorded in this report.

## Acceptance results

| Acceptance area | Current conclusion |
| --- | --- |
| Admission | `eligible` |
| Pre-final candidate identity and clean worktree | `passed`: unchanged candidate HEAD and zero dirty entries |
| Exact final | `succeeded`: four commands succeeded, zero failures |
| Three-repository official V11 | `passed`: 54 of 54 required steps passed |
| Authenticated Codex targets | `passed`: `volicord`, `small-python`, and `polyglot-medium` |
| Credential-retention audit | `passed`: all three recorded counts are zero |
| Blocking classification | `null` |
| Phase 8 entry | `phase_8_ready = true` |

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

Begin Phase 8 under the maintained repeated-dogfood and replacement-gate plan.
Retain the accepted production, provider, credential, and validation lifecycle
boundaries while evaluating Phase 8 quality.

## Rejected alternatives and reasons

None recorded. This maintained conclusion directly reflects the complete,
internally consistent sanitized gate capsule without reconstructing ignored raw
evidence or substituting a different candidate identity.

## Reusable primitive decision

`reference_only` for production. The V11 harness remains maintained external
validation orchestration and does not own product semantics. The self-authored
fixtures remain reusable validation inputs.

## Decision revisit trigger status

The capsule reports `active_decision_revisit_triggers = []` and
`decision_revisit_trigger_assessment = independent_documentation_review_required`.
Independent review of the accepted Q1–Q13 criteria found no active Decision
revisit trigger, consistent with the observed passed gate.

## Follow-up work

Phase 8 may begin with repeated dogfood and replacement-gate evaluation. Phase
9 cutover remains separate and must not begin until its maintained gate is
satisfied.

## Artifacts

- Supplied sanitized evidence kind: `validation_handoff_capsule`.
- Exact-final summary SHA-256:
  `61ce1dfce9f5a349a02c200779428afd3bed8f8085e8c935dac2f956659258de`.
- Official V11 result SHA-256:
  `66d65c73fadb6b69437375b60642972f9f7c8661635fce7489ca3b8f8f2cbb22`.
- Maintained inputs: this report, `harness.py`, fixture
  `v11-polyglot-medium`, reused fixture `v01-python`, and
  `rebuild/validation/shared/fixture-manifest.json`.

Raw logs and local validation artifacts remain ignored and are not durable
cross-session dependencies for this conclusion.
