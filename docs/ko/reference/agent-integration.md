# Agent 통합 참조

## 이 문서로 할 수 있는 일

이 참조는 agent 접점을 Harness에 연결할 때, 그 접점이 실제로 보장할 수 있는 수준을 과장하지 않도록 돕습니다.

이 문서는 공통 커넥터 계약을 담당합니다. Capability tier, capability profile, generated manifest 기대사항, context push/pull 원칙, fallback 의미, Role Lens 동작, reference surface 계약, connector conformance 개요를 정의합니다.

사용자에게 보이는 agent 절차는 [에이전트 세션 흐름](../use/agent-session-flow.md)을 봅니다. 접점별 설정 메모는 [Surface Cookbook](surface-cookbook.md)을 봅니다.

이 문서는 참조 문서입니다. 문서 세트가 구현 계획에 사용할 수 있다고 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 제품 MVP 목표는 v0.1 Kernel MVP이며, Kernel Smoke가 좁은 conformance profile로 이를 실행합니다. v0.2부터 v0.4까지는 Agency-Hardened MVP reference conformance target을 향한 staged pack이고, v1+ Expansion은 owner 문서가 승격하고 증명하기 전까지 roadmap 범위에 둡니다.

## 이런 때 읽기

- agent 접점용 connector를 구현하거나 검토할 때.
- 접점 capability profile을 선언하거나 점검할 때.
- 연결된 profile이 guarantee level, guard, freeze, fallback, MCP availability를 어떻게 표시해야 하는지 정할 때.
- connector conformance coverage를 작성할 때.
- 공통 contract와 surface recipe의 경계를 확인해야 할 때.

## 읽기 전에

사용자에게 보이는 절차는 [에이전트 세션 흐름](../use/agent-session-flow.md)을 읽고, 쓰기와 닫기 권한은 [커널 참조](kernel.md)를 읽고, MCP exposure, generated-file, stale-context, artifact, secret, capability-overclaiming threat는 [보안 위협 모델 참조](security-threat-model.md)를 읽습니다. 이 참조는 connector behavior와 capability 표시를 설명하며, kernel state transition을 정의하지 않습니다.

## 핵심 생각

Connector는 agent에게 작고 최신인 context를 주고, 상태 변경을 Harness로 라우팅하며, 입증된 capability profile이 실제로 제공할 수 있는 보장만 말해야 합니다. Cooperative 또는 detective surface는 hold하거나 detect할 수 있습니다. 실행 전 차단이라고 말할 수 있는 것은 fixture로 입증된 pre-tool blocking을 갖춘 covered preventive path뿐이며, isolated path는 approval이나 verification이 아니라 separation으로 설명해야 합니다.

## 통합을 쉬운 말로 설명하면

Agent 접점은 사용자가 agent와 대화하는 접점입니다. Harness는 Task 상태, 쓰기 권한, 근거, verification, Manual QA, acceptance, projection, reconcile 동작을 대화 기록 밖에 두는 로컬 권한 계층입니다.

Connector는 agent에게 작고 최신인 context를 주고, 상태 변경을 Harness MCP tool로 라우팅하고, 접점이 할 수 있으면 실제로 일어난 일을 캡처하며, 연결된 profile의 실제 guarantee level을 이름 붙여야 합니다. Capability는 구체적이어야 합니다. 실제 host, target profile, version/configuration, workspace policy, capture path, guard 또는 isolation path별로 선언되고 입증되어야 합니다. 접점 이름만으로 해당 capability를 갖췄다고 주장하면 안 됩니다.

공통 구조는 다음과 같습니다.

```text
user conversation surface
  -> short always-on rules/context
  -> harness skill, command, or playbook
  -> harness MCP server
  -> harness Core
  -> adapter, hook, sidecar, validator, or isolation layer
```

Always-on rule과 context는 짧고 최신이어야 하며, 그 자체가 권한 출처가 아닙니다. 언제 Harness를 쓰는지, current status 또는 Journey Card를 어디서 읽는지, product write에는 `prepare_write`가 필요하다는 점, 사용자 소유 판단은 Decision Packet으로 라우팅한다는 점, status가 실행 전에 실제로 막을 수 있는 것과 실행 뒤에만 감지할 수 있는 것을 보여야 한다는 점, authoritative MCP를 사용할 수 없으면 product write를 보류한다는 점만 알려주면 충분합니다. Schema dump, 오래된 task history, evidence body 복사본, reference contract 복제본으로 늘리면 안 됩니다. 세션 절차 자체는 [에이전트 세션 흐름](../use/agent-session-flow.md)이 담당합니다.

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
| Security asset, trust boundary, threat category, control category | [보안 위협 모델 참조](security-threat-model.md) |

