# Design quality

## 1. Owns / Does not own

This Reference page owns the baseline design-quality routing boundary.

Role: design-quality observations route to judgment, evidence, or scope owners when they identify:

- product decisions
- technical decisions
- scope decisions
- evidence gaps
- residual-risk visibility issues
- close blockers already owned by active Core/API categories

It does not define an independent active gate, active design-quality `CloseReadinessBlocker.category`, active validator family, design-policy waiver route, severity-based blocking policy, evidence record, QA record, acceptance record, residual-risk record, or close authority.

It owns:

- the active design-quality role as judgment-routing and evidence/scope reference
- how design-quality observations route to `judgment_kind=product_decision`, `judgment_kind=technical_decision`, and `judgment_kind=scope_decision`
- how design-quality observations point to existing active blocker categories such as `scope`, `user_judgment`, `evidence`, `artifact_availability`, `residual_risk_visibility`, or `surface_capability`
- the active design-quality severity boundary: severity-like wording is advisory triage unless an active owner path separately requires action
- the boundary between design-quality observations, active `ValidatorResult.validator_id` values, and out-of-scope design-policy catalogs

It does not own:

- Core lifecycle, gates, blockers, `prepare_write`, `close_task`, Write Authorization, final acceptance, residual-risk acceptance, or non-substitution rules; see [Core Model Reference](core-model.md)
- MCP request/response schemas, `ValidatorResult`, `UserJudgment`, `AcceptedRiskInput`, public errors, or active/out-of-scope schema values; see the [API Methods](api/methods.md), method owner documents, [API Schema Core](api/schema-core.md), [API Judgment Schemas](api/schema-judgment.md), and [API Errors](api/errors.md)
- SQLite DDL and persisted tables; see [Storage Records](storage-records.md)
- validator-run storage effects; see [Storage Effects](storage-effects.md)
- artifact storage; see [Artifact Storage](storage-artifacts.md)
- projection authority; see [Projection Authority Reference](projection-and-templates.md)
- template bodies, status cards, or rendered reports; see [Template Bodies](template-bodies.md)
- broad design-policy validators, design-policy waiver, severity-based active blocking policy, steward policies, full review procedure, operations/reporting candidates, or out-of-scope conformance catalogs

Use these owner links when a design-quality finding crosses another contract:

| Question | Owner |
|---|---|
| Core non-substitution, close readiness, waiver, accepted-risk, and residual-risk meaning | [Core Model Reference](core-model.md) |
| `UserJudgment`, `RecordUserJudgmentPayload`, `SensitiveActionScope`, and `AcceptedRiskInput` shapes | [API Judgment Schemas](api/schema-judgment.md) |
| User-judgment method behavior | [User-judgment methods](api/method-user-judgment.md) |
| Status method behavior | [Status method](api/method-status.md) |
| Close-task method behavior | [Close-task method](api/method-close-task.md) |
| Method-to-storage effects for active API method branches | [Storage Effects](storage-effects.md) |
| Deferred design gates, policy blockers, broad validators, waiver candidates, and policy catalogs | [Scope Reference](scope.md) |

Documentation in this repository remains planning reference material. It does not mean a Harness Server, runtime state, generated evidence, QA record, Acceptance record, residual-risk record, or close record exists here today.

## 2. Baseline design-quality role

