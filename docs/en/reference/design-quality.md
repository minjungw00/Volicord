# Design quality

<a id="1-owns--does-not-own"></a>
## 1. Boundary

This reference owns the baseline design-quality owner boundary. A
design-quality finding makes a concern legible and routes it to an existing
judgment, evidence, scope, residual-risk, connection-capability, or
close-readiness owner. It does not create independent authority.

This document owns:

- the baseline role of design-quality findings
- routes from findings to supported judgment kinds, blocker categories, and
  evidence or scope owners
- the advisory boundary for severity-like wording
- the boundary between a finding, a supported
  `ValidatorResult.validator_id`, and out-of-scope quality policy

Neighboring contracts stay with their owners:

| Question | Owner |
|---|---|
| Core non-substitution, close readiness, waiver, accepted-risk, and residual-risk meaning | [Core Model Reference](core-model.md) |
| Judgment shapes and values | [API Judgment Schemas](api/schema-judgment.md) and [API Value Sets](api/schema-value-sets.md) |
| Blocker shapes and category values | [API State Schemas](api/schema-state.md) and [API Value Sets](api/schema-value-sets.md) |
| User-owned judgment request and record behavior | [Request-user-action method](api/method-request-user-action.md) and [Resolve-user-action method](api/method-resolve-user-action.md) |
| Status and close behavior | [Status method](api/method-status.md) and [Close-task method](api/method-close-task.md) |
| Agent Connection capability and public capability errors | [Agent Connection](agent-connection.md) and [API error codes](api/error-codes.md) |
| Method-to-storage effects | [Storage Effects](storage-effects.md) |
| Out-of-scope design-quality policy families | [Scope Reference](scope.md) |

This document does not define product acceptance, final acceptance,
residual-risk acceptance, close authority, independent quality gates,
quality-waiver routes, severity-based blocking policy, API behavior, storage
effects, schema fields, validator families, evidence authority, QA results,
conformance catalogs, projections, reports, or template bodies.

A finding also does not create Volicord state, user-owned judgment, a write
ticket, sensitive-action approval, evidence, final acceptance, residual-risk
acceptance, or close-readiness state.

<a id="2-baseline-design-quality-role"></a>
<a id="3-routing-rules"></a>
## 2. Route a finding

A finding has a baseline product effect only when the relevant owner defines
that effect. Use the narrowest applicable route below.

| Concern | Owner-defined route | Close effect |
|---|---|---|
| <a id="design-quality-product-decision-needed"></a><a id="design-quality-route-product-direction"></a>Product behavior, UX, wording, a release promise, or user value needs a decision. | Use a choice `UserActionDraft` with `judgment_kind=product_decision`. | Use `CloseReadinessBlocker.category=user_action` only when the applicable close-readiness contract requires that action. |
| <a id="design-quality-technical-decision-needed"></a><a id="design-quality-route-technical-direction"></a>Architecture, a dependency, migration, public interface, compatibility, security/privacy, or another material technical direction needs a decision. | Use a choice `UserActionDraft` with `judgment_kind=technical_decision`. | Use `CloseReadinessBlocker.category=user_action` only when the applicable close-readiness contract requires that action. |
| <a id="design-quality-scope-boundary-change"></a><a id="design-quality-route-scope-boundary"></a>Scope expansion, non-goal removal, a Change Unit boundary, or an Autonomy Boundary must change. | Use `judgment_kind=scope_decision` or `CloseReadinessBlocker.category=scope`, as defined by the affected scope or judgment contract. | The route affects close only when that contract defines the dependency. |
| <a id="design-quality-missing-close-relevant-support"></a><a id="design-quality-route-evidence"></a>A close-relevant claim lacks required support. | Request evidence through the Core evidence authority. Use `CloseReadinessBlocker.category=evidence_claim`, `CloseReadinessBlocker.category=evidence_provenance`, or `CloseReadinessBlocker.category=artifact_availability` only where the evidence and close-readiness owners allow them. | Missing evidence blocks close only when those owners require it. |
| <a id="design-quality-residual-risk-visibility"></a><a id="design-quality-route-residual-risk"></a>A known limitation, unchecked condition, or trade-off matters to close. | Make the risk visible. Use `CloseReadinessBlocker.category=residual_risk_visibility`, or `CloseReadinessBlocker.category=residual_risk_acceptance` when the applicable owner requires acceptance. | The risk affects close only through the applicable residual-risk contract. |
| <a id="design-quality-connection-capability-gap"></a><a id="design-quality-route-connection-capability"></a>The Agent Connection cannot support the claimed operation or guarantee. | Use `CloseReadinessBlocker.category=connection_capability`, `CAPABILITY_INSUFFICIENT`, or a lower guarantee display through the capability and API error owners. | The gap affects close only when those owners define the effect. |
| <a id="design-quality-advisory-severity"></a>The finding describes relative urgency. | Treat severity-like wording as advisory triage unless an owner separately requires action. | Severity has no close effect by itself. |
| <a id="design-quality-focused-next-action"></a>One narrow action can resolve or clarify an owner-defined requirement. | Ask one focused user judgment, request evidence, make residual risk visible, show an advisory next action, or take no action. | The action affects close only when its owner makes it close-relevant. |
| <a id="design-quality-no-applicable-owner-path"></a>The required owner is absent, unclear, or too broad to define a product effect. | Name the gap and link the closest owner. Do not fill the gap with design-quality prose. | The result is advisory text or no action. The gap does not block close by itself. |

