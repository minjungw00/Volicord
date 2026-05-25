# Build: MVP Plan

## What this document helps you do

This document turns the MVP scope material into an implementable build sequence. It keeps delivery stages separate from storage schemas, DDL, projection template bodies, and operator command syntax.

This is planning documentation; it does not authorize runtime or server implementation before the redesigned docs are accepted.

Use this when you need to plan what to build after the first runnable slice. Use the reference docs for exact contracts.

## Read this when

- You are planning delivery after Kernel Smoke.
- You need to review MVP scope stage by stage.
- You want to separate the implementation sequence from storage, schema, and template details.

## Before you read

Read [Implementation Overview](implementation-overview.md) and [First Runnable Slice](first-runnable-slice.md). For exact API contracts, use [MCP API And Schemas](../reference/mcp-api-and-schemas.md). For storage details and DDL, use [Storage And DDL](../reference/storage-and-ddl.md). For design-quality gate and validator behavior, use [Design Quality Policies](../reference/design-quality-policies.md).

## Main idea

MVP delivery moves from a narrow Kernel Smoke path to an Agency-Hardened MVP, while later automation stays outside the boundary unless it is separately specified and proven.

The center of the plan is Core state, `task_events`, artifact refs, evidence, blockers, and the minimal reference surface and MCP reachability needed to exercise them. Projection-template polish, dashboards, indexes, hook expansion, broad connector ecosystems or marketplaces, surface-specific connector automation, and broad automation become useful after that path exists; they are not the first build target.

## MVP scope in plain language

The MVP is a local kernel-authority and agency-conformance project. It is not a broad agent platform.

The MVP should let one local project and one reference agent surface operate through Harness with:

- canonical Task state and `task_events`
- scoped Change Units for product writes
- `prepare_write` and durable Write Authorizations
- approvals for sensitive categories
- Decision Packets for user-owned product judgment or material technical judgment
- Runs, artifact refs, and Evidence Manifests
- verification, Manual QA, residual-risk visibility, acceptance, and close blockers
- MCP resources and tools over Core
- projection jobs and MVP-required projection renderers
- reconcile for human-editable input or managed-block drift
- doctor, recover, export, artifact integrity, and conformance smoke entrypoints

Keep this scope local, inspectable, and fixture-proven.

In practical terms, build the state and authority path before presentation polish. A projection or UI may make status easier to read, but it must be downstream from Core records.

## Kernel Smoke

Kernel Smoke is the first runnable conformance target. It crosses MVP-0 through early MVP-3, but only for the selected authority path.

It must prove:

- project and Task state
- one scoped Change Unit
- `prepare_write` allow and block behavior
- durable Write Authorization creation
- `record_run` consumption of that authorization
- artifact registration
- Evidence Manifest basics
- minimal required projection freshness or enqueueing
- blocked writes or Runs when write authority is missing
- blocked close when evidence or decision requirements are missing
- basic Core fixture execution

Kernel Smoke is useful because it proves the Harness write authority loop before the rest of the system exists. It is not final MVP conformance.

At this point, the user or operator can observe a small but complete loop: current Task status, scoped write block/allow, durable Write Authorization creation and consumption, artifact and Evidence Manifest links, projection freshness or enqueueing, and structured close blockers.

## Agency-Hardened MVP

Agency-Hardened MVP is the final reference conformance target. It completes the rest of MVP-3 and then adds MVP-4 and MVP-5.

It must prove:

- Decision Packet quality and user-judgment routing
- approval, Decision Packet, and Write Authorization separation
- residual-risk visibility before acceptance and close
- detached verification independence
- Manual QA records and blockers
- stewardship and context-hygiene validators
- feedback-loop and TDD checks
- codebase stewardship coverage
- projection and reconcile completeness
- recover, export, and artifact integrity behavior
- later-boundary checks
- required agency conformance fixtures

Passing Agency-Hardened MVP means the local reference MVP is coherent enough for implementation use. It does not promote later automation into MVP.

At this point, the user or operator can observe not just that writes are controlled, but why work can proceed, pause, verify, require Manual QA, expose residual risk, accept, recover, export, or close.

## MVP-0 Through MVP-5

### MVP-0: Runtime Bootstrap

Build the local runtime home and register one project.

Focus on:

- project registration
- project state initialization
- static project configuration
- artifact store initialization
- reference surface registration with honest cooperative or detective capability
- doctor/readiness reporting

Do not add multi-project orchestration or connector ecosystems here.

### MVP-1: Core State, Journey/Decision Skeleton, MCP Facade

Build the Core state transition foundation and the first MCP-facing reads and tools.

Focus on:

- transaction wrapper, locks, state version checks, and idempotency replay
- current records plus task event append behavior
- active Task absent status
- advisor Task intake, read-only progress, and close
- Journey Spine reconstruction and Journey Card inputs
- Decision Packet records and `decision_gate` aggregation
- `harness.status`, `harness.intake`, and `harness.next`
- read-only recommended playbooks and Role Lens recommendations that do not create authority

Do not let display guidance satisfy gates, authorize writes, create evidence, waive QA or verification, accept risk, accept results, close Tasks, or upgrade assurance.

### MVP-2: Shaping Kernel, Write Gate, Approval, Baseline, Artifacts

Build the first write-capable authority path.

Focus on:

- Change Unit records and active scope
- autonomy boundary fields
- baseline capture and freshness checks
- `harness.prepare_write`
- durable Write Authorization records
- approval request and decision flow for sensitive categories
- minimal changed-path, scope, approval, baseline, decision, autonomy, and capability checks
- raw artifact registration with integrity and redaction metadata

