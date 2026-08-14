# V11 — End-to-end multi-repository journey

## Status

Failed. Phase 8 is blocked. The clean official run completed all scheduled steps
for all three repository classes and reported 27 `passed`, 3 `partial`, 18
`unsupported`, 3 `failed`, 3 `environment_blocked`, and 0 `skipped` outcomes.

The exact Phase 7 final aggregate passed at production HEAD
`c15a864a4723deeee5db7dbe0b5346d08fecfff6`. The V11 support commit
`1ac4135dc961cdbd9faa70dffcd34c13c3df7be2` is validation-only; no production,
installer, runtime-contract, or design-owner change followed the production
gate.

## Goal

Rehearse one installed Volicord journey against the Volicord repository, a
small single-language application, and a medium documented polyglot repository,
using current CLI and MCP boundaries without validation-only substitutes for
Candidate, Inquiry, merge, Guarded, canonical, or recovery semantics.

## Accepted decisions being validated

- Q1 staged Inquiry and terminal material Question handling.
- Q2 polyglot capability and honest per-language degradation.
- Q3 local-first operation, Project opt-in, and background-provider isolation.
- Q4 installed CLI, MCP, viewer-facing logical surfaces, and user inspectability.
- Q5 four source-grounded document types in Markdown and self-contained HTML.
- Q6 portable Project identity, explicit clone binding, divergence, and conflict.
- Q7 correction, semantic supersession, and privacy-prioritized deletion.
- Q8-A Linux/Codex installation and connection; Q8-B fresh-service legacy exclusion.
- Q9 bounded read-only Recall; Q10 Candidate collection/promotion boundaries.
- Q11 source-grounded Checkpoint; Q12 exact Guarded confirmation/effect behavior.
- Q13 Decision applicability, reuse, and evidence-driven re-questioning.

## Input repositories and revisions

| Class | Run revision | Deterministic content identity | Origin/license |
| --- | --- | --- | --- |
| Volicord reconstruction repository | `1ac4135dc961cdbd9faa70dffcd34c13c3df7be2` | `sha256:3ecf5e172144a44a69a3ad611ee390393a8e6e1eb19bfa15e56f42ed4b2b74d8` | Current Git HEAD; repository license |
| Small Python application | `4903c02b70a28addf453c0817d21ad59a1c6a6f1` | fixture `v01-python`, `sha256:7feb9a79db3c37b10399171c615294286531cb12e0265263df2e6ec5d50c5867` | Self-authored, CC0-1.0 |
| Medium polyglot repository | `22a6acdd1807e5a768272c50bc84bec08c574fdd` | fixture `v11-polyglot-medium`, `sha256:7cb34ff3435dfd91a55f261e27ca407bfef7f3654aa8d8dac5c90eaa245edafb` | Self-authored, CC0-1.0 |

The polyglot fixture contains documentation plus Java/Maven, Python/pyproject,
and TypeScript/Node components. Its documented JSON process boundary is not
treated as a direct syntax or semantic call.

## Environment and tool versions

- Linux `6.18.33.2-microsoft-standard-WSL2`, x86_64 GNU/Linux.
- `rustc 1.97.1`, `cargo 1.97.1`, Python `3.12.3`, Git `2.43.0`.
- `codex-cli 0.145.0` with an available copied authentication file.
- Each target used a separate prefix, replacement Runtime Home, Codex home,
  temporary repository/clone, and legacy bait Runtime Home below ignored V11
  state. All three legacy sentinels remained byte- and timestamp-identical.

## Candidate approaches

The selected approach installed the current release binaries independently for
each target, used structured CLI JSON and the advertised MCP tool catalog, and
preserved every material child operation with separate stdout, stderr, exit or
termination, and duration evidence. The harness continued after bounded
failures so one repository or step could not hide another.

The maintained V01 Python fixture was reused for the small application. The
existing four-file V01 polyglot fixture did not satisfy the medium-repository
role, so V11 added one bounded self-authored fixture and registered its hash,
origin, and license in the shared catalog.

