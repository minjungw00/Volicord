# Build: Implementation Overview

## What this document helps you do

Use this page as the Build source of truth for current repository state, maintainer handoff status, and the future implementation path. It tells a future Harness Server implementer what to read first and where exact contracts live.

This repository is documentation-only today. It is intended to become the Harness Server source repository only after documentation acceptance and a separate implementation-planning readiness decision. No Harness Server/runtime implementation, runtime state, generated operational artifacts, executable fixture files, generated projections, conformance runner, or product code exists here yet.

Build docs summarize staged implementation. They do not define exact schemas, DDL, API request/response shapes, storage tables, projection template bodies, fixture formats, or security guarantees. Those remain in the Reference owners linked below.

## Read this when

- You are reviewing whether the docs are ready for maintainer acceptance.
- You are planning future Harness Server implementation after the handoff gates are accepted.
- You need to separate the internal engineering checkpoint from MVP-1 user value.

## Main idea

Build has four active pages:

| Page | Role |
|---|---|
| [Implementation Overview](implementation-overview.md) | Current repository state, implementation approach, handoff status, readiness criteria, and reader path. |
| [Engineering Checkpoint](engineering-checkpoint.md) | First internal authority-loop smoke. It is not a product MVP and not user-value validation. |
| [MVP-1 User Work Loop](mvp-user-work-loop.md) | First user-value implementation plan, MVP-1 inclusions/exclusions, owner links, and central server-coding decision log. |
| [Runtime Walkthrough](runtime-walkthrough.md) | Design walkthrough of intended request-to-close behavior. It is not evidence that runtime exists. |

The implementation path is deliberately staged:

1. Accept documentation and implementation-planning readiness.
2. Build Engineering Checkpoint: one local Core authority loop.
3. Build MVP-1 User Work Loop: first narrow user-facing value.
4. Defer Assurance Profile, Operations Profile, and Roadmap candidates until owner docs promote them.

