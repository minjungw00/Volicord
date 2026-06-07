# Checks

Use these checks after documentation edits and before major review handoff. They are read-only Markdown documentation checks, not runtime checks.

Use `PASS`, `WARN`, and `FAIL` only as docs-maintenance labels. They help reviewers decide what to inspect next; they do not decide acceptance or readiness.

## 1. What This Checks

These checks look for documentation drift:

- compact route drift, stale route wording, broken links, anchors, and README routes
- bilingual semantic parity problems
- Korean prose that reads like a literal English translation
- duplicate strict contracts outside their owner
- active/later boundary drift and active/profile-gated value confusion
- unsupported security claims that overstate the guarantee level
- user judgment routes that substitute for each other
- projection-derived display wording that treats generated views as source authority
- one-language-per-`doc_id` agent retrieval problems
- stale rewrite history, resolved issue records, and old review prose

## 2. What This Does Not Prove

This page does not prove runtime behavior, runtime conformance, implementation readiness, documentation acceptance, development readiness, final acceptance, close readiness, QA, evidence sufficiency, residual-risk acceptance, or permission to start server coding.

Do not use these checks to create runtime state, `task_events`, generated projections, generated operational artifacts, executable fixtures, conformance reports, QA records, acceptance records, close records, residual-risk records, or product writes.

`PASS` means only that the checked documentation appears internally consistent for that item. `WARN` means a human should review uncertain wording. `FAIL` means docs-maintenance drift was found and should be routed to the owner.

## 3. Compact Route Check

Inspect README files, Maintain docs, route tables, navigation summaries, paired-language links, and retrieval guidance.

Pass when README and Maintain routes point only to:

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

Fail when active routing points to deleted files, stale route families, inactive migration records, wrong-language owners, old structure labels, or deep owner files instead of the compact owner index.

## 4. Link And Anchor Check

Inspect relative Markdown links, paired-language links, owner routes, heading anchors, old path names, deleted route names, and old structure labels.

Pass when every active link resolves to a current file and current anchor. Fail when active docs point to a missing file, old heading, inactive migration record, wrong-language owner, stale route family, or old structure name.

## 5. Bilingual Semantic Parity Check

Inspect `docs/en` and `docs/ko` for the same active file map, reader purpose, section coverage, owner routing, and exact identifiers.

Pass when paired files preserve the same meaning while Korean remains natural. Fail when a Korean file omits active English meaning, translates an exact identifier, changes an owner route, or moves active material into later scope or later material into active scope.

## 6. Korean Natural Prose Check

Inspect Korean explanatory prose, headings, examples, and maintain guidance.

Pass when Korean reads as natural Korean technical documentation, keeps exact identifiers unchanged, and does not leave English noun phrases in Korean prose unless they are exact identifiers or intentional Harness labels. Fail when Korean is a literal line-by-line English translation, preserves English noun phrases as prose, or changes meaning to follow English sentence order.

## 7. Owner-Boundary Check

Inspect schemas, DDL, enum values, state transitions, gate rules, algorithms, fixture body shapes, template bodies, storage rules, security guarantees, validator IDs, and official definitions.

Pass when each strict contract is defined in one owner and non-owner docs use a short local consequence plus compact owner route. Fail when Start, Use, Build, Maintain, README, or a non-owner Reference summary creates a second normative definition.

## 8. Active/Profile-Gated Value Check

Inspect active schemas, API docs, DDL, Build scope wording, Later docs, later candidates, profile/capability tables, connector modes, and examples.

Pass when default active blocks contain only active MVP material, profile-gated values are clearly labeled and owned, and later candidates stay in the Later index or promoted owners. Fail when later enum values, methods, tables, commands, templates, assurance behavior, operations behavior, fixture families, or profile-gated values are presented as default active requirements.

## 9. Unsupported Security Claim Check

Inspect claims using cooperative, detective, preventive, isolated, guard, freeze, careful-mode, sandbox, permission, blocking, tamper-proof, or isolation language.

Pass when the claim matches the documented guarantee level and names the owner/proof path for preventive or isolated behavior. Fail when cooperative or detective behavior is described as OS permission, arbitrary-tool sandboxing, tamper-proof storage, universal pre-tool blocking, or security isolation without a proven owner path.

## 10. User Judgment Boundary Check

Inspect judgment prompts, examples, close wording, approval wording, final acceptance wording, QA waiver wording, and residual-risk wording.

Pass when product decisions, technical decisions, scope decisions, sensitive-action approval, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, and cancellation stay distinct. Fail when broad approval, sensitive-action approval, final acceptance, QA waiver, evidence, verification, or residual-risk acceptance silently substitutes for another route.

## 11. Projection-Derived-Display Check

Inspect projection and template wording, generated-display examples, status cards, summaries, user-facing views, and diagrams.

Pass when projections and rendered displays are described as derived views with freshness and owner boundaries. Fail when generated displays are treated as source-of-truth records, runtime state, evidence, QA, acceptance, close records, residual-risk records, Write Authorization, or permission to perform product/runtime writes.

## 12. One-Language-Per-`doc_id` Agent Retrieval Check

Inspect agent guidance, context-loading advice, README routes, Reference routes, and any always-on context examples.

Pass when agent-facing docs retrieve only one language for a given `doc_id` during normal work, load paired languages only for translation or parity review, retrieve only the owner section needed for the next action, and keep always-on context compact. Fail when docs instruct agents to load both languages for the same `doc_id` by default, broad reference sets, full schemas, full templates, historical logs, generated artifacts, or stale migration records.

## 13. Stale Content Check

Inspect Maintain docs and nearby routes for historical rewrite reviews, resolved issue records, old acceptance records, old delivery-label explanations, old stage label history, legacy alias history, later-candidate localization audit records, past translation problem records, past audit result narrative, and temporary migration plans.

Pass when Maintain docs contain only living editing rules and current checks. Old stage label history may remain only as a minimal compatibility rule when a current owner needs it. Fail when old review prose is preserved as active guidance, issue-resolution or audit-result narrative remains, archive copies are created, or scratch migration files remain after the edit.
