# 에이전트 통합 참조

## 이 문서로 할 수 있는 일

향후 하네스 동작에 에이전트 접점을 연결할 때, 그 접점이 실제로 보장할 수 있는 수준을 과장하지 않도록 이 참조를 사용합니다.

이 참조는 공통 커넥터 계약을 담당합니다. Capability tier, capability profile, generated manifest 기대사항, Context Push/Pull Principles, Fallback Semantics, Role Lens 동작, 기준 접점 기대사항, connector conformance 개요를 다룹니다.

사용자 세션에서 에이전트가 무엇을 말하고 어떻게 행동해야 하는지는 [에이전트 가이드](../use/agent-guide.md)를 봅니다. 접점별 설정 메모는 [Surface Cookbook](surface-cookbook.md)을 봅니다. 현재 저장소 단계와 구현 인계 상태는 [구현 개요](../build/implementation-overview.md#문서-수락-상태)가 담당합니다.

## 이런 때 읽기

- 에이전트 접점용 커넥터를 구현하거나 검토할 때.
- 접점 capability profile을 선언, 갱신, 점검할 때.
- 보장 수준, guard, freeze, fallback, MCP availability를 정직하게 표시해야 할 때.
- Connector conformance coverage를 작성할 때.
- 공통 계약과 접점별 recipe의 경계를 확인해야 할 때.

## 읽기 전에

행동 규칙은 [에이전트 가이드](../use/agent-guide.md)를 읽습니다. 정확한 소유자 섹션은 커넥터 행동에 필요할 때만 가져옵니다. 쓰기와 닫기 권한은 [Core Model 참조](core-model.md), active MVP-1 method contract는 [MVP API](api/mvp-api.md), shared shape는 [API Schema Core](api/schema-core.md), public error behavior는 [API Errors](api/errors.md), threat와 guarantee wording은 [보안 참조](security.md)를 사용합니다.

이 참조는 모든 Reference 문서를 에이전트 맥락에 넣으라는 지시가 아닙니다.

## 핵심 생각

커넥터는 에이전트에게 작고 최신인 맥락을 주고, 상태를 바꾸는 행동은 하네스로 라우팅하고, 접점이 할 수 있으면 실제로 일어난 일을 캡처하며, 실제 사용 중인 접점 프로필에서 입증된 보장만 보고합니다.

접점 이름은 capability가 아닙니다. 협력형, 사후 확인, 사전 차단, 격리형 동작은 실제 host, profile, version/configuration, workspace policy, MCP posture, capture path, guard path, separation boundary가 선언되고 확인된 범위에서만 주장할 수 있습니다.

## 통합을 쉬운 말로 설명하면

에이전트 접점은 사용자가 에이전트와 대화하는 곳입니다. 하네스는 범위, 사용자 판단, 쓰기 확인, 근거 참조, 작업 수락, 잔여 위험 수용, 닫기 준비 상태를 대화 기록 밖에 두는 로컬 권한 계층입니다.

공통 경로는 다음과 같습니다.

```text
user conversation surface
  -> short always-on rules/context
  -> Harness skill, command, or playbook
  -> Harness MCP server
  -> Harness Core
  -> adapter, hook, sidecar, validator, capture path, or isolation layer
```

항상 적용되는 커넥터 규칙은 짧아야 합니다. 언제 하네스를 쓰는지, current status 또는 에이전트 맥락 패킷을 어디서 읽는지, 제품 파일 쓰기에는 `prepare_write`를 쓴다는 점, 사용자 소유 판단은 판단 요청으로 라우팅한다는 점, 요구사항 구체화는 사용자에게 묻기 전에 repo/docs/current state를 확인한다는 점, 상태가 실제로 실행 전에 막을 수 있는 것과 나중에 감지할 수 있는 것을 구분해야 한다는 점, authoritative MCP를 사용할 수 없으면 제품 파일 쓰기를 보류한다는 점이면 충분합니다.

세션 절차는 [에이전트 가이드](../use/agent-guide.md)가 담당합니다. 커넥터 설정과 접점별 경로는 [Surface Cookbook](surface-cookbook.md)이 담당합니다.

## Use 문서와 Reference 문서의 경계

| 영역 | 담당 |
|---|---|
| 사용자 세션에서 에이전트가 무엇을 보여주고, 묻고, 말해야 하는지 | [에이전트 가이드](../use/agent-guide.md) |
| 범위, 근거, 검증, QA, 잔여 위험, 닫기에 대한 사용자용 설명 | [사용자 가이드](../use/user-guide.md) |
| 공통 커넥터 capability, 맥락, fallback, conformance 계약 | 이 참조 |
| 구체적인 접점별 recipe | [Surface Cookbook](surface-cookbook.md) |
| Public MCP request/response schema | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md), [API Schema Later](api/schema-later.md) |
| Core state transition과 write/close rule | [Core Model 참조](core-model.md) |
| Security guarantee 의미 | [보안 참조](security.md#정직한-guarantee-display) |

## Capability Tiers

| Tier | 의미 | 대표 capability |
|---|---|---|
| `T0 Context` | 접점이 하네스 원칙을 읽을 수 있습니다. | rules/context file |
| `T1 Skill` | 접점이 하네스 절차를 따를 수 있습니다. | skill, command, prompt, playbook |
| `T2 MCP` | 접점이 하네스 tool과 resource를 호출할 수 있습니다. | MCP server connection |
| `T3 Capture` | 접점이 diff, log, run output을 신뢰할 만하게 반환할 수 있습니다. | structured output, wrapper, adapter |
| `T4 Guard` | Fixture coverage가 구체적인 경로를 입증한 경우, 접점이 대상 경로를 실행 전에 막거나 중단할 수 있습니다. | hook, permission system, policy engine, sidecar |
| `T5 Isolation` | 접점이 검증 또는 위험한 작업을 문서화된 separation boundary 뒤에서 실행할 수 있습니다. | worktree, sandbox, fresh process, isolated runner |
| `T6 QA Capture` | 접점이 browser, screenshot, walkthrough, workflow-recording, 수동 QA artifact를 구조화할 수 있습니다. | browser runner, screenshot capture, console/network capture, accessibility snapshot, QA note capture |

내부 엔지니어링 점검과 MVP-1 커넥터는 구체적인 프로필이 더 강한 capability를 입증하지 않는 한 협력형 또는 사후 확인 동작을 전제로 삼아야 합니다. `T4`와 `T5`는 기본 OS 격리, 임의 도구 sandboxing, 변조 방지 파일, 도구 실행 전 차단을 뜻하지 않습니다.

## Capability Profiles

커넥터는 제품명, 접점 이름, mode label에서 동작을 가정하지 않고 capability profile을 사용해야 합니다. Profile은 실제 작업을 실행할 host/profile에 한정됩니다.

모든 profile은 커넥터가 소유한 field 이름으로 다음 사실을 담아야 합니다.

- surface id, surface kind, target profile, 사용할 수 있으면 detected version, profile version, last verified time
- support tier와 guarantee level
- MCP tool/resource availability와 local exposure posture
- raw secret이나 private configuration value가 없는 access-control material class
- capture, QA capture, guard, isolation, changed-path detection, redaction, artifact-retention behavior
- 알려진 risk와 fallback
- 선언을 최신으로 만든 conformance result 또는 operator check

예시 모양:

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

Version, MCP config, hook, permission, workspace policy, generated file, managed block, conformance result, capture method, QA capture method, redaction policy, artifact retention, access-control material class, local bind/reachability posture, guard wrapper, isolation wrapper가 바뀌면 profile을 갱신합니다.

Local 범위를 넘는 MCP 노출은 owner docs가 승격하고 증명하기 전까지 내부 엔지니어링 점검 baseline과 staged delivery 밖에 둡니다. 그런 근거 없이 remote 또는 shared MCP exposure를 안전한 기본값처럼 표시하면 안 됩니다.

## Guarantee Levels

보장 수준 표시는 [보안 참조](security.md#정직한-guarantee-display)를 따릅니다. 이 참조는 connector profile이 그 수준을 어떻게 보고하는지 담당합니다.

| Level | 표시 책임 |
|---|---|
| `cooperative` | 접점이 하네스 지시를 따르도록 기대된다고 말합니다. 보류는 지시로 이루어지며 물리적 차단이 아닙니다. |
| `detective` | 하네스가 행동 뒤에 changed path, log, artifact, drift를 관찰하고 상태를 stale, partial, blocked, failed로 표시할 수 있다고 말합니다. |
| `preventive` | Fixture로 입증된 hook, wrapper, permission layer, policy engine, sidecar path와 실행 전에 막을 수 있는 covered operation을 이름 붙입니다. |
| `isolated` | 문서화된 separation boundary를 이름 붙입니다. 해당 profile이 정확한 mechanism을 입증하지 않으면 OS sandboxing, 권한 격리, 변조 방지 저장소를 암시하지 않습니다. |

Guard, freeze, careful-mode label은 실제 profile 위의 표시 label입니다. 무엇을 실행 전에 실제로 막을 수 있고 무엇을 나중에만 감지할 수 있는지 말해야 합니다. 이것들은 민감 동작 승인, 검증, 작업 수락, 잔여 위험 수용, 닫기 준비 상태, kernel gate가 아닙니다.

## Generated Manifest 기대사항

커넥터는 rules, skill, MCP config snippet, prompt, adapter file을 생성할 수 있습니다. 생성되거나 관리되는 path, managed block, MCP snippet, profile freshness marker는 connector manifest에 기록되어야 합니다.

Manifest는 다음을 해야 합니다.

- 생성/관리 path를 이름 붙입니다.
- managed block id와 hash를 기록합니다.
- 생성 당시 사용한 capability profile을 기록합니다.
- raw token, secret, private config, omitted secret value, blocked payload byte 없이 MCP exposure posture와 display-safe handle을 기록합니다.
- profile이 증명하는 것보다 크게 주장하지 않고 capture, QA capture, guard, isolation, fallback behavior를 기록합니다.
- 관련 surface, configuration, policy, generated file, conformance, capture, redaction, artifact-retention, guard, isolation 근거가 바뀌면 profile 또는 block을 stale로 표시합니다.
- 사람이 편집한 내용을 덮어쓰기 전에 drift를 감지합니다.
- 필요하면 drift를 reconcile로 보냅니다.
- 편집된 generated file이 canonical Task state가 아님을 보고합니다.

접점별 generated filename은 [Surface Cookbook](surface-cookbook.md)이 담당합니다.

## Context Push/Pull Principles

커넥터는 작은 현재 맥락을 push하고, 더 큰 reference는 다음 행동에 필요할 때만 pull해야 합니다. Context packet은 운영 지원 맥락이지 agent memory, chat history, old projection text, full report, complete reference dump가 아닙니다.

항상 주입되는 에이전트 맥락은 한 화면 안팎이어야 하며 다음만 포함합니다.

- 현재 Task 요약 또는 명시적인 `none` / `unknown`
- 작업 모양
- 범위와 비목표
- 대기 중인 사용자 판단
- 활성 막힘
- 다음 안전한 행동
- 근거 공백
- 닫기 막힘
- 잔여 위험 요약
- 보장 수준
- 출처 참조와 최신성

기본으로 push하지 않는 것: 전체 Reference 문서, 전체 schema, 전체 Storage DDL, complete history, historical event log, 읽기용 요약 전체 본문, artifact 전체 본문, raw log/screenshot/diff/trace, 전체 template, 관련 없는 template, future catalog, 오래된 task history, 관련 없는 Roadmap material.

단계별 pull 맥락을 사용합니다.

| 단계 | 최소 pull 대상 |
|---|---|
| 세션 시작 / 이어가기 | 현재 `harness.status`, 현재 Task/status resource, [에이전트 가이드: 상태 보고](../use/agent-guide.md#10-상태-보고). |
| 계획 / 요구사항 구체화 | 현재 저장소/문서/상태 참조와 [에이전트 가이드: 요구사항 구체화](../use/agent-guide.md#4-요구사항-구체화). |
| 작업 모양 분류 | 현재 범위/상태 참조와 [에이전트 가이드: 작업 모양 분류](../use/agent-guide.md#3-작업-모양-분류). |
| 판단 요청 | 현재 판단 참조 또는 후보와 [에이전트 가이드: 판단 요청](../use/agent-guide.md#5-판단-요청). |
| 쓰기 준비 | 현재 범위/상태와 [에이전트 가이드: 쓰기 전 범위 확인](../use/agent-guide.md#8-쓰기-전-범위-확인), 그리고 의도한 쓰기에 필요한 `prepare_write` 소유자 섹션. |
| 실행 / 근거 | 현재 Run/아티팩트 참조와 [에이전트 가이드: 근거 기록](../use/agent-guide.md#9-근거-기록). |
| 닫기 준비 상태 | 현재 소유자 기록과 [에이전트 가이드: 닫기](../use/agent-guide.md#11-닫기). |
| 복구 / 오류 | 현재 사용 가능 여부/최신성 상태, [Fallback Semantics](#fallback-semantics), 특정 오류 소유자 섹션. |

상태 카드, 읽기용 요약, 렌더링된 template, 추천, 검색된 맥락, 대화 기억은 읽기 전용입니다. 무엇을 확인할지 가리킬 수 있지만 쓰기를 허가하거나, gate를 만족하거나, 근거를 만들거나, 사용자 판단을 해소하거나, 민감 동작 승인을 주거나, 검증을 수행하거나, QA를 기록하거나, 작업을 수락하거나, 잔여 위험을 받아들이거나, 읽기용 요약 최신성을 복구하거나, Task를 닫을 수 없습니다.

토큰을 아낀다는 이유로 사용자 소유 판단, 막힘, 범위 제한, 안전 경계, 근거 공백, 닫기 막힘, 닫기 관련 잔여 위험을 숨기면 안 됩니다.

## Fallback Semantics

Fallback은 접점 이름이 아니라 보장 수준과 risk로 설명합니다.

| Fallback | 쓰는 경우 | 경계 |
|---|---|---|
| Cooperative | 접점이 지시를 따를 수 있지만 강제할 수 없을 때. | Authoritative MCP 또는 쓰기 범위 확인을 사용할 수 없으면 제품 파일 쓰기를 지시로 보류합니다. |
| Detective | 하네스가 행동 뒤 changed file, log, artifact, projection drift, artifact gap을 볼 수 있을 때. | 상태를 stale, partial, blocked, failed로 표시하고 repair, reconcile, fresh evidence를 요구합니다. |
| Preventive | Fixture로 입증된 hook, permission layer, wrapper, policy engine, sidecar가 실행 전에 막을 수 있을 때. | 입증된 blocking path가 cover하는 operation만 주장합니다. |
| Isolated | Risk 때문에 분리가 필요할 때. | Profile이 이름 붙인 documented boundary를 사용합니다. Separation만으로 민감 동작 승인, 검증, 작업 수락, 잔여 위험 수용, 닫기, assurance upgrade가 되지는 않습니다. |

MCP를 사용할 수 없으면 커넥터는 authoritative state update를 주장하면 안 됩니다. `MCP_SERVER_UNAVAILABLE`은 call path가 Core에 닿지 못한다는 뜻입니다. `SURFACE_MCP_UNAVAILABLE`은 연결된 접점이 usable MCP를 갖지 못했거나, MCP configuration이 오래됐거나, 필요한 tool을 호출할 수 없다는 뜻입니다. 이들은 diagnostic condition이며, `MCP_UNAVAILABLE`이 stable public availability code입니다.

Core에 닿을 수 없는 동안 chat memory, generated file, cached projection, old status text, operator prose에서 Core state, Write Authorization, gate status, approval, evidence, work acceptance, residual-risk acceptance, projection repair, close readiness를 만들어 내지 않습니다.

Projection staleness는 Core state와 분리해서 보고합니다. 커넥터가 current Core state를 직접 읽을 수 있으면 그 상태에서 계속할 수 있지만, 오래된 읽기용 요약에 의존하는 행동은 먼저 refresh 또는 reconcile해야 합니다.

이 문서 전용 저장소의 문서 유지보수 편집은 Authoring Guide가 관리하며 런타임 하네스 절차가 아닙니다. 그런 편집은 runtime state, Write Authorization, evidence, QA, acceptance, residual-risk acceptance, close readiness, projection, `task_events`, runtime transition을 만들지 않습니다.

## Role Lens 동작

Role Lens는 사용자가 익숙한 review posture로 에이전트를 조정하도록 돕는 비권한 skill 또는 playbook 접점입니다. 예: product review, engineering review, design review, security review, QA review, release handoff.

Lens는 사용자 판단, 근거 수집, 검증, 수동 QA, 민감 동작 permission, 잔여 위험 처리, 범위 업데이트, 다음 playbook을 추천할 수 있습니다. 추천은 기존 MCP/Core mutation path가 실제 행동을 기록하기 전까지 읽기 전용입니다.

같은 세션 review는 자체 확인 맥락입니다. Active verification owner path가 조건을 충족하기 전에는 detached verification으로 표시하면 안 됩니다.

## AFK와 public commitment 표시

AFK, unattended, 또는 "내가 없는 동안 계속해" 지시는 새 권한을 만들지 않습니다. 제품 파일 쓰기는 활성 범위, 활성 autonomy boundary, 필요할 때 부여된 민감 동작 permission, 호환되는 `prepare_write` / Write Authorization 안에 남아야 합니다.

범위 확장, 새 민감 동작, QA 또는 검증 면제, 작업 수락, 잔여 위험 수용, public API 또는 module contract 변경, release/support promise, 독자가 의존할 수 있는 내용을 바꾸는 문서 약속, 그 밖의 사용자 소유 제품 또는 중요한 기술 판단이 필요한 public commitment 전에는 멈추고 가장 작은 해소 방법을 보여줍니다.

## 기준 접점 계약

내부 엔지니어링 점검은 하나의 로컬 프로젝트 등록과 Core authority path를 확인하는 데 필요한 기준 접점 support만 사용합니다. 이 섹션의 later bullet은 profile target이지 내부 엔지니어링 점검 requirement가 아닙니다.

내부 엔지니어링 점검의 최소 기준 기대:

- 첫 authority loop에 필요한 public tool/resource subset을 위한 `T2 MCP`
- local-only 또는 owner-approved access posture
- 제품 파일 쓰기 전 cooperative `prepare_write`, write-capable `record_run` 전 compatible Write Authorization
- run 뒤 changed-path와 artifact validation을 사후 확인
- 기본 OS sandbox, arbitrary-tool sandboxing, tamper-proof local file, pre-tool blocking claim 없음
- 최소 authority loop를 위한 run summary와 수동 제공 또는 capture된 artifact/evidence ref 하나 이상
- guard, freeze, careful-mode label을 표시할 때 pre-action stop과 after-action detection을 정직하게 구분

Later profile target에는 user-readable status/next card, 작은 사용자 판단 표시, 근거와 닫기 준비 상태 요약, Evidence Manifest support, manual verification bundle 또는 fresh evaluator instructions, 수동 QA note/artifact support, connector manifest, projection freshness, reconcile flow, operator diagnostics가 포함됩니다.

## Connector Conformance 개요

Connector conformance는 profile이 선언한 capability tier에서 공통 계약을 지킬 수 있음을 증명해야 합니다. Scenario list는 aggregate profile map이지 하나의 내부 엔지니어링 점검 checklist가 아닙니다.

내부 엔지니어링 점검 connector check에는 다음이 포함됩니다.

- active Task가 있을 때와 없을 때 status
- Use procedure가 요구하는 경우 significant resume 전 compact current-position status
- 선택된 path/tool/command에 대한 basic scope checking
- `prepare_write` allowed/blocked path
- 허용된 write에 대한 Write Authorization 생성과 write-capable `record_run`의 소비
- minimal artifact/evidence ref가 있는 `record_run`
- local-only MCP default 또는 owner-approved alternative
- MCP unavailable product-write hold
- 추천 action이 이후 Core mutation path를 따르기 전까지 read-only status recommendation
- guard, freeze, careful-mode label에 대한 honest guarantee display

Later profile scenario에는 선택지와 결과가 있는 사용자 판단 라우팅, 민감 동작 permission path, full Change Unit handling, evidence와 artifact integrity, verification bundle, 수동 QA, 작업 수락, 잔여 위험 표시와 수용, stale projection/reconcile flow, generated-file drift, capability fallback, stale context refusal, surface capability mismatch handling이 포함됩니다.

Exact fixture format은 [Conformance Fixtures 참조](conformance-fixtures.md)가 담당하고, operational command는 [Operations And Conformance 참조](operations-and-conformance.md)가 담당합니다.
