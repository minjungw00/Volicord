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

Current active examples may assert `cooperative` and supported `detective` facts. `preventive` or `isolated` assertions are valid only for a promoted profile with an owner-defined proof path for that profile; conformance planning text does not make those display values currently executable or proven.

Non-authoritative material includes prose scenario descriptions, comments, author notes, rendered Markdown, generated reports, status prose, agent summaries, documentation-check labels, and projections. Projection freshness or availability may be asserted only when projection support is explicitly in scope.

## 6. Representative active examples

These are compact behavior references only. They are not fixture files, full YAML bodies, or current runtime results.

| Example | Behavior | Future assertion focus |
|---|---|---|
| `MVP-ACTIVE-project-state-version-single-clock` | Every committed non-dry-run mutation uses the single project-wide `project_state.state_version`; `tasks.state_version` is not an active conflict or concurrency basis. | Fresh mutations compare `ToolEnvelope.expected_state_version` with `project_state.state_version`; committed mutations increment that value by exactly 1; `ToolResponseBase.state_version`, `tool_invocations.basis_state_version`, and `task_events.state_version` use the project-wide model, including `close_task intent=supersede` when it updates both Task lifecycle and `project_state.active_task_id`. |
| `MVP-ACTIVE-prepare-write-blocked-or-dry-run-no-durable-authorization` | A blocked or dry-run `prepare_write` does not create durable authorization. | The response has no consumable Write Authorization; `write_authorizations` has no inserted active row; no Run, artifact, evidence, close, final-acceptance, or residual-risk state changes. |
| `MVP-ACTIVE-prepare-write-committed-scoped-authorization` | A committed allowed `prepare_write` records a scoped single-use Write Authorization. | Response authorization scope, Core state, and `write_authorizations.attempt_scope_json` agree on Task, Change Unit, project-wide `basis_state_version`, surface, intended product-file paths, sensitive categories, baseline refs, related judgments, and guarantee display level. |
| `MVP-ACTIVE-stage-artifact-promotes-only-through-record-run` | `harness.stage_artifact` creates a temporary same-project same-Task `StagedArtifactHandle`; only compatible `harness.record_run` can consume it and promote it to a persistent `ArtifactRef`. | Staging alone creates no Core state, event, replay row, evidence summary, persistent artifact, gate result, or close effect. `record_run` rejects expired, mismatched, already-consumed, or cross-task handles without creating a Run, artifact, evidence update, or state-version increment. A valid consumed handle produces registered artifact metadata and owner links that match the staged `sha256`, `size_bytes`, `content_type`, `redaction_state`, project, and Task. |
| `MVP-ACTIVE-close-task-blocks-missing-acceptance-or-risk-condition` | `close_task` blocks when required final acceptance is missing. It also blocks when close-relevant residual risk is not visible at the required level, or when that risk is not accepted when the active close path requires acceptance. | Response blockers use owner categories and `required_judgment_kind` where applicable; Task is not completed; close state does not substitute for missing acceptance or risk acceptance; evidence, final acceptance, and residual-risk state remain separate. |

## 7. Catalog-only future boundary

Future fixture families belong in [Later Candidate Index: Future Fixture Families](../later/index.md#future-fixture-families). That index keeps names only as later candidates, and this page does not reproduce the catalog.

Future-family names are not scenario scripts, fixture bodies, active API payload examples, runner or reporting requirements, active MVP scope, implementation tasks, current results, or current runtime proof. A future owner must promote a narrow behavior with scope, fallback behavior, exact contracts, and proof-path expectations for future promotion before executable fixture material exists.

## 8. Metrics boundary

Metrics are not conformance authority in the current documentation set. Future local metrics may be useful for diagnostics or planning, but until an owner promotes them they remain read-only derived displays.

Metrics must not create Core state, satisfy evidence, pass QA or verification, authorize writes, accept final results, accept residual risk, close work, prove implementation readiness, or replace runtime conformance. If a future metric is promoted, its owner must define source records, freshness boundary, display wording, and the non-substitution rule.