All implementation verbs in Build docs describe future work after the readiness gates are accepted. While [Documentation acceptance status](#documentation-acceptance-status) says implementation planning readiness is not accepted, Build is planning guidance only.

## Three implementation layers

Implementers should separate these layers before reading later-profile or Roadmap material:

| Layer | Active scope | Not in this layer |
|---|---|---|
| First executable authority loop | Engineering Checkpoint. One local project state, one registered reference `capability_profile`, one active Task, one active Change Unit/scope boundary, `harness.prepare_write`, one active single-use Write Authorization, `harness.record_run` consumption, minimal artifact/evidence recording, and a narrow status/close-blocker check. | Natural-language intake, stored user judgment flow, full close semantics, full projection renderer, rich reports, operations, conformance runner, broad connector APIs, hosted connector registry, cross-surface orchestration, or later-profile storage. |
| First user work loop | MVP-1 User Work Loop. Adds ordinary-language intake, status with `harness.status.next_actions`, the same one reference surface guarantee display, focused user judgment request/record, Core-owned evidence summary, close result/blockers, final-acceptance and residual-risk visibility, accepted-risk close when explicitly accepted, four user-facing compact outputs, and one agent-facing context packet. | Full Evidence Manifest, detached Eval, full Manual QA matrix, Assurance hardening, operations/export/recover, dashboards, hosted UI, broad connectors, hosted connector registry, cross-surface orchestration, automation, or detailed reports. |
| Later/profile scope | Assurance Profile, Operations Profile, and Roadmap. Includes full Manual QA matrix, detached Eval system, export/recover suite, dashboard/hosted UI, broad connector ecosystem, hosted connector registry, automated Browser QA Capture, preventive guard expansion, parallel orchestration, cross-surface orchestration, and detailed report projections. | Engineering Checkpoint and minimum MVP-1 exit criteria unless an owner explicitly promotes a narrow behavior with stage impact. |

The active MVP-1 method set is exactly `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, and `harness.close_task`. `harness.next` is not active MVP-1; next safe actions are represented through `harness.status.next_actions`.

The active MVP-1 compact output set is split by audience: user-facing `status-card`, `judgment-request`, `run-evidence-summary`, and `close-result`, plus agent-facing `agent-context-packet`. Persisted Journey Card, full Evidence Manifest, Eval report, Manual QA report, TDD Trace, Module Map, Interface Contract, Export report, and other detailed reports stay later/profile unless an owner explicitly marks a narrowed use as non-required or promotes it with stage impact.

## Current review baseline

The documentation set is a post-redesign review baseline and documentation acceptance candidate. It is not final accepted implementation material unless maintainers deliberately update the status table below.

Current facts:

- Documentation review status: pending maintainer acceptance.
- Implementation planning readiness: not accepted.
- Runtime implementation status: not started.
- Server-coding decisions: not accepted for coding.
- Future repository role: Harness Server source repository after acceptance and readiness, not the user's Product Repository and not a Harness Runtime Home.

If review finds a major schema/design, stage-boundary, guarantee-level, storage/API, or fixture-semantics decision that blocks coding, record it only in [MVP-1 User Work Loop: Implementation decisions needed before server coding](mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding). Do not scatter major-decision markers through active docs.

## Documentation acceptance status

This table is the maintainer-updated handoff marker. Do not infer acceptance from surrounding prose or from completed checklists.

| Status category | Current status | Boundary |
|---|---|---|
| Documentation review status | Post-redesign review; documentation acceptance candidate only. | Maintainer acceptance is still pending. Documentation acceptance does not start server/runtime implementation. |
| Implementation planning readiness | Not accepted. | First runtime-batch planning may not begin until maintainers deliberately accept readiness criteria or reclassify blockers. |
| Runtime implementation status | Not started. | No runtime/server code, runtime data, executable fixtures, generated projections, conformance results, or generated operational artifacts exist here. |
| Server-coding decision log | Not accepted for coding. | Documentation-resolved and still-open decisions are centralized in [MVP-1 User Work Loop](mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding). |

## Maintainer handoff summary

This handoff says what the docs currently define and what still blocks implementation readiness. It is a documentation handoff, not runtime state, conformance evidence, implementation authorization, or generated output.

Defined by this documentation baseline:

- Product thesis: Harness is a local authority record for scope, user-owned judgment, evidence, verification expectations, QA expectations, final acceptance, residual risk, and close readiness.
- Staged Build path: Engineering Checkpoint first, then MVP-1 User Work Loop, then later Assurance and Operations profiles.
- Owner boundaries: exact Core, API, storage, projection/template, security, operations, conformance, agent-integration, glossary, runtime-architecture, and design-quality contracts live in Reference docs.
- Documentation maintenance rules: owner boundaries, bilingual parity, status wording, link hygiene, and drift routing live in Maintain docs.

Not defined or not present today:

- Runnable Harness Server/runtime code.
- Runtime state, generated operational artifacts, generated projections, or Harness Runtime Home contents.
- Executable fixture files, fixture runners, or current runtime conformance results.
- Accepted server-coding decisions.

Current delivery meaning:

- Engineering Checkpoint proves the smallest local Core authority loop. It is internal engineering confidence, not product MVP.
- MVP-1 User Work Loop proves first user value: ordinary work can be tracked, scoped, explained, blocked honestly, and closed or held with visible evidence/judgment/risk boundaries.
- Assurance Profile and Operations Profile are later hardening. They should not be built as part of the internal checkpoint or minimum MVP-1 unless owner docs explicitly promote a narrow item.
- Roadmap remains future scope unless promoted through Roadmap criteria and owner contracts.

Before server coding starts, maintainers must deliberately update [Documentation acceptance status](#documentation-acceptance-status), accept or reclassify the [Implementation-readiness criteria](#implementation-readiness-criteria), and accept or defer the decision-log items in [MVP-1 User Work Loop](mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding) with stage impact.

## Implementation-readiness criteria

Maintainers must accept these criteria, or explicitly defer a criterion with named stage impact, before first runtime-batch planning or server coding begins.

| Criterion | What must be true | Owner path |
|---|---|---|
| Repository identity | Docs consistently say this repo is documentation-only now and future Harness Server source later. | This page, READMEs, Maintain guides. |
| Stage boundary | Engineering Checkpoint, MVP-1 User Work Loop, Assurance Profile, Operations Profile, and Roadmap are not blurred. | This page, [MVP-1 User Work Loop](mvp-user-work-loop.md), [Roadmap](../roadmap.md). |
| MVP-1 scope | MVP-1 includes user-visible scope, judgment, Core-owned evidence summary, close blockers, next action, final acceptance when required, residual-risk visibility, and accepted-risk close only through explicit residual-risk acceptance, while excluding later profiles. | [MVP-1 User Work Loop](mvp-user-work-loop.md#mvp-1-included). |
| Reference surface scope | Active MVP targets one reference `capability_profile`; capability labels do not grant write authority and unsupported fields lower or block guarantee claims. Broad connector ecosystems, hosted connector registries, and cross-surface orchestration stay later/profile. | [Agent Integration Reference](../reference/agent-integration.md#capability-profiles), [Surface Cookbook](../reference/surface-cookbook.md#reference-local-surface). |
| Design-quality blocking boundary | Active MVP design-quality blockers stay limited to Autonomy Boundary exceeded, unresolved user judgment, missing active scope, missing required evidence, stale context affecting write/close, and surface capability insufficient for a claimed guarantee; the broader policy catalog is routed candidate or advisory/later by default. | [Design Quality Policies](../reference/design-quality-policies.md#active-mvp-blocking-set). |
| API owner agreement | Active MVP-1 APIs, shared schemas, resources, errors, idempotency, and state-conflict behavior must have an accepted owner agreement before affected API implementation starts. | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md). |
| Storage owner agreement | Minimal storage profile, runtime home layout, locks, artifacts, migrations, and later-profile storage boundaries must have an accepted owner agreement before DDL, runtime-data, or artifact-storage implementation starts. | [Storage](../reference/storage.md). |
| Core owner agreement | Task, scope, user judgment, `prepare_write`, Write Authorization, `record_run`, blockers, and close semantics must have an accepted Core owner agreement for the active stages before affected Core paths are coded. | [Core Model Reference](../reference/core-model.md). |
| Security posture | MVP-1 guarantee wording and local-access posture must have an accepted Security owner agreement before API/MCP exposure; until then, wording remains cooperative plus limited detective and must not claim OS sandboxing, arbitrary-tool isolation, tamper-proof storage, default pre-tool blocking, or permission isolation. | [Security Reference](../reference/security.md). |
| Compact output boundary | Engineering Checkpoint uses status/blocker output; MVP-1 uses four user-facing compact outputs plus one agent-facing packet without making projections authoritative. | [MVP-1 User Work Loop](mvp-user-work-loop.md#mvp-1-included), [Projection And Templates Reference](../reference/projection-and-templates.md). |
| Future conformance boundary | Fixture docs remain future-oriented; no executable runner or pass/fail result is implied by documentation. | [Conformance Fixtures Reference](../reference/conformance-fixtures.md), [Future Fixtures](../later/future-fixtures.md). |
| Open decision routing | Major implementation decisions are centralized in the MVP-1 decision log, not scattered through active docs. | [MVP-1 User Work Loop](mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding). |
| English/Korean parity | Paired Build docs preserve meaning, owner links, and active file coverage while Korean remains natural. | English/Korean Build docs and Maintain guides. |

## Implementation approach

After readiness is accepted, build the smallest local system that can prove Core-owned authority before broadening into user value.

| Area | Engineering Checkpoint approach | MVP-1 approach | Exact owner |
|---|---|---|---|
| Process | One local Harness process or server with clear modules is enough. | Same local path, with user-facing intake/status behavior. | [Runtime Architecture Reference](../reference/runtime-architecture.md). |
| Core | Mutates canonical state through one authority loop. | Adds user-facing work-loop state and close/status paths without a second authority model. | [Core Model Reference](../reference/core-model.md). |
| API | Minimal status/blocker read, owner-valid setup path, `prepare_write`, `record_run`, one artifact/evidence ref path, and narrow close-blocker check. Natural-language intake is not required. | Adds the exact MVP-1 public method set for status/next actions, intake, user judgment, write checks, run/evidence, and close. | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md). |
| Surface integration | Register one reference `capability_profile` and display its limits honestly. | Reuse that profile for user-visible fallback, blocker, and guarantee display behavior. | [Agent Integration Reference](../reference/agent-integration.md), [Surface Cookbook](../reference/surface-cookbook.md). |
| Storage | Use only the minimal owner-approved persistence needed for the authority loop. | Add only the active slice needed for MVP-1 user judgment, artifacts and artifact links, minimal evidence summaries, blockers, replay/audit, and compact outputs derived from current records. | [Storage](../reference/storage.md). |
| Security | Cooperative plus limited detective. | Same baseline with clearer user-visible blockers and honest guarantee display. | [Security Reference](../reference/security.md). |
| Projections/views | Status/blocker output only; no full renderer required. | Four user-facing compact outputs plus one agent-facing packet may satisfy the user loop. | [Projection And Templates Reference](../reference/projection-and-templates.md), [Template Reference](../reference/templates/README.md). |
| Operations/conformance | Future smoke authoring plan only. | Behavior examples until runtime fixtures are materialized. | [Operations And Conformance Reference](../reference/operations-and-conformance.md), [Conformance Fixtures Reference](../reference/conformance-fixtures.md). |

## What not to build yet

These should not be built as Engineering Checkpoint or minimum MVP-1 prerequisites:

- Full Assurance Profile: detached verification hardening, Manual QA matrix, full Approval lifecycle, full residual-risk lifecycle, stewardship validators, TDD trace policy, feedback-loop policy, and broad context-hygiene validators.
- Full Operations Profile: doctor/readiness suite, recover/export, artifact integrity operations, release handoff, projection refresh/reconcile operations, conformance runner, and broad operator coverage.
- Detailed report projections: persisted Journey Card, full Evidence Manifest, Eval report, Manual QA report, TDD Trace, Module Map, Interface Contract, Export report, and other polished reports unless explicitly promoted as non-required display.
- Roadmap candidates: dashboard, hosted workflow UI, Context Index, broad connector marketplace, hosted connector registry, automated Browser QA Capture, Cross-Surface Verification automation, cross-surface orchestration, preventive guard expansion, native hook expansion, Advanced Sidecar Watcher, Local Derived Metrics, team workflow, permissions, parallel orchestration, deployment, canary, rollback, or production monitoring.

Later capabilities may read, display, or wrap the authority loop only after owner docs define exact contracts, fallback behavior, fixture/conformance expectations, redaction/secret handling where needed, and no projection-as-canonical dependency.

## Build reading path

Recommended order for future implementers:

1. [Implementation Overview](implementation-overview.md) for current status and handoff.
2. [Engineering Checkpoint](engineering-checkpoint.md) for the first internal authority-loop smoke.
3. [MVP-1 User Work Loop](mvp-user-work-loop.md) for the first user-value plan and decision log.
4. [Runtime Walkthrough](runtime-walkthrough.md) for the intended request-to-close behavior.
5. [Reference Index](../reference/README.md) to pick exact owner contracts.

Core owner links:

- [Core Model Reference](../reference/core-model.md)
- [MVP API](../reference/api/mvp-api.md)
- [API Schema Core](../reference/api/schema-core.md)
- [API Errors](../reference/api/errors.md)
- [Storage](../reference/storage.md)
- [Security Reference](../reference/security.md)
- [Projection And Templates Reference](../reference/projection-and-templates.md)
- [Runtime Architecture Reference](../reference/runtime-architecture.md)
- [Operations And Conformance Reference](../reference/operations-and-conformance.md)
- [Conformance Fixtures Reference](../reference/conformance-fixtures.md)
