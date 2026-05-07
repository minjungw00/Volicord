# Glossary

## Official Terms

### Agency Conformance

The degree to which harness behavior, projections, validators, and close decisions preserve the user's Strategic Agency. Agency conformance checks whether the work journey is followable, product judgment is explicit, autonomy boundaries are respected, Decision Packets exist for blocking product judgment, and residual risk is visible before acceptance.

### Acceptance

The user's judgment that the result and remaining trade-offs are acceptable. Acceptance is separate from approval, assurance, verification, and Manual QA.

### Acceptance Gate

The kernel gate that records whether required user acceptance is not required, required, pending, accepted, or rejected. Acceptance cannot substitute for QA or verification.

### Approval

A prior user decision allowing a sensitive change to proceed within a defined scope. Approval is bound to paths, tools, commands or command classes, network targets, secret scope, baseline, sensitive categories, and expiry conditions.

### Approval Gate

The kernel gate for sensitive-change approval. It is required only when sensitive categories are present. Granted approval does not prove correctness or imply acceptance.

### Artifact

A recorded output used for evidence, recovery, or audit. See Raw Artifact for the canonical evidence-file boundary.

### Artifact Reference

A structured pointer to a raw artifact file registered in the artifact store, including identity, kind, URI or path, hash, size, content type, redaction state, and task/run relationship.

### Autonomy Boundary

The recorded boundary inside which an agent may proceed without asking for additional product judgment. It is shaped by work mode, active Change Unit scope, approvals, policy requirements, Decision Gates, surface capability, and current blockers. It does not override `prepare_write`, Change Unit scope, sensitive approval, policy validators, QA, verification risk acceptance, or final acceptance.

### Assurance

The technical confidence level supported by recorded checks and verification independence.

```text
none | self_checked | detached_verified
```

An Eval verdict alone does not upgrade assurance. `detached_verified` requires passed verification with valid independence and no same-session self-review violation.

### Baseline

A captured repository state used to judge scope, approval drift, evidence freshness, and verification validity.

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

The scoped implementation unit for product writes. A product write requires an active Change Unit whose scope covers the intended paths, tools, commands, network targets, and sensitive categories.

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

A generated manifest that records connector-managed files, managed block hashes, capability profile, surface target profile, and drift status. It prevents generated surface files from being silently overwritten.

### Context Hygiene

The policy of keeping current state, evidence, and relevant references in context while avoiding stale chat, old PRDs, closed issues, and oversized raw artifacts unless they are explicitly needed.

### Decision Gate

A state-level blocker that requires product judgment before progress, write, or close can continue. A Decision Gate is resolved through a recorded decision path and does not substitute for approval, verification, Manual QA, or acceptance.

### Decision Packet

A recorded decision-support packet for blocking product judgment. It names the decision needed, options, recommendation when available, trade-offs, affected scope, evidence, residual risk, owner, status, and next action. Its canonical form is kernel state; Markdown renderings are projections or proposal surfaces.

### Design Gate

The kernel gate for required design-quality preconditions such as shared design, domain language, TDD trace, module/interface review, or other policy-pack requirements.

### Design-Quality Policy Pack

The set of policy contracts for shared design, decision quality, autonomy boundary, domain language, vertical slice, feedback loop/TDD trace, module/interface review, codebase stewardship, Manual QA, and context hygiene. It influences design, QA, evidence, and close blockers but does not redefine the kernel state machine.

### Detached Verification

Verification performed across a meaningful independence boundary, such as a fresh session, fresh worktree, sandbox, or manual evaluator bundle. Same-session self-review is not detached verification, and subagent context is not detached by default.

### Detective Guarantee

A guarantee level where the harness can detect violations and mark state blocked, stale, partial, or failed after observation.

### Direct

A work mode for small, low-risk changes with obvious scope and result. Direct product writes still require an active scoped Change Unit.

### Domain Language

The product's canonical vocabulary and meanings. The canonical source is `domain_terms`; Markdown domain-language documents are projections and proposal surfaces.

### Domain Term

A canonical structured record in `domain_terms` that stores a product term, meaning, code representation, related terms, source, status, and boundaries such as "not this."

### Evidence

Recorded support for claims about the work, such as diffs, logs, tests, run summaries, screenshots, Eval records, or Manual QA records.

### Evidence Gate

The kernel gate for required evidence coverage.

```text
not_required | none | partial | sufficient | stale | blocked
```

`not_required` means the evidence gate does not apply. `none` means evidence is required but no evidence has been recorded.

### Evidence Manifest

A state record mapping acceptance criteria or completion conditions to supporting evidence references.

### Evidence Profile

A named evidence sufficiency profile, such as `advisor`, `direct docs-only`, `direct code`, `work feature`, `UI/UX/copy work`, `sensitive work`, or `verification-required work`, that tells validators what evidence is enough for the task shape.