## Commands and configuration

Support checks:

```text
rebuild/scripts/validate focused v11-harness-self-check-amend -- rebuild/validation/end-to-end/multi-repository/harness.py self-check
rebuild/scripts/validate focused v11-fixture-manifest-amend -- rebuild/scripts/check-fixture-manifest rebuild/validation/shared/fixture-manifest.json
```

Clean official preflight:

```text
rebuild/scripts/validate focused v11-official-preflight-amended -- rebuild/validation/end-to-end/multi-repository/harness.py preflight --validated-head c15a864a4723deeee5db7dbe0b5346d08fecfff6 --final-artifact /home/minjungw00/projects/Volicord-rebuild/rebuild/.local/validation/20260814T125416.288396Z-final-26wy6xxw/summary.json
```

Official run:

```text
rebuild/scripts/validate focused v11-official-run-amended -- rebuild/validation/end-to-end/multi-repository/harness.py run --validated-head c15a864a4723deeee5db7dbe0b5346d08fecfff6 --final-artifact /home/minjungw00/projects/Volicord-rebuild/rebuild/.local/validation/20260814T125416.288396Z-final-26wy6xxw/summary.json --output-dir /home/minjungw00/projects/Volicord-rebuild/rebuild/.local/v11/20260814-official-1ac4135d
```

The exact final aggregate was not rerun.

## Observed results

The following result was identical by repository class unless noted.

| Journey boundary | Outcome | Observation |
| --- | --- | --- |
| Clean install and replacement runtime | `passed` | Three executable binaries and four replacement stores were installed separately; legacy bait was untouched. |
| Direct MCP connection | `passed` | All 14 advertised high-level tools were discoverable; `project_health` returned connected/healthy. |
| Authenticated Codex turn | `environment_blocked` | All three exact child calls exited 1 after network requests failed with `Operation not permitted`; the requested external retry was not authorized. |
| Project init/bind | `passed` | Stable Project IDs and exact repository bindings were returned. |
| Inventory, structural capability, and understanding | `passed` | Structured analysis and MCP maps exposed entities, gaps, and source-grounded coverage. |
| Candidate collection/inspection/promotion | `unsupported` | Inspection returned an empty read-only list after analysis; no collection, promotion, dismissal, or expiry tool exists in the public catalog. |
| Staged Inquiry and Decision | `unsupported` | The frontier remained empty because no public path collected/promoted a material Question; `decision_record` therefore had no valid exact revision to answer. |
| Ordinary repository work | `passed` | A controlled ordinary file write completed without changing the Guarded store. |
| Guarded exact confirmation/effect | `unsupported` | Exact request fields, mismatch rejection, and Source-linked confirmation were observable. No public dispatch path exists, so completion/failure/indeterminate, consumption reuse, and no-silent-retry could not be exercised. The controlled target remained present. |
| Source-grounded Checkpoint | `failed` | CLI `checkpoint record ... handoff` exited 1 in every repository. The CLI supplies `handoff_to: None`; the canonical owner requires an explicit handoff target. |
| Restart and Recall | `partial` | A fresh MCP process provided read-only Recall, but required Decision rationale and successful Checkpoint context were absent. |
| Portable export/import/bind | `passed` | The bundle imported with the same Project identity and bound explicitly to another clone. |
| Divergent conflict handling | `unsupported` | Independent additions imported, but public surfaces expose neither same-record conflict creation/inspection nor user resolution/branching. |
| Correction, supersession, deletion | `unsupported` | Explicit Source deletion succeeded. No integrated Decision or Context Item existed to exercise correction and supersession. |
| Four document outputs | `passed` | Four Markdown plus four HTML artifacts were published per repository; before/after canonical bundles were byte-identical. |
| Parser degradation | `passed` | Controlled malformed Rust/Python areas produced scoped `partial` results with failed scopes while unaffected capability remained. |
| Derived-index corruption/recovery | `passed` | Corruption degraded health; supported repair published a fresh Source/Analysis basis and preserved Recall meaning in all three repositories. |
| Provider unavailable/recovery | `unsupported` | Local-only analysis remained usable with provider unconfigured, but no public operation requests background semantic dispatch, so unavailable-provider failure and recovery could not be executed. |

