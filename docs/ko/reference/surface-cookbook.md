# Surface Cookbook 참조

## 이 문서로 할 수 있는 일

이 참조는 Codex, Claude Code, Gemini, GitHub Copilot, Cursor용 접점별 커넥터 recipe를 확인하는 데 씁니다.

이 문서는 접점마다 달라지는 로컬 설정 메모, 생성 파일 이름, MCP 설정 힌트, 캡처·가드·격리 선택지, 공통 fallback, conformance 관점의 위험을 담당합니다. 공통 커넥터 계약은 [Agent 통합 참조](agent-integration.md)가 담당합니다.

이 문서는 향후 Harness 동작을 위한 참조 문서입니다. 현재 저장소 단계와 구현 인계 상태는 [구현 개요](../build/implementation-overview.md#문서-수락-상태)에 있습니다.

## 이런 때 읽기

- 특정 agent surface의 connector recipe를 작성하거나 리뷰할 때.
- 접점별 설정을 공통 connector contract와 분리해서 유지해야 할 때.
- Capture, guard, isolation, fallback, conformance risk를 guarantee level보다 강하게 말하지 않고 설명해야 할 때.

## 읽기 전에

공통 connector contract, capability profile, 단계별 맥락 profile은 [Agent 통합 참조](agent-integration.md)를 읽습니다. Local access, API error, conformance boundary는 현재 질문에 필요한 특정 소유자 섹션만 [런타임 아키텍처](runtime-architecture.md), [MCP API와 스키마](mcp-api-and-schemas.md), [운영과 Conformance 참조](operations-and-conformance.md)에서 사용합니다. 이 cookbook은 프롬프트 묶음이 아니며 connector에게 전체 참조 문서를 불러오라고 요구하지 않습니다.

## 핵심 생각

접점 이름만으로 guarantee level을 추론하면 안 됩니다. 모든 connector는 실제 사용하는 host/profile/configuration에 대한 capability profile을 선언해야 하며, profile이 입증한 capability가 guarantee level을 결정합니다.

Generic capability profile 예시는 [Agent 통합 참조](agent-integration.md#capability-profile-예시)를 봅니다.

Surface recipe는 로컬 접근용 public error code나 OS 수준 보안 보장을 새로 정의하지 않습니다. Runtime, MCP API, Agent Integration의 v0.1/default reference local-only MCP posture를 그대로 따릅니다. 초기 recipe는 cooperative/detective 수준의 표현을 기본으로 삼아야 하며, preventive와 isolated option은 구체적인 profile이 구현하고 증명하기 전까지 향후 또는 profile별 범위로 남습니다. Recipe는 접점별 local transport, config snippet, setup hint, access-control material class, hook, wrapper, isolation option을 이름 붙일 수 있지만 recipe text만으로 guarantee level을 올리지 않습니다. Agent Integration capability profile과 conformance가 해당 surface가 cooperative, detective, preventive, isolated 중 무엇인지 결정합니다. Recipe는 raw token, secret, private configuration value를 노출하면 안 되며, surface name만으로 remote, shared, non-loopback, forwarded, tunneled MCP exposure가 암시되지는 않습니다. MCP access가 unavailable, stale, unknown, weak이거나 registered profile 밖에 있으면 API와 operations path를 사용합니다. 즉 diagnostic detail이 있는 `MCP_UNAVAILABLE` 또는 `CAPABILITY_INSUFFICIENT`, claim mismatch에 대한 normal state-conflict/scope/capability checks, 실제 guarantee display로 처리합니다. MCP server에 닿을 수 없으면 해당 call path에서 authoritative Core response가 없으며, cooperative recipe는 write를 instruction으로 hold하고, preventive 표현은 fixture로 입증된 guard가 operation을 cover할 때만 허용되며, isolated 표현은 connector profile이 exact separation boundary를 이름 붙이고 증명한 경우에만 허용됩니다. Surface별 `UNAUTHORIZED` public code를 도입하지 않습니다. 현재 reference API는 local-access profile mismatch에 대해 `UNAUTHORIZED` code를 추가하지 않습니다.

## Recipe shape

각 recipe에는 접점별 내용만 둡니다.

- 해당 접점에서 가능한 target profile
- 생성 파일 또는 instruction
- MCP 설정 힌트
- 접점에 따라 달라지는 MCP exposure posture와 local transport 전제
- version, hook, permission, workspace policy, generated file, capture method, QA capture method, redaction policy, artifact retention behavior, conformance result 차이가 [Agent 통합 참조](agent-integration.md#capability-profiles)에 따라 profile refresh를 요구하는 host/profile별 capability 차이
- 캡처·가드·격리 선택지
- 접점별 맥락 전략: 항상 주입되는 운영 맥락을 역할, 현재 단계와 맥락 프로필, 현재 상태 요약, 활성 blocker, 대기 중인 사용자 소유 판단, 다음 허용 행동으로 제한하는 방법과 세션 시작, 요구사항 구체화 (Discovery), 사용자 판단 요청, 쓰기 준비, 실행/근거, 닫기 준비 상태, 복구/오류 프로필 사이를 전환하는 방법, 그리고 [Agent 통합: Context Push/Pull Principles](agent-integration.md#context-pushpull-principles)에 따라 각 프로필에서 불러올 최소 소유자 섹션
- 보장 경계 설명: 해당 접점이 실행 전에 막을 수 있는 것, 실행 뒤에만 감지할 수 있는 것, native capture로 처리되는 것, manual artifact 또는 manual verification bundle로 fallback하는 것
- 공통 fallback
- conformance 관점의 위험

Generic kernel rule, public API schema, policy contract, 전체 참조 문서, 관련 없는 template, historical log, 전체 Storage DDL, 전체 Conformance 카탈로그, 관련 없는 Roadmap 항목, 읽기용 요약 전체 본문을 여기서 반복하지 않습니다. 공통 contract가 cooperative, detective, preventive, isolated의 의미를 정합니다. Recipe는 그 동작을 제공할 수 있는 접점별 path만 이름 붙입니다. Guard, freeze, careful-mode label은 connected profile의 실제 capability 위에 얹힌 label로만 쓸 수 있습니다. Recipe가 이런 label을 쓰면 그 동작이 범위 보류인지, 사후 감지인지, fixture로 입증된 도구 실행 전 차단인지, 문서화된 separation boundary인지 밝혀야 합니다. 이런 label은 write를 authorize하거나, gate를 충족하거나, verification을 기록하거나, acceptance를 기록하거나, 새 authority tier를 만들지 않습니다.

Generated 또는 managed recipe output은 [Agent 통합 참조](agent-integration.md#generated-manifest-기대사항)의 connector manifest contract를 따라야 합니다. Recipe는 접점별 file, config snippet, managed block을 이름 붙일 수 있지만 drift는 overwrite 전에 보고합니다. Existing file 또는 managed block은 reconcile 또는 explicit reconnect decision이 replacement를 선택하기 전까지 그대로 두며, drift된 generated file은 canonical Task state로 취급하지 않습니다.

아래 `guarantee_boundary` block은 recipe 문서용 설명일 뿐 public schema, DDL shape, canonical Capability Profile field가 아닙니다. Connector는 [Agent 통합 참조](agent-integration.md) contract에 맞는 경우에만 같은 사실을 Capability Profile 또는 Connector Manifest에 기록할 수 있습니다. Surface Cookbook은 접점별 path와 예시를 이름 붙일 뿐 guarantee level을 다시 정의하지 않습니다.

Recipe가 `fallback_isolation` 아래에 manual verification bundle을 적을 때는 verification/evaluator 입력으로 쓰는 fallback이라는 뜻입니다. Manual verification bundle만으로 연결된 surface가 `preventive` 또는 `isolated`로 올라가지 않습니다. `isolated` guarantee에는 여전히 문서화되고 입증된 separation boundary가 필요합니다. Worktree 또는 fresh bundle은 verification independence나 stale-context control을 뒷받침할 수 있습니다. OS sandbox 격리, 권한 격리, hard process/container isolation, 변조 불가능한 보안 경계는 connector profile이 exact mechanism을 이름 붙이고 증명해야 합니다. Isolation은 Approval, QA, 작업 수락, 잔여 위험 수용, close, verification result와 분리됩니다.

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
  can_block_before_execution: concrete Codex profile에서 사용 가능하고 fixture로 입증된 wrapper, sidecar, host permission, host hook이 다루는 covered operation만 가능
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
  - 도구 실행 전 guard가 fixture로 입증되지 않으면 cooperative prepare_write discipline
  - detective changed-path validation
  - manual artifact capture
  - manual verification bundle
  - docs-authoring override only for exact pre-MVP docs allowlists
conformance_risks:
  - 도구 실행 전 guard 강도는 host environment에 따라 달라지며 fixture로 입증해야 함
  - artifact capture may need a wrapper or explicit record_run discipline
  - long AGENTS.md files can bury current Harness status and authority context
  - document rewrite sessions can sprawl without batch boundaries
```

Codex connector work에서는 `AGENTS.md`를 매 turn 훑을 수 있을 만큼 짧게 유지해야 합니다. 이것은 항상 적용되는 compass이지 procedure manual, schema reference, project history, 전체 맥락 bundle이 아닙니다. 주입되는 현재 상태 맥락은 [Agent 통합](agent-integration.md#context-pushpull-principles)의 예산에 맞춰 역할, 현재 단계와 맥락 프로필, 현재 상태 요약, 활성 blocker, 대기 중인 사용자 소유 판단, 다음 허용 행동으로 제한합니다. 절차의 깊이는 skill, command, MCP resource에 두고, 긴 reference는 현재 profile이 필요로 할 때만 가져옵니다.

Codex-facing wording은 "이 task를 이 paths로 freeze해" 또는 "실행 전에 실제로 막을 수 있는 것과 실행 뒤에만 감지할 수 있는 것을 보여줘" 같은 phrase를 보여줄 수 있습니다. Fixture로 입증된 도구 실행 전 차단이 없는 profile에서는 freeze를 cooperative 범위 보류 또는 다음 행동을 더 엄격하게 제한하는 상태와, 가능할 때 detective changed-path validation으로 설명해야 하며 preventive guard로 설명하면 안 됩니다.

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
  can_block_before_execution: 설정되어 있고 fixture로 입증된 PreToolUse hook, wrapper, sidecar, permission이 다루는 covered operation만 가능
  can_detect_after_action: 설정된 PostToolUse 또는 Stop hook을 통한 changed file, command output, log artifact, stop summary
  native_capture: 설정된 hook, wrapper, structured run summary
  fallback_capture: diff, log, screenshot, command output, QA note에 대한 manual artifact capture
  fallback_isolation: read-only evaluator, fresh worktree evaluator, manual verification bundle
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
  - hooks가 없거나 fixture로 입증되지 않았을 때 cooperative 범위 보류 또는 careful-mode instruction
conformance_risks:
  - hook behavior is version and configuration dependent
  - read-only verification profile must be tested by conformance
  - PreToolUse can claim preventive guard only for covered operations it is fixture-proven to block before execution
```

Claude Code recipe는 해당 hook이 설정되어 있고 fixture coverage가 covered operation을 실행 전에 차단할 수 있음을 입증한 경우에만 "guard"를 `PreToolUse`에 매핑할 수 있습니다. 그렇지 않으면 freeze와 careful mode는 cooperative 범위 보류 또는 다음 행동을 더 엄격하게 제한하는 instruction과 사용 가능한 post-tool capture로 남습니다.

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
  can_block_before_execution: fixture로 입증된 CLI wrapper, sidecar-controlled run, policy layer, host permission이 다루는 covered path 또는 command만 가능
  can_detect_after_action: wrapper, sidecar, validator가 active일 때 changed path, command output, artifact gap, generated-file drift
  native_capture: 설정된 CLI wrapper, sidecar, host capture
  fallback_capture: diff, log, screenshot, command output, QA note에 대한 manual artifact capture
  fallback_isolation: isolated evaluator bundle 또는 manual verification bundle
capture_guard_isolation_options:
  - CLI wrapper for command and artifact capture
  - sidecar-controlled run for covered paths and commands
  - native capture가 없을 때 manual artifact capture
  - 수동 QA note artifact when browser capture is unavailable
  - isolated evaluator bundle when host capture is weak
common_fallbacks:
  - CLI wrapper
  - sidecar-controlled run
  - manual artifact capture
  - 수동 QA note artifact
  - manual verification bundle
  - cooperative hold or narrowed boundary when only extension wording is available
conformance_risks:
  - extension context can become too large
  - capture and guard behavior varies by host profile and must be fixture-proven for covered operations
  - extension wording alone must not be reported as a guard
```

Gemini connector는 단계별 맥락에 필요한 compact context만 전달하는 편이 좋습니다. 예를 들면 current Harness status, active Decision Packet summary, Autonomy Boundary summary, Change Unit scope, close 근처의 residual-risk summary입니다. Longer standard, domain language, module map, interface contract, schema, template는 현재 profile이 필요로 할 때만 MCP resource로 가져오게 합니다.

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
  can_block_before_execution: fixture로 입증된 VS Code task wrapper, terminal wrapper, sidecar, host permission, cloud policy path가 다루는 covered operation만 가능
  can_detect_after_action: wrapper, sidecar, validator가 active일 때 task output, changed file, command log, artifact gap, generated-file drift
  native_capture: 설정된 VS Code task, terminal wrapper, sidecar, profile-specific capture
  fallback_capture: diff, log, screenshot, command output, QA note에 대한 manual artifact capture
  fallback_isolation: fresh worktree/evaluator profile 또는 manual verification bundle
capture_guard_isolation_options:
  - VS Code task wrapper for owned task capture
  - sidecar adapter for changed-file or command observation
  - task, wrapper, sidecar capture가 없을 때 manual artifact capture
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
  - 사용자에게 보이는 freeze card는 allowed paths와 실행 전에 실제로 막을 수 있는 것/실행 뒤에만 감지할 수 있는 것을 보여줘야 함
```

Copilot recipe는 IDE behavior와 cloud behavior를 흐리지 않아야 합니다. VS Code task wrapper는 fixture coverage가 그 동작을 입증한 경우 자신이 소유하는 task에 대해 detective capture 또는 preventive blocking을 지원할 수 있지만, chat instruction만으로는 cooperative입니다.

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
  can_block_before_execution: fixture로 입증된 IDE permission support, wrapper, sidecar, policy path가 다루는 covered operation만 가능
  can_detect_after_action: sidecar 또는 validator가 active일 때 changed file, generated-file drift, artifact gap, validator finding
  native_capture: 설정된 sidecar, wrapper, IDE capture
  fallback_capture: diff, log, screenshot, command output, QA note에 대한 manual artifact capture
  fallback_isolation: manual verification bundle 또는 설정된 fresh worktree/evaluator profile
capture_guard_isolation_options:
  - sidecar changed-file detection
  - generated file drift detection
  - IDE permission support when available and fixture-proven
  - native capture가 없을 때 manual artifact capture
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

Cursor connector는 project rule을 짧게 유지하고, 절차의 깊이와 단계별 맥락 retrieval은 skill/playbook과 MCP로 제공해야 합니다. Project rule 문구만으로는 cooperative 수준입니다. preventive guard 동작을 주장하려면 IDE permission이나 sidecar에 대한 fixture 증명이 필요합니다.
