# Later: Assurance Profile

Use this page to route later assurance hardening without pulling it into the MVP implementation path.

This is planning and navigation documentation for future Harness behavior. It is not an MVP-1 requirement and it is not implemented runtime behavior. It does not authorize runtime/server implementation, generated operational files, executable fixtures, runtime data, or product code in this repository.

## Read This When

- You are checking what belongs after MVP-1 User Work Loop.
- You need to keep verification strengthening, Manual QA, detailed evidence, risk review, and detailed evaluation output separate from MVP-1.
- You need the right owner document for an assurance contract.

## Bucket Boundary

Assurance Profile is later than MVP-1. It can harden the user-value loop with stronger assurance behavior, but it is not the first user-value path and not an operations/export/recover profile.

| Assurance bucket | Belongs here | Still out of this profile unless promoted |
|---|---|---|
| Verification strengthening | Detached verification policy, independence display, verification waiver routing, verification gaps, and stronger assurance claims backed by owner records. | Cross-surface verification automation and evaluator orchestration remain Roadmap candidates until promoted. |
| Manual QA | Full Manual QA expectations, QA waiver detail, QA evidence refs, and QA close impact. | Browser QA Capture automation and QA dashboards remain Roadmap candidates until promoted. |
| Detailed evidence | Detailed Evidence Manifest behavior, artifact refs, evidence sufficiency detail, redaction/omission display, and evidence gaps. | Full export bundles and release handoff packaging belong to Operations Profile. |
| Risk review | Rich residual-risk lifecycle, visibility before work acceptance or close, residual-risk acceptance routing, and risk-review summaries. | Team risk workflows, policy dashboards, and hosted review flows remain Roadmap candidates. |
| Detailed evaluation output | Eval result detail, Verification Result Card displays, detailed `EVAL` projection output, and assurance-level explanation when the Eval owner path is active. | Metrics products, analytics, and automation that treat Eval as orchestration remain Roadmap candidates. |

Design-quality, stewardship, TDD trace, feedback-loop, and context-hygiene material belongs here only when it supports one of the assurance buckets above. Broader dashboards, hosted workflows, team workflows, broader connectors, orchestration, preventive security, and isolation remain [Roadmap](../roadmap.md) candidates unless an owner promotes and proves a concrete mechanism.

## Main Path

Start with the MVP boundary in [MVP-1 User Work Loop](../build/mvp-user-work-loop.md), then use only the owner needed for the assurance question:

| Need | Owner |
|---|---|
| Core gates, user judgment, close, waiver, acceptance, and residual-risk meaning | [Core Model Reference](../reference/core-model.md) |
| Later/profile-gated API methods and schema material | [API Schema Later](../reference/api/schema-later.md) |
| Design-quality policies, validator IDs, severity composition, and waiver impact | [Design Quality Policies](../reference/design-quality-policies.md) |
| Fixture mechanics and profile proof model | [Conformance Fixtures Reference](../reference/conformance-fixtures.md) |
| Future assurance scenario candidates | [Future Fixtures](future-fixtures.md) |
| Projection display boundaries for assurance reports | [Projection And Templates Reference](../reference/projection-and-templates.md) and [Template Reference](../reference/templates/README.md) |

## Boundary

Assurance Profile does not create authority by report text. Verification, Manual QA, detailed evidence, risk review, and detailed Eval displays remain separate owner records, refs, or derived views. None substitutes for work acceptance, residual-risk acceptance, close readiness, or Core state.

Listing an item here does not make it an MVP-1 requirement, an implemented runtime behavior, or executable conformance. Future fixture rows stay in [Future Fixtures](future-fixtures.md) until an owner promotes the exact behavior and materializes exact-shape fixtures.