## Coverage and failures

The run exercised every scheduled step for every required repository; no step
was skipped. Per target there were 9 passed, 1 partial, 6 unsupported, 1 failed,
and 1 environment-blocked results.

The three repeated CLI Checkpoint failures are owned by the Local Operations
CLI adapter at the boundary with the Canonical Context Checkpoint invariant.
The Candidate/Inquiry gap spans Host and User Adapters, Inquiry and Decision,
and Candidate lifecycle operations. Guarded dispatch and outcome recovery are
owned by Local Operations. Conflict presentation/resolution is owned by
Portable Context plus a Host/User surface. Provider failure/recovery requires a
Repository Intelligence/Provider Boundary operation exposed through Local
Operations.

## Performance and resource observations

The focused official wrapper ran for `181912.926 ms`; the structured harness
duration was `181345.752 ms`. The ignored evidence tree contained 121 child
operation directories and was about 1.9 GiB, dominated by three isolated
release installations/build trees. Authenticated Codex attempts lasted about
38 seconds each. Peak memory was not measured. No timeout or signal termination
was reported by the focused wrapper.

## Privacy and external transmission

Background semantic configuration remained disabled for every Project. Direct
MCP, inventory, structural analysis, canonical operations, documents, and
recovery were local. No provider transmission was recorded, and portable
bundles did not use legacy or copied raw-session input.

The authenticated Codex prompts contained the bounded Project ID and requested
health operation. Network connection failed before a successful turn or
Volicord tool result. A requested external retry was rejected because it could
transmit Project identifiers, local paths, and returned health metadata; V11
therefore retains `environment_blocked` rather than claiming success.

## Acceptance results

| Acceptance scenario | V11 conclusion |
| --- | --- |
| A, P — install, connection, fresh-service boundary | `partial`: install/direct MCP/legacy exclusion passed; authenticated Codex was environment-blocked. |
| B–E — inventory, structural, polyglot understanding | `passed` for the three V11 repositories, including honest malformed-area degradation; this is not a new all-language Phase 8 qualification. |
| F, L — Inquiry, Decision, applicability/reuse | `unsupported`: no integrated public Question promotion path. |
| G — Candidate boundary | `unsupported`: inspection exists; collection and lifecycle actions do not. |
| H — ordinary work and Checkpoint | `failed`: ordinary work passed; CLI Handoff Checkpoint failed. |
| I — new-session Recall | `partial`: read-only restart worked, but required Decision/Checkpoint content was incomplete. |
| J — portable clone and conflict | `partial/unsupported`: export/import/bind and independent additions worked; semantic conflict handling did not. |
| K — correction, supersession, deletion | `partial/unsupported`: deletion worked; correction/supersession did not have an integrated target. |
| M — Guarded effect | `unsupported`: request/confirmation evidence exists, dispatch/outcome/reuse behavior does not have a public path. |
| N — viewer/document projection | `partial`: all required outputs and canonical purity passed, but their integrated Decision/Checkpoint inputs were incomplete. |
| O — degraded recovery | `partial`: parser and index recovery passed; provider failure/recovery was unsupported. |
| Q — provider privacy | `partial`: local-only/no-transmission held; unavailable provider dispatch and recovery were not executable. |

The final V11 acceptance condition is failed. A successful repository cannot
mask the repeated failure/unsupported result in the other repositories; here
the blockers reproduced in all three.

## Known limits

- Authenticated Codex evidence is environment-blocked by network policy, not a
  Volicord connection result.