## Capability Tiers

| Tier | 의미 | 대표 capability |
|---|---|---|
| `T0 Context` | 접점이 Harness 원칙을 읽을 수 있습니다. | rules/context file |
| `T1 Skill` | 접점이 Harness 절차를 따를 수 있습니다. | skill, command, prompt, playbook |
| `T2 MCP` | 접점이 Harness tool과 resource를 호출할 수 있습니다. | MCP server connection |
| `T3 Capture` | 접점이 diff, log, run output을 신뢰할 만하게 반환할 수 있습니다. | structured output, wrapper, adapter |
| `T4 Guard` | Fixture coverage가 해당 profile의 구체적인 path를 입증한 경우, 접점이 대상 out-of-scope file, command, network, secret을 실행 전에 차단하거나 중단할 수 있습니다. | hook, permission system, policy engine, sidecar |
| `T5 Isolation` | 접점이 verification 또는 risky work를 별도 경계에서 실행할 수 있습니다. | worktree, sandbox, fresh process, isolated runner |
| `T6 QA Capture` | 접점이 browser, screenshot, walkthrough, workflow-recording, Manual QA artifact를 구조화할 수 있습니다. | browser runner, screenshot capture, console/network capture, accessibility snapshot, QA note capture |

일반적인 interactive Harness 사용은 `T2` 이상에서 가장 자연스럽습니다. Reliable detached verification에는 보통 `T3` capture와 실제 independence boundary가 필요합니다. High-risk work에는 가능하면 fixture로 입증된 `T4` guard 또는 `T5` isolation을 사용해야 합니다. `T6`는 UI/UX evidence를 보강하지만 Manual QA judgment, final acceptance, detached verification을 대체하지 않으며, human Manual QA note와 수동으로 제공된 artifact를 기록할 수 있다면 v0.1/default reference posture나 Agency-Hardened Manual QA coverage의 필수 조건이 아닙니다.

`T6 QA Capture` profile은 지원하는 capture type과 fallback 동작을 이름으로 밝혀야 합니다. Candidate capture type에는 screenshot, console log, network trace, accessibility snapshot, workflow recording이 있습니다. Captured file은 durable storage 전에 redaction과 secret/PII handling을 따라야 하며, Manual QA record 또는 feedback loop execution에 붙는 artifact ref로 등록되어야 합니다.

## Capability Profiles

Harness connector는 product 또는 surface name에서 동작을 가정하지 않고 capability profile을 사용해야 합니다. Profile은 실제 작업을 실행할 host/profile에 한정됩니다. 여기에는 detected version, MCP configuration, hook/permission/workspace policy posture, capture method, QA capture method, redaction policy, artifact retention behavior, 그리고 선언을 최신으로 만드는 conformance 또는 operator-check 근거가 포함됩니다. 다른 host, profile, version, configuration, permission model, capture path, conformance result를 쓰려면 같은 capability를 주장하기 전에 profile을 갱신해야 합니다.

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

- v0.1 baseline과 staged-delivery `local_only` 자세가 적용되는지 여부
- localhost TCP, local socket, in-process/stdio, process-scoped configuration material, 또는 이에 준하는 local IPC 같은 로컬 transport 전제
- bind scope, socket path class, process pipe/stdio, per-project token handle, process-scoped config handle, 또는 이에 준하는 local control 같은 access-control material class. raw token, secret, private configuration value는 포함하지 않습니다.
- 관련 없는 호출자가 endpoint를 사용하지 못하게 하는 access-control contract
- remote 또는 shared MCP 노출이 disabled, unsupported, 또는 profile에 의해 명시적으로 enabled 중 어디에 해당하는지
- local 범위를 넘는 노출이 있다면, owner-doc 및 conformance-promotion basis, secret/PII 처리 정책, redaction 또는 omission 동작, guarantee display, 그 노출이 권한을 조용히 올려 주지 않음을 증명하는 conformance coverage

이 field가 필요한 security reason은 [보안 위협 모델 참조](security-threat-model.md)가 담당하고, 이 참조 문서는 connector profile이 이를 어떻게 보고하는지 담당합니다.

