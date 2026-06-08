# Build: MVP Plan

Use this as the single Build entry point for pre-server documentation planning. It records repository status, the current MVP slice, excluded later material, the first smoke target, request-to-close planning, server-coding decisions that remain blocked, Reference owners, and documentation-planning exit criteria.

Build docs are planning guidance only. They do not define exact schemas, enum value sets, DDL, API request/response shapes, storage tables, projection template bodies, fixture formats, or security guarantee claims. Those contracts stay with the Reference owners linked below.

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

The active MVP is the smallest user work loop that proves the product thesis: Harness is a local authority record for user-owned scope, judgments, write authorization, evidence, artifacts, residual risk, and closure state. It is not a prompt pack and not an enforcement sandbox.

The current active MVP scope list is deliberately closed. It includes only:

- plain-language intake and Task creation through `harness.intake`
- active scope and Change Unit updates through `harness.update_scope`
- user-owned judgment requests and recorded answers through `harness.request_user_judgment` and `harness.record_user_judgment`
- sensitive-action approval recording as the active `sensitive_approval` judgment path
- path-level `harness.prepare_write` and single-use Write Authorization for product-file writes
- `harness.record_run` for shaping, direct, and implementation Runs
- staged artifact registration through the active `stage_artifact` utility and `record_run` artifact inputs
- compact `EvidenceSummary`
- `harness.close_task` blocker calculation for close/cancel/supersede requests, without adding richer assurance close behavior
- read-time `harness.status` and compact read-time projection/status outputs
- registered local surface access for the reference local MCP surface, including `surface_id=reference-local-mcp`
- cooperative guarantee display
- detective guarantee display only when the relevant active capability check has actually passed

