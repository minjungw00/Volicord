# Build: MVP Plan

Use this as the single Build entry point before server coding. It records repository status, the current MVP slice, excluded later material, the first smoke target, request-to-close planning, server-coding decisions, Reference owners, and documentation-planning exit criteria.

Build docs are planning guidance only. They do not define exact schemas, DDL, API request/response shapes, storage tables, projection template bodies, fixture formats, or security guarantees. Those contracts stay with the Reference owners linked below.

<a id="documentation-acceptance-status"></a>
## Repository status

This repository is documentation-only and remains in documentation review. It is source material for a future Harness Server; it is not a Harness Server implementation, Harness Runtime Home, Product Repository, runtime record store, or implementation-complete behavior.

Server coding must not begin until maintainers resolve, accept, or explicitly defer the implementation-blocking decisions in [Implementation decisions before server coding](#implementation-decisions-before-server-coding) with named scope impact.

## What exists now

- Paired English and Korean planning docs in the compact Start, Use, Build, Reference, Later, and Maintain structure.
- Reference owner documents for Core, API, storage, security, projection/templates, agent integration, runtime boundaries, conformance, design quality, and glossary contracts.
- One Later candidate index for material outside the active MVP.
- `docs/doc-index.yaml` for bilingual retrieval routing and one-language-per-`doc_id` context discipline.
- This Build plan as the central implementation-planning entry point.

## What does not exist now

- Server/runtime implementation.
- Executable conformance runner.
- Generated runtime reports.
- OS-level permission control.
- Arbitrary-tool sandboxing.
- Tamper-proof storage.
- Default pre-tool blocking.
- Active operations profile.

## Active MVP slice

The active MVP is the smallest user work loop that proves the product thesis: Harness is a local authority record for scope, user-owned judgment, evidence, verification expectations, close readiness, and residual-risk visibility. It is not a prompt pack and not an enforcement sandbox.

The active slice includes:

- ordinary-language start or resume for tracked work
- work-shape classification, scope, non-goals, success criteria, current status, and next safe action
- minimal user judgment handling through the owner path, with sensitive-action approval, final acceptance, waiver, verification-risk acceptance, and residual-risk acceptance kept distinct when relevant
- one reference `capability_profile` for `surface_id=reference-local-mcp`, including honest unavailable/fallback behavior and guarantee display
- cooperative pre-write scope checking through `prepare_write`
- single-use Write Authorization behavior where the Core owner requires it
- `record_run` for recording work/checks and linking registered artifact/evidence refs or the compact evidence summary path
- compact Core-owned evidence summary rather than a full Evidence Manifest
- close blockers for insufficient required evidence, unresolved required judgment, missing required final acceptance, or close-relevant residual risk that is not visible at the required level or is not accepted when the active close path requires acceptance
- residual-risk visibility before close when close-relevant risk exists
- compact derived outputs for the current user work loop, with projections treated as derived reads rather than authority

Current MVP wording is cooperative with limited detective visibility. It must not claim OS-level permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, permission isolation, or security isolation.

## Excluded later material

The following material stays outside the active MVP unless an owner document promotes a narrow behavior with scope, fallback behavior, and proof expectations:

- Full Evidence Manifest, detailed evidence catalogs, persisted Journey Cards, detailed run reports, TDD Trace, Module Map, Interface Contract, Domain Language, export reports, rich templates, and later-profile templates
- detached Eval, detached verification hardening, full Manual QA, full waiver machinery, rich approval lifecycle, rich residual-risk lifecycle, and broad stewardship or context-hygiene validators
- dashboard, hosted UI, artifact dashboard, hosted connector registry, connector marketplace, broad connector ecosystem, cross-surface orchestration, team workflow, metrics, and automation candidates
- active operations profile, doctor/readiness suites, recover/export flows, artifact integrity operations, release handoff, projection refresh/reconcile operations, broad operator coverage, conformance runner, executable fixture catalog, and generated conformance artifacts
- preventive guard expansion, native hook expansion, broad isolated execution, permission isolation, deployment, canary, rollback, and production monitoring

Reference-schema presence does not expand the active MVP. Required fields apply only when the owning tool, record, or promoted later candidate is active or actually used.

## First internal smoke target

The first internal smoke target is not the product MVP and not a conformance runner. It is the narrowest planned check that can exercise the Core record/state-transition path before the user-facing loop is broadened.

It should show:

- one local project registration and one reference `capability_profile`
- one active Task and one active Change Unit or owner-approved scope boundary
- `prepare_write` compatible, blocked, dry-run, and replay behavior through the owner path
- one compatible `record_run` that consumes a required Write Authorization once
- blocked handling for missing, stale, consumed, or observed-outside-authorized-scope attempts without creating completion evidence
- one artifact/evidence ref and a compact evidence coverage/gap read
- status/blocker output that reads Core state without mutating it
- a narrow close-blocker read that can show missing evidence, unresolved judgment, or visible residual risk without implementing full assurance close semantics

This smoke target may use an owner-valid setup or seed path instead of ordinary-language intake. It does not require generated runtime reports, full projection rendering, a dashboard, hosted UI, active operations profile, executable fixtures, or broad connector support.

## User work loop

The user work loop starts or resumes ordinary work without requiring the user to know Harness internal labels. The loop should clarify what the user wants, what the repository or Harness state can support, what remains uncertain, and what judgment the user still owns.

MVP shaping persists only through active Task, scope/Change Unit, and user judgment owner paths. Shaping itself does not create separate committed planning, evidence, acceptance, residual-risk, or close records unless the relevant owner path explicitly records that item.

The next safe action must remain visible. If Core, MCP, or the reference surface cannot support a claim, status must say so instead of fabricating authority.

## Request-to-close path

1. The user asks for work in ordinary language.
2. Harness shapes or resumes a Task, summarizes scope and non-goals, and asks for minimal user judgment only when the user owns the decision.
3. Before a product write, the agent or surface calls `prepare_write`; compatible work receives the owner-defined Write Authorization result, and incompatible work returns a blocker or owner-defined error.
4. After the write or direct work, `record_run` records what happened and links registered artifact/evidence refs or the compact evidence summary path.
5. Status and compact outputs show current scope, pending judgments, evidence gaps, blockers, next safe action, guarantee level, and residual-risk visibility as derived reads from Core records.
6. `close_task` either closes through the owner-defined active path or returns close blockers. MVP close keeps final acceptance, residual-risk acceptance, verification, QA, and evidence sufficiency distinct.

`compatible`, `blocked`, and `allowed` are Harness record-compatibility results. They do not mean physical OS blocking, arbitrary-tool prevention, sandbox isolation, or permission isolation unless a future promoted mechanism proves that exact behavior.

<a id="implementation-decisions-before-server-coding"></a>
## Implementation decisions before server coding

Server coding must not begin until each row is accepted, decided, or deferred with explicit scope impact by maintainers.

| Decision item | Current status | What must be decided before coding |
|---|---|---|
| Implementation-planning readiness | Not accepted. | Maintainers must accept that the compact documentation set is ready for the first runtime-batch plan, or name the blocker and affected scope. |
| Core transition acceptance | Not accepted for coding. | Active Task/scope, `user_judgment`, `prepare_write`, Write Authorization, `record_run`, blocker, status, compact evidence summary, residual-risk visibility, and `close_task` semantics must be accepted for active MVP paths. |
| Public API and schema acceptance | Not accepted for coding. | The active MVP method set, API request/response shapes, shared schemas, resources, errors, idempotency/replay behavior, unavailable Core/MCP behavior, and later-candidate exclusions must be accepted before affected tools or resources are coded. |
| Storage and runtime-home acceptance | Not accepted for coding. | The minimal storage profile, runtime home layout, locks, artifact refs, migrations, replay/audit needs, and later-candidate storage boundary must be accepted before DDL, runtime data files, or artifact storage are created. |
| Security and local-access acceptance | Not accepted for coding. | The local-only posture and cooperative/limited-detective security guarantee wording must be accepted before API/MCP exposure. MVP must not claim OS-level permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, permission isolation, or security isolation. |
| Surface and compact-output acceptance | Not accepted for coding. | The one reference `capability_profile`, compact user-facing views, compact agent-facing packet, freshness/unavailable behavior, and projection-as-derived-read boundary must be accepted before display or connector code is implemented. |

## Reference owners

Build summarizes sequence and scope only. Use these Reference owners for exact contracts:

| Need | Owner docs |
|---|---|
| Core authority, Task/scope lifecycle, user judgments, `prepare_write`, Write Authorization, `record_run`, blockers, status, evidence gates, residual risk, and `close_task` | [Core Model Reference](../reference/core-model.md). |
| Active public API methods | [MVP API](../reference/api/mvp-api.md). |
| Shared envelopes, refs, value sets, resources, and active schema shapes | [API Schema Core](../reference/api/schema-core.md). |
| Public errors, idempotency, replay, stale-state, and state-conflict behavior | [API Errors](../reference/api/errors.md). |
| Storage layout, DDL, locks, migrations, artifact refs, and later-candidate storage boundaries | [Storage](../reference/storage.md). |
| Security guarantee wording, local-access posture, trust boundaries, and non-claims | [Security Reference](../reference/security.md). |
| Compact derived views, projection authority boundaries, freshness, and active template ownership | [Projection And Templates Reference](../reference/projection-and-templates.md). |
| Reference surface `capability_profile`, connector behavior, context surfaces, and fallback semantics | [Agent Integration Reference](../reference/agent-integration.md). |
| Product Repository, Harness Server, Runtime Home, process boundaries, and non-isolation claims | [Runtime Boundaries Reference](../reference/runtime-boundaries.md). |
| Future fixture shape, assertion authority, and conformance meaning without an executable runner | [Conformance Reference](../reference/conformance.md). |
| Design-quality activation, close-blocking findings, waiver boundary, and validator IDs | [Design Quality](../reference/design-quality.md). |
| Official terminology | [Glossary Reference](../reference/glossary.md). |
| Later candidates and promotion rule | [Later Candidate Index](../later/index.md). |

## Exit criteria for documentation planning

Documentation planning can exit only when maintainers explicitly confirm:

- this Build plan is the active Build entry point
- the active MVP boundary and excluded later material are accepted, or any remaining boundary issue has a named scope impact
- every server-coding decision above is decided, accepted, or deferred with named scope impact
- Reference owners agree on the active Core, API, storage, security, projection/template, agent-integration, runtime-boundary, conformance, design-quality, and glossary boundaries needed for the active MVP
- English and Korean Build pages preserve the same implementation decisions and active MVP boundary
- no later-candidate material is presented as required for the active MVP
- documentation remains source material only, with no server/runtime code, generated runtime state, executable fixture, conformance result, generated runtime report, or product implementation output created here

Passing these documentation-planning criteria does not implement Harness, prove runtime conformance, or close any future product work.
