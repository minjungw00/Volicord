# Build: Implementation Overview

## What this document helps you do

This document tells implementers what to build before they consult the specific Reference owner specs needed for a planning or implementation question. It is the bridge between the reader-centered docs and the detailed contracts in the kernel, runtime, MCP, storage, projection, and conformance references.

This is planning documentation for documentation redesign / review and maintainer handoff. The repository is documentation-only today, and its intended future role is the Harness Server source repository. Server/runtime implementation in this repository may start only after documentation acceptance and a separate implementation-planning readiness decision; no Harness Server/runtime implementation, executable fixture files, generated runtime records, generated projections, or runnable Harness Server conformance tests exist here yet. This revision is in post-redesign review and is a documentation acceptance candidate for maintainer review, not an accepted implementation start. The first future implementation target is Engineering Checkpoint, with Kernel Smoke as a narrow future smoke-check authoring label for the smallest local authority loop. The first user-value target is MVP-1 User Work Loop. Assurance Profile and Operations Profile harden agency assurance, operations, and handoff behavior. Roadmap remains future scope unless owner docs promote and prove it.

This Build page intentionally carries detailed phase and implementation-status warnings so Learn and Use pages can stay focused on the user experience. The current review baseline and acceptance status below are the detailed handoff sections to update when maintainers change status.

Use it to answer three questions:

- What are the runtime pieces that must exist first?
- What proof should the first internal Core authority smoke slice produce?
- What must be true before the first user-value slice can be called complete?

This document does not define SQLite DDL, public MCP schemas, projection template bodies, or command syntax. Those details stay in the reference docs.

## Read this when

- You are planning the first implementation shape after maintainer handoff explicitly accepts implementation-planning readiness for the first runtime batch.
- You need to review whether a proposed staged build keeps the right scope.
- You want the short map before reading the strict reference specs.

## Before you read

You should already understand the basic Harness concepts from the Learn path. For exact behavior, use the Reference docs linked at the end of this page. For Roadmap candidates and promotion criteria, use the [Roadmap](../roadmap.md).

## Main idea

Harness is a local work ledger and judgment router for AI-assisted product work. It records what may change, who must decide, what evidence exists, what risk remains, and whether the work can close. The first implementation path should prove that the local ledger works through the smallest Core authority loop, then prove the first narrow user-facing value slice.

Build Engineering Checkpoint first: the smallest local Core authority path, with Kernel Smoke as a narrow future smoke-check authoring label. This is an internal smoke milestone, not a product MVP. Then build MVP-1 User Work Loop so ordinary users can experience core Harness value at small scope: ordinary-language start/resume, work-shape classification, scope/non-goals/success criteria, minimal user judgment, evidence summary, close blockers, residual-risk visibility, and separated sensitive approval / work acceptance / risk acceptance display. Evidence and compact status output support that experience; they are not a full assurance, QA, evaluation, reporting, operations, or dashboard system. Assurance Profile and Operations Profile harden that path.

