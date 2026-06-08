# 용어집 참조

이 용어집은 하네스 용어, 대소문자, 정확한 식별자, 담당 문서 경로를 확인하는 짧은 문서입니다. 계획된 하네스 동작을 위한 원천 문서일 뿐이며, [MVP 계획](../build/mvp-plan.md)이 다르게 말하지 않는 한 이 저장소는 문서 전용입니다.

용어집은 이름과 경로를 정합니다. Core 동작, API 스키마, Storage DDL, 보안 보장, Projection 템플릿, 커넥터 동작, 적합성 fixture, 이후 후보 계약의 전체 정의는 각 담당 문서에 남습니다.

## 공개 용어

사용자용 문서, 프롬프트, 상태 요약에서는 아래 공개 용어를 먼저 씁니다. 정확한 하네스 식별자는 차단 사유, 경계, 출처 참조, 담당 경로를 설명할 때만 덧붙입니다.

| 공개 용어 | 뜻 | 담당 경로 |
|---|---|---|
| 작업 | 사용자가 끝내거나, 답을 얻거나, 조사하거나, 결정하고 싶은 일입니다. 내부 기록을 말할 때만 `Task`를 씁니다. | [Core Model](core-model.md) |
| 범위 | 무엇을 바꿀 수 있고, 무엇이 범위 밖이며, 에이전트가 어디에서 멈춰야 하는지입니다. | [Core Model](core-model.md) |
| 범위 밖 | 현재 범위에서 제외된 파일, 동작, 판단, 주장, 행동입니다. | [Core Model](core-model.md) |
| 요구사항 구체화 | 구현 계획이나 쓰기 가능한 작업 전에 요구사항을 분명히 하는 과정입니다. 내부 참조에서는 `Discovery`라고 부를 수 있습니다. | [Core Model](core-model.md) |
| 작업 조각 | 작게 나눈 작업 범위입니다. 내부 참조에서는 쓰기 가능한 범위 단위를 `Change Unit`이라고 부를 수 있습니다. | [Core Model](core-model.md) |
| 사용자가 소유하는 판단 | 에이전트 추론, 증거, 표시 문구, 넓은 동의에서 추론하지 않고 사용자의 선택으로 보존해야 하는 판단입니다. | [Core Model](core-model.md) |
| 판단 요청 | 사용자가 소유하는 판단 하나를 묻는 집중된 질문입니다. API 참조에서는 `UserJudgment`를 씁니다. | [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md) |
| 제품 판단 | 제품 동작, 문구, 흐름, UX, 사용자 가치에 대한 사용자가 소유하는 판단입니다. | [Core Model](core-model.md) |
| 기술 판단 | 아키텍처, 의존성, 마이그레이션, 인터페이스, 보안/개인정보, 중요한 기술 방향에 대한 사용자가 소유하는 판단입니다. | [Core Model](core-model.md) |
| 범위 판단 | 범위 확장, 비목표 제거, `Change Unit` 경계, `Autonomy Boundary` 변경에 대한 사용자가 소유하는 판단입니다. | [Core Model](core-model.md) |
| 민감 동작 승인 | 경계가 정해진 범위 안에서 이름 붙은 민감한 단계 하나를 진행해도 된다는 사용자 권한 부여입니다. 최종 수락이나 넓은 동의가 아닙니다. | [Core Model](core-model.md) |
| 증거 | 작업에 대한 주장을 뒷받침하는 오래 남는 자료입니다. 변경 경로, 변경 차이, 로그, 스크린샷, 검사 메모, `ArtifactRef`가 될 수 있습니다. | [API Schema Core](api/schema-core.md), [Storage](storage.md) |
| 검증 | 담당 경로가 요구할 때 기록되는 정확성 확인입니다. 증거, QA, 최종 수락, 잔여 위험 수락을 대신하지 않습니다. | [Core Model](core-model.md) |
| 수동 QA | 자동 확인이나 증거만으로는 부족하고 사람이 직접 판단해야 하는 품질 확인입니다. | [Core Model](core-model.md), [Later](../later/index.md) |
| QA 면제 판단 | 향후 담당 경로가 허용할 때 QA 기대치를 면제하거나 줄이는 later/reserved 사용자 소유 판단 후보입니다. 증거나 최종 수락을 만들지 않습니다. | [Later](../later/index.md), [Core Model](core-model.md) |
| 최종 수락 | 작업 경로가 수락을 요구할 때 사용자가 결과를 받아들이는 판단입니다. 그 자체로 민감 동작을 승인하거나 잔여 위험을 수락하지 않습니다. | [Core Model](core-model.md) |
| 잔여 위험 | 닫기에 영향을 주는 알려진 남은 불확실성, 확인하지 못한 조건, 한계, 절충입니다. | [Core Model](core-model.md) |
| 잔여 위험 수락 | 알려진 잔여 위험을 받아들이는 사용자가 소유하는 판단입니다. 최종 수락, later/reserved QA 면제 판단, 검증 위험 수락과 구분됩니다. | [Core Model](core-model.md), [API Schema Core](api/schema-core.md) |
| 닫기 가능 여부 | 작업을 지금 정직하게 닫을 수 있는지와 닫기 전에 남은 일을 보여주는 상태입니다. | [Core Model](core-model.md) |
| 닫기 차단 사유 | 진행, 쓰기, 닫기를 정직하게 계속할 수 없게 하는 구체적인 이유입니다. 해결하거나 유효하게 미뤄야 합니다. | [Core Model](core-model.md) |
| 다음 안전한 행동 | 해결되지 않은 범위, 판단, 증거, QA, 검증, 수락, 위험을 숨기지 않고 진행할 수 있는 다음 행동입니다. | [API Schema Core](api/schema-core.md) |
| 권한 경계 | 무엇이 하네스 권한을 만들고 무엇이 정보로만 쓰이는지를 나누는 선입니다. 채팅, Projection, 보고서는 권한이 아닙니다. | [런타임 경계](runtime-boundaries.md) |
| 파생 표시 | 상태 카드나 Projection처럼 담당 기록에서 렌더링된 사용자 표시입니다. Core가 소유한 상태를 대체하지 않습니다. | [Projection과 Template](projection-and-templates.md) |
| 현재 MVP | 활성 계획 기준 MVP 참조 범위입니다. 런타임/서버 구현이 존재한다는 증거가 아닙니다. | [MVP 계획](../build/mvp-plan.md) |
| 이후 후보 | 담당 문서가 범위, 대체 동작, 증명 기대치와 함께 승격하기 전까지 현재 MVP 밖에 남는 향후 자료입니다. | [이후 후보 색인](../later/index.md) |