Capability profile은 detected version, MCP config, hook, permission, workspace policy, generated file 또는 managed block, conformance result, capture method, QA capture method, browser test environment, redaction policy, artifact 보존 동작, access-control material class, local bind/reachability posture, isolation/guard wrapper 동작이 바뀌면 갱신해야 합니다. Beyond-local exposure는 owner docs와 conformance가 승격하기 전까지 v0.1 baseline과 staged delivery 밖에 남으며, connector prose는 이를 안전한 v0.1 또는 staged-delivery default처럼 표시하면 안 됩니다.

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
  - sidecar guard for fixture-proven covered operations
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

이 참조는 connector 프로필이 그 level을 어떻게 보고하고 표시하는지 담당합니다. Surface name, product name, recipe name, mode label에서 더 강한 level을 추론하면 안 되며, 보장 수준을 Approval, Write Authorization, verification, QA, 결과 수락, residual-risk acceptance, close readiness, kernel gate로 취급하면 안 됩니다.

| Level | 표시 책임 |
|---|---|
| `cooperative` | 접점이 Harness 결정을 따르도록 지시받음을 보여줍니다. 보류는 지시에 따른 것이며 Harness가 실행 전 물리적 차단을 주장하지 않습니다. |
| `detective` | Harness가 실행 뒤에 changed path, log, artifact, projection drift를 관찰하고 상태를 `stale`, `blocked`, `partial`, `failed`로 표시할 수 있음을 보여줍니다. 이를 prevention이 아니라 detection으로 표시해야 합니다. |
| `preventive` | 실행 전에 차단할 수 있음이 fixture로 입증된 hook, wrapper, permission layer, policy engine, sidecar path와 covered operation을 보여줍니다. |
| `isolated` | Risky work 또는 verification에 쓰는 별도 worktree, sandbox, process, evaluator bundle 또는 동등한 경계를 보여줍니다. Isolation만으로 approval, acceptance, verification이 된 것처럼 보여주면 안 됩니다. |

Guard, freeze, careful-mode label은 실제 profile 위에 얹힌 safety-control label이지 authority tier가 아닙니다. 표시할 때는 실행 전에 실제로 막을 수 있는 것과 실행 뒤에만 감지할 수 있는 것을 나눠야 합니다.

| 사용자 표현 | 실제 경계 |
|---|---|
| Freeze | 현재 work 주변의 눈에 보이는 범위 보류 또는 다음 행동을 더 엄격하게 제한하는 상태입니다. Cooperative profile에서는 지시에 따른 보류입니다. Detective profile에서는 사후 validation과 함께 표시할 수 있습니다. Covered operation에 대해 fixture로 입증된 pre-tool blocking이 있을 때만 hard prevention입니다. Persistent owner-record change는 여전히 normal Core path를 거칩니다. |
| Guard | 입증된 profile과 현재 적용 경로에 따른 cooperative, detective, preventive, isolated protection입니다. Preventive 표현은 fixture로 입증된 pre-tool blocking이 있는 covered operation에만 씁니다. |
| Careful mode | 더 엄격한 `prepare_write`, scope, evidence, status refresh, user-question posture입니다. 새로운 authority tier가 아니며 그 자체로 차단하지 않고 gate나 decision을 충족하지 않습니다. |

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
- surface version, MCP config, hook, permission, workspace policy, wrapper, sidecar, generated file, managed file, conformance result, capture method, QA capture method, redaction policy, artifact retention behavior가 바뀌면 profile 또는 generated block을 stale로 표시
- 사람의 편집을 덮어쓰기 전에 drift 탐지
- drift가 감지되면 explicit reconcile 또는 reconnect decision이 replacement를 허가하기 전까지 existing file 또는 managed block을 그대로 유지
- 필요하면 drift를 reconcile로 라우팅하고, 편집된 generated file이 canonical Task state가 아님을 보고

Manifest concept은 공통입니다. 접점별 생성 파일 이름은 [Surface Cookbook](surface-cookbook.md)이 담당합니다.

## Context Push/Pull Principles

Implementation agent에게는 매 turn마다 compact always-on Harness context envelope를 주고, 긴 reference는 필요할 때만 가져오게 해야 합니다. 이 envelope는 current operational state이지 chat history, old projection text, complete reference dump가 아닙니다. id, 한 줄 summary, freshness marker를 사용해야 하며, 한 화면 안팎으로 유지하는 것은 유용한 guidance이지 schema limit은 아닙니다.

