# API 오류

이 문서는 하네스 API 응답의 공개 오류 계약을 정의합니다. 렌더링 라벨, 메시지 문구, 템플릿, 저장소 행, 런타임 출력은 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 공개 `ErrorCode` 식별자: 공개 코드 집합, 공개 의미, 각 코드를 실을 수 있는 공개 경로입니다.
- 오류 우선순위: 응답 분기에 공개 오류가 여러 개 있을 때 `errors[0]`을 고르는 방식입니다.
- 오류와 차단 사유 경로: 거부 응답, 차단 결과, `dry_run` 미리보기 중 조건이 속하는 곳입니다.
- `STATE_VERSION_CONFLICT`: 공개 오래된 상태와 멱등성 충돌 동작입니다.
- `ToolError.details`에 실리는 기계 판독용 오류 세부 필드와 보조 값입니다.

이 문서는 담당하지 않습니다.

- 메서드 요청 본문 스키마, 응답 필드 형태, 공통 요청/응답 래퍼:
  - [API 코어 스키마](schema-core.md)
  - [API 메서드](methods.md)가 안내하는 메서드 담당 문서
  - API 스키마 담당 문서
- Core 권한 확인, 사용자 판단 의미, 닫기 준비 상태 의미:
  - [Core 모델](../core-model.md)
  - [사용자 판단 메서드](method-user-judgment.md)
  - [Task 닫기 메서드](method-close-task.md)
- `CloseReadinessBlocker`, `WriteDecisionReason`, `PlannedBlocker`, 값 집합 필드 정의:
  - [API 상태 스키마](schema-state.md)
  - [API 코어 스키마](schema-core.md)
  - [API 값 집합](schema-value-sets.md)
- 저장소 행, 재실행 행, DDL, 잠금, 마이그레이션, 저장 효과:
  - [저장소 기록](../storage-records.md)
  - [저장 효과](../storage-effects.md)
  - [저장소 버전 관리](../storage-versioning.md)
- 보안 보장 표현과 접근 경계 주장:
  - [보안](../security.md)
- 사용자 표시 라벨, 렌더링 메시지 문구, 템플릿 표현:
  - [템플릿 본문](../template-bodies.md)

## 오류와 차단 사유

