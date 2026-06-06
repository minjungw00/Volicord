# Conformance Fixtures Reference

## What this document helps you do

Use this reference to look up the three-layer boundary for Harness conformance material: documentation checks, active structured fixture drafts, and future runtime conformance. It explains what future conformance will prove, the active Kernel Smoke, MVP-1 user-loop, security/capability, and artifact/evidence draft families, canonical active fixture-value rules, exact structured fixture draft shape, future runner execution behavior, fixture assertion semantics, current-phase status, and the boundary to the future fixture catalog.

This is a lookup document for conformance authors, implementers, and maintainers. It is not an operator procedure; use [Operations And Conformance Reference](operations-and-conformance.md) for operator entrypoints and the `harness conformance run` overview.

This is reference documentation for future conformance work. The current repository is documentation-only and contains no runnable Harness Server conformance tests; current phase and handoff status are tracked in [Implementation Overview](../build/implementation-overview.md#documentation-acceptance-status).

## Read this when

- You are writing or reviewing the future fixture-based conformance design.
- You need the exact fixture body fields, the canonical active value boundary, the `request.payload` public request schema rule, or runner isolation behavior.
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
| How a runner loads, seeds, executes, captures, and compares | [Conformance Execution](#conformance-execution) |
| Default comparison modes for `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, and `forbidden_side_effects` | [Fixture Assertion Semantics](#fixture-assertion-semantics) |
| Active structured fixture draft families | [Kernel Smoke Behavior Examples](#engineering-checkpoint-behavior-examples), [MVP-1 User Work Loop Behavior Examples](#mvp-1-user-work-loop-behavior-examples), [Security And Capability Behavior Examples](#security-and-capability-behavior-examples), and [Artifact And Evidence Behavior Examples](#artifact-and-evidence-behavior-examples) |
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

These draft families are the active future-authoring target for Engineering Checkpoint and MVP-1. They are not executable fixtures yet, not generated runtime artifacts, and not current pass/fail criteria. The tables below preserve the active scenario IDs and proof intent, but they are not fixture bodies. When a future implementation materializes a fixture body for any row, the body must follow [Conformance Fixture Format](#conformance-fixture-format) and the canonical active value rules below.

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
- `expected_events` must name stable event facts only after the Core owner promotes them. Human labels such as `owner-promoted Run recording event` are authoring notes, not active event values.
- `expected_artifacts` must use active `ArtifactRef`, `ArtifactInput`, relation owner, redaction, retention, and artifact status values from [API Schema Core](api/schema-core.md#artifactref), [ArtifactInput](api/schema-core.md#artifactinput), and [Storage](storage.md).
- Active `redaction_state` values in `ArtifactInput`, `ArtifactRef`, `expected_artifacts`, and `expected_storage_rows.artifacts` are exactly `none`, `redacted`, `secret_omitted`, and `blocked`. Use `none` only for stored bytes allowed without redaction, `redacted` when content was removed before storage, `secret_omitted` when secret or PII material is omitted or replaced by handles, and `blocked` when raw-payload storage or exposure is blocked. Values such as `visible`, `hidden`, `safe`, and `unsafe` are not redaction states.
- `expected_blockers` and `expected_response.blockers` must use active blocker categories, `required_judgment_kind` values, related refs, and close-blocker shapes from [MVP API](api/mvp-api.md#harnessclose_task), [API Errors](api/errors.md#harnessclose_task-close-blockers), and Core/storage owners. Active close/status blocker assertions must not use detached verification, Manual QA, full Evidence Manifest, export/report freshness, profile-required verification, or broad approval lifecycle categories.
- Sensitive-action approval expectations must use active `user_judgment` / `judgment_kind=sensitive_approval`, `approval_scope`, `approval_gate`, active `sensitive_approval` blocker category, or API-owned `APPROVAL_REQUIRED` / `APPROVAL_DENIED` / `APPROVAL_EXPIRED` codes. They must not assert broad approval text or a committed Approval record lifecycle. `decision_required` / `DECISION_REQUIRED` remains for user-owned judgments that are not sensitive-action permission and must not be used as a synonym for sensitive-action approval.
- `harness.close_task` fixture bodies must use `CloseTaskRequest.intent` only as `complete`, `cancel`, or `supersede`. Normal completion and accepted-risk completion both use `intent=complete`; accepted risk is expressed through `requested_close_reason=completed_with_risk_accepted` and compatible active Core state, not by changing `intent`. Cancellation uses `intent=cancel` with `requested_close_reason=cancelled`; supersession uses `intent=supersede` with `requested_close_reason=superseded` and the API-owned supersession fields when applicable. Active fixture bodies must not use close reasons or later/profile assurance values as intent values.
- `expected_errors` must use active public `ErrorCode` values and primary-error precedence from [API Errors](api/errors.md). Validator IDs or policy finding codes belong under owner-defined validator/state assertions, not as primary `expected_errors[].code` unless the public API owner selects that code.
- `harness.record_run` error fixtures must use the active mapping in [API Errors](api/errors.md#error-taxonomy): missing required authorization uses `WRITE_AUTHORIZATION_REQUIRED` with `authorization_reason=missing` when details assert the reason; stale, expired, revoked, consumed, or incompatible authorization uses `WRITE_AUTHORIZATION_INVALID` with the matching `authorization_reason`; observed work outside the stored `AuthorizedAttemptScope` uses `SCOPE_VIOLATION`; unsupported observation or insufficient surface capability for a required comparison uses `CAPABILITY_INSUFFICIENT`; forbidden secret or artifact handling uses `VALIDATION_FAILED`, `SCOPE_VIOLATION`, or `ARTIFACT_MISSING` according to the owner mapping.
- `forbidden_side_effects` may be readable in documentation drafts, but materialized executable fixtures should expand each forbidden effect into owner-record absence, row-effect, artifact, event, projection, or generated-output assertions where practical.
- `harness.record_run` fixture bodies must align `RecordRunRequest.kind`, `RecordRunPayload.kind`, and the one non-null `RecordRunPayload` branch exactly. Active bodies may use only `shaping_update`, `implementation`, or `direct`: Discovery and requirements-shaping updates use `shaping_update`; implementation writes and implementation attempts use `implementation`; write-free direct observations and non-product operations use `direct`. Legacy or shorthand run-kind values, unknown payload branch names, and multiple non-null payload branches are invalid.
- Later/profile-only values, branches, methods, refs, projection kinds, table families, status values, and errors must not appear in active MVP fixture bodies. They stay in [Schema Later](api/schema-later.md), promoted later/profile owner docs, or [Future Fixtures](../later/future-fixtures.md) until an owner promotes the narrower path.

Deterministic IDs such as `task-fixture-001` are acceptable only as ordinary string IDs inside valid owner records and matching refs. A symbolic ID must not stand in for omitted required records, omitted request fields, unsupported schema branches, fixture-local status values, or unexpanded artifact refs.

<a id="engineering-checkpoint-behavior-examples"></a>

### Kernel Smoke Behavior Examples

Kernel Smoke is the narrow authoring label for the first executable authority loop. The rows below define active proof targets, not current fixture files. A future materialized body for any row must use the public request schema named here and active owner values in every expected section.

| Scenario ID | Public request owner | Canonical payload and value notes | Required structured proof |
|---|---|---|---|
| `MVP-ACTIVE-task-change-unit-setup` | Owner-valid setup path; if exposed through `harness.intake`, use `IntakeRequest`. | `request.payload` must include `envelope`, `user_request`, `requested_mode`, `resume_policy`, `acceptance_criteria`, `constraints`, and `initial_context_refs`. Scope belongs in active Task/Change Unit fields such as `constraints.allowed_paths`, `change_units.allowed_paths_json`, or owner-defined scope fields, not `initial_scope`. | One current Task pointer, one Change Unit or scope boundary for implementation-ready work, valid `tasks.mode`, `tasks.lifecycle_phase=ready`, no Write Authorization, Run, artifact, evidence, final acceptance, residual-risk acceptance, close, or projection-as-authority effect. |
| `MVP-ACTIVE-shaping-update-persists` | `harness.record_run` / `RecordRunRequest`. | Use `kind=shaping_update`; `payload.kind=shaping_update`; `payload.shaping_update` is the only non-null branch. Do not use shorthand run-kind values or top-level `shaping_update` outside `payload`. | Shaping updates persist into Task/Change Unit owner fields with `tasks.lifecycle_phase=shaping` and a valid `runs.kind=shaping_update` row without product-write authority. |
| `MVP-ACTIVE-prepare-write-allowed-authorization` | `harness.prepare_write` / `PrepareWriteRequest`. | Use `product_file_write_intended`, `intended_paths`, `intended_tools`, `intended_commands`, `intended_network`, `intended_secret_scope`, `sensitive_categories`, and `baseline_ref` as defined by the public schema. `dry_run` and `expected_state_version` live in `envelope`. | `decision=allowed`, `authorization_effect=created`, `tasks.lifecycle_phase=ready`, active `write_authorizations.status=active`, stored `AuthorizedAttemptScope`, replay row only for committed non-dry-run response, and no OS-permission, sandbox, preventive, isolated, evidence, or close claim. |
| `MVP-ACTIVE-prepare-write-blocked-no-authorization` | `harness.prepare_write` / `PrepareWriteRequest`. | Same public request shape as the allowed case; incompatible paths, missing active scope, or no active Change Unit must be represented through public fields and active Core state. | Structured blocker/error such as `SCOPE_REQUIRED`, `NO_ACTIVE_CHANGE_UNIT`, or `SCOPE_VIOLATION`; Task lifecycle uses `tasks.lifecycle_phase=blocked`; no consumable Write Authorization, Run, artifact, replay row for pre-commit failure, or projection job. |
| `MVP-ACTIVE-prepare-write-idempotent-replay` | `harness.prepare_write` / `PrepareWriteRequest`. | The repeated call uses the same public request payload and `envelope.idempotency_key`. Do not put `canonical_request_hash` in `request.payload`; the hash is a stored/captured assertion under `tool_invocations`. | Original committed response and original `write_authorization_ref` are returned; no duplicate authorization, event, artifact, replay update, projection job, or state-version increment. |
| `MVP-ACTIVE-idempotency-key-hash-conflict` | Any active state-changing public tool, commonly `harness.prepare_write`. | The conflicting call uses the same `envelope.idempotency_key` with a different canonical public payload. The fixture asserts the stored and observed hash facts under storage/comparison fields, not as public request fields. | Primary `STATE_CONFLICT`, preserved original replay row, and no merged response fields, events, artifacts, projection jobs, owner relations, or replay row update. |
| `MVP-ACTIVE-record-run-consumes-authorization` | `harness.record_run` / `RecordRunRequest`. | Use `kind=implementation`; `payload.kind=implementation`; `payload.implementation` is the only non-null branch. `observed_changes.changed_paths[]` uses `ChangedPath` objects, not bare path strings. | One compatible Run is recorded, the active Write Authorization is consumed exactly once, Task execution assertions use `tasks.lifecycle_phase=executing`, registered evidence/artifact refs use active schemas, and no final acceptance, residual-risk acceptance, later assurance/profile state, or close state is created. |
| `MVP-ACTIVE-record-run-missing-authorization-blocked` | `harness.record_run` / `RecordRunRequest`. | Use `kind=implementation`; `payload.kind=implementation`; `payload.implementation` is the only non-null branch. The product-write implementation attempt uses `write_authorization_id=null` to test missing authorization. | Primary `WRITE_AUTHORIZATION_REQUIRED`; Task lifecycle uses `tasks.lifecycle_phase=blocked`; no Run, artifact link, evidence update, authorization consumption, projection job, event, state-version advance, or replay row. |
| `MVP-ACTIVE-record-run-stale-authorization-blocked` | `harness.record_run` / `RecordRunRequest`. | Use `kind=implementation`; `payload.kind=implementation`; `payload.implementation` is the only non-null branch. The product-write implementation attempt supplies an existing Write Authorization whose compatibility basis is stale for the current Task state. | Primary `WRITE_AUTHORIZATION_INVALID` with `authorization_reason=stale` when details assert the reason; Task lifecycle uses `tasks.lifecycle_phase=blocked`; the stale authorization is not consumed, and no Run, artifact link, evidence update, projection job, event, state-version advance, or replay row is created. |
| `MVP-ACTIVE-record-run-observed-out-of-scope` | `harness.record_run` / `RecordRunRequest`. | Use `kind=implementation`; `payload.kind=implementation`; `payload.implementation` is the only non-null branch. Observed paths, commands, network, secrets, and sensitive categories use active `ImplementationPayload` observation fields. | Primary `SCOPE_VIOLATION`; Task lifecycle uses `tasks.lifecycle_phase=blocked`; invalid authorization is not consumed, and the observation is not completion evidence or close readiness. |
| `MVP-ACTIVE-record-run-capability-insufficient` | `harness.record_run` / `RecordRunRequest`. | Use `kind=implementation`; `payload.kind=implementation`; `payload.implementation` is the only non-null branch. The scenario requires a comparison over observed commands, network, secret access, artifact capture, pre-tool blocking, isolation, or changed paths that the connected `capability_profile` cannot observe or attest. | Primary `CAPABILITY_INSUFFICIENT`; unsupported facts are not marked verified, guarantee display is lowered or blocked through active response/state fields, and no Run, artifact link, evidence update, authorization consumption, completion evidence, close readiness, event, state-version advance, or replay row is created for the rejected attempt. |

<a id="mvp-1-user-work-loop-behavior-examples"></a>

### MVP-1 User Work Loop Behavior Examples

MVP-1 behavior examples describe user-visible Harness value without growing into the broad assurance or operations catalog. Future fixtures may use exactly `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, and `harness.close_task` where those methods are active for the stage. A separate `harness.next` fixture belongs to later/compatibility material.

| Scenario ID | Public request owner | Canonical payload and value notes | Required structured proof |
|---|---|---|---|
| `MVP-ACTIVE-evidence-summary-insufficient` | `harness.status` / `StatusRequest`, or a promoted evidence owner read. | `StatusRequest` uses `envelope` plus `include` flags. Status is read-only and does not participate in committed replay. | Existing `evidence_summaries.status` remains an active value such as `partial`, `stale`, or `blocked`; evidence blockers use active `EVIDENCE_INSUFFICIENT` semantics when the close/write path depends on them; Task lifecycle uses `tasks.lifecycle_phase=blocked` for the close/write-blocked status; no mutation occurs. |
| `MVP-ACTIVE-evidence-summary-sufficient` | `harness.record_run` / `RecordRunRequest`. | Use `kind=implementation`; `payload.kind=implementation`; `payload.implementation` is the only non-null branch. Artifact evidence uses active `ArtifactInput` with `input_id`, `source_kind`, `kind`, `redaction_state`, `produced_by`, `retention_class`, and `relation`; do not use bare `staged_uri` entries in `artifact_inputs`. | Registered `ArtifactRef` values and `artifact_links` support an active `evidence_summaries.status=sufficient` update while the Task remains `tasks.lifecycle_phase=executing` until close; no full Evidence Manifest, later assurance/profile state, final acceptance, or residual-risk acceptance is created. |
| `MVP-ACTIVE-final-acceptance-missing-close-blocker` | `harness.close_task` / `CloseTaskRequest`. | Use `intent=complete` and `requested_close_reason=completed_self_checked` for this blocker case. Do not use `completed_with_risk_accepted` as an `intent`; close reasons stay in `requested_close_reason`. | Close remains blocked with a structured final-acceptance blocker and primary `ACCEPTANCE_REQUIRED`; Task lifecycle assertions use `tasks.lifecycle_phase=blocked`, and no final_acceptance judgment is fabricated. |
| `MVP-ACTIVE-residual-risk-visible-not-accepted-blocker` | `harness.close_task` / `CloseTaskRequest`. | Use `intent=complete` and `requested_close_reason=completed_with_risk_accepted` to exercise the missing residual-risk-acceptance blocker. Residual-risk acceptance is read from active blockers and `user_judgments`, not from a rich residual-risk record. | Structured residual-risk-acceptance blocker with `required_judgment_kind=residual_risk_acceptance`; primary `DECISION_REQUIRED` or `DECISION_UNRESOLVED`; Task lifecycle assertions use `tasks.lifecycle_phase=blocked`, and no close state or accepted-risk claim is fabricated. |
| `MVP-ACTIVE-accepted-risk-close` | `harness.close_task` / `CloseTaskRequest`. | Use `intent=complete` with `requested_close_reason=completed_with_risk_accepted`. The request does not carry accepted-risk refs; Core reads compatible `judgment_kind=residual_risk_acceptance` state and blocker refs. | Task closes with `tasks.lifecycle_phase=completed` and `close_reason=completed_with_risk_accepted`, accepted-risk refs point to active `user_judgment` / `blocker` refs, and no Approval, final acceptance, rich residual-risk row, later assurance/profile state, or assurance upgrade is created. |
| `MVP-ACTIVE-display-label-not-canonical` | `harness.request_user_judgment` / `RequestUserJudgmentRequest`. | Use `judgment_kind=product_decision`, `presentation=short`, and full public judgment request fields. Localized labels may appear only in rendered response/display fields, never as schema, storage, blocker, gate, or close aggregation keys. | `user_judgments.judgment_kind=product_decision`; Task lifecycle uses `tasks.lifecycle_phase=waiting_user` for the pending user-owned judgment; `display_label` is not a storage column or canonical state value; no decision is resolved by requesting it. |

<a id="security-and-capability-behavior-examples"></a>

### Security And Capability Behavior Examples

Security and capability examples prove honest local capability display and unavailable-path behavior. Active MVP fixture bodies may assert `CAPABILITY_INSUFFICIENT`, cooperative/detective profile facts, lowered guarantee display, or no-authority unavailable responses through active API/Core/storage fields. They must not include preventive guard expansion, isolated profile claims, OS permission, arbitrary-tool sandboxing, tamper-proof storage, or pre-tool blocking values unless a promoted owner path defines and proves that later/profile behavior.

<a id="artifact-and-evidence-behavior-examples"></a>

### Artifact And Evidence Behavior Examples

Artifact examples prove registered bytes and metadata, not report wording. Active fixture bodies use `ArtifactInput` and `ArtifactRef` exactly as defined by API Schema Core. A raw secret, token, arbitrary absolute path, unsupported capture source, or full sensitive log in an active artifact input is rejected before mutation with the public error mapping owned by [API Errors](api/errors.md#error-taxonomy), commonly `VALIDATION_FAILED` for forbidden input shape/source or raw secret payload before mutation. `ARTIFACT_MISSING` applies to missing or integrity-failed committed artifact refs. Broader export non-leakage remains later/profile catalog material.

A materialized `MVP-ACTIVE-raw-secret-artifact-blocked` fixture must choose one consistent assertion branch. If Core rejects the raw-secret artifact before mutation, `expected_errors` includes `VALIDATION_FAILED`, `expected_artifacts: []`, `expected_storage_rows` asserts no `artifacts`, `artifact_links`, or evidence-sufficiency mutation, and `forbidden_side_effects` includes no raw secret storage, rendering, or export. If Core commits an owner-approved metadata notice instead, `expected_errors: []`, `expected_artifacts` contains only the committed notice with `redaction_state=blocked` or `redaction_state=secret_omitted`, `expected_storage_rows` asserts only those artifact/link/evidence effects, and `forbidden_side_effects` still forbids raw secret bytes. A fixture must not both block raw-payload storage and expect a stored raw secret artifact.

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

For an MCP tool request, future executable fixture `request.tool` names the public tool or operator action and `request.payload` is the tool's public request object as defined by the API docs. Active Engineering Checkpoint and MVP-1 fixture bodies must include `envelope: ToolEnvelope` and every required public request field before validation, canonicalization, request hashing, or Core execution. Suite metadata may help authors choose deterministic envelope values, but the materialized fixture body is invalid until those values are expanded into `request.payload`. The payload Core receives is the same public payload a surface would send to that MCP tool; there is no alternate request schema for fixtures.

Fixture shorthand is not a second API. Active Engineering Checkpoint and MVP-1 fixture bodies must not use shorthand values for public requests, seeded owner records, expected state, storage rows, events, artifacts, blockers, errors, or refs. Human-oriented tables in this document may use scenario IDs and compact summaries outside the fixture body, but a materialized active body must expand them to owner-defined records and public schemas. Later-profile shorthand details belong in [Future Fixtures: Later-Profile Fixture Shorthand Notes](../later/future-fixtures.md#later-profile-fixture-shorthand-notes) and are not active requirements for Engineering Checkpoint or MVP-1.

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

Runners may use this metadata to choose, order, or report suites. Core receives only `request.tool` and the public `request.payload`; metadata must not change seed expansion, fixture comparison semantics, tool request schemas, or expected owner records.

## Conformance Execution

Future `harness conformance run` will execute fixtures through the same Core entrypoints used by MCP tools and operator commands. It must not assert behavior by inspecting prose output alone.

Future runtime fixture execution semantics:

1. Load fixture YAML files and validate the exact fixture body shape, canonical active values, public `request.payload` schema, and absence of fixture-only shorthand.
2. Create a fresh fixture-only runtime home and temporary Product Repository for the fixture, unless the fixture explicitly targets an existing read-only sample. This fixture isolation is test hygiene for deterministic comparison; it is not an `isolated` guarantee level, OS sandboxing, permission isolation, or tamper-proof storage claim. The runner must not reuse the developer's real Harness Runtime Home or Product Repository for state-changing fixture execution.
3. Seed `registry.sqlite`, `project.yaml`, `state.sqlite`, artifact files, projection files when the fixture requires them, and connector manifests from `initial_state`.
4. Execute `request.tool` through Core. MCP tool actions use the public request schema; fixture `request.payload` must be the same request payload a surface would send to that MCP tool. Operator actions such as `projection_refresh`, `doctor_surface`, `recover`, and `artifacts_check` use the operator semantics in [Operations And Conformance Reference](operations-and-conformance.md).
5. Capture returned response facts, resulting state summaries, storage effects, appended owner events, validator results when emitted, artifact registry/file integrity, structured blockers, projection job status when relevant, reconcile items when relevant, and returned error code.
6. Compare the captured results with `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, and `forbidden_side_effects`; empty expected sections mean the fixture asserts no relevant effect for that section.
7. Report fixture id, pass/fail, observed response/state/storage/event/artifact/blocker/error summary, projection freshness when relevant, and forbidden-side-effect comparison.

Runner sequence summary: the numbered sequence above is the contract summary. A future runner loads an exact fixture body, seeds a fixture-only runtime home, executes the request through Core, compares response/state/storage/events/artifacts/blockers/errors/forbidden side effects, and emits a report.

When a fixture `request.payload.envelope` includes `expected_state_version`, the runner compares it according to the Core-resolved primary Task, not only `ToolEnvelope.task_id`. Primary Task resolution order is tool-specific `task_id`, `ToolEnvelope.task_id`, then active Task resolution. Task-scoped actions compare against the seeded or Core-resolved primary Task State Version; project-scoped actions with no resolved primary Task compare against the Project State Version. Captured response, `EventRef.state_version`, and `task_events.state_version` values are compared as resulting affected-scope versions. Read-only fixtures may assert the unchanged version for the primary read scope. This clarifies comparison semantics without changing fixture body shape.

A stale `expected_state_version` fixture is a stale-authority test, not only a concurrent-write test. Exact idempotent replay is the exception: when a committed replay row exists and the canonical request hash matches, the fixture should assert the original committed response is returned and no current state-version freshness check is re-run. When no replay row exists and a state-changing action conflicts before commit, the fixture should assert that no current records changed, no `task_events` were appended, no artifacts were registered, no projection jobs were enqueued, and no `tool_invocations` replay row was created for the conflicting request unless an owner document explicitly defines a different recovery action. When the same key is reused with a changed canonical request hash, the fixture should assert `STATE_CONFLICT`, preserved original replay row, and no merged artifacts, events, projection jobs, response fields, or owner relations. For `dry_run=true`, fixtures should assert that diagnostics or `would_create` effects are returned without current records, `task_events`, artifacts, consumable Write Authorizations, projection jobs, or `tool_invocations` replay rows, and that the key is not reserved for later non-dry-run use. Replayed `prepare_write` must not create a duplicate authorization; replayed `record_run` must not consume authorization twice.

Fixture execution should be deterministic. Network access, wall-clock-sensitive expiry, and external tool output must be stubbed or represented as seeded fixture inputs unless a suite explicitly declares itself an integration smoke.

Fixture isolation is part of the pass condition. A fixture may seed files into its temporary Product Repository and runtime home, execute one Core or operator action there, and compare the captured result. This does not upgrade the product guarantee level. The fixture must not depend on existing local runtime records, generated operational files, or prose reports from a previous run.

Seed validation happens before action execution, and captured-state validation happens after action execution. Both sides of the comparison use owner-defined state loaders and value sets rather than fixture-local string labels.

Conformance runners must seed and inspect JSON `TEXT` fields through the same Core storage loaders used by MCP tools and operator commands. A fixture with malformed JSON or schema-incompatible JSON in `initial_state` must surface invalid state, or a repairable state issue when the fixture action is a recovery path and safe reconstruction is possible. The runner must not skip shape validation by treating JSON fields as opaque strings, and this expectation does not change the fixture body shape.

Conformance runners must also seed and inspect status-like `TEXT` fields through the owner-bound hardening map in [Storage](storage.md#canonical-enum-hardening). For the main Engineering Checkpoint / MVP-1 path, fixture seed loaders validate only the owner values actually present in the active stage's seeded records, and artifact/ref enum assertions use the API [stage-specific active value sets](api/schema-core.md#stage-specific-active-value-sets). Examples include registry/project surface guarantee, Run kind/status, Write Authorization status/guarantee, sensitive-action approval user-judgment status when that active judgment path is present, minimal evidence summary coverage/status when evidence support is active, residual-risk visibility/status when risk visibility is active, projection job kind/status when projection assertions are in scope, and current Task or Change Unit status when those owner records are used. Committed Approval record lifecycle status and full Evidence Manifest status are later/profile-gated. Later-profile status fields stay with promoted owner docs and the future catalog until those profiles are active. Unknown status values remain invalid unless a scenario explicitly tests recovery from invalid state; expected-state status assertions compare captured owner values, not prose labels.

## Fixture Assertion Semantics

Fixture assertion modes are runner defaults or suite catalog metadata. They are not Core input, are not passed to MCP tools, and must not add fields to the fixture body. The fixture body remains exactly `scenario_id`, `initial_state`, `request`, `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, and `forbidden_side_effects`.

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

Conformance runners order captured `task_events` by `event_seq`. `state_version`, `created_at`, and `event_id` are not tie-breakers for `expected_events` ordering.

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
| 1 | `MVP-ACTIVE-task-change-unit-setup` | `harness.intake` | Registered local project with no current Task | Task `tasks.lifecycle_phase=ready`, one Change Unit or scope boundary, current-task pointer, and no write authority. | None | No Run, artifact, evidence, final acceptance, residual-risk acceptance, close, or projection-as-authority effect. |
| 2 | `MVP-ACTIVE-shaping-update-persists` | `harness.record_run` with `kind=shaping_update`, `payload.kind=shaping_update`, and `product_write=false` represented by the active payload branch | Task `tasks.lifecycle_phase=shaping` and Change Unit | Shaping updates persist into Task/Change Unit state and a `runs.kind=shaping_update` row without product-write authority. | None | No Write Authorization, product-write Run, Evidence Manifest, projection job, final acceptance, or residual-risk acceptance. |
| 3 | `MVP-ACTIVE-prepare-write-allowed-authorization` | `harness.prepare_write` | Task `tasks.lifecycle_phase=ready`, compatible scope, current expected state | `decision=allowed`, `tasks.lifecycle_phase=ready`, one active Write Authorization, replay row, no Run. | None | No OS permission, sandbox, preventive, isolated, evidence, or close claim. |
| 4 | `MVP-ACTIVE-prepare-write-blocked-no-authorization` | `harness.prepare_write` | Task `tasks.lifecycle_phase=ready` with incompatible requested path or missing compatible scope | Structured blocked response, Task `tasks.lifecycle_phase=blocked`, and no consumable Write Authorization. | `SCOPE_REQUIRED`, `NO_ACTIVE_CHANGE_UNIT`, or `SCOPE_VIOLATION` as owned by the API/Core path. | No authorization, Run, artifact, replay row for pre-commit failure, or projection job. |
| 5 | `MVP-ACTIVE-prepare-write-idempotent-replay` | `harness.prepare_write` replay | Existing committed replay row and original active authorization | Original response and original `write_authorization_ref` are returned. | None | No duplicate authorization, event, artifact, replay update, projection job, or state-version increment. |
| 6 | `MVP-ACTIVE-idempotency-key-hash-conflict` | State-changing tool with same idempotency key and different hash | Existing committed replay row | `STATE_CONFLICT`; original replay row remains unchanged. | `STATE_CONFLICT` | No merged response, event, artifact, projection job, owner relation, or replay row update. |
| 7 | `MVP-ACTIVE-record-run-consumes-authorization` | `harness.record_run` with `kind=implementation`, `payload.kind=implementation`, and only `payload.implementation` non-null | Task `tasks.lifecycle_phase=ready`, compatible scope, active compatible Write Authorization | One Run is recorded, the authorization is consumed exactly once, and Task execution assertions use `tasks.lifecycle_phase=executing`. | None | No second consumption, final acceptance, residual-risk acceptance, later assurance/profile state, or close. |
| 8 | `MVP-ACTIVE-record-run-missing-authorization-blocked` | `harness.record_run` with `kind=implementation`, `payload.kind=implementation`, only `payload.implementation` non-null, and `write_authorization_id=null` | Task `tasks.lifecycle_phase=ready` and product-write Run request with no authorization | Product-write Run is blocked before commit and Task lifecycle uses `tasks.lifecycle_phase=blocked`. | `WRITE_AUTHORIZATION_REQUIRED` | No Run, consumption, completion evidence, artifact link, projection job, or replay row. |
| 9 | `MVP-ACTIVE-record-run-stale-authorization-blocked` | `harness.record_run` with `kind=implementation`, `payload.kind=implementation`, only `payload.implementation` non-null, and a stale existing `write_authorization_id` | Task `tasks.lifecycle_phase=ready`, changed state version or stale authorization basis, and product-write Run request | Product-write Run is blocked before commit and Task lifecycle uses `tasks.lifecycle_phase=blocked`. | `WRITE_AUTHORIZATION_INVALID` with `authorization_reason=stale` when details assert the reason | No Run, consumption, completion evidence, artifact link, projection job, event, state-version advance, or replay row. |
| 10 | `MVP-ACTIVE-record-run-observed-out-of-scope` | `harness.record_run` with `kind=implementation`, `payload.kind=implementation`, and only `payload.implementation` non-null | Active compatible Write Authorization whose stored scope excludes observed path | Out-of-scope observation is rejected or recorded only through an owner-defined violation/audit path without consuming the authorization as success; Task lifecycle uses `tasks.lifecycle_phase=blocked`. | `SCOPE_VIOLATION` | Invalid authorization is not consumed; observation is not completion evidence or close readiness. |
| 11 | `MVP-ACTIVE-record-run-capability-insufficient` | `harness.record_run` with `kind=implementation`, `payload.kind=implementation`, only `payload.implementation` non-null, and a required observation the surface cannot provide | Task `tasks.lifecycle_phase=ready`, product-write Run request, and a `capability_profile` that cannot observe or attest a required comparison fact | Required comparison is blocked or narrowed through active capability semantics; unsupported facts are not marked verified. | `CAPABILITY_INSUFFICIENT` | No Run, authorization consumption, completion evidence, artifact link, projection job, event, state-version advance, or replay row for the rejected attempt. |
| 12 | `MVP-ACTIVE-raw-secret-artifact-blocked` | `harness.record_run` with `kind=direct`, `payload.kind=direct`, only `payload.direct` non-null, `write_authorization_id=null`, `product_write=false`, and active `ArtifactInput` | Task `tasks.lifecycle_phase=executing`, Run path, and active `ArtifactInput` shape that attempts forbidden raw-secret evidence | Raw secret bytes are rejected before mutation, or only an owner-approved metadata notice with `redaction_state=blocked` or `redaction_state=secret_omitted` is committed. The expected artifact, storage-row, error, and forbidden-side-effect assertions must use the same branch. | `VALIDATION_FAILED` for forbidden input shape/source or raw secret payload before mutation; `ARTIFACT_MISSING` only for missing or integrity-failed committed artifact refs. | No raw secret storage, rendering, export, evidence sufficiency, authorization consumption, or close. |
| 13 | `MVP-ACTIVE-evidence-summary-insufficient` | `harness.status` or evidence owner read | Task `tasks.lifecycle_phase=blocked` with partial/missing evidence summary | Evidence summary remains insufficient/partial and close-relevant blocker is structured. | `EVIDENCE_INSUFFICIENT` blocker when close/write path depends on it | Status prose or Markdown evidence list does not repair missing refs. |
| 14 | `MVP-ACTIVE-evidence-summary-sufficient` | `harness.record_run` with `kind=implementation`, `payload.kind=implementation`, only `payload.implementation` non-null, and active `ArtifactInput` | Task `tasks.lifecycle_phase=executing`, compatible authorization, and a non-secret staged artifact allowed as `redaction_state=none` unless redaction or omission applies. | Registered artifact refs and evidence summary become sufficient from owner records while Task remains `tasks.lifecycle_phase=executing` until close. | None | No full Evidence Manifest, later assurance/profile state, final acceptance, or residual-risk acceptance. |
| 15 | `MVP-ACTIVE-final-acceptance-missing-close-blocker` | `harness.close_task` with `intent=complete`, `requested_close_reason=completed_self_checked` | Task with evidence sufficient but required final acceptance missing | Close remains blocked with final-acceptance blocker and Task lifecycle assertions use `tasks.lifecycle_phase=blocked`. | `ACCEPTANCE_REQUIRED` | No `tasks.lifecycle_phase=completed` or `tasks.lifecycle_phase=cancelled`, fabricated acceptance, residual-risk acceptance, later assurance/profile state, or close report authority. |
| 16 | `MVP-ACTIVE-residual-risk-visible-not-accepted-blocker` | `harness.close_task` with `intent=complete`, `requested_close_reason=completed_with_risk_accepted` | Task with visible close-relevant residual risk and no compatible `judgment_kind=residual_risk_acceptance` user judgment | Residual-risk acceptance remains required and Task lifecycle assertions use `tasks.lifecycle_phase=blocked`. | `DECISION_REQUIRED` or `DECISION_UNRESOLVED` with `required_judgment_kind=residual_risk_acceptance` | Visible risk is not accepted risk; no rich Residual Risk record, later assurance/profile state, or close state is fabricated. |
| 17 | `MVP-ACTIVE-accepted-risk-close` | `harness.close_task` with `intent=complete`, `requested_close_reason=completed_with_risk_accepted` | Task with sufficient evidence, visible risk, and compatible `judgment_kind=residual_risk_acceptance` | Task closes with `tasks.lifecycle_phase=completed`, accepted-risk close reason, and refs to the user judgment. | None | Accepted risk does not create Approval, final acceptance, later assurance/profile state, or assurance upgrade. |
| 18 | `MVP-ACTIVE-display-label-not-canonical` | `harness.request_user_judgment` | Task `tasks.lifecycle_phase=ready` and Change Unit | Response may render localized display label; storage and blocker state use canonical `judgment_kind`; pending user-owned judgment assertions use `tasks.lifecycle_phase=waiting_user`. | None | `display_label` and localized labels are not canonical state, gate keys, storage identity, or close aggregation keys. |

The queue above is intentionally small. Engineering Checkpoint does not require a full conformance suite, broad catalog family coverage, final-acceptance success semantics, later assurance checks, export/recover, reconcile, stewardship, context hygiene, browser QA capture, or future guarantee-level checks. MVP-1 adds the listed user-loop judgment, evidence, close-blocker, and accepted-risk drafts without promoting later assurance checks, full Evidence Manifest, export, or profile fixtures.

## Future Fixtures

Scenario families have moved to [Future Fixtures](../later/future-fixtures.md) so the early reference stays focused on the core conformance model. That catalog contains compact future-oriented inventory for browser QA capture, cross-surface behavior, export non-leakage, context hygiene, reconcile, stewardship, full operations, advanced projection rendering, artifact redaction and integrity, and future guarantee-level checks.

Those catalog entries are design inventory only until a promoted owner path materializes exact-shape executable fixtures. They are not required for Engineering Checkpoint, do not expand MVP-1 by themselves, and do not count as runtime conformance while this repository remains documentation-only.

## Metrics Boundary

Long-term operational metrics are derived analytics, not staged-delivery-critical state or conformance requirements. Keep metrics such as approval turnaround, verification latency, projection stale duration, same-session guard frequency, and surface fallback rate in the [roadmap](../roadmap.md) as read-only diagnostics until a future version promotes them with owner docs, fixtures or a conformance target, fallback behavior, relevant redaction/retention policy, no projection-as-canonical dependency, and implementation ownership.