### Evidence Sufficiency

The close-relevant judgment that required acceptance criteria or completion conditions are supported by the Evidence Manifest plus related state records and artifact refs. It is not judged from chat text or Markdown report prose alone.

### Eval

A verification result record with verdict, checks performed, evidence reviewed, independence qualifier, blockers, and artifact references.

### Feedback Loop

A recorded path from checks and findings back into state, scope, design, evidence, follow-up work, or close status. Inputs can include tests, typecheck, lint, build, browser smoke, TDD red/green/refactor traces, Manual QA, Eval findings, user decisions, operational findings, and residual-risk decisions. Feedback loops keep findings from vanishing into chat.

### Fresh Session

A verification independence profile where the evaluator starts from a task/evidence bundle rather than continuing the lead chat context, reviews the Evidence Manifest and changed files, and records an Eval.

### Fresh Worktree

A verification independence profile where the evaluator checks baseline, changed paths, artifacts, and Evidence Manifest in a separate worktree or equivalent isolated repository state.

### Gate

A canonical kernel field that controls whether a Task may write, proceed, or close. Gates are state, not display text.

### Generated File

A repository file or managed block produced by a connector, projector, or operator tool. Generated files must be tracked by a manifest or projection job when they can drift from canonical state.

### Guarantee Display

The user-facing and connector-facing display of the actual guarantee level for a status or write decision, including limitation notes when enforcement is cooperative or detective.

### Guarantee Level

The strength of enforcement available for a connected surface or runtime path.

```text
cooperative | detective | preventive | isolated
```

Capability affects validator results, blocked reasons, and display; it is not a kernel gate.

### Harness Core

The runtime component that owns state transitions, gate updates, validator interpretation, artifact registration, projection job enqueueing, and close decisions.

### Harness Runtime Home

The local runtime storage area that contains `registry.sqlite`, per-project `project.yaml`, per-project `state.sqlite`, and artifact directories.

### Human-editable Area

A Markdown area where a human can write notes, proposals, questions, or corrections. It is an input surface, not canonical state. Its authority path is `human-editable input -> reconcile_items -> accepted state event/record`.

### Isolated Guarantee

A guarantee level where risky work is separated by a worktree, sandbox, process boundary, or equivalent isolation mechanism.

### Journey Card

A compact human-readable projection of the current Task position: state, next action, scope, blockers, Decision Gates, evidence, verification, QA, acceptance, residual risk, and projection freshness. A Journey Card is display, not canonical state.

### Journey Spine

The ordered, state-derived thread of a Task's work journey across Change Units, runs, decisions, Decision Packets, evidence, QA, acceptance, residual risk, and close status. It is reconstructed from kernel state and artifact references, not from chat memory.

### Interface Contract

The canonical record of a module or external boundary's public interface, inputs, outputs, errors, compatibility impact, callers, and boundary tests. The canonical source is `interface_contracts`.

### Manual QA

Human inspection of experiential product quality such as UX, workflow, copy, visual output, accessibility, and product fit.

### Manual Bundle

A verification handoff package for a human or separate evaluator. It includes task summary, acceptance criteria, Change Unit scope, approval scope, diff/log/test artifacts, Evidence Manifest, known risks, and enough context to record an Eval verdict.

### Manual QA Record

A record-level Manual QA result, including performer, profile, result, artifacts, findings, waiver reason when applicable, and next action. It feeds `qa_gate` but is not itself the canonical gate.

### Managed Block

A Markdown block delimited by harness markers and regenerated by the projector from state records and artifact refs. Direct edits to a managed block create drift or reconcile candidates; they do not become state by themselves.

### MCP Resource

A read-only MCP surface for current project, task, design, policy, status, or bundle information. Resources do not mutate state.

### MCP Tool

A public MCP operation that asks Core to validate, record, transition, or close state. State changes must go through tools or reconcile actions, not resource reads.

### Markdown Report

A human-readable document generated from state records and artifact references. A Markdown report is not a raw artifact by default and does not become canonical state.

### Module Map

The product's map of modules, responsibilities, public interfaces, dependency direction, and test boundaries. The canonical source is `module_map_items`.

### Module Map Item

A canonical structured record in `module_map_items` that stores a module's role, public interface, dependencies, internal complexity, test boundary, owner decision, and watchpoints.

### Policy Contract

The standard form used by design-quality policies: `name`, `applies_when`, `default_requirement`, `allowed_waiver`, `required_record`, `validator`, `evidence`, and `close_impact`.

### Preventive Guarantee

A guarantee level where the harness or connector can block a violating action before it executes.

### Projection

A human-readable rendering of canonical state records and artifact references. Projection is useful for reading and decision-making, but it cannot override canonical state.

### Projection Freshness

The relationship between a projection and its source records, managed hash, artifact refs, and projection job state. Freshness may be `current`, `stale`, `failed`, or `unknown`.

