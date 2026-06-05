# Conformance Fixtures Reference

## What this document helps you do

Use this reference to look up the three-layer boundary for Harness conformance material: documentation checks, MVP behavior examples, and future runtime conformance. It explains what future conformance will prove, the small Engineering Checkpoint and MVP-1 behavior examples, exact future fixture body shape, future runner execution behavior, fixture assertion semantics, current-phase status, and the boundary to the future fixture catalog.

This is a lookup document for conformance authors, implementers, and maintainers. It is not an operator procedure; use [Operations And Conformance Reference](operations-and-conformance.md) for operator entrypoints and the `harness conformance run` overview.

This is reference documentation for future conformance work. The current repository is documentation-only and contains no runnable Harness Server conformance tests; current phase and handoff status are tracked in [Implementation Overview](../build/implementation-overview.md#documentation-acceptance-status).

## Read this when

- You are writing or reviewing the future fixture-based conformance design.
- You need the exact fixture body fields, fixture shorthand boundary, `ToolEnvelope` expansion convention, or runner isolation behavior.
- You need fixture assertion modes for state, events, artifacts, projections, errors, validators, close blockers, and redaction effects.
- You need the small Engineering Checkpoint behavior examples, the MVP-1 User Work Loop behavior examples, the clarification-quality examples, or the boundary between those examples and the future fixture catalog.

## Before you read

Use [Operations And Conformance Reference](operations-and-conformance.md#conformance-run) for the conformance run entrypoint, suite-selection overview, docs-maintenance profile boundary, and operator procedures. Use [MVP API](api/mvp-api.md) and [API Schema Core](api/schema-core.md) for public request/response schemas, [Storage](storage.md) for storage layout and seed-loader owner values, [Core Model Reference](core-model.md) for state transition and stable event semantics, [Projection And Templates Reference](projection-and-templates.md) for projection freshness, [Design Quality Policies](design-quality-policies.md) for policy validator behavior, and [Agent Integration Reference](agent-integration.md) for connector conformance overview.

## Main idea

Today this document is a future conformance design, not a set of runnable tests. It defines behavior-example IDs and required behavior for later implementation planning; it does not create fixture files, runner code, generated outputs, runtime state, or a runnable Harness Server conformance suite. Do not create actual fixture files from these examples during the documentation-only phase.

Keep three layers separate:

- Documentation checks are editorial checks over Markdown docs: link integrity, terminology consistency, stage boundaries, security wording, user-language checks, owner-boundary drift, and English/Korean parity. They do not execute fixture actions or create runtime results.
- MVP behavior examples are compact design examples for Engineering Checkpoint and MVP-1. They describe expected behavior but are not executable fixtures yet and are not generated runtime artifacts.
- Runtime conformance is future Harness Server implementation work. Only after server implementation and fixture materialization will exact-shape fixtures run through Core or operator entrypoints and produce runtime pass/fail results.

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
| Small MVP behavior examples | [Engineering Checkpoint Behavior Examples](#engineering-checkpoint-behavior-examples), [MVP-1 User Work Loop Behavior Examples](#mvp-1-user-work-loop-behavior-examples), and [Clarification Quality Behavior Examples](#clarification-quality-fixture-group) |
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
| Engineering Checkpoint fixtures, with Kernel Smoke as the authoring label | Engineering Checkpoint | Minimal authority loop only: project/Task setup, one registered reference `capability_profile`, active Change Unit/scope boundary, non-dry-run in-scope `prepare_write.decision=allowed`, out-of-scope write block from Harness authority state, durable single-use Write Authorization, compatible `record_run` consumption/linking, missing artifact/evidence blocker/status or narrow close blocker, and non-mutating status read. | MVP-1 User Work Loop value, natural-language intake, profile-specific user judgment quality, full Evidence Manifest, projection renderer support, multiple projection kinds, residual-risk acceptance semantics, final-acceptance semantics, Manual QA, detached verification, export/recover, release handoff, full conformance suite, future fixture catalog, broad connector ecosystem, hosted connector registry, cross-surface orchestration, higher guard guarantees, and broad operations. |
| MVP-1 User Work Loop fixtures | MVP-1 User Work Loop | Ordinary requests become tracked work without Harness vocabulary; clarification quality, judgment separation, evidence blockers, residual-risk visibility, honest authority/fallback behavior from the same reference `capability_profile`, derived-summary non-authority, and the small design-quality blocking set are visible through Core-owned state and structured responses. | Full agency assurance hardening, detached verification independence, full Manual QA matrix, stewardship policy suite, full TDD/module/interface/domain-language catalogs, full feedback-loop audits, export/recover, release handoff, broad connector ecosystem, hosted connector registry, cross-surface orchestration, and automation beyond the MVP-1 user-value path. |
| Assurance Profile fixtures | Assurance Profile | User-owned judgment, sensitive-action Approval, Write Authorization, Manual QA, verification, final acceptance, residual-risk acceptance, stewardship, design-quality, context-hygiene, TDD, and feedback-loop boundaries stay separate and fixture-proven through Core records. | Operator recovery/export completeness, release handoff, broad operations coverage, dashboard/hosted workflow UI, broad connector automation, and unproven preventive or isolated guarantee claims. |
| Operations Profile / promoted Roadmap fixtures | Operations Profile and Roadmap | Export/recover, artifact integrity, release handoff, operator readiness, reconcile, broader conformance coverage, and any promoted future higher guarantee level or automation profile. | Any stronger security, isolation, preventive guard, browser-capture, remote/shared MCP, or automation claim until owner docs define the mechanism and fixtures prove the covered behavior. |

## MVP Behavior Examples

These behavior examples are design targets for the active MVP path. They are intentionally short and testable so future conformance can stay focused on Harness differentiation: local authority state, user-owned judgment routing, evidence and risk visibility, and honest guarantee wording. They are not executable fixtures yet, not generated runtime artifacts, and not current pass/fail criteria. They do not require the broad future catalog to satisfy Engineering Checkpoint or MVP-1.

<a id="engineering-checkpoint-behavior-examples"></a>

### Engineering Checkpoint Behavior Examples

Engineering Checkpoint behavior examples describe only the first local authority loop. If a future owner materializes them as fixtures, each fixture must assert Core-owned state, events when stable owner events exist, artifact refs where relevant, and structured errors or blockers. Projection assertions default to no requirement.

| Example ID | Behavior path | Required behavior assertion |
|---|---|---|
| `ENG-CHECK-project-task-scope-setup` | owner setup path or validated seed path | One local project, one active Task, and one active Change Unit or scoped work boundary exist in Core-owned state; setup alone creates no Write Authorization and no product-write Run. |
| `ENG-CHECK-reference-surface-profile-honest` | `harness.status`, `harness://project/current`, or owner profile read | One registered reference `capability_profile` for `surface_id=reference-local-mcp` reports `mcp_available=true`, `cooperative_prepare_write_supported=true`, `changed_path_detection_supported=true`, `artifact_capture_supported=false`, `manual_artifact_attachment_supported=true`, `command_observation_supported=false`, `secret_access_observation_supported=false`, `pre_tool_blocking_supported=false`, `isolation_supported=false`, `max_guarantee_level=detective`, and `conformance_smoke_status=planned_not_run` or equivalent planned/not-run status. The profile does not create Write Authorization, replace Core gates, or claim `preventive`, `isolated`, native artifact capture, command observation, or secret-access observation. Unsupported fields lower the displayed guarantee or produce `CAPABILITY_INSUFFICIENT` / structured blocked reasons when required. |
| `ENG-CHECK-prepare-write-in-scope-allowed` | `harness.prepare_write` | A compatible non-dry-run in-scope product-write request returns `decision=allowed`, no primary error, and creates one durable active Write Authorization tied to the Task, scope/Change Unit, intended operation, and basis state version. |
| `ENG-CHECK-prepare-write-out-of-scope-blocked` | `harness.prepare_write` | An out-of-scope intended write is refused by Harness authority state with a structured blocker or `SCOPE_VIOLATION`-equivalent primary error; no Write Authorization, Run, artifact, projection job, or state-authorizing side effect is created. |
| `ENG-CHECK-write-authorization-single-use` | `harness.record_run` | The first compatible product-write Run may consume an active authorization; a second distinct Run using the same authorization is blocked with `WRITE_AUTHORIZATION_INVALID` and `authorization_reason=consumed`, or an owner-equivalent structured error, and no second consumption is recorded. |
| `ENG-CHECK-record-run-consumes-and-links-authorization` | `harness.record_run` | A compatible Run records observed work, links `consumed_by_run_id` or the owner-equivalent relation to the Write Authorization, and preserves the authorization basis instead of treating chat/tool output as authority. |
| `ENG-CHECK-record-run-invalid-authorization-not-evidence` | `harness.record_run` | A product-write Run with missing, stale, expired, revoked, consumed, or incompatible Write Authorization is rejected or recorded only as violation/audit according to the active contract; the invalid authorization is not marked consumed, attempted authorization refs appear only in findings/payloads/events, and the result does not count as completion evidence. |
| `ENG-CHECK-missing-artifact-evidence-ref-blocker` | `harness.status`, narrow `harness.close_task` smoke, or owner blocker read | Missing required artifact/evidence support is reported as structured status/blocker state such as `ARTIFACT_MISSING` or `EVIDENCE_INSUFFICIENT`; rendered prose or Markdown cannot satisfy the missing ref. |
| `ENG-CHECK-status-read-no-mutation` | `harness.status` read, including `status.next_actions` when present | Status returns current Task, scope, write-authority summary, evidence/artifact support, blockers, next actions, and state version without appending events, creating artifacts, enqueueing projections, authorizing writes, satisfying evidence, or closing work. |
| `ENG-CHECK-task-scoped-stale-state-version` | state-changing task-scoped tool | A stale `expected_state_version` is compared against the Core-resolved primary Task's `tasks.state_version`; the response is `STATE_CONFLICT`, and no current record, event, artifact, projection job, or replay row is created for the conflicting new attempt. |
| `ENG-CHECK-project-scoped-stale-state-version` | state-changing project-scoped tool with no primary Task | A stale `expected_state_version` is compared against `project_state.state_version`; the response is `STATE_CONFLICT`, and no Task row, project current record, event, artifact, projection job, or replay row is created for the conflicting new attempt. |
| `ENG-CHECK-idempotency-same-key-same-hash-replay` | committed state-changing tool replay | Reusing the same `idempotency_key` with the same canonical `request_hash` returns the original committed response without re-running freshness checks or appending events, registering artifacts, enqueueing projections, or updating the replay row. |
| `ENG-CHECK-idempotency-same-key-different-hash-conflict` | committed state-changing tool replay | Reusing the same `idempotency_key` with a different canonical `request_hash` returns `STATE_CONFLICT`, preserves the original replay row, and does not merge new artifacts, events, projection jobs, response fields, or owner relations into the old result. |
| `ENG-CHECK-dry-run-idempotency-non-consumption` | any state-changing tool with `dry_run=true` | Dry-run may return diagnostics or would-change effects, but it creates no current record, `task_events` row, artifact, consumable Write Authorization, projection job, or `tool_invocations` replay row, and it does not reserve or consume an `idempotency_key`. |
| `ENG-CHECK-prepare-write-replay-no-duplicate-authorization` | `harness.prepare_write` replay | Replaying the same committed allowed `prepare_write` returns the original response and original `write_authorization_ref`; exactly one durable authorization exists and no duplicate authorization, event, projection job, or state-version increment is recorded. |
| `ENG-CHECK-record-run-replay-no-double-consumption` | `harness.record_run` replay | Replaying the same committed `record_run` returns the original response; the linked Write Authorization remains consumed once, and no second consumption, Run, artifact registration, event, projection job, or state-version increment is recorded. |

<a id="mvp-1-user-work-loop-behavior-examples"></a>

### MVP-1 User Work Loop Behavior Examples

MVP-1 behavior examples describe user-visible Harness value without growing into the broad assurance or operations catalog. If future fixtures materialize these examples, they may use exactly `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, and `harness.close_task` where those methods are active for the stage. A separate `harness.next` fixture belongs to later/compatibility material.

| Example ID | Required behavior assertion |
|---|---|
| `MVP1-natural-language-starts-tracked-work` | Ordinary user language starts or resumes tracked work without requiring "Harness," `Task`, `Change Unit`, `Decision Packet`, or another startup phrase; the request alone does not authorize product writes. |
| `MVP1-codebase-answerable-facts-checked-before-question` | Current seeded repo/codebase refs, Harness state refs, or connector/session facts are used before asking the user to repeat facts that are already answerable; unresolved user-owned judgments still route to focused questions. |
| `MVP1-product-technical-scope-judgments-separated` | Product, technical, and scope decisions are represented as separate user-owned judgment requests or candidates, distinct from sensitive approval, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, and cancellation. |
| `MVP1-small-typo-direct-change-stays-light` | A small typo or direct change keeps a light procedural budget while still preserving scope, write authority where product writes apply, evidence/self-check support, and any relevant user-owned judgment. |
| `MVP1-ambiguous-feature-enters-clarification` | An ambiguous feature request enters clarification or user judgment routing instead of premature implementation or broad approval. |
| `MVP1-unresolved-user-judgment-blocks-write-or-close` | When a relevant product, technical, scope, sensitive approval, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, or cancellation judgment is missing, pending, rejected, blocked, stale, or incompatible, affected write or close is blocked through structured Core/API results with `required_judgment_kind` and the judgment refs that need action. |
| `MVP1-missing-evidence-blocks-close-when-required` | When evidence is required and `evidence_summary.status` is `none`, `partial`, `stale`, or `blocked`, close is blocked with `category=evidence` or `artifact_availability` as applicable; report prose, status-card text, or Markdown cannot satisfy the gap. |
| `MVP1-artifact-integrity-metadata-required-for-sufficient-evidence` | Critical artifact evidence cannot make `evidence_summary.status=sufficient` unless the supporting `ArtifactRef` has current owner relation, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, `retention_class`, and availability metadata. Missing metadata, missing bytes, unresolved owner links, raw secret/full sensitive-log storage, or `hash_mismatch` marks affected evidence `stale` or `blocked`; close is blocked when required evidence is affected. |
| `MVP1-residual-risk-visible-before-final-acceptance-risk-close` | Known close-relevant residual risk is visible before successful final acceptance or close; hidden or stale risk blocks the relevant route through `residual_risk_visibility`. |
| `MVP1-accepted-risk-close-requires-explicit-risk-acceptance` | `completed_with_risk_accepted` succeeds only when the risk is visible and a compatible `judgment_kind=residual_risk_acceptance` user judgment records acceptance with related blocker/evidence refs; final acceptance or broad "go ahead" text does not satisfy it. |
| `MVP1-ambiguous-go-ahead-does-not-resolve-route` | Ambiguous consent phrases such as "yes, do it," "go ahead," "looks good," "좋아," or "진행해" do not resolve ambiguous user-judgment routes, waive QA, accept verification risk, accept residual risk, cancel work, or authorize out-of-scope work. |
| `MVP1-mcp-core-unavailable-does-not-fabricate-authority` | If MCP/Core is unavailable, the surface reports inability to read or mutate authority state and does not fabricate Task state, Write Authorization, evidence, close readiness, approval, or acceptance. |
| `MVP1-unsupported-surface-lowers-or-blocks-claim` | If a requested write, evidence claim, or guarantee display depends on an unsupported reference `capability_profile` field, the result lowers the displayed guarantee or returns `CAPABILITY_INSUFFICIENT` / a structured blocked reason. Product writes do not proceed silently, and unsupported native capture, command observation, secret-access observation, pre-tool blocking, or isolation is not treated as available. |
| `MVP1-projection-template-output-not-state` | Projection, template, user-facing compact outputs (`status-card`, `judgment-request`, `run-evidence-summary`, `close-result`), agent-facing `agent-context-packet`, or Markdown output remains derived; reading or editing it cannot create state, satisfy gates, authorize writes, attach evidence, record final acceptance, accept residual risk, or close a Task. User-facing outputs must not be used as agent authority, and the agent packet must not be presented as user status. |
| `MVP1-detached-verification-not-claimed-unless-recorded` | Detached verification is not claimed unless the active profile requires it and a compatible recorded Eval or owner verification path exists; same-session review or prose alone does not upgrade assurance. |
| `MVP1-design-quality-blocking-set-limited` | Design-quality findings block write or close by default only for Autonomy Boundary exceeded, unresolved user judgment, missing active scope, missing required evidence, stale context affecting write/close, or surface capability insufficient for a claimed guarantee; full domain-language, module/interface, TDD, stewardship, feedback-loop, Manual QA, and detached-verification catalog findings route as candidate/advisory unless an active owner path promotes them. |
| `MVP1-policy-finding-one-next-action` | Each design-quality finding has exactly one routed action: block write, block close, ask one focused user judgment, request evidence, mark residual risk, show advisory next action, or no action; one blocker produces one next action and does not create an infinite review loop. |

<a id="clarification-quality-fixture-group"></a>

### Clarification Quality Behavior Examples

Clarification-quality behavior examples belong to the MVP-1 User Work Loop path when they show that Harness should ask for user judgment without substituting for it. Deeper policy-specific user judgment coverage remains Assurance Profile unless an MVP-1 path needs a minimal blocker.

| Example ID | Required behavior assertion |
|---|---|
| `CLARIFY-codebase-answerable-question-not-asked` | The system does not ask the user for facts already available in current seeded repo/codebase refs, Harness state refs, or connector/session facts. |
| `CLARIFY-unclear-requirements-not-one-superficial-question` | When requirements remain materially unclear, the system does not stop after one superficial question or proceed as if scope is settled. |
| `CLARIFY-no-long-questionnaire-dump` | Clarification does not dump a long questionnaire; it asks the smallest useful set for the next safe action. |
| `CLARIFY-blocking-vs-useful-questions-separated` | Blocking questions are separated from useful-but-not-blocking questions so the user can tell what prevents write or close. |
| `CLARIFY-user-owned-judgment-choices-and-consequences` | User-owned judgments present choices, consequences, and any recommended route without broad approval language substituting for the judgment. |
| `CLARIFY-product-and-technical-decisions-separated` | Product decisions and material technical architecture decisions are separated when they ask the user to own different kinds of judgment. |

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
  - CORE-active-status-no-task
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

Use this queue as future authoring guidance for the [Engineering Checkpoint Behavior Examples](#engineering-checkpoint-behavior-examples). Kernel Smoke is the narrow authoring label for the first internal authority loop, not the first user-value slice, not a full conformance suite, and not the future fixture catalog. These rows do not imply executable fixture files already exist. They are a compact authoring order; a first implementation plan may materialize only the smallest subset that proves the one authority loop named by Build.

Kernel Smoke defaults to no projection requirement. A fixture may assert projection freshness or enqueue/failure facts only when the minimal owner path already produces those facts and they help prove the target behavior. Projection-template polish, detailed report templates, multiple projection kinds, browser QA capture, export/recover, reconcile, stewardship, context hygiene, full operations, and future guarantee-level fixtures stay outside Engineering Checkpoint unless owner docs later promote a specific narrow path.

In the table, `None` means the existing fixture field stays empty or `expected_error: null`; it is not a new sentinel value.

| Queue | Fixture candidate | Intended Core or operator action | Minimum seeded records | Main expected state assertion | Expected stable event assertion | Expected artifact assertion | Expected projection assertion | Expected primary error |
|---|---|---|---|---|---|---|---|---|
| 1 | `ENG-CHECK-project-task-scope-setup` | Owner setup path or validated seed path | Registered local project, or empty fixture-only runtime home if the setup action registers it | One local project, one active Task, and one active Change Unit or scoped work boundary exist; setup alone creates no Write Authorization or product-write Run | Owner-promoted setup events only | None | No projection requirement | None |
| 1a | `ENG-CHECK-reference-surface-profile-honest` | `harness.status`, `harness://project/current`, or owner profile read | Registered local project and reference `capability_profile` for `surface_id=reference-local-mcp` | Profile fields match the active reference surface contract; `conformance_smoke_status` is planned/not run, not passed; unsupported fields lower guarantee display or block required claims; profile does not create Write Authorization or replace Core gates | None | None | No projection requirement | `CAPABILITY_INSUFFICIENT` only when the scenario requests an unsupported required capability |
| 2 | `ENG-CHECK-prepare-write-in-scope-allowed` | `harness.prepare_write` | Active Task, compatible scope, compatible baseline if required, compatible surface guarantee, no unresolved required judgment, `dry_run=false` | `decision=allowed`; one durable Write Authorization is created for the compatible Task, scope/Change Unit, intended operation, `basis_state_version`, `status=active`, and `consumed_by_run_id=null` | `prepare_write_allowed`, `write_authorization_created` only when stable events are promoted | None | No projection requirement; `TASK` stale/enqueued is allowed only if the owner path already invalidates projections | None |
| 3 | `ENG-CHECK-prepare-write-out-of-scope-blocked` | `harness.prepare_write` | Active Task with scoped boundary that excludes the requested path/tool/operation | Out-of-scope intended write is blocked by Harness authority state; no Write Authorization, Run, artifact, or state-authorizing side effect is created | No stable event for pre-commit rejection unless owner catalog promotes one | None | No projection job for pre-commit rejection | `SCOPE_VIOLATION` or owner-equivalent structured blocker/error |
| 4 | `ENG-CHECK-write-authorization-single-use` | `harness.record_run` | Active Task, compatible scope, compatible Write Authorization consumed by a prior Run | Reuse of a consumed authorization is blocked; the original consumed relation remains unchanged and no second Run is committed | No stable event for pre-commit reuse rejection unless owner catalog promotes one | None | No projection job for pre-commit rejection | `WRITE_AUTHORIZATION_INVALID` with `authorization_reason=consumed`, or owner-equivalent error |
| 5 | `ENG-CHECK-record-run-consumes-and-links-authorization` | `harness.record_run` with `kind=direct` or `kind=implementation` | Active Task, compatible scope, compatible active Write Authorization, baseline if required | Run commits once, records observed work, and links consumption to the supplied Write Authorization | `run_recorded`, `write_authorization_consumed` only when promoted | None unless the Run also registers an artifact | No projection requirement; `TASK` stale/enqueued is allowed if state changes | None |
| 6 | `ENG-CHECK-record-run-invalid-authorization-not-evidence` | `harness.record_run` with `kind=direct` or `kind=implementation` | Active Task, compatible scope, and a missing, stale, expired, revoked, consumed, or incompatible Write Authorization case | No valid consumption is recorded; rejection or violation/audit state, when supported, does not become completion evidence; attempted refs appear only in finding/payload/event context | No `write_authorization_consumed` event; violation/audit event only when promoted | None unless an audit path registers a non-completion artifact | No projection job for pre-commit rejection; violation projection only when that owner path exists | `WRITE_AUTHORIZATION_REQUIRED` or `WRITE_AUTHORIZATION_INVALID` with the matching authorization reason |
| 7 | `ENG-CHECK-missing-artifact-evidence-ref-blocker` | `harness.status`, narrow `harness.close_task` smoke, or owner blocker read | Active Task whose current path requires artifact/evidence support but has no compatible ref | Missing artifact/evidence support is visible as structured blocker/status; report prose, Markdown, or tool text does not satisfy the ref | `close_blocked` only when a close-task smoke is the owner path and stable event is promoted | None | No projection requirement | `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, or owner-equivalent blocker/error |
| 8 | `ENG-CHECK-status-read-no-mutation` | `harness.status` read, including `status.next_actions` when present | Active Task with current scope, write-authority summary, and artifact/evidence summary | Read returns current state, blockers, next actions, and state version without appending events, creating artifacts, enqueueing projections, authorizing writes, satisfying evidence, or closing work | None | None | No projection enqueue from read-only status/blocker output | None |

The queue above is intentionally small. Engineering Checkpoint does not require a full conformance suite, broad catalog family coverage, final-acceptance semantics, Manual QA, detached verification, export/recover, reconcile, stewardship, context hygiene, browser QA capture, or future guarantee-level checks.

## Future Fixtures

Scenario families have moved to [Future Fixtures](../later/future-fixtures.md) so the early reference stays focused on the core conformance model. That catalog contains compact future-oriented inventory for browser QA capture, cross-surface behavior, export non-leakage, context hygiene, reconcile, stewardship, full operations, advanced projection rendering, artifact redaction and integrity, and future guarantee-level checks.

Those catalog entries are design inventory only until a promoted owner path materializes exact-shape executable fixtures. They are not required for Engineering Checkpoint, do not expand MVP-1 by themselves, and do not count as runtime conformance while this repository remains documentation-only.

## Metrics Boundary

Long-term operational metrics are derived analytics, not staged-delivery-critical state or conformance requirements. Keep metrics such as approval turnaround, verification latency, projection stale duration, same-session guard frequency, and surface fallback rate in the [roadmap](../roadmap.md) as read-only diagnostics until a future version promotes them with owner docs, fixtures or a conformance target, fallback behavior, relevant redaction/retention policy, no projection-as-canonical dependency, and implementation ownership.
