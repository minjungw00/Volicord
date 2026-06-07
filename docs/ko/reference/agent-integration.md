# 에이전트 통합 참조

에이전트 접점을 향후 하네스 동작에 연결할 때 이 참조를 사용합니다. 목표는 맥락 비용을 낮게 유지하고, 보장 표시 수준을 정직하게 보여주며, 사용자가 소유하는 판단을 보존하는 것입니다. 이 저장소는 아직 문서 전용이며 문서 검토 단계입니다. 이 문서는 계획된 하네스 동작을 설명할 뿐, 런타임 서버나 커넥터 구현이 이미 있다는 뜻이 아닙니다.

사용자 세션에서 에이전트가 무엇을 말해야 하는지는 [에이전트 가이드](../use/agent-guide.md)를 봅니다. Core, API, 스키마, 저장소, Projection, 보안, 적합성, 운영 계약은 다음 행동에 필요한 담당 부분만 가져옵니다. later 후보, 접점별 메모, 적합성 계획을 active 요구사항으로 바꾸면 안 됩니다.

## 1. 담당하는 것 / 담당하지 않는 것

이 참조가 담당하는 것:

- 에이전트 접점의 `capability_profile`
- 연결된 접점의 보장 표시 수준
- 맥락 주입/가져오기 규칙과 항상 주입되는 맥락 예산
- 저렴한 검색을 위한 단계별 맥락 선택 규칙
- 커넥터 경계에서의 판단 요청 동작
- Role Lens를 사용하는 접점에서의 비권한 동작
- Core, MCP, Projection, 필요한 역량을 사용할 수 없을 때의 대체 동작
- 에이전트가 어떤 맥락을 넣을지 판단하게 돕는 짧은 접점별 메모
- 커넥터 적합성 경계

이 참조가 담당하지 않는 것:

