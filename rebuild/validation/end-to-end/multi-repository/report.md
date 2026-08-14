# V11 — End-to-end multi-repository journey

## Status

Blocked at the maintained preflight. The required prior exact-final aggregate
artifact was absent, so the corrected official rehearsal did not start and no
repository-step matrix was produced. The current maintained gate is
`phase_8_ready = false`.

The committed self-check passed all 18 required-step evidence assignments, the
hard-coded-status regression checks, and the synthetic ephemeral-authentication
lifecycle. That self-check is harness evidence only and does not substitute for
an official authenticated Codex rehearsal.

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

| Class | Planned revision or identity | Deterministic content identity | Origin/license |
| --- | --- | --- | --- |
| Volicord reconstruction repository | candidate HEAD `148aaa258c7d5a66c56629643d233e30310b5f2e` | Not measured because the official run did not start | Repository license |
| Small Python application | deterministic fixture revision `9eeedb3710e2ac53ab6d09981db5efde44f5a9b4` | fixture `v01-python`, `sha256:7feb9a79db3c37b10399171c615294286531cb12e0265263df2e6ec5d50c5867` | Self-authored, CC0-1.0 |
| Medium polyglot repository | deterministic fixture revision `93c9e5de4f5028673331723955442dd10e7b4839` | fixture `v11-polyglot-medium`, `sha256:7cb34ff3435dfd91a55f261e27ca407bfef7f3654aa8d8dac5c90eaa245edafb` | Self-authored, CC0-1.0 |

The two fixture hashes match the maintained shared fixture manifest. The
polyglot input still contains documentation plus Java/Maven,
Python/pyproject, and TypeScript/Node components. The official harness did not
clone or mutate any target after preflight failed.

## Environment and tool versions

- Linux `6.18.33.2-microsoft-standard-WSL2`, x86_64.
- `rustc 1.97.1`, `cargo 1.97.1`, Python `3.12.3`, Git `2.43.0`.
- `codex-cli 0.145.0`; authentication was available but was not read or staged
  because the official run did not start.
- The worktree was clean at HEAD
  `148aaa258c7d5a66c56629643d233e30310b5f2e` before the self-check and
  preflight.
- Commits `1030c4463b4461054083cb2cfdb737ff33bbcc8c` and
  `148aaa258c7d5a66c56629643d233e30310b5f2e` change only the V11 harness.
  There is no production or design-contract diff from final-validated HEAD
  `f64ee3eb8f5b66a9b458adbba0b4d66c979a4c8f`.

## Candidate approaches

The maintained approach remains one non-fail-fast rehearsal through the
installed CLI and MCP surfaces, with structured per-child stdout, stderr,
duration, exit, spawn, and termination evidence. Authenticated Codex receives
an isolated temporary home outside the retained V11 tree, and that staging
must be removed before each authenticated step can pass.

No fallback aggregate, mock Codex execution, validation-only product behavior,
or reconstructed prior-final artifact was used. The official run was withheld
after its required preflight failed.

## Commands and configuration

Required-step and authentication-lifecycle self-check:

```text
rebuild/scripts/validate focused v11-self-check-credential-safe -- rebuild/validation/end-to-end/multi-repository/harness.py self-check
```

Maintained official preflight:

```text
rebuild/scripts/validate focused v11-official-preflight-credential-safe -- rebuild/validation/end-to-end/multi-repository/harness.py preflight --validated-head f64ee3eb8f5b66a9b458adbba0b4d66c979a4c8f --final-artifact /home/minjungw00/projects/Volicord-rebuild/rebuild/.local/validation/20260814T170301.578433Z-final-qxhz9229/summary.json
```

The preflight exited 1 because the exact `--final-artifact` path did not
exist. The official `run` command was not invoked.

## Observed results

The self-check exited 0. It reported 18 required steps, 18 evidence-driven
assignments, a passed hard-coded-status regression policy, a passed synthetic
authentication lifecycle, and the expected polyglot fixture hash.

The preflight wrapper exited 1 without a signal, spawn error, or child
termination. The harness raised `FileNotFoundError` while opening the required
prior aggregate summary. Searches of the project worktrees and `/tmp` found no
retained exact-final summary matching the maintained report path; the remaining
aggregate-shaped artifact was the runner's intentionally failing fake
self-test and was not used.

## Coverage and failures

| Repository class | Required steps | Officially executed | Official outcome |
| --- | ---: | ---: | --- |
| Volicord | 18 | 0 | Not produced |
| Small Python | 18 | 0 | Not produced |
| Medium polyglot | 18 | 0 | Not produced |
| **Total** | **54** | **0** | **Blocked at preflight** |

There is no corrected official result artifact, repository/step status matrix,
or aggregate V11 status. No product pass, partial, unsupported, failed,
environment-blocked, or skipped row is inferred from an unstarted run.

