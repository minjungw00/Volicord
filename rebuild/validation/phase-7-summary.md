# Phase 7 product-surface validation and V11 handoff state

- Exact-final production/test candidate:
  `4b1c87e31caec9ef88865467610c9ddc8a20c14e`; aggregate `succeeded` with four
  commands succeeded and zero failures
- V11 integrated result: `passed` with 54 of 54 required steps passed; Phase 8
  entry: `phase_8_ready = true`
- Scope: qualified local platform primitives, source-grounded Project
  projections and documents, repository-bound read-only Project discovery,
  Project-scoped provider privacy, Local Operations, exact Guarded confirmation,
  live local viewer, clean Linux installation, and discoverable high-level
  Codex/MCP integration
- Excluded claim: this summary does not claim Phase 8 dogfood quality, Phase 9
  cutover, or legacy deletion

## Maintained validation status

| Validation | Status | Maintained evidence | Boundary retained |
| --- | --- | --- | --- |
| V06 | passed | `projections/source-grounded-documents` | four source-grounded documents, read-only Project projections, explicit publication/adoption |
| V07 | passed | `privacy/local-only-boundary` | local-first utility, Project opt-in, truthful transmission and deletion state |
| V08 | passed | `linux-codex-integration` | authenticated live viewer HTTP, exact MCP schemas, authenticated Codex product-tool use, fresh repository provenance, recovery and preservation |
| V10 | passed | `local-platform-primitives` | qualified Linux process/filesystem/storage primitives without legacy API promotion |

Exact final sealed production/test candidate HEAD
`4b1c87e31caec9ef88865467610c9ddc8a20c14e`. Its four-command aggregate
`succeeded` with zero failures; summary SHA-256 is
`c80c05bbd0fbed8e0c787ef00e5d8fb4f690e7c6c0bfea86ad70a8875d637a9c`.
The same-session official V11 then passed all 54 required steps with no failed,
partial, unsupported, skipped, or environment-blocked status. Result SHA-256
is `1c40bcb8cead793b730e87bcb6569e5dae9ba0852e395f568feed4e95b35f1b4`.
All three authenticated Codex targets passed, the credential-retention audit
passed with zero auth-named files, credential-content matches, or scan errors,
the sanitized evidence archive was independently verified, and
`phase_8_ready = true` is recorded in
`end-to-end/multi-repository/report.md`.

The later documentation-only conclusion records that observed gate result. It
is not part of the exact-final candidate and does not require another final
aggregate.

## Phase 7 capability conclusion

The user surfaces have executable-boundary evidence. The real
viewer process uses its actual ephemeral loopback authority, serves
request-specific `overview`, `working`, and `deep` projections, reads current
state on each request, and exposes a browser-received request-authenticity value.
Authenticated same-origin requests route canonical memory correction, exact
Guarded response, and explicit document publication through Local Operations.
Missing/wrong authenticity, cross-origin, cross-site, and alternate-Host
requests are rejected with no canonical, Guarded, or filesystem side effect;
an untrusted Host cannot retrieve the page value, and the value is absent from
URLs and durable outputs. Stale, mismatched, reused, malformed, and unsupported
requests retain their existing rejection behavior.

The actual installed MCP server advertises 18 closed concrete schemas. The
`project_resolve` surface canonicalizes an absolute repository path and reads
the existing canonical binding without creating or revising a Project; it
returns either the matching Project and current binding identity/revision or an
explicit `not_found`. Host guidance requires this resolution before Recall in
a fresh repository-scoped session and keeps explicit initialization separate.
A maintained client-side interpreter constructs representative Recall,
Checkpoint, and Guarded calls from `tools/list`; missing and additional
arguments are rejected consistently. A bounded authenticated `codex exec` turn
selected and completed `volicord.project_health` using the advertised
`project_id` shape and received connected/healthy structured state.

Supported `repair derived-analysis` and `reindex` now have a two-Project CLI
journey. Controlled Project-owned corruption degrades visibly. Repository
changes before `repair` and `reindex` are read into new Repository and Analysis
Snapshots that reference the fresh canonical repository Source for each scan.
Portable state changes only by the required repository observation bookkeeping;
user Questions, Decisions, corrected Context Items, Checkpoints, supersession,
forgetting, and non-repository Sources remain unchanged. Earlier repository
Sources retain their historical basis, the unrelated Project remains unchanged,
and an unsupported canonical repair fails.

## Legacy exclusion audit

Cargo metadata places every reconstruction package below `rebuild/`. No package
depends on a legacy Volicord workflow or MCP crate. The clean installer and all
corrected probes use an explicit replacement Runtime Home. The bait legacy
sentinel remains byte- and timestamp-identical, and current derived recovery
uses only Project-scoped replacement-owned analysis directories.

## Accepted-Decision revisit triggers

No accepted Q1–Q13 Decision revisit trigger is active. The sanitized capsule
records `decision_revisit_trigger_assessment = reported_by_official_v11`, an
empty active-trigger list, and the assessed Decision-register identity. This
is official V11-owned evidence, not an independent documentation inference.
Phase 7 retains the
Project as the user-facing unit, local-first canonical authority, current-host
user Source provenance, semantic Decision correction/supersession distinction,
Candidate non-canonical lifecycle, explicit document adoption/publication,
Project-scoped provider consent, and exact Guarded-effect confirmation.

The Codex CLI's per-MCP-tool approval mode is an adapter/configuration fact. The
authenticated read-only probe uses a narrow isolated `approve` setting for
`project_health`; it does not broaden Volicord's Guarded product contract or
background-provider authority.

## Known limits and next boundary

- The environment-dependent Codex probe requires valid authentication, model
  service network access, and a CLI that supports bounded noninteractive mode;
  it is not part of deterministic workspace unit tests.
- The Linux journey does not qualify other operating systems, concurrent
  clients, abrupt power loss, hostile filesystem races, large-repository
  latency, accessibility, or long-duration resource ceilings.
- V06 narrative quality, V07 commercial-provider behavior, and V10 platform
  portability retain the known limits in their individual reports.
- Official V11 establishes entry eligibility for a fresh Phase 8 naturalistic
  Dogfood campaign. It does not establish dogfood quality or completion. Phase
  8 must still evaluate naturalistic activation, Question relevance, Decision
  comprehension, repeated use, interruption cost, document usefulness, size,
  latency, accessibility, and sustained resource behavior under its maintained
  plan.
- The unavailable production provider path proved exact Guarded confirmation,
  terminal cleanup, truthful no-transmission failure, and unaffected local
  operation. It did not qualify a commercial provider or successful external
  semantic result.
- The Volicord document outputs and ignored evidence were large; document
  usefulness, size, latency, accessibility, and sustained resource use remain
  Phase 8 observations rather than V11 conclusions.

## Maintained references

- `rebuild/validation/projections/source-grounded-documents/report.md`
- `rebuild/validation/privacy/local-only-boundary/report.md`
- `rebuild/validation/linux-codex-integration/report.md`
- `rebuild/validation/local-platform-primitives/report.md`
- `rebuild/validation/end-to-end/multi-repository/report.md`
- `rebuild/validation/shared/fixture-manifest.json`
- `rebuild/docs/design/validation-plan.md`
- `rebuild/docs/design/architecture.md`
- `rebuild/docs/design/failure-and-recovery.md`
- `rebuild/docs/design/cutover-plan.md`
