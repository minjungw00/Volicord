# Build: Staged Delivery Plan

## What this document helps you do

This document turns the broad early-stage planning material into a deliberately smaller staged delivery plan. v0.1 is Core Authority Smoke: an internal smoke slice for Core-owned authority state. v0.2 is the First User-Value Slice: the first narrow user-facing slice where ordinary work can be tracked, explained, and blocked honestly without becoming a full assurance, QA, evaluation, reporting, or operations system.

This is planning documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, fixture files, or runtime data before documentation acceptance and a separate implementation-planning readiness decision. Conformance fixture documentation is a future verification plan; the current documentation-only repository does not contain runnable Harness Server conformance tests. The first runnable target is v0.1 Core Authority Smoke, with Kernel Smoke as a narrow future smoke-check authoring label. The first user-value target is v0.2 First User-Value Slice. Later packs harden agency assurance, operations, and handoff behavior. v1+ Expansion remains roadmap scope unless owner docs promote and prove it.

Use this when you need to plan what to build after documentation acceptance and a separate implementation-planning readiness decision. Use the reference docs for exact contracts.

## Read this when

- You are separating the first internal authority proof from the first user-value slice.
- You need to review staged delivery scope without expanding the first implementation batch.
- You want to keep implementation order separate from storage, schema, fixture, and template details.

## Before you read

