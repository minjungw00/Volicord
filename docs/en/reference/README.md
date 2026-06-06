# Reference Index

Use Reference when you need the owner document for an exact Harness planning contract. It is an index for future Harness Server review, not a first-read tutorial and not the implementation plan.

These documents describe future Harness Server contracts under post-redesign review. They do not mean a server/runtime, Harness Runtime Home, generated projection system, conformance runner, runtime data, or implementation-complete behavior exists in this repository today.

## Reading Rules

- Do not load all Reference docs by default. Pick the one owner document for the question in front of you, then follow links only when that owner delegates a stricter detail.
- Do not load English and Korean paired docs for the same owner in the same prompt. Choose the working language for the task, and keep bilingual comparison to a separate, targeted check.
- Keep this README as an index. Do not copy contract details here.
- Keep the active/later boundary with the active owner documents and [Later Candidate Index](../later/index.md).

## Owner Routing

The table routes agents and implementers to the compact owner documents that currently exist.

| Contract area | Owner document |
|---|---|
| Core authority, task lifecycle, user judgment boundaries, gates, close blockers | [core-model.md](core-model.md) |
| Active public API methods | [api/mvp-api.md](api/mvp-api.md) |
| Shared schema, envelope, active value sets | [api/schema-core.md](api/schema-core.md) |
| Public errors and precedence | [api/errors.md](api/errors.md) |
| Storage and DDL | [storage.md](storage.md) |
| Runtime spaces and process boundaries | [runtime-boundaries.md](runtime-boundaries.md) |
| Security guarantees and non-claims | [security.md](security.md) |
| Agent context, connector behavior, surface capability | [agent-integration.md](agent-integration.md) |
| Projections, rendered views, active templates | [projection-and-templates.md](projection-and-templates.md) |
| Conformance model and representative fixture shape | [conformance.md](conformance.md) |
| Design-quality rules that affect active gates | [design-quality.md](design-quality.md) |
| Official terms | [glossary.md](glossary.md) |
| Later candidates | [../later/index.md](../later/index.md) |

## No Duplicate Injection

Non-owner docs may summarize the reader-visible consequence and link to the owner. They should not paste schemas, DDL, enum tables, transition tables, template bodies, fixture assertions, public error precedence, security guarantees, or glossary definitions.

Documentation authoring, translation, review, link hygiene, owner-boundary drift, and docs-maintenance checks belong to [Authoring Guide](../maintain/authoring-guide.md), [Translation Guide](../maintain/translation-guide.md), and [Checks](../maintain/checks.md). Implementation sequencing and maintainer status decisions belong to [MVP Plan](../build/mvp-plan.md).
