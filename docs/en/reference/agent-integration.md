# Agent Integration Reference

## What this document helps you do

Use this reference to connect an agent surface to Harness without overstating what that surface can enforce.

It owns the common connector contract: capability tiers, capability profiles, generated manifest expectations, context push/pull principles, fallback semantics, Role Lens behavior, the reference surface contract, and connector conformance overview.

For the user-facing agent procedure, read [Agent Session Flow](../use/agent-session-flow.md). For surface-specific setup notes, read [Surface Cookbook](surface-cookbook.md).

## Read this when

- You are implementing or reviewing a connector for an agent surface.
- You need to declare or audit a surface capability profile.
- You need to decide how a connected profile should display guarantee level, guard, freeze, fallback, or MCP availability.
- You are writing connector conformance coverage.
- You need to know which parts belong in a surface recipe instead of the common contract.

## Integration in plain language

An agent surface is where the user talks to an agent. Harness is the local authority layer that keeps task state, write authority, evidence, verification, Manual QA, acceptance, projections, and reconcile behavior outside the chat transcript.

A connector should give the agent small current context, route state changes through Harness MCP tools, capture what happened when the surface can do so, and name the actual guarantee level for the connected profile. A surface name is never enough to claim a capability.

The common structure is:

```text
user conversation surface
  -> short always-on rules/context
  -> harness skill, command, or playbook
  -> harness MCP server
  -> harness Core
  -> adapter, hook, sidecar, validator, or isolation layer
```

Always-on rules should stay short. They should say when to use Harness, where to read status or the Journey Card, that product writes require `prepare_write`, that user-owned judgment routes through Decision Packets, that status must show what can actually be blocked and what can only be detected later, and that product writes hold when authoritative MCP is unavailable. The session procedure itself belongs in [Agent Session Flow](../use/agent-session-flow.md).

## What belongs in Use docs vs this Reference doc

