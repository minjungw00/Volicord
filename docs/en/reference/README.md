# Reference Index

Use Reference when you need the owner document for an exact Harness planning contract. It is an index for future Harness Server review, not a first-read tutorial and not the implementation plan.

These documents describe future Harness Server contracts under current documentation review. They do not mean a server/runtime, Harness Runtime Home, generated projection system, conformance runner, runtime data, or implementation-complete behavior exists in this repository today.

## Reading Rules

- Do not load all Reference docs by default. Pick the one owner document for the question in front of you, then follow links only when that owner delegates a stricter detail.
- Do not load English and Korean paired docs for the same owner in the same prompt. Choose the working language for the task, and keep bilingual comparison to a separate, targeted check.
- Keep this README as an index. Do not copy contract details here.
- Keep the active/later boundary with the active owner documents and [Later Candidate Index](../later/index.md).

## Active MVP Boundary

The current active MVP is closed in [MVP Plan](../build/mvp-plan.md). It includes ordinary-language intake and Task creation, `harness.update_scope`, user judgment recording, sensitive approval recording, path-level `harness.prepare_write` and Write Authorization, `harness.record_run`, staged artifact registration through `harness.stage_artifact`, compact `EvidenceSummary`, `harness.close_task` blocker calculation, read-time read-only status/projection output, verified local surface access through a registered surface, cooperative guarantee display, and detective guarantee display only after the relevant capability check has actually passed.

Everything else is later-only unless the owning Reference document explicitly promotes it with scope, fallback behavior, and proof expectations. That includes `captured_artifact`, native artifact capture, projection reconcile, persistent projection jobs, managed block drift repair, full Evidence Manifest, `qa_gate`, `verification_gate`, command execution observation, network observation, secret access observation, command/network/secret pre-tool blocking, Question Queue, Assumption Register, and Discovery Brief as a persistent artifact. Use [Later Candidate Index](../later/index.md) for later-only names and promotion boundaries.

## Owner Routing

The table routes agents and implementers to the compact owner documents that currently exist.

| Contract area | Owner document |
|---|---|
| Active MVP boundary, excluded later material, implementation sequencing, and maintainer readiness decisions | [MVP Plan](../build/mvp-plan.md) |
| Core authority, task lifecycle, `ShapingReadiness` meaning, user-owned product/technical/scope/sensitive/final/residual-risk/cancellation judgment boundaries, final/residual-risk non-substitution, active gate meaning, `CompletionPolicy` close effect, `EvidenceSummary` close effect, `close_task` blocker matrix, waivers, and residual risk | [core-model.md](core-model.md) |
| Method-level behavior for active public API methods, verified local surface request conditions, `harness.update_scope`, `harness.prepare_write` authorization effects, `harness.stage_artifact`, `harness.record_run`, and `harness.close_task` method behavior | [api/mvp-api.md](api/mvp-api.md) |
| Exact active method-name set, `ToolEnvelope.expected_state_version`, `LocalSurfaceRegistration`, `VerifiedSurfaceContext`, `StagedArtifactHandle`, `ArtifactInput`, `CompletionPolicy`, `EvidenceSummary`, `SensitiveActionScope`, product-file `AuthorizedAttemptScope`, close blocker schemas, `ShapingReadiness` fields, active enum/value sets, rendered-label boundaries, and `GuaranteeDisplay.level` values | [api/schema-core.md](api/schema-core.md) |
| Public errors, error precedence, local surface errors, `STATE_VERSION_CONFLICT`, blocked/dry-run response semantics, and public error mapping for `close_task` blockers | [api/errors.md](api/errors.md) |
| Storage, DDL, `project_state.state_version` as the single public project-wide state clock, `surfaces`, `write_authorizations`, staged artifact storage, persisted evidence-summary rows, idempotency, and migrations | [storage.md](storage.md) |
| Runtime spaces, mutation authority, Product Repository / Harness Server / Runtime Home separation, and non-isolation / OS-sandboxing non-claims | [runtime-boundaries.md](runtime-boundaries.md) |
| Security guarantees, cooperative/detective wording, capability-backed detective gating, OS-sandboxing non-claims, sensitive-action permission versus product-file write scope, and profile-gated `preventive` / `isolated` labels | [security.md](security.md) |
| Agent context, connector behavior, `capability_profile`, verified surface context in agent packets, detective display gating from capability checks, fallback semantics, surface recipes, and one-language-per-`doc_id` retrieval | [agent-integration.md](agent-integration.md) |
| Read-only projections/status cards as derived display, projection authority boundaries, rendered labels, active templates, freshness wording, and the boundary that projection reconcile, persistent projection jobs, and managed block drift repair are later-only | [projection-and-templates.md](projection-and-templates.md) |
| Conformance model, future fixture shape, assertion authority, active smoke-target examples, capability honesty assertions, and non-executable suite boundary | [conformance.md](conformance.md) |
| Narrow design-quality routing, close impact, waiver boundary, and validator ID boundary | [design-quality.md](design-quality.md) |
| Official terms | [glossary.md](glossary.md) |
| Later-only concepts and promotion boundaries, including `captured_artifact`, native artifact capture, projection reconcile, persistent projection jobs, managed block drift repair, full Evidence Manifest, `qa_gate`, `verification_gate`, full-format judgment presentation, future fixture families, and future operations | [../later/index.md](../later/index.md) |
| Documentation authoring rules, owner-boundary hygiene, active/later checks, Korean quality, semantic parity, maintain checks, and translation rules | [Authoring Guide](../maintain/authoring-guide.md), [Translation Guide](../maintain/translation-guide.md), and [Checks](../maintain/checks.md) |

## No Duplicate Injection

Non-owner docs may summarize the reader-visible consequence and link to the owner. They should not paste schemas, DDL, enum tables, transition tables, template bodies, fixture assertions, public error precedence, security guarantees, or glossary definitions.

Documentation authoring, translation, review, link hygiene, owner-boundary drift, and docs-maintenance checks belong to [Authoring Guide](../maintain/authoring-guide.md), [Translation Guide](../maintain/translation-guide.md), and [Checks](../maintain/checks.md). Implementation sequencing and maintainer status decisions belong to [MVP Plan](../build/mvp-plan.md).
