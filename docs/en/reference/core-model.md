# Core model reference

This reference owns the Core authority model for Volicord state. It defines how Core, a `Task`, a Change Unit, user-owned judgment, evidence, artifact references, `Write Authorization`, close readiness, blockers, acceptance, and residual risk relate to each other.

Core is the local authority record for Volicord state. It is not chat memory, generated Markdown, a status report, a tutorial, a storage layout, or an API response shape.

## 1. Owner boundary

This document owns:

- authority relationships among Core concepts
- non-substitution rules for judgment, evidence, acceptance, risk, write authorization, and close
- the product meaning of `Task`, Change Unit, user-owned judgment, evidence, close readiness, blockers, `Write Authorization`, final acceptance, and residual-risk acceptance
- conceptual lifecycle and authority-check boundaries

This document does not own:

- API request fields, response branches, schema shapes, exact value sets, or method behavior
- storage records, DDL, state-version effects, artifact bytes, locks, migrations, or persistence layout
- rendered projection bodies, template text, display labels, or user workflow instructions
- security guarantee wording, access-boundary claims, or out-of-scope capability catalogs

When this page names an exact identifier, it names the authority concept only. The linked owner documents define wire shape, method behavior, storage effect, display text, security wording, and exact values.

## 2. Authority invariants

Core-owned state is authority.

- Core state is the local record Volicord authority checks use to decide current scope, required judgment, evidence support, write compatibility, blockers, close readiness, and residual risk.
- Chat, reports, generated Markdown, projections, template output, and summaries can describe Core state, but they do not replace it.

Volicord governs Volicord records.

- Core authority applies to Volicord records and Volicord state transitions.
- It is not a general security-control surface and does not claim OS-level enforcement.

Scope bounds work.

- A `Task` defines the user-value unit. A Change Unit defines the current write-capable work boundary inside that `Task`.
- Product-file writes, evidence claims, final acceptance, and close claims must stay compatible with the current scope and current Change Unit.
- A resolved scope judgment does not silently mutate current scope; current scope must be updated through the scope owner-defined transition.

User-owned judgment stays user-owned.

- Core must ask for or preserve a user-owned judgment instead of inferring it from agent confidence, broad approval, evidence, display text, or a generated summary.
- One user answer can satisfy multiple authority needs only when those distinct questions were made visible and the recorded judgment remains compatible with each affected object, scope, consequence, and close or write impact.
- A recorded response is not automatically approval. Judgment lifecycle status and resolution outcome are separate: `status=resolved` means an answer was recorded, while only `resolution_outcome=accepted` can satisfy an authority-bearing requirement.

`Write Authorization` is narrow.

- `Write Authorization` authorizes one compatible product-file write attempt under current Volicord state.
- It is not reusable scope, ordinary write approval, command approval, shell permission, sensitive-action approval, user-owned judgment, OS permission, deployment approval, final acceptance, residual-risk acceptance, evidence, or proof that the write occurred.

Runs and evidence record support, not authority substitutes.

- A Run records execution or observation. Evidence records support only the claims, scope, and context they actually record.
- A Run, log, screenshot, artifact, or `ArtifactRef` does not retroactively create missing scope, missing judgment, missing approval, or missing `Write Authorization`.

Close must stay honest.

- Close readiness asks whether the current `Task` can close without hiding unresolved owner-defined requirements.
- If close-relevant blockers remain, Core must expose blockers instead of treating the `Task` as successfully complete.

Acceptance and risk acceptance are specific.

- Final acceptance is the user's judgment of the visible close basis.
- Residual-risk acceptance is the user's acceptance of named visible residual risk for the requested close.
- Neither fills evidence gaps, changes scope, grants write authority, proves verification, or makes the result risk-free.

Scope and close-basis revisions are internal current-state coordinates.

- Every `Task` has a `scope_revision` and a `close_basis_revision`.
- Material current-scope or current Change Unit changes increment `scope_revision`; semantically identical normalized updates do not.
- A committed Run recording increments `close_basis_revision`. A material scope change also invalidates the current close basis and increments `close_basis_revision`.
- Recording a user-owned judgment does not increment either revision.
- Callers do not choose these revisions, and a revision value is not authority by itself.

## 3. Core Concepts

### Core

Core is the local authority record for Volicord state within a project. It records current authority state and applies changes through owner-defined transitions.

