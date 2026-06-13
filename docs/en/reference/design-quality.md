# Design quality

## 1. Owns / does not own

This reference owns the baseline design-quality routing boundary. The boundary makes quality observations legible as routes to existing judgment, evidence, scope, residual-risk, surface-capability, or close-readiness owners.

Design-quality observations route to judgment, evidence, or scope owners when they identify:

- product decisions
- technical decisions
- scope decisions
- evidence gaps
- residual-risk visibility issues
- close blockers already owned by supported Core/API categories

It owns:

- the baseline design-quality role as judgment-routing and evidence/scope reference
- how design-quality observations route to `judgment_kind=product_decision`, `judgment_kind=technical_decision`, and `judgment_kind=scope_decision`
- how design-quality observations point to existing supported blocker categories such as `scope`, `user_judgment`, `evidence`, `artifact_availability`, `residual_risk_visibility`, or `surface_capability`
- the baseline design-quality severity boundary: severity-like wording is advisory triage unless the affected owner separately requires action
- the boundary between design-quality observations, any supported `ValidatorResult.validator_id` value, and out-of-scope quality-policy material

Neighboring contracts stay with their owners:

| Question | Owner |
|---|---|
| Core non-substitution, close readiness, waiver, accepted-risk, and residual-risk meaning | [Core Model Reference](core-model.md) |
| `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, and `AcceptedRiskInput` shapes | [API Judgment Schemas](api/schema-judgment.md) |
| User-judgment method behavior | [User-judgment methods](api/method-user-judgment.md) |
| Status method behavior | [Status method](api/method-status.md) |
| Close-task method behavior | [Close-task method](api/method-close-task.md) |
| Method-to-storage effects for supported API method branches | [Storage Effects](storage-effects.md) |
| Out-of-scope design-quality policy families | [Scope Reference](scope.md) |

This reference does not define an independent baseline gate, supported design-quality close category, supported validator family, quality-waiver route, severity-based blocking policy, evidence authority, QA result, acceptance decision, residual-risk decision, close authority, operations report, conformance catalog, SQLite DDL, persisted table, projection authority, rendered report, or template body.

Reference text documents the design-quality boundary and owner routing. It does not create Harness Server state, evidence, QA, acceptance, residual-risk decisions, or close-readiness state.

## 2. Baseline design-quality role

Baseline design quality is a narrow judgment-routing and evidence/scope reference layer. It makes a quality concern legible, then routes the concern to the affected owner.

A design-quality finding can do only these things in the baseline:

| Finding type | Details |
|---|---|
| product decision needed | See [Product decision needed](#design-quality-product-decision-needed) |
| technical decision needed | See [Technical decision needed](#design-quality-technical-decision-needed) |
| scope boundary change | See [Scope boundary change](#design-quality-scope-boundary-change) |
| missing close-relevant support | See [Missing close-relevant support](#design-quality-missing-close-relevant-support) |
| residual risk visibility | See [Residual risk visibility](#design-quality-residual-risk-visibility) |
| surface capability gap | See [Surface capability gap](#design-quality-surface-capability-gap) |
| advisory severity | See [Advisory severity](#design-quality-advisory-severity) |
| focused next action | See [Focused next action](#design-quality-focused-next-action) |
| no affected owner | See [No affected owner](#design-quality-no-applicable-owner-path) |

Baseline owner-boundary rules:

| Boundary | Rule |
|---|---|
| Independent close authority | A design-quality label does not create a close blocker, close category, scope override, evidence rule, or guarantee. |
| Evidence and risk | Evidence, final acceptance, residual-risk visibility, and residual-risk acceptance keep their Core/API owner meanings. |
| Severity | Severity-like wording is advisory unless the affected owner separately requires action. |
| Focused action | A next action must stay limited to what the affected owner contract requires. |
| Owner gap | When no affected owner applies, the baseline result is advisory text or no action. |

<a id="design-quality-product-decision-needed"></a>
### Product decision needed

Condition:
- The concern is a product behavior, UX, wording, release promise, or user-value choice that needs the user.

Route:
- Use `judgment_kind=product_decision`.

Close effect:
- Blocks close only when the applicable close-readiness contract already requires `CloseReadinessBlocker.category=user_judgment`.

<a id="design-quality-technical-decision-needed"></a>
### Technical decision needed

Condition:
- The concern is architecture, dependency, migration, public-interface, compatibility, security/privacy, or another material technical direction choice that needs the user.

Route:
- Use `judgment_kind=technical_decision`.

Close effect:
- Blocks close only when the applicable close-readiness contract already requires `CloseReadinessBlocker.category=user_judgment`.

<a id="design-quality-scope-boundary-change"></a>
### Scope boundary change

Condition:
- The concern is scope expansion, non-goal removal, Change Unit boundary change, or Autonomy Boundary change.

Route:
- Use `judgment_kind=scope_decision` or `CloseReadinessBlocker.category=scope`, depending on the affected scope or judgment contract.

Close effect:
- Blocks close only when the scope or judgment contract defines that blocker.

<a id="design-quality-missing-close-relevant-support"></a>
### Missing close-relevant support

Condition:
- A close-relevant claim lacks support.

Route:
- Request evidence through Core evidence rules.
- Use `CloseReadinessBlocker.category=evidence` or `CloseReadinessBlocker.category=artifact_availability` only when the evidence and close-readiness contracts allow that category.

Close effect:
- Required evidence can block close only when Core evidence rules and close-readiness contracts require it.

<a id="design-quality-residual-risk-visibility"></a>
### Residual risk visibility

Condition:
- A known limitation, unchecked condition, or trade-off matters to close.

Route:
- Use residual-risk visibility.
- Use `CloseReadinessBlocker.category=residual_risk_acceptance` only when the applicable close-readiness contract requires acceptance.

Close effect:
- Affects close only when the residual-risk visibility or residual-risk acceptance contract defines the dependency.

<a id="design-quality-surface-capability-gap"></a>
### Surface capability gap

Condition:
- The connected surface cannot honestly support the claimed operation or guarantee.

Route:
- Use `CloseReadinessBlocker.category=surface_capability`, `CAPABILITY_INSUFFICIENT`, or a lower guarantee display through the relevant capability and API error contracts.

Close effect:
- Affects close only when the relevant capability or API error contract defines the effect.

<a id="design-quality-advisory-severity"></a>
### Advisory severity

Condition:
- The finding describes relative urgency or attention for the concern.

Route:
- Treat severity-like wording as advisory triage unless the affected owner separately requires action.

Close effect:
- Severity-like wording has no close effect by itself.

<a id="design-quality-focused-next-action"></a>
### Focused next action

Condition:
- One narrow action can unblock or clarify the affected owner contract.

Route:
- Ask one focused user judgment, request evidence, mark residual risk visible, show an advisory next action, or take no action.

Close effect:
- Can affect close only when that affected contract uses the action.

<a id="design-quality-no-applicable-owner-path"></a>
### No affected owner

Condition:
- No affected owner applies.

Close effect:
- The baseline scope result is advisory text or no action.

Baseline design quality does not create:

- new Core state or schemas
- new validator result fields
- supported policy validators
- quality-waiver routes
- separate design-review authority

Design quality must not turn ordinary work into an open-ended planning loop.

Quality-policy material outside baseline scope can be advisory only, unless another affected owner explicitly requires a narrow action.

## 3. Routing rules

A design-quality observation affects baseline scope state only when a focused owner defines that effect. The observation must name the route it depends on:

| Concern | Details |
|---|---|
| undecided product direction | See [Undecided product direction](#design-quality-route-product-direction) |
| undecided technical direction | See [Undecided technical direction](#design-quality-route-technical-direction) |
| scope boundary change needed | See [Scope boundary route](#design-quality-route-scope-boundary) |
| close-relevant support missing | See [Evidence route](#design-quality-route-evidence) |
| residual risk matters to close | See [Residual-risk route](#design-quality-route-residual-risk) |
| surface capability cannot support claim | See [Surface capability route](#design-quality-route-surface-capability) |

<a id="design-quality-route-product-direction"></a>
### Undecided product direction

Condition:
- Product behavior, UX, wording, release promise, or user value is undecided.

Route:
- Use `judgment_kind=product_decision`.
- Use `CloseReadinessBlocker.category=user_judgment` only when the applicable close-readiness contract requires that judgment.

Close effect:
- Blocks close only when the affected owner requires that user decision.

<a id="design-quality-route-technical-direction"></a>
### Undecided technical direction

Condition:
- Architecture, dependency, migration, public interface, compatibility, security/privacy, or material technical direction is undecided.

Route:
- Use `judgment_kind=technical_decision`.
- Use `CloseReadinessBlocker.category=user_judgment` only when the applicable close-readiness contract requires that judgment.

Close effect:
- Blocks close only when the affected owner requires that user decision.

<a id="design-quality-route-scope-boundary"></a>
### Scope boundary route

Condition:
- Scope expansion, non-goal removal, Change Unit boundary, or Autonomy Boundary change is needed.

Route:
- Use `judgment_kind=scope_decision` or `CloseReadinessBlocker.category=scope`, depending on the affected scope or judgment contract.

Close effect:
- Blocks close only when the scope or judgment contract defines that blocker.

<a id="design-quality-route-evidence"></a>
### Evidence route

Condition:
- A close-relevant claim lacks support.

Route:
- Use `CloseReadinessBlocker.category=evidence`, `CloseReadinessBlocker.category=artifact_availability`, or an evidence request through Core evidence rules.

Close effect:
- Required evidence can block close only when Core evidence rules and close-readiness contracts require it.

<a id="design-quality-route-residual-risk"></a>
### Residual-risk route

Condition:
- A known limitation or unchecked condition matters to close.

Route:
- Use residual-risk visibility through `CloseReadinessBlocker.category=residual_risk_visibility`.
- Use `CloseReadinessBlocker.category=residual_risk_acceptance` only when the applicable close-readiness contract requires acceptance.

Close effect:
- Affects close only when the applicable residual-risk contract defines the dependency.

<a id="design-quality-route-surface-capability"></a>
### Surface capability route

Condition:
- The connected surface cannot honestly support the claimed operation or guarantee.

Route:
- Use `CloseReadinessBlocker.category=surface_capability`, `CAPABILITY_INSUFFICIENT`, or a lower guarantee display through the relevant capability and API error contracts.

Close effect:
- Affects close only when the applicable capability or API error contract defines the effect.

A design-quality label, policy name, severity value, validator ID, or review phrase does not create the route. If no affected owner applies, the baseline scope result is advisory text or no action.

<a id="when-a-finding-blocks-close"></a>
## 4. When a finding blocks close

A design-quality observation blocks close only when a focused owner defines a close-relevant blocker or judgment.

| Close-blocking question | Details |
|---|---|
| applicable close dependency | See [Applicable close dependency](#design-quality-close-applicable-dependency) |
| focused unblock path | See [Focused unblock path](#design-quality-close-focused-unblock-path) |
| unsupported policy basis | See [Unsupported policy basis](#design-quality-close-unsupported-policy-basis) |
| advisory-only policy phrase | See [Advisory-only policy phrase](#design-quality-close-advisory-only-policy-phrase) |
| supported close category | See [Supported close category](#design-quality-close-supported-category) |

<a id="design-quality-close-applicable-dependency"></a>
### Applicable close dependency

Condition:
- The observation is tied to the active `Task` or Change Unit and the attempted close.
- The observation names an existing supported `CloseReadinessBlocker.category`, supported `judgment_kind`, supported API error, or another affected owner in the close-blocking set.

Close effect:
- The observation can block close only when the named close dependency would block close without the design-quality label.

<a id="design-quality-close-focused-unblock-path"></a>
### Focused unblock path

Condition:
- One affected contract can be unblocked, deferred under that contract, supported with required evidence, or marked as visible residual risk.

Close effect:
- Can affect close only by giving exactly one next action for that affected contract.

<a id="design-quality-close-unsupported-policy-basis"></a>
### Unsupported policy basis

Condition:
- The observation relies on a quality-policy route outside baseline scope, broad policy material, or severity alone.

Close effect:
- The observation does not block close on that basis.

<a id="design-quality-close-advisory-only-policy-phrase"></a>
### Advisory-only policy phrase

Condition:
- The finding mentions a quality-policy family outside baseline scope.

Route:
- Use an advisory next action, evidence request, focused user judgment, or residual-risk marker only when an affected owner needs that narrow action.

Close effect:
- The finding does not block close merely because it mentions one of those topics.

<a id="design-quality-close-supported-category"></a>
### Supported close category

Condition:
- A design-quality observation affects close.

Route:
- Use a supported `CloseReadinessBlocker.category` value owned by [API Value Sets](api/schema-value-sets.md).

Close effect:
- The close-readiness finding remains in the supported category defined by the applicable close-readiness contract.

## 5. No separate quality waiver

The baseline scope has no separate quality-waiver route. If an affected owner allows a requirement to be deferred, accepted as risk, or resolved by user judgment, use that owner's exact `judgment_kind`, blocker category, or evidence behavior.

A waiver-like decision or accepted-risk answer records the responsible user judgment about a named requirement or a named visible risk.

It does not:
- erase the facts
- remove the underlying limitation from the close basis
- create evidence
- prove verification
- pass QA
- replace final acceptance
- automatically make close successful

Keep the judgment routes separate:

| Route | Details |
|---|---|
| `final_acceptance` | See [`final_acceptance`](#design-quality-route-final-acceptance) |
| `residual_risk_acceptance` | See [`residual_risk_acceptance`](#design-quality-route-residual-risk-acceptance) |
| supported `UserJudgment.judgment_kind` values | See [Supported user judgment values](#design-quality-route-supported-user-judgment-values) |

<a id="design-quality-route-final-acceptance"></a>
### `final_acceptance`

Condition:
- The close basis is visible and the affected owner asks for the user's result judgment.

Effect:
- Records the user's result judgment after the close basis is visible.

Close effect:
- Does not override close blockers by itself.

Boundary:
- `final_acceptance` is not evidence creation, residual-risk acceptance, QA, verification, or blocker override.

<a id="design-quality-route-residual-risk-acceptance"></a>
### `residual_risk_acceptance`

Condition:
- A named visible residual risk remains for the requested close.

Effect:
- Records the user's acceptance of a named visible residual risk for the requested close.

Close effect:
- Affects close only when the applicable residual-risk contract defines the dependency.

Boundary:
- `residual_risk_acceptance` is not correctness proof, evidence sufficiency, final acceptance, a no-risk result, or automatic success.

<a id="design-quality-route-supported-user-judgment-values"></a>
### Supported user judgment values

Condition:
- A focused user-owned decision is required.

Effect:
- Records focused user-owned decisions.

Owner links:
- Values are owned by [API Value Sets](api/schema-value-sets.md).

Close effect:
- They can affect close only through the owner that asked for the judgment.

Boundary:
- Supported user judgment values are not broad approval, a separate quality waiver, or unsupported judgment categories.
- Broad approval, a friendly "looks good", or a general go-ahead is not one of these judgments unless the affected owner asks for that specific judgment.

## 6. Evidence expectation

Design-quality observations may identify evidence gaps, but required evidence belongs to Core evidence rules.

| Evidence question | Details |
|---|---|
| evidence gap that may be requested | See [Evidence gap that may be requested](#design-quality-evidence-gap-request) |
| useful evidence references | See [Useful evidence references](#design-quality-useful-evidence-references) |
| references that do not automatically satisfy evidence | See [References that do not automatically satisfy evidence](#design-quality-evidence-non-satisfying-references) |
| non-required evidence gaps | See [Non-required evidence gaps](#design-quality-non-required-evidence-gaps) |

<a id="design-quality-evidence-gap-request"></a>
### Evidence gap that may be requested

Condition:
- The affected owner needs support for a claim that affects write safety, close readiness, user judgment, residual risk, or guarantee honesty.

Route:
- Ask for evidence through Core evidence rules.

Close effect:
- Required evidence can block close only when Core evidence rules and close-readiness contracts require it.

<a id="design-quality-useful-evidence-references"></a>
### Useful evidence references

Allowed examples:

- persisted `ArtifactRef` values, Run refs, command/check summaries, or source refs
- current state/version/freshness refs when stale context affects the close basis
- user-judgment refs for product, technical, scope, final-acceptance, or residual-risk decisions
- residual-risk refs when a known limitation remains visible at close

<a id="design-quality-evidence-non-satisfying-references"></a>
### References that do not automatically satisfy evidence

Boundary:
- Chat claims, general summaries, rendered projection text, unregistered files, screenshots without a recorded owner relation, test-pass status by itself, final acceptance, or residual-risk acceptance do not automatically satisfy required evidence.

Close effect:
- These references do not remove a required-evidence blocker by themselves.

<a id="design-quality-non-required-evidence-gaps"></a>
### Non-required evidence gaps

Condition:
- The evidence gap is not required by Core evidence rules.

Route:
- Use `request evidence`, `show advisory next action`, or residual-risk visibility as appropriate.

Close effect:
- The gap does not block close as required evidence.

## 7. Validator ID boundary

Validator IDs are reporting labels. They do not create Core invariants, gates, close blockers, waivers, evidence records, or user judgments.

`ValidatorResult` shape is owned by [API State Schemas](api/schema-state.md). Severity-like values and the boundary for any supported stable `ValidatorResult.validator_id` value are owned by [API Value Sets](api/schema-value-sets.md).

This document does not publish:

- supported design-policy validator IDs
- a policy-to-validator mapping

Validator IDs outside a supported value published by [API Value Sets](api/schema-value-sets.md) have no baseline effect unless [Scope](scope.md) and the affected owners define a narrow supported contract.

## 8. Out-of-scope policy material

Design-quality policy material beyond this routing boundary is not baseline scope.

This page does not publish unsupported gate names, blocker categories, waiver branches, validator families, workflow branches, or promotion checklists. Use [Scope](scope.md) for category-level baseline exclusions.

Out-of-scope quality material must not be presented as baseline requirements, blockers, waiver rules, evidence expectations, validator mappings, conformance scenario requirements, operations reports, or implementation tasks.
