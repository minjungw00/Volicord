# Conformance Fixtures Reference

## What this document helps you do

Use this reference to look up the three-layer boundary for Harness conformance material: documentation checks, active structured fixture drafts, and future runtime conformance. It explains what future conformance will prove, the active Kernel Smoke, MVP-1 user-loop, security/capability, and artifact/evidence draft families, exact structured fixture draft shape, future runner execution behavior, fixture assertion semantics, current-phase status, and the boundary to the future fixture catalog.

This is a lookup document for conformance authors, implementers, and maintainers. It is not an operator procedure; use [Operations And Conformance Reference](operations-and-conformance.md) for operator entrypoints and the `harness conformance run` overview.

This is reference documentation for future conformance work. The current repository is documentation-only and contains no runnable Harness Server conformance tests; current phase and handoff status are tracked in [Implementation Overview](../build/implementation-overview.md#documentation-acceptance-status).

## Read this when

- You are writing or reviewing the future fixture-based conformance design.
- You need the exact fixture body fields, fixture shorthand boundary, `ToolEnvelope` expansion convention, or runner isolation behavior.
- You need fixture assertion modes for response facts, Core state, storage rows, events, artifacts, blockers, errors, forbidden side effects, and projection facts when promoted.
- You need the active Kernel Smoke, MVP-1 User Work Loop, security/capability, or artifact/evidence fixture drafts, or the boundary between those drafts and the future fixture catalog.

## Before you read

Use [Operations And Conformance Reference](operations-and-conformance.md#conformance-run) for the conformance run entrypoint, suite-selection overview, docs-maintenance profile boundary, and operator procedures. Use [MVP API](api/mvp-api.md) and [API Schema Core](api/schema-core.md) for public request/response schemas, [Storage](storage.md) for storage layout and seed-loader owner values, [Core Model Reference](core-model.md) for state transition and stable event semantics, [Projection And Templates Reference](projection-and-templates.md) for projection freshness, [Design Quality Policies](design-quality-policies.md) for policy validator behavior, and [Agent Integration Reference](agent-integration.md) for connector conformance overview.

## Main idea

Today this document is a future conformance design, not a set of runnable tests. It defines behavior-example IDs and required behavior for later implementation planning; it does not create fixture files, runner code, generated outputs, runtime state, or a runnable Harness Server conformance suite. Do not create actual fixture files from these examples during the documentation-only phase.

Keep three layers separate:

- Documentation checks are read-only editorial checks over Markdown docs: link integrity, terminology consistency, stage boundaries, security wording, user-language checks, owner-boundary drift, and English/Korean parity. They may report Markdown drift, but they do not execute fixture actions, append `task_events`, create artifacts, refresh projections, create QA or acceptance state, affect close readiness, create implementation readiness, or create runtime results.
- Active MVP fixture drafts are compact structured design drafts for Engineering Checkpoint and MVP-1. They describe expected behavior through assertion fields but are not executable fixtures yet and are not generated runtime artifacts.
- Runtime conformance is future Harness Server implementation work. It applies to implemented Core/API/storage/surface behavior and is judged by executable fixtures and structured assertions, not documentation prose. Only after server implementation and fixture materialization will exact-shape fixtures run through Core or operator entrypoints and produce runtime pass/fail results.

The core model and small active MVP fixture drafts stay in this file. Detailed later scenarios stay in [Future Fixtures](../later/future-fixtures.md). This keeps Engineering Checkpoint Kernel Smoke and MVP-1 user-facing value understandable without making later catalog coverage look like an early implementation requirement.

After implementation begins, conformance will prove Harness behavior with executable fixtures. A passing runtime fixture will drive a Core or operator request and compare captured response facts, Core state, storage rows, events, artifacts, blockers, errors, and forbidden side effects against structured expectations.

Assertion authority is layered:

- Prose scenario descriptions, comments, rendered Markdown, Journey Card prose, status text, close report prose, and agent summaries are explanatory only.
- Captured response facts, Core state, storage rows, `task_events`, validator results, returned primary errors, structured blocker fields, and forbidden-side-effect checks are authoritative for fixture pass/fail.
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
- future fixture profiles by behavior proved, the reduced Engineering Checkpoint / MVP-1 structured drafts, and the reduced Kernel Smoke authoring queue
- current-phase status and the boundary between runtime conformance and docs-maintenance checks
- links to the future-oriented catalog without making its scenarios Engineering Checkpoint or MVP-1 requirements

## Not covered here

This reference does not own operator command procedures, docs-maintenance reporting, public MCP schemas, SQLite DDL, projection template bodies, policy contracts, or the compact future scenario inventory. Those remain with their owning Reference documents. Suite metadata, examples, and catalog rows here do not add fixture-body fields, public request fields, storage rows, projection kinds, or runtime implementation readiness.

## Conformance Navigation Map

| If you are looking for... | Go to |
|---|---|
| The exact fixture body fields | [Conformance Fixture Format](#conformance-fixture-format) |
| How a runner loads, seeds, executes, captures, and compares | [Conformance Execution](#conformance-execution) |
| Default comparison modes for `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, and `forbidden_side_effects` | [Fixture Assertion Semantics](#fixture-assertion-semantics) |
| Active structured fixture drafts | [Kernel Smoke Behavior Examples](#engineering-checkpoint-behavior-examples), [MVP-1 User Work Loop Behavior Examples](#mvp-1-user-work-loop-behavior-examples), [Security And Capability Behavior Examples](#security-and-capability-behavior-examples), and [Artifact And Evidence Behavior Examples](#artifact-and-evidence-behavior-examples) |
| Suite intent and authoring order | [Conformance staging](operations-and-conformance.md#conformance-staging), [Kernel Smoke Authoring Queue](#kernel-smoke-authoring-queue), and [Future Fixtures: Fixture Suites](../later/future-fixtures.md#fixture-suites) |
| Core model and current-phase boundary | [Core Conformance Model](#core-conformance-model) and [Fixture Current-Phase Status](#fixture-current-phase-status) |
| Future scenario inventory by concern | [Future Fixtures](../later/future-fixtures.md) |

## Core Conformance Model

The core conformance model defines what future runtime conformance proves and where assertion authority lives. A passing fixture proves behavior by driving one Core or operator request and comparing captured response facts, Core state, storage rows, events, artifacts, blockers, errors, and forbidden side effects with fixture expectations. It does not prove behavior by matching prose, generated Markdown, Journey Card text, status prose, close prose, or agent summaries.

Assertion types remain deliberately small:

- State and storage assertions compare Core-owned records, storage row effects, `task_events`, validator results, returned primary errors, structured blockers, owner refs, and state-version behavior.
- Artifact assertions compare registered artifact identity, owner links, `sha256`, `size_bytes`, `content_type`, `redaction_state`, relation owner, retention class, availability, and file-integrity facts where the scenario depends on evidence bytes.
- Projection assertions compare freshness, enqueue or job status, source-state-version display, readability, and availability only when projection support is in scope. They never replace Core state or satisfy authority, evidence, close, acceptance, or risk judgments.
- Error assertions compare the API-owned primary `ErrorCode` and optional details according to public schema precedence.

State and storage assertions answer "what did Core own after the request, and which durable row effects occurred?" Artifact assertions answer "what evidence bytes or metadata were safely registered and linked?" Projection assertions answer "is a derived readable view current, stale, available, failed, or queued?" These are separate assertion locations, and projection output must not substitute for state or artifact proof.

## Fixture Profiles By Proven Behavior

Fixture profiles are grouped by the behavior they prove, not by how polished the rendered output is. The profile name does not add fixture-body fields, does not require a renderer to be authoritative, and does not imply fixture files exist in this documentation-only repository.

The hardened local reference target is an umbrella target reached through Assurance Profile and Operations Profile. It is not a fifth fixture profile and must not be used as a suite name.

| Profile | Stage name | Behavior proved | Out of scope for that profile |
|---|---|---|---|
| Engineering Checkpoint fixtures, with Kernel Smoke as the authoring label | Engineering Checkpoint | The first executable authority loop: no-active-Task status, owner-valid setup/intake creating one active Task, active Change Unit requirement, in-scope/out-of-scope `prepare_write`, dry-run and replay behavior, single-use Write Authorization, `record_run` consumption and invalid-authorization blockers, minimal artifact metadata, evidence summary, close blockers, residual-risk visibility, and honest cooperative/detective guarantee display. | Ordinary natural-language intake quality, full user-loop judgment UX, full Evidence Manifest, projection renderer support, final-acceptance or residual-risk acceptance success semantics, Manual QA, detached verification, export/recover, release handoff, full conformance runner, broad future catalog coverage, hosted connector registry, cross-surface orchestration, preventive guard expansion, and broad operations. |
| MVP-1 User Work Loop fixtures | MVP-1 User Work Loop | Ordinary requests become tracked work without Harness vocabulary; focused user judgment, status next safe action, non-substitution boundaries for broad approval, sensitive approval, final acceptance, residual-risk acceptance, evidence, and proof that active MVP does not fabricate detached verification are visible through Core-owned state and structured responses. | Full agency assurance hardening, detached verification independence, full Manual QA matrix, stewardship policy suite, full TDD/module/interface/domain-language catalogs, full feedback-loop audits, export/recover, release handoff, broad connector ecosystem, hosted connector registry, cross-surface orchestration, and automation beyond the MVP-1 user-value path. |
| Assurance Profile fixtures | Assurance Profile | User-owned judgment, sensitive-action Approval, Write Authorization, Manual QA, verification, final acceptance, residual-risk acceptance, stewardship, design-quality, context-hygiene, TDD, and feedback-loop boundaries stay separate and fixture-proven through Core records. | Operator recovery/export completeness, release handoff, broad operations coverage, dashboard/hosted workflow UI, broad connector automation, and unproven preventive or isolated guarantee claims. |
| Operations Profile / promoted Roadmap fixtures | Operations Profile and Roadmap | Export/recover, artifact integrity, release handoff, operator readiness, reconcile, broader conformance coverage, and any promoted future higher guarantee level or automation profile. | Any stronger security, isolation, preventive guard, browser-capture, remote/shared MCP, or automation claim until owner docs define the mechanism and fixtures prove the covered behavior. |

## Active MVP Fixture Draft Families

These structured fixture drafts are the active future-authoring target for Engineering Checkpoint and MVP-1. They are not executable fixtures yet, not generated runtime artifacts, and not current pass/fail criteria. They use symbolic owner refs such as `TASK-1`, `CU-1`, and `WA-1` to show the expected record relationships; future materialized fixtures must replace those symbols with exact owner-schema payloads and public request shapes.

Every draft below asserts Core state, storage row effects, `task_events` families when stable owner events exist, artifact metadata, structured blockers, public errors, and forbidden side effects. No draft can pass because rendered Markdown, status prose, close prose, Journey Card text, report text, or an agent summary looks plausible.

<a id="engineering-checkpoint-behavior-examples"></a>

### Kernel Smoke Behavior Examples

Kernel Smoke is the narrow authoring label for the first executable authority loop. The drafts in this section are future fixture candidates, not current fixture files. "Intake" here means the owner-valid setup/intake path that creates one active Task for the smoke; ordinary natural-language intake quality remains MVP-1 coverage.

```yaml
scenario_id: MVP-ACTIVE-task-change-unit-setup
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: null, state_version: 1}
  surfaces:
    - {surface_id: reference-local-mcp, max_guarantee_level: detective, pre_tool_blocking_supported: false, isolation_supported: false}
request:
  tool: harness.intake
  payload:
    user_request: "Prepare a small documentation update."
    requested_mode: work
    initial_scope:
      included_paths: ["docs/en/reference/conformance-fixtures.md"]
      excluded_paths: ["server/runtime implementation"]
expected_response:
  result: created_active_task
  refs: {task_id: TASK-1, change_unit_id: CU-1}
  state_version: advanced
expected_state_changes:
  project_state: {active_task_id: TASK-1}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths_contains: ["docs/en/reference/conformance-fixtures.md"]}]
expected_storage_rows:
  tasks: {inserted: 1}
  change_units: {inserted: 1}
  project_state: {updated: 1}
  tool_invocations: {inserted: 1}
  write_authorizations: {inserted: 0}
  runs: {inserted: 0}
  artifacts: {inserted: 0}
expected_events:
  - event_family: owner-promoted Task setup event
  - event_family: owner-promoted Change Unit setup event
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - no Write Authorization is created
  - no Run, evidence summary, final acceptance, residual-risk acceptance, or close state is created
  - no rendered Markdown or generated projection is treated as authority
```

```yaml
scenario_id: MVP-ACTIVE-shaping-update-persists
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 2}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, current_goal_summary: "Draft docs update"}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, success_criteria: []}]
request:
  tool: harness.record_run
  payload:
    kind: shaping
    task_id: TASK-1
    change_unit_id: CU-1
    product_write: false
    write_authorization_id: null
    shaping_update:
      task_update: {current_goal_summary: "Add structured non-executable fixture drafts"}
      change_unit_update: {success_criteria: ["Drafts assert Core state and storage effects"]}
      confirmed_facts: ["No conformance runner exists yet"]
