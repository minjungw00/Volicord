# Authoring Guide

Use this guide before changing Harness documentation. It is a living editing rulebook for documentation work only. It does not authorize Harness Server/runtime implementation, product-repository writes, generated operational files, runtime state, projections, evidence records, QA records, acceptance records, close records, residual risk records, executable fixtures, or conformance runners.

The repository is documentation-only today and remains in documentation review unless the maintainer handoff owner says otherwise in [MVP Plan](../build/mvp-plan.md). Treat the docs as source material for a future Harness Server, not as accepted implementation-ready runtime behavior.

## 1. Reading And Scope

- Read root `AGENTS.md` before working in this repository.
- Read this guide before English-facing documentation edits.
- For bilingual or terminology-affecting edits, read [Translation Guide](translation-guide.md).
- Before touching Korean docs, read [Korean Authoring Guide](../../ko/maintain/authoring-guide.md) and [Korean Translation Guide](../../ko/maintain/translation-guide.md).
- Keep the work documentation-only. Do not create runtime state, generated projections, generated operational artifacts, product code, server code, executable fixtures, conformance reports, or scratch Harness runtime objects.
- Prefer small batches. Report changed and deleted files.
- Do not create commits unless the user explicitly asks.

When stale prose conflicts with the current product thesis, owner boundaries, Korean quality rules, active/later boundaries, or honest security wording, rewrite it. Preserve the durable rule, not the former section shape.

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

Use [Reference Index](../reference/README.md) to choose exact contract owners. Do not turn reference subpages, stale route families, migration notes, or historical review files into active route tables. If a stale path appears, replace it with the compact route that now owns the reader need or delete the stale wording.

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

Use these principles whenever writing active MVP API, access, error, or artifact prose:

- Public error codes are governed by the API Errors document selected through [Reference Index](../reference/README.md). Do not invent public error synonyms in Maintain, Use, Start, or examples.
- `access_class` describes the method-level verified request context. The active MVP uses one `access_class` value per request; do not describe per-artifact or mixed `access_class` values inside one request unless an owner promotes a later shape.
- Artifact lifecycle descriptions must distinguish staging through `stage_artifact` / `artifact_registration`, promotion from a staged handle after provenance and scope validation, persistent artifact linking, and artifact body read. `existing_artifact` links an existing persistent artifact; it does not register new artifact body bytes.
- `record_run` uses the `run_recording` path. Do not describe `record_run` as requiring both `run_recording` and `artifact_registration`, and do not place `record_run` inside `artifact_registration`.
- Staged handle provenance or scope validation failure maps to `VALIDATION_FAILED`, not `LOCAL_ACCESS_MISMATCH` or `CAPABILITY_INSUFFICIENT`.
- Active MVP public state-version conflict uses `STATE_VERSION_CONFLICT`. Do not document another public `ErrorCode` alias, synonym, deprecated name, alternate spelling, or storage-layer public error name for that mismatch.
- New public method responses must be written as `MethodResult | ToolDryRunResponse | ToolRejectedResponse` unless the method is strictly read-only and explicitly omits `ToolDryRunResponse` by contract. Use the concrete method result name in the owner, such as `PrepareWriteResult`, `StageArtifactResult`, `RecordRunResult`, or `CloseTaskResult`.
- If a public method contains both read-only intents and state-effecting intents, document branch selection by the selected intent's state effect, not by method name alone. `dry_run=true` on a selected read-only operation may return the method-specific `MethodResult` with `base.dry_run=true` and `effect_kind=read_only`; a valid dry-run preview for a selected state-effecting operation uses `ToolDryRunResponse`.
- Method-specific result fields belong only in the method `MethodResult` branch. `ToolRejectedResponse` and `ToolDryRunResponse` must not require result-only fields such as `decision`, `task_ref`, `run_summary`, `staged_artifact_handle`, `write_authorization_ref`, `user_judgment_ref`, or `close_state`, and dry-run prose must not require generated refs for records that do not exist. Rejected responses and `ToolDryRunResponse` branches have `effect_kind=no_effect`: no replay row, no state-version increment, no staged-handle consumption, and no Write Authorization creation or consumption.
- For `close_task`, define preflight rejection before the close matrix. Preflight rejection returns `ToolRejectedResponse`; close matrix blockers return `CloseTaskResult(close_state=blocked)` only after preflight succeeds and the semantic matrix finds blockers.
- Do not mix preflight failure `ErrorCode` values with `CloseBlocker.code`. `STATE_VERSION_CONFLICT`, stale `WriteAuthorization.basis_state_version`, and `idempotency_key` reuse with a different request hash are preflight rejection cases, not committed `write_compatibility` or `recovery` blockers.
- Do not assign the same state effects to `ToolRejectedResponse` and committed blocked `CloseTaskResult`. A close preflight `ToolRejectedResponse` creates no `CloseBlocker`, `task_event`, `task_events` append, replay row, `tool_invocations.response_json`, `close_state` mutation, Write Authorization creation or consumption, staged-handle consumption, artifact promotion or link, evidence update, or `project_state.state_version` increment; a committed blocked `CloseTaskResult` may have only the effects allowed by the `close_task` owner contract.

