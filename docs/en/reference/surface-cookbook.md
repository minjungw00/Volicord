# Surface Cookbook Reference

## What this document helps you do

Use this reference to check surface-specific connector recipes for Codex, Claude Code, Gemini, GitHub Copilot, and Cursor.

This document owns local setup notes, generated file names, MCP configuration hints, capture/guard/isolation options, common fallbacks, and conformance risks that vary by surface. The common connector contract lives in [Agent Integration Reference](agent-integration.md).

A surface name never implies a guarantee level. Every connector still declares a capability profile, and the profile's proven capabilities determine the guarantee level.

For generic capability profile examples, see [Agent Integration Reference](agent-integration.md#capability-profile-examples).

Surface recipes do not define local-access error codes or OS-level security guarantees. If MCP access is unavailable, stale, unknown, weak, or outside the registered profile, route through the API and operations paths: `MCP_UNAVAILABLE` or `CAPABILITY_INSUFFICIENT` with diagnostic detail, normal state-conflict/scope/capability checks for mismatched claims, and honest guarantee display. Do not introduce a surface-specific MVP `UNAUTHORIZED` code.

## Recipe shape

Each recipe should keep only surface-specific material:

- target profiles that are plausible for the surface
- generated files or instructions
- MCP configuration hints
- capture, guard, and isolation options
- common fallbacks
- conformance risks

Do not repeat generic kernel rules, public API schemas, or policy contracts here. Guard, freeze, and careful-mode labels may appear only as labels over the connected profile's actual cooperative, detective, preventive, or isolated capability.

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
  - record generated MCP config paths and managed hashes in the connector manifest
capture_guard_isolation_options:
  - sidecar changed-file watcher
  - changed_paths validator
  - wrapper or explicit record_run discipline for command output and artifacts
  - manual verification bundle when fresh evaluator support is unavailable
common_fallbacks:
  - cooperative prepare_write discipline unless pre-tool guard is proven
  - detective changed-path validation
  - manual verification bundle
  - docs-authoring override only for exact pre-MVP docs allowlists
conformance_risks:
  - pre-tool guard strength depends on host environment and must be proven
  - artifact capture may need a wrapper or explicit record_run discipline
  - long AGENTS.md files can bury current Harness status and authority context
  - document rewrite sessions can sprawl without batch boundaries
```

Codex connector work should keep `AGENTS.md` short enough to scan every turn. Treat it as an always-on compass, not a procedure manual, schema reference, or project history. Put procedural depth in a skill, command, or MCP resource.

Codex-facing wording may expose phrases such as "freeze this task to these paths" or "show current guard level." For profiles without proven pre-tool blocking, describe that as cooperative freeze plus detective changed-path validation when available, not as preventive guard.

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
  - record hook and MCP generated paths in the connector manifest
capture_guard_isolation_options:
  - SessionStart hook for Journey Card or status card injection
  - UserPromptSubmit hook for intake and shaping guidance
  - PreToolUse hook for covered edit, command, network, or secret guard
  - PostToolUse hook for changed files, command output, and log artifact candidates
  - Stop hook for run summary and verify/QA needs
  - PreCompact hook for Task summary and artifact refs
  - read-only evaluator or fresh worktree evaluator profile
common_fallbacks:
  - read-only evaluator profile
  - fresh worktree evaluator
  - stop-hook report draft
  - cooperative freeze or careful-mode instruction when hooks are absent or unproven
conformance_risks:
  - hook behavior is version and configuration dependent
  - read-only verification profile must be tested by conformance
  - PreToolUse can claim preventive guard only for covered operations it actually blocks
```

Claude Code recipes may map "guard" to `PreToolUse` only when that hook is configured and conformance proves it can block the covered operation before execution. Otherwise, freeze and careful mode remain cooperative instructions plus any available post-tool capture.

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
  - record extension, wrapper, sidecar, and MCP generated paths in the connector manifest
capture_guard_isolation_options:
  - CLI wrapper for command and artifact capture
  - sidecar-controlled run for covered paths and commands
  - Manual QA note artifact when browser capture is unavailable
  - isolated evaluator bundle when host capture is weak
common_fallbacks:
  - CLI wrapper
  - sidecar-controlled run
  - Manual QA note artifact
  - cooperative hold or narrowed boundary when only extension wording is available
conformance_risks:
  - extension context can become too large
  - capture and guard behavior varies by host
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
  - record generated custom instruction, task, wrapper, and MCP paths in the connector manifest
capture_guard_isolation_options:
  - VS Code task wrapper for owned task capture
  - sidecar adapter for changed-file or command observation
  - profile-specific guard only when the host can block covered operations
  - explicit approval card display for sensitive changes
common_fallbacks:
  - VS Code task wrapper
  - sidecar adapter
  - explicit approval card
  - cooperative chat instruction for profiles without wrapper or sidecar support
conformance_risks:
  - cloud and IDE profiles may differ materially
  - write guard and artifact capture need profile-specific verification
  - user-facing freeze cards must show allowed paths and actual guarantee level
```

Copilot recipes should not blur IDE and cloud behavior. A VS Code task wrapper may support detective capture or preventive blocking for tasks it owns, while chat instructions alone remain cooperative.

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
  - record generated rule, sidecar, and MCP paths in the connector manifest
capture_guard_isolation_options:
  - sidecar changed-file detection
  - generated file drift detection
  - IDE permission support when available and proven
  - manual verification bundle
common_fallbacks:
  - sidecar changed-file detection
  - generated file drift detection
  - manual verification bundle
  - cooperative project-rule instruction when IDE permissions are unproven
conformance_risks:
  - project rules can become too verbose
  - guard behavior depends on IDE profile and permissions
  - generated project rules must become reconcile candidates when locally edited
```

Cursor connectors should keep project rules short and use the skill/playbook plus MCP for procedural depth. Project-rule wording alone is cooperative; IDE permission or sidecar proof is required before claiming preventive guard behavior.
