# Conformance Fixtures Reference

## What this document helps you do

Use this reference to look up the three-layer boundary for Harness conformance material: documentation checks, active structured fixture drafts, and future runtime conformance. It explains what future conformance will prove, the active Kernel Smoke, MVP-1 user-loop, artifact/evidence draft families, canonical active fixture-value rules, exact structured fixture draft shape, future runner execution behavior, fixture assertion semantics, current-phase status, and the boundary to the future fixture catalog.

This is a lookup document for conformance authors, implementers, and maintainers. It is not an operator procedure; use [Operations And Conformance Reference](operations-and-conformance.md) for operator entrypoints and the `harness conformance run` overview.

This is reference documentation for future conformance work. The current repository is documentation-only and contains no runnable Harness Server conformance tests; current phase and handoff status are tracked in [MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

## Read this when

- You are writing or reviewing the future fixture-based conformance design.
- You need the exact fixture body fields, the canonical active value boundary, the `request.payload` public request schema rule, or future runner isolation behavior.
- You need fixture assertion modes for response facts, Core state, storage rows, events, artifacts, blockers, errors, forbidden side effects, and projection facts when promoted.
- You need the active Kernel Smoke, MVP-1 User Work Loop, or artifact/evidence fixture drafts, or the boundary between those drafts and the future fixture catalog.

## Before you read

Use [Operations And Conformance Reference](operations-and-conformance.md#conformance-run) for the conformance run entrypoint, suite-selection overview, docs-maintenance profile boundary, and operator procedures. Use [MVP API](api/mvp-api.md) and [API Schema Core](api/schema-core.md) for public request/response schemas, [Storage](storage.md) for storage layout and seed-loader owner values, [Core Model Reference](core-model.md) for state transition and stable event semantics, [Projection And Templates Reference](projection-and-templates.md) for projection freshness, [Design Quality Policies](design-quality-policies.md) for policy validator behavior, and [Agent Integration Reference](agent-integration.md) for connector conformance overview.

## Main idea

Today this document is a future conformance design, not a set of runnable tests. It defines behavior-example IDs and required behavior for later implementation planning; it does not create fixture files, runner code, generated outputs, runtime state, or a runnable Harness Server conformance suite. Do not create actual fixture files from these examples during the documentation-only phase.

Keep three layers separate:

- Documentation checks are read-only editorial checks over Markdown docs: link integrity, terminology consistency, stage boundaries, security wording, user-language checks, owner-boundary drift, and English/Korean parity. They may report Markdown drift, but they do not execute fixture actions, append `task_events`, create artifacts, refresh projections, create QA or acceptance state, affect close readiness, create implementation readiness, or create runtime results.
- Active MVP fixture drafts are compact structured design drafts for Engineering Checkpoint and MVP-1. They describe expected behavior through assertion fields but are not executable fixtures yet and are not generated runtime artifacts.
- Runtime conformance is future Harness Server implementation work. It applies to implemented Core/API/storage/surface behavior and is judged by executable fixtures and structured assertions, not documentation prose. Only after server implementation and fixture materialization will exact-shape fixtures run through Core or operator entrypoints and produce runtime pass/fail results.
- Active MVP fixture bodies use the same canonical active values as the public API, schema, Core, storage, and error owner docs. They must not use fixture-only shorthand, fixture-local enum values, pseudo-fields, display labels as state values, or later/profile-only values.

The core model and small active MVP fixture drafts stay in this file. Detailed later scenarios stay in [Future Fixtures](../later/future-fixtures.md). This keeps Engineering Checkpoint Kernel Smoke and MVP-1 user-facing value understandable without making later catalog coverage look like an early implementation requirement.

After implementation begins, conformance will prove Harness behavior with executable fixtures. A passing runtime fixture will drive a Core or operator request and compare captured response facts, Core state, storage rows, events, artifacts, blockers, errors, and forbidden side effects against structured expectations.

Assertion authority is layered:

- Prose scenario descriptions, comments, rendered Markdown, Journey Card prose, status text, close report prose, and agent summaries are explanatory only.
- Captured response facts, Core state, storage rows, `task_events`, validator results, returned primary errors, structured blocker fields, and forbidden-side-effect checks are authoritative for fixture pass/fail.
- Artifact reference, owner-link, `sha256`, `size_bytes`, `content_type`, `redaction_state`, relation owner, retention, availability, and file-integrity assertions are authoritative where the scenario depends on artifacts or evidence bytes.
- Projection output may be checked for freshness, source-state-version display, readability, and availability when projection support is in scope, but renderer output must not replace Core state, satisfy evidence, authorize writes, close work, create final acceptance, create residual-risk acceptance, or become the source of conformance truth. Engineering Checkpoint does not require projection assertions beyond an empty or "no projection requirement" field.

## Reference scope

This document owns:

- conformance fixture body shape
- canonical active value boundary for the active Engineering Checkpoint / MVP-1 path
- `request.payload` public-schema requirements for active fixture bodies
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
| How a future runner loads, seeds, executes, captures, and compares | [Conformance Execution](#conformance-execution) |
| Default comparison modes for `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, and `forbidden_side_effects` | [Fixture Assertion Semantics](#fixture-assertion-semantics) |
| Active structured fixture draft families | [Active Structured Fixture Drafts](#engineering-checkpoint-behavior-examples) |
| Suite intent and authoring order | [Conformance staging](operations-and-conformance.md#conformance-staging), [Kernel Smoke Authoring Queue](#kernel-smoke-authoring-queue), and [Future Fixtures: Fixture Suites](../later/future-fixtures.md#fixture-suites) |
| Core model and current-phase boundary | [Core Conformance Model](#core-conformance-model) and [Fixture Current-Phase Status](#fixture-current-phase-status) |
| Future scenario inventory by concern | [Future Fixtures](../later/future-fixtures.md) |

## Core Conformance Model

The core conformance model defines what future runtime conformance proves and where assertion authority lives. A passing fixture proves behavior by driving one Core or operator request and comparing captured response facts, Core state, storage rows, events, artifacts, blockers, errors, and forbidden side effects with fixture expectations. It does not prove behavior by matching prose, generated Markdown, Journey Card text, status prose, close prose, or agent summaries.

Assertion types remain deliberately small:

- State and storage assertions compare Core-owned records, storage row effects, `task_events`, validator results, returned primary errors, structured blockers, owner refs, and state-version behavior.
- Artifact assertions compare registered artifact identity, owner links, `sha256`, `size_bytes`, `content_type`, `redaction_state`, relation owner, retention class, availability, and file-integrity facts where the scenario depends on evidence bytes.
- Projection assertions compare freshness, enqueue or job status, source-state-version display, readability, and availability only when projection support is in scope. They never replace Core state or satisfy authority, evidence, close, final acceptance, or residual-risk acceptance.
- Error assertions compare the API-owned primary `ErrorCode` and optional details according to public schema precedence.

State and storage assertions answer "what did Core own after the request, and which durable row effects occurred?" Artifact assertions answer "what evidence bytes or metadata were safely registered and linked?" Projection assertions answer "is a derived readable view current, stale, available, failed, or queued?" These are separate assertion locations, and projection output must not substitute for state or artifact proof.

## Fixture Profiles By Proven Behavior

Fixture profiles are grouped by the behavior they prove, not by how polished the rendered output is. The profile name does not add fixture-body fields, does not require a renderer to be authoritative, and does not imply fixture files exist in this documentation-only repository.

The hardened local reference target is an umbrella target reached through Assurance Profile and Operations Profile. It is not a fifth fixture profile and must not be used as a suite name.

| Profile | Stage name | Behavior proved | Out of scope for that profile |
|---|---|---|---|
| Engineering Checkpoint fixtures, with Kernel Smoke as the authoring label | Engineering Checkpoint | The first executable authority loop: no-active-Task status, owner-valid setup/intake creating one active Task, active Change Unit requirement, in-scope/out-of-scope `prepare_write`, dry-run and replay behavior, single-use Write Authorization, `record_run` consumption and invalid-authorization blockers, minimal artifact metadata, evidence summary, close blockers, residual-risk visibility, and honest cooperative/detective guarantee display. | Ordinary natural-language intake quality, full user-loop judgment UX, full Evidence Manifest, projection renderer support, final-acceptance or residual-risk acceptance success semantics, later assurance checks, export/recover, release handoff, full conformance runner, broad future catalog coverage, hosted connector registry, cross-surface orchestration, preventive guard expansion, and broad operations. |
| MVP-1 User Work Loop fixtures | MVP-1 User Work Loop | Ordinary requests become tracked work without Harness vocabulary; focused user judgment, status next safe action, non-substitution boundaries for broad approval text, sensitive-action approval, final acceptance, residual-risk acceptance, evidence, and proof that active MVP does not fabricate later assurance state are visible through Core-owned state and structured responses. | Full agency assurance hardening details, stewardship policy suite, full TDD/module/interface/domain-language catalogs, full feedback-loop audits, export/recover, release handoff, broad connector ecosystem, hosted connector registry, cross-surface orchestration, and automation beyond the MVP-1 user-value path. |
| Assurance Profile fixtures | Assurance Profile | User-owned judgment, sensitive-action Approval, Write Authorization, Manual QA, verification, final acceptance, residual-risk acceptance, stewardship, design-quality, context-hygiene, TDD, and feedback-loop boundaries stay separate and fixture-proven through Core records. | Operator recovery/export completeness, release handoff, broad operations coverage, dashboard/hosted workflow UI, broad connector automation, and unproven preventive or isolated guarantee claims. |
| Operations Profile / promoted Roadmap fixtures | Operations Profile and Roadmap | Export/recover, artifact integrity, release handoff, operator readiness, reconcile, broader conformance coverage, and any promoted future higher guarantee level or automation profile. | Any stronger security, isolation, preventive guard, browser-capture, remote/shared MCP, or automation claim until owner docs define the mechanism and fixtures prove the covered behavior. |

## Active MVP Fixture Draft Families

These draft families are the active future-authoring target for Engineering Checkpoint and MVP-1. They are not executable fixtures yet, not generated runtime artifacts, and not current pass/fail criteria. The structured draft bodies below preserve the active scenario IDs, proof intent, public request owner, expected Core/storage effects, and owner links. They remain documentation drafts until future implementation materializes them as executable fixture files.

### Canonical Active Fixture Values

Active MVP fixture bodies use the same canonical active values as the public owner docs. They must not introduce fixture-only shorthand, alternate enum values, compact pseudo-fields, display labels as state values, pseudo event names, pseudo storage rows, or later/profile-only values. This keeps fixture drafts close enough to the public contracts that a future runner can validate them without a separate fixture dialect.

#### Active Fixture Value Owners

Conformance fixture drafts consume active contracts; this document does not redefine active contracts. The table below pins fixture value areas to the owner documents for both language trees. Active fixture drafts must not invent enum values, table shapes, request fields, blocker categories, or error codes. If a fixture appears to need a new value, the owner document must be clarified first; the fixture document must not silently create it. Later/profile-only fixture material belongs outside the active MVP fixture set.

| Fixture value area | Active owner contract | Fixture authoring rule |
|---|---|---|
| API request shape | [MVP API](api/mvp-api.md) (`docs/*/reference/api/mvp-api.md`) | `request.tool` and `request.payload` use the public method request shape; fixtures do not add fixture-only request fields. |
| Active schema values | [API Schema Core](api/schema-core.md) (`docs/*/reference/api/schema-core.md`) | Active enum values, shared refs, response fields, and schema-owned value sets come from the active schema owner. |
| Core lifecycle and state transitions | [Core Model Reference](core-model.md) (`docs/*/reference/core-model.md`) | `lifecycle_phase`, gate effects, Core-owned state changes, and transition outcomes use the Core owner values. |
| Storage row shape | [Storage](storage.md) (`docs/*/reference/storage.md`) | Tables, columns, JSON `TEXT` shapes, row effects, and storage hardening values come from Storage. |
| Error codes | [API Errors](api/errors.md) (`docs/*/reference/api/errors.md`) | `ErrorCode` values, primary-error precedence, and error details follow the API error owner. |
| Blocker categories | [API Schema Core](api/schema-core.md) (`docs/*/reference/api/schema-core.md`) and [Core Model Reference](core-model.md) (`docs/*/reference/core-model.md`) | Blocker categories, `required_judgment_kind`, related refs, and owner-state blocker facts use schema and Core owner values. |
| Close semantics | [MVP API](api/mvp-api.md) (`docs/*/reference/api/mvp-api.md`) and [Core Model Reference](core-model.md) (`docs/*/reference/core-model.md`) | `close_task` request/response shape and close state effects follow the API and Core owners; fixtures do not create fixture-local close states. |
| Artifact and evidence summary shape | [API Schema Core](api/schema-core.md) (`docs/*/reference/api/schema-core.md`) and [Storage](storage.md) (`docs/*/reference/storage.md`) | `ArtifactRef`, `ArtifactInput`, artifact relation values, and evidence-summary row or JSON shapes use schema and Storage owner values. |
| Later/profile-only fixture material | [API Schema Later](api/schema-later.md) (`docs/*/reference/api/schema-later.md`) and later docs such as [Future Fixtures](../later/future-fixtures.md) | Later/profile-only values, methods, refs, fixture branches, and catalog material stay outside active MVP fixture bodies until an owner promotes them. |

For active Engineering Checkpoint and MVP-1 fixture bodies:

- `request.payload` must be the public request object for `request.tool`, including `envelope: ToolEnvelope` and every required field from the corresponding method request schema in [MVP API](api/mvp-api.md) and [API Schema Core](api/schema-core.md). In short, `request.payload` must match the corresponding public method request schema; fixtures do not get a narrower or looser payload dialect. Suite metadata may help an author choose default envelope values, but a materialized active fixture body must contain the expanded public request before validation, canonical request hashing, or Core execution.
- `expected_state_changes` must assert active Core-owned fields and values from [Core Model Reference](core-model.md), [MVP API](api/mvp-api.md), and [API Schema Core](api/schema-core.md). When asserting `tasks.lifecycle_phase`, active fixture bodies use only `intake`, `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, or `cancelled`. They must not use status words such as `active`, `open`, or `terminal` as lifecycle values.
- `expected_storage_rows` must assert active tables, columns, JSON payload shapes, and owner-bound value sets from [Storage](storage.md), including the hardening map in [Storage Validation And Enum Hardening](storage.md#canonical-enum-hardening).
- Active `expected_storage_rows` may use only active Storage-owned record areas: `project_state`, `surfaces` or an equivalent reference-surface registration record, `tasks`, `task_events`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries` or an equivalent minimal evidence coverage record, `blockers`, and `tool_invocations`. Other table families are later/profile material and must stay outside active fixture bodies unless Storage and the owning profile promote them.
- Requirements-shaping fixture assertions store shaping output only on active owner rows: `tasks`, `change_units`, `user_judgments` when a committed judgment request exists, `blockers` when Core commits a blocker, `evidence_summaries` when a minimal evidence coverage effect exists, and `runs.kind=shaping_update` for the committed shaping run. A shaping fixture must not require separate design, clarification-catalog, or candidate-record storage outside the active owner set.
- `expected_storage_rows.write_authorizations` must preserve the active `AuthorizedAttemptScope` under `attempt_scope_json`: `task_id`, `change_unit_id`, `basis_state_version`, `surface_id`, intended operation, intended paths/tools/commands and command classes, product-file-write intent, intended network targets, intended secret handles/scope, sensitive categories, `baseline_ref`, related user judgment refs, and `guarantee_level`. A committed non-dry-run `prepare_write.decision=allowed` fixture must assert the proposed attempt-scope fields in `request.payload`, `expected_response.write_authorization.attempt_scope`, and `expected_storage_rows.write_authorizations.attempt_scope_json` as the same resolved `AuthorizedAttemptScope`. Do not assert only paths when the request or proof claim includes commands, tools, network, secrets, baseline, sensitive categories, surface, or guarantee facts. Only committed non-dry-run `decision=allowed` creates `write_authorizations.status=active`; `blocked`, `approval_required`, `decision_required`, and `state_conflict` are response/blocker/error outcomes, not authorization rows.
- `expected_storage_rows.runs` must assert a committed active run shape. Use only `runs.kind` values `shaping_update`, `implementation`, or `direct` and `runs.status` values from Storage. Every committed Run row must contain either `observed_attempt_json` carrying the active `RecordRunPayload` branch and comparison outcome, or `observed_changes_json` carrying active `ObservedChanges`; product-write `implementation` and `direct` Runs assert both, plus the compatible `write_authorization_id`, comparison against the stored `AuthorizedAttemptScope`, and consumed authorization effect. Pre-commit rejected `record_run` fixtures assert no Run row, no artifact/link/evidence mutation, no authorization consumption, and no replay row unless the fixture explicitly targets the owner-defined violation/audit exception.
- `expected_storage_rows.user_judgments` must use `judgment_kind`, `presentation`, `status`, owner refs, and payload JSON from the active user-judgment schema. `display_label` and localized labels are rendered display text only; they must not appear as storage columns, canonical row identity, validator inputs or keys, state-compatibility assertions, blocker keys, gate keys, compatibility inputs, or close aggregation keys.
- Assertions about `UserJudgmentCandidate` must treat it as candidate-only output from read, validation, dry-run, or compatibility paths. A candidate has no committed `StateRecordRef`, creates no `user_judgments` row, and satisfies no blocker, gate, sensitive-action permission, final acceptance, residual-risk acceptance, close, Write Authorization, or evidence assertion. A future fixture may assert a pending `user_judgments` row only after a committed `dry_run=false` `harness.request_user_judgment` call records that active request; it may assert a resolved judgment effect only after `harness.record_user_judgment` records the user's answer for the same stored `judgment_kind`.
- `expected_storage_rows.artifacts` and `expected_storage_rows.artifact_links` are active only when the fixture commits a registered artifact or safe metadata notice that Storage and `ArtifactRef` support. A rejected raw-secret artifact branch asserts no `artifacts`, no `artifact_links`, and no evidence-sufficiency mutation; a committed notice branch asserts only safe artifact/link/evidence effects with `redaction_state=blocked` or `secret_omitted`.
- `expected_storage_rows.tool_invocations` applies only to committed replayable non-dry-run responses. Dry runs, pre-commit state conflicts, validation failures before mutation, and pre-commit rejected `record_run` paths assert no replay row.
- `expected_events` must name stable event facts only after the Core owner promotes them. Human labels such as `owner-promoted Run recording event` are authoring notes, not active event values.
- `expected_artifacts` must use active `ArtifactRef`, `ArtifactInput`, relation owner, redaction, retention, and artifact status values from [API Schema Core](api/schema-core.md#artifactref), [ArtifactInput](api/schema-core.md#artifactinput), and [Storage](storage.md).
- Active `redaction_state` values in `ArtifactInput`, `ArtifactRef`, `expected_artifacts`, and `expected_storage_rows.artifacts` are exactly `none`, `redacted`, `secret_omitted`, and `blocked`. Use `none` only for stored bytes allowed without redaction, `redacted` when content was removed before storage, `secret_omitted` when secret or PII material is omitted or replaced by handles, and `blocked` when raw-payload storage or exposure is blocked. Values such as `visible`, `hidden`, `safe`, and `unsafe` are not redaction states.
- `expected_blockers` and `expected_response.blockers` must use active blocker categories, `required_judgment_kind` values, related refs, and close-blocker shapes from [MVP API](api/mvp-api.md#harnessclose_task), [API Errors](api/errors.md#harnessclose_task-close-blockers), and Core/storage owners. Active close/status blocker assertions must not use categories or response fields that Schema Core excludes from MVP-1.
- Sensitive-action approval expectations must use active `user_judgment` / `judgment_kind=sensitive_approval`, `approval_scope`, `approval_gate`, active `sensitive_approval` blocker category, or API-owned `APPROVAL_REQUIRED` / `APPROVAL_DENIED` / `APPROVAL_EXPIRED` codes. They must not assert broad permission text or a separate permission-record lifecycle. `decision_required` / `DECISION_REQUIRED` remains for user-owned judgments that are not sensitive-action permission and must not be used as a synonym for sensitive-action approval.
- `harness.close_task` fixture bodies must use `CloseTaskRequest.intent` only as `complete`, `cancel`, or `supersede`. Normal completion and accepted-risk completion both use `intent=complete`; accepted risk is expressed through `requested_close_reason=completed_with_risk_accepted` and compatible active Core state, not by changing `intent`. Cancellation uses `intent=cancel` with `requested_close_reason=cancelled`; supersession uses `intent=supersede` with `requested_close_reason=superseded` and the API-owned supersession fields when applicable. Active fixture bodies must not use close reasons or later/profile assurance values as intent values.
- `expected_errors` must use active public `ErrorCode` values and primary-error precedence from [API Errors](api/errors.md). Validator IDs or policy finding codes belong under owner-defined validator/state assertions, not as primary `expected_errors[].code` unless the public API owner selects that code.
- `harness.record_run` error fixtures must use the active mapping in [API Errors](api/errors.md#error-taxonomy): missing required authorization uses `WRITE_AUTHORIZATION_REQUIRED` with `authorization_reason=missing` when details assert the reason; stale, expired, revoked, consumed, or incompatible authorization uses `WRITE_AUTHORIZATION_INVALID` with the matching `authorization_reason`; observed work outside the stored `AuthorizedAttemptScope` uses `SCOPE_VIOLATION`; unsupported observation or insufficient surface capability for a required comparison uses `CAPABILITY_INSUFFICIENT`; forbidden secret or artifact handling uses `VALIDATION_FAILED`, `SCOPE_VIOLATION`, or `ARTIFACT_MISSING` according to the owner mapping.
- `forbidden_side_effects` may be readable in documentation drafts, but materialized executable fixtures should expand each forbidden effect into owner-record absence, row-effect, artifact, event, derived-view, or generated-output assertions where practical. For failed operations, `expected_storage_rows` and `forbidden_side_effects` must agree: a fixture must not forbid a Run, artifact, replay row, evidence mutation, authorization consumption, derived-view job, or non-active record while also expecting that row or effect. When absence is part of the behavior being proved, use `expected_storage_rows` table-effect assertions and compatible exact-mode metadata or explicit negative side-effect assertions.
- `harness.record_run` fixture bodies must align `RecordRunRequest.kind`, `RecordRunPayload.kind`, and the one non-null `RecordRunPayload` branch exactly. Active bodies may use only `shaping_update`, `implementation`, or `direct`: Discovery and requirements-shaping updates use `shaping_update`; implementation writes and implementation attempts use `implementation`; write-free direct observations and non-product operations use `direct`. Legacy or shorthand run-kind values, unknown payload branch names, and multiple non-null payload branches are invalid.
- Later/profile-only values, branches, methods, refs, table families, status values, and errors must not appear in active MVP fixture bodies. They stay in [Schema Later](api/schema-later.md), promoted later/profile owner docs, or [Future Fixtures](../later/future-fixtures.md) until an owner promotes the narrower path.

Deterministic IDs such as `task-fixture-001` are acceptable only as ordinary string IDs inside valid owner records and matching refs. A symbolic ID must not stand in for omitted required records, omitted request fields, unsupported schema branches, fixture-local status values, or unexpanded artifact refs.

<a id="engineering-checkpoint-behavior-examples"></a>
<a id="mvp-1-user-work-loop-behavior-examples"></a>
<a id="security-and-capability-behavior-examples"></a>
<a id="artifact-and-evidence-behavior-examples"></a>

### Active Structured Fixture Drafts

The active drafts use one uniform shape. They are compact enough for review but still name the public request fields, active storage rows, public error codes, and blocker categories that a future materialized fixture must expand and validate. Keep these fenced bodies YAML-loadable as structured source drafts; quoting Markdown-led or YAML-indicator-led scalars is fixture body validity cleanup, not a new fixture contract or runner claim. The evidence summary family has two request paths because insufficiency is read/close-visible state, while sufficiency is created by a committed active `record_run`.

```yaml
scenario_id: MVP-ACTIVE-task-change-unit-setup
purpose: Active Task / Change Unit setup.
initial_state:
  project_state:
    project_id: PROJ-001
    state_version: 1
    active_task_id: null
    default_surface_id: reference-local-mcp
  surfaces:
    - surface_id: reference-local-mcp
      guarantee_level: cooperative
      status: active
request:
  tool: harness.intake
  payload:
    envelope:
      request_id: REQ-001
      idempotency_key: IDEMP-001
      expected_state_version: 1
      project_id: PROJ-001
      task_id: null
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: user
      dry_run: false
    user_request: "Implement the narrow settings copy change."
    requested_mode: work
    resume_policy: create_new
    acceptance_criteria:
      - "Settings copy is updated in the allowed path."
    constraints:
      allowed_paths: ["app/settings/page.tsx"]
      non_goals: ["No settings behavior change"]
      sensitive_categories: []
    initial_context_refs: []
expected_response:
  base:
    errors: []
  task_id: TASK-001
  created: true
  resumed: false
  change_unit_id: CU-001
  state:
    mode: work
    lifecycle_phase: ready
    result: none
    close_reason: none
expected_state_changes:
  project_state:
    active_task_id: TASK-001
  tasks:
    TASK-001:
      mode: work
      lifecycle_phase: ready
      active_change_unit_id: CU-001
  change_units:
    CU-001:
      task_id: TASK-001
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
expected_storage_rows:
  project_state:
    updated:
      rows:
        - project_id: PROJ-001
          active_task_id: TASK-001
  tasks:
    inserted:
      rows:
        - task_id: TASK-001
          mode: work
          lifecycle_phase: ready
          active_change_unit_id: CU-001
  change_units:
    inserted:
      rows:
        - change_unit_id: CU-001
          task_id: TASK-001
          status: active
  write_authorizations:
    inserted:
      count: 0
  runs:
    inserted:
      count: 0
  artifacts:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - No Write Authorization, Run, artifact, evidence summary, final acceptance, residual-risk acceptance, close state, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessintake
  schema: docs/*/reference/api/schema-core.md
  core: docs/*/reference/core-model.md
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md
```

```yaml
scenario_id: MVP-ACTIVE-shaping-update-persists
purpose: Shaping update persisted into active state.
initial_state:
  project_state:
    project_id: PROJ-001
    state_version: 2
    active_task_id: TASK-001
    default_surface_id: reference-local-mcp
  tasks:
    - task_id: TASK-001
      mode: work
      lifecycle_phase: shaping
      active_change_unit_id: CU-001
      state_version: 2
  change_units:
    - change_unit_id: CU-001
      task_id: TASK-001
      status: active
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-002
      idempotency_key: IDEMP-002
      expected_state_version: 2
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: shaping_update
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: null
    summary: "Clarified the current goal and first allowed path."
    artifact_inputs: []
    payload:
      kind: shaping_update
      shaping_update:
        shaping_kind: scope
        task_update:
          title: null
          original_user_request: null
          current_goal_summary: "Update the settings copy only."
          mode: work
          success_criteria: ["Settings copy is updated."]
          non_goals: ["No behavior change"]
          affected_areas: ["settings"]
          affected_path_candidates: ["app/settings/page.tsx"]
          constraints:
            allowed_paths: ["app/settings/page.tsx"]
            sensitive_categories: []
        change_unit_update:
          change_unit_id: CU-001
          operation: update
          scope_summary: "Settings copy update."
          affected_areas: ["settings"]
          affected_path_candidates: ["app/settings/page.tsx"]
          allowed_paths: ["app/settings/page.tsx"]
          denied_paths: []
          non_goals: ["No behavior change"]
          success_criteria: ["Settings copy is updated."]
          sensitive_categories: []
          baseline_ref: BASE-001
          autonomy_boundary: null
        user_judgment_candidates: []
        confirmed_facts: ["The requested file is inside the active scope."]
        remaining_uncertainties: []
        blocking_question: null
        useful_non_blocking_questions: []
        next_safe_action: "Run prepare_write for the settings copy change."
        source_refs:
          - record_kind: task
            record_id: TASK-001
        evidence_refs:
          state_refs: []
          artifact_refs: []
      implementation: null
      direct: null
expected_response:
  base:
    errors: []
  run_id: RUN-001
  state:
    mode: work
    lifecycle_phase: shaping
  write_authorization_ref: null
  registered_artifacts: []
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: shaping
      current_goal_summary: "Update the settings copy only."
      next_safe_action: "Run prepare_write for the settings copy change."
  change_units:
    CU-001:
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
  runs:
    RUN-001:
      kind: shaping_update
      status: completed
      product_write: false
expected_storage_rows:
  tasks:
    updated:
      rows:
        - task_id: TASK-001
          lifecycle_phase: shaping
          current_goal_summary: "Update the settings copy only."
  change_units:
    updated:
      rows:
        - change_unit_id: CU-001
          status: active
  runs:
    inserted:
      rows:
        - run_id: RUN-001
          kind: shaping_update
          status: completed
          product_write: false
          write_authorization_id: null
  write_authorizations:
    inserted:
      count: 0
  artifacts:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - No product-write Run, Write Authorization, non-active row/effect, final acceptance, or residual-risk acceptance is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#record-run-payloads
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-allowed-authorization
purpose: prepare_write allowed creates Write Authorization.
initial_state:
  project_state:
    project_id: PROJ-001
    active_task_id: TASK-001
    default_surface_id: reference-local-mcp
  tasks:
    - task_id: TASK-001
      mode: work
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 3
  change_units:
    - change_unit_id: CU-001
      task_id: TASK-001
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
      baseline_ref: BASE-001
request:
  tool: harness.prepare_write
  payload:
    envelope:
      request_id: REQ-003
      idempotency_key: IDEMP-003
      expected_state_version: 3
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    change_unit_id: CU-001
    intended_operation: "Update settings copy."
    intended_paths: ["app/settings/page.tsx"]
    intended_tools: ["edit"]
    intended_commands: []
    product_file_write_intended: true
    intended_network: []
    intended_secret_scope: []
    sensitive_categories: []
    baseline_ref: BASE-001
expected_response:
  base:
    errors: []
  decision: allowed
  state:
    lifecycle_phase: ready
  change_unit_id: CU-001
  baseline_ref: BASE-001
  write_authorization_ref:
    record_kind: write_authorization
    record_id: WA-001
  write_authorization:
    write_authorization_id: WA-001
    status: active
    attempt_scope:
      task_id: TASK-001
      change_unit_id: CU-001
      basis_state_version: 3
      surface_id: reference-local-mcp
      intended_operation: "Update settings copy."
      intended_paths: ["app/settings/page.tsx"]
      intended_tools: ["edit"]
      intended_commands: []
      product_file_write_intended: true
      intended_network: []
      intended_secret_scope: []
      sensitive_categories: []
      baseline_ref: BASE-001
      related_user_judgment_refs: []
      guarantee_level: cooperative
  authorization_effect: created
  active_user_judgment_refs: []
  blocked_reasons: []
expected_state_changes:
  write_authorizations:
    WA-001:
      status: active
      basis_state_version: 3
      consumed_by_run_id: null
  tasks:
    TASK-001:
      lifecycle_phase: ready
expected_storage_rows:
  write_authorizations:
    inserted:
      rows:
        - write_authorization_id: WA-001
          task_id: TASK-001
          change_unit_id: CU-001
          surface_id: reference-local-mcp
          status: active
          basis_state_version: 3
          attempt_scope_json:
            task_id: TASK-001
            change_unit_id: CU-001
            basis_state_version: 3
            surface_id: reference-local-mcp
            intended_operation: "Update settings copy."
            intended_paths: ["app/settings/page.tsx"]
            intended_tools: ["edit"]
            intended_commands: []
            product_file_write_intended: true
            intended_network: []
            intended_secret_scope: []
            sensitive_categories: []
            baseline_ref: BASE-001
            related_user_judgment_refs: []
            guarantee_level: cooperative
  tool_invocations:
    inserted:
      rows:
        - tool_name: harness.prepare_write
          idempotency_key: IDEMP-003
          task_id: TASK-001
          basis_state_version: 3
          status: committed
  runs:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - No OS permission, sandboxing, tamper-proof enforcement, preventive blocking, isolated guarantee, Run, artifact, evidence sufficiency, close state, final acceptance, or residual-risk acceptance is claimed or created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessprepare_write
  schema: docs/*/reference/api/schema-core.md#evidence-and-pre-write-scope-schemas
  core: docs/*/reference/core-model.md#prepare_write
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-blocked-no-authorization
purpose: prepare_write blocked creates no Write Authorization.
initial_state:
  project_state:
    project_id: PROJ-001
    active_task_id: TASK-001
    default_surface_id: reference-local-mcp
  tasks:
    - task_id: TASK-001
      mode: work
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 4
  change_units:
    - change_unit_id: CU-001
      task_id: TASK-001
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
request:
  tool: harness.prepare_write
  payload:
    envelope:
      request_id: REQ-004
      idempotency_key: IDEMP-004
      expected_state_version: 4
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    change_unit_id: CU-001
    intended_operation: "Update billing copy outside the active scope."
    intended_paths: ["app/billing/page.tsx"]
    intended_tools: ["edit"]
    intended_commands: []
    product_file_write_intended: true
    intended_network: []
    intended_secret_scope: []
    sensitive_categories: []
    baseline_ref: BASE-001
expected_response:
  base:
    errors: []
  decision: blocked
  write_authorization_ref: null
  write_authorization: null
  authorization_effect: none
  blocked_reasons:
    - code: out_of_scope
      related_error: SCOPE_VIOLATION
      required_judgment_kind: scope_decision
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: blocked
  blockers:
    - task_id: TASK-001
      blocked_action: prepare_write
      blocker_kind: scope
      status: open
expected_storage_rows:
  blockers:
    inserted:
      rows:
        - task_id: TASK-001
          blocked_action: prepare_write
          blocker_kind: scope
          status: open
  write_authorizations:
    inserted:
      count: 0
  runs:
    inserted:
      count: 0
  artifacts:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers:
  - code: SCOPE_VIOLATION
    blocker_kind: scope
    required_judgment_kind: scope_decision
expected_errors: []
forbidden_side_effects:
  - No consumable Write Authorization row, Run, artifact, evidence summary, close state, final acceptance, residual-risk acceptance, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessprepare_write
  schema: docs/*/reference/api/schema-core.md#evidence-and-pre-write-scope-schemas
  core: docs/*/reference/core-model.md#prepare_write
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-idempotent-replay
purpose: Idempotent replay returns the original committed prepare_write response.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 5
  write_authorizations:
    - write_authorization_id: WA-001
      task_id: TASK-001
      change_unit_id: CU-001
      surface_id: reference-local-mcp
      status: active
      basis_state_version: 5
      attempt_scope_json:
        task_id: TASK-001
        change_unit_id: CU-001
        basis_state_version: 5
        surface_id: reference-local-mcp
        intended_operation: "Update settings copy."
        intended_paths: ["app/settings/page.tsx"]
        intended_tools: ["edit"]
        intended_commands: []
        product_file_write_intended: true
        intended_network: []
        intended_secret_scope: []
        sensitive_categories: []
        baseline_ref: BASE-001
        related_user_judgment_refs: []
        guarantee_level: cooperative
  tool_invocations:
    - tool_name: harness.prepare_write
      idempotency_key: IDEMP-005
      request_hash: HASH-ORIGINAL
      task_id: TASK-001
      basis_state_version: 5
      status: committed
      response_json:
        decision: allowed
        write_authorization_ref:
          record_kind: write_authorization
          record_id: WA-001
request:
  tool: harness.prepare_write
  payload:
    envelope:
      request_id: REQ-005-REPLAY
      idempotency_key: IDEMP-005
      expected_state_version: 5
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    change_unit_id: CU-001
    intended_operation: "Update settings copy."
    intended_paths: ["app/settings/page.tsx"]
    intended_tools: ["edit"]
    intended_commands: []
    product_file_write_intended: true
    intended_network: []
    intended_secret_scope: []
    sensitive_categories: []
    baseline_ref: BASE-001
expected_response:
  base:
    errors: []
  decision: allowed
  write_authorization_ref:
    record_kind: write_authorization
    record_id: WA-001
  authorization_effect: returned
expected_state_changes: {}
expected_storage_rows:
  write_authorizations:
    inserted:
      count: 0
    updated:
      count: 0
  tool_invocations:
    inserted:
      count: 0
    updated:
      count: 0
    unchanged:
      rows:
        - tool_name: harness.prepare_write
          idempotency_key: IDEMP-005
          request_hash: HASH-ORIGINAL
          status: committed
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - No duplicate Write Authorization, event, artifact, replay-row update, state-version increment, Run, evidence, close, final acceptance, residual-risk acceptance, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessprepare_write
  schema: docs/*/reference/api/schema-core.md#tool-envelope
  core: docs/*/reference/core-model.md#prepare_write
  storage: docs/*/reference/storage.md#event-and-idempotency-semantics
  errors: docs/*/reference/api/errors.md#idempotency
```

```yaml
scenario_id: MVP-ACTIVE-idempotency-key-hash-conflict
purpose: Same idempotency key with a different canonical request hash returns conflict.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 6
  tool_invocations:
    - tool_name: harness.prepare_write
      idempotency_key: IDEMP-006
      request_hash: HASH-ORIGINAL
      task_id: TASK-001
      basis_state_version: 6
      status: committed
request:
  tool: harness.prepare_write
  payload:
    envelope:
      request_id: REQ-006-CONFLICT
      idempotency_key: IDEMP-006
      expected_state_version: 6
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    change_unit_id: CU-001
    intended_operation: "Update a different path with the reused key."
    intended_paths: ["app/account/page.tsx"]
    intended_tools: ["edit"]
    intended_commands: []
    product_file_write_intended: true
    intended_network: []
    intended_secret_scope: []
    sensitive_categories: []
    baseline_ref: BASE-001
expected_response:
  base:
    errors:
      - code: STATE_CONFLICT
        retryable: true
        details:
          stored_request_hash: HASH-ORIGINAL
          received_request_hash: HASH-DIFFERENT
expected_state_changes: {}
expected_storage_rows:
  tool_invocations:
    inserted:
      count: 0
    updated:
      count: 0
    unchanged:
      rows:
        - tool_name: harness.prepare_write
          idempotency_key: IDEMP-006
          request_hash: HASH-ORIGINAL
          status: committed
  write_authorizations:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors:
  - code: STATE_CONFLICT
forbidden_side_effects:
  - No merged response, new Write Authorization, event, artifact, owner relation, replay-row update, Run, evidence, close, final acceptance, residual-risk acceptance, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessprepare_write
  schema: docs/*/reference/api/schema-core.md#tool-envelope
  core: docs/*/reference/core-model.md#prepare_write
  storage: docs/*/reference/storage.md#event-and-idempotency-semantics
  errors: docs/*/reference/api/errors.md#idempotency
```

```yaml
scenario_id: MVP-ACTIVE-record-run-consumes-authorization
purpose: record_run consumes a compatible Write Authorization.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 7
  change_units:
    - change_unit_id: CU-001
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
  write_authorizations:
    - write_authorization_id: WA-007
      task_id: TASK-001
      change_unit_id: CU-001
      surface_id: reference-local-mcp
      status: active
      basis_state_version: 7
      attempt_scope_json:
        task_id: TASK-001
        change_unit_id: CU-001
        basis_state_version: 7
        surface_id: reference-local-mcp
        intended_operation: "Update settings copy."
        intended_paths: ["app/settings/page.tsx"]
        intended_tools: ["edit"]
        intended_commands: []
        product_file_write_intended: true
        intended_network: []
        intended_secret_scope: []
        sensitive_categories: []
        baseline_ref: BASE-001
        related_user_judgment_refs: []
        guarantee_level: cooperative
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-007
      idempotency_key: IDEMP-007
      expected_state_version: 7
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: implementation
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: WA-007
    summary: "Updated settings copy."
    artifact_inputs:
      - input_id: ARTIN-007-DIFF
        source_kind: staged_file
        existing_artifact_ref: null
        staged:
          staged_uri: harness-staging://PROJ-001/RUN-007/settings.diff
          display_name: settings.diff
          content_type: text/x-diff
          expected_sha256: SHA256-DIFF-007
          expected_size_bytes: 2048
        capture: null
        kind: diff
        redaction_state: none
        produced_by: lead_agent
        retention_class: task
        relation:
          task_id: TASK-001
          run_id: null
          record_kind: run
          record_id_hint: RUN-007
        description: "Diff for settings copy."
    payload:
      kind: implementation
      shaping_update: null
      implementation:
        outcome: completed
        product_write: true
        observed_changes:
          changed_paths:
            - path: app/settings/page.tsx
              change_kind: modified
              product_file: true
              within_change_unit: true
              before_sha256: SHA256-BEFORE-007
              after_sha256: SHA256-AFTER-007
          diff_artifact_input_ids: ["ARTIN-007-DIFF"]
          no_product_changes: false
        command_results: []
        tool_invocations:
          - tool_name: edit
            purpose: "Apply settings copy change."
            status: succeeded
            artifact_input_ids: ["ARTIN-007-DIFF"]
            summary: "Changed one scoped file."
        network_accesses: []
        secret_accesses: []
        evidence_updates:
          coverage_updates:
            - claim_or_criterion: "Settings copy is updated."
              coverage_state: supported
              supporting_state_refs: []
              supporting_artifact_input_ids: ["ARTIN-007-DIFF"]
              note: "Diff supports the copy update."
          gap_blocker_refs: []
          summary: "Implementation evidence recorded."
        implementation_notes: []
        follow_up_needed: []
      direct: null
expected_response:
  base:
    errors: []
  run_id: RUN-007
  state:
    lifecycle_phase: executing
  write_authorization_ref:
    record_kind: write_authorization
    record_id: WA-007
  registered_artifacts:
    - artifact_id: ART-007-DIFF
      kind: diff
      redaction_state: none
      retention_class: task
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: executing
  write_authorizations:
    WA-007:
      status: consumed
      consumed_by_run_id: RUN-007
  runs:
    RUN-007:
      kind: implementation
      status: completed
      product_write: true
expected_storage_rows:
  runs:
    inserted:
      rows:
        - run_id: RUN-007
          task_id: TASK-001
          change_unit_id: CU-001
          write_authorization_id: WA-007
          kind: implementation
          status: completed
          product_write: true
  write_authorizations:
    updated:
      rows:
        - write_authorization_id: WA-007
          status: consumed
          consumed_by_run_id: RUN-007
  artifacts:
    inserted:
      rows:
        - artifact_id: ART-007-DIFF
          kind: diff
          redaction_state: none
          retention_class: task
          status: available
  artifact_links:
    inserted:
      rows:
        - artifact_id: ART-007-DIFF
          task_id: TASK-001
          owner_record_kind: run
          owner_record_id: RUN-007
  tool_invocations:
    inserted:
      rows:
        - tool_name: harness.record_run
          idempotency_key: IDEMP-007
          status: committed
expected_events: []
expected_artifacts:
  - artifact_id: ART-007-DIFF
    kind: diff
    redaction_state: none
    relation_owner:
      record_kind: run
      record_id: RUN-007
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - The Write Authorization is not consumed twice.
  - No final acceptance, residual-risk acceptance, close state, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#record-run-payloads
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-record-run-missing-authorization-blocked
purpose: record_run rejects a product write without authorization.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 8
  change_units:
    - change_unit_id: CU-001
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-008
      idempotency_key: IDEMP-008
      expected_state_version: 8
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: implementation
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: null
    summary: "Product file was changed without a pre-write scope check."
    artifact_inputs: []
    payload:
      kind: implementation
      shaping_update: null
      implementation:
        outcome: completed
        product_write: true
        observed_changes:
          changed_paths:
            - path: app/settings/page.tsx
              change_kind: modified
              product_file: true
              within_change_unit: true
              before_sha256: SHA256-BEFORE-008
              after_sha256: SHA256-AFTER-008
          diff_artifact_input_ids: []
          no_product_changes: false
        command_results: []
        tool_invocations: []
        network_accesses: []
        secret_accesses: []
        evidence_updates:
          coverage_updates: []
          gap_blocker_refs: []
          summary: "Rejected before evidence mutation."
        implementation_notes: []
        follow_up_needed: []
      direct: null
expected_response:
  base:
    errors:
      - code: WRITE_AUTHORIZATION_REQUIRED
        details:
          authorization_reason: missing
  run_id: null
  state:
    lifecycle_phase: ready
  write_authorization_ref: null
  registered_artifacts: []
expected_state_changes: {}
expected_storage_rows:
  runs:
    inserted:
      count: 0
  write_authorizations:
    updated:
      count: 0
  artifacts:
    inserted:
      count: 0
  artifact_links:
    inserted:
      count: 0
  evidence_summaries:
    inserted:
      count: 0
    updated:
      count: 0
  tool_invocations:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors:
  - code: WRITE_AUTHORIZATION_REQUIRED
    details:
      authorization_reason: missing
forbidden_side_effects:
  - No Run, artifact, artifact link, evidence summary mutation, authorization consumption, blocker/gate update, task event, state-version advance, replay row, completion evidence, final acceptance, residual-risk acceptance, close state, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#record-run-payloads
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-record-run-observed-out-of-scope
purpose: Observed attempt outside authorized scope is rejected.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 9
  write_authorizations:
    - write_authorization_id: WA-009
      task_id: TASK-001
      change_unit_id: CU-001
      surface_id: reference-local-mcp
      status: active
      basis_state_version: 9
      attempt_scope_json:
        task_id: TASK-001
        change_unit_id: CU-001
        basis_state_version: 9
        surface_id: reference-local-mcp
        intended_operation: "Update settings copy."
        intended_paths: ["app/settings/page.tsx"]
        intended_tools: ["edit"]
        intended_commands: []
        product_file_write_intended: true
        intended_network: []
        intended_secret_scope: []
        sensitive_categories: []
        baseline_ref: BASE-001
        related_user_judgment_refs: []
        guarantee_level: cooperative
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-009
      idempotency_key: IDEMP-009
      expected_state_version: 9
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: implementation
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: WA-009
    summary: "Observed change includes a path outside the authorized scope."
    artifact_inputs: []
    payload:
      kind: implementation
      shaping_update: null
      implementation:
        outcome: completed
        product_write: true
        observed_changes:
          changed_paths:
            - path: app/billing/page.tsx
              change_kind: modified
              product_file: true
              within_change_unit: false
              before_sha256: SHA256-BEFORE-009
              after_sha256: SHA256-AFTER-009
          diff_artifact_input_ids: []
          no_product_changes: false
        command_results: []
        tool_invocations: []
        network_accesses: []
        secret_accesses: []
        evidence_updates:
          coverage_updates: []
          gap_blocker_refs: []
          summary: "Rejected before evidence mutation."
        implementation_notes: []
        follow_up_needed: []
      direct: null
expected_response:
  base:
    errors:
      - code: SCOPE_VIOLATION
  run_id: null
  state:
    lifecycle_phase: ready
  write_authorization_ref:
    record_kind: write_authorization
    record_id: WA-009
  registered_artifacts: []
expected_state_changes: {}
expected_storage_rows:
  runs:
    inserted:
      count: 0
  write_authorizations:
    unchanged:
      rows:
        - write_authorization_id: WA-009
          status: active
          consumed_by_run_id: null
  artifacts:
    inserted:
      count: 0
  artifact_links:
    inserted:
      count: 0
  evidence_summaries:
    inserted:
      count: 0
    updated:
      count: 0
  tool_invocations:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors:
  - code: SCOPE_VIOLATION
forbidden_side_effects:
  - The active Write Authorization is not consumed as success.
  - No Run, artifact, artifact link, evidence mutation, replay row, close readiness, completion evidence, final acceptance, residual-risk acceptance, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#record-run-payloads
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-raw-secret-artifact-blocked
purpose: Raw secret artifact storage is blocked before mutation.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: executing
      active_change_unit_id: CU-001
      state_version: 10
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-010
      idempotency_key: IDEMP-010
      expected_state_version: 10
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: direct
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: null
    summary: "Attempt to register a staged artifact classified as raw secret material."
    artifact_inputs:
      - input_id: ARTIN-010-SECRET
        source_kind: staged_file
        existing_artifact_ref: null
        staged:
          staged_uri: harness-staging://PROJ-001/RUN-010/raw-secret.log
          display_name: raw-secret.log
          content_type: text/plain
          expected_sha256: SHA256-SECRET-STAGED
          expected_size_bytes: 512
        capture: null
        kind: log
        redaction_state: none
        produced_by: lead_agent
        retention_class: task
        relation:
          task_id: TASK-001
          run_id: null
          record_kind: evidence_summary
          record_id_hint: EVID-001
        description: "Rejected because staged bytes are classified as raw secret material."
    payload:
      kind: direct
      shaping_update: null
      implementation: null
      direct:
        result_kind: no_change
        product_write: false
        direct_summary: "No product change; artifact registration was rejected."
        observed_changes:
          changed_paths: []
          diff_artifact_input_ids: []
          no_product_changes: true
        command_results: []
        tool_invocations: []
        network_accesses: []
        secret_accesses: []
        evidence_updates:
          coverage_updates:
            - claim_or_criterion: "Raw secret log must not be stored."
              coverage_state: blocked
              supporting_state_refs: []
              supporting_artifact_input_ids: ["ARTIN-010-SECRET"]
              note: "Rejected before artifact commit."
          gap_blocker_refs: []
          summary: "Forbidden raw-secret artifact input."
        user_visible_result: "Artifact storage was blocked."
        follow_up_needed: ["Provide a redacted or secret-omitted artifact."]
expected_response:
  base:
    errors:
      - code: VALIDATION_FAILED
  run_id: null
  registered_artifacts: []
expected_state_changes: {}
expected_storage_rows:
  runs:
    inserted:
      count: 0
  artifacts:
    inserted:
      count: 0
  artifact_links:
    inserted:
      count: 0
  evidence_summaries:
    updated:
      count: 0
  tool_invocations:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors:
  - code: VALIDATION_FAILED
forbidden_side_effects:
  - No raw secret bytes, token value, full sensitive log, rendered raw-secret content, external package, artifact row, artifact link, evidence sufficiency mutation, authorization consumption, close state, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#artifactinput
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md#artifact-and-evidence-boundary
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-evidence-summary-insufficient
purpose: Evidence summary insufficient remains visible as an active blocker.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: blocked
      active_change_unit_id: CU-001
      state_version: 11
  evidence_summaries:
    - evidence_summary_id: EVID-011
      task_id: TASK-001
      change_unit_id: CU-001
      status: partial
      gap_blocker_ids_json: ["BLK-011"]
  blockers:
    - blocker_id: BLK-011
      task_id: TASK-001
      blocked_action: close_task
      blocker_kind: evidence
      status: open
request:
  tool: harness.status
  payload:
    envelope:
      request_id: REQ-011
      idempotency_key: null
      expected_state_version: null
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    include:
      task: true
      gates: true
      projections: false
      pending_user_judgments: true
      guarantees: true
      user_judgments: true
      autonomy_boundary: true
      write_authority: true
      residual_risk: true
expected_response:
  base:
    errors: []
  active_task:
    lifecycle_phase: blocked
  evidence_summary:
    evidence_summary_ref:
      record_kind: evidence_summary
      record_id: EVID-011
    status: partial
  blocker_refs:
    - record_kind: blocker
      record_id: BLK-011
expected_state_changes: {}
expected_storage_rows:
  evidence_summaries:
    unchanged:
      rows:
        - evidence_summary_id: EVID-011
          status: partial
  blockers:
    unchanged:
      rows:
        - blocker_id: BLK-011
          blocker_kind: evidence
          status: open
  tool_invocations:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers:
  - code: EVIDENCE_INSUFFICIENT
    blocker_kind: evidence
expected_errors: []
forbidden_side_effects:
  - Status prose, Markdown evidence text, readable-view output, or agent summary does not repair missing evidence refs, create evidence, create artifacts, create final acceptance, accept residual risk, or close the Task.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessstatus
  schema: docs/*/reference/api/schema-core.md#current-position-display-schemas
  core: docs/*/reference/core-model.md#close_task
  storage: docs/*/reference/storage.md#fields-needed-for-close-blocker-calculation
  errors: docs/*/reference/api/errors.md#harnessclose_task-close-blockers
```

```yaml
scenario_id: MVP-ACTIVE-evidence-summary-sufficient
purpose: Evidence summary sufficient is supported by active Run and artifact refs.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: executing
      active_change_unit_id: CU-001
      state_version: 12
  write_authorizations:
    - write_authorization_id: WA-012
      task_id: TASK-001
      change_unit_id: CU-001
      surface_id: reference-local-mcp
      status: active
      basis_state_version: 12
      attempt_scope_json:
        task_id: TASK-001
        change_unit_id: CU-001
        basis_state_version: 12
        surface_id: reference-local-mcp
        intended_operation: "Update settings copy."
        intended_paths: ["app/settings/page.tsx"]
        intended_tools: ["edit"]
        intended_commands: []
        product_file_write_intended: true
        intended_network: []
        intended_secret_scope: []
        sensitive_categories: []
        baseline_ref: BASE-001
        related_user_judgment_refs: []
        guarantee_level: cooperative
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-012
      idempotency_key: IDEMP-012
      expected_state_version: 12
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: implementation
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: WA-012
    summary: "Implemented and checked the scoped copy update."
    artifact_inputs:
      - input_id: ARTIN-012-DIFF
        source_kind: staged_file
        existing_artifact_ref: null
        staged:
          staged_uri: harness-staging://PROJ-001/RUN-012/settings.diff
          display_name: settings.diff
          content_type: text/x-diff
          expected_sha256: SHA256-DIFF-012
          expected_size_bytes: 4096
        capture: null
        kind: diff
        redaction_state: none
        produced_by: lead_agent
        retention_class: task
        relation:
          task_id: TASK-001
          run_id: null
          record_kind: evidence_summary
          record_id_hint: EVID-012
        description: "Diff supporting the required evidence summary."
    payload:
      kind: implementation
      shaping_update: null
      implementation:
        outcome: completed
        product_write: true
        observed_changes:
          changed_paths:
            - path: app/settings/page.tsx
              change_kind: modified
              product_file: true
              within_change_unit: true
              before_sha256: SHA256-BEFORE-012
              after_sha256: SHA256-AFTER-012
          diff_artifact_input_ids: ["ARTIN-012-DIFF"]
          no_product_changes: false
        command_results: []
        tool_invocations:
          - tool_name: edit
            purpose: "Apply scoped copy update."
            status: succeeded
            artifact_input_ids: ["ARTIN-012-DIFF"]
            summary: "Changed the allowed file."
        network_accesses: []
        secret_accesses: []
        evidence_updates:
          coverage_updates:
            - claim_or_criterion: "Settings copy is updated."
              coverage_state: supported
              supporting_state_refs: []
              supporting_artifact_input_ids: ["ARTIN-012-DIFF"]
              note: "Diff supports the required claim."
          gap_blocker_refs: []
          summary: "Evidence is sufficient for the scoped update."
        implementation_notes: []
        follow_up_needed: []
      direct: null
expected_response:
  base:
    errors: []
  run_id: RUN-012
  state:
    lifecycle_phase: executing
  evidence_summary:
    evidence_summary_ref:
      record_kind: evidence_summary
      record_id: EVID-012
    status: sufficient
  registered_artifacts:
    - artifact_id: ART-012-DIFF
      kind: diff
      redaction_state: none
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: executing
  evidence_summaries:
    EVID-012:
      status: sufficient
  write_authorizations:
    WA-012:
      status: consumed
      consumed_by_run_id: RUN-012
expected_storage_rows:
  runs:
    inserted:
      rows:
        - run_id: RUN-012
          kind: implementation
          status: completed
          product_write: true
          write_authorization_id: WA-012
  artifacts:
    inserted:
      rows:
        - artifact_id: ART-012-DIFF
          kind: diff
          redaction_state: none
          status: available
  artifact_links:
    inserted:
      rows:
        - artifact_id: ART-012-DIFF
          owner_record_kind: evidence_summary
          owner_record_id: EVID-012
  evidence_summaries:
    inserted:
      rows:
        - evidence_summary_id: EVID-012
          task_id: TASK-001
          change_unit_id: CU-001
          status: sufficient
  write_authorizations:
    updated:
      rows:
        - write_authorization_id: WA-012
          status: consumed
          consumed_by_run_id: RUN-012
expected_events: []
expected_artifacts:
  - artifact_id: ART-012-DIFF
    kind: diff
    redaction_state: none
    relation_owner:
      record_kind: evidence_summary
      record_id: EVID-012
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - No final acceptance, residual-risk acceptance, close state, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#evidence-and-pre-write-scope-schemas
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md#artifact-and-evidence-boundary
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-final-acceptance-missing-close-blocker
purpose: Final acceptance missing is a close blocker.
initial_state:
  tasks:
    - task_id: TASK-001
      mode: work
      lifecycle_phase: executing
      result: none
      active_change_unit_id: CU-001
      state_version: 13
  evidence_summaries:
    - evidence_summary_id: EVID-013
      task_id: TASK-001
      change_unit_id: CU-001
      status: sufficient
  user_judgments: []
request:
  tool: harness.close_task
  payload:
    envelope:
      request_id: REQ-013
      idempotency_key: IDEMP-013
      expected_state_version: 13
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    intent: complete
    requested_close_reason: completed_self_checked
    user_note: null
    superseded_by_task_id: null
expected_response:
  base:
    errors:
      - code: ACCEPTANCE_REQUIRED
  close_state: blocked
  closed: false
  close_reason: none
  assurance_level: none
  evidence_summary:
    evidence_summary_ref:
      record_kind: evidence_summary
      record_id: EVID-013
    status: sufficient
  acceptance_state:
    status: required
    accepted_by_ref: null
    required_before_close: true
  blockers:
    - code: ACCEPTANCE_REQUIRED
      category: final_acceptance
      required_judgment_kind: final_acceptance
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: executing
      result: none
      close_reason: none
expected_storage_rows:
  tasks:
    unchanged:
      rows:
        - task_id: TASK-001
          lifecycle_phase: executing
          result: none
  user_judgments:
    inserted:
      count: 0
  blockers:
    inserted:
      rows:
        - task_id: TASK-001
          blocked_action: close_task
          blocker_kind: final_acceptance
          status: open
expected_events: []
expected_artifacts: []
expected_blockers:
  - code: ACCEPTANCE_REQUIRED
    category: final_acceptance
    required_judgment_kind: final_acceptance
expected_errors:
  - code: ACCEPTANCE_REQUIRED
forbidden_side_effects:
  - No terminal Task update, fabricated final_acceptance judgment, residual-risk acceptance, close record, final report authority, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessclose_task
  schema: docs/*/reference/api/schema-core.md#userjudgment
  core: docs/*/reference/core-model.md#close_task
  storage: docs/*/reference/storage.md#fields-needed-for-close-blocker-calculation
  errors: docs/*/reference/api/errors.md#harnessclose_task-close-blockers
```

```yaml
scenario_id: MVP-ACTIVE-residual-risk-visible-not-accepted-blocker
purpose: Residual risk visible but not accepted is a close blocker.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: executing
      active_change_unit_id: CU-001
      state_version: 14
  evidence_summaries:
    - evidence_summary_id: EVID-014
      task_id: TASK-001
      status: sufficient
  blockers:
    - blocker_id: BLK-RISK-014
      task_id: TASK-001
      blocked_action: close_task
      blocker_kind: residual_risk_visibility
      status: open
  user_judgments: []
request:
  tool: harness.close_task
  payload:
    envelope:
      request_id: REQ-014
      idempotency_key: IDEMP-014
      expected_state_version: 14
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    intent: complete
    requested_close_reason: completed_with_risk_accepted
    user_note: null
    superseded_by_task_id: null
expected_response:
  base:
    errors:
      - code: DECISION_REQUIRED
  close_state: blocked
  closed: false
  close_reason: none
  residual_risk_state:
    status: visible
    visible_refs:
      - record_kind: blocker
        record_id: BLK-RISK-014
    unaccepted_refs:
      - record_kind: blocker
        record_id: BLK-RISK-014
  blockers:
    - code: DECISION_REQUIRED
      category: residual_risk_acceptance
      required_judgment_kind: residual_risk_acceptance
      related_refs:
        - record_kind: blocker
          record_id: BLK-RISK-014
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: executing
      close_reason: none
expected_storage_rows:
  tasks:
    unchanged:
      rows:
        - task_id: TASK-001
          lifecycle_phase: executing
  user_judgments:
    inserted:
      count: 0
  blockers:
    unchanged:
      rows:
        - blocker_id: BLK-RISK-014
          blocker_kind: residual_risk_visibility
          status: open
expected_events: []
expected_artifacts: []
expected_blockers:
  - code: DECISION_REQUIRED
    category: residual_risk_acceptance
    required_judgment_kind: residual_risk_acceptance
expected_errors:
  - code: DECISION_REQUIRED
forbidden_side_effects:
  - Visible risk is not treated as accepted risk.
  - No residual-risk acceptance judgment, final acceptance, terminal Task update, close report authority, or non-active row/effect is fabricated.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessclose_task
  schema: docs/*/reference/api/schema-core.md#acceptedriskinput
  core: docs/*/reference/core-model.md#close_task
  storage: docs/*/reference/storage.md#fields-needed-for-close-blocker-calculation
  errors: docs/*/reference/api/errors.md#harnessclose_task-close-blockers
```

```yaml
scenario_id: MVP-ACTIVE-accepted-risk-close
purpose: Accepted-risk close succeeds only from active residual-risk acceptance state.
initial_state:
  tasks:
    - task_id: TASK-001
      mode: work
      lifecycle_phase: executing
      result: none
      active_change_unit_id: CU-001
      state_version: 15
  evidence_summaries:
    - evidence_summary_id: EVID-015
      task_id: TASK-001
      status: sufficient
  blockers:
    - blocker_id: BLK-RISK-015
      task_id: TASK-001
      blocked_action: close_task
      blocker_kind: residual_risk_visibility
      status: open
  user_judgments:
    - user_judgment_id: UJ-RISK-015
      task_id: TASK-001
      judgment_kind: residual_risk_acceptance
      presentation: short
      status: resolved
      judgment_payload_json:
        residual_risk_acceptance:
          risk_refs:
            - record_kind: blocker
              record_id: BLK-RISK-015
          accepted_scope: ["MVP-1 accepted-risk close path"]
request:
  tool: harness.close_task
  payload:
    envelope:
      request_id: REQ-015
      idempotency_key: IDEMP-015
      expected_state_version: 15
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    intent: complete
    requested_close_reason: completed_with_risk_accepted
    user_note: "Close with the visible accepted risk."
    superseded_by_task_id: null
expected_response:
  base:
    errors: []
  close_state: closed
  closed: true
  close_reason: completed_with_risk_accepted
  assurance_level: self_checked
  residual_risk_state:
    status: accepted
    accepted_refs:
      - record_kind: user_judgment
        record_id: UJ-RISK-015
  acceptance_state:
    status: not_required
    accepted_by_ref: null
    required_before_close: false
  state:
    lifecycle_phase: completed
    result: passed
    close_reason: completed_with_risk_accepted
  blockers: []
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: completed
      result: passed
      close_reason: completed_with_risk_accepted
  blockers:
    BLK-RISK-015:
      status: resolved
expected_storage_rows:
  tasks:
    updated:
      rows:
        - task_id: TASK-001
          lifecycle_phase: completed
          result: passed
          close_reason: completed_with_risk_accepted
  blockers:
    updated:
      rows:
        - blocker_id: BLK-RISK-015
          status: resolved
  user_judgments:
    unchanged:
      rows:
        - user_judgment_id: UJ-RISK-015
          judgment_kind: residual_risk_acceptance
          status: resolved
  tool_invocations:
    inserted:
      rows:
        - tool_name: harness.close_task
          idempotency_key: IDEMP-015
          status: committed
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - Accepted risk does not create sensitive-action permission, final acceptance, non-active row/effect, or an assurance upgrade beyond active `assurance_level` values.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessclose_task
  schema: docs/*/reference/api/schema-core.md#acceptedriskinput
  core: docs/*/reference/core-model.md#close_task
  storage: docs/*/reference/storage.md#fields-needed-for-close-blocker-calculation
  errors: docs/*/reference/api/errors.md#harnessclose_task-close-blockers
```

```yaml
scenario_id: MVP-ACTIVE-display-label-not-canonical
purpose: "Judgment fixture proving `judgment_kind` is the canonical judgment identity and display/localized labels are rendering text only."
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 16
  change_units:
    - change_unit_id: CU-001
      task_id: TASK-001
      status: active
request:
  tool: harness.request_user_judgment
  payload:
    envelope:
      request_id: REQ-016
      idempotency_key: IDEMP-016
      expected_state_version: 16
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    change_unit_id: CU-001
    judgment_kind: product_decision
    presentation: short
    context:
      why_now: "A product copy choice is needed before implementation."
      source_refs:
        - record_kind: task
          record_id: TASK-001
      evidence_refs:
        state_refs: []
        artifact_refs: []
    state_summary_at_request:
      mode: work
      lifecycle_phase: ready
      result: none
      close_reason: none
      assurance_level: none
      gates:
        scope_gate: passed
        decision_gate: required
        approval_gate: not_required
        design_gate: not_required
        evidence_gate: not_required
        acceptance_gate: not_required
    question: "Which settings copy should be used?"
    what_user_is_judging: "Product wording for the settings page."
    why_agent_cannot_decide: "The choice affects product behavior and tone."
    no_decision_consequence: "Implementation waits."
    what_agent_may_decide_without_user: ["Prepare the scoped edit after the decision."]
    affected_scope:
      task_ref:
        record_kind: task
        record_id: TASK-001
      change_unit_ref:
        record_kind: change_unit
        record_id: CU-001
      affected_object_refs: []
      write_refs: []
      close_refs: []
      scope_refs:
        - record_kind: change_unit
          record_id: CU-001
      product_areas: ["settings"]
      files_or_paths: ["app/settings/page.tsx"]
      acceptance_criteria_refs: []
      note: null
    affected_gates:
      - gate: decision_gate
        blocked_action: prepare_write
    affected_acceptance_criteria: []
    judgment_payload:
      options:
        - option_id: concise
          label: "Use concise copy"
          details: null
      recommendation:
        option_id: concise
        reason: "It keeps the narrow change clear."
        uncertainty: null
        when_to_revisit: null
      rationale: "The user owns product wording."
      uncertainty: null
      deferral_consequence: "The write remains blocked."
      user_context: null
      approval_scope: null
      covers: ["Settings copy choice"]
      does_not_cover: ["Sensitive-action approval", "final acceptance", "residual-risk acceptance"]
      acceptance: null
      qa_waiver: null
      verification_risk_acceptance: null
      residual_risk_acceptance: null
      cancellation: null
      separate_judgments_required: []
    expires_at: null
expected_response:
  base:
    errors: []
  user_judgment_id: UJ-016
  user_judgment_ref:
    record_kind: user_judgment
    record_id: UJ-016
  user_judgment:
    user_judgment_id: UJ-016
    task_id: TASK-001
    judgment_kind: product_decision
    presentation: short
    status: pending_user
  state:
    lifecycle_phase: waiting_user
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: waiting_user
  user_judgments:
    UJ-016:
      judgment_kind: product_decision
      presentation: short
      status: pending_user
expected_storage_rows:
  user_judgments:
    inserted:
      rows:
        - user_judgment_id: UJ-016
          task_id: TASK-001
          change_unit_id: CU-001
          judgment_kind: product_decision
          presentation: short
          status: pending_user
  tasks:
    updated:
      rows:
        - task_id: TASK-001
          lifecycle_phase: waiting_user
  tool_invocations:
    inserted:
      rows:
        - tool_name: harness.request_user_judgment
          idempotency_key: IDEMP-016
          status: committed
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - "`display_label` is absent from request payload, `expected_response.user_judgment`, expected storage rows, validator keys, gate keys, blocker keys, state-compatibility inputs, owner refs, close aggregation, and every canonical identity field."
  - "Localized labels (`Product decision`, `Technical decision`, `Scope decision`, `제품 판단`, `기술 판단`, `범위 판단`) may appear only as renderer output derived from `judgment_kind`; they are not accepted as authoritative request input and are not compared for compatibility, validators, gates, blockers, storage identity, state compatibility, or close aggregation."
  - "The pending judgment remains identified only by `user_judgment_id=UJ-016`, `judgment_kind=product_decision`, `presentation=short`, and `status=pending_user`; rendered labels do not resolve or mutate it."
  - No separate permission record, Write Authorization, evidence, final acceptance, residual-risk acceptance, close state, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrequest_user_judgment
  schema: docs/*/reference/api/schema-core.md#userjudgment
  core: docs/*/reference/core-model.md
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md
```

### Later/Profile Fixture Boundary

Detailed clarification catalogs, later-profile verification, full Evidence Manifest cases, Manual QA matrices, export non-leakage, browser QA capture, full operations recovery/export, broad connector conformance, preventive guard expansion, and isolated security profiles remain later/profile or Roadmap material unless an owner promotes a narrower fixture with stage impact and proof expectations. Listing a family in [Future Fixtures](../later/future-fixtures.md) does not make it an Engineering Checkpoint or MVP-1 requirement.

## Conformance Fixture Format

Future runtime conformance is fixture-based after Harness Server implementation and fixture materialization. A behavior-example table is not enough; each materialized test fixture must drive one request and assert structured response facts, Core state changes, storage rows, events, artifacts, blockers, errors, and forbidden side effects.

Each structured fixture draft must include this shape:

```yaml
scenario_id: string
purpose: string
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
schema_owners: object
```

Fixture shape summary: suite metadata can group fixtures, but the fixture body keeps one exact request-and-expectation shape for future executable conformance. The YAML block above is the contract summary.

Future fixture files and suite catalogs may carry metadata outside the fixture body. The fixture body itself uses only the fields above so a future conformance runner can compare behavior consistently. `purpose` states the behavior being constrained, and `schema_owners` names the active owner docs used to validate public request shape, schema values, Core transitions, storage rows, and errors. They are not public MCP request fields and are not passed to Core. Do not add fixture-body fields for suite delivery stage, assertion mode, docs-maintenance result, prose status, rendered Markdown, or authoring notes; those belong in suite catalog metadata, docs-maintenance reports, display owners, or surrounding documentation.

Fixture body type notation follows the API [Schema notation convention](api/schema-core.md#schema-notation-convention). All top-level fixture body fields above are required. Use `{}` or `[]` when the fixture intentionally supplies an empty object, object map, or array; omitting a required top-level field is an invalid fixture body, not "not asserted." For Engineering Checkpoint and MVP-1 active drafts, projection rendering is normally absent and active `expected_storage_rows` must not require `projection_jobs`. If a later promoted owner requires projection freshness, that promoted later/profile fixture asserts the Core/storage fact in `expected_state_changes.checks`, `expected_storage_rows.projection_jobs`, or another owner-defined structured location, not by matching rendered Markdown.

For an MCP tool request, future executable fixture `request.tool` names the public tool or operator action and `request.payload` is the tool's public request object as defined by the API docs. Active Engineering Checkpoint and MVP-1 fixture bodies must include `envelope: ToolEnvelope` and every required public request field before validation, canonicalization, request hashing, or Core execution. Suite metadata may help authors choose deterministic envelope values, but the materialized fixture body is invalid until those values are expanded into `request.payload`. The payload Core receives is the same public payload a surface would send to that MCP tool; there is no alternate request schema for fixtures.

Fixture shorthand is not a second API. Active Engineering Checkpoint and MVP-1 fixture bodies must not use shorthand values for public requests, seeded owner records, expected state, storage rows, events, artifacts, blockers, errors, or refs. Human-oriented tables in this document may use scenario IDs and compact summaries outside the fixture body, but a materialized active body must expand them to owner-defined records and public schemas. Later-profile shorthand details belong in [Future Fixtures: Later-Profile Fixture Shorthand Notes](../later/future-fixtures.md#later-profile-fixture-shorthand-notes) and are not active requirements for Engineering Checkpoint or MVP-1.

Future executable fixtures that seed `write_authorizations` must produce valid stored rows. Each seeded authorization row must include `basis_state_version` explicitly, or the future fixture loader must derive it from the seeded affected-scope state version for the row's Task before inserting into `state.sqlite`. This is a storage-loader derivation rule only; it does not add fixture top-level fields or change the fixture body shape. Partial `expected_state_changes.write_authorizations` or `expected_storage_rows.write_authorizations` assertions may omit `basis_state_version` unless the fixture is testing idempotent replay, stale detection, expiry, or audit behavior. `basis_state_version` is the `decision=allowed` basis, not the resulting `ToolResponseBase.state_version`. Future fixture loaders must not seed `blocked`, `approval_required`, `decision_required`, or `state_conflict` outcomes as `write_authorizations` rows; those outcomes use response decisions, blockers, validator findings, or errors.

Suite catalog metadata is not passed to Core and is not part of a fixture body. It can group exact-shape fixtures by suite, delivery stage, and tags:

```yaml
suite: agency
earliest_delivery_stage: "Assurance Profile"
tags: [decision-gate, residual-risk, autonomy-boundary]
fixtures:
  - AGENCY-user-judgment-required-before-product-tradeoff-write
  - AGENCY-residual-risk-visible-before-acceptance
```

A future runner may use this metadata to choose, order, or report suites. Core receives only `request.tool` and the public `request.payload`; metadata must not change seed expansion, fixture comparison semantics, tool request schemas, or expected owner records.

## Conformance Execution

Future `harness conformance run` will execute fixtures through the same Core entrypoints used by MCP tools and operator commands. It must not assert behavior by inspecting prose output alone.

Future runtime fixture execution semantics:

1. Load fixture YAML files and validate the exact fixture body shape, canonical active values, public `request.payload` schema, and absence of fixture-only shorthand.
2. Create a fresh fixture-only runtime home and temporary Product Repository for the fixture, unless the fixture explicitly targets an existing read-only sample. This fixture isolation is test hygiene for deterministic comparison; it is not an `isolated` guarantee level, OS sandboxing, permission isolation, or tamper-proof storage claim. The future runner must not reuse the developer's real Harness Runtime Home or Product Repository for state-changing fixture execution.
3. Seed `registry.sqlite`, `project.yaml`, `state.sqlite`, artifact files, and connector manifests from `initial_state`; seed projection files only for later/profile fixtures that have promoted projection requirements.
4. Execute `request.tool` through Core. MCP tool actions use the public request schema; fixture `request.payload` must be the same request payload a surface would send to that MCP tool. Operator actions such as `projection_refresh`, `doctor_surface`, `recover`, and `artifacts_check` use the operator semantics in [Operations And Conformance Reference](operations-and-conformance.md).
5. Capture returned response facts, resulting state summaries, storage effects, appended owner events, validator results when emitted, artifact registry/file integrity, structured blockers, projection job status when relevant, reconcile items when relevant, and returned error code.
6. Compare the captured results with `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, and `forbidden_side_effects`; empty expected sections mean the fixture asserts no relevant effect for that section.
7. Report fixture id, pass/fail, observed response/state/storage/event/artifact/blocker/error summary, projection freshness when relevant, and forbidden-side-effect comparison.

Future runner sequence summary: the numbered sequence above is the contract summary. A future runner loads an exact fixture body, seeds a fixture-only runtime home, executes the request through Core, compares response/state/storage/events/artifacts/blockers/errors/forbidden side effects, and emits a report.

When a fixture `request.payload.envelope` includes `expected_state_version`, the future runner compares it according to the Core-resolved primary Task, not only `ToolEnvelope.task_id`. Primary Task resolution order is tool-specific `task_id`, `ToolEnvelope.task_id`, then active Task resolution. Task-scoped actions compare against the seeded or Core-resolved primary Task State Version; project-scoped actions with no resolved primary Task compare against the Project State Version. Captured response, `EventRef.state_version`, and `task_events.state_version` values are compared as resulting affected-scope versions. Read-only fixtures may assert the unchanged version for the primary read scope. This clarifies comparison semantics without changing fixture body shape.

A stale `expected_state_version` fixture is a stale-authority test, not only a concurrent-write test. Exact idempotent replay is the exception: when a committed replay row exists and the canonical request hash matches, the fixture should assert the original committed response is returned and no current state-version freshness check is re-run. When no replay row exists and a state-changing action conflicts before commit, the fixture should assert that no current records changed, no `task_events` were appended, no artifacts were registered, no projection jobs were enqueued, and no `tool_invocations` replay row was created for the conflicting request unless an owner document explicitly defines a different recovery action. When the same key is reused with a changed canonical request hash, the fixture should assert `STATE_CONFLICT`, preserved original replay row, and no merged artifacts, events, projection jobs, response fields, or owner relations. For `dry_run=true`, fixtures should assert that diagnostics or `would_create` effects are returned without current records, `task_events`, artifacts, consumable Write Authorizations, projection jobs, or `tool_invocations` replay rows, and that the key is not reserved for later non-dry-run use. Replayed `prepare_write` must not create a duplicate authorization; replayed `record_run` must not consume authorization twice.

Fixture execution should be deterministic. Network access, wall-clock-sensitive expiry, and external tool output must be stubbed or represented as seeded fixture inputs unless a suite explicitly declares itself an integration smoke.

Fixture isolation is part of the pass condition. A fixture may seed files into its temporary Product Repository and runtime home, execute one Core or operator action there, and compare the captured result. This does not upgrade the product guarantee level. The fixture must not depend on existing local runtime records, generated operational files, or prose reports from a previous run.

Seed validation happens before action execution, and captured-state validation happens after action execution. Both sides of the comparison use owner-defined state loaders and value sets rather than fixture-local string labels.

Future conformance runners must seed and inspect JSON `TEXT` fields through the same Core storage loaders used by MCP tools and operator commands. A fixture with malformed JSON or schema-incompatible JSON in `initial_state` must surface invalid state, or a repairable state issue when the fixture action is a recovery path and safe reconstruction is possible. The future runner must not skip shape validation by treating JSON fields as opaque strings, and this expectation does not change the fixture body shape.

Future conformance runners must also seed and inspect status-like `TEXT` fields through the owner-bound hardening map in [Storage](storage.md#canonical-enum-hardening). For the main Engineering Checkpoint / MVP-1 path, future fixture seed loaders validate only the owner values actually present in the active stage's seeded records, and artifact/ref enum assertions use the API [stage-specific active value sets](api/schema-core.md#stage-specific-active-value-sets). Examples include registry/project surface guarantee, Run kind/status, Write Authorization status/guarantee, sensitive-action approval user-judgment status when that active judgment path is present, minimal evidence summary coverage/status when evidence support is active, residual-risk visibility/status when risk visibility is active, and current Task or Change Unit status when those owner records are used. Projection job kind/status belongs only to later/profile fixtures when a projection owner promotes durable projection-job storage. Committed Approval record lifecycle status and full Evidence Manifest status are later/profile-gated. Later-profile status fields stay with promoted owner docs and the future catalog until those profiles are active. Unknown status values remain invalid unless a scenario explicitly tests recovery from invalid state; expected-state status assertions compare captured owner values, not prose labels.

## Fixture Assertion Semantics

Fixture assertion modes are runner defaults or suite catalog metadata. They are not Core input, are not passed to MCP tools, and must not add fields to the fixture body. The fixture body remains exactly `scenario_id`, `purpose`, `initial_state`, `request`, `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, `forbidden_side_effects`, and `schema_owners`.

Within partial assertion objects, omission means "not asserted." A listed field with value `null` asserts that the captured field is present and equals JSON `null`. A listed array value `[]` asserts a present empty array. A listed object-map value `{}` asserts a present empty map when the owner schema says that field is a map. For structured objects under `partial_deep`, fixture authors should list at least one child field unless they are deliberately asserting only that the object exists.

These omission rules are assertion rules only. They do not make omitted fields valid in public MCP `request.payload`; fixture `request.payload` still validates against the owning public request schema.

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

`expected_events` comparisons are over the [Core Model Stable Event Catalog](core-model.md#stable-event-catalog) projection of captured `task_events`. API tool detail/audit event lists do not expand this set. Non-catalog detail or local-audit events captured in `task_events` must not make a normal staged-delivery fixture fail. When suite metadata sets `expected_events: exact`, exactness applies to the stable-event projection of the captured stream unless a future Roadmap/local suite explicitly opts into implementation-specific detail-event assertions. Validator IDs, Core check names, projection status notes, fixture authoring labels, and scenario catalog IDs are not event names. Prose examples may mention non-catalog event names as illustrative or future extension ideas, but executable staged-delivery fixtures must not require them until the Core Model event catalog promotes them.

Future conformance runners order captured `task_events` by `event_seq`. `state_version`, `created_at`, and `event_id` are not tie-breakers for `expected_events` ordering.

Fixture authors should use `VALIDATOR_FAILED` as an `expected_errors[].code` only when API precedence selects the generic validator fallback; a more specific active typed code such as `EVIDENCE_INSUFFICIENT`, `PROJECTION_STALE` for a readable-view freshness request, or `ARTIFACT_MISSING` remains primary when it applies. `PROJECTION_STALE` is not an active MVP close blocker, and QA-specific codes stay later/profile material until an owner promotes them.

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

Future fixture runners must use the same canonicalization rules as the reference implementation for `request_hash`, baseline `tree_hash`, and projection `managed_hash`. The detailed algorithms remain owned by [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [Storage](storage.md), and [Projection And Templates Reference](projection-and-templates.md) as applicable; conformance fixtures assert deterministic behavior without redefining those source-of-truth boundaries.

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
| 1 | `MVP-ACTIVE-task-change-unit-setup` | `harness.intake` | Registered local project with no current Task | Task `tasks.lifecycle_phase=ready`, one Change Unit or scope boundary, current-task pointer, and no write authority. | None | No Run, artifact, evidence, final acceptance, residual-risk acceptance, close, or authority-rendering effect. |
| 2 | `MVP-ACTIVE-shaping-update-persists` | `harness.record_run` with `kind=shaping_update`, `payload.kind=shaping_update`, and `product_write=false` represented by the active payload branch | Task `tasks.lifecycle_phase=shaping` and Change Unit | Shaping updates persist into Task/Change Unit state and a `runs.kind=shaping_update` row without product-write authority. | None | No Write Authorization, product-write Run, non-active row/effect, final acceptance, or residual-risk acceptance. |
| 3 | `MVP-ACTIVE-prepare-write-allowed-authorization` | `harness.prepare_write` | Task `tasks.lifecycle_phase=ready`, compatible scope, current expected state, and proposed attempt-scope fields in the public request | `decision=allowed`, `tasks.lifecycle_phase=ready`, one active Write Authorization whose `attempt_scope_json` matches `WriteAuthorizationSummary.attempt_scope`, replay row, no Run. | None | No OS permission, sandbox, preventive, isolated, evidence, or close claim. |
| 4 | `MVP-ACTIVE-prepare-write-blocked-no-authorization` | `harness.prepare_write` | Task `tasks.lifecycle_phase=ready` with incompatible requested path or missing compatible scope | Structured blocked response, Task `tasks.lifecycle_phase=blocked`, `write_authorization_ref=null`, `write_authorization=null`, and no consumable Write Authorization row. | `SCOPE_REQUIRED`, `NO_ACTIVE_CHANGE_UNIT`, or `SCOPE_VIOLATION` as owned by the API/Core path. | No authorization, Run, artifact, evidence mutation, non-active effect, close, final acceptance, or residual-risk acceptance. |
| 5 | `MVP-ACTIVE-prepare-write-idempotent-replay` | `harness.prepare_write` replay | Existing committed replay row, original stored `request_hash`, and original active authorization | Original response, original stored `request_hash`, original `write_authorization_ref`, and `authorization_effect=returned` are returned. | None | No duplicate authorization, event, artifact, replay update, non-active effect, or state-version increment. |
| 6 | `MVP-ACTIVE-idempotency-key-hash-conflict` | State-changing tool with same idempotency key and different hash | Existing committed replay row | `STATE_CONFLICT`; original replay row and stored `request_hash` remain unchanged. | `STATE_CONFLICT` | No merged response, new authorization, event, artifact, non-active effect, owner relation, or replay row update. |
| 7 | `MVP-ACTIVE-record-run-consumes-authorization` | `harness.record_run` with `kind=implementation`, `payload.kind=implementation`, and only `payload.implementation` non-null | Task `tasks.lifecycle_phase=ready`, compatible scope, active Write Authorization whose stored `AuthorizedAttemptScope` matches the observed attempt | One Run is recorded with compatible `observed_attempt_json`, the authorization is consumed exactly once, and Task execution assertions use `tasks.lifecycle_phase=executing`. | None | No second consumption, final acceptance, residual-risk acceptance, non-active assurance state, or close. |
| 8 | `MVP-ACTIVE-record-run-missing-authorization-blocked` | `harness.record_run` with `kind=implementation`, `payload.kind=implementation`, only `payload.implementation` non-null, and `write_authorization_id=null` | Task `tasks.lifecycle_phase=ready` and product-write Run request with no authorization | Product-write Run is rejected before commit and the stored Task state remains unchanged. | `WRITE_AUTHORIZATION_REQUIRED` with `authorization_reason=missing` when details assert the reason | No Run, authorization consumption, completion evidence, artifact link, evidence mutation, non-active effect, event, state-version advance, or replay row. |
| 9 | `MVP-ACTIVE-record-run-observed-out-of-scope` | `harness.record_run` with `kind=implementation`, `payload.kind=implementation`, and only `payload.implementation` non-null | Active Write Authorization whose stored `AuthorizedAttemptScope` excludes the observed path, command, network target, secret, sensitive category, baseline, Task, Change Unit, or surface | Out-of-scope observation is rejected before commit in the active draft and does not consume the authorization as success. | `SCOPE_VIOLATION` | Authorization is not consumed; no Run, artifact, evidence mutation, replay row, completion evidence, close readiness, or non-active row is created. |
| 10 | `MVP-ACTIVE-raw-secret-artifact-blocked` | `harness.record_run` with `kind=direct`, `payload.kind=direct`, only `payload.direct` non-null, `write_authorization_id=null`, `product_write=false`, and active `ArtifactInput` | Task `tasks.lifecycle_phase=executing`, Run path, and active `ArtifactInput` shape that attempts forbidden raw-secret evidence | Raw secret bytes are rejected before mutation in the active draft; a separate committed metadata-notice branch would need matching artifact/storage/error assertions. | `VALIDATION_FAILED` for forbidden input shape/source or raw secret payload before mutation; `ARTIFACT_MISSING` only for missing or integrity-failed committed artifact refs. | No raw secret storage, rendering, export, evidence sufficiency, authorization consumption, or close. |
| 11 | `MVP-ACTIVE-evidence-summary-insufficient` | `harness.status` | Task `tasks.lifecycle_phase=blocked` with partial/missing evidence summary and active evidence blocker | Evidence summary remains `partial` and close-relevant evidence blocker stays visible without mutation. | `EVIDENCE_INSUFFICIENT` blocker when close/write path depends on it | Status prose or Markdown evidence list does not repair missing refs, create artifacts, or close the Task. |
| 12 | `MVP-ACTIVE-evidence-summary-sufficient` | `harness.record_run` with `kind=implementation`, `payload.kind=implementation`, only `payload.implementation` non-null, and active `ArtifactInput` | Task `tasks.lifecycle_phase=executing`, compatible authorization whose stored `AuthorizedAttemptScope` matches the observed attempt, and a non-secret staged artifact allowed as `redaction_state=none` unless redaction or omission applies. | Registered artifact refs and evidence summary become sufficient from owner records while Task remains `tasks.lifecycle_phase=executing` until close. | None | No non-active evidence/assurance row, final acceptance, residual-risk acceptance, or close state. |
| 13 | `MVP-ACTIVE-final-acceptance-missing-close-blocker` | `harness.close_task` with `intent=complete`, `requested_close_reason=completed_self_checked` | Task with evidence sufficient but required final acceptance missing | Close remains blocked with a final-acceptance blocker and no terminal Task update. | `ACCEPTANCE_REQUIRED` | No `tasks.lifecycle_phase=completed` or `tasks.lifecycle_phase=cancelled`, fabricated acceptance, residual-risk acceptance, non-active assurance state, or close report authority. |
| 14 | `MVP-ACTIVE-residual-risk-visible-not-accepted-blocker` | `harness.close_task` with `intent=complete`, `requested_close_reason=completed_with_risk_accepted` | Task with visible close-relevant residual risk and no compatible `judgment_kind=residual_risk_acceptance` user judgment | Residual-risk acceptance remains required and close does not mark the Task terminal. | `DECISION_REQUIRED` or `DECISION_UNRESOLVED` with `required_judgment_kind=residual_risk_acceptance` | Visible risk is not accepted risk; no non-active risk/assurance row or close state is fabricated. |
| 15 | `MVP-ACTIVE-accepted-risk-close` | `harness.close_task` with `intent=complete`, `requested_close_reason=completed_with_risk_accepted` | Task with sufficient evidence, visible risk, and compatible `judgment_kind=residual_risk_acceptance` | Task closes with `tasks.lifecycle_phase=completed`, accepted-risk close reason, and refs to the user judgment. | None | Accepted risk does not create Approval, final acceptance, non-active assurance state, or assurance upgrade. |
| 16 | `MVP-ACTIVE-display-label-not-canonical` | `harness.request_user_judgment` | Task `tasks.lifecycle_phase=ready`, Change Unit, and no preexisting committed user judgment for this request | Committed non-dry-run request creates one pending `user_judgments` row and response `UserJudgment` with `judgment_kind=product_decision`, `presentation=short`, and `status=pending_user`; any `display_label`, Product decision, Technical decision, Scope decision, `제품 판단`, `기술 판단`, or `범위 판단` text is rendering text only. | None | `display_label` and localized labels are not authoritative request input, canonical state, validator keys, gate keys, blocker keys, storage identity, state-compatibility inputs, compatibility checks, or close aggregation keys. |

The queue above is intentionally small. Engineering Checkpoint does not require a full conformance suite, broad catalog family coverage, final-acceptance success semantics, later assurance checks, export/recover, reconcile, stewardship, context hygiene, browser QA capture, or future guarantee-level checks. MVP-1 adds the listed user-loop judgment, evidence, close-blocker, and accepted-risk drafts without promoting later assurance checks, export, or profile fixtures.

## Future Fixtures

Scenario families have moved to [Future Fixtures](../later/future-fixtures.md) so the early reference stays focused on the core conformance model. That catalog contains compact future-oriented inventory for browser QA capture, cross-surface behavior, export non-leakage, context hygiene, reconcile, stewardship, full operations, advanced projection rendering, artifact redaction and integrity, and future guarantee-level checks.

Those catalog entries are design inventory only until a promoted owner path materializes exact-shape executable fixtures. They are not required for Engineering Checkpoint, do not expand MVP-1 by themselves, and do not count as runtime conformance while this repository remains documentation-only.

## Metrics Boundary

Long-term operational metrics are derived analytics, not staged-delivery-critical state or conformance requirements. Keep metrics such as approval turnaround, verification latency, projection stale duration, same-session guard frequency, and surface fallback rate in the [roadmap](../roadmap.md) as read-only diagnostics until a future version promotes them with owner docs, fixtures or a conformance target, fallback behavior, relevant redaction/retention policy, no projection-as-canonical dependency, and implementation ownership.
