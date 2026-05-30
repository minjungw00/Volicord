# Glossary Reference

## What this document helps you do

Use this glossary to confirm official Harness terms, capitalization, record names, and non-substitution boundaries while reading other docs.

This is reference documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before the documentation set is accepted for implementation planning. The first runnable target is v0.1 Core Authority Slice, with Kernel Smoke as its narrow conformance authoring profile. The first product MVP target is v0.2 User-Facing Harness MVP. v0.3 and v0.4 harden assurance, stewardship, operations, and handoff behavior, and v1+ Expansion remains roadmap scope unless owner docs promote and prove it.

## Read this when

Read this when you need to check a Harness term, avoid mixing authority paths, or find the reference owner for exact behavior.

## Before you read

For a first explanation of Harness concepts, use the Learn path. For exact behavior, follow the owner links below or the links inside individual definitions.

## Main idea

The glossary is a lookup aid and owner map. It keeps public terms, internal implementation terms, capitalization, and short non-substitution reminders consistent, but it is not a substitute for the owner reference documents.

## Reference scope

This glossary owns official term wording, capitalization reminders, record-name orientation, and owner routing. It does not own kernel behavior, public MCP schemas, storage DDL, projection rules, template bodies, connector capability profiles, or conformance fixture semantics.

## Public Terms

Use these words first in user-facing docs, prompts, and status summaries. They are intentionally plain so users can work with Harness without learning record names.

| Public term | Plain meaning |
|---|---|
| work | The thing the user wants completed, answered, investigated, or decided. |
| scope | What may change, what is out of bounds, and where the agent should stop before continuing. |
| judgment | A user-owned choice, such as a product direction, material technical trade-off, sensitive-action approval, scoped QA or verification waiver, final acceptance, or residual-risk acceptance. |
| evidence | Durable support for a claim about the work. |
| close readiness | What still has to be true before the work can finish or close. |
| risk | Known uncertainty, limitation, skipped check, trade-off, or possible consequence that should remain visible. |

User-facing docs should explain the plain concept first. Add exact Harness labels in parentheses only when they help explain a boundary, blocker, source ref, or reference link.

## Internal Implementation Terms

These are implementation labels used by references, APIs, schemas, records, and status refs. Users do not need to use these terms in prompts; agents should translate ordinary requests into the right Harness procedure.

| Internal term | Plain-language explanation |
|---|---|
| Change Unit | The bounded work scope for product writes. It says what may change but does not authorize a write by itself. |
| Decision Packet | The recorded path for a specific user-owned judgment that blocks progress, write, waiver, acceptance, risk handling, or close. |
| Write Authorization | The Harness result that one specific product-write attempt may proceed now after scope and other checks. |
| Evidence Manifest | A record mapping completion conditions or acceptance criteria to supporting evidence refs. |
| Projection | A readable view rendered from Harness state, such as a report or Journey Card. It displays state but does not replace it. |
| Autonomy Boundary | The choices the agent may make inside the active scope without asking the user again. |
| `task_events` | The internal event log table for task state changes. It is a reference/schema term, not user-facing vocabulary. |

## Owner map

| Term family | Reference owner |
|---|---|
| Task, Change Unit, gates, close, sensitive-action approval, final acceptance, verification, QA, residual risk, write authority | [Kernel Reference](kernel.md) |
| MCP resources, MCP tools, public schemas, errors, `ValidatorResult`, `ProjectionKind` | [MCP API And Schemas](mcp-api-and-schemas.md) |
| SQLite records, artifact layout, enum hardening, `tree_hash`, `request_hash` storage use | [Storage And DDL](storage-and-ddl.md) |
| Projections, managed blocks, projection freshness, Markdown reports, template bodies | [Document Projection Reference](document-projection.md); [Template Reference](templates/README.md) |
| Discovery and Shared Design, design quality, stewardship, Feedback Loop finding routing, context hygiene, severity composition, policy contracts | [Design Quality Policies](design-quality-policies.md) |
| Surface capability, guarantee display, connector behavior | [Agent Integration Reference](agent-integration.md) |
| Security assets, trust boundaries, threat categories, high-risk control expectations | [Security Threat Model Reference](security-threat-model.md) |
| Operator procedures, conformance run overview, docs-maintenance reporting | [Operations And Conformance Reference](operations-and-conformance.md) |
| Conformance fixture body shape, assertion semantics, suite catalogs, and examples | [Conformance Fixtures Reference](conformance-fixtures.md) |

## Official Terms

### Agency Conformance

The degree to which harness behavior, projections, validators, and close decisions preserve the user's Strategic Agency. Agency conformance checks whether the work journey is followable, user-owned judgment is explicit, autonomy boundaries are respected, Decision Packets exist for blocking user-owned judgment, and residual risk is visible before acceptance.

### Acceptance

The user's final judgment that the result of the work is acceptable after evidence, verification, Manual QA status, and close-relevant residual risk are shown or confirmed absent. Required Acceptance is recorded through the kernel acceptance path, including a Decision Packet user decision, `task_gates.acceptance_gate`, and `state.sqlite.task_events`. Acceptance is separate from sensitive-action approval, assurance, verification, Manual QA, evidence sufficiency, waiver, and residual-risk acceptance. It does not authorize more writes, accept known risk by itself, erase residual risk, or retroactively satisfy a missing check.

### Acceptance Gate

