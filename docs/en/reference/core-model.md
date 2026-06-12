# Core model reference

This reference defines the Harness Core authority model as documentation reference material only. This repository still has no Harness runtime or server implementation, and the current documentation is not implementation-complete unless the maintainer-owned status says so in [Implementation Guide](../build/implementation-guide.md).

Core is the local authority record for task scope, user-owned judgment, evidence, verification expectations, close readiness, and residual risk. It owns the product meaning of those boundaries. Security guarantee wording and non-claims belong to [Security](security.md).

## 1. Owns / Does not own

This document owns:

- Core authority invariants and non-substitution rules.
- Product meaning for Task scope, Change Unit boundaries, user-owned judgment, evidence, close readiness, close honesty, waivers, and residual risk.
- Conceptual lifecycle and gate boundaries.
- The difference between `WriteDecisionReason`, close-readiness blocking reasons, and `CloseReadinessBlocker`.
- Cross-owner routing when Core concepts touch API, Storage, Security, Projection, or Later material.

This document does not own:

- Public API payload schemas, response branch shapes, envelopes, or method result structures. Use the [API Methods](api/methods.md), method owner documents, [API Schema Core](api/schema-core.md), and the API schema owners.
- Storage DDL, persisted JSON layout, locks, migrations, runtime-home placement, or method-to-storage effects. Use [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), and [Storage Versioning](storage-versioning.md).
- Exact active enum-like values and wire field lists. Use [API Value Sets](api/schema-value-sets.md) and [API State Schemas](api/schema-state.md).
- Public error code definitions or error precedence. Use [API Errors](api/errors.md).
- Rendered projection bodies, template text, connector recipes, security guarantee vocabulary, or out-of-scope capability catalogs.

Exact identifiers may appear here when needed to explain meaning. Their schema shape, value set, storage effect, and public error behavior remain with the linked owner documents.

## 2. Kernel invariants