When a non-owner repeats a contract, update the owner if needed, then replace the duplicate with a short consequence and compact owner route.

## 5. Active/Later Boundary

Active docs must not turn later candidates, diagnostic, operations, export, rich template, or future conformance-runner material into an active requirement by wording.

A value, method, table, fixture family, command, template, or security guarantee is active only when its owner promotes it with scope, fallback behavior, and proof expectations. Reference presence alone does not expand active delivery scope.

Do not list profile-gated values as default active MVP values. If a value is available only under a named profile, capability, connector mode, or future configuration, keep it out of default active value lists or mark the profile gate in the owner.

Do not describe later candidates as active MVP requirements, required defaults, required checks, required templates, or server obligations. If an example needs a later candidate, label it as later material and route the normative detail to [Later Index](../later/index.md) or the owner that promoted it.

If an active schema or DDL block contains inactive values, fix the owner boundary rather than relying on nearby prose to explain that those values are not active.

Treat active/later ambiguity as a contract failure, not a style issue:

- Later-only concepts must be marked later-only at the point of use unless an owner has promoted them into the active MVP.
- Reference docs must not imply that later-only features are required for active MVP implementation merely because the reference page names them.
- Active MVP text fails when it describes `captured_artifact` as an active artifact input path. New artifact bytes use the active staged-artifact path unless an owner promotes another path.
- Active MVP text fails when it uses both task-scoped and project-scoped `state_version` as public conflict clocks. The public active conflict basis is the project-wide `project_state.state_version` unless an owner promotes another clock.
- Active MVP text fails when it treats `reconcile` or projection reconcile as a Core state mutation path. Projection reconcile is later-only unless promoted, and projections remain derived displays.

## 6. User Judgment Boundary

Harness preserves user-owned judgment. Product behavior, material technical direction, scope expansion, sensitive-action approval, final acceptance, residual risk acceptance, and cancellation are distinct active judgment routes. Later/reserved QA waiver and verification-risk acceptance routes must stay distinct when mentioned or promoted.

Do not treat broad approval such as "go ahead" or "looks good" as a substitute for a specific judgment. Sensitive-action approval permits a named sensitive step only; it does not decide product behavior, architecture, final acceptance, or residual risk. Final acceptance does not create evidence, erase evidence gaps, or accept residual risk unless the residual risk path asks for that judgment.

Fail any wording that treats `sensitive_approval` or `SensitiveActionScope` as equivalent to product-file `AuthorizedAttemptScope`, Write Authorization, or path authority. `sensitive_approval` is a user judgment for a named sensitive action; `AuthorizedAttemptScope` is the product-file write-attempt scope owned by the write path.

Fail any wording that lets final acceptance or residual risk acceptance substitute for missing required evidence. Evidence must exist through the evidence owner path before those judgments can accept a result basis or a named risk.

When blocker wording combines more than one negative requirement, state each negative explicitly. For residual risk close blockers, preserve the meaning as "not visible, or not accepted when required"; do not drop the visibility negative or otherwise compress the condition into an ambiguous form.

User-facing docs and templates should start with what the user can ask, what the agent should clarify, what is blocked, what evidence exists, what judgment is needed, and what close means. Introduce internal labels only after the visible user situation is clear. Fail user-facing documentation or templates when they expose internal enum, schema, or error-code terms such as `EvidenceSummary`, `CloseBlocker.category`, `judgment_kind`, `guarantee_level`, internal error codes, or raw enum values where natural display wording would be enough.

## 7. Security Wording Rule

Security wording must match the documented guarantee level.

- Use cooperative wording when Harness can guide or record expected behavior but cannot technically block the action.
- Use detective wording when Harness can detect or report a supported observable fact after the action, and for current MVP claims only after the relevant capability check has passed.
- Use preventive wording only when the documented surface can block before the covered action and the proof path exists for that operation.
- Use isolated wording only when a documented separation boundary exists. Name the boundary.

Do not imply early Harness provides OS permissions, arbitrary-tool sandboxing, tamper-proof local files, universal pre-tool blocking, or security isolation unless the exact mechanism is documented and proven. Write Authorization is a cooperative Harness record/check, not OS permission, sandboxing, tamper-proof enforcement, preventive blocking, or isolation.

Do not make unsupported preventive, isolation, sandboxing, tamper-proof, or default tool-blocking claims in user-facing summaries, examples, checklists, or diagrams. A short, honest cooperative claim, or a capability-backed detective claim, is better than a stronger claim the current owner cannot prove.

Fail any text that treats `surface_id` as proof of authority, local access, binding, or capability. A copied `surface_id` is only an identifier; authority and capability claims require the registered surface context and passed owner checks.

Fail any text that displays or claims a `detective` guarantee without passed capability verification for the covered observable scope.

## 8. Korean Quality Rule

Korean documentation must read as natural Korean technical prose, not line-by-line English. Put the Korean concept first in user-facing prose, add exact English identifiers only when precision or search needs them, and preserve identifiers exactly.