Read [Implementation Overview](implementation-overview.md), including its [Documentation Acceptance Status](implementation-overview.md#documentation-acceptance-status), before using this stage plan. Use [First Runnable Slice](first-runnable-slice.md) for the v0.1 smoke sequence and [Runtime Walkthrough](runtime-walkthrough.md) for the request-to-close runtime path.

For exact contracts, use the [Reference Index](../reference/README.md) and pick the owner for the question in front of you. For v1+ Expansion candidates and promotion criteria, use the [Roadmap](../roadmap.md).

## Main idea

Harness value is not merely that a write authority loop exists. Harness should preserve scope, user-owned judgment, evidence references, close readiness, acceptance boundaries, and residual risk in a local authority record. Delivery therefore has two early targets:

- v0.1 Core Authority Smoke proves the smallest coherent internal Core authority loop.
- v0.2 First User-Value Slice proves that ordinary users can start or resume tracked work, see the scope and judgment boundaries, and understand evidence, blockers, next action, and residual-risk visibility without a full assurance system.

The first slice stays intentionally narrow. It proves one local project registration, one active Task, one scoped boundary, one `prepare_write` authority path, one single-use Write Authorization, one recorded Run, one artifact/evidence reference, and one structured status/blocker response. It is not a product MVP. v0.2 begins user value when the user-facing path can translate normal work into scope, non-goals, success criteria, user-owned judgment, evidence summary, close blockers, and residual-risk visibility without confusing sensitive-action approval, work acceptance, and risk acceptance.

Projection-template polish, detailed reports, dashboards or hosted workflow UI, indexes, broad connector ecosystems or marketplaces, team workflow, surface-specific connector automation, metrics, parallel orchestration, and broad automation become useful after the authority record and user-facing value path exist. They are not first-slice requirements.

The early output model is intentionally small:

- v0.1 needs only minimal status/blocker output from Core state; it does not need a projection renderer.
- v0.2 needs only a compact Core-derived status card and minimum user-readable summaries for current work status, next output, user judgment request/record, evidence summary, close blocker summary, residual-risk visibility, and separate display of sensitive approval, work acceptance, and risk acceptance. These summaries are not a full projection/reporting system.
- Journey Card, Journey Spine, Run Summary, TDD Trace, Module Map, Interface Contract, Export, detailed Evidence Manifest, and detailed Eval outputs remain Future/diagnostic projections or other later-profile scope unless an owner profile explicitly promotes them.

## Staged delivery

| Stage | Delivery target | What it proves | What it does not prove |
|---|---|---|---|
| v0.1 | Core Authority Smoke | A first runnable internal Core authority loop over one local project registration, one active Task, one scoped boundary, one `prepare_write` authority path, one single-use Write Authorization, one recorded Run, one artifact/evidence ref, and one structured status/blocker response. | User-facing product value, natural-language intake, full Discovery, full Decision Packet, full Evidence Manifest, Eval, Manual QA, Acceptance, residual-risk acceptance, full close semantics, projection rendering, conformance runner, operations/export/recover, dashboards, and connectors. |
| v0.2 | First User-Value Slice | Users can start or resume tracked work in ordinary language and see Core-derived scope, non-goals, success criteria, judgment separation, status/next output, evidence summary, close blockers, residual-risk visibility, and separated sensitive approval / work acceptance / risk acceptance display. | Full detached verification independence unless an active profile requires it, full Manual QA matrix, full waiver machinery, polished Journey/Spine/reporting, detailed Eval, TDD Trace, Module Map, Interface Contract, Export/Recover, broad connectors, operations suite, and dashboard. |
| v0.3 | Agency Assurance Pack | The v0.2 user-value path is hardened with verification, QA, residual-risk, work-acceptance, and stewardship profiles. | Operator recovery/export completeness, release handoff, broad operations coverage, roadmap automation. |
| v0.4 | Operations & Handoff Pack | The same Core model supports doctor/readiness, recover/export, artifact integrity, release handoff, and broader conformance coverage. | Dashboard, hosted workflow UI, broad connectors, Browser QA Capture automation, Cross-Surface Verification automation, Context Index, team workflow, orchestration. |

Stage map summary: staged delivery moves from a narrow Core authority smoke loop to first user value, then assurance, then operations and handoff; v1+ remains promoted roadmap scope.

```mermaid
flowchart LR
  Core["v0.1 Core Authority Smoke"] --> Value["v0.2 First User-Value Slice"]
  Value --> Assurance["v0.3 Agency Assurance Pack"]
  Assurance --> Ops["v0.4 Operations & Handoff Pack"]
  Ops -. roadmap .-> Expansion["v1+ Expansion"]
```

Kernel Smoke remains a narrow future authoring label for v0.1 Core Authority Smoke checks. The label does not make v0.1 a product MVP, and it does not require a full conformance suite, conformance runner, or future fixture catalog before the internal Core authority path is proven.

Conformance fixture profiles follow the same stage boundaries: Core Authority Smoke fixtures for v0.1 Core Authority Smoke, First User-Value Slice fixtures for v0.2 First User-Value Slice, Agency Assurance Pack fixtures for v0.3 Agency Assurance Pack, and Operations & Handoff Pack or promoted-expansion fixtures for v0.4 Operations & Handoff Pack and promoted v1+ Expansion candidates.

These fixture profile names remain the conformance labels. The hardened local reference target is only the aggregate target reached by v0.3 Agency Assurance Pack and v0.4 Operations & Handoff Pack, not a profile name or separate delivery stage.

### Security guarantee staging

Build staging does not upgrade security guarantees by itself. Security wording follows the [Security Threat Model stage map](../reference/security-threat-model.md#guarantee-levels-by-stage):

| Stage | Guarantee posture to plan for |
|---|---|
| v0.1 Core Authority Smoke | Cooperative plus limited detective behavior. Core can refuse invalid state changes and return structured blockers, but the reference path does not stop arbitrary local processes or isolate tools by default. |
| v0.2 First User-Value Slice | Cooperative/detective behavior with honest user-visible blockers, MCP availability, evidence gaps, close readiness, and honest guarantee display. |
| v0.3 Agency Assurance Pack | Stronger separation and detective assurance around verification, Manual QA, residual risk, work acceptance, Approval, and stewardship. |
| v0.4 Operations & Handoff Pack | Detective operations around doctor/readiness, recover/export, artifact integrity, projection freshness, and release handoff. |
| v1+ Expansion | Preventive or isolated candidates only after owner docs implement and prove exact covered operations or real isolation boundaries. |

### API surface by stage

The MCP API reference defines exact schemas for every method it documents. Staged delivery decides when a method/profile is active. Use the API [Stage Profile Manifest](../reference/mcp-api-and-schemas.md#stage-profile-manifest) as the owner table; later-profile fields stay exact for their profile, but they are not part of an earlier stage exit.

| Stage | Active API surface | Later-profile fields to keep out of the stage exit |
|---|---|---|
| v0.1 Core Authority Smoke | Minimal `harness.status` status/blocker read, `harness.prepare_write`, `harness.record_run`, one owner-valid active Task/scope setup path, and optionally minimal `harness.next` or a narrow `harness.close_task` blocker smoke. | Natural-language intake, full Discovery, full Decision Packet, Evidence Manifest, Eval, Manual QA, Acceptance, residual-risk acceptance, full close semantics, projection rendering, conformance runner, reconcile, export/recover, broad operations. |
| v0.2 First User-Value Slice | User-facing intake/start/resume behavior, work-shape classification, `harness.status.next_actions` with optional `harness.next`, minimal `harness.request_user_judgment` / `harness.record_user_judgment` or successor naming, evidence summaries through `harness.record_run`, close blocker summaries through `harness.close_task`, and a compact Core-derived status card. | Full detached verification independence unless required by active profile/user request/task type/risk profile, full Manual QA matrix, full waiver machinery, Approval hardening, detailed Eval, TDD Trace, Module Map, Interface Contract, export/recover, broad operations. |
| v0.3 Agency Assurance Pack | `harness.launch_verify`, `harness.record_eval`, `harness.record_manual_qa`, assurance/waiver/approval/risk profiles of judgment methods, evidence/feedback/TDD profiles of `harness.record_run`, and ValidatorResult-emitting assurance paths. | Operator recover/export completeness, broad projection/reconcile operations, release handoff. |
| v0.4 Operations & Handoff Pack | Projection freshness in API responses, reconcile judgment profile, operator readiness/recover/export/artifact-integrity/conformance surfaces owned by Operations. | Dashboard, hosted workflow UI, broad connectors, automation, team workflow, orchestration unless promoted later. |

### Read-only MCP resources by stage

MCP resources are read-only and follow the same staged delivery boundary as public tools. Reading a resource must not create Task records, decisions, projection jobs, reconcile items, or state changes.

| Stage | Resource scope in stage | Keep out of the stage exit |
|---|---|---|
| v0.1 Core Authority Smoke | `harness://project/current`, `harness://task/active`, `harness://task/{task_id}`, and optional `harness://task/{task_id}/summary` / `harness://status/card` for current state, blockers, write authority, and minimal Run/artifact/evidence refs. | Journey, Spine, Decision Packet storage, Evidence Manifest, bundle, reports, design/domain maps, module maps, interface contracts, projection jobs, and full projection rendering. |
| v0.2 First User-Value Slice | v0.1 resources plus minimal user-judgment context for current work. Evidence summary, close blocker summary, work-acceptance display, sensitive-approval display, and residual-risk visibility can appear through status/card or task summary output. | Detailed Evidence Manifest resource, detached verification/QA resources unless profile-required, reports, bundles, Journey/Spine polish, design maps, module maps, interface contracts, export/recover. |
| v0.3 Agency Assurance Pack | Profile-gated assurance reads such as `harness://policy/sensitive-categories` and `harness://task/{task_id}/evidence-manifest` when evidence/assurance support is enabled. | Operator report/export completeness and broad operations resources. |
| v0.4 Operations & Handoff Pack | Operations reads such as broad `harness://project/surfaces`, `harness://task/{task_id}/reports/latest`, and `harness://task/{task_id}/bundle/current` when connector freshness, report, export, recover, or handoff profiles are in scope. | Dashboard, hosted workflow UI, broad connector automation, and roadmap resources unless promoted later. |
| Future/diagnostic | Owner-promoted reads such as `harness://task/{task_id}/spine`, `harness://task/{task_id}/journey`, `harness://task/{task_id}/change-unit-dag`, `harness://design/domain-language`, `harness://design/module-map`, and `harness://design/interface-contracts`. | Treating diagnostic resources as required for v0.1 or minimum v0.2. |

### Operator surface by stage

Operator commands are illustrative implementation choices. The stage requirement is the behavior, not the final command spelling.

| Stage | Operator behavior in scope | Operator behavior outside the stage |
|---|---|---|
| v0.1 Core Authority Smoke | Minimal local connect/register, basic status or diagnostic read, and local API/MCP exposure only if the first slice requires that boundary. | Projection refresh, reconcile, recover, export, artifacts check, conformance runner, release handoff, and broad doctor/readiness. |
| v0.2 First User-Value Slice | The same minimal surface plus user-facing status/next diagnostics for current work, user judgments, evidence state, close blockers, residual-risk visibility, and separated sensitive approval / work acceptance / risk acceptance display. | Assurance operations, recover/export, release handoff, broad projection/reconcile operations, full conformance run, and broad operations coverage. |
| v0.3 Agency Assurance Pack | Assurance-profile support for verification, Manual QA, residual-risk, work-acceptance, stewardship, and context-hygiene behavior through owner paths. | Operator recover/export completeness, release handoff, broad projection/reconcile operations, and full operations conformance. |
| v0.4 Operations & Handoff Pack | Full local operations support: doctor/readiness, projection refresh, reconcile, recover, export, artifacts check, release handoff where defined, and conformance run after runtime suites are materialized. | Remote/shared operations, dashboards, hosted workflow UI, broad connector automation, team workflow, and orchestration unless later promoted. |
| v1+ Expansion | Promoted roadmap operations only after owner docs define exact contracts, guarantee level, fixtures, and fallback behavior. | Unpromoted roadmap candidates remain outside staged delivery. |

### Boundary after staged delivery: v1+ Expansion

v1+ Expansion is roadmap scope, not a Build-owned staged delivery phase. Dashboard, hosted workflow UI, Browser QA Capture automation, Cross-Surface Verification automation, Context Index, broader connectors, metrics, team workflow, orchestration, and similar candidates stay outside v0.1 through v0.4 unless owner docs explicitly promote and prove a future item.

## v0.1 Core Authority Smoke

v0.1 is an internal Core authority smoke slice for implementer confidence. It should prove only the smallest coherent loop that makes Harness a local authority record instead of chat memory or generated Markdown. It is not user value validation and must not be described as a product MVP.

v0.1 must prove:

- one local project registration
- one active Task in Core-owned state
- one scoped boundary for the intended change, represented by the Change Unit owner shape only where the reference contract requires it
- one `prepare_write` allow/structured-blocker path
- one durable single-use Write Authorization
- one `record_run` that consumes that authorization
- one registered `ArtifactRef` or equivalent evidence reference owned by Core/API contracts
- one structured status/blocker response for missing scope, missing write authority, or missing artifact/evidence support

The matching storage profile is [Storage And DDL: Core Authority Smoke schema](../reference/storage-and-ddl.md#core-authority-smoke-schema). That profile is the v0.1 minimum. User-facing Decision Packet tables, Approval records, Evidence Manifest, Manual QA, Eval, residual-risk acceptance records, projection jobs, reconcile items, validator runs, Journey records, and diagnostic/stewardship tables remain later-profile storage unless a profile owner explicitly promotes them.

v0.1 explicitly excludes natural-language intake, full Discovery, full Decision Packet, full Evidence Manifest, Eval, Manual QA, Acceptance, residual-risk acceptance, full close semantics, detached verification, product/UX versus architecture judgment presentation, stewardship, feedback-loop policy, projection rendering, conformance runner, operations/export/recover, dashboards, connectors, broad operator entrypoints, future fixture catalog, and release handoff. Those are later stages or roadmap scope.

Kernel Smoke candidates for v0.1 should assert only the minimal authority loop through Core state, the required owner records for that loop, artifact/evidence refs, and structured blockers. Projection polish, detailed templates, renderer output, and broad fixture catalogs are not first-slice conformance truth.

At this point, an implementer can observe that Core owns the minimal state, a scoped write is allowed or rejected with a structured blocker, one authorization is consumed once, an artifact/evidence ref is linked to the recorded Run, and status/blocker output can return structured blockers. This is implementer confidence, not proof that users experience Harness value.

### Contract field staging

Reference schemas may list fields that become necessary only when the related capability is in scope. Build does not redefine field requiredness; it tells implementers when a capability enters the staged plan. Read each field through the owner contract and the active stage:

| Stage | Build reading rule | Owner contracts to apply |
|---|---|---|
| v0.1 Core Authority Smoke | Use only the owner-defined fields needed to prove the narrow authority loop and the [Core Authority Smoke schema](../reference/storage-and-ddl.md#core-authority-smoke-schema). Avoid creating future-profile records just to satisfy a broad checklist; if a minimal seeded blocker uses an owner ref, apply only the valid shape for that owner path, not profile-specific user-facing Decision Packet quality. | [Kernel Reference](../reference/kernel.md), [MCP API And Schemas](../reference/mcp-api-and-schemas.md), [Storage And DDL](../reference/storage-and-ddl.md), [Conformance Fixtures Reference](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue). |
| v0.2 First User-Value Slice | Add the fields and display summaries needed for users to understand current work shape, scope/non-goals/success criteria, pending user judgment, evidence summary, close blockers, residual-risk visibility, and separated approval/acceptance/risk-acceptance displays. Work-acceptance and residual-risk facts stay distinct when relevant, but they fit inside the minimal summaries. | [MCP API And Schemas](../reference/mcp-api-and-schemas.md), [Kernel Reference](../reference/kernel.md), [Document Projection Reference](../reference/document-projection.md), [Template Reference](../reference/templates/README.md). |
| v0.3 Agency Assurance Pack / v0.4 Operations & Handoff Pack | Add verification, QA, residual-risk, work-acceptance, stewardship, projection/reconcile, operations, export/recover, artifact-integrity, and release-handoff profiles only where owner docs define them. | [Design Quality Policies](../reference/design-quality-policies.md), [Operations And Conformance](../reference/operations-and-conformance.md), [Conformance Fixtures Reference](../reference/conformance-fixtures.md), [Future Fixture Catalog](../reference/future-fixture-catalog.md), [Storage And DDL](../reference/storage-and-ddl.md). |

Required in an API schema therefore means required when that tool call, record, or profile is implemented or used. It does not make a future-profile field part of the smallest runnable slice by itself.

### Implementation decisions needed before server coding

This open decision ledger is the central server-coding decision log for decisions found during maintainer review or first runtime-batch planning. Do not create scattered `TODO_DECISION` markers or vague follow-ups for major implementation choices.

| Decision-log item | Current status | Decision condition |
|---|---|---|
| Simplified judgment model and naming | Open. Current docs point toward user-facing judgment categories and internal routes, but v0.2 must settle the minimum model before API/DDL coding. | Decide the v0.2 record names, required fields, display labels, and relationship to later full Decision Packet semantics. |
| `request_user_decision` vs `request_user_judgment` | Open. Current docs mostly use `harness.request_user_judgment`; naming still needs maintainer acceptance before public API freeze. | Choose the method name and migration path, or explicitly accept `harness.request_user_judgment` as the v0.2 public name. |
| `harness.next` separate method vs `status.next_actions` | Open. v0.1/v0.2 may not need both. | Decide whether v0.2 exposes next action as a separate method, a field on `harness.status`, or both with one canonical source. |
| v0.2 storage minimum | Open. v0.2 needs minimal user judgment, status/card, evidence summary, blocker, acceptance display, and residual-risk visibility, but not later assurance tables by default. | Accept the exact minimum tables/fields and explicitly defer later-profile storage. |
| Local access error taxonomy | Open. MCP/Core unavailable, local access denied/untrusted, stale state, and unsupported surface need stable user-visible and API error handling. | Accept error codes, precedence, and display wording for v0.1/v0.2. |
| Compact status card scope | Open. v0.2 needs a Core-derived compact card, but the included fields and freshness rules are not yet accepted. | Decide required card fields, omitted fields, stale/unavailable behavior, and whether it is a resource, status payload, or both. |
| Small direct change evidence requirement | Open. Small direct work must not bypass authority, but the minimum evidence expectation needs a stage decision. | Decide what evidence summary/ref is required for small direct changes and when a missing ref blocks close. |
| Acceptance and residual risk minimal records | Open. v0.2 displays sensitive approval, work acceptance, and risk acceptance separately, but the minimum record shape needs acceptance. | Decide whether v0.2 creates minimal records, display-only derived state, or a staged subset of later acceptance/risk records. |
| Implementation-readiness judgment | Not accepted. | Maintainers must deliberately update [Implementation Overview: Documentation acceptance status](implementation-overview.md#documentation-acceptance-status) after the readiness criteria are satisfied or remaining blockers are reclassified. |
| Documentation drift | Not a server-coding decision by default. | If a docs-maintenance finding exposes a real owner-contract decision or stage blocker, promote it into this log with stage impact; otherwise route it through the Authoring Guide tracker. |

When a confirmed decision is added, record:

- owner document or owner section
- affected behavior, field, table, fixture semantics, guarantee level, or stage boundary
- affected stage
- options considered
- decision needed before server code or DDL changes
- whether the item blocks documentation acceptance, implementation planning, server coding, or only a later stage

### Implementation readiness checklist

This checklist is not accepted yet. Maintainers must accept each item, or explicitly defer it with stage impact, before first runtime-batch planning or server coding begins.

- v0.1 API subset accepted.
- v0.1 DDL accepted.
- State transitions accepted.
- Write Authorization lifecycle accepted.
- Artifact/evidence ref shape accepted.
- Structured blocker shape accepted.
- Local access posture accepted.
- v0.2 promotion criteria accepted.

### Core Authority Smoke flow

Core Authority Smoke summary: this planning flow proves one authority loop around project/Task setup, scope, `prepare_write`, Write Authorization, `record_run`, artifact/evidence refs, and structured status/blocker output. It is not an implemented runtime flow in this repository today.

```mermaid
flowchart LR
  Register["project registered"] --> Task["Task"]
  Task --> Scope["scope"]
  Scope --> Check["prepare_write"]
  Check -->|allowed| Authorization["Write Authorization"]
  Authorization --> Run["record_run"]
  Run --> Evidence["ArtifactRef"]
  Check -->|not allowed| Blocker["structured blocker"]
  Evidence --> Status["status / next action<br/>or blocker"]
  Blocker --> Status
```

Exact state and blocker behavior is owned by [Kernel Reference](../reference/kernel.md), public tool shapes by [MCP API And Schemas](../reference/mcp-api-and-schemas.md), and future fixture semantics by [Conformance Fixtures Reference](../reference/conformance-fixtures.md#conformance-fixture-format). This flow does not add pack gates, projection-renderer requirements, or fixture body requirements.

For future smoke authoring order, use the [Kernel Smoke Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue). It maps candidate checks to this internal slice without implying executable fixture files already exist or that v0.1 requires a full conformance suite.

## v0.2 First User-Value Slice

v0.2 is the first user-value slice. It is not a full product MVP, assurance system, QA matrix, evaluation harness, reporting suite, operations suite, or dashboard. It is defined by the smallest ordinary-language experience that lets a user see Harness preserving scope, user-owned judgment, evidence summary, close blockers, and residual-risk visibility in Core-owned local state.

The slice must demonstrate:

- ordinary-language start or resume of tracked work without requiring Harness vocabulary
- work shape classification, including a small direct change vs tracked work distinction
- scope, non-goals, and success criteria summary
- codebase-answerable or state-answerable facts are checked before asking the user to repeat them
- clarification asks enough to unblock the next safe action without dumping a long questionnaire
- product/UX judgments and material technical architecture judgments can be presented separately from each other and from sensitive-action approval, work acceptance, and risk acceptance
- minimal user judgment request and record
- small changes and tracked work have different procedural budgets without letting small-change labeling bypass authority
- ambiguous feature requests enter clarification instead of premature implementation
- status and next-output explain current scope, missing judgments, evidence state, close blockers, residual-risk visibility, and safe next action
- evidence summary
- close blocker summary when required evidence or a required user-owned judgment is missing
- residual risk visibility before acceptance and close when known close-relevant risk exists
- sensitive-action Approval, work acceptance, and risk acceptance are displayed separately
- a compact status card derived from Core state, not from chat or rendered Markdown
- ambiguous consent such as "go ahead" or "looks good" does not resolve ambiguous judgment routes, waive evidence, accept residual risk, or authorize out-of-scope work
- MCP/Core unavailable status does not fabricate authority state
- projection/template output remains derived and cannot become state
- verification is required only when the active profile, user request, task type, or risk profile requires it
- verification waiver is needed only when required verification is intentionally skipped
- readable summaries or cards show current work status, user judgment request, evidence summary, and close blockers without template polish becoming the source of truth

Evidence records, readable summaries, and projection freshness support this experience. They are not the identity of the stage, and projection polish beyond this compact user-readable path stays out of scope.

v0.2 explicitly excludes full detached verification independence unless an active profile requires it, the full Manual QA matrix, full waiver machinery, polished Journey/Spine/reporting, detailed Eval, TDD Trace, Module Map, Interface Contract, Export/Recover, broad connectors, operations suite, dashboard, stewardship validators, feedback-loop policy, release handoff, detailed Evidence Manifest, Browser QA Capture, Cross-Surface Verification automation, Context Index, metrics, team workflow, and orchestration.

Passing v0.2 means a user can see why Harness is more than an authorization wrapper: it keeps the work's scope, judgments, evidence summary, close blockers, acceptance boundaries, and risk visibility locally inspectable.

## v0.3 Agency Assurance Pack

v0.3 hardens the v0.2 user-value path so the local reference path can route verification, QA, residual risk, work acceptance, and stewardship with honest boundaries.

Focus on:

- profile-specific Decision Packet quality and user-judgment routing
- sensitive-action Approval, Decision Packet, Write Authorization, work acceptance, and residual-risk acceptance separation
- detached verification independence, including same-session verification guard behavior
- Manual QA policy matrix, Manual QA blockers, and valid QA waivers
- residual-risk accepted close full semantics
- stewardship validators and codebase stewardship coverage
- TDD trace behavior where policy requires it
- feedback-loop policy where policy requires it
- context-hygiene validators and current-state versus stale-context boundaries
- Agency Assurance Pack conformance fixtures that prove judgment, QA, verification, residual-risk, and acceptance separation through Core state, events, artifacts, projection/freshness facts, and errors

Passing this pack means the user-value path is agency-preserving, policy-aware, and honest about verification, QA, residual risk, acceptance, and stewardship boundaries. It does not promote v1+ Expansion automation into staged delivery.

## v0.4 Operations & Handoff Pack

v0.4 completes the local operational proof around the same Core state model.

Focus on:

- doctor/readiness categories for runtime home, project state, artifact store, reference surface, MCP availability, projections, reconcile, validators/checks, and agency/stewardship/context
- recover handling for interrupted or drifted operational state
- export behavior for state snapshots, report projection snapshots, artifact refs, redaction status, omitted-secret notes, and retained, expired, or unavailable artifact status
- artifact integrity checks
- release handoff report/export profile where owner docs define it
- operator smoke over the v0.4 operations profile: connect, doctor, serve MCP, projection refresh, reconcile, recover, export, artifacts check, and conformance run, with earlier stages retaining only their smaller subsets
- operations/future fixture coverage for export/recover, artifact integrity, release handoff, operator readiness, and higher guarantee levels only where owner docs define and prove them
- later-boundary checks that keep roadmap items in v1+ Expansion unless separately proven and promoted

Do not create a second state model for operator commands. Operators diagnose, repair, export, or run fixtures over the same Core state model.

Docs-maintenance remains a separate read-only documentation profile. It may report documentation drift, but it is not v0.1 Core Authority Smoke, not v0.2 First User-Value Slice, not Agency Assurance Pack or operations runtime conformance, and not an implementation-readiness signal.

## Roadmap-scoped v1+ Expansion candidates

Keep these outside staged delivery unless a future plan promotes them through owner docs under the [Roadmap promotion criteria](../roadmap.md#promotion-criteria). Promotion must preserve user-owned judgment, avoid bypassing Core authority, use stage-appropriate security guarantee wording, state evidence/verification/QA/work-acceptance/residual-risk implications, avoid inflating v0.1 through v0.4, and define the needed capability profile, exact contracts, redaction/secret/PII policy, artifact retention and test-environment rules when runtime surfaces are captured, fixtures or conformance target, fallback behavior, and no projection-as-canonical dependency.

| Candidate | Stage boundary |
|---|---|
| Dashboard, hosted workflow UI, artifact dashboard, rich card expansion | May display state; must not become authority, implementation readiness, close readiness, acceptance, or risk acceptance. |
| Broad connector marketplace or surface ecosystem | May extend surfaces later; must not replace the first Core authority-loop proof or widen MCP exposure by default. |
| Browser QA Capture automation | May assist Manual QA after promotion; must not replace human QA judgment, work acceptance, or profile-required detached verification. |
| Cross-Surface Verification automation | May automate evaluator routing after promotion; must not satisfy Eval or assurance without Core-owned return records and any independence semantics required by the active profile. |
| Preventive guard expansion, native hooks, Advanced Sidecar Watcher | May strengthen surfaces after a proven pre-tool blocking or observation path; must not be claimed by label alone. |
| Context Index, Local Derived Metrics, long-term metrics | May provide read-only retrieval or diagnostics; must not authorize writes, satisfy gates, refresh projections, or close Tasks. |
| Team workflow, permissions, orchestration, parallel lanes | May coordinate future work; must not become required for staged delivery or single-project local authority. |
| Deployment, canary, rollback, merge, production monitoring | May be future integration work; release handoff remains a report/export boundary unless owner docs promote more. |

If a later feature is useful during implementation, keep it as read-only display, metadata, artifact candidate, or fixture candidate until owner docs define and prove its authority path. Build owns staged delivery; the Roadmap tracks candidate examples only.

## Exit criteria by stage

Use these as implementation-readable checklists for future runtime planning after documentation acceptance and a separate implementation-planning readiness decision. They restate staged exits; they do not add schemas, fixtures, DDL, or new runtime requirements, and they do not authorize implementation while the [Documentation Acceptance Status](implementation-overview.md#documentation-acceptance-status) still blocks first runtime-batch planning.

### v0.1 Core Authority Smoke exit checklist

- One local project is registered.
- One Task exists in Core-owned state.
- One scoped work boundary names the intended change boundary.
- Product writes without compatible scope are refused by Core with a structured blocker; this is not a default pre-tool security block.
- Out-of-scope intended writes are refused by Core with a structured blocker; this is not a default pre-tool security block.
- Allowed `prepare_write` creates a durable single-use Write Authorization.
- A compatible `record_run` consumes the authorization once.
- A second distinct product-write Run cannot reuse the consumed authorization.
- One artifact/evidence ref is registered and linked to the Run or minimal owner relation.
- Status/blocker output reports current state or a blocker without mutating state.
- A structured blocker/status response reports missing scope, missing write authority, or missing artifact/evidence support.

### v0.2 First User-Value Slice exit checklist

- Ordinary user language can start or resume tracked work without requiring Harness vocabulary.
- The user-facing path classifies work shape and distinguishes small direct changes from tracked work.
- The user-facing path summarizes scope, non-goals, success criteria, evidence expectations, close readiness, and judgment boundaries.
- Codebase-answerable or state-answerable facts are checked before asking the user to repeat them.
- Clarification quality is sufficient for the next safe action: it does not stop at one superficial question, does not dump a long questionnaire, separates blocking from useful-but-not-blocking questions, and gives choices and consequences for user-owned judgments.
- Product/UX judgment and material technical architecture judgment can be presented separately from each other and from sensitive-action approval, work acceptance, and risk acceptance.
- Minimal user judgment requests and records exist for v0.2 decisions without requiring full Decision Packet machinery.
- Small direct changes and tracked work use different procedural budgets without bypassing write authority, evidence, or a required user judgment.
- Ambiguous feature requests enter clarification instead of premature implementation.
- Status/next output explains current scope, missing judgments, evidence summary, residual-risk visibility, close blockers, next output, and next safe action.
- Close blocker summary reports when required evidence is missing.
- Close reports a blocker when a required user judgment is missing or unresolved.
- Residual risk is visible before successful acceptance or close when known close-relevant risk exists.
- Ambiguous consent phrases such as "go ahead," "looks good," "좋아," or "진행해" do not resolve ambiguous routes, waive evidence, accept residual risk, or authorize out-of-scope work.
- MCP/Core unavailable status reports the lack of authority access and does not fabricate Task state, Write Authorization, evidence, approval, acceptance, or close readiness.
- Work acceptance is recorded or represented separately from sensitive-action Approval and residual-risk acceptance.
- Residual-risk acceptance, when supported, is visibly distinct from work acceptance.
- A compact status card is derived from Core records and is sufficient for the v0.2 path without making template polish authoritative.
- Projection/template output does not become state.
- Detached verification is not required by default.
- Verification is required only when the active profile, user request, task type, or risk profile requires it.
- Verification waiver is needed only when required verification is intentionally skipped.

### v0.3 Agency Assurance Pack exit checklist

- Decision Packet quality and user-judgment routing are fixture-proven.
- Sensitive-action Approval does not substitute for Decision Packets, Write Authorization, Manual QA, verification, work acceptance, or residual-risk acceptance.
- Detached verification independence and same-session verification guard behavior are fixture-proven.
- Manual QA policy matrix and QA blockers are fixture-proven where policy requires them.
- Risk-accepted close cites accepted Residual Risk refs under the owner semantics.
- Stewardship validators, feedback-loop policy, TDD trace behavior, and context-hygiene behavior are covered where policy requires them.
- Agency conformance proves Journey visibility, user-judgment routing, Autonomy Boundary respect, distinct judgment categories/routes, and residual-risk handling.

### v0.4 Operations & Handoff Pack exit checklist

- Doctor/readiness reports runtime home, project state, artifact store, reference surface, MCP availability, projections, reconcile, validators/checks, and agency/stewardship/context categories.
- Recover handles interrupted or drifted operational state without treating recovery artifacts as successful completion proof.
- Export includes state snapshots, report projection snapshots, artifact refs, redaction status, omitted-secret notes, and retained, expired, or unavailable artifact status.
- Artifact integrity check reports missing or mismatched artifacts through existing diagnostics.
- Release handoff report/export behavior follows its owner profile without taking over deployment, merge, rollback, or production authority.
- Operations/future fixture coverage proves export/recover, artifact integrity, release handoff, operator readiness, and promoted higher guarantee levels through exact-shape fixtures, not prose.
- Later-boundary checks keep v1+ Expansion items out of staged delivery unless owner docs promote and prove them.

## Observable by stage

| Stage | What the user or operator can observe |
|---|---|
| v0.1 Core Authority Smoke | An implementer can see one local Task move through a scoped boundary, `prepare_write`, Write Authorization, `record_run`, artifact/evidence ref, and structured status/blocker output. |
| v0.2 First User-Value Slice | A user can see ordinary work clarified into scope, non-goals, success criteria, user-owned judgment, evidence summary, close blockers, work acceptance display, and residual-risk visibility, with close reporting a blocker when required evidence or a required user judgment is missing. |
| v0.3 Agency Assurance Pack | The local path explains verification, Manual QA, residual-risk acceptance, work acceptance, stewardship, TDD, feedback, context hygiene, and close behavior through Core records and fixtures. |
| v0.4 Operations & Handoff Pack | Operators can diagnose, recover, reconcile, export, check artifacts, run conformance, and prepare release handoff over the same Core state. |

After staged delivery, promoted roadmap items can read, display, wrap, or extend the authority loop only after owner docs define exact contracts and fixture coverage.