Core authority is about Volicord records. Security guarantee levels, local-access posture, and stronger isolation non-claims belong to [Security](security.md).

### `Task`

A `Task` is the user-value unit being shaped, executed, blocked, or closed.

A `Task` owns the main work path for scope, Change Units, required judgments, verification criteria, evidence support, close readiness, final outcome, and residual risk. Exact lifecycle values and state fields belong to the API state and value-set owners.

### Change Unit

A Change Unit is the currently applied work boundary for write-capable work inside a `Task`.

It defines what the current work may change and what must stay outside the current work. It is not final acceptance, evidence, broad approval, or permission to widen scope silently.

### Autonomy Boundary

An Autonomy Boundary is the agent latitude inside the current Change Unit.

It does not allow scope expansion, sensitive-action approval, user-owned judgment, or write authorization by inference.

### User-owned judgment

User-owned judgment is the boundary where the user owns the decision. Core may record the judgment, but it must not invent it.

User-owned judgment can concern product direction, technical direction, scope, a sensitive step, final acceptance, residual-risk acceptance, or cancellation. Exact judgment schema fields and value names belong to API schema and value-set owners.

### Run

A Run records an execution or observation with the available context and references.

It can support evidence and close-readiness review. It cannot approve missing preconditions after the fact.

### Evidence

Evidence is recorded support for a specific claim at a specific scope.

Evidence can show that a named test ran, a named output was observed, or a recorded artifact supports a recorded claim. It is not broad correctness, final acceptance, residual-risk acceptance, or separate QA or verification unless the relevant owners define that path.

### `ArtifactRef`

`ArtifactRef` is a public pointer to a registered persistent artifact.

Core may treat an artifact reference as evidence-eligible only when the artifact owners allow that use. The reference itself does not prove readable bytes, content sufficiency, safety, or integrity beyond the facts recorded by artifact owners.

### `Write Authorization`

`Write Authorization` is the named Core authorization for one compatible product-file write attempt.

It depends on current Core state, current scope, current Change Unit compatibility, required user-owned judgments, and `Write Authorization` compatibility rules.

Its exact method behavior, API shape, storage effect, and stale-state handling belong to their owners.

### Blocker

A blocker is a structured reason that progress, write preparation, Run recording, or close cannot proceed honestly.

A close blocker is the close-relevant form: it prevents honest close readiness until the responsible owner-defined authority condition is resolved. A blocker is not projection prose, broad approval, storage proof by itself, or a successful-looking close.

### Close readiness

Close readiness is the Core authority concept for whether the current `Task` can close honestly.

It considers the current `Task`, current scope, current Change Unit, required judgments, write and Run compatibility, evidence support, artifact availability, unresolved blockers, final acceptance, residual-risk visibility, residual-risk acceptance, and recovery constraints.

### Current close basis

`CurrentCloseBasis` is the current result and risk state used for close-readiness decisions. It contains the current `Task`, current Change Unit, `scope_revision`, `close_basis_revision`, baseline, result summary, result references, evidence-summary reference, residual risks, sensitive categories, sensitive action requirements, recovery constraints, source Run reference, and update time.

`CurrentCloseBasis` is pre-close authority input. A successful terminal close may produce a terminal close summary, but that terminal summary is not the current pre-close basis and must not be used to recreate one for an open `Task`.

### Final acceptance

Final acceptance is a user-owned judgment that the visible close basis is acceptable for the requested close.

It does not create evidence, approve sensitive action, change scope, accept residual risk, waive blockers, or prove verification.

### Residual risk

Residual risk is known remaining uncertainty, an unchecked condition, limitation, or trade-off that matters to close.

Residual-risk acceptance applies only to the named visible risk for the requested close. Each current residual risk has an opaque Core-generated `risk_id`; display text is not authoritative identity. Residual-risk acceptance does not cover all unknowns, replace evidence, replace final acceptance, or make the result risk-free.

### Derived display

Projection output, template output, status cards, summaries, and reports are derived display. They can help a reader see Core state, but they do not become Core authority, evidence, acceptance, or risk acceptance.

<a id="4-user-owned-judgment"></a>
## 4. User-owned judgment

Core preserves the boundary between what the agent may decide and what the user must decide.

