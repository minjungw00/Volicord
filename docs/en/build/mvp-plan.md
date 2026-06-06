# Build: MVP Plan

Use this as the single Build entry point before coding. It keeps the active MVP boundary, first smoke target, request-to-close path, server-coding decisions, and Reference owner links in one compact place.

Build docs are planning guidance only. They do not define exact schemas, DDL, API request/response shapes, storage tables, projection template bodies, fixture formats, or security guarantees. Those contracts stay with the Reference owners linked below.

<a id="documentation-acceptance-status"></a>
<a id="maintainer-handoff-summary"></a>
## Repository status

This repository is documentation-only and is still in post-redesign review. It is intended to become the Harness Server source repository only after documentation acceptance and a separate implementation-planning readiness decision.

No Harness Server/runtime implementation, runtime state, generated projections, generated operational artifacts, executable fixture files, conformance runner, Harness Runtime Home contents, or product code exists here now. Documentation files are source material, not Harness runtime records.

Server coding must not begin until the open decisions in [Implementation decisions before server coding](#implementation-decisions-before-server-coding) are resolved, accepted, or explicitly deferred with stage impact by maintainers.

## What exists now

- Bilingual planning documentation for Start, Use, Build, Reference, Later, Maintain, and Roadmap readers.
- Reference owner documents for Core, API, storage, security, projection/templates, agent integration, runtime architecture, operations/conformance, design quality, and glossary terms.
- Later/Profile documents that preserve future assurance, operations, fixture, and roadmap material outside the active MVP.
- This Build plan as the compact implementation-planning route before any future server code is written.

## What does not exist now

- Server/runtime implementation code.
- Runtime state, generated operational files, generated projections, runtime artifacts, or Harness Runtime Home data.
- Executable fixture files, a conformance runner, generated conformance artifacts, or current runtime conformance results.
- Dashboard, hosted UI, connector marketplace, hosted connector registry, operations suite, or production deployment machinery.
- Product Repository code or product implementation changes.

<a id="main-idea"></a>
<a id="mvp-1-included"></a>
## Active current MVP slice

The active current MVP is the smallest user work loop that shows Harness is a local authority record for scope, user-owned judgment, evidence, close readiness, and residual-risk visibility.

It includes:

- ordinary-language start or resume for tracked work
- work-shape classification, scope, non-goals, and success criteria summary
- minimal user judgment handling through the owner API path, including separate display of sensitive-action approval, final acceptance, and residual-risk acceptance when relevant
- one reference `capability_profile` for `surface_id=reference-local-mcp`, with honest fallback and guarantee display
- cooperative pre-write scope checking through `prepare_write`
- durable single-use Write Authorization behavior where the Core owner requires it
- `record_run` plus registered artifact/evidence refs or the minimum evidence summary path
- compact Core-owned evidence summary, not a full Evidence Manifest
- current status and next safe action through the owner-defined status surface
- close blockers for insufficient required evidence, unresolved required user judgment, missing required final acceptance, or close-relevant residual risk that is not visible or accepted as required
- residual-risk visibility before close when close-relevant risk exists
- compact Core-derived outputs for the current work loop, with projections treated as derived reads rather than authority

Current MVP wording is cooperative plus limited detective. It must not imply operating-system permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, or security isolation.

<a id="mvp-1-excluded"></a>
<a id="later-profiles-not-to-build-yet"></a>
## Excluded later material

The following material stays outside the active current MVP unless an owner document explicitly promotes a narrow behavior with scope, fallback behavior, and proof expectations:

- full Evidence Manifest behavior, detailed evidence catalogs, persisted Journey Cards, detailed run reports, TDD Trace, Module Map, Interface Contract, Domain Language, Export report, and later-profile templates
- detached Eval, detached verification hardening, full Manual QA matrix, full waiver machinery, rich Approval lifecycle, rich residual-risk lifecycle, and broad stewardship or context-hygiene validators
- dashboard, hosted workflow UI, artifact dashboard, hosted connector registry, connector marketplace, broad connector ecosystem, cross-surface orchestration, team workflow, metrics, and automation candidates
- operations profile material such as doctor/readiness suites, recover/export, artifact integrity operations, release handoff, projection refresh/reconcile operations, broad operator coverage, conformance runner, executable fixture catalog, and generated conformance artifacts
- preventive guard expansion, native hook expansion, broad isolated execution, permission isolation, deployment, canary, rollback, and production monitoring

Reference-schema presence does not expand the active MVP by itself. Required fields apply only when the owning tool, record, or profile is active or used.

## First internal smoke target

The first internal smoke target is not the product MVP. It proves the smallest Core authority loop before the user-facing work loop is broadened.

It should be able to show:

- one local project registration and one reference `capability_profile`
- one active Task and one active Change Unit or owner-approved scope boundary
- `prepare_write` compatible, blocked, dry-run, and replay behavior through the owner path
- one compatible `record_run` that consumes a Write Authorization once
- blocked handling for missing, stale, consumed, or observed-outside-authorized-scope attempts without creating completion evidence
- one artifact/evidence ref and a compact evidence coverage/gap read
- status/blocker output that reads Core state without mutating it
- a narrow close-blocker check that can show missing evidence, unresolved judgment, or visible residual risk without implementing full assurance close semantics

This smoke target may use an owner-valid setup or seed path instead of ordinary-language intake. It does not require a full projection renderer, detailed templates, dashboard, hosted UI, operations suite, conformance runner, or broad connector platform.

## User work loop

The user work loop starts or resumes ordinary work without requiring the user to know Harness internal labels. The loop should first clarify what the user wants, what can be checked from the repository or Harness state, what remains uncertain, and what judgment the user still owns.

MVP shaping persists only through active Task, scope/Change Unit, and user judgment owner paths. It is not a separate committed Discovery Brief, Shared Design record, Question Queue, Assumption Register, evidence record, Write Authorization, final acceptance, residual-risk acceptance, or close record.

The loop should keep the next safe action visible. If Core, MCP, or the reference surface cannot support the claim, the status must say so rather than fabricate authority.

## Request-to-close path

1. The user asks for work in ordinary language.
2. Harness shapes or resumes a Task, summarizes scope and non-goals, and requests minimal user judgment only when the user owns the decision.
3. Before a product write, the agent or surface calls `prepare_write`; compatible work receives the owner-defined Write Authorization result, and incompatible work returns a blocker or owner-defined error.
4. After the write or direct work, `record_run` records what happened and links registered artifact/evidence refs or the compact evidence summary path.
5. Status and compact outputs show current scope, pending judgments, evidence gaps, blockers, next safe action, guarantee level, and residual-risk visibility as derived reads from Core records.
6. `close_task` either closes through the owner-defined active path or returns close blockers. MVP close must keep final acceptance, residual-risk acceptance, verification, QA, and evidence sufficiency distinct.

`compatible`, `blocked`, and `allowed` are Harness authority results. They do not mean physical OS blocking, arbitrary-tool prevention, sandbox isolation, or permission isolation unless a future promoted profile proves that exact mechanism.

<a id="implementation-decisions-needed-before-server-coding"></a>
<a id="implementation-decisions-still-open"></a>
## Implementation decisions before server coding

Server coding must not begin until each row is accepted, resolved, or deferred with explicit stage impact by maintainers.

| Decision item | Current status | What must be decided before coding |
|---|---|---|
| Implementation-planning readiness | Not accepted. | Maintainers must accept that the documentation planning baseline is ready for first runtime-batch planning, or name the remaining blocker and affected stage. |
| Public API coding acceptance | Not accepted for coding. | The active MVP method set, shared schemas, resources, errors, idempotency/replay behavior, unavailable Core/MCP behavior, and later/profile exclusions must be accepted in the API owners before affected tools or resources are coded. |
| Storage/DDL coding acceptance | Not accepted for coding. | The minimal storage profile, runtime home layout, locks, artifacts, migrations, replay/audit needs, and later-profile storage boundary must be accepted before DDL, runtime data files, or artifact storage are created. |
| Core transition acceptance | Not accepted for coding. | Active Task/scope, `user_judgment`, `prepare_write`, Write Authorization, `record_run`, blocker, status, evidence summary, and `close_task` semantics must be accepted for the active MVP paths. |
| Security and local-access acceptance | Not accepted for coding. | The local-only posture and cooperative/limited-detective guarantee wording must be accepted before API/MCP exposure. MVP must not claim OS sandboxing, arbitrary-tool isolation, tamper-proof storage, default pre-tool blocking, permission isolation, or security isolation. |
| Surface and compact-output boundary | Not accepted for coding. | The one reference `capability_profile`, compact user-facing views, compact agent-facing packet, freshness/unavailable behavior, and projection-as-derived-read boundary must be accepted before display code is implemented. |
| Newly discovered owner conflict | None currently recorded. | If review finds a real schema/design, stage-boundary, guarantee-level, fixture-semantics, or storage/API conflict, add it here with owner, stage impact, options, and decision needed before coding. |

<a id="mvp-1-owner-docs"></a>
<a id="api-docs-needed-for-mvp-1"></a>
<a id="storage-docs-needed-for-mvp-1"></a>
<a id="security-guarantees-for-mvp-1"></a>
## Reference owners

Build summarizes sequence and scope only. Use these Reference owners for exact contracts:

| Need | Owner docs |
|---|---|
| Active MVP public tools and resources | [MVP API](../reference/api/mvp-api.md). |
| Shared envelopes, refs, staged API values, resources, and active schema shapes | [API Schema Core](../reference/api/schema-core.md). |
| Public errors, idempotency, replay, stale-state, and state conflict behavior | [API Errors](../reference/api/errors.md). |
| Task, scope, user judgment, `prepare_write`, Write Authorization, `record_run`, evidence gates, blockers, status, and close semantics | [Core Model Reference](../reference/core-model.md). |
| Runtime home layout, minimal storage profile, locks, migrations, artifacts, and later-profile storage boundaries | [Storage](../reference/storage.md). |
| MVP security guarantee wording and local-access posture | [Security Reference](../reference/security.md). |
| Compact derived views, projection authority boundaries, freshness, and active template ownership | [Projection And Templates Reference](../reference/projection-and-templates.md). |
| Reference surface `capability_profile` and user-facing surface behavior | [Agent Integration Reference](../reference/agent-integration.md). |
| Runtime boundaries and local Core authority placement | [Runtime Boundaries Reference](../reference/runtime-boundaries.md). |
| Active design-quality blocking boundary | [Design Quality](../reference/design-quality.md#when-a-finding-blocks-close). |
| Future fixture/conformance and operations material | [Conformance Reference](../reference/conformance.md), [Later Candidate Index: Future Fixture Families](../later/index.md#future-fixture-families), and [Later Candidate Index: Operations Candidates](../later/index.md#operations-candidates). |

<a id="implementation-readiness-criteria"></a>
<a id="exit-checklist"></a>
## Exit criteria for documentation planning

Documentation planning can exit only when maintainers have explicitly confirmed:

- this single Build plan is the active Build entry point and old Build routes have been retired
- the current MVP boundary and later/profile exclusions are accepted or any remaining boundary issue is reclassified with stage impact
- the server-coding decisions above are resolved, accepted, or deferred with named stage impact
- Reference owners agree on the active API, Core, Storage, Security, projection/template, and surface boundaries needed for the active MVP
- English and Korean Build pages preserve the same implementation decisions and active MVP boundary
- no later/profile material is presented as required for the active MVP
- documentation remains source material only and no server/runtime code, generated runtime state, executable fixture, conformance result, generated operational artifact, or product implementation output has been created

Passing these documentation-planning criteria does not itself implement Harness, prove runtime conformance, or close any future product work.