| Area | Owner |
|---|---|
| What the agent shows, asks, and says during a user session | [Agent Session Flow](../use/agent-session-flow.md) |
| User-facing explanation of scope, evidence, verification, QA, residual risk, and close | [User Guide](../use/user-guide.md) |
| Common connector contract, capability profiles, manifests, context model, fallback semantics, Role Lens, reference surface, conformance overview | This reference |
| Concrete surface recipes for Codex, Claude Code, Gemini, GitHub Copilot, and Cursor | [Surface Cookbook](surface-cookbook.md) |
| Public MCP request/response schemas | [MCP API And Schemas](mcp-api-and-schemas.md) |
| Kernel state transitions and write/close rules | [Kernel Reference](kernel.md) |
| Runtime guarantee level definitions | [Runtime Architecture Reference](runtime-architecture.md#guarantee-levels) |

## Capability Tiers

| Tier | Meaning | Typical capability |
|---|---|---|
| `T0 Context` | Surface can read Harness principles. | rules/context file |
| `T1 Skill` | Surface can follow a Harness procedure. | skill, command, prompt, playbook |
| `T2 MCP` | Surface can call Harness tools and resources. | MCP server connection |
| `T3 Capture` | Surface can return diffs, logs, and run output reliably. | structured output, wrapper, adapter |
| `T4 Guard` | Surface can block or interrupt covered out-of-scope files, commands, network, or secrets before execution when the profile proves that path. | hook, permission system, policy engine, sidecar |
| `T5 Isolation` | Surface can run verification or risky work in a separate boundary. | worktree, sandbox, fresh process, isolated runner |
| `T6 QA Capture` | Surface can structure browser, screenshot, walkthrough, workflow-recording, or Manual QA artifacts. | browser runner, screenshot capture, console/network capture, accessibility snapshot, QA note capture |

Normal interactive Harness use is most natural at `T2` or higher. Reliable detached verification usually needs `T3` capture plus a real independence boundary. High-risk work should use a proven `T4` guard or `T5` isolation when available. `T6` improves UI/UX evidence, but it does not replace Manual QA judgment and is not required for MVP when a human QA note can be recorded.

`T6 QA Capture` profiles must name supported capture types and fallback behavior. Candidate capture types include screenshot, console log, network trace, accessibility snapshot, and workflow recording. Captured files must follow redaction and secret/PII handling before durable storage and should be registered as artifact refs attached to the Manual QA record or feedback loop execution.

## Capability Profiles

Harness connectors must use a capability profile instead of assuming behavior from a product or surface name.

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
  project_rules: true
  skills_or_commands: true
  mcp_tools: true
  mcp_resources: true
  structured_output: false
  artifact_capture: manual
  hooks: false
  pre_tool_guard: false
  explicit_permissions: false
  changed_path_detection: validator
  fresh_verify: manual_bundle
  worktree_isolation: false
  local_sidecar: false
  browser_qa_capture: false
  screenshot_capture: false
  console_log_capture: false
  network_trace_capture: false
  accessibility_snapshot_capture: false
  workflow_recording_capture: false
risks:
  - no pre-tool guard
fallbacks:
  - cooperative prepare_write discipline
  - changed_paths validator
  - manual verification bundle
  - human Manual QA notes and manually supplied QA artifacts
```

Target profile values may include:

- `local_cli`
- `ide_chat`
- `ide_agent`
- `cloud_agent`
- `extension`
- `custom_agent`
- `manual_bundle`

Capability profiles must be refreshed when version, MCP config, hooks, permissions, workspace policy, generated files or managed blocks, conformance result, capture method, QA capture method, browser test environment, redaction policy, artifact retention behavior, or isolation/guard wrapper behavior changes.

## Capability Profile Examples

These are examples of profile shapes. A tier or example does not automatically upgrade a concrete surface's guarantee level. A concrete connector must prove the capability for its actual host/profile before claiming it.

### Cooperative MCP Profile

```yaml
surface_id: SURF-0001
surface_kind: generic_agent
target_profile: ide_chat
support_tier: T2
guarantee_level: cooperative
capabilities:
  project_rules: true
  skills_or_commands: true
  mcp_tools: true
  mcp_resources: true
  structured_output: false
  artifact_capture: manual
  pre_tool_guard: false
  changed_path_detection: validator
  fresh_verify: manual_bundle
  worktree_isolation: false
fallbacks:
  - cooperative prepare_write
  - changed_paths validator
  - manual verify bundle
```

### Detective Capture Profile

```yaml
surface_id: SURF-0002
surface_kind: generic_agent
target_profile: local_cli
support_tier: T3
guarantee_level: detective
capabilities:
  project_rules: true
  skills_or_commands: true
  mcp_tools: true
  mcp_resources: true
  structured_output: true
  artifact_capture: wrapper
  pre_tool_guard: false
  changed_path_detection: sidecar
  command_output_capture: wrapper
  fresh_verify: manual_bundle
  worktree_isolation: false
fallbacks:
  - sidecar changed-file watcher
  - artifact integrity check
  - fresh evaluator instructions
```

### Guarded Local Profile

```yaml
surface_id: SURF-0003
surface_kind: generic_agent
target_profile: local_cli
support_tier: T4
guarantee_level: preventive
capabilities:
  project_rules: true
  skills_or_commands: true
  mcp_tools: true
  mcp_resources: true
  structured_output: true
  artifact_capture: wrapper
  hooks: true
  pre_tool_guard: true
  explicit_permissions: true
  changed_path_detection: sidecar
  command_output_capture: wrapper
  fresh_verify: fresh_session
  worktree_isolation: optional
fallbacks:
  - sidecar guard for proven covered operations
  - approval card
  - fresh evaluator profile
```

### Isolated Verification Profile

```yaml
surface_id: SURF-0004
surface_kind: manual_bundle
target_profile: manual_bundle
support_tier: T5
guarantee_level: isolated
capabilities:
  mcp_tools: false
  mcp_resources: false
  structured_output: true
  artifact_capture: bundle
  pre_tool_guard: read_only_bundle
  changed_path_detection: bundle_manifest
  fresh_verify: fresh_worktree
  worktree_isolation: true
fallbacks:
  - read-only evaluator bundle
  - operator record_eval
```

## Guarantee Levels

Integration uses the guarantee levels defined in [Runtime Architecture Reference](runtime-architecture.md#guarantee-levels) and applies them to connected surface profiles, current enforcement paths, and fallback choices.

This reference owns how connector profiles report and display those levels. It must not infer a stronger level from a surface name, and it must not treat guarantee level as approval, verification, QA, acceptance, or a kernel gate.

| Level | Display responsibility |
|---|---|
| `cooperative` | Show that the surface is expected to follow Harness decisions, but Harness does not claim physical blocking before execution. |
| `detective` | Show that Harness can observe changed paths, logs, artifacts, or projection drift after action and mark state stale, blocked, partial, or failed; display this as detection, not prevention. |
| `preventive` | Show the proven hook, wrapper, permission layer, policy engine, or sidecar path and the covered operations it can block before execution. |
| `isolated` | Show the separate worktree, sandbox, process, evaluator bundle, or equivalent boundary used for risky work or verification. |

Guard, freeze, and careful-mode labels are safety-control labels over the actual profile. Their display must say what can actually be blocked before execution and what can only be detected later.

| User wording | Actual boundary |
|---|---|
| Freeze | A visible scope hold or stricter next-action posture around current work. On cooperative or detective profiles it is not hard prevention; persistent owner-record changes still route through the normal Core path. |
| Guard | Cooperative, detective, preventive, or isolated protection according to the proven profile and current enforcement path. Use preventive wording only for covered operations with proven pre-execution blocking. |
| Careful mode | Stricter `prepare_write`, scope, evidence, status refresh, and user-question posture. It is not a new authority tier and does not block by itself. |

## Generated Manifest Expectations

Connectors may generate rules, skills, MCP config snippets, prompts, or local adapter files. Every generated or managed path, managed block, MCP config snippet, and profile freshness marker must be recorded in a connector manifest.

The manifest must:

- name generated and managed paths, including MCP config snippets and local adapter files
- record managed block ids and hashes
- record the capability profile used when generated, including `capability_profile_version`, `detected_version`, `last_verified_at`, and the conformance result or operator check that made it current
- record the target surface profile and MCP tool/resource scope
- record configured capture, QA capture, guard, and isolation mechanisms without claiming more than the profile proves
- record manual artifact capture and manual verification bundle fallbacks when native capture or isolation is unavailable
- record creation and update times
- mark the profile or generated block stale when the surface version, MCP config, hooks, permissions, wrapper, sidecar, managed file, capture method, redaction policy, or retention behavior changes
- detect drift before overwriting human edits
- route drift to reconcile when needed

The manifest concept is common. Surface-specific generated filenames belong in [Surface Cookbook](surface-cookbook.md).

## Context Push/Pull Principles

Implementation agents should receive a compact always-on Harness context envelope every turn and pull larger references only when needed. The envelope is operational state, not history. It should use ids, one-line summaries, and freshness markers; keeping it around a screenful is useful guidance, not a schema limit.

Push every turn when available:

| Envelope item | Push shape |
|---|---|
| Active Task | Task id, title, mode, and lifecycle phase. |
| Next safe action | The next action and smallest unblocker if blocked. |
| Active Change Unit | One-line summary of in-scope work, out-of-bounds areas, and active Autonomy Boundary. |
| Blocking decisions | Decision Packet ids and one-line questions, or `none`. |
| Write authority | Display status such as not requested, allowed, blocked, stale, or unavailable, with scoped path/tool summary when relevant. |
| Guarantee level | Actual connected profile level and the guard or detection behavior it can prove. Do not infer this from a surface name. |
| Connector profile freshness | Connector manifest ref, `capability_profile_version`, `last_verified_at`, and stale reason when generated files, MCP config, hooks, wrappers, sidecars, capture, or isolation behavior changed. |
| Gate summary | Scope, approval, decision, design, evidence, verification, QA, acceptance, close blocker, Manual QA, and residual-risk status as compact values when relevant. |
| Projection freshness | Projection id or ref, `source_state_version` when known, freshness state, and refresh/reconcile warning when needed. |

Push refs or one-line summaries when relevant:

- Journey Card or compact status card
- current acceptance criteria snapshot
- approval status
- latest evidence manifest ref and coverage summary
- latest Run, Eval, Manual QA, report, and residual-risk refs
- relevant policy, TDD trace, stewardship, module/interface, and domain refs

Keep these refs-first and pull the body only when needed:

- Evidence, Run, Eval, and Manual QA records
- artifacts, logs, screenshots, diffs, workflow recordings, and large traces
- older PRDs, old designs, closed issues, stale docs, and moved-path notes
- module maps, interface contracts, domain language, coding standards, and TDD guidance

Refs-first means the connector should push stable ids, paths, hashes, summaries, outcomes, and freshness, not paste large bodies into the default prompt. Embed excerpts only when the next safe action requires inspecting the content, and keep the excerpt tied to its source ref.

The compact status card renders the envelope for "where are we and what happens next?" Judgment-context is separate. Use judgment-context only when user judgment is needed, and include the decision question, options, recommendation, uncertainty, deferral effect, and relevant refs without turning the full evidence or artifact body into always-on context.

Evaluators should receive a tighter verification bundle: acceptance criteria, changed files, approval scope, relevant Decision Packets, residual risk summary, Autonomy Boundary, deferred decisions, codebase stewardship refs, evidence manifest refs, required TDD trace refs, Manual QA requirement, artifact refs, freshness state, and forbidden patterns.

A later Context Index may help retrieve relevant projections, artifact refs, repo files, docs, or notes. It is a read-only context provider, not a connector authority path.

## Fallback Semantics

Fallbacks are described by guarantee level and risk, not by surface name.

| Fallback | Use when | Boundary |
|---|---|---|
| Cooperative | The surface can follow instructions but cannot enforce them. | Tell the agent to use `prepare_write`, hold on blocked decisions, and record runs. Product writes pause if authoritative MCP is unavailable or write scope cannot be checked. |
| Detective | Harness can observe changed files, logs, projection drift, or artifact gaps after action. | Validators may mark state stale, partial, blocked, or failed and require repair, reconcile, or fresh verification. |
| Preventive | A proven hook, permission layer, wrapper, policy engine, or sidecar can block before execution. | Claim only the operations that the proven blocking path actually covers. |
| Isolated | Risk requires separation. | Launch work or verification in a separate worktree, sandbox, process, or manual evaluator bundle. |

If MCP is unavailable, the connector must not claim authoritative state updates. `MCP_SERVER_UNAVAILABLE` and `SURFACE_MCP_UNAVAILABLE` are diagnostic conditions, not additional public `ErrorCode` values. `MCP_UNAVAILABLE` remains the stable public availability code.

`MCP_SERVER_UNAVAILABLE` means the tool call cannot reach Core, so no authoritative Core response is possible. `SURFACE_MCP_UNAVAILABLE` means Core or an operator can observe that the connected surface lacks usable MCP, has stale MCP configuration, or cannot call required tools. Product/runtime/code writes hold until MCP is reconnected or diagnosed, unless the work is an explicit pre-MVP documentation-authoring batch under `DOCS_AUTHORING_OVERRIDE` with an exact path allowlist. That override is a documentation-maintainer override only; it is not Core authorization, Write Authorization, evidence, verification, QA, acceptance, residual-risk acceptance, close, or a canonical state transition.

If MCP works but pre-tool guard is weak, low-risk direct work may proceed with cooperative `prepare_write` and detective changed-path validation. Medium/high-risk work should require stricter validation, a proven sidecar guard, explicit approval, detached verification, or isolation.

If native capture is unavailable, the connector should fall back to manual artifact capture: named artifact refs for diffs, logs, screenshots, workflow notes, command output, or QA notes supplied by the user or operator. If native isolation or fresh evaluator support is unavailable, it should fall back to a manual verification bundle with acceptance criteria, changed files, relevant refs, artifact refs, freshness state, and forbidden patterns. These fallbacks are explicit evidence routes, not upgrades to preventive or isolated guarantee levels.

Projection staleness is reported separately from state. If `source_state_version` is older than the canonical state, unknown, or missing where it is expected, the connector should warn that readable projection context may be stale. A connector may continue from canonical state if it can read state directly, but actions that depend on Markdown projection should refresh or reconcile first and should not treat the stale projection as authority.

## Role Lens Behavior

Role Lens is a non-authoritative skill or playbook surface that helps the user steer the agent from a familiar review posture. Initial lenses are:

- `product-review`
- `eng-review`
- `design-review`
- `security-review`
- `qa-review`
- `release-handoff`

A connector may expose these as slash commands, buttons, prompt snippets, or recommended playbooks. The lens name selects a review posture; it does not select an authority path.

Role Lens output may surface or recommend routes for:

- a `DecisionPacketCandidate` or a route to an existing Decision Packet
- a validator finding candidate or suggested `ValidatorResult` route for an actual validator/check to emit
- an evidence requirement
- an Eval or verification need
- a Manual QA requirement
- an Approval need
- a residual-risk candidate
- a Change Unit update recommendation when appropriate
- release handoff report input
- a recommended next playbook

These are display and routing outputs until an existing Core/MCP state-changing path records the underlying action. Role Lens output must not introduce schemas or canonical records, mutate canonical state by itself, authorize writes, grant Approval, satisfy a Decision Packet, waive QA or verification, accept residual risk, accept the result, close a Task, or upgrade assurance. When a lens identifies work that needs a state change, the surface routes through the normal MCP tool and Core path.

Two-stage review display should keep the stages visibly separate:

| Stage | Question |
|---|---|
| Spec Compliance Review | Is the requested work complete under current Harness authority: acceptance criteria, Change Unit completion conditions, scope/write authority compatibility, Decision Packet compatibility, evidence coverage, and residual-risk visibility? |
| Code Quality / Stewardship Review | Is the implementation maintainable: domain language, module/interface boundary, vertical slice shape, feedback loop or TDD trace, codebase stewardship, context hygiene, and follow-up risk? |

Same-session review may be useful self-checking, but it is not detached verification and must not display `assurance_level=detached_verified`.

## Reference Surface Contract

The MVP targets one reference surface. The reference surface should demonstrate the kernel rather than broad ecosystem support.

Minimum reference expectations:

- `T2 MCP` available for public tools and resources
- cooperative `prepare_write` before product writes
- detective changed-path and artifact validation after runs
- run summary and artifact capture sufficient for evidence manifests
- manual verification bundle or fresh evaluator instructions
- Manual QA note artifact support
- connector manifest for generated files, managed blocks, MCP config snippets, and profile freshness
- manual artifact capture fallback when native capture is unavailable
- actual block-vs-detect status when guard, freeze, or careful-mode labels are shown
- conformance smoke covering common state and fallback paths

Reference surface behavior details and surface-specific setup belong in [Surface Cookbook](surface-cookbook.md) only when they name a concrete surface.

## Connector Conformance Overview

Connector conformance should prove that a profile can uphold the common contract at its declared capability tier.

Overview scenarios:

- status with and without an active Task
- current Journey Card shown before significant work resumes when required by the Use procedure
- intake classification into `advisor`, `direct`, or `work`
- work shaping with shared design and decisions
- Change Unit scope and vertical/horizontal exception handling
- one blocking question with recommendation and uncertainty when available
- Decision Packet shown instead of broad approval for blocking user-owned judgment
- Autonomy Boundary breach stops or routes to Decision Packet
- AFK work remains covered by active Change Unit scope, Autonomy Boundary latitude, any granted sensitive approval that applies, and compatible `prepare_write` / Write Authorization before actual product writes
- `prepare_write` allowed and blocked paths
- Write Authorization created for allowed writes and exposed through Write Authority Summary
- write-capable `record_run` consumes a compatible Write Authorization
- sensitive approval request, granted, denied, and expired paths
- `record_run` with artifacts and evidence update
- direct result projection
- verification launch or manual verification bundle
- same-session verification guard
- Manual QA required, passed, failed, and waived
- QA waiver with product/user risk routes through Decision Packet
- acceptance required and recorded
- close-relevant residual risk visible before acceptance or successful close
- risk-accepted close additionally requires accepted Residual Risk refs
- stale projection and reconcile flow
- generated file drift detection
- connector manifest profile freshness and stale capability profile detection
- capability fallback when a required tier is missing
- MCP unavailable product-write hold

Exact fixture format and operational commands are owned by the operations and conformance docs.
