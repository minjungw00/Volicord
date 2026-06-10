# Core model reference

This reference defines the future Harness Core authority model as documentation source material only. This repository still has no Harness runtime or server implementation, and the current documentation is not implementation-complete unless the maintainer-owned status says so in [MVP Plan](../build/mvp-plan.md).

Core is the local authority record for task scope, user-owned judgment, evidence, verification expectations, close readiness, and residual risk. It owns the product meaning of those boundaries. It does not grant OS permissions, sandbox arbitrary tools, make files tamper-proof, or provide isolation unless another owner documents and proves that exact mechanism.

## 1. Owns / does not own

| This document owns | This document does not own |
|---|---|
| Core authority invariants and non-substitution rules. | Public API payload schemas, response branch shapes, envelopes, or method result structures. Use [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), and the API schema owners. |
| Product meaning for Task scope, Change Unit boundaries, user-owned judgment, evidence, close readiness, close honesty, waivers, and residual risk. | Storage DDL, persisted JSON layout, locks, migrations, runtime-home placement, or method-to-storage effects. Use [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Artifact Storage](storage-artifacts.md), and [Storage Versioning](storage-versioning.md). |
| Conceptual lifecycle and gate boundaries. | Exact active enum-like values and wire field lists. Use [API Value Sets](api/schema-value-sets.md) and [API State Schemas](api/schema-state.md). |
| The difference between `WriteDecisionReason`, close-readiness blocking reasons, and `CloseReadinessBlocker`. | Public error code definitions or error precedence. Use [API Errors](api/errors.md). |
| Cross-owner routing when Core concepts touch API, Storage, Security, Projection, or Later material. | Rendered projection bodies, template text, connector recipes, security guarantee vocabulary, or later candidate catalogs. |

Exact identifiers may appear here when needed to explain meaning. Their schema shape, value set, storage effect, and public error behavior remain with the linked owner documents.

## 2. Kernel invariants

| Invariant | Consequence |
|---|---|
| Core-owned state is the authority for Harness operations. | Chat, reports, generated Markdown, projections, and template output are displays or context, not authority. |
| Harness governs Harness records and state transitions. | It is not an OS permission system, arbitrary-tool sandbox, tamper-proof store, or security isolation layer by default. |
| Product writes require compatible active scope. | A write path outside the current Task and Change Unit must be reshaped before it can be compatible. |
| User-owned judgment stays user-owned. | Agent inference, broad consent, evidence, projection text, and generated summaries cannot replace a required user judgment. |
| A non-dry-run allowed `prepare_write` path is the only Core path that creates a consumable `Write Authorization`. | `Write Authorization` is single-use for one compatible product-file attempt. It is not reusable scope and not general permission. |
| Runs record what happened. | `record_run` cannot retroactively authorize work that lacked scope, required judgment, sensitive-action approval, or `Write Authorization`. |
| Evidence records support only the claims they actually record. | Evidence does not become acceptance, QA, verification, residual-risk acceptance, or proof of unrecorded facts. |
| Close must stay honest. | If close-relevant blockers remain, Core reports blockers instead of treating the Task as successfully completed. |
| Current MVP and later candidates stay separate. | Later verification, Manual QA, rich waiver, and assurance material is inactive until an owner promotes it. |

## 3. Core entities

These entities describe authority relationships, not storage tables or API bodies.

| Entity | Core meaning | Boundary |
|---|---|---|
| Task | The user-value unit being shaped, executed, blocked, or closed. | Exact lifecycle values and public state fields are owned by [API Value Sets](api/schema-value-sets.md) and [API State Schemas](api/schema-state.md). |
| Change Unit | The active scoped work boundary for write-capable work. | It is not final acceptance, evidence, or permission to widen scope silently. |
| Autonomy Boundary | The agent latitude inside a Change Unit. | It is not scope expansion, sensitive-action approval, or permission to make user-owned judgments. |
| `user_judgment` | The record family for decisions the user owns. | It feeds compatibility but does not by itself mutate active scope, create evidence, authorize writes, accept risk, or close a Task. |
| `Write Authorization` | A durable, single-use Core authorization for one compatible product-file write attempt. | It is not OS permission, command approval, sensitive-action approval, final acceptance, or reusable scope. |
| Run | A record of execution or observation. | Read-only and shaping-only Runs do not make later product writes compatible. |
| Evidence summary | The compact Core path for close-relevant support, gaps, refs, and coverage expectations. | Full `Evidence Manifest` behavior is not active unless promoted by an owner. |
| `ArtifactRef` | A durable reference to an evidence-eligible artifact when the artifact owners allow it. | Artifact shape, staging, promotion, integrity, and body-read rules are owned by [API Artifact Schemas](api/schema-artifacts.md) and [Artifact Storage](storage-artifacts.md). |
| Blocker | A structured reason progress, write, Run recording, or close cannot proceed honestly. | Schema shape and active value sets belong to the API schema/value owners. |
| Residual-risk summary | The compact visibility path for known remaining uncertainty, limits, or trade-offs. | Rich residual-risk records and assurance displays are later candidate material until promoted. |
| Projection output | Derived display from Core state and refs. | Authority and freshness boundaries belong to [Projection Authority Reference](projection-and-templates.md). |
| Template output | Rendered body text for cards, requests, summaries, results, and packets. | Body expectations belong to [Template Bodies](template-bodies.md); readability or manual editing does not turn output into authority. |

