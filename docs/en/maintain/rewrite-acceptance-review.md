# Rewrite Acceptance Review

## What this review is

This is a maintainer-facing documentation-redesign acceptance review for the current Harness documentation baseline. It is a Maintain document for maintainer handoff only.

This review does not accept implementation planning by itself. It does not authorize Harness Server/runtime implementation, product code, generated operational artifacts, generated projections, executable fixtures, conformance runners, runtime state, evidence records, QA records, Acceptance records, Residual Risk records, close records, or Harness Runtime Home contents. It does not claim runtime conformance has passed.

## Recommendation

Recommendation: conditionally ready for maintainer implementation-planning review.

The redesigned documentation is coherent enough to hand to maintainers for a separate implementation-planning readiness decision. The condition is that maintainers must deliberately update [Implementation Overview: Documentation acceptance status](../build/implementation-overview.md#documentation-acceptance-status), accept or reclassify the [implementation-readiness criteria](../build/implementation-overview.md#implementation-readiness-criteria), and accept or defer the centralized decision-log items in [MVP-1 User Work Loop: Implementation decisions needed before server coding](../build/mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding).

This is not a recommendation to start server coding now. Current documented status remains:

- Documentation review status: post-redesign review and documentation acceptance candidate only.
- Implementation planning readiness: not accepted.
- Runtime implementation status: not started.
- Server-coding decisions: not accepted for coding.

## Review Basis

This review is based on the active documentation set, especially:

- [Implementation Overview](../build/implementation-overview.md)
- [Engineering Checkpoint](../build/engineering-checkpoint.md)
- [MVP-1 User Work Loop](../build/mvp-user-work-loop.md)
- [Rewrite Plan](rewrite-plan.md)
- [Documentation Checks](documentation-checks.md)
- [Authoring Guide](authoring-guide.md)
- [Translation Guide](translation-guide.md)

It is not a historical diff of every redesign commit. It summarizes the current shape of the baseline.

## Preserved Core Principles

Status: preserved in the active baseline.

The docs consistently preserve these principles:

- Harness is not a prompt pack. It is a local authority record for scope, user-owned judgment, evidence, verification expectations, QA expectations, work acceptance, close readiness, and residual risk.
- User-owned judgment stays with the user. Product/UX judgment, material technical judgment, QA expectations, waivers, work acceptance, and residual-risk acceptance are not silently delegated to the agent.
- Evidence, verification, Manual QA, work acceptance, close readiness, and residual risk remain separate. None substitutes for another.
- Chat, connector output, Markdown-rendered projections, and generated documents are not operational truth.
- Core-owned local state and artifact references are the future operational authority.
- Documentation files are source material for understanding and implementing Harness. They are not Harness runtime objects.

## Deleted, Reduced, Or Moved Design/Prose

Status: acceptable for handoff, with owner links retained.

The redesign no longer treats broad workflow, dashboard, reporting, hosted-agent, evaluation-harness, or generic MCP-wrapper prose as the product center. Those ideas are either removed from active MVP framing, reduced to non-goal language, or moved into [Roadmap](../roadmap.md) and later-profile docs.

Major reductions and moves:

- Broad report/dashboard/export/handoff material is moved to [Operations Profile](../later/operations-profile.md), [Operations And Conformance Reference](../reference/operations-and-conformance.md), and template owners where appropriate.
- Full assurance material such as detached verification hardening, Manual QA matrices, detailed Evidence Manifest behavior, detailed Eval output, risk-review hardening, and stewardship validators is moved to [Assurance Profile](../later/assurance-profile.md) or the relevant Reference owner.
- Conformance-runner and executable fixture language is kept future-oriented in [Conformance Fixtures Reference](../reference/conformance-fixtures.md) and [Future Fixtures](../later/future-fixtures.md), not treated as current runnable validation.
- Strict schemas, DDL, state transitions, error semantics, projection rules, template bodies, storage rules, and security guarantees are routed to Reference owners instead of repeated in Learn, Use, Build, or Maintain pages.
- User-facing prose has been reduced away from internal labels as the default starting point. User docs are expected to start from ordinary user situations.

## Current Stage Model

Status: coherent.

The active stage model is:

| Label | Current meaning |
|---|---|
| Engineering Checkpoint | First internal local Core authority-loop smoke. It is not the product MVP and not user-value validation. |
| Kernel Smoke | Narrow future smoke-check authoring label under Engineering Checkpoint. It is not a stage and not a current executable fixture suite. |
| MVP-1 User Work Loop | First user-value milestone after Engineering Checkpoint. |
| Assurance Profile | Later hardening for assurance behavior. |
| Operations Profile | Later hardening for operations, recovery, export, and handoff. |
| Roadmap | Future candidates outside staged delivery unless promoted by owner docs. |

## MVP-1 User Work Loop Scope

Status: scoped and usable for planning review.

MVP-1 is the first user-value path. It includes ordinary-language start or resume, work-shape classification, scope/non-goals/success criteria, minimal user judgment, separate judgment route display, cooperative `prepare_write`, `record_run`, evidence refs or evidence summary, status and next-safe-action output, evidence gaps, close blockers, residual-risk visibility, compact Core-derived views, and honest Core/MCP unavailable behavior.

MVP-1 explicitly excludes full assurance, full operations, broad reports, dashboards, hosted UI, broad connectors, conformance runners, generated conformance artifacts, executable fixture catalogs, OS-level sandboxing, arbitrary-tool isolation, permission isolation, tamper-proof local storage, and default preventive pre-tool blocking.

## Engineering Checkpoint Scope

Status: scoped and narrower than MVP-1.

Engineering Checkpoint proves the smallest local Core authority loop:

- one local project
- one active Task
- one scope boundary
- `prepare_write` allow/block behavior
- one durable single-use Write Authorization
- one compatible `record_run`
- one artifact/evidence ref
- status/blocker output that reads Core state without mutating it

It does not include ordinary-language intake, full judgment presentation, detailed Evidence Manifest behavior, detached verification, Eval, Manual QA, work acceptance, residual-risk acceptance, full close semantics, full projection rendering, dashboards, reports, export, recover, conformance runner, broad connectors, team workflow, orchestration, metrics, hooks, preventive guard expansion, or Roadmap automation.

## Later Profile And Roadmap Boundaries

Status: separated from early scope.

Assurance Profile remains after MVP-1. It can harden verification, Manual QA, detailed evidence, risk review, Eval display, stewardship, TDD trace, feedback-loop, and context-hygiene behavior when owner docs define the exact contract.

Operations Profile remains after MVP-1 and Assurance Profile. It organizes export, recovery, handoff, operator readiness, doctor/readiness surfaces, artifact integrity operations, projection refresh/reconcile operations, and future conformance run entrypoints after the relevant owners define them.

Roadmap remains candidate material. Dashboard, hosted workflows, team workflows, broader connectors, metrics, automation, preventive guard expansion, hooks, deployment, canary, rollback, production monitoring, and other expansion candidates do not become active stage requirements unless promoted by owner docs with scope, fallback behavior, proof expectations, and no projection-as-canonical dependency.

## Remaining Open Implementation Decisions

Status: centralized, not scattered.

The open implementation decisions are centralized in [MVP-1 User Work Loop: Implementation decisions still open](../build/mvp-user-work-loop.md#implementation-decisions-still-open).

Still-open items currently recorded there:

- Implementation-readiness judgment: not accepted.
- Public API coding acceptance: not accepted for coding.
- Storage/DDL coding acceptance: not accepted for coding.
- Core transition acceptance: not accepted for coding.
- Security/local-access acceptance: not accepted for coding.
- Newly discovered owner conflict: none currently recorded.

These are implementation-planning and coding gates. They do not mean the docs are unusable for maintainer review, but they do block server coding until accepted or explicitly deferred with stage impact.

## Docs Are Not Runtime Objects

Status: confirmed by current repo guidance.

The docs repeatedly say that documentation files are source material, not runtime state, generated projections, evidence records, QA records, Acceptance records, Residual Risk records, close records, operational truth, or conformance artifacts. This review follows that boundary.

## Repository Identity

Status: confirmed by current repo guidance.

This repository is documentation-only today. Its intended future role is Harness Server source repository after documentation acceptance and a separate implementation-planning readiness decision. It is not the user's Product Repository and not a Harness Runtime Home. No Harness Server/runtime implementation, runtime data, generated projection system, conformance runner, product code, or generated operational artifact exists here yet.

## User Terminology Burden Review

Status: acceptable for handoff, with continued review needed during final acceptance.

The authoring and translation guides now require user-facing docs to start from ordinary user situations rather than labels such as `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, or `task_events`. The Build and README routes also frame internal terms as implementation or reference labels, not user prerequisites.

No full user-language audit was run in this batch. Final maintainers should still use [Documentation Checks: User-Language Check](documentation-checks.md#user-language-check) before acceptance.

## Security Wording Review

Status: acceptable for handoff, with implementation proof still future.

The current baseline states that MVP-1 uses cooperative plus limited detective wording. It explicitly rejects OS-level permission control, arbitrary-tool sandboxing, tamper-proof local files, default pre-tool blocking, permission isolation, and security isolation unless a future promoted owner path proves the exact covered operation.

This review did not prove preventive or isolated enforcement. Such proof is future-runtime-only and cannot be produced by documentation review.

## Context, Projection, And State Separation Review

Status: acceptable for handoff.

The docs keep always-on agent context compact and phase-relevant. Detailed contracts route to owner docs instead of prompt-loading the whole reference set.

Projection and template docs are framed as derived display. Compact MVP-1 views and status outputs do not authorize writes, satisfy evidence, record acceptance, accept risk, close tasks, or become canonical state. Core-owned local state and artifact references remain the future operational authority.

## Template And Artifact Scope Review

Status: acceptable for handoff.

The active baseline keeps full template bodies with Template Reference owners and keeps future export/report templates out of MVP-1 unless promoted. Engineering Checkpoint needs only status/blocker output and one artifact/evidence ref. MVP-1 may use compact Core-derived views, while later export/report/handoff templates stay later-profile material.

Artifacts are treated as references registered through owner paths, not as free-form documentation outputs that become authority.

## Targeted Cleanup Review

Status: targeted checks refreshed for this final validation pass; not a full documentation acceptance pass.

| Area | Current review finding |
|---|---|
| Later-profile Decision Packet template parity and authority wording | Manually checked `docs/en/reference/templates/later-profile/decision-packet.md`, `docs/ko/reference/templates/later-profile/decision-packet.md`, and an `rg` scan for Decision Packet authority wording. The pair is semantically aligned: `DEC` is an optional full-format presentation for a specific `user_judgment`, the ordinary MVP-1 path remains a compact judgment request, the five display labels match, legacy names such as `decision_packet_id`, `judgment_category`, `judgment_route`, and `display_depth` are limited to migration or compatibility context, and `presentation=short` / `presentation=full` changes rendered context rather than authority. Prior Decision Packet authority-path wording cleanup is resolved in this pass; no checked wording treats Decision Packet as the canonical authority path for user-owned judgment. |
| Korean later-profile template localization check | Scanned all 19 files in `docs/ko/reference/templates/later-profile/` for English cue-label candidates, headings, table labels, and prose labels, and manually spot-checked `README.md`, `decision-packet.md`, `export.md`, headings, table-label hits, and targeted search hits. The folder has a Korean rendering-label rule, and the checked rendered headings and most checked prose/table labels are natural Korean while exact template IDs, schema/API names, field names, refs, placeholders, enum values, and stable lookup labels are preserved. The exact older examples `display label:`, `active path:`, `write authorization:`, `approval refs`, `Write Authority Summary`, `Close Summary`, and `QA waiver user judgment` were not found in the targeted exact scan. The cleanup is improved but not fully closed: current broader scans still show rendered table/cue-label candidates that need final classification or localization, especially `blocked_by`, `unblocks`, `parallelizable_with`, and `manifest hash`. Other hits such as `Residual Risk`, `Change Unit`, `Run`, `EVIDENCE-MANIFEST`, `source_state_version`, `run_summary`, and `approval_scope` appear in exact/stable-label, field-name, placeholder, or Korean-explained lookup contexts. Treat the remaining polish as documentation drift/localization follow-up, not runtime conformance or server-coding status. |
| Core Model judgment routes and schema wording | Manually checked `docs/en/reference/core-model.md` and `docs/ko/reference/core-model.md`. The prior Core Model wording drift is resolved in the checked source docs. The route boundary is aligned: route verbs are internal owner-path metadata, broad approval is absent from the user-facing model, display depth is presentation metadata, users see the same five display types, and the `User Judgment` summary paragraphs plus canonical-schema bullets name the same `user_judgment`, request/record actions, `judgment_type`, `presentation`, `display_label`, and compatibility/legacy terms. No remaining Core Model English/Korean User Judgment wording issue was found in this pass. |
| v01/v02 and legacy fixture identifiers | Checked with `rg` for `v0.1`, `v0.2`, `v01`, `v02`, `CORE-v01`, `MVP-v02`, `Core Authority Smoke`, and `First User-Value Slice`. Remaining matches are legacy-label guidance in translation/glossary docs plus this review's own check description. No active stage name, current fixture identifier, executable fixture claim, or current conformance result used those legacy labels. |
| Implementation-readiness wording | Checked `docs/en/build/implementation-overview.md`, `docs/ko/build/implementation-overview.md`, the MVP-1 decision-log links from those pages, Maintain guidance, and this review. The docs distinguish documentation redesign review, pending documentation acceptance, not-yet-accepted implementation-planning readiness, not-yet-accepted server-coding decisions, not-started runtime implementation, and future runtime conformance. |
| Future fixture catalog scope pressure | Manually checked `docs/en/later/future-fixtures.md` and `docs/ko/later/future-fixtures.md`, plus an `rg` scan for MVP requirement, active API/DDL, current conformance, runnable/executable fixture, runner, generated conformance, server implementation, and runtime-state wording. The catalog remains a compact future scenario-family inventory. It explicitly says rows are not fixture bodies, public request schemas, storage schemas, DDL rows, stage exit criteria, generated artifacts, runtime results, implementation tasks, MVP-1 requirements, active API/DDL, executable suites, or current conformance. Search hits were negative boundary statements or future promotion criteria, not scope pressure into MVP-1. |

## Link, Diagram, And Bilingual Review Status

Status: scriptable checks plus targeted spot checks. Do not treat this as runtime validation, runtime conformance, server-coding acceptance, implementation-planning readiness, or a full manual documentation acceptance pass.

Checks actually run during this review:

- Local relative link and anchor checker over top-level repository Markdown entrypoints and `docs/**/*.md`, including explicit `<a id="...">` / `<a name="...">` anchors: checked 131 Markdown files, 2,284 inline/reference-style local Markdown links outside fenced blocks, and 867 fragment/anchor links. The checker did not count raw autolinks or links inside fenced examples. No unresolved relative links or anchors were reported.
- English/Korean active file-map check for `docs/en` and `docs/ko`: 64 Markdown files on each side; no active file-map differences were reported.
- Fenced code block balance check over the same 131 Markdown files: 462 fenced code blocks checked; no unclosed fenced code blocks were reported.
- Legacy stage/fixture term check using `rg` for `v01`, `v02`, `CORE-v01`, `MVP-v02`, `v0.1`, `v0.2`, `Core Authority Smoke`, and `First User-Value Slice`: no active misuse was found; remaining matches are legacy-label guidance or this review's own check text.
- Decision Packet authority-path wording check using `rg` plus manual review of the Core Model and later-profile Decision Packet template: no checked wording describes Decision Packet as the authority path for user-owned judgment. It remains a full-format/later-profile or legacy presentation label for `user_judgment`.
- Security wording spot check over Security Reference and Implementation Overview: checked cooperative/detective/preventive/isolated wording and local-access non-claims; checked wording does not claim OS permission, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, permission isolation, or security isolation for early stages.
- User-language/internal-term scan using `rg` over Learn and Use docs, followed by spot review of Learn Overview, the Learn concepts page, User Guide opening and advanced-terms section, Agent Guide guidance, and the English/Korean Judgment Request Cookbook openings. Internal labels appear in explicit lookup, advanced, reference-link, stable-anchor, or "do not require this startup language" contexts, not as required user startup language in the scanned openings.
- Korean later-profile template localization check: targeted exact scan plus broader rendered-label, heading, table-label, and cue-label review found that cleanup is improved but not complete; remaining rendered table/cue-label candidates that need final classification or localization are listed above as follow-up.
- Core Model English/Korean judgment schema wording spot check: the prior wording drift is resolved. The checked summary paragraphs and canonical-schema bullets are aligned on `user_judgment`, `harness.request_user_judgment`, `harness.record_user_judgment`, `judgment_type`, `presentation`, `display_label`, and compatibility/legacy terms.
- Mermaid inventory and basic fence review: found 24 actual Mermaid fences in paired Build/Reference docs; all actual fences begin with `flowchart LR`, `flowchart TD`, or `flowchart TB`. Mermaid rendering or parser validation was not run because `command -v mmdc` found no renderer in `PATH`.
- Open-marker spot check using `rg` for `TODO_DECISION`, `TODO_IMPLEMENT`, and `TODO_REWRITE`: matches were limited to Maintain guidance and this review's own check description; no scattered implementation-decision TODOs were found.
- Future fixture catalog scope-pressure spot check: manually checked the English and Korean Future Fixtures catalogs and targeted search hits; no wording was found that turns the catalog into an MVP-1 requirement, executable fixture suite, active API/DDL, current conformance result, or implementation checklist.

Checks not run:

- Mermaid parser or renderer. `mmdc` was not available in `PATH` during this review, so Mermaid syntax/rendering was not validated.
- Full bilingual semantic review of every paired file.
- Full user-language audit across every Learn and Use sentence.
- Full owner-boundary duplicate-contract audit across all docs.
- Full Korean later-profile rendered-label cleanup. A broad scan plus targeted heading, table-label, cue-label, and prose spot check was performed, but not every remaining candidate was rewritten or definitively classified as an exact/stable identifier.
- Runtime conformance, executable fixture execution, conformance-runner checks, generated projection checks, generated operational artifact checks, or runtime state checks. Those do not exist in this documentation-only repository phase.

## Unresolved Blockers And Risks

No newly discovered owner conflict is recorded in the current MVP-1 decision log.

Known blockers before implementation planning or coding:

- Maintainer documentation acceptance is still pending.
- Implementation-planning readiness is not accepted.
- API, Storage/DDL, Core transition, and Security/local-access coding acceptances are not accepted.
- Mermaid syntax rendering, full paired-file semantic review, full manual user-language audit, full owner-boundary duplicate-contract audit, and full Korean later-profile rendered-label cleanup/classification were not completed in this review batch.
- Korean later-profile template localization polish remains for rendered template table/cue labels that may still be user-visible English or field-like labels, especially `blocked_by`, `unblocks`, `parallelizable_with`, and `manifest hash`. Exact/stable identifiers with Korean explanation remain allowed. Route this as documentation drift/localization follow-up, not as a runtime or server-coding blocker.

These blockers should be handled by maintainer acceptance review, not by creating runtime artifacts or conformance reports.

## Final Handoff Statement

The redesigned documentation is conditionally ready for maintainer implementation-planning review. It is not yet accepted implementation-ready material, and it does not authorize server coding. Maintainers should use this review with [Implementation Overview](../build/implementation-overview.md), [MVP-1 User Work Loop](../build/mvp-user-work-loop.md), and [Documentation Checks](documentation-checks.md) to make the next explicit readiness decision.