오래된 chat memory와 pull한 context는 agent가 살펴볼 ref를 찾는 데 도움을 줄 수 있지만, write를 허가하거나, gate를 충족하거나, Task를 close하거나, 결과를 수락하거나, QA 또는 verification을 면제하거나, 잔여 위험을 받아들이거나, current owner record를 대체하거나, stale projection을 고칠 수 없습니다. 권한에 영향을 주는 오래된 context는 먼저 담당 Core path로 reconcile되어야 합니다.

Always-on compact context 규칙 세트는 다음 10개 이하로 유지합니다.

1. 중요한 작업 또는 resume 전에 current status 또는 Journey Card를 먼저 읽습니다.
2. Product/runtime/code write에는 compatible `prepare_write`와 Write Authorization이 필요합니다.
3. 사용자 소유 제품 판단 또는 중요한 기술 판단은 Decision Packet으로 라우팅합니다.
4. Approval은 product judgment, result acceptance, residual-risk acceptance가 아닙니다.
5. Projection은 읽기용 output이지 canonical state가 아닙니다.
6. Evidence는 artifact ref와 state ref를 사용하며, 붙여 넣은 log나 복사한 evidence body를 권한으로 삼지 않습니다.
7. Same-session review는 self-checking context이지 detached verification이 아닙니다.
8. MCP unavailable이면 authoritative state update, gate update, evidence, acceptance, residual-risk, projection repair, close 주장을 하지 않습니다.
9. Acceptance 또는 close 전에 blocker와 close-relevant residual risk를 보여줍니다.
10. 다음 action에 필요할 때만 Reference docs, schema, historical record, large artifact를 pull합니다.

다음은 compact current-state envelope에 들어갈 수 있는 후보 field입니다. 표 전체를 매번 보내라는 뜻이 아닙니다. Active phase bundle, next safe action, freshness, relevance가 어떤 field를 표시할지 결정합니다.

현재 phase bundle으로 걸러서 push할 수 있는 envelope 후보:

| Envelope item | Push shape |
|---|---|
| Active Task | Task id, title, schema mode, 파생된 작업 모양 표시 label, lifecycle phase. |
| Current display | Journey Card 또는 compact status card ref. 렌더링된 card가 없으면 한 줄 current status. |
| Next safe action | 다음 action과, 막힌 경우 가장 작은 unblocker. |
| Active scoped Change Unit | In-scope work와 out-of-bounds area의 한 줄 summary. |
| Autonomy Boundary | Agent가 혼자 판단해도 되는 것과 여전히 사용자 판단이 필요한 것. |
| Active Decision Packet | Decision Packet id와 한 줄 question, 또는 `none`. |
| Write Authority Summary | Not requested, allowed, blocked, stale, unavailable 같은 display status와, relevant한 경우 scoped path/tool summary. |
| 수용 기준 | 다음 행동이나 close가 의존하는 경우 current 수용 기준 snapshot 또는 ref. |
| Approval status | Relevant할 때 active sensitive-action Approval status 또는 `not_required`. |
| Evidence refs | Evidence가 다음 행동이나 close에 영향을 주는 경우 latest Evidence Manifest ref와 짧은 coverage summary. |
| Residual-risk summary | Known close-relevant residual risk summary와 refs, 또는 close나 acceptance가 의존하는 경우 명시적인 absence. |
| Guarantee level | 실제 연결 profile level과 입증 가능한 guard 또는 detection behavior. 접점 이름에서 추론하지 않습니다. |
| Connector profile freshness | Connector manifest ref, `capability_profile_version`, `last_verified_at`, 그리고 generated file, MCP config, hook, wrapper, sidecar, capture, isolation 동작이 바뀐 경우 stale reason. |
| Gate summary | Relevant할 때 scope, approval, decision, design, evidence, verification, QA, acceptance, close blocker, Manual QA, residual-risk status의 compact value. |
| Projection freshness | Projection id 또는 ref, known이면 `source_state_version`, freshness state, 필요한 refresh/reconcile warning. |

Relevant할 때 ref 또는 한 줄 summary로 push하는 것:

- latest Run, Eval, Manual QA, report, residual-risk ref
- relevant policy, TDD trace, stewardship, module/interface, domain ref

다음 항목은 refs-first로 두고 body는 필요할 때만 pull합니다.

- Evidence, Run, Eval, Manual QA records
- artifact, log, screenshot, diff, workflow recording, large trace
- 오래된 PRD, 오래된 design, closed issue, stale doc, old projection, moved-path note
- module map, interface contract, domain language, coding standard, TDD guidance

