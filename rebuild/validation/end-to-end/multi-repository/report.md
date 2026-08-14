# V11 — End-to-end multi-repository journey

## Status

Passed. The clean official rehearsal classified all 54 required repository-step
outcomes as `passed`, with zero `partial`, `unsupported`, `failed`,
`environment_blocked`, or `skipped` outcomes. The maintained gate evaluated
`phase_8_ready = true`.

The exact final aggregate passed at final-validated HEAD
`f64ee3eb8f5b66a9b458adbba0b4d66c979a4c8f`. The last production change is
`c2caea39d46229e880c8c906a0fffe91d8c2cb9c`; `f64ee3eb` changes only this V11
harness. The fresh preflight found no production diff, no later commit, and a
clean worktree.

## Goal

Rehearse one installed Volicord journey against the Volicord reconstruction
repository, a small single-language application, and a medium documented
polyglot repository. Use current CLI and MCP boundaries for Candidate,
Inquiry, Guarded provider, canonical, portable, document, and recovery
semantics without validation-only substitutes.

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

| Class | Run revision | Deterministic content identity | Origin/license |
| --- | --- | --- | --- |
| Volicord reconstruction repository | `f64ee3eb8f5b66a9b458adbba0b4d66c979a4c8f` | `sha256:9845d20b1e1afd676e4166fa32d88efa17a7c72cd1ea2d71d8a5e0307a462b34` | Final-validated current Git HEAD; repository license |
| Small Python application | `9eeedb3710e2ac53ab6d09981db5efde44f5a9b4` | fixture `v01-python`, `sha256:7feb9a79db3c37b10399171c615294286531cb12e0265263df2e6ec5d50c5867` | Self-authored, CC0-1.0 |
| Medium polyglot repository | `93c9e5de4f5028673331723955442dd10e7b4839` | fixture `v11-polyglot-medium`, `sha256:7cb34ff3435dfd91a55f261e27ca407bfef7f3654aa8d8dac5c90eaa245edafb` | Self-authored, CC0-1.0 |

The two fixture content hashes exactly match the maintained shared fixture
manifest. The polyglot input contains documentation plus Java/Maven,
Python/pyproject, and TypeScript/Node components connected by an explicitly
documented JSON process boundary; that boundary is not represented as a direct
syntax or semantic call.

## Environment and tool versions

- Linux `6.18.33.2-microsoft-standard-WSL2`, x86_64.
- `rustc 1.97.1`, `cargo 1.97.1`, Python `3.12.3`, Git `2.43.0`.
- `codex-cli 0.145.0` with available authentication and authorized model-service access.
- Each target used a separate prefix, replacement Runtime Home, Codex home,
  temporary repository/clone, and legacy bait Runtime Home. All three legacy
  sentinels remained byte- and timestamp-identical.

## Candidate approaches

The selected approach independently installed the current release binaries for
each target, used structured CLI JSON and the advertised MCP catalog, and
preserved every material child command with complete separate stdout, stderr,
exit, duration, spawn, and termination metadata. The non-fail-fast harness ran
all scheduled steps for every repository so one result could not mask another.

The authenticated Codex probe used the installed MCP registration and a narrow
approval for `project_health`. Background semantic behavior used the production
provider boundary with an intentionally unavailable provider, allowing exact
Guarded preparation, confirmation, terminal outcome, no-transmission, and local
degradation behavior to be observed without fabricating a provider success.

## Commands and configuration

Fresh clean preflight:

```text
rebuild/scripts/validate focused v11-official-preflight-remediation-conclusions -- rebuild/validation/end-to-end/multi-repository/harness.py preflight --validated-head f64ee3eb8f5b66a9b458adbba0b4d66c979a4c8f --final-artifact /home/minjungw00/projects/Volicord-rebuild/rebuild/.local/validation/20260814T170301.578433Z-final-qxhz9229/summary.json
```

Single official rehearsal:

```text
rebuild/scripts/validate focused v11-official-run-remediation-conclusions -- rebuild/validation/end-to-end/multi-repository/harness.py run --validated-head f64ee3eb8f5b66a9b458adbba0b4d66c979a4c8f --final-artifact /home/minjungw00/projects/Volicord-rebuild/rebuild/.local/validation/20260814T170301.578433Z-final-qxhz9229/summary.json --output-dir /home/minjungw00/projects/Volicord-rebuild/rebuild/.local/v11/20260815-official-f64ee3eb
```

The exact final aggregate was not rerun.

## Observed results

Every row below passed for Volicord, small Python, and medium polyglot.

| Journey boundary | Outcome | Observation |
| --- | --- | --- |
| Clean install and replacement runtime | `passed` | Three executable binaries were independently installed per target; the legacy bait remained untouched. |
| Direct MCP and authenticated Codex connection | `passed` | All 16 advertised tools were discoverable, direct health was connected/healthy, and each authenticated Codex turn selected the installed `project_health` tool. |
| Project init/bind | `passed` | A stable Project ID and exact repository binding were returned for each target. |
| Inventory, capability analysis, and understanding | `passed` | Structured analysis and MCP understanding exposed entities, relations, coverage, gaps, freshness, and source bases. |
| Candidate research and promotion | `passed` | A `research_required` Candidate remained absent from the frontier; insufficient evidence did not make it askable; premature readiness was rejected; sufficient source-grounded research allowed `ready_to_ask`, inspection, and explicit promotion. |
| Staged Inquiry and Decision | `passed` | The promoted exact Question revision appeared in the frontier, and one explicit current-host response created an inspectable Source-linked Decision. |
| Ordinary repository work | `passed` | A controlled ordinary file write completed without changing the Guarded store. |
| Guarded provider operation | `passed` | Exact inspection worked; denied preparation was discarded; missing and mismatched dispatch stayed unconsumed and not dispatched; confirmed dispatch consumed the Source-linked confirmation and truthfully ended `provider_unavailable`/`not_dispatched`; durable inspection matched; denied and consumed preparations were not reusable. |
| Source-grounded Handoff Checkpoint | `passed` | Each CLI Checkpoint used the Decision response Source and explicit `next Codex session` handoff target. |
| Restart and new-session Recall | `passed` | A fresh MCP process recovered the integrated Decision, rationale, known limits, and exact next step read-only. |
| Portable export/import/bind | `passed` | The bundle preserved Project identity and was explicitly rebound to another clone. |
| Divergent conflict handling | `passed` | Both clones superseded the same Decision; comparison exposed `semantic_decision_conflict` alongside independent additions; explicit resolution produced a provenance-bearing branch. |
| Correction, supersession, deletion | `passed` | The integrated Decision was corrected to revision 2 and superseded by a new Decision; the disposable Source was forgotten and absent on inspection. |
| Four document outputs | `passed` | Project & Architecture Guide, Decision Report, Implementation Plan, and Handoff / Resume Document were each published as grounded Markdown and self-contained HTML with no canonical mutation. |
| Provider degradation | `passed` | The unavailable production adapter recorded zero transmission while canonical inspection and local structural analysis remained usable. |
| Parser degradation | `passed` | Controlled malformed areas returned truthful scoped `partial` analysis with failed scopes and an unaffected usable remainder. |
| Derived-index corruption/recovery | `passed` | Corruption degraded health; repair published a fresh repository Source/Analysis basis, retained the expected parser partial state, and preserved Recall meaning. |

The official run directly observed denied and consumed terminal-preparation
cleanup in all three integrated repositories. The final-validated public Host
test `subsequent_host_interaction_cleans_an_expired_provider_preparation` also
passed in the consumed exact aggregate at the same HEAD. No production change
separates that expiry evidence from this rehearsal.

## Coverage and failures

