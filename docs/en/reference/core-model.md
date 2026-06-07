# Core Model Reference

This reference defines the future Harness Core authority model. It is source documentation only: this repository still has no Harness runtime or server implementation, and the current documentation is not implementation-complete unless the maintainer-owned status says so in [MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

Core is the local authority record for task scope, user-owned judgment, evidence, verification expectation, close readiness, and residual risk. It has authority over Harness records and Harness state transitions. It does not grant OS permissions, sandbox arbitrary tools, make files tamper-proof, or provide isolation unless another owner documents and proves that exact mechanism.

## 1. Owns / Does not own

This document owns:

- Core invariants and authority boundaries.
- Entity relationship semantics where they affect state, write compatibility, gate behavior, or close.
- User-owned judgment boundaries and non-substitution rules.
- Gate meaning, blocker meaning, lifecycle principles, and state-transition principles.
- `prepare_write`, Write Authorization, `record_run`, `close_task`, waivers, residual-risk visibility, and close honesty.
- Cross-owner authority links where Core, API, Storage, Projection, Security, and Later material must stay separate.

This document does not own:

- Public MCP request or response shapes. Use [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), and [API Errors](api/errors.md).
- Exact active method-name, enum, and schema value sets. Use [API Schema Core](api/schema-core.md#current-mvp-value-sets).
- Storage tables, DDL, runtime home layout, locks, migrations, or persisted JSON layout. Use [Storage](storage.md).
- Rendered projection bodies or template text. Use [Projection And Templates Reference](projection-and-templates.md).
- Connector capability profiles or surface recipes. Use [Agent Integration Reference](agent-integration.md).
- Security guarantee vocabulary beyond Core authority consequences. Use [Security Reference](security.md).
- Later candidate catalogs. Use [Later](../later/index.md) until an owner promotes material into active scope.

Exact API request fields and storage table definitions may be named here only by reference. Core state values are discussed only when needed to explain authority and transition meaning.

## 2. Kernel invariants

1. Core-owned state is canonical for Harness operations; chat, reports, generated Markdown, status cards, projections, and template output are derived or contextual.
2. Harness governs Harness records and state transitions, not OS permissions, arbitrary-tool control or sandboxing, tamper-proof storage, default pre-tool blocking, or security isolation.
3. Product writes require explicit compatible scope before `prepare_write` can return an allowed compatibility result.
4. A non-dry-run allowed `prepare_write` path is the only Core path that creates a consumable Write Authorization.
5. A Write Authorization is single-use for one compatible attempt. It is not reusable scope and not OS permission.
6. `record_run` records what happened and consumes the compatible Write Authorization; it cannot retroactively authorize work that lacked scope, user judgment, sensitive-action approval, or Write Authorization.
7. User-owned judgment cannot be replaced by agent inference, broad consent, generated prose, evidence, or projection text.
8. Product judgment, technical judgment, scope judgment, sensitive-action approval, final acceptance, residual-risk acceptance, and cancellation are distinct active judgment routes. Later/reserved QA waiver and verification-risk acceptance routes must stay distinct if promoted.
9. Evidence, verification, Manual QA, final acceptance, residual-risk visibility, residual-risk acceptance, and close readiness do not substitute for one another.
10. `close_task` must return blockers instead of a successful close while close-relevant blockers remain; known residual risk must be visible before a successful close path depends on it.
11. Active current MVP scope and later candidate material stay separate. A later candidate becomes active only when its owner promotes it with scope, fallback behavior, and proof expectations.

## 3. Entity model

These entities define authority relationships, not storage tables or API bodies.

- Task: the user-value unit whose state records current mode, scope relationship, blockers, judgment needs, evidence status, close readiness, acceptance state, residual-risk state, and latest run relationship.
- Change Unit: the active scoped work boundary for write-capable work. Product writes must be covered by a compatible active Change Unit.
- <a id="autonomy-boundary"></a>Autonomy Boundary: the latitude an agent has inside a Change Unit. It is not scope, sensitive-action approval, evidence, final acceptance, or residual-risk acceptance.
- `user_judgment`: the canonical record family for choices the user owns. It feeds decision compatibility but does not create evidence, Write Authorization, or close by itself.
- <a id="write-authorization"></a>Write Authorization: the durable single-use Core record created only by compatible non-dry-run `prepare_write`. Its lifecycle can be active, consumed, stale, expired, or revoked. `allowed` is a `prepare_write` decision, not a durable authorization status; `blocked` is not an authorization status.
- Run: an execution or observation record. Product-write Runs must consume compatible active Write Authorization. Read-only or shaping-only Runs do not make later writes compatible.
- Evidence summary: the active compact Core evidence path for close-relevant claims, Runs, blockers, user judgments, and `ArtifactRef` values. A full Evidence Manifest is not active unless an owner enables it.
- `ArtifactRef`: the durable evidence reference shape owned by API/Storage. Core treats it as evidence-eligible only when it is registered, integrity-aware, redaction-aware, and linked to an owner record.
- Blocker: a structured reason progress, write, or close cannot proceed honestly.
- Residual-risk summary: the active compact visibility path for known remaining uncertainty, unchecked conditions, limits, or trade-offs. Rich residual-risk records are later candidate material until promoted.
- Projection and templates: derived displays from Core state and refs. They do not become authority by being readable or edited.

Discovery and requirement shaping persist through Task, Change Unit, and `user_judgment` owner paths. Separate shaping briefs, design displays, journey or reconcile records, rich risk records, Eval records, Manual QA records, and full evidence manifests are not active current MVP Core state unless an owner explicitly promotes them.

<a id="finding-routing"></a>

Findings from commands, Runs, reviews, validators, diagnostics, QA, or verification affect Core only when routed through an active owner path such as blocker, evidence summary, user judgment, Change Unit update, or close blocker. A finding left in chat or report prose is not state.

## 4. User-owned judgment boundaries

User-owned judgment is the boundary where Harness must ask or preserve the user's choice instead of inferring it. The exact `UserJudgment` schema and API fields live in [API Schema Core](api/schema-core.md) and [MVP API](api/mvp-api.md); this section owns the meaning of the boundaries.

Active current MVP judgment kinds stay distinct:

- Product judgment: product behavior, UX, wording, release-facing promise, or user value.
- Technical judgment: architecture, dependency, migration, public interface, compatibility, security/privacy, or material technical direction.
- Scope judgment: scope expansion, non-goal removal, Change Unit boundary, or Autonomy Boundary change.
- Sensitive-action approval: permission for a named sensitive step inside a bounded scope.
- Final acceptance: the user's result judgment when the path requires acceptance.
- Residual-risk acceptance: acceptance of a named visible residual risk for the requested close.
- Cancellation: stopping the Task without a successful result.

Later/reserved judgment candidates stay conceptual until promoted by an owner and cataloged in [Later](../later/index.md): QA waiver for a policy-allowed Manual QA requirement, and verification-risk acceptance for the risk from missing or waived required verification. They are not active current MVP `UserJudgment.judgment_kind` values.

Ambiguous consent is narrow. "Go ahead", "looks good", or similar broad approval cannot silently satisfy another judgment kind. One user reply may satisfy multiple judgment routes only when the prompt explicitly asked those distinct questions and Core records each compatible judgment with its affected object, scope, consequence, and close or write impact.

## 5. Non-substitution rules

Core must preserve these separations:

- Chat, generated Markdown, projection prose, or report text does not substitute for Core state.
- Evidence, logs, screenshots, artifacts, or test output do not substitute for final acceptance, Manual QA, verification, or residual-risk acceptance.
- QA is not final acceptance; a later QA waiver would not be QA evidence or QA pass.
- Later verification-risk acceptance would not be verification, detached verification, or assurance upgrade.
- Sensitive-action approval does not decide product direction, technical direction, scope, correctness, evidence, QA, final acceptance, residual-risk acceptance, or Write Authorization.
- Product judgment, technical judgment, and scope judgment do not substitute for one another.
- Final acceptance does not create evidence, erase evidence gaps, satisfy QA, prove verification, grant sensitive-action approval, change scope, accept residual risk, or override blockers.
- Residual-risk acceptance does not verify work, make a no-risk close, satisfy evidence, satisfy QA, or imply final acceptance.
- A stale or failed projection does not block or allow close by itself; the current Core close state and blockers do.

These rules apply even when a user-facing surface compresses the display. Compact output can be friendly, but it must not collapse authority boundaries.

## 6. Active Gates And Reserved Gate Names

Gates are Core compatibility dimensions for progress, write, run recording, and close. The active current MVP gate fields exposed in public schemas are only the fields owned by [API Schema Core](api/schema-core.md#current-mvp-value-sets). A gate name in planning prose does not create an active schema field, storage record, validator, or close requirement.

- <a id="scope-gate"></a>Scope Gate: whether active scope covers the requested write or close-relevant work.
- <a id="decision-gate"></a>Decision Gate: whether unresolved user-owned judgment blocks progress, write, or close. It does not replace sensitive-action approval, evidence, verification, QA, final acceptance, or residual-risk acceptance.
- <a id="approval-gate"></a>Approval Gate: whether scoped sensitive-action approval is needed, pending, usable, denied, expired, or drifted. It is permission for the sensitive action only.
- <a id="design-gate"></a>Design Gate: whether an enabled design-quality finding routes into a Core-backed blocker. Broad design-quality catalogs are not default active MVP blockers.
- <a id="evidence-gate"></a>Evidence Gate: whether required close-relevant evidence is absent, partial, sufficient, stale, or blocked.
- <a id="acceptance-gate"></a>Acceptance Gate: whether final acceptance is required and, if so, recorded after the close basis is visible.
- <a id="capability-boundary"></a>Capability Boundary: surface capability affects blockers, validator findings, and guarantee display, but it is not a gate that creates authority. Missing capability must narrow the claim, hold the action through the owner path, or produce a capability blocker rather than pretending verification or prevention happened.

Reserved gate names:

- <a id="verification-gate"></a>Verification Gate is a later/reserved gate name. The active MVP has no `verification_gate` public schema field and no detached verification workflow. A future owner must promote exact fields, requiredness, fallback behavior, and proof expectations before it affects active close semantics.
- <a id="qa-gate"></a>QA Gate is a later/reserved gate name. The active MVP has no `qa_gate` public schema field and no Manual QA gate. A future owner must promote exact fields, waiver behavior, artifact handling, and proof expectations before it affects active close semantics.

Gate state exposure in public responses is owned by [API Schema Core](api/schema-core.md) and method owners. Core owns the compatibility meaning and the rule that stale gate summaries must be recomputed before write or close relies on them.

## 7. Task lifecycle

The lifecycle is a Core state-transition discipline, not a display script. Active fixture and schema owners may expose exact values, but the Core principles are:

- A Task can be shaped, made ready, executed, wait for user judgment, become blocked, complete, cancel, or be superseded only through owner paths.
- Advice/read-only work must not produce product-file writes. Write-capable direct and tracked work must pass through compatible scope and the Write Authorization path.
- A product write path moves through scope establishment, user-judgment and sensitive-action checks when applicable, `prepare_write`, one compatible product-write Run, `record_run`, evidence/blocker update, and `close_task`.
- `close_ready` is derived. It is not a lifecycle phase and does not move a Task to completed; only `close_task` can do that.
- Idempotency replay must not duplicate state transitions, events, Write Authorizations, Runs, artifacts, evidence updates, or close effects.
- Dry-run calls describe possible outcomes but create no authoritative state, no consumable Write Authorization, no artifact, no close state, and no replay row.

<a id="stable-event-catalog"></a>

Stable event names are append-only history labels for Core changes, not authority by themselves. The catalog should cover Task lifecycle updates, `prepare_write` decisions, Write Authorization creation/consumption/staling/expiry/revocation, Run recording, user judgment updates, gate recompute, evidence updates, blocker updates, residual-risk visibility or acceptance, close attempts, and close success or cancellation. Waiver event names are reserved for owner-promoted later paths. Exact event payloads and persistence are owned by API and Storage.

<a id="prepare_write"></a>

## 8. prepare_write authority

`prepare_write` is the unique pre-write compatibility decision point for product-file writes. It checks the intended operation against active Task, Change Unit, scope, baseline, Autonomy Boundary, required user-owned judgment, sensitive-action approval, surface capability, and active design-policy preconditions.

Only a compatible non-dry-run allowed path creates a consumable Write Authorization. Dry-run responses, `blocked`, `approval_required`, `decision_required`, and `state_conflict` remain response, blocker, or error states only. They must not create a consumable authorization row, replay row, evidence record, close state, or Harness write-compatibility record.

Write Authorization is a cooperative Harness record. It can tell a connected agent or surface that the intended write is compatible with current Harness state; it does not grant OS permission, enforce a sandbox, prevent arbitrary tools, make storage tamper-proof, or isolate the operation.

When MCP or the connected surface cannot perform the needed cooperative check, the honest result is a hold, blocker, degraded guarantee display, or capability error. Preventive or isolated wording is allowed only when the documented surface proves that exact boundary for the covered operation.

<a id="record_run"></a>

## 9. record_run authority

`record_run` records execution or observation. It is not a second chance to authorize a write.

For a product-write Run, Core must load a compatible active Write Authorization, compare the observed attempt against the stored authorized attempt and current state to the extent the surface can honestly observe it, and consume the authorization exactly once when compatible. Missing, stale, expired, revoked, consumed, incompatible, or insufficiently observable authorization cannot be recorded as successful consumption.

`record_run` may register or link `ArtifactRef` values only through owner-approved artifact paths. Raw secrets, tokens, forbidden sensitive logs, arbitrary caller paths, or untrusted bytes must be rejected, redacted, represented as omitted/blocked, or routed through an approved safe handle rather than stored to make evidence look complete.

Read-only and shaping-only Runs may be recorded without Write Authorization only when they do not report product-file changes. A violation or audit record may document an observed problem when an active owner path supports it, but it does not satisfy completion evidence, final acceptance, residual-risk acceptance, close readiness, QA, or verification until repaired through the relevant owner records.

<a id="close_task"></a>

## 10. close_task authority

`close_task` is the single completion decision point. Agent summaries, final reports, acceptance-looking chat, projections, Evals, QA notes, and evidence displays may inform close, but they do not close a Task by themselves.

For a successful close, Core must confirm the close intent against current Task state, open Runs, scope, user-owned judgments, sensitive-action approval when applicable, Write Authorization and Run compatibility, baseline and surface capability when relevant, required evidence sufficiency, close-relevant artifact availability, final acceptance when required, residual-risk visibility when close-relevant risk exists, residual-risk acceptance when the active close path requires acceptance, recovery constraints, and cancellation or supersession conflicts.

MVP close must keep later assurance material out of active response semantics. `verification_gate`, `qa_gate`, detached verification, `completed_verified`, detailed Manual QA close fields, full Evidence Manifest behavior, and assurance display detail are later candidate behavior unless their owners explicitly activate them.

`close_task` must return blockers instead of pretending close is complete when required task/scope correctness, user-owned judgment, sensitive-action approval, Write Authorization or Run compatibility, evidence, artifact availability, final acceptance, residual-risk visibility, residual-risk acceptance, cancellation/supersession handling, surface capability, baseline, or recovery conditions remain unresolved. A public response may choose one primary error, but secondary close blockers and refs must remain visible enough for the next safe action.

Cancellation and supersession are honest terminal paths, not successful completion. Risk-accepted close is successful close with named accepted risk; it is not verified close and not no-risk close.

## 11. Blockers

Blockers are structured reasons a transition cannot proceed honestly. They can block progress, a write, Run recording, or close. They should name the affected Task or Change Unit when available, the category, the missing or incompatible condition, related refs, and the next safe action.

Common blocker categories include missing active Task, missing active scope, out-of-scope write intent, unresolved user-owned judgment, missing sensitive-action approval, incompatible Autonomy Boundary, insufficient surface capability, missing or invalid Write Authorization, stale baseline, missing evidence, stale or unavailable artifact support, missing final acceptance, hidden residual risk, unaccepted close-relevant residual risk, unsafe open Run, recovery constraint, cancellation conflict, and supersession conflict.

Invalid state combinations must become blockers, rejections, or repair paths. They must not be papered over by projection prose, broad approval, a waiver that does not apply, or a close result that hides the conflict.

## 12. Waivers

A waiver is a scoped exception to a named requirement where policy allows it. It must preserve what requirement was skipped, the affected Task and Change Unit, the reason, actor, timing, affected gate or close impact, expiry or required next action when needed, and any close-relevant residual risk.

Allowed waiver paths are narrow:

- Design-policy waiver only when the design policy owner allows it.
- Later/reserved QA waiver only if a future owner promotes required Manual QA and policy allows waiver.
- Later/reserved verification-risk acceptance only if a future owner promotes required verification and the user accepts the named risk of missing or waived verification.

Not allowed:

- Scope waiver for product writes.
- Sensitive-action approval waiver.
- Evidence waiver where evidence is required for completion.
- Final acceptance waiver where acceptance is required.
- Residual-risk visibility waiver.

Decision deferral is not waiver. Later QA waiver is not QA pass. Later verification-risk acceptance is not verification. A waiver can unblock only the requirement it names and only through the owner path that permits it.

## 13. Residual risk

Residual risk is known remaining uncertainty, an unchecked condition, limitation, or trade-off that matters to close. Known close-relevant residual risk must be visible before successful close. If close depends on accepting that risk, Core requires a compatible residual-risk acceptance `user_judgment` tied to the visible risk and related refs.

Residual-risk acceptance does not verify the work, satisfy evidence, satisfy QA, grant sensitive-action approval, create final acceptance, or make the result no-risk. It records that the user accepts a named visible risk for the requested close.

The active current path uses compact residual-risk summary, blockers, evidence refs, and `user_judgment` refs. Rich residual-risk records, review workflows, handoff reports, and later assurance displays remain later candidate material until promoted.

## 14. Cross-owner links

Use these owners when Core authority touches another contract:

- Public API method behavior, request/response shapes, active method-name and schema value sets, envelopes, state conflicts, and errors: [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), and [API Errors](api/errors.md).
- Storage tables, DDL, runtime home layout, locks, migrations, artifact storage, and enum hardening: [Storage](storage.md).
- Projection freshness, readable views, managed blocks, human-editable sections, and active rendered template bodies: [Projection And Templates Reference](projection-and-templates.md).
- Security guarantee language, cooperative/detective/preventive/isolated labels, and local access posture: [Security Reference](security.md).
- Runtime boundary placement and Core-only mutation authority: [Runtime Boundaries Reference](runtime-boundaries.md).
- Design-quality active role and close-impact boundary: [Design Quality](design-quality.md).
- Connector capability profiles and surface-specific fallback behavior: [Agent Integration Reference](agent-integration.md).
- Conformance examples, future fixture boundaries, and operations entrypoint candidates: [Conformance Reference](conformance.md), [Later Candidate Index: Future Fixture Families](../later/index.md#future-fixture-families), and [Later Candidate Index: Operations Candidates](../later/index.md#operations-candidates).

If another document needs an exact schema, DDL table, rendered template body, or later candidate catalog, it must link to the owner instead of redefining it here.
