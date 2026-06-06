# Checks

Use these checks after documentation edits and before major review handoff. They are read-only Markdown documentation checks, not runtime checks.

Use `PASS`, `WARN`, and `FAIL` only as docs-maintenance labels. They help reviewers decide what to inspect next; they do not decide acceptance or readiness.

## 1. What this checks

These checks look for documentation drift:

- broken links, anchors, and README routes
- English/Korean file-map or semantic mismatch
- duplicate strict contracts outside their owner
- active/later boundary drift
- security wording that overclaims the guarantee level
- user judgment routes that substitute for each other
- agent retrieval guidance that would overload or misroute context
- stale rewrite history, resolved issue records, and old review prose

## 2. What this does not prove

This page does not prove runtime behavior, runtime conformance, implementation readiness, documentation acceptance, development readiness, final acceptance, close readiness, QA, evidence sufficiency, residual-risk acceptance, or permission to start server coding.

Do not use these checks to create runtime state, `task_events`, generated projections, generated operational artifacts, executable fixtures, conformance reports, QA records, acceptance records, close records, residual-risk records, or product writes.

`PASS` means only that the checked documentation appears internally consistent for that item. `WARN` means a human should review uncertain wording. `FAIL` means docs-maintenance drift was found and should be routed to the owner.

## 3. Link check

Inspect relative Markdown links, README routes, paired-language links, owner links, and heading anchors.

Pass when every active link resolves to a current file and anchor. Fail when a route points to a deleted file, old heading, inactive migration record, or wrong-language owner.

## 4. Bilingual map check

Inspect `docs/en` and `docs/ko` for the same active file map, reader purpose, section coverage, owner links, and exact identifiers.

Pass when paired files preserve the same meaning while Korean remains natural. Fail when a Korean file omits active English meaning, translates an exact identifier, changes an owner route, or moves active material into later scope or later material into active scope.

## 5. Owner-boundary check

Inspect schemas, DDL, enum values, state transitions, gate rules, algorithms, fixture body shapes, template bodies, storage rules, security guarantees, validator IDs, and official definitions.

Pass when each strict contract is defined in one owner and non-owner docs use a short local consequence plus owner link. Fail when Start, Use, Build, Maintain, README, or a non-owner Reference summary creates a second normative definition.

## 6. Active/later check

Inspect active schemas, API docs, DDL, Build scope wording, Later docs, Roadmap candidates, and examples.

Pass when active blocks contain only active material and later/profile candidates stay in later/profile owners unless promoted. Fail when later enum values, methods, tables, commands, templates, assurance behavior, operations behavior, or fixture families are presented as active requirements.

## 7. Security wording check

Inspect claims using cooperative, detective, preventive, isolated, guard, freeze, careful-mode, sandbox, permission, blocking, tamper-proof, or isolation language.

Pass when the claim matches the documented guarantee level and names the owner/proof path for preventive or isolated behavior. Fail when cooperative or detective behavior is described as OS permission, arbitrary-tool sandboxing, tamper-proof storage, universal pre-tool blocking, or security isolation without a proven owner path.

## 8. User judgment boundary check

Inspect judgment prompts, examples, close wording, approval wording, final acceptance wording, QA waiver wording, and residual-risk wording.

Pass when product decisions, technical decisions, scope decisions, sensitive-action approval, QA waiver, verification-risk acceptance, final acceptance, residual-risk acceptance, and cancellation stay distinct. Fail when broad approval, sensitive-action approval, final acceptance, QA waiver, evidence, verification, or residual-risk acceptance silently substitutes for another route.

## 9. Agent retrieval check

Inspect agent guidance, context-loading advice, README routes, Reference routes, and any always-on context examples.

Pass when agent-facing docs retrieve only the owner section needed for the next action and keep always-on context compact. Fail when docs instruct agents to load broad reference sets, full schemas, full templates, historical logs, generated artifacts, or stale migration records by default.

## 10. Stale content check

Inspect Maintain docs and any nearby routes for historical rewrite reviews, resolved issue records, old acceptance records, old stage label explanations, legacy alias history, later-profile localization audit records, past translation problem records, and temporary migration plans.

Pass when Maintain docs contain only living editing rules and current checks. Fail when old review prose is preserved as active guidance, archive copies are created, or scratch migration files remain after the edit.