| Repository class | Passed | Partial | Unsupported | Failed | Environment blocked | Skipped |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Volicord | 18 | 0 | 0 | 0 | 0 | 0 |
| Small Python | 18 | 0 | 0 | 0 | 0 | 0 |
| Medium polyglot | 18 | 0 | 0 | 0 | 0 | 0 |
| **Total** | **54** | **0** | **0** | **0** | **0** | **0** |

The evidence tree contains 142 recorded child commands. All 142 succeeded;
none had a spawn error, timeout, signal termination, or nonzero exit. Expected
domain rejections were returned as structured public Host results and were
included in the corresponding capability assertions.

## Performance and resource observations

The focused wrapper ran for `146301.052 ms`; the structured journey ran for
`145186.157 ms`. Authenticated Codex probes took approximately 13.8–14.3
seconds per repository. The ignored evidence tree is about 2.1 GiB, including
three isolated release installations and generated documents. The Volicord
documents were approximately 97 MB per Markdown output and 148 MB per HTML
output; the small and medium fixture documents were substantially smaller.
Peak memory was not measured.

## Privacy and external transmission

Authenticated Codex probes sent the bounded temporary Project ID and requested
the installed `project_health` result; they did not ask the model to read or
transmit repository source. All three probes completed and selected the
Volicord MCP tool.

Background semantic configuration was explicitly enabled for one bounded file
per Project against `v11-unavailable-provider`. Exact Guarded requests exposed
the provider, model, purpose, source scope, filter result, byte count, revision,
expiration, and fingerprint. The production adapter reported
`provider_unavailable`; every manifest entry remained `not_transmitted` with
zero transmitted bytes. Canonical, structural, document, portable, and
recovery operations remained local.

## Acceptance results

| Acceptance scenario | V11 conclusion |
| --- | --- |
| A, P — install, connection, fresh-service boundary | `passed`: clean install, direct MCP, authenticated Codex, and legacy exclusion passed in all three targets. |
| B–E — inventory, structural, polyglot understanding | `passed`: each repository exposed honest source-grounded capability and scoped parser degradation. This does not replace Phase 8's broader language/fallback dogfood. |
| F, L — Inquiry, Decision, applicability/reuse | `passed`: the research-gated promoted Question received one exact current-host response and produced the integrated Decision used downstream. |
| G — Candidate boundary | `passed`: submission, read-only inspection, research gating, explicit promotion, and promoted disposition were observed. |
| H — ordinary work and Checkpoint | `passed`: ordinary work remained unguarded and explicit Handoff Checkpoints were source-grounded. |
| I — new-session Recall | `passed`: fresh-process Recall restored the integrated Decision and Handoff next step without mutation. |
| J — portable clone and conflict | `passed`: import/bind preserved identity and semantic Decision divergence was inspected and explicitly branched. |
| K — correction, supersession, deletion | `passed`: all three mutation classes succeeded on integrated records. |
| M — Guarded effect | `passed`: exact fields, Source linkage, no-dispatch-before-confirmation, terminal cleanup, single use, truthful outcome, and no silent retry held. |
| N — viewer/document projection | `passed`: all eight requested artifacts per repository carried grounding metadata and left canonical bundles unchanged. |
| O — degraded recovery | `passed`: parser and index degradation/recovery were truthful and provider unavailability preserved unaffected local functionality. |
| Q — provider privacy | `passed`: Project opt-in and exact source scope were inspectable; unavailable-provider execution transmitted no source. |

The final V11 acceptance condition is passed. No repository failure is hidden by
another repository, no required external step is environment-blocked, and the
maintained structured result evaluates `phase_8_ready = true`.

## Known limits

- V11 is the entry gate, not Phase 8 itself. Repeated dogfood, narrative
  usefulness, broader first-structural-language/fallback coverage, and actual
  user learning/resumption quality remain Phase 8 work.
- The intentionally unavailable provider validates privacy, confirmation,
  failure, and local degradation behavior; it does not qualify a commercial
  provider, successful external semantic result, vendor retention, or deletion.