A judgment is user-owned when it changes or accepts a user-visible product outcome, a material technical direction, current scope, a named sensitive step, final acceptance, residual risk, or cancellation.

Product decisions include user-visible behavior, user flow, copy, UX, accessibility, release promises, product trade-offs, and user value.

Technical decisions include architecture, dependency or external service introduction, authentication direction, migration, public interface changes, compatibility breaks, data retention, privacy, security, and other costly-to-reverse technical directions.

Scope decisions include scope expansion, non-goal removal, Change Unit boundary changes, and Autonomy Boundary changes.

Sensitive-action approval is permission for a named sensitive step inside a bounded `SensitiveActionScope`. It is not `Write Authorization`, security authority, product correctness, or final acceptance.

Final acceptance is the user's result judgment for the visible close basis.

Residual-risk acceptance is the user's acceptance of a named visible residual risk for the requested close.

Cancellation is a user-owned decision to stop the `Task` without a successful completed result.

Authority-bearing judgment kinds are scope decision, sensitive approval, final acceptance, residual-risk acceptance, and cancellation. These judgments require a selected Core-created authority option, a stored `machine_action` that maps to `resolution_outcome=accepted`, a compatible current basis, `resolved_by_actor_kind=user`, and verified actor provenance for a bound surface whose role is `user_interaction`. Rejected or deferred outcomes remain durable user decisions but do not approve, accept, authorize, waive, or close anything. A resolved judgment missing machine-readable action or outcome, resolution payload, timestamp, compatible basis, or required actor provenance is invalid owner state and cannot satisfy current authority requirements.

For authority-bearing prompts, callers do not define visible-label-to-machine-outcome mappings. Core creates the canonical authority options: `machine_action=accept` maps to `resolution_outcome=accepted`, `machine_action=reject` maps to `resolution_outcome=rejected`, and `machine_action=defer` maps to `resolution_outcome=deferred` only where the method or semantic owner permits deferral. `blocked` is not a judgment resolution outcome. Core also creates localized labels and consequences; labels, explanatory text, free-form notes, or answer-payload prose are display-only and must not invert the selected option's machine-readable action or outcome.

Core creates a basis snapshot for each stored judgment from current state. The basis ties the judgment to the current `Task`, Change Unit when applicable, `scope_revision`, close-basis revision when applicable, baseline, result references, named residual-risk IDs, sensitive-action scope when applicable, and creation state version. Callers do not submit scope revisions or close-basis revisions.

Judgment compatibility:

- Final acceptance must match the current `Task`, current Change Unit, `scope_revision`, `close_basis_revision`, baseline, and result references.
- Residual-risk acceptance must match the current `close_basis_revision` and exact current `risk_id` values.
- Sensitive-action approval must match the current `scope_revision`, current Change Unit, operation, normalized paths, sensitive categories, baseline, and Change Unit-linked sensitive action requirement.
- Scope decision authority for a scope update must have `judgment_kind=scope_decision`, `status=resolved`, `machine_action=accept`, `resolution_outcome=accepted`, a current basis, `required_for` that includes scope update, verified actor provenance for `user_interaction`, and compatible `Task`, Change Unit, `scope_revision`, and affected refs. Rejected, deferred, stale, superseded, expired, judgments with invalid basis state, or agent-recorded scope decisions do not authorize a scope transition.
- Cancellation authority must have `machine_action=accept`, `resolution_outcome=accepted`, and match the current `Task`, current scope revision, current Change Unit, and verified `user_interaction` actor provenance. Rejected, deferred, stale, superseded, judgments with invalid basis state, or agent-recorded cancellation judgments do not permit cancellation.
- A scope decision records the user's decision but does not mutate current scope by itself.
- A stale pending judgment cannot be answered successfully.
- Scope changes and Run changes do not delete historical judgments; they make incompatible judgments ineligible for current close, write, or sensitive-approval requirements.

Judgments without a stored basis are invalid owner state. Pending judgments may become `superseded`; resolved judgments may remain stored while becoming `stale`.

Pending-judgment relevance:

- A pending judgment blocks an operation only when it is current and pending, its `required_for` operation target includes that operation, its judgment kind is relevant to that operation, and its `Task`, Change Unit, affected refs, and basis are compatible.
- Sensitive approval questions block only when they overlap the current sensitive action requirement.
- Informational judgments do not block write, Run recording, or close by themselves.

