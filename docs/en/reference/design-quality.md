# Design Quality

<a id="owns-does-not-own"></a>
## 1. Owns / Does not own

This Reference page owns the active current MVP design-quality boundary: how design-quality findings may affect gates, close blockers, waivers, and evidence expectations without becoming Core invariants.

It owns:

- the active design-quality role for current MVP close behavior
- finding severity interpretation where it affects a visible blocker or next action
- when a design-quality finding may become a Core-backed close blocker
- the waiver boundary for design-quality expectations
- evidence expectations for design-quality findings
- the boundary between validator IDs, active close impact, and later policy catalogs

It does not own:

- Core lifecycle, gates, blockers, `prepare_write`, `close_task`, Write Authorization, final acceptance, residual-risk acceptance, or non-substitution rules; see [Core Model Reference](core-model.md)
- MCP request/response schemas, `ValidatorResult`, public errors, or active/later schema values; see [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), and [API Errors](api/errors.md)
- SQLite DDL, persisted tables, validator-run storage, or artifact storage; see [Storage](storage.md)
- projection template bodies, status cards, or rendered reports; see [Projection And Templates Reference](projection-and-templates.md)
- full policy-to-validator mapping, steward profiles, full review-stage procedure, operations/reporting material, or future conformance catalogs

Documentation in this repository remains planning source material. It does not mean a Harness Server, runtime state, generated evidence, QA record, Acceptance record, residual-risk record, or close record exists here today.

<a id="active-current-mvp-design-quality-role"></a>
## 2. Active current MVP design-quality role

Active current MVP design quality is a narrow routing layer. It makes close-relevant quality findings visible, then routes each finding through an existing owner path. It does not create new Core state, new gates, new schemas, new validator result fields, or a separate design-review authority.

The active role is limited to these effects:

- flag a finding that affects scope, user-owned judgment, required evidence, stale close/write context, surface capability, guarantee honesty, or visible residual risk
- route one focused next action: `ask one focused user judgment`, `request evidence`, `mark residual risk`, `show advisory next action`, or `no action`
- route `block write` or `block close` only when the Core owner path already supports that blocker
- keep user-owned product, material technical, QA waiver, verification-risk, final-acceptance, and residual-risk judgments distinct
- keep evidence, verification, Manual QA, final acceptance, residual-risk visibility, residual-risk acceptance, and close readiness distinct

Design quality must not turn ordinary work into an open-ended planning loop. Full domain-language audits, full module/interface review, full TDD trace, full feedback-loop audit, full codebase-stewardship review, detailed Manual QA policy, detached verification, two-stage review displays, and steward profiles are not active current MVP blockers unless another active owner path explicitly requires a narrow piece of that work.

<a id="finding-severity"></a>
## 3. Finding severity