Refs-first는 connector가 default prompt에 큰 본문을 붙여 넣지 않고 stable id, path, hash, summary, outcome, freshness를 push해야 한다는 뜻입니다. 다음 safe action이 content inspection을 요구할 때만 excerpt를 embed하고, 그 excerpt는 source ref와 연결해 둡니다. Retrieved, indexed, remembered, summarized context도 같은 규칙을 따릅니다. Agent가 다음에 무엇을 살펴볼지 알려 줄 수는 있지만, owner path가 실제 state change를 기록하기 전까지는 pull-only context로 남습니다. Write를 허가하거나 Write Authorization을 만들거나, Decision Packet을 해소하거나, Approval을 부여하거나, gate를 충족하거나, evidence를 만들거나, verification을 수행 또는 기록하거나, QA를 기록하거나, QA 또는 verification을 면제하거나, 결과를 수락하거나, 잔여 위험을 받아들이거나, projection freshness를 바꾸거나, Task를 close하면 안 됩니다.

Agent가 전체 문서 세트를 읽지 않도록 phase-based bundle을 사용합니다.

| Phase | Context에 push | 필요할 때 pull |
|---|---|---|
| Intake | Active 또는 likely Task id, 필요할 때 schema mode, 파생된 작업 모양 표시 label, current status 또는 Journey Card, 네 가지 display group, next safe action, primary blocker, known source refs, guarantee/MCP availability. | Classification, authority, blocker 표시가 불명확할 때만 task history, user guide, session-flow detail, Reference docs. |
| Discovery | 요구사항 구체화 summary 또는 Discovery Brief ref, decision area별 blocking question, parked assumption, 확인 가능한 repo/Harness facts, visible user-owned judgment candidate, QA/verification expectation, 첫 구현 후보 또는 작업 분할 제안. | 에이전트가 확인할 수 있는 사실과 사용자 결정을 분리하고 안전한 다음 작업을 scope하는 데 필요한 fact에 한해 repo docs, module/interface/domain refs, older PRD/design, design-quality policy, Decision Packet guidance. |
| Write | Active Change Unit, Autonomy Boundary, intended paths/tools/commands/network/secrets summary, baseline, Approval status, active Decision Packet, Write Authority Summary, capability guarantee. | Intended write가 해당 boundary를 건드리거나 connector가 check를 구현할 때만 정확한 `prepare_write`, Kernel, security, approval, policy reference. |
| Evidence | Changed-path summary, latest Run summary, Evidence Manifest ref, integrity/freshness가 있는 artifact ref, evidence gap, next evidence action. | Evidence를 해석, 수리, 등록할 때만 log, diff, screenshot, trace, raw artifact, artifact-storage detail, evidence contract section. |
| Verification | Acceptance criteria, changed file, evidence ref, artifact ref, approval scope, active 또는 relevant Decision Packet, residual-risk summary, Manual QA requirement, independence/freshness profile, forbidden pattern. | Evaluator가 확인해야 할 때만 full evaluator bundle material, source file, log, exact Eval/Manual QA contract, verification guidance. |
| Close | Close-readiness summary, close blocker, evidence/verification/QA/acceptance status, residual-risk summary 또는 accepted refs, projection freshness, smallest unblocker. | Blocker 또는 close attempt가 exact contract나 source content에 의존할 때만 `close_task`, acceptance, residual-risk, Manual QA, verification, artifact detail. |

Discovery phase의 `첫 구현 후보`와 `작업 분할 제안` 같은 표현은 context proposal/support phrase이며 standalone schema, canonical record type, gate value, projection kind, authority path가 아닙니다.

사용자에게 mode를 표시할 때 connector는 읽기/조언, 작은 변경, 추적되는 작업을 먼저 보여줘야 합니다. 이 label은 파생된 표시 text일 뿐 schema field, enum value, canonical record type, projection kind, gate value, authority path가 아닙니다. Envelope 또는 context bundle이 작업 모양 표시 label을 언급한다면, 이는 현재 schema mode에서 파생한 표시 label이라는 뜻이며, future schema owner가 명시적으로 정의하기 전까지 새 API field가 아닙니다. State, conformance, API payload에서는 schema-owned 값 `advisor`, `direct`, `work`가 그대로 유지됩니다. 표시 번역은 제품 파일 쓰기 권한 확인, 사용자 소유 판단 라우팅, sensitive-action Approval, evidence, QA, verification, acceptance, 잔여 위험 표시, close rule을 줄이면 안 됩니다.