Agent latitude:

- Inside accepted scope and acceptance criteria, the agent may choose ordinary implementation details that do not change product behavior, material technical direction, scope, security or privacy posture, compatibility, or costly-to-reverse architecture.
- The agent may treat "go ahead", "looks good", or similar broad language as another judgment kind only when the prompt made that distinct judgment visible and Core records it compatibly.
- The agent must not treat broad language alone as another judgment kind.

## 5. Non-substitution rules

Generated text does not substitute for Core state.

- Chat, reports, generated Markdown, projection prose, status cards, and template bodies are not authority records.

Evidence does not substitute for user judgment.

- Evidence, logs, screenshots, artifacts, `ArtifactRef` values, and Run records do not replace final acceptance, residual-risk acceptance, sensitive-action approval, scope decisions, or other user-owned judgments.

User judgment does not substitute for evidence.

- Final acceptance, residual-risk acceptance, sensitive-action approval, and broad approval do not create missing evidence, prove correctness, satisfy separate verification, or make a close blocker disappear.

Recorded judgment status does not substitute for accepted outcome.

- `status=resolved` records that an answer exists. It does not by itself create final acceptance, residual-risk acceptance, sensitive approval, cancellation authority, or any other approval.

Sensitive-action approval does not substitute for `Write Authorization`.

- Sensitive-action approval authorizes the named sensitive step the user was asked about. It does not authorize product-file writes, commands, hosts, network, secrets, deployments, destructive operations, or final acceptance.

`Write Authorization` does not substitute for acceptance.

- `Write Authorization` makes one product-file write attempt compatible with Volicord state. It does not prove the write occurred, record evidence, accept the result, accept risk, close the `Task`, or grant system access.

Blocker data does not substitute across authority questions.

- A prepare-write decision reason and a close blocker answer different authority questions.
- `CloseReadinessBlocker` is an API data representation for close blocking reasons. It is not the whole close-readiness concept and does not prove persistence by itself.

A waiver or accepted risk does not create automatic success.

- A waiver can matter only for the named requirement and only where the responsible owner allows it.
- Accepted risk does not replace evidence, final acceptance, verification, or remaining requirements for close.

<a id="6-task-lifecycle"></a>
## 6. Task lifecycle

The lifecycle here is conceptual authority meaning, not an API state table.

| Area | Authority meaning |
|---|---|
| Intake and shaping | User intent becomes a concrete goal, scope boundary, non-goals, acceptance criteria, Autonomy Boundary, and first safe Change Unit when the relevant owners define support. |
| Scope update | Accepted scope or Change Unit changes become currently applied only through the scope owner-defined transition. A judgment record alone does not mutate current scope. |
| Execution and observation | Runs record actions and observations. Product-file writes must be compatible with current scope and `Write Authorization`; read-only work does not authorize subsequent writes. |
| Waiting or blocked | If required owner-defined authority data is missing, stale, incompatible, or unsafe to bypass, Core exposes the blocker and the next required step instead of hiding the gap. |
| Close attempt | Core evaluates whether the current state can close honestly. A final chat summary or generated report is not enough by itself. |
| Terminal outcome | Completion, cancellation, or supersession ends the `Task` path. Cancellation and supersession are terminal, but they are not successful completion and do not satisfy completion evidence, acceptance, or risk requirements. |

## 7. Authority checks

Authority checks summarize whether a Core action or close claim can proceed honestly. Public fields, exact values, response branches, and method behavior belong to API owners.

| Check area | Authority meaning |
|---|---|
| Scope | The requested work, write, evidence claim, or close claim must fit the current `Task` scope and current Change Unit. |
| User-owned judgment | Required product, technical, scope, sensitive-action, final-acceptance, residual-risk, or cancellation judgment must be resolved by the user with the required stored outcome and compatible with the affected object and consequence. |
| Sensitive action | A named sensitive step must have its own compatible user approval when that approval is required. |
| Write compatibility | A product-file write attempt must be compatible with current scope and a consumable `Write Authorization`. |
| Run and evidence | Recorded Runs, evidence summaries, and evidence-eligible artifacts must support the claims they are used for. |
| Final acceptance | Required final acceptance must be tied to the visible close basis. |
| Residual risk | Known close-relevant residual risk must be visible, and required risk acceptance must be compatible with the requested close. |
| Close readiness | All close-relevant owner-defined requirements must support an honest terminal transition; remaining blockers keep the `Task` open. |