All implementation verbs in this Build path describe future runtime-batch planning after the maintainer handoff explicitly accepts implementation-planning readiness for that batch. While [Documentation Acceptance Status](#documentation-acceptance-status) says implementation planning readiness is not accepted, use this document only to review scope and handoff readiness. Documentation acceptance alone does not start implementation or prove runtime conformance.

When that handoff changes, implementation is expected to happen in this repository as the Harness Server / Installation source code. This repository is still not the user's Product Repository and not the Harness Runtime Home; runtime state, artifacts, projection output, and logs belong in a Harness Runtime Home.

The local kernel is a coordination and authority record, not a replacement for the product repository, source control, tests, code review, conversation, or user-owned product and material technical judgment. Build the first path so status/blocker output can explain the minimal authority state and what is missing, while leaving ordinary-language intake, close blocker summaries, work acceptance display, residual-risk visibility, and the compact user-facing explanation for MVP-1 and later.

The first authority loop is narrow: `prepare_write` is the only product-write authorization decision point, a returned Write Authorization is durable and single-use, and `record_run` consumes it for one compatible direct Run or implementation Run while recording observed changes and one artifact/evidence ref. Engineering Checkpoint may use status or a narrow close-task smoke for blockers, but it does not prove work acceptance or residual-risk close semantics. Exact state logic lives in [Core Model Reference](../reference/core-model.md#prepare_write) and public request/response details live in [MVP API](../reference/api/mvp-api.md).

Start with canonical state, one local project registration, one active Task, one scoped boundary represented by the Change Unit owner shape only where the reference contract requires it, one Write Authorization path, one recorded Run, one artifact/evidence link, Core tool behavior, and only the MCP reachability needed to exercise that path. The initial implementation assumption is one local process with modules, not a distributed platform. Treat natural-language intake, full Discovery, full-format user judgment presentation, full Evidence Manifest behavior, Eval, Manual QA, Acceptance, residual-risk acceptance, full close semantics, detached verification, projection rendering, conformance runner, dashboards or hosted workflow UI, indexes, broad connector ecosystems or marketplaces, team workflow, surface-specific connector automation, hook expansion, Browser QA automation, derived metrics, parallel orchestration, operations/export/recover, broad operator entrypoints, and broad automation as later or non-authoritative things that read from or wrap that authority loop after it exists.

If a proposed implementation starts with the full user-facing system, Assurance Profile or Operations Profile behavior as one large first batch, projection template polish, a dashboard or hosted workflow UI, a Context Index, a connector marketplace, hook expansion, metrics, parallel orchestration, or broad automation lanes, it is starting beyond the first implementation smoke slice.

## Current review baseline

The current documentation set is still documentation-only and in post-redesign review. This repository's intended future role is the Harness Server source repository. Runtime/server implementation has not started and may start only after documentation acceptance and a separate implementation-planning readiness decision. The current state is not fully accepted, implementation-complete, implementation-ready, or approved for server coding unless the maintainer-updated status table below explicitly says so.

No server/runtime implementation decisions have been formally accepted for coding in this repository phase. The decision ledger in [MVP-1 User Work Loop: Implementation decisions needed before server coding](mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding) records documentation-resolved MVP-1 contract decisions and the implementation-readiness items that still must be accepted or explicitly deferred before coding the affected stage.

Remaining drift and review risks are checked in [Authoring Guide: Known redesign issues and regression checks](../maintain/authoring-guide.md#known-redesign-issues-and-regression-checks). That section gives actionable regression checks and routes confirmed findings into the categories below. Review risks are not open implementation decisions by default, but if verification exposes a server-coding decision or stage blocker, record it in [MVP-1 User Work Loop: Implementation decisions needed before server coding](mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding) with owner doc, affected behavior or field, affected stage, options, and decision needed.

| Remaining item category | Meaning | Where it belongs | Blocking meaning |
|---|---|---|---|
| Documentation drift | Wording, owner-boundary, link, open-marker, terminology, or English/Korean parity mismatch. | Authoring Guide risk/regression checks and the affected docs. | May block documentation acceptance when it makes docs contradictory or non-actionable; not runtime conformance and not server code by itself. |
| Schema/design decision | A real choice about state, API, DDL, security guarantee, fixture semantics, or another owner contract. | Owning Reference doc plus the MVP-1 User Work Loop decision log when it must be decided before server coding. | Blocks implementation planning or server coding for the affected behavior until decided or deliberately deferred with stage impact. |
| Stage boundary decision | A choice about whether a capability belongs in Engineering Checkpoint, MVP-1 User Work Loop, Assurance Profile, Operations Profile, or Roadmap. | Implementation Overview, MVP-1 User Work Loop, owner docs, or Roadmap promotion when applicable. | Blocks implementing the affected stage until the boundary is accepted. It may be non-blocking for documentation review if explicitly recorded. |
| Implementation-readiness criterion | A condition maintainers must confirm before first runtime-batch planning begins. | This document's [Implementation-readiness criteria](#implementation-readiness-criteria). | Blocks first runtime-batch planning until satisfied or explicitly reclassified by maintainers. |
| Future roadmap item | A useful capability outside Engineering Checkpoint through Operations Profile unless promoted. | [Roadmap](../roadmap.md) and owner docs after promotion. | Does not block documentation review, Engineering Checkpoint, or MVP-1 unless an owner deliberately promotes it into a staged target. |

## Documentation acceptance status

This is a maintainer-updated documentation handoff marker. It separates documentation review status, implementation planning readiness, and runtime implementation status. It is not a Reference contract, conformance result, generated operational record, generated projection, runtime record, or runtime implementation authorization. Do not infer acceptance from the checklist below; maintainers must change this table deliberately.

Current revision status: post-redesign documentation review and documentation acceptance candidate for maintainer review. Documentation acceptance remains No unless maintainers deliberately change it. This status marker is not runtime/server implementation, runtime conformance, implementation completeness, or implementation readiness.

| Status category | Current status | Boundary |
|---|---|---|
| Documentation review status | Post-redesign review; documentation acceptance candidate only. Maintainer acceptance is still pending. | Documentation may be in review, candidate, or accepted state only when this table says so. Acceptance does not automatically start runtime implementation or create runtime conformance. |
| Implementation planning readiness | Not accepted. First runtime-batch planning may not begin until maintainers change this row after the readiness criteria below are satisfied. | Editorial cleanup is separate from schema/design decisions and stage boundary decisions. Remaining implementation-readiness criteria require maintainer judgment. |
| Runtime implementation status | Not started. This repository still contains documentation, not Harness runtime/server implementation. | No server/runtime code, runtime state, generated operational artifacts, executable fixtures, fixture files, generated projections, runtime records, or runnable Harness Server conformance tests exist here yet. |
| Server-coding decision log | MVP-1 scope decisions from the old ledger are documented as resolved in the MVP-1 User Work Loop, but no server/runtime implementation decision has been accepted for coding yet. | Maintainers must accept the documentation-resolved decisions and the implementation-readiness gate in [MVP-1 User Work Loop: Implementation decisions needed before server coding](mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding) before coding the affected API, DDL, state transition, local access, status card, evidence, acceptance, or risk behavior. |

Build readers should treat this table as the entry gate. Until maintainer handoff explicitly accepts implementation planning, even Engineering Checkpoint remains planning-only in this repository and runtime/server implementation must not start.

## Maintainer handoff summary

This section is the final documentation handoff for this revision. It explains what the documentation set defines, what remains open or unverified, and what must be true before Harness Server implementation planning can begin in this repository. It is a documentation handoff only; it does not create runtime state, acceptance records, generated projections, conformance results, runtime records, implementation authority, or server code.

What this documentation set defines:

- The Harness product thesis: a local authority record and judgment-routing layer for scope, user-owned judgment, evidence, verification, QA expectations, work acceptance, residual-risk status, and close readiness.
- The reader-facing Learn, Use, Build, Reference, Maintain, and Roadmap documentation structure.
- A future staged implementation plan for the Harness Server / Installation, starting with Engineering Checkpoint and then MVP-1 User Work Loop.
- Owner locations for exact contracts: Kernel, MCP/API schemas, storage/DDL, projection/templates, conformance fixtures, operations, security, agent integration, design quality, glossary, and runtime architecture.
- Documentation-maintenance rules for owner boundaries, English/Korean parity, status wording, open-marker hygiene, and drift routing.

It does not define runnable server code, executable fixture files, generated runtime artifacts, generated projections, runtime conformance results, implementation acceptance records, or a Harness Runtime Home.

Current phase and future repository role:

- The repository is in post-redesign documentation review and is a documentation acceptance candidate only.
- The repository's intended future role is the Harness Server source repository; server/runtime implementation here may start only after documentation acceptance and a separate implementation-planning readiness decision.
- It is not the user's Product Repository and not a Harness Runtime Home.
- No Harness Server/runtime implementation, runtime state, generated operational artifacts, executable fixtures, fixture files, generated projections, runtime records, or runnable Harness Server conformance tests exist here yet.

Preserved Harness principles:

- Harness is a local authority record for scope, user-owned judgment, evidence, verification, QA expectations, work acceptance, residual-risk status, and close readiness.
- Harness preserves user-owned judgment. Product/UX judgment, Technical judgment including architecture and security/privacy considerations, QA expectations, Work acceptance, waivers, and Residual risk acceptance remain user-owned judgments unless the owner contracts explicitly say otherwise.
- Evidence, verification, Manual QA, work acceptance, and residual risk are separate records and judgments. None of them substitutes for the others.
- Chat, connector output, generated documents, and Markdown-rendered projections are not operational authority. Core-owned local state and artifact references are authoritative.

Current delivery model:

- Engineering Checkpoint proves the smallest local Core authority loop with Kernel Smoke as a narrow future smoke-check authoring label.
- MVP-1 User Work Loop proves ordinary user value at narrow scope: plain-language start/resume, scope/non-goals/success criteria, user judgment request/record, cooperative pre-write scope checking through Core rather than OS-level blocking, run/evidence reference recording, compact status and next safe action, close blocker display, and minimal separation between work acceptance and residual-risk acceptance.
- Assurance Profile hardens verification, Manual QA, residual-risk accepted close, work acceptance separation, stewardship, user judgment quality, Approval separation, TDD, feedback-loop policy, and context hygiene.
- Operations Profile hardens doctor/readiness, recover/export, artifact integrity, release handoff, broader fixture coverage, and later-boundary checks.
- Roadmap remains future scope unless a future owner decision promotes an item with exact contracts, fixtures, fallback behavior, and no projection-as-canonical dependency.

What has been clarified:

- Repository identity is explicit: documentation-only now; intended future role is the Harness Server source repository; server/runtime implementation is separately gated.
- The product thesis is explicit: Harness is not a prompt pack, dashboard, broad hosted agent platform, or generated Markdown system.
- The judgment model separates Approval, User Judgments, work acceptance, residual-risk acceptance, QA/verification waiver judgments, and Write Authorization.
- Projections and chat are readable or conversational surfaces, not the operational source of truth.
- Projection scope is staged: Engineering Checkpoint may expose freshness/read facts only when an owner path already produces them, MVP-1 needs only the five compact Core-derived views (`status-card`, `agent-context-packet`, `judgment-request`, `run-evidence-summary`, and `close-result`), and detailed reports/templates are later-profile scope unless promoted.
- Security wording is bounded to actual enforcement levels: cooperative, detective, preventive, and isolated claims require the documented capability and fixture-proven path for the covered operation.
- Agent context is bounded: always-on context includes only current Task summary, work shape, scope/non-goals, pending user judgments, active blockers, next safe actions, evidence gaps, close blockers, residual-risk summary, guarantee level, and source refs/freshness, with detailed contracts and large bodies loaded from owner docs or retrieval paths only when needed.
- Conformance fixture documentation is a staged, future-oriented verification plan. It does not mean executable fixture files or runnable conformance tests exist today.

Current readiness status:

- Documentation acceptance: pending. This revision is a candidate for maintainer acceptance review, not accepted documentation.
- Implementation planning readiness: not accepted. First runtime-batch planning must not begin until maintainers explicitly accept the readiness criteria below or reclassify remaining blockers.
- Runtime implementation: not started. Server coding, fixture materialization, runtime conformance, and generated operational output remain out of scope for this repository phase.
- Server/runtime implementation decisions: the old MVP-1 scope ledger items are resolved in this documentation baseline, and those documentation-resolved items still require maintainer acceptance before coding. Additional design issues may still be found during maintainer review or implementation-readiness review.

Server-coding decision-log status:

- Documentation-resolved server-coding decision-ledger items are recorded in [MVP-1 User Work Loop: Implementation decisions needed before server coding](mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding).
- The previously open items for simplified judgment naming, `request_user_judgment`, moving MVP-1 next-action output to `harness.status.next_actions` while leaving separate `harness.next` as later/compatibility material, MVP-1 storage minimum, local access error taxonomy, MVP-1 compact view scope, small direct change evidence requirement, and acceptance/residual-risk minimal records are resolved in the documentation baseline but not yet accepted for coding.
- No server/runtime implementation decision has been formally accepted for coding. Maintainers must accept the relevant owner docs or explicitly defer an affected behavior with stage impact before changing server code or DDL for that behavior.

Documentation drift and review-risk status:

- No major implementation-decision TODOs are intentionally left scattered through active docs at this baseline.
- The [Authoring Guide risk/regression checks](../maintain/authoring-guide.md#known-redesign-issues-and-regression-checks) remain the review checklist for candidate drift and regression risks. They give default routing for confirmed findings as documentation drift, schema/design decisions, stage boundary decisions, implementation-readiness criteria, or future Roadmap items.
- The previously tracked judgment-model drift is resolved in this documentation baseline: canonical docs use `user_judgment`, `harness.request_user_judgment`, `judgment_type`, `presentation`, and `display_label`, and treat Decision Packet as optional full-format/legacy presentation. If review exposes a remaining owner-contract decision, route it to the MVP-1 User Work Loop decision log instead of scattering TODOs.
- Candidate review areas still requiring maintainer verification include stage-name drift, heavy user-facing disclaimers, early Discovery/Change Unit convergence, early Storage/API/DDL scope, projection/template scope, conformance-fixture detail, early operations entrypoints, security guarantee wording, agent context load, Korean technical-noun load, roadmap-boundary drift, and optimistic decision-log wording.

Maintainer acceptance conditions:

- Maintainers deliberately update [Documentation acceptance status](#documentation-acceptance-status); acceptance must not be inferred from this checklist.
- Any confirmed documentation drift is fixed or classified with owner, affected stage, and blocking meaning.
- Any confirmed schema/design decision, stage boundary decision, or other server-coding decision is recorded in the MVP-1 User Work Loop before server code or DDL changes begin. Documentation-resolved decisions must be accepted for coding, and any implementation decision still open must be resolved or explicitly deferred with stage impact.
- The [Implementation-readiness criteria](#implementation-readiness-criteria) are satisfied or explicitly reclassified by maintainers.
- Maintainers complete the final docs-maintenance pass in the [Authoring Guide](../maintain/authoring-guide.md#final-pre-acceptance-review) during acceptance review, including English/Korean parity, link/anchor checks, owner-boundary checks, open-marker hygiene, and current status wording.
- Only after documentation acceptance and a separate implementation-planning readiness decision may first runtime-batch planning begin. Server/runtime implementation still remains blocked until that readiness decision is explicitly accepted.

## Implementation-readiness criteria

Use this checkpoint to decide what must be true before maintainers can switch the implementation planning readiness status from documentation maintenance to first runtime-batch planning. It is a planning handoff only: it does not authorize runtime or server implementation by itself, and it does not define exact schemas, DDL, fixture semantics, or runtime contracts.

First implementation planning means Engineering Checkpoint planning first, not MVP-1 User Work Loop, Assurance Profile, Operations Profile, or roadmap automation. Editorial cleanup is necessary but not sufficient: schema/design decisions and stage boundary decisions must either be settled in their owner docs or recorded in the MVP-1 User Work Loop with stage impact before server coding begins. First implementation planning may start only when all of these are true:

- Engineering Checkpoint API subset is accepted.
- Engineering Checkpoint DDL is accepted.
- State transitions are accepted.
- Write Authorization lifecycle is accepted.
- Artifact/evidence ref shape is accepted.
- Structured blocker shape is accepted.
- Local access posture is accepted.
- MVP-1 promotion criteria are accepted.

- Repository identity is clear in the root README, docs README, language READMEs, Build docs, and relevant Reference docs: documentation-only now; intended future role is the Harness Server source repository; server/runtime implementation may start only after documentation acceptance and a separate implementation-planning readiness decision; not a Product Repository; not a Harness Runtime Home.
- The user-facing flow is understandable without requiring users to know internal terms before they can start, resume, unblock, accept, or close work.
- Discovery and requirements clarification preserve shared understanding and user-owned judgment before convergence on a Change Unit or first safe implementation unit. A Change Unit may express scoped work when the owner path requires it, but Discovery is not premature Change Unit selection.
- The judgment model is schema-aligned across Kernel, MCP/API schemas, storage, templates, fixtures, Learn/Use explanations, and glossary terms. `judgment_type` owns the internal judgment type, `presentation` owns compact/full display shape, and `display_label` is limited to Product/UX judgment, Technical judgment, Sensitive action approval, Work acceptance, or Residual risk acceptance; affected gates or blocked actions stay in separate owner fields.
- Judgment prompts are proportional to the judgment. Small explicit unblockers use `presentation=short`; complex, high-risk, close-affecting, sensitive-action approval, waiver, work-acceptance, residual-risk acceptance, or reconcile prompts may use `presentation=full` or optional full-format Decision Packet presentation without making every small judgment heavyweight.
- Approval, work acceptance, and residual-risk acceptance are distinct in examples, templates, API/schema wording, close behavior, and user-facing routing.
- Stages are coherent: Engineering Checkpoint is not a product MVP; MVP-1 User Work Loop is the first narrow user-value slice, not a full assurance/evaluation/QA/reporting system; Assurance Profile hardens verification, QA, residual risk, work acceptance, and stewardship; Operations Profile adds operational handoff capabilities; Roadmap stays future scope until promoted.
- Kernel, API, storage, reference, and Build contracts agree on Core ownership, state transitions, write authority, evidence, judgment records, close semantics, idempotency, state conflict behavior, artifacts, projection jobs, and fixture semantics.
- Storage and API are staged. Reference schemas and DDL may define future-profile fields or tables, but implementation follows the active method, record, or profile; future-profile presence does not expand Engineering Checkpoint by itself.
- Projection scope is staged and non-authoritative: readable projections and cards derive from Core records and artifact refs, do not create authority, and do not become the first proof.
- Security guarantees match actual enforcement levels. Cooperative, detective, preventive, and isolated claims are used only where the documented surface and fixture-proven path support that guarantee for the covered operation.
- Agent context strategy is defined: always-on context stays one screen or less and includes only current Task summary, work shape, scope/non-goals, pending user judgments, active blockers, next safe actions, evidence gaps, close blockers, residual-risk summary, guarantee level, and source refs/freshness; full reference docs, schemas, DDL, historical logs, projection bodies, artifact contents, unrelated templates, and future catalog material stay pull-on-demand through appropriate owner/retrieval paths.
- The conformance fixture plan is staged and future-oriented: Kernel Smoke is only the narrow Engineering Checkpoint authoring label for small smoke checks; later suite profiles align with MVP-1 User Work Loop, Assurance Profile, Operations Profile, and promoted Roadmap items; no text implies fixture files, a future fixture catalog, a full Engineering Checkpoint conformance suite, or runnable conformance tests already exist.
- The operations surface is staged. Minimal local status/diagnostic behavior may support early stages, but doctor/readiness, reconcile, recover, export, artifact checks, release handoff, and conformance run entrypoints do not become Engineering Checkpoint requirements unless the owning stage includes them.
- Korean user-facing docs are readable and consistent. Natural Korean comes first, while stable schema identifiers, API names, enum values, DDL names, file names, validator IDs, and official product/stage names remain exact where precision needs them.
- Links, open markers, terminology, and English/Korean semantic parity are clean. There are no unresolved major-decision TODOs scattered through active docs; implementation decisions needed before server coding are classified in [MVP-1 User Work Loop](mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding), with documentation-resolved decisions separated from implementation decisions still open.
- Maintainers complete the final docs-maintenance drift pass before accepting implementation-planning readiness. Any remaining item is explicitly classified as documentation drift, schema/design decision, stage boundary decision, implementation-readiness criterion, or future roadmap item. If it is non-blocking for documentation review but blocking before implementation planning or server coding, that later block is named. Docs-maintenance remains a read-only documentation check; see [Authoring Guide](../maintain/authoring-guide.md#docs-maintenance-checks) and [Operations And Conformance Reference](../reference/operations-and-conformance.md#docs-maintenance-profile).
- The local-only MCP exposure baseline is accepted for Engineering Checkpoint. Remote, shared, tunneled, or non-loopback exposure remains outside the Engineering Checkpoint baseline unless owner docs promote and prove a connector profile; see [Runtime Architecture](../reference/runtime-architecture.md#local-access-expectations), [Security Reference](../reference/security.md#mcp-local-access-and-caller-boundaries), and [API Schema Core](../reference/api/schema-core.md#mcp-boundary-and-caller-trust).
- Any reference-surface capability used to exercise the first authority path is accepted as a concrete declaration for the actual host/profile/configuration in use. Broad connector profile and surface recipe details stay in [Agent Integration Reference](../reference/agent-integration.md#capability-profiles) and [Surface Cookbook](../reference/surface-cookbook.md).
- The Core-only mutation model is accepted: Core alone changes canonical operational state, while resources, projections, reports, diagnostics, MCP callers, and operator entrypoints remain read-only or derived unless they enter a Core state-changing path. See [Core process model](../reference/runtime-architecture.md#core-process-model), [State transaction flow](../reference/runtime-architecture.md#state-transaction-flow), and the API [Idempotency](../reference/api/errors.md#idempotency) and [State conflict behavior](../reference/api/errors.md#state-conflict-behavior) sections.
- The Kernel Smoke fixture queue is identified as the Engineering Checkpoint conformance authoring order and future verification plan. Exact fixture format, assertions, and catalog semantics stay in [Conformance Fixtures Reference](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue); this checkpoint does not mean fixture files or runnable conformance tests already exist.
- The first implementation slice remains local, single-project, and limited to the minimal authority loop. Use [Engineering Checkpoint](engineering-checkpoint.md) for the planning checklist.
- Roadmap features remain outside Engineering Checkpoint, MVP-1 User Work Loop, Assurance Profile, and Operations Profile unless promoted by owner docs through the [Roadmap promotion criteria](../roadmap.md#promotion-criteria).

This handoff does not promote roadmap items, dashboards or hosted workflow UI, Browser QA Capture automation, Context Index, broad connector ecosystems or marketplaces, team workflow, remote MCP exposure, preventive guard expansion, Local Derived Metrics or long-term metrics, or parallel orchestration into Engineering Checkpoint, MVP-1 User Work Loop, Assurance Profile, or Operations Profile. Keep exact contracts in Reference docs and use this section only as the short readiness checkpoint.

## Proof boundaries

| Boundary | What it proves | What the user or operator can observe |
|---|---|---|
| Engineering Checkpoint | One local Task can go through the first Core authority loop: local project registration, active Task, one scoped boundary represented by the Change Unit owner shape only where the reference contract requires it, `prepare_write`, single-use Write Authorization, `record_run`, one artifact/evidence ref, and structured status/blocker response. | Status/blocker output shows current Task, scope, write authority, artifact/evidence support, and blockers. `prepare_write` refuses out-of-scope write authorization, compatible scoped work is authorized and consumed once, and status or a narrow close-task smoke refuses missing scope, write authority, or artifact/evidence support with structured blockers. |
| MVP-1 User Work Loop | Ordinary user work is clarified into scope, non-goals, success criteria, user-owned judgment, evidence summary, close blocker summary, next safe action, work-acceptance display, and residual-risk visibility. | Users can start or resume work in plain language, see work shape classification, product/UX and architecture judgments separately, codebase-answerable facts checked before questions, small direct changes and tracked work using different procedural budgets, cooperative pre-write scope checks through Core rather than OS-level blocking, run/evidence refs recorded, ambiguous feature requests held for clarification, close blocked by missing evidence or required user-owned judgments, residual risk displayed, ambiguous consent not treated as judgment resolution, MCP/Core unavailability not fabricated as authority, the five compact Core-derived MVP views, detached verification required only when active profile/user request/task type/risk profile requires it, waiver only when required verification is intentionally skipped, and work acceptance kept distinct from Approval and residual-risk acceptance. |
| Assurance Profile | The MVP-1 user-value path handles verification, Manual QA, residual-risk accepted close, work-acceptance separation, stewardship, profile-specific user judgment quality, Approval separation, TDD, feedback-loop policy, and context hygiene with honest boundaries. | Fixtures show why work can or cannot proceed, verify, require QA, accept, accept risk, or close through the same Core records and errors. |
| Operations Profile | Operator readiness, recover/export, artifact integrity, release handoff, broader fixture suite coverage, and later-boundary checks complete the [hardened local reference target](../reference/glossary.md#hardened-local-reference-target). | Operator entrypoints diagnose, recover, export, check artifacts, run conformance, and prepare release handoff over the same Core state without creating a second authority model. |
| Roadmap boundary | Later surfaces or automation can be considered only after the local kernel and agency proof are stable. | Optional capabilities remain read-only, display-only, metadata-only, or artifact-candidate-only until an owner promotes them through the [Roadmap promotion criteria](../roadmap.md#promotion-criteria) with exact contracts and fixtures. |

## What you are building

After maintainer handoff explicitly accepts implementation-planning readiness for the first runtime batch, Harness implementation starts in this repository with Engineering Checkpoint as the internal kernel for a local work ledger and judgment router. MVP-1 User Work Loop is the first milestone where that ledger becomes visible as user value. Engineering Checkpoint keeps only the durable local state, write authority record, Run record, artifact/evidence ref, and structured status/blocker output needed to prove the authority loop. MVP-1 adds plain-language intake/resume, scope/non-goals/success criteria, minimal judgment request/record, cooperative pre-write scope checking, run/evidence reference recording, compact status and next safe action, close blocker display, and minimal work-acceptance / residual-risk separation; full journey projections and polished reports remain later derived output. Product history, executable checking, review, and user judgment remain with the existing engineering process. The agency-preserving local authority kernel principle remains the implementation center: Core owns canonical local state, and user-owned judgment stays with the user. The initial implementation assumption is one local system with clear internal modules, not a distributed platform.

The sections below describe future responsibilities for that runtime batch. They are not work orders for the current documentation-acceptance phase.

### Local Server / Process

Build one local Harness server or process that exposes the MCP boundary, owns Core transitions, and reads and writes the runtime home. Validators, projection enqueueing, reconcile, recovery, export, and conformance entrypoints are later or profile-specific capabilities that must use the same Core rules when they enter scope.

Engineering Checkpoint can be one process with modules. It does not need separate services for Core, projection, validation, and operator tools.

### Core

Core is the only path that mutates canonical operational state. The future Core implementation must follow the transaction order owned by [Runtime Architecture](../reference/runtime-architecture.md#state-transaction-flow): envelope and state-version validation, lock acquisition, current-state read, owner checks or validators that are in scope, record update, owner-required event append, optional projection job enqueue when projection support is in scope, and commit. At this Build level, that means Core must:

- validate tool envelopes, idempotency keys, and expected state versions before a new mutation
- acquire the relevant project or task lock
- read current records
- run Core checks and only the validators required by the active stage
- update current records, append owner-required events, and enqueue projection work only when projection support is in scope
- return blockers and refs that explain the result

Agents, MCP tools, operator commands, projectors, and recovery flows must either enter through Core or preserve the same Core compatibility rules. None of them may maintain a second canonical state model.

### State Store

The state store keeps canonical operational state for the authority loop. For Engineering Checkpoint that means project and Task state, scoped boundary, write authority, one Run, one artifact/evidence ref, and the minimal owner records needed for status/blocker output. Judgment records, projection/reconcile tracking, full Evidence Manifest behavior, Eval, Manual QA, and broader event history are later or owner-profile scope.

Do not design this from scratch in the Build layer. Storage details and DDL are owned by [Storage](../reference/storage.md).

### Artifact Store

The artifact store keeps durable evidence files and integrity metadata. Raw artifacts may include diffs, logs, screenshots, bundles, manifests, checkpoints, export components, or other evidence files.

The artifact store is not a loose file dump. Any artifact that supports Harness state must be registered through the artifact owner path and linked to the Task or owner record that uses it. Exact artifact refs, integrity fields, redaction states, and retention rules belong to [API Schema Core](../reference/api/schema-core.md#artifactref) and [Storage](../reference/storage.md#artifact-directory-layout).

### MCP API

The future MCP server contract exposes read resources and public tools. MCP resources are read-only. State-changing work goes through public tools and Core.

If the MCP server cannot be reached, no authoritative Core response is available from that call path. The first implementation should report that as MCP unavailable, hold write-capable work according to the actual local caller or surface guarantee level when one is declared, and avoid inventing state from cached projections, generated files, or chat text.

For Engineering Checkpoint, prioritize only:

- minimal status/blocker reads over current Core state
- one owner-valid path to create or seed the first Task and scope
- the write-authority path: `prepare_write`, one compatible single-use Write Authorization, and `record_run`
- one artifact/evidence owner path
- structured blocker behavior for missing scope, missing write authority, or missing artifact/evidence support

For MVP-1 User Work Loop, broaden the same API surface only enough for ordinary requests to start or resume tracked work, classify work shape, summarize scope/non-goals/success criteria, request and record minimal user judgment, perform cooperative pre-write scope checks through Core, record run/evidence refs, show status/next output, summarize evidence and close blockers, show residual-risk visibility, and display sensitive approval, work acceptance, and risk acceptance separately.

The active MVP-1 public request and response contracts belong to [MVP API](../reference/api/mvp-api.md). Shared envelopes, refs, resources, and staged value sets belong to [API Schema Core](../reference/api/schema-core.md). Later/profile-gated methods belong to [API Schema Later](../reference/api/schema-later.md).

State conflict and idempotency replay behavior are part of that public tool contract. Build code should use [API Errors: Idempotency](../reference/api/errors.md#idempotency) and [State conflict behavior](../reference/api/errors.md#state-conflict-behavior), with durable storage details left to [Storage](../reference/storage.md).

### Projections

Projections are readable derived views from Core state records and artifact refs. `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, and other report projections are not canonical state.

Build projection output from the Core source records it depends on, such as Task, gate, Run, artifact, evidence, Eval, QA, and other owner records after those records exist. Engineering Checkpoint does not require a full projection renderer or multiple projection kinds; minimal status/blocker output is enough. It may report freshness/read facts only if an owner path already produces them, but projection rendering is not the proof. MVP-1 User Work Loop should provide enough readable summary or card output for current work status, next output, user judgment request/record, evidence summary, close blocker summary, work acceptance display, and residual-risk visibility. Those outputs support the user experience; they do not turn MVP-1 into a projection, evidence, reporting, evaluation, QA, operations, or dashboard pack. Projection templates cannot create authority, satisfy evidence, replace state, shape the state model, or become the first proof.

Later profiles may enable optional, future, and diagnostic `ProjectionKind` values when their source records exist or change and an owner profile promotes them. `ProjectionKind` values and API-owned support classes belong to [API Schema Core](../reference/api/schema-core.md#shared-schemas).
[Projection And Templates Reference](../reference/projection-and-templates.md#template-implementation-classes) owns projection authority boundaries, source-record rules, freshness rules, and template implementation classes; [Template Reference](../reference/templates/README.md) owns rendered template bodies and display cards.

Projection failure must not roll back committed Core state. It should mark projection freshness or job status and leave recovery or reconcile to a later action. `source_state_version` and freshness are display/readiness facts: close/readiness output should show when a readable view is stale or failed, but stale Markdown cannot authorize work, satisfy close, replace current Core state, replace source control, replace tests, or replace review.

Human-editable projection sections are proposal surfaces. The implementation path should route proposal -> reconcile item -> accepted Core state-changing action and `task_events` row, or reject, defer, or note. Direct managed-block edits are drift, not state changes.

### Operator Commands

Operator entrypoints are surfaces over Core behavior, not a second state model. They are not broad Engineering Checkpoint requirements. Build them as command-independent capabilities only when the relevant stage or owner profile brings them into scope:

| Stage | Operator capability boundary |
|---|---|
| Engineering Checkpoint | Minimal connect/register, basic status or diagnostic read, and local MCP/API exposure only if the first slice requires that boundary. |
| MVP-1 User Work Loop | User-facing status/next diagnostics for current work, user judgments, evidence state, close blockers, work-acceptance need/status, sensitive approval display, and residual-risk visibility. |
| Assurance Profile | Assurance-profile diagnostics and owner-path support for verification, Manual QA, residual risk, work acceptance, stewardship, and context hygiene. |
| Operations Profile | Full local operations: doctor/readiness, projection refresh, reconcile, recover, export, artifact integrity, release handoff where defined, and conformance run after suites are materialized. |
| Roadmap | Remote/shared operations, dashboards, broad connector automation, team workflow, orchestration, and higher automation only after promotion. |

Exact command names and flags can come later. The important part is the command-independent behavior contract: operator behavior uses the same Core state, `task_events`, artifacts, projections, and existing errors or diagnostics as MCP tools. State-changing operator outcomes must enter Core or a documented recovery path that preserves Core ordering; operator output must not become a parallel source of state truth.

## What you are not building yet

Keep the first implementation narrow. Do not build these as prerequisites unless owner docs promote them:

| Capability | Stage boundary |
|---|---|
| Dashboard, hosted workflow UI, or rich UI | Not authority, evidence, close readiness, work acceptance, or residual-risk acceptance for Engineering Checkpoint through Operations Profile. |
| Broad connector ecosystem or marketplace | Outside staged delivery beyond the first local authority path unless promoted. |
| Context Index | Read-only Roadmap candidate; not authority or read/write prerequisite. |
| Browser QA Capture | Roadmap candidate; not required automation, Manual QA replacement, or acceptance replacement. |
| Cross-Surface Verification | Roadmap automation candidate; detached verification can be proven locally before this. |
| Native hook expansion, Advanced Sidecar Watcher, preventive guard expansion | Capability-dependent enhancements only; claims require a proven concrete pre-tool block or observation path. |
| Local Derived Metrics or long-term metrics | Read-only diagnostics; not staged-delivery-critical state, authority, or readiness. |
| Team workflow, shared workspaces, permissions, profile import/export, parallel orchestration | Future coordination scope; not required for the local single-project authority path. |

Engineering Checkpoint may display cooperative or detective guard/freeze status and may hold or narrow work through existing scope, Autonomy Boundary where already present, and `prepare_write` behavior. Surface labels do not upgrade the stored guarantee level.

Useful later capabilities can appear only as read-only displays, metadata, artifact candidates for existing owner paths, or fixture candidates until their owner docs define capability profile, redaction/secret/PII policy, retention or test-environment rules when needed, fixture coverage, fallback behavior, and no projection-as-canonical dependency. They must not be required to run Engineering Checkpoint, to complete MVP-1 User Work Loop, or to claim staged-delivery close readiness.

## The first proof

The first future implementation target is Engineering Checkpoint: the smallest implementation path that proves Harness can make and enforce one authority decision after implementation begins. Kernel Smoke is the narrow future authoring label for this target's smoke checks, not a full conformance suite.

Engineering Checkpoint proves the internal authority loop, not the first user-value slice, not template completeness, and not broad automation.

It should show:

- one registered local project
- one Task with current Core-owned state
- one scoped work boundary for an intended change
- `prepare_write` refuses write authorization without compatible scope and allows one compatible scoped write
- allowed `prepare_write` creates a durable single-use Write Authorization
- `record_run` consumes that authorization for one direct Run or implementation Run and records observed changes
- one artifact/evidence ref can be registered and linked to the Run or minimal owner relation
- status/blocker output is non-mutating
- status or a close-task smoke blocks with structured blockers when scope, write authority, or artifact/evidence support is missing
- the same behavior can be mapped to future small Kernel Smoke candidates

Engineering Checkpoint is not a product MVP or the MVP-1 User Work Loop. It proves the write authority path is alive. Use [Engineering Checkpoint](engineering-checkpoint.md#doc-level-acceptance-checks) for doc-level acceptance checks, and use [Conformance Fixtures Reference](../reference/conformance-fixtures.md#conformance-fixture-format) for exact fixture semantics.

## The first user-value proof

The first user-value target is MVP-1 User Work Loop. It is reached after Engineering Checkpoint, not by expanding the first implementation batch. It proves that when a user starts work in plain language, Harness preserves a local basis for the work scope, pending user judgments, evidence summary, close blockers, next safe action, and minimal separation between work acceptance and residual-risk acceptance. Evidence and compact status output are supporting mechanisms, not a full product system by themselves.

It should show:

- ordinary user language can start or resume tracked work without requiring Harness vocabulary
- work shape is classified, including small direct change vs tracked work
- the work is clarified into scope, non-goals, success criteria, evidence expectations, close readiness, and judgment boundaries
- codebase-answerable or state-answerable facts are checked before asking the user to repeat them
- clarification separates blocking questions from useful-but-not-blocking questions without becoming a long questionnaire
- Product/UX judgment and Technical judgment can be presented separately
- a minimal user judgment request and record exists without requiring full-format judgment presentation machinery
- small direct changes and tracked work use different procedural budgets without bypassing authority
- pre-write scope checking is cooperative Core behavior through `prepare_write`, not OS-level blocking or permission isolation
- Run and evidence refs are recorded through `record_run`, registered artifacts, or the minimum evidence summary path
- ambiguous feature requests enter clarification instead of premature implementation
- close blocks when required evidence or user judgment is missing
- residual risk is visible before successful acceptance or close when close-relevant risk exists
- ambiguous consent such as "go ahead," "looks good," "좋아," or "진행해" does not resolve ambiguous routes or accept risk
- MCP/Core unavailable status does not fabricate authority
- work acceptance is distinct from sensitive-action Approval and residual-risk acceptance
- residual-risk acceptance, when supported, is visibly distinct from work acceptance
- the five compact MVP views are derived from Core records and are sufficient without template polish becoming authoritative
- detached verification is not required by default
- verification is required only when the active profile, user request, task type, or risk profile requires it
- verification waiver is needed only when required verification is intentionally skipped

MVP-1 should not include dashboard, hosted UI, full Decision Packet output for every judgment, OS-level sandbox or permission isolation, detailed Evidence Manifest, full report/export, runtime conformance suite, or operations/recovery/handoff pack.

## The hardened local reference proof

The later [hardened local reference target](../reference/glossary.md#hardened-local-reference-target) is reached through Assurance Profile and Operations Profile after MVP-1 User Work Loop, not as the first implementation batch. It is an umbrella target rather than a separate stage, fixture profile, or alternate implementation path. It adds the remaining conformance needed for an agent to act with honest boundaries:

- User judgment quality and routing
- separation between Sensitive action approval, User Judgments, and Write Authorizations
- residual-risk visibility before acceptance and close
- detached verification independence
- Manual QA records and QA blockers
- feedback-loop, TDD, stewardship, and context-hygiene validators
- projection and reconcile completeness
- recovery, export, and artifact integrity behavior
- release handoff report/export behavior where owner docs define it
- later-boundary checks that keep broad automation in Roadmap
- fixture coverage through the named Assurance Profile fixtures and Operations Profile or promoted-expansion fixtures

The hardened local reference target is complete only when future conformance proves behavior through Core state, events, artifacts, projection/freshness facts, and errors rather than rendered prose or renderer output alone.

## Build reading path

Read the Build layer in this order:

1. [Implementation Overview](implementation-overview.md) for current status, maintainer handoff, and the future system shape.
2. [MVP-1 User Work Loop](mvp-user-work-loop.md) for Engineering Checkpoint through Operations Profile staged delivery, stage boundaries, and the server-coding decision log.
3. [Engineering Checkpoint](engineering-checkpoint.md) for the Engineering Checkpoint implementation sequence.
4. [Runtime Walkthrough](runtime-walkthrough.md) for the request-to-close runtime path.

Use [Roadmap](../roadmap.md) for Roadmap candidates and promotion rules.

Then use the [Reference Index](../reference/README.md) to pick the current owner for exact behavior:

- [Core Model Reference](../reference/core-model.md) for entities, gates, state logic, `prepare_write`, and `close_task`.
- [Runtime Architecture Reference](../reference/runtime-architecture.md) for runtime spaces, Core flow, artifacts, projection/reconcile, and guarantee levels.
- [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md), and [API Schema Later](../reference/api/schema-later.md) for public resources, tools, schemas, errors, artifact refs, idempotency, state conflict behavior, and later/profile-gated API material.
- [Storage](../reference/storage.md) for runtime layout, staged DDL, migrations, locks, artifacts, and later-profile baseline, projection-job, and validator-run candidates.
- [Operations And Conformance Reference](../reference/operations-and-conformance.md) for operator semantics and conformance run overview.
- [Conformance Fixtures Reference](../reference/conformance-fixtures.md) for the core conformance model, fixture body shape, assertion semantics, and the reduced Kernel Smoke queue.
- [Future Fixtures](../later/future-fixtures.md) for detailed later scenario candidates that are not early-stage requirements by themselves.
