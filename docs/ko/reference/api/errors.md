# API 오류

## 이 문서로 할 수 있는 일

현재 MVP의 공개 오류 코드, 기본 오류 우선순위, 차단 응답과 `dry_run` 동작, 멱등 재실행, 상태 충돌 처리, 닫기 차단 사유 동작, 사용자 표시 라벨 지침을 확인할 때 이 참조를 사용합니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 문서 저장소에 MCP 서버가 구현되어 있다는 뜻이 아닙니다.

## 현재 MVP 보장과 profile-gated 주장 경계

`guarantee_display.level`은 승격된 프로필이 profile-gated 표시 값을 명시적으로 지원하지 않는 한 현재 MVP 값인 `cooperative`와 `detective`를 사용합니다. 보안 의미는 [보안 참조: 정직한 보장 표시](../security.md#정직한-guarantee-display)가 담당하고, 정확한 값 집합 경계는 [API Schema Core](schema-core.md#current-mvp-value-sets)가 담당합니다.

프로필 지원 없이 profile-gated 보장을 요청하거나 표시하는 것은 해당 보장이 존재한다는 증거가 아니라, 보장 주장 경계 오류입니다. 필요한 차단, 격리, 관찰, 증명 지원이 접점에 없으면 `CAPABILITY_INSUFFICIENT`를 사용합니다. 요청한 값이 활성 프로필이나 요청 형태에서 유효하지 않으면 `VALIDATION_FAILED`를 사용합니다. 어떤 오류도 문서 전용인 현재 저장소에 런타임 강제가 있다는 뜻은 아닙니다.

| 수준 또는 이름 | 오류/상태 의미 |
|---|---|
| `cooperative` | 에이전트나 도구가 문서화된 경로를 따를 때 하네스가 확인하고 기록할 수 있습니다. OS 권한, 샌드박스, 변조 방지 저장소, 실행 전 차단이 아닙니다. |
| `detective` | 하네스 또는 연결된 접점이 관찰 가능한 불일치를 동작 중이나 이후에 감지, 기록, 보고할 수 있습니다. 예방이 아닙니다. |
| `preventive` | profile-gated 표시 값 이름입니다. 대상 동작에 대한 승격된 도구 실행 전 차단 지원이 없으면 역량 부족 또는 검증 오류를 반환하고 표시 보장을 낮춥니다. |
| `isolated` | profile-gated 표시 값 이름입니다. 이름 붙은 경계에 대한 승격된 격리 지원이 없으면 역량 부족 또는 검증 오류를 반환하고 표시 보장을 낮춥니다. |

활성 MVP 동작은 기본적으로 협력형 확인입니다. 연결된 접점이 사실을 정직하게 관찰할 수 있을 때만 제한된 탐지형 보고를 함께 표시합니다. 이런 보안 비주장은 경계 설명이며 런타임 오류나 강제되는 역량이 아닙니다. 닫기 차단 사유는 사용자 판단, 증거, 잔여 위험 가시성, 잔여 위험 수락 상태를 다루는 구조화된 작업 준비 상태 결과입니다. `preventive` 수준의 도구 실행 전 차단, `isolated` 수준의 격리, 샌드박스, 변조 방지 저장소의 증거가 아닙니다.

| 조건 | 공개 경로 | 에이전트 규칙 |
|---|---|---|
| `core_unavailable` | `MCP_UNAVAILABLE` | 하네스 상태를 만들어 내지 않습니다. Core에 다시 닿거나 사용자가 하네스 밖 진행을 명시적으로 선택하기 전까지 하네스에 의존하는 쓰기와 닫기를 보류합니다. |
| `local_access_denied` | `LOCAL_ACCESS_MISMATCH` 또는 `CAPABILITY_INSUFFICIENT` | 로컬 파일이나 명령 사실을 추측하지 않습니다. 가능한 로컬 접점을 쓰거나, 역량 등록을 고치거나, 범위를 줄이거나, 입력을 미검증으로 표시합니다. |
| `stale_state` | `STATE_CONFLICT`, `BASELINE_STALE`, `PROJECTION_STALE`, 오래된 `WRITE_AUTHORIZATION_INVALID` | 의존하기 전에 현재 상태, baseline, 읽기용 상태 보기, 쓰기 전 확인을 새로 확인합니다. |
| `unsupported_surface` | `CAPABILITY_INSUFFICIENT` 또는 `VALIDATION_FAILED` | 요청을 줄이거나, 역량이 맞는 접점으로 옮기거나, 차단 사유를 반환합니다. 지원하지 않는 권한을 설명 문구로 흉내 내지 않습니다. |
| `out_of_scope` | `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `NO_ACTIVE_CHANGE_UNIT`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | 영향을 받는 행동을 보류하고, 불일치를 보여 주며, 현재 범위로 줄이거나 구체적인 사용자 소유 범위 판단을 요청합니다. |
| `missing_judgment` | `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED` | 집중된 `UserJudgment`를 묻거나 해결합니다. 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락, QA 면제 판단, 검증 위험 수락, 취소 판단을 넓은 승인 하나로 합치지 않습니다. |
| `missing_evidence` | `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING` | 영향을 받는 주장, 참조, 증거 상태, 차단 해소에 필요한 최소 조치를 보여 줍니다. 테스트 결과, 아티팩트 무결성, 증거 충분성을 만들어 내지 않습니다. |
| `close_blocked` | `CloseTaskResponse.close_state=blocked`와 기본 `ErrorCode` | 구조화된 차단 사유와 다음 행동을 반환합니다. Task를 종료 상태로 표시하지 않습니다. |
| `residual_risk_present` | `RESIDUAL_RISK_NOT_VISIBLE`, `DECISION_REQUIRED`, 또는 `DECISION_UNRESOLVED` | 잔여 위험을 보여 주고, 활성 닫기 또는 수락 경로가 요구할 때만 `judgment_kind=residual_risk_acceptance`를 묻습니다. |

<a id="error-taxonomy"></a>

## 오류 분류

| 코드 | 의미 |
|---|---|
| `VALIDATION_FAILED` | 요청 본문 형태, enum 값, 활성화 규칙, 프로필별 검증이 변경 전에 실패했습니다. |
| `STATE_CONFLICT` | `expected_state_version`이 오래되었거나, 상태 잠금 소유권이 바뀌었거나, 같은 멱등 키를 다른 정규화된 요청으로 다시 사용했습니다. |
| `NO_ACTIVE_TASK` | Task가 필요하지만 활성 Task나 지정된 Task가 없습니다. |
| `NO_ACTIVE_CHANGE_UNIT` | 쓰기를 할 수 있거나 닫기와 관련된 동작에 활성 범위 지정 Change Unit이 없습니다. |
| `SCOPE_REQUIRED` | 요청한 쓰기나 동작 전에 범위 확인이 필요합니다. |
| `SCOPE_VIOLATION` | 의도했거나 관찰된 경로, 도구, 명령, 네트워크 대상, 비밀 접근, 민감 범주가 활성 범위 또는 저장된 `AuthorizedAttemptScope`를 넘었습니다. |
| `WRITE_AUTHORIZATION_REQUIRED` | 쓰기 가능한 Run에 `harness.prepare_write`에서 요구하는 Write Authorization이 없습니다. |
| `WRITE_AUTHORIZATION_INVALID` | 제공된 Write Authorization이 `missing`, `expired`, `stale`, `revoked`, 재실행 밖에서 `consumed`, 또는 `incompatible` 상태입니다. |
| `DECISION_REQUIRED` | 동작 전에 차단 중인 사용자 소유 판단을 요청해야 합니다. |
| `DECISION_UNRESOLVED` | 관련 사용자 판단이 `pending`, 적용 범위 없는 `deferred`, `rejected`, `blocked`, `stale`, `superseded`, 또는 `incompatible` 상태입니다. |
| `AUTONOMY_BOUNDARY_EXCEEDED` | 의도한 동작이 활성 Change Unit Autonomy Boundary를 넘었습니다. |
| `APPROVAL_REQUIRED` | 진행 전에 민감 동작 승인이 필요합니다. |
| `APPROVAL_DENIED` | 관련 민감 동작 승인이 거부되었습니다. |
| `APPROVAL_EXPIRED` | 관련 민감 동작 승인이 만료되었거나 범위/baseline에서 달라졌습니다. |
| `CAPABILITY_INSUFFICIENT` | 접점은 인식되었지만 필요한 관찰, 캡처, 로컬 접근, 차단/격리 조건, 보장 주장, 활성 동작을 충족할 수 없습니다. |
| `MCP_UNAVAILABLE` | 필요한 MCP/Core 접근을 사용할 수 없거나, 오래되었거나, 닿을 수 없습니다. |
| `LOCAL_ACCESS_MISMATCH` | 도달 가능한 로컬 호출자/접근 경로가 등록된 로컬 프로필 밖에 있거나 필요한 로컬 접근 권한이 없습니다. |
| `EVIDENCE_INSUFFICIENT` | 필요한 증거 범위가 없거나, 부분적이거나, 오래되었거나, 막혔습니다. |
| `ACCEPTANCE_REQUIRED` | 필요한 최종 수락이 대기 중이거나, 거부되었거나, 표시된 결과 근거와 호환되지 않습니다. |
| `PROJECTION_STALE` | 요청한 읽기용 상태/보기가 오래되었거나 실패했습니다. Core 상태가 아니며 그 자체로 닫기 차단 사유가 아닙니다. |
| `RESIDUAL_RISK_NOT_VISIBLE` | 닫기에 영향을 주는 알려진 잔여 위험이 최종 수락 또는 닫기 전에 보이지 않았습니다. |
| `ARTIFACT_MISSING` | 참조한 아티팩트가 없거나 무결성/메타데이터 확인에 실패했습니다. |
| `BASELINE_STALE` | 동작에 필요한 저장소 상태와 baseline이 더 이상 맞지 않습니다. |
| `VALIDATOR_FAILED` | 필수 활성 validator 또는 차단 사유 확인이 실패했고, 더 구체적인 타입 코드가 없을 때 쓰는 대체 코드입니다. |

`ToolError.details.authorization_reason`은 정확히 다음 값만 사용합니다.

```text
missing | expired | stale | revoked | consumed | incompatible
```

필요한 권한이 제공되지 않았으면 `authorization_reason=missing`과 함께 `WRITE_AUTHORIZATION_REQUIRED`를 사용합니다. 기존 권한을 소비할 수 없으면 `WRITE_AUTHORIZATION_INVALID`를 사용합니다.

<a id="primary-error-code-precedence"></a>

## 기본 `ErrorCode` 우선순위

`ToolResponseBase.errors`가 비어 있지 않으면 메서드 섹션이 더 좁은 순서를 정의하지 않는 한 `errors[0]`이 아래 순서로 선택된 기본 오류입니다. 보조 차단 사유는 메서드별 필드와 `ToolError.details`에 남을 수 있습니다.

| 우선순위 | 기본 `ErrorCode` |
|---:|---|
| 1 | `VALIDATION_FAILED` |
| 2 | `STATE_CONFLICT` |
| 3 | `MCP_UNAVAILABLE` |
| 4 | `LOCAL_ACCESS_MISMATCH` |
| 5 | `NO_ACTIVE_TASK` |
| 6 | `NO_ACTIVE_CHANGE_UNIT` |
| 7 | `BASELINE_STALE` |
| 8 | `SCOPE_REQUIRED` |
| 9 | `SCOPE_VIOLATION` |
| 10 | `WRITE_AUTHORIZATION_REQUIRED` |
| 11 | `WRITE_AUTHORIZATION_INVALID` |
| 12 | `APPROVAL_DENIED` |
| 13 | `APPROVAL_EXPIRED` |
| 14 | `APPROVAL_REQUIRED` |
| 15 | `DECISION_UNRESOLVED` |
| 16 | `AUTONOMY_BOUNDARY_EXCEEDED` |
| 17 | `DECISION_REQUIRED` |
| 18 | `CAPABILITY_INSUFFICIENT` |
| 19 | `EVIDENCE_INSUFFICIENT` |
| 20 | `RESIDUAL_RISK_NOT_VISIBLE` |
| 21 | `ACCEPTANCE_REQUIRED` |
| 22 | `PROJECTION_STALE` |
| 23 | `ARTIFACT_MISSING` |
| 24 | `VALIDATOR_FAILED` |

<a id="blocked-and-dry-run-behavior"></a>

## 차단 응답과 `dry_run` 동작

차단 응답은 커밋 전 실패와 다릅니다. 메서드 담당 문서가 차단 사유 기록을 허용하는 경우에만 Core가 차단 응답을 커밋할 수 있습니다. 커밋된 차단 응답은 `blockers`, 이벤트, 상태 버전, 멱등 재실행을 업데이트할 수 있지만, 차단 사유가 없다고 말하는 권한을 만들면 안 됩니다.

`dry_run=true`는 항상 기준 권한이 아닙니다. 검증하고 진단, 후보 차단 사유, 변경 예상 요약을 반환할 수 있지만 현재 기록, 이벤트, 아티팩트, 증거 요약, 소비 가능한 Write Authorization, 닫기 상태, 커밋된 재실행 행을 만들거나 업데이트하면 안 됩니다. 이후 비 `dry_run` 호출은 현재 상태를 기준으로 다시 검증해야 합니다.

<a id="idempotency"></a>

## 멱등성

커밋되는 상태 변경 메서드는 모두 `idempotency_key`를 요구합니다. 키는 `(project_id, tool_name, idempotency_key)` 범위를 가집니다.

`request_hash`는 도구 이름, 스키마 정규화된 요청 본문, 그리고 `request_id`와 `idempotency_key`를 제외한 모든 `ToolEnvelope` 필드에 대한 정규 JSON에서 계산합니다.

같은 키와 같은 hash를 가진 커밋된 재실행 행이 있으면 Core는 최신성 확인을 다시 실행하거나 이벤트 추가, 아티팩트 등록, 권한 소비, 차단 사유 업데이트, 재실행 행 변경을 하지 않고 원래 커밋된 응답을 반환합니다. 같은 키를 다른 hash로 재사용하면 Core는 `STATE_CONFLICT`를 반환하고 원래 재실행 행을 보존합니다.

`dry_run` 호출과 커밋 전 실패는 재실행 행을 만들거나 예약하지 않습니다.

<a id="state-conflict-behavior"></a>

## 상태 충돌 처리

커밋된 재실행 행이 없는 새 상태 변경 시도에서 Core는 최신성 확인 전에 기본 Task를 찾습니다. 해석 순서는 도구별 `task_id`, `ToolEnvelope.task_id`, 활성 Task입니다.

Task 범위 변경은 `expected_state_version`을 `tasks.state_version`과 비교합니다. 찾은 기본 Task가 없는 프로젝트 범위 변경은 `project_state.state_version`과 비교합니다. 불일치하면 `STATE_CONFLICT`를 반환하고 현재 기록, 이벤트, 아티팩트, 증거 요약, Write Authorization, 닫기 상태, 재실행 행을 만들지 않습니다.

`STATE_CONFLICT.details`에는 다음 값을 담아야 합니다.

```yaml
scope: task | project
current_state_version: integer
expected_state_version: integer
project_id: string
task_id: string | null
```

`WriteAuthorization.basis_state_version`은 허용 결정의 호환성 근거입니다. 반드시 결과 `ToolResponseBase.state_version`과 같지는 않습니다.

<a id="harnessclose_task-close-blockers"></a>

## `harness.close_task` 닫기 차단 사유

`CloseTaskResponse.blockers`는 [API Schema Core](schema-core.md#current-position-display-schemas)의 구조화된 `CloseBlocker` 객체를 사용해야 합니다. 설명 문구만 있는 상태 텍스트, 보고서 텍스트, 렌더링된 보기, 에이전트 요약은 닫기 차단 사유 결과가 아닙니다.

닫기 차단 사유는 공개 오류와 매핑될 때 기본 오류 우선순위에 따라 정렬합니다. 증거 차단 사유는 보통 `EVIDENCE_INSUFFICIENT`를 사용합니다. 아티팩트 사용 가능성 차단 사유는 `ARTIFACT_MISSING`을 사용합니다. 해결되지 않은 사용자 판단 차단 사유는 `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED`를 사용합니다. 민감 동작 승인 차단 사유는 `APPROVAL_*` 코드를 사용합니다. 범위 차단 사유는 범위와 baseline 코드를 사용합니다.

닫기에 영향을 주는 알려진 잔여 위험이 아직 보이지 않으면 `RESIDUAL_RISK_NOT_VISIBLE`를 사용합니다. 보이지만 수락되지 않은 닫기 관련 잔여 위험은 이 코드 아래 숨기지 않습니다. 잔여 위험 수락이 필요하면 닫기 차단 사유는 `category=residual_risk_acceptance`와 `required_judgment_kind=residual_risk_acceptance`를 사용하고, `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED`를 반환합니다.

`PROJECTION_STALE`은 읽기용 보기 최신성 오류입니다. 그 자체로 활성 닫기 차단 사유 `category`가 아닙니다.

## 사용자 표시 라벨 지침

아래 라벨은 표시 지침이지 새 공개 오류 코드가 아닙니다.

| API 조건 | 사용자 표시 라벨 | 차단 해소에 필요한 최소 조치 |
|---|---|---|
| `VALIDATION_FAILED` | 잘못된 요청 | 다시 시도하기 전에 요청 본문, enum 값, 활성화 규칙, 필드 집합을 고칩니다. |
| `STATE_CONFLICT` | 상태 충돌 | 현재 상태를 새로 고치고 현재 상태 버전으로 다시 시도하거나 원래 멱등 요청을 재실행합니다. |
| `MCP_UNAVAILABLE` | Core 사용 불가 | 상태 변경, gate 업데이트, 쓰기 호환성, 닫기를 주장하기 전에 Core 접근을 다시 연결하거나 진단합니다. |
| `LOCAL_ACCESS_MISMATCH` | 로컬 접근 거부 또는 역량 불일치 | 등록된 로컬 접점을 사용하거나, 로컬 접근을 복구하거나, 역량이 있는 접점으로 옮깁니다. |
| `CAPABILITY_INSUFFICIENT` | 지원되지 않거나 부족한 접점 | 역량이 있는 접점을 사용하거나, 동작을 줄이거나, 누락된 역량이 필요 없는 경로를 선택합니다. |
| `NO_ACTIVE_TASK` | 활성 Task 없음 | Task 범위 동작 전에 Task를 선택하거나 생성합니다. |
| `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | 범위, 경계, baseline 문제 | 범위를 확인하거나 좁히고, Change Unit이나 baseline을 업데이트하거나, 필요한 사용자 판단을 요청합니다. |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | 쓰기 전 범위 확인 없음 또는 오래됨 | 정확한 동작, 현재 범위, 현재 상태로 `harness.prepare_write`를 호출하거나 다시 시도합니다. |
| `DECISION_REQUIRED`, `DECISION_UNRESOLVED` | 판단 필요 | 종류, 참조, 선택지, 결과와 함께 집중된 `UserJudgment`를 보여 주거나 해결합니다. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` | 민감 동작 승인 필요 또는 사용 불가 | `judgment_kind=sensitive_approval` 사용자 판단을 요청, 해결, 갱신합니다. |
| `EVIDENCE_INSUFFICIENT` | 증거 필요 | 누락된 확인을 기록하거나 다시 실행하고, 증거 공백과 차단 해소에 필요한 최소 조치를 보여 줍니다. |
| `ACCEPTANCE_REQUIRED` | 최종 수락 필요 | 표시된 결과 근거에 대해 `judgment_kind=final_acceptance`를 요청하거나 해결합니다. |
| `RESIDUAL_RISK_NOT_VISIBLE` | 잔여 위험이 보이지 않음 | 최종 수락 또는 닫기 전에 닫기 관련 잔여 위험을 보여 줍니다. |
| `PROJECTION_STALE` | 읽기용 보기 오래됨 | 그 보기를 새로 고친 뒤 의존합니다. 기준 닫기 상태로 취급하지 않습니다. |
| `ARTIFACT_MISSING` | 아티팩트 문제 | 누락되었거나 실패한 아티팩트를 다시 첨부, 다시 생성, 교체한 뒤 의존합니다. |
| `VALIDATOR_FAILED` | 확인 또는 차단 사유 실패 | 특정 validator 또는 차단 사유를 보여 줍니다. 타입 있는 차단 사유가 없을 때만 이 대체 코드를 사용합니다. |
