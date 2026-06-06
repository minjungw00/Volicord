# Surface Cookbook Reference

## What this document helps you do

Use this reference to check the active reference local surface recipe and to keep other surface notes out of MVP scope until an owner promotes them.

This document owns local setup notes, generated file names, MCP configuration hints, capture/guard/isolation options, common fallbacks, and conformance risks that vary by surface. The common surface contract and the active reference `capability_profile` live in [Agent Integration Reference](agent-integration.md).

This is reference documentation for future Harness behavior. Current repository phase and implementation handoff status are tracked in [MVP Plan](../build/mvp-plan.md#documentation-acceptance-status).

## Read this when

- You are writing or reviewing the active reference local surface recipe.
- You need to keep later surface notes separate from the common surface contract.
- You need to describe capture, guard, isolation, fallback, or conformance risks without overstating guarantee level.

## Before you read

Read [Agent Integration Reference](agent-integration.md) for the common connector contract, capability profiles, and phase context profiles. Use only the specific [Runtime Architecture](runtime-architecture.md), [MVP API](api/mvp-api.md), [API Errors](api/errors.md), or [Operations And Conformance Reference](operations-and-conformance.md) owner section needed for the current local-access, API-error, or conformance question. This cookbook is not a prompt-loading bundle and does not ask a connector to load all Reference docs.

## Main idea

A surface name never implies a guarantee level. The active MVP targets one reference local surface profile, and that profile's proven fields determine the guarantee level.

For generic capability profile rules, see [Agent Integration Reference](agent-integration.md#capability-profiles).

Surface recipes do not define local-access public error codes or OS-level security guarantees. They inherit the Engineering Checkpoint/default reference local-only MCP posture from [Runtime Architecture](runtime-architecture.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [Security Reference](security.md), and [Agent Integration](agent-integration.md). The active recipe uses cooperative/detective wording only; preventive and isolated options remain future/profile-specific until a concrete profile is promoted and proven. Recipe text does not upgrade the guarantee level or create authority beyond Harness records, state transitions, scope, user judgments, evidence, and close readiness. A recipe must not expose raw token, secret, or private configuration values, and remote, shared, non-loopback, forwarded, or tunneled MCP exposure is never implied by the surface name. If MCP/Core cannot be reached, route through `MCP_UNAVAILABLE` or operations diagnostics such as `MCP_SERVER_UNAVAILABLE`; if a reachable local caller or access mode is outside the registered profile, use `LOCAL_ACCESS_MISMATCH` with display-safe diagnostic detail; if a recognized surface/profile cannot satisfy a required capability, use `CAPABILITY_INSUFFICIENT`. Do not introduce a surface-specific `UNAUTHORIZED` public code.

## Recipe shape

Each active recipe should keep only surface-specific material:

- target profiles that are plausible for the surface
- generated files or instructions
- MCP configuration hints
- MCP exposure posture and local transport assumptions when they vary by surface
- host/profile-specific capability differences, including version, hooks, permissions, workspace policy, generated files, capture methods, QA capture methods, redaction policy, artifact retention behavior, and conformance result differences that require a refreshed profile under the [Agent Integration Reference](agent-integration.md#capability-profiles)
- capture, guard, and isolation options
- surface-specific context strategy: how the recipe keeps always-on context to current task summary, work shape, scope/non-goals, pending user judgments, active blockers, next safe actions, evidence gaps, close blockers, residual-risk summary, guarantee level, and source refs/freshness; how it switches among planning/clarification, write preparation, execution/run recording, evidence review, close readiness, user judgment request, and recovery/error profiles; and which minimal owner section it pulls for each profile according to [Agent Integration: Context Push/Pull Principles](agent-integration.md#context-pushpull-principles)
- guarantee boundary notes: what the named surface can block before execution, what it can only detect after action, what capture is native, and what falls back to manual artifacts or a manual verification bundle
- common fallbacks
- conformance risks

Do not repeat generic kernel rules, public API schemas, policy contracts, full reference docs, unrelated templates, historical logs, full Storage DDL, full Conformance catalogs, unrelated Roadmap items, or full projection bodies here. The common contract determines what cooperative, detective, preventive, and isolated mean. A recipe only names the surface-specific path that can provide that behavior. Guard, freeze, and careful-mode labels may appear only as labels over the connected profile's actual capability. When a recipe uses one of those labels, it must say whether the behavior is a scope hold, a post-action detector, a fixture-proven pre-tool block, or a documented separation boundary. Those labels do not authorize writes, satisfy gates, record verification, record acceptance, or create a new authority tier.

Generated or managed recipe outputs must follow the connector manifest contract in [Agent Integration Reference](agent-integration.md#generated-manifest-expectations). Recipes may name the surface-specific files, config snippets, or managed blocks, but drift is reported before overwrite. The existing file or managed block stays in place until reconcile or an explicit reconnect decision chooses replacement, and the drifted generated file is not treated as canonical Task state.

The `guarantee_boundary` blocks below are recipe documentation notes, not public schema, DDL shape, or canonical Capability Profile fields. A connector may record equivalent facts in its Capability Profile or Connector Manifest only according to the [Agent Integration Reference](agent-integration.md) contract. Surface Cookbook names surface-specific paths and examples; it does not redefine guarantee levels.

When a recipe lists a manual verification bundle under `fallback_isolation`, read it as verification/evaluator fallback input. A manual verification bundle does not by itself upgrade the connected surface to `preventive` or `isolated`. An `isolated` guarantee still requires a documented and proven separation boundary. Worktrees or fresh bundles can support verification independence or stale-context control; OS sandboxing, permission isolation, hard process/container isolation, or tamper-proof security requires the connector profile to name and prove that exact mechanism. Isolation remains separate from Approval, QA, final acceptance, residual-risk acceptance, close, and verification results.

## Reference Local Surface

```yaml
surface_kind: reference_local_mcp
target_profiles:
  - local_mcp_reference
generated_files_or_instructions:
  - a short managed agent-instruction block only when the selected surface needs one
  - MCP config snippet
  - connector manifest entry
mcp_configuration_hints:
  - local-only registered project posture
  - use only the public tools and resources listed in the active capability_profile
  - record generated MCP config paths, managed hashes, and capability_profile freshness in the connector manifest
capability_profile_fields:
  surface_id: reference-local-mcp
  surface_name: Reference local MCP surface
  mcp_available: true
  public_tools_available: active MVP-1 method set, with Engineering Checkpoint using a subset
  resources_available: Engineering Checkpoint and MVP-1 read-only resource subset
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
  conformance_smoke_status: planned_not_run
guarantee_boundary:
  default_level: cooperative
  max_level: detective only for supported after-action observation
  can_block_before_execution: false
  can_detect_after_action: changed paths and artifact gaps when the owner path observes them
  native_capture: none in the minimum reference profile
  fallback_capture: manual artifact attachment for diffs, logs, screenshots, command output, and QA notes
  fallback_isolation: not supported by the active reference profile
capture_guard_isolation_options:
  - changed_paths validator
  - explicit record_run discipline for summaries and artifact refs
  - manual artifact attachment when native capture is unavailable
  - no pre-tool blocking claim
  - no isolation claim
common_fallbacks:
  - cooperative prepare_write discipline
  - detective changed-path validation
  - manual artifact attachment
  - hold product writes by instruction when MCP/Core or required capability is unavailable
conformance_risks:
  - conformance_smoke_status must not be reported as passed before runtime fixtures are materialized and run
  - native artifact capture, command observation, network observation, secret access observation, pre-tool blocking, and isolation are unsupported
  - unsupported capabilities must lower guarantee display or return CAPABILITY_INSUFFICIENT / structured blocked reasons
  - product writes must not proceed silently when the required Harness record/check path or required capability is unavailable
```

The reference local surface is enough for the active MVP path because it exercises the kernel through MCP, Core authority checks, single-use cooperative Write Authorization, `record_run`, manual artifacts, and honest guarantee display. It is deliberately not a broad connector platform.

Reference-surface wording may expose phrases such as "hold this task to these paths" or "show whether this profile can block before execution and what it can only detect later." Because `pre_tool_blocking_supported=false`, describe the hold as cooperative scope discipline plus detective changed-path validation when available, not as preventive guard.

## Later Surface Notes

The named surfaces below are not active MVP requirements. They are later notes for future owner promotion.

| Surface note | Later-only boundary |
|---|---|
| Codex | May use `AGENTS.md`, skills, commands, or MCP snippets in a future profile. Instruction text alone stays cooperative; pre-tool blocking, native capture, and isolation require exact profile proof. |
| Claude Code | May use hook concepts such as `PreToolUse` only after a promoted profile proves covered behavior for the concrete host/version. Until then, guard/freeze wording stays cooperative or detective. |
| Gemini | May use extension or CLI-wrapper notes later. Extension wording alone is not a guard, and context must stay compact. |
| GitHub Copilot | IDE and cloud behavior must not be blurred. Task wrappers or cloud policy paths need separate promoted profiles and proof. |
| Cursor | Project rules are instruction context. IDE permission or sidecar behavior must be proven before any preventive claim. |

These notes are not a connector marketplace, hosted connector registry, broad connector ecosystem, or cross-surface orchestration plan. If a later profile promotes one of them, it must define exact `capability_profile` fields, fallback behavior, conformance expectations, redaction/secret handling when needed, and the rule that connector output never replaces Core authority.