## Core 용어

아래 용어는 Core 권한을 이해하기 위한 길잡이입니다. 정확한 생명주기, 관문, 닫기, 면제, 대체 불가능 규칙은 [Core Model 참조](core-model.md)가 담당합니다.

| Core 용어 | 짧은 설명 | 담당 경로 |
|---|---|---|
| Core가 소유한 상태 | 하네스 운영 권한이 되는 커밋된 담당 기록과 `state.sqlite.task_events`입니다. | [Core Model](core-model.md), [Storage](storage.md) |
| `Task` | 사용자의 작업, 상태, 차단 사유, 증거 상태, 닫기 가능 여부, 결과를 담는 내부 단위입니다. | [Core Model](core-model.md) |
| `Task.lifecycle_phase` | 지속 저장되는 Task 생명주기 필드입니다. 값은 `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`, `superseded`입니다. `intake`는 값이 아니며, `superseded`는 종료 값입니다. | [Core Model](core-model.md), [API Schema Core](api/schema-core.md), [Storage](storage.md) |
| `Task.close_reason` | 지속 저장되는 닫기 사유 세부값입니다. 값은 `none`, `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, `superseded`입니다. 생명주기와 굵은 결과와는 별도입니다. | [Core Model](core-model.md), [API Schema Core](api/schema-core.md), [Storage](storage.md) |
| `Task.result` | 작업 수준의 굵은 결과입니다. 값은 `none`, `advice_only`, `completed`, `cancelled`, `superseded`입니다. `passed`와 `failed`는 종료 Task 결과 값이 아닙니다. | [Core Model](core-model.md), [API Schema Core](api/schema-core.md), [Storage](storage.md) |
| `Change Unit` | 쓰기 가능한 작업의 활성 범위 경계입니다. 그 자체로 쓰기를 승인하지 않습니다. | [Core Model](core-model.md) |
| `Autonomy Boundary` | 활성 `Change Unit` 안에서 에이전트가 다시 묻지 않고 결정할 수 있는 선택의 경계입니다. 범위 부여, 승인, 쓰기 권한이 아닙니다. | [Core Model](core-model.md) |
| `user_judgment` | 사용자가 소유하는 판단을 위한 기준 기록 계열입니다. | [Core Model](core-model.md), [API Schema Core](api/schema-core.md) |
| `Gate` | 진행, 쓰기, 실행 기록, 닫기에 대한 Core 호환성 축입니다. 필요한지는 활성 담당 경로가 정합니다. | [Core Model](core-model.md) |
| `Blocker` | 진행, 쓰기, 닫기 또는 요청된 다음 단계를 정직하게 계속할 수 없는 구조화된 이유입니다. | [Core Model](core-model.md) |
| `Write Authorization` | 호환되는 non-dry-run `prepare_write`만 만드는 1회용 협력형 Core 기록입니다. OS 권한이나 격리가 아닙니다. | [Core Model](core-model.md) |
| `Run` | 실행 또는 관찰을 남기는 커밋된 기록입니다. 제품 쓰기 `Run`은 호환되는 활성 `Write Authorization`을 소비해야 합니다. | [Core Model](core-model.md) |
| `update_scope` | `harness.intake` 이후 활성 Task 범위와 활성 Change Unit을 갱신하는 Core 경로입니다. 공개 API 메서드는 `harness.update_scope`입니다. | [Core Model](core-model.md), [MVP API](api/mvp-api.md) |
| `prepare_write` | 제품 파일 쓰기를 위한 Core의 쓰기 전 호환성 판단 지점입니다. 공개 API 메서드는 `harness.prepare_write`입니다. | [Core Model](core-model.md), [MVP API](api/mvp-api.md) |
| `record_run` | 실행 또는 관찰을 기록하고 필요한 경우 호환되는 `Write Authorization`을 소비하는 Core 경로입니다. 공개 API 메서드는 `harness.record_run`입니다. | [Core Model](core-model.md), [MVP API](api/mvp-api.md) |
| `close_task` | Core의 완료 판단 지점입니다. 공개 API 메서드는 `harness.close_task`입니다. | [Core Model](core-model.md), [MVP API](api/mvp-api.md) |

## API/스키마 식별자

스키마, API 문서, 기록, 예시, 파일 경로, 진단 출력, 코드 형태 문장에서는 아래 내부 식별자를 정확히 유지합니다. 의미와 값 집합은 [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [API Errors](api/errors.md)가 담당합니다.

| 식별자 | 짧은 설명 | 담당 경로 |
|---|---|---|
| 활성 MCP 메서드 | `harness.intake`, `harness.status`, `harness.update_scope`, `harness.prepare_write`, `harness.record_run`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.close_task`. | [MVP API](api/mvp-api.md) |
| `ToolEnvelope` / `ToolResponseBase` / `ToolError` / `EventRef` | 공통 호출 식별, 응답, 오류, 이벤트 참조 형태입니다. | [API Schema Core](api/schema-core.md) |
| `StateSummary` / `StateRecordRef` / `NextActionSummary` / `GuaranteeDisplay` | 현재 상태, 담당 기록 참조, 다음 행동, 보장 표시 형태입니다. | [API Schema Core](api/schema-core.md) |
| `ArtifactRef` / `ArtifactInput` | 공개 아티팩트 포인터와 `record_run`이 받을 수 있는 아티팩트 입력 형태입니다. | [API Schema Core](api/schema-core.md), [Storage](storage.md) |
| `EvidenceSummary` / `EvidenceCoverageItem` | 간결한 증거 상태와 주장별 범위 형태입니다. | [API Schema Core](api/schema-core.md) |
| `AuthorizedAttemptScope` | 허용된 쓰기 시도 하나의 저장된 범위입니다. | [API Schema Core](api/schema-core.md), [Core Model](core-model.md) |
| `WriteAuthorizationSummary` / `WriteAuthoritySummary` | `Write Authorization` 하나와 현재 쓰기 권한 위치를 보여주는 공개 요약입니다. | [API Schema Core](api/schema-core.md) |
| `RunSummary` / `ObservedChanges` | 공개 실행 결과와 관찰된 변경 요약 형태입니다. | [API Schema Core](api/schema-core.md) |
| `UserJudgment` / `UserJudgmentCandidate` / `UserJudgmentResolution` / `RecordUserJudgmentPayload` / `AcceptedRiskInput` | 판단 요청, 후보, 저장된 해결 기록, 답변 세부정보, 잔여 위험 수락 입력 형태입니다. | [API Schema Core](api/schema-core.md) |
| `judgment_kind` | 사용자 판단 종류의 기준 필드입니다. 값은 정확히 유지하고 지역화된 라벨로 바꾸지 않습니다. | [API Schema Core](api/schema-core.md) |
| `presentation` | 활성 간결한 프롬프트/세부 표시 필드입니다. `short`는 활성이고 `full`은 이후 전체 형식 표시입니다. | [API Schema Core](api/schema-core.md), [Later](../later/index.md) |
| `CloseTaskResponse.close_state` | `harness.close_task`가 돌려주는 응답 수준의 닫기 상태입니다. 값은 `ready`, `blocked`, `closed`, `cancelled`, `superseded`입니다. 지속 저장되는 `Task.lifecycle_phase`와는 별도입니다. | [MVP API](api/mvp-api.md) |
| `CloseBlocker` | 구조화된 닫기/진행 차단 결과입니다. 산문 보고 문구만으로는 차단 결과가 아닙니다. | [API Schema Core](api/schema-core.md), [API Errors](api/errors.md) |
| `ValidatorResult` | 구조화된 validator 출력입니다. 활성 안정 validator ID: `surface_capability_check`. | [API Schema Core](api/schema-core.md) |
| 민감 범주 | `auth_change`, `destructive_write`, `privacy_or_pii_change`, `data_export`, `policy_override` 같은 정확한 값입니다. | [API Schema Core](api/schema-core.md) |
| 공개 오류 코드 | `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `PROJECTION_STALE` 같은 안정적인 공개 오류입니다. | [API Errors](api/errors.md) |

## Storage 용어

Storage 용어는 향후 하네스 기록이 어디에 사는지 알려줍니다. 정확한 테이블 역할, JSON `TEXT` 규칙, 상태 버전, 잠금, 마이그레이션, 아티팩트 처리는 [Storage](storage.md)가 담당합니다.

| Storage 용어 | 짧은 설명 | 담당 경로 |
|---|---|---|
| Product Repository | 사용자의 제품 작업 공간입니다. 제품 파일은 가까이 있다는 이유만으로 하네스 운영 권한이 되지 않습니다. | [런타임 경계](runtime-boundaries.md) |
| Harness Server / Installation | 향후 로컬 Harness 제어 평면 프로그램입니다. 일반 OS 샌드박스나 권한 시스템이 아닙니다. | [런타임 경계](runtime-boundaries.md) |
| Harness Runtime Home | 레지스트리, 프로젝트 상태, 아티팩트를 담는 사용자별/설치별 운영 데이터 홈입니다. | [런타임 경계](runtime-boundaries.md), [Storage](storage.md) |
| 런타임 식별 파일 | `registry.sqlite`, `project.yaml`, `state.sqlite`는 Runtime Home, 정적 프로젝트 설정, 프로젝트별 Core 상태를 식별합니다. | [Storage](storage.md) |
| 활성 저장 기록 | 활성 테이블 이름에는 `project_state`, `surfaces`, `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, `tool_invocations`가 포함됩니다. | [Storage](storage.md) |
| JSON `TEXT` 열 | Core/API/storage 검증 이후 담당 문서 형태를 따르는 JSON을 저장하는 SQLite `TEXT` 열입니다. 임의 JSON 컨테이너가 아닙니다. | [Storage](storage.md) |
| 아티팩트 저장 연결 | `artifacts`와 `artifact_links`는 증거 바이트나 안전한 메타데이터를 등록하고 담당 기록과 연결합니다. 연결 자체가 `Gate`를 만족하지는 않습니다. | [Storage](storage.md) |
| 이벤트/재실행 저장 | `task_events`는 커밋된 변경 감사 추적 기록이고, `tool_invocations`는 커밋된 멱등성 재실행 행입니다. | [Storage](storage.md) |
| 상태 버전과 해시 | `state_version`, `project_state.state_version`, `tasks.state_version`, `tree_hash`, `request_hash`는 오래된 상태, 기준선, 멱등성 확인을 지원합니다. | [Storage](storage.md), [API Errors](api/errors.md) |

