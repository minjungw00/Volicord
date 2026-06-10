# Design Quality

## 1. Owns / Does not own

This Reference page owns the active current MVP design-quality routing boundary as a judgment-routing and evidence/scope reference: how design-quality observations identify product decisions, technical decisions, scope decisions, evidence gaps, residual-risk visibility issues, or close blockers that are already owned by active Core/API categories.

It does not define an independent active gate, active design-quality `CloseReadinessBlocker.category`, active validator family, design-policy waiver route, severity-based blocking policy, evidence record, QA record, acceptance record, residual-risk record, or close authority.

It owns:

- the active design-quality role as judgment-routing and evidence/scope reference
- how design-quality observations route to `judgment_kind=product_decision`, `judgment_kind=technical_decision`, and `judgment_kind=scope_decision`
- how design-quality observations point to existing active blocker categories such as `scope`, `user_judgment`, `evidence`, `artifact_availability`, `residual_risk_visibility`, or `surface_capability`
- the boundary between design-quality observations, active `ValidatorResult.validator_id` values, and later design-policy catalogs

It does not own:

- Core lifecycle, gates, blockers, `prepare_write`, `close_task`, Write Authorization, final acceptance, residual-risk acceptance, or non-substitution rules; see [Core Model Reference](core-model.md)
- MCP request/response schemas, `ValidatorResult`, public errors, or active/later schema values; see [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), and [API Errors](api/errors.md)
- SQLite DDL and persisted tables; see [Storage Records](storage-records.md)
- validator-run storage effects; see [Storage Effects](storage-effects.md)
- artifact storage; see [Artifact Storage](storage-artifacts.md)
- projection authority; see [Projection Authority Reference](projection-and-templates.md)
- template bodies, status cards, or rendered reports; see [Template Bodies](template-bodies.md)
- broad design-policy validators, design-policy waiver, severity-based active blocking policy, steward policies, full review procedure, operations/reporting candidates, or future conformance catalogs

Documentation in this repository remains planning source material. It does not mean a Harness Server, runtime state, generated evidence, QA record, Acceptance record, residual-risk record, or close record exists here today.

## 2. Active current MVP design-quality role

Active current MVP design quality is a narrow judgment-routing and evidence/scope reference layer. It makes a quality concern legible, then sends the concern to an existing active owner path. It does not create new Core state, `StateSummary.gates.design_gate`, `CloseReadinessBlocker.category=design_policy`, new schemas, new validator result fields, active design-policy validators, design-policy waiver, or a separate design-review authority.

The active role is limited to these effects:

- identify a product behavior, UX, wording, release promise, or user-value choice as `judgment_kind=product_decision`
- identify an architecture, dependency, migration, public-interface, compatibility, security/privacy, or material technical direction choice as `judgment_kind=technical_decision`
- identify scope expansion, non-goal removal, Change Unit boundary, or Autonomy Boundary change as `judgment_kind=scope_decision`
- point to `CloseReadinessBlocker.category=scope`, `CloseReadinessBlocker.category=user_judgment`, `CloseReadinessBlocker.category=evidence`, or `CloseReadinessBlocker.category=artifact_availability` when the matching active owner path already requires that close-readiness finding
- point to `CloseReadinessBlocker.category=residual_risk_visibility`, `CloseReadinessBlocker.category=residual_risk_acceptance`, `CloseReadinessBlocker.category=surface_capability`, or another already-active category only when that owner path truly applies
- route one focused next action: ask one focused user judgment, request evidence, mark residual risk visible, show an advisory next action, or no action
- keep user-owned product, material technical, scope, final-acceptance, residual-risk, and cancellation judgments distinct
- keep evidence, verification, Manual QA, final acceptance, residual-risk visibility, residual-risk acceptance, and close readiness distinct; verification and Manual QA are not active current MVP gates

Design quality must not turn ordinary work into an open-ended planning loop. Full domain-language audits, full module/interface review, full TDD trace, full feedback-loop audit, full codebase-stewardship review, detailed Manual QA policy, detached verification, two-stage review displays, and steward policies are not active current MVP blockers unless another active owner path explicitly requires a narrow piece of that work.

## 3. Routing Rules

A design-quality observation affects current MVP state only through an active owner path. The observation must name the active route it depends on:

| Concern | Active current MVP route |
|---|---|
| Product behavior, UX, wording, release promise, or user value is undecided. | `judgment_kind=product_decision`; use `CloseReadinessBlocker.category=user_judgment` only when the active close path requires that judgment. |
| Architecture, dependency, migration, public interface, compatibility, security/privacy, or material technical direction is undecided. | `judgment_kind=technical_decision`; use `CloseReadinessBlocker.category=user_judgment` only when the active close path requires that judgment. |
| Scope expansion, non-goal removal, Change Unit boundary, or Autonomy Boundary change is needed. | `judgment_kind=scope_decision` or `CloseReadinessBlocker.category=scope`, depending on the owner path. |
| A close-relevant claim lacks support. | `CloseReadinessBlocker.category=evidence`, `CloseReadinessBlocker.category=artifact_availability`, or an evidence request through the Core evidence owner path. |
| A known limitation or unchecked condition matters to close. | Residual-risk visibility through `CloseReadinessBlocker.category=residual_risk_visibility`, and `CloseReadinessBlocker.category=residual_risk_acceptance` only when the active close path requires acceptance. |
| The connected surface cannot honestly support the claimed operation or guarantee. | `CloseReadinessBlocker.category=surface_capability`, `CAPABILITY_INSUFFICIENT`, or a lower guarantee display through the capability owner path. |