Do not treat approval as user-owned judgment. User-owned product trade-offs, architecture choices, material technical choices, unresolved security or product-security judgment, QA waivers, verification risk, acceptance, and residual-risk acceptance still require compatible Decision Packets when they apply.

### MVP-3: Runs, Evidence, Feedback Loop, Projection, Reconcile

Build the post-write recording and readable-output path.

Focus on:

- `harness.record_run`
- Run records and Write Authorization consumption
- Evidence Manifest records and evidence gate updates
- Feedback Loop and TDD support records where policy requires them
- codebase stewardship checks
- projection jobs and MVP-required renderers whose source records exist before verification
- managed block hashes
- reconcile item creation for managed drift and human-editable proposals

Build MVP-required renderers from records that already exist. Do not make projection templates, template polish, or additional renderer-first work the driver for Task, Run, evidence, or verification design.

At this stage, `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, and `DIRECT-RESULT` can be built when their source records exist. `EVAL` remains MVP-required, but its executable render path completes with MVP-4 when Eval source records exist.

Projection failure remains separate from Core state failure.

### MVP-4: Verification, Manual QA, Residual Risk, Close

Build the close-readiness and assurance path.

Focus on:

- `harness.launch_verify`
- `harness.record_eval`
- `harness.record_manual_qa`
- `harness.close_task`
- verification independence checks
- same-session verification guard
- Manual QA aggregation and QA blockers
- residual-risk visibility before acceptance and close
- acceptance and risk-accepted close rules
- Decision Packet close checks
- close blocker reporting

Do not require automated Browser QA Capture for MVP. Browser screenshots, console logs, network traces, accessibility snapshots, or workflow recordings may be attached when supplied, but Manual QA records and artifact refs are the MVP requirement.

### MVP-5: Operator Smoke, Agency Conformance, Later-Boundary Checks

Build the operator and conformance proof layer.

Focus on:

- doctor
- recover
- reconcile
- export
- artifact integrity check
- fixture-based conformance smoke
- agency conformance for Journey visibility, user judgment, Autonomy Boundary respect, and residual-risk visibility
- later-boundary checks that keep parallel orchestration, broad connector automation, and preventive guard expansion out of MVP unless separately proven

Do not create a second state model for operator commands. Operators diagnose, repair, export, or run fixtures over the same Core state model.

## Exit criteria by stage

| Stage | Exit criteria |
|---|---|
| MVP-0 | One project is registered, project state exists before mutations use expected state versions, the reference surface is registered, runtime files and artifact storage exist, and doctor/readiness can report project/runtime status. |
| MVP-1 | No-active-Task status works, an advisor Task can intake and close through Core, Task status exposes Journey/Decision state, read guidance is non-authoritative, blocking user judgment can create or associate a Decision Packet, and every state mutation updates current records plus `task_events` in one transaction. |
| MVP-2 | Product writes without active scoped Change Unit block, sensitive changes require approval, Autonomy Boundary violations block or route to Decision Packets, unresolved blocking Decision Packets block affected writes, allowed `prepare_write` creates durable Write Authorization refs, idempotent replay works, approval drift can block or expire approval, shaping records the needed boundaries, and raw artifacts store integrity/redaction metadata. |
| MVP-3 | Direct and implementation Runs register artifacts and update evidence, Runs consume compatible Write Authorizations, observed out-of-scope changes are detected, findings route back into state/evidence/Decision Packets/Change Units/blockers, stewardship issues are visible, MVP-required pre-verification projections can enqueue or render, projection failure is state-isolated, and managed Markdown edits create reconcile items. |
| MVP-4 | Work cannot close as detached verified from same-session self-review, verification waivers close only with accepted risk, Manual QA and acceptance block independently when required, close-relevant residual risk is visible before successful close, risk-accepted close requires accepted Residual Risk refs, acceptance follows risk visibility, blocking Decision Packets block close, and direct work can close self-checked unless policy or the user requires detached verification. |
| MVP-5 | Conformance smoke covers core, connector, agency, stewardship, context-hygiene, and design-quality paths; agency checks prove Journey visibility, unresolved decisions, agent latitude, and residual-risk visibility; dependency DAG support remains metadata-only; export includes state snapshots, report projections, artifact refs, and redaction status. |

## Observable by stage

| Stage | What the user or operator can observe |
|---|---|
| MVP-0 | Doctor/readiness can show the runtime home, registered project, artifact store, reference surface, and idle state. |
| MVP-1 | Status, intake, and next-action reads can show the active Task, Journey/Decision state, and non-authoritative guidance without mutating state. |
| MVP-2 | `prepare_write` can explain missing scope, out-of-scope paths, sensitive approval needs, stale baseline, unresolved decisions, and compatible Write Authorization creation. |
| MVP-3 | `record_run` consumes authority, Runs cite artifacts, evidence updates, projection freshness is visible, and reconcile items appear for managed drift. |
| MVP-4 | Verification, Manual QA, residual-risk visibility, acceptance, and `close_task` blockers explain whether the Task can close. |
| MVP-5 | Doctor, recover, reconcile, export, artifact integrity, and conformance fixtures prove the same Core state and keep later automation outside the MVP boundary. |

## Later boundary

Keep these outside MVP unless a future plan promotes them with exact contracts and fixtures:

- dashboard or hosted workflow UI
- broad connector marketplace or surface ecosystem
- Browser QA Capture as required automation
- preventive `T4` guard expansion without a proven pre-tool blocking path
- Context Index and derived analytics
- deployment, canary, rollback, or production monitoring automation
- parallel orchestration and concurrent lane scheduling
- team workflow, permissions, and team profile export/import
- long-term operational metrics as MVP-critical state

If a later feature is useful during implementation, keep it as display, metadata, optional attachment, or fixture candidate until the owner docs define its authority path.
