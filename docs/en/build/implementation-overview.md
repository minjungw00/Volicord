# Build: Implementation Overview

## What this document helps you do

This document tells implementers what to build before they consult the specific Reference owner specs needed for a planning or implementation question. It is the bridge between the reader-centered docs and the detailed contracts in the kernel, runtime, MCP, storage, projection, and conformance references.

This is planning documentation for documentation redesign / review and maintainer handoff. The repository is documentation-only today, and its intended future role is the Harness Server source repository. Server/runtime implementation in this repository may start only after documentation acceptance and a separate implementation-planning readiness decision; no Harness Server/runtime implementation, executable fixture files, generated runtime records, generated projections, or runnable Harness Server conformance tests exist here yet. This revision is in post-redesign review and is a documentation acceptance candidate for maintainer review, not an accepted implementation start. The first runnable target is v0.1 Core Authority Slice, with Kernel Smoke as a narrow future smoke-check authoring label for the smallest local authority loop. The first product MVP target is v0.2 User-Facing Harness MVP. v0.3 Agency Assurance Pack and v0.4 Operations & Handoff Pack harden agency assurance, operations, and handoff behavior. v1+ Expansion remains roadmap scope unless owner docs promote and prove it.

This Build page intentionally carries detailed phase and implementation-status warnings so Learn and Use pages can stay focused on the user experience. The current review baseline and acceptance status below are the detailed handoff sections to update when maintainers change status.

Use it to answer three questions:

- What are the runtime pieces that must exist first?
- What proof should the first internal Core authority slice produce?
- What must be true before the first user-facing Harness MVP can be called complete?

This document does not define SQLite DDL, public MCP schemas, projection template bodies, or command syntax. Those details stay in the reference docs.

## Read this when

- You are planning the first implementation shape after maintainer handoff explicitly accepts implementation-planning readiness for the first runtime batch.
- You need to review whether a proposed staged build keeps the right scope.
- You want the short map before reading the strict reference specs.

## Before you read

You should already understand the basic Harness concepts from the Learn path. For exact behavior, use the Reference docs linked at the end of this page. For v1+ Expansion candidates and promotion criteria, use the [Roadmap](../roadmap.md).

## Main idea

Harness is a local work ledger and judgment router for AI-assisted product work. It records what may change, who must decide, what evidence exists, what risk remains, and whether the work can close. The first implementation path should prove that the local ledger works through the smallest Core authority loop, then prove the first user-facing MVP value.

Build v0.1 Core Authority Slice first: the smallest local Core authority path, with Kernel Smoke as a narrow future smoke-check authoring label. This is an internal runnable milestone, not the product MVP. Then build v0.2 User-Facing Harness MVP so ordinary users can experience core Harness value: scope preservation, judgment routing, evidence, close readiness, final acceptance separation, and residual-risk visibility. Evidence and projection support that experience; they are not the stage's primary identity. v0.3 Agency Assurance Pack and v0.4 Operations & Handoff Pack harden that path.