`ValidatorResult.findings.severity` is owned by [API Schema Core](api/schema-core.md#validatorresult). Design Quality interprets severity only for the visible next action and possible close impact.

| Severity | Active current MVP meaning |
|---|---|
| `info` | Useful context. It does not block write or close. |
| `warning` | A concern the agent should show or route to one bounded next action. It does not block write or close by itself. |
| `error` | A quality expectation is unmet. It may request evidence, ask one focused user judgment, mark residual risk, or show an advisory next action. It blocks close only when [When a finding blocks close](#when-a-finding-blocks-close) applies. |
| `blocker` | A claimed blocker must name the active Core-backed blocker, gate, or API error path. Without that owner path, it must not be presented as a close blocker. |

For the same affected concern, show the strongest valid active action and keep weaker findings visible. Separate concerns stay separate. A later catalog warning must not inherit blocker status from a different Core-backed concern.

<a id="when-a-finding-blocks-close"></a>
## 4. When a finding blocks close

A design-quality finding blocks close only when all of these are true:

- it is tied to the active Task or Change Unit and the attempted close
- it names an existing Core-backed close blocker, gate, API error, or owner path
- it gives exactly one next action that can unblock, defer, waive where allowed, or mark residual risk
- it fits one of the active current MVP blocker conditions below

Active current MVP blocker conditions:

| Condition | Owner path |
|---|---|
| Required user-owned judgment is unresolved. | `decision_gate`, `user_judgment`, and Core close semantics. |
| Active scope is missing, incompatible, or exceeded for the close-relevant work. | Scope Gate, Change Unit, Autonomy Boundary, `prepare_write`, and close blockers. |
| Required evidence is missing, unavailable, stale, or blocked. | Core evidence summary, artifact availability, and `EVIDENCE_INSUFFICIENT` paths. |
| Stale context makes the close basis unsafe. | Core freshness, projection/source refs when used for the visible close basis, and reconcile/recovery owner paths. |
| The surface cannot support the claimed operation or guarantee. | Capability boundary, `CAPABILITY_INSUFFICIENT`, and honest guarantee display owners. |

A finding does not block close merely because it mentions domain language, vertical slice shape, TDD, module/interface review, stewardship, Manual QA, detached verification, review stages, or a future policy family. Those may produce an advisory next action, an evidence request, a focused user judgment, or a residual-risk marker only when an active owner path needs that narrow action.

<a id="waiver-boundary"></a>
## 5. Waiver boundary

A design-quality waiver can affect only a design-quality expectation that the active owner path allows to be waived. It must be explicit, scoped to the affected Task/Change Unit or finding, and recorded through the relevant user-judgment or owner path when the decision belongs to the user.

A design-quality waiver does not waive:

- missing active scope or incompatible Write Authorization
- sensitive-action approval
- required evidence coverage or artifact availability
- required final acceptance
- required residual-risk visibility or residual-risk acceptance
- verification independence
- Core-owned close blockers

Keep the judgment routes separate:

- `qa_waiver` waives a scoped QA requirement only where the QA owner path allows it; it is not QA evidence or a passed QA result.
- `verification_risk_acceptance` accepts the risk of missing or waived verification; it does not create detached verification.
- `final_acceptance` is the user's result judgment after the close basis is visible; it does not create evidence or accept residual risk.
- `residual_risk_acceptance` accepts a named visible residual risk; it does not prove correctness or replace final acceptance.

Broad approval, a friendly "looks good", or a general go-ahead must not be treated as any of these judgments unless the active owner path asked for that specific judgment.

<a id="evidence-expectation"></a>
## 6. Evidence expectation

Design-quality evidence expectations are narrow and close-relevant. A finding should ask for evidence only when the active owner path needs support for a claim that affects write safety, close readiness, user judgment, residual risk, or guarantee honesty.

Useful evidence references can include:

- registered `ArtifactRef` values, Run refs, command/check summaries, or source refs
- current state/version/freshness refs when stale context affects the close basis
- user-judgment refs for product, technical, scope, QA waiver, verification-risk, final-acceptance, or residual-risk decisions
- residual-risk refs when a known limitation remains visible at close
- Manual QA or verification refs only when those owner paths are active or explicitly required

Chat assertions, generic summaries, rendered projection prose, unregistered files, screenshots without an owner path, passing tests alone, QA waiver, final acceptance, or residual-risk acceptance do not automatically satisfy required evidence. Required evidence can block close only through the Core evidence owner path. Non-required evidence gaps should be routed as `request evidence`, `show advisory next action`, or residual-risk visibility as appropriate.

<a id="validator-id-boundary"></a>
## 7. Validator ID boundary

Validator IDs are reporting labels. They do not create Core invariants, gates, close blockers, waivers, evidence records, or user judgments.

`ValidatorResult` shape and severity values are owned by [API Schema Core](api/schema-core.md#validatorresult). Later stable validator ID sets remain candidates in [Later Candidate Index: Later Schema Candidates](../later/index.md#later-schema-candidates) unless an owner promotes a narrow active contract.

This document does not publish a full policy-to-validator mapping. If a current or future validator result reports a design-quality finding, close impact still comes from [When a finding blocks close](#when-a-finding-blocks-close) and the relevant Core/API owner path, not from the validator ID alone.

<a id="later-policy-catalog-boundary"></a>
## 8. Later policy catalog boundary

The full design-quality policy catalog is not active current MVP scope. Future policy families, steward profiles, detailed review displays, operations/reporting material, full validator mappings, and future conformance fixtures stay in [Later Candidate Index](../later/index.md) until a named owner promotes a narrow behavior with scope, fallback behavior, exact contracts, and proof expectations.

Later candidates may keep names only. They must not be presented as active current MVP requirements, blockers, waiver rules, evidence expectations, validator mappings, fixture requirements, operations reports, or implementation tasks.