## Performance and resource observations

The focused self-check ran for `94.994 ms`. The focused preflight ran for
`102.263 ms`. No target installation, repository copy, model turn, generated
document, or official V11 evidence tree was created, so target performance,
output size, and peak memory were not measured.

## Privacy and external transmission

The self-check used only its committed synthetic authentication lifecycle. An
explicit post-check audit found zero retained `auth.json` files, zero retained
copies of the synthetic credential content, and zero remaining V11
authentication staging directories in `/tmp`.

The preflight failed before real authentication was read, copied, transmitted,
or recorded. Therefore this session produced no real-credential retention, but
it also did not prove the real authenticated target lifecycle required by the
official V11 gate. No credential content or reusable credential fingerprint is
included in structured results or this report.

## Acceptance results

| Acceptance area | Current conclusion |
| --- | --- |
| Required-step static policy | `passed`: all 18 steps are evidence-driven and the hard-coded-status regression check passed. |
| Synthetic authentication lifecycle | `passed`: success, child failure, and handled-exception cleanup paths retained no synthetic authentication. |
| Clean official preflight | `failed`: the required prior exact-final aggregate artifact was unavailable. |
| Three-repository official rehearsal | `not run`: preflight did not authorize the run. |
| Real authenticated Codex lifecycle | `not run`: no target reached authenticated execution. |
| Credential-retention audit | `partial evidence only`: synthetic cleanup passed; the required real target and complete-rehearsal audits have no run to inspect. |
| V11 gate | `blocked`: `phase_8_ready = false`. |

The V11 acceptance condition is not satisfied. Phase 8 entry remains closed
even though no product failure was observed.

## Known limits

- The missing ignored prior-final artifact prevents the maintained preflight
  from proving its final-validated production anchor.
- Synthetic cleanup evidence does not qualify real authenticated Codex
  execution, per-target cleanup, or complete-rehearsal cleanup.
- No official repository identity, target matrix, provider/parser/index
  recovery result, document output, or model-service result was produced.
- This blocked result does not change any accepted Q1–Q13 product Decision and
  does not diagnose a production defect.

## Recommended implementation choice

Retain the current production and credential-staging responsibility boundaries.
Do not enter Phase 8 from this result. A separately authorized V11 session may
use a newly retained successful exact-final aggregate as its preflight anchor
and perform one corrected official rehearsal; this session must not retry the
official run.

## Rejected alternatives and reasons

- Reconstructing or fabricating the missing prior-final JSON was rejected
  because it would not be authoritative validation evidence.
- Using the runner's fake aggregate self-test was rejected because it is
  intentionally not the repository final suite.
- Proceeding to target execution after a failed maintained preflight was
  rejected because it would not satisfy the official-run contract.
- Treating synthetic cleanup as proof of real credential safety was rejected
  because authenticated target execution was not observed.

## Reusable primitive decision

`reference_only` for production. The Python harness remains maintained
external validation orchestration and does not own product semantics. The
self-authored fixtures remain reusable validation inputs.

## Decision revisit trigger status

No accepted Q1–Q13 Decision revisit trigger is active. The blocker is missing
validation evidence, not evidence against an accepted product Decision.
Nevertheless, the corrected V11 gate is not satisfied and Phase 8 remains
blocked.

## Follow-up work

1. Preserve a successful exact-final aggregate artifact from an authorized
   final-validation session.
2. In a new V11 session, pass the maintained preflight and perform exactly one
   corrected official rehearsal with real supported authenticated Codex.
3. Audit every target and the complete evidence tree for ephemeral
   authentication before reconsidering `phase_8_ready`.

## Artifacts

- Focused self-check:
  `rebuild/.local/validation/20260814T184951.497693Z-v11-self-check-credential-safe-di360evt/result.json`,
  SHA-256 `4e5c5802dde9f36874d985933a44ac2d24db9c82d05d297cadf66a4533e77c08`.
- Failed maintained preflight:
  `rebuild/.local/validation/20260814T185001.611692Z-v11-official-preflight-credential-safe-kltc_w0l/result.json`,
  SHA-256 `a382327e582847265c09aefe5be2cbe7fdbb57cfac800071ae4885dc214accad`.
- Missing required prior-final path:
  `rebuild/.local/validation/20260814T170301.578433Z-final-qxhz9229/summary.json`.
- Maintained inputs: this report, `harness.py`, fixture
  `v11-polyglot-medium`, reused fixture `v01-python`, and
  `rebuild/validation/shared/fixture-manifest.json`.

Raw logs and local validation artifacts remain ignored. No official target
worktree, Codex home, runtime, generated document, or credential artifact was
retained because the run did not start.