All implementation verbs in this Build path describe future runtime-batch planning after the maintainer handoff explicitly accepts implementation-planning readiness for that batch. While [Documentation Acceptance Status](#documentation-acceptance-status) says implementation planning readiness is not accepted, use this document only to review scope and handoff readiness. Documentation acceptance alone does not start implementation or prove runtime conformance.

When that handoff changes, implementation is expected to happen in this repository as the Harness Server / Installation source code. This repository is still not the user's Product Repository and not the Harness Runtime Home; runtime state, artifacts, projection output, and logs belong in a Harness Runtime Home.

The local kernel is a coordination and authority record, not a replacement for the product repository, source control, tests, code review, conversation, or user-owned product and material technical judgment. Build the first path so status/blocker output can explain the minimal authority state and what is missing, while leaving close readiness, final acceptance, residual-risk language, and the full user-facing explanation for v0.2 and later.

The first authority loop is narrow: `prepare_write` is the only product-write authorization decision point, a returned Write Authorization is durable and single-use, and `record_run` consumes it for one compatible direct Run or implementation Run while recording observed changes and one artifact/evidence ref. v0.1 may use status or a narrow close-task smoke for blockers, but it does not prove final acceptance or residual-risk close semantics. Exact state logic lives in [Kernel Reference](../reference/kernel.md#prepare_write) and public request/response details live in [MCP API And Schemas](../reference/mcp-api-and-schemas.md#public-tools).

Start with canonical state, one local project registration, one scoped work boundary represented by the Change Unit owner shape only where the reference contract requires it, one Write Authorization path, one recorded Run, one artifact/evidence link, Core tool behavior, and only the MCP reachability needed to exercise that path. The initial implementation assumption is one local process with modules, not a distributed platform. Treat projection-template polish, full Evidence Manifest behavior, Manual QA, detached verification, residual-risk acceptance semantics, final acceptance semantics, dashboards or hosted workflow UI, indexes, broad connector ecosystems or marketplaces, team workflow, surface-specific connector automation, hook expansion, Browser QA automation, derived metrics, parallel orchestration, broad operator entrypoints, and broad automation as later or non-authoritative things that read from or wrap that authority loop after it exists.

If a proposed implementation starts with the user-facing MVP, v0.3 Agency Assurance Pack or v0.4 Operations & Handoff Pack behavior as one large first batch, projection template polish, a dashboard or hosted workflow UI, a Context Index, a connector marketplace, hook expansion, metrics, parallel orchestration, or broad automation lanes, it is starting beyond the first runnable slice.

## Current review baseline

The current documentation set is still documentation-only and in post-redesign review. This repository's intended future role is the Harness Server source repository. Runtime/server implementation has not started and may start only after documentation acceptance and a separate implementation-planning readiness decision. The current state is not fully accepted, implementation-complete, implementation-ready, or approved for server coding unless the maintainer-updated status table below explicitly says so.

Remaining drift and review risks are tracked in the [Authoring Guide](../maintain/authoring-guide.md#known-redesign-issues-tracker). That tracker separates observed drift, candidates to verify, regression-prevention checks, and baseline status checks, and routes confirmed findings into the categories below. Review risks are not open implementation decisions by default, but if verification exposes a server-coding decision or stage blocker, record it in [MVP Plan: Implementation decisions needed before server coding](mvp-plan.md#implementation-decisions-needed-before-server-coding) with owner doc, affected behavior or field, affected stage, options, and decision needed.

| Remaining item category | Meaning | Where it belongs | Blocking meaning |
|---|---|---|---|
| Documentation drift | Wording, owner-boundary, link, TODO, terminology, or English/Korean parity mismatch. | Authoring Guide tracker and the affected docs. | May block documentation acceptance when it makes docs contradictory or non-actionable; not runtime conformance and not server code by itself. |
| Schema/design decision | A real choice about state, API, DDL, security guarantee, fixture semantics, or another owner contract. | Owning Reference doc plus the MVP Plan decision log when it must be decided before server coding. | Blocks implementation planning or server coding for the affected behavior until decided or deliberately deferred with stage impact. |
| Stage boundary decision | A choice about whether a capability belongs in v0.1 Core Authority Slice, v0.2 User-Facing Harness MVP, v0.3 Agency Assurance Pack, v0.4 Operations & Handoff Pack, or v1+ Expansion. | Implementation Overview, MVP Plan, owner docs, or Roadmap promotion when applicable. | Blocks implementing the affected stage until the boundary is accepted. It may be non-blocking for documentation review if explicitly recorded. |
| Implementation-readiness criterion | A condition maintainers must confirm before first runtime-batch planning begins. | This document's [Implementation-readiness criteria](#implementation-readiness-criteria). | Blocks first runtime-batch planning until satisfied or explicitly reclassified by maintainers. |
| Future roadmap item | A useful capability outside v0.1 through v0.4 unless promoted. | [Roadmap](../roadmap.md) and owner docs after promotion. | Does not block documentation review, v0.1, or v0.2 unless an owner deliberately promotes it into a staged target. |

## Documentation acceptance status

This is a maintainer-updated documentation handoff marker. It separates documentation review status, implementation planning readiness, and runtime implementation status. It is not a Reference contract, conformance result, generated operational record, generated projection, runtime record, or runtime implementation authorization. Do not infer acceptance from the checklist below; maintainers must change this table deliberately.

Current revision status: post-redesign documentation review and documentation acceptance candidate for maintainer review. Documentation acceptance remains No unless maintainers deliberately change it. This status marker is not runtime/server implementation, runtime conformance, implementation completeness, or implementation readiness.

| Status category | Current status | Boundary |
|---|---|---|
| Documentation review status | Post-redesign review; documentation acceptance candidate only. Maintainer acceptance is still pending. | Documentation may be in review, candidate, or accepted state only when this table says so. Acceptance does not automatically start runtime implementation or create runtime conformance. |
| Implementation planning readiness | Not accepted. First runtime-batch planning may not begin until maintainers change this row after the readiness criteria below are satisfied. | Editorial cleanup is separate from schema/design decisions and stage boundary decisions. Remaining implementation-readiness criteria require maintainer judgment. |
| Runtime implementation status | Not started. This repository still contains documentation, not Harness runtime/server implementation. | No server/runtime code, runtime state, generated operational artifacts, executable fixtures, fixture files, generated projections, runtime records, or runnable Harness Server conformance tests exist here yet. |
| Server-coding decision log | Empty at this baseline. This is a decision-log content statement, not proof that no decisions remain. | If maintainer review finds a schema/design decision, stage boundary decision, or other server-coding decision, record it only in [MVP Plan: Implementation decisions needed before server coding](mvp-plan.md#implementation-decisions-needed-before-server-coding) with owner, affected behavior or field, affected stage, options, and decision needed. |

Build readers should treat this table as the entry gate. Until maintainer handoff explicitly accepts implementation planning, even v0.1 Core Authority Slice remains planning-only in this repository and runtime/server implementation must not start.

## Maintainer handoff summary

This section is the final documentation handoff for this revision. It explains what the documentation set defines, what remains open or unverified, and what must be true before Harness Server implementation planning can begin in this repository. It is a documentation handoff only; it does not create runtime state, acceptance records, generated projections, conformance results, runtime records, implementation authority, or server code.

What this documentation set defines:

- The Harness product thesis: a local authority record and judgment-routing layer for scope, user-owned judgment, evidence, verification, QA expectations, final acceptance, residual-risk status, and close readiness.
- The reader-facing Learn, Use, Build, Reference, Maintain, and Roadmap documentation structure.
- A future staged implementation plan for the Harness Server / Installation, starting with v0.1 Core Authority Slice and then v0.2 User-Facing Harness MVP.
- Owner locations for exact contracts: Kernel, MCP/API schemas, storage/DDL, projection/templates, conformance fixtures, operations, security, agent integration, design quality, glossary, and runtime architecture.
- Documentation-maintenance rules for owner boundaries, English/Korean parity, status wording, TODO hygiene, and drift routing.

It does not define runnable server code, executable fixture files, generated runtime artifacts, generated projections, runtime conformance results, implementation acceptance records, or a Harness Runtime Home.

Current phase and future repository role:

- The repository is in post-redesign documentation review and is a documentation acceptance candidate only.
- The repository's intended future role is the Harness Server source repository; server/runtime implementation here may start only after documentation acceptance and a separate implementation-planning readiness decision.
- It is not the user's Product Repository and not a Harness Runtime Home.
- No Harness Server/runtime implementation, runtime state, generated operational artifacts, executable fixtures, fixture files, generated projections, runtime records, or runnable Harness Server conformance tests exist here yet.

Preserved Harness principles:

- Harness is a local authority record for scope, user-owned judgment, evidence, verification, QA expectations, final acceptance, residual-risk status, and close readiness.
- Harness preserves user-owned judgment. Product/UX judgment, technical architecture judgment, security/privacy judgment, QA expectations, final acceptance, waivers, and residual-risk acceptance remain user-owned judgments unless the owner contracts explicitly say otherwise.
- Evidence, verification, Manual QA, final acceptance, and residual risk are separate records and judgments. None of them substitutes for the others.
- Chat, connector output, generated documents, and Markdown-rendered projections are not operational authority. Core-owned local state and artifact references are authoritative.

Current stage model:

- v0.1 Core Authority Slice proves the smallest local Core authority loop with Kernel Smoke as a narrow future smoke-check authoring label.
- v0.2 User-Facing Harness MVP proves ordinary user value: scope preservation, user-owned judgment routing, evidence, close readiness, final acceptance separation, and residual-risk visibility.
- v0.3 Agency Assurance Pack hardens verification, Manual QA, residual-risk accepted close, final acceptance separation, stewardship, Decision Packets, Approval separation, TDD, feedback-loop policy, and context hygiene.
- v0.4 Operations & Handoff Pack hardens doctor/readiness, recover/export, artifact integrity, release handoff, broader fixture coverage, and later-boundary checks.
- v1+ Expansion remains roadmap scope unless a future owner decision promotes an item with exact contracts, fixtures, fallback behavior, and no projection-as-canonical dependency.

What has been clarified:

- Repository identity is explicit: documentation-only now; intended future role is the Harness Server source repository; server/runtime implementation is separately gated.
- The product thesis is explicit: Harness is not a prompt pack, dashboard, broad hosted agent platform, or generated Markdown system.
- The judgment model separates Approval, Decision Packets, final acceptance, residual-risk acceptance, QA/verification waiver decisions, and Write Authorization.
- Projections and chat are readable or conversational surfaces, not the operational source of truth.
- Projection scope is staged: v0.1 may expose freshness/read facts, v0.2 needs enough user-readable output for MVP comprehension, and detailed reports/templates are later-profile scope unless promoted.
- Security wording is bounded to actual enforcement levels: cooperative, detective, preventive, and isolated claims require the documented capability and fixture-proven path for the covered operation.
- Agent context is bounded: always-on context stays short and current, with detailed contracts loaded from owner docs or retrieval paths only when needed.
- Conformance fixture documentation is a staged, future-oriented verification plan. It does not mean executable fixture files or runnable conformance tests exist today.

Current readiness status:

- Documentation acceptance: pending. This revision is a candidate for maintainer acceptance review, not accepted documentation.
- Implementation planning readiness: not accepted. First runtime-batch planning must not begin until maintainers explicitly accept the readiness criteria below or reclassify remaining blockers.
- Runtime implementation: not started. Server coding, fixture materialization, runtime conformance, and generated operational output remain out of scope for this repository phase.

Server-coding decision-log status:

- Confirmed server-coding decision-log entries: none are recorded at this baseline in [MVP Plan: Implementation decisions needed before server coding](mvp-plan.md#implementation-decisions-needed-before-server-coding).
- This is not a "no open decisions" claim. The implementation-readiness review may still uncover schema/design decisions, stage boundary decisions, or other server-coding decisions.
- If a decision is found, record it only in the MVP Plan with owner doc, affected behavior or field, affected stage, options, and the decision condition before changing server code or DDL.

Documentation drift and review-risk status:

- No major implementation-decision TODOs are intentionally left scattered through active docs at this baseline.
- The [Authoring Guide tracker](../maintain/authoring-guide.md#known-redesign-issues-tracker) remains the review checklist for candidate drift and regression risks. It gives default routing for confirmed findings as documentation drift, schema/design decisions, stage boundary decisions, implementation-readiness criteria, or future roadmap items.
- Candidate review areas still requiring maintainer verification include stage-name drift, heavy user-facing disclaimers, early Discovery/Change Unit convergence, `judgment_domain` and `decision_kind` alignment, small-decision Decision Packet weight, early Storage/API/DDL scope, projection/template scope, conformance-fixture detail, early operations entrypoints, security guarantee wording, agent context load, Korean technical-noun load, roadmap-boundary drift, and optimistic decision-log wording.

Maintainer acceptance conditions:

- Maintainers deliberately update [Documentation acceptance status](#documentation-acceptance-status); acceptance must not be inferred from this checklist.
- Any confirmed documentation drift is fixed or classified with owner, affected stage, and blocking meaning.
- Any confirmed schema/design decision, stage boundary decision, or other server-coding decision is recorded in the MVP Plan before server code or DDL changes begin.
- The [Implementation-readiness criteria](#implementation-readiness-criteria) are satisfied or explicitly reclassified by maintainers.
- The final docs-maintenance pass in the [Authoring Guide](../maintain/authoring-guide.md#final-pre-acceptance-review) is complete, including English/Korean parity, link/anchor checks, owner-boundary checks, TODO hygiene, and current status wording.
- Only after documentation acceptance and a separate implementation-planning readiness decision may first runtime-batch planning begin. Server/runtime implementation still remains blocked until that readiness decision is explicitly accepted.

## Implementation-readiness criteria

Use this checkpoint to decide what must be true before maintainers can switch the implementation planning readiness status from documentation maintenance to first runtime-batch planning. It is a planning handoff only: it does not authorize runtime or server implementation by itself, and it does not define exact schemas, DDL, fixture semantics, or runtime contracts.

First implementation planning means v0.1 Core Authority Slice planning first, not User-Facing Harness MVP, v0.3 Agency Assurance Pack, v0.4 Operations & Handoff Pack, or roadmap automation. Editorial cleanup is necessary but not sufficient: schema/design decisions and stage boundary decisions must either be settled in their owner docs or recorded in the MVP Plan with stage impact before server coding begins. First implementation planning may start only when all of these are true:

- Repository identity is clear in the root README, docs README, language READMEs, Build docs, and relevant Reference docs: documentation-only now; intended future role is the Harness Server source repository; server/runtime implementation may start only after documentation acceptance and a separate implementation-planning readiness decision; not a Product Repository; not a Harness Runtime Home.
- The user-facing flow is understandable without requiring users to know internal terms before they can start, resume, unblock, accept, or close work.
- Discovery and requirements clarification preserve shared understanding and user-owned judgment before convergence on a Change Unit or first safe implementation unit. A Change Unit may express scoped work when the owner path requires it, but Discovery is not premature Change Unit selection.
- The judgment model is schema-aligned across Kernel, MCP/API schemas, storage, templates, fixtures, Learn/Use explanations, and glossary terms. `decision_kind` owns lifecycle, gate, payload, and state-transition semantics; `decision_profile` owns prompt-depth and profile-specific requiredness; `judgment_domain` owns schema-defined user-visible grouping; affected gates or blocked actions stay in separate owner fields.
- Decision Packet profiles are proportional to the decision. Small explicit unblockers can use `minimal_decision`; full trade-off, approval-shaped, waiver, final acceptance, residual-risk acceptance, reconcile, and mixed profiles carry the extra context their owner contracts require without making every small decision heavyweight.
- Approval, final acceptance, and residual-risk acceptance are distinct in examples, templates, API/schema wording, close behavior, and user-facing routing.
- MVP stages are coherent: v0.1 Core Authority Slice is not the product MVP; v0.2 is the first user-facing MVP; v0.3 Agency Assurance Pack hardens verification, QA, residual risk, final acceptance, and stewardship; v0.4 Operations & Handoff Pack adds operational handoff capabilities; v1+ Expansion stays roadmap scope until promoted.
- Kernel, API, storage, reference, and Build contracts agree on Core ownership, state transitions, write authority, evidence, judgment records, close semantics, idempotency, state conflict behavior, artifacts, projection jobs, and fixture semantics.
- Storage and API are staged. Reference schemas and DDL may define future-profile fields or tables, but stage-required implementation follows the active method, record, or profile; future-profile presence does not expand v0.1 by itself.
- Projection scope is staged and non-authoritative: readable projections and cards derive from Core records and artifact refs, do not create authority, and do not become the first proof.
- Security guarantees match actual enforcement levels. Cooperative, detective, preventive, and isolated claims are used only where the documented surface and fixture-proven path support that guarantee for the covered operation.
- Agent context strategy is defined: always-on context stays one screen or less, current-state based, and profile-scoped; full reference docs, schemas, old logs, and projection bodies are loaded only through appropriate owner/retrieval paths.
- The conformance fixture plan is staged and future-oriented: Kernel Smoke is only the narrow v0.1 authoring label for small smoke checks; later suite profiles align with v0.2 User-Facing Harness MVP, v0.3 Agency Assurance Pack, v0.4 Operations & Handoff Pack, and promoted v1+ items; no text implies fixture files, a future fixture catalog, a full v0.1 conformance suite, or runnable conformance tests already exist.
- The operations surface is staged. Minimal local status/diagnostic behavior may support early stages, but doctor/readiness, reconcile, recover, export, artifact checks, release handoff, and conformance run entrypoints do not become v0.1 requirements unless the owning stage includes them.
- Korean user-facing docs are readable and consistent. Natural Korean comes first, while stable schema identifiers, API names, enum values, DDL names, file names, validator IDs, and official product/stage names remain exact where precision needs them.
- Links, TODOs, terminology, and English/Korean semantic parity are clean. There are no unresolved major-decision TODOs scattered through active docs; any implementation decision needed before server coding is classified and recorded in [MVP Plan](mvp-plan.md#implementation-decisions-needed-before-server-coding).
- The final docs-maintenance drift pass is complete. Any remaining item is explicitly classified as documentation drift, schema/design decision, stage boundary decision, implementation-readiness criterion, or future roadmap item. If it is non-blocking for documentation review but blocking before implementation planning or server coding, that later block is named. Docs-maintenance remains a read-only documentation check; see [Authoring Guide](../maintain/authoring-guide.md#docs-maintenance-checks) and [Operations And Conformance Reference](../reference/operations-and-conformance.md#docs-maintenance-profile).
- The local-only MCP exposure baseline is accepted for v0.1 Core Authority Slice. Remote, shared, tunneled, or non-loopback exposure remains outside the v0.1 baseline unless owner docs promote and prove a connector profile; see [Runtime Architecture](../reference/runtime-architecture.md#local-access-expectations), [Security Threat Model Reference](../reference/security-threat-model.md#mcp-local-access-and-caller-boundaries), and [MCP API And Schemas](../reference/mcp-api-and-schemas.md#mcp-boundary-and-caller-trust).
- Any reference-surface capability used to exercise the first authority path is accepted as a concrete declaration for the actual host/profile/configuration in use. Broad connector profile and surface recipe details stay in [Agent Integration Reference](../reference/agent-integration.md#capability-profiles) and [Surface Cookbook](../reference/surface-cookbook.md).
- The Core-only mutation model is accepted: Core alone changes canonical operational state, while resources, projections, reports, diagnostics, MCP callers, and operator entrypoints remain read-only or derived unless they enter a Core state-changing path. See [Core process model](../reference/runtime-architecture.md#core-process-model), [State transaction flow](../reference/runtime-architecture.md#state-transaction-flow), and the MCP [Idempotency](../reference/mcp-api-and-schemas.md#idempotency) and [State conflict behavior](../reference/mcp-api-and-schemas.md#state-conflict-behavior) sections.
- The Kernel Smoke fixture queue is identified as the v0.1 Core Authority Slice conformance authoring order and future verification plan. Exact fixture format, assertions, and catalog semantics stay in [Conformance Fixtures Reference](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue); this checkpoint does not mean fixture files or runnable conformance tests already exist.
- The first runnable slice remains local, single-project, and limited to the minimal authority loop. Use [First Runnable Slice](first-runnable-slice.md) for the planning checklist.
- v1+ Expansion features remain outside v0.1 Core Authority Slice, v0.2 User-Facing Harness MVP, v0.3 Agency Assurance Pack, and v0.4 Operations & Handoff Pack unless promoted by owner docs through the [Roadmap promotion criteria](../roadmap.md#promotion-criteria).

This handoff does not promote roadmap items, dashboards or hosted workflow UI, Browser QA Capture automation, Context Index, broad connector ecosystems or marketplaces, team workflow, remote MCP exposure, preventive guard expansion, Local Derived Metrics or long-term metrics, or parallel orchestration into v0.1 Core Authority Slice, v0.2 User-Facing Harness MVP, v0.3 Agency Assurance Pack, or v0.4 Operations & Handoff Pack. Keep exact contracts in Reference docs and use this section only as the short readiness checkpoint.

## Proof boundaries

| Boundary | What it proves | What the user or operator can observe |
|---|---|---|
| v0.1 Core Authority Slice | One local Task can go through the first Core authority loop: local project registration, Task, one scoped work boundary represented by the Change Unit owner shape only where the reference contract requires it, `prepare_write`, single-use Write Authorization, `record_run`, one artifact/evidence ref, and structured blocker/status response. | Status/blocker output shows current Task, scope, write authority, artifact/evidence support, and blockers. `prepare_write` refuses out-of-scope write authorization, compatible scoped work is authorized and consumed once, and status or a narrow close-task smoke refuses missing scope, write authority, or artifact/evidence support with structured blockers. |
| v0.2 User-Facing Harness MVP | Ordinary user work is clarified into scope, user-owned judgment, evidence, close readiness, final-acceptance, and residual-risk language. | Users can see product/UX and architecture judgments separately, small changes and tracked work using different procedural budgets, close blocked by missing evidence or required user-owned decisions, residual risk displayed, and final acceptance kept distinct from Approval and residual-risk acceptance. |
| v0.3 Agency Assurance Pack | The MVP path handles verification, Manual QA, residual-risk accepted close, final acceptance separation, stewardship, full Decision Packet quality, Approval separation, TDD, feedback-loop policy, and context hygiene with honest boundaries. | Fixtures show why work can or cannot proceed, verify, require QA, accept, accept risk, or close through the same Core records and errors. |
| v0.4 Operations & Handoff Pack | Operator readiness, recover/export, artifact integrity, release handoff, broader fixture suite coverage, and later-boundary checks complete the [hardened local reference target](../reference/glossary.md#hardened-local-reference-target). | Operator entrypoints diagnose, recover, export, check artifacts, run conformance, and prepare release handoff over the same Core state without creating a second authority model. |
| Roadmap boundary: v1+ Expansion | Later surfaces or automation can be considered only after the local kernel and agency proof are stable. | Optional capabilities remain read-only, display-only, metadata-only, or artifact-candidate-only until an owner promotes them through the [Roadmap promotion criteria](../roadmap.md#promotion-criteria) with exact contracts and fixtures. |

## What you are building

After maintainer handoff explicitly accepts implementation-planning readiness for the first runtime batch, Harness implementation starts in this repository with v0.1 Core Authority Slice as the internal kernel for a local work ledger and judgment router. v0.2 User-Facing Harness MVP is the first milestone where that ledger becomes visible as user value. v0.1 keeps only the durable local state, write authority record, Run record, artifact/evidence ref, and structured status/blocker output needed to prove the authority loop. v0.2 adds enough readable status or cards for the user-facing path; full journey projections and polished reports remain later derived output. Product history, executable checking, review, and user judgment remain with the existing engineering process. The agency-preserving local authority kernel principle remains the implementation center: Core owns canonical local state, and user-owned judgment stays with the user. The initial implementation assumption is one local system with clear internal modules, not a distributed platform.

The sections below describe future responsibilities for that runtime batch. They are not work orders for the current documentation-acceptance phase.

### Local Server / Process

Build one local Harness server or process that exposes the MCP boundary, owns Core transitions, and reads and writes the runtime home. Validators, projection enqueueing, reconcile, recovery, export, and conformance entrypoints are later or profile-specific capabilities that must use the same Core rules when they enter scope.

v0.1 Core Authority Slice can be one process with modules. It does not need separate services for Core, projection, validation, and operator tools.

### Core

Core is the only path that mutates canonical operational state. Implement the transaction order owned by [Runtime Architecture](../reference/runtime-architecture.md#state-transaction-flow): envelope and state-version validation, lock acquisition, current-state read, owner checks or validators that are in scope, record update, owner-required event append, optional projection job enqueue when projection support is in scope, and commit. At this Build level, that means Core must:

- validate tool envelopes, idempotency keys, and expected state versions before a new mutation
- acquire the relevant project or task lock
- read current records
- run Core checks and only the validators required by the active stage
- update current records, append owner-required events, and enqueue projection work only when projection support is in scope
- return blockers and refs that explain the result

Agents, MCP tools, operator commands, projectors, and recovery flows must either enter through Core or preserve the same Core compatibility rules. None of them may maintain a second canonical state model.

### State Store

The state store keeps canonical operational state for the authority loop. For v0.1 that means project and Task state, scoped work boundary, write authority, one Run, one artifact/evidence ref, and the minimal owner records needed for status/blocker output. Judgment records, projection/reconcile tracking, full Evidence Manifest behavior, Eval, Manual QA, and broader event history are later or owner-profile scope.

Do not design this from scratch in the Build layer. Storage details and DDL are owned by [Storage And DDL](../reference/storage-and-ddl.md).

### Artifact Store

The artifact store keeps durable evidence files and integrity metadata. Raw artifacts may include diffs, logs, screenshots, bundles, manifests, checkpoints, export components, or other evidence files.

The artifact store is not a loose file dump. Any artifact that supports Harness state must be registered through the artifact owner path and linked to the Task or owner record that uses it. Exact artifact refs, integrity fields, redaction states, and retention rules belong to [MCP API And Schemas](../reference/mcp-api-and-schemas.md#artifactref) and [Storage And DDL](../reference/storage-and-ddl.md#artifact-directory-layout).

### MCP API

The MCP server exposes read resources and public tools. MCP resources are read-only. State-changing work goes through public tools and Core.

If the MCP server cannot be reached, no authoritative Core response is available from that call path. The first implementation should report that as MCP unavailable, hold write-capable work according to the actual local caller or surface guarantee level when one is declared, and avoid inventing state from cached projections, generated files, or chat text.

For v0.1 Core Authority Slice, prioritize only:

- minimal status/blocker reads over current Core state
- one owner-valid path to create or seed the first Task and scope
- the write-authority path: `prepare_write`, one compatible single-use Write Authorization, and `record_run`
- one artifact/evidence owner path
- structured blocker behavior for missing scope, missing write authority, or missing artifact/evidence support

For v0.2 User-Facing Harness MVP, broaden the same API surface so ordinary requests can be clarified into scope, user-owned judgment, evidence expectations, close readiness, final acceptance, and residual-risk display.

The public request and response contracts belong to [MCP API And Schemas](../reference/mcp-api-and-schemas.md).

State conflict and idempotency replay behavior are part of that public tool contract. Build code should use the owner sections for [Idempotency](../reference/mcp-api-and-schemas.md#idempotency) and [State conflict behavior](../reference/mcp-api-and-schemas.md#state-conflict-behavior), with durable storage details left to [Storage And DDL](../reference/storage-and-ddl.md).

### Projections

Projections are readable derived views from Core state records and artifact refs. `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, and other report projections are not canonical state.

Build projection output from the Core source records it depends on, such as Task, gate, Run, artifact, evidence, Eval, QA, and other owner records after those records exist. v0.1 Core Authority Slice does not require a full projection renderer or multiple projection kinds; minimal status/blocker output is enough. It may report freshness/read facts only if an owner path already produces them, but projection rendering is not the proof. v0.2 User-Facing Harness MVP should provide enough readable summary or card output for current work status, user decision request, evidence summary, close readiness, final acceptance, and residual risk. Those outputs support the user experience; they do not turn v0.2 into a projection or evidence component pack. Projection templates cannot create authority, satisfy evidence, replace state, shape the state model, or become the first proof.

Later packs may enable optional, future, and diagnostic `ProjectionKind` values when their source records exist or change and an owner profile promotes them. `ProjectionKind` values and API-owned support classes belong to [MCP API And Schemas](../reference/mcp-api-and-schemas.md#shared-schemas).
[Document Projection Reference](../reference/document-projection.md#template-implementation-classes) owns projection authority boundaries, source-record rules, freshness rules, and template implementation classes; [Template Reference](../reference/templates/README.md) owns rendered template bodies and display cards.

Projection failure must not roll back committed Core state. It should mark projection freshness or job status and leave recovery or reconcile to a later action. `source_state_version` and freshness are display/readiness facts: close/readiness output should show when a readable view is stale or failed, but stale Markdown cannot authorize work, satisfy close, replace current Core state, replace source control, replace tests, or replace review.

Human-editable projection sections are proposal surfaces. The implementation path should route proposal -> reconcile item -> accepted Core state-changing action and `task_events` row, or reject, defer, or note. Direct managed-block edits are drift, not state changes.

### Operator Commands

Operator entrypoints are surfaces over Core behavior, not a second state model. They are not broad v0.1 requirements. Build them as command-independent capabilities only when the relevant stage or owner profile brings them into scope:

| Stage | Operator capability boundary |
|---|---|
| v0.1 Core Authority Slice | Minimal connect/register, basic status or diagnostic read, and local MCP/API exposure only if the first slice requires that boundary. |
| v0.2 User-Facing Harness MVP | User-facing status/next diagnostics for current work, user decisions, evidence state, close blockers, final-acceptance need/status, and residual-risk visibility. |
| v0.3 Agency Assurance Pack | Assurance-profile diagnostics and owner-path support for verification, Manual QA, residual risk, final acceptance, stewardship, and context hygiene. |
| v0.4 Operations & Handoff Pack | Full local operations: doctor/readiness, projection refresh, reconcile, recover, export, artifact integrity, release handoff where defined, and conformance run after suites are materialized. |
| v1+ Expansion | Remote/shared operations, dashboards, broad connector automation, team workflow, orchestration, and higher automation only after promotion. |

Exact command names and flags can come later. The important part is the command-independent behavior contract: operator behavior uses the same Core state, `task_events`, artifacts, projections, and existing errors or diagnostics as MCP tools. State-changing operator outcomes must enter Core or a documented recovery path that preserves Core ordering; operator output must not become a parallel source of state truth.

## What you are not building yet

Keep the first implementation narrow. Do not build these as prerequisites unless owner docs promote them:

| Capability | Stage boundary |
|---|---|
| Dashboard, hosted workflow UI, or rich UI | Not authority, evidence, close readiness, final acceptance, or residual-risk acceptance for v0.1 through v0.4. |
| Broad connector ecosystem or marketplace | Outside staged delivery beyond the first local authority path unless promoted. |
| Context Index | Read-only v1+ candidate; not authority or read/write prerequisite. |
| Browser QA Capture | v1+ candidate; not required automation, Manual QA replacement, or acceptance replacement. |
| Cross-Surface Verification | v1+ automation candidate; detached verification can be proven locally before this. |
| Native hook expansion, Advanced Sidecar Watcher, preventive guard expansion | Capability-dependent enhancements only; claims require a proven concrete pre-tool block or observation path. |
| Local Derived Metrics or long-term metrics | Read-only diagnostics; not staged-delivery-critical state, authority, or readiness. |
| Team workflow, shared workspaces, permissions, profile import/export, parallel orchestration | Future coordination scope; not required for the local single-project authority path. |

v0.1 Core Authority Slice may display cooperative or detective guard/freeze status and may hold or narrow work through existing scope, Autonomy Boundary where already present, and `prepare_write` behavior. Surface labels do not upgrade the stored guarantee level.

Useful later capabilities can appear only as read-only displays, metadata, artifact candidates for existing owner paths, or fixture candidates until their owner docs define capability profile, redaction/secret/PII policy, retention or test-environment rules when needed, fixture coverage, fallback behavior, and no projection-as-canonical dependency. They must not be required to run v0.1 Core Authority Slice, to complete v0.2 User-Facing Harness MVP, or to claim staged-delivery close readiness.

## The first proof

The first runnable target is v0.1 Core Authority Slice: the smallest runnable path that proves Harness can make and enforce one authority decision. Kernel Smoke is the narrow future authoring label for this target's smoke checks, not a full conformance suite.

v0.1 proves the internal authority loop, not the product MVP, not template completeness, and not broad automation.

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

v0.1 Core Authority Slice is not the User-Facing Harness MVP. It proves the write authority path is alive. Use [First Runnable Slice](first-runnable-slice.md#doc-level-acceptance-checks) for doc-level acceptance checks, and use [Conformance Fixtures Reference](../reference/conformance-fixtures.md#conformance-fixture-format) for exact fixture semantics.

## The user-facing MVP proof

The first product MVP target is v0.2 User-Facing Harness MVP. It is reached after v0.1 Core Authority Slice, not by expanding the first runnable batch. It proves that ordinary requests can become visible Harness work: scope, user-owned judgment, evidence, close readiness, final acceptance, and residual risk preserved in a local authority record. Evidence and projection are supporting mechanisms, not the product value by themselves.

It should show:

- ordinary user language can start or resume tracked work without requiring Harness vocabulary
- the work is clarified into scope, non-goals, acceptance criteria, evidence expectations, close readiness, and judgment boundaries
- product/UX judgment and material technical architecture judgment can be presented separately
- small direct changes and tracked work use different procedural budgets without bypassing authority
- close blocks when required evidence or user judgment is missing
- residual risk is visible before successful acceptance or close when close-relevant risk exists
- final acceptance is distinct from sensitive-action Approval and residual-risk acceptance
- residual-risk acceptance, when supported, is visibly distinct from final acceptance
- user-facing projections or cards are derived from Core records and are sufficient without template polish becoming authoritative

## The hardened local reference proof

The later [hardened local reference target](../reference/glossary.md#hardened-local-reference-target) is reached through v0.3 Agency Assurance Pack and v0.4 Operations & Handoff Pack after v0.2 User-Facing Harness MVP, not as the first implementation batch. It is an umbrella target rather than a separate stage, fixture profile, or alternate implementation path. It adds the remaining conformance needed for an agent to act with honest boundaries:

- Decision Packet quality and user-judgment routing
- separation between sensitive-action Approval, Decision Packets, and Write Authorizations
- residual-risk visibility before acceptance and close
- detached verification independence
- Manual QA records and QA blockers
- feedback-loop, TDD, stewardship, and context-hygiene validators
- projection and reconcile completeness
- recovery, export, and artifact integrity behavior
- release handoff report/export behavior where owner docs define it
- later-boundary checks that keep broad automation in v1+ Expansion
- fixture coverage through the named Agency Assurance Pack fixtures and Operations & Handoff Pack or promoted-expansion fixtures

The hardened local reference target is complete only when future conformance proves behavior through Core state, events, artifacts, projection/freshness facts, and errors rather than rendered prose or renderer output alone.

## Build reading path

Read the Build layer in this order:

1. [Implementation Overview](implementation-overview.md) for current status, maintainer handoff, and the future system shape.
2. [MVP Plan](mvp-plan.md) for v0.1 through v0.4 staged delivery, stage boundaries, and the server-coding decision log.
3. [First Runnable Slice](first-runnable-slice.md) for the v0.1 implementation sequence.
4. [Runtime Walkthrough](runtime-walkthrough.md) for the request-to-close runtime path.

Use [Roadmap](../roadmap.md) for v1+ Expansion candidates and promotion rules.

Then use the [Reference Index](../reference/README.md) to pick the current owner for exact behavior:

- [Kernel Reference](../reference/kernel.md) for entities, gates, state logic, `prepare_write`, and `close_task`.
- [Runtime Architecture Reference](../reference/runtime-architecture.md) for runtime spaces, Core flow, artifacts, projection/reconcile, and guarantee levels.
- [MCP API And Schemas](../reference/mcp-api-and-schemas.md) for public resources, tools, schemas, errors, artifact refs, idempotency, and state conflict behavior.
- [Storage And DDL](../reference/storage-and-ddl.md) for runtime layout, DDL, migrations, locks, artifacts, baselines, projection jobs, and validator-run storage.
- [Operations And Conformance Reference](../reference/operations-and-conformance.md) for operator semantics and conformance run overview.
- [Conformance Fixtures Reference](../reference/conformance-fixtures.md) for the core conformance model, fixture body shape, assertion semantics, and the reduced Kernel Smoke queue.
- [Future Fixture Catalog](../reference/future-fixture-catalog.md) for detailed later scenario candidates that are not early-stage requirements by themselves.
