# Surface Cookbook Reference

## What this document helps you do

Use this reference to check surface-specific connector recipes for Codex, Claude Code, Gemini, GitHub Copilot, and Cursor.

This document owns local setup notes, generated file names, MCP configuration hints, capture/guard/isolation options, common fallbacks, and conformance risks that vary by surface. The common connector contract lives in [Agent Integration Reference](agent-integration.md).

This is reference documentation. It does not authorize runtime/server implementation, generated operational files, executable fixtures, or runtime data before the documentation set is accepted for implementation planning. The first implementation/proof target remains Kernel Smoke; Agency-Hardened MVP and post-MVP automation stay out of scope unless their owner docs promote and prove them.

## Read this when

- You are writing or reviewing a connector recipe for a specific agent surface.
- You need to keep surface-specific setup separate from the common connector contract.
- You need to describe capture, guard, isolation, fallback, or conformance risks without overstating guarantee level.

## Before you read

Read [Agent Integration Reference](agent-integration.md) for the common connector contract and capability profiles. Use [Runtime Architecture](runtime-architecture.md), [MCP API And Schemas](mcp-api-and-schemas.md), and [Operations And Conformance Reference](operations-and-conformance.md) for local access, API errors, and conformance boundaries.

## Main idea

A surface name never implies a guarantee level. Every connector still declares a capability profile for the actual host/profile/configuration in use, and the profile's proven capabilities determine the guarantee level.

