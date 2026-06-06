# Agent Integration Reference

## What this document helps you do

Use this reference when connecting the active reference agent surface to future Harness behavior without overstating what that surface can enforce.

This reference owns the common surface contract: capability tiers, the active reference `capability_profile`, generated manifest expectations, context push/pull principles, fallback semantics, Role Lens behavior, reference-surface expectations, and later connector conformance overview.

For what the agent says and does in a user session, read [Agent Guide](../use/agent-guide.md). For surface-specific setup notes, read [Surface Cookbook](surface-cookbook.md). Current repository phase and implementation handoff status are tracked in [MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

## Read this when

- You are implementing or reviewing the one active reference surface profile.
- You need to declare, refresh, or audit a surface `capability_profile`.
- You need to display guarantee level, guard, freeze, fallback, or MCP availability honestly.
- You are writing future reference-surface smoke or later connector conformance coverage.
- You need to know what belongs in this common contract instead of a surface recipe or later connector note.

## Before you read

Read [Agent Guide](../use/agent-guide.md) for behavior rules. Pull exact owner sections only when the connector action needs them: [Core Model Reference](core-model.md) for write and close authority, [MVP API](api/mvp-api.md) for active MVP-1 method contracts, [API Schema Core](api/schema-core.md) for shared shapes, [API Errors](api/errors.md) for public error behavior, and [Security Reference](security.md) for threat and guarantee wording.

This reference is not an instruction to load all Reference docs into agent context.

## Main idea

The active MVP path targets one reference surface profile. That profile gives the agent small current context, routes state-changing actions through Harness, records what happened when the surface can do so, and reports only guarantees proven for the actual `capability_profile` in use.

Surface name is not capability. A profile may claim cooperative, detective, preventive, or isolated behavior only for the concrete host, profile, version/configuration, workspace policy, MCP posture, capture path, guard path, or separation boundary that has been declared and checked. Broad connector ecosystems, hosted connector registries, cross-surface orchestration, remote/shared MCP exposure, and multi-surface automation are later/profile or Roadmap scope unless an owner explicitly promotes a narrower path.

## Integration In Plain Language

An agent surface is where the user talks to an agent. Harness is the local authority layer for Harness records and state transitions. It keeps scope, user judgment, write checks, evidence refs, final acceptance, residual-risk acceptance, and close readiness outside the chat transcript without claiming OS-level permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, or security isolation.

The common path is:

```text
user conversation surface
  -> short always-on rules/context
  -> Harness skill, command, or playbook
  -> Harness MCP server
  -> Harness Core
  -> reference surface adapter and validators
  -> later/profile hook, sidecar, capture path, or isolation layer only when promoted
```

Always-on connector rules should stay short: when to use Harness, where to read current status or an agent context packet, that product writes use `prepare_write`, that user-owned judgment routes through judgment requests, that clarification checks repo/docs/current state before asking, that status must show what can actually be blocked or only detected later, and that product writes hold when authoritative MCP is unavailable.

The session procedure belongs in [Agent Guide](../use/agent-guide.md). Connector setup and surface-specific paths belong in [Surface Cookbook](surface-cookbook.md).

## Use Docs Vs Reference Docs

| Area | Owner |
|---|---|
| What the agent shows, asks, and says during a user session | [Agent Guide](../use/agent-guide.md) |
| User-facing explanation of scope, evidence, verification, QA, residual risk, and close | [User Guide](../use/user-guide.md) |
| Common connector capability, context, fallback, and conformance contract | This reference |
| Concrete surface recipes | [Surface Cookbook](surface-cookbook.md) |
| Public MCP request/response schemas | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [API Schema Later](../later/index.md#later-schema-candidates) |
| Core state transitions and write/close rules | [Core Model Reference](core-model.md) |
| Security guarantee meanings | [Security Reference](security.md#honest-guarantee-display) |

## Capability Tiers

| Tier | Meaning | Typical capability |
|---|---|---|
| `T0 Context` | Surface can read Harness principles. | rules/context file |
| `T1 Skill` | Surface can follow a Harness procedure. | skill, command, prompt, playbook |
| `T2 MCP` | Surface can call Harness tools and resources. | MCP server connection |
| `T3 Capture` | Surface can return diffs, logs, and run output reliably. | structured output, wrapper, adapter |
| `T4 Guard` | Surface can block or interrupt covered paths before execution when fixture coverage proves that concrete path. | hook, permission system, policy engine, sidecar |
| `T5 Isolation` | Surface can run verification or risky work behind a documented separation boundary. | worktree, sandbox, fresh process, isolated runner |
| `T6 QA Capture` | Surface can structure browser, screenshot, walkthrough, workflow-recording, or Manual QA artifacts. | browser runner, screenshot capture, console/network capture, accessibility snapshot, QA note capture |

Engineering Checkpoint and MVP-1 target the one reference surface profile. That profile should assume cooperative or limited detective behavior unless a concrete owner-promoted profile proves a stronger capability. `T4` and `T5` do not imply default OS isolation, arbitrary-tool sandboxing, tamper-proof files, or pre-tool blocking.

## Capability Profiles

Surfaces must use a `capability_profile` instead of assuming behavior from a product name, surface name, or mode label. A profile is scoped to the actual host/profile that will run the work.

A `capability_profile` is not write authority, not a first-class replacement for Core gates, and not a way to bypass active Task, active Change Unit, `prepare_write`, the single-use cooperative Write Authorization record, or `record_run`. Capability affects validator results, Harness `allowed`/`blocked` compatibility outcomes, fallback behavior, and guarantee display. Here `allowed` means compatible with current Harness state and active surface capability; `blocked` means not allowed by the Harness protocol, state, or capability. Neither word means OS-level permission or physical prevention unless a proven preventive profile names the covered operation. Product writes must not proceed silently on an unsupported surface.

The active MVP reference profile uses these fields:

```yaml
capability_profile:
  surface_id: reference-local-mcp
  surface_name: Reference local MCP surface
  mcp_available: true
  public_tools_available:
    - harness.status
    - harness.intake
    - harness.request_user_judgment
    - harness.record_user_judgment
    - harness.prepare_write
    - harness.record_run
    - harness.close_task
  resources_available:
    - harness://project/current
    - harness://task/active
    - harness://task/{task_id}
    - harness://task/{task_id}/summary
    - harness://status/card
    - harness://task/{task_id}/user-judgments
    - harness://task/{task_id}/judgment-context
  cooperative_prepare_write_supported: true
  changed_path_detection_supported: true
  artifact_capture_supported: false
  manual_artifact_attachment_supported: true
  command_observation_supported: false
  network_observation_supported: false
  secret_access_observation_supported: false
  pre_tool_blocking_supported: false
  isolation_supported: false
  max_guarantee_level: detective
  unsupported_behavior:
    - lower guarantee display when a claim depends on an unsupported field
    - return CAPABILITY_INSUFFICIENT or a structured blocked reason when the missing capability is required
    - hold product writes by instruction instead of silently continuing
  fallback_instruction: "Use cooperative prepare_write, validate changed paths after action, and attach manual artifacts before making evidence or guarantee claims."
  conformance_smoke_status: planned_not_run
```

`max_guarantee_level` is a ceiling, not the default for every action. The reference profile can display `detective` only for behavior supported by `changed_path_detection_supported` or other observable after-action facts. Because `pre_tool_blocking_supported=false` and `isolation_supported=false`, it must not claim `preventive` or `isolated` behavior.

Refresh the profile when version, MCP config, hooks, permissions, workspace policy, generated files, managed blocks, conformance result, capture method, QA capture method, redaction policy, artifact retention, access-control material class, local bind/reachability posture, guard wrapper, or isolation wrapper changes.

Beyond-local MCP exposure remains outside the Engineering Checkpoint baseline and staged delivery unless owner docs promote and prove it. A connector must not present remote or shared MCP exposure as the safe default without that basis.

## Guarantee Levels

Guarantee level display follows [Security Reference](security.md#honest-guarantee-display). This reference owns how connector profiles report that level.

| Level | Display responsibility |
|---|---|
| `cooperative` | Say the surface is expected to follow Harness instructions. Holds are by instruction, not physical blocking. |
| `detective` | Say Harness can observe changed paths, logs, artifacts, or drift after action and mark state stale, partial, blocked, or failed. |
| `preventive` | Name the fixture-proven hook, wrapper, permission layer, policy engine, or sidecar path and the covered operations it can block before execution. |
| `isolated` | Name the documented separation boundary. Do not imply OS sandboxing, permission isolation, or tamper-proof storage unless the profile proves that exact mechanism. |

Guard, freeze, and careful-mode labels are display labels over the actual profile. They must say what can actually be blocked before execution and what can only be detected later. They are not approval, verification, final acceptance, residual-risk acceptance, close readiness, or a kernel gate.

Field mapping for the active reference surface:

| Capability field | Guarantee effect |
|---|---|
| `mcp_available=false` | Core authority is unavailable from that surface; use `MCP_UNAVAILABLE` behavior and do not claim state mutation. |
| `cooperative_prepare_write_supported=true` | The surface can participate in the cooperative `prepare_write` path, but Core still decides and any Write Authorization still comes only from Core. |
| `changed_path_detection_supported=true` | The surface may support detective changed-path validation after action; it does not prove pre-tool blocking. |
| `artifact_capture_supported=false` with `manual_artifact_attachment_supported=true` | Native artifact capture claims must be blocked or lowered; manual artifact attachment can support evidence refs only after the owner path registers them. |
| `command_observation_supported=false`, `network_observation_supported=false`, or `secret_access_observation_supported=false` | Claims that depend on command, network, or secret-access observation must be blocked, narrowed, or marked unverified. |
| `pre_tool_blocking_supported=false` | `preventive` display is unavailable; product writes hold by instruction when unsupported. |
| `isolation_supported=false` | `isolated` display is unavailable; worktrees or bundles cannot be called security isolation without a promoted proof. |

## Generated Manifest Expectations

Connectors may generate rules, skills, MCP config snippets, prompts, or adapter files. Every generated or managed path, managed block, MCP snippet, and profile freshness marker must be recorded in a connector manifest.

The manifest must:

- name generated and managed paths
- record managed block ids and hashes
- record the capability profile used when generated
- record MCP exposure posture and display-safe handles without raw tokens, secrets, private config, omitted secret values, or blocked payload bytes
- record capture, QA capture, guard, isolation, and fallback behavior without claiming more than the profile proves
- mark the profile or block stale when the relevant surface, configuration, policy, generated file, conformance, capture, redaction, artifact-retention, guard, or isolation basis changes
- detect drift before overwriting human edits
- route drift to reconcile when needed
- report that edited generated files are not canonical Task state

Surface-specific generated filenames belong in [Surface Cookbook](surface-cookbook.md).

## Context Push/Pull Principles

Connectors should push compact current context and pull larger references only when the next action needs them. The context packet is operational support, not agent memory, chat history, old projection text, a full report, or a complete reference dump.

Always-on agent context should fit on one screen or less and include only:

- current task summary, or explicit `none` / `unknown`
- work shape
- scope and non-goals
- pending user judgments
- active blockers
- next safe actions
- evidence gaps
- close blockers
- residual-risk summary
- guarantee level, or the unavailable/capability condition when Core or required MCP cannot answer
- source refs and freshness

Do not push by default: full Reference docs, full schemas, full Storage DDL, complete history, historical event logs, full projection bodies, full artifact contents, raw logs/screenshots/diffs/traces, full templates, unrelated templates, future catalogs, old task history, or unrelated Roadmap material.

Use phase-specific pull context:

| Phase | Minimal pull target |
|---|---|
| Session start / resume | Current `harness.status`, current task/status resources, and [Agent Guide: Report status for the user's next decision](../use/agent-guide.md#8-report-status-for-the-users-next-decision). |
| Planning / clarification | Current repo/docs/state refs and [Agent Guide: Clarify without endless planning loops](../use/agent-guide.md#4-clarify-without-endless-planning-loops). |
| Work-shape classification | Current scope/status refs and [Agent Guide: Classify the work shape](../use/agent-guide.md#3-classify-the-work-shape). |
| User judgment request | Current judgment refs or candidates and [Agent Guide: Request user judgment narrowly](../use/agent-guide.md#5-request-user-judgment-narrowly). |
| Write preparation | Current scope/state and [Agent Guide: Check scope before product writes](../use/agent-guide.md#6-check-scope-before-product-writes), plus `prepare_write` owner sections only for the intended write. |
| Execution / evidence | Current run/artifact refs and [Agent Guide: Record evidence after meaningful action](../use/agent-guide.md#7-record-evidence-after-meaningful-action). |
| Close readiness | Current owner records and [Agent Guide: Close work honestly](../use/agent-guide.md#10-close-work-honestly). |
| Recovery / error | Current availability/freshness state, [Fallback Semantics](#fallback-semantics), and the specific error owner section. |

Status cards, projections, rendered templates, recommendations, retrieved context, and chat memory are read-only. They can point the agent toward refs to inspect, but they cannot authorize writes, satisfy gates, create evidence, resolve user judgments, grant approval, perform verification, record QA, accept work, accept residual risk, repair projection freshness, or close a Task.

Token savings must not hide user-owned judgments, blockers, scope limits, safety boundaries, evidence gaps, close blockers, or close-relevant residual risk.

## Fallback Semantics

Fallbacks are described by guarantee level and risk, not by surface name.

| Fallback | Use when | Boundary |
|---|---|---|
| Cooperative | The surface can follow instructions but cannot enforce them. | Hold product writes by instruction when authoritative MCP or write scope checks are unavailable. |
| Detective | Harness can observe changed files, logs, artifacts, projection drift, or artifact gaps after action. | Mark state stale, partial, blocked, or failed and require repair, reconcile, or fresh evidence. |
| Preventive | A fixture-proven hook, permission layer, wrapper, policy engine, or sidecar can block before execution. | Claim only the operations covered by the proven blocking path. |
| Isolated | Risk requires separation. | Use the documented boundary named by the profile. Separation alone is not approval, verification, final acceptance, residual-risk acceptance, close, or assurance upgrade. |

If MCP is unavailable, the connector must not claim authoritative state updates. `MCP_SERVER_UNAVAILABLE` means the call path cannot reach Core. `SURFACE_MCP_UNAVAILABLE` means the connected surface lacks usable MCP, has stale MCP configuration, or cannot call required tools. These are diagnostic conditions; `MCP_UNAVAILABLE` remains the stable public availability code.

While Core is unreachable, do not invent Core state, Write Authorization, gate status, approvals, evidence, final acceptance, residual-risk acceptance, projection repair, or close readiness from chat memory, generated files, cached projections, old status text, or operator prose.

Projection staleness is separate from Core state. A connector may continue from current Core state if it can read it directly, but actions depending on stale readable projections must refresh or reconcile first.

Documentation-maintenance edits in this documentation-only repository are governed by the Authoring Guide, not by runtime Harness procedures. They do not create runtime state, Write Authorization, evidence, QA, acceptance, residual-risk acceptance, close readiness, projections, `task_events`, or runtime transitions.

## Role Lens Behavior

Role Lens is a non-authoritative skill or playbook surface that helps the user steer an agent from a familiar review posture, such as product review, engineering review, design review, security review, QA review, or release handoff.

A lens may recommend user judgment, evidence collection, verification, Manual QA, sensitive-action permission, residual-risk handling, scope updates, or a next playbook. The recommendation is read-only until an existing MCP/Core mutation path records the underlying action.

Same-session review is self-checking context. It is not detached verification and must not display detached verification unless the active verification owner path qualifies it.

## AFK And Public Commitment Display

AFK, unattended, or "continue while I am away" instructions do not create new authority. Product writes must stay inside active scope, active autonomy boundaries, granted sensitive-action permission when needed, and compatible `prepare_write` / Write Authorization.

Stop and show the smallest unblocker before scope expansion, new sensitive action, QA waiver or verification-risk acceptance, final acceptance, residual-risk acceptance, public API or module contract change, release/support promise, documentation promise that changes reader reliance, or another public commitment that needs user-owned product or material technical decision.

## Reference Surface Contract

Engineering Checkpoint and MVP-1 use only the reference-surface support needed to exercise one local project registration and the Core authority path. Later bullets in this section are profile targets, not Engineering Checkpoint or MVP-1 requirements.

Engineering Checkpoint minimum reference expectations:

- one registered `capability_profile` for `surface_id=reference-local-mcp`
- `mcp_available=true` for the public tool/resource subset needed by the first authority loop
- local-only or owner-approved access posture
- cooperative `prepare_write` before product writes and a compatible single-use Write Authorization record before write-capable `record_run`
- detective changed-path and artifact validation after runs
- no default OS sandbox, arbitrary-tool sandboxing, tamper-proof local files, isolation, or pre-tool blocking claim
- a run summary and at least one manually supplied artifact/evidence ref for the minimal authority loop
- honest display of pre-action stop versus after-action detection when guard, freeze, or careful-mode labels are shown

Later profile targets include user-readable status/next cards, compact user judgment display, evidence and close readiness summaries, evidence manifest support, manual verification bundles or fresh evaluator instructions, Manual QA note/artifact support, connector manifests, projection freshness, reconcile flow, and operator diagnostics. Broad connector ecosystems, hosted connector registries, and cross-surface orchestration stay later/profile or Roadmap until promoted.

## Connector Conformance Overview

Connector conformance should prove that a profile can uphold the common contract at its declared capability tier. Scenario lists are aggregate profile maps, not a single Engineering Checkpoint checklist. The active smoke target is the reference `capability_profile`, not a connector ecosystem.

Engineering Checkpoint reference-surface checks include:

- status with and without an active Task
- compact current-position status before significant resume when required by the Use procedure
- one registered reference `capability_profile` with `conformance_smoke_status` reported as planned or not run until runtime fixtures exist
- guarantee display derived from the actual profile fields, with no `preventive` or `isolated` claim when `pre_tool_blocking_supported=false` and `isolation_supported=false`
- basic scope checking for the selected path/tool/command
- `prepare_write` allowed and blocked paths, where allowed/blocked are Harness compatibility outcomes rather than OS permission or physical prevention
- single-use cooperative Write Authorization created only after `prepare_write.decision=allowed` and consumed by write-capable `record_run`
- `record_run` with a minimal artifact/evidence ref
- local-only MCP default or owner-approved alternative
- MCP-unavailable product-write hold
- `CAPABILITY_INSUFFICIENT` or an equivalent blocked reason when a requested write or guarantee depends on an unsupported capability
- read-only status recommendations unless a recommended action later follows a Core mutation path
- honest guarantee display for guard, freeze, or careful-mode labels

Later profile scenarios include user judgment routing with options and consequences, sensitive-action permission paths, full Change Unit handling, evidence and artifact integrity, verification bundles, Manual QA, final acceptance, residual-risk visibility and acceptance, stale projection/reconcile flow, generated-file drift, capability fallback, stale context refusal, surface capability mismatch handling, broader connectors, hosted connector registries, and cross-surface orchestration only after promotion.

Exact fixture format is owned by [Conformance Fixtures Reference](conformance-fixtures.md), and operational commands are owned by [Operations And Conformance Reference](operations-and-conformance.md).