- The run does not qualify large-repository latency, accessibility, concurrent
  clients, abrupt power loss, hostile path races, or non-Linux hosts.
- V11 did not replace Phase 8 first-structural-language/fallback fixtures or
  narrative quality scoring.
- The exact Checkpoint CLI error renders only the outer message; source
  inspection locates the incompatible `handoff_to` construction without
  changing production code.
- The first diagnostic run used an over-strict derived-recovery comparison. It
  is not cited as the official conclusion; the committed harness and clean
  rerun compare Recall meaning while requiring the repository Source to refresh.

## Recommended implementation choice

Do not enter Phase 8 and do not repair production code in this validation
session. Plan a new production remediation workstream from the concrete V11
evidence. That workstream must expose existing owner semantics through a
coherent supported journey for Candidate lifecycle/Question promotion,
Guarded dispatch/outcome inspection, provider failure/retry, and portable
conflict resolution, and must correct the CLI Handoff Checkpoint construction.
It requires its own exact production final gate before V11 is attempted again.

## Rejected alternatives and reasons

- Seeding Candidate, Question, Decision, conflict, or Guarded outcome directly
  through validation-only storage/API calls was rejected because it could pass
  while installed production surfaces remained unusable.
- Inventing a validation-only guarded effect dispatcher was rejected for the
  same reason.
- Treating an unconfigured provider as an unavailable-provider dispatch was
  rejected because it did not exercise failure/retry ownership.
- Flattening partial/unsupported/environment-blocked outcomes into pass was
  rejected by the V11 acceptance contract.
- Production fixes were rejected as out of scope for this session.

## Reusable primitive decision

`reference_only` for production. The Python harness remains maintained external
validation orchestration; it does not own or qualify domain semantics for
promotion. The self-authored polyglot fixture remains reusable validation input.

## Decision revisit trigger status

No accepted Q1–Q13 revisit trigger is active. The evidence demonstrates missing
or defective implementation/public integration paths, not that the accepted
Candidate, Inquiry, portable, Guarded, provider, Checkpoint, or Linux/Codex
contracts are infeasible. No unresolved product Question was added.

## Follow-up work

1. Open a separately scoped production remediation workstream for the owners
   named under Coverage and failures.
2. Run that workstream's focused tests and exact final aggregate at its final
   production HEAD.
3. Re-run V11 independently against all three repository classes, including an
   authorized authenticated Codex environment.
4. Start Phase 8 only after the repeated V11 report is fully passed with no
   active Decision revisit trigger.

## Artifacts

- Phase 7 final aggregate:
  `rebuild/.local/validation/20260814T125416.288396Z-final-26wy6xxw/summary.json`,
  SHA-256 `2830792885c625be63eb27368b96436bd4b42ab5fe764389bcabacaf97b20a16`.
- Clean V11 preflight:
  `rebuild/.local/validation/20260814T131621.141459Z-v11-official-preflight-amended-avotb41c/result.json`,
  SHA-256 `4321937ef4d79e8102cbc543b489e1a94f3623e6301b1a0bb4ef8b29f71169e2`.
- Focused official wrapper:
  `rebuild/.local/validation/20260814T131632.123725Z-v11-official-run-amended-gcauhvov/result.json`,
  SHA-256 `f607c9d4dbe697520546a4d69d33ffcb744df6012e4df375ab0df13c40fa5cf6`.
- Structured official result:
  `rebuild/.local/v11/20260814-official-1ac4135d/result.json`, SHA-256
  `ffe64dcbcdae74c12bd3cc59e84789a6fc2efaa002df535c6029bd4c75c2e91e`.
- Maintained inputs: this report, `harness.py`, fixture
  `v11-polyglot-medium`, reused fixture `v01-python`, and
  `rebuild/validation/shared/fixture-manifest.json`.

Raw child logs, installed binaries, cloned repositories, Runtime Homes,
generated documents, and copied authentication material remain ignored local
artifacts and are not maintained source.
