# Conformance Reference

## 1. Current status

This repository is documentation-only and still in documentation review. No Harness Server runtime, conformance runner, executable fixture files, generated conformance reports, generated runtime artifacts, or current runtime conformance results exist here.

This document is not an executable conformance suite. It is the planning owner for conformance meaning, future fixture shape, assertion authority, and compact representative examples. Current phase and handoff status remain owned by [MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

## 2. What conformance means

Conformance means that, after a Harness Server and runner exist, future executable checks can compare a specific owner-defined action with owner-defined authority records. A future check drives one Core, API, or operator action, captures response facts and owner-state effects, and compares them with structured expectations, including forbidden side effects that must remain absent.

Documentation checks are separate. Markdown maintenance checks inspect links, terminology, owner boundaries, active/later wording, security wording, and bilingual parity. They are current documentation aids only, not runtime conformance.

Conformance does not judge generated prose, agent summaries, rendered reports, or status wording. It judges only the facts that an owning document has made authoritative.

## 3. What does not exist yet

The following are future implementation work, not current repository contents:

- Harness Server runtime or Harness Runtime Home data
- executable fixture files or a fixture directory
- a conformance runner or `harness conformance run` implementation
- generated conformance reports, generated runtime artifacts, projections, operational files, or runtime state
- current runtime results for active MVP behavior or later candidates
- current `changed_path_detection_verification=passed` result for the baseline `reference-local-mcp` surface
- current runtime proof of preventive blocking, OS permission control, arbitrary-tool sandboxing, tamper-proof storage, security isolation, or profile-gated `preventive` / `isolated` guarantee claims

Examples on this page may guide planning, but they do not create runtime state, acceptance evidence, close readiness, residual-risk acceptance, generated reports, or implementation readiness.

## 4. Fixture shape

Fixture shape is future structure, not current files. After the Harness Server and runner exist, a promoted fixture should be a compact structured record with these parts:

| Part | Purpose |
|---|---|
| `scenario_id` | Stable identifier for the behavior under review. |
| authority context | The Task, Change Unit, state version, surface, owner refs, Core state, storage rows, artifact refs, and capability facts needed before the action. |
| action | One public Core, API, or operator request using the owner request schema. |
| expected assertions | Structured response facts, owner-state effects, storage or artifact facts, blocker facts, error facts, guarantee-display facts, and required absence of forbidden side effects. |
| owner links | The API, Core, Storage, Security, Agent Integration, ArtifactRef, and policy owners that define exact values and meaning. |

Materialized fixtures must use public owner schemas. They must not invent fixture-only enum values, pseudo-fields, localized display labels as state, prose-only expectations, or later-candidate-only values.

## 5. Assertion authority

Assertion authority is the narrow set of facts a future fixture may judge. Authority comes from owner-defined facts, not from scenario prose or generated summaries.

Authoritative future assertions may use:

- response facts returned by public owner APIs
- Core-owned Task, Change Unit, user judgment, Write Authorization, Run or evidence summary, blocker, close, and residual-risk state
- Storage-owned row effects, idempotency/replay facts, project-wide `project_state.state_version` facts, and artifact-integrity facts when artifacts are in scope
- temporary `StagedArtifactHandle` response facts only where the `harness.stage_artifact` owner is in scope, with persistent artifact authority asserted only after compatible `record_run` promotion
- stable `task_events` only after the Core owner promotes event names
- primary `ErrorCode`, structured blocker fields, and guarantee-display facts that match the API, Core, Security, and Agent Integration owners
- absence assertions for forbidden side effects, such as no durable authorization, no Run row, no artifact mutation, or no close-state change

Current active examples may assert `cooperative` and supported `detective` facts. For the baseline `reference-local-mcp` surface, `detective` assertions are valid only when `changed_path_detection_verification=passed` and only for the verified changed-path detection scope. `not_run`, legacy `planned_not_run` wording, `failed`, and `stale` are not passing assertion states and cannot justify a `detective` label. `preventive` or `isolated` assertions are valid only for a promoted profile with an owner-defined proof path for that profile; conformance planning text does not make those display values currently executable or proven.

Non-authoritative material includes prose scenario descriptions, comments, author notes, rendered Markdown, generated reports, status prose, agent summaries, documentation-check labels, and projections. Projection freshness or availability may be asserted only when projection support is explicitly in scope.

## 6. Representative active examples

These are compact behavior references only. They are not fixture files, full YAML bodies, current runtime results, a complete conformance suite, or an implementation plan. The first internal documentation smoke target in [MVP Plan](../build/mvp-plan.md#first-internal-smoke-target) may draw from these rows, but future executable coverage still needs owner-promoted fixtures and a runner.

| Example | Behavior | Future assertion focus |
|---|---|---|
| `MVP-ACTIVE-surface-verification-success-and-failure` | Registered local surface verification succeeds only for the current project/surface/session binding and fails honestly when the surface is unavailable, mismatched, revoked, or lacks the needed active capability. | Success facts use the owner-derived `VerifiedSurfaceContext` and one reference `capability_profile`; `detective` assertions require `changed_path_detection_verification=passed` in the verified changed-path scope. Failure facts use `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, or `CAPABILITY_INSUFFICIENT`; no copied `surface_id`, prose claim, artifact read, write compatibility, or close state becomes authority. |
| `MVP-ACTIVE-shaping-readiness-gap-blocks-or-asks` | A `ShapingReadiness` read can expose an incomplete goal, scope, Change Unit, Autonomy Boundary, named user-owned blocker, or next safe action without creating persistent planning artifacts. | Missing readiness returns an active blocker, `StateSummary.shaping_readiness` gap, or pending `UserJudgment` candidate according to the owner path; no Discovery Brief, Question Queue, Assumption Register, evidence, final acceptance, residual-risk acceptance, close state, or later planning artifact is created by the read. |
| `MVP-ACTIVE-project-state-version-single-clock` | Every committed non-dry-run mutation uses the single project-wide `project_state.state_version`; `tasks.state_version` is not an active conflict or concurrency basis. | Fresh mutations compare `ToolEnvelope.expected_state_version` with `project_state.state_version`; stale attempts return `STATE_CONFLICT` without current records, events, artifacts, evidence, Write Authorization, close state, replay rows, or state-version increments; committed mutations increment the project-wide value by exactly 1, including `close_task intent=supersede` when it updates both Task lifecycle and `project_state.active_task_id`. |
| `MVP-ACTIVE-sensitive-action-scope-recorded-separately` | Sensitive-action approval is recorded as a `judgment_kind=sensitive_approval` user judgment with `SensitiveActionScope`, separate from path-level `AuthorizedAttemptScope`. | The recorded scope names the sensitive action and honest capability claim; it does not create Write Authorization, evidence, final acceptance, residual-risk acceptance, OS enforcement, sandboxing, blocking, isolation, or artifact authority. Product-file Write Authorization still requires the owner `prepare_write` path. |
| `MVP-ACTIVE-prepare-write-blocked-approval-or-dry-run-no-durable-authorization` | A `blocked`, `approval_required`, `decision_required`, or dry-run `prepare_write` does not create durable authorization. | The response has no consumable Write Authorization; `write_authorizations` has no inserted active row; any persisted blockers or judgment candidates stay within the method-state-effect matrix; no Run, artifact, evidence, close, final-acceptance, or residual-risk state changes. |
| `MVP-ACTIVE-prepare-write-committed-scoped-authorization` | A committed allowed `prepare_write` records a scoped single-use Write Authorization. | Response authorization scope, Core state, and `write_authorizations.attempt_scope_json` agree on Task, Change Unit, project-wide `basis_state_version`, surface, intended product-file paths, sensitive categories, baseline refs, related judgments, and guarantee display level. Any `detective` display requires `changed_path_detection_verification=passed`; otherwise the display remains `cooperative` or the method returns `CAPABILITY_INSUFFICIENT` when the stronger capability is required. |
| `MVP-ACTIVE-record-run-consumes-write-authorization-once` | A compatible product-write `harness.record_run` consumes the matching active Write Authorization exactly once. | The Run links to one compatible authorization and sets the authorization consumed by that Run; idempotent replay returns the original response without consuming again. Missing, stale, expired, revoked, consumed, incompatible, or observed-outside-authorized-scope authorization attempts create no successful Run, evidence, artifact promotion, close state, or state-version increment. |
| `MVP-ACTIVE-stage-artifact-promotes-only-through-record-run` | `harness.stage_artifact` creates a temporary same-project same-Task `StagedArtifactHandle`; only compatible `harness.record_run` can consume it and promote it to a persistent `ArtifactRef`. | Staging alone creates no Core state, event, replay row, evidence summary, persistent artifact, gate result, or close effect. `record_run` rejects expired, mismatched, already-consumed, or cross-task handles without creating a Run, artifact, evidence update, or state-version increment. A valid consumed handle produces registered artifact metadata and owner links that match the staged `sha256`, `size_bytes`, `content_type`, `redaction_state`, project, and Task. |
| `MVP-ACTIVE-close-task-complete-blocker-matrix` | `close_task intent=complete` applies the deterministic blocker order from Task validity through recovery constraints before committing completion. | Response blockers follow the Core matrix order. The smoke target must cover at least evidence insufficient, artifact unavailable or missing, required final acceptance, and visible but unaccepted residual risk. Final acceptance and residual-risk acceptance never replace required evidence or required artifacts; Task remains open when blockers remain and closes only when no owner-defined blocker remains. |
| `MVP-ACTIVE-close-task-check-read-only` | `close_task intent=check` computes readiness and blockers without changing state. | No `tasks`, `blockers`, `task_events`, `tool_invocations`, close state, evidence summary, artifact, Write Authorization, or `project_state.state_version` mutation occurs. |
| `MVP-ACTIVE-close-task-cancel-supersede-not-completion` | `close_task intent=cancel` and `intent=supersede` are terminal non-completion paths. | They require valid Task identity, lifecycle, local access, recovery compatibility, and valid `superseding_task_id` when applicable; they do not require evidence sufficiency, final acceptance, or residual-risk acceptance; stored `close_reason` is `cancelled` or `superseded`; valid supersession updates lifecycle/result and `project_state.active_task_id` under one project-wide `state_version` mutation, while invalid supersession returns the applicable blocker. |

## 7. Catalog-only future boundary

Future fixture families belong in [Later Candidate Index: Future Fixture Families](../later/index.md#future-fixture-families). That index keeps names only as later candidates, and this page does not reproduce the catalog.

Future-family names are not scenario scripts, fixture bodies, active API payload examples, runner or reporting requirements, active MVP scope, implementation tasks, current results, or current runtime proof. A future owner must promote a narrow behavior with scope, fallback behavior, exact contracts, and proof-path expectations for future promotion before executable fixture material exists.

## 8. Metrics boundary

Metrics are not conformance authority in the current documentation set. Future local metrics may be useful for diagnostics or planning, but until an owner promotes them they remain read-only derived displays.

Metrics must not create Core state, satisfy evidence, pass QA or verification, authorize writes, accept final results, accept residual risk, close work, prove implementation readiness, or replace runtime conformance. If a future metric is promoted, its owner must define source records, freshness boundary, display wording, and the non-substitution rule.
