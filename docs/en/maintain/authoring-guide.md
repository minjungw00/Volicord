# Authoring Guide

Use this guide before changing Harness documentation. It is a living editing rulebook for documentation work only. It does not authorize Harness Server/runtime implementation, product-repository writes, generated operational files, runtime state, projections, evidence records, QA records, acceptance records, close records, residual-risk records, executable fixtures, or conformance runners.

The repository is documentation-only today and remains in documentation review unless the maintainer handoff owner says otherwise in [MVP Plan](../build/mvp-plan.md). Treat the docs as source material for a future Harness Server, not as accepted implementation-ready runtime behavior.

## 1. Reading And Scope

- Read root `AGENTS.md` before working in this repository.
- Read this guide before English-facing documentation edits.
- For bilingual or terminology-affecting edits, read [Translation Guide](translation-guide.md).
- Before touching Korean docs, read [Korean Authoring Guide](../../ko/maintain/authoring-guide.md) and [Korean Translation Guide](../../ko/maintain/translation-guide.md).
- Keep the work documentation-only. Do not create runtime state, generated projections, generated operational artifacts, product code, server code, executable fixtures, conformance reports, or scratch Harness runtime objects.
- Prefer small batches. Report changed and deleted files.
- Do not create commits unless the user explicitly asks.

When old prose conflicts with the current product thesis, owner boundaries, Korean quality rules, active/later boundaries, or honest security wording, rewrite it. Preserve the durable rule, not the old section shape.

## 2. Compact Route Rule

README files and Maintain docs route only to the compact structure below:

- `docs/doc-index.yaml`
- `docs/*/start.md`
- `docs/*/use/user-guide.md`
- `docs/*/use/agent-guide.md`
- `docs/*/use/judgment-examples.md`
- `docs/*/build/mvp-plan.md`
- `docs/*/reference/README.md`
- `docs/*/later/index.md`
- `docs/*/maintain/authoring-guide.md`
- `docs/*/maintain/translation-guide.md`
- `docs/*/maintain/checks.md`

Use [Reference Index](../reference/README.md) to choose exact contract owners. Do not turn reference subpages, old route families, migration notes, or historical review files into active route tables. If an old path appears, replace it with the compact route that now owns the reader need or delete the stale wording.

Use [doc-index.yaml](../../doc-index.yaml) only as documentation retrieval metadata. It is not runtime configuration, implementation state, or permission to load both languages for one `doc_id`.

## 3. Bilingual Pair Rule

English and Korean docs are both active. A meaning change in `docs/en` must be mirrored in `docs/ko` in the same batch, and a meaning change found while editing Korean must be reflected back into English.

Paired docs must keep the same active file map, reader purpose, semantic section coverage, owner routing, active/later boundary, and exact identifiers. Korean headings and paragraphing may differ when they read naturally and preserve the same meaning.

Preserve exact file paths, `doc_id` values, API method names, schema names, field names, enum values, error codes, table names, validator IDs, template names, and code-like strings in both languages.

## 4. One Owner Per Contract

Every strict contract has one owner. The owner is the only place to define exact fields, enum values, DDL, schemas, algorithms, state transitions, gate rules, fixture body shapes, template bodies, storage rules, security guarantees, error precedence, and official definitions.

Non-owner docs may name the reader-visible consequence and route to the owner through [Reference Index](../reference/README.md), [Later Index](../later/index.md), or [MVP Plan](../build/mvp-plan.md). They must not create a second definition.

Use this owner routing:

| Contract area | Route |
|---|---|
| Core transitions, gates, `prepare_write`, Write Authorization, `record_run`, `close_task`, waivers, and non-substitution rules | [Reference Index](../reference/README.md) |
| Public API methods, active request/response shapes, shared schemas, and public errors | [Reference Index](../reference/README.md) |
| Later candidate API and schema material | [Later Index](../later/index.md) |
| Storage layout, DDL, persisted records, locks, artifacts, and migrations | [Reference Index](../reference/README.md) |
| Projection rules, template bodies, freshness, and derived display boundaries | [Reference Index](../reference/README.md) |
| Security assets, trust boundaries, guarantee levels, and honest security wording | [Reference Index](../reference/README.md) |
| Conformance meaning, future fixture shape, and assertion authority | [Reference Index](../reference/README.md) |
| Agent connector behavior, capability profiles, context surfaces, and fallback semantics | [Reference Index](../reference/README.md) |
| Runtime space separation and non-isolation claims | [Reference Index](../reference/README.md) |
| Design-quality activation, finding severity, waiver boundary, and validator IDs | [Reference Index](../reference/README.md) |
| Official terminology | [Reference Index](../reference/README.md) |
| Implementation sequencing and maintainer readiness/status decisions | [MVP Plan](../build/mvp-plan.md) |

When a non-owner repeats a contract, update the owner if needed, then replace the duplicate with a short consequence and compact owner route.

## 5. Active/Later Boundary

Active docs must not turn later candidates, diagnostic, operations, export, rich template, or future conformance-runner material into an active requirement by wording.

A value, method, table, fixture family, command, template, or security guarantee is active only when its owner promotes it with scope, fallback behavior, and proof expectations. Reference presence alone does not expand active delivery scope.

Do not list profile-gated values as default active MVP values. If a value is available only under a named profile, capability, connector mode, or future configuration, keep it out of default active value lists or mark the profile gate in the owner.

