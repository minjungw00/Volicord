# Agent 통합 참조

## 이 문서로 할 수 있는 일

이 참조는 agent 접점을 Harness에 연결할 때, 그 접점이 실제로 보장할 수 있는 수준을 과장하지 않도록 돕습니다.

이 문서는 공통 커넥터 계약을 담당합니다. Capability tier, capability profile, generated manifest 기대사항, context push/pull 원칙, fallback 의미, Role Lens 동작, reference surface 계약, connector conformance 개요를 정의합니다.

사용자에게 보이는 agent 절차는 [에이전트 세션 흐름](../use/agent-session-flow.md)을 봅니다. 접점별 설정 메모는 [Surface Cookbook](surface-cookbook.md)을 봅니다.

## 이런 때 읽기

- agent 접점용 connector를 구현하거나 검토할 때.
- 접점 capability profile을 선언하거나 점검할 때.
- 연결된 profile이 guarantee level, guard, freeze, fallback, MCP availability를 어떻게 표시해야 하는지 정할 때.
- connector conformance coverage를 작성할 때.
- 공통 contract와 surface recipe의 경계를 확인해야 할 때.

## 통합을 쉬운 말로 설명하면

Agent 접점은 사용자가 agent와 대화하는 접점입니다. Harness는 Task 상태, 쓰기 권한, 근거, verification, Manual QA, acceptance, projection, reconcile 동작을 대화 기록 밖에 두는 로컬 권한 계층입니다.

Connector는 agent에게 작고 최신인 context를 주고, 상태 변경을 Harness MCP tool로 라우팅하고, 접점이 할 수 있으면 실제로 일어난 일을 캡처하며, 연결된 profile의 실제 guarantee level을 이름 붙여야 합니다. 접점 이름만으로 해당 capability를 갖췄다고 주장하면 안 됩니다.

공통 구조는 다음과 같습니다.

```text
user conversation surface
  -> short always-on rules/context
  -> harness skill, command, or playbook
  -> harness MCP server
  -> harness Core
  -> adapter, hook, sidecar, validator, or isolation layer
```

Always-on rule은 짧게 둡니다. 언제 Harness를 쓰는지, status 또는 Journey Card를 어디서 읽는지, product write에는 `prepare_write`가 필요하다는 점, 사용자 소유 판단은 Decision Packet으로 라우팅한다는 점, status가 실행 전에 실제로 막을 수 있는 것과 실행 뒤에만 감지할 수 있는 것을 보여야 한다는 점, authoritative MCP를 사용할 수 없으면 product write를 보류한다는 점만 알려주면 충분합니다. 세션 절차 자체는 [에이전트 세션 흐름](../use/agent-session-flow.md)이 담당합니다.

## Use 문서와 이 Reference 문서의 경계