Baseline design quality is a narrow judgment-routing and evidence/scope reference layer. It makes a quality concern legible, then sends the concern to an existing active owner path.

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
| no active owner path | See [No active owner path](#design-quality-no-active-owner-path) |

<a id="design-quality-product-decision-needed"></a>
### Product decision needed

Condition:
- The concern is a product behavior, UX, wording, release promise, or user-value choice that needs the user.

Route:
- Use `judgment_kind=product_decision`.

Close effect:
- Blocks close only when the active close path already requires `CloseReadinessBlocker.category=user_judgment`.

Not allowed:
- Do not treat the design-quality label as an independent close blocker.

<a id="design-quality-technical-decision-needed"></a>
### Technical decision needed

Condition:
- The concern is architecture, dependency, migration, public-interface, compatibility, security/privacy, or another material technical direction choice that needs the user.

Route:
- Use `judgment_kind=technical_decision`.

Close effect:
- Blocks close only when the active close path already requires `CloseReadinessBlocker.category=user_judgment`.

Not allowed:
- Do not treat the design-quality label as an independent close blocker.

<a id="design-quality-scope-boundary-change"></a>
### Scope boundary change

Condition:
- The concern is scope expansion, non-goal removal, Change Unit boundary change, or Autonomy Boundary change.

Route:
- Use `judgment_kind=scope_decision` or `CloseReadinessBlocker.category=scope`, depending on the owner path.

Close effect:
- Blocks close only through the active scope or judgment owner path.

Not allowed:
- Do not treat the design-quality label as a scope override.

<a id="design-quality-missing-close-relevant-support"></a>
### Missing close-relevant support

Condition:
- A close-relevant claim lacks support.

Route:
- Request evidence through the Core evidence owner path.
- Use `CloseReadinessBlocker.category=evidence` or `CloseReadinessBlocker.category=artifact_availability` only through that owner path.

Close effect:
- Required evidence can block close only through the Core evidence owner path.

Not allowed:
- Do not create a design-quality evidence rule outside that owner path.

<a id="design-quality-residual-risk-visibility"></a>
### Residual risk visibility

Condition:
- A known limitation, unchecked condition, or trade-off matters to close.

Route:
- Use residual-risk visibility.
- Use `CloseReadinessBlocker.category=residual_risk_acceptance` only when the active close path requires acceptance.

Close effect:
- Affects close only through the residual-risk visibility or residual-risk acceptance owner path.

Not allowed:
- Accepted risk does not prove success or erase the risk.

<a id="design-quality-surface-capability-gap"></a>
### Surface capability gap

Condition:
- The connected surface cannot honestly support the claimed operation or guarantee.

Route:
- Use `CloseReadinessBlocker.category=surface_capability`, `CAPABILITY_INSUFFICIENT`, or a lower guarantee display through the capability owner path.

Close effect:
- Affects close only through the capability owner path.

Not allowed:
- The design-quality label does not strengthen the guarantee.

<a id="design-quality-advisory-severity"></a>
### Advisory severity

Condition:
- The finding describes relative urgency or attention for the concern.

Route:
- Treat severity-like wording as advisory triage unless an active owner path separately requires action.

Close effect:
- Severity-like wording has no close effect by itself.

Not allowed:
- Severity alone never creates a blocker, validator mapping, waiver, evidence expectation, or close result.

<a id="design-quality-focused-next-action"></a>
### Focused next action

Condition:
- One narrow action can unblock or clarify the named owner path.

Route:
- Ask one focused user judgment, request evidence, mark residual risk visible, show an advisory next action, or take no action.

Close effect:
- Can affect close only when the named owner path uses that action.

Not allowed:
- The action must not widen beyond what the named owner path needs.

<a id="design-quality-no-active-owner-path"></a>
### No active owner path

Condition:
- No active owner path applies.

Close effect:
- The baseline scope result is advisory text or no action.

Not allowed:
- Do not create a new gate, blocker, validator mapping, waiver route, evidence rule, or close authority.

Active design quality does not create:

- new Core state
- `StateSummary.gates.design_gate`
- `CloseReadinessBlocker.category=design_policy`
- new schemas
- new validator result fields
- active design-policy validators
- design-policy waiver
- separate design-review authority

Design quality must not turn ordinary work into an open-ended planning loop.

Not supported blockers unless another active owner path explicitly requires a narrow piece:
- full domain-language audits
- full module/interface review
- full TDD trace
- full feedback-loop audit
- full codebase-stewardship review
- detailed Manual QA policy
- detached verification
- two-stage review displays
- steward policies

## 3. Routing rules

A design-quality observation affects baseline scope state only through an active owner path. The observation must name the active route it depends on:

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
- Use `CloseReadinessBlocker.category=user_judgment` only when the active close path requires that judgment.

Close effect:
- Blocks close only when the active owner path requires that user decision.

<a id="design-quality-route-technical-direction"></a>
### Undecided technical direction

Condition:
- Architecture, dependency, migration, public interface, compatibility, security/privacy, or material technical direction is undecided.

Route:
- Use `judgment_kind=technical_decision`.
- Use `CloseReadinessBlocker.category=user_judgment` only when the active close path requires that judgment.

Close effect:
- Blocks close only when the active owner path requires that user decision.

<a id="design-quality-route-scope-boundary"></a>
### Scope boundary route

Condition:
- Scope expansion, non-goal removal, Change Unit boundary, or Autonomy Boundary change is needed.

Route:
- Use `judgment_kind=scope_decision` or `CloseReadinessBlocker.category=scope`, depending on the owner path.

Close effect:
- Blocks close only through the active scope or judgment owner path.

<a id="design-quality-route-evidence"></a>
### Evidence route

Condition:
- A close-relevant claim lacks support.

Route:
- Use `CloseReadinessBlocker.category=evidence`, `CloseReadinessBlocker.category=artifact_availability`, or an evidence request through the Core evidence owner path.

Close effect:
- Required evidence can block close only through the Core evidence owner path.

<a id="design-quality-route-residual-risk"></a>
### Residual-risk route

Condition:
- A known limitation or unchecked condition matters to close.

Route:
- Use residual-risk visibility through `CloseReadinessBlocker.category=residual_risk_visibility`.
- Use `CloseReadinessBlocker.category=residual_risk_acceptance` only when the active close path requires acceptance.

Close effect:
- Affects close only through the active residual-risk owner path.

<a id="design-quality-route-surface-capability"></a>
### Surface capability route

Condition:
- The connected surface cannot honestly support the claimed operation or guarantee.

Route:
- Use `CloseReadinessBlocker.category=surface_capability`, `CAPABILITY_INSUFFICIENT`, or a lower guarantee display through the capability owner path.

Close effect:
- Affects close only through the active capability owner path.

A design-quality label, policy name, severity value, validator ID, or review phrase does not create the route. If no active owner path applies, the baseline scope result is advisory text or no action.

<a id="when-a-finding-blocks-close"></a>
## 4. When a finding blocks close

A design-quality observation blocks close only through an active owner path.

| Close-blocking question | Details |
|---|---|
| active close dependency | See [Active close dependency](#design-quality-close-active-dependency) |
| focused unblock path | See [Focused unblock path](#design-quality-close-focused-unblock-path) |
| inactive design-policy basis | See [Inactive design-policy basis](#design-quality-close-inactive-design-policy-basis) |
| advisory-only policy phrase | See [Advisory-only policy phrase](#design-quality-close-advisory-only-policy-phrase) |
| active close category | See [Active close category](#design-quality-close-active-category) |

<a id="design-quality-close-active-dependency"></a>
### Active close dependency

Condition:
- The observation is tied to the active Task or Change Unit and the attempted close.
- The observation names an existing active `CloseReadinessBlocker.category`, `judgment_kind`, API error, or owner path from the active close-blocking set.

Close effect:
- The observation can block close only when the named owner path would block close without the design-quality label.

Not allowed:
- Do not treat a design-quality label as independent close authority.

<a id="design-quality-close-focused-unblock-path"></a>
### Focused unblock path

Condition:
- One named owner path can be unblocked, deferred through that owner path, supported with required evidence, or marked as visible residual risk.

Close effect:
- Can affect close only by giving exactly one next action for that owner path.

Not allowed:
- Do not widen the next action into a broad design review or open-ended planning loop.

<a id="design-quality-close-inactive-design-policy-basis"></a>
### Inactive design-policy basis

Condition:
- The observation relies on `design_gate`, `CloseReadinessBlocker.category=design_policy`, a design-policy waiver, a broad policy catalog, or severity alone.

Close effect:
- The observation does not block close on that basis.

Not allowed:
- Do not treat out-of-scope design-policy material as an active gate, close blocker, waiver route, evidence rule, or close authority.

<a id="design-quality-close-advisory-only-policy-phrase"></a>
### Advisory-only policy phrase

Condition:
- The finding mentions domain language, vertical slice shape, TDD, module/interface review, stewardship, Manual QA, detached verification, review stages, or an out-of-scope policy family.

Route:
- Use an advisory next action, evidence request, focused user judgment, or residual-risk marker only when an active owner path needs that narrow action.

Close effect:
- The finding does not block close merely because it mentions one of those topics.

Not allowed:
- Do not present an out-of-scope policy family as a baseline requirement.

<a id="design-quality-close-active-category"></a>
### Active close category

Condition:
- A design-quality observation affects close.

Route:
- Use an active `CloseReadinessBlocker.category` value owned by [API Value Sets](api/schema-value-sets.md).

Close effect:
- The close-readiness finding remains in the active category owned by that close path.

Not allowed:
- Do not create a design-quality-specific close category in the baseline scope.

## 5. No current design-policy waiver

The baseline scope has no active design-quality waiver or design-policy waiver route. If an owner path allows a requirement to be deferred, accepted as risk, or resolved by user judgment, use that active owner path and its exact `judgment_kind`, blocker category, or evidence behavior.

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
| active `UserJudgment.judgment_kind` values | See [Active user judgment values](#design-quality-route-active-user-judgment-values) |
| out-of-scope design-quality waiver candidates | See [Out-of-scope waiver candidates](#design-quality-route-future-waiver-candidates) |

<a id="design-quality-route-final-acceptance"></a>
### `final_acceptance`

Condition:
- The close basis is visible and the active owner path asks for the user's result judgment.

Effect:
- Records the user's result judgment after the close basis is visible.

Close effect:
- Does not override close blockers by itself.

Not allowed:
- Do not treat it as evidence creation, residual-risk acceptance, QA, verification, or blocker override.

<a id="design-quality-route-residual-risk-acceptance"></a>
### `residual_risk_acceptance`

Condition:
- A named visible residual risk remains for the requested close.

Effect:
- Records the user's acceptance of a named visible residual risk for the requested close.

Close effect:
- Affects close only through the active residual-risk owner path.

Not allowed:
- Do not treat it as correctness proof, evidence sufficiency, final acceptance, no-risk result, or automatic success.

<a id="design-quality-route-active-user-judgment-values"></a>
### Active user judgment values

Condition:
- A focused user-owned decision is required.

Effect:
- Records focused user-owned decisions.

Owner links:
- Values are owned by [API Value Sets](api/schema-value-sets.md).

Close effect:
- They can affect close only through the active owner path that asked for the judgment.

Not allowed:
- Do not treat them as design-policy waiver, broad approval, QA waiver, verification-risk acceptance, or any out-of-scope candidate that has not been promoted.

<a id="design-quality-route-future-waiver-candidates"></a>
### Out-of-Scope Waiver Candidates

Condition:
- A proposed design-quality waiver or policy waiver is not promoted into the baseline.

Effect:
- Remain out-of-scope material.

Owner links:
- [Scope](scope.md)

Close effect:
- They have no active close effect.

Not allowed:
- Do not treat them as active requirements, close blockers, validator behavior, or evidence rules.

Broad approval, a friendly "looks good", or a general go-ahead must not be treated as any of these judgments unless the active owner path asked for that specific judgment.

## 6. Evidence expectation

Design-quality observations may identify evidence gaps, but required evidence belongs to the Core evidence owner path.

| Evidence question | Details |
|---|---|
| evidence gap that may be requested | See [Evidence gap that may be requested](#design-quality-evidence-gap-request) |
| useful evidence references | See [Useful evidence references](#design-quality-useful-evidence-references) |
| references that do not automatically satisfy evidence | See [References that do not automatically satisfy evidence](#design-quality-evidence-non-satisfying-references) |
| non-required evidence gaps | See [Non-required evidence gaps](#design-quality-non-required-evidence-gaps) |

<a id="design-quality-evidence-gap-request"></a>
### Evidence gap that may be requested

Condition:
- The active owner path needs support for a claim that affects write safety, close readiness, user judgment, residual risk, or guarantee honesty.

Route:
- Ask for evidence through the Core evidence owner path.

Close effect:
- Required evidence can block close only through the Core evidence owner path.

<a id="design-quality-useful-evidence-references"></a>
### Useful evidence references

Allowed examples:

- persisted `ArtifactRef` values, Run refs, command/check summaries, or source refs
- current state/version/freshness refs when stale context affects the close basis
- user-judgment refs for product, technical, scope, final-acceptance, or residual-risk decisions
- residual-risk refs when a known limitation remains visible at close
- future Manual QA or verification refs only after those later owner paths are promoted

<a id="design-quality-evidence-non-satisfying-references"></a>
### References that do not automatically satisfy evidence

Not allowed:
- Do not automatically treat chat assertions, generic summaries, rendered projection prose, unregistered files, screenshots without an owner path, passing tests alone, future waiver candidates, final acceptance, or residual-risk acceptance as required evidence.

Close effect:
- These references do not remove a required-evidence blocker by themselves.

<a id="design-quality-non-required-evidence-gaps"></a>
### Non-required evidence gaps

Condition:
- The evidence gap is not required by the Core evidence owner path.

Route:
- Use `request evidence`, `show advisory next action`, or residual-risk visibility as appropriate.

Close effect:
- The gap does not block close as required evidence.

## 7. Validator ID boundary

Validator IDs are reporting labels. They do not create Core invariants, gates, close blockers, waivers, evidence records, or user judgments.

`ValidatorResult` shape is owned by [API State Schemas](api/schema-state.md). Severity-like values and the active stable `ValidatorResult.validator_id` set are owned by [API Value Sets](api/schema-value-sets.md).

This document does not publish:

- active design-policy validator IDs
- a policy-to-validator mapping

Out-of-scope stable validator ID sets remain candidates in [Policy and conformance: `ValidatorResult` stable IDs and policy families](scope.md) unless an owner promotes a narrow active contract.

## 8. Out-of-Scope Policy Catalog Boundary

The full design-quality policy catalog is not baseline scope. These ideas are out of scope until a named owner promotes a narrow behavior with scope, fallback behavior, exact contracts, and proof expectations.

| Out-of-scope idea | Details |
|---|---|
| `design_gate` and `CloseReadinessBlocker.category=design_policy` | See [`design_gate`](#design-quality-later-design-gate) |
| Design-policy waiver | See [Design-policy waiver](#design-quality-later-design-policy-waiver) |
| Broad design validators and severity-based blocking | See [Broad validators](#design-quality-later-broad-validators) |
| Full design-quality policy families and steward policies | See [Policy families](#design-quality-later-policy-families) |
| Detailed review displays and reporting material | See [Detailed review displays](#design-quality-later-detailed-review-displays) |

<a id="design-quality-later-design-gate"></a>
### `design_gate`

Not allowed:
- No active gate, active close blocker, or close-readiness category exists for `design_gate` or `CloseReadinessBlocker.category=design_policy`.

Promotion would need:
- Core/API owner changes plus value-set, schema, close-readiness, and storage-effect ownership.

<a id="design-quality-later-design-policy-waiver"></a>
### Design-policy waiver

Not allowed:
- No active waiver route or automatic success path exists.

Promotion would need:
- A named owner path, non-substitution rules, judgment behavior, and close-readiness effects.

<a id="design-quality-later-broad-validators"></a>
### Broad validators

Not allowed:
- No active validator IDs, severity meanings, policy-to-validator mapping, or severity-only blocker exists.

Promotion would need:
- Stable validator set ownership, severity semantics, API/schema boundaries, and fallback behavior.

<a id="design-quality-later-policy-families"></a>
### Policy families

Not allowed:
- No active policy catalog, stewardship gate, or full review procedure exists.

Promotion would need:
- A scoped policy owner, reader-facing behavior, proof expectations, and active/out-of-scope migration path.

<a id="design-quality-later-detailed-review-displays"></a>
### Detailed review displays

Not allowed:
- No active operations report, fixture requirement, implementation task, or conformance obligation exists.

Promotion would need:
- Promotion through [Scope Reference](scope.md), promotion-time owner updates, and documentation acceptance before implementation work starts.

Out-of-scope capabilities may keep names only. They must not be presented as baseline requirements, blockers, waiver rules, evidence expectations, validator mappings, fixture requirements, operations reports, or implementation tasks.