Phase bundle은 context discipline이지 새 schema나 gate가 아닙니다. Phase가 바뀌면 connector가 기본으로 push하는 항목이 바뀔 뿐이며, write를 허가하거나, decision을 해소하거나, evidence를 만들거나, verification을 수행하거나, risk를 받아들이거나, Task를 close하지 않습니다.

Compact status card는 "현재 어디이고 다음은 무엇인가?"를 위해 envelope를 렌더링합니다. Judgment-context는 별도입니다. Judgment-context는 사용자 판단이 필요할 때만 사용하며, decision question, options, recommendation, uncertainty, deferral effect, relevant refs를 포함하되 전체 evidence나 artifact body를 always-on context로 만들지 않습니다.

Status, next-action, recommendation, recommended-playbook output은 read-only guidance입니다. `prepare_write`, Decision Packet, Change Unit update, evidence collection, verification, QA, reconcile, close attempt를 추천할 수는 있지만, 그 추천 자체가 state를 mutate하거나 gate를 충족하지는 않습니다. 추천된 action이 기존 MCP/Core mutation path를 거친 뒤에만 state effect가 생깁니다.

Evaluator는 더 좁은 verification bundle을 받아야 합니다. 여기에는 수용 기준, changed file, approval scope, relevant Decision Packet, residual risk summary, Autonomy Boundary, deferred decision, codebase stewardship ref, evidence manifest ref, required TDD trace ref, Manual QA requirement, artifact ref, freshness state, forbidden pattern이 포함됩니다.

