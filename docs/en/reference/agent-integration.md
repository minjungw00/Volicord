# Agent Integration Reference

## What this document helps you do

Use this reference when connecting an agent surface to future Harness behavior without overstating what that surface can enforce.

This reference owns the common connector contract: capability tiers, capability profiles, generated manifest expectations, context push/pull principles, fallback semantics, Role Lens behavior, reference-surface expectations, and connector conformance overview.

For what the agent says and does in a user session, read [Agent Guide](../use/agent-guide.md). For surface-specific setup notes, read [Surface Cookbook](surface-cookbook.md). Current repository phase and implementation handoff status are tracked in [Implementation Overview](../build/implementation-overview.md#documentation-acceptance-status).

## Read this when

- You are implementing or reviewing a connector for an agent surface.
- You need to declare, refresh, or audit a surface capability profile.
- You need to display guarantee level, guard, freeze, fallback, or MCP availability honestly.
- You are writing connector conformance coverage.
- You need to know what belongs in this common contract instead of a surface recipe.

## Before you read

Read [Agent Guide](../use/agent-guide.md) for behavior rules. Pull exact owner sections only when the connector action needs them: [Core Model Reference](core-model.md) for write and close authority, [MVP API](api/mvp-api.md) for active MVP-1 method contracts, [API Schema Core](api/schema-core.md) for shared shapes, [API Errors](api/errors.md) for public error behavior, and [Security Reference](security.md) for threat and guarantee wording.

This reference is not an instruction to load all Reference docs into agent context.

## Main idea

A connector gives the agent small current context, routes state-changing actions through Harness, captures what happened when the surface can do so, and reports only guarantees proven for the actual surface profile in use.

Surface name is not capability. A connector may claim cooperative, detective, preventive, or isolated behavior only for the concrete host, profile, version/configuration, workspace policy, MCP posture, capture path, guard path, or separation boundary that has been declared and checked.

## Integration In Plain Language

An agent surface is where the user talks to an agent. Harness is the local authority layer that keeps scope, user judgment, write checks, evidence refs, work acceptance, residual-risk acceptance, and close readiness outside the chat transcript.

The common path is:

```text
user conversation surface
  -> short always-on rules/context
  -> Harness skill, command, or playbook
  -> Harness MCP server
  -> Harness Core
  -> adapter, hook, sidecar, validator, capture path, or isolation layer
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
| Public MCP request/response schemas | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [API Schema Later](api/schema-later.md) |
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

Engineering Checkpoint and MVP-1 connectors should assume cooperative or detective behavior unless a concrete profile proves a stronger capability. `T4` and `T5` do not imply default OS isolation, arbitrary-tool sandboxing, tamper-proof files, or pre-tool blocking.

## Capability Profiles

Connectors must use a capability profile instead of assuming behavior from a product name, surface name, or mode label. A profile is scoped to the actual host/profile that will run the work.

Every profile must include, in connector-owned field names:

- surface id, surface kind, target profile, detected version when available, profile version, and last verified time
- support tier and guarantee level
- MCP tool/resource availability and local exposure posture
- access-control material class without raw secrets or private configuration values
- capture, QA capture, guard, isolation, changed-path detection, redaction, and artifact-retention behavior
- known risks and fallbacks
- the conformance result or operator check that made the declaration current

Example shape:

```yaml
surface_id: SURF-0001
surface_kind: generic_agent
target_profile: local_cli
detected_version: optional string
capability_profile_version: 1
last_verified_at: 2026-05-06T10:05:00+09:00
support_tier: T2
guarantee_level: cooperative
capabilities:
  mcp_tools: true
  mcp_resources: true
  structured_output: false
  artifact_capture: manual
  pre_tool_guard: false
  changed_path_detection: validator
  worktree_isolation: false
risks:
  - no proven pre-tool guard
fallbacks:
  - cooperative prepare_write discipline
  - changed-path validation
  - manual artifact capture
```

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

Guard, freeze, and careful-mode labels are display labels over the actual profile. They must say what can actually be blocked before execution and what can only be detected later. They are not approval, verification, work acceptance, residual-risk acceptance, close readiness, or a kernel gate.

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
- guarantee level
- source refs and freshness

Do not push by default: full Reference docs, full schemas, full Storage DDL, complete history, historical event logs, full projection bodies, full artifact contents, raw logs/screenshots/diffs/traces, full templates, unrelated templates, future catalogs, old task history, or unrelated Roadmap material.

Use phase-specific pull context:

| Phase | Minimal pull target |
|---|---|
| Session start / resume | Current `harness.status`, current task/status resources, and [Agent Guide: Report Status](../use/agent-guide.md#10-report-status). |
| Planning / clarification | Current repo/docs/state refs and [Agent Guide: Clarify Requirements](../use/agent-guide.md#4-clarify-requirements). |
| Work-shape classification | Current scope/status refs and [Agent Guide: Classify Work Shape](../use/agent-guide.md#3-classify-work-shape). |
| User judgment request | Current judgment refs or candidates and [Agent Guide: Request User Judgment](../use/agent-guide.md#5-request-user-judgment). |
| Write preparation | Current scope/state and [Agent Guide: Pre-Write Scope Check](../use/agent-guide.md#8-pre-write-scope-check), plus `prepare_write` owner sections only for the intended write. |
| Execution / evidence | Current run/artifact refs and [Agent Guide: Record Evidence](../use/agent-guide.md#9-record-evidence). |
| Close readiness | Current owner records and [Agent Guide: Close Work](../use/agent-guide.md#11-close-work). |
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
| Isolated | Risk requires separation. | Use the documented boundary named by the profile. Separation alone is not approval, verification, acceptance, risk acceptance, close, or assurance upgrade. |

If MCP is unavailable, the connector must not claim authoritative state updates. `MCP_SERVER_UNAVAILABLE` means the call path cannot reach Core. `SURFACE_MCP_UNAVAILABLE` means the connected surface lacks usable MCP, has stale MCP configuration, or cannot call required tools. These are diagnostic conditions; `MCP_UNAVAILABLE` remains the stable public availability code.

While Core is unreachable, do not invent Core state, Write Authorization, gate status, approvals, evidence, work acceptance, residual-risk acceptance, projection repair, or close readiness from chat memory, generated files, cached projections, old status text, or operator prose.

Projection staleness is separate from Core state. A connector may continue from current Core state if it can read it directly, but actions depending on stale readable projections must refresh or reconcile first.

Documentation-maintenance edits in this documentation-only repository are governed by the Authoring Guide, not by runtime Harness procedures. They do not create runtime state, Write Authorization, evidence, QA, acceptance, residual-risk acceptance, close readiness, projections, `task_events`, or runtime transitions.

## Role Lens Behavior

Role Lens is a non-authoritative skill or playbook surface that helps the user steer an agent from a familiar review posture, such as product review, engineering review, design review, security review, QA review, or release handoff.

A lens may recommend user judgment, evidence collection, verification, Manual QA, sensitive-action permission, residual-risk handling, scope updates, or a next playbook. The recommendation is read-only until an existing MCP/Core mutation path records the underlying action.

Same-session review is self-checking context. It is not detached verification and must not display detached verification unless the active verification owner path qualifies it.

## AFK And Public Commitment Display

AFK, unattended, or "continue while I am away" instructions do not create new authority. Product writes must stay inside active scope, active autonomy boundaries, granted sensitive-action permission when needed, and compatible `prepare_write` / Write Authorization.

Stop and show the smallest unblocker before scope expansion, new sensitive action, QA or verification waiver, work acceptance, residual-risk acceptance, public API or module contract change, release/support promise, documentation promise that changes reader reliance, or another public commitment that needs user-owned product or material technical judgment.

## Reference Surface Contract

Engineering Checkpoint uses only the reference-surface support needed to exercise one local project registration and the Core authority path. Later bullets in this section are profile targets, not Engineering Checkpoint requirements.

Engineering Checkpoint minimum reference expectations:

- `T2 MCP` for the public tool/resource subset needed by the first authority loop
- local-only or owner-approved access posture
- cooperative `prepare_write` before product writes and compatible Write Authorization before write-capable `record_run`
- detective changed-path and artifact validation after runs
- no default OS sandbox, arbitrary-tool sandboxing, tamper-proof local files, or pre-tool blocking claim
- a run summary and at least one manually supplied or captured artifact/evidence ref for the minimal authority loop
- honest display of pre-action stop versus after-action detection when guard, freeze, or careful-mode labels are shown

Later profile targets include user-readable status/next cards, compact user judgment display, evidence and close readiness summaries, evidence manifest support, manual verification bundles or fresh evaluator instructions, Manual QA note/artifact support, connector manifests, projection freshness, reconcile flow, and operator diagnostics.

## Connector Conformance Overview

Connector conformance should prove that a profile can uphold the common contract at its declared capability tier. Scenario lists are aggregate profile maps, not a single Engineering Checkpoint checklist.

Engineering Checkpoint connector checks include:

- status with and without an active Task
- compact current-position status before significant resume when required by the Use procedure
- basic scope checking for the selected path/tool/command
- `prepare_write` allowed and blocked paths
- Write Authorization created for allowed writes and consumed by write-capable `record_run`
- `record_run` with a minimal artifact/evidence ref
- local-only MCP default or owner-approved alternative
- MCP-unavailable product-write hold
- read-only status recommendations unless a recommended action later follows a Core mutation path
- honest guarantee display for guard, freeze, or careful-mode labels

Later profile scenarios include user judgment routing with options and consequences, sensitive-action permission paths, full Change Unit handling, evidence and artifact integrity, verification bundles, Manual QA, work acceptance, residual-risk visibility and acceptance, stale projection/reconcile flow, generated-file drift, capability fallback, stale context refusal, and surface capability mismatch handling.

Exact fixture format is owned by [Conformance Fixtures Reference](conformance-fixtures.md), and operational commands are owned by [Operations And Conformance Reference](operations-and-conformance.md).