These items cover ordinary-language intake, status, scope updates, write compatibility, staged artifact registration, run recording, user judgments, and close checks only through their active owners. Exact active method names and active schema value sets are owned by [API Schema Core](../reference/api/schema-core.md#current-mvp-value-sets). Method behavior, storage, Core transition meaning, and security wording remain with their Reference owners. This Build plan does not promote extra enum values, extra gates, extra storage records, validators beyond `surface_capability_check`, operations, richer evidence formats, QA paths, Eval paths, connector ecosystems, export/handoff formats, projection jobs, reconcile workflows, or workflow candidates.

`stage_artifact` is the active MVP staging utility for turning owner-approved local evidence material into a safe staged handle that `record_run` may register. This Build plan names the utility only to fix the active boundary; detailed API and schema work belongs to the API and storage owners.

Current MVP wording is cooperative by default. Detective wording is allowed only for facts supported by the registered local surface and a passed capability check. The MVP must not claim OS-level permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, permission isolation, security isolation, native artifact capture, command execution observation, network observation, or secret access observation.

## Excluded later material

The following material stays outside the active MVP unless an owner document promotes a narrow behavior with scope, fallback behavior, and proof-path expectations for future promotion:

- `verification_gate`, Manual QA workflow, `qa_waiver`, `verification_risk_acceptance`, `design_gate`, `design_policy`, broader validators beyond `surface_capability_check`, detached Eval, evaluation workflows, full Manual QA, full waiver machinery, rich approval lifecycle, and rich residual-risk lifecycle
- Full Evidence Manifest, detailed evidence catalogs, persisted Journey Cards, Discovery Brief as a persistent artifact, Question Queue, Assumption Register, detailed run reports, full Decision Packet format, TDD Trace, Module Map, Interface Contract, Domain Language, rich templates, and later-profile templates
- `harness.record_manual_qa`, `harness.launch_verify`, `harness.record_eval`, later `record_run` branches, later user-judgment branches, later next-action values, later schema fields, later artifact/ref values, and `captured_artifact`
- command execution observation, network observation, secret access observation, command/network/secret observation as an active proof requirement, command/network/secret observation schema values, preventive or isolated profiles, command/network/secret pre-tool blocking, preventive guard expansion, native hook expansion, broad isolated execution, permission isolation, and stronger local capability profiles
- dashboard, hosted UI, artifact dashboard, hosted connector registry, connector marketplace, broad connector ecosystem, connector conformance ecosystem, cross-surface orchestration, team workflow, metrics, and automation candidates
- captured artifact handles, native artifact capture, active operations profile, doctor/readiness suites, recover/export flows, export/handoff formats, artifact integrity operations, release handoff, persistent projection jobs, projection refresh/reconcile operations, managed block drift repair, broad operator coverage, conformance runner, executable fixture catalog, generated conformance artifacts, deployment, canary, rollback, and production monitoring

Reference-schema presence does not expand the active MVP. Required fields apply only when the owning tool, record, or promoted later candidate is active or actually used.

## First internal smoke target

The first internal smoke target is not the product MVP and not a conformance runner. It is the narrowest planned check that can exercise the Core record/state-transition path before the user-facing loop is broadened.

It should show:

- one local project registration and one reference `capability_profile`
- one active Task and one active Change Unit or owner-approved scope boundary
- `prepare_write` compatible, blocked, dry-run, and replay behavior through the owner path
- one compatible `record_run` that consumes a required Write Authorization once
- blocked handling for missing, stale, consumed, or observed-outside-authorized-scope attempts without creating completion evidence
- one `stage_artifact`-produced staged handle, one registered artifact/evidence ref, and a compact evidence coverage/gap read
- status/blocker output that reads Core state without mutating it
- a narrow close-blocker read that can show missing evidence, unresolved judgment, or visible residual risk without implementing full assurance close semantics

This smoke target may use an owner-valid setup or seed path instead of ordinary-language intake. It does not require generated runtime reports, full projection rendering, persistent projection jobs, projection reconcile, managed block drift repair, a dashboard, hosted UI, active operations profile, executable fixtures, native artifact capture, or broad connector support.

## User work loop

The user work loop starts or resumes ordinary work without requiring the user to know Harness internal labels. The loop should clarify what the user wants, what the repository or Harness state can support, what remains uncertain, and what judgment the user still owns.

MVP shaping persists only through active Task, scope/Change Unit, and user judgment owner paths. Shaping itself does not create separate committed planning, evidence, acceptance, residual-risk, or close records unless the relevant owner path explicitly records that item.

The next safe action must remain visible. If Core, MCP, or the reference surface cannot support a claim, status must say so instead of fabricating authority.

## Request-to-close path

1. The user asks for work in ordinary language.
2. Harness shapes or resumes a Task, summarizes scope and non-goals, and asks for minimal user judgment only when the user owns the decision.
3. Before a product write, the agent or surface calls `prepare_write`; compatible work receives the owner-defined Write Authorization result, and incompatible work returns a blocker or owner-defined error.
4. Before registering artifact bytes, the active `stage_artifact` utility creates a safe staged handle; after the write or direct work, `record_run` records what happened and links registered artifact/evidence refs or the compact evidence summary path.
5. Status and compact outputs show current scope, pending judgments, evidence gaps, blockers, next safe action, guarantee display level, and residual-risk visibility as derived reads from Core records.
6. `close_task` calculates close blockers for the owner-defined active path. MVP close keeps final acceptance, residual-risk acceptance, evidence sufficiency, and later verification/QA candidates distinct without adding a current verification or Manual QA gate.

`compatible`, `blocked`, and `allowed` are Harness record-compatibility results. They do not mean physical OS blocking, arbitrary-tool prevention, sandbox isolation, or permission isolation unless a future promoted mechanism proves that exact behavior.

<a id="implementation-decisions-before-server-coding"></a>
## Implementation decisions before server coding

Server coding must not begin until maintainers mark each row accepted, decided, or deferred with explicit scope impact.

| Decision item | Current status | What must be decided before coding |
|---|---|---|
| Implementation-planning readiness | Not maintainer-accepted. | Maintainers must accept that the compact documentation set is ready for the first runtime-batch plan, or name the blocker and affected scope. |
| Core transition maintainer acceptance | Not maintainer-accepted for coding. | Active Task/scope, `user_judgment`, sensitive approval, `prepare_write`, Write Authorization, `record_run`, staged artifact registration, blocker, status, compact evidence summary, residual-risk visibility, and `close_task` semantics must be maintainer-accepted for active MVP paths. |
| Public API and schema maintainer acceptance | Not maintainer-accepted for coding. | Method behavior, the Schema Core-owned active method-name value set, API request/response shapes, shared schemas, resources, errors, idempotency/replay behavior, unavailable Core/MCP behavior, and later-candidate exclusions must be maintainer-accepted before affected tools or resources are coded. |
| Storage and runtime-home maintainer acceptance | Not maintainer-accepted for coding. | The minimal storage profile, runtime home layout, locks, artifact refs, migrations, replay/audit needs, and later-candidate storage boundary must be maintainer-accepted before DDL, runtime data files, or artifact storage are created. |
| Security and local-access maintainer acceptance | Not maintainer-accepted for coding. | The local-only posture and cooperative/limited-detective security guarantee-claim wording must be maintainer-accepted before API/MCP exposure. Detective claims require a passed active capability check. MVP must not claim OS-level permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, permission isolation, security isolation, native artifact capture, command execution observation, network observation, or secret access observation. |
| Surface and compact-output maintainer acceptance | Not maintainer-accepted for coding. | The one reference `capability_profile`, compact user-facing views, compact agent-facing packet, freshness/unavailable behavior, and projection-as-derived-read boundary must be maintainer-accepted before display or connector code is implemented. |

## Reference owners

Build summarizes sequence and scope only. Use these Reference owners for exact contracts:

| Need | Owner docs |
|---|---|
| Core authority, Task/scope lifecycle, user judgments, `prepare_write`, Write Authorization, `record_run`, blockers, status, compact evidence meaning, residual risk, and `close_task` | [Core Model Reference](../reference/core-model.md). |
| Method-level behavior for active public API methods | [MVP API](../reference/api/mvp-api.md). |
| Exact active method-name set, shared envelopes, refs, enum value sets, resources, and active schema shapes | [API Schema Core](../reference/api/schema-core.md). |
| Public errors, idempotency, replay, stale-state, and state-conflict behavior | [API Errors](../reference/api/errors.md). |
| Storage layout, DDL, locks, migrations, artifact refs, and later-candidate storage boundaries | [Storage](../reference/storage.md). |
| Security guarantee-claim wording, local-access posture, trust boundaries, and non-claims | [Security Reference](../reference/security.md). |
| Compact derived views, projection authority boundaries, freshness, and active template ownership | [Projection And Templates Reference](../reference/projection-and-templates.md). |
| Reference surface `capability_profile`, connector behavior, context surfaces, and fallback semantics | [Agent Integration Reference](../reference/agent-integration.md). |
| Product Repository, Harness Server, Runtime Home, process boundaries, and non-isolation claims | [Runtime Boundaries Reference](../reference/runtime-boundaries.md). |
| Future fixture shape, assertion authority, and conformance meaning without an executable runner | [Conformance Reference](../reference/conformance.md). |
| Design-quality later-candidate boundaries and non-promotion of design-policy validator families | [Design Quality](../reference/design-quality.md). |
| Official terminology | [Glossary Reference](../reference/glossary.md). |
| Later candidates and promotion rule | [Later Candidate Index](../later/index.md). |

## Exit criteria for documentation planning

Documentation planning can exit only when maintainers explicitly confirm:

- this Build plan is the active Build entry point
- the active MVP boundary and excluded later material are maintainer-accepted, or any remaining boundary issue has a named scope impact
- every server-coding decision above has a maintainer decision, acceptance, or deferral with named scope impact
- Reference owners agree on the active Core, API, storage, security, projection/template, agent-integration, runtime-boundary, conformance, design-quality non-promotion, and glossary boundaries needed for the active MVP
- English and Korean Build pages preserve the same implementation decisions and active MVP boundary
- no later-candidate material is presented as required for the active MVP
- documentation remains source material only, with no server/runtime code, generated runtime state, executable fixture, conformance result, generated runtime report, or product implementation output created here

Passing these documentation-planning criteria does not implement Harness, prove runtime conformance, or close any future product work.