Separate QA and external verification workflows are not separate baseline authority records unless [Scope](scope.md) and the affected owners define them as supported.

## 8. `Write Authorization`

`Write Authorization` is Core authority for one compatible product-file write attempt.

It has these authority properties:

- Scope-limited: it covers the intended product-file write attempt, not subsequent attempts or a broader project area.
- State-bound: it is based on current Volicord state and can become stale when relevant state changes.
- Single-use: one compatible product-write Run consumes it once.
- Cooperative: it tells a connected agent or surface what is compatible with Volicord state; it does not claim OS-level prevention or sandboxing.

It is not:

- command approval
- ordinary write approval
- dependency approval
- shell permission
- OS permission
- host, network, or secret access
- deployment approval
- destructive-action approval
- system access
- sensitive-action approval
- user-owned judgment
- final acceptance
- evidence
- residual-risk acceptance
- proof that a write happened
- `Task` close

The prepare-write, record-run, API state schema, storage, and security owners define the method behavior, public shapes, storage effects, replay and stale-state behavior, and guarantee wording.

<a id="9-evidence-and-run-authority"></a>
## 9. Evidence and Run authority

Evidence authority is scoped to recorded claims.

Run authority:

- A Run can establish that an execution or observation was recorded with the available context and references.
- A Run cannot establish that missing authorization, missing judgment, missing approval, or missing `Write Authorization` existed retroactively.

Evidence authority:

- Evidence can establish that recorded support exists for a named claim, gap, reference, or coverage item.
- Evidence cannot establish unrecorded behavior, broad correctness, final acceptance, residual-risk acceptance, or a no-risk result.

`ArtifactRef` authority:

- An `ArtifactRef` can identify a registered artifact available for evidence use when artifact owners allow that use.
- An `ArtifactRef` cannot by itself establish that artifact content is safe, sufficient, readable, or unredacted beyond recorded artifact-owner facts.

Display authority:

- A projection, template, report, or status card can establish that a display was derived from available state and references.
- The display itself is not Core authority, evidence, acceptance, or residual-risk acceptance.

<a id="close_task"></a>
## 10. Close readiness

Close readiness is the Core authority concept for whether the current `Task` can close honestly.

Close readiness considers:

- `Task` lifecycle eligibility for the requested terminal path
- current scope, current Change Unit, acceptance criteria, and completion policy
- required user-owned judgments
- required sensitive-action approval
- write and Run compatibility
- evidence sufficiency for the close basis
- close-relevant artifact availability
- unresolved blockers
- required final acceptance
- residual-risk visibility and required residual-risk acceptance
- recovery, repair, corruption, reconciliation, or other constraints that would make close dishonest

Close readiness uses `CurrentCloseBasis` as the current close input. It does not use a terminal close summary as the current pre-close basis.

Close-basis authority:

- Caller-supplied close-basis result and risk refs must be accepted only from owner-allowed result/evidence kinds and must exist, belong to the same project and `Task`, and be canonicalized by Core.
- Baseline allowed caller-supplied result/evidence kinds are Run, Artifact, EvidenceSummary, and ChangeUnit unless an owner explicitly adds another kind.
- ProjectState, `Write Authorization`, UserJudgment, Blocker, TaskEvent, LocalSurfaceRegistration, and Task are not caller-supplied result refs unless an owner explicitly adds them.
- Artifact refs used for close evidence must be linked to the `Task` and have current-byte verified integrity at use time. Evidence refs must identify the current `Task` evidence summary. Run refs must identify a recorded current Run compatible with the current `Task`, current Change Unit, current scope revision, and compatible baseline. Historical Runs are audit records unless a current Run explicitly reuses their verified artifacts or evidence and records that reuse.
- Core stores canonical refs and never treats caller-supplied state-version metadata as authority. Core may add the current Run, current Change Unit, and current EvidenceSummary refs.
- Sensitive action requirements in the current close basis are derived by Core from committed Runs and consumed `Write Authorization` records. Category-only caller input cannot establish or erase a requirement.

The current close basis changes through owner-defined transitions:

- A committed `record_run` increments `close_basis_revision` and either establishes a new current close basis from its close assessment or records that no current close basis is established.
- A material scope or current Change Unit change increments `scope_revision`, invalidates the current close basis, and increments `close_basis_revision`.
- Recording user-owned judgment may make a requirement satisfied, stale, or rejected, but it does not increment `scope_revision` or `close_basis_revision`.

Residual-risk identity for close readiness uses opaque `risk_id` values from the current close basis. Risk summary or consequence text can explain the risk to the user, but text matching is not authority.

Cancellation path:

- `intent=cancel` requires a current accepted cancellation judgment with `machine_action=accept`, `resolution_outcome=accepted`, bound to the `Task`, current scope revision, current Change Unit, and verified `user_interaction` actor provenance.
- Cancellation does not require completion-only evidence, final acceptance, or residual-risk acceptance.
- Missing or incompatible cancellation authority is a close-readiness blocker for cancellation, not fabricated acceptance.

Close readiness is not:

- `CloseReadinessBlocker`
- `intent=complete`
- user acceptance alone
- evidence alone
- a generated close summary
- an API preflight rejection

Close blockers:

- A close blocker is a close-relevant reason that prevents honest close readiness.
- If a close blocker remains, the `Task` stays open until the responsible owner-defined requirement is resolved.
- `CloseReadinessBlocker` is the API data representation for close blockers, not the whole close-readiness concept.

Close transition:

- When no close blocker remains and the method owner permits the requested terminal path, the terminal transition may proceed through API-owned method behavior.
- Rejected requests before close-readiness evaluation, stale state, local access failures, and public error precedence belong to API and error owners.

## 11. Blockers, waivers, and residual risk

Blockers preserve honesty.

- A blocker identifies the owner-defined requirement that must be handled before progress, write, Run recording, or close can proceed honestly.
- A blocker must not be hidden by broad approval, projection prose, a generated success summary, or unrelated evidence.

Waivers are narrow.

- A waiver is a scoped exception to one named requirement where the responsible owner allows it.
- A waiver does not create scope, sensitive-action approval, required evidence, final acceptance, residual-risk visibility, QA evidence, verification, or an assurance upgrade.

Residual risk must be visible before it can be accepted.

- Known close-relevant residual risk must be visible enough for the user to judge before successful close depends on accepting it.
- Residual-risk acceptance applies to the named visible risk for the requested close, not to every unknown.
- The supported baseline path uses compact residual-risk visibility, blockers, evidence references, artifact references, and user-judgment references. Rich risk workflows remain outside the baseline unless the scope and semantic owners promote them.

## 12. Related owners

Use this table for owner routing. Do not copy the linked contracts into this page.

| Topic | Owner |
|---|---|
| API method list and method routing | [API Methods](api/methods.md) |
| Method behavior | Method owner documents listed by [API Methods](api/methods.md) |
| Common API envelopes and response branches | [API Schema Core](api/schema-core.md) |
| State-shaped API data, including `ShapingReadiness`, `CloseReadinessBlocker`, and `WriteDecisionReason` | [API State Schemas](api/schema-state.md) and [API Value Sets](api/schema-value-sets.md) |
| User judgment schema shapes, `SensitiveActionScope`, and accepted-risk input shapes | [API Judgment Schemas](api/schema-judgment.md) |
| Artifact refs, artifact input shapes, staging handles, and artifact schema rules | [API Artifact Schemas](api/schema-artifacts.md) |
| Public error code meanings, error routing, and error precedence | [API error codes](api/error-codes.md), [API error routing](api/error-routing.md), and [API error precedence](api/error-precedence.md) |
| Storage records, storage effects, state-version effects, and persistence layout | [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), and [Storage Versioning](storage-versioning.md) |
| Artifact storage lifecycle and body-read rules | [Artifact Storage](storage-artifacts.md) |
| Projection authority and derived display boundaries | [Projection Authority Reference](projection-and-templates.md) |
| Template bodies and rendered display wording | [Template Bodies](template-bodies.md) |
| Security guarantees and access-boundary wording | [Security](security.md) |
| Baseline and out-of-scope capability boundaries | [Scope](scope.md) |
| Runtime and repository separation | [Runtime Boundaries](runtime-boundaries.md) |
| Agent integration and surface capability posture | [Agent Integration](agent-integration.md) |