| 개념 | 공개 형태 | 세부 항목 |
|---|---|---|
| 거부 응답 | `ToolRejectedResponse.errors[]` | [거부 응답](#error-vs-blocker-rejected-response) |
| 차단 결과 | 메서드별 결과 필드 | [차단 결과](#error-vs-blocker-blocked-result) |
| `dry_run` 미리보기 | `ToolDryRunResponse` | [`dry_run` 미리보기](#error-vs-blocker-dry-run-preview) |

<a id="error-vs-blocker-rejected-response"></a>
거부 응답:
- 공개 형태: `ToolRejectedResponse.errors[]`와 `ToolError.code: ErrorCode`.
- 의미: 메서드가 커밋되는 동작으로 진행하지 않았다는 뜻입니다.
- 조건: 공개 전송, 요청, 최신성, 로컬 접근, 역량, 선행조건 거부입니다.
- 상태 영향: 커밋된 동작이 없고 상태 변경도 없습니다.

<a id="error-vs-blocker-blocked-result"></a>
차단 결과:
- 공개 형태: `write_decision_reasons`나 `blockers` 같은 메서드별 결과 필드입니다.
- 의미: 메서드가 동작별 차단 결과를 반환했을 수 있다는 뜻입니다.
- 비주장: 공개 전송 또는 스키마 오류가 아닙니다.
- 상태 영향: 메서드 담당 문서가 허용한 커밋된 차단 결과나 읽기 전용 차단 사유 데이터만 가능합니다.

<a id="error-vs-blocker-dry-run-preview"></a>
`dry_run` 미리보기:
- 공개 형태: `DryRunSummary.would_errors[]` 또는 `DryRunSummary.would_blockers[]`를 담은 `ToolDryRunResponse`입니다.
- 의미: 유효한 `dry_run` 요청에서 미리 볼 수 있는 진단입니다.
- 상태 영향: 커밋된 쓰기가 아니며 저장된 차단 사유 상태도 아닙니다.

`ErrorCode` 값은 공개 API 식별자입니다. 차단 사유 코드는 동작별 결과 값입니다. 공개 `ErrorCode`는 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 차단 사유 코드로 재사용하면 안 됩니다.

렌더링 라벨과 메시지는 [템플릿 본문](../template-bodies.md)이 담당하는 표시 문구입니다. 이 값을 `ErrorCode`, 차단 사유 코드, 기계 판독용 `ToolError.details` 키로 사용하면 안 됩니다.

<a id="error-taxonomy"></a>
## 공개 `ErrorCode` 요약

| 공개 `ErrorCode` | 세부 항목 |
|---|---|
| `VALIDATION_FAILED` | [`VALIDATION_FAILED`](#errorcode-validation-failed) |
| `STATE_VERSION_CONFLICT` | [`STATE_VERSION_CONFLICT`](#errorcode-state-version-conflict) |
| `MCP_UNAVAILABLE` | [`MCP_UNAVAILABLE`](#errorcode-mcp-unavailable) |
| `LOCAL_ACCESS_MISMATCH` | [`LOCAL_ACCESS_MISMATCH`](#errorcode-local-access-mismatch) |
| `NO_ACTIVE_TASK` | [`NO_ACTIVE_TASK`](#errorcode-no-active-task) |
| `NO_ACTIVE_CHANGE_UNIT` | [`NO_ACTIVE_CHANGE_UNIT`](#errorcode-no-active-change-unit) |
| `BASELINE_STALE` | [`BASELINE_STALE`](#errorcode-baseline-stale) |
| `SCOPE_REQUIRED` | [`SCOPE_REQUIRED`](#errorcode-scope-required) |
| `SCOPE_VIOLATION` | [`SCOPE_VIOLATION`](#errorcode-scope-violation) |
| `WRITE_AUTHORIZATION_REQUIRED` | [`WRITE_AUTHORIZATION_REQUIRED`](#errorcode-write-authorization-required) |
| `WRITE_AUTHORIZATION_INVALID` | [`WRITE_AUTHORIZATION_INVALID`](#errorcode-write-authorization-invalid) |
| `APPROVAL_DENIED` | [`APPROVAL_DENIED`](#errorcode-approval-denied) |
| `APPROVAL_EXPIRED` | [`APPROVAL_EXPIRED`](#errorcode-approval-expired) |
| `APPROVAL_REQUIRED` | [`APPROVAL_REQUIRED`](#errorcode-approval-required) |
| `DECISION_UNRESOLVED` | [`DECISION_UNRESOLVED`](#errorcode-decision-unresolved) |
| `AUTONOMY_BOUNDARY_EXCEEDED` | [`AUTONOMY_BOUNDARY_EXCEEDED`](#errorcode-autonomy-boundary-exceeded) |
| `DECISION_REQUIRED` | [`DECISION_REQUIRED`](#errorcode-decision-required) |
| `CAPABILITY_INSUFFICIENT` | [`CAPABILITY_INSUFFICIENT`](#errorcode-capability-insufficient) |
| `EVIDENCE_INSUFFICIENT` | [`EVIDENCE_INSUFFICIENT`](#errorcode-evidence-insufficient) |
| `RESIDUAL_RISK_NOT_VISIBLE` | [`RESIDUAL_RISK_NOT_VISIBLE`](#errorcode-residual-risk-not-visible) |
| `ACCEPTANCE_REQUIRED` | [`ACCEPTANCE_REQUIRED`](#errorcode-acceptance-required) |
| `PROJECTION_STALE` | [`PROJECTION_STALE`](#errorcode-projection-stale) |
| `ARTIFACT_MISSING` | [`ARTIFACT_MISSING`](#errorcode-artifact-missing) |
| `VALIDATOR_FAILED` | [`VALIDATOR_FAILED`](#errorcode-validator-failed) |

<a id="errorcode-validation-failed"></a>
### `VALIDATION_FAILED`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 요청 본문 형태, enum 값, 활성화 규칙, 프로필 검증, 아티팩트 입력 형태가 유효하지 않습니다.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

허용되지 않는 것:
- 요청 거부에서는 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-state-version-conflict"></a>
### `STATE_VERSION_CONFLICT`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- `expected_state_version`이 오래된 상태입니다.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

허용되지 않는 것:
- 이 값을 닫기 준비 상태 차단 사유 코드로 사용하지 않습니다.

관련 충돌 세부사항:
- 오래된 `WriteAuthorization.basis_state_version`과 멱등 요청 해시 충돌은 [상태 버전 충돌](#state-conflict-behavior)에서 다룹니다.

<a id="errorcode-mcp-unavailable"></a>
### `MCP_UNAVAILABLE`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 필요한 Core, MCP, 접점 도달 가능성을 사용할 수 없습니다.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

허용되지 않는 것:
- 요청 거부에서는 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-local-access-mismatch"></a>
### `LOCAL_ACCESS_MISMATCH`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 도달 가능한 로컬 접근이 등록된 전송 경로, 세션, 바인딩, 프로젝트, 접점 인스턴스와 맞지 않거나 접근이 철회되었습니다.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

허용되지 않는 것:
- 요청 거부에서는 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-no-active-task"></a>
### `NO_ACTIVE_TASK`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- `Task`가 필요하지만 활성 `Task`나 지정된 `Task`가 없습니다.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-no-active-change-unit"></a>
### `NO_ACTIVE_CHANGE_UNIT`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 쓰기 가능하거나 닫기와 관련된 동작에 활성 범위 지정 Change Unit이 없습니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-baseline-stale"></a>
### `BASELINE_STALE`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 동작에 필요한 저장소 상태와 기준 상태가 더 이상 맞지 않습니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-scope-required"></a>
### `SCOPE_REQUIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 요청한 쓰기나 동작 전에 범위 확인이 필요합니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-scope-violation"></a>
### `SCOPE_VIOLATION`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 의도했거나 관찰된 경로 또는 민감 범주가 활성 범위나 저장된 승인 범위를 넘었습니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-write-authorization-required"></a>
### `WRITE_AUTHORIZATION_REQUIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 쓰기 가능한 실행 기록에 필요한 `Write Authorization`이 없습니다.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-write-authorization-invalid"></a>
### `WRITE_AUTHORIZATION_INVALID`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 제공된 `Write Authorization`이 만료, 철회, 소비, 또는 버전 외 사유로 비호환입니다.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-approval-denied"></a>
### `APPROVAL_DENIED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 필요한 민감 동작 승인이 거부되었습니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-approval-expired"></a>
### `APPROVAL_EXPIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 필요한 민감 동작 승인이 만료되었거나 범위 또는 기준 상태와 달라졌습니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-approval-required"></a>
### `APPROVAL_REQUIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 진행 전에 민감 동작 승인이 필요합니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-decision-unresolved"></a>
### `DECISION_UNRESOLVED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 관련 사용자 판단이 대기, 적용 범위 없는 보류, 거부, 차단, 오래됨, 대체됨, 비호환 상태입니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-autonomy-boundary-exceeded"></a>
### `AUTONOMY_BOUNDARY_EXCEEDED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 의도한 동작이 활성 Change Unit Autonomy Boundary를 넘었습니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-decision-required"></a>
### `DECISION_REQUIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 진행 전에 차단 중인 사용자 소유 판단을 요청해야 합니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-capability-insufficient"></a>
### `CAPABILITY_INSUFFICIENT`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 접점은 인식되었지만 필요한 접근 등급, 관찰, 캡처, 보장 지원, 활성 동작이 없습니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 기준 메서드나 스키마 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-evidence-insufficient"></a>
### `EVIDENCE_INSUFFICIENT`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 필요한 증거 범위가 없거나, 부분적이거나, 오래되었거나, 막혔습니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 닫기 준비 상태 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-residual-risk-not-visible"></a>
### `RESIDUAL_RISK_NOT_VISIBLE`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 닫기에 영향을 주는 알려진 잔여 위험이 최종 수락이나 닫기 전에 보이지 않았습니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 닫기 준비 상태 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-acceptance-required"></a>
### `ACCEPTANCE_REQUIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 필요한 최종 수락이 대기 중이거나, 거부되었거나, 표시된 결과 근거와 호환되지 않습니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 닫기 준비 상태 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-projection-stale"></a>
### `PROJECTION_STALE`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 요청한 읽기용 상태나 보기가 오래되었거나 실패했습니다.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

허용되지 않는 것:
- 이 값만으로 닫기 준비 상태 차단 사유 코드를 만들지 않습니다.

<a id="errorcode-artifact-missing"></a>
### `ARTIFACT_MISSING`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 참조한 지속 아티팩트가 없거나, 사용할 수 없거나, 닫기 근거로 쓸 수 없거나, 무결성/메타데이터 확인에 실패했습니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 닫기 준비 상태 담당 문서가 명시적으로 허용하지 않는 한 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="errorcode-validator-failed"></a>
### `VALIDATOR_FAILED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 필요한 활성 검증기나 차단 사유 확인이 실패했고 더 구체적인 타입 코드가 없을 때 쓰는 대체 코드입니다.

상태 영향:
- 거부 경로에서는 커밋되는 동작이 진행되지 않고 담당 상태 변경이 발생하지 않습니다.
- 담당 문서가 정의한 결과 경로에서는 담당 메서드나 스키마만 커밋된 결과의 상태 영향을 정의할 수 있습니다.

허용되지 않는 것:
- 더 구체적인 활성 코드가 있으면 이 대체 코드를 사용하지 않습니다.
- 담당 메서드나 스키마의 대체 코드 범위 밖에서 이 값을 차단 사유 코드로 사용하지 않습니다.

`ToolError.details.authorization_reason`은 `missing`, `expired`, `stale`, `revoked`, `consumed`, `incompatible`만 사용합니다. 오래된 `WriteAuthorization.basis_state_version`은 `WRITE_AUTHORIZATION_INVALID`가 아니라 `STATE_VERSION_CONFLICT`를 사용합니다.

`ToolError.details.artifact_input_error.reason`은 아래 세부 보조 값을 사용합니다. 이 값들은 최상위 공개 `ErrorCode` 값이 아닙니다. 스테이징된 아티팩트 핸들 검증 실패는 실제 실패가 요청 수준 로컬 접근이나 역량 확인이 아닌 한 공개 코드 `VALIDATION_FAILED`를 유지합니다.

| `artifact_input_error.reason` | 의미 |
|---|---|
| `staged_handle_expired` | 스테이징된 아티팩트 핸들의 사용 가능 시간이 지났습니다. |
| `staged_handle_consumed` | 스테이징된 아티팩트 핸들이 이미 소비되었습니다. |
| `staged_handle_project_mismatch` | 스테이징된 아티팩트 핸들이 다른 프로젝트에 속합니다. |
| `staged_handle_task_mismatch` | 스테이징된 아티팩트 핸들이 다른 `Task`에 속합니다. |
| `staged_handle_surface_mismatch` | 스테이징된 아티팩트 핸들의 출처가 확인된 접점과 맞지 않습니다. |
| `staged_handle_checksum_mismatch` | 스테이징된 바이트가 예상 체크섬과 맞지 않습니다. |
| `staged_handle_size_mismatch` | 스테이징된 바이트가 예상 크기와 맞지 않습니다. |
| `staged_handle_not_found` | 스테이징된 아티팩트 핸들을 찾을 수 없습니다. |

<a id="primary-error-code-precedence"></a>

## 오류 우선순위

오류를 담는 분기의 `errors`가 비어 있지 않으면 메서드 담당 문서가 더 좁은 메서드별 순서를 정의하지 않는 한 아래 순서로 `errors[0]` 공개 주 오류를 고릅니다.

| 우선순위 | 주 `ErrorCode` | 세부 항목 |
|---:|---|---|
| 1 | `VALIDATION_FAILED` | [`VALIDATION_FAILED`](#precedence-validation-failed) |
| 2 | `STATE_VERSION_CONFLICT` | [`STATE_VERSION_CONFLICT`](#state-version-conflict-precedence-exclusion) |
| 3 | `MCP_UNAVAILABLE` | [`MCP_UNAVAILABLE`](#precedence-mcp-unavailable) |
| 4 | `LOCAL_ACCESS_MISMATCH` | [`LOCAL_ACCESS_MISMATCH`](#precedence-local-access-mismatch) |
| 5 | `NO_ACTIVE_TASK` | [`NO_ACTIVE_TASK`](#precedence-no-active-task) |
| 6 | `NO_ACTIVE_CHANGE_UNIT` | [`NO_ACTIVE_CHANGE_UNIT`](#precedence-no-active-change-unit) |
| 7 | `BASELINE_STALE` | [`BASELINE_STALE`](#precedence-baseline-stale) |
| 8 | `SCOPE_REQUIRED` | [`SCOPE_REQUIRED`](#precedence-scope-required) |
| 9 | `SCOPE_VIOLATION` | [`SCOPE_VIOLATION`](#precedence-scope-violation) |
| 10 | `WRITE_AUTHORIZATION_REQUIRED` | [`WRITE_AUTHORIZATION_REQUIRED`](#precedence-write-authorization-required) |
| 11 | `WRITE_AUTHORIZATION_INVALID` | [`WRITE_AUTHORIZATION_INVALID`](#precedence-write-authorization-invalid) |
| 12 | `APPROVAL_DENIED` | [`APPROVAL_DENIED`](#precedence-approval-denied) |
| 13 | `APPROVAL_EXPIRED` | [`APPROVAL_EXPIRED`](#precedence-approval-expired) |
| 14 | `APPROVAL_REQUIRED` | [`APPROVAL_REQUIRED`](#precedence-approval-required) |
| 15 | `DECISION_UNRESOLVED` | [`DECISION_UNRESOLVED`](#precedence-decision-unresolved) |
| 16 | `AUTONOMY_BOUNDARY_EXCEEDED` | [`AUTONOMY_BOUNDARY_EXCEEDED`](#precedence-autonomy-boundary-exceeded) |
| 17 | `DECISION_REQUIRED` | [`DECISION_REQUIRED`](#precedence-decision-required) |
| 18 | `CAPABILITY_INSUFFICIENT` | [`CAPABILITY_INSUFFICIENT`](#precedence-capability-insufficient) |
| 19 | `EVIDENCE_INSUFFICIENT` | [`EVIDENCE_INSUFFICIENT`](#precedence-evidence-insufficient) |
| 20 | `RESIDUAL_RISK_NOT_VISIBLE` | [`RESIDUAL_RISK_NOT_VISIBLE`](#precedence-residual-risk-not-visible) |
| 21 | `ACCEPTANCE_REQUIRED` | [`ACCEPTANCE_REQUIRED`](#precedence-acceptance-required) |
| 22 | `PROJECTION_STALE` | [`PROJECTION_STALE`](#precedence-projection-stale) |
| 23 | `ARTIFACT_MISSING` | [`ARTIFACT_MISSING`](#precedence-artifact-missing) |
| 24 | `VALIDATOR_FAILED` | [`VALIDATOR_FAILED`](#precedence-validator-failed) |

<a id="precedence-validation-failed"></a>
### 우선순위 1: `VALIDATION_FAILED`

적용 대상:
- 거부된 요청 형태 또는 검증 실패입니다.

<a id="precedence-mcp-unavailable"></a>
### 우선순위 3: `MCP_UNAVAILABLE`

적용 대상:
- Core, MCP, 접점 도달 가능성 실패로 거부된 경우입니다.

<a id="precedence-local-access-mismatch"></a>
### 우선순위 4: `LOCAL_ACCESS_MISMATCH`

적용 대상:
- 로컬 접근 바인딩 불일치나 철회로 거부된 경우입니다.

<a id="precedence-no-active-task"></a>
### 우선순위 5: `NO_ACTIVE_TASK`

적용 대상:
- `Task` 식별자가 없어 거부된 경우입니다.

<a id="precedence-no-active-change-unit"></a>
### 우선순위 6: `NO_ACTIVE_CHANGE_UNIT`

적용 대상:
- 활성 Change Unit이 없는 경우입니다.

<a id="precedence-baseline-stale"></a>
### 우선순위 7: `BASELINE_STALE`

적용 대상:
- 기준 상태가 오래된 경우입니다.

<a id="precedence-scope-required"></a>
### 우선순위 8: `SCOPE_REQUIRED`

적용 대상:
- 필요한 범위 확인이 없는 경우입니다.

<a id="precedence-scope-violation"></a>
### 우선순위 9: `SCOPE_VIOLATION`

적용 대상:
- 범위 또는 승인된 시도 범위를 위반한 경우입니다.

<a id="precedence-write-authorization-required"></a>
### 우선순위 10: `WRITE_AUTHORIZATION_REQUIRED`

적용 대상:
- 필요한 Write Authorization이 없는 경우입니다.

<a id="precedence-write-authorization-invalid"></a>
### 우선순위 11: `WRITE_AUTHORIZATION_INVALID`

적용 대상:
- 버전 외 사유로 Write Authorization을 사용할 수 없는 경우입니다.

<a id="precedence-approval-denied"></a>
### 우선순위 12: `APPROVAL_DENIED`

적용 대상:
- 민감 동작 승인이 거부된 경우입니다.

<a id="precedence-approval-expired"></a>
### 우선순위 13: `APPROVAL_EXPIRED`

적용 대상:
- 민감 동작 승인이 만료되었거나 달라진 경우입니다.

<a id="precedence-approval-required"></a>
### 우선순위 14: `APPROVAL_REQUIRED`

적용 대상:
- 민감 동작 승인이 없는 경우입니다.

<a id="precedence-decision-unresolved"></a>
### 우선순위 15: `DECISION_UNRESOLVED`

적용 대상:
- 기존 사용자 판단을 사용할 수 없는 경우입니다.

<a id="precedence-autonomy-boundary-exceeded"></a>
### 우선순위 16: `AUTONOMY_BOUNDARY_EXCEEDED`

적용 대상:
- 자율성 경계를 넘은 경우입니다.

<a id="precedence-decision-required"></a>
### 우선순위 17: `DECISION_REQUIRED`

적용 대상:
- 새 사용자 소유 판단이 필요한 경우입니다.

<a id="precedence-capability-insufficient"></a>
### 우선순위 18: `CAPABILITY_INSUFFICIENT`

적용 대상:
- 접점 역량이 부족한 경우입니다.

<a id="precedence-evidence-insufficient"></a>
### 우선순위 19: `EVIDENCE_INSUFFICIENT`

적용 대상:
- 증거 범위가 충분하지 않은 경우입니다.

<a id="precedence-residual-risk-not-visible"></a>
### 우선순위 20: `RESIDUAL_RISK_NOT_VISIBLE`

적용 대상:
- 닫기 관련 위험이 보이지 않는 경우입니다.

<a id="precedence-acceptance-required"></a>
### 우선순위 21: `ACCEPTANCE_REQUIRED`

적용 대상:
- 최종 수락이 필요하거나 호환되지 않는 경우입니다.

<a id="precedence-projection-stale"></a>
### 우선순위 22: `PROJECTION_STALE`

적용 대상:
- 읽기용 보기가 오래되었거나 실패한 경우입니다.

<a id="precedence-artifact-missing"></a>
### 우선순위 23: `ARTIFACT_MISSING`

적용 대상:
- 지속 아티팩트가 없거나, 사용할 수 없거나, 사용할 수 있는 상태가 아니거나, 실패한 경우입니다.

<a id="precedence-validator-failed"></a>
### 우선순위 24: `VALIDATOR_FAILED`

적용 대상:
- 더 구체적인 활성 코드가 없을 때 쓰는 타입 있는 대체 코드입니다.

<a id="state-version-conflict-precedence-exclusion"></a>
### `STATE_VERSION_CONFLICT` 우선순위 제외

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 오래된 `expected_state_version` 때문에 메서드가 진행될 수 없어 거부 응답이 선택됩니다.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

허용되지 않는 것:
- `STATE_VERSION_CONFLICT`를 `MethodResult.base.errors[0]`, `CloseTaskResult(close_state=blocked).errors[0]`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, `PlannedBlocker.code`로 선택하지 않습니다.

관련 충돌 세부사항:
- 오래된 `WriteAuthorization.basis_state_version`과 멱등 요청 해시 충돌은 [상태 버전 충돌](#state-conflict-behavior)에서 다룹니다.

<a id="blocked-and-dry-run-behavior"></a>

## 거부 응답 동작

| 조건 | 세부 항목 |
|---|---|
| 요청 검증이 진행 전에 실패 | [요청 검증 실패](#rejected-request-validation-failure) |
| 선행조건이 커밋 전에 실패 | [선행조건 실패](#rejected-precondition-failure) |
| 상태 또는 멱등성 충돌 | [상태 또는 멱등성 충돌](#rejected-state-or-idempotency-conflict) |
| `dry_run=true` 미리보기 전 실패 | [`dry_run=true` 미리보기 전 실패](#rejected-dry-run-pre-preview-failure) |

<a id="rejected-request-validation-failure"></a>
### 요청 검증 실패

조건:
- 메서드가 진행되기 전에 요청 형태, 스키마, 프로필, 스테이징된 아티팩트 핸들 검증이 실패합니다.

라우팅:
- `ToolRejectedResponse.errors[]`.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

허용되지 않는 것:
- 메서드별 결과 전용 필드를 넣지 않습니다.

<a id="rejected-precondition-failure"></a>
### 선행조건 실패

조건:
- 커밋 전에 Core, MCP, 로컬 접근, 접점 역량, 상태 조회, `Task` 식별자, 필요한 선행조건이 실패합니다.

라우팅:
- `ToolRejectedResponse.errors[]`.

상태 영향:
- 기록, 재실행 행, 아티팩트, 이벤트, Write Authorization 소비, 닫기 상태 변경, 상태 버전 증가가 없습니다.

<a id="rejected-state-or-idempotency-conflict"></a>
### 상태 또는 멱등성 충돌

조건:
- `expected_state_version`, `WriteAuthorization.basis_state_version`, 멱등 요청 해시가 오래되었거나 충돌합니다.

라우팅:
- `STATE_VERSION_CONFLICT`를 담은 `ToolRejectedResponse.errors[]`.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

허용되지 않는 것:
- 이 충돌은 차단 사유가 아닙니다.

<a id="rejected-dry-run-pre-preview-failure"></a>
### `dry_run=true` 미리보기 전 실패

조건:
- `dry_run=true` 요청이 읽기 결과나 `dry_run` 미리보기를 만들기 전에 실패합니다.

라우팅:
- `dry_run=true`인 `ToolRejectedResponse`.

상태 영향:
- 커밋되는 동작이나 `dry_run` 미리보기가 만들어지지 않습니다.

허용되지 않는 것:
- 이 거부를 `DryRunSummary.would_errors[]`나 `PlannedBlocker`로 표현하지 않습니다.

거부 응답은 메서드가 커밋되는 동작으로 진행하지 않았다는 뜻입니다. 거부 응답은 차단 결과가 아니며, 요청에 없던 권한, 증거, 수락, 닫기 상태를 만들지 않습니다.

## 차단 결과 동작

| 차단 경로 | 세부 항목 |
|---|---|
| `PrepareWriteResult` 차단 판단 | [`PrepareWriteResult` 차단 판단](#blocked-prepare-write-result) |
| `CloseTaskResult(close_state=blocked)` | [`CloseTaskResult(close_state=blocked)`](#blocked-close-task-result) |
| 읽기 전용 닫기 차단 사유 관찰 | [읽기 전용 관찰](#blocked-read-only-observation) |

<a id="blocked-prepare-write-result"></a>
### `PrepareWriteResult` 차단 판단

조건:
- `PrepareWriteResult`가 `decision=blocked`, `decision=approval_required`, `decision=decision_required` 중 하나입니다.

라우팅:
- `write_decision_reasons: WriteDecisionReason[]`.

상태 영향:
- 커밋된 차단 결과의 상태 영향은 메서드 담당 문서만 정의할 수 있습니다.

결과 데이터:
- 메서드 담당 판단 사유를 사용합니다.

허용되지 않는 것:
- `CloseReadinessBlocker`를 반환하지 않습니다.

<a id="blocked-close-task-result"></a>
### `CloseTaskResult(close_state=blocked)`

조건:
- 유효한 닫기 준비 상태 평가가 닫기 차단 사유를 반환합니다.

라우팅:
- `blockers: CloseReadinessBlocker[]`.

상태 영향:
- 커밋된 차단 결과의 상태 영향은 `close_task` 메서드 담당 문서만 정의할 수 있습니다.

결과 데이터:
- 닫기 차단 사유 매핑을 사용합니다.

허용되지 않는 것:
- `STATE_VERSION_CONFLICT`를 쓰면 안 됩니다.

<a id="blocked-read-only-observation"></a>
### 읽기 전용 관찰

조건:
- `StatusResult.close_blockers` 또는 `harness.close_task intent=check`가 차단 사유 관찰 데이터를 반환합니다.

라우팅:
- 읽기 전용 `CloseReadinessBlocker` 관찰 데이터.

허용되지 않는 것:
- 읽기 때문에 저장된 차단 사유나 상태 버전 증가가 생기지 않습니다.

차단 결과는 메서드가 동작별 차단 결과를 반환했을 수 있다는 뜻입니다. 공개 전송 또는 스키마 오류가 아닙니다. 커밋된 차단 결과와 상태 영향은 [API 메서드](methods.md)가 안내하는 관련 메서드 담당 문서와 [저장 효과](../storage-effects.md)가 허용해야 합니다.

## `dry_run` 동작

| `dry_run` 경우 | 세부 항목 |
|---|---|
| 유효한 읽기 전용 호출 | [유효한 읽기 전용 `dry_run=true`](#dry-run-valid-read-only) |
| 유효한 상태 영향 또는 스테이징 미리보기 | [유효한 `dry_run` 미리보기](#dry-run-valid-preview) |
| 미리보기의 예상 차단 사유 | [`dry_run` 미리보기의 예상 차단 사유](#dry-run-expected-blockers) |
| 커밋 전 실패 | [`dry_run=true`의 커밋 전 실패](#dry-run-pre-commit-failure) |

<a id="dry-run-valid-read-only"></a>
### 유효한 읽기 전용 `dry_run=true`

조건:
- 유효한 읽기 전용 호출이 `dry_run=true`를 설정합니다.

응답 경로:
- `base.dry_run=true`와 `base.effect_kind=read_only`를 담은 메서드별 결과입니다.

허용되지 않는 것:
- `dry_run=true`를 `ToolDryRunResponse`의 동의어로 보지 않습니다.

<a id="dry-run-valid-preview"></a>
### 유효한 `dry_run` 미리보기

조건:
- 유효한 상태 영향 동작이나 저장소 담당 스테이징 동작이 `dry_run=true`를 설정합니다.

응답 경로:
- `DryRunSummary`를 담은 `ToolDryRunResponse`입니다.

상태 영향:
- `dry_run` 미리보기는 커밋된 쓰기가 아닙니다.

<a id="dry-run-expected-blockers"></a>
### `dry_run` 미리보기의 예상 차단 사유

조건:
- 유효한 `dry_run` 미리보기에 예상 차단 사유가 있습니다.

응답 경로:
- `DryRunSummary.would_blockers: PlannedBlocker[]`.

허용되지 않는 것:
- 미리보기 차단 사유는 저장된 `CloseReadinessBlocker` 객체가 아닙니다.
- `PlannedBlocker.code`는 `STATE_VERSION_CONFLICT`가 될 수 없습니다.

<a id="dry-run-pre-commit-failure"></a>
### `dry_run=true`의 커밋 전 실패

조건:
- `dry_run=true` 요청에 커밋 전 실패가 있습니다.

응답 경로:
- `ToolRejectedResponse`.

허용되지 않는 것:
- 실패를 `dry_run` 미리보기 데이터로 표현하지 않습니다.
- 오래된 상태는 미리보기 전에 거부됩니다.

<a id="idempotency"></a>
<a id="state-conflict-behavior"></a>
## 상태 버전 충돌

| 충돌 경우 | 세부 항목 |
|---|---|
| 오래된 `expected_state_version` | [오래된 `expected_state_version`](#state-conflict-expected-state-version) |
| 오래된 `WriteAuthorization.basis_state_version` | [오래된 Write Authorization 근거 버전](#state-conflict-write-authorization-basis) |
| 멱등 요청 해시 충돌 | [멱등 요청 해시 충돌](#state-conflict-idempotency-hash) |

`STATE_VERSION_CONFLICT`의 기준 범위 의미는 하나뿐입니다. 프로젝트 전체의 커밋 전 최신성 또는 멱등성 충돌입니다.

<a id="state-conflict-expected-state-version"></a>
### 오래된 `expected_state_version`

조건:
- `ToolEnvelope.expected_state_version`이 `project_state.state_version`보다 오래되었습니다.

공개 코드:
- `STATE_VERSION_CONFLICT`

응답 경로:
- `ToolRejectedResponse.errors[]`

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

세부정보 지침:
- 가능하면 `state_clock: project_state.state_version`, `current_state_version`, `expected_state_version`, `project_id`, `task_id`를 포함합니다.

허용되지 않는 것:
- 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="state-conflict-write-authorization-basis"></a>
### 오래된 Write Authorization 근거 버전

조건:
- 소비 전 `WriteAuthorization.basis_state_version`이 오래된 상태입니다.

공개 코드:
- `STATE_VERSION_CONFLICT`

응답 경로:
- `ToolRejectedResponse.errors[]`

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.
- Write Authorization이 소비되지 않습니다.

세부정보 지침:
- 오래된 권한 근거와 현재 `project_state.state_version`을 식별합니다.

허용되지 않는 것:
- 이 값을 차단 사유 코드로 사용하지 않습니다.

<a id="state-conflict-idempotency-hash"></a>
### 멱등 요청 해시 충돌

조건:
- 같은 `idempotency_key`가 다른 요청 해시와 함께 재사용되었습니다.

공개 코드:
- `STATE_VERSION_CONFLICT`

응답 경로:
- `ToolRejectedResponse.errors[]`

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

세부정보 지침:
- 민감한 요청 본문을 노출하지 않고 `idempotency_key`와 요청 해시 불일치를 식별합니다.

허용되지 않는 것:
- 이 값을 차단 사유 코드로 사용하지 않습니다.
- 이 충돌을 `dry_run` 미리보기 데이터, `MethodResult.decision`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, `PlannedBlocker.code`로 표현하지 않습니다.

## 금지된 차단 사유 코드 규칙

| 금지된 사용 | 세부 항목 |
|---|---|
| 오래된 상태 공개 오류를 차단 사유 코드로 사용 | [오래된 상태 차단 사유 코드](#forbidden-stale-state-blocker-code) |
| 커밋 전 공개 오류를 차단 사유 배열로 복사 | [커밋 전 공개 오류 복사](#forbidden-pre-commit-public-error-copy) |
| 공개 `ErrorCode`를 담당 문서 허용 없이 재사용 | [공개 코드 재사용](#forbidden-public-code-reuse) |
| 사용자 표시 라벨을 API 식별자로 사용 | [표시 라벨 식별자](#forbidden-user-facing-label-identifier) |
| `dry_run` 오래된 상태 충돌을 미리보기로 표현 | [`dry_run` 오래된 상태 미리보기](#forbidden-dry-run-stale-state-preview) |

<a id="forbidden-stale-state-blocker-code"></a>
### 오래된 상태 차단 사유 코드

허용되지 않는 것:
- `STATE_VERSION_CONFLICT`를 `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, `PlannedBlocker.code`, `MethodResult.decision`, 커밋된 차단 결과의 주 오류 코드로 사용하지 않습니다.

대신 사용할 것:
- `effect_kind=no_effect`인 `ToolRejectedResponse.errors[]`를 반환합니다.

<a id="forbidden-pre-commit-public-error-copy"></a>
### 커밋 전 공개 오류 복사

허용되지 않는 것:
- 커밋 전 공개 오류를 차단 사유 배열로 복사하지 않습니다.

대신 사용할 것:
- `ToolRejectedResponse.errors[]`를 반환합니다.

<a id="forbidden-public-code-reuse"></a>
### 공개 코드 재사용

허용되지 않는 것:
- 담당 문서의 명시적 허용 없이 공개 `ErrorCode`를 차단 사유 코드로 재사용하지 않습니다.

대신 사용할 것:
- 메서드/스키마 담당 문서의 차단 사유 코드나 결과 사유를 사용합니다.

<a id="forbidden-user-facing-label-identifier"></a>
### 표시 라벨 식별자

허용되지 않는 것:
- 사용자 표시 라벨을 API 식별자로 사용하지 않습니다.

대신 사용할 것:
- 공개 `ErrorCode`는 그대로 두고 표시 문구만 지역화합니다.

<a id="forbidden-dry-run-stale-state-preview"></a>
### `dry_run` 오래된 상태 미리보기

허용되지 않는 것:
- `dry_run` 미리보기의 오래된 상태 충돌을 `DryRunSummary.would_errors[]`나 `DryRunSummary.would_blockers[]`로 표현하지 않습니다.

대신 사용할 것:
- `STATE_VERSION_CONFLICT`로 요청을 거부합니다.

<a id="harnessclose_task-close-blockers"></a>

## `close_task` 차단 사유 매핑

- 닫기 준비 상태 평가 전 사전 확인 실패:
  - [사전 확인 실패](#close-task-preflight-failure)
- 유효한 읽기인 `intent=check`:
  - [`intent=check`](#close-task-intent-check)
- 닫기 차단 사유를 찾은 `intent=complete`:
  - [차단된 `intent=complete`](#close-task-intent-complete-blocked)
- 닫기 차단 사유가 없는 `intent=complete`:
  - [닫힌 `intent=complete`](#close-task-intent-complete-closed)
- 유효하지 않은 `intent=cancel` 또는 `intent=supersede` 종료 전이:
  - [유효하지 않은 종료 전이](#close-task-invalid-terminal-transition)

<a id="close-task-preflight-failure"></a>
### 사전 확인 실패

조건:
- 닫기 준비 상태 평가 전에 오래된 상태, 오래된 `Write Authorization` 근거, 멱등성 충돌, 검증 실패, 로컬 접근 실패, 역량 실패, Core 상태 읽기 실패, 프로젝트/`Task` 식별 실패가 발생합니다.

응답 경로:
- `ToolRejectedResponse.errors[]`

공개 코드 규칙:
- `STATE_VERSION_CONFLICT`와 다른 커밋 전 오류는 거부 응답에 남습니다.

허용되지 않는 것:
- `CloseReadinessBlocker` 항목을 반환하지 않습니다.

<a id="close-task-intent-check"></a>
### `intent=check`

조건:
- 요청이 유효한 읽기입니다.

응답 경로:
- 읽기 전용 `CloseTaskResult`

허용되는 것:
- `CloseReadinessBlocker` 관찰 데이터를 반환할 수 있습니다.

상태 영향:
- 저장된 차단 사유와 상태 버전 증가가 없습니다.

<a id="close-task-intent-complete-blocked"></a>
### 차단된 `intent=complete`

조건:
- 유효한 평가에서 닫기 차단 사유를 찾습니다.

응답 경로:
- `CloseTaskResult(close_state=blocked)`

허용되는 것:
- `CloseReadinessBlocker[]`를 반환할 수 있습니다.

허용되지 않는 것:
- `STATE_VERSION_CONFLICT`를 사용하지 않습니다.

<a id="close-task-intent-complete-closed"></a>
### 닫힌 `intent=complete`

조건:
- 담당 문서가 정의한 닫기 차단 사유가 더 없습니다.

응답 경로:
- `CloseTaskResult(close_state=closed)`

공개 코드 규칙:
- 닫기 차단 사유가 없습니다.

<a id="close-task-invalid-terminal-transition"></a>
### 유효하지 않은 종료 전이

조건:
- `intent=cancel` 또는 `intent=supersede`의 종료 전이가 유효하지 않습니다.

응답 경로:
- 메서드 담당 결과 또는 거부 경로

공개 코드 규칙:
- 차단 사유는 전이 유효성으로 제한합니다.

허용되지 않는 것:
- 취소나 대체에 증거 충분성, 최종 수락, 잔여 위험 수락을 요구하지 않습니다.

### 닫기 준비 상태 발견 사항 코드 요약

이 표는 닫기 준비 상태 발견 사항에 대응하는 공개 오류 코드 묶음을 요약합니다. 공개 `ErrorCode` 값을 차단 사유 코드로 바꾸는 규칙이 아닙니다.

| 닫기 준비 상태 발견 사항 | 세부 항목 |
|---|---|
| 증거 공백 | [증거 공백](#close-mapping-evidence-gap) |
| 지속 아티팩트 문제 | [지속 아티팩트 문제](#close-mapping-artifact-issue) |
| 최종 수락 문제 | [최종 수락 문제](#close-mapping-final-acceptance) |
| 잔여 위험이 보이지 않음 | [잔여 위험이 보이지 않음](#close-mapping-residual-risk-not-visible) |
| 수락되지 않은 잔여 위험 | [수락되지 않은 잔여 위험](#close-mapping-unaccepted-residual-risk) |
| 미해결 사용자 소유 판단 | [해결되지 않은 사용자 소유 판단](#close-mapping-unresolved-user-judgment) |
| 민감 동작 승인 문제 | [민감 동작 승인 문제](#close-mapping-sensitive-approval) |
| 범위, 경계, 기준 상태 | [범위, 경계, 기준 상태 차단 사유](#close-mapping-scope-boundary-baseline) |
| 읽기용 보기 최신성 | [읽기용 보기 최신성 문제](#close-mapping-readable-view-freshness) |
| 오래된 상태 거부 | [오래된 상태는 거부](#close-mapping-stale-state-rejected) |

<a id="close-mapping-evidence-gap"></a>
### 증거 공백

조건:
- 닫기 준비 상태 평가에서 증거 공백을 찾습니다.

공개 코드 매핑:
- `EVIDENCE_INSUFFICIENT`

<a id="close-mapping-artifact-issue"></a>
### 지속 아티팩트 문제

조건:
- 닫기에 영향을 주는 지속 아티팩트가 없거나, 사용할 수 없거나, 닫기 근거로 쓸 수 없거나, 실패했습니다.

공개 코드 매핑:
- `ARTIFACT_MISSING`

<a id="close-mapping-final-acceptance"></a>
### 최종 수락 문제

조건:
- 필요한 최종 수락이 없거나 호환되지 않습니다.

공개 코드 매핑:
- `ACCEPTANCE_REQUIRED`

<a id="close-mapping-residual-risk-not-visible"></a>
### 잔여 위험이 보이지 않음

조건:
- 닫기에 영향을 주는 알려진 잔여 위험이 보이지 않습니다.

공개 코드 매핑:
- `RESIDUAL_RISK_NOT_VISIBLE`

<a id="close-mapping-unaccepted-residual-risk"></a>
### 수락되지 않은 잔여 위험

조건:
- 잔여 위험은 보였지만 수락되지 않았습니다.

공개 코드 매핑:
- `category=residual_risk_acceptance`와 함께 `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED`

<a id="close-mapping-unresolved-user-judgment"></a>
### 해결되지 않은 사용자 소유 판단

조건:
- 사용자 소유 판단이 해결되지 않았습니다.

공개 코드 매핑:
- `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED`

<a id="close-mapping-sensitive-approval"></a>
### 민감 동작 승인 문제

조건:
- 민감 동작 승인이 없거나, 거부되었거나, 만료되었거나, 달라졌습니다.

공개 코드 매핑:
- `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`

<a id="close-mapping-scope-boundary-baseline"></a>
### 범위, 경계, 기준 상태 차단 사유

조건:
- 유효한 평가에서 범위, 자율성 경계, 기준 상태 차단 사유를 찾습니다.

공개 코드 매핑:
- `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE`

허용되지 않는 것:
- 담당 문서가 허용하지 않으면 이 매핑을 사용하지 않습니다.

<a id="close-mapping-readable-view-freshness"></a>
### 읽기용 보기 최신성 문제

조건:
- 읽기용 보기 최신성 문제가 있습니다.

공개 코드 매핑:
- `PROJECTION_STALE`

허용되지 않는 것:
- `PROJECTION_STALE`만으로 닫기 차단 사유를 만들지 않습니다.

<a id="close-mapping-stale-state-rejected"></a>
### 오래된 상태는 거부

조건:
- 프로젝트 전체 상태나 `WriteAuthorization.basis_state_version`이 오래된 상태입니다.

응답 경로:
- `STATE_VERSION_CONFLICT`를 담은 `ToolRejectedResponse.errors[]`

허용되지 않는 것:
- 이 값을 닫기 차단 사유로 사용하지 않습니다.

담당 문서:
- 닫기 준비 상태 의미와 대체 금지 규칙: [Core 모델의 닫기 준비 상태](../core-model.md#close_task)
- 메서드 동작과 닫기 준비 상태 평가 순서: [`harness.close_task`](method-close-task.md)
- `CloseReadinessBlocker` 형태와 범주: [API 상태 스키마](schema-state.md)와 [API 값 집합](schema-value-sets.md)

<a id="documentation-smoke-error-coverage"></a>

## 담당 문서 링크

- 공개 `ErrorCode` 값, 의미, 우선순위:
  - 이 문서입니다.
- 응답 분기 형태:
  - [API 코어 스키마](schema-core.md)
  - `ToolRejectedResponse`, `ToolDryRunResponse`, `ToolError`, `ToolResultBase`, `DryRunSummary`에 적용됩니다.
- 메서드 동작, 분기 선택, 메서드별 요청 본문:
  - [API 메서드](methods.md)가 안내하는 메서드 담당 문서
- 상태와 닫기 준비 상태 데이터 형태:
  - [API 상태 스키마](schema-state.md)
  - `WriteDecisionReason`, `CloseReadinessBlocker`, 상태 요약에 적용됩니다.
- enum 형태 API 값:
  - [API 값 집합](schema-value-sets.md)
  - `response_kind`, `effect_kind`, `PlannedBlocker.source_kind`, 차단 사유 범주에 적용됩니다.
- 아티팩트 입력과 참조 형태:
  - [API 아티팩트 스키마](schema-artifacts.md)
  - `ArtifactInput`, `ArtifactRef`, `StagedArtifactHandle`에 적용됩니다.
- 스테이징된 아티팩트 핸들 저장소 검증과 아티팩트 승격 생명주기:
  - [아티팩트 저장소](../storage-artifacts.md)
- 사용자 판단, 승인, 수락, 잔여 위험 수락 형태:
  - [API 판단 스키마](schema-judgment.md)
  - [Core 모델](../core-model.md)
- 닫기 준비 상태 의미와 대체 금지 규칙:
  - [Core 모델의 닫기 준비 상태](../core-model.md#close_task)
- 닫기 준비 상태 메서드 동작과 평가 순서:
  - [`harness.close_task`](method-close-task.md)
- 저장 효과, 재실행 행, 상태 시계, DDL:
  - [저장 효과](../storage-effects.md)
  - [저장소 버전 관리](../storage-versioning.md)
  - [저장소 기록](../storage-records.md)
- 보안 보장 표현과 접근 경계 주장:
  - [보안](../security.md)
- 사용자 표시 라벨, 렌더링 오류 메시지 문구, 템플릿 표현:
  - [템플릿 본문](../template-bodies.md)