The kernel gate for required user acceptance. Its value set and compatibility meaning are owned by [Acceptance Gate](kernel.md#acceptance-gate). Acceptance cannot substitute for QA or verification.

Required Acceptance in the current reference model is recorded through a Decision Packet user decision, `task_gates.acceptance_gate`, and `state.sqlite.task_events`; there is no separate acceptance record or table.

### Approval

A limited prior user authorization allowing a specific sensitive action or bounded sensitive operation to proceed within a defined scope. Approval is bound to paths, tools, commands or command classes, network targets, secret scope, baseline, sensitive categories, and expiry conditions. When Approval is requested, Core captures the user judgment through an approval-shaped Decision Packet and linked Approval record; granted sensitive-action Approval still requires a later compatible `prepare_write` result before any Write Authorization exists. Approval is sensitive-action permission only: it is not generic agreement, final acceptance, residual-risk acceptance, QA waiver, verification waiver, correctness proof, or a substitute for user-owned product or material technical judgment.

### Approval Gate

The kernel gate for sensitive-action Approval. It is required only when sensitive categories are present. Granted sensitive-action Approval does not prove correctness, imply final acceptance, accept residual risk, waive QA or verification, resolve user-owned judgment, or create Write Authorization.

### Assumption Register

A Discovery or Shared Design support/projection list of assumptions the agent is using before implementation planning. It should name source, confidence, owner, and what would change if the assumption fails. These are recommended display/support contents, not a standalone schema or canonical record field list. The register helps shape a Discovery Brief, first implementation candidate, work split, or First Safe Change Unit Candidate, but it is not user approval, sensitive-action Approval, acceptance, residual-risk acceptance, evidence, close readiness, scope authority, or Write Authorization.

### Artifact

A recorded output used for evidence, recovery, or audit. See Raw Artifact for the canonical evidence-file boundary.

### Artifact Reference

A structured pointer to a raw artifact file registered in the artifact store, including identity, kind, URI or path, hash, size, content type, redaction state, and task/run relationship. In [Storage And DDL](storage-and-ddl.md), artifact refs and `artifact_links` are Task-scoped. Artifact kinds such as `bundle`, `manifest`, or `export_component` describe files; owner links still point to existing state or Task-scoped projection records.

### Autonomy Boundary

The Change Unit semantics that record the user-owned judgment boundary inside which an agent may proceed without asking for additional user judgment. In plain terms, it says what the agent may decide alone inside the active Change Unit. Routine implementation details may be inside the boundary; public API or module contract changes, security or privacy trade-offs, UX or product behavior trade-offs, material dependency or migration direction, scope expansion, and residual-risk acceptance require explicit user judgment and must not be inferred from broad autonomy.

It is not a scope grant or write authority and does not authorize paths, tools, commands, network targets, secret access, or sensitive categories outside the active Change Unit. A Decision Packet may authorize updating the Autonomy Boundary or proposing a Change Unit update, but the resulting write still requires compatible Change Unit scope and sensitive-action Approval when sensitive categories apply. Exact kernel behavior is owned by [Autonomy Boundary](kernel.md#autonomy-boundary), with policy placement in [Design Quality Policies](design-quality-policies.md#autonomy-boundary-autonomy_boundary).

### Assurance

The technical confidence level supported by recorded checks and verification independence.

```text
none | self_checked | detached_verified
```

An Eval verdict alone does not upgrade assurance. `detached_verified` requires passed verification with valid independence and no same-session self-review violation.

### Baseline

A captured repository state used to judge scope, approval drift, evidence freshness, and verification validity.

### `tree_hash`

The deterministic hash of a baseline file snapshot, computed from sorted NFC-normalized relative POSIX paths after ignored paths are excluded, with file bytes, size, executable bit, and symlink target handling defined by [Storage And DDL](storage-and-ddl.md).

### Capability Profile

A declared and verified description of what a connected agent surface can actually do. It records target profile, support tier, guarantee level, supported features, risks, fallbacks, and last verification time. The harness does not infer capability from product name alone.

### Capability Tier

A coarse integration level for a connected surface.

```text
T0 Context | T1 Skill | T2 MCP | T3 Capture |
T4 Guard | T5 Isolation | T6 QA Capture
```

Capability tiers describe available integration support; they are not kernel gates.

### Change Unit

The scoped implementation unit that bounds product writes. A product write requires an active Change Unit whose scope covers the intended paths, tools, commands, network targets, and sensitive categories, but the Change Unit does not itself authorize the write. Core allows the write through `prepare_write` and applicable gates.

### Close Reason

The canonical reason a Task reached a terminal close state.

```text
none | completed_verified | completed_self_checked |
completed_with_risk_accepted | cancelled | superseded
```

### Codebase Stewardship

The responsibility to preserve the product codebase as a durable asset. It includes attention to domain language, module boundaries, interface contracts, dependency direction, testability, maintainability, and future-change risk.

### Common Tool Envelope

The shared fields carried by public MCP tool calls: `request_id`, `idempotency_key`, `expected_state_version`, `project_id`, optional `task_id`, `surface_id`, optional `run_id`, `actor_kind`, and `dry_run`.

### Cooperative Guarantee

A guarantee level where the agent surface is expected to follow harness instructions and MCP decisions. The harness can guide behavior, but the surface may not provide hard pre-execution enforcement.

### Connector Manifest

A generated manifest that records connector-generated and connector-managed paths, MCP config snippets, managed block hashes, capability/profile freshness, capture/guard/isolation notes or mechanisms, manual fallback notes, and drift or stale status. It prevents generated or managed surface files from being silently overwritten. The full manifest contract is owned by [Agent Integration Reference](agent-integration.md#generated-manifest-expectations).

### Context Hygiene

The policy of keeping always-on context short and current: keep the compact rule set to ten items or fewer, read current status or the Journey Card first, push the current phase bundle, and keep larger records pull-on-demand. Phase-relevant pushed context may include the Journey Card or compact status card, active Decision Packet, Autonomy Boundary, Write Authority Summary, active scoped Change Unit, acceptance criteria, approval status, evidence refs, residual-risk summary, gate summary, and projection freshness. Older PRDs, designs, logs, module maps, old projections, closed issues, Reference contracts, and oversized raw artifacts are pulled only when the current Intake, Discovery, Write, Evidence, Verification, or Close phase needs them. Indexed, retrieved, remembered, or summarized context belongs here as refs or source-linked excerpts. It helps decide what to inspect, not what Harness has authorized, verified, accepted, waived, risk-accepted, or closed.

Stale chat memory is pull-only context. It cannot authorize writes, satisfy gates, close tasks, accept results, waive QA or verification, accept residual risk, replace current state, or repair stale projections unless the relevant owner path records the change.

### Context Index

A later read-only context provider that may surface relevant projections, artifact refs, repo files, docs, or notes. Until promoted through owner docs, it is a v1+ Expansion candidate and non-authoritative retrieval only; even after promotion, it cannot replace existing authority paths unless those owner docs explicitly change. Retrieved context may point to sources to inspect, but it must not authorize writes, resolve decisions, grant Approval, create evidence, perform verification, accept risk, satisfy gates, or close Tasks. The exact future-feature boundary is owned by [Roadmap: Context Index](../roadmap.md#context-index), with connector handling in [Agent Integration](agent-integration.md#context-pushpull-principles).

### Decision Gate

The Task-level aggregate gate for blocking user-owned judgment before progress, write, or close can continue. The canonical field is `decision_gate`; its value set and recompute rule are owned by [Decision Gate](kernel.md#decision-gate). It is recomputed from relevant blocking Decision Packets and detected blockers, and it does not substitute for approval, verification, Manual QA, or acceptance.

### Decision Packet

A canonical kernel state record for blocking user-owned judgment. It names the decision needed, `decision_kind`, `judgment_domain`, options, recommendation when available, trade-offs, affected scope, evidence, residual risk, owner, status, and next action. Decision Packet record IDs use `DEC-*`; record-level status is owned by [Decision Gate Aggregate Recompute](kernel.md#decision-gate-aggregate-recompute) and the public `DecisionPacket` schema, and relevant statuses feed the Task-level `decision_gate`. Required Decision Packet visibility is provided through Task/status/next/judgment-context and decision-packet surfaces; standalone `DEC` Markdown renderings are optional projections or proposal surfaces unless enabled. Public API/interface choices, architecture direction, domain-language conflicts, module boundary changes, waivers, acceptance, and residual-risk choices use this path when user-owned product judgment or material technical judgment blocks progress, writes, close, or a public commitment. Broad approval text does not satisfy a Decision Packet unless it answers the specific recorded route and option.

`judgment_domain` is the schema-owned user-visible grouping for a Decision Packet. Values are `product_ux`, `technical_architecture`, `security_privacy`, `qa_acceptance`, `residual_risk`, `scope_autonomy`, and `mixed`; displays may translate them to friendly labels such as Product / UX or Security / privacy. `decision_kind` controls lifecycle, payload branch, gate meaning, and state-transition semantics. `judgment_domain` helps readers understand the kind of judgment being asked, but it is not a status, gate, owner record, validator input, close aggregation rule, or authority path. Cross-cutting decisions should show secondary considerations in trade-offs, affected gates, risk, evidence, or follow-up rather than treating the domain as exclusive. Displays should make the decision title, what the user is deciding, why it is needed now, options, trade-offs, recommendation, uncertainty, deferral consequence, and residual risk when relevant visible without changing the owner contracts for `decision_kind`, Approval, acceptance, QA, residual-risk acceptance, close, or Write Authorization.

### Decision Request

Optional routing, interaction, idempotency replay, or compatibility handoff metadata that may point to a canonical Decision Packet. A minimal v0.1 Core Authority Slice implementation may omit it. A Decision Request is not decision authority, never satisfies `decision_gate`, approval, acceptance, waiver, residual-risk acceptance, or close by itself, and is only relevant to gate aggregation through a linked compatible `decision_packet_id`.

### Design Gate

The kernel gate for required design-quality preconditions such as shared design, domain language, TDD trace, module/interface review, or other policy-pack requirements.

### Design-Quality Policy Pack

The owner document for design-quality policy contracts and severity composition. It covers shared design, decision quality, autonomy boundary, domain language, vertical slice, feedback loop, TDD trace, module/interface review, codebase stewardship, Manual QA, and context hygiene. It influences gates, validators, evidence, write blockers, and close blockers but does not redefine the kernel state machine.

### Detached Verification

Verification performed across a meaningful independence boundary, such as a fresh session, fresh worktree, sandbox, or manual evaluator bundle. Same-session self-review is not detached verification, and subagent context is not detached by default.

### Discovery

A workflow posture before implementation planning and before write authority where the agent clarifies requirements. It separates goal, user value, non-goals, acceptance criteria, facts the agent can inspect from repo/docs/Harness state, assumptions, judgments only the user can make, product/UX judgment candidates, technical architecture judgment candidates, security/privacy judgment candidates, QA and verification expectations, and first implementation candidates or work split proposals. It asks the user only for decisions the codebase and current Harness context cannot answer, may ask multiple targeted questions grouped by decision area, and can pause when inspectable facts and user-owned decisions are separated enough to propose safe next work without hiding unresolved judgment. Discovery outputs route to Shared Design, Decision Packet candidates, and Change Unit shaping. Phrases such as first implementation candidate and work split proposal are proposal/support phrases, not standalone schema fields, canonical record types, gate values, projection kinds, or authority paths. Discovery is not approval, sensitive-action Approval, Write Authorization, evidence, verification, QA, acceptance, residual-risk acceptance, close, scope authority, or a new authority path.

### Discovery Brief

A compact Discovery or Shared Design support/projection summary of the clarified goal, user value, non-goals, acceptance criteria, inspectable facts, question queue, assumption register, separated user-owned judgments, product/UX, technical architecture, security/privacy, QA and verification expectations, and first implementation candidate or work split. It may include a First Safe Change Unit Candidate when product writes are near. These are recommended display/support contents, not a standalone schema or canonical record field list. A Discovery Brief can inform Shared Design, Decision Packet candidates, and Change Unit shaping, but does not by itself create canonical scope, resolve decisions, authorize writes, prove evidence, record residual-risk acceptance, accept results, or close a task.

### Detective Guarantee

A guarantee level where the harness can detect violations and mark state blocked, stale, partial, or failed after observation.

### Direct

A work mode for small, low-risk changes with obvious scope and result. Direct product writes still require an active scoped Change Unit. Direct includes the tiny direct profile for trivial typo, single-sentence docs, or obvious rename work; Tiny is not a top-level mode and does not bypass user-owned judgment, sensitive-action Approval, security boundaries, evidence, scope, Write Authorization, residual-risk visibility, or close rules.

### Docs-Maintenance Conformance

A read-only documentation maintenance check profile that detects drift in bilingual parity, links, owner boundaries, stable catalogs, glossary terms, source-of-truth phrasing, TODO usage, and non-owner duplicate contracts. Its rule bodies are owned by the [Authoring Guide](../maintain/authoring-guide.md#docs-maintenance-checks), and operator reporting and entrypoint expectations are owned by [Operations And Conformance Reference](operations-and-conformance.md#docs-maintenance-profile). It is a docs-only profile, not runtime conformance or task state authority.

### Domain Language

The product's canonical vocabulary and meanings. The canonical source is `domain_terms`; Markdown domain-language documents are projections and proposal surfaces. A term conflict can affect `design_gate` through policy validation, and it routes to a Decision Packet when choosing the meaning is user-owned product judgment or material technical judgment.

### Domain Term

A canonical structured record in `domain_terms` that stores a product term, meaning, code representation, related terms, source, status, and boundaries such as "not this." Public state refs use `record_kind=domain_term`.

### Evidence

Recorded support for claims about the work, such as diffs, logs, tests, run summaries, screenshots, Eval records, Manual QA records, and registered artifact refs. Evidence supports specific acceptance criteria, completion conditions, or close-relevant claims through owner records such as Evidence Manifests and ArtifactRefs; it is not the agent merely saying the work is done, and it is not made sufficient by Markdown report prose alone.

### Evidence Gate

The kernel gate for required evidence coverage. Its value set and close meaning are owned by [Evidence Gate](kernel.md#evidence-gate).

### Evidence Manifest

A state record mapping acceptance criteria or completion conditions to supporting evidence references. Sufficiency depends on the coverage of those criteria and conditions by current owner records and `ArtifactRef` refs, not on artifact count or report prose.

### Evidence Profile

A named evidence sufficiency profile, such as `advisor`, `direct docs-only`, `direct code`, `work feature`, `UI/UX/copy work`, `sensitive work`, or `verification-required work`, that tells validators what evidence is enough for the task shape. Tiny direct docs-only work is handled under Direct evidence expectations with the smallest changed-path, patch-summary or diff-ref, and self-check support; it is not a separate authorization path.

### Evidence Sufficiency

The close-relevant judgment that required acceptance criteria or completion conditions are supported by the Evidence Manifest plus related state records and artifact refs. It is criteria-based: each required row needs compatible current support. It is not judged from chat text or Markdown report prose alone, and evidence can become stale through baseline drift, changed files, approval drift, missing artifacts, or relevant design record changes.

### Eval

A verification result record with verdict, checks performed, evidence reviewed, independence qualifier, blockers, and artifact references.

### Feedback Loop

A canonical support record and recorded path from checks and findings back into state, scope, design, evidence, follow-up work, or close status. Inputs can include tests, typecheck, lint, build, browser smoke, TDD red/green/refactor traces, Manual QA, Eval findings, user decisions, operational findings, and residual-risk decisions. Public refs use `StateRecordRef.record_kind=feedback_loop`; public mutation uses `FeedbackLoopUpdate` on `record_run` or a Manual QA execution link. Feedback loops keep findings from vanishing into chat by routing them to existing owner paths such as Evidence Manifest coverage, Decision Packets, Change Unit updates, Residual Risk records, Manual QA or Eval records, close blockers, or follow-up Task/Change Unit records where applicable.

### Finding

An observed issue, gap, risk, blocker, or noteworthy result from a Run, Eval, Manual QA record, validator, review display, operator diagnostic, or conformance check. A finding is not a standalone authority path and does not affect gates or close by staying in chat or report prose. It becomes state-relevant only when routed through existing owner records or structured results, such as Evidence Manifest gaps, Decision Packet candidates or records, Change Unit updates, Feedback Loop or TDD Trace updates, Manual QA or Eval records, Residual Risk records, reconcile items, close blockers, or follow-up Task/Change Unit records. The routing contract is owned by [Design Quality Policies](design-quality-policies.md#finding-routing) and [Kernel Reference](kernel.md#finding-routing).

### First Safe Change Unit Candidate

The internal Change Unit-shaped expression of a first implementation candidate when product writes are near. It should name included behavior, out-of-bounds behavior, completion conditions, known sensitive areas, and stop conditions without hiding unresolved user-owned judgment. Discovery or Shared Design may produce it after inspectable facts and user-owned decisions are separated, but Discovery does not exist only to find this candidate. These are recommended display/support contents, not a standalone schema or canonical record field list. It is a candidate only: an active Change Unit, compatible scope gate state, and later `prepare_write` are still required before product writes.

### Fixture Assertion Semantics

The conformance comparison rules that say how `expected_state`, `expected_events`, `expected_artifacts`, `expected_projection`, and `expected_error` are matched against captured Core results. They are owned by [Conformance Fixtures Reference](conformance-fixtures.md#fixture-assertion-semantics), live outside the fixture body, and do not allow prose-only matching to pass a fixture.

### Fresh Session

A verification independence profile where the evaluator starts from a task/evidence bundle rather than continuing the lead chat context, reviews the Evidence Manifest and changed files, and records an Eval.

### Fresh Worktree

A verification independence profile where the evaluator checks baseline, changed paths, artifacts, and Evidence Manifest in a separate worktree or equivalent isolated repository state.

### Freeze

A user-facing safety control that requests a hold or narrower posture around current work. Freeze can hold product writes, make the next action stricter, or cause `prepare_write` to block or hold when existing scope is incompatible. It does not directly mutate Change Unit scope, allowed paths, Autonomy Boundary, AFK stop conditions, or related owner records; persistent owner-record changes still use the existing Core state-changing path, Decision Packet route, or owner-record update path. Freeze does not create Write Authorization, approval, evidence, verification, QA, acceptance, residual-risk acceptance, close, or a new authority tier.

### Gate

A canonical kernel field that controls whether a Task may write, proceed, or close. Gates are state, not display text.

### Generated File

A repository file or managed block produced by a connector, projector, or operator tool. Generated files must be tracked by a manifest or projection job when they can drift from canonical state.

### Guarantee Display

The user-facing and connector-facing display of the actual guarantee level for a status or write decision, including limitation notes when enforcement is cooperative or detective.

### Guarantee Level

The honest enforcement strength available for a connected surface or runtime path.

```text
cooperative | detective | preventive | isolated
```

Capability affects validator results, blocked reasons, and display; it is not Approval, Write Authorization, verification, QA, acceptance, residual-risk acceptance, close readiness, or a kernel gate. Exact level meanings are owned by [Runtime Architecture](runtime-architecture.md#guarantee-levels).

### Guard

A user-facing safety control that applies the connected profile's actual enforcement or detection layer. Guard may be cooperative, detective, preventive, or isolated; the name does not imply pre-execution blocking unless a proven `T4` path covers the operation.

### Harness Core

The runtime component that owns state transitions, gate updates, validator interpretation, artifact registration, projection job enqueueing, and close decisions.

### Harness Runtime Home

The local runtime storage area that contains `registry.sqlite`, per-project `project.yaml`, per-project `state.sqlite`, and artifact directories.

### Human-editable Area

A Markdown area where a human can write notes, proposals, questions, or corrections. It is an input surface, not canonical state. Its authority path is `human-editable input -> reconcile_items -> accepted state event/record`.

### Implementation Micro-Plan

A managed `TASK` projection section that shows small execution steps or slices, their purpose, active Change Unit scope alignment or likely paths, selected feedback loop or TDD status when relevant, expected evidence, and stop conditions. It is an execution aid, not canonical state, not a `ProjectionKind`, not scope authority, not approval, and not Write Authorization. Editing its text does not mutate state except through an accepted reconcile outcome or Core state-changing action.

### Isolated Guarantee

A guarantee level where risky work is separated by a worktree, sandbox, process boundary, or equivalent isolation mechanism.

### Journey Card

A compact human-readable projection of the current Task position: state, next action, scope, active scoped Change Unit, Autonomy Boundary, blockers, active Decision Packet, Write Authority Summary, acceptance criteria, approval status, evidence, verification, QA, acceptance, residual risk, and projection freshness. A Journey Card is display, not canonical state, and it is rendered from current owner records rather than stale chat memory.

### Journey Spine

The state-derived continuity model for a Task's ordered work journey. It is reconstructed from Task, Change Unit, Run, Decision Packet, Approval, Evidence Manifest, Eval, Manual QA, Residual Risk, `task_gates.acceptance_gate`, acceptance Decision Packet user-decision state, close events, artifact references, and `state.sqlite.task_events`, not from chat memory. Journey Card and Journey Spine Markdown views are projections.

### Journey Spine Entry

A canonical support record for durable continuity annotations that cannot be fully reconstructed from existing state events or owner records. Journey Spine Entry records supplement the Journey Spine; they do not replace Task, Change Unit, Run, Decision Packet, Residual Risk, evidence, verification, QA, acceptance gate/decision state, close state/events, artifact, or event authority.

### Interface Contract

The canonical record of a module or external boundary's public interface, inputs, outputs, errors, compatibility impact, callers, and boundary tests. The canonical source is `interface_contracts`. Public state refs use `record_kind=interface_contract`. The record documents the interface understanding; it is not Approval, acceptance, residual-risk acceptance, or Write Authorization. Public interface or compatibility choices route through the existing design-quality and Decision Packet paths when user-owned judgment is required.

### JSON `TEXT` Field

A SQLite `TEXT` column whose stored value is JSON. The `TEXT` type is reference storage flexibility only; Core must validate the value before commit against the API-owned or storage-owned shape, and malformed or schema-incompatible JSON is invalid state.

### Local Derived Metrics

Later diagnostic-only metrics derived from local records such as `state.sqlite.task_events`, runs, validator results, projection jobs, and reconcile items. Until promoted through owner docs, metric readouts may report rates, counts, durations, or guard-trigger summaries only as read-only diagnostics. The exact non-authority boundary is owned by [Roadmap: Local Derived Metrics](../roadmap.md#local-derived-metrics).

### Manual QA

Human inspection of experiential product quality such as UX, workflow, copy, visual output, accessibility, and product fit. Manual QA is recorded through the Manual QA record or a valid QA waiver path when required; browser smoke, screenshots, Browser QA artifacts, tests, or verifier notes may support context but are not Manual QA judgment by themselves. Exact gate behavior is owned by [QA Gate](kernel.md#qa-gate), with policy requirements in [Design Quality Policies](design-quality-policies.md#manual-qa-manual_qa).

### Manual Bundle

A verification handoff package for a human or separate evaluator. It includes task summary, acceptance criteria, Change Unit scope, approval scope, diff/log/test artifacts, Evidence Manifest, known risks, and enough context to record an Eval verdict.

### Manual QA Record

A record-level Manual QA result, including performer, profile, result, artifacts, findings, waiver reason when applicable, and next action. Its result value set is owned by [QA Gate](kernel.md#qa-gate) and [`harness.record_manual_qa`](mcp-api-and-schemas.md#harnessrecord_manual_qa). Pending required QA is represented by `qa_gate=pending`; it is not a Manual QA record result.

### `managed_hash`

The drift-detection hash of the projector-owned managed block body, excluding `HARNESS:BEGIN` and `HARNESS:END` marker lines. It is not canonical state and does not make a Markdown projection authoritative.

### Managed Block

A Markdown block delimited by harness markers and regenerated by the projector from state records and artifact refs. Direct edits to a managed block create drift or reconcile candidates; they do not become state by themselves.

### MCP Resource

A read-only MCP surface for current project, task, design, policy, status, or bundle information. Resources do not mutate state.

### MCP Server Unavailable

`MCP_SERVER_UNAVAILABLE` is the diagnostic condition where a tool call cannot reach Core. No authoritative Core response is possible, and the caller must diagnose or reconnect before claiming state changes. The stable public error code remains `MCP_UNAVAILABLE`.

### Surface MCP Unavailable

`SURFACE_MCP_UNAVAILABLE` is the diagnostic condition where Core or an operator can observe that the connected surface lacks usable MCP, has stale MCP configuration, or cannot call required MCP tools. Product writes are held by instruction on cooperative surfaces or blocked by stronger guards when available. Core responses may use `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` with `details.mcp_unavailable_kind`; the diagnostic label is not a public `ErrorCode` value.

### MCP Tool

A public MCP operation that asks Core to validate, record, transition, or close state. State changes must go through tools or reconcile actions, not resource reads.

### Markdown Report

A human-readable document generated from state records and artifact references. A Markdown report is not a raw artifact by default and does not become canonical state.

### Natural-Language Consent

A user utterance such as "go ahead," "proceed," or "looks good" that may answer a pending question only when the active prompt makes the exact decision route, option, scope, affected gates, consequences, and remaining non-approved items unambiguous. Natural-language consent is not its own authority path. Ambiguous consent must be clarified rather than broadened into sensitive-action Approval, final acceptance, residual-risk acceptance, QA waiver, verification waiver, or Write Authorization.

### Module Map

The product's map of modules, responsibilities, public interfaces, dependency direction, internal complexity, test boundaries, owner decisions, and watchpoints. The canonical source is `module_map_items`. A module boundary update records the shared technical understanding; it does not approve writes or accept risk. Boundary changes that shift product commitments, caller obligations, or architecture direction route through design-quality policy and Decision Packet paths when user-owned judgment is required.

### Module Map Item

A canonical structured record in `module_map_items` that stores a module's role, public interface, dependencies, internal complexity, test boundary, owner decision, and watchpoints. Public state refs use `record_kind=module_map_item`.

### Policy Contract

The standard form used by design-quality policies: `name`, `applies_when`, `default_requirement`, `allowed_waiver`, `required_record`, `validator`, `evidence`, and `close_impact`.

### Preventive Guarantee

A guarantee level where the harness or connector can block a violating action before it executes.

### Projection

A human-readable rendering of canonical state records and artifact references. Projection is useful for reading and decision-making, but it cannot override canonical state.

### ProjectionKind

The API enum for projection job and template kinds. Tiers, values, and extension rules are owned by [Shared schemas](mcp-api-and-schemas.md#shared-schemas). Tier labels such as `Reference-required` are support expectations, not v0.1 Core Authority Slice run obligations; v0.1 has no projection-rendering exit requirement beyond preserving any owner-produced freshness/read facts. No ProjectionKind makes a projection canonical state.

### Projection Freshness

The relationship between a projection and its source records, managed hash, artifact refs, and projection job state. Its value set is owned by [MCP API And Schemas](mcp-api-and-schemas.md) and [Document Projection Reference](document-projection.md).

### Projection Job

A durable outbox record that asks the projector to render a Markdown projection from committed state records and artifact refs. `record_kind=projection` identity is `projection_jobs.projection_job_id`; project-level projection jobs do not by themselves create project-scoped artifact links in the current Task-scoped artifact DDL.

### Question Queue

A Discovery or Shared Design support/projection list of open questions classified as blocking, useful-but-not-blocking, or codebase-answerable. These are recommended display/support contents, not a standalone schema or canonical record field list. Blocking questions may route to a Decision Packet candidate when user-owned judgment is required. Useful-but-not-blocking questions can be parked, deferred, or turned into follow-up work. Codebase-answerable questions should be answered from current repo, docs, Harness state, or source refs rather than asked of the user. The queue is not a Decision Packet, gate, approval, evidence, acceptance, close, or Write Authorization.

### QA Gate

The canonical kernel gate for required Manual QA. `manual_qa_record.result` is record-level; `qa_gate` is the close-relevant aggregate state. `qa_gate=pending` means required QA has not yet produced a satisfying Manual QA record, or the latest relevant Manual QA record does not satisfy policy.

### Raw Artifact

A durable evidence file in the artifact store, such as a diff, log, bundle, screenshot, checkpoint, or manifest file. Raw artifacts are distinct from state records and Markdown reports.

### Reconcile

The process that turns human-editable input or projection drift into an accepted state change, rejected proposal, note, decision, or deferred item.

### Reconcile Item

The canonical candidate record created from human-editable input or projection drift before a reconcile decision accepts, rejects, converts, or defers it.

### Reference Surface

The single agent surface targeted by v0.1 Core Authority Slice. It demonstrates the kernel and connector contract without implying broad connector-surface support.

### Recommended Playbook

Non-authoritative status/next display guidance computed from current state and policy/playbook context. It suggests a procedure for the current stage, such as review, TDD, QA, guard check, release handoff, or browser-QA candidacy. Its `playbook_id` is a stable display/routing string identifier, not a Core-owned closed enum or DDL-backed value set. It is not a canonical kernel record, has no DDL table, task event, or projection job of its own, does not authorize writes, satisfy gates, accept results, accept residual risk, or close tasks, and routes user-owned judgment to Decision Packet paths or other existing Core/MCP mutation paths.

### Release Handoff

An optional report/export profile that summarizes release readiness for external PR, review, deployment, rollback, and monitoring processes. It includes close readiness, blockers, evidence refs, verification refs, Manual QA refs, residual-risk refs, changed files, projection freshness, redaction notes, and suggested checklist items. The exact report/export authority boundary is owned by [Operations And Conformance](operations-and-conformance.md#release-handoff-export-profile).

### Role Lens

A non-authoritative skill or playbook surface that lets a user ask for a product, engineering, design, security, QA, or release-handoff review posture. Role Lens output reuses existing routes such as `RecommendedPlaybook`, `DecisionPacketCandidate`, validator/check routes, evidence, Eval or verification, Manual QA, Approval, residual-risk, Change Unit update, and release handoff routes. It is read-only guidance until an existing Core/MCP path records the underlying action, so it does not mutate state, authorize writes, satisfy gates, accept results, accept residual risk, close tasks, or upgrade assurance by itself. The exact non-authority boundary is owned by [Agent Integration](agent-integration.md#role-lens-behavior).

### Report Projection

A Markdown report generated from state records and artifact references, such as a Task report, approval report, run summary, evidence manifest report, Eval report, or direct-result report.

The named report projection kinds are projections generated from state records and artifact refs; state authority stays with Core records and evidence-file authority stays with registered artifact files. Exact projection rules are owned by [Document Projection Reference](document-projection.md), and full rendered bodies are owned by [Template Reference](templates/README.md).

### Review Stages

A managed display/procedure split that separates Spec Compliance Review from Code Quality / Stewardship Review. Spec Compliance Review asks whether the requested work is complete under current Harness authority. Code Quality / Stewardship Review asks whether the implementation is maintainable inside the codebase. Review Stages can route findings to validator results, evidence gaps, Decision Packet candidates, Eval or verification needs, Manual QA needs, Approval needs, residual-risk candidates, Change Unit update recommendations, or close blockers. They are not canonical records, `ProjectionKind` values, approval, evidence, verification, QA, acceptance, residual-risk acceptance, close, or Write Authorization. Their exact display-only boundary is owned by [Design Quality Policies](design-quality-policies.md#two-stage-review-display); same-session Review Stages do not create `assurance_level=detached_verified`.

### `request_hash`

The idempotency hash of a tool request, computed from canonical UTF-8 JSON covering `tool_name`, the schema-normalized request body, and the envelope fields other than `request_id` and `idempotency_key`.

### Residual Risk

A canonical close-relevant support record for known remaining uncertainty, trade-off, limitation, or unchecked condition after evidence, verification, QA, and acceptance work. It records source refs, affected scope, related Decision Packet when applicable, visibility status, accepted risk when applicable, follow-up requirement, and close impact. Known close-relevant Residual Risk must be visible before any successful acceptance or close, or `ResidualRiskSummary.status=none` must confirm no known close-relevant risk. Residual-risk acceptance means the user explicitly accepts a named known remaining risk; it does not mean the result is otherwise verified, accepted, approved for sensitive action, or waived. Accepted risk is metadata/state on the Residual Risk record in the current reference model, not a separate `accepted_risk` state record.

### Risk Accepted Close

A successful close where the user accepts visible close-relevant residual risk, including verification risk when verification was waived. It uses `close_reason=completed_with_risk_accepted`, requires accepted Residual Risk refs, and must not display `assurance_level=detached_verified`. User-facing summaries must keep it distinct from normal `completed_verified` or `completed_self_checked` close.

### Run

An execution attempt by an agent, evaluator, operator, or other actor against a Task and optionally a Change Unit. Runs record baseline, surface, observed changes, commands, artifacts, and summary. A rejected pre-commit `record_run` request is not a Run and must not receive a fabricated Run ID; an audit or violation attempt becomes a Run only when Core deliberately commits it.

### Scope Gate

The kernel gate requiring product writes to be covered by an active scoped Change Unit. Scope is required for write-capable direct and work modes even when approval is not required. Scope Gate does not grant sensitive-action Approval, resolve user-owned judgment, or create Write Authorization; exact values and compatibility are owned by [Scope Gate](kernel.md#scope-gate).

### Severity Composition

The policy-owned rule for merging multiple applicable task-shape defaults, policy contracts, and validator findings. The same concern is the same policy-relevant target, not the whole Task or merely the same validator ID. The rule keeps all findings visible, preserves impacts across different affected gates or blocker targets, and uses the strongest applicable impact only for competing impacts on the same concern. It affects validators, gates, write blockers, close blockers, waivers, and Decision Packet needs, while public primary `ToolError` selection remains API-owned. Exact policy behavior is owned by [Severity composition rule](design-quality-policies.md#severity-composition-rule).

### Shared Design

The minimum recorded shared understanding of a task before implementation hardens into a plan: goal, user value, scope, non-goals, acceptance criteria, inspectable facts, assumptions, decisions, rejected options, domain/module/interface impact, QA and verification expectations, and safe next work. Discovery Briefs, Question Queues, Assumption Registers, first implementation candidates or work splits, and First Safe Change Unit Candidates can feed Shared Design. Shared Design can support shaping and `design_gate` readiness, but it is not final approval, sensitive-action Approval, Acceptance, residual-risk acceptance, QA judgment, evidence, close readiness, or Write Authorization. Markdown renderings of Shared Design are projections and proposal surfaces. Exact policy requirements are owned by [Design Quality Policies](design-quality-policies.md#shared-design-shared_design).

### Source-of-truth

The authoritative source for a fact. In Harness, operational state is canonical in `state.sqlite` current records plus `state.sqlite.task_events`, raw evidence files are canonical in the artifact store, and Markdown documents are projections. Product repository files remain the source for product content; they do not become Harness operational state unless an existing Core, reconcile, artifact-registration, or owner-record path records the relevant Harness fact.

### `state.sqlite.task_events`

The append-only event history table inside `state.sqlite`. Reference event storage does not use a separate event store. Deterministic order is `task_events.event_seq`, not timestamps or event IDs.

### Stable Event Catalog

The kernel-owned compact list of `task_events.event_type` names that staged/reference conformance fixtures may assert in `expected_events`. It classifies stable event names separately from prose examples, fixture shorthand, non-stable implementation-local detail or audit events, validator IDs, Core check names, projection status shorthands, and future extension events.

### State Record

A canonical structured record in kernel state, such as a Task, Change Unit, Decision Packet, Journey Spine Entry, Residual Risk, Run, Approval, Write Authorization, Evidence Manifest, Eval, Manual QA record, Artifact record, Shared Design record, Domain Term, Module Map Item, Interface Contract, Feedback Loop, TDD Trace, or Reconcile Item.

### State Version

An optimistic-concurrency clock for a Core-resolved state scope. Core resolves the primary Task from the envelope, tool-specific input, or active Task when one applies. `expected_state_version`, `ToolResponseBase.state_version`, `EventRef.state_version`, and `task_events.state_version` are interpreted by that affected scope, not as one global event-store sequence.

### Project State Version

The project-scoped state clock stored in `project_state.state_version`. Project-scoped mutations with no Core-resolved primary Task compare `expected_state_version` against this value and return the resulting value as the primary response `state_version`.

### Task State Version

The task-scoped state clock stored in `tasks.state_version`. Task-scoped mutations compare `expected_state_version` against the Core-resolved primary Task's value and return the resulting value as the primary response `state_version`.

### Strategic Agency

The user's durable authority to understand the work journey and make or withhold judgment over goals, scope, design, trade-offs, codebase stewardship, QA, acceptance, and residual risk. The harness preserves Strategic Agency by making state, decisions, evidence, blockers, and remaining risk explicit outside chat.

### Secret Handle

A display-safe reference to sensitive material such as credentials, tokens, certificates, keys, or other secret values. A secret handle may support evidence or approval scope without storing the raw secret in artifacts, connector manifests, projections, exports, screenshots, logs, summaries, or prompt context. Exact storage and API behavior stays with the storage and MCP/API owners.

### Security Threat Model

The reference owner for Harness security assets, trust boundaries, threat categories, and control expectations. It explains risks such as prompt injection in repo docs, projection tampering, stale approval replay, out-of-scope writes, MCP-unavailable state claims, secret leakage through evidence artifacts, artifact hash mismatch, malicious generated connector files, capability overclaiming, and stale context poisoning. It does not own exact DDL, public API schemas, or kernel transitions.

### Surface Capability Check

A validator that reports whether a connected agent surface can satisfy required harness behavior. It affects blocked reasons and guarantee display, but it is not a kernel gate.

### Surface Cookbook

The reference document that contains surface-specific connector notes, generated file details, and profile examples. Common integration rules belong in the agent integration document, not the cookbook.

### Subagent Context

A verification independence profile where a subagent or helper reviews work with some inherited implementation context. It is not detached by default and can qualify only when stricter profile metadata proves a real independence boundary.

### Task

The user value unit tracked by the kernel. It carries mode, lifecycle phase, gates, result, close reason, assurance, current summary, decisions, evidence, and projection status.

### Task Level

A display and routing label for task shape: Tiny, Direct, Work, or High-risk Work. Tiny is a profile under `direct`; Direct is small low-risk code or docs work; Work covers features, UX workflow, auth-facing behavior, schema, public API/interface, and multi-file or multi-step delivery; High-risk Work covers auth, security, privacy, secrets, infra, and similarly sensitive categories. Task Level is not a new kernel `mode` enum, gate, schema field, approval, or Write Authorization source.

### TDD Trace

A record of red, green, and refactor evidence for a Change Unit or behavior slice, or a recorded non-TDD justification where policy allows it. A RED target or plan describes the intended failing check; RED evidence means an actual failing test artifact/log/result or another explicit policy-recognized failing-check evidence. When required, the normal path records RED evidence before non-test implementation writes, GREEN evidence after implementation, and refactor/check evidence when relevant, then links the trace to Evidence Manifest coverage. TDD Trace can be execution evidence for a Feedback Loop, but it is not the canonical selected-loop record; a waiver must point back to the alternate Feedback Loop that will prove behavior.

### Tiny Direct Profile

A Direct subprofile for a typo, single docs sentence, or obvious rename where scope, result, and no-user-judgment boundary are immediately clear. It keeps interaction minimal, but it must escalate to ordinary Direct when scope broadens while remaining low-risk and narrow, or when Evidence Manifest coverage, artifact refs, link/render proof, or other evidence beyond the tiny result note is needed. It must route to Work when product judgment, material technical judgment, architecture choice, public interface/API impact, UX workflow, schema, sensitive category, or multi-step delivery appears.

### Trust Boundary

A separation between Harness surfaces, files, callers, or runtime spaces where input from one side must not be treated as authority on the other side without an owner path. For example, chat text, Product Repository documents, projections, generated connector files, artifact bytes, and MCP caller claims can inform Harness, but they do not become canonical operational state unless Core or another documented owner path accepts their meaning. The trust-boundary map is owned by [Security Threat Model Reference](security-threat-model.md).

### Verification

The process of checking whether the result satisfies the relevant criteria. Verification may support assurance when recorded through a valid Eval path and independence profile, but same-session self-check is not detached verification. Verification is separate from approval, Manual QA, acceptance, and residual-risk acceptance. Exact gate and independence behavior is owned by [Verification Gate](kernel.md#verification-gate) and [`harness.record_eval`](mcp-api-and-schemas.md#harnessrecord_eval).

### Verification Gate

The kernel gate for required verification. A user waiver sets `verification_gate=waived_by_user`; it does not create `detached_verified` assurance.

### Verification Independence Profile

A named minimum qualification for an Eval independence context, such as `same_session`, `subagent_context`, `fresh_session`, `fresh_worktree`, `sandbox`, or `manual_bundle`. A passed Eval must satisfy a valid profile before it can support `detached_verified`.

### Validator Result

A structured result from a validator, including status, guarantee level, target, findings, blocked reasons, and suggested next action.

### Vertical Slice

A Change Unit shape that connects a thin path from trigger/input through domain logic, persistence or state, caller/API boundary, observable output, tests, and optional Manual QA.

### Waiver

An explicit recorded exception to a gate or policy requirement where policy allows it. A waiver names the policy or gate, Task and Change Unit, skipped check or surface, reason, actor, expiry or follow-up when needed, affected gate or close impact, and any close-relevant residual risk that must be visible or accepted through the residual-risk path when required. Verification waiver, design waiver, and QA waiver are allowed under defined rules only when explicit and scoped. Product-write scope, sensitive-action Approval, required evidence coverage, and required acceptance are not waived for successful completion. Verification waiver and QA waiver do not upgrade assurance, imply final acceptance, accept unrelated residual risk, or make skipped checks appear passed.

### Write Authorization

A durable state record created by `prepare_write` for a specific allowed write attempt. It records `basis_state_version`, the affected-scope state version used as the compatibility basis for replay, stale detection, and audit. Distinct compatible `prepare_write` requests create distinct authorizations; idempotent replay may return the committed response. It is single-use for a committed implementation or direct Run, and it does not replace Change Unit scope, sensitive-action Approval, Decision Packet compatibility, evidence, verification, Manual QA, acceptance, or residual-risk visibility.

### Write Authorization Lifecycle Events

The stable event-name set for Write Authorization creation, return, consumption, expiry, staling, revocation, and violation detection. The exact vocabulary and its relationship to `scope_violation_detected` are owned by the [Kernel Stable Event Catalog](kernel.md#stable-event-catalog).

### Write Authority Summary

A user-facing display summary of current write authority for an intended operation, derived from active Change Unit scope, `prepare_write`, approval, baseline, guarantee, Decision Packet refs, and any Write Authorization ref. It is display, not a separate authority record, and it does not authorize work by itself.