| 영역 | 담당 문서 |
|---|---|
| 사용자 세션에서 agent가 무엇을 보여주고, 묻고, 말해야 하는지 | [에이전트 세션 흐름](../use/agent-session-flow.md) |
| scope, evidence, verification, QA, residual risk, close에 대한 사용자용 설명 | [사용자 가이드](../use/user-guide.md) |
| 공통 커넥터 계약, capability profile, manifest, context model, fallback 의미, Role Lens, reference surface, conformance overview | 이 참조 |
| Codex, Claude Code, Gemini, GitHub Copilot, Cursor의 구체적인 접점별 recipe | [Surface Cookbook](surface-cookbook.md) |
| Public MCP request/response schema | [MCP API와 스키마](mcp-api-and-schemas.md) |
| Kernel state transition과 write/close rule | [커널 참조](kernel.md) |
| Runtime guarantee level 정의 | [런타임 아키텍처 참조](runtime-architecture.md#보장-수준) |

## Capability Tiers

| Tier | 의미 | 대표 capability |
|---|---|---|
| `T0 Context` | 접점이 Harness 원칙을 읽을 수 있습니다. | rules/context file |
| `T1 Skill` | 접점이 Harness 절차를 따를 수 있습니다. | skill, command, prompt, playbook |
| `T2 MCP` | 접점이 Harness tool과 resource를 호출할 수 있습니다. | MCP server connection |
| `T3 Capture` | 접점이 diff, log, run output을 신뢰할 만하게 반환할 수 있습니다. | structured output, wrapper, adapter |
| `T4 Guard` | Profile이 해당 path를 입증한 경우, 접점이 대상 out-of-scope file, command, network, secret을 실행 전에 차단하거나 중단할 수 있습니다. | hook, permission system, policy engine, sidecar |
| `T5 Isolation` | 접점이 verification 또는 risky work를 별도 경계에서 실행할 수 있습니다. | worktree, sandbox, fresh process, isolated runner |
| `T6 QA Capture` | 접점이 browser, screenshot, walkthrough, workflow-recording, Manual QA artifact를 구조화할 수 있습니다. | browser runner, screenshot capture, console/network capture, accessibility snapshot, QA note capture |

일반적인 interactive Harness 사용은 `T2` 이상에서 가장 자연스럽습니다. Reliable detached verification에는 보통 `T3` capture와 실제 independence boundary가 필요합니다. High-risk work에는 가능하면 입증된 `T4` guard 또는 `T5` isolation을 사용해야 합니다. `T6`는 UI/UX evidence를 보강하지만 Manual QA judgment를 대체하지 않으며, human QA note를 기록할 수 있다면 MVP 필수 조건은 아닙니다.

`T6 QA Capture` profile은 지원하는 capture type과 fallback 동작을 이름으로 밝혀야 합니다. Candidate capture type에는 screenshot, console log, network trace, accessibility snapshot, workflow recording이 있습니다. Captured file은 durable storage 전에 redaction과 secret/PII handling을 따라야 하며, Manual QA record 또는 feedback loop execution에 붙는 artifact ref로 등록되어야 합니다.

## Capability Profiles

Harness connector는 product 또는 surface name에서 동작을 가정하지 않고 capability profile을 사용해야 합니다.

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

Target profile 값에는 다음이 포함될 수 있습니다.

- `local_cli`
- `ide_chat`
- `ide_agent`
- `cloud_agent`
- `extension`
- `custom_agent`
- `manual_bundle`

모든 capability profile은 MCP exposure posture를 contract 수준에서 밝혀야 합니다. 정확한 field name은 connector가 소유하지만, profile은 다음 사실을 보이게 해야 합니다.

- MVP 기본값인 `local_only` 자세가 적용되는지 여부
- localhost TCP, local socket, in-process/stdio, process-scoped configuration material, 또는 이에 준하는 local IPC 같은 로컬 transport 전제
- bind scope, socket path class, process pipe/stdio, per-project token handle, process-scoped config handle, 또는 이에 준하는 local control 같은 access-control material class. raw token, secret, private configuration value는 포함하지 않습니다.
- 관련 없는 호출자가 endpoint를 사용하지 못하게 하는 access-control contract
- remote 또는 shared MCP 노출이 disabled, unsupported, 또는 profile에 의해 명시적으로 enabled 중 어디에 해당하는지
- local 범위를 넘는 노출이 있다면, owner-doc 및 conformance-promotion basis, secret/PII 처리 정책, redaction 또는 omission 동작, guarantee display, 그 노출이 권한을 조용히 올려 주지 않음을 증명하는 conformance coverage

Capability profile은 version, MCP config, hook, permission, workspace policy, generated file 또는 managed block, conformance result, capture method, QA capture method, browser test environment, redaction policy, artifact 보존 동작, access-control material class, local bind/reachability posture, isolation/guard wrapper 동작이 바뀌면 갱신해야 합니다. Beyond-local exposure는 owner docs와 conformance가 승격하기 전까지 MVP 밖에 남으며, connector prose는 이를 안전한 MVP 기본값처럼 표시하면 안 됩니다.

## Capability Profile 예시

다음은 profile shape 예시입니다. Tier 또는 예시가 구체적인 surface의 guarantee level을 자동으로 올려 주지는 않습니다. 구체적인 connector는 실제 host/profile에서 capability를 입증한 뒤에만 해당 capability를 갖췄다고 주장할 수 있습니다.

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

Integration은 [런타임 아키텍처 참조](runtime-architecture.md#보장-수준)의 guarantee level 정의를 사용하고, 이를 연결된 접점 프로필, 현재 적용 경로, fallback 선택지에 적용합니다.

이 참조는 connector 프로필이 그 level을 어떻게 보고하고 표시하는지 담당합니다. 접점 이름에서 더 강한 level을 추론하면 안 되며, 보장 수준을 Approval, verification, QA, 결과 수락, kernel gate로 취급하면 안 됩니다.

| Level | 표시 책임 |
|---|---|
| `cooperative` | 접점이 Harness 결정을 따르도록 지시받지만, Harness가 실행 전 물리적 차단을 주장하지 않음을 보여줍니다. |
| `detective` | Harness가 실행 뒤에 changed path, log, artifact, projection drift를 관찰하고 상태를 `stale`, `blocked`, `partial`, `failed`로 표시할 수 있음을 보여줍니다. 이를 prevention이 아니라 detection으로 표시해야 합니다. |
| `preventive` | 실행 전에 차단할 수 있음이 입증된 hook, wrapper, permission layer, policy engine, sidecar path와 대상 operation을 보여줍니다. |
| `isolated` | Risky work 또는 verification에 쓰는 별도 worktree, sandbox, process, evaluator bundle 또는 동등한 경계를 보여줍니다. |

Guard, freeze, careful-mode label은 실제 profile 위에 얹힌 safety-control label입니다. 표시할 때는 실행 전에 실제로 막을 수 있는 것과 실행 뒤에만 감지할 수 있는 것을 나눠야 합니다.

| 사용자 표현 | 실제 경계 |
|---|---|
| Freeze | 현재 work 주변의 눈에 보이는 범위 보류 또는 다음 행동을 더 엄격하게 제한하는 상태입니다. Cooperative 또는 detective profile에서는 실행 전 강제 차단이 아닙니다. Persistent owner-record change는 여전히 normal Core path를 거칩니다. |
| Guard | 입증된 profile과 현재 적용 경로에 따른 cooperative, detective, preventive, isolated protection입니다. Preventive 표현은 입증된 실행 전 차단이 있는 covered operation에만 씁니다. |
| Careful mode | 더 엄격한 `prepare_write`, scope, evidence, status refresh, user-question posture입니다. 새로운 authority tier가 아니며 그 자체로 차단하지 않습니다. |

## Generated Manifest 기대사항

Connector는 rule, skill, MCP config snippet, prompt, local adapter file을 생성할 수 있습니다. 생성되거나 managed되는 모든 path, managed block, MCP config snippet, profile freshness marker는 connector manifest에 기록해야 합니다.

Manifest는 다음을 해야 합니다.

- MCP config snippet과 local adapter file을 포함한 generated/managed path 이름 기록
- managed block id와 hash 기록
- generated 당시 사용한 capability profile 기록. 여기에는 `capability_profile_version`, `detected_version`, `last_verified_at`, 그리고 그 profile을 최신으로 만든 conformance result 또는 operator check가 포함됩니다.
- 대상 접점 프로필과 MCP tool/resource scope 기록
- raw token, secret, private configuration value, omitted secret value, blocked payload bytes를 저장하지 않고 MCP exposure posture, access-control material class, bind/reachability posture, profile freshness basis, 필요할 때 display-safe handle 또는 fingerprint 기록
- profile이 입증한 범위를 넘지 않도록 configured capture, QA capture, guard, isolation mechanism 기록
- native capture 또는 isolation이 없을 때 manual artifact capture와 manual verification bundle fallback 기록
- creation/update time 기록
- surface version, MCP config, hook, permission, wrapper, sidecar, managed file, capture method, redaction policy, retention behavior가 바뀌면 profile 또는 generated block을 stale로 표시
- 사람의 편집을 덮어쓰기 전에 drift 탐지
- 필요하면 drift를 reconcile로 라우팅

Manifest concept은 공통입니다. 접점별 생성 파일 이름은 [Surface Cookbook](surface-cookbook.md)이 담당합니다.

## Context Push/Pull Principles

Implementation agent에게는 매 turn마다 compact always-on Harness context envelope를 주고, 긴 reference는 필요할 때만 가져오게 해야 합니다. 이 envelope는 history가 아니라 operational state입니다. id, 한 줄 summary, freshness marker를 사용해야 하며, 한 화면 안팎으로 유지하는 것은 유용한 guidance이지 schema limit은 아닙니다.

사용 가능하면 매 turn push하는 것:

| Envelope item | Push shape |
|---|---|
| Active Task | Task id, title, mode, lifecycle phase. |
| Next safe action | 다음 action과, 막힌 경우 가장 작은 unblocker. |
| Active Change Unit | In-scope work, out-of-bounds area, active Autonomy Boundary의 한 줄 summary. |
| Blocking decisions | Decision Packet id와 한 줄 question, 또는 `none`. |
| Write authority | Not requested, allowed, blocked, stale, unavailable 같은 display status와, relevant한 경우 scoped path/tool summary. |
| Guarantee level | 실제 연결 profile level과 입증 가능한 guard 또는 detection behavior. 접점 이름에서 추론하지 않습니다. |
| Connector profile freshness | Connector manifest ref, `capability_profile_version`, `last_verified_at`, 그리고 generated file, MCP config, hook, wrapper, sidecar, capture, isolation 동작이 바뀐 경우 stale reason. |
| Gate summary | Relevant할 때 scope, approval, decision, design, evidence, verification, QA, acceptance, close blocker, Manual QA, residual-risk status의 compact value. |
| Projection freshness | Projection id 또는 ref, known이면 `source_state_version`, freshness state, 필요한 refresh/reconcile warning. |

Relevant할 때 ref 또는 한 줄 summary로 push하는 것:

- Journey Card 또는 compact status card
- 현재 수용 기준 snapshot
- approval status
- latest evidence manifest ref와 coverage summary
- latest Run, Eval, Manual QA, report, residual-risk ref
- relevant policy, TDD trace, stewardship, module/interface, domain ref

다음 항목은 refs-first로 두고 body는 필요할 때만 pull합니다.

- Evidence, Run, Eval, Manual QA records
- artifact, log, screenshot, diff, workflow recording, large trace
- 오래된 PRD, 오래된 design, closed issue, stale doc, moved-path note
- module map, interface contract, domain language, coding standard, TDD guidance

Refs-first는 connector가 default prompt에 큰 본문을 붙여 넣지 않고 stable id, path, hash, summary, outcome, freshness를 push해야 한다는 뜻입니다. 다음 safe action이 content inspection을 요구할 때만 excerpt를 embed하고, 그 excerpt는 source ref와 연결해 둡니다. Retrieved 또는 indexed context도 같은 규칙을 따릅니다. Agent가 다음에 무엇을 살펴볼지 알려 줄 수는 있지만, owner path가 실제 state change를 기록하기 전까지는 pull-only context로 남습니다.

Compact status card는 "현재 어디이고 다음은 무엇인가?"를 위해 envelope를 렌더링합니다. Judgment-context는 별도입니다. Judgment-context는 사용자 판단이 필요할 때만 사용하며, decision question, options, recommendation, uncertainty, deferral effect, relevant refs를 포함하되 전체 evidence나 artifact body를 always-on context로 만들지 않습니다.

Evaluator는 더 좁은 verification bundle을 받아야 합니다. 여기에는 수용 기준, changed file, approval scope, relevant Decision Packet, residual risk summary, Autonomy Boundary, deferred decision, codebase stewardship ref, evidence manifest ref, required TDD trace ref, Manual QA requirement, artifact ref, freshness state, forbidden pattern이 포함됩니다.

이후 Context Index는 relevant projection, artifact ref, repo file, docs, note를 찾아오는 데 도움을 줄 수 있습니다. 하지만 owner 문서가 승격하기 전까지는 읽기 전용 context provider일 뿐 connector 권한 경로가 아닙니다. Context Index와 retrieved-context의 전체 권한 없음 경계는 [Roadmap: Context Index](../roadmap.md#context-index)가 담당합니다.

## Fallback Semantics

Fallback은 접점 이름이 아니라 guarantee level과 risk로 설명합니다.

| Fallback | 쓰는 경우 | 경계 |
|---|---|---|
| Cooperative | 접점이 지시를 따를 수 있지만 강제할 수 없을 때. | Agent에게 `prepare_write`를 쓰고, blocked decision에서 보류하고, run을 기록하라고 지시합니다. Authoritative MCP를 사용할 수 없거나 write scope를 확인할 수 없으면 product write를 멈춥니다. |
| Detective | Harness가 실행 뒤에 changed file, log, projection drift, artifact gap을 관찰할 수 있을 때. | Validator가 상태를 `stale`, `partial`, `blocked`, `failed`로 표시하고 repair, reconcile, fresh verification을 요구할 수 있습니다. |
| Preventive | 입증된 hook, permission layer, wrapper, policy engine, sidecar가 실행 전에 차단할 수 있을 때. | 입증된 blocking path가 실제로 포함하는 operation에 대해서만 주장합니다. |
| Isolated | Risk가 separation을 요구할 때. | 별도 worktree, sandbox, process, manual evaluator bundle에서 work 또는 verification을 실행합니다. |

MCP가 unavailable이면 connector는 권한 있는 상태 업데이트를 주장하면 안 됩니다. `MCP_SERVER_UNAVAILABLE`과 `SURFACE_MCP_UNAVAILABLE`은 diagnostic condition이지 추가 public `ErrorCode` 값이 아닙니다. `MCP_UNAVAILABLE`은 stable public availability code로 남습니다.

`MCP_SERVER_UNAVAILABLE`은 tool call이 Core에 닿지 못해 authoritative Core response가 없다는 뜻입니다. `SURFACE_MCP_UNAVAILABLE`은 Core 또는 operator가 연결된 접점에서 사용할 수 있는 MCP가 없거나 MCP configuration이 최신이 아니거나 required tool을 호출할 수 없다고 관찰할 수 있다는 뜻입니다. Product/runtime/code write는 MCP가 다시 연결되거나 진단될 때까지 보류합니다. 예외는 exact path allowlist가 있는 명시적 pre-MVP documentation-authoring batch인 `DOCS_AUTHORING_OVERRIDE`뿐입니다. 이 override는 documentation-maintainer override일 뿐이며 Core authorization, Write Authorization, evidence, verification, QA, 결과 수락, 남은 위험을 받아들이는 판단, close, 기준 상태 전이가 아닙니다.

MCP는 동작하지만 pre-tool guard가 약하면 low-risk direct work는 cooperative `prepare_write`와 detective changed-path validation으로 진행할 수 있습니다. Medium/high-risk work에는 stricter validation, 입증된 sidecar guard, explicit approval, detached verification, isolation을 요구해야 합니다.

Native capture가 없으면 connector는 manual artifact capture로 fallback해야 합니다. 즉 diff, log, screenshot, workflow note, command output, QA note를 사용자나 operator가 제공한 named artifact ref로 기록합니다. Native isolation 또는 fresh evaluator support가 없으면 수용 기준, changed file, relevant ref, artifact ref, freshness state, forbidden pattern을 담은 manual verification bundle로 fallback합니다. 이런 fallback은 명시적 evidence route일 뿐 preventive 또는 isolated guarantee level로 올려 주지 않습니다.

Projection `stale` 상태는 상태와 별도로 보고합니다. `source_state_version`이 canonical state보다 오래됐거나, unknown이거나, expected인데 없으면 connector는 읽기용 projection context가 stale일 수 있다고 warning해야 합니다. Connector가 기준 상태를 직접 읽을 수 있으면 거기서 계속할 수 있지만, Markdown projection에 의존하는 action은 먼저 refresh 또는 reconcile을 해야 하며 stale projection을 authority로 취급하면 안 됩니다.

## Role Lens 동작

Role Lens는 사용자가 익숙한 검토 관점으로 agent를 이끌 수 있게 하는 non-authoritative skill 또는 playbook 접점입니다. Initial lenses는 다음과 같습니다.

- `product-review`
- `eng-review`
- `design-review`
- `security-review`
- `qa-review`
- `release-handoff`

Connector는 이를 slash command, button, prompt snippet, recommended playbook으로 보여줄 수 있습니다. Lens name은 검토 관점을 고를 뿐 권한 경로를 고르지 않습니다.

Role Lens output은 다음 항목을 표시하거나 경로로 추천할 수 있습니다.

- `DecisionPacketCandidate` 또는 existing Decision Packet route
- 실제 validator/check가 낼 validator finding candidate 또는 suggested `ValidatorResult` route
- evidence requirement
- Eval 또는 verification 필요
- Manual QA requirement
- Approval 필요
- residual-risk candidate
- 필요한 경우 Change Unit update recommendation
- release handoff 보고서 input
- recommended next playbook

이 항목들은 기존 Core/MCP state-changing path가 실제 동작을 기록하기 전까지 display 및 routing output일 뿐입니다. Role Lens output은 schema나 기준 record를 도입하거나, 그 자체로 기준 상태를 변경하거나, write를 허가하거나, Approval을 부여하거나, Decision Packet을 충족하거나, QA 또는 verification을 면제하거나, 남은 위험을 받아들이거나, 결과를 수락하거나, Task를 닫거나, assurance를 올리면 안 됩니다. Lens가 상태 변경이 필요한 일을 찾아내면 접점은 normal MCP tool과 Core path로 라우팅합니다.

Two-stage review display는 두 stage가 분명히 분리되어 보이게 해야 합니다.

| Stage | 질문 |
|---|---|
| Spec Compliance Review | 현재 Harness 권한 안에서 요청한 작업이 완료되었는가: 수용 기준 충족, Change Unit completion condition, scope/write authority 호환성, Decision Packet compatibility, evidence coverage, Residual Risk 표시? |
| Code Quality / Stewardship Review | implementation이 codebase 안에서 유지보수하기 좋은가: domain language, module/interface boundary, vertical slice shape, feedback loop 또는 TDD trace, codebase stewardship, context hygiene, follow-up risk? |

Same-session review는 유용한 self-check일 수 있지만 detached verification이 아니며 `assurance_level=detached_verified`로 표시하면 안 됩니다.

## 기준 접점 계약

MVP는 하나의 기준 접점을 목표로 합니다. 기준 접점은 넓은 ecosystem 지원이 아니라 kernel을 증명해야 합니다.

Minimum reference expectations:

- public tool과 resource에 `T2 MCP` 사용 가능
- product write 전 cooperative `prepare_write`
- run 이후 detective changed-path와 artifact validation
- evidence manifest에 충분한 run summary와 artifact capture
- manual verification bundle 또는 fresh evaluator instruction
- Manual QA note artifact support
- generated file, managed block, MCP config snippet, profile freshness용 connector manifest
- native capture가 없을 때 manual artifact capture fallback
- guard, freeze, careful-mode label이 표시될 때 실제 차단 가능 범위와 사후 감지 범위 표시
- common state와 fallback path를 다루는 conformance smoke

Reference surface 동작 세부사항과 접점별 설정은 concrete surface를 이름으로 부를 때만 [Surface Cookbook](surface-cookbook.md)에 둡니다.

## Connector Conformance 개요

Connector conformance는 profile이 선언한 capability tier에서 공통 contract를 지킬 수 있음을 입증해야 합니다.

Overview scenario:

- active Task가 있을 때와 없을 때의 status
- Use procedure가 요구할 때 significant work 재개 전 current Journey Card 표시
- intake를 `advisor`, `direct`, `work`로 분류
- shared design과 decision을 포함한 work shaping
- Change Unit scope와 vertical/horizontal exception handling
- 가능할 때 recommendation과 uncertainty가 있는 one blocking question
- 차단하는 사용자 소유 판단에 broad approval 대신 Decision Packet 표시
- Autonomy Boundary breach가 stop하거나 Decision Packet으로 route
- AFK work가 active Change Unit scope, Autonomy Boundary latitude, 적용되는 granted sensitive approval, 실제 product write 전 compatible `prepare_write` / Write Authorization 안에 머무름
- `prepare_write` allowed/blocked path
- allowed write에 Write Authorization 생성 및 Write Authority Summary 표시
- write-capable `record_run`이 compatible Write Authorization consume
- sensitive approval request, granted, denied, expired path
- artifact와 evidence update를 포함한 `record_run`
- direct result projection
- verification launch 또는 manual verification bundle
- same-session verification guard
- Manual QA required, passed, failed, waived
- product/user risk가 있는 QA 면제를 Decision Packet으로 route
- acceptance required와 recorded
- acceptance 또는 successful close 전 close-relevant residual risk visible
- risk-accepted close에는 accepted Residual Risk refs 추가 요구
- 최신이 아닌 projection과 reconcile flow
- generated file drift detection
- connector manifest profile freshness와 stale capability profile detection
- required tier가 없을 때 capability fallback
- local-only MCP 기본값과, profile 밖 remote/shared 노출이 held, failed, 또는 capability-insufficient로 보고되는 동작
- MCP unavailable product-write 보류

정확한 fixture 형식과 operator command 의미는 operations and conformance docs가 담당합니다.