A design-quality label, policy name, severity value, validator ID, or review phrase does not create the route. If no active owner path applies, the current MVP result is advisory text or no action.

<a id="when-a-finding-blocks-close"></a>
## 4. When a finding blocks close

A design-quality observation blocks close only when all of these are true:

- it is tied to the active Task or Change Unit and the attempted close
- it names an existing active `CloseReadinessBlocker.category`, `judgment_kind`, API error, or owner path from the active close-blocking set
- the named owner path would block close even if no design-quality label existed
- it gives exactly one next action that can unblock, defer through the owning path, request the required evidence, or mark residual risk visible
- it does not rely on `design_gate`, `CloseReadinessBlocker.category=design_policy`, a design-policy waiver, a broad policy catalog, or severity alone

A finding does not block close merely because it mentions domain language, vertical slice shape, TDD, module/interface review, stewardship, Manual QA, detached verification, review stages, or a future policy family. Those may produce an advisory next action, an evidence request, a focused user judgment, or a residual-risk marker only when an active owner path needs that narrow action.

When a design-quality observation affects close, the close-readiness finding must use an active `CloseReadinessBlocker.category` value owned by [API Value Sets](api/schema-value-sets.md).

## 5. No Current Design-Policy Waiver

The current MVP has no active design-quality waiver or design-policy waiver route. If an owner path allows a requirement to be deferred, accepted as risk, or resolved by user judgment, use that active owner path and its exact `judgment_kind`, blocker category, or evidence behavior.

Keep the judgment routes separate:

- `final_acceptance` is the user's result judgment after the close basis is visible; it does not create evidence or accept residual risk.
- `residual_risk_acceptance` accepts a named visible residual risk; it does not prove correctness or replace final acceptance.
- Active current MVP `UserJudgment.judgment_kind` values are owned by [API Value Sets](api/schema-value-sets.md). Other future candidates stay in [Later](../later/index.md) until promoted.

Broad approval, a friendly "looks good", or a general go-ahead must not be treated as any of these judgments unless the active owner path asked for that specific judgment.

## 6. Evidence expectation

Design-quality observations may identify evidence gaps, but required evidence belongs to the Core evidence owner path. A finding should ask for evidence only when that active owner path needs support for a claim that affects write safety, close readiness, user judgment, residual risk, or guarantee honesty.

Useful evidence references can include:

- persisted `ArtifactRef` values, Run refs, command/check summaries, or source refs
- current state/version/freshness refs when stale context affects the close basis
- user-judgment refs for product, technical, scope, final-acceptance, or residual-risk decisions
- residual-risk refs when a known limitation remains visible at close
- future Manual QA or verification refs only after those later owner paths are promoted

Chat assertions, generic summaries, rendered projection prose, unregistered files, screenshots without an owner path, passing tests alone, future waiver candidates, final acceptance, or residual-risk acceptance do not automatically satisfy required evidence. Required evidence can block close only through the Core evidence owner path. Non-required evidence gaps should be routed as `request evidence`, `show advisory next action`, or residual-risk visibility as appropriate.

## 7. Validator ID boundary

Validator IDs are reporting labels. They do not create Core invariants, gates, close blockers, waivers, evidence records, or user judgments.

`ValidatorResult` shape is owned by [API State Schemas](api/schema-state.md). Severity-like values and the active stable `ValidatorResult.validator_id` set are owned by [API Value Sets](api/schema-value-sets.md).

This document does not publish active design-policy validator IDs or a policy-to-validator mapping. Later stable validator ID sets remain candidates in [Later Candidate Index: Later Schema Candidates](../later/index.md#later-schema-candidates) unless an owner promotes a narrow active contract.

## 8. Later policy catalog boundary

The full design-quality policy catalog is not active current MVP scope. Broad design-policy validators, design-policy waiver, severity-based active blocking policy, richer policy families, steward policies, detailed review displays, operations/reporting candidates, full validator mappings, and future conformance fixtures stay in [Later Candidate Index](../later/index.md) until a named owner promotes a narrow behavior with scope, fallback behavior, exact contracts, and proof expectations.

Later candidates may keep names only. They must not be presented as active current MVP requirements, blockers, waiver rules, evidence expectations, validator mappings, fixture requirements, operations reports, or implementation tasks.