### Projection Job

A durable outbox record that asks the projector to render a Markdown projection from committed state records and artifact refs.

### QA Gate

The canonical kernel gate for required Manual QA. `manual_qa_record.result` is record-level; `qa_gate` is the close-relevant aggregate state.

### Raw Artifact

A durable evidence file in the artifact store, such as a diff, log, bundle, screenshot, checkpoint, or manifest file. Raw artifacts are distinct from state records and Markdown reports.

### Reconcile

The process that turns human-editable input or projection drift into an accepted state change, rejected proposal, note, decision, or deferred item.

### Reconcile Item

The canonical candidate record created from human-editable input or projection drift before a reconcile decision accepts, rejects, converts, or defers it.

### Reference Surface

The single agent surface targeted by the MVP implementation. It demonstrates the kernel and connector contract without implying broad MVP surface support.

### Report Projection

A Markdown report generated from state records and artifact references, such as a Task report, approval report, run summary, evidence manifest report, Eval report, or direct-result report.

The named report projection kinds are projections or records by default; evidence-file authority stays with registered artifact files.

### Residual Risk

Known remaining uncertainty, trade-off, limitation, or unchecked condition after evidence, verification, QA, and acceptance work. Residual risk must remain visible when it affects close, and user acceptance of risk does not create detached verification.

### Risk Accepted Close

A successful close where the user accepts remaining verification risk. It uses `close_reason=completed_with_risk_accepted` and must not display `assurance_level=detached_verified`.

### Run

An execution attempt by an agent, evaluator, operator, or other actor against a Task and optionally a Change Unit. Runs record baseline, surface, observed changes, commands, artifacts, and summary.

### Scope Gate

The kernel gate requiring product writes to be covered by an active scoped Change Unit. Scope is required for write-capable direct and work modes even when approval is not required.

### Shared Design

A canonical design-support record of the shared understanding for a task: goal, scope, non-goals, acceptance criteria, assumptions, decisions, rejected options, domain impact, module/interface impact, and first Change Unit shape. Markdown renderings of Shared Design are projections and proposal surfaces.

### Source-of-truth

The authoritative source for a fact. In the harness, operational state is canonical in `state.sqlite` current records plus `state.sqlite.task_events`; raw evidence is canonical in the artifact store; Markdown documents are projections.

### `state.sqlite.task_events`

The append-only event history table inside `state.sqlite`. MVP does not use a separate event store.

### State Record

A canonical structured record in kernel state, such as a Task, Change Unit, Run, Approval, Decision Packet, Shared Design or other design-support record, Evidence Manifest, Eval, Manual QA record, Artifact record, residual-risk record, or Reconcile Item.

### Strategic Agency

The user's durable authority to understand the work journey and make or withhold judgment over goals, scope, design, trade-offs, codebase stewardship, QA, acceptance, and residual risk. The harness preserves Strategic Agency by making state, decisions, evidence, blockers, and remaining risk explicit outside chat.

### Surface Capability Check

A validator that reports whether a connected agent surface can satisfy required harness behavior. It affects blocked reasons and guarantee display, but it is not a kernel gate.

### Surface Cookbook

The appendix that contains surface-specific connector notes, generated file details, and profile examples. Common integration rules belong in the agent integration document, not the cookbook.

### Subagent Context

A verification independence profile where a subagent or helper reviews work with some inherited implementation context. It is not detached by default and can qualify only when stricter profile metadata proves a real independence boundary.

### Task

The user value unit tracked by the kernel. It carries mode, lifecycle phase, gates, result, close reason, assurance, current summary, decisions, evidence, and projection status.

### TDD Trace

A record of red, green, and refactor evidence for a Change Unit, or a recorded non-TDD justification where policy allows it.

### Verification

The process of checking whether the result satisfies the relevant criteria. Verification is separate from approval, Manual QA, and acceptance.

### Verification Gate

The kernel gate for required verification. A user waiver sets `verification_gate=waived_by_user`; it does not create `detached_verified` assurance.

### Verification Independence Profile

A named minimum qualification for an Eval independence context, such as `same_session`, `subagent_context`, `fresh_session`, `fresh_worktree`, `sandbox`, or `manual_bundle`. A passed Eval must satisfy a valid profile before it can support `detached_verified`.

### Validator Result

A structured result from a validator, including status, guarantee level, target, findings, blocked reasons, and suggested next action.

### Vertical Slice

A Change Unit shape that connects a thin path from trigger/input through domain logic, persistence or state, caller/API boundary, observable output, tests, and optional Manual QA.

### Waiver

An explicit recorded exception to a gate requirement where policy allows it. Verification waiver, design waiver, and QA waiver are allowed under defined rules. Scope, sensitive approval, required evidence, and required acceptance are not waived for successful completion.
