# Build: MVP-1 User Work Loop

## What this document helps you do

Use this page to plan MVP-1 User Work Loop, the first user-value implementation milestone. It also centralizes implementation decisions that still block server coding.

Engineering Checkpoint comes first and proves an internal Core authority loop. MVP-1 User Work Loop comes after that checkpoint and proves that ordinary user work can be tracked, explained, blocked honestly, and closed or held with visible authority boundaries.

This is planning documentation only. It does not authorize runtime/server implementation, generated operational artifacts, executable fixture files, runtime data, product code, or conformance runners before the handoff gates in [Implementation Overview](implementation-overview.md#documentation-acceptance-status) are accepted.

## Read this when

- You need to distinguish internal checkpoint scope from the first user-value slice.
- You need to know what MVP-1 includes and excludes.
- You need the owner links for MVP-1 API, storage, and security without duplicating those contracts.
- You need the central decision log before server coding.

## Main idea

MVP-1 User Work Loop target:

> When a user starts or resumes work in ordinary language, Harness preserves a local basis for scope, pending user judgments, evidence summary, close blockers, next safe action, work acceptance, and residual-risk visibility.

MVP-1 is intentionally narrow. It is enough to show why Harness is more than a prompt pack or a pre-write wrapper, but it is not a full assurance system, QA matrix, evaluation harness, reporting suite, operations suite, dashboard, hosted UI, or connector platform.

## MVP-1 included

MVP-1 includes:

- ordinary-language start or resume for tracked work
- work-shape classification, including small direct change versus tracked work
- scope, non-goals, and success criteria summary
- codebase-answerable or state-answerable fact checking before asking the user to repeat facts
- minimal user judgment request and record through the owner API path
- separate display of Product/UX judgment, Technical judgment, Sensitive action approval, Work acceptance, and Residual risk acceptance when those routes are relevant
- cooperative pre-write scope checking through Core and `prepare_write`
- `record_run` plus registered artifact/evidence refs or the minimum evidence summary path
- status and next-safe-action output
- evidence summary and evidence-gap display
- close blocker summary when required evidence or required user judgment is missing
- residual-risk visibility before acceptance or close when close-relevant risk exists
- compact Core-derived views for the MVP-1 path, using the set owned by [Projection And Templates Reference](../reference/projection-and-templates.md#mvp-1-view-set) and [Template Reference](../reference/templates/README.md#mvp-1-template-set)
- honest MCP/Core unavailable behavior: no fabricated authority state when Core cannot be reached

## MVP-1 excluded

MVP-1 excludes these future buckets:

| Bucket | Keep out of MVP-1 |
|---|---|
| Assurance Profile | Verification strengthening beyond the active minimal path, full detached verification, full Manual QA matrix, detailed Evidence Manifest, detailed Eval output, full waiver machinery, full Approval lifecycle hardening, rich residual-risk lifecycle, risk-review hardening, stewardship validators, TDD trace, feedback-loop policy, and broad context-hygiene validators. |
| Operations Profile | Full report/export, recover/export, release handoff, artifact integrity operations, projection refresh/reconcile suite, doctor/readiness suite, broad operator surface, runtime conformance suite, conformance runner, generated conformance artifacts, and executable fixture catalog. |
| Roadmap | Dashboard, hosted workflow UI, artifact dashboard, rich card expansion, broad connectors, connector marketplace, team workflow, orchestration, metrics, Browser QA Capture, Cross-Surface Verification automation, hosted/remote workflows, preventive guard expansion, hooks, deployment, canary, rollback, production monitoring, and other expansion candidates. |
| Security non-claims | OS-level sandboxing, arbitrary-tool isolation, permission isolation, tamper-proof local storage, or default preventive pre-tool blocking. |

If a feature is useful but appears in the excluded buckets, keep it in [Assurance Profile](../later/assurance-profile.md), [Operations Profile](../later/operations-profile.md), [Future Fixtures](../later/future-fixtures.md), or [Roadmap](../roadmap.md) unless an owner explicitly promotes a narrower behavior with stage impact.

## MVP-1 owner docs

Build docs do not duplicate exact schemas, DDL, or API definitions. Use these owners:

| Need | Owner docs |
|---|---|
| MVP-1 public tools and resources | [MVP API](../reference/api/mvp-api.md). |
| Shared envelopes, refs, staged API values, and resources | [API Schema Core](../reference/api/schema-core.md). |
| Errors, idempotency, replay, stale-state, and state conflict behavior | [API Errors](../reference/api/errors.md). |
| Task, scope, user judgment, `prepare_write`, Write Authorization, `record_run`, evidence gates, blockers, and close semantics | [Core Model Reference](../reference/core-model.md). |
| Runtime home layout, minimal storage profile, locks, migrations, artifacts, and later-profile storage boundaries | [Storage](../reference/storage.md). |
| MVP-1 security guarantee wording and local-access posture | [Security Reference](../reference/security.md). |
| Compact derived views, projection authority boundaries, freshness, and template ownership | [Projection And Templates Reference](../reference/projection-and-templates.md), [Template Reference](../reference/templates/README.md). |
| Connector capability profiles and user-facing surface behavior | [Agent Integration Reference](../reference/agent-integration.md), [Surface Cookbook](../reference/surface-cookbook.md). |
| Future conformance model and future smoke authoring | [Conformance Fixtures Reference](../reference/conformance-fixtures.md). |

## API docs needed for MVP-1

An implementer should read these in order:

1. [MVP API](../reference/api/mvp-api.md) for the active MVP-1 public tools and resources.
2. [API Schema Core](../reference/api/schema-core.md) for envelopes, `ArtifactRef`, shared refs, staged value sets, and read-only resources.
3. [API Errors](../reference/api/errors.md) for public errors, idempotency, replay, unavailable Core/MCP behavior, and state conflicts.
4. [API Schema Later](../reference/api/schema-later.md) only when confirming that a method or field is later/profile-gated and should stay out of MVP-1.

MVP-1 should satisfy next-safe-action output through `harness.status.next_actions`; a separate `harness.next` method is later/compatibility material unless an owner promotes it.

## Storage docs needed for MVP-1

Use [Storage](../reference/storage.md) for exact DDL, runtime home layout, artifact storage, locks, migrations, and staged storage profiles.

For MVP-1 planning, storage should be limited to the owner-approved records needed for local project state, Task/scope state, user judgments, write authorizations, runs, evidence refs or evidence summaries, blockers, and minimal replay/audit support. Later-profile storage for rich Approval lifecycle, detailed Evidence Manifests, Manual QA, Eval, projection jobs, reconcile items, recover/export, validator runs, Journey records, and broad diagnostics should not be required for MVP-1 exit unless an owner promotes the specific behavior.

## Security guarantees for MVP-1

MVP-1 uses cooperative plus limited detective wording.

It can:

- require Core-compatible records before Harness-compatible product writes are recorded
- return structured blockers for missing scope, missing judgment, missing evidence, stale state, unavailable Core/MCP, or close blockers
- show honest guarantee status and evidence/risk gaps
- ask connected agents or surfaces to hold by instruction when the Harness record/check path is unavailable or incompatible

It must not claim:

- OS-level permission control
- arbitrary-tool sandboxing
- tamper-proof local files
- default pre-tool blocking
- permission isolation or security isolation
- preventive or isolated behavior unless a future promoted owner profile proves the exact covered operation

Use [Security Reference](../reference/security.md#guarantee-levels-by-stage) for guarantee levels and [API Errors](../reference/api/errors.md) for user-visible unavailable or mismatch behavior.

## Implementation decisions needed before server coding

This section is the central server-coding decision log. Major implementation decisions found during review or first runtime-batch planning belong here, not as scattered open markers.

### Documentation-resolved decisions for MVP-1

These decisions are resolved in the documentation baseline but still require maintainer acceptance before coding.

| Decision | Documentation baseline | Coding boundary |
|---|---|---|
| Judgment naming | Use `UserJudgment` / `user_judgment`, `harness.request_user_judgment`, `harness.record_user_judgment`, `judgment_type`, `presentation`, and `display_label`. | Compatibility aliases must not create extra authority paths. |
| Next action | Use `harness.status.next_actions` for MVP-1 next-safe-action output. | A separate `harness.next` method stays later/compatibility unless promoted. |
| MVP-1 compact views | Use the compact Core-derived view set owned by [Projection And Templates Reference](../reference/projection-and-templates.md#mvp-1-view-set) and [Template Reference](../reference/templates/README.md#mvp-1-template-set). | These views do not authorize writes, satisfy evidence, record acceptance, accept risk, close tasks, or become canonical state. |
| Minimal storage boundary | Keep MVP-1 storage to the minimal active owner records needed for the user work loop. | Later-profile tables/records stay out unless owner docs promote them. |
| Acceptance boundaries | Sensitive action approval, work acceptance, and residual-risk acceptance stay separate. | Work acceptance is not Approval, and residual-risk acceptance is not work acceptance. |
| Small direct changes | Small changes still need explicit scope, compatible `prepare_write`, `record_run`, and required evidence support. | Small-change labeling must not bypass authority, user judgment, evidence, or risk visibility. |
| Local access and errors | Use the API, Operations, and Security owner contracts for local access, unavailable Core/MCP, state conflict, and display-safe details. | Build docs do not define new public error codes or precedence. |

### Implementation decisions still open

| Decision item | Current status | What blocks readiness |
|---|---|---|
| Implementation-readiness judgment | Not accepted. | Maintainers must update [Implementation Overview: Documentation acceptance status](implementation-overview.md#documentation-acceptance-status) after readiness criteria are satisfied or reclassified. |
| Public API coding acceptance | Not accepted for coding. | Maintainers must accept the relevant API owner docs, including active MVP-1 surface and later/profile exclusions, before coding affected tools/resources. |
| Storage/DDL coding acceptance | Not accepted for coding. | Maintainers must accept the Storage owner profile and any migrations before DDL or runtime data files are created. |
| Core transition acceptance | Not accepted for coding. | Maintainers must accept active Core state transitions, blocker semantics, and close/status behavior before coding the affected path. |
| Security/local-access acceptance | Not accepted for coding. | Maintainers must accept the local-only posture and cooperative/detective guarantee wording before exposing the API/MCP surface. |
| Newly discovered owner conflict | None currently recorded. | If review finds a real schema/design, stage-boundary, guarantee-level, fixture-semantics, or storage/API conflict, add it here with owner, stage impact, options, and decision needed before coding. |

When adding a decision, record owner document, affected behavior or field, affected stage, options considered, decision needed, and whether it blocks documentation acceptance, implementation planning, server coding, or only a later stage.

## Later profiles not to build yet

Do not build these as MVP-1 prerequisites:

| Later area | Keep out of MVP-1 |
|---|---|
| [Assurance Profile](../later/assurance-profile.md) | Verification strengthening, Manual QA, detailed evidence, risk review, detailed evaluation output, full Approval lifecycle, stewardship validators, TDD trace, feedback-loop policy, and context-hygiene validators. |
| [Operations Profile](../later/operations-profile.md) | Export, recovery, handoff, operator readiness, doctor/readiness surfaces, artifact integrity operations, projection refresh/reconcile operations, conformance runner, and broad operator surface. |
| [Roadmap](../roadmap.md) | Dashboard, hosted workflows, team workflows, broader connectors, Browser QA Capture, Cross-Surface Verification, Context Index, metrics, preventive guard expansion, hooks, permissions, orchestration, deployment, canary, rollback, production monitoring, and other expansion candidates. |

## Exit checklist

MVP-1 User Work Loop can be considered complete only when a user can observe:

- ordinary work started or resumed without knowing Harness internal labels
- scope, non-goals, success criteria, and work shape
- pending user judgments with choices and consequences where needed
- separate display of product/technical judgment, sensitive approval, work acceptance, and residual-risk acceptance
- compatible pre-write scope checks through Core
- recorded Run and evidence refs or evidence summaries
- current status, next safe action, evidence gaps, close blockers, and residual-risk visibility
- close held when required evidence or required user judgment is missing
- no fabricated authority when MCP/Core is unavailable
- compact views derived from Core records, with stale or failed freshness visible where applicable

Passing this checklist does not accept Assurance Profile, Operations Profile, Roadmap scope, or runtime conformance suites.
