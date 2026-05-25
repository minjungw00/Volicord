# Surface Cookbook 참조

## 이 문서로 할 수 있는 일

이 참조는 Codex, Claude Code, Gemini, GitHub Copilot, Cursor용 접점별 커넥터 recipe를 확인하는 데 씁니다.

이 문서는 접점마다 달라지는 로컬 설정 메모, 생성 파일 이름, MCP 설정 힌트, 캡처·가드·격리 선택지, 공통 fallback, conformance 관점의 위험을 담당합니다. 공통 커넥터 계약은 [Agent 통합 참조](agent-integration.md)가 담당합니다.

접점 이름만으로 guarantee level을 추론하면 안 됩니다. 모든 connector는 여전히 capability profile을 선언해야 하며, profile이 입증한 capability가 guarantee level을 결정합니다.

Generic capability profile 예시는 [Agent 통합 참조](agent-integration.md#capability-profile-예시)를 봅니다.

Surface recipe는 로컬 접근용 error code나 OS 수준 보안 보장을 새로 정의하지 않습니다. MCP access가 unavailable, stale, unknown, weak이거나 registered profile 밖에 있으면 API와 operations path를 사용합니다. 즉 diagnostic detail이 있는 `MCP_UNAVAILABLE` 또는 `CAPABILITY_INSUFFICIENT`, claim mismatch에 대한 normal state-conflict/scope/capability checks, 실제 guarantee display로 처리합니다. Surface별 MVP `UNAUTHORIZED` code를 도입하지 않습니다.

## Recipe shape

각 recipe에는 접점별 내용만 둡니다.

- 해당 접점에서 가능한 target profile
- 생성 파일 또는 instruction
- MCP 설정 힌트
- 캡처·가드·격리 선택지
- 보장 경계 설명: 해당 접점이 실행 전에 막을 수 있는 것, 실행 뒤에만 감지할 수 있는 것, native capture로 처리되는 것, manual artifact 또는 manual verification bundle로 fallback하는 것
- 공통 fallback
- conformance 관점의 위험

Generic kernel rule, public API schema, policy contract를 여기서 반복하지 않습니다. 공통 contract가 cooperative, detective, preventive, isolated의 의미를 정합니다. Recipe는 그 동작을 제공할 수 있는 접점별 path만 이름 붙입니다. Guard, freeze, careful-mode label은 connected profile의 실제 capability 위에 얹힌 label로만 쓸 수 있습니다. Recipe가 이런 label을 쓰면 그 동작이 범위 보류인지, 사후 감지인지, 입증된 실행 전 차단인지, isolation인지 밝혀야 합니다.

아래 `guarantee_boundary` block은 recipe 문서용 설명일 뿐 public schema, DDL shape, canonical Capability Profile field가 아닙니다. Connector는 [Agent 통합 참조](agent-integration.md) contract에 맞는 경우에만 같은 사실을 Capability Profile 또는 Connector Manifest에 기록할 수 있습니다. Surface Cookbook은 접점별 path와 예시를 이름 붙일 뿐 guarantee level을 다시 정의하지 않습니다.

Recipe가 `fallback_isolation` 아래에 manual verification bundle을 적을 때는 verification/evaluator 입력으로 쓰는 fallback이라는 뜻입니다. Manual verification bundle만으로 연결된 surface가 `preventive` 또는 `isolated`로 올라가지 않습니다. `isolated` guarantee에는 여전히 입증된 별도 worktree, sandbox, process boundary, read-only bundle 또는 동등한 independence/isolation boundary가 필요합니다.

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
  - generated MCP config path, managed hash, profile 최신성을 connector manifest에 기록
guarantee_boundary:
  default_level: AGENTS.md, skill, command wording만으로는 cooperative
  can_block_before_execution: concrete Codex profile에서 사용 가능하고 입증된 wrapper, sidecar, host permission, host hook이 다루는 covered operation만 가능
  can_detect_after_action: validator 또는 sidecar가 active일 때 changed path, run/artifact gap, generated-file drift
  native_capture: 설정된 wrapper 또는 explicit record_run discipline
  fallback_capture: diff, log, screenshot, command output, QA note에 대한 manual artifact capture
  fallback_isolation: 설정된 manual verification bundle 또는 fresh worktree/evaluator profile
capture_guard_isolation_options:
  - sidecar changed-file watcher
  - changed_paths validator
  - wrapper or explicit record_run discipline for command output and artifacts
  - wrapper 또는 structured capture가 없을 때 manual artifact capture
  - fresh evaluator support가 없을 때 manual verification bundle
common_fallbacks:
  - cooperative prepare_write discipline unless pre-tool guard is proven
  - detective changed-path validation
  - manual artifact capture
  - manual verification bundle
  - docs-authoring override only for exact pre-MVP docs allowlists
conformance_risks:
  - pre-tool guard strength depends on host environment and must be proven
  - artifact capture may need a wrapper or explicit record_run discipline
  - long AGENTS.md files can bury current Harness status and authority context
  - document rewrite sessions can sprawl without batch boundaries
```

Codex connector work에서는 `AGENTS.md`를 매 turn 훑을 수 있을 만큼 짧게 유지해야 합니다. 이것은 always-on compass이지 procedure manual, schema reference, project history가 아닙니다. 절차의 깊이는 skill, command, MCP resource에 둡니다.

Codex-facing wording은 "이 task를 이 paths로 freeze해" 또는 "실행 전에 실제로 막을 수 있는 것과 실행 뒤에만 감지할 수 있는 것을 보여줘" 같은 phrase를 보여줄 수 있습니다. Proven pre-tool blocking이 없는 profile에서는 freeze를 cooperative 범위 보류 또는 다음 행동을 더 엄격하게 제한하는 상태와, 가능할 때 detective changed-path validation으로 설명해야 하며 preventive guard로 설명하면 안 됩니다.

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
  - hook path, MCP generated path, managed hash, profile 최신성을 connector manifest에 기록
guarantee_boundary:
  default_level: CLAUDE.md 또는 skill wording만으로는 cooperative
  can_block_before_execution: 설정되어 있고 conformance로 입증된 PreToolUse hook, wrapper, sidecar, permission이 다루는 covered operation만 가능
  can_detect_after_action: 설정된 PostToolUse 또는 Stop hook을 통한 changed file, command output, log artifact, stop summary
  native_capture: 설정된 hook, wrapper, structured run summary
  fallback_capture: diff, log, screenshot, command output, QA note에 대한 manual artifact capture
  fallback_isolation: read-only evaluator, fresh worktree evaluator, manual verification bundle
capture_guard_isolation_options:
  - SessionStart hook for Journey Card or status card injection
  - UserPromptSubmit hook for intake and shaping guidance
  - PreToolUse hook for covered edit, command, network, or secret guard when configured and proven
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
  - hooks가 없거나 입증되지 않았을 때 cooperative 범위 보류 또는 careful-mode instruction
conformance_risks:
  - hook behavior is version and configuration dependent
  - read-only verification profile must be tested by conformance
  - PreToolUse can claim preventive guard only for covered operations it is proven to block
```

Claude Code recipe는 해당 hook이 설정되어 있고 conformance가 covered operation을 실행 전에 차단할 수 있음을 입증한 경우에만 "guard"를 `PreToolUse`에 매핑할 수 있습니다. 그렇지 않으면 freeze와 careful mode는 cooperative 범위 보류 또는 다음 행동을 더 엄격하게 제한하는 instruction과 사용 가능한 post-tool capture로 남습니다.

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
  - extension, wrapper, sidecar, MCP generated path, managed hash, profile 최신성을 connector manifest에 기록
guarantee_boundary:
  default_level: extension 또는 prompt package wording만으로는 cooperative
  can_block_before_execution: 입증된 CLI wrapper, sidecar-controlled run, policy layer, host permission이 다루는 covered path 또는 command만 가능
  can_detect_after_action: wrapper, sidecar, validator가 active일 때 changed path, command output, artifact gap, generated-file drift
  native_capture: 설정된 CLI wrapper, sidecar, host capture
  fallback_capture: diff, log, screenshot, command output, QA note에 대한 manual artifact capture
  fallback_isolation: isolated evaluator bundle 또는 manual verification bundle
capture_guard_isolation_options:
  - CLI wrapper for command and artifact capture
  - sidecar-controlled run for covered paths and commands
  - native capture가 없을 때 manual artifact capture
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
  - capture and guard behavior varies by host profile and must be proven for covered operations
  - extension wording alone must not be reported as a guard
```

Gemini connector는 current Harness status, active Decision Packet summary, Autonomy Boundary summary, Change Unit scope, close 근처의 residual-risk summary만 전달하는 편이 좋습니다. Longer standard, domain language, module map, interface contract는 MCP resource로 가져오게 합니다.

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
  - generated custom instruction, task, wrapper, MCP path, managed hash, profile 최신성을 connector manifest에 기록
guarantee_boundary:
  default_level: custom instruction 또는 chat wording만으로는 cooperative
  can_block_before_execution: 입증된 VS Code task wrapper, terminal wrapper, sidecar, host permission, cloud policy path가 다루는 covered operation만 가능
  can_detect_after_action: wrapper, sidecar, validator가 active일 때 task output, changed file, command log, artifact gap, generated-file drift
  native_capture: 설정된 VS Code task, terminal wrapper, sidecar, profile-specific capture
  fallback_capture: diff, log, screenshot, command output, QA note에 대한 manual artifact capture
  fallback_isolation: fresh worktree/evaluator profile 또는 manual verification bundle
capture_guard_isolation_options:
  - VS Code task wrapper for owned task capture
  - sidecar adapter for changed-file or command observation
  - task, wrapper, sidecar capture가 없을 때 manual artifact capture
  - profile-specific guard only when the host is proven to block covered operations
  - explicit approval card display for sensitive changes
common_fallbacks:
  - VS Code task wrapper
  - sidecar adapter
  - manual artifact capture
  - manual verification bundle
  - explicit approval card
  - cooperative chat instruction for profiles without wrapper or sidecar support
conformance_risks:
  - cloud and IDE profiles may differ materially
  - write guard coverage and artifact capture need profile-specific verification
  - 사용자에게 보이는 freeze card는 allowed paths와 실행 전에 실제로 막을 수 있는 것/실행 뒤에만 감지할 수 있는 것을 보여줘야 함
```

Copilot recipe는 IDE behavior와 cloud behavior를 흐리지 않아야 합니다. VS Code task wrapper는 profile이 그 동작을 입증한 경우 자신이 소유하는 task에 대해 detective capture 또는 preventive blocking을 지원할 수 있지만, chat instruction만으로는 cooperative입니다.

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
  - generated rule, sidecar, MCP path, managed hash, profile 최신성을 connector manifest에 기록
guarantee_boundary:
  default_level: Cursor project-rule wording만으로는 cooperative
  can_block_before_execution: 입증된 IDE permission support, wrapper, sidecar, policy path가 다루는 covered operation만 가능
  can_detect_after_action: sidecar 또는 validator가 active일 때 changed file, generated-file drift, artifact gap, validator finding
  native_capture: 설정된 sidecar, wrapper, IDE capture
  fallback_capture: diff, log, screenshot, command output, QA note에 대한 manual artifact capture
  fallback_isolation: manual verification bundle 또는 설정된 fresh worktree/evaluator profile
capture_guard_isolation_options:
  - sidecar changed-file detection
  - generated file drift detection
  - IDE permission support when available and proven
  - native capture가 없을 때 manual artifact capture
  - manual verification bundle
common_fallbacks:
  - sidecar changed-file detection
  - generated file drift detection
  - manual artifact capture
  - manual verification bundle
  - cooperative project-rule instruction when IDE permissions are unproven
conformance_risks:
  - project rules can become too verbose
  - guard coverage depends on IDE profile and proven permissions
  - generated project rules must become reconcile candidates when locally edited
```

Cursor connector는 project rule을 짧게 유지하고, 절차의 깊이는 skill/playbook과 MCP로 제공해야 합니다. Project rule 문구만으로는 cooperative 수준입니다. preventive guard 동작을 주장하려면 IDE permission이나 sidecar proof가 필요합니다.
