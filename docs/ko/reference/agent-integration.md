# 에이전트 통합 참조

에이전트 접점을 향후 하네스 동작에 연결할 때 이 참조를 사용합니다. 목표는 맥락 비용을 낮게 유지하고, 보장 수준을 정직하게 표시하며, 사용자가 소유하는 판단을 보존하는 것입니다. 이 저장소는 아직 문서 전용이며 문서 검토 단계입니다. 이 문서는 계획된 하네스 동작을 설명할 뿐, 런타임 서버나 커넥터 구현이 이미 있다는 뜻이 아닙니다.

사용자 세션에서 에이전트가 무엇을 말해야 하는지는 [에이전트 가이드](../use/agent-guide.md)를 봅니다. Core, API, schema, storage, projection, security, conformance, operations의 정확한 계약은 다음 행동에 필요한 owner section만 가져옵니다.

## 1. 담당하는 것 / 담당하지 않는 것

이 참조가 담당하는 것:

- 에이전트 접점의 `capability_profile`
- 연결된 접점의 보장 수준 표시
- 맥락 주입 / 필요할 때 가져오기 규칙과 항상 주입되는 맥락 예산
- 저렴한 검색을 위한 단계별 맥락 profile
- 커넥터 경계에서의 판단 요청 동작
- 접점이 lens를 쓸 때의 Role Lens 비권한 동작
- Core, MCP, projection, capability를 사용할 수 없을 때의 대체 동작
- 에이전트가 어떤 맥락을 넣을지 판단하게 돕는 짧은 접점 recipe
- 커넥터 conformance 경계

이 참조가 담당하지 않는 것:

- 사용자 세션 절차. [에이전트 가이드](../use/agent-guide.md)를 봅니다.
- 범위, 증거, QA, 최종 수락, 잔여 위험, 닫기에 대한 사용자 대상 설명. [사용자 가이드](../use/user-guide.md)를 봅니다.
- Core 상태 전이, gate, `prepare_write`, Write Authorization, `record_run`, `close_task`. [Core Model 참조](core-model.md)를 봅니다.
- 공개 MCP method 계약, schema, public error. [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)를 봅니다.
- Storage DDL, 지속 상태, artifact layout. [Storage](storage.md)를 봅니다.
- Projection/template 권한과 active rendered template body. [Projection과 Template 참조](projection-and-templates.md)를 봅니다.
- 위협 모델과 guarantee-level 의미. [보안 참조](security.md)를 봅니다.
- future fixture 형태 또는 주장 권한. [적합성 참조](conformance.md)를 봅니다.
- 활성 Reference 범위로서의 운영자 명령과 진단. 향후 운영 후보는 [Later 후보 색인: 운영 후보](../later/index.md#operations-candidates)에 남습니다.
- 향후 커넥터 marketplace, 호스팅 에이전트 가정, broad connector ecosystem, 여러 접점 orchestration

이 문서의 접점 recipe는 통합 지침입니다. Core 상태 권한, 쓰기 권한, 증거, 검증, QA, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 닫기 준비 상태, 새로운 보안 권한 경계를 만들지 않습니다.

<a id="role-lens-behavior"></a>

Role Lens가 쓰일 때 그 출력은 읽기 전용 검토 자세 안내입니다. Lens는 판단 요청, 증거 수집, 검증, 수동 QA, 민감 동작 승인, 잔여 위험 처리, 범위 업데이트, 다음 playbook을 추천할 수 있습니다. 하지만 owner API/Core path가 실제 행동을 기록하기 전까지 추천에는 권한이 없습니다.

<a id="capability-profiles"></a>

## 2. `capability_profile`

접점 이름 자체는 역량이 아닙니다. 커넥터는 실제로 쓰는 host, version/configuration, workspace policy, MCP posture, capture path, guard path, separation boundary에 맞춘 `capability_profile`을 사용해야 합니다.

`capability_profile`은 쓰기 권한이 아니며 active Task 범위, active Change Unit 범위, `prepare_write`, 한 번만 쓰는 협력형 Write Authorization, `record_run`, Core close rule을 우회하지 않습니다. 역량 정보는 차단 사유, 대체 동작, validator result, 보장 수준 표시에 영향을 줍니다. `allowed`와 `blocked`는 증명된 preventive profile이 covered operation을 이름 붙이지 않는 한 하네스 호환성 결과입니다.

활성 기준 `capability_profile`은 의도적으로 작습니다.

```yaml
capability_profile:
  surface_id: reference-local-mcp
  surface_name: Reference local MCP surface
  mcp_available: true
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
```

정확한 public tool과 resource 계약은 API owner가 담당합니다. 커넥터는 사용할 수 있는 subset을 요약할 수 있지만, 전체 method schema를 모델에 전달되는 맥락에 중복해서 넣으면 안 됩니다.

접점 version, MCP configuration, hook, permission, workspace policy, generated file, managed block, capture path, QA capture path, redaction policy, artifact retention, local access posture, guard wrapper, isolation wrapper, conformance 근거가 바뀌면 `capability_profile`을 갱신합니다.

<a id="generated-manifest-expectations"></a>

Generated rule, skill, MCP snippet, adapter file, managed block에는 connector manifest가 필요합니다. Manifest는 generated path, managed block id와 hash, MCP exposure posture, 표시해도 안전한 handle, profile freshness, drift, 대체 동작을 기록합니다. Raw token, secret, private config value, blocked payload byte, canonical Task state는 저장하지 않습니다.

## 3. 보장 수준

보장 수준 표시는 [보안 참조](security.md#정직한-guarantee-display)를 따릅니다. 이 참조는 커넥터가 `capability_profile`을 사용자에게 보이는 표시로 어떻게 연결하는지 담당합니다.

| Level | 커넥터 표시 규칙 |
|---|---|
| `cooperative` | 접점이 하네스 지시를 따를 것으로 기대된다고 말합니다. 보류는 지시로 이루어지며 물리적 차단이 아닙니다. |
| `detective` | 하네스가 행동 뒤에 changed path, log, artifact, drift 같은 지원되는 사실을 보고 상태를 stale, partial, blocked, failed로 표시할 수 있다고 말합니다. |
| `preventive` | Fixture로 입증된 hook, wrapper, permission layer, policy engine, sidecar path와 실행 전에 막을 수 있는 정확한 operation을 이름 붙입니다. |
| `isolated` | 문서화된 separation boundary를 이름 붙입니다. 해당 메커니즘이 입증되지 않았으면 운영체제 샌드박싱, 권한 격리, 변조 방지 저장소를 암시하지 않습니다. |

Reference local MCP profile은 `cooperative` 동작과, changed-path 또는 artifact-gap 관찰이 뒷받침하는 제한된 `detective` 동작만 표시할 수 있습니다. `pre_tool_blocking_supported=false`와 `isolation_supported=false`이므로 `preventive`나 `isolated` 동작을 주장하면 안 됩니다.

Guard, freeze, careful-mode label은 실제 `capability_profile` 위에 놓이는 표시 label입니다. 무엇을 실행 전에 실제로 멈출 수 있고 무엇은 나중에만 감지할 수 있는지 말해야 합니다. 이것들은 민감 동작 승인, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태, Core gate가 아닙니다.

현재 MVP가 문서화하지 않은 보안 보장을 주장하지 않습니다. 하네스는 기본 OS 권한, 임의 도구 샌드박스, 변조 방지 로컬 파일, 도구 실행 전 차단, 보안 격리를 제공하지 않습니다.

<a id="context-pushpull-principles"></a>

## 4. 맥락 주입 / 필요할 때 가져오기

커넥터는 작고 최신인 현재 맥락을 주입하고, 큰 owner 문서는 다음 행동에 필요할 때 가져오기 방식으로 사용해야 합니다. Context packet은 다음 에이전트 행동을 돕는 운영 지원 맥락입니다. 에이전트 기억, 대화 기록, 전체 보고서, 캐시된 projection 본문, Reference 전체 덤프가 아닙니다.

검색 비용 원칙:

- 전체 Reference set을 기본으로 주입하지 않습니다.
- 전체 schema를 기본으로 주입하지 않습니다.
- 전체 Storage DDL, 전체 template, 전체 projection body, complete history, 전체 event log, raw artifact content, raw log, raw screenshot, raw trace, 관련 없는 Roadmap material을 기본으로 주입하지 않습니다.
- 향후/later catalog material을 기본으로 주입하지 않습니다.
- 다음 행동에 필요한 owner section만 가져오고 멈춥니다.
- 일반 작업 prompt에서는 하나의 언어를 고릅니다. 같은 `doc_id`의 영어/한국어 대응 문서를 한 prompt에 함께 넣지 않습니다. 이중 언어 검토는 대응 문서 전체를 밀어 넣지 말고 필요한 section만 비교합니다.

상태 카드, projection, rendered template, 검색된 맥락, 추천, 대화 기억은 읽기 전용입니다. 무엇을 확인할지 가리킬 수는 있지만 쓰기를 승인하거나, gate를 만족하거나, 증거를 만들거나, 사용자 판단을 해결하거나, 민감 동작 승인을 부여하거나, 검증을 수행하거나, QA를 기록하거나, 결과를 수락하거나, 잔여 위험을 수락하거나, projection freshness를 고치거나, Task를 닫을 수 없습니다.

토큰을 아끼기 위해 사용자가 소유하는 판단, 범위 제한, 차단 사유, 안전 경계, 증거 공백, 닫기 차단 사유, 닫기와 관련된 잔여 위험을 숨기면 안 됩니다.

## 5. 항상 주입되는 맥락 예산

항상 주입되는 맥락은 한 화면 안팎이어야 합니다. 현재 행동에 필요한 상태만 포함합니다.

- 현재 Task 요약 또는 명시적인 `none` / `unknown`
- 작업 모양
- 범위와 비목표
- 대기 중인 사용자 판단
- 활성 차단 사유
- 다음 안전한 행동
- 증거 공백
- 닫기 차단 사유
- 잔여 위험 요약
- 보장 수준. Core 또는 필요한 MCP가 답할 수 없으면 unavailable/capability condition
- 출처 참조와 최신성

Reference 자료 전체, 전체 schema, 전체 DDL, projection 본문 전체, artifact body 전체, 관련 없는 template, 향후 catalog, 최신이 아니거나 관련 없는 task history, 과거 log를 항상 주입되는 맥락에 넣지 않습니다.

## 6. 단계별 맥락 profile

다음 질문에 답하는 가장 좁은 profile을 사용합니다.

| 단계 | 이것만 가져오기 |
|---|---|
| 세션 시작 / 이어가기 | 현재 `harness.status`, 현재 Task/status resource, [에이전트 가이드: 사용자의 다음 결정을 위한 상태 보고](../use/agent-guide.md#8-사용자의-다음-결정을-위한-상태-보고). |
| 계획 / 요구사항 구체화 | 현재 repo/docs/state 참조와 [에이전트 가이드: 끝없는 계획 루프 없이 구체화하기](../use/agent-guide.md#4-끝없는-계획-루프-없이-구체화하기). |
| 쓰기 준비 | 현재 scope/state, [에이전트 가이드: 제품 파일 쓰기 전에 범위 확인](../use/agent-guide.md#6-제품-파일-쓰기-전에-범위-확인), 의도한 쓰기에 필요한 `prepare_write` owner section만. |
| 실행 / run 기록 | 현재 Write Authorization, run/evidence ref, [에이전트 가이드: 의미 있는 행동 뒤에는 증거 기록](../use/agent-guide.md#7-의미-있는-행동-뒤에는-증거-기록). |
| 증거 검토 | 현재 evidence ref, artifact ref, freshness fact, 빠진 증거, 필요할 때만 정확한 evidence 또는 projection owner section. |
| 닫기 준비 상태 | 현재 owner record, blocker, 잔여 위험 요약, [에이전트 가이드: 정직하게 닫기](../use/agent-guide.md#10-정직하게-닫기). |
| 사용자 판단 요청 | 현재 judgment ref 또는 후보, 결과, 불확실성, [에이전트 가이드: 판단 요청은 좁고 분명하게](../use/agent-guide.md#5-판단-요청은-좁고-분명하게). |
| 복구 / 오류 | 현재 availability/freshness 상태, [대체 동작](#8-대체-동작), 특정 error owner section. |

행동에 엄격한 계약이 필요하면 owner section을 link하거나 가져옵니다. 일반 prompt에 owner 문서 전체를 붙여 넣지 않습니다.

## 7. 판단 요청 동작

에이전트는 사용자가 소유하는 판단을 보존합니다. 커넥터는 요청을 형식화하고, 응답을 모으고, owner API path로 기록을 라우팅할 수 있습니다. 하지만 사용자 대신 결정하면 안 됩니다.

판단 요청은 다음을 보존해야 합니다.

- 사용자가 해야 하는 판단
- 가능한 선택지
- 결과와 장단점
- 불확실성 또는 빠진 증거
- 에이전트 추천이 있다면 그 추천
- 에이전트가 사용자 대신 결정하지 않는 것
- 짧은 prompt인지(`presentation=short`), full Decision Packet 형식 표시인지(`presentation=full`)

에이전트는 최종 수락, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 잔여 위험 수락을 사용자 대신 결정하면 안 됩니다. 사용자 소유 제품 판단, 중요한 기술 판단, 범위 확장 판단도 조용히 대신하면 안 됩니다. 넓은 의미의 "좋아" 또는 "계속해" 메시지는 필요한 judgment path를 대신하지 않습니다.

Judgment record는 증거, 검증, 수동 QA, 최종 수락, 잔여 위험, 닫기 준비 상태와 분리됩니다. 어느 하나도 다른 하나를 대신하지 않습니다.

<a id="8-fallback-behavior"></a>

## 8. 대체 동작

대체 동작은 접점 이름이나 브랜드가 아니라 보장 수준과 risk로 설명합니다.

| 대체 동작 | 쓰는 경우 | 경계 |
|---|---|---|
| Cooperative | 접점이 지시를 따를 수 있지만 강제할 수 없을 때. | Authoritative MCP 또는 쓰기 범위 확인을 사용할 수 없으면 제품 파일 쓰기를 지시로 보류합니다. |
| Detective | 하네스가 행동 뒤 지원되는 사실을 관찰할 수 있을 때. | 상태를 stale, partial, blocked, failed로 표시하고 repair, reconcile, fresh evidence를 요구합니다. |
| Capability insufficient | 요청한 쓰기, capture, guard, isolation, guarantee가 지원하지 않는 field에 의존할 때. | `CAPABILITY_INSUFFICIENT` 또는 structured blocked reason을 반환하고 표시 보장을 낮춥니다. |
| MCP unavailable | 접점 또는 호출 경로가 authoritative Core에 닿지 못할 때. | 안정적인 공개 `MCP_UNAVAILABLE` 동작을 사용하고 상태 변경을 주장하지 않습니다. |
| Local access mismatch | Caller 또는 transport가 registered local profile 밖일 때. | 표시해도 안전한 diagnostic과 함께 `LOCAL_ACCESS_MISMATCH`를 사용합니다. 접점별 `UNAUTHORIZED` code를 만들지 않습니다. |

`MCP_SERVER_UNAVAILABLE`과 `SURFACE_MCP_UNAVAILABLE`은 diagnostic condition입니다. `MCP_UNAVAILABLE`은 안정적인 공개 availability code로 남습니다.

Core에 닿을 수 없는 동안 chat memory, generated file, cached projection, stale status text, operator prose에서 Core state, Write Authorization, gate status, approval, evidence, final acceptance, residual-risk acceptance, projection repair, close readiness를 만들어 내지 않습니다.

Projection staleness는 Core state와 분리됩니다. 커넥터가 current Core state를 직접 읽을 수 있으면 그 상태에서 계속할 수 있습니다. Stale projection에 의존하는 행동은 먼저 refresh 또는 reconcile해야 합니다.

이 문서 전용 저장소의 문서 유지보수 편집은 [문서 작성 가이드](../maintain/authoring-guide.md)가 관리합니다. Runtime Harness procedure가 아닙니다. 그런 편집은 runtime state, Write Authorization, evidence, QA, acceptance, residual-risk acceptance, close readiness, projection, `task_events`, runtime transition을 만들지 않습니다.

## 9. Surface recipes

Surface recipe는 에이전트가 어떤 맥락을 포함할지 판단하게 돕는 짧은 통합 메모입니다. 별도 reference owner가 아니며 긴 접점별 workflow로 커지면 안 됩니다.

Recipe에는 아래만 둡니다.

- 대상 `capability_profile`
- generated 또는 managed instruction/config path가 있으면 그 path
- MCP posture와 표시해도 안전한 handle
- profile refresh가 필요한 surface-specific capability 차이
- 해당 profile이 입증한 capture, guard, isolation fact
- required capability가 없을 때의 대체 동작
- 해당 profile의 conformance status

일반 Core 규칙, public API schema, Reference 문서 전체, 향후 커넥터 확장 구상, 호스팅 에이전트 가정, 감사 메모, 관련 없는 Roadmap 항목, projection 본문 전체, 긴 설정 튜토리얼을 넣지 않습니다.

<a id="reference-local-surface"></a>

Reference local MCP recipe:

```yaml
surface_kind: reference_local_mcp
target_profile: reference-local-mcp
mcp_posture: local-only registered project, or owner-approved alternative
context_strategy: compact always-on context plus phase-relevant owner pulls
write_behavior: cooperative prepare_write discipline before product writes
run_behavior: record_run with summary and owner-registered artifact refs
capture_boundary:
  native_capture: unsupported in the minimum reference profile
  fallback_capture: manual artifact attachment
guarantee_boundary:
  default_level: cooperative
  max_level: detective only for supported after-action observation
  can_block_before_execution: false
  isolation_supported: false
fallbacks:
  - hold product writes by instruction when MCP/Core is unavailable
  - lower claims or return CAPABILITY_INSUFFICIENT for unsupported capabilities
conformance_smoke_status: planned_not_run
```

`pre_tool_blocking_supported=false`이므로 "hold" 표현은 협력형 범위 규율과, 가능할 때의 탐지형 changed-path validation을 뜻합니다. 예방적 guard 동작이 아닙니다.

<a id="connector-conformance-overview"></a>

## 10. 커넥터 conformance 경계

커넥터 conformance는 선언된 `capability_profile`이 해당 capability level에서 이 공통 계약을 지킬 수 있음을 증명합니다. 넓은 커넥터 생태계, 호스팅 registry, remote/shared MCP exposure, 여러 접점 orchestration, implementation readiness, 이 문서 저장소의 runtime conformance, 최종 문서 수락을 증명하지 않습니다.

활성 smoke target은 connector marketplace가 아니라 reference `capability_profile`입니다. Runtime fixture가 생기고 실행되기 전까지 `conformance_smoke_status`는 `planned_not_run`이어야 합니다.

Reference-surface check에는 다음이 포함됩니다.

- active Task가 있을 때와 없을 때 status
- Use 절차가 요구할 때 중요한 이어가기 전 compact current-position status
- 실제 profile field에서 파생된 guarantee display
- profile이 증명하지 못하면 `preventive` 또는 `isolated` claim 없음
- OS-permission 표현 없는 `prepare_write` allowed/blocked compatibility outcome
- `prepare_write.decision=allowed` 뒤에만 생기는 한 번만 쓰는 협력형 Write Authorization
- summary와 owner-registered artifact ref가 있는 `record_run`
- MCP-unavailable product-write hold
- unsupported capability에 대한 `CAPABILITY_INSUFFICIENT` 또는 동등한 blocked reason
- 나중에 Core mutation path가 action을 기록하기 전까지 읽기 전용 recommendation

Future fixture 형태와 주장 권한은 [적합성 참조](conformance.md)가 담당합니다. 운영자 명령과 진단은 future owner가 승격하기 전까지 [Later 후보 색인: 운영 후보](../later/index.md#operations-candidates)에 남는 later 후보입니다.