## 보안 보장 용어

보안 표현은 담당 문서가 정의하고 증명한 통제 수준과 맞아야 합니다. 정확한 보장 의미와 비보장 항목은 [보안 참조](security.md)가 담당합니다.

| 보안 용어 | 뜻 | 담당 경로 |
|---|---|---|
| `cooperative` | 연결된 접점이 절차를 따를 때 하네스가 하네스 상태 변경 경로를 안내, 기록, 비교, 거부할 수 있다는 뜻입니다. 물리적 차단이 아닙니다. | [보안](security.md), [Agent 통합](agent-integration.md) |
| `detective` | 행동 이후나 관찰 가능해진 시점에 지원되는 사실을 감지, 기록, 보고할 수 있다는 뜻입니다. 현재 MVP에서는 관련 역량 확인이 통과한 뒤에만 이 라벨을 쓸 수 있습니다. 사전 차단이 아닙니다. | [보안](security.md), [Agent 통합](agent-integration.md) |
| `preventive` | 이름 붙은 메커니즘이 대상 동작을 실행 전에 막을 수 있다는 주장입니다. 현재 MVP에는 기본 예방형 주장이 없습니다. | [보안](security.md) |
| `isolated` | 이름 붙고 증명된 분리 경계가 대상 동작에서 어떤 것을 다른 것에서 격리한다는 주장입니다. 현재 MVP에는 기본 격리 주장이 없습니다. | [보안](security.md), [런타임 경계](runtime-boundaries.md) |
| 정직한 보장 표시 | 사용자에게 보이는 보장 문구는 `capability_profile` 사실과 담당 문서의 증명 수준에 맞아야 합니다. 지원되지 않는 강한 주장은 낮추거나 차단해야 합니다. | [보안](security.md), [API Errors](api/errors.md) |
| 명시적 비보장 / 신뢰 경계 | 현재 MVP는 OS 수준 권한 제어, 임의 도구 샌드박싱, 변조 방지 저장소, 기본 도구 실행 전 차단, 보안 격리를 제공하지 않습니다. | [보안](security.md), [런타임 경계](runtime-boundaries.md) |