Do not describe later candidates as active MVP requirements, required defaults, required checks, required templates, or server obligations. If an example needs a later candidate, label it as later material and route the normative detail to [Later Index](../later/index.md) or the owner that promoted it.

If an active schema or DDL block contains inactive values, fix the owner boundary rather than relying on nearby prose to explain that those values are not active.

## 6. User Judgment Boundary

Harness preserves user-owned judgment. Product behavior, material technical direction, scope expansion, sensitive-action approval, QA waiver, final acceptance, verification-risk acceptance, residual-risk acceptance, and cancellation are distinct judgment routes.

Do not treat broad approval such as "go ahead" or "looks good" as a substitute for a specific judgment. Sensitive-action approval permits a named sensitive step only; it does not decide product behavior, architecture, final acceptance, or residual risk. Final acceptance does not create evidence, erase evidence gaps, or accept residual risk unless the residual-risk path asks for that judgment.

User-facing docs should start with what the user can ask, what the agent should clarify, what is blocked, what evidence exists, what judgment is needed, and what close means. Introduce internal labels only after the visible user situation is clear.

## 7. Security Wording Rule

Security wording must match the documented guarantee level.

- Use cooperative wording when Harness can guide or record expected behavior but cannot technically block the action.
- Use detective wording when Harness can detect or report after the action.
- Use preventive wording only when the documented surface can block before the covered action and the proof path exists for that operation.
- Use isolated wording only when a documented separation boundary exists. Name the boundary.

Do not imply early Harness provides OS permissions, arbitrary-tool sandboxing, tamper-proof local files, universal pre-tool blocking, or security isolation unless the exact mechanism is documented and proven. Write Authorization is a cooperative Harness record/check, not OS permission, sandboxing, tamper-proof enforcement, preventive blocking, or isolation.

Do not make unsupported preventive, isolation, sandboxing, tamper-proof, or default tool-blocking claims in user-facing summaries, examples, checklists, or diagrams. A short, honest cooperative or detective claim is better than a stronger claim the current owner cannot prove.

## 8. Korean Quality Rule

Korean documentation must read as natural Korean technical prose, not line-by-line English. Put the Korean concept first in user-facing prose, add exact English identifiers only when precision or search needs them, and preserve identifiers exactly.

Do not leave English noun phrases in Korean prose unless they are exact identifiers or intentional Harness labels. Use the terms in [Translation Guide](translation-guide.md) and the Korean guide pair, including "한영 문서 동시 유지", "의미 일치", "줄 단위 번역 아님", "에이전트 중복 주입 금지", "현재 MVP", "담당 문서", and "profile-gated 값" where they fit.

## 9. Stale Content Deletion Rule

Maintain docs should guide future editing. They should not preserve historical rewrite reviews, resolved issue records, old acceptance notes, old delivery-label explanations, legacy alias history, later-candidate localization audit records, past translation problem records, or scratch migration plans.

When stale review history contains a still-useful rule, extract the rule and delete the historical framing. Do not keep past audit result narrative, issue-resolution records, or old acceptance prose as active maintain guidance.

Use these durable triage categories when deciding what to do with old prose:

| Category | Use when | Action |
|---|---|---|
| `preserve` | The text supports the product thesis, belongs to the right owner, and helps the reader. | Keep the meaning and polish if useful. |
| `shrink` | The text is right but too long, repetitive, internal, or contract-heavy for its document family. | Keep only the reader-visible consequence and compact owner route. |
| `move` | The text belongs in another owner or document family. | Move the meaning to that owner or route there, then remove the old copy. |
| `delete` | The text is obsolete, misleading, duplicative, historical, or conflicts with the product thesis, owner boundary, Korean quality rule, active/later boundary, or guarantee level. | Delete it. Do not keep it for continuity. |
| `decision-needed` | The edit exposes a real unresolved choice about schema, state, API, active/later boundary, security guarantee, fixture semantics, terminology, or implementation readiness. | Route the decision to the owning document. Major server-coding decisions belong in [MVP Plan](../build/mvp-plan.md), not scattered TODOs. |

Delete temporary migration plans and scratch files before finishing.

## 10. Post-Edit Checklist

- [ ] The edit stayed documentation-only.
- [ ] English and Korean paired files preserve the same meaning and active file coverage.
- [ ] Korean prose is natural Korean, not line-by-line English.
- [ ] Exact identifiers, file paths, schema/API names, enum values, error codes, table names, validator IDs, and template names are preserved.
- [ ] README and Maintain routes point only to the compact route structure and `docs/doc-index.yaml`.
- [ ] Strict contracts remain in one owner; non-owner duplicates are summaries with compact owner routes.
- [ ] Active/later boundaries are not blurred, and profile-gated values are not listed as default active MVP values.
- [ ] User-owned judgment routes remain distinct.
- [ ] Security wording matches the documented guarantee level and does not make unsupported preventive, isolation, sandboxing, tamper-proof, or default tool-blocking claims.
- [ ] Links, anchors, README routes, and paired-language links resolve.
- [ ] Deleted routes and old structure names are not used as active paths.
- [ ] Stale rewrite history, resolved issue records, legacy alias history, and old review prose were deleted instead of archived.
- [ ] No temporary migration plan or scratch file remains.
- [ ] Relevant checks in [Checks](checks.md) were run or reported as not run.