`ShapingReadiness` is a compact derived view over Task, Change Unit, pending judgments, evidence summary, blockers, and next-action state. Core owns the readiness meaning: whether the current owner state is concrete enough for the next safe action. The wire fields are owned by [API State Schemas](api/schema-state.md).

## 4. User-owned judgment

User-owned judgment is the boundary where Harness must ask or preserve the user's choice instead of inferring it. Exact schema fields are owned by [API Judgment Schemas](api/schema-judgment.md); this page owns the product meaning.

| Judgment kind | User owns the decision when the question concerns |
|---|---|
| `product_decision` | User-visible behavior, user flow, copy, UX, accessibility, release promise, product trade-off, or user value. |
| `technical_decision` | Architecture, dependency or external service introduction, authentication direction, migration, public interface, compatibility break, data retention, privacy, security, or another material and costly-to-reverse technical direction. |
| `scope_decision` | Scope expansion, non-goal removal, Change Unit boundary changes, or Autonomy Boundary changes. |
| `sensitive_approval` | Permission for a named sensitive step inside a bounded `SensitiveActionScope`. |
| `final_acceptance` | The user's result judgment when the close path requires acceptance. |
| `residual_risk_acceptance` | The user's acceptance of a named visible residual risk for the requested close. |
| `cancellation` | Stopping the Task without a successful completed result. |

Agent latitude remains narrow: inside accepted scope and acceptance criteria, the agent may choose ordinary implementation details that do not change product behavior, technical direction, scope, security/privacy posture, compatibility, or costly-to-reverse architecture. That latitude is not a new permission system.

Broad consent is also narrow. "Go ahead", "looks good", or similar wording cannot silently satisfy another judgment kind. A single reply may satisfy multiple judgments only when the prompt asked those distinct questions and Core records each compatible judgment with its affected object, scope, consequence, and close or write impact.

## 5. Non-substitution rules

| One thing | Does not substitute for |
|---|---|
| Chat, reports, generated Markdown, projection prose, or status cards | Core-owned state. |
| Evidence, logs, screenshots, artifacts, test output, or Run records | Final acceptance, future Manual QA, future verification, or residual-risk acceptance. |
| `final_acceptance` | Evidence, QA, verification, sensitive-action approval, scope change, residual-risk acceptance, or blocker override. |
| `residual_risk_acceptance` | Verification, evidence sufficiency, QA, final acceptance, or a no-risk result. |
| `sensitive_approval` | Product direction, technical direction, scope, correctness, evidence, QA, final acceptance, residual-risk acceptance, or `Write Authorization`. |
| `Write Authorization` and `AuthorizedAttemptScope` | Command approval, dependency approval, host/network/secret access, deployment approval, destructive-action approval, system access, or final acceptance. |
| `WriteDecisionReason` | A close-readiness blocker or `CloseReadinessBlocker`. |
| `CloseReadinessBlocker` | A prepare-write decision reason, the entire close-readiness concept, evidence, acceptance, or storage effect by itself. |
| Waiver or accepted risk | Automatic success, verification, evidence, final acceptance, or close without the remaining required owner paths. |

Compact user-facing displays may summarize these boundaries, but they must not collapse them.

## 6. Task lifecycle

