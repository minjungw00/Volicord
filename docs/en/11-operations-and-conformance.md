# Operations And Conformance

## Document Role

This document owns operator procedures, fixture-based conformance, and docs-maintenance smoke reporting for the harness: connect, doctor, serve MCP, projection refresh, reconcile, recover, export, artifact integrity, and conformance suites.

It does not own daily user workflow, MCP request/response schemas, SQLite DDL, or long-term analytics as MVP requirements.

## Operations Scope

Every operator entrypoint is a surface over the same Core rules used by the agent. Operator tools may diagnose, repair, export, or run fixtures, but they must not create a second state model.

Required MVP operator entrypoints:

```text
harness connect
harness doctor
harness serve mcp
harness projection refresh
harness reconcile
harness recover
harness export
harness artifacts check
harness conformance run
```

```mermaid
flowchart TD
  Core["Core rules and state authority"]
  Core --> Connect["harness connect"]
  Core --> Doctor["harness doctor"]
  Core --> Serve["harness serve mcp"]
  Core --> Refresh["harness projection refresh"]
  Core --> Reconcile["harness reconcile"]
  Core --> Recover["harness recover"]
  Core --> Export["harness export"]
  Core --> Artifacts["harness artifacts check"]
  Core --> Conformance["harness conformance run"]
  Connect --> Setup["link repo, runtime home, surface"]
  Doctor --> Readiness["report readiness, drift, repair options"]
  Serve --> MCP["expose MCP resources and tools through Core"]
  Refresh --> Projection["regenerate derived Markdown views"]
  Reconcile --> Decisions["turn edits or drift into explicit decisions"]
  Recover --> Repair["repair interrupted operational state"]
  Export --> Bundle["create review or archival bundle"]
  Artifacts --> Integrity["compare artifact records with files"]
  Conformance --> Fixtures["execute exact-shape fixtures"]
```

Exact command flags may vary by implementation, but the semantics below are required for the reference MVP.

## Conformance Staging

Conformance can run incrementally, but staged execution must not change the fixture body shape or reduce final MVP requirements.

Kernel Smoke is the first runnable conformance target, drawn from a selected smoke slice across MVP-0 through early MVP-3 capabilities. It should prove project and Task state, scoped Change Unit behavior, `prepare_write` allow/block behavior, durable Write Authorization creation, `record_run` authorization consumption, artifact and evidence manifest basics, minimal projection enqueue/current behavior, writes or runs blocked when write authority is missing, close blocked when evidence or decision requirements are missing, and basic Core fixture execution. Passing Kernel Smoke proves the first runnable kernel authority path; it does not claim final MVP conformance.

Agency-Hardened MVP is the final reference conformance target. It must add Decision Packet quality, sensitive approval lifecycle separation, residual-risk visibility before acceptance and close, detached verification guards, Manual QA, stewardship and context-hygiene validators, full feedback-loop checks, codebase stewardship coverage, projection/reconcile completeness, recover/export/artifact integrity behavior, later-boundary checks, and broader fixture coverage. Suite catalog metadata may map scenarios to the earliest MVP stage, but executable fixtures still assert through Core state, events, artifacts, projections, and errors.

Guard/freeze conformance for MVP asserts honest display and behavior at cooperative/detective levels: freeze requests can hold work, make the next action stricter, or cause `prepare_write` to block or hold when existing scope is incompatible; persistent owner-record changes must be asserted only when they happen through an existing Core state-changing path, Decision Packet route, or owner-record update path. Guard displays report whether the current path is cooperative or detective and what violations can only be detected after the fact. Preventive `T4` guard fixtures are v1-or-later unless a reference surface implements and proves pre-tool blocking for the covered operation.

Browser QA Capture conformance is a v1 priority candidate, not an MVP smoke requirement. Future fixtures should prove declared `T6 QA Capture` behavior only after capability profile fields, redaction and secret/PII handling, browser test environment, artifact retention, capture artifact mapping, and unsupported-surface fallback behavior are defined. MVP fixtures still prove Manual QA records, artifact refs, QA waiver behavior, and close blockers without requiring automated browser capture.

```mermaid
flowchart LR
  Kernel["Kernel Smoke<br/>first runnable authority path"] --> Harden["Agency-Hardened MVP<br/>final reference conformance"]
  Kernel --> K1["project and Task state"]
  Kernel --> K2["prepare_write and Write Authorization"]
  Kernel --> K3["artifact, evidence, projection basics"]
  Kernel --> K4["close and write blockers"]
  Harden --> H1["Decision Packet quality"]
  Harden --> H2["residual risk, verification, Manual QA"]
  Harden --> H3["stewardship and context hygiene"]
  Harden --> H4["recover, export, artifact integrity"]
  Harden --> H5["projection and reconcile completeness"]
```

## Docs-Maintenance Smoke Profile

A docs-maintenance smoke profile may be run by an operator or reviewed manually to catch drift in the documentation set. It is a read-only maintenance check over Markdown docs, not Core fixture conformance, a runtime validator, evidence, residual-risk acceptance, or a canonical state transition. It must not append `task_events`, create artifacts, refresh projections, create QA or acceptance state, affect close readiness, or claim runtime implementation readiness.