- Integrated V11 directly covered denial and consumed-preparation cleanup;
  expiry cleanup is composed from the exact final aggregate's production
  public-Host test at the same validated HEAD.
- Volicord document and evidence sizes are large. Output usefulness, latency,
  size ceilings, accessibility, and bounded resource behavior need observation
  during Phase 8 rather than being inferred from successful publication.
- The run does not qualify concurrent clients, abrupt power loss, hostile path
  races, non-Linux hosts, or peak-memory ceilings.

## Recommended implementation choice

Retain the current production responsibility boundaries and enter Phase 8
repeated dogfood. Use Phase 8 to evaluate real comprehension, document size and
narrative quality, larger and more varied repositories, fallback behavior, and
longer-duration resource use. Do not treat this one V11 rehearsal as Phase 8
completion or as Phase 9 cutover evidence.

## Rejected alternatives and reasons

- Seeding Candidate, Question, Decision, conflict, or Guarded outcome through
  validation-only storage calls was rejected because it would not prove the
  installed public journey.
- Flattening truthful provider/parser `partial` or unavailable internal states
  into an unqualified subsystem success was rejected; the step passed only
  because the required degradation contract was observed.
- Reusing a denied, expired, or consumed provider preparation was rejected by
  the public Host boundary.
- Treating the unavailable provider as successful transmission was rejected;
  the recorded outcome remains `provider_unavailable` and `not_transmitted`.
- Repeating the official rehearsal after its successful completion was rejected
  by the one-run evidence boundary.

## Reusable primitive decision

`reference_only` for production. The Python harness remains maintained external
validation orchestration and does not own product semantics. The self-authored
fixtures remain reusable validation inputs.

## Decision revisit trigger status

No accepted Q1–Q13 Decision revisit trigger is active. The run supports the
accepted Candidate research boundary, staged Inquiry, local-first/provider
separation, portable conflict handling, Checkpoint/Recall, document grounding,
and Guarded effect contract. The document-size observation is a Phase 8 quality
limit, not yet evidence that Q5 portability or accessibility is infeasible.

## Follow-up work

1. Begin Phase 8 repeated dogfood using this exact validated candidate and V11
   result as the entry evidence.
2. Measure document usefulness/size, repository diversity, fallback behavior,
   resumption quality, and sustained resource use during repeated real work.
3. Keep Phase 9 cutover closed until its separately owned conditions pass.

## Artifacts

- Exact final aggregate:
  `rebuild/.local/validation/20260814T170301.578433Z-final-qxhz9229/summary.json`,
  SHA-256 `85775064da154cf7f0bc77297fd5ca82add83112675a8120631b70c5a909f4a8`.
- Clean official preflight:
  `rebuild/.local/validation/20260814T172306.608730Z-v11-official-preflight-remediation-conclusions-y8f6jixp/result.json`,
  SHA-256 `71ad10ccff33cb14bca6a61fbc2a25faca92bf4694e00c5680834bc2be4d8acf`.
- Focused official wrapper:
  `rebuild/.local/validation/20260814T172338.951173Z-v11-official-run-remediation-conclusions-k2zlbc67/result.json`,
  SHA-256 `7f483093492f2627e1d219f46169f6a36dc077e0c044369ed8e88fffa0ad6ea5`.
- Structured official result:
  `rebuild/.local/v11/20260815-official-f64ee3eb/result.json`, SHA-256
  `32e52ba0c3b9308577671d763293755231da6d340a0e2da90b6a14fc524c0c1a`.
- Maintained inputs: this report, `harness.py`, fixture
  `v11-polyglot-medium`, reused fixture `v01-python`, and
  `rebuild/validation/shared/fixture-manifest.json`.

Raw child logs, installed binaries, cloned repositories, Runtime Homes,
generated documents, and copied authentication material remain ignored local
artifacts and are not maintained source.