expected_response:
  result: recorded
  refs: {run_id: RUN-SHAPE-1}
  state_version: advanced
expected_state_changes:
  tasks: [{task_id: TASK-1, current_goal_summary: "Add structured non-executable fixture drafts"}]
  change_units: [{change_unit_id: CU-1, success_criteria_contains: ["Drafts assert Core state and storage effects"]}]
expected_storage_rows:
  runs: {inserted: 1, row_filter: {kind: shaping, product_write: false}}
  tasks: {updated: 1}
  change_units: {updated: 1}
  tool_invocations: {inserted: 1}
  write_authorizations: {inserted: 0, consumed: 0}
expected_events:
  - event_family: owner-promoted Run recording event
  - event_family: owner-promoted shaping/state update event
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - no product-file Write Authorization is required or created
  - no artifact, Evidence Manifest, projection job, final acceptance, residual-risk acceptance, or close state is created
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-allowed-authorization
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 3}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 3}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths: ["docs/en/reference/conformance-fixtures.md"]}]
request:
  tool: harness.prepare_write
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    dry_run: false
    idempotency_key: IDEMP-PW-1
    expected_state_version: 3
    intended_operation: edit_file
    intended_paths: ["docs/en/reference/conformance-fixtures.md"]
    product_write: true
expected_response:
  decision: allowed
  write_authorization_ref: {record_kind: write_authorization, record_id: WA-1}
  authorization_effect: created
  primary_error: null
expected_state_changes:
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 3, consumed_by_run_id: null}]
expected_storage_rows:
  write_authorizations: {inserted: 1, updated: 0}
  tool_invocations: {inserted: 1, request_hash: canonical_hash_of_request}
  blockers: {inserted: 0}
  runs: {inserted: 0}
expected_events:
  - event_family: owner-promoted prepare_write allowed or Write Authorization created event
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - no Run is recorded
  - no artifact or evidence sufficiency is created by `prepare_write`
  - Write Authorization is not described as OS permission, sandboxing, preventive blocking, or isolation
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-blocked-no-authorization
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 4}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 4}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths: ["docs/en/reference/conformance-fixtures.md"]}]
request:
  tool: harness.prepare_write
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    dry_run: false
    idempotency_key: IDEMP-PW-BLOCKED
    expected_state_version: 4
    intended_operation: edit_file
    intended_paths: ["docs/en/reference/storage.md"]
    product_write: true
expected_response:
  decision: blocked
  write_authorization_ref: null
  primary_error: SCOPE_VIOLATION
expected_state_changes:
  write_authorizations: []
  current_scope_unchanged: true
expected_storage_rows:
  write_authorizations: {inserted: 0}
  tool_invocations: {inserted: 0}
  runs: {inserted: 0}
  artifacts: {inserted: 0}
