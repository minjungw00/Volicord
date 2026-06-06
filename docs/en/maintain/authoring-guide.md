# Authoring Guide

Use this guide before changing Harness documentation. It is a living editing rulebook for documentation work only. It does not authorize Harness Server/runtime implementation, product-repository writes, generated operational files, runtime state, projections, evidence records, QA records, acceptance records, close records, residual-risk records, executable fixtures, or conformance runners.

The repository is documentation-only today and remains in post-redesign review unless the maintainer handoff owner says otherwise. Treat the docs as source material for a future Harness Server, not as accepted implementation-ready runtime behavior.

## 1. Editing rule

- Read root `AGENTS.md` before working in this repository.
- Read this guide before documentation edits.
- For bilingual or terminology-affecting edits, read [Translation Guide](translation-guide.md).
- Before touching Korean docs, read [Korean Authoring Guide](../../ko/maintain/authoring-guide.md) and [Korean Translation Guide](../../ko/maintain/translation-guide.md).
- Keep the work documentation-only. Do not create runtime state, generated projections, generated operational artifacts, product code, server code, executable fixtures, conformance reports, or scratch Harness runtime objects.
- Prefer small batches. Report changed and deleted files.
- Do not create commits unless the user explicitly asks.

When old prose conflicts with the current product thesis, owner boundaries, Korean quality rules, active/later boundaries, or honest security wording, rewrite it. Preserve the durable principle, not the old section shape.

## 2. Bilingual pair rule

English and Korean docs are paired. A meaning change in `docs/en` must be mirrored in `docs/ko` in the same batch, and a meaning change found while editing Korean must be reflected back into English.

Paired docs must keep the same active file map, reader purpose, semantic section coverage, owner links, active/later boundary, and exact identifiers. Korean headings and paragraphing may differ when they read naturally and preserve the same meaning.

Preserve exact file paths, `doc_id` values, API method names, schema names, field names, enum values, error codes, table names, validator IDs, and code-like strings in both languages.

## 3. One owner per contract

Every strict contract has one owner. The owner is the only place to define exact fields, enum values, DDL, schemas, algorithms, state transitions, gate rules, fixture body shapes, template bodies, storage rules, security guarantees, error precedence, and official definitions.

Non-owner docs may name the reader-visible consequence and link to the owner. They must not create a second definition.

Use this owner split:

| Area | Owner |
|---|---|
| Core transitions, gates, `prepare_write`, Write Authorization, `record_run`, `close_task`, waivers, and non-substitution rules | [Core Model Reference](../reference/core-model.md) |
| Public API methods and active request/response shapes | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md) |
| Later/profile API and schema candidates | [Later Candidate Index](../later/index.md#later-schema-candidates) |
| Storage layout, DDL, persisted records, locks, artifacts, and migrations | [Storage](../reference/storage.md) |
| Projection rules, template bodies, freshness, and derived display boundaries | [Projection And Templates Reference](../reference/projection-and-templates.md) |
| Security assets, trust boundaries, guarantee levels, and honest security wording | [Security Reference](../reference/security.md) |
| Conformance meaning, future fixture shape, and assertion authority | [Conformance Reference](../reference/conformance.md) |
| Agent connector behavior, capability profiles, context surfaces, and fallback semantics | [Agent Integration Reference](../reference/agent-integration.md) |
| Runtime space separation and non-isolation claims | [Runtime Boundaries Reference](../reference/runtime-boundaries.md) |
| Design-quality activation, finding severity, waiver boundary, and validator IDs | [Design Quality](../reference/design-quality.md) |
| Official terminology | [Glossary Reference](../reference/glossary.md) |
| Implementation sequencing and maintainer readiness/status decisions | [MVP Plan](../build/mvp-plan.md) |

When a non-owner repeats a contract, update the owner if needed, then replace the duplicate with a short summary and owner link.

## 4. Active/later boundary

Active docs must not turn later/profile, Roadmap, diagnostic, operations, export, rich template, or future conformance-runner material into an active requirement by wording.

A value, method, table, fixture family, command, template, or security guarantee is active only when its owner promotes it with scope, fallback behavior, and proof expectations. Reference presence alone does not expand active delivery scope.

Keep later material in `docs/*/later/*`, explicitly labeled later/profile sections, or the owner that has promoted it. If an active schema or DDL block contains inactive values, fix the owner boundary rather than relying on nearby prose to explain that those values are not active.

## 5. User judgment boundary

Harness preserves user-owned judgment. Product behavior, material technical direction, scope expansion, sensitive-action approval, QA waiver, final acceptance, verification-risk acceptance, residual-risk acceptance, and cancellation are distinct judgment routes.

Do not treat broad approval such as "go ahead" or "looks good" as a substitute for a specific judgment. Sensitive-action approval permits a named sensitive step only; it does not decide product behavior, architecture, final acceptance, or residual risk. Final acceptance does not create evidence, erase evidence gaps, or accept residual risk unless the residual-risk path asks for that judgment.

User-facing docs should start with what the user can ask, what the agent should clarify, what is blocked, what evidence exists, what judgment is needed, and what close means. Introduce internal labels only after the visible user situation is clear.

## 6. Security wording rule

Security wording must match the documented guarantee level.

- Use cooperative wording when Harness can guide or record expected behavior but cannot technically block the action.
- Use detective wording when Harness can detect or report after the action.
- Use preventive wording only when the documented surface can block before the covered action and the proof path exists for that operation.
- Use isolated wording only when a documented separation boundary exists. Name the boundary.

Do not imply early Harness provides OS permissions, arbitrary-tool sandboxing, tamper-proof local files, universal pre-tool blocking, or security isolation unless the exact mechanism is documented and proven. Write Authorization is a cooperative Harness record/check, not OS permission, sandboxing, tamper-proof enforcement, preventive blocking, or isolation.

## 7. Link rule

When you rename, move, split, merge, or delete documentation, update links and anchors in both languages in the same batch.

Before editing, search for old paths, old headings, old anchors, old title text, README routes, owner links, and paired-language links. After editing, search again. Active docs must not point to removed files, stale anchors, inactive routes, migration notes, or old structures.

Prefer owner links over secondary summaries. Do not create archive copies of removed maintain pages.

## 8. Stale content deletion rule

Maintain docs should guide future editing. They should not preserve historical rewrite reviews, resolved issue records, old acceptance notes, old stage label explanations, legacy alias history, later-profile localization audit records, past translation problem records, or scratch migration plans.

Use these durable triage categories when deciding what to do with old prose:

| Category | Use when | Action |
|---|---|---|
| `preserve` | The text supports the product thesis, belongs to the right owner, and helps the reader. | Keep the meaning and polish if useful. |
| `shrink` | The text is right but too long, repetitive, internal, or contract-heavy for its document family. | Keep only the reader-visible consequence and owner link. |
| `move` | The text belongs in another owner or document family. | Move the meaning to that owner or link there, then remove the old copy. |
| `delete` | The text is obsolete, misleading, duplicative, historical, or conflicts with the product thesis, owner boundary, Korean quality rule, active/later boundary, or guarantee level. | Delete it. Do not keep it for continuity. |
| `decision-needed` | The edit exposes a real unresolved choice about schema, state, API, active/later boundary, security guarantee, fixture semantics, terminology, or implementation readiness. | Route the decision to the owning document. Major server-coding decisions belong in [MVP Plan](../build/mvp-plan.md), not scattered TODOs. |

Delete temporary migration plans and scratch files before finishing.

## 9. Post-edit checklist

- [ ] The edit stayed documentation-only.
- [ ] English and Korean paired files preserve the same meaning and active file coverage.
- [ ] Korean prose is natural Korean, not line-by-line English.
- [ ] Exact identifiers, file paths, schema/API names, enum values, error codes, table names, and validator IDs are preserved.
- [ ] Strict contracts remain in one owner; non-owner duplicates are summaries with owner links.
- [ ] Active/later boundaries are not blurred.
- [ ] User-owned judgment routes remain distinct.
- [ ] Security wording matches the documented guarantee level.
- [ ] Links, anchors, README routes, and paired-language links resolve.
- [ ] Stale rewrite history, resolved issue records, legacy alias history, and old review prose were deleted instead of archived.
- [ ] No temporary migration plan or scratch file remains.
- [ ] Relevant checks in [Checks](checks.md) were run or reported as not run.