- 사용자 세션 절차. [에이전트 가이드](../use/agent-guide.md)를 봅니다.
- 범위, 증거, QA, 최종 수락, 잔여 위험, 닫기에 대한 사용자 대상 설명. [사용자 가이드](../use/user-guide.md)를 봅니다.
- Core 상태 전이, gate, `prepare_write`, Write Authorization, `record_run`, `close_task`. [Core Model 참조](core-model.md)를 봅니다.
- 공개 MCP 메서드 계약, 스키마, 공개 오류. [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)를 봅니다.
- Storage DDL, 지속 상태, 아티팩트 배치. [Storage](storage.md)를 봅니다.
- Projection/템플릿 권한과 active 렌더링 템플릿 본문. [Projection과 Template 참조](projection-and-templates.md)를 봅니다.
- 위협 모델과 보장 표시 의미. [보안 참조](security.md)를 봅니다.
- 향후 fixture 형식 또는 주장 권한. [적합성 참조](conformance.md)를 봅니다.
- 활성 참조 범위로서의 운영자 명령과 진단. 향후 운영 후보는 [Later 후보 색인: 운영 후보](../later/index.md#operations-candidates)에 남습니다.
- 향후 커넥터 마켓플레이스, 호스팅 에이전트 가정, 넓은 커넥터 생태계, 여러 접점 오케스트레이션

이 문서의 접점별 메모는 통합 지침입니다. Core 상태 권한, Write Authorization, 증거, 검증, QA, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 닫기 준비 상태를 만들지 않고, later 후보를 active 의무로 바꾸지 않으며, 새로운 보안 권한 경계를 만들지 않습니다.

Role Lens 출력은 읽기 전용 검토 자세 안내입니다. Role Lens는 판단 요청, 증거 수집, 검증, 수동 QA, 민감 동작 승인, 잔여 위험 처리, 범위 업데이트, 다음 절차를 추천할 수 있습니다. 하지만 담당 API/Core 경로가 해당 행동을 기록하기 전까지 추천에는 권한이 없습니다.

## 2. `capability_profile`

접점 이름 자체는 역량이 아닙니다. 커넥터는 실제 호스트, 버전/설정, 작업공간 정책, MCP 연결 태세, 캡처 경로, guard 경로, 분리 경계에 맞춘 `capability_profile`을 사용해야 합니다.

`capability_profile`은 Write Authorization이 아니며 쓰기 호환성을 만들지 않습니다. 활성 Task 범위, 활성 Change Unit 범위, `prepare_write`, 한 번만 쓰는 협력형 Write Authorization, `record_run`, Core 닫기 규칙도 우회하지 않습니다. 역량 정보는 차단 사유, 대체 동작, validator 결과, 보장 라벨 표시에 영향을 줍니다. `allowed`와 `blocked`는 증명된 `preventive` 프로필이 대상 동작을 이름 붙이지 않는 한 하네스 호환성 결과입니다. 런타임 경계는 권한 경계와 저장 위치 경계이지 OS 수준 격리 경계가 아닙니다.

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

정확한 공개 tool/resource 계약은 API 담당 문서가 담당합니다. 커넥터는 사용할 수 있는 부분집합을 요약할 수 있지만, 메서드 스키마 전체를 프롬프트 맥락에 중복 주입하면 안 됩니다.

접점 버전, MCP 설정, 후크, 권한, 작업공간 정책, 생성 파일, 관리형 block, 캡처 경로, QA 캡처 경로, redaction 정책, 아티팩트 보존 방식, 로컬 접근 태세, guard 래퍼, 격리 래퍼, 적합성 근거가 바뀌면 `capability_profile`을 갱신합니다.

생성된 rule, skill, MCP snippet, adapter file, 관리형 block에는 connector manifest가 필요합니다. Manifest는 생성 경로, 관리형 block id와 hash, MCP 노출 태세, 표시해도 안전한 handle, 프로필 최신성, drift, 대체 동작을 기록합니다. 원시 token, secret, 비공개 설정값, 차단된 payload byte, 기준 Task state는 저장하지 않습니다.

## 3. 보장 표시 수준

`guarantee_display.level` 표시는 [보안 참조](security.md#정직한-guarantee-display)를 따릅니다. 정확한 스키마 값 집합은 [API Schema Core](api/schema-core.md#current-mvp-value-sets)가 담당합니다. 이 참조는 커넥터가 `capability_profile`을 사용자에게 보이는 표시로 어떻게 연결하는지 담당합니다.

현재 MVP 커넥터 표시 값은 다음과 같습니다.

| 수준 | 커넥터 표시 규칙 |
|---|---|
| `cooperative` | 접점이 하네스 지시를 따를 것으로 기대된다고 말합니다. 보류는 지시로 이루어지며 물리적 차단이 아닙니다. |
| `detective` | 하네스가 행동 뒤에 변경 경로, 로그, 아티팩트, drift 같은 지원되는 사실을 보고 상태를 stale, partial, blocked, failed로 표시할 수 있다고 말합니다. |

profile-gated 표시 값 이름은 다음과 같습니다.

| 이름 | 커넥터 표시 규칙 |
|---|---|
| `preventive` | 승격된 프로필에 명시적인 프로필 지원이 있을 때만 사용합니다. fixture로 입증된 후크, 래퍼, 권한 계층, 정책 엔진, 사이드카 경로와 실행 전에 막을 수 있는 정확한 동작을 이름 붙입니다. |
| `isolated` | 승격된 프로필에 명시적인 프로필 지원이 있을 때만 사용합니다. 문서화된 분리 경계를 이름 붙입니다. 해당 메커니즘이 입증되지 않았으면 운영체제 샌드박싱, 권한 격리, 변조 방지 저장소를 암시하지 않습니다. |

에이전트는 사용자가 더 강한 안전성을 요청했거나, guard/freeze/careful mode를 요청했거나, 대화에서 더 강한 표현을 썼다는 이유만으로 `preventive` 또는 `isolated`를 고르면 안 됩니다. 활성 프로필이 더 강한 주장을 뒷받침하지 못하면 커넥터는 표시되는 `guarantee_display.level` 값을 낮추거나 `CAPABILITY_INSUFFICIENT`를 반환해야 합니다.

기준 로컬 MCP 프로필은 `cooperative` 동작과, 변경 경로 또는 아티팩트 공백 관찰이 뒷받침하는 제한된 `detective` 동작만 표시할 수 있습니다. `pre_tool_blocking_supported=false`와 `isolation_supported=false`이므로 `preventive`나 `isolated` 동작을 주장하면 안 됩니다.

Guard, freeze, careful-mode 라벨은 실제 `capability_profile` 위에 놓이는 표시 라벨입니다. 무엇을 실행 전에 실제로 멈출 수 있고 무엇은 나중에만 감지할 수 있는지 말해야 합니다. 이것들은 민감 동작 승인, 검증, QA, 최종 수락, 잔여 위험 수락, 닫기 준비 상태, Core gate가 아닙니다.

현재 MVP가 문서화하지 않은 보안 보장 주장을 하지 않습니다. 하네스는 기본 OS 권한, 임의 도구 샌드박스, 변조 방지 로컬 파일, 도구 실행 전 차단, 보안 격리를 제공하지 않습니다.

## 4. 맥락 주입/가져오기

커넥터는 작고 최신인 현재 맥락을 주입하고, 큰 담당 문서는 다음 행동에 필요할 때만 가져옵니다. 맥락 패킷은 다음 에이전트 행동을 돕는 운영 지원 맥락입니다. 에이전트 기억, 대화 기록, 전체 보고서, 캐시된 Projection 본문, 참조 문서 전체 본문이 아닙니다.

검색 비용 원칙:

- 참조 문서 전체 묶음을 기본으로 주입하지 않습니다.
- 전체 스키마를 기본으로 주입하지 않습니다.
- 전체 Storage DDL, 템플릿 전체, Projection 본문 전체, 전체 이력, 전체 event log, 가공하지 않은 아티팩트 내용, 로그, 스크린샷, trace, 관련 없는 later 후보 자료를 기본으로 주입하지 않습니다.
- 향후/later 목록 자료를 기본으로 주입하지 않습니다.
- later 후보, 향후 목록 항목, 접점별 메모, 적합성 계획을 active 요구사항으로 승격하지 않습니다.
- 다음 행동에 필요한 담당 부분만 가져오고 멈춥니다.
- 일반 작업 프롬프트에서는 하나의 언어를 고릅니다. 같은 `doc_id`의 영어/한국어 대응 문서를 같은 프롬프트에 함께 넣는 에이전트 중복 주입 금지를 지킵니다. 이중 언어 검토는 대응 문서 전체를 주입하지 말고 필요한 부분만 비교합니다.

상태 카드, Projection, 렌더링된 템플릿, 검색된 맥락, 추천, 대화 기억은 읽기 전용입니다. 무엇을 확인할지 가리킬 수는 있지만 쓰기를 승인하거나, gate를 만족하거나, 증거를 만들거나, 사용자 판단을 해결하거나, 민감 동작 승인을 부여하거나, 검증을 수행하거나, QA를 기록하거나, 결과를 수락하거나, 잔여 위험을 수락하거나, Projection 최신성을 고치거나, Task를 닫을 수 없습니다.

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
- 보장 표시 수준. Core 또는 필요한 MCP가 답할 수 없으면 unavailable/capability 조건
- 출처 참조와 최신성

참조 자료 전체, 전체 스키마, 전체 DDL, Projection 본문 전체, 아티팩트 본문 전체, 관련 없는 템플릿, 향후 목록, 최신이 아니거나 관련 없는 Task 이력, 과거 로그를 항상 주입되는 맥락에 넣지 않습니다.

## 6. 단계별 맥락 선택

다음 질문에 답하는 가장 좁은 맥락을 사용합니다.

| 단계 | 이것만 가져오기 |
|---|---|
| 세션 시작 / 이어가기 | 현재 `harness.status`, 현재 Task/status 리소스, [에이전트 가이드: 사용자의 다음 결정을 위한 상태 보고](../use/agent-guide.md#8-사용자의-다음-결정을-위한-상태-보고). |
| 계획 / 요구사항 구체화 | 현재 저장소, 문서, 상태 참조와 [에이전트 가이드: 끝없는 계획 루프 없이 구체화하기](../use/agent-guide.md#4-끝없는-계획-루프-없이-구체화하기). |
| 쓰기 준비 | 현재 범위와 상태, [에이전트 가이드: 제품 파일 쓰기 전에 범위 확인](../use/agent-guide.md#6-제품-파일-쓰기-전에-범위-확인), 의도한 쓰기에 필요한 `prepare_write` 담당 부분만. |
| 실행 / run 기록 | 현재 Write Authorization, 실행/증거 참조, [에이전트 가이드: 의미 있는 행동 뒤에는 증거 기록](../use/agent-guide.md#7-의미-있는-행동-뒤에는-증거-기록). |
| 증거 검토 | 현재 증거 참조, 아티팩트 참조, 최신성 사실, 빠진 증거, 필요할 때만 정확한 증거 또는 Projection 담당 부분. |
| 닫기 준비 상태 | 현재 담당 기록, 차단 사유, 잔여 위험 요약, [에이전트 가이드: 정직하게 닫기](../use/agent-guide.md#10-정직하게-닫기). |
| 사용자 판단 요청 | 현재 판단 참조 또는 후보, 결과, 불확실성, [에이전트 가이드: 판단 요청은 좁고 분명하게](../use/agent-guide.md#5-판단-요청은-좁고-분명하게). |
| 복구 / 오류 | 현재 가용성/최신성 상태, [대체 동작](#8-대체-동작), 특정 오류 담당 부분. |

행동에 엄격한 계약이 필요하면 담당 섹션을 연결하거나 가져옵니다. 일반 프롬프트에 담당 문서 전체를 붙여 넣지 않습니다.

## 7. 판단 요청 동작

에이전트는 사용자가 소유하는 판단을 보존합니다. 커넥터는 요청을 형식화하고, 응답을 모으고, 담당 API 경로로 기록을 라우팅할 수 있습니다. 하지만 사용자 대신 결정하면 안 됩니다.

판단 요청은 다음을 보존해야 합니다.

- 사용자가 해야 하는 판단
- 가능한 선택지
- 결과와 장단점
- 불확실성 또는 빠진 증거
- 에이전트 추천이 있다면 그 추천
- 에이전트가 사용자 대신 결정하지 않는 것
- 짧은 표시인지(`presentation=short`), 전체 형식인지(`presentation=full`)

에이전트는 최종 수락, 민감 동작 승인, QA 면제 판단, 검증 위험 수락, 잔여 위험 수락을 사용자 대신 결정하면 안 됩니다. 사용자 소유 제품 판단, 중요한 기술 판단, 범위 확장 판단도 조용히 대신하면 안 됩니다. 넓은 의미의 "좋아" 또는 "계속해" 메시지는 필요한 판단 경로를 대신하지 않습니다.

판단 기록은 증거, 검증, 수동 QA, 최종 수락, 잔여 위험, 닫기 준비 상태와 분리됩니다. 어느 하나도 다른 하나를 대신하지 않습니다.

## 8. 대체 동작

대체 동작은 접점 이름이나 브랜드가 아니라 보장 표시 수준과 위험으로 설명합니다.

| 대체 동작 | 쓰는 경우 | 경계 |
|---|---|---|
| `cooperative` | 접점이 지시를 따를 수 있지만 강제할 수 없을 때. | Core/MCP 담당 경로 또는 쓰기 범위 확인을 사용할 수 없으면 제품 파일 쓰기를 지시로 보류합니다. |
| `detective` | 하네스가 행동 뒤 지원되는 사실을 관찰할 수 있을 때. | 상태를 stale, partial, blocked, failed로 표시하고 복구, 조정, 새 증거를 요구합니다. |
| 역량 부족 | 요청한 쓰기, 캡처, guard, 격리, 보장 주장이 지원하지 않는 필드에 의존할 때. | `CAPABILITY_INSUFFICIENT` 또는 구조화된 차단 사유를 반환하고 표시되는 `guarantee_display.level` 값을 낮춥니다. |
| MCP 사용 불가 | 접점 또는 호출 경로가 현재 Core 권한 경로에 닿지 못할 때. | 안정적인 공개 `MCP_UNAVAILABLE` 동작을 사용하고 상태 변경을 주장하지 않습니다. |
| 로컬 접근 불일치 | 호출자 또는 전송 경로가 등록된 로컬 프로필 밖일 때. | 표시해도 안전한 진단과 함께 `LOCAL_ACCESS_MISMATCH`를 사용합니다. 접점별 `UNAUTHORIZED` code를 만들지 않습니다. |

`MCP_SERVER_UNAVAILABLE`과 `SURFACE_MCP_UNAVAILABLE`은 진단 조건입니다. `MCP_UNAVAILABLE`은 안정적인 공개 가용성 코드로 남습니다.

Core에 닿을 수 없는 동안 대화 기억, 생성 파일, 캐시된 Projection, 오래된 상태 텍스트, 운영자 서술에서 Core 상태, Write Authorization, gate 상태, 승인, 증거, 최종 수락, 잔여 위험 수락, Projection 복구, 닫기 준비 상태를 만들어 내지 않습니다.

Projection 최신성 문제는 Core 상태와 분리됩니다. 커넥터가 현재 Core 상태를 직접 읽을 수 있으면 그 상태에서 계속할 수 있습니다. 오래된 Projection에 의존하는 행동은 먼저 새로 고치거나 조정해야 합니다.

이 문서 전용 저장소의 문서 유지보수 편집은 [문서 작성 가이드](../maintain/authoring-guide.md)가 관리합니다. 런타임 Harness 절차가 아닙니다. 그런 편집은 런타임 상태, Write Authorization, 증거, QA, 수락, 잔여 위험 수락, 닫기 준비 상태, Projection, `task_events`, 런타임 전이를 만들지 않습니다.

## 9. 접점별 메모

접점별 메모는 에이전트가 어떤 맥락을 포함할지 판단하게 돕는 짧은 통합 메모입니다. 별도 참조 담당 문서가 아니며 긴 접점별 작업 흐름으로 커지면 안 됩니다.

접점별 메모에는 아래 항목만 둡니다.

- 대상 `capability_profile`
- generated 또는 managed 지침/설정 경로가 있으면 그 경로
- MCP 태세와 표시해도 안전한 handle
- `capability_profile` 갱신이 필요한 접점별 역량 차이
- 해당 `capability_profile`이 입증한 캡처, guard, 격리 사실
- 필요한 역량이 없을 때의 대체 동작
- 해당 `capability_profile`의 적합성 상태

일반 Core 규칙, 공개 API 스키마, 참조 문서 전체, 향후 커넥터 확장 구상, 호스팅 에이전트 가정, 감사 메모, 관련 없는 later 후보 항목, Projection 본문 전체, 긴 설정 튜토리얼을 넣지 않습니다. 접점별 메모가 later 자료를 가리킬 수는 있지만, 그 자료를 현재 MVP 필수사항으로 만들면 안 됩니다.

기준 로컬 MCP 접점별 메모:

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

`pre_tool_blocking_supported=false`이므로 "hold" 표현은 협력형 범위 규율과, 가능할 때의 탐지형 변경 경로 검증을 뜻합니다. `preventive` guard 동작이 아닙니다.

## 10. 커넥터 적합성 경계

커넥터 적합성은 선언된 `capability_profile`이 해당 역량 수준에서 이 공통 계약을 지킬 수 있음을 보여주기 위한 향후 경계입니다. 넓은 커넥터 생태계, 호스팅 registry, 원격/공유 MCP 노출, 여러 접점 오케스트레이션, 구현 준비 상태, 이 문서 저장소의 런타임 적합성, 최종 문서 수락을 증명하지 않습니다.

활성 smoke 대상은 커넥터 마켓플레이스가 아니라 기준 `capability_profile`입니다. 런타임 fixture가 생기고 실행되기 전까지 `conformance_smoke_status`는 `planned_not_run`이어야 합니다.

기준 접점 점검에는 다음이 포함됩니다.

- active Task가 있을 때와 없을 때의 상태
- Use 절차가 요구할 때 중요한 이어가기 전 간결한 현재 위치 상태
- 실제 `capability_profile` 필드에서 파생된 보장 표시 수준
- `capability_profile`이 그 표시 주장을 뒷받침하지 못하면 `preventive` 또는 `isolated` 주장 없음
- OS 권한 표현 없는 `prepare_write` `allowed`/`blocked` 호환성 결과
- `prepare_write.decision=allowed` 뒤에만 생기는 한 번만 쓰는 협력형 Write Authorization
- 요약과 담당 경로에 등록된 아티팩트 참조가 있는 `record_run`
- MCP를 사용할 수 없을 때의 제품 쓰기 보류
- 지원되지 않는 역량에 대한 `CAPABILITY_INSUFFICIENT` 또는 동등한 차단 사유
- 별도 Core 변경 경로가 행동을 기록하기 전까지 읽기 전용 추천

향후 fixture 형식과 주장 권한은 [적합성 참조](conformance.md)가 담당합니다. 운영자 명령과 진단은 향후 담당 문서가 승격하기 전까지 [Later 후보 색인: 운영 후보](../later/index.md#operations-candidates)에 남는 later 후보입니다.