expected_events: []
expected_artifacts: []
expected_blockers:
  - {blocker_kind: scope, code: SCOPE_VIOLATION, affected_paths: ["docs/en/reference/storage.md"]}
expected_errors:
  - {code: SCOPE_VIOLATION}
forbidden_side_effects:
  - no consumable Write Authorization row is created
  - no replay row reserves the idempotency key for the pre-commit failure
  - no projection job, artifact, Run, or evidence state is created
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-idempotent-replay
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 5}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 5}]
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 5, consumed_by_run_id: null}]
  tool_invocations: [{tool_name: harness.prepare_write, idempotency_key: IDEMP-PW-REPLAY, request_hash: HASH-A, response_ref: WA-1}]
request:
  tool: harness.prepare_write
  payload:
    idempotency_key: IDEMP-PW-REPLAY
    canonical_request_hash: HASH-A
    task_id: TASK-1
    change_unit_id: CU-1
    intended_paths: ["docs/en/reference/conformance-fixtures.md"]
expected_response:
  decision: allowed
  write_authorization_ref: {record_kind: write_authorization, record_id: WA-1}
  authorization_effect: returned
  replayed: true
expected_state_changes:
  write_authorizations: [{write_authorization_id: WA-1, status: active, consumed_by_run_id: null}]
  state_version_advanced: false
expected_storage_rows:
  write_authorizations: {inserted: 0, updated: 0}
  tool_invocations: {inserted: 0, updated: 0}
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - no duplicate Write Authorization is created
  - no duplicate event, artifact, projection job, or state-version increment is produced
```

```yaml
scenario_id: MVP-ACTIVE-idempotency-key-hash-conflict
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 6}
  tool_invocations: [{tool_name: harness.prepare_write, idempotency_key: IDEMP-PW-CONFLICT, request_hash: HASH-A, response_ref: WA-1}]
request:
  tool: harness.prepare_write
  payload:
    idempotency_key: IDEMP-PW-CONFLICT
    canonical_request_hash: HASH-B
    task_id: TASK-1
    change_unit_id: CU-1
    intended_paths: ["docs/en/reference/operations-and-conformance.md"]
expected_response:
  result: error
  primary_error: STATE_CONFLICT
  replayed: false
expected_state_changes:
  state_version_advanced: false
  current_records_changed: false
expected_storage_rows:
  tool_invocations: {inserted: 0, updated: 0, original_request_hash_preserved: HASH-A}
  write_authorizations: {inserted: 0, updated: 0}
  runs: {inserted: 0}
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors:
  - {code: STATE_CONFLICT, reason: same_idempotency_key_different_hash}
forbidden_side_effects:
  - no merged response fields or owner relations from the conflicting request
  - no artifact, event, projection job, Run, blocker, or replay row is created for the conflict
```

```yaml
scenario_id: MVP-ACTIVE-record-run-consumes-authorization
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 7}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 7}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths: ["docs/en/reference/conformance-fixtures.md"]}]
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 7, consumed_by_run_id: null, attempt_scope_paths: ["docs/en/reference/conformance-fixtures.md"]}]
request:
  tool: harness.record_run
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-RUN-1
    expected_state_version: 7
    kind: implementation
    product_write: true
    write_authorization_id: WA-1
    observed_changes: {changed_paths: ["docs/en/reference/conformance-fixtures.md"]}
expected_response:
  result: recorded
  refs: {run_id: RUN-1, write_authorization_id: WA-1}
  primary_error: null
expected_state_changes:
  runs: [{run_id: RUN-1, task_id: TASK-1, change_unit_id: CU-1, product_write: true, write_authorization_id: WA-1}]
  write_authorizations: [{write_authorization_id: WA-1, status: consumed, consumed_by_run_id: RUN-1}]
expected_storage_rows:
  runs: {inserted: 1}
  write_authorizations: {updated: 1, inserted: 0}
  tool_invocations: {inserted: 1}
expected_events:
  - event_family: owner-promoted Run recorded event
  - event_family: owner-promoted Write Authorization consumed event
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - authorization is consumed exactly once
  - chat or tool prose is not treated as authority
  - no final acceptance, residual-risk acceptance, or close state is created
```

```yaml
scenario_id: MVP-ACTIVE-record-run-missing-authorization-blocked
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 8}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 8}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths: ["docs/en/reference/conformance-fixtures.md"]}]
request:
  tool: harness.record_run
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-RUN-MISSING-WA
    expected_state_version: 8
    kind: implementation
    product_write: true
    write_authorization_id: null
    observed_changes: {changed_paths: ["docs/en/reference/conformance-fixtures.md"]}
expected_response:
  result: blocked
  primary_error: WRITE_AUTHORIZATION_REQUIRED
expected_state_changes:
  runs: []
  write_authorizations: []
  state_version_advanced: false
expected_storage_rows:
  runs: {inserted: 0}
  write_authorizations: {updated: 0, inserted: 0}
  tool_invocations: {inserted: 0}
  evidence_summaries: {inserted: 0, updated: 0}
expected_events: []
expected_artifacts: []
expected_blockers:
  - {blocker_kind: write_compatibility, code: WRITE_AUTHORIZATION_REQUIRED}
expected_errors:
  - {code: WRITE_AUTHORIZATION_REQUIRED}
forbidden_side_effects:
  - no Run, artifact link, evidence update, projection job, or replay row is committed
  - no authorization is fabricated or consumed
```

```yaml
scenario_id: MVP-ACTIVE-record-run-observed-out-of-scope
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 9}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 9}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths: ["docs/en/reference/conformance-fixtures.md"]}]
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 9, consumed_by_run_id: null, attempt_scope_paths: ["docs/en/reference/conformance-fixtures.md"]}]
request:
  tool: harness.record_run
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-RUN-SCOPE-VIOLATION
    expected_state_version: 9
    kind: implementation
    product_write: true
    write_authorization_id: WA-1
    observed_changes: {changed_paths: ["docs/en/reference/storage.md"]}
expected_response:
  result: blocked
  primary_error: SCOPE_VIOLATION
expected_state_changes:
  write_authorizations: [{write_authorization_id: WA-1, status: active, consumed_by_run_id: null}]
  runs: []
expected_storage_rows:
  runs: {inserted: 0}
  write_authorizations: {updated: 0}
  tool_invocations: {inserted: 0}
  blockers: {inserted_or_reported: [{blocker_kind: scope, code: SCOPE_VIOLATION}]}
expected_events: []
expected_artifacts: []
expected_blockers:
  - {blocker_kind: scope, code: SCOPE_VIOLATION, observed_paths: ["docs/en/reference/storage.md"]}
expected_errors:
  - {code: SCOPE_VIOLATION}
forbidden_side_effects:
  - invalid authorization is not marked consumed
  - out-of-scope observation is not completion evidence
  - no final acceptance, residual-risk acceptance, or close readiness is created
```

<a id="mvp-1-user-work-loop-behavior-examples"></a>

### MVP-1 User Work Loop Behavior Examples

MVP-1 behavior examples describe user-visible Harness value without growing into the broad assurance or operations catalog. If future fixtures materialize these drafts, they may use exactly `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, and `harness.close_task` where those methods are active for the stage. A separate `harness.next` fixture belongs to later/compatibility material.

```yaml
scenario_id: MVP-ACTIVE-evidence-summary-insufficient
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 10}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1}]
  evidence_summaries: [{evidence_summary_id: EVID-1, task_id: TASK-1, status: partial, required_refs_missing: ["ART-REQ-1"]}]
request:
  tool: harness.status
  payload:
    task_id: TASK-1
expected_response:
  result: ok
  evidence_summary: {status: partial, sufficient: false, missing_refs: ["ART-REQ-1"]}
  next_actions_contains: ["record missing evidence"]
expected_state_changes:
  state_version_advanced: false
  evidence_summaries: [{evidence_summary_id: EVID-1, status: partial}]
expected_storage_rows:
  tasks: {inserted: 0, updated: 0}
  evidence_summaries: {inserted: 0, updated: 0}
  tool_invocations: {inserted: 0}
expected_events: []
expected_artifacts: []
expected_blockers:
  - {blocker_kind: evidence, code: EVIDENCE_INSUFFICIENT, related_refs: ["EVID-1"]}
expected_errors: []
forbidden_side_effects:
  - status read does not create evidence, artifacts, events, acceptance, risk acceptance, or close state
  - Markdown evidence-list prose does not repair the missing ref
```