The [Authoring Guide](99-authoring-guide.md#docs-maintenance-conformance) owns the rule bodies, pass/warn/fail interpretation, and checklist. This document owns only the operator-maintenance expectation for reporting and entrypoint exposure.

Minimal operator wiring contract: when exposed through `harness conformance run` or another operator entrypoint, docs-maintenance is an explicitly selected docs-only profile, conventionally named `docs-maintenance`. Runtime conformance runs must not include it unless an operator selects that profile. Even when selected, report it separately from runtime Core fixture suites and do not count it toward runtime fixture pass/fail or implementation readiness. It must not affect Task state, MVP runtime validator IDs, projection freshness, QA, acceptance, close readiness, or any canonical state transition.

Console output or an ephemeral report from the docs-maintenance profile is the only output defined here. Generated operational report files require a future explicit implementation contract; this documentation batch does not define stored artifacts, projection jobs, DDL, or state records for this check.

Minimum report fields:

- profile name and documentation revision
- pass, warn, or fail per category
- affected file path and heading or anchor when available
- canonical owner doc and expected source section
- suggested fix class: update owner, replace duplicate with summary plus link, mirror translation, repair link, or add `TODO_DECISION` / `TODO_IMPLEMENT`
- statement that no canonical state transition was performed

Smoke categories should reference, not restate, the Authoring Guide checks: bilingual file and heading parity, broken cross references, owner-boundary drift, fixture/action schema drift, enum and event/error-code drift, stable `ValidatorResult` ID drift, `ProjectionKind` tier drift, glossary and source-of-truth phrasing drift, TODO rule compliance, and non-owner duplicate full-contract paragraphs.

```mermaid
flowchart LR
  Start["docs-maintenance smoke profile"] --> Check["check Markdown docs against Authoring Guide categories"]
  Check --> Report["report pass, warn, or fail per category"]
  Report --> Output["console output or ephemeral report"]
  Output --> Hold["no task_events, artifacts, projections, QA, acceptance, or close state"]
```

## Connect

`connect` links a Product Repository, Harness Runtime Home, and one reference agent surface.

Required behavior:

- identify the repository root
- register or reuse the local project
- create or validate static project configuration
- initialize per-project state and artifact storage
- register the reference surface and capability profile
- create or refresh connector-managed files through a manifest
- confirm MCP configuration can reach the harness server
- run a conformance smoke check or print the command to run it

```mermaid
sequenceDiagram
  participant Op as Operator
  participant Repo as Product Repository
  participant Runtime as Harness Runtime Home
  participant Surface as Reference Surface
  participant MCP as MCP Config
  participant Core as Core Smoke
  Op->>Repo: identify repository root
  Op->>Runtime: register or reuse project
  Runtime->>Runtime: initialize state and artifact storage
  Op->>Surface: register capability profile
  Op->>Repo: create or refresh connector-managed files via manifest
  Runtime->>MCP: confirm server reachability
  Op->>Core: run smoke or print command
```

Connect must report generated-file drift instead of overwriting human edits silently. Surface-specific generated file names belong in the surface cookbook.

## Doctor

`doctor` reports readiness, drift, and repair options.

Required categories:

| Category | Checks |
|---|---|
| project | registered project, repo root, static config validity |
| state | current state readability, JSON field parse and shape validity, locks, active Task consistency |
| MCP | server reachability, Core reachability, read resource availability, public tool availability |
| surface | capability profile, generated manifest, MCP config freshness, required MCP tool-call ability |
| artifacts | file existence, hash, size, redaction state, task/run or artifact-link relation |
| projections | queued jobs, freshness, managed hash drift, failed renders |
| reconcile | pending human edits, managed block drift, generated-file drift |
| validators/checks | required stable ValidatorResult-emitting validators, plus separately captured Core check/precondition categories |
| agency/stewardship/context | Decision Packet and decision gate readiness, Autonomy Boundary readiness, residual-risk visibility, codebase stewardship, context freshness |

```mermaid
flowchart TD
  Doctor["harness doctor"] --> Project["project"]
  Doctor --> State["state"]
  Doctor --> MCP["MCP"]
  Doctor --> Surface["surface"]
  Doctor --> Artifacts["artifacts"]
  Doctor --> Projections["projections"]
  Doctor --> Reconcile["reconcile"]
  Doctor --> Validators["validators/checks"]
  Doctor --> Agency["agency/stewardship/context"]
  State --> JSON["JSON TEXT parse and shape validity"]
  Projections --> Freshness["freshness and failed renders"]
  Validators --> Stable["stable ValidatorResult IDs and Core checks"]
```

Output levels:

```text
OK
WARN
FAIL
REPAIRABLE
MANUAL
```

Doctor must distinguish current state failures from projection stale or projection failed status.

State checks include JSON `TEXT` fields in `registry.sqlite` and `state.sqlite`. Malformed JSON is a state failure. Schema-incompatible JSON is a state failure; doctor may mark it `REPAIRABLE` only when Core can safely reconstruct the expected value from other canonical state or raw artifacts without inventing product judgment, otherwise it reports `FAIL` or `MANUAL`.

## Serve MCP

`serve mcp` starts or prints connection information for the local MCP server.

Required behavior:

- expose read resources without mutation
- expose public tools through Core, not shell shortcuts
- require state-changing calls to use Core conflict and idempotency behavior
- report the active project and connected surface profile
- fail clearly when the server cannot reach runtime state or artifact storage

```mermaid
flowchart TD
  Start["harness serve mcp"] --> Server["server can reach runtime state and artifact storage?"]
  Server -- "no" --> ServerFail["diagnostic<br/>MCP_SERVER_UNAVAILABLE<br/>no authoritative Core response"]
  Server -- "yes" --> Core["Core reachable for public tools"]
  Core --> Resources["read resources exposed without mutation"]
  Resources --> Surface["connected surface can use required MCP tools?"]
  Surface -- "yes" --> Ready["MCP server ready for this surface"]
  Surface -- "no" --> SurfaceFail["diagnostic<br/>SURFACE_MCP_UNAVAILABLE<br/>surface cannot use required MCP tools"]
```

If MCP is unavailable, operations must distinguish diagnostic condition `MCP_SERVER_UNAVAILABLE` from diagnostic condition `SURFACE_MCP_UNAVAILABLE`. These labels are not additional public `ErrorCode` values. When either condition is surfaced through `ToolError`, operations must use the API-owned error selection and details shape: `MCP_UNAVAILABLE` remains the stable public availability code, while surface-side availability or capability cases may use `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` with `details.mcp_unavailable_kind` according to context. With `MCP_SERVER_UNAVAILABLE`, a tool call cannot reach Core and no authoritative Core response is possible; the next action is server diagnosis or reconnect before any state-change claim. With `SURFACE_MCP_UNAVAILABLE`, Core or an operator can observe that the connected surface lacks usable MCP, has stale MCP configuration, or cannot call required MCP tools. Cooperative surfaces must hold product/runtime/code writes by instruction; stronger profiles may enforce the hold preventively or through isolation. Operations must still report the actual guarantee level.

## Projection Refresh

Projection refresh regenerates Product Repository Markdown from committed state records and artifact refs.

Required behavior:

- render only the latest projection version for a target
- preserve human-editable sections
- compare managed block hashes before overwrite
- create reconcile items for managed-block drift
- mark projection jobs `completed`, `failed`, `pending`, or `skipped`
- keep projection failure separate from Task result

Supported targets:

```text
one Task
all active Tasks
approval/run/evidence/eval/direct reports for a Task
design-quality projections when enabled
```

```mermaid
flowchart TD
  Target["select refresh target"] --> Latest["render latest projection version"]
  Latest --> Preserve["preserve human-editable sections"]
  Preserve --> Hash["compare managed block hash"]
  Hash -- "hash drift" --> Reconcile["create reconcile item"]
  Hash -- "matches" --> Write["write derived Markdown view"]
  Reconcile --> Skipped["mark job skipped or pending"]
  Write --> Completed["mark job completed"]
  Latest -- "render error" --> Failed["mark job failed"]
  Completed --> Separate["keep projection status separate from Task result"]
  Failed --> Separate
  Skipped --> Separate
```

For MVP, Decision Packet visibility is rendered through `TASK` projections, status/next responses, judgment-context resources, and decision-packet resources; Journey Card visibility is rendered through status, journey, next, and significant resume surfaces. Dedicated extension / appendix refresh targets for `DEC`, `DESIGN`, `EXPORT`, and persisted `JOURNEY-CARD` are optional when enabled, not required MVP smoke targets.

## Reconcile

Reconcile turns human-editable input or generated/managed drift into an explicit decision.

Targets:

- Task user notes and proposals
- managed block edits
- Domain Language proposals
- Module Map proposals
- Interface Contract proposals
- connector generated-file drift
- stale projection references that affect current work

Decision outcomes:

| Outcome | Meaning |
|---|---|
| merge | apply the proposal through Core and append state history |
| reject | leave canonical state unchanged and refresh projection if needed |
| convert_to_note | keep the content as a human note, not state |
| create_decision | turn the proposal into a pending user decision |
| defer | keep the reconcile item open |

```mermaid
flowchart TD
  Input["human edit or managed/generated drift"] --> Item["create reconcile item"]
  Item --> Review["review against canonical state and owner docs"]
  Review --> Merge["merge"]
  Review --> Reject["reject"]
  Review --> Note["convert_to_note"]
  Review --> Decision["create_decision"]
  Review --> Defer["defer"]
  Merge --> Core["apply through Core and append state history"]
  Reject --> Refresh["canonical state unchanged; refresh if needed"]
  Note --> Human["preserve as human note"]
  Decision --> Pending["pending user decision"]
  Defer --> Open["reconcile item remains open"]
```

Reconcile must not treat edited Markdown as canonical state by itself.

## Recover

Recover repairs interrupted or inconsistent operational state without rewriting history.

Required scenarios:

| Scenario | Recovery behavior |
|---|---|
| agent crash during write | commit a recovery Run with `runs.status=interrupted` and capture diff/log artifacts when possible; captured artifacts are recovery evidence, not proof of successful completion |
| stale approval baseline | expire or re-request approval when scope is affected |
| evaluator observes drift | mark verification blocked or evidence stale |
| artifact registry mismatch | rescan files, mark missing artifacts stale, preserve hashes |
| projection job failed | retry or mark failed and create reconcile guidance |
| managed Markdown edited | create reconcile item |
| malformed or schema-incompatible storage JSON | repair only if Core can reconstruct the expected shape from canonical state or raw artifacts; otherwise fail or require manual recovery |
| lock expired | append recovery event and release or reacquire according to lock policy |
| MCP unavailable | report diagnostic condition `MCP_SERVER_UNAVAILABLE` or `SURFACE_MCP_UNAVAILABLE`, keep product/runtime/code writes held, and give the next diagnosis or reconnect step |

```mermaid
flowchart TD
  Scenario["failure scenario"] --> Classify["classify recovery path"]
  Classify --> Interrupted["agent crash during write<br/>runs.status=interrupted"]
  Classify --> Baseline["stale approval baseline"]
  Classify --> Evidence["evaluator drift or artifact mismatch"]
  Classify --> Projection["projection job failed or managed Markdown edited"]
  Classify --> Storage["malformed or schema-incompatible storage JSON"]
  Classify --> Lock["lock expired"]
  Classify --> MCP["MCP unavailable"]
  Interrupted --> Event["append compensating event"]
  Baseline --> Approval["expire or re-request approval"]
  Evidence --> Stale["mark evidence or verification stale/blocked"]
  Projection --> Recon["retry, fail, or create reconcile guidance"]
  Storage --> Repair["repair only from canonical state or raw artifacts"]
  Lock --> Release["release or reacquire by policy"]
  MCP --> Hold["hold product/runtime/code writes and diagnose"]
```

Recovery may append compensating events. It must not silently delete evidence, rewrite event history, or make projections authoritative.

## Export

Export creates a review or archival bundle for a Task.

Required contents:

- export manifest with created time, task id, projection freshness, and redaction summary
- state snapshots for the Task and related records
- Decision Packets, user decisions, residual risks with accepted-risk metadata/refs, Journey Spine entries or continuity refs, and relevant Change Unit Autonomy Boundary summaries
- projection snapshots for relevant reports
- artifact references and included raw artifact files when allowed
- artifact integrity manifest
- redaction and omission notes for secrets, sensitive logs, and PII

```mermaid
flowchart TD
  Export["Task export bundle"] --> Manifest["export manifest"]
  Export --> State["state snapshots"]
  Export --> Decisions["Decision Packets and user decisions"]
  Export --> Risks["residual risks and accepted-risk refs"]
  Export --> Journey["Journey Spine or continuity refs"]
  Export --> Projections["projection snapshots"]
  Export --> Artifacts["artifact refs and allowed raw files"]
  Export --> Integrity["artifact integrity manifest"]
  Export --> Redaction["redaction and omission notes"]
```

Exported projection snapshots may have hashes, but that does not make the Markdown projection the canonical evidence. Raw evidence remains the artifact files and their registered refs.

## Artifact Integrity

Artifact integrity check compares artifact records with stored files.

Required checks:

- file exists
- hash matches
- size matches
- content type is known or explicitly `other`
- redaction state is valid
- task/run or artifact-link relation is valid
- linked state owner exists in the same Task scope as the artifact link, or `record_kind=projection` resolves to a completed same-Task `projection_jobs` row
- owner-link relation semantics are compatible with the artifact's kind, including artifacts whose kind is `bundle`, `manifest`, or `export_component`
- for projection artifact links, `artifact_links.record_id` must equal `projection_jobs.projection_job_id`; integrity validates that job/output identity through the same Task scope as the artifact link, `target_ref`, `status=completed`, and `output_path` or a documented projection ref instead of looking for a separate `projections` table. Project-level projection jobs are not project-scoped artifact links in the current MVP.
- bundle, manifest, and export-component artifacts are validated through their artifact row and owner links; the check must not look for nonexistent `verification_bundle` or `export` state tables
- retention class is valid
- projection or evidence refs resolve

```mermaid
flowchart TD
  Check["artifact integrity check"] --> Record["artifact record exists and links resolve"]
  Check --> File["stored file exists"]
  Check --> Hash["hash and size match"]
  Check --> Type["content type and redaction state valid"]
  Check --> Relation["task/run or artifact-link relation valid"]
  Check --> Retention["retention class valid"]
  Check --> ProjectionRefs["projection or evidence refs resolve"]
  Record --> Consequence["mark related evidence, projection freshness, or close readiness stale/blocked on failure"]
  File --> Consequence
  Hash --> Consequence
  Type --> Consequence
  Relation --> Consequence
  Retention --> Consequence
  ProjectionRefs --> Consequence
```

Failures should mark related evidence, projection freshness, or close readiness stale/blocked according to Core rules. Missing artifacts are not fixed by editing Markdown reports.

## Conformance Fixture Format

Conformance is fixture-based. A scenario table is not enough; each test fixture must drive an action and assert state, events, artifacts, projections, and errors.

Each fixture must include this shape:

```yaml
scenario_id: string
initial_state: object
input: object
action: string
expected_state: object
expected_events: list
expected_artifacts: list
expected_projection: object
expected_error: object | null
```

```mermaid
classDiagram
  class FixtureBody {
    scenario_id
    initial_state
    input
    action
    expected_state
    expected_events
    expected_artifacts
    expected_projection
    expected_error
  }
  class SuiteCatalogMetadata {
    suite
    earliest_mvp_stage
    tags
    assertion_modes
    fixtures
  }
  SuiteCatalogMetadata ..> FixtureBody : groups exact-shape fixtures
```

Fixture files and suite catalogs may carry metadata outside the fixture body. The fixture body itself uses only the fields above so conformance runners can compare behavior consistently.

For an MCP tool action, executable fixture `input` is the tool's public request payload as defined by the API docs. The runner must validate `input` against the request schema for `action`, including `envelope: ToolEnvelope` when that schema requires it. Examples in this document may omit `ToolEnvelope` only under this envelope-expansion convention: before validation, canonicalization, request hashing, or Core execution, the runner supplies a deterministic valid envelope from `initial_state`, suite defaults, and fixture metadata. The expanded request is what Core receives. This convention does not add fixture fields, change the fixture body shape, or create an alternate request schema.

Fixture shorthand is intentionally narrow. It is allowed for `initial_state` seeding, suite catalog metadata, and documented seed-loader expansion of compact examples such as `owner_records`, `stewardship_findings`, or feedback-loop shorthand. Executable fixture files must map that shorthand to owner records, validator runs, residual risks, or other records owned by DDL/API docs. The shorthand must not create a second API or state model. Public mutation must not be encoded as scenario-only shorthand inside `input`; fixtures must use the public request branch for `record_run`, `record_eval`, `record_manual_qa`, `record_user_decision`, or else seed owner records in `initial_state` when the scenario is about preexisting state. `close_task` fixture `input` is only `CloseTaskRequest` after any documented envelope expansion; evidence profiles, changed paths, artifact refs, acceptance-criteria support, self-check summaries, and Manual QA records must be seeded in `initial_state` or recorded by a preceding public mutation fixture. `StewardshipImpactSummary` assertions are derived display, not canonical current records, and should appear under `expected_state.derived` or projection assertions. `owner_records.feedback_loops` seeds canonical `feedback_loops` rows. Bare `FBL-*` values in example fields such as `feedback_loop_refs` map to `StateRecordRef { record_kind: feedback_loop, record_id: ... }` in executable fixtures. Fixture bodies that exercise public mutation instead of seeded state must express definition changes as `FeedbackLoopUpdate` under `record_run.payload.shaping_update.feedback_loop_updates`, execution/status changes under `evidence_updates.feedback_loop_updates`, or Manual QA execution through `record_manual_qa.feedback_loop_ref`. When an example shows only `feedback_loop_id` and `status`, the fixture runner must derive or supply the remaining required `feedback_loops` storage fields from the surrounding Task, Change Unit, selected-loop, and evidence shorthand before inserting or building the corresponding `FeedbackLoopUpdate`. Accepted residual risk in fixture shorthand is state on seeded `residual_risk` records, not a standalone accepted-risk record. When fixture examples use bare `RISK-*` values in risk-ref arrays such as `visible_refs`, `accepted_refs`, `not_visible_refs`, `unaccepted_refs`, or `residual_risk_refs`, executable fixtures must map them to `StateRecordRef { record_kind: residual_risk, record_id: ... }`. These bare IDs are fixture shorthand only, not DDL/API fields. Executable MVP fixtures must not require standalone `ARISK-*` records.

Executable fixtures that seed `write_authorizations` must produce valid stored rows. Each seeded authorization row must include `basis_state_version` explicitly, or the runner must derive it from the seeded affected-scope state version for the row's Task before inserting into `state.sqlite`. This is a storage-loader derivation rule only; it does not add fixture top-level fields or change the fixture body shape. Partial `expected_state.write_authorization` assertions may omit `basis_state_version` unless the fixture is testing idempotent replay, stale detection, expiry, or audit behavior. `basis_state_version` is the allow-decision basis, not the resulting `ToolResponseBase.state_version`.

Suite catalog metadata is not passed to Core and is not part of a fixture body. It can group exact-shape fixtures by suite, stage, and tags:

```yaml
suite: agency
earliest_mvp_stage: MVP-4
tags: [decision-gate, residual-risk, autonomy-boundary]
fixtures:
  - AGENCY-decision-packet-required-before-product-tradeoff-write
  - AGENCY-residual-risk-visible-before-acceptance
```

## Conformance Execution

`harness conformance run` executes fixtures through the same Core entrypoints used by MCP tools and operator commands. It must not assert behavior by inspecting prose output alone.

MVP execution semantics:

1. Load fixture YAML files and validate the exact fixture body shape.
2. Create an isolated runtime home and temporary Product Repository for the fixture, unless the fixture explicitly targets an existing read-only sample.
3. Seed `registry.sqlite`, `project.yaml`, `state.sqlite`, artifact files, projection files, and connector manifests from `initial_state`.
4. Execute `action` through Core. MCP tool actions use the public request schema; after any documented `ToolEnvelope` expansion, fixture `input` must be the same request payload a surface would send to that MCP tool. Operator actions such as `projection_refresh`, `doctor_surface`, `recover`, and `artifacts_check` use the operator semantics in this document.
5. Capture resulting state summaries, appended `task_events`, validator results, artifact registry/file integrity, projection job status, reconcile items, and returned error code.
6. Compare the captured results with `expected_state`, `expected_events`, `expected_artifacts`, `expected_projection`, and `expected_error`.
7. Report fixture id, pass/fail, observed state summary, observed events, artifact integrity result, projection freshness, and error comparison.

```mermaid
sequenceDiagram
  participant Runner as Conformance Runner
  participant Fixture as Fixture YAML
  participant Runtime as Isolated Runtime
  participant Core as Core Entrypoint
  participant Report as Conformance Report
  Runner->>Fixture: load and validate exact body shape
  Runner->>Runtime: seed state, artifacts, projections, manifests
  Runner->>Core: execute action
  Core-->>Runner: state, events, artifacts, projection status, error
  Runner->>Runner: compare expected_state/events/artifacts/projection/error
  Runner->>Report: emit fixture id, pass/fail, observed summaries
```

When a fixture action includes `expected_state_version`, the runner compares it according to the Core-resolved primary Task, not only `ToolEnvelope.task_id`. Task-scoped actions compare against the seeded or Core-resolved primary Task State Version; project-scoped actions with no resolved primary Task compare against the Project State Version. Captured response and `task_events` `state_version` values are compared as resulting affected-scope versions. Read-only fixtures may assert the unchanged version for the primary read scope. This clarifies comparison semantics without changing fixture body shape.

Fixture execution should be deterministic. Network access, wall-clock-sensitive expiry, and external tool output must be stubbed or represented as seeded fixture inputs unless a suite explicitly declares itself an integration smoke.

Conformance runners must seed and inspect JSON `TEXT` fields through the same Core storage loaders used by MCP tools and operator commands. A fixture with malformed JSON or schema-incompatible JSON in `initial_state` must surface invalid state, or a repairable state issue when the fixture action is a recovery path and safe reconstruction is possible. The runner must not skip shape validation by treating JSON fields as opaque strings, and this expectation does not change the fixture body shape.

Conformance runners must also seed and inspect status-like `TEXT` fields through the owner-bound hardening map in [Reference MVP](06-reference-mvp.md#canonical-enum-hardening). Fixture seed loaders must validate both compact shorthand and expanded rows for fields with promoted owner values, including `project_surfaces.guarantee_level` when seeding registry/project surface state, `runs.kind`, `runs.status`, `write_authorizations.status`, `write_authorizations.guarantee_level`, `approvals.status`, `evidence_manifests.status`, `residual_risks.visibility_status`, `feedback_loops.loop_kind`, `feedback_loops.status`, `tdd_traces.status`, `validator_runs.status`, `validator_runs.guarantee_level`, `projection_jobs.projection_kind`, `projection_jobs.status`, `connector_manifests.status`, `baselines.status`, `change_units.status`, `tool_invocations.status`, `decision_requests.status`, `residual_risks.status`, `task_spine_entries.status`, `change_unit_dependencies.status`, `shared_designs.status`, `reconcile_items.status`, `domain_terms.status`, `module_map_items.status`, and `interface_contracts.review_status`. For `decision_requests.status`, validation applies only when the optional `decision_requests` table is retained or a fixture seeds `decision_requests` rows; minimal MVP implementations may still omit the table. These promoted values are still owner-bound storage values, not scenario prose labels; for example, `runs.status: completed`, `runs.status: interrupted`, and `runs.status: violation` are valid only with the Reference MVP compatibility meanings for committed Runs, while `shared_designs.status: active` is a current design basis and not final acceptance or approval. Executable fixtures must not seed unknown status values unless the scenario explicitly tests recovery from invalid state; expected-state status assertions compare captured owner values, not prose labels.

## Fixture Assertion Semantics

Fixture assertion modes are runner defaults or suite catalog metadata. They are not Core input, are not passed to MCP tools, and must not add fields to the fixture body. The fixture body remains exactly `scenario_id`, `initial_state`, `input`, `action`, `expected_state`, `expected_events`, `expected_artifacts`, `expected_projection`, and `expected_error`.

Default comparison modes:

| Fixture field | Default assertion mode |
|---|---|
| `expected_state` | `partial_deep`; listed fields must match recursively and unlisted fields are not asserted. Suite metadata may set `expected_state: exact`. |
| `expected_events` | `contains_ordered` over the stable-catalog projection of captured `task_events`; listed stable events must appear in ascending `task_events.event_seq` order, with unrelated stable events allowed before, between, or after them. Suite metadata may set `expected_events: exact`. |
| `expected_artifacts` | `contains_by_identity`; each listed artifact must match a registered artifact with the same `artifact_id` and `kind`, then any other listed artifact fields are matched recursively. |
| `expected_projection` | `partial_by_kind`; each listed projection kind must satisfy the listed status assertion or partial object assertion for that kind. |
| `expected_error` | `expected_error: null` asserts that the action returned no error. When `expected_error` is an object, `expected_error.code` is required and matched exactly against the primary API `ErrorCode` in `ToolError.code`, meaning `ToolResponseBase.errors[0].code` when the response has errors, selected by API-owned [Primary Error Code Precedence](05-mcp-api-and-schemas.md#primary-error-code-precedence). It must not match an arbitrary secondary error, validator finding code, policy finding code, or local diagnostic label. `expected_error.details` is optional; when omitted, no details fields are asserted. When `details` is present, it is matched with `partial_deep` unless suite metadata sets `expected_error.details: exact`. |

```mermaid
flowchart TD
  Modes["runner defaults or suite catalog metadata"] --> State["expected_state<br/>partial_deep by default"]
  Modes --> Events["expected_events<br/>contains_ordered stable events"]
  Modes --> Artifacts["expected_artifacts<br/>contains_by_identity"]
  Modes --> Projection["expected_projection<br/>partial_by_kind"]
  Modes --> Error["expected_error<br/>primary ToolError.code exact"]
  Modes --> Boundary["comparison modes are not Core input, API enums, DDL, or fixture body fields"]
```

`expected_events` comparisons are over the [Kernel Stable Event Catalog](03-kernel-spec.md#stable-event-catalog) projection of captured `task_events`. API tool detail/audit event lists do not expand this set. Non-catalog detail or local-audit events captured in `task_events` must not make a normal MVP fixture fail. When suite metadata sets `expected_events: exact`, exactness applies to the stable-event projection of the captured stream unless a future non-MVP/local suite explicitly opts into implementation-specific detail-event assertions. Validator IDs, Core check names, projection status shorthands, fixture seed shorthand, and scenario catalog IDs are not event names. Prose examples may mention non-catalog event names as illustrative or future extension ideas, but executable MVP fixtures must not require them until the kernel catalog promotes them.

Conformance runners order captured `task_events` by `event_seq`. `state_version`, `created_at`, and `event_id` are not tie-breakers for `expected_events` ordering.

Fixture authors should use `VALIDATOR_FAILED` as `expected_error.code` only when API precedence selects the generic validator fallback; a more specific typed blocker such as `EVIDENCE_INSUFFICIENT`, `QA_REQUIRED`, `PROJECTION_STALE`, or `ARTIFACT_MISSING` remains primary when it applies.

`CloseTaskResponse.blockers[].code` is also an API `ErrorCode` value. Policy-specific or validator-specific finding codes belong under `expected_state.validators`, validator finding assertions, or equivalent expected validator output, not in `expected_error.code` or close blocker `code`.

Validator assertions nested under `expected_state.validators` are keyed by validator ID. Each listed validator ID must exist in the captured validator results and match the listed fields partially; unlisted validator IDs and unlisted validator fields are not asserted.

When fixtures assert design-quality severity, all relevant validator findings should remain visible under `expected_state.validators`, while fixtures assert the merged gate, write-blocker, close-blocker, waiver, or Decision Packet outcome produced by the policy-owned [Severity Composition Rule](08-design-quality-policy-pack.md#severity-composition-rule). Fixtures must not add policy schemas or suppress lower-severity findings merely because a stronger merged blocker is also present.

Core check and precondition assertions nested under `expected_state.checks` are keyed by check/precondition name. These entries are compared against captured Core check output, blocked reasons, response summaries, or equivalent runner-observed check status. They are not validator IDs and must not be nested under `expected_state.validators` unless the MCP API or Reference MVP explicitly promotes that ID to a stable ValidatorResult.

`expected_state.checks.projection_freshness` asserts the Core mechanical projection freshness check. `expected_state.validators.context_hygiene_check` asserts the stable ValidatorResult for higher-level context hygiene; that validator may consider projection freshness, but it is not the fixture assertion location for the mechanical check itself.

Absence of a nested field inside any `expected_*` value means "not asserted", not "expected null". Empty default-mode collections such as `expected_artifacts: []` or `expected_projection: {}` are valid and assert no required entries. `expected_events: []` asserts that no stable catalog events are required; it does not assert that no `task_events` rows were appended, because committed transitions may append non-stable detail or local-audit events. A suite that needs to assert no extra stable entries must use compatible exact-mode metadata outside the fixture body.

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

Suite catalogs may override assertion modes without changing fixtures:

```yaml
suite: core
assertion_modes:
  expected_state: exact
  expected_events: exact
  expected_error.details: exact
fixtures:
  - CORE-active-status-no-task
```

Conformance must prove behavior through captured Core state, `task_events`, validator results, artifact registry/file integrity, projection job or freshness state, and returned error codes. Matching rendered Markdown, Journey Card prose, status prose, or agent prose alone cannot pass a fixture.

Fixture runners must use the same canonicalization rules as the reference implementation for `request_hash`, baseline `tree_hash`, and projection `managed_hash`. The detailed algorithms remain owned by the MCP API, Reference MVP storage, and Document Projection docs; conformance fixtures assert deterministic behavior without redefining those source-of-truth boundaries.

## Agency, Stewardship, And Context Suites

Agency, stewardship, and context hygiene are MVP conformance suites. They test state behavior through Core entrypoints such as `prepare_write`, `request_user_decision`, `record_user_decision`, `record_manual_qa`, `close_task`, `next`, and operator actions that call Core. They must not pass by matching Journey Card, Decision Packet, residual-risk, or status prose.

Required suite responsibilities:

| Suite | Required behavior |
|---|---|
| agency | Blocking product judgment requires a compatible Decision Packet before affected write or close; decision request routing metadata is optional compatibility data and alone must not satisfy `decision_gate`; product trade-off writes are held; sensitive approval lifecycle keeps approval, Decision Packet, and Write Authorization distinct; AFK Autonomy Boundary stop conditions block public commitments; known close-relevant residual risk must be visible before any successful close; if no known close-relevant risk exists, `ResidualRiskSummary.status=none` satisfies residual-risk visibility; risk-accepted close additionally requires accepted Residual Risk refs whose risks were visible before acceptance; approval, QA, acceptance, and residual-risk acceptance remain distinct. |
| stewardship | Design-quality and codebase-stewardship validators affect `design_gate`, `decision_gate`, `qa_gate`, close blockers, and waiver eligibility through canonical owner records, refs, and policy-owned severity composition; public interface, module, domain, feedback-loop, TDD, Manual QA, and waiver checks do not duplicate schemas or DDL; Review Stage displays separate Spec Compliance Review from Code Quality / Stewardship Review without creating new authority. |
| context-hygiene | Current Task state, Journey refs, evidence refs, and freshness state are authoritative; stale PRDs, stale projections, closed issues, old design docs, and long logs are pull-only context until reconciled; stale context cannot authorize writes, close, acceptance, or current-state replacement. |

Status/next recommendations, including Role Lens recommendations, are fixture-observable only as read responses. Fixtures may assert `recommended_playbooks` when relevant, but must also prove no state event, gate satisfaction, projection enqueue, artifact, evidence, verification, QA, acceptance, residual-risk acceptance, close, or assurance upgrade resulted from the recommendation itself. If a recommendation or role lens implies product judgment, the expected behavior is a Decision Packet ref or Decision Packet request path, not a satisfied `decision_gate`. If it identifies validator, evidence, Manual QA, residual-risk, or release-handoff work, the expected behavior is a routed recommendation or candidate, not a committed owner record unless a later public mutation fixture records it through Core.

`browser-qa-candidate` recommendations are subject to the same read-only rule. A recommendation may name Browser QA Capture as useful for a `T6 QA Capture` surface, but the recommendation alone must not mutate state, enqueue projections, create artifacts, create or satisfy evidence, perform or record verification, record QA, waive QA or verification, accept residual risk, accept the result, close a Task, or upgrade assurance. Actual artifacts, Manual QA records, QA gate updates, Eval results, or close effects require a later public mutation through Core.

```mermaid
flowchart LR
  Suites["MVP conformance suites"] --> Agency["agency"]
  Suites --> Stewardship["stewardship"]
  Suites --> Context["context-hygiene"]
  Agency --> A1["Decision Packet and decision gate"]
  Agency --> A2["approval, residual risk, Autonomy Boundary"]
  Stewardship --> S1["design-quality validators and policy composition"]
  Stewardship --> S2["domain, module, interface, feedback loop"]
  Stewardship --> S3["two-stage review routing"]
  Context --> C1["current Task state and Journey refs"]
  Context --> C2["stale context is pull-only until reconciled"]
```

## Hardened MVP Fixture Coverage

The hardened evidence, verification, and connector rules should be covered by fixtures with the required shape. Suite catalogs may map scenario IDs to the earliest MVP stage where the behavior must be implemented, but stage metadata is not part of the fixture body.

```yaml
scenario_id: CORE-evidence-direct-docs-only-sufficient
initial_state:
  active_task:
    task_id: TASK-DOCS-001
    mode: direct
    lifecycle_phase: executing
    acceptance_criteria: ["AC-01 typo corrected"]
    gates:
      scope_gate: passed
      evidence_gate: sufficient
      verification_gate: not_required
  runs:
    - run_id: RUN-DOCS-001
      kind: direct
      status: completed
      summary: "Rendered Markdown heading and checked typo fix."
      observed_changes:
        changed_paths: ["docs/help.md"]
      artifact_refs: [ART-DIFF-001]
  evidence_manifests:
    - evidence_manifest_id: EM-DOCS-001
      status: sufficient
      criteria:
        AC-01:
          status: supported
          refs: [ART-DIFF-001]
      changed_files: ["docs/help.md"]
      supporting_refs: [RUN-DOCS-001, ART-DIFF-001]
  artifacts:
    - artifact_id: ART-DIFF-001
      kind: diff
input:
  task_id: TASK-DOCS-001
  intent: complete
  requested_close_reason: completed_self_checked
  user_note: "Self-check recorded in RUN-DOCS-001."
  superseded_by_task_id: null
action: close_task
expected_state:
  lifecycle_phase: completed
  result: passed
  close_reason: completed_self_checked
  assurance_level: self_checked
  gates:
    evidence_gate: sufficient
  residual_risk_summary:
    status: none
    close_relevant_count: 0
expected_events:
  - close_requested
  - task_closed
expected_artifacts:
  - artifact_id: ART-DIFF-001
    kind: diff
expected_projection:
  TASK: enqueued
expected_error: null
```

```yaml
scenario_id: CORE-evidence-work-ac-missing-blocks-close
initial_state:
  active_task:
    task_id: TASK-WORK-AC-001
    mode: work
    lifecycle_phase: verifying
    acceptance_criteria: ["AC-01 saves profile", "AC-02 shows validation error"]
    gates:
      scope_gate: passed
      approval_gate: not_required
      evidence_gate: partial
      verification_gate: pending
  evidence_manifests:
    - evidence_manifest_id: EM-WORK-AC-001
      status: partial
      criteria:
        AC-01:
          status: supported
          refs: [ART-TEST-001]
        AC-02:
          status: unsupported
          refs: []
      supporting_refs: [ART-TEST-001]
  artifacts:
    - artifact_id: ART-TEST-001
      kind: log
input:
  task_id: TASK-WORK-AC-001
  intent: complete
  requested_close_reason: completed_verified
  user_note: null
  superseded_by_task_id: null
action: close_task
expected_state:
  lifecycle_phase: blocked
  gates:
    evidence_gate: partial
expected_events:
  - close_requested
  - close_blocked
expected_artifacts:
  - artifact_id: ART-TEST-001
    kind: log
expected_projection:
  TASK: enqueued
expected_error:
  code: EVIDENCE_INSUFFICIENT
```

```yaml
scenario_id: CORE-evidence-ui-manual-qa-pending-blocks-close
initial_state:
  active_task:
    task_id: TASK-UI-QA-001
    mode: work
    lifecycle_phase: qa
    acceptance_criteria: ["AC-01 button copy updated"]
    gates:
      scope_gate: passed
      evidence_gate: sufficient
      verification_gate: passed
      qa_gate: pending
  manual_qa_records: []
input:
  task_id: TASK-UI-QA-001
  intent: complete
  requested_close_reason: completed_verified
  user_note: null
  superseded_by_task_id: null
action: close_task
expected_state:
  lifecycle_phase: qa
  gates:
    qa_gate: pending
expected_events:
  - close_requested
  - close_blocked
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: QA_REQUIRED
```

```yaml
scenario_id: CORE-verify-manual-bundle-detached-passed
initial_state:
  active_task:
    task_id: TASK-VERIFY-BUNDLE-001
    mode: work
    lifecycle_phase: verifying
    active_change_unit_id: CU-VERIFY-BUNDLE-001
    gates:
      evidence_gate: sufficient
      verification_gate: pending
  active_change_unit:
    change_unit_id: CU-VERIFY-BUNDLE-001
    allowed_paths: ["src/profile/editor.ts"]
  runs:
    - run_id: RUN-VERIFY-BUNDLE-TARGET-001
      kind: implementation
      status: completed
      artifact_refs: [ART-DIFF-001, ART-TEST-001]
  evidence_manifests:
    - evidence_manifest_id: EM-VERIFY-BUNDLE-001
      status: sufficient
      supporting_refs: [RUN-VERIFY-BUNDLE-TARGET-001, ART-DIFF-001, ART-TEST-001]
  artifacts:
    - artifact_id: ART-BUNDLE-001
      kind: bundle
    - artifact_id: ART-DIFF-001
      kind: diff
    - artifact_id: ART-TEST-001
      kind: log
input:
  task_id: TASK-VERIFY-BUNDLE-001
  change_unit_id: CU-VERIFY-BUNDLE-001
  evaluator_run_id: null
  target_run_id: RUN-VERIFY-BUNDLE-TARGET-001
  verdict: passed
  checks_performed:
    - check_id: manual-bundle-review
      result: passed
      summary: "Reviewed the task summary, acceptance criteria, Change Unit scope, approval scope, diff, test log, evidence manifest, and known risks from the manual bundle."
  evidence_reviewed:
    state_refs:
      - record_kind: task
        record_id: TASK-VERIFY-BUNDLE-001
        projection_path: null
      - record_kind: change_unit
        record_id: CU-VERIFY-BUNDLE-001
        projection_path: null
      - record_kind: run
        record_id: RUN-VERIFY-BUNDLE-TARGET-001
        projection_path: null
      - record_kind: evidence_manifest
        record_id: EM-VERIFY-BUNDLE-001
        projection_path: null
    artifact_refs:
      - artifact_id: ART-BUNDLE-001
        kind: bundle
        uri: harness-artifact://PROJECT-VERIFY/ART-BUNDLE-001
        sha256: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
        size_bytes: 4096
        content_type: application/json
        redaction_state: none
        task_id: TASK-VERIFY-BUNDLE-001
        run_id: RUN-VERIFY-BUNDLE-TARGET-001
        created_at: "2026-05-10T00:00:00Z"
        produced_by: harness
        retention_class: task
      - artifact_id: ART-DIFF-001
        kind: diff
        uri: harness-artifact://PROJECT-VERIFY/ART-DIFF-001
        sha256: dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd
        size_bytes: 2048
        content_type: text/x-diff
        redaction_state: none
        task_id: TASK-VERIFY-BUNDLE-001
        run_id: RUN-VERIFY-BUNDLE-TARGET-001
        created_at: "2026-05-10T00:00:00Z"
        produced_by: lead_agent
        retention_class: task
      - artifact_id: ART-TEST-001
        kind: log
        uri: harness-artifact://PROJECT-VERIFY/ART-TEST-001
        sha256: 7777777777777777777777777777777777777777777777777777777777777777
        size_bytes: 3072
        content_type: text/plain
        redaction_state: none
        task_id: TASK-VERIFY-BUNDLE-001
        run_id: RUN-VERIFY-BUNDLE-TARGET-001
        created_at: "2026-05-10T00:00:00Z"
        produced_by: lead_agent
        retention_class: task
  independence:
    context: manual_bundle
    write_capable: false
    baseline_reverified: true
    evaluator_surface_id: SURFACE-EVAL-MANUAL-BUNDLE-001
    parent_run_id: null
  blockers: []
  artifact_inputs:
    - input_id: ART-IN-BUNDLE-001
      source_kind: existing_artifact
      existing_artifact_ref:
        artifact_id: ART-BUNDLE-001
        kind: bundle
        uri: harness-artifact://PROJECT-VERIFY/ART-BUNDLE-001
        sha256: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
        size_bytes: 4096
        content_type: application/json
        redaction_state: none
        task_id: TASK-VERIFY-BUNDLE-001
        run_id: RUN-VERIFY-BUNDLE-TARGET-001
        created_at: "2026-05-10T00:00:00Z"
        produced_by: harness
        retention_class: task
      staged: null
      kind: bundle
      redaction_state: none
      produced_by: harness
      retention_class: task
      relation:
        task_id: TASK-VERIFY-BUNDLE-001
        run_id: null
        record_kind: eval
        record_id_hint: EVAL-VERIFY-BUNDLE-001
      description: "Manual verification bundle reviewed by the evaluator."
action: record_eval
expected_state:
  lifecycle_phase: verifying
  assurance_level: detached_verified
  gates:
    verification_gate: passed
expected_events:
  - eval_recorded
  - verification_passed
expected_artifacts:
  - artifact_id: ART-BUNDLE-001
    kind: bundle
expected_projection:
  EVAL: enqueued
  TASK: enqueued
expected_error: null
```

```yaml
scenario_id: CORE-verify-subagent-context-not-detached-by-default
initial_state:
  active_task:
    task_id: TASK-VERIFY-SUBAGENT-001
    mode: work
    lifecycle_phase: verifying
    gates:
      verification_gate: pending
  evidence_manifests:
    - evidence_manifest_id: EM-VERIFY-SUBAGENT-001
      status: sufficient
      supporting_refs: [RUN-VERIFY-SUBAGENT-TARGET-001]
  runs:
    - run_id: RUN-VERIFY-SUBAGENT-TARGET-001
      kind: implementation
      status: completed
input:
  task_id: TASK-VERIFY-SUBAGENT-001
  change_unit_id: null
  evaluator_run_id: null
  target_run_id: RUN-VERIFY-SUBAGENT-TARGET-001
  verdict: passed
  checks_performed:
    - check_id: inherited-subagent-context
      result: passed
      summary: "Evidence checks passed, but the evaluator inherited subagent context from the parent run and did not satisfy a detached verification profile."
  evidence_reviewed:
    state_refs:
      - record_kind: run
        record_id: RUN-VERIFY-SUBAGENT-TARGET-001
        projection_path: null
      - record_kind: evidence_manifest
        record_id: EM-VERIFY-SUBAGENT-001
        projection_path: null
    artifact_refs: []
  independence:
    context: subagent_context
    write_capable: false
    baseline_reverified: false
    evaluator_surface_id: SURFACE-EVAL-SUBAGENT-001
    parent_run_id: RUN-VERIFY-SUBAGENT-TARGET-001
  blockers: []
  artifact_inputs: []
action: record_eval
expected_state:
  lifecycle_phase: verifying
  assurance_level: none
  gates:
    verification_gate: pending
expected_events:
  - eval_recorded
  - verify_not_detached_detected
expected_artifacts: []
expected_projection:
  EVAL: enqueued
  TASK: enqueued
expected_error:
  code: VERIFY_NOT_DETACHED
```

```yaml
scenario_id: CORE-verify-waiver-risk-accepted-visible-succeeds
initial_state:
  active_task:
    task_id: TASK-VERIFY-RISK-001
    mode: work
    lifecycle_phase: waiting_user
    assurance_level: self_checked
    gates:
      scope_gate: passed
      decision_gate: resolved
      evidence_gate: sufficient
      verification_gate: waived_by_user
      qa_gate: not_required
      acceptance_gate: accepted
  residual_risks:
    - risk_id: RISK-VERIFY-001
      close_relevant: true
      visibility: visible
      accepted: true
  decision_packets:
    - decision_packet_id: DEC-VERIFY-WAIVER-001
      decision_kind: verification_waiver
      status: resolved
    - decision_packet_id: DEC-RISK-ACCEPT-001
      decision_kind: residual_risk_acceptance
      status: resolved
      residual_risk_refs: [RISK-VERIFY-001]
input:
  task_id: TASK-VERIFY-RISK-001
  intent: complete
  requested_close_reason: completed_with_risk_accepted
  user_note: "User accepts remaining verification risk for urgent local-only fix."
  superseded_by_task_id: null
action: close_task
expected_state:
  lifecycle_phase: completed
  result: passed
  close_reason: completed_with_risk_accepted
  assurance_level: self_checked
  residual_risk_summary:
    status: accepted
    accepted_refs: [RISK-VERIFY-001]
expected_events:
  - close_requested
  - risk_accepted_close_recorded
  - task_closed
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error: null
```

```yaml
scenario_id: CORE-verify-waiver-risk-accepted-hidden-blocks-close
initial_state:
  active_task:
    task_id: TASK-VERIFY-RISK-HIDDEN-001
    mode: work
    lifecycle_phase: waiting_user
    assurance_level: self_checked
    gates:
      scope_gate: passed
      evidence_gate: sufficient
      verification_gate: waived_by_user
      qa_gate: not_required
      acceptance_gate: accepted
  residual_risks:
    - risk_id: RISK-VERIFY-HIDDEN-001
      close_relevant: true
      visibility: not_visible
      accepted: false
  decision_packets:
    - decision_packet_id: DEC-VERIFY-WAIVER-002
      decision_kind: verification_waiver
      status: resolved
input:
  task_id: TASK-VERIFY-RISK-HIDDEN-001
  intent: complete
  requested_close_reason: completed_with_risk_accepted
  user_note: "User accepts remaining verification risk for urgent local-only fix."
  superseded_by_task_id: null
action: close_task
expected_state:
  lifecycle_phase: waiting_user
  assurance_level: self_checked
  gates:
    verification_gate: waived_by_user
    acceptance_gate: accepted
  residual_risk_summary:
    status: not_visible
    not_visible_refs: [RISK-VERIFY-HIDDEN-001]
expected_events:
  - close_requested
  - close_blocked
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: RESIDUAL_RISK_NOT_VISIBLE
```

```yaml
scenario_id: CONN-cooperative-guarantee-display
initial_state:
  surface:
    surface_id: SURF-0001
    guarantee_level: cooperative
    changed_path_detection: validator
  active_task:
    mode: direct
    lifecycle_phase: ready
input:
  include:
    task: false
    gates: false
    projections: false
    pending_decisions: false
    guarantees: true
    journey_card: false
    decision_packets: false
    autonomy_boundary: false
    write_authority: false
    residual_risk: false
action: status
expected_state:
  guarantee_display:
    level: cooperative
    notes:
      - "This surface is expected to follow Harness decisions, but Harness may not physically block an out-of-scope write before it happens. Changed-path validation can detect violations afterward."
expected_events: []
expected_artifacts: []
expected_projection: {}
expected_error: null
```

```yaml
scenario_id: CONN-mcp-unavailable-write-hold
initial_state:
  surface:
    guarantee_level: cooperative
    mcp_available: false
  active_task:
    task_id: TASK-MCP-HOLD-001
    mode: direct
    lifecycle_phase: ready
    active_change_unit_id: CU-MCP-HOLD-001
    gates:
      scope_gate: passed
  active_change_unit:
    change_unit_id: CU-MCP-HOLD-001
    allowed_paths: ["src/profile/ProfileForm.tsx"]
    allowed_tools: ["edit"]
input:
  task_id: TASK-MCP-HOLD-001
  change_unit_id: CU-MCP-HOLD-001
  intended_operation: "Edit the profile form through a cooperative surface while MCP is unavailable."
  intended_paths: ["src/profile/ProfileForm.tsx"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: []
  baseline_ref: BASE-MCP-HOLD-001
action: prepare_write
expected_state:
  lifecycle_phase: blocked
  write_held: true
  write_decision: blocked
  validators:
    surface_capability_check:
      status: blocked
expected_events:
  - prepare_write_blocked
  - capability_insufficient_detected
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: MCP_UNAVAILABLE
  details:
    mcp_unavailable_kind: surface_mcp_unavailable
```

## Core Fixture Examples

`prepare_write` allowed examples expect the Task to move from `ready` to `executing` because the kernel transition table owns and defines that transition.

Approval lifecycle coverage should be materialized as separate exact-shape fixtures or as suite catalog sequencing, not by adding fixture body fields. These fixtures assert owner-defined observable effects from [Kernel `prepare_write` State Logic](03-kernel-spec.md#prepare_write-state-logic), [`harness.prepare_write`](05-mcp-api-and-schemas.md#harnessprepare_write), and the [APR projection summary](07-document-projection.md#apr), rather than redefining the lifecycle.

Fixture authors should keep these observable assertions:

- the first uncovered sensitive `prepare_write` returns `approval_required`, includes an approval candidate, returns no Write Authorization, and sets or keeps `approval_gate=required` when blocker state is committed
- committed blocker state may enqueue `TASK`, but the non-mutating candidate must not enqueue `APR`
- dry-run or candidate-display-only paths must not assert committed `TASK` changes unless blocker state was actually committed
- `request_user_decision(decision_kind=approval)` creates the approval-shaped Decision Packet plus pending Approval state, sets `approval_gate=pending`, and enqueues `APR`
- `record_user_decision` updates Approval/Decision Packet state and `approval_gate`, may enqueue `APR`, but still creates no Write Authorization
- only a later compatible `prepare_write` retry with a fresh idempotency key and current `expected_state_version` may create the Write Authorization

UI or status assertions for the first payload must call it candidate display, not an `APR` projection.

```yaml
scenario_id: CORE-prepare-write-no-change-unit
initial_state:
  active_task:
    task_id: TASK-NO-CU-001
    mode: work
    lifecycle_phase: ready
    active_change_unit: null
input:
  task_id: TASK-NO-CU-001
  change_unit_id: null
  intended_operation: "Edit login without an active Change Unit."
  intended_paths: ["src/auth/login.ts"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: []
  baseline_ref: null
action: prepare_write
expected_state:
  lifecycle_phase: blocked
  gates:
    scope_gate: blocked
expected_events:
  - prepare_write_blocked
expected_artifacts: []
expected_projection:
  TASK: stale_or_enqueued
expected_error:
  code: NO_ACTIVE_CHANGE_UNIT
```

```yaml
scenario_id: CORE-prepare-write-allowed-creates-write-authorization
initial_state:
  active_task:
    task_id: TASK-WRITE-001
    mode: direct
    lifecycle_phase: ready
    active_change_unit_id: CU-WRITE-001
    gates:
      scope_gate: passed
      decision_gate: not_required
      approval_gate: not_required
      design_gate: passed
  active_change_unit:
    change_unit_id: CU-WRITE-001
    allowed_paths: ["src/a.ts"]
    allowed_tools: ["edit"]
    allowed_commands: []
    baseline_ref: BASE-WRITE-001
input:
  task_id: TASK-WRITE-001
  change_unit_id: CU-WRITE-001
  intended_operation: "Edit the scoped direct file."
  intended_paths: ["src/a.ts"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: []
  baseline_ref: BASE-WRITE-001
action: prepare_write
expected_state:
  lifecycle_phase: executing
  gates:
    scope_gate: passed
    decision_gate: not_required
    approval_gate: not_required
  write_decision: allowed
  write_authorization_ref:
    record_kind: write_authorization
    record_id: WA-WRITE-001
  write_authorization:
    write_authorization_id: WA-WRITE-001
    status: allowed
    change_unit_id: CU-WRITE-001
    intended_paths: ["src/a.ts"]
    consumed_by_run_id: null
  checks:
    scope_coverage: passed
    changed_paths_intent: passed
expected_events:
  - prepare_write_allowed
  - write_authorization_created
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error: null
```

```yaml
scenario_id: CORE-record-run-without-write-authorization-blocked
initial_state:
  active_task:
    task_id: TASK-WRITE-002
    mode: direct
    lifecycle_phase: executing
    active_change_unit_id: CU-WRITE-002
    gates:
      scope_gate: passed
      evidence_gate: none
  active_change_unit:
    change_unit_id: CU-WRITE-002
    allowed_paths: ["src/a.ts"]
    allowed_tools: ["edit"]
    baseline_ref: BASE-WRITE-002
input:
  kind: direct
  task_id: TASK-WRITE-002
  change_unit_id: CU-WRITE-002
  run_id: null
  baseline_ref: BASE-WRITE-002
  write_authorization_id: null
  summary: "Direct edit was attempted without a prepare_write authorization."
  artifact_inputs: []
  payload:
    direct:
      observed_changes:
        changed_paths: ["src/a.ts"]
        created_paths: []
        deleted_paths: []
      command_results: []
      evidence_updates:
        acceptance_criteria: []
        feedback_loop_updates: []
      self_check_summary: "Self-check cannot count because Write Authorization is missing."
      escalation:
        value: none
        reason: null
action: record_run
expected_state:
  lifecycle_phase: executing
  gates:
    scope_gate: passed
    evidence_gate: none
  run_recorded: false
  write_authorization_ref: null
  checks:
    changed_paths: blocked
    scope_coverage: passed
expected_events: []
expected_artifacts: []
expected_projection: {}
expected_error:
  code: WRITE_AUTHORIZATION_REQUIRED
```

This fixture intentionally has `run_recorded: false`, no stable events, no artifacts, and no projection changes. The corresponding `RecordRunResponse.run_id` is `null`; no fabricated Run ID is required or allowed.

```yaml
scenario_id: CORE-record-run-observed-path-outside-authorization-blocks-or-stales
initial_state:
  active_task:
    task_id: TASK-WRITE-003
    mode: work
    lifecycle_phase: executing
    active_change_unit_id: CU-WRITE-003
    gates:
      scope_gate: passed
      approval_gate: not_required
      evidence_gate: partial
  active_change_unit:
    change_unit_id: CU-WRITE-003
    allowed_paths: ["src/a.ts"]
    allowed_tools: ["edit"]
    baseline_ref: BASE-WRITE-003
  write_authorizations:
    - write_authorization_id: WA-WRITE-003
      status: allowed
      change_unit_id: CU-WRITE-003
      basis_state_version: 1
      intended_paths: ["src/a.ts"]
      consumed_by_run_id: null
input:
  kind: implementation
  task_id: TASK-WRITE-003
  change_unit_id: CU-WRITE-003
  run_id: RUN-WRITE-003
  baseline_ref: BASE-WRITE-003
  write_authorization_id: WA-WRITE-003
  summary: "Implementation touched an observed path outside the authorization."
  artifact_inputs: []
  payload:
    implementation:
      observed_changes:
        changed_paths: ["src/a.ts", "src/b.ts"]
        created_paths: []
        deleted_paths: []
      command_results: []
      evidence_updates:
        acceptance_criteria: []
        feedback_loop_updates: []
      tdd_trace_update: null
action: record_run
expected_state:
  lifecycle_phase: blocked
  gates:
    scope_gate: blocked
    evidence_gate: stale
  close_readiness: blocked
  projection_status: stale
  run_recorded: true
  run:
    run_id: RUN-WRITE-003
    kind: implementation
    status: violation
    write_authorization_id: null
    observed_changes:
      changed_paths: ["src/a.ts", "src/b.ts"]
    violation_payload:
      attempted_write_authorization_id: WA-WRITE-003
    evidence_sufficiency_allowed: false
  write_authorization:
    write_authorization_id: WA-WRITE-003
    status: stale
    consumed_by_run_id: null
  observed_change_violation:
    outside_authorized_paths: ["src/b.ts"]
  checks:
    changed_paths: blocked
    scope_coverage: blocked
expected_events:
  - run_recorded
  - write_authorization_violation_detected
  - write_authorization_staled
  - scope_violation_detected
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: SCOPE_VIOLATION
```

```yaml
scenario_id: CORE-record-run-consumed-write-authorization-invalid
initial_state:
  active_task:
    task_id: TASK-WRITE-004
    mode: direct
    lifecycle_phase: executing
    active_change_unit_id: CU-WRITE-004
    gates:
      scope_gate: passed
      evidence_gate: none
  active_change_unit:
    change_unit_id: CU-WRITE-004
    allowed_paths: ["src/a.ts"]
    allowed_tools: ["edit"]
    baseline_ref: BASE-WRITE-004
  write_authorizations:
    - write_authorization_id: WA-WRITE-004
      status: consumed
      change_unit_id: CU-WRITE-004
      basis_state_version: 1
      intended_paths: ["src/a.ts"]
      consumed_by_run_id: RUN-WRITE-PREV-004
input:
  kind: direct
  task_id: TASK-WRITE-004
  change_unit_id: CU-WRITE-004
  run_id: null
  baseline_ref: BASE-WRITE-004
  write_authorization_id: WA-WRITE-004
  summary: "Direct run tried to reuse a consumed Write Authorization."
  artifact_inputs: []
  payload:
    direct:
      observed_changes:
        changed_paths: ["src/a.ts"]
        created_paths: []
        deleted_paths: []
      command_results: []
      evidence_updates:
        acceptance_criteria: []
        feedback_loop_updates: []
      self_check_summary: "Path scope matches, but the authorization is already consumed."
      escalation:
        value: none
        reason: null
action: record_run
expected_state:
  lifecycle_phase: executing
  gates:
    scope_gate: passed
    evidence_gate: none
  run_recorded: false
  write_authorization:
    write_authorization_id: WA-WRITE-004
    status: consumed
    consumed_by_run_id: RUN-WRITE-PREV-004
  checks:
    changed_paths: passed
    scope_coverage: passed
  invalid_authorization_reason: already_consumed
expected_events: []
expected_artifacts: []
expected_projection: {}
expected_error:
  code: WRITE_AUTHORIZATION_INVALID
```

```yaml
scenario_id: CORE-same-session-verify-not-detached
initial_state:
  active_task:
    task_id: TASK-SAME-SESSION-VERIFY-001
    mode: work
    lifecycle_phase: verifying
    gates:
      verification_gate: pending
  runs:
    - run_id: RUN-SAME-SESSION-TARGET-001
      kind: implementation
      status: completed
input:
  task_id: TASK-SAME-SESSION-VERIFY-001
  change_unit_id: null
  evaluator_run_id: null
  target_run_id: RUN-SAME-SESSION-TARGET-001
  verdict: passed
  checks_performed:
    - check_id: same-session-review
      result: passed
      summary: "The same session reviewed its own target run; checks passed but the evaluator is not detached."
  evidence_reviewed:
    state_refs:
      - record_kind: run
        record_id: RUN-SAME-SESSION-TARGET-001
        projection_path: null
    artifact_refs: []
  independence:
    context: same_session
    write_capable: true
    baseline_reverified: false
    evaluator_surface_id: SURFACE-SAME-SESSION-001
    parent_run_id: RUN-SAME-SESSION-TARGET-001
  blockers: []
  artifact_inputs: []
action: record_eval
expected_state:
  assurance_level: none
  gates:
    verification_gate: pending
expected_events:
  - eval_recorded
  - verify_not_detached_detected
expected_artifacts: []
expected_projection:
  EVAL: enqueued
  TASK: enqueued
expected_error:
  code: VERIFY_NOT_DETACHED
```

```yaml
scenario_id: CORE-projection-failure-state-current
initial_state:
  active_task:
    mode: direct
    lifecycle_phase: completed
    result: passed
    projection_status: current
input:
  projection_kind: TASK
  render_error: permission_denied
action: projection_refresh
expected_state:
  lifecycle_phase: completed
  result: passed
  projection_status: failed
expected_events:
  - projection_refresh_failed
expected_artifacts: []
expected_projection:
  TASK: failed
expected_error:
  code: PROJECTION_STALE
```

## Agency Fixture Examples

```yaml
scenario_id: AGENCY-decision-packet-required-before-product-tradeoff-write
initial_state:
  active_task:
    task_id: TASK-TRADEOFF-001
    mode: work
    lifecycle_phase: ready
    active_change_unit_id: CU-TRADEOFF-001
    gates:
      scope_gate: passed
      decision_gate: not_required
      approval_gate: not_required
      design_gate: passed
  active_change_unit:
    change_unit_id: CU-TRADEOFF-001
    allowed_paths: ["src/pricing/checkout.ts"]
    baseline_ref: BASE-TRADEOFF-001
    autonomy_boundary:
      status: active
      what_agent_may_do: ["Implement the selected checkout discount behavior."]
      what_requires_user_judgment: ["Choose the revenue versus conversion trade-off."]
    blocking_decision_requirements:
      - decision_kind: product_tradeoff
        status: absent
        affected_paths: ["src/pricing/checkout.ts"]
        topic: revenue_vs_conversion
        options_known: true
input:
  task_id: TASK-TRADEOFF-001
  change_unit_id: CU-TRADEOFF-001
  intended_operation: "Change checkout discount precedence from margin-safe to conversion-optimized."
  intended_paths: ["src/pricing/checkout.ts"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: []
  baseline_ref: BASE-TRADEOFF-001
action: prepare_write
expected_state:
  lifecycle_phase: waiting_user
  gates:
    decision_gate: required
  write_decision: decision_required
  decision_packet_candidate:
    decision_kind: product_tradeoff
    affected_gates: [decision_gate]
expected_events:
  - prepare_write_blocked
  - decision_required
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: DECISION_REQUIRED
```

```yaml
scenario_id: AGENCY-residual-risk-visible-before-acceptance
initial_state:
  active_task:
    mode: work
    lifecycle_phase: waiting_user
    gates:
      evidence_gate: sufficient
      verification_gate: passed
      qa_gate: passed
      acceptance_gate: pending
  residual_risks:
    - risk_id: RISK-ACCEPT-001
      close_relevant: true
      visibility: not_visible
      accepted: false
  decision_packets:
    - decision_packet_id: DEC-ACCEPT-001
      decision_kind: acceptance
      status: pending_user
      user_context:
        minimum_context: ["acceptance criteria", "evidence summary"]
input:
  decision_packet_id: DEC-ACCEPT-001
  decision_kind: acceptance
  selected_option_id: accept
  decision:
    acceptance:
      value: accepted
  note: "Acceptance attempted before close-relevant residual risk was visible."
  waiver_reason: null
  accepted_risks: []
action: record_user_decision
expected_state:
  lifecycle_phase: waiting_user
  gates:
    acceptance_gate: pending
  residual_risk_summary:
    status: not_visible
    not_visible_refs: [RISK-ACCEPT-001]
  decision_packets:
    DEC-ACCEPT-001: pending_user
expected_events: []
expected_artifacts: []
expected_projection: {}
expected_error:
  code: RESIDUAL_RISK_NOT_VISIBLE
```

```yaml
scenario_id: AGENCY-acceptance-no-known-residual-risk-none-succeeds
initial_state:
  active_task:
    mode: work
    lifecycle_phase: waiting_user
    gates:
      evidence_gate: sufficient
      verification_gate: passed
      qa_gate: passed
      acceptance_gate: pending
  residual_risks: []
  decision_packets:
    - decision_packet_id: DEC-ACCEPT-NONE-001
      decision_kind: acceptance
      status: pending_user
      user_context:
        minimum_context: ["acceptance criteria", "evidence summary", "ResidualRiskSummary.status=none"]
input:
  decision_packet_id: DEC-ACCEPT-NONE-001
  decision_kind: acceptance
  selected_option_id: accept
  decision:
    acceptance:
      value: accepted
  note: "Acceptance recorded after confirming no known close-relevant residual risk."
  waiver_reason: null
  accepted_risks: []
action: record_user_decision
expected_state:
  lifecycle_phase: waiting_user
  gates:
    acceptance_gate: accepted
  residual_risk_summary:
    status: none
    close_relevant_count: 0
  decision_packets:
    DEC-ACCEPT-NONE-001: resolved
expected_events: []
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error: null
```

```yaml
scenario_id: AGENCY-close-hidden-residual-risk-blocks-close
initial_state:
  active_task:
    task_id: TASK-CLOSE-HIDDEN-RISK-001
    mode: work
    lifecycle_phase: waiting_user
    assurance_level: detached_verified
    gates:
      scope_gate: passed
      decision_gate: resolved
      approval_gate: not_required
      design_gate: passed
      evidence_gate: sufficient
      verification_gate: passed
      qa_gate: passed
      acceptance_gate: accepted
  residual_risks:
    - risk_id: RISK-CLOSE-HIDDEN-001
      close_relevant: true
      visibility: not_visible
      accepted: false
input:
  task_id: TASK-CLOSE-HIDDEN-RISK-001
  intent: complete
  requested_close_reason: completed_verified
  user_note: null
  superseded_by_task_id: null
action: close_task
expected_state:
  lifecycle_phase: waiting_user
  result: none
  assurance_level: detached_verified
  gates:
    evidence_gate: sufficient
    verification_gate: passed
    qa_gate: passed
    acceptance_gate: accepted
  residual_risk_summary:
    status: not_visible
    not_visible_refs: [RISK-CLOSE-HIDDEN-001]
expected_events:
  - close_requested
  - close_blocked
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: RESIDUAL_RISK_NOT_VISIBLE
```

```yaml
scenario_id: AGENCY-afk-boundary-blocks-public-api-change
initial_state:
  active_task:
    task_id: TASK-API-001
    mode: work
    lifecycle_phase: ready
    active_change_unit_id: CU-API-001
    gates:
      scope_gate: passed
      decision_gate: not_required
      approval_gate: granted
      design_gate: passed
  active_change_unit:
    change_unit_id: CU-API-001
    allowed_paths: ["src/api/public.ts"]
    sensitive_categories: ["public_api_change"]
    autonomy_boundary:
      autonomy_profile: afk_eligible
      status: active
      what_agent_may_do: ["Refactor internal handler code."]
      stop_conditions: ["public_api_change"]
  approvals:
    - approval_id: APR-API-001
      sensitive_categories: ["public_api_change"]
      allowed_paths: ["src/api/public.ts"]
      status: granted
input:
  task_id: TASK-API-001
  change_unit_id: CU-API-001
  intended_operation: "Add a response field to the public API while the user is AFK."
  intended_paths: ["src/api/public.ts"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: ["public_api_change"]
  baseline_ref: BASE-API-001
action: prepare_write
expected_state:
  lifecycle_phase: waiting_user
  gates:
    decision_gate: required
    approval_gate: granted
  autonomy_boundary_summary:
    status: exceeded
    triggered_stop_conditions: ["public_api_change"]
  write_decision: decision_required
expected_events:
  - prepare_write_blocked
  - autonomy_boundary_exceeded
  - decision_required
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: AUTONOMY_BOUNDARY_EXCEEDED
```

## Connector Fixture Examples

```yaml
scenario_id: CONN-generated-file-drift-reconcile
initial_state:
  connector_manifest:
    status: current
input:
  changed_generated_path: ".harness/agent/generated/rules.md"
action: doctor_surface
expected_state:
  reconcile_required: true
expected_events:
  - generated_file_drift_detected
  - reconcile_item_created
expected_artifacts: []
expected_projection: {}
expected_error:
  code: RECONCILE_REQUIRED
```

```yaml
scenario_id: CONN-journey-card-shown-before-significant-resume
initial_state:
  surface:
    guarantee_level: cooperative
  active_task:
    task_id: TASK-RESUME-001
    state_version: 42
    mode: work
    lifecycle_phase: executing
    active_change_unit_id: CU-RESUME-001
    gates:
      scope_gate: passed
      decision_gate: pending
      approval_gate: not_required
      evidence_gate: partial
  active_change_unit:
    change_unit_id: CU-RESUME-001
    allowed_paths: ["src/resume/current.ts"]
  journey_refs:
    journey_card_ref:
      record_kind: projection
      record_id: JOURNEY-CARD-RESUME-001
    journey_spine_entry_refs:
      - record_kind: journey_spine_entry
        record_id: JSE-RESUME-001
  evidence_refs:
    state_refs:
      - record_kind: evidence_manifest
        record_id: EVIDENCE-RESUME-001
    artifact_refs:
      - artifact_id: ART-DIFF-RESUME-001
        kind: diff
  decision_packets:
    - decision_packet_id: DEC-RESUME-001
      decision_kind: product_tradeoff
      status: pending_user
  residual_risks:
    - risk_id: RISK-RESUME-001
      close_relevant: true
      visibility: visible
      accepted: false
  projection_freshness:
    status: current
  resume_context:
    kind: significant
input:
  task_id: TASK-RESUME-001
  focus: implementation
  include_instruction_bundle: true
action: next
expected_state:
  state_version: 42
  no_state_mutation: true
  next_response:
    state:
      lifecycle_phase: executing
    judgment_context:
      journey_card:
        task_id: TASK-RESUME-001
        active_change_unit_ref:
          record_kind: change_unit
          record_id: CU-RESUME-001
        write_authority_summary:
          active_change_unit_ref:
            record_kind: change_unit
            record_id: CU-RESUME-001
          write_authorization_ref: null
          approval_status: not_required
          guarantee_display:
            level: cooperative
            notes: []
          note: "Autonomy Boundary is judgment latitude, not write authority."
        active_decision_packet_refs:
          - record_kind: decision_packet
            record_id: DEC-RESUME-001
        residual_risk_summary:
          status: visible
          close_relevant_count: 1
          visible_refs:
            - record_kind: residual_risk
              record_id: RISK-RESUME-001
          unaccepted_refs:
            - record_kind: residual_risk
              record_id: RISK-RESUME-001
        projection_freshness:
          status: current
      evidence_refs:
        state_refs:
          - record_kind: evidence_manifest
            record_id: EVIDENCE-RESUME-001
        artifact_refs:
          - artifact_id: ART-DIFF-RESUME-001
      active_decision_packet_refs:
        - record_kind: decision_packet
          record_id: DEC-RESUME-001
    instruction_bundle:
      relevant_refs:
        - record_kind: journey_spine_entry
          record_id: JSE-RESUME-001
        - record_kind: evidence_manifest
          record_id: EVIDENCE-RESUME-001
      artifact_refs:
        - artifact_id: ART-DIFF-RESUME-001
    pending_decisions:
      - record_kind: decision_packet
        record_id: DEC-RESUME-001
expected_events: []
expected_artifacts: []
expected_projection: {}
expected_error: null
```

```yaml
scenario_id: CONN-decision-packet-not-broad-approval
initial_state:
  active_task:
    task_id: TASK-CONN-DEC-001
    mode: work
    lifecycle_phase: ready
    active_change_unit_id: CU-CONN-DEC-001
    gates:
      scope_gate: passed
      decision_gate: not_required
      approval_gate: not_required
  active_change_unit:
    change_unit_id: CU-CONN-DEC-001
    allowed_paths: ["src/pricing/discount.ts"]
    baseline_ref: BASE-CONN-DEC-001
    autonomy_boundary:
      status: active
      what_agent_may_do: ["Implement the already selected pricing rule."]
      what_requires_user_judgment: ["Choose a margin versus conversion trade-off."]
    blocking_decision_requirements:
      - decision_kind: product_tradeoff
        broad_approval_requested: false
input:
  task_id: TASK-CONN-DEC-001
  change_unit_id: CU-CONN-DEC-001
  intended_operation: "Choose and implement a new discount priority."
  intended_paths: ["src/pricing/discount.ts"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: []
  baseline_ref: BASE-CONN-DEC-001
action: prepare_write
expected_state:
  lifecycle_phase: waiting_user
  gates:
    decision_gate: required
    approval_gate: not_required
  write_decision: decision_required
  approval_request_candidate: null
  write_authorization_ref: null
  decision_packet_candidate:
    decision_kind: product_tradeoff
    affected_gates: [decision_gate]
  validators:
    decision_quality_check:
      status: blocked
expected_events:
  - prepare_write_blocked
  - decision_required
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: DECISION_REQUIRED
```

```yaml
scenario_id: CONN-autonomy-boundary-breach-stops-or-routes-to-decision
initial_state:
  active_task:
    task_id: TASK-CONN-AB-001
    mode: work
    lifecycle_phase: ready
    active_change_unit_id: CU-CONN-AB-001
    gates:
      scope_gate: passed
      decision_gate: not_required
      approval_gate: not_required
  active_change_unit:
    change_unit_id: CU-CONN-AB-001
    allowed_paths: ["src/onboarding/copy.ts"]
    baseline_ref: BASE-CONN-AB-001
    autonomy_boundary:
      autonomy_profile: afk_eligible
      status: active
      what_agent_may_do: ["Edit onboarding copy within the approved tone."]
      what_requires_user_judgment: ["Change the onboarding promise or product positioning."]
      stop_conditions: ["product_positioning_change"]
input:
  task_id: TASK-CONN-AB-001
  change_unit_id: CU-CONN-AB-001
  intended_operation: "Change the onboarding promise from guided setup to automatic migration."
  intended_paths: ["src/onboarding/copy.ts"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: []
  baseline_ref: BASE-CONN-AB-001
action: prepare_write
expected_state:
  lifecycle_phase: waiting_user
  gates:
    decision_gate: required
  autonomy_boundary_summary:
    status: exceeded
    triggered_stop_conditions: ["product_positioning_change"]
  write_decision: decision_required
  write_held: true
  decision_packet_candidate:
    decision_kind: autonomy_boundary
    affected_gates: [decision_gate]
  validators:
    autonomy_boundary_check:
      status: blocked
expected_events:
  - prepare_write_blocked
  - autonomy_boundary_exceeded
  - decision_required
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: AUTONOMY_BOUNDARY_EXCEEDED
```

### Connector Agency Catalog Entries

These are catalog entries, not fixture bodies. The concrete fixture examples above materialize the highest-priority entries with the exact fixture shape and assert Core state, events, projection refs, and errors rather than rendered prose.

| Scenario ID | Core action | Required assertions |
|---|---|---|
| `CONN-journey-card-shown-before-significant-resume` | `next` | `next` returns current Task state version, current Journey Card or journey ref, active Change Unit ref, pending Decision Packet refs, residual-risk summary, and projection freshness before returning a significant resume instruction bundle; no state events are appended for the read. |
| `CONN-recommended-playbooks-read-only-guidance` | `next` | `next` may return `recommended_playbooks` for the current stage, but the read appends no state events, enqueues no projections, creates no artifacts or evidence, does not change any gate, and does not authorize writes. Any playbook that would require product judgment routes to an existing Decision Packet or Decision Packet request path. |
| `CONN-role-lens-non-authoritative-routing` | `next` | `next` may recommend role-lens playbooks such as `product-review`, `eng-review`, `design-review`, `security-review`, `qa-review`, or `release-handoff`; the read does not mutate state, satisfy gates, authorize writes, create evidence, perform or record verification, record QA, waive QA or verification, accept residual risk, accept the result, close a Task, or upgrade assurance. Lens outputs that need action are represented as existing Decision Packet refs, `DecisionPacketCandidate` routes, validator/evidence/Manual QA/residual-risk candidates, release-handoff input, or a recommended next playbook. |
| `CONN-freeze-narrows-current-boundary` | `prepare_write` or `next` | A freeze request is reflected as display guidance, a held write, a stricter next action, or a `prepare_write` block/hold when existing scope is incompatible. If the fixture asserts a persistent Change Unit, allowed-path, Autonomy Boundary, AFK stop-condition, or related owner-record update, that update must occur through the existing Core state-changing path, Decision Packet route, or owner-record update path; the freeze label does not mutate owner records by itself. |
| `CONN-guard-display-matches-capability` | `status` or `prepare_write` | Guard display reports the connected profile's actual `guarantee_level` and limitation notes. Cooperative guard does not claim prevention; detective guard requires changed-path/log/artifact validation assertions; preventive guard is not required for MVP unless a proven pre-tool blocking path exists. |
| `CONN-careful-mode-no-authority-upgrade` | `next` or `prepare_write` | Careful mode may make the next action stricter by requiring fresher status, narrower scope, explicit `prepare_write`, evidence mapping, or one blocking user question. It does not mutate state by itself, satisfy gates, grant approval, authorize writes, create evidence, perform or record verification, record QA, waive QA or verification, accept residual risk, accept the result, close the Task, or upgrade assurance. |
| `CONN-decision-packet-not-broad-approval` | `prepare_write` | Product judgment outside the active Decision Packet returns `decision_required` with a `decision_packet_candidate`; any decision request metadata is optional routing/replay compatibility data and cannot satisfy `decision_gate` without a compatible Decision Packet; it does not return `approval_required`, does not create a broad approval candidate, and does not set `approval_gate=granted`. |
| `CONN-autonomy-boundary-breach-stops-or-routes-to-decision` | `prepare_write` | Exceeding the active Autonomy Boundary returns `blocked` or `decision_required`, appends `autonomy_boundary_exceeded`, keeps the write held, and either references an existing compatible Decision Packet or returns a candidate decision packet. |

## Design-Quality Fixture Examples

```yaml
scenario_id: DESIGN-horizontal-feature-without-exception
initial_state:
  active_task:
    task_id: TASK-DESIGN-HORIZONTAL-001
    mode: work
    lifecycle_phase: ready
    active_change_unit_id: CU-DESIGN-HORIZONTAL-001
    gates:
      scope_gate: passed
      design_gate: pending
  active_change_unit:
    change_unit_id: CU-DESIGN-HORIZONTAL-001
    slice_type: horizontal-exception
    horizontal_exception_reason: null
    allowed_paths: ["src/shared/crossCutting.ts"]
    baseline_ref: BASE-DESIGN-HORIZONTAL-001
input:
  task_id: TASK-DESIGN-HORIZONTAL-001
  change_unit_id: CU-DESIGN-HORIZONTAL-001
  intended_operation: "Apply a horizontal exception without the required exception reason."
  intended_paths: ["src/shared/crossCutting.ts"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: []
  baseline_ref: BASE-DESIGN-HORIZONTAL-001
action: prepare_write
expected_state:
  lifecycle_phase: blocked
  gates:
    design_gate: partial
  write_decision: blocked
  validators:
    codebase_stewardship_check:
      status: blocked
expected_events:
  - prepare_write_blocked
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: VALIDATOR_FAILED
```

```yaml
scenario_id: DESIGN-manual-qa-required-missing
initial_state:
  active_task:
    task_id: TASK-DESIGN-QA-001
    mode: work
    lifecycle_phase: qa
    gates:
      qa_gate: pending
  manual_qa_records: []
input:
  task_id: TASK-DESIGN-QA-001
  intent: complete
  requested_close_reason: completed_verified
  user_note: null
  superseded_by_task_id: null
action: close_task
expected_state:
  lifecycle_phase: qa
  gates:
    qa_gate: pending
expected_events:
  - close_requested
  - close_blocked
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: QA_REQUIRED
```

```yaml
scenario_id: DESIGN-two-stage-review-critical-spec-finding-blocks-close
initial_state:
  active_task:
    task_id: TASK-REVIEW-SPEC-001
    mode: work
    lifecycle_phase: verifying
    active_change_unit_id: CU-REVIEW-SPEC-001
    acceptance_criteria:
      - criteria_id: AC-LOGIN-001
        statement: "Locked-account login returns the documented error state."
      - criteria_id: AC-LOGIN-002
        statement: "Successful login remains unchanged."
    gates:
      scope_gate: passed
      decision_gate: not_required
      approval_gate: not_required
      design_gate: passed
      evidence_gate: partial
      verification_gate: passed
      qa_gate: not_required
      acceptance_gate: accepted
  active_change_unit:
    change_unit_id: CU-REVIEW-SPEC-001
    completion_conditions:
      - "All login acceptance criteria have evidence."
    allowed_paths: ["src/auth/login.ts", "test/auth/login.test.ts"]
  runs:
    - run_id: RUN-REVIEW-SPEC-001
      kind: implementation
      status: completed
      summary: "Same-session review found AC-LOGIN-001 still missing evidence; no stewardship blocker was found."
  validator_results:
    codebase_stewardship_check:
      status: passed
    context_hygiene_check:
      status: passed
  evals:
    - eval_id: EVAL-REVIEW-SPEC-001
      verdict: passed
      independence_qualifier: manual_bundle
      target_run_id: RUN-REVIEW-SPEC-001
  evidence_manifests:
    - evidence_manifest_id: EM-REVIEW-SPEC-001
      status: partial
      coverage:
        AC-LOGIN-001: missing
        AC-LOGIN-002: covered
input:
  task_id: TASK-REVIEW-SPEC-001
  intent: complete
  requested_close_reason: completed_verified
  user_note: null
  superseded_by_task_id: null
action: close_task
expected_state:
  lifecycle_phase: verifying
  gates:
    evidence_gate: partial
    design_gate: passed
    verification_gate: passed
  close_blockers:
    - code: EVIDENCE_INSUFFICIENT
      related_refs:
        - record_kind: evidence_manifest
          record_id: EM-REVIEW-SPEC-001
        - record_kind: run
          record_id: RUN-REVIEW-SPEC-001
expected_response:
  display:
    review_stages:
      display_only: true
      canonical_state_record_created: false
      spec_compliance_review:
        status: failed
        finding_code: ACCEPTANCE_CRITERION_UNCOVERED
        acceptance_criteria_refs: [AC-LOGIN-001]
        routed_to: close_blocker
      code_quality_stewardship_review:
        status: passed
      authority_boundary:
        satisfies_gates: false
        authorizes_writes: false
        accepts_risk: false
        closes_task: false
        creates_detached_assurance: false
expected_events:
  - close_requested
  - close_blocked
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: EVIDENCE_INSUFFICIENT
```

```yaml
scenario_id: DESIGN-tdd-required-non-test-write-blocked-before-red
initial_state:
  active_task:
    task_id: TASK-TDD-RED-001
    mode: work
    lifecycle_phase: ready
    active_change_unit_id: CU-TDD-RED-001
    gates:
      scope_gate: passed
      approval_gate: not_required
      decision_gate: not_required
      design_gate: pending
  active_change_unit:
    change_unit_id: CU-TDD-RED-001
    allowed_paths: ["src/auth/login.ts", "test/auth/login.test.ts"]
    baseline_ref: BASE-TDD-RED-001
    stewardship_refs:
      feedback_loop_refs: [FBL-TDD-RED-001]
      tdd_trace_refs: [TDD-RED-001]
  tdd_policy:
    required: true
    behavior_slice: "Reject locked account login."
    red_evidence_required_before_non_test_write: true
  owner_records:
    feedback_loops:
      - feedback_loop_id: FBL-TDD-RED-001
        loop_kind: tdd
        planned_loop: "Add failing locked-account login test, implement, then pass."
        status: defined
        tdd_trace_refs: [TDD-RED-001]
    tdd_traces:
      - tdd_trace_id: TDD-RED-001
        status: required
        red_refs: []
        green_refs: []
        refactor_refs: []
        non_tdd_justification: null
input:
  task_id: TASK-TDD-RED-001
  change_unit_id: CU-TDD-RED-001
  intended_operation: "Implement locked-account login handling before recording the RED test."
  intended_paths: ["src/auth/login.ts"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: []
  baseline_ref: BASE-TDD-RED-001
action: prepare_write
expected_state:
  lifecycle_phase: blocked
  gates:
    design_gate: partial
  write_decision: blocked
  validators:
    feedback_loop_check:
      status: passed
    tdd_trace_required:
      status: blocked
      findings:
        - code: TDD_RED_REQUIRED_BEFORE_NON_TEST_WRITE
          severity: blocker
  evidence_manifest_coverage:
    tdd_trace: missing_red
expected_events:
  - prepare_write_blocked
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: VALIDATOR_FAILED
```

## Stewardship Fixture Examples

```yaml
scenario_id: STEWARDSHIP-qa-waiver-reason-required
initial_state:
  active_task:
    task_id: TASK-QA-WAIVER-001
    mode: work
    lifecycle_phase: qa
    gates:
      qa_gate: pending
      decision_gate: not_required
  manual_qa_policy:
    required: true
    waiver_decision_packet_required: false
    waiver_reason_required: true
input:
  task_id: TASK-QA-WAIVER-001
  change_unit_id: null
  qa_profile: ui_quality
  performed_by: user
  result: waived
  findings: []
  artifact_inputs: []
  waiver_reason: null
  waiver_decision_packet_ref: null
  feedback_loop_ref: null
  next_action: waive
action: record_manual_qa
expected_state:
  lifecycle_phase: qa
  gates:
    qa_gate: pending
    decision_gate: not_required
  manual_qa_record_created: false
  checks:
    qa_waiver_reason: blocked
expected_events: []
expected_artifacts: []
expected_projection: {}
expected_error:
  code: QA_REQUIRED
```

```yaml
scenario_id: STEWARDSHIP-qa-waiver-product-risk-requires-decision-packet
initial_state:
  active_task:
    task_id: TASK-QA-WAIVER-RISK-001
    mode: work
    lifecycle_phase: qa
    gates:
      qa_gate: pending
      decision_gate: not_required
  manual_qa_policy:
    required: true
    waiver_decision_packet_required: true
    waiver_reason_required: true
    product_or_user_risk: true
input:
  task_id: TASK-QA-WAIVER-RISK-001
  change_unit_id: null
  qa_profile: workflow
  performed_by: user
  result: waived
  findings: []
  artifact_inputs: []
  waiver_reason: "Known workflow risk accepted for a time-sensitive release."
  waiver_decision_packet_ref: null
  feedback_loop_ref: null
  next_action: waive
action: record_manual_qa
expected_state:
  lifecycle_phase: qa
  gates:
    qa_gate: pending
    decision_gate: required
  manual_qa_record_created: false
  validators:
    decision_quality_check:
      status: blocked
  checks:
    qa_waiver_reason: passed
expected_events: []
expected_artifacts: []
expected_projection: {}
expected_error:
  code: DECISION_REQUIRED
```

```yaml
scenario_id: STEWARDSHIP-public-interface-change-requires-module-interface-review
initial_state:
  active_task:
    task_id: TASK-PUBLIC-IFACE-001
    mode: work
    lifecycle_phase: ready
    active_change_unit_id: CU-PUBLIC-IFACE-001
    gates:
      scope_gate: passed
      approval_gate: granted
      decision_gate: resolved
      design_gate: passed
  active_change_unit:
    change_unit_id: CU-PUBLIC-IFACE-001
    allowed_paths: ["src/api/public.ts"]
    sensitive_categories: ["public_api_change"]
    baseline_ref: BASE-PUBLIC-API-001
    stewardship_refs:
      domain_terms: [TERM-API-RESOURCE-001]
      module_map_items: []
      interface_contracts: []
      feedback_loop_refs: [FBL-PUBLIC-API-001]
  approvals:
    - approval_id: APR-PUBLIC-API-001
      sensitive_categories: ["public_api_change"]
      allowed_paths: ["src/api/public.ts"]
      status: granted
  decision_packets:
    - decision_packet_id: DEC-PUBLIC-API-001
      decision_kind: architecture_choice
      topic: public_interface_commitment
      status: resolved
  owner_records:
    domain_terms:
      - domain_term_id: TERM-API-RESOURCE-001
        status: active
    module_map_items: []
    interface_contracts: []
    feedback_loops:
      - feedback_loop_id: FBL-PUBLIC-API-001
        status: defined
input:
  task_id: TASK-PUBLIC-IFACE-001
  change_unit_id: CU-PUBLIC-IFACE-001
  intended_operation: "Change exported response fields on the public API."
  intended_paths: ["src/api/public.ts"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: ["public_api_change"]
  baseline_ref: BASE-PUBLIC-API-001
action: prepare_write
expected_state:
  lifecycle_phase: blocked
  gates:
    approval_gate: granted
    decision_gate: resolved
    design_gate: partial
  write_decision: blocked
  checks:
    approval_scope: passed
  validators:
    codebase_stewardship_check:
      status: blocked
      findings:
        - code: MODULE_INTERFACE_REVIEW_REQUIRED
          severity: blocker
        - code: INTERFACE_CONTRACT_REVIEW_REQUIRED
          severity: blocker
  derived:
    stewardship_impact:
      domain_language_impact: none
      module_boundary_impact: unresolved
      interface_contract_impact: unresolved
      feedback_loop_status: defined
      future_change_risk: unresolved
      close_impact: blocks_close
expected_events:
  - prepare_write_blocked
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: VALIDATOR_FAILED
```

```yaml
scenario_id: STEWARDSHIP-domain-language-conflict-marks-design-stale-or-partial
initial_state:
  active_task:
    task_id: TASK-DOMAIN-TERM-001
    mode: work
    lifecycle_phase: ready
    active_change_unit_id: CU-DOMAIN-TERM-001
    gates:
      scope_gate: passed
      approval_gate: not_required
      decision_gate: not_required
      design_gate: passed
  active_change_unit:
    change_unit_id: CU-DOMAIN-TERM-001
    allowed_paths: ["src/billing/customer.ts"]
    baseline_ref: BASE-DOMAIN-TERM-001
    stewardship_refs:
      domain_terms: [TERM-CUSTOMER-001, TERM-CUSTOMER-002]
      module_map_items: [MOD-BILLING-001]
      interface_contracts: []
      feedback_loop_refs: [FBL-BILLING-001]
  owner_records:
    domain_terms:
      - domain_term_id: TERM-CUSTOMER-001
        term: Customer
        meaning_id: account_identity
        status: active
      - domain_term_id: TERM-CUSTOMER-002
        term: Customer
        meaning_id: billing_contact
        status: conflict
    module_map_items:
      - module_map_item_id: MOD-BILLING-001
        status: active
    feedback_loops:
      - feedback_loop_id: FBL-BILLING-001
        status: defined
  context_refs:
    - record_kind: projection
      record_id: NOTE-STALE-001
      freshness: stale
      claims:
        proposed_local_term:
          term: Customer
          meaning_id: billing_contact
input:
  task_id: TASK-DOMAIN-TERM-001
  change_unit_id: CU-DOMAIN-TERM-001
  intended_operation: "Use Customer in billing code based on an unreconciled note."
  intended_paths: ["src/billing/customer.ts"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: []
  baseline_ref: BASE-DOMAIN-TERM-001
action: prepare_write
expected_state:
  lifecycle_phase: blocked
  gates:
    design_gate: stale
  write_decision: blocked
  validators:
    codebase_stewardship_check:
      status: failed
      findings:
        - code: DOMAIN_LANGUAGE_CONFLICT
          severity: error
  canonical_terms_unchanged:
    - TERM-CUSTOMER-001
    - TERM-CUSTOMER-002
  derived:
    stewardship_impact:
      domain_language_impact: conflict
      module_boundary_impact: local
      interface_contract_impact: none
      feedback_loop_status: defined
      future_change_risk: visible
      close_impact: blocks_close
expected_events:
  - prepare_write_blocked
expected_artifacts: []
expected_projection:
  TASK: enqueued
  DOMAIN-LANGUAGE: stale_or_enqueued
expected_error:
  code: VALIDATOR_FAILED
```

```yaml
scenario_id: STEWARDSHIP-close-blocked-by-public-interface-future-change-risk
initial_state:
  active_task:
    task_id: TASK-PUBLIC-RISK-001
    mode: work
    lifecycle_phase: verifying
    active_change_unit_id: CU-PUBLIC-RISK-001
    gates:
      scope_gate: passed
      approval_gate: granted
      decision_gate: resolved
      design_gate: passed
      evidence_gate: sufficient
      verification_gate: passed
      qa_gate: not_required
      acceptance_gate: accepted
  active_change_unit:
    change_unit_id: CU-PUBLIC-RISK-001
    allowed_paths: ["src/reports/publicExport.ts"]
    stewardship_refs:
      domain_terms: [TERM-REPORT-001]
      module_map_items: [MOD-REPORTS-001]
      interface_contracts: [IFACE-PUBLIC-EXPORT-001]
      feedback_loop_refs: [FBL-REPORTS-001]
  owner_records:
    domain_terms:
      - domain_term_id: TERM-REPORT-001
        status: active
    module_map_items:
      - module_map_item_id: MOD-REPORTS-001
        public_boundary: true
    interface_contracts:
      - interface_contract_id: IFACE-PUBLIC-EXPORT-001
        compatibility_impact: breaking
        review_status: reviewed
    feedback_loops:
      - feedback_loop_id: FBL-REPORTS-001
        status: defined
  stewardship_findings:
    - finding_id: STEW-FIND-PUBLIC-RISK-001
      kind: future_change_risk
      close_relevant: true
      status: unresolved
      refs: [MOD-REPORTS-001, IFACE-PUBLIC-EXPORT-001]
  residual_risks:
    - risk_id: RISK-PUBLIC-FUTURE-001
      close_relevant: true
      visibility: visible
      accepted: false
      source_refs: [STEW-FIND-PUBLIC-RISK-001, IFACE-PUBLIC-EXPORT-001]
input:
  task_id: TASK-PUBLIC-RISK-001
  intent: complete
  requested_close_reason: completed_verified
  user_note: null
  superseded_by_task_id: null
action: close_task
expected_state:
  lifecycle_phase: waiting_user
  result: none
  gates:
    decision_gate: required
    design_gate: partial
    evidence_gate: sufficient
    verification_gate: passed
    acceptance_gate: accepted
  validators:
    codebase_stewardship_check:
      status: blocked
      findings:
        - code: STEWARDSHIP_FUTURE_CHANGE_RISK
          severity: blocker
    residual_risk_visibility_check:
      status: passed
  residual_risk_summary:
    status: visible
    visible_refs: [RISK-PUBLIC-FUTURE-001]
  close_blockers:
    - code: DECISION_REQUIRED
      related_refs:
        - record_kind: residual_risk
          record_id: RISK-PUBLIC-FUTURE-001
        - record_kind: interface_contract
          record_id: IFACE-PUBLIC-EXPORT-001
  decision_packet_candidate:
    decision_kind: residual_risk_acceptance
    topic: public_interface_future_change_risk
    affected_gates: [decision_gate, design_gate]
    residual_risk_refs: [RISK-PUBLIC-FUTURE-001]
    finding_refs: [STEW-FIND-PUBLIC-RISK-001]
  derived:
    stewardship_impact:
      domain_language_impact: none
      module_boundary_impact: public_boundary
      interface_contract_impact: breaking
      feedback_loop_status: defined
      future_change_risk: visible
      close_impact: requires_decision
expected_events:
  - close_requested
  - close_blocked
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: DECISION_REQUIRED
```

### Stewardship Catalog Entries

These remaining catalog entries are not fixture bodies. Each materialized fixture must drive the named Core action and assert validator results, gate changes, events, projections, and error code.

| Scenario ID | Core action | Required assertions |
|---|---|---|
| `STEWARDSHIP-shared-design-required-for-ambiguous-work` | `prepare_write` | Ambiguous `work` without a Shared Design record keeps or sets `design_gate=pending` or `partial`, reports `codebase_stewardship_check` failed or blocked with a shared-design finding, and returns `VALIDATOR_FAILED` or `DECISION_REQUIRED` according to whether user judgment can resolve it. |
| `STEWARDSHIP-feedback-loop-required-before-behavior-write` | `prepare_write` | Behavior-affecting write without a feedback-loop record keeps the write held, reports `feedback_loop_check` blocked, keeps `design_gate=pending` or `partial`, and does not rely on agent prose claiming a check will happen later. |
| `STEWARDSHIP-tdd-required-test-path-write-can-create-red-check` | `prepare_write` | When `tdd_trace_required` applies and the intended write is limited to the scoped test path that creates the failing RED check described by the RED target or plan, `prepare_write` may allow the write if all other scope, baseline, approval, autonomy, decision, and capability checks pass; the fixture must still assert the RED target or plan does not satisfy Evidence Manifest coverage and no GREEN evidence is credited until a later run records it. |
| `STEWARDSHIP-two-stage-review-display-is-not-authority` | `close_task` | Review Stage display text may summarize passed or failed findings, but close depends on canonical gates, evidence, residual-risk visibility, QA, acceptance, and close blockers; a passed display alone cannot close, accept risk, or create detached assurance. |

## Context Hygiene Fixture Examples

```yaml
scenario_id: CONTEXT-HYGIENE-stale-prd-not-treated-as-current-state
initial_state:
  active_task:
    task_id: TASK-SEARCH-001
    mode: work
    lifecycle_phase: ready
    active_change_unit_id: CU-SEARCH-001
    acceptance_criteria:
      - criteria_id: AC-01
        statement: "Server-side search filters archived records."
    gates:
      scope_gate: passed
      design_gate: passed
  active_change_unit:
    change_unit_id: CU-SEARCH-001
    allowed_paths: ["src/search/serverFilter.ts"]
    baseline_ref: BASE-CURRENT
  context_refs:
    - record_kind: projection
      record_id: PRD-2025-OLD
      label: "legacy search PRD"
      freshness: stale
      claims:
        acceptance_criteria:
          - "Client-side search filters archived records."
        allowed_paths: ["src/search/clientFilter.ts"]
input:
  task_id: TASK-SEARCH-001
  change_unit_id: CU-SEARCH-001
  intended_operation: "Implement the stale PRD client-side filter."
  intended_paths: ["src/search/clientFilter.ts"]
  intended_tools: ["edit"]
  intended_commands: []
  intended_network: []
  intended_secrets: []
  sensitive_categories: []
  baseline_ref: BASE-CURRENT
action: prepare_write
expected_state:
  lifecycle_phase: blocked
  gates:
    scope_gate: blocked
  write_decision: blocked
  canonical_acceptance_criteria:
    - criteria_id: AC-01
      statement: "Server-side search filters archived records."
  context_hygiene:
    stale_refs: [PRD-2025-OLD]
    stale_refs_treated_as: pull_only
  validators:
    context_hygiene_check:
      status: failed
  checks:
    scope_coverage: blocked
expected_events:
  - prepare_write_blocked
  - scope_required
expected_artifacts: []
expected_projection:
  TASK: enqueued
expected_error:
  code: SCOPE_VIOLATION
```

```yaml
scenario_id: CONTEXT-HYGIENE-resume-uses-current-state-not-chat-memory
initial_state:
  active_task:
    task_id: TASK-CONTEXT-001
    state_version: 88
    mode: work
    lifecycle_phase: verifying
    active_change_unit_id: CU-CONTEXT-001
    acceptance_criteria:
      - criteria_id: AC-CURRENT-001
        statement: "Server-side export preserves account filters."
    gates:
      scope_gate: passed
      decision_gate: pending
      evidence_gate: sufficient
      verification_gate: pending
  active_change_unit:
    change_unit_id: CU-CONTEXT-001
    allowed_paths: ["src/export/serverExport.ts"]
    baseline_ref: BASE-CURRENT-CTX
  journey_refs:
    journey_card_ref:
      record_kind: projection
      record_id: JOURNEY-CARD-CONTEXT-001
    journey_spine_entry_refs:
      - record_kind: journey_spine_entry
        record_id: JSE-CONTEXT-001
  evidence_refs:
    state_refs:
      - record_kind: evidence_manifest
        record_id: EVIDENCE-CONTEXT-001
      - record_kind: run
        record_id: RUN-CONTEXT-001
    artifact_refs:
      - artifact_id: ART-CONTEXT-TEST-001
        kind: log
  decision_packets:
    - decision_packet_id: DEC-CONTEXT-001
      decision_kind: verification_waiver
      status: pending_user
  projection_freshness:
    status: stale
    stale_refs:
      - record_kind: projection
        record_id: TASK-PROJECTION-OLD-001
  chat_memory_claims:
    - claim_id: CHAT-MEM-OLD-001
      freshness: stale
      claims:
        lifecycle_phase: executing
        active_change_unit_id: CU-OLD-CHAT-001
        allowed_paths: ["src/export/clientExport.ts"]
        evidence_gate: partial
input:
  task_id: TASK-CONTEXT-001
  focus: verification
  include_instruction_bundle: true
action: next
expected_state:
  state_version: 88
  no_state_mutation: true
  current_state_authority: current_task_record
  next_response:
    state:
      lifecycle_phase: verifying
      gates:
        evidence_gate: sufficient
        verification_gate: pending
    judgment_context:
      task_ref:
        record_kind: task
        record_id: TASK-CONTEXT-001
      journey_card:
        task_id: TASK-CONTEXT-001
        projection_freshness:
          status: stale
      relevant_refs:
        - record_kind: journey_spine_entry
          record_id: JSE-CONTEXT-001
        - record_kind: change_unit
          record_id: CU-CONTEXT-001
      evidence_refs:
        state_refs:
          - record_kind: evidence_manifest
            record_id: EVIDENCE-CONTEXT-001
          - record_kind: run
            record_id: RUN-CONTEXT-001
        artifact_refs:
          - artifact_id: ART-CONTEXT-TEST-001
      active_decision_packet_refs:
        - record_kind: decision_packet
          record_id: DEC-CONTEXT-001
      stale_or_missing_refs:
        - record_kind: projection
          record_id: TASK-PROJECTION-OLD-001
    instruction_bundle:
      relevant_refs:
        - record_kind: change_unit
          record_id: CU-CONTEXT-001
        - record_kind: evidence_manifest
          record_id: EVIDENCE-CONTEXT-001
      artifact_refs:
        - artifact_id: ART-CONTEXT-TEST-001
    pending_decisions:
      - record_kind: decision_packet
        record_id: DEC-CONTEXT-001
  context_hygiene:
    stale_chat_claim_refs: [CHAT-MEM-OLD-001]
    stale_chat_claim_treated_as: pull_only_non_authoritative
    did_not_replace_current_task_state: true
    did_not_satisfy_gates: true
  validators:
    context_hygiene_check:
      status: warning
expected_events: []
expected_artifacts: []
expected_projection: {}
expected_error: null
```

### Context Hygiene Catalog Entries

These catalog entries are not fixture bodies. Materialized fixtures, including the resume fixture above, prove behavior through Core responses and captured state, not by matching resume, status, or evaluator prose.

| Scenario ID | Core action | Required assertions |
|---|---|---|
| `CONTEXT-HYGIENE-stale-task-projection-cannot-authorize-write` | `prepare_write` | A stale `TASK` projection that lists broader paths or older acceptance criteria cannot authorize the write; current Change Unit scope and current Task state win, `context_hygiene_check` fails or warns, and the write returns `SCOPE_VIOLATION`, `BASELINE_STALE`, or `PROJECTION_STALE` according to the seeded state. |
| `CONTEXT-HYGIENE-resume-uses-current-state-not-chat-memory` | `next` | Resume reads current state, Journey refs, evidence refs, active Decision Packets, and projection freshness from Core; stale chat-memory claims are treated as non-authoritative input and do not mutate state or satisfy gates. |
| `CONTEXT-HYGIENE-evaluator-bundle-stale-evidence-blocks-verification` | `record_eval` | An evaluator bundle with stale or missing evidence refs cannot set detached verification passed; `verification_gate` remains pending or blocked, stale evidence refs are reported, and the fixture returns `EVIDENCE_INSUFFICIENT` or `VALIDATOR_FAILED`. |

### V1 Browser QA Capture Candidate Entries

These catalog entries are future candidates, not MVP fixture bodies or MVP smoke requirements. They become executable only after the Browser QA Capture capability profile, redaction policy, test environment, artifact retention, fixture target, and fallback semantics are defined.

| Scenario ID | Core action | Required assertions |
|---|---|---|
| `BROWSER-QA-capture-artifacts-attach-to-manual-qa` | `record_manual_qa` | A capable `T6 QA Capture` profile registers supported screenshot, `qa_capture`, log or console log, network trace, accessibility snapshot, or workflow recording artifacts; links them to the Manual QA record or Feedback Loop execution; applies redaction and retention policy; and updates `qa_gate` only through normal Manual QA result semantics. |
| `BROWSER-QA-capture-not-final-acceptance-or-detached-verification` | `record_manual_qa` or `record_eval` | Browser QA artifacts can support evidence, but they do not record final acceptance, do not replace human Manual QA judgment where required, and do not set `assurance_level=detached_verified` unless a separate Eval path satisfies independence requirements. |
| `BROWSER-QA-unsupported-surface-falls-back-to-human-notes` | `record_manual_qa` or `next` | A surface without browser capture capability reports the missing `T6` capability, recommends human Manual QA notes and manually supplied artifacts, and does not fail MVP smoke solely because automated browser capture is unavailable. |

## Fixture Suites

Minimum MVP suites:

- core: active status, advisor close, direct close, write gate, Write Authorization creation/required/invalid coverage, approval required and approval lifecycle retry, evidence insufficient, same-session verification guard, QA required, acceptance required, projection failure separation
- connector: capability profile, MCP unavailable hold, generated manifest drift, changed-path detection, artifact capture, fallback guarantee display, current Journey Card before significant resume, Decision Packet not broad approval, Autonomy Boundary breach routing
- connector guard/freeze: cooperative/detective freeze and guard display, careful-mode non-authority behavior; preventive `T4` pre-tool blocking only when a surface-specific fixture proves the hook, wrapper, sidecar, or permission layer can block the covered operation before execution
- agency: Decision Packet required for blocking product judgment, product trade-off write guard, AFK Autonomy Boundary stop conditions, known close-relevant residual-risk visibility before any successful close, `ResidualRiskSummary.status=none` for no known close-relevant risk, accepted Residual Risk refs whose risks were visible before acceptance for risk-accepted close, distinct approval/QA/acceptance judgments
- stewardship: shared design required, codebase stewardship close blockers, domain language conflicts, vertical slice or exception, feedback loop and TDD trace required or waived, public interface module/interface review, public interface stewardship close blocker, two-stage review display and close-blocker routing, Manual QA policy and waiver checks
- context-hygiene: current-state bundle, stale projection and stale PRD handling, stale `TASK` projection write guard, stale context pull-only behavior, evaluator bundle freshness, resume from current state rather than chat memory
- design-quality: policy-pack smoke coverage that composes agency, stewardship, context-hygiene, and close-impact validators without redefining kernel authority

Future v1 candidate suites:

- browser-qa-capture: declared `T6 QA Capture` support, redaction and retention policy, browser test environment, capture artifact mapping, Manual QA attachment, detached-verification boundary, final-acceptance boundary, and unsupported-surface fallback

Conformance output must include fixture id, pass/fail, observed state summary, observed events, artifact integrity result, projection freshness, and error code comparison.

## Metrics Boundary

Long-term operational metrics are derived analytics, not MVP-critical state or conformance requirements. Keep metrics such as approval turnaround, verification latency, projection stale duration, same-session guard frequency, and surface fallback rate in [Appendix C](appendix/C-later-roadmap.md) until a future version promotes them with fixtures and implementation ownership.