| Lifecycle area | Core meaning | Required honesty |
|---|---|---|
| Intake and shaping | Turn ordinary user intent into a concrete goal, active scope, non-goals, acceptance criteria, Autonomy Boundary, and next safe action. | If a user-owned issue blocks the next safe action, expose the judgment need instead of guessing. |
| Scope update | Move accepted scope or Change Unit changes through `harness.update_scope`. | `scope_decision` records may support the change, but they do not mutate active scope by themselves. |
| Execution and observation | Run records describe actions or observations. | Product-file writes must be compatible with active scope and `Write Authorization`; read-only work does not authorize later writes. |
| Waiting or blocked | Progress pauses because an owner path is missing, stale, incompatible, or unsafe to bypass. | The blocker should point to the next safe owner path rather than hide the gap. |
| Close attempt | Core evaluates whether the Task can close honestly. | Close readiness is evaluated from current Core state, not from a final chat summary alone. |
| Terminal outcome | Completion, cancellation, or supersession ends the Task path. | Cancellation and supersession are terminal, but they are not successful completion and do not satisfy evidence, acceptance, or risk requirements for completion. |

## 7. Active gates

Gates are compatibility summaries for progress, write, Run recording, and close. This page owns their product meaning. Public fields, exact values, and wire shapes are owned by [API State Schemas](api/schema-state.md) and [API Value Sets](api/schema-value-sets.md).

| Gate area | Meaning | Common confusion to avoid |
|---|---|---|
| Scope gate | Whether active scope and Change Unit cover the requested work. | It does not decide product or technical questions for the user. |
| Decision gate | Whether unresolved user-owned judgment blocks progress, write, or close. | It does not replace evidence, sensitive-action approval, final acceptance, or residual-risk acceptance. |
| Sensitive-action approval gate | Whether a named sensitive step inside `SensitiveActionScope` is approved. | It is not `Write Authorization` and not broad permission. |
| Write-compatibility gate | Whether a product-file write attempt is compatible with active scope and a consumable `Write Authorization`. | It does not approve commands, hosts, network, secrets, deployments, or destructive operations. |
| Evidence gate | Whether close-relevant required support is present and usable enough for the close path. | Evidence does not prove more than recorded and does not replace user acceptance. |
| Acceptance gate | Whether required final acceptance is present for the visible close basis. | It cannot fill evidence gaps or accept residual risk. |
| Residual-risk gate | Whether close-relevant residual risk is visible and, when required, accepted. | Accepted risk is not verification and does not make the result risk-free. |
| Close-readiness gate | Whether all close-relevant checks support an honest close. | A close blocker means the Task remains open until the owner path addresses it. |

Verification and Manual QA are conceptual boundaries in the current MVP, not active gates. They must not be described as active close requirements unless a future owner promotes them.

## 8. Write authorization boundary

`Write Authorization` is the Core record that makes one product-file write attempt compatible with current Harness state. It is created only through the compatible non-dry-run `prepare_write` path defined by the API owner.

| Boundary | Meaning |
|---|---|
| Scope-limited | It covers the intended product-file write attempt, not future work or a broader project area. |
| Single-use | A compatible product-write Run consumes it once. Reuse, replay, and stale-state behavior are API/storage-owned details. |
| Cooperative | It tells a connected agent or surface what is compatible with Harness state; it does not enforce OS-level prevention. |
| Separate from sensitive approval | `sensitive_approval` may be required for a sensitive step, but it is not `Write Authorization`. |
| Separate from close | A valid authorization does not prove the write happened, create evidence, satisfy acceptance, accept risk, or close the Task. |

`WriteDecisionReason` belongs to prepare-write decision output. `CloseReadinessBlocker` belongs to close-readiness blocking data. They answer different questions and must not be interchanged.

## 9. Evidence and run authority

| Record | What it can establish | What it cannot establish |
|---|---|---|
| Run | That an execution or observation was recorded with the available context and refs. | That missing authorization, missing judgment, or missing approval existed retroactively. |
| Evidence summary | That specific close-relevant claims have recorded support, gaps, refs, or coverage expectations. | That unrecorded behavior happened, that the result is accepted, or that risk is accepted. |
| `ArtifactRef` | That an artifact reference is available for evidence use when artifact owners allow it. | That the artifact content is safe, sufficient, or readable beyond the recorded integrity/redaction/availability facts. |
| Projection or report | That a display was generated from available state and refs. | That the display itself is authority, evidence, or acceptance. |

Evidence records must be read at their recorded scope. A passing test log supports the test it names; a screenshot supports the visible state it captures; an artifact supports only the content and integrity facts represented by the artifact owners. Evidence must not be inflated into proof of broader correctness.

## 10. Close readiness

<a id="close_task"></a>

Close readiness is the Core evaluation concept for whether the current Task can close honestly. It considers current Core state, active scope, required user-owned judgments, sensitive-action approval, write/run compatibility, evidence, artifacts, final acceptance, residual risk, and recovery constraints.