```yaml
scenario_id: MVP-ACTIVE-evidence-summary-sufficient
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 11}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 11}]
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 11, consumed_by_run_id: null}]
  staged_artifacts: [{staged_uri: staged://fixture/test-output.txt, sha256: SHA256-1, size_bytes: 128, content_type: text/plain, redaction_state: visible}]
request:
  tool: harness.record_run
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-RUN-EVIDENCE
    expected_state_version: 11
    kind: implementation
    product_write: true
    write_authorization_id: WA-1
    observed_changes: {changed_paths: ["docs/en/reference/conformance-fixtures.md"]}
    artifact_inputs:
      - {staged_uri: staged://fixture/test-output.txt, relation: {record_kind: run, record_id: RUN-1}}
    evidence_updates: {claim: "fixture drafts added", required_artifact_refs: ["ART-1"]}
expected_response:
  result: recorded
  refs: {run_id: RUN-1, evidence_summary_id: EVID-1}
  registered_artifacts: [{artifact_id: ART-1}]
expected_state_changes:
  runs: [{run_id: RUN-1, product_write: true}]
  write_authorizations: [{write_authorization_id: WA-1, status: consumed, consumed_by_run_id: RUN-1}]
  evidence_summaries: [{evidence_summary_id: EVID-1, status: sufficient, artifact_refs: ["ART-1"]}]
expected_storage_rows:
  runs: {inserted: 1}
  artifacts: {inserted: 1}
  artifact_links: {inserted: 1}
  evidence_summaries: {inserted_or_updated: 1}
  write_authorizations: {updated: 1}
  tool_invocations: {inserted: 1}
expected_events:
  - event_family: owner-promoted Run recording event
  - event_family: owner-promoted evidence summary update event
expected_artifacts:
  - {artifact_id: ART-1, sha256: SHA256-1, size_bytes: 128, content_type: text/plain, redaction_state: visible, relation_owner: {record_kind: run, record_id: RUN-1}}
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - evidence sufficiency is derived from registered refs, not prose
  - no full Evidence Manifest, Manual QA, detached verification, final acceptance, or residual-risk acceptance is created
```

```yaml
scenario_id: MVP-ACTIVE-final-acceptance-missing-close-blocker
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 12}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, requires_final_acceptance: true}]
  evidence_summaries: [{evidence_summary_id: EVID-1, task_id: TASK-1, status: sufficient}]
  user_judgments: []
request:
  tool: harness.close_task
  payload:
    task_id: TASK-1
    idempotency_key: IDEMP-CLOSE-MISSING-ACCEPTANCE
    expected_state_version: 12
    intent: complete
expected_response:
  result: blocked
  primary_error: ACCEPTANCE_REQUIRED
  terminal: false
expected_state_changes:
  tasks: [{task_id: TASK-1, lifecycle_phase: active}]
expected_storage_rows:
  tasks: {updated_terminal: 0}
  blockers: {inserted_or_reported: [{blocker_kind: final_acceptance, code: ACCEPTANCE_REQUIRED}]}
  tool_invocations: {inserted: 1, only_if_committed_blocked_close_is_owner_enabled: true}
expected_events:
  - event_family: owner-promoted close blocked event, only_if_committed_blocked_close_is_owner_enabled: true
expected_artifacts: []
expected_blockers:
  - {blocker_kind: final_acceptance, code: ACCEPTANCE_REQUIRED, required_judgment_kind: final_acceptance}
expected_errors:
  - {code: ACCEPTANCE_REQUIRED}
forbidden_side_effects:
  - Task is not marked terminal
  - no final_acceptance user judgment is fabricated
  - no residual-risk acceptance, evidence, artifact, Manual QA, detached verification, or generated close report is created
```

```yaml
scenario_id: MVP-ACTIVE-residual-risk-visible-not-accepted-blocker
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 13}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1}]
  evidence_summaries: [{evidence_summary_id: EVID-1, task_id: TASK-1, status: sufficient}]
  blockers: [{blocker_id: BLK-RISK-1, task_id: TASK-1, blocker_kind: residual_risk_acceptance, visible_to_user: true, status: open}]
  user_judgments: []
request:
  tool: harness.close_task
  payload:
    task_id: TASK-1
    idempotency_key: IDEMP-CLOSE-RISK-NOT-ACCEPTED
    expected_state_version: 13
    intent: complete
expected_response:
  result: blocked
  primary_error: DECISION_REQUIRED
  residual_risk_state: {visible: true, accepted: false}
expected_state_changes:
  tasks: [{task_id: TASK-1, lifecycle_phase: active}]
  blockers: [{blocker_id: BLK-RISK-1, blocker_kind: residual_risk_acceptance, status: open}]
expected_storage_rows:
  tasks: {updated_terminal: 0}
  user_judgments: {inserted: 0}
  blockers: {updated_or_reported: [{blocker_kind: residual_risk_acceptance}]}
expected_events:
  - event_family: owner-promoted close blocked event, only_if_committed_blocked_close_is_owner_enabled: true
expected_artifacts: []
expected_blockers:
  - {blocker_kind: residual_risk_acceptance, code: DECISION_REQUIRED, required_judgment_kind: residual_risk_acceptance, related_refs: ["BLK-RISK-1"]}
expected_errors:
  - {code: DECISION_REQUIRED}
forbidden_side_effects:
  - visible risk is not treated as accepted risk
  - no final acceptance, detached verification, Manual QA, rich Residual Risk record, or close state is fabricated
```

```yaml
scenario_id: MVP-ACTIVE-accepted-risk-close
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 14}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1}]
  evidence_summaries: [{evidence_summary_id: EVID-1, task_id: TASK-1, status: sufficient}]
  blockers: [{blocker_id: BLK-RISK-1, task_id: TASK-1, blocker_kind: residual_risk_acceptance, visible_to_user: true, status: resolved}]
  user_judgments:
    - {user_judgment_id: UJ-RISK-1, task_id: TASK-1, judgment_kind: residual_risk_acceptance, status: resolved, accepted_risks: ["BLK-RISK-1"]}
request:
  tool: harness.close_task
  payload:
    task_id: TASK-1
    idempotency_key: IDEMP-CLOSE-ACCEPTED-RISK
    expected_state_version: 14
    intent: completed_with_risk_accepted
expected_response:
  result: closed
  close_reason: completed_with_risk_accepted
  accepted_risk_refs: [{record_kind: user_judgment, record_id: UJ-RISK-1}]
  primary_error: null
expected_state_changes:
  tasks: [{task_id: TASK-1, lifecycle_phase: terminal, result: passed}]
  blockers: [{blocker_id: BLK-RISK-1, status: resolved}]
expected_storage_rows:
  tasks: {updated_terminal: 1}
  user_judgments: {inserted: 0, matched_existing: ["UJ-RISK-1"]}
  blockers: {updated: 1}
  tool_invocations: {inserted: 1}
expected_events:
  - event_family: owner-promoted successful close event
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - accepted risk does not create detached verification, Manual QA, final acceptance, Approval, or assurance upgrade
  - no standalone active-MVP residual_risk row is required
  - no generated close report is treated as close authority
```