These rules apply to every route:

- A finding does not become an independent close blocker, acceptance gate,
  scope override, evidence rule, or guarantee.
- A finding does not replace user-owned judgment, a write ticket,
  sensitive-action approval, evidence, final acceptance, or residual-risk
  acceptance.
- A next action stays within the applicable owner contract. Documentation
  convenience cannot expand it.
- A policy label, severity value, validator ID, or review phrase does not create
  a route.
- Design-quality review must not turn ordinary work into an open-ended planning
  loop.

<a id="when-a-finding-blocks-close"></a>
<a id="4-close-dependency-boundary"></a>
## 3. Close dependency boundary

Design quality has no separate blocking mechanism. A finding affects close only
through a dependency defined by a close-readiness, scope, judgment, evidence,
capability, or method owner.

- <a id="design-quality-close-applicable-dependency"></a>**Applicable
  dependency:** the finding is tied to the current `Task` or Change Unit and
  names a supported blocker category, judgment kind, API error, or other close
  dependency. Only that dependency can block close.
- <a id="design-quality-close-focused-unblock-path"></a>**Focused unblock
  path:** show the one next action required by the relevant owner. This may
  resolve, owner-permitted defer, supply required evidence, or make residual
  risk visible.
- <a id="design-quality-close-unsupported-policy-basis"></a>**Unsupported
  policy basis:** out-of-scope policy or severity alone does not block close.
- <a id="design-quality-close-advisory-only-policy-phrase"></a>**Advisory-only
  policy phrase:** merely naming an out-of-scope quality-policy family has no
  close effect.
- <a id="design-quality-close-supported-category"></a>**Supported category:**
  if the finding affects close, use a supported
  `CloseReadinessBlocker.category` owned by
  [API Value Sets](api/schema-value-sets.md).

<a id="5-no-separate-quality-waiver"></a>
## 4. No separate quality waiver

The baseline has no quality-waiver route. If an owner allows a requirement to
be deferred, accepted as risk, or resolved by user judgment, use that owner's
exact judgment kind, blocker category, or evidence behavior.

A waiver-like decision does not erase facts, remove a limitation from the close
basis, create evidence, prove verification, pass QA, replace final acceptance,
or make close succeed automatically.

| Route | Meaning and boundary |
|---|---|
| <a id="design-quality-route-final-acceptance"></a>`final_acceptance` | Records the user's result judgment after the close basis is visible. It is not evidence, residual-risk acceptance, QA, verification, or a blocker override. |
| <a id="design-quality-route-residual-risk-acceptance"></a>`residual_risk_acceptance` | Records acceptance of one named visible risk for the requested close. It affects close only through the residual-risk owner and is not proof of correctness, evidence sufficiency, final acceptance, or a no-risk result. |
| <a id="design-quality-route-supported-user-action-values"></a>Supported choice `UserActionDraft.judgment_kind` values | Request focused user-owned decisions. [API Value Sets](api/schema-value-sets.md) owns the values. Broad approval counts only when the relevant contract asked the specific question. |

<a id="6-evidence-routing-boundary"></a>
## 5. Evidence boundary

A finding may identify an evidence gap, but it does not create an evidence
requirement.

| Question | Boundary |
|---|---|
| <a id="design-quality-evidence-gap-request"></a>When may evidence be requested? | When an applicable owner requires support for a claim that affects write safety, close readiness, user judgment, residual risk, or guarantee honesty. Ask through the Core evidence authority. |
| <a id="design-quality-useful-evidence-references"></a>Which references may be useful? | Persisted `ArtifactRef` values, Run refs, command or check summaries, source refs, current state/version/freshness refs, user-judgment refs, and residual-risk refs, when their owners make them relevant. |
| <a id="design-quality-evidence-non-satisfying-references"></a>What does not satisfy evidence automatically? | Chat claims, general summaries, rendered projection text, unregistered files, screenshots without a recorded owner relation, test-pass status alone, final acceptance, and residual-risk acceptance. |
| <a id="design-quality-non-required-evidence-gaps"></a>What if the evidence is not required? | Show an advisory next action, request optional support, or make residual risk visible as appropriate. The gap does not block close as required evidence. |

<a id="7-validator-id-boundary"></a>
## 6. Validator ID boundary

Validator IDs are reporting labels. They do not create Core invariants, product
gates, close blockers, waivers, evidence records, user judgments, write tickets,
final acceptance, or residual-risk acceptance.

[API State Schemas](api/schema-state.md) owns `ValidatorResult` shape.
[API Value Sets](api/schema-value-sets.md) owns severity-like values and the
boundary for any supported stable `ValidatorResult.validator_id` value. This
document publishes neither design-policy validator IDs nor policy-to-validator
mappings.

Other validator IDs have no baseline effect unless [Scope](scope.md) and the
affected owners define a narrow supported contract.

<a id="8-out-of-scope-policy-material"></a>
## 7. Out-of-scope policy material

Design-quality policy beyond this owner-routing boundary is outside the
baseline. This page does not publish unsupported gate names, blocker
categories, waiver branches, validator families, workflow branches, or
promotion checklists.

Do not present out-of-scope quality material as a baseline requirement,
blocker, waiver rule, evidence requirement, verification criterion, validator
mapping, conformance scenario, operations report, or implementation task. Use
[Scope](scope.md) for category-level exclusions.