## 에이전트/맥락 용어

에이전트와 커넥터 용어는 접점이 담당 기록을 낮은 맥락 비용으로 쓰는 방법을 설명합니다. 정확한 커넥터 동작은 [Agent 통합 참조](agent-integration.md)가 담당합니다.

| 에이전트/맥락 용어 | 짧은 설명 | 담당 경로 |
|---|---|---|
| 에이전트 접점 / `surface_id` | 연결된 환경과 API 호출자 식별자입니다. 접점 이름이나 `surface_id`만으로 기능이나 권한이 생기지 않습니다. | [Agent 통합](agent-integration.md) |
| `capability_profile` | 접점이 실제로 할 수 있는 일을 선언하고 갱신한 사실입니다. MCP 태세, 관찰, 캡처, 보호, 격리 지원을 포함합니다. | [Agent 통합](agent-integration.md), [보안](security.md) |
| 커넥터 매니페스트 | 커넥터가 관리하는 경로, 스니펫, 관리 블록 해시, 프로필 최신성, 드리프트, 대체 동작 요약입니다. | [Agent 통합](agent-integration.md) |
| 항상 주입되는 맥락 | 한 화면 이하의 현재 맥락입니다. 작업 요약, 범위, 대기 중인 판단, 차단 사유, 다음 안전한 행동, 증거 공백, 닫기 차단 사유, 잔여 위험, 보장 수준, 최신 참조만 둡니다. | [Agent 통합](agent-integration.md) |
| 단계별 맥락 / push-pull | 간결한 현재 맥락을 먼저 주고, 다음 행동에 필요한 담당 섹션만 가져오는 방식입니다. | [Agent 통합](agent-integration.md), [참조 색인](README.md) |
| Role Lens | 읽기 전용 역할 관점 안내입니다. `Role Lens` 추천은 담당 경로가 행동을 기록하기 전까지 권한이 없습니다. | [Agent 통합](agent-integration.md) |
| 기준 로컬 MCP 접점 | 활성 참조 통합 프로필인 `reference-local-mcp`입니다. 협력형 동작을 기본으로 하며, 제한된 탐지형 동작은 지원되는 범위와 관련 역량 확인이 통과한 경우에만 표시합니다. | [Agent 통합](agent-integration.md) |
| 대체 동작 | Core, MCP, Projection, 로컬 접근, 기능을 사용할 수 없거나 기능이 부족할 때의 커넥터 응답입니다. | [Agent 통합](agent-integration.md), [API Errors](api/errors.md) |