```yaml
scenario_id: MVP-ACTIVE-display-label-not-canonical
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 15}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1}]
request:
  tool: harness.request_user_judgment
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-JUDGMENT-LABEL
    expected_state_version: 15
    judgment_kind: product_decision
    presentation: short
    locale: ko
    question: "이 제품 동작을 선택할까요?"
expected_response:
  result: requested
  user_judgment_ref: {record_kind: user_judgment, record_id: UJ-1}
  rendered_display_label: "제품 판단"
expected_state_changes:
  user_judgments: [{user_judgment_id: UJ-1, judgment_kind: product_decision, presentation: short, status: pending_user}]
expected_storage_rows:
  user_judgments: {inserted: 1, forbidden_columns: ["display_label"]}
  blockers: {inserted_or_updated: [{blocker_kind: user_judgment, required_judgment_kind: product_decision}]}
  tool_invocations: {inserted: 1}
expected_events:
  - event_family: owner-promoted user judgment requested event
expected_artifacts: []
expected_blockers:
  - {blocker_kind: user_judgment, required_judgment_kind: product_decision, canonical_key: product_decision}
expected_errors: []
forbidden_side_effects:
  - no canonical state, blocker key, gate key, or storage identity uses "제품 판단" or `display_label`
  - no product decision is resolved by requesting it
  - no Write Authorization, final acceptance, residual-risk acceptance, evidence, artifact, or close state is created
```

<a id="security-and-capability-behavior-examples"></a>

### Security And Capability Behavior Examples

Security and capability examples prove honest local capability display and unavailable-path behavior. They do not create stronger guarantees by naming them. Active MVP drafts may assert `CAPABILITY_INSUFFICIENT`, cooperative/detective profile facts, or no-authority unavailable responses, but preventive guard expansion and isolated profiles remain later/profile or Roadmap material.

<a id="artifact-and-evidence-behavior-examples"></a>

### Artifact And Evidence Behavior Examples

Artifact examples prove registered bytes and metadata, not report wording. They apply where the active stage uses artifact refs or evidence summaries; broader export non-leakage remains later/profile catalog material.

```yaml
scenario_id: MVP-ACTIVE-raw-secret-artifact-blocked
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 16}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 16}]
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 16, consumed_by_run_id: null}]
  staged_artifacts:
    - {staged_uri: staged://fixture/raw-secret.txt, content_class: raw_secret, redaction_state: blocked}
request:
  tool: harness.record_run
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-RUN-SECRET-BLOCKED
    expected_state_version: 16
    kind: implementation
    product_write: true
    write_authorization_id: WA-1
    observed_changes: {changed_paths: ["docs/en/reference/conformance-fixtures.md"]}
    artifact_inputs:
      - {staged_uri: staged://fixture/raw-secret.txt, relation: {record_kind: run, record_id: RUN-SECRET-1}}
expected_response:
  result: blocked
  primary_error: ARTIFACT_MISSING
  registered_artifacts: []
expected_state_changes:
  write_authorizations: [{write_authorization_id: WA-1, status: active, consumed_by_run_id: null}]
  evidence_summaries: [{task_id: TASK-1, status: blocked, reason: artifact_redaction_blocked}]
expected_storage_rows:
  artifacts: {inserted: 0, raw_bytes_stored: false}
  artifact_links: {inserted: 0}
  runs: {inserted: 0}
  write_authorizations: {updated: 0}
  evidence_summaries: {inserted_or_updated: 1, status: blocked}
expected_events: []
expected_artifacts:
  - {artifact_id: null, redaction_state: blocked, raw_secret_value_asserted: false}
expected_blockers:
  - {blocker_kind: artifact_availability, code: ARTIFACT_MISSING, reason: raw_secret_blocked}
expected_errors:
  - {code: ARTIFACT_MISSING}
forbidden_side_effects:
  - raw secret or PII bytes are not stored, asserted, rendered, exported, or copied into a generated report
  - blocked artifact input does not satisfy evidence, QA, detached verification, final acceptance, residual-risk acceptance, or close
  - authorization is not consumed by a blocked artifact attempt
```

### Later/Profile Fixture Boundary

Detailed clarification catalogs, later-profile verification, full Evidence Manifest cases, Manual QA matrices, export non-leakage, browser QA capture, full operations recovery/export, broad connector conformance, preventive guard expansion, and isolated security profiles remain later/profile or Roadmap material unless an owner promotes a narrower fixture with stage impact and proof expectations. Listing a family in [Future Fixtures](../later/future-fixtures.md) does not make it an Engineering Checkpoint or MVP-1 requirement.

## Conformance Fixture Format

Future runtime conformance is fixture-based after Harness Server implementation and fixture materialization. A behavior-example table is not enough; each materialized test fixture must drive one request and assert structured response facts, Core state changes, storage rows, events, artifacts, blockers, errors, and forbidden side effects.

Each structured fixture draft must include this shape:

```yaml
scenario_id: string
initial_state: object
request: object
expected_response: object
expected_state_changes: object
expected_storage_rows: object
expected_events: object[]
expected_artifacts: object[]
expected_blockers: object[]
expected_errors: object[]
forbidden_side_effects: string[] | object[]
```

Fixture shape summary: suite metadata can group fixtures, but the fixture body keeps one exact request-and-expectation shape for future executable conformance. The YAML block above is the contract summary.

Future fixture files and suite catalogs may carry metadata outside the fixture body. The fixture body itself uses only the fields above so conformance runners can compare behavior consistently. Do not add fixture-body fields for suite delivery stage, assertion mode, docs-maintenance result, prose status, rendered Markdown, or authoring notes; those belong in suite catalog metadata, docs-maintenance reports, display owners, or surrounding documentation.