`CloseReadinessBlocker` is not the concept itself. It is a state-shaped API data representation for blocking reasons, owned by [API State Schemas](api/schema-state.md) and [API Value Sets](api/schema-value-sets.md). Method behavior, response branches, persistence, and public errors are owned by [MVP API](api/mvp-api.md), [Storage Effects](storage-effects.md), and [API Errors](api/errors.md).

For a complete close attempt, Core evaluates blockers in this conceptual order. Later rows do not satisfy earlier rows.

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
| 14 | Close transition | If no blocker remains, the terminal transition may proceed through the API-owned method behavior; otherwise the Task stays open. |

Close readiness is separate from preflight rejection. Stale state, invalid request identity, local access failure before evaluation, and similar API-owned failures are not semantic close-readiness findings. They are routed through the API and error owners.

## 11. Blockers and waivers

| Concept | Core meaning | Not allowed |
|---|---|---|
| Blocker | A structured reason progress, write, Run recording, or close cannot proceed honestly. | Hiding it in projection prose, broad approval, or a successful-looking close result. |
| Close blocker | A close-relevant reason that prevents honest close readiness. | Treating it as `WriteDecisionReason` or as proof of storage effects by itself. |
| `CloseReadinessBlocker` | The API data representation of close blocking reasons. | Treating it as the whole close-readiness concept or as a prepare-write reason. |
| Waiver | A scoped exception to a named requirement where the responsible owner allows it. | Using a waiver to create scope, sensitive-action approval, required evidence, final acceptance, or residual-risk visibility. |
| Accepted risk | A user judgment that accepts a named visible residual risk for a requested close. | Treating accepted risk as verification, evidence sufficiency, final acceptance, or automatic success. |

A waiver can unblock only the named requirement and only through the owner path that permits it. Decision deferral is not waiver. A future quality-review waiver would not be QA evidence or a QA pass. A future missing-check risk path would not be verification or assurance upgrade.

## 12. Residual risk

Residual risk is known remaining uncertainty, an unchecked condition, limitation, or trade-off that matters to close. Known close-relevant residual risk must be visible before successful close. If close depends on accepting that risk, Core requires compatible `residual_risk_acceptance` tied to the visible risk and related refs.

| Rule | Consequence |
|---|---|
| Visibility comes before acceptance. | The user cannot accept a risk that has not been made visible enough to judge. |
| Acceptance is scoped. | It applies to the named visible risk for the requested close, not to all unknowns. |
| Acceptance is not proof. | It does not verify work, satisfy evidence, satisfy QA, grant sensitive-action approval, create final acceptance, or make the result no-risk. |
| Rich risk workflows are later material. | The current path is compact residual-risk summary, blockers, evidence refs, and `user_judgment` refs unless an owner promotes more. |

## 13. Cross-owner links

| Topic | Owner |
|---|---|
| API method behavior, request/response shapes, envelopes, dry-run/rejection branches, and method effects | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md) |
| State-shaped API data, `ShapingReadiness`, `CloseReadinessBlocker`, `ValidatorResult`, and public state fields | [API State Schemas](api/schema-state.md), [API Value Sets](api/schema-value-sets.md) |
| User judgment schema, `SensitiveActionScope`, and accepted-risk input shapes | [API Judgment Schemas](api/schema-judgment.md) |
| `ArtifactRef`, `ArtifactInput`, `StagedArtifactHandle`, artifact staging, promotion, integrity, redaction, and body-read eligibility | [API Artifact Schemas](api/schema-artifacts.md), [Artifact Storage](storage-artifacts.md) |
| Public error codes, error routing, and error precedence | [API Errors](api/errors.md) |
| Storage records, DDL, persisted JSON layout, locks, migrations, and method-to-storage effects | [Storage Records](storage-records.md), [Storage Effects](storage-effects.md), [Storage Versioning](storage-versioning.md) |
| Projection authority and read-only display boundaries | [Projection Authority Reference](projection-and-templates.md) |
| Status card, judgment request, run/evidence summary, close result, and agent context packet bodies | [Template Bodies](template-bodies.md) |
| Security guarantee wording, cooperative/detective/preventive claims, and local access posture | [Security Reference](security.md) |
| Product Repository, Harness Server, and Harness Runtime Home separation | [Runtime Boundaries Reference](runtime-boundaries.md) |
| Design-quality boundaries and non-gate routing | [Design Quality](design-quality.md) |
| Connector behavior and surface capability posture | [Agent Integration Reference](agent-integration.md) |
| Later candidates and future assurance, waiver, QA, verification, and fixture material | [Later Candidate Index](../later/index.md) |

If another document needs exact schema, DDL, rendered template text, public error codes, or later candidate catalogs, it must link to the owner instead of redefining them here.
