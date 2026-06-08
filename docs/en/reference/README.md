# Reference Index

Use Reference when you need the owner document for an exact Harness planning contract. It is an index for future Harness Server review, not a first-read tutorial and not the implementation plan.

These documents describe future Harness Server contracts under current documentation review. They do not mean a server/runtime, Harness Runtime Home, generated projection system, conformance runner, runtime data, or implementation-complete behavior exists in this repository today.

## Reading Rules

- Do not load all Reference docs by default. Pick the one owner document for the question in front of you, then follow links only when that owner delegates a stricter detail.
- Do not load English and Korean paired docs for the same owner in the same prompt. Choose the working language for the task, and keep bilingual comparison to a separate, targeted check.
- Keep this README as an index. Do not copy contract details here.
- Keep the active/later boundary with the active owner documents and [Later Candidate Index](../later/index.md).

## Active MVP Boundary

The current active MVP is closed to ordinary-language intake and Task creation, `harness.update_scope`, user judgment recording, sensitive approval recording, path-level `harness.prepare_write` and Write Authorization, `harness.record_run`, staged artifact registration through `stage_artifact`, compact `EvidenceSummary`, `harness.close_task` blocker calculation, read-time status/projection, registered local surface access, cooperative guarantee display, and detective guarantee display only after the relevant capability check has actually passed.

Everything else is outside the active MVP unless the owning Reference document explicitly promotes it with scope, fallback behavior, and proof expectations. That includes `captured_artifact`, native artifact capture, projection reconcile, persistent projection jobs, managed block drift repair, Full Evidence Manifest, QA gate, verification gate, command execution observation, network observation, secret access observation, command/network/secret pre-tool blocking, Question Queue, Assumption Register, and Discovery Brief as a persistent artifact.

## Owner Routing

The table routes agents and implementers to the compact owner documents that currently exist.

| Contract area | Owner document |
|---|---|
| Core authority, task lifecycle, user judgment boundaries, final/residual-risk non-substitution, gates, close blockers | [core-model.md](core-model.md) |
| Method-level behavior for active public API methods, including `harness.update_scope` scope updates and `harness.prepare_write` authorization effects | [api/mvp-api.md](api/mvp-api.md) |
| Exact active method-name set, shared schema, envelope, active enum/value sets, rendered-label boundaries, and `GuaranteeDisplay.level` values | [api/schema-core.md](api/schema-core.md) |
| Public errors and precedence | [api/errors.md](api/errors.md) |
| Storage, DDL, persisted rows such as `write_authorizations`, and idempotency | [storage.md](storage.md) |
| Runtime spaces, mutation authority, and non-isolation / OS-sandboxing non-claims | [runtime-boundaries.md](runtime-boundaries.md) |
| Security guarantees, OS-sandboxing non-claims, and profile-gated `preventive` / `isolated` labels | [security.md](security.md) |
| Agent context, connector behavior, surface capability, and one-language-per-`doc_id` retrieval | [agent-integration.md](agent-integration.md) |
| Projections/status cards as derived display, rendered labels, active templates | [projection-and-templates.md](projection-and-templates.md) |
| Conformance model, future fixture shape, assertion authority, and non-executable suite boundary | [conformance.md](conformance.md) |
| Narrow design-quality routing, close impact, waiver boundary, and validator ID boundary | [design-quality.md](design-quality.md) |
| Official terms | [glossary.md](glossary.md) |
| Later candidates, including full-format judgment presentation and future fixture families | [../later/index.md](../later/index.md) |

## No Duplicate Injection

Non-owner docs may summarize the reader-visible consequence and link to the owner. They should not paste schemas, DDL, enum tables, transition tables, template bodies, fixture assertions, public error precedence, security guarantees, or glossary definitions.

Documentation authoring, translation, review, link hygiene, owner-boundary drift, and docs-maintenance checks belong to [Authoring Guide](../maintain/authoring-guide.md), [Translation Guide](../maintain/translation-guide.md), and [Checks](../maintain/checks.md). Implementation sequencing and maintainer status decisions belong to [MVP Plan](../build/mvp-plan.md).