Fixture body type notation follows the API [Schema notation convention](api/schema-core.md#schema-notation-convention). All top-level fixture body fields above are required. Use `{}` or `[]` when the fixture intentionally supplies an empty object, object map, or array; omitting a required top-level field is an invalid fixture body, not "not asserted." For Engineering Checkpoint and MVP-1 active drafts, projection rendering is normally absent; if a later promoted owner requires projection freshness, assert the Core/storage fact in `expected_state_changes.checks`, `expected_storage_rows.projection_jobs`, or another owner-defined structured location, not by matching rendered Markdown.

For an MCP tool request, future executable fixture `request.tool` names the public tool or operator action and `request.payload` is the tool's public request payload as defined by the API docs. The runner must validate `request.payload` against the request schema for `request.tool`, including `envelope: ToolEnvelope` when that schema requires it. Drafts in this document may omit `ToolEnvelope` only under this envelope-expansion convention: before validation, canonicalization, request hashing, or Core execution, the runner supplies a deterministic valid envelope from `initial_state`, suite defaults, and fixture metadata. The expanded request is what Core receives. This convention does not add fixture fields, change the fixture body shape, or create an alternate request schema.

Fixture shorthand is not a second API. In the main Engineering Checkpoint / MVP-1 path, shorthand may compact only `initial_state` seeding, symbolic owner refs in non-executable drafts, or suite catalog metadata while preserving owner-defined records and public schemas. Public mutations must use the documented public request branch for the selected `request.tool` under `request.payload` after any `ToolEnvelope` expansion. Later-profile shorthand details belong in [Future Fixtures: Later-Profile Fixture Shorthand Notes](../later/future-fixtures.md#later-profile-fixture-shorthand-notes) and are not active requirements for Engineering Checkpoint or MVP-1.

Future executable fixtures that seed `write_authorizations` must produce valid stored rows. Each seeded authorization row must include `basis_state_version` explicitly, or the runner must derive it from the seeded affected-scope state version for the row's Task before inserting into `state.sqlite`. This is a storage-loader derivation rule only; it does not add fixture top-level fields or change the fixture body shape. Partial `expected_state_changes.write_authorizations` or `expected_storage_rows.write_authorizations` assertions may omit `basis_state_version` unless the fixture is testing idempotent replay, stale detection, expiry, or audit behavior. `basis_state_version` is the `decision=allowed` basis, not the resulting `ToolResponseBase.state_version`. Fixture loaders must not seed `blocked`, `approval_required`, `decision_required`, or `state_conflict` outcomes as `write_authorizations` rows; those outcomes use response decisions, blockers, validator findings, or errors.

Suite catalog metadata is not passed to Core and is not part of a fixture body. It can group exact-shape fixtures by suite, delivery stage, and tags:

```yaml
suite: agency
earliest_delivery_stage: "Assurance Profile"
tags: [decision-gate, residual-risk, autonomy-boundary]
fixtures:
  - AGENCY-user-judgment-required-before-product-tradeoff-write
  - AGENCY-residual-risk-visible-before-acceptance
```

Runners may use this metadata to choose, order, or report suites. Core receives only `request.tool` and public `request.payload` after any documented envelope expansion; metadata must not change seed expansion, fixture comparison semantics, tool request schemas, or expected owner records.

## Conformance Execution

Future `harness conformance run` will execute fixtures through the same Core entrypoints used by MCP tools and operator commands. It must not assert behavior by inspecting prose output alone.

Future runtime fixture execution semantics:

1. Load fixture YAML files and validate the exact fixture body shape.
2. Create a fresh fixture-only runtime home and temporary Product Repository for the fixture, unless the fixture explicitly targets an existing read-only sample. This fixture isolation is test hygiene for deterministic comparison; it is not an `isolated` guarantee level, OS sandboxing, permission isolation, or tamper-proof storage claim. The runner must not reuse the developer's real Harness Runtime Home or Product Repository for state-changing fixture execution.
3. Seed `registry.sqlite`, `project.yaml`, `state.sqlite`, artifact files, projection files when the fixture requires them, and connector manifests from `initial_state`.
4. Execute `request.tool` through Core. MCP tool actions use the public request schema; after any documented `ToolEnvelope` expansion, fixture `request.payload` must be the same request payload a surface would send to that MCP tool. Operator actions such as `projection_refresh`, `doctor_surface`, `recover`, and `artifacts_check` use the operator semantics in [Operations And Conformance Reference](operations-and-conformance.md).
5. Capture returned response facts, resulting state summaries, storage effects, appended owner events, validator results when emitted, artifact registry/file integrity, structured blockers, projection job status when relevant, reconcile items when relevant, and returned error code.
6. Compare the captured results with `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, and `forbidden_side_effects`; empty expected sections mean the fixture asserts no relevant effect for that section.
7. Report fixture id, pass/fail, observed response/state/storage/event/artifact/blocker/error summary, projection freshness when relevant, and forbidden-side-effect comparison.

Runner sequence summary: the numbered sequence above is the contract summary. A future runner loads an exact fixture body, seeds a fixture-only runtime home, executes the request through Core, compares response/state/storage/events/artifacts/blockers/errors/forbidden side effects, and emits a report.

When a fixture `request.payload` includes `expected_state_version`, the runner compares it according to the Core-resolved primary Task, not only `ToolEnvelope.task_id`. Primary Task resolution order is tool-specific `task_id`, `ToolEnvelope.task_id`, then active Task resolution. Task-scoped actions compare against the seeded or Core-resolved primary Task State Version; project-scoped actions with no resolved primary Task compare against the Project State Version. Captured response, `EventRef.state_version`, and `task_events.state_version` values are compared as resulting affected-scope versions. Read-only fixtures may assert the unchanged version for the primary read scope. This clarifies comparison semantics without changing fixture body shape.

A stale `expected_state_version` fixture is a stale-authority test, not only a concurrent-write test. Exact idempotent replay is the exception: when a committed replay row exists and the canonical request hash matches, the fixture should assert the original committed response is returned and no current state-version freshness check is re-run. When no replay row exists and a state-changing action conflicts before commit, the fixture should assert that no current records changed, no `task_events` were appended, no artifacts were registered, no projection jobs were enqueued, and no `tool_invocations` replay row was created for the conflicting request unless an owner document explicitly defines a different recovery action. When the same key is reused with a changed canonical request hash, the fixture should assert `STATE_CONFLICT`, preserved original replay row, and no merged artifacts, events, projection jobs, response fields, or owner relations. For `dry_run=true`, fixtures should assert that diagnostics or `would_create` effects are returned without current records, `task_events`, artifacts, consumable Write Authorizations, projection jobs, or `tool_invocations` replay rows, and that the key is not reserved for later non-dry-run use. Replayed `prepare_write` must not create a duplicate authorization; replayed `record_run` must not consume authorization twice.

Fixture execution should be deterministic. Network access, wall-clock-sensitive expiry, and external tool output must be stubbed or represented as seeded fixture inputs unless a suite explicitly declares itself an integration smoke.

Fixture isolation is part of the pass condition. A fixture may seed files into its temporary Product Repository and runtime home, execute one Core or operator action there, and compare the captured result. This does not upgrade the product guarantee level. The fixture must not depend on existing local runtime records, generated operational files, or prose reports from a previous run.

Seed validation happens before action execution, and captured-state validation happens after action execution. Both sides of the comparison use owner-defined state loaders and value sets rather than fixture-local string labels.

Conformance runners must seed and inspect JSON `TEXT` fields through the same Core storage loaders used by MCP tools and operator commands. A fixture with malformed JSON or schema-incompatible JSON in `initial_state` must surface invalid state, or a repairable state issue when the fixture action is a recovery path and safe reconstruction is possible. The runner must not skip shape validation by treating JSON fields as opaque strings, and this expectation does not change the fixture body shape.

Conformance runners must also seed and inspect status-like `TEXT` fields through the owner-bound hardening map in [Storage](storage.md#canonical-enum-hardening). For the main Engineering Checkpoint / MVP-1 path, fixture seed loaders validate only the owner values actually present in the active stage's seeded records, and artifact/ref enum assertions use the API [stage-specific active value sets](api/schema-core.md#stage-specific-active-value-sets). Examples include registry/project surface guarantee, Run kind/status, Write Authorization status/guarantee, Approval status when that owner path is active, minimal evidence summary coverage/status when evidence support is active, residual-risk visibility/status when risk visibility is active, projection job kind/status when projection assertions are in scope, and current Task or Change Unit status when those owner records are used. Full Evidence Manifest status is later/profile-gated. Later-profile status fields stay with promoted owner docs and the future catalog until those profiles are active. Unknown status values remain invalid unless a scenario explicitly tests recovery from invalid state; expected-state status assertions compare captured owner values, not prose labels.

## Fixture Assertion Semantics

Fixture assertion modes are runner defaults or suite catalog metadata. They are not Core input, are not passed to MCP tools, and must not add fields to the fixture body. The fixture body remains exactly `scenario_id`, `initial_state`, `request`, `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, and `forbidden_side_effects`.

Within partial assertion objects, omission means "not asserted." A listed field with value `null` asserts that the captured field is present and equals JSON `null`. A listed array value `[]` asserts a present empty array. A listed object-map value `{}` asserts a present empty map when the owner schema says that field is a map. For structured objects under `partial_deep`, fixture authors should list at least one child field unless they are deliberately asserting only that the object exists.

These omission rules are assertion rules only. They do not make omitted fields valid in public MCP `request.payload`; fixture `request.payload` still validates against the owning public request schema after any documented envelope expansion.

Default comparison modes:

| Fixture field | Default assertion mode |
|---|---|
| `expected_response` | `partial_deep`; listed response fields, refs, decisions, state versions, and primary-error summaries must match recursively. It must not match rendered prose alone. |
| `expected_state_changes` | `partial_deep`; listed Core-owned record changes must match recursively and unlisted fields are not asserted. Suite metadata may set `expected_state_changes: exact`. |
| `expected_storage_rows` | `table_effects`; listed table insert/update/delete/no-change counts and row filters must match captured storage effects. Suite metadata may set table effects to exact for selected tables. |
| `expected_events` | `contains_ordered` over the stable-catalog projection of captured `task_events`; listed stable events must appear in ascending `task_events.event_seq` order, with unrelated stable events allowed before, between, or after them. Suite metadata may set `expected_events: exact`. |
| `expected_artifacts` | `contains_by_identity`; each listed artifact must match a registered artifact with the same `artifact_id` and `kind`, then any other listed artifact fields are matched recursively. |
| `expected_blockers` | `contains_by_kind_and_code`; each listed blocker must match a structured response or Core/storage blocker with the same blocker kind and API code when a code is listed. |
| `expected_errors` | `contains_primary_ordered`; `expected_errors: []` asserts no returned API errors. When an object is listed, `code` is required and matched exactly against the primary API `ErrorCode` selected by [Primary Error Code Precedence](api/errors.md#primary-error-code-precedence), unless the fixture explicitly lists secondary errors under owner-defined details. |
| `forbidden_side_effects` | Negative assertions over captured state, storage, events, artifacts, projections, generated outputs, and secret handling. Drafts may use readable strings; materialized executable fixtures should expand them into owner-record absence checks where practical. |

Because `expected_events` defaults to `contains_ordered`, `expected_events: []` means the fixture requires no specific stable events; it does not by itself assert that the captured stable-event stream is empty. To assert no stable events, suite metadata must set `expected_events: exact` for that fixture or suite. `expected_artifacts: []`, `expected_blockers: []`, and `expected_errors: []` assert no required entries of those kinds under their default modes; use compatible exact-mode metadata or `forbidden_side_effects` when absence is part of the behavior being proved.

`expected_events` comparisons are over the [Core Model Stable Event Catalog](core-model.md#stable-event-catalog) projection of captured `task_events`. API tool detail/audit event lists do not expand this set. Non-catalog detail or local-audit events captured in `task_events` must not make a normal staged-delivery fixture fail. When suite metadata sets `expected_events: exact`, exactness applies to the stable-event projection of the captured stream unless a future Roadmap/local suite explicitly opts into implementation-specific detail-event assertions. Validator IDs, Core check names, projection status shorthands, fixture shorthand labels, and scenario catalog IDs are not event names. Prose examples may mention non-catalog event names as illustrative or future extension ideas, but executable staged-delivery fixtures must not require them until the Core Model event catalog promotes them.

Conformance runners order captured `task_events` by `event_seq`. `state_version`, `created_at`, and `event_id` are not tie-breakers for `expected_events` ordering.

Fixture authors should use `VALIDATOR_FAILED` as an `expected_errors[].code` only when API precedence selects the generic validator fallback; a more specific typed blocker such as `EVIDENCE_INSUFFICIENT`, `QA_REQUIRED`, `PROJECTION_STALE`, or `ARTIFACT_MISSING` remains primary when it applies.

`CloseTaskResponse.blockers[].code` is also an API `ErrorCode` value. Policy-specific or validator-specific finding codes belong under `expected_state_changes.validators`, validator finding assertions, or equivalent expected validator output, not in `expected_errors[].code` or close blocker `code`. Fixtures that exercise blocked close must assert the structured blockers returned by Core under `expected_blockers` and, when committed state changes are expected, the captured equivalent under `expected_state_changes.close_blockers` or `expected_storage_rows.blockers`. Matching report prose, Journey Card text, status text, or agent summaries alone cannot prove a close blocker.

Validator assertions nested under `expected_state_changes.validators` are keyed by validator ID. Each listed validator ID must exist in the captured validator results and match the listed fields partially; unlisted validator IDs and unlisted validator fields are not asserted.

When fixtures assert design-quality impact, all relevant validator findings should remain visible under `expected_state_changes.validators`, while fixtures assert the merged impact class, routed action, gate, write-blocker, close-blocker, waiver, or user judgment outcome produced by the policy-owned [Severity Composition Rule](design-quality-policies.md#severity-composition-rule) and [Active MVP impact defaults](design-quality-policies.md#active-mvp-impact-defaults). Fixtures must not add policy schemas, invent new action values, suppress lower-severity findings merely because a stronger merged blocker is also present, or treat advisory/later catalog findings as MVP blockers.

Core check and precondition assertions nested under `expected_state_changes.checks` are keyed by check/precondition name. These entries are compared against captured Core check output, blocked reasons, response summaries, or equivalent runner-observed check status. They are not validator IDs and must not be nested under `expected_state_changes.validators` unless [API Schema Core](api/schema-core.md#validatorresult), [API Schema Later](api/schema-later.md#validatorresult-stable-ids), or [Storage](storage.md) explicitly promotes that ID to a stable `ValidatorResult`.

`expected_state_changes.checks.projection_freshness` asserts the Core mechanical projection freshness check when a promoted owner brings that check into scope. `expected_state_changes.validators.context_hygiene_check` asserts the stable ValidatorResult for higher-level context hygiene; that validator may consider projection freshness, but it is not the fixture assertion location for the mechanical check itself.

Fixtures that cover `secret_omitted` or `blocked` artifacts should assert any committed artifact `redaction_state` under `expected_artifacts`, storage effects under `expected_storage_rows`, and downstream evidence or blocker effects under `expected_state_changes` and `expected_blockers`. Fixtures must not assert the omitted secret or PII value. Export, Release Handoff, full Evidence Manifest, Manual QA, Eval, detached verification, and broad artifact non-leakage cases remain later/profile catalog material until promoted.

Artifact redaction, blocked-input, integrity, and export non-leakage scenario families are future catalog inventory. See [Future Fixtures: Artifact Redaction And Export Non-Leakage Catalog Entries](../later/future-fixtures.md#artifact-redaction-and-export-non-leakage-catalog-entries).

Projection assertions compare only owner-defined freshness, enqueue status, source-state-version display, and related job facts when projection support is in scope. They belong in `expected_state_changes`, `expected_storage_rows`, or another owner-defined structured field, not in rendered Markdown. Projection failures must not roll back or rewrite captured Core state and events.

Suite catalogs may override assertion modes without changing fixtures:

```yaml
suite: core
assertion_modes:
  expected_state_changes: exact
  expected_storage_rows.tasks: exact
  expected_events: exact
  expected_errors.details: exact
fixtures:
  - MVP-ACTIVE-task-change-unit-setup
```

Future conformance must prove behavior through captured response fields, Core state, storage rows, `task_events`, validator results, artifact registry/file integrity, projection job or freshness state when promoted, returned error codes, structured blockers, and forbidden-side-effect checks. Matching rendered Markdown, Journey Card prose, status prose, close report prose, or agent prose alone cannot pass a fixture.

Fixture runners must use the same canonicalization rules as the reference implementation for `request_hash`, baseline `tree_hash`, and projection `managed_hash`. The detailed algorithms remain owned by [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [Storage](storage.md), and [Projection And Templates Reference](projection-and-templates.md) as applicable; conformance fixtures assert deterministic behavior without redefining those source-of-truth boundaries.

## Fixture Current-Phase Status

This repository is documentation-only. No executable fixture files, executable fixture catalog files, generated projections, runtime state, databases, or Harness Server conformance tests are being created by this documentation batch.

MVP structured drafts and fixture-authoring queues are future authoring plans. They become runnable only after documentation acceptance, a separate implementation-planning readiness decision, Harness Server implementation, and a deliberate fixture-materialization step. Documentation checks may report Markdown drift, but they are not runtime conformance and do not create Core fixture results.

## Catalog-Only Fixture Skeleton Guidance

Catalog skeleton guidance is for turning promoted future catalog families into exact-shape fixtures. It is not an executable fixture body, public request schema, DDL extension, runner design, or stage-exit requirement. Delivery-stage mapping belongs in suite catalog metadata, not in the fixture body. "Minimum seeded records" means owner records placed in `initial_state` after expansion and validation by Storage rules; public mutations still use the exact MCP request payload under `request.payload`.

Future scenario-family inventory lives in [Future Fixtures](../later/future-fixtures.md).

## Kernel Smoke Authoring Queue

Use this queue as future authoring guidance for the [Kernel Smoke Behavior Examples](#engineering-checkpoint-behavior-examples). Kernel Smoke is the narrow authoring label for the first internal authority loop, not the first user-value slice, not a full conformance suite, and not the future fixture catalog. These rows do not imply executable fixture files already exist. They are a compact authoring order; a first implementation plan may materialize only the smallest subset that proves the one authority loop named by Build.

Kernel Smoke defaults to no projection requirement. A fixture may assert projection freshness or enqueue/failure facts only when the minimal owner path already produces those facts and they help prove the target behavior. Projection-template polish, detailed report templates, multiple projection kinds, browser QA capture, export/recover, reconcile, stewardship, context hygiene, full operations, and future guarantee-level fixtures stay outside Engineering Checkpoint unless owner docs later promote a specific narrow path.

In the table, `None` means the matching draft field stays `[]`, `{}`, or otherwise empty. It is not a new sentinel value.

| Queue | Fixture draft family | Request path | Minimum seeded records | Required structured assertion | Expected blockers/errors | Forbidden side effects to preserve |
|---|---|---|---|---|---|---|
| 1 | `MVP-ACTIVE-task-change-unit-setup` | `harness.intake` | Registered local project with no active Task | One active Task, one active Change Unit or scope boundary, current-task pointer, and no write authority. | None | No Run, artifact, evidence, final acceptance, residual-risk acceptance, close, or projection-as-authority effect. |
| 2 | `MVP-ACTIVE-shaping-update-persists` | `harness.record_run` with `kind=shaping` and `product_write=false` | Active Task and Change Unit | Shaping updates persist into Task/Change Unit state and a shaping Run without product-write authority. | None | No Write Authorization, product-write Run, Evidence Manifest, projection job, acceptance, or risk acceptance. |
| 3 | `MVP-ACTIVE-prepare-write-allowed-authorization` | `harness.prepare_write` | Active Task, compatible scope, current expected state | `decision=allowed`, one active Write Authorization, replay row, no Run. | None | No OS permission, sandbox, preventive, isolated, evidence, or close claim. |
| 4 | `MVP-ACTIVE-prepare-write-blocked-no-authorization` | `harness.prepare_write` | Active Task with incompatible requested path or missing compatible scope | Structured blocked response and no consumable Write Authorization. | `SCOPE_REQUIRED`, `NO_ACTIVE_CHANGE_UNIT`, or `SCOPE_VIOLATION` as owned by the API/Core path. | No authorization, Run, artifact, replay row for pre-commit failure, or projection job. |
| 5 | `MVP-ACTIVE-prepare-write-idempotent-replay` | `harness.prepare_write` replay | Existing committed replay row and original active authorization | Original response and original `write_authorization_ref` are returned. | None | No duplicate authorization, event, artifact, replay update, projection job, or state-version increment. |
| 6 | `MVP-ACTIVE-idempotency-key-hash-conflict` | State-changing tool with same idempotency key and different hash | Existing committed replay row | `STATE_CONFLICT`; original replay row remains unchanged. | `STATE_CONFLICT` | No merged response, event, artifact, projection job, owner relation, or replay row update. |
| 7 | `MVP-ACTIVE-record-run-consumes-authorization` | `harness.record_run` | Active Task, compatible scope, active compatible Write Authorization | One Run is recorded and the authorization is consumed exactly once. | None | No second consumption, final acceptance, residual-risk acceptance, detached verification, or close. |
| 8 | `MVP-ACTIVE-record-run-missing-authorization-blocked` | `harness.record_run` | Active Task and product-write Run request with no authorization | Product-write Run is blocked before commit. | `WRITE_AUTHORIZATION_REQUIRED` | No Run, consumption, completion evidence, artifact link, projection job, or replay row. |
| 9 | `MVP-ACTIVE-record-run-observed-out-of-scope` | `harness.record_run` | Active compatible Write Authorization whose stored scope excludes observed path | Out-of-scope observation is blocked or recorded only through owner violation/audit path without consuming the authorization as success. | `SCOPE_VIOLATION` or owner-equivalent structured blocker | Invalid authorization is not consumed; observation is not completion evidence or close readiness. |
| 10 | `MVP-ACTIVE-raw-secret-artifact-blocked` | `harness.record_run` with artifact input | Active Task/Run path and staged raw-secret artifact input | Raw secret bytes are blocked or represented only through safe blocked/omitted metadata and downstream evidence/blocker effects. | `ARTIFACT_MISSING` or owner-equivalent artifact blocker | No raw secret storage, rendering, export, evidence sufficiency, authorization consumption, or close. |
| 11 | `MVP-ACTIVE-evidence-summary-insufficient` | `harness.status` or evidence owner read | Active Task with partial/missing evidence summary | Evidence summary remains insufficient/partial and close-relevant blocker is structured. | `EVIDENCE_INSUFFICIENT` blocker when close/write path depends on it | Status prose or Markdown evidence list does not repair missing refs. |
| 12 | `MVP-ACTIVE-evidence-summary-sufficient` | `harness.record_run` | Active Task, compatible authorization, visible staged artifact | Registered artifact refs and evidence summary become sufficient from owner records. | None | No full Evidence Manifest, Manual QA, detached verification, final acceptance, or risk acceptance. |
| 13 | `MVP-ACTIVE-final-acceptance-missing-close-blocker` | `harness.close_task` | Active Task with evidence sufficient but required final acceptance missing | Close remains blocked with final-acceptance blocker. | `ACCEPTANCE_REQUIRED` | No terminal Task, fabricated acceptance, residual-risk acceptance, Manual QA, detached verification, or close report authority. |
| 14 | `MVP-ACTIVE-residual-risk-visible-not-accepted-blocker` | `harness.close_task` | Active Task with visible close-relevant residual risk and no compatible acceptance judgment | Residual-risk acceptance remains required and Task stays open. | `DECISION_REQUIRED` or `DECISION_UNRESOLVED` with `required_judgment_kind=residual_risk_acceptance` | Visible risk is not accepted risk; no rich Residual Risk record, detached verification, or close state is fabricated. |
| 15 | `MVP-ACTIVE-accepted-risk-close` | `harness.close_task` | Active Task with sufficient evidence, visible risk, and compatible `judgment_kind=residual_risk_acceptance` | Task closes with accepted-risk close reason and refs to the user judgment. | None | Accepted risk does not create detached verification, Manual QA, Approval, final acceptance, or assurance upgrade. |
| 16 | `MVP-ACTIVE-display-label-not-canonical` | `harness.request_user_judgment` | Active Task and Change Unit | Response may render localized display label; storage and blocker state use canonical `judgment_kind`. | None | `display_label` and localized labels are not canonical state, gate keys, storage identity, or close aggregation keys. |

The queue above is intentionally small. Engineering Checkpoint does not require a full conformance suite, broad catalog family coverage, final-acceptance success semantics, Manual QA, detached verification, export/recover, reconcile, stewardship, context hygiene, browser QA capture, or future guarantee-level checks. MVP-1 adds the listed user-loop judgment, evidence, close-blocker, and accepted-risk drafts without promoting later verification, full Evidence Manifest, full Manual QA, export, or profile fixtures.

## Future Fixtures

Scenario families have moved to [Future Fixtures](../later/future-fixtures.md) so the early reference stays focused on the core conformance model. That catalog contains compact future-oriented inventory for browser QA capture, cross-surface behavior, export non-leakage, context hygiene, reconcile, stewardship, full operations, advanced projection rendering, artifact redaction and integrity, and future guarantee-level checks.

Those catalog entries are design inventory only until a promoted owner path materializes exact-shape executable fixtures. They are not required for Engineering Checkpoint, do not expand MVP-1 by themselves, and do not count as runtime conformance while this repository remains documentation-only.

## Metrics Boundary

Long-term operational metrics are derived analytics, not staged-delivery-critical state or conformance requirements. Keep metrics such as approval turnaround, verification latency, projection stale duration, same-session guard frequency, and surface fallback rate in the [roadmap](../roadmap.md) as read-only diagnostics until a future version promotes them with owner docs, fixtures or a conformance target, fallback behavior, relevant redaction/retention policy, no projection-as-canonical dependency, and implementation ownership.