| Invariant | Details |
|---|---|
| Core-owned state is authority. | See [Core state authority](#core-invariant-state-authority). |
| Harness governs Harness records. | See [Harness record boundary](#core-invariant-harness-record-boundary). |
| Product writes require compatible active scope. | See [Product write scope](#core-invariant-product-write-scope). |
| User-owned judgment stays user-owned. | See [User-owned judgment authority](#core-invariant-user-owned-judgment). |
| `Write Authorization` creation is narrow. | See [`Write Authorization` creation](#core-invariant-write-authorization-creation). |
| Runs record what happened. | See [Run record authority](#core-invariant-run-record-authority). |
| Evidence records support only recorded claims. | See [Evidence record authority](#core-invariant-evidence-record-authority). |
| Close must stay honest. | See [Honest close](#core-invariant-honest-close). |
| Current MVP and out-of-scope capabilities stay separate. | See [Current MVP and later boundary](#core-invariant-mvp-later-boundary). |

<a id="core-invariant-state-authority"></a>
### Core state authority

Concept:
- Core-owned state is the authority for Harness operations.

Not the same as:
- chat
- reports
- generated Markdown
- projections
- template output

Owner links:
- [Projection Authority Reference](projection-and-templates.md)
- [Template Bodies](template-bodies.md)

<a id="core-invariant-harness-record-boundary"></a>
### Harness record boundary

Concept:
- Harness governs Harness records and state transitions.

Not the same as:
- a general security-control surface

Owner links:
- [Security](security.md)

<a id="core-invariant-product-write-scope"></a>
### Product write scope

Condition:
- Product writes require compatible active scope.

Effect:
- A write path outside the current Task and Change Unit must be reshaped before it can be compatible.

Owner links:
- [Prepare Write Method](api/method-prepare-write.md)
- [API State Schemas](api/schema-state.md)

<a id="core-invariant-user-owned-judgment"></a>
### User-owned judgment authority

Concept:
- User-owned judgment stays user-owned.

Not the same as:
- agent inference
- broad consent
- evidence
- projection text
- generated summaries

Owner links:
- [API Judgment Schemas](api/schema-judgment.md)

<a id="core-invariant-write-authorization-creation"></a>
### `Write Authorization` creation

Condition:
- Only a non-dry-run allowed `prepare_write` path creates a consumable `Write Authorization`.

Effect:
- `Write Authorization` is single-use for one compatible product-file attempt.

Not allowed:
- It is not reusable scope and not general permission.

Owner links:
- [Prepare Write Method](api/method-prepare-write.md)

<a id="core-invariant-run-record-authority"></a>
### Run record authority

Concept:
- Runs record what happened.

Not the same as:
- retroactive authorization for work that lacked scope, required judgment, sensitive-action approval, or `Write Authorization`

Owner links:
- [Record Run Method](api/method-record-run.md)

<a id="core-invariant-evidence-record-authority"></a>
### Evidence record authority

Concept:
- Evidence records support only the claims they actually record.

Not the same as:
- acceptance
- QA
- verification
- residual-risk acceptance
- proof of unrecorded facts

Owner links:
- [API Artifact Schemas](api/schema-artifacts.md)
- [API State Schemas](api/schema-state.md)

<a id="core-invariant-honest-close"></a>
### Honest close

Condition:
- Close-relevant blockers remain.

Effect:
- Core reports blockers instead of treating the Task as successfully completed.

Owner links:
- [Close Task Method](api/method-close-task.md)
- [API State Schemas](api/schema-state.md)

<a id="core-invariant-mvp-later-boundary"></a>
### Current MVP and later boundary

Concept:
- Current MVP and out-of-scope capabilities stay separate.

Not active until promoted:
- later verification
- Manual QA
- rich waiver
- assurance material

Owner links:
- [Scope](scope.md)
- [Scope Reference](scope.md)

## 3. Core entities

These entities describe authority relationships, not storage tables or API bodies.

| Entity | Details |
|---|---|
| Task | See [Task](#core-entity-task-boundary). |
| Change Unit | See [Change Unit](#core-entity-change-unit). |
| Autonomy Boundary | See [Autonomy Boundary](#core-entity-autonomy-boundary). |
| `user_judgment` | See [`user_judgment`](#core-entity-user-judgment). |
| `Write Authorization` | See [`Write Authorization`](#core-entity-write-authorization-boundary). |
| Run | See [Run](#core-entity-run). |
| Evidence summary | See [Evidence summary](#core-entity-evidence-summary). |
| `ArtifactRef` | See [`ArtifactRef`](#core-entity-artifactref-boundary). |
| Blocker | See [Blocker](#core-entity-blocker). |
| Residual-risk summary | See [Residual-risk summary](#core-entity-residual-risk-summary). |
| Projection output | See [Projection output](#core-entity-projection-output). |
| Template output | See [Template output](#core-entity-template-output-boundary). |
| `ShapingReadiness` | See [`ShapingReadiness`](#core-entity-shaping-readiness). |

<a id="core-entity-task-boundary"></a>
### Task

Concept:
- A Task is the user-value unit being shaped, executed, blocked, or closed.

Owner links:
- Exact lifecycle values and public state fields are owned by [API Value Sets](api/schema-value-sets.md) and [API State Schemas](api/schema-state.md).

<a id="core-entity-change-unit"></a>
### Change Unit

Concept:
- A Change Unit is the active scoped work boundary for write-capable work.

Not the same as:
- final acceptance
- evidence
- permission to widen scope silently

<a id="core-entity-autonomy-boundary"></a>
### Autonomy Boundary

Concept:
- An Autonomy Boundary is the agent latitude inside a Change Unit.

Not the same as:
- scope expansion
- sensitive-action approval
- permission to make user-owned judgments

<a id="core-entity-user-judgment"></a>
### `user_judgment`

Concept:
- `user_judgment` is the record family for decisions the user owns.

Effect:
- It can feed compatibility when the recorded judgment matches the affected object, scope, consequence, and close or write impact.

Not the same as:
- active scope mutation
- evidence creation
- `Write Authorization`
- sensitive-action approval, final acceptance, or residual-risk acceptance unless that exact judgment kind was asked and recorded
- Task close by itself

Owner links:
- [API Judgment Schemas](api/schema-judgment.md)

<a id="core-entity-write-authorization-boundary"></a>
### `Write Authorization`

Concept:
- `Write Authorization` is a durable, single-use Core authorization for one compatible product-file write attempt.

Not allowed:
- It is not OS permission, command approval, sensitive-action approval, final acceptance, or reusable scope.

Owner links:
- [Prepare Write Method](api/method-prepare-write.md)
- [Record Run Method](api/method-record-run.md)
- [API State Schemas](api/schema-state.md)
- [Storage Effects](storage-effects.md)

<a id="core-entity-run"></a>
### Run

Concept:
- A Run is a record of execution or observation.

Not the same as:
- Retroactive authorization for missing scope, missing judgment, missing approval, or missing `Write Authorization`.
- Compatibility for later product writes when the Run was read-only or shaping-only.

Owner links:
- [Record Run Method](api/method-record-run.md)

<a id="core-entity-evidence-summary"></a>
### Evidence summary

Concept:
- An evidence summary is the compact Core path for close-relevant support, gaps, refs, and coverage expectations.

Not the same as:
- Final acceptance.
- Residual-risk acceptance.
- Full `Evidence Manifest` behavior unless promoted by an owner.

Owner links:
- [API State Schemas](api/schema-state.md)
- [API Artifact Schemas](api/schema-artifacts.md)

<a id="core-entity-artifactref-boundary"></a>
### `ArtifactRef`

Concept:
- `ArtifactRef` is a durable reference to an evidence-eligible artifact when artifact owners allow that use.

Owner links:
- Artifact shape, staging, promotion, integrity, and body-read rules are owned by [API Artifact Schemas](api/schema-artifacts.md) and [Artifact Storage](storage-artifacts.md).

<a id="core-entity-blocker"></a>
### Blocker

Concept:
- A blocker is a structured reason progress, write, Run recording, or close cannot proceed honestly.

Not the same as:
- projection prose
- broad approval
- a successful-looking close result

Owner links:
- [API State Schemas](api/schema-state.md)
- [API Value Sets](api/schema-value-sets.md)

<a id="core-entity-residual-risk-summary"></a>
### Residual-risk summary

Concept:
- A residual-risk summary is the compact visibility path for known remaining uncertainty, limits, or trade-offs.

Not the same as:
- verification
- evidence sufficiency
- final acceptance
- a no-risk result
- rich residual-risk records or assurance displays unless promoted by an owner

Owner links:
- [API Judgment Schemas](api/schema-judgment.md)
- [API State Schemas](api/schema-state.md)
- [Scope Reference](scope.md)

<a id="core-entity-projection-output"></a>
### Projection output

Concept:
- Projection output is derived display from Core state and refs.

Not the same as:
- authority
- evidence
- acceptance

Owner links:
- [Projection Authority Reference](projection-and-templates.md)

<a id="core-entity-template-output-boundary"></a>
### Template output

Concept:
- Template output is rendered body text for cards, requests, summaries, results, and packets.

Owner links:
- Body expectations belong to [Template Bodies](template-bodies.md).

Not allowed:
- Readability or manual editing does not turn output into authority.

<a id="core-entity-shaping-readiness"></a>
### `ShapingReadiness`

Concept:
- `ShapingReadiness` is a compact derived view over Core state for the next safe action.

Inputs:
- Task.
- Change Unit.
- Pending judgments.
- Evidence summary.
- Blockers.
- Next-action state.

Owner links:
- Core owns the readiness meaning: whether current owner state is concrete enough for the next safe action.
- Wire fields are owned by [API State Schemas](api/schema-state.md).

## 4. User-owned judgment

Concept:
- User-owned judgment is the boundary where Harness must ask the user or preserve the user's recorded choice instead of inferring it.
- This page owns the product meaning. Exact schema fields and input shapes belong to the judgment schema owner.

Inputs:
- A product, technical, scope, sensitive-action, final-acceptance, residual-risk, or cancellation question that belongs to the user.
- The affected object, scope, consequence, and close or write impact when one user reply is meant to satisfy more than one judgment kind.

Not the same as:
- Agent inference, broad consent, evidence, projection text, or generated summaries.
- Active scope mutation, `Write Authorization`, sensitive-action approval, final acceptance, or residual-risk acceptance unless that exact judgment kind was asked and recorded.

Owner links:
- [API Judgment Schemas](api/schema-judgment.md)

Judgment kinds:

| Judgment kind | User owns the decision when the question concerns |
|---|---|
| `product_decision` | User-visible behavior, user flow, copy, UX, accessibility, release promise, product trade-off, or user value. |
| `technical_decision` | See [`technical_decision`](#core-judgment-technical-decision). |
| `scope_decision` | Scope expansion, non-goal removal, Change Unit boundary changes, or Autonomy Boundary changes. |
| `sensitive_approval` | Permission for a named sensitive step inside a bounded `SensitiveActionScope`. |
| `final_acceptance` | The user's result judgment when the close path requires acceptance. |
| `residual_risk_acceptance` | The user's acceptance of a named visible residual risk for the requested close. |
| `cancellation` | Stopping the Task without a successful completed result. |

<a id="core-judgment-technical-decision"></a>
### `technical_decision`

Condition:
- The question concerns architecture, dependency or external service introduction, authentication direction, or migration.
- The question concerns public interface, compatibility break, data retention, privacy, or security.
- The question concerns another material and costly-to-reverse technical direction.

Agent latitude:
- Inside accepted scope and acceptance criteria, the agent may choose ordinary implementation details that do not change product behavior, technical direction, scope, security/privacy posture, compatibility, or costly-to-reverse architecture.

Not the same as:
- A new permission system.
- Broad consent that silently satisfies another judgment kind.

Multiple judgments:
- "Go ahead", "looks good", or similar wording cannot silently satisfy another judgment kind.
- A single reply may satisfy multiple judgments only when the prompt asked those distinct questions and Core records each compatible judgment with its affected object, scope, consequence, and close or write impact.

## 5. Non-substitution rules

| Boundary | Details |
|---|---|
| Displays and generated text | See [Displays and generated text](#core-non-substitution-displays). |
| Evidence and Run records | See [Evidence and Run records](#core-non-substitution-evidence-runs). |
| `final_acceptance` | See [`final_acceptance`](#core-non-substitution-final-acceptance). |
| `residual_risk_acceptance` | See [`residual_risk_acceptance`](#core-non-substitution-residual-risk-acceptance). |
| `sensitive_approval` | See [`sensitive_approval`](#core-non-substitution-sensitive-approval). |
| `Write Authorization` and `AuthorizedAttemptScope` | See [`Write Authorization` and `AuthorizedAttemptScope`](#core-non-substitution-write-authorization). |
| `WriteDecisionReason` | See [`WriteDecisionReason`](#core-non-substitution-write-decision-reason). |
| `CloseReadinessBlocker` | See [`CloseReadinessBlocker`](#core-non-substitution-close-readiness-blocker). |
| Waiver or accepted risk | See [Waiver or accepted risk](#core-non-substitution-waiver-risk). |

<a id="core-non-substitution-displays"></a>
### Displays and generated text

Applies to:
- Chat, reports, generated Markdown, projection prose, and status cards.

Does not substitute for:
- Core-owned state.

<a id="core-non-substitution-evidence-runs"></a>
### Evidence and Run records

Applies to:
- Evidence, logs, screenshots, artifacts, test output, and Run records.

Does not substitute for:
- final acceptance
- future Manual QA
- future verification
- residual-risk acceptance

<a id="core-non-substitution-final-acceptance"></a>
### `final_acceptance`

Does not substitute for:
- evidence
- QA
- verification
- sensitive-action approval
- scope change
- residual-risk acceptance
- blocker override

<a id="core-non-substitution-residual-risk-acceptance"></a>
### `residual_risk_acceptance`

Does not substitute for:
- verification
- evidence sufficiency
- QA
- final acceptance
- a no-risk result

<a id="core-non-substitution-sensitive-approval"></a>
### `sensitive_approval`

Does not substitute for:
- product direction
- technical direction
- scope
- correctness
- evidence
- QA
- final acceptance
- residual-risk acceptance
- `Write Authorization`

<a id="core-non-substitution-write-authorization"></a>
### `Write Authorization` and `AuthorizedAttemptScope`

Does not substitute for:
- command approval
- dependency approval
- host, network, or secret access
- deployment approval
- destructive-action approval
- system access
- final acceptance

<a id="core-non-substitution-write-decision-reason"></a>
### `WriteDecisionReason`

Does not substitute for:
- a close-readiness blocker
- `CloseReadinessBlocker`

<a id="core-non-substitution-close-readiness-blocker"></a>
### `CloseReadinessBlocker`

Does not substitute for:
- a prepare-write decision reason
- the entire close-readiness concept
- evidence
- acceptance
- storage effect by itself

<a id="core-non-substitution-waiver-risk"></a>
### Waiver or accepted risk

Does not substitute for:
- automatic success
- verification
- evidence
- final acceptance
- close without the remaining required owner paths

Compact user-facing displays may summarize these boundaries, but they must not collapse them.

## 6. Task lifecycle

| Lifecycle area | Details |
|---|---|
| Intake and shaping | See [Intake and shaping](#core-lifecycle-intake-and-shaping). |
| Scope update | See [Scope update](#core-lifecycle-scope-update). |
| Execution and observation | See [Execution and observation](#core-lifecycle-execution-observation). |
| Waiting or blocked | See [Waiting or blocked](#core-lifecycle-waiting-blocked). |
| Close attempt | See [Close attempt](#core-lifecycle-close-attempt). |
| Terminal outcome | See [Terminal outcome](#core-lifecycle-terminal-outcome). |

<a id="core-lifecycle-intake-and-shaping"></a>
### Intake and shaping

Effect:
- Turns ordinary user intent into a concrete goal, active scope, non-goals, acceptance criteria, Autonomy Boundary, and next safe action.

Required honesty:
- If a user-owned issue blocks the next safe action, expose the judgment need instead of guessing.

<a id="core-lifecycle-scope-update"></a>
### Scope update

Effect:
- Accepted scope or Change Unit changes move through `harness.update_scope`.

Not the same as:
- `scope_decision` records mutating active scope by themselves.

<a id="core-lifecycle-execution-observation"></a>
### Execution and observation

Effect:
- Run records describe actions or observations.

Required honesty:
- Product-file writes must be compatible with active scope and `Write Authorization`.
- Read-only work does not authorize later writes.

<a id="core-lifecycle-waiting-blocked"></a>
### Waiting or blocked

Condition:
- An owner path is missing, stale, incompatible, or unsafe to bypass.

Effect:
- Progress pauses.
- The blocker points to the next safe owner path rather than hiding the gap.

<a id="core-lifecycle-close-attempt"></a>
### Close attempt

Concept:
- Core evaluates whether the Task can close honestly.

Input:
- Current Core state, not a final chat summary alone.

Owner links:
- [Close Task Method](api/method-close-task.md)
- [API State Schemas](api/schema-state.md)

<a id="core-lifecycle-terminal-outcome"></a>
### Terminal outcome

Effect:
- Completion, cancellation, or supersession ends the Task path.

Not allowed:
- Cancellation and supersession are terminal, but they are not successful completion.
- They do not satisfy evidence, acceptance, or risk requirements for completion.

## 7. Active gates

Gates are compatibility summaries for progress, write, Run recording, and close. This page owns their product meaning. Public fields, exact values, and wire shapes are owned by [API State Schemas](api/schema-state.md) and [API Value Sets](api/schema-value-sets.md).

| Gate area | Details |
|---|---|
| Scope gate | See [Scope gate](#core-gate-scope). |
| Decision gate | See [Decision gate](#core-gate-decision). |
| Sensitive-action approval gate | See [Sensitive-action approval gate](#core-gate-sensitive-action-approval). |
| Write-compatibility gate | See [Write-compatibility gate](#core-gate-write-compatibility). |
| Evidence gate | See [Evidence gate](#core-gate-evidence). |
| Acceptance gate | See [Acceptance gate](#core-gate-acceptance). |
| Residual-risk gate | See [Residual-risk gate](#core-gate-residual-risk). |
| Close-readiness gate | See [Close-readiness gate](#core-gate-close-readiness). |

<a id="core-gate-scope"></a>
### Scope gate

Condition:
- Active scope and Change Unit must cover the requested work.

Not the same as:
- deciding product questions for the user
- deciding technical questions for the user

Owner links:
- [Update Scope Method](api/method-update-scope.md)
- [API State Schemas](api/schema-state.md)

<a id="core-gate-decision"></a>
### Decision gate

Condition:
- A user-owned decision is required before progress, write, Run recording, or close can continue.

Not the same as:
- evidence
- sensitive-action approval
- final acceptance
- residual-risk acceptance

Owner links:
- [API Judgment Schemas](api/schema-judgment.md)
- [API State Schemas](api/schema-state.md)

<a id="core-gate-sensitive-action-approval"></a>
### Sensitive-action approval gate

Condition:
- A named sensitive step inside `SensitiveActionScope` requires approval.

Not the same as:
- `Write Authorization`
- broad permission
- product correctness

Owner links:
- [API Judgment Schemas](api/schema-judgment.md)
- [Prepare Write Method](api/method-prepare-write.md)

<a id="core-gate-write-compatibility"></a>
### Write-compatibility gate

Concept:
- Whether a product-file write attempt is compatible with active scope and a consumable `Write Authorization`.

Not allowed:
- It does not approve commands, hosts, network, secrets, deployments, or destructive operations.

Owner links:
- [Prepare Write Method](api/method-prepare-write.md)
- [Record Run Method](api/method-record-run.md)
- [API State Schemas](api/schema-state.md)

<a id="core-gate-evidence"></a>
### Evidence gate

Condition:
- Close-relevant required support must be present and usable enough for the close path.

Not the same as:
- proof beyond what was recorded
- user acceptance
- residual-risk acceptance

Owner links:
- [API State Schemas](api/schema-state.md)
- [API Artifact Schemas](api/schema-artifacts.md)

<a id="core-gate-acceptance"></a>
### Acceptance gate

Condition:
- Required final acceptance must be present for the visible close basis.

Not the same as:
- filling evidence gaps
- accepting residual risk
- changing scope

Owner links:
- [API Judgment Schemas](api/schema-judgment.md)
- [Close Task Method](api/method-close-task.md)

<a id="core-gate-residual-risk"></a>
### Residual-risk gate

Condition:
- Close-relevant residual risk must be visible and, when required, accepted.

Not the same as:
- verification
- a risk-free result
- final acceptance

Owner links:
- [API Judgment Schemas](api/schema-judgment.md)
- [API State Schemas](api/schema-state.md)

<a id="core-gate-close-readiness"></a>
### Close-readiness gate

Concept:
- The close-readiness gate summarizes whether all close-relevant checks support an honest close.

Effect:
- If a close blocker remains, the Task stays open until the owner path addresses it.

Not the same as:
- `CloseReadinessBlocker`
- `intent=complete`
- user acceptance alone

Owner links:
- [Close Task Method](api/method-close-task.md)
- [API State Schemas](api/schema-state.md)

Verification and Manual QA are conceptual boundaries in the current MVP, not active gates. They must not be described as active close requirements unless a future owner promotes them.

## 8. Write authorization boundary

Concept:
- `Write Authorization` is the Core record that makes one product-file write attempt compatible with current Harness state.

Creation:
- It is created only through the compatible non-dry-run `prepare_write` path defined by the API owner.

Inputs:
- Current Harness state.
- Active Task and Change Unit scope.
- The intended product-file write attempt.
- A compatible non-dry-run `prepare_write` result.

Properties:
- Scope-limited: it covers the intended product-file write attempt, not future work or a broader project area.
- Single-use: a compatible product-write Run consumes it once. Reuse, replay, and stale-state behavior are API/storage-owned details.
- Cooperative: it tells a connected agent or surface what is compatible with Harness state; it does not enforce OS-level prevention.

Not the same as:
- `sensitive_approval`, command approval, dependency approval, host/network/secret access, deployment approval, destructive-action approval, system access, or final acceptance.
- Proof that the write happened, evidence creation, acceptance, residual-risk acceptance, or Task close.

Owner links:
- [Prepare Write Method](api/method-prepare-write.md)
- [Record Run Method](api/method-record-run.md)
- [API State Schemas](api/schema-state.md)
- [Storage Effects](storage-effects.md)

Decision reason boundary:
- `WriteDecisionReason` belongs to prepare-write decision output.
- `CloseReadinessBlocker` belongs to close-readiness blocking data.
- They answer different questions and must not be interchanged.

## 9. Evidence and run authority

| Record | Details |
|---|---|
| Run | See [Run authority](#core-evidence-run-authority). |
| Evidence summary | See [Evidence summary authority](#core-evidence-summary-authority). |
| `ArtifactRef` | See [`ArtifactRef` evidence use](#core-evidence-artifactref-use). |
| Projection or report | See [Projection or report authority](#core-evidence-projection-report-authority). |

<a id="core-evidence-run-authority"></a>
### Run authority

Can establish:
- An execution or observation was recorded with the available context and refs.

Cannot establish:
- Missing authorization, missing judgment, or missing approval existed retroactively.

Owner links:
- [Record Run Method](api/method-record-run.md)

<a id="core-evidence-summary-authority"></a>
### Evidence summary authority

Can establish:
- Specific close-relevant claims have recorded support, gaps, refs, or coverage expectations.

Cannot establish:
- Unrecorded behavior happened.
- The result is accepted.
- Risk is accepted.

Owner links:
- [API State Schemas](api/schema-state.md)
- [API Artifact Schemas](api/schema-artifacts.md)

<a id="core-evidence-artifactref-use"></a>
### `ArtifactRef` evidence use

Can establish:
- An artifact reference is available for evidence use when artifact owners allow it.

Cannot establish:
- The artifact content is safe, sufficient, or readable beyond the recorded integrity/redaction/availability facts.

Owner links:
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)

<a id="core-evidence-projection-report-authority"></a>
### Projection or report authority

Can establish:
- A display was generated from available state and refs.

Cannot establish:
- The display itself is authority.
- The display itself is evidence.
- The display itself is acceptance.

Owner links:
- [Projection Authority Reference](projection-and-templates.md)
- [Template Bodies](template-bodies.md)

### Evidence authority

Concept:
- Evidence records support only the claims they record at their recorded scope.

Inputs:
- Run records.
- Evidence summaries.
- Evidence-eligible artifacts and `ArtifactRef` values when artifact owners allow them.
- Related refs and coverage expectations.

Can establish:
- A passing test log supports the test it names.
- A screenshot supports the visible state it captures.
- An artifact supports only the content and integrity facts represented by the artifact owners.

Not the same as:
- Proof of broader correctness.
- Final acceptance, future Manual QA, future verification, or residual-risk acceptance.
- Proof of unrecorded behavior.

Owner links:
- [API Artifact Schemas](api/schema-artifacts.md)
- [Artifact Storage](storage-artifacts.md)
- [API Judgment Schemas](api/schema-judgment.md)

<a id="close_task"></a>
## 10. Close readiness

Concept:
- Close readiness is the Core evaluation concept for whether the current Task can close honestly.

Inputs:
- Current Core state.
- Task scope and Change Unit scope.
- Required evidence and close-relevant artifacts.
- Required user-owned judgments.
- Required sensitive-action approval.
- Write and Run compatibility.
- Unresolved blockers.
- Final acceptance, when required.
- Accepted residual risk, where applicable.
- Recovery constraints.

Not the same as:
- `CloseReadinessBlocker`.
- `intent=complete`.
- User acceptance alone.
- Preflight rejection.

Schema boundary:
- `CloseReadinessBlocker` is a data representation for close blocking reasons, not the close-readiness evaluation concept.

Owner links:
- [Close Task Method](api/method-close-task.md)
- [API State Schemas](api/schema-state.md)
- [API Value Sets](api/schema-value-sets.md)
- [Storage Effects](storage-effects.md)
- [API Errors](api/errors.md)

For an `intent=complete` close attempt, Core evaluates blockers in this conceptual order. Later rows do not satisfy earlier rows.

| Order | Check area | Close-readiness meaning |
|---:|---|---|
| 1 | Task lifecycle | The selected Task must be eligible for the requested terminal path. |
| 2 | Open or unrepaired Runs | Close cannot rely on open, unsafe, interrupted, incompatible, or unrepaired Run state. |
| 3 | Scope and Change Unit | Active scope, acceptance criteria, and the applicable completion policy must support the close claim. |
| 4 | User-owned judgment | Required product, technical, scope, and other non-sensitive user judgments must be resolved and compatible. |
| 5 | Sensitive-action approval | Required sensitive-action approval must be present and compatible with the bounded step. |
| 6 | Write and Run compatibility | Product-write claims must be backed by compatible authorization and recorded Run relationships. |
| 7 | Baseline and surface capability | The baseline and connected surface must honestly support the close claim and any guarantee display. |
| 8 | Evidence sufficiency | Required evidence coverage must be present, current, and usable for the close basis. |
| 9 | Artifact availability | Close-relevant artifacts must be available and usable under artifact-owner rules. |
| 10 | Final acceptance | Required final acceptance must be tied to the visible close basis. |
| 11 | Residual-risk visibility | Known close-relevant risk must be visible enough for the user to judge. |
| 12 | Residual-risk acceptance | Required acceptance of visible residual risk must be compatible with the requested close. |
| 13 | Recovery constraints | Remaining repair, corruption, reconciliation, or recovery work must be handled before close. |
| 14 | Close transition | See [Close transition](#core-close-readiness-close-transition). |

<a id="core-close-readiness-close-transition"></a>
### Close transition

Condition:
- No close blocker remains.

Effect:
- The terminal transition may proceed through the API-owned method behavior.

If blocked:
- The Task stays open.

Owner links:
- [Close Task Method](api/method-close-task.md)

Preflight failures:
- Stale state, invalid request identity, local access failure before evaluation, and similar API-owned failures are not semantic close-readiness findings.
- They are routed through the API and error owners.

## 11. Blockers and waivers

### Blocker

Concept:
- A blocker is a structured reason progress, write, Run recording, or close cannot proceed honestly.

Not the same as:
- Projection prose.
- Broad approval.
- A successful-looking close result.

Owner links:
- [API State Schemas](api/schema-state.md)
- [API Value Sets](api/schema-value-sets.md)

### Close blocker

Concept:
- A close blocker is a close-relevant reason that prevents honest close readiness.

Not the same as:
- `WriteDecisionReason`.
- Proof of storage effects by itself.

Owner links:
- [Close Task Method](api/method-close-task.md)
- [API State Schemas](api/schema-state.md)
- [API Value Sets](api/schema-value-sets.md)

### `CloseReadinessBlocker`

Concept:
- `CloseReadinessBlocker` is the API data representation of close blocking reasons.

Not the same as:
- The whole close-readiness concept.
- A prepare-write reason.
- Proof of persistence by itself.

Owner links:
- [API State Schemas](api/schema-state.md)
- [API Value Sets](api/schema-value-sets.md)
- [API Errors](api/errors.md)

### Waiver

Concept:
- A waiver is a scoped exception to a named requirement where the responsible owner allows it.

Allowed effect:
- It can unblock only the named requirement and only through the owner path that permits it.

Not the same as:
- Decision deferral.
- Scope creation, sensitive-action approval, required evidence, final acceptance, or residual-risk visibility.
- QA evidence, a QA pass, verification, or an assurance upgrade.

Owner links:
- [Scope Reference](scope.md)

## 12. Residual risk

Concept:
- Residual risk is known remaining uncertainty, an unchecked condition, limitation, or trade-off that matters to close.

Inputs:
- The visible named risk.
- The requested close and visible close basis.
- Related evidence, artifact, blocker, or Run refs.
- Compatible `residual_risk_acceptance` when close depends on accepting the risk.

Required order:
- Known close-relevant residual risk must be visible before successful close.
- The user cannot accept a risk that has not been made visible enough to judge.

Scope:
- Acceptance applies to the named visible risk for the requested close, not to all unknowns.

Not the same as:
- Verification, evidence sufficiency, QA, sensitive-action approval, final acceptance, or a no-risk result.
- A waiver or automatic success.

Current MVP path:
- The current path is compact residual-risk summary, blockers, evidence refs, and `user_judgment` refs unless an owner promotes more.
- Rich risk workflows are later material.

Owner links:
- [API Judgment Schemas](api/schema-judgment.md)
- [API State Schemas](api/schema-state.md)
- [Scope Reference](scope.md)

## 13. Cross-owner links

| Topic | Details |
|---|---|
| API methods and envelopes | See [API methods and envelopes](#core-owner-api-methods-envelopes). |
| State-shaped API data | See [State-shaped API data](#core-owner-state-shaped-api-data). |
| User judgment schemas | See [User judgment schemas](#core-owner-user-judgment-schemas). |
| Artifact schemas and lifecycle | See [Artifact schemas and lifecycle](#core-owner-artifact-schemas-lifecycle). |
| Public errors | See [Public errors](#core-owner-public-errors). |
| Storage records and effects | See [Storage records and effects](#core-owner-storage-records-effects). |
| Projection authority | See [Projection authority](#core-owner-projection-authority). |
| Template bodies | See [Template bodies](#core-owner-template-bodies). |
| Security wording | See [Security wording](#core-owner-security-wording). |
| Runtime boundaries | See [Runtime boundaries](#core-owner-runtime-boundaries). |
| Design quality | See [Design quality](#core-owner-design-quality). |
| Agent integration | See [Agent integration](#core-owner-agent-integration). |
| Out-of-scope capabilities | See [Out-of-scope capabilities](#core-owner-out-of-scopes). |

<a id="core-owner-api-methods-envelopes"></a>
### API methods and envelopes

Applies to:
- API method behavior.
- Request and response shapes.
- Envelopes.
- Dry-run and rejection branches.
- Method effects.

Owner links:
- [API Methods](api/methods.md) and the method owner documents it lists.
- [API Schema Core](api/schema-core.md).

<a id="core-owner-state-shaped-api-data"></a>
### State-shaped API data

Applies to:
- `ShapingReadiness`.
- `CloseReadinessBlocker`.
- `ValidatorResult`.
- Public state fields.

Owner links:
- [API State Schemas](api/schema-state.md).
- [API Value Sets](api/schema-value-sets.md).

<a id="core-owner-user-judgment-schemas"></a>
### User judgment schemas

Applies to:
- User judgment schema.
- `SensitiveActionScope`.
- Accepted-risk input shapes.

Owner links:
- [API Judgment Schemas](api/schema-judgment.md).

<a id="core-owner-artifact-schemas-lifecycle"></a>
### Artifact schemas and lifecycle

Owner links:
- [API Artifact Schemas](api/schema-artifacts.md).
- [Artifact Storage](storage-artifacts.md).

<a id="core-owner-public-errors"></a>
### Public errors

Applies to:
- Public error codes.
- Error routing.
- Error precedence.

Owner links:
- [API Errors](api/errors.md).

<a id="core-owner-storage-records-effects"></a>
### Storage records and effects

Owner links:
- [Storage Records](storage-records.md).
- [Storage Effects](storage-effects.md).
- [Storage Versioning](storage-versioning.md).

<a id="core-owner-projection-authority"></a>
### Projection authority

Applies to:
- Projection authority.
- Read-only display boundaries.

Owner links:
- [Projection Authority Reference](projection-and-templates.md).

<a id="core-owner-template-bodies"></a>
### Template bodies

Applies to:
- Status card bodies.
- Judgment request bodies.
- Run and evidence summary bodies.
- Close result bodies.
- Agent context packet bodies.

Owner links:
- [Template Bodies](template-bodies.md).

<a id="core-owner-security-wording"></a>
### Security wording

Applies to:
- Security guarantee wording.
- Cooperative, detective, and preventive claims.
- Local access posture.

Owner links:
- [Security Reference](security.md).

<a id="core-owner-runtime-boundaries"></a>
### Runtime boundaries

Applies to:
- Product Repository separation.
- Harness Server separation.
- Harness Runtime Home separation.

Owner links:
- [Runtime Boundaries Reference](runtime-boundaries.md).

<a id="core-owner-design-quality"></a>
### Design quality

Applies to:
- Design-quality boundaries.
- Non-gate routing.

Owner links:
- [Design Quality](design-quality.md).

<a id="core-owner-agent-integration"></a>
### Agent integration

Applies to:
- Connector behavior.
- Surface capability posture.

Owner links:
- [Agent Integration Reference](agent-integration.md).

<a id="core-owner-out-of-scopes"></a>
### Out-of-scope capabilities

Applies to:
- Out-of-scope capabilities.
- Future assurance, waiver, QA, verification, and fixture material.

Owner links:
- [Scope Reference](scope.md).

If another document needs exact schema, DDL, rendered template text, public error codes, or out-of-scope capability catalogs, it must link to the owner instead of redefining them here.
