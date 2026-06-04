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

Status: targeted checks updated for the latest cleanup; not a full validation pass.

| Area | Current review finding |
|---|---|
| Later-profile Decision Packet template parity | Manually checked `docs/en/reference/templates/later-profile/decision-packet.md` and `docs/ko/reference/templates/later-profile/decision-packet.md`. The pair is semantically aligned: `DEC` is an optional full-format presentation for a specific `user_judgment`, the ordinary MVP-1 path remains a compact judgment request, the five display labels match, legacy names such as `decision_packet_id`, `judgment_category`, `judgment_route`, and `display_depth` are limited to migration or compatibility context, and `presentation=short` / `presentation=full` changes rendered context rather than authority. |
| Core Model judgment routes and display-depth semantics | Manually checked `docs/en/reference/core-model.md` and `docs/ko/reference/core-model.md`. The route boundary is aligned: route verbs are internal owner-path metadata, broad approval is absent from the user-facing model, display depth is presentation metadata, and users see the same five display types. Minor follow-up: the Korean canonical-schema bullet list is more explicit about `user_judgment`, `harness.request_user_judgment`, `presentation`, and `display_label` than the English bullet list. This is not a known contradiction, but maintainers may want to normalize the wording for easier parity review. |
| v01/v02 and legacy fixture identifiers | Checked with `rg` for `v0.1`, `v0.2`, `v01`, `v02`, and old scenario-prefix patterns. No active v01/v02 fixture identifiers were found. Historical `v0.x` stage labels remain only as legacy-label guidance in translation/glossary docs. Current behavior-example IDs use `ENG-CHECK-*`, `MVP1-*`, and `CLARIFY-*`; the illustrative `CORE-active-status-no-task` appears only in a suite metadata example, not as a current executable fixture. |
| Implementation-readiness wording | Checked in README, Build handoff, MVP-1 decision log, Maintain guidance, and this review. The docs distinguish documentation redesign review, pending documentation acceptance, not-yet-accepted implementation-planning readiness, not-yet-accepted server-coding decisions, and not-started runtime implementation. |
| Future fixture catalog scope pressure | Checked in [Conformance Fixtures Reference](../reference/conformance-fixtures.md) and [Future Fixtures](../later/future-fixtures.md). The future catalog is now a compact scenario-family inventory. Old long pseudo-fixture payloads and fixture skeletons are removed from the catalog, and catalog rows are not Engineering Checkpoint, MVP-1, current conformance, or implementation tasks. |

## Link, Diagram, And Bilingual Review Status

Status: scriptable link/anchor check plus targeted spot checks. Do not treat this as runtime validation or a full manual documentation acceptance pass.

Checks actually run during this review:

- Full local relative link and anchor checker over `AGENTS.md` and Markdown under `docs`: checked 130 Markdown files; no unresolved relative links or anchors were reported.
- English/Korean active file-map spot check using `rg --files` and `comm`: no differences reported.
- Mermaid inventory using `rg -n '```mermaid' docs/en docs/ko`: Mermaid blocks were found in paired Reference and Build docs, but syntax rendering was not run.
- Open-marker spot check using `rg` for `TODO_DECISION`, `TODO_IMPLEMENT`, and `TODO_REWRITE`: no scattered implementation-decision TODOs were found outside Maintain guidance references.
- User-language/internal-term scan using `rg` over Learn and Use docs: expected glossary, cookbook, and agent-guide uses were found; no full manual user-language audit was run.

Checks not run:

- Mermaid parser or renderer. `mmdc` was not available in `PATH` during this review.
- Full bilingual semantic review of every paired file.
- Full user-language audit across all Learn and Use docs.
- Full owner-boundary duplicate-contract audit across all docs.

## Unresolved Blockers And Risks

No newly discovered owner conflict is recorded in the current MVP-1 decision log.

Known blockers before implementation planning or coding:

- Maintainer documentation acceptance is still pending.
- Implementation-planning readiness is not accepted.
- API, Storage/DDL, Core transition, and Security/local-access coding acceptances are not accepted.
- Mermaid syntax rendering, full paired-file semantic review, full manual user-language audit, and full owner-boundary duplicate-contract audit were not run in this review batch.
- Minor documentation follow-up: normalize the English/Korean Core Model canonical-schema bullets around `presentation` and `display_label` if maintainers want easier line-by-line parity review.

These blockers should be handled by maintainer acceptance review, not by creating runtime artifacts or conformance reports.

## Final Handoff Statement

The redesigned documentation is conditionally ready for maintainer implementation-planning review. It is not yet accepted implementation-ready material, and it does not authorize server coding. Maintainers should use this review with [Implementation Overview](../build/implementation-overview.md), [MVP-1 User Work Loop](../build/mvp-user-work-loop.md), and [Documentation Checks](documentation-checks.md) to make the next explicit readiness decision.
