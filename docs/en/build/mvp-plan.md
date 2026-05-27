# Build: MVP Plan

## What this document helps you do

This document turns the MVP scope material into an implementable build sequence. It keeps delivery stages separate from storage schemas, DDL, projection template bodies, and operator command syntax.

This is planning documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before the documentation set is accepted for implementation planning. The first implementation/proof target is Kernel Smoke: one local process with modules proving one authority loop. Agency-Hardened MVP is a later hardening and conformance target after Kernel Smoke, and roadmap automation stays outside MVP unless owner docs promote and prove it.

Use this when you need to plan what to build after the first runnable slice. Use the reference docs for exact contracts.

## Read this when

- You are planning delivery after Kernel Smoke.
- You need to review MVP scope stage by stage.
- You want to separate the implementation sequence from storage, schema, and template details.

## Before you read

Read [Implementation Overview](implementation-overview.md), including its [Documentation Acceptance Status](implementation-overview.md#documentation-acceptance-status), and [First Runnable Slice](first-runnable-slice.md). For exact API contracts, use [MCP API And Schemas](../reference/mcp-api-and-schemas.md). For storage details and DDL, use [Storage And DDL](../reference/storage-and-ddl.md). For design-quality gate and validator behavior, use [Design Quality Policies](../reference/design-quality-policies.md). For post-MVP candidates and promotion criteria, use the [Roadmap](../roadmap.md).

## Main idea

MVP delivery starts with the narrow Kernel Smoke path and only then hardens toward Agency-Hardened MVP. Later automation stays outside the boundary unless a future owner promotes it through the [Roadmap promotion rule](../roadmap.md#promotion-rule) and proves it separately.

The center of the plan is Core state, `task_events`, artifact refs, evidence, blockers, and the minimal reference surface and MCP reachability needed to exercise them. The initial implementation assumption remains one local process with modules. Projection-template polish, dashboards or hosted workflow UI, indexes, hook expansion, broad connector ecosystems or marketplaces, team workflow, surface-specific connector automation, metrics, parallel orchestration, and broad automation become useful after that path exists; they are not the first build target.

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
- doctor/readiness, recover, export, artifact integrity, and conformance smoke entrypoints that report through Core state, `task_events`, artifacts, projections, and existing errors or diagnostics

Keep this scope local, inspectable, and fixture-proven.

In practical terms, build the state and authority path before presentation polish. A projection or UI may make status easier to read, but it must be downstream from Core records.

## Kernel Smoke

Kernel Smoke is the first runnable conformance target and the first implementation/proof target. It crosses MVP-0 through early MVP-3, but only for the selected authority path.

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

For practical fixture authoring order, use the [Kernel Smoke Authoring Queue](../reference/operations-and-conformance.md#kernel-smoke-authoring-queue). It maps the first runtime fixture candidates to this stage without changing the exact fixture body shape.

Kernel Smoke pass/fail comes from runtime fixtures that drive Core or operator actions and compare captured state, `task_events`, artifacts, projections, and primary errors. Status prose, Journey Card text, close prose, and scenario descriptions are observable context only; exact fixture body and assertion rules stay in [Operations And Conformance](../reference/operations-and-conformance.md#conformance-fixture-format).

## Agency-Hardened MVP

Agency-Hardened MVP is the later hardening and final reference conformance target, not the first implementation batch. It completes the rest of MVP-3 and then adds MVP-4 and MVP-5.

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
- recover, export, and artifact integrity behavior, including the rule that recovery artifacts do not prove successful completion
- later-boundary checks
- required agency conformance fixtures

Passing Agency-Hardened MVP means the local reference MVP is coherent enough for implementation use. It does not promote later automation into MVP.

At this point, the user or operator can observe not just that writes are controlled, but why work can proceed, pause, verify, require Manual QA, expose residual risk, accept, recover, export, or close.

## MVP-0 Through MVP-5

The stage descriptions below use implementation verbs for future planning after documentation acceptance. They are not permission to begin runtime/server implementation, generated operational files, executable fixtures, or runtime data during the current documentation-acceptance phase.

### MVP-0: Runtime Bootstrap

Build the local runtime home and register one project.

Focus on:

- project registration
- project state initialization
- static project configuration
- artifact store initialization
- reference surface registration with honest cooperative or detective capability
- doctor/readiness reporting

Do not add multi-project orchestration, team workflow, hosted workflow UI, metrics, or connector ecosystems here.

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

Display guidance remains read-only routing and status context. Exact Role Lens/playbook boundaries live in [Agent Integration](../reference/agent-integration.md#role-lens-behavior), and projection/report boundaries live in [Document Projection Reference](../reference/document-projection.md#projection-principles).

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
- evaluator bundle freshness
- Manual QA aggregation and QA blockers
- residual-risk visibility before acceptance and close
- acceptance and risk-accepted close rules
- distinct Approval, Manual QA, verification-waiver, acceptance, and residual-risk-acceptance judgments
- Decision Packet close checks
- close blocker reporting

Do not require automated Browser QA Capture or hosted workflow automation for MVP. Browser screenshots, console logs, network traces, accessibility snapshots, or workflow recordings may support QA evidence only when registered and linked through existing Manual QA/artifact paths, but Manual QA records and artifact refs are the MVP requirement. Captured material does not replace Manual QA judgment, final acceptance, or detached verification unless a separate Eval path satisfies independence. Unsupported surfaces fall back to human Manual QA notes and manually supplied artifacts.

### MVP-5: Operator Smoke, Agency Conformance, Later-Boundary Checks

Build the operator and conformance proof layer.

Focus on:

- doctor/readiness categories for runtime home, project state, artifact store, reference surface, MCP availability, projections, reconcile, validators/checks, and agency/stewardship/context
- recover handling for baseline drift, approval drift, evaluator repo drift, artifact missing or hash mismatch, projection failure, managed Markdown direct edits, MCP unavailable, and surface capability mismatch
- reconcile
- export of state snapshots, report projection snapshots, artifact refs, redaction status, omitted-secret notes, and retained, expired, or unavailable artifact status
- artifact integrity check
- fixture-based conformance smoke over state, events, artifacts, projections, and errors, with suite catalog metadata kept outside the fixture body
- coverage-map conformance for core, connector, connector guard/freeze, agency, stewardship, context-hygiene, and design-quality paths
- agency conformance for Journey visibility, user judgment, Autonomy Boundary respect, distinct user judgments, and residual-risk visibility
- connector and context conformance for MCP unavailable holds, surface capability mismatch, generated-file drift, stale projection write guards, stale PRD/chat-memory pull-only behavior, evaluator bundle freshness, and artifact integrity effects
- later-boundary checks that keep Dashboard, hosted workflow UI, Browser QA Capture, Cross-Surface Verification, Context Index, parallel orchestration, team workflow, broad connector automation, native hook or sidecar expansion, derived metrics, and preventive guard expansion out of MVP unless separately proven and promoted

Do not create a second state model for operator commands. Operators diagnose, repair, export, or run fixtures over the same Core state model. Exact command names and flags can vary; the contract is the command-independent behavior over Core state, `task_events`, artifacts, projections, and existing errors or diagnostics.

Docs-maintenance remains a separate read-only documentation profile. It may report documentation drift, but it is not Kernel Smoke, not Agency-Hardened runtime conformance, and not an implementation-readiness signal.

## Exit criteria by stage

Use these as implementation-readable checklists for future runtime planning after documentation acceptance. They restate the stage exit criteria; they do not add schemas, fixtures, DDL, or new runtime requirements, and they do not authorize implementation while the [Documentation Acceptance Status](implementation-overview.md#documentation-acceptance-status) still blocks first runtime-batch planning.

Read the stage exits in two layers:

| Stage | Kernel Smoke reading | Agency-Hardened MVP reading |
|---|---|---|
| MVP-0 | Required foundation for the first local project, runtime home, artifact store, reference surface, and idle readiness. | The same foundation remains in force and later supports broader doctor/readiness categories. |
| MVP-1 | Required only for the Task state, state-version, `task_events`, minimal status/intake, and decision-blocker visibility needed by the first authority loop. | Completes the Journey/Decision skeleton and read-only guidance boundaries needed by the final local MVP. |
| MVP-2 | Required for one active Change Unit, `prepare_write` allow/block, durable Write Authorization creation, and artifact registration basics. | Adds the broader approval, baseline, autonomy, sensitive-category, and drift handling needed for hardened conformance. |
| MVP-3 | Required for one compatible `record_run`, Write Authorization consumption, artifact-backed Evidence Manifest basics, and minimal `TASK` projection freshness or enqueueing. | Completes feedback-loop, TDD, stewardship, projection, and reconcile coverage for the local reference MVP. |
| MVP-4 | Not required to pass Kernel Smoke. Missing MVP-4 behavior is simply not yet proven by the first slice. | Required for verification, Manual QA, residual-risk visibility, acceptance, and close-readiness hardening. |
| MVP-5 | Not required to pass Kernel Smoke. Missing MVP-5 behavior is simply not yet proven by the first slice. | Required for operator smoke, agency conformance, recover/export/artifact-integrity proof, and later-boundary checks. |

Kernel Smoke may pass with only the selected MVP-0 through early MVP-3 subset above. Agency-Hardened MVP requires the remaining stage criteria and fixture coverage owned by [Operations And Conformance](../reference/operations-and-conformance.md#hardened-mvp-fixture-coverage).

### MVP-0 exit checklist

- One project is registered.
- Project state exists before mutations use expected state versions.
- The reference surface is registered.
- Runtime files and artifact storage exist.
- Doctor/readiness can report runtime home, project state, artifact store, reference surface, and MCP availability status without creating state.

### MVP-1 exit checklist

- No-active-Task status works.
- An advisor Task can intake and close through Core.
- Task status exposes Journey/Decision state.
- Read guidance is non-authoritative.
- Blocking user judgment can create or associate a Decision Packet.
- Every state mutation updates current records plus `task_events` in one transaction.

### MVP-2 exit checklist

- Product writes without an active scoped Change Unit block.
- Sensitive changes require approval.
- Autonomy Boundary violations block or route to Decision Packets.
- Unresolved blocking Decision Packets block affected writes.
- Allowed `prepare_write` creates durable Write Authorization refs.
- Idempotent replay works.
- Approval drift can block or expire approval.
- Shaping records the needed boundaries.
- Raw artifacts store integrity/redaction metadata.

### MVP-3 exit checklist

- Direct and implementation Runs register artifacts and update evidence.
- Runs consume compatible Write Authorizations.
- Observed out-of-scope changes are detected.
- Findings route back into state, evidence, Decision Packets, Change Units, or blockers.
- Stewardship issues are visible.
- MVP-required pre-verification projections can enqueue or render.
- Projection failure is state-isolated.
- Managed Markdown edits create reconcile items.

### MVP-4 exit checklist

- Work cannot close as detached verified from same-session self-review.
- Stale evaluator bundles cannot record detached verification as passed.
- Verification waivers close only with accepted risk.
- Manual QA and acceptance block independently when required.
- Close-relevant residual risk is visible before successful close.
- Risk-accepted close requires accepted Residual Risk refs.
- Acceptance follows risk visibility.
- Approval, Manual QA, verification waiver, acceptance, and residual-risk acceptance stay separate.
- Blocking Decision Packets block close.
- Direct work can close self-checked unless policy or the user requires detached verification.

### MVP-5 exit checklist

- Conformance smoke covers core, connector, connector guard/freeze, agency, stewardship, context-hygiene, and design-quality paths.
- Catalog scenario coverage includes artifact integrity, MCP unavailable, surface capability mismatch, generated-file drift, stale projection write guards, stale PRD/chat-memory context, evaluator bundle freshness, residual-risk visibility, and distinct user judgments.
- Suite catalog metadata groups exact-shape fixtures by suite, stage, and tags without being passed to Core.
- Agency checks prove Journey visibility, unresolved decisions, agent latitude, distinct user judgments, and residual-risk visibility.
- Dependency DAG support remains metadata-only.
- Export includes state snapshots, report projection snapshots, artifact refs, redaction status, omitted-secret notes, and retained, expired, or unavailable artifact status.
- Browser QA Capture entries remain future candidates unless promoted through owner docs.

## Observable by stage

| Stage | What the user or operator can observe |
|---|---|
| MVP-0 | Doctor/readiness can show runtime home, project state, artifact store, reference surface, MCP availability, and idle state. |
| MVP-1 | Status, intake, and next-action reads can show the active Task, Journey/Decision state, and non-authoritative guidance without mutating state. |
| MVP-2 | `prepare_write` can explain missing scope, out-of-scope paths, sensitive approval needs, stale baseline, unresolved decisions, and compatible Write Authorization creation. |
| MVP-3 | `record_run` consumes authority, Runs cite artifacts, evidence updates, projection freshness is visible, and reconcile items appear for managed drift. |
| MVP-4 | Verification, Manual QA, residual-risk visibility, acceptance, and `close_task` blockers explain whether the Task can close. |
| MVP-5 | Doctor, recover, reconcile, export, artifact integrity, and conformance fixtures prove the same Core state and keep later automation outside the MVP boundary. |

## Later boundary

Keep these outside MVP unless a future plan promotes them through owner docs with a capability profile, exact contracts, redaction/secret/PII policy, artifact retention and test-environment rules when runtime surfaces are captured, fixtures or a conformance target, fallback behavior, and no projection-as-canonical dependency:

- dashboard, hosted workflow UI, or local metrics as authority, implementation-readiness, or close-readiness surfaces
- broad connector marketplace or surface ecosystem beyond the one reference surface
- Browser QA Capture as required automation or acceptance replacement
- Cross-Surface Verification as a required assurance path
- preventive `T4` guard expansion without a proven pre-tool blocking path
- native hook expansion or Advanced Sidecar Watcher beyond a concrete reference-surface capability
- Context Index as authority or read/write prerequisite
- deployment, canary, rollback, or production monitoring automation
- parallel orchestration and concurrent lane scheduling
- team workflow, permissions, and team profile export/import
- Local Derived Metrics or long-term operational metrics as MVP-critical state

If a later feature is useful during implementation, keep it as read-only display, metadata, artifact candidates for existing owner paths, or fixture candidate until owner docs define and prove its authority path. It must not become a prerequisite for Kernel Smoke or Agency-Hardened MVP.