## 이후 후보 용어

이후 후보 용어는 후보 또는 전달 라벨입니다. 담당 문서가 승격하기 전에는 활성 API/스키마/저장소 계약, fixture 본문, 런타임 동작, 생성된 아티팩트, 현재 MVP 요구사항이 아닙니다.

| 이후 후보 용어 | 현재 상태 | 담당 경로 |
|---|---|---|
| Context Index | 나중의 읽기 전용 검색 지원입니다. 쓰기 승인, `Gate` 충족, 위험 수락, 닫기를 대신하지 않습니다. | [Later](../later/index.md) |
| Journey Card / Journey Spine | 나중의 연속성 표시입니다. 활성화되고 최신이면 방향 잡기에 도움을 줄 수 있지만 Core가 소유한 상태는 아닙니다. | [Later](../later/index.md) |
| Browser QA Capture | 이후 캡처 지원 후보입니다. 그 자체로 수동 QA, 최종 수락, 분리된 검증이 아닙니다. | [Later](../later/index.md) |

## 폐기/호환 용어

아래 호환 용어는 호환 라벨과의 혼동을 막을 때만 남깁니다. 새 활성 문서의 주 개념으로 쓰지 않습니다.

| 용어 | 호환 메모 | 현재 경로 |
|---|---|---|
| Decision Packet | 전체 형식의 이후 후보 표시입니다. 현재 사용자 경로의 필수 형식이 아닙니다. | [API Schema Core](api/schema-core.md), [Later](../later/index.md) |
| `MVP-1` | 현재 MVP 범위를 가리키던 이전 라벨입니다. 호환 설명이 필요한 곳에서만 사용하고, 새 활성 문서에서는 현재 MVP를 씁니다. | [MVP 계획](../build/mvp-plan.md) |