이후 Context Index는 relevant projection, artifact ref, repo file, docs, note를 찾아오는 데 도움을 줄 수 있습니다. 하지만 owner 문서가 승격하기 전까지는 읽기 전용 context provider일 뿐 connector 권한 경로가 아닙니다. Context Index와 retrieved-context의 전체 권한 없음 경계는 [Roadmap: Context Index](../roadmap.md#context-index)가 담당합니다.

## Fallback Semantics

Fallback은 접점 이름이 아니라 guarantee level과 risk로 설명합니다.

| Fallback | 쓰는 경우 | 경계 |
|---|---|---|
| Cooperative | 접점이 지시를 따를 수 있지만 강제할 수 없을 때. | Agent에게 `prepare_write`를 쓰고, blocked decision에서 보류하고, run을 기록하라고 지시합니다. Authoritative MCP를 사용할 수 없거나 write scope를 확인할 수 없으면 product/runtime/code write를 instruction으로 멈춥니다. |
| Detective | Harness가 실행 뒤에 changed file, log, projection drift, artifact gap을 관찰할 수 있을 때. | Validator가 상태를 `stale`, `partial`, `blocked`, `failed`로 표시하고 repair, reconcile, fresh verification을 요구할 수 있습니다. |
| Preventive | Fixture로 입증된 hook, permission layer, wrapper, policy engine, sidecar가 실행 전에 차단할 수 있을 때. | Fixture로 입증된 blocking path가 실제로 포함하는 operation에 대해서만 주장합니다. |
| Isolated | Risk가 separation을 요구할 때. | 별도 worktree, sandbox, process, manual evaluator bundle에서 work 또는 verification을 실행합니다. 관련 owner path가 결과를 기록하지 않는 한 separation을 approval, acceptance, verification으로 취급하지 않습니다. |

MCP가 unavailable이면 connector는 권한 있는 상태 업데이트를 주장하면 안 됩니다. `MCP_SERVER_UNAVAILABLE`과 `SURFACE_MCP_UNAVAILABLE`은 diagnostic condition이지 추가 public `ErrorCode` 값이 아닙니다. `MCP_UNAVAILABLE`은 stable public availability code로 남습니다.

`MCP_SERVER_UNAVAILABLE`은 tool call이 Core에 닿지 못해 해당 call path에서 authoritative Core response가 없다는 뜻입니다. Connector는 Core에 닿을 수 없는 동안 chat memory, generated file, cached projection, old status/next recommendation, operator prose에서 Core state, Write Authorization, gate status, evidence, acceptance, residual-risk acceptance, close readiness를 만들어 내면 안 됩니다. `SURFACE_MCP_UNAVAILABLE`은 Core 또는 operator가 연결된 접점에서 사용할 수 있는 MCP가 없거나 MCP configuration이 최신이 아니거나 required tool을 호출할 수 없다고 관찰할 수 있다는 뜻입니다. Product/runtime/code write는 MCP가 다시 연결되거나 진단될 때까지 보류합니다. Cooperative surface는 instruction으로 hold하고, detective surface는 실행 뒤 mismatch도 보고할 수 있으며, stronger profile은 fixture로 입증된 guard가 operation을 cover하거나 실제 isolation boundary를 사용할 때만 실행 전에 차단할 수 있습니다. 예외는 exact path allowlist가 있는 명시적 pre-MVP documentation-authoring batch인 `DOCS_AUTHORING_OVERRIDE`뿐입니다. 이 override는 documentation-maintainer override일 뿐이며 Core authorization, Write Authorization, evidence, verification, QA, 결과 수락, 잔여 위험을 받아들이는 판단, close, 기준 상태 전이가 아닙니다.

MCP는 동작하지만 pre-tool guard가 약하면 low-risk direct work는 cooperative `prepare_write`와 detective changed-path validation으로 진행할 수 있습니다. Medium/high-risk work는 assessed threat/control path가 preventive 또는 isolated control을 요구하는 경우 cooperative-only claim에 의존하면 안 됩니다. [보안 위협 모델](security-threat-model.md)은 security reason을 이름 붙이고, 정확한 behavior는 connector profile, operations, API, kernel, conformance owner가 정의합니다.

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

이 항목들은 기존 Core/MCP state-changing path가 실제 동작을 기록하기 전까지 display 및 routing output일 뿐입니다. Role Lens output은 status/next recommendation output과 마찬가지로 schema나 기준 record를 도입하거나, 그 자체로 기준 상태를 변경하거나, write를 허가하거나, Approval을 부여하거나, Decision Packet을 충족하거나, QA 또는 verification을 면제하거나, 잔여 위험을 받아들이거나, 결과를 수락하거나, Task를 닫거나, assurance를 올리면 안 됩니다. Lens가 상태 변경이 필요한 일을 찾아내면 접점은 normal MCP tool과 Core path로 라우팅합니다.

Two-stage review display는 두 stage가 분명히 분리되어 보이게 해야 합니다.

| Stage | 질문 |
|---|---|
| Spec Compliance Review | 현재 Harness 권한 안에서 요청한 작업이 완료되었는가: 수용 기준 충족, Change Unit completion condition, scope/write authority 호환성, Decision Packet compatibility, evidence coverage, Residual Risk 표시? |
| Code Quality / Stewardship Review | implementation이 codebase 안에서 유지보수하기 좋은가: domain language, module/interface boundary, vertical slice shape, feedback loop 또는 TDD trace, codebase stewardship, context hygiene, follow-up risk? |

Same-session review는 유용한 self-check일 수 있지만 detached verification이 아니며 `assurance_level=detached_verified`로 표시하면 안 됩니다.

## AFK와 public commitment 표시

AFK, unattended, 또는 "내가 없는 동안 계속해" 지시는 connector 표시와 진행 자세에 관한 것이며 새 권한을 만들지 않습니다. Connector는 AFK 작업을 active Change Unit, active Autonomy Boundary, granted sensitive-action Approvals, 실제 제품 파일 쓰기 전 compatible `prepare_write` / Write Authorization 안에 유지해야 합니다.

Surface는 scope expansion, Autonomy Boundary breach, Approval 없는 새 sensitive action, residual-risk acceptance, final acceptance, QA 또는 verification waiver, public API 또는 module contract 변경, release/support promise, 문서 독자가 의존할 내용을 바꾸는 documentation promise, 사용자 소유 제품 판단 또는 중요한 기술 판단이 필요한 다른 public commitment 전에 멈추고 가장 작은 unblocker를 보여줘야 합니다.

멈춤 표시는 capability profile에 맞춰야 합니다. Cooperative profile에서는 connector가 agent에게 hold를 지시합니다. Detective profile에서는 실행 뒤 mismatch를 감지하고 보고할 수 있는 validation도 설명할 수 있습니다. Preventive wording은 fixture로 입증된 pre-tool blocking이 해당 operation을 cover할 때만 허용됩니다. Isolated wording은 입증된 separation boundary를 사용하는 경우에만 허용됩니다.

## 기준 접점 계약

v0.1 Kernel MVP는 하나의 기준 접점을 목표로 합니다. 기준 접점은 넓은 ecosystem 지원이 아니라 kernel을 증명해야 합니다.

Minimum reference expectations:

- public tool과 resource에 `T2 MCP` 사용 가능
- product write 전 cooperative `prepare_write`
- run 이후 detective changed-path와 artifact validation
- evidence manifest에 충분한 run summary와 artifact capture
- manual verification bundle 또는 fresh evaluator instruction
- Manual QA note artifact support
- generated file, managed block, MCP config snippet, profile freshness용 connector manifest
- native capture가 없을 때 manual artifact capture fallback
- captured 또는 manually supplied artifact의 artifact integrity status
- guard, freeze, careful-mode label이 표시될 때 실제 차단 가능 범위와 사후 감지 범위 표시
- [운영과 Conformance 참조](operations-and-conformance.md#doctor)가 이름 붙인 common state, MCP availability, surface capability와 mismatch handling, generated-file drift, reconcile, artifact integrity, artifact/capture fallback, stale context, evaluator bundle freshness, projection freshness, security/threat-model category를 다루는 conformance smoke

Reference surface 동작 세부사항과 접점별 설정은 concrete surface를 이름으로 부를 때만 [Surface Cookbook](surface-cookbook.md)에 둡니다.

## Connector Conformance 개요

Connector conformance는 profile이 선언한 capability tier에서 공통 contract를 지킬 수 있음을 입증해야 합니다.

Overview scenario:

- active Task가 있을 때와 없을 때의 status
- Use procedure가 요구할 때 significant work 재개 전 current Journey Card 표시
- intake를 `advisor`, `direct`, `work`로 분류하고, 사용자에게 보일 때는 읽기/조언, 작은 변경, 추적되는 작업으로 표시
- shared design과 decision을 포함한 work shaping
- Change Unit scope와 vertical/horizontal exception handling
- 가능할 때 recommendation과 uncertainty가 있는 one blocking question
- 차단하는 사용자 소유 판단에 broad approval 대신 Decision Packet 표시
- Autonomy Boundary breach가 stop하거나 Decision Packet으로 route
- AFK work가 active Change Unit scope, Autonomy Boundary latitude, 적용되는 granted sensitive-action Approval, 실제 product write 전 compatible `prepare_write` / Write Authorization 안에 머무르고, stop wording이 입증된 guarantee level과 맞음
- public commitment가 사용자 소유 제품 판단 또는 중요한 기술 판단을 필요로 하면 Decision Packet 또는 다른 기존 owner path로 route
- `prepare_write` allowed/blocked path
- allowed write에 Write Authorization 생성 및 Write Authority Summary 표시
- write-capable `record_run`이 compatible Write Authorization consume
- sensitive-action Approval request, granted, denied, expired path
- artifact와 evidence update를 포함한 `record_run`
- `DIRECT-RESULT` projection
- verification launch 또는 manual verification bundle
- same-session verification guard
- detached verification 전 evaluator bundle freshness
- Manual QA required, passed, failed, waived
- product/user risk가 있는 QA 면제를 Decision Packet으로 route
- acceptance required와 recorded
- approval, QA, verification waiver, final acceptance, residual-risk acceptance가 서로 다른 판단으로 남음
- acceptance 또는 successful close 전 close-relevant residual risk visible
- risk-accepted close에는 accepted Residual Risk refs 추가 요구
- 최신이 아닌 projection과 reconcile flow
- stale projection write guard
- generated file drift detection
- generated file과 managed block의 safe non-overwrite 동작 및 reconcile로의 drift routing
- connector manifest profile freshness와 stale capability profile detection
- version, MCP config, hook, permission, workspace policy, generated-file, conformance-result, capture-method, QA-capture-method, redaction-policy, artifact-retention 변경 이후 profile refresh
- required tier가 없을 때 capability fallback
- surface capability mismatch가 unsafe write를 보류하고 낮아진 guarantee를 보고
- local-only MCP 기본값과, profile 밖 remote/shared 노출이 held, failed, 또는 capability-insufficient로 보고되는 동작
- MCP unavailable product-write 보류
- stale PRD, stale chat memory, 기타 pull-only context가 owner path로 reconcile되기 전에는 write를 허가하거나, gate를 충족하거나, 결과를 수락하거나, Task를 close하지 않는 동작
- artifact integrity mismatch가 dependent evidence, verification, export, close-readiness claim을 stale, blocked, insufficient 상태로 유지
- status/next recommendation과 Role Lens output이 추천된 action이 기존 Core mutation path를 따르기 전까지 read-only guidance로 남는 동작

정확한 fixture 형식은 [Conformance Fixtures 참조](conformance-fixtures.md)가 담당하고, operator command 의미는 [운영과 Conformance 참조](operations-and-conformance.md)가 담당합니다.