For generic capability profile examples, see [Agent Integration Reference](agent-integration.md#capability-profile-examples).

Surface recipes do not define local-access error codes or OS-level security guarantees. They inherit the MVP local-only MCP default from Runtime, MCP API, and Agent Integration. A recipe may name the surface-specific local transport, config snippet, or access-control material class, but it must not expose raw token, secret, or private configuration values, and remote or shared MCP exposure is never implied by the surface name. If MCP access is unavailable, stale, unknown, weak, or outside the registered profile, route through the API and operations paths: `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` with diagnostic detail, normal state-conflict/scope/capability checks for mismatched claims, and honest guarantee display. If the MCP server cannot be reached, no authoritative Core response is available from that call path; cooperative recipes hold writes by instruction, and stronger wording is allowed only when a fixture-proven guard or isolation boundary actually covers the operation. Do not introduce a surface-specific MVP `UNAUTHORIZED` code.

## Recipe shape

Each recipe should keep only surface-specific material:

- target profiles that are plausible for the surface
- generated files or instructions
- MCP configuration hints
- MCP exposure posture and local transport assumptions when they vary by surface
- host/profile-specific capability differences, including version, hooks, permissions, workspace policy, generated files, capture methods, QA capture methods, redaction policy, artifact retention behavior, and conformance result differences that require a refreshed profile under the [Agent Integration Reference](agent-integration.md#capability-profiles)
- capture, guard, and isolation options
- guarantee boundary notes: what the named surface can block before execution, what it can only detect after action, what capture is native, and what falls back to manual artifacts or a manual verification bundle
- common fallbacks
- conformance risks

Do not repeat generic kernel rules, public API schemas, or policy contracts here. The common contract determines what cooperative, detective, preventive, and isolated mean. A recipe only names the surface-specific path that can provide that behavior. Guard, freeze, and careful-mode labels may appear only as labels over the connected profile's actual capability. When a recipe uses one of those labels, it must say whether the behavior is a scope hold, a post-action detector, a fixture-proven pre-tool block, or isolation. Those labels do not authorize writes, satisfy gates, record verification, record acceptance, or create a new authority tier.

Generated or managed recipe outputs must follow the connector manifest contract in [Agent Integration Reference](agent-integration.md#generated-manifest-expectations). Recipes may name the surface-specific files, config snippets, or managed blocks, but drift is reported before overwrite. The existing file or managed block stays in place until reconcile or an explicit reconnect decision chooses replacement, and the drifted generated file is not treated as canonical Task state.

The `guarantee_boundary` blocks below are recipe documentation notes, not public schema, DDL shape, or canonical Capability Profile fields. A connector may record equivalent facts in its Capability Profile or Connector Manifest only according to the [Agent Integration Reference](agent-integration.md) contract. Surface Cookbook names surface-specific paths and examples; it does not redefine guarantee levels.

When a recipe lists a manual verification bundle under `fallback_isolation`, read it as verification/evaluator fallback input. A manual verification bundle does not by itself upgrade the connected surface to `preventive` or `isolated`. An `isolated` guarantee still requires a proven separate worktree, sandbox, process boundary, read-only bundle, or equivalent independence/isolation boundary, and it remains separate from Approval, QA, acceptance, and verification results.

## Codex

```yaml
surface_kind: codex
target_profiles:
  - local_cli
  - ide_chat
  - custom_agent
generated_files_or_instructions:
  - AGENTS.md or a managed Harness section inside AGENTS.md
  - local skill or command instructions when supported
  - MCP config snippet
  - connector manifest entry
mcp_configuration_hints:
  - prefer direct MCP tool calls for T2 or higher profiles
  - record generated MCP config paths, managed hashes, and profile freshness in the connector manifest
guarantee_boundary:
  default_level: cooperative for AGENTS.md, skill, or command wording alone
  can_block_before_execution: only covered operations through a wrapper, sidecar, host permission, or host hook that is available and fixture-proven for the concrete Codex profile
  can_detect_after_action: changed paths, run/artifact gaps, and generated-file drift when validators or sidecars are active
  native_capture: wrapper or explicit record_run discipline when configured
  fallback_capture: manual artifact capture for diffs, logs, screenshots, command output, and QA notes
  fallback_isolation: manual verification bundle or fresh worktree/evaluator profile when configured
capture_guard_isolation_options:
  - sidecar changed-file watcher
  - changed_paths validator
  - wrapper or explicit record_run discipline for command output and artifacts
  - manual artifact capture when wrapper or structured capture is unavailable
  - manual verification bundle when fresh evaluator support is unavailable
common_fallbacks:
  - cooperative prepare_write discipline unless pre-tool guard is fixture-proven
  - detective changed-path validation
  - manual artifact capture
  - manual verification bundle
  - docs-authoring override only for exact pre-MVP docs allowlists
conformance_risks:
  - pre-tool guard strength depends on host environment and must be fixture-proven
  - artifact capture may need a wrapper or explicit record_run discipline
  - long AGENTS.md files can bury current Harness status and authority context
  - document rewrite sessions can sprawl without batch boundaries
```

Codex connector work should keep `AGENTS.md` short enough to scan every turn. Treat it as an always-on compass, not a procedure manual, schema reference, or project history. Put procedural depth in a skill, command, or MCP resource.

Codex-facing wording may expose phrases such as "freeze this task to these paths" or "show what can actually be blocked and what can only be detected later." For profiles without fixture-proven pre-tool blocking, describe freeze as a cooperative scope hold or stricter next-action posture plus detective changed-path validation when available, not as preventive guard.

## Claude Code

```yaml
surface_kind: claude_code
target_profiles:
  - local_cli
  - ide_chat
  - custom_agent
generated_files_or_instructions:
  - CLAUDE.md or managed Harness section inside CLAUDE.md
  - skill-style procedure files when supported
  - hook configuration snippets
  - MCP config snippet
  - connector manifest entry
mcp_configuration_hints:
  - keep MCP tool and resource availability explicit per host profile
  - record hook paths, MCP generated paths, managed hashes, and profile freshness in the connector manifest
guarantee_boundary:
  default_level: cooperative for CLAUDE.md or skill wording alone
  can_block_before_execution: only covered operations through configured and fixture-proven PreToolUse hooks, wrappers, sidecars, or permissions
  can_detect_after_action: changed files, command output, log artifacts, and stop summaries through PostToolUse or Stop hooks when configured
  native_capture: hook, wrapper, or structured run summary when configured
  fallback_capture: manual artifact capture for diffs, logs, screenshots, command output, and QA notes
  fallback_isolation: read-only evaluator, fresh worktree evaluator, or manual verification bundle
capture_guard_isolation_options:
  - SessionStart hook for Journey Card or status card injection
  - UserPromptSubmit hook for intake and shaping guidance
  - PreToolUse hook for covered edit, command, network, or secret guard when configured and fixture-proven
  - PostToolUse hook for changed files, command output, and log artifact candidates
  - Stop hook for run summary and verify/QA needs
  - PreCompact hook for Task summary and artifact refs
  - read-only evaluator or fresh worktree evaluator profile
common_fallbacks:
  - read-only evaluator profile
  - fresh worktree evaluator
  - manual artifact capture
  - manual verification bundle
  - stop-hook report draft
  - cooperative scope hold or careful-mode instruction when hooks are absent or not fixture-proven
conformance_risks:
  - hook behavior is version and configuration dependent
  - read-only verification profile must be tested by conformance
  - PreToolUse can claim preventive guard only for covered operations it is fixture-proven to block before execution
```

Claude Code recipes may map "guard" to `PreToolUse` only when that hook is configured and fixture coverage proves it can block the covered operation before execution. Otherwise, freeze and careful mode remain cooperative scope-hold or stricter next-action instructions plus any available post-tool capture.

## Gemini

```yaml
surface_kind: gemini
target_profiles:
  - local_cli
  - extension
  - ide_chat
  - custom_agent
generated_files_or_instructions:
  - extension instruction package or prompt package
  - local CLI wrapper instructions when applicable
  - sidecar configuration when used
  - MCP config snippet
  - connector manifest entry
mcp_configuration_hints:
  - keep extension context small and let the agent pull longer references through MCP resources
  - record extension, wrapper, sidecar, MCP generated paths, managed hashes, and profile freshness in the connector manifest
guarantee_boundary:
  default_level: cooperative for extension or prompt package wording alone
  can_block_before_execution: only covered paths or commands through a fixture-proven CLI wrapper, sidecar-controlled run, policy layer, or host permission
  can_detect_after_action: changed paths, command output, artifact gaps, and generated-file drift when wrapper, sidecar, or validators are active
  native_capture: CLI wrapper, sidecar, or host capture when configured
  fallback_capture: manual artifact capture for diffs, logs, screenshots, command output, and QA notes
  fallback_isolation: isolated evaluator bundle or manual verification bundle
capture_guard_isolation_options:
  - CLI wrapper for command and artifact capture
  - sidecar-controlled run for covered paths and commands
  - manual artifact capture when native capture is unavailable
  - Manual QA note artifact when browser capture is unavailable
  - isolated evaluator bundle when host capture is weak
common_fallbacks:
  - CLI wrapper
  - sidecar-controlled run
  - manual artifact capture
  - Manual QA note artifact
  - manual verification bundle
  - cooperative hold or narrowed boundary when only extension wording is available
conformance_risks:
  - extension context can become too large
  - capture and guard behavior varies by host profile and must be fixture-proven for covered operations
  - extension wording alone must not be reported as a guard
```

Gemini connectors should push only current Harness status, active Decision Packet summary, Autonomy Boundary summary, Change Unit scope, and residual-risk summary near close. Longer standards, domain language, module maps, and interface contracts should be pulled through MCP resources.

## GitHub Copilot

```yaml
surface_kind: github_copilot
target_profiles:
  - vscode_chat
  - vscode_agent
  - cloud_agent
  - custom_agent
generated_files_or_instructions:
  - workspace custom instructions
  - VS Code task or terminal wrapper configuration
  - approval card display instructions when supported
  - MCP config snippet for MCP-capable profiles
  - connector manifest entry
mcp_configuration_hints:
  - distinguish VS Code local profiles from cloud profiles
  - prefer task or terminal wrappers when command output must become a run artifact
  - record generated custom instruction, task, wrapper, MCP paths, managed hashes, and profile freshness in the connector manifest
guarantee_boundary:
  default_level: cooperative for custom instruction or chat wording alone
  can_block_before_execution: only covered operations through a fixture-proven VS Code task wrapper, terminal wrapper, sidecar, host permission, or cloud policy path
  can_detect_after_action: task output, changed files, command logs, artifact gaps, and generated-file drift when wrapper, sidecar, or validators are active
  native_capture: VS Code task, terminal wrapper, sidecar, or profile-specific capture when configured
  fallback_capture: manual artifact capture for diffs, logs, screenshots, command output, and QA notes
  fallback_isolation: fresh worktree/evaluator profile or manual verification bundle
capture_guard_isolation_options:
  - VS Code task wrapper for owned task capture
  - sidecar adapter for changed-file or command observation
  - manual artifact capture when task, wrapper, or sidecar capture is unavailable
  - profile-specific guard only when the host is fixture-proven to block covered operations before execution
  - explicit Approval Card display for sensitive actions
common_fallbacks:
  - VS Code task wrapper
  - sidecar adapter
  - manual artifact capture
  - manual verification bundle
  - explicit Approval Card
  - cooperative chat instruction for profiles without wrapper or sidecar support
conformance_risks:
  - cloud and IDE profiles may differ materially
  - write guard coverage and artifact capture need profile-specific verification
  - user-facing freeze cards must show allowed paths and what can actually be blocked versus detected later
```

Copilot recipes should not blur IDE and cloud behavior. A VS Code task wrapper may support detective capture or preventive blocking for tasks it owns when fixture coverage proves that behavior, while chat instructions alone remain cooperative.

## Cursor

```yaml
surface_kind: cursor
target_profiles:
  - ide_agent
  - local_cli
  - custom_agent
generated_files_or_instructions:
  - Cursor project rules or managed Harness section inside project rules
  - skill/playbook instructions when supported
  - sidecar configuration when used
  - MCP config snippet
  - connector manifest entry
mcp_configuration_hints:
  - keep project rules short and use MCP resources for longer references
  - record generated rule, sidecar, MCP paths, managed hashes, and profile freshness in the connector manifest
guarantee_boundary:
  default_level: cooperative for Cursor project-rule wording alone
  can_block_before_execution: only covered operations through fixture-proven IDE permission support, wrapper, sidecar, or policy path
  can_detect_after_action: changed files, generated-file drift, artifact gaps, and validator findings when sidecar or validators are active
  native_capture: sidecar, wrapper, or IDE capture when configured
  fallback_capture: manual artifact capture for diffs, logs, screenshots, command output, and QA notes
  fallback_isolation: manual verification bundle or fresh worktree/evaluator profile when configured
capture_guard_isolation_options:
  - sidecar changed-file detection
  - generated file drift detection
  - IDE permission support when available and fixture-proven
  - manual artifact capture when native capture is unavailable
  - manual verification bundle
common_fallbacks:
  - sidecar changed-file detection
  - generated file drift detection
  - manual artifact capture
  - manual verification bundle
  - cooperative project-rule instruction when IDE permissions are not fixture-proven
conformance_risks:
  - project rules can become too verbose
  - guard coverage depends on IDE profile and fixture-proven permissions
  - generated project rules must become reconcile candidates when locally edited
```

Cursor connectors should keep project rules short and use the skill/playbook plus MCP for procedural depth. Project-rule wording alone is cooperative; IDE permission or sidecar fixture proof is required before claiming preventive guard behavior.