For contract checks and authoring principles, Korean prose must explain the concept naturally while preserving exact identifiers such as API method names, schema fields, enum values, and error codes.

User-facing Korean should use natural Korean labels, not raw enum names, unless the raw enum or status value is the subject. Schema fields, method names, enum values, code identifiers, file paths, validator IDs, and error codes stay exact English in schema and code-like contexts.

Do not leave English noun phrases in Korean prose merely because the English source used them. If the phrase is explanatory rather than an exact identifier or intentional Harness label, translate the concept into Korean. Do not translate the same concept differently across files; update [Translation Guide](translation-guide.md) or the glossary when a new shared term is needed.

Treat Korean that preserves English prose, English sentence order, or literal translation where natural Korean explanation is required as a failed edit. Exact identifiers stay exact; explanatory prose must be Korean.

Use the terms in [Translation Guide](translation-guide.md) and the Korean guide pair, including "한영 문서 동시 유지", "의미 일치", "줄 단위 번역 아님", "에이전트 중복 주입 금지", "현재 MVP", "담당 문서", "사용자 소유 판단", "최종 수락", "잔여 위험 수락", "협력형 보장", "탐지형 보장", and "profile-gated 값" where they fit.

## 9. Stale Content Deletion Rule

Maintain docs should guide future editing. They should not preserve historical rewrite reviews, closed issue records, obsolete acceptance notes, obsolete delivery-label explanations, obsolete alias history, later-candidate localization audit records, past translation problem records, or scratch migration plans.

When stale review history contains a still-useful rule, extract the rule and delete the historical framing. Do not keep past audit result narrative, issue-resolution records, or obsolete acceptance prose as active maintain guidance.

Use these durable triage categories when deciding what to do with stale prose:

| Category | Use when | Action |
|---|---|---|
| `preserve` | The text supports the product thesis, belongs to the right owner, and helps the reader. | Keep the meaning and polish if useful. |
| `shrink` | The text is right but too long, repetitive, internal, or contract-heavy for its document family. | Keep only the reader-visible consequence and compact owner route. |
| `move` | The text belongs in another owner or document family. | Move the meaning to that owner or route there, then remove the stale copy. |
| `delete` | The text is obsolete, misleading, duplicative, historical, or conflicts with the product thesis, owner boundary, Korean quality rule, active/later boundary, or guarantee level. | Delete it. Do not keep it for continuity. |
| `decision-needed` | The edit exposes a real unresolved choice about schema, state, API, active/later boundary, security guarantee, fixture semantics, terminology, or implementation readiness. | Route the decision to the owning document. Major server-coding decisions belong in [MVP Plan](../build/mvp-plan.md), not scattered TODOs. |

Delete temporary migration plans and scratch files before finishing.

## 10. Post-Edit Checklist

- [ ] The edit stayed documentation-only.
- [ ] English and Korean paired files preserve the same meaning and active file coverage.
- [ ] Korean prose is natural Korean, not line-by-line English, and user-facing Korean uses natural labels instead of raw enum names unless the raw value is being explained.
- [ ] Exact identifiers, file paths, schema/API names, enum values, error codes, table names, validator IDs, and template names are preserved.
- [ ] The same concept uses the same Korean term across files, and English noun phrases were not left in Korean prose unless they are exact identifiers or intentional Harness labels.
- [ ] README and Maintain routes point only to the compact route structure and `docs/doc-index.yaml`.
- [ ] Strict contracts remain in one owner; non-owner duplicates are summaries with compact owner routes.
- [ ] Active/later boundaries are not blurred, and profile-gated values are not listed as default active MVP values.
- [ ] Later-only concepts are explicitly marked later-only where they appear, and reference docs do not make later-only features look required for active MVP implementation.
- [ ] Active MVP text does not promote `captured_artifact`, projection reconcile, task-scoped public conflict clocks, or other later-only paths by implication.
- [ ] User-owned judgment routes remain distinct.
- [ ] `sensitive_approval` / `SensitiveActionScope` is not collapsed into `AuthorizedAttemptScope`, Write Authorization, final acceptance, residual-risk acceptance, evidence, or artifact authority.
- [ ] Final acceptance and residual risk acceptance do not substitute for missing required evidence.
- [ ] Blocker conditions with multiple negative requirements state each negative explicitly, especially residual risk close blocker wording.
- [ ] Security wording matches the documented guarantee level and does not make unsupported preventive, isolation, sandboxing, tamper-proof, or default tool-blocking claims.
- [ ] `surface_id` is not treated as proof of authority, and `detective` is not displayed or claimed without passed capability verification.
- [ ] User-facing templates do not expose internal enum or schema terms unnecessarily.
- [ ] Links, anchors, README routes, and paired-language links resolve.
- [ ] Deleted routes and stale structure names are not used as active paths.
- [ ] Stale rewrite history, closed issue records, obsolete alias history, and stale review prose were deleted instead of archived.
- [ ] No temporary migration plan or scratch file remains.
- [ ] Relevant checks in [Checks](checks.md) were run or reported as skipped.
