# Conformance Fixtures Reference

## What this document helps you do

Use this reference to look up the three-layer boundary for Harness conformance material: documentation checks, active state-assertion behavior examples, and future runtime conformance. It explains what future conformance will prove, the active Kernel Smoke, MVP-1 user-loop, security/capability, and artifact/evidence behavior examples, exact future fixture body shape, future runner execution behavior, fixture assertion semantics, current-phase status, and the boundary to the future fixture catalog.

This is a lookup document for conformance authors, implementers, and maintainers. It is not an operator procedure; use [Operations And Conformance Reference](operations-and-conformance.md) for operator entrypoints and the `harness conformance run` overview.

This is reference documentation for future conformance work. The current repository is documentation-only and contains no runnable Harness Server conformance tests; current phase and handoff status are tracked in [Implementation Overview](../build/implementation-overview.md#documentation-acceptance-status).

## Read this when

- You are writing or reviewing the future fixture-based conformance design.
- You need the exact fixture body fields, fixture shorthand boundary, `ToolEnvelope` expansion convention, or runner isolation behavior.
- You need fixture assertion modes for state, events, artifacts, projections, errors, validators, close blockers, and redaction effects.
- You need the active Kernel Smoke, MVP-1 User Work Loop, security/capability, or artifact/evidence behavior examples, or the boundary between those examples and the future fixture catalog.

## Before you read

Use [Operations And Conformance Reference](operations-and-conformance.md#conformance-run) for the conformance run entrypoint, suite-selection overview, docs-maintenance profile boundary, and operator procedures. Use [MVP API](api/mvp-api.md) and [API Schema Core](api/schema-core.md) for public request/response schemas, [Storage](storage.md) for storage layout and seed-loader owner values, [Core Model Reference](core-model.md) for state transition and stable event semantics, [Projection And Templates Reference](projection-and-templates.md) for projection freshness, [Design Quality Policies](design-quality-policies.md) for policy validator behavior, and [Agent Integration Reference](agent-integration.md) for connector conformance overview.

## Main idea

Today this document is a future conformance design, not a set of runnable tests. It defines behavior-example IDs and required behavior for later implementation planning; it does not create fixture files, runner code, generated outputs, runtime state, or a runnable Harness Server conformance suite. Do not create actual fixture files from these examples during the documentation-only phase.

Keep three layers separate:

- Documentation checks are read-only editorial checks over Markdown docs: link integrity, terminology consistency, stage boundaries, security wording, user-language checks, owner-boundary drift, and English/Korean parity. They may report Markdown drift, but they do not execute fixture actions, append `task_events`, create artifacts, refresh projections, create QA or acceptance state, affect close readiness, create implementation readiness, or create runtime results.
- MVP behavior examples are compact design examples for Engineering Checkpoint and MVP-1. They describe expected behavior but are not executable fixtures yet and are not generated runtime artifacts.
- Runtime conformance is future Harness Server implementation work. It applies to implemented Core/API/storage/surface behavior and is judged by executable fixtures and state assertions, not documentation prose. Only after server implementation and fixture materialization will exact-shape fixtures run through Core or operator entrypoints and produce runtime pass/fail results.

The core model and small MVP behavior examples stay in this file. Detailed later scenarios stay in [Future Fixtures](../later/future-fixtures.md). This keeps Engineering Checkpoint Kernel Smoke and MVP-1 user-facing value understandable without making later catalog coverage look like an early implementation requirement.

After implementation begins, conformance will prove Harness behavior with executable fixtures. A passing runtime fixture will drive a Core or operator action and compare captured Core/API or operator results against structured expectations.

Assertion authority is layered:

- Prose scenario descriptions, comments, rendered Markdown, Journey Card prose, status text, close report prose, and agent summaries are explanatory only.
- Captured Core state, `task_events`, validator results, returned primary errors, and structured tool-specific blocker fields are authoritative for fixture pass/fail.
- Artifact reference, owner-link, `sha256`, `size_bytes`, `content_type`, `redaction_state`, relation owner, retention, availability, and file-integrity assertions are authoritative where the scenario depends on artifacts or evidence bytes.
- Projection output may be checked for freshness, source-state-version display, readability, and availability when projection support is in scope, but renderer output must not replace Core state, satisfy evidence, authorize writes, close work, accept results, accept risk, or become the source of conformance truth. Engineering Checkpoint does not require projection assertions beyond an empty or "no projection requirement" field.

## Reference scope

This document owns:

- conformance fixture body shape
- fixture shorthand boundary for the active Engineering Checkpoint / MVP-1 path
- `ToolEnvelope` expansion convention for examples
- isolated fixture execution behavior for test hygiene, which is not an `isolated` security guarantee
- fixture assertion semantics and comparison modes
- suite catalog metadata boundaries
- future fixture profiles by behavior proved, the reduced Engineering Checkpoint / MVP-1 behavior examples, and the reduced Kernel Smoke authoring queue
- current-phase status and the boundary between runtime conformance and docs-maintenance checks
- links to the future-oriented catalog without making its scenarios Engineering Checkpoint or MVP-1 requirements

## Not covered here

This reference does not own operator command procedures, docs-maintenance reporting, public MCP schemas, SQLite DDL, projection template bodies, policy contracts, or the compact future scenario inventory. Those remain with their owning Reference documents. Suite metadata, examples, and catalog rows here do not add fixture-body fields, public request fields, storage rows, projection kinds, or runtime implementation readiness.

## Conformance Navigation Map

| If you are looking for... | Go to |
|---|---|
| The exact fixture body fields | [Conformance Fixture Format](#conformance-fixture-format) |
| How a runner loads, seeds, executes, captures, and compares | [Conformance Execution](#conformance-execution) |
| Default comparison modes for `expected_state`, `expected_events`, `expected_artifacts`, `expected_projection`, and `expected_error` | [Fixture Assertion Semantics](#fixture-assertion-semantics) |
| Active state-assertion behavior examples | [Kernel Smoke Behavior Examples](#engineering-checkpoint-behavior-examples), [MVP-1 User Work Loop Behavior Examples](#mvp-1-user-work-loop-behavior-examples), [Security And Capability Behavior Examples](#security-and-capability-behavior-examples), and [Artifact And Evidence Behavior Examples](#artifact-and-evidence-behavior-examples) |
| Suite intent and authoring order | [Conformance staging](operations-and-conformance.md#conformance-staging), [Kernel Smoke Authoring Queue](#kernel-smoke-authoring-queue), and [Future Fixtures: Fixture Suites](../later/future-fixtures.md#fixture-suites) |
| Core model and current-phase boundary | [Core Conformance Model](#core-conformance-model) and [Fixture Current-Phase Status](#fixture-current-phase-status) |
| Future scenario inventory by concern | [Future Fixtures](../later/future-fixtures.md) |

## Core Conformance Model

The core conformance model defines what future runtime conformance proves and where assertion authority lives. A passing fixture proves behavior by driving one Core or operator action and comparing captured structured results with fixture expectations. It does not prove behavior by matching prose, generated Markdown, Journey Card text, status prose, close prose, or agent summaries.

Assertion types remain deliberately small:

- State assertions compare Core-owned records, `task_events`, validator results, returned primary errors, structured blockers, owner refs, and state-version behavior.
- Artifact assertions compare registered artifact identity, owner links, `sha256`, `size_bytes`, `content_type`, `redaction_state`, relation owner, retention class, availability, and file-integrity facts where the scenario depends on evidence bytes.
- Projection assertions compare freshness, enqueue or job status, source-state-version display, readability, and availability only when projection support is in scope. They never replace Core state or satisfy authority, evidence, close, acceptance, or risk judgments.
- Error assertions compare the API-owned primary `ErrorCode` and optional details according to public schema precedence.

State assertions answer "what did Core own after the action?" Artifact assertions answer "what evidence bytes or metadata were safely registered and linked?" Projection assertions answer "is a derived readable view current, stale, available, failed, or queued?" These are separate assertion locations, and projection output must not substitute for state or artifact proof.

## Fixture Profiles By Proven Behavior

Fixture profiles are grouped by the behavior they prove, not by how polished the rendered output is. The profile name does not add fixture-body fields, does not require a renderer to be authoritative, and does not imply fixture files exist in this documentation-only repository.

The hardened local reference target is an umbrella target reached through Assurance Profile and Operations Profile. It is not a fifth fixture profile and must not be used as a suite name.

| Profile | Stage name | Behavior proved | Out of scope for that profile |
|---|---|---|---|
| Engineering Checkpoint fixtures, with Kernel Smoke as the authoring label | Engineering Checkpoint | The first executable authority loop: no-active-Task status, owner-valid setup/intake creating one active Task, active Change Unit requirement, in-scope/out-of-scope `prepare_write`, dry-run and replay behavior, single-use Write Authorization, `record_run` consumption and invalid-authorization blockers, minimal artifact metadata, evidence summary, close blockers, residual-risk visibility, and honest cooperative/detective guarantee display. | Ordinary natural-language intake quality, full user-loop judgment UX, full Evidence Manifest, projection renderer support, final-acceptance or residual-risk acceptance success semantics, Manual QA, detached verification, export/recover, release handoff, full conformance runner, broad future catalog coverage, hosted connector registry, cross-surface orchestration, preventive guard expansion, and broad operations. |
| MVP-1 User Work Loop fixtures | MVP-1 User Work Loop | Ordinary requests become tracked work without Harness vocabulary; focused user judgment, status next safe action, non-substitution boundaries for broad approval, sensitive approval, final acceptance, residual-risk acceptance, evidence, and detached verification are visible through Core-owned state and structured responses. | Full agency assurance hardening, detached verification independence, full Manual QA matrix, stewardship policy suite, full TDD/module/interface/domain-language catalogs, full feedback-loop audits, export/recover, release handoff, broad connector ecosystem, hosted connector registry, cross-surface orchestration, and automation beyond the MVP-1 user-value path. |
| Assurance Profile fixtures | Assurance Profile | User-owned judgment, sensitive-action Approval, Write Authorization, Manual QA, verification, final acceptance, residual-risk acceptance, stewardship, design-quality, context-hygiene, TDD, and feedback-loop boundaries stay separate and fixture-proven through Core records. | Operator recovery/export completeness, release handoff, broad operations coverage, dashboard/hosted workflow UI, broad connector automation, and unproven preventive or isolated guarantee claims. |
| Operations Profile / promoted Roadmap fixtures | Operations Profile and Roadmap | Export/recover, artifact integrity, release handoff, operator readiness, reconcile, broader conformance coverage, and any promoted future higher guarantee level or automation profile. | Any stronger security, isolation, preventive guard, browser-capture, remote/shared MCP, or automation claim until owner docs define the mechanism and fixtures prove the covered behavior. |

## Active Behavior Examples

These behavior examples are the active future-authoring target for Engineering Checkpoint and MVP-1. They are not executable fixtures yet, not generated runtime artifacts, and not current pass/fail criteria. When materialized, each row must become an exact-shape fixture that drives one Core or operator action and asserts structured results in `expected_state`, `expected_events`, `expected_artifacts`, `expected_projection`, and `expected_error`. A fixture must not pass because rendered Markdown, status prose, close prose, or a user-facing card looks plausible.

<a id="engineering-checkpoint-behavior-examples"></a>

### Kernel Smoke Behavior Examples

Kernel Smoke is the narrow authoring label for the first executable authority loop. The `KS-*` IDs below are future fixture candidates, not current fixture files. "Intake" here means the owner-valid setup/intake path that creates one active Task for the smoke; ordinary natural-language intake quality remains MVP-1 coverage.

| Example ID | Action path | Required structured assertion |
|---|---|---|
| `KS-no-active-task-status` | `harness.status` | With no active Task, the structured response reports no active Task or registered-idle state; it creates no Task, `task_events`, Write Authorization, Run, artifact, projection job, close state, or replay row. |
| `KS-intake-creates-active-task` | owner-valid setup/intake path | Exactly one active Task row and current-task pointer exist after the action; setup/intake may append only owner-promoted stable events and creates no Write Authorization, Run, artifact, evidence, close, or acceptance state. |
| `KS-change-unit-required-for-product-write` | `harness.prepare_write` | A product-write intent without an active compatible Change Unit or scoped boundary is blocked with structured scope-required state or error; no Write Authorization, Run, artifact, projection job, or state-authorizing side effect is created. |
| `KS-out-of-scope-intended-path-blocks` | `harness.prepare_write` | An intended path, tool, or operation outside the active scope is blocked with `SCOPE_VIOLATION` or owner-equivalent blocker/error; current scope state is unchanged and no Write Authorization, Run, artifact, projection job, or replay row is committed. |
| `KS-prepare-write-allowed-creates-active-authorization` | `harness.prepare_write` | A compatible non-dry-run in-scope request returns `decision=allowed`, no primary error, and exactly one durable active Write Authorization tied to Task, scope/Change Unit, intended operation, guarantee level, `basis_state_version`, and `consumed_by_run_id=null`. |
| `KS-prepare-write-dry-run-no-authorization` | `harness.prepare_write` with `dry_run=true` | A dry-run allowed result may return diagnostics or `would_create` effects, but it creates no Write Authorization, current record, `task_events` row, artifact, projection job, or `tool_invocations` replay row, and does not reserve the `idempotency_key`. |
| `KS-prepare-write-replay-original-response` | committed `harness.prepare_write` replay | Reusing the same `idempotency_key` and canonical request hash returns the original response and original `write_authorization_ref`; exactly one authorization and replay row remain, with no duplicate event, artifact, projection job, or state-version increment. |
| `KS-record-run-consumes-authorization` | `harness.record_run` | A compatible product-write Run commits once, records observed work, links `consumed_by_run_id` or owner-equivalent relation, marks the Write Authorization consumed, and does not treat chat/tool output as authority. |
| `KS-consumed-authorization-reuse-blocked` | `harness.record_run` | Reusing a consumed authorization is blocked with `WRITE_AUTHORIZATION_INVALID` and `authorization_reason=consumed` or owner-equivalent detail; the original Run/consumption relation remains unchanged and no second Run or evidence state is committed. |
| `KS-missing-authorization-blocks-record-run` | `harness.record_run` | A product-write Run without a required authorization is blocked with `WRITE_AUTHORIZATION_REQUIRED` or owner-equivalent blocker/error; no Run, consumption, completion evidence, artifact link, or projection job is committed. |
| `KS-stale-authorization-blocks-record-run` | `harness.record_run` | A Write Authorization whose `basis_state_version`, scope, intended path, expiry, or compatibility basis is stale is blocked with `WRITE_AUTHORIZATION_INVALID` or owner-equivalent stale detail; it is not consumed and no product-write Run or completion evidence is committed. |
| `KS-artifact-registration-hash-redaction` | artifact registration owner path or `harness.record_run` with artifact refs | Registered artifact refs include owner relation, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, retention/availability metadata, and safe file-integrity facts; raw secret or forbidden payload bytes are not stored as evidence. |
| `KS-evidence-summary-partial-sufficient` | evidence-summary owner path, `harness.record_run`, or `harness.status` | Minimal `evidence_summary` state is computed from owner records and artifact refs: missing/incomplete refs remain `partial`, compatible current refs may become `sufficient`, and the assertion lands in Core-owned evidence state rather than rendered prose. |
| `KS-close-blocked-missing-evidence` | narrow `harness.close_task` smoke | Required missing or insufficient evidence returns structured close blockers such as `EVIDENCE_INSUFFICIENT` or `ARTIFACT_MISSING`; the Task remains not closed, no acceptance/risk state is created, and prose cannot satisfy the missing ref. |
| `KS-close-blocked-unresolved-user-judgment` | narrow `harness.close_task` smoke | An unresolved required user judgment returns structured blockers with `required_judgment_kind` and related judgment refs; the Task remains not closed and broad approval text is not recorded as the missing judgment. |
| `KS-close-shows-residual-risk-before-acceptance` | `harness.status` or narrow `harness.close_task` smoke | Close-relevant residual risk appears in structured residual-risk visibility or blocker state before any successful close; no residual-risk acceptance, final acceptance, or detached verification state is fabricated. |
| `KS-cooperative-guarantee-no-preventive-claim` | `harness.status`, `harness.prepare_write`, or profile read | The reference `capability_profile` reports cooperative/detective limits with `pre_tool_blocking_supported=false`, `isolation_supported=false`, and no `guarantee_level=preventive` or `isolated` claim; required stronger claims lower the display or block with `CAPABILITY_INSUFFICIENT`. |

<a id="mvp-1-user-work-loop-behavior-examples"></a>

### MVP-1 User Work Loop Behavior Examples

MVP-1 behavior examples describe user-visible Harness value without growing into the broad assurance or operations catalog. If future fixtures materialize these examples, they may use exactly `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, and `harness.close_task` where those methods are active for the stage. A separate `harness.next` fixture belongs to later/compatibility material.

| Example ID | Required behavior assertion |
|---|---|
| `MVP1-ordinary-natural-language-intake` | Ordinary user language starts or resumes tracked work without requiring "Harness," `Task`, `Change Unit`, `Decision Packet`, or another startup phrase. The fixture asserts Task/work-shape state, optional scope candidates, and no write authority from the request alone. |
| `MVP1-focused-judgment-request` | When user-owned judgment is required, Core records or returns one focused user judgment request with `judgment_kind`, `presentation`, options/consequences, related refs, and a locale-derived display label; it does not dump a broad questionnaire or choose for the user. |
| `MVP1-status-next-safe-action` | `harness.status.next_actions` is derived from current Core state and returns the next safe action with blocker/evidence/judgment refs; reading status creates no events, authorizations, artifacts, acceptance, risk acceptance, or close state. |
| `MVP1-broad-approval-not-product-judgment` | Broad phrases such as "go ahead" or "looks good" do not satisfy a required `judgment_kind=product_decision`; the pending judgment and affected write/close blocker remain until a compatible user judgment record exists. |
| `MVP1-sensitive-approval-not-product-decision` | A compatible `judgment_kind=sensitive_approval` grants only the named sensitive-action permission and does not satisfy product, technical, scope, final acceptance, residual-risk acceptance, or Write Authorization requirements. |
| `MVP1-final-acceptance-not-evidence` | A `judgment_kind=final_acceptance` record may satisfy final acceptance when required, but it does not create evidence, mark `evidence_summary.status=sufficient`, repair missing `ArtifactRef` metadata, or replace verification/QA records. |
| `MVP1-residual-risk-acceptance-not-detached-verification` | `judgment_kind=residual_risk_acceptance` records explicit acceptance of a visible named risk; it does not create detached verification, Eval results, Manual QA, final acceptance, or evidence sufficiency. |
| `MVP1-close-result-explains-blockers` | Blocked `harness.close_task` returns structured blockers for missing evidence, unresolved judgment, final acceptance, residual-risk visibility/acceptance, or capability gaps, plus owner refs and next safe action; prose-only close text cannot prove or clear blockers. |

<a id="security-and-capability-behavior-examples"></a>

### Security And Capability Behavior Examples

Security and capability examples prove honest local capability display and unavailable-path behavior. They do not create stronger guarantees by naming them.

| Example ID | Required behavior assertion |
|---|---|
| `SEC-mcp-unavailable-no-authority` | When MCP/Core is unavailable, the response uses the API-owned unavailable error/diagnostic path and creates no Task state, Write Authorization, evidence, close readiness, user judgment, approval, acceptance, or risk state. |
| `SEC-cooperative-hold-instruction` | A cooperative surface can hold by instruction or return a structured blocker when Core authority is unavailable or incompatible, but the captured state/profile still reports cooperative or detective behavior and no pre-tool enforcement fact. |
| `SEC-detective-changed-path-mismatch` | When changed-path detection finds a path mismatch after work, the mismatch is captured as structured state, finding, blocker, stale evidence, or violation/audit according to the owner path; it does not retroactively authorize the write or count as successful evidence. |
| `SEC-unsupported-capability-lowers-guarantee` | If requested behavior depends on unsupported `capability_profile` fields, the result lowers `guarantee_level` or blocks with `CAPABILITY_INSUFFICIENT` or owner-equivalent reason; product writes do not proceed silently. |
| `SEC-stronger-guarantee-requires-profile-proof` | `preventive` or `isolated` display is allowed only when the connected profile names the covered operation and fixture-proof basis; otherwise the state/response remains cooperative/detective or blocked, and no stronger guarantee is recorded. |

<a id="artifact-and-evidence-behavior-examples"></a>

### Artifact And Evidence Behavior Examples

Artifact examples prove registered bytes and metadata, not report wording. They apply where the active stage uses artifact refs or evidence summaries; broader export non-leakage remains later/profile catalog material.

| Example ID | Required behavior assertion |
|---|---|
| `ART-staged-file-hash-success` | A file staged through the approved artifact path registers an `ArtifactRef` with owner relation, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, retention/availability metadata, and file-integrity match. |
| `ART-absolute-path-rejected` | An arbitrary absolute path outside the approved staging or runtime artifact boundary is rejected with a structured error/blocker; no artifact row, artifact link, evidence summary, or projection freshness update trusts that path. |
| `ART-raw-secret-blocked-secret-omitted` | Raw secret or forbidden sensitive payload bytes are blocked or represented only by `redaction_state=secret_omitted` or `blocked` metadata; fixtures assert safe metadata and downstream state effects without asserting the secret value. |
| `ART-missing-artifact-evidence-stale` | When a required artifact ref points to missing bytes or unavailable storage, affected evidence becomes `stale` or `blocked`, close remains blocked when that evidence is required, and Markdown evidence lists cannot repair it. |
| `ART-hash-mismatch-evidence-projection-stale` | A registered artifact whose stored bytes no longer match `sha256` marks affected evidence stale/blocked and any dependent projection freshness stale or failed; Core state is not rewritten by the projection or report. |

### Later/Profile Fixture Boundary

Detailed clarification catalogs, full Evidence Manifest cases, detached verification, Manual QA matrices, export non-leakage, browser QA capture, full operations recovery/export, broad connector conformance, preventive guard expansion, and isolated security profiles remain later/profile or Roadmap material unless an owner promotes a narrower fixture with stage impact and proof expectations. Listing a family in [Future Fixtures](../later/future-fixtures.md) does not make it an Engineering Checkpoint or MVP-1 requirement.

## Conformance Fixture Format

Future runtime conformance is fixture-based after Harness Server implementation and fixture materialization. A behavior-example table is not enough; each materialized test fixture must drive an action and assert state, events, artifacts, projection status when relevant, and errors.

Each fixture must include this shape:

```yaml
scenario_id: string
initial_state: object
input: object
action: string
expected_state: object
expected_events: object[]
expected_artifacts: object[]
expected_projection: object
expected_error: object | null
```

Fixture shape summary: suite metadata can group fixtures, but the fixture body keeps one exact action-and-expectation shape for future executable conformance. The YAML block above is the contract summary.

Future fixture files and suite catalogs may carry metadata outside the fixture body. The fixture body itself uses only the fields above so conformance runners can compare behavior consistently. Do not add fixture-body fields for suite delivery stage, assertion mode, docs-maintenance result, prose status, or authoring notes; those belong in suite catalog metadata, docs-maintenance reports, or surrounding documentation.

Fixture body type notation follows the API [Schema notation convention](api/schema-core.md#schema-notation-convention). All top-level fixture body fields above are required. Use `{}` or `[]` when the fixture intentionally supplies an empty object, object map, or array; omitting a required top-level field is an invalid fixture body, not "not asserted." For Engineering Checkpoint checks, `expected_projection` may be `{}` or an explicit no-requirement assertion because projection rendering is not a Engineering Checkpoint exit criterion.

For an MCP tool action, future executable fixture `input` is the tool's public request payload as defined by the API docs. The runner must validate `input` against the request schema for `action`, including `envelope: ToolEnvelope` when that schema requires it. Examples in this document may omit `ToolEnvelope` only under this envelope-expansion convention: before validation, canonicalization, request hashing, or Core execution, the runner supplies a deterministic valid envelope from `initial_state`, suite defaults, and fixture metadata. The expanded request is what Core receives. This convention does not add fixture fields, change the fixture body shape, or create an alternate request schema.

Fixture shorthand is not a second API. In the main Engineering Checkpoint / MVP-1 path, shorthand may compact only `initial_state` seeding or suite catalog metadata while preserving owner-defined records and public schemas. Public mutations must use the documented public request branch for the selected `action` under `input` after any `ToolEnvelope` expansion. Later-profile shorthand details belong in [Future Fixtures: Later-Profile Fixture Shorthand Notes](../later/future-fixtures.md#later-profile-fixture-shorthand-notes) and are not active requirements for Engineering Checkpoint or MVP-1.

Future executable fixtures that seed `write_authorizations` must produce valid stored rows. Each seeded authorization row must include `basis_state_version` explicitly, or the runner must derive it from the seeded affected-scope state version for the row's Task before inserting into `state.sqlite`. This is a storage-loader derivation rule only; it does not add fixture top-level fields or change the fixture body shape. Partial `expected_state.write_authorization` assertions may omit `basis_state_version` unless the fixture is testing idempotent replay, stale detection, expiry, or audit behavior. `basis_state_version` is the `decision=allowed` basis, not the resulting `ToolResponseBase.state_version`. Fixture loaders must not seed `blocked`, `approval_required`, `decision_required`, or `state_conflict` outcomes as `write_authorizations` rows; those outcomes use response decisions, blockers, validator findings, or errors.

Suite catalog metadata is not passed to Core and is not part of a fixture body. It can group exact-shape fixtures by suite, delivery stage, and tags:

```yaml
suite: agency
earliest_delivery_stage: "Assurance Profile"
tags: [decision-gate, residual-risk, autonomy-boundary]
fixtures:
  - AGENCY-user-judgment-required-before-product-tradeoff-write
  - AGENCY-residual-risk-visible-before-acceptance
```

Runners may use this metadata to choose, order, or report suites. Core receives only the action and public `input` after any documented envelope expansion; metadata must not change seed expansion, fixture comparison semantics, tool request schemas, or expected owner records.

## Conformance Execution

Future `harness conformance run` will execute fixtures through the same Core entrypoints used by MCP tools and operator commands. It must not assert behavior by inspecting prose output alone.

Future runtime fixture execution semantics:

1. Load fixture YAML files and validate the exact fixture body shape.
2. Create a fresh fixture-only runtime home and temporary Product Repository for the fixture, unless the fixture explicitly targets an existing read-only sample. This fixture isolation is test hygiene for deterministic comparison; it is not an `isolated` guarantee level, OS sandboxing, permission isolation, or tamper-proof storage claim. The runner must not reuse the developer's real Harness Runtime Home or Product Repository for state-changing fixture execution.
3. Seed `registry.sqlite`, `project.yaml`, `state.sqlite`, artifact files, projection files when the fixture requires them, and connector manifests from `initial_state`.
4. Execute `action` through Core. MCP tool actions use the public request schema; after any documented `ToolEnvelope` expansion, fixture `input` must be the same request payload a surface would send to that MCP tool. Operator actions such as `projection_refresh`, `doctor_surface`, `recover`, and `artifacts_check` use the operator semantics in [Operations And Conformance Reference](operations-and-conformance.md).
5. Capture resulting state summaries, appended owner events, validator results when emitted, artifact registry/file integrity, projection job status when relevant, reconcile items when relevant, and returned error code.
6. Compare the captured results with `expected_state`, `expected_events`, `expected_artifacts`, `expected_projection`, and `expected_error`; empty expected sections mean the fixture asserts no relevant effect for that section.
7. Report fixture id, pass/fail, observed state summary, observed events, artifact integrity result, projection freshness, and error comparison.

Runner sequence summary: the numbered sequence above is the contract summary. A future runner loads an exact fixture body, seeds a fixture-only runtime home, executes through Core, compares state/events/artifacts/projection/errors, and emits a report.

When a fixture action includes `expected_state_version`, the runner compares it according to the Core-resolved primary Task, not only `ToolEnvelope.task_id`. Primary Task resolution order is tool-specific `task_id`, `ToolEnvelope.task_id`, then active Task resolution. Task-scoped actions compare against the seeded or Core-resolved primary Task State Version; project-scoped actions with no resolved primary Task compare against the Project State Version. Captured response, `EventRef.state_version`, and `task_events.state_version` values are compared as resulting affected-scope versions. Read-only fixtures may assert the unchanged version for the primary read scope. This clarifies comparison semantics without changing fixture body shape.

A stale `expected_state_version` fixture is a stale-authority test, not only a concurrent-write test. Exact idempotent replay is the exception: when a committed replay row exists and the canonical request hash matches, the fixture should assert the original committed response is returned and no current state-version freshness check is re-run. When no replay row exists and a state-changing action conflicts before commit, the fixture should assert that no current records changed, no `task_events` were appended, no artifacts were registered, no projection jobs were enqueued, and no `tool_invocations` replay row was created for the conflicting request unless an owner document explicitly defines a different recovery action. When the same key is reused with a changed canonical request hash, the fixture should assert `STATE_CONFLICT`, preserved original replay row, and no merged artifacts, events, projection jobs, response fields, or owner relations. For `dry_run=true`, fixtures should assert that diagnostics or `would_create` effects are returned without current records, `task_events`, artifacts, consumable Write Authorizations, projection jobs, or `tool_invocations` replay rows, and that the key is not reserved for later non-dry-run use. Replayed `prepare_write` must not create a duplicate authorization; replayed `record_run` must not consume authorization twice.

Fixture execution should be deterministic. Network access, wall-clock-sensitive expiry, and external tool output must be stubbed or represented as seeded fixture inputs unless a suite explicitly declares itself an integration smoke.

Fixture isolation is part of the pass condition. A fixture may seed files into its temporary Product Repository and runtime home, execute one Core or operator action there, and compare the captured result. This does not upgrade the product guarantee level. The fixture must not depend on existing local runtime records, generated operational files, or prose reports from a previous run.

Seed validation happens before action execution, and captured-state validation happens after action execution. Both sides of the comparison use owner-defined state loaders and value sets rather than fixture-local string labels.

Conformance runners must seed and inspect JSON `TEXT` fields through the same Core storage loaders used by MCP tools and operator commands. A fixture with malformed JSON or schema-incompatible JSON in `initial_state` must surface invalid state, or a repairable state issue when the fixture action is a recovery path and safe reconstruction is possible. The runner must not skip shape validation by treating JSON fields as opaque strings, and this expectation does not change the fixture body shape.

Conformance runners must also seed and inspect status-like `TEXT` fields through the owner-bound hardening map in [Storage](storage.md#canonical-enum-hardening). For the main Engineering Checkpoint / MVP-1 path, fixture seed loaders validate only the owner values actually present in the active stage's seeded records, and artifact/ref enum assertions use the API [stage-specific active value sets](api/schema-core.md#stage-specific-active-value-sets). Examples include registry/project surface guarantee, Run kind/status, Write Authorization status/guarantee, Approval status when that owner path is active, minimal evidence summary coverage/status when evidence support is active, residual-risk visibility/status when risk visibility is active, projection job kind/status when projection assertions are in scope, and current Task or Change Unit status when those owner records are used. Full Evidence Manifest status is later/profile-gated. Later-profile status fields stay with promoted owner docs and the future catalog until those profiles are active. Unknown status values remain invalid unless a scenario explicitly tests recovery from invalid state; expected-state status assertions compare captured owner values, not prose labels.

## Fixture Assertion Semantics

Fixture assertion modes are runner defaults or suite catalog metadata. They are not Core input, are not passed to MCP tools, and must not add fields to the fixture body. The fixture body remains exactly `scenario_id`, `initial_state`, `input`, `action`, `expected_state`, `expected_events`, `expected_artifacts`, `expected_projection`, and `expected_error`.

Within partial assertion objects, omission means "not asserted." A listed field with value `null` asserts that the captured field is present and equals JSON `null`. A listed array value `[]` asserts a present empty array. A listed object-map value `{}` asserts a present empty map when the owner schema says that field is a map. For structured objects under `partial_deep`, fixture authors should list at least one child field unless they are deliberately asserting only that the object exists.

These omission rules are assertion rules only. They do not make omitted fields valid in public MCP `input`; fixture `input` still validates against the owning public request schema after any documented envelope expansion.

Default comparison modes:

| Fixture field | Default assertion mode |
|---|---|
| `expected_state` | `partial_deep`; listed fields must match recursively and unlisted fields are not asserted. Suite metadata may set `expected_state: exact`. |
| `expected_events` | `contains_ordered` over the stable-catalog projection of captured `task_events`; listed stable events must appear in ascending `task_events.event_seq` order, with unrelated stable events allowed before, between, or after them. Suite metadata may set `expected_events: exact`. |
| `expected_artifacts` | `contains_by_identity`; each listed artifact must match a registered artifact with the same `artifact_id` and `kind`, then any other listed artifact fields are matched recursively. |
| `expected_projection` | `partial_by_kind`; each listed projection kind must satisfy the listed status assertion or partial object assertion for that kind. |
| `expected_error` | `expected_error: null` asserts that the action returned no error. When `expected_error` is an object, `expected_error.code` is required and matched exactly against the primary API `ErrorCode` in `ToolError.code`, meaning `ToolResponseBase.errors[0].code` when the response has errors, selected by API-owned [Primary Error Code Precedence](api/errors.md#primary-error-code-precedence). It must not match an arbitrary secondary error, validator finding code, policy finding code, or local diagnostic label. `expected_error.details` is optional; when omitted, no details fields are asserted. When `details` is present, it is matched with `partial_deep` unless suite metadata sets `expected_error.details: exact`. |

Because `expected_events` defaults to `contains_ordered`, `expected_events: []` means the fixture requires no specific stable events; it does not by itself assert that the captured stable-event stream is empty. To assert no stable events, suite metadata must set `expected_events: exact` for that fixture or suite. Similarly, `expected_artifacts: []` and `expected_projection: {}` assert no required artifact or projection entries under their default modes; they do not ban captured artifacts or projection observations unless compatible exact-mode metadata says so.

`expected_events` comparisons are over the [Core Model Stable Event Catalog](core-model.md#stable-event-catalog) projection of captured `task_events`. API tool detail/audit event lists do not expand this set. Non-catalog detail or local-audit events captured in `task_events` must not make a normal staged-delivery fixture fail. When suite metadata sets `expected_events: exact`, exactness applies to the stable-event projection of the captured stream unless a future Roadmap/local suite explicitly opts into implementation-specific detail-event assertions. Validator IDs, Core check names, projection status shorthands, fixture shorthand labels, and scenario catalog IDs are not event names. Prose examples may mention non-catalog event names as illustrative or future extension ideas, but executable staged-delivery fixtures must not require them until the Core Model event catalog promotes them.

Conformance runners order captured `task_events` by `event_seq`. `state_version`, `created_at`, and `event_id` are not tie-breakers for `expected_events` ordering.

Fixture authors should use `VALIDATOR_FAILED` as `expected_error.code` only when API precedence selects the generic validator fallback; a more specific typed blocker such as `EVIDENCE_INSUFFICIENT`, `QA_REQUIRED`, `PROJECTION_STALE`, or `ARTIFACT_MISSING` remains primary when it applies.

`CloseTaskResponse.blockers[].code` is also an API `ErrorCode` value. Policy-specific or validator-specific finding codes belong under `expected_state.validators`, validator finding assertions, or equivalent expected validator output, not in `expected_error.code` or close blocker `code`. Fixtures that exercise blocked close must assert the structured blockers returned by Core, such as `CloseTaskResponse.blockers` or the captured equivalent under `expected_state.close_blockers`; matching report prose, Journey Card text, status text, or agent summaries alone cannot prove a close blocker.

Validator assertions nested under `expected_state.validators` are keyed by validator ID. Each listed validator ID must exist in the captured validator results and match the listed fields partially; unlisted validator IDs and unlisted validator fields are not asserted.

When fixtures assert design-quality impact, all relevant validator findings should remain visible under `expected_state.validators`, while fixtures assert the merged impact class, routed action, gate, write-blocker, close-blocker, waiver, or user judgment outcome produced by the policy-owned [Severity Composition Rule](design-quality-policies.md#severity-composition-rule) and [Active MVP impact defaults](design-quality-policies.md#active-mvp-impact-defaults). Fixtures must not add policy schemas, invent new action values, suppress lower-severity findings merely because a stronger merged blocker is also present, or treat advisory/later catalog findings as MVP blockers.

Core check and precondition assertions nested under `expected_state.checks` are keyed by check/precondition name. These entries are compared against captured Core check output, blocked reasons, response summaries, or equivalent runner-observed check status. They are not validator IDs and must not be nested under `expected_state.validators` unless [API Schema Core](api/schema-core.md#validatorresult), [API Schema Later](api/schema-later.md#validatorresult-stable-ids), or [Storage](storage.md) explicitly promotes that ID to a stable `ValidatorResult`.

`expected_state.checks.projection_freshness` asserts the Core mechanical projection freshness check. `expected_state.validators.context_hygiene_check` asserts the stable ValidatorResult for higher-level context hygiene; that validator may consider projection freshness, but it is not the fixture assertion location for the mechanical check itself.

Fixtures that cover `secret_omitted` or `blocked` artifacts should assert the committed artifact `redaction_state` under `expected_artifacts` and the downstream state or display effect under the owning assertion location: evidence or QA state under `expected_state`, verification outcome under Eval-related state or error assertions, projection freshness/display availability under `expected_projection` or `expected_state.checks.projection_freshness`, and export or Release Handoff behavior through the existing fixture assertions captured from the operator action. Fixtures must not assert the omitted secret or PII value.

Artifact redaction, blocked-input, integrity, and export non-leakage scenario families are future catalog inventory. See [Future Fixtures: Artifact Redaction And Export Non-Leakage Catalog Entries](../later/future-fixtures.md#artifact-redaction-and-export-non-leakage-catalog-entries).

Allowed `expected_projection` status assertions:

| Assertion | Meaning |
|---|---|
| `enqueued` | A refresh job or equivalent projection outbox entry for the projection kind is pending after the action. |
| `current` | The projection kind is current for the committed state version and managed hash. |
| `stale` | The projection kind is stale because state, evidence, or managed content moved ahead of the rendered projection. |
| `failed` | The latest applicable projection refresh for the kind failed. |
| `skipped` | The latest applicable projection job for the kind was skipped, for example because it was superseded or blocked by managed-block drift. |
| `stale_or_enqueued` | Either `stale` or `enqueued` is acceptable. Use this when the scenario proves projection invalidation or enqueueing and the runner may observe either side of the refresh boundary. |
| `stale_or_failed` | Either `stale` or `failed` is acceptable. Use this when a render failure may be surfaced as failed freshness or as stale freshness with a failed job. |

Projection shorthand such as `TASK: stale_or_enqueued` is a scalar status assertion for the `TASK` projection kind. Object form may assert additional captured projection fields while still using `partial_by_kind`, for example `TASK: {status: current}`. These assertion operators are fixture-comparison semantics, not new projection DDL or API enum values unless the owning schema documents define them.

Projection assertions compare projection freshness, enqueue status, source-state-version display, and related job facts. They do not compare rendered Markdown as canonical state, and they do not let a failed render roll back or rewrite the captured Core state and events.

Suite catalogs may override assertion modes without changing fixtures:

```yaml
suite: core
assertion_modes:
  expected_state: exact
  expected_events: exact
  expected_error.details: exact
fixtures:
  - KS-no-active-task-status
```

Future conformance must prove behavior through captured Core state, `task_events`, validator results, artifact registry/file integrity, projection job or freshness state, returned error codes, and structured tool-specific blocker fields when applicable. Matching rendered Markdown, Journey Card prose, status prose, close report prose, or agent prose alone cannot pass a fixture.

Fixture runners must use the same canonicalization rules as the reference implementation for `request_hash`, baseline `tree_hash`, and projection `managed_hash`. The detailed algorithms remain owned by [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [Storage](storage.md), and [Projection And Templates Reference](projection-and-templates.md) as applicable; conformance fixtures assert deterministic behavior without redefining those source-of-truth boundaries.

## Fixture Current-Phase Status

This repository is documentation-only. No executable fixture files, executable fixture catalog files, generated projections, runtime state, databases, or Harness Server conformance tests are being created by this documentation batch.

MVP behavior examples and fixture-authoring queues are future authoring plans. They become runnable only after documentation acceptance, a separate implementation-planning readiness decision, Harness Server implementation, and a deliberate fixture-materialization step. Documentation checks may report Markdown drift, but they are not runtime conformance and do not create Core fixture results.

## Catalog-Only Fixture Skeleton Guidance

Catalog skeleton guidance is for turning promoted future catalog families into exact-shape fixtures. It is not an executable fixture body, public request schema, DDL extension, runner design, or stage-exit requirement. Delivery-stage mapping belongs in suite catalog metadata, not in the fixture body. "Minimum seeded records" means owner records placed in `initial_state` after expansion and validation by Storage rules; public mutations still use the exact MCP request payload under `input`.

Future scenario-family inventory lives in [Future Fixtures](../later/future-fixtures.md).

## Kernel Smoke Authoring Queue

Use this queue as future authoring guidance for the [Kernel Smoke Behavior Examples](#engineering-checkpoint-behavior-examples). Kernel Smoke is the narrow authoring label for the first internal authority loop, not the first user-value slice, not a full conformance suite, and not the future fixture catalog. These rows do not imply executable fixture files already exist. They are a compact authoring order; a first implementation plan may materialize only the smallest subset that proves the one authority loop named by Build.

Kernel Smoke defaults to no projection requirement. A fixture may assert projection freshness or enqueue/failure facts only when the minimal owner path already produces those facts and they help prove the target behavior. Projection-template polish, detailed report templates, multiple projection kinds, browser QA capture, export/recover, reconcile, stewardship, context hygiene, full operations, and future guarantee-level fixtures stay outside Engineering Checkpoint unless owner docs later promote a specific narrow path.

In the table, `None` means the existing fixture field stays empty or `expected_error: null`; it is not a new sentinel value.

| Queue | Fixture candidate | Intended Core or operator action | Minimum seeded records | Required state/storage assertion | Expected stable event assertion | Expected artifact assertion | Expected projection assertion | Expected primary error |
|---|---|---|---|---|---|---|---|---|
| 1 | `KS-no-active-task-status` | `harness.status` | Registered local project with no active Task | Structured no-active-Task or registered-idle state; no Task, Write Authorization, Run, artifact, close, projection job, or replay row created | None | None | No projection requirement | None unless API owner uses an unavailable/no-active-task error |
| 2 | `KS-intake-creates-active-task` | Owner-valid setup/intake path | Registered local project, no active Task | Exactly one active Task row and current-task pointer; no Write Authorization, Run, artifact, evidence, close, or acceptance state | Owner-promoted task-created/setup events only | None | No projection requirement | None |
| 3 | `KS-change-unit-required-for-product-write` | `harness.prepare_write` | Active Task without compatible active Change Unit/scope boundary | Product-write intent blocked; no Write Authorization, Run, artifact, projection job, or replay row committed | No pre-commit rejection event unless promoted | None | No projection job for pre-commit rejection | `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, or owner-equivalent blocker/error |
| 4 | `KS-out-of-scope-intended-path-blocks` | `harness.prepare_write` | Active Task with scoped boundary excluding requested path/tool/operation | Out-of-scope intent blocked; scope state unchanged; no Write Authorization, Run, artifact, projection job, or replay row committed | No pre-commit rejection event unless promoted | None | No projection job for pre-commit rejection | `SCOPE_VIOLATION` or owner-equivalent blocker/error |
| 5 | `KS-prepare-write-allowed-creates-active-authorization` | `harness.prepare_write` | Active Task, compatible scope, compatible baseline if required, `dry_run=false` | `decision=allowed`; exactly one active Write Authorization with Task, scope, intended operation, guarantee, `basis_state_version`, `status=active`, and `consumed_by_run_id=null` | `prepare_write_allowed` / `write_authorization_created` only when promoted | None | No projection requirement unless owner path invalidates status projection | None |
| 6 | `KS-prepare-write-dry-run-no-authorization` | `harness.prepare_write` with `dry_run=true` | Same as allowed path, but dry-run | Would-allow diagnostics only; no current record, `task_events`, Write Authorization, artifact, projection job, or `tool_invocations` replay row | None | None | No projection job | None |
| 7 | `KS-prepare-write-replay-original-response` | Committed `harness.prepare_write` replay | Existing replay row and original active authorization | Original response and `write_authorization_ref` returned; no duplicate authorization, event, artifact, projection job, replay update, or state-version increment | None beyond the original committed events | None | No new projection job | None |
| 8 | `KS-record-run-consumes-authorization` | `harness.record_run` | Active Task, compatible scope, active compatible Write Authorization | One Run committed; authorization consumed once with `consumed_by_run_id` or owner-equivalent link; observed work recorded | `run_recorded` / `write_authorization_consumed` only when promoted | None unless the Run registers artifact refs | No projection requirement unless state change invalidates status projection | None |
| 9 | `KS-consumed-authorization-reuse-blocked` | `harness.record_run` | Compatible authorization already consumed by prior Run | Reuse blocked; original Run/authorization relation unchanged; no second Run, evidence, artifact link, or close state committed | No `write_authorization_consumed` event for the rejected attempt | None | No projection job for pre-commit rejection | `WRITE_AUTHORIZATION_INVALID` with `authorization_reason=consumed`, or owner-equivalent detail |
| 10 | `KS-missing-authorization-blocks-record-run` | `harness.record_run` | Active Task and product-write Run input with no required authorization | Missing authorization blocked; no Run, consumption, completion evidence, artifact link, projection job, or replay row committed | No `run_recorded` / `write_authorization_consumed` events | None | No projection job | `WRITE_AUTHORIZATION_REQUIRED` or owner-equivalent blocker/error |
| 11 | `KS-stale-authorization-blocks-record-run` | `harness.record_run` | Authorization whose basis, scope, path, expiry, or compatibility is stale | Stale authorization not consumed; no product-write Run, completion evidence, artifact link, or close state committed | No `write_authorization_consumed` event | None | No projection job for pre-commit rejection | `WRITE_AUTHORIZATION_INVALID` with stale detail, `STATE_CONFLICT`, or owner-equivalent blocker/error |
| 12 | `KS-artifact-registration-hash-redaction` | Artifact registration owner path or `harness.record_run` with artifact refs | Active Task/Run and staged artifact input | Artifact registry/storage rows include owner link, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, retention/availability, and file-integrity facts | Artifact-linked event only when promoted | Matching `ArtifactRef` and hash/redaction metadata; no raw secret bytes | No projection requirement | None |
| 13 | `KS-evidence-summary-partial-sufficient` | Evidence-summary owner path, `harness.record_run`, or `harness.status` | Active Task with partial and then compatible current artifact/evidence refs | `evidence_summary.status` remains `partial` for incomplete refs and becomes `sufficient` only from compatible current refs; assertion is in evidence state, not Markdown | Evidence-summary event only when promoted | Required refs linked when sufficient | No projection requirement | None |
| 14 | `KS-close-blocked-missing-evidence` | Narrow `harness.close_task` smoke | Active Task requiring evidence but missing compatible sufficient evidence | Structured close blocker for evidence/artifact gap; Task remains not closed; no acceptance or risk state created | `close_blocked` only when promoted | None | No projection requirement | `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING`, or owner-equivalent blocker/error |
| 15 | `KS-close-blocked-unresolved-user-judgment` | Narrow `harness.close_task` smoke | Active Task with required unresolved user judgment | Structured blocker includes `required_judgment_kind` and related refs; Task remains not closed and broad approval text is ignored | `close_blocked` only when promoted | None | No projection requirement | User-judgment blocker/error owned by API/Core |
| 16 | `KS-close-shows-residual-risk-before-acceptance` | `harness.status` or narrow `harness.close_task` smoke | Active Task with close-relevant residual risk and no compatible risk acceptance | Structured residual-risk visibility/blocker state appears before close; no final acceptance, residual-risk acceptance, or detached verification is fabricated | `close_blocked` only when promoted | None | No projection requirement | Residual-risk blocker/error when close is attempted |
| 17 | `KS-cooperative-guarantee-no-preventive-claim` | `harness.status`, profile read, or `harness.prepare_write` | Registered reference `capability_profile` for `surface_id=reference-local-mcp` | Profile/response reports cooperative or detective limits; `pre_tool_blocking_supported=false`, `isolation_supported=false`; no `preventive` or `isolated` guarantee recorded without proof | None | None | No projection requirement | `CAPABILITY_INSUFFICIENT` only when a stronger required capability is requested |

The queue above is intentionally small. Engineering Checkpoint does not require a full conformance suite, broad catalog family coverage, final-acceptance semantics, Manual QA, detached verification, export/recover, reconcile, stewardship, context hygiene, browser QA capture, or future guarantee-level checks.

## Future Fixtures

Scenario families have moved to [Future Fixtures](../later/future-fixtures.md) so the early reference stays focused on the core conformance model. That catalog contains compact future-oriented inventory for browser QA capture, cross-surface behavior, export non-leakage, context hygiene, reconcile, stewardship, full operations, advanced projection rendering, artifact redaction and integrity, and future guarantee-level checks.

Those catalog entries are design inventory only until a promoted owner path materializes exact-shape executable fixtures. They are not required for Engineering Checkpoint, do not expand MVP-1 by themselves, and do not count as runtime conformance while this repository remains documentation-only.

## Metrics Boundary

Long-term operational metrics are derived analytics, not staged-delivery-critical state or conformance requirements. Keep metrics such as approval turnaround, verification latency, projection stale duration, same-session guard frequency, and surface fallback rate in the [roadmap](../roadmap.md) as read-only diagnostics until a future version promotes them with owner docs, fixtures or a conformance target, fallback behavior, relevant redaction/retention policy, no projection-as-canonical dependency, and implementation ownership.
