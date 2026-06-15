# API 오류 코드

이 문서는 하네스 API 응답의 공개 `ErrorCode` 식별자, 의미, 발생 위치 요약을 담당합니다.

공개 코드가 무엇을 뜻하고 어디에 나타날 수 있는지 확인할 때 이 문서를 사용합니다. 선택 순서, 분기 경로, 스키마, 저장 효과, 보안 보장, 표시 문구는 이웃 담당 문서를 사용합니다.

## 담당 경계

이 문서가 담당합니다.

- 공개 `ErrorCode` 값 집합.
- 각 코드의 공개 의미와 허용되는 공개 발생 경로.
- 코드가 `ToolRejectedResponse.errors[]`나 담당 문서가 정의한 결과 경로에 나타날 수 있는지 여부.

이웃 담당 문서:

- 주 코드 선택과 상태 버전 충돌 동작: [API 오류 우선순위](error-precedence.md).
- 거부 응답, 차단 결과, `dry_run` 분기 경로: [API 오류 처리 경로](error-routing.md).
- 닫기 차단 사유와 API 응답 사이의 경계: [API 차단 사유 처리 경로](blocker-routing.md).
- 메서드별 동작: [`harness.close_task`](method-close-task.md)와 다른 메서드 담당 문서.
- `ToolError.details` 필드와 보조 값: [API 오류 세부사항](error-details.md).
- 공통 응답 분기 형태: [API 코어 스키마](schema-core.md).
- 표시 문구만: [템플릿 본문](../template-bodies.md).
- 저장 효과: [저장 효과](../storage-effects.md).

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

## 발생 경로 요약

| 발생 경로 | 규칙 |
|---|---|
| 거부 응답 오류 | 거부된 공개 API 요청에서는 공개 `ErrorCode` 값이 `ToolRejectedResponse.errors[]`에 나타날 수 있습니다. 분기 경로는 [API 오류 처리 경로](error-routing.md)가 담당합니다. |
| 담당 문서가 정의한 결과 경로 | 메서드, 스키마, 닫기 준비 상태 담당 문서는 공개 오류 코드 묶음이 담당 문서가 정의한 결과 경로에 나타나는지 정할 수 있습니다. 그 결과 경로 사용은 이 문서가 담당하는 공개 의미를 바꾸지 않습니다. |
| 오류와 차단 사유의 경계 | 공개 API 오류와 `CloseReadinessBlocker` 데이터 사이의 담당 경계는 [API 차단 사유 처리 경로](blocker-routing.md)에서 다룹니다. |

<a id="errorcode-validation-failed"></a>
### `VALIDATION_FAILED`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 요청 본문 형태, enum 값, 적용 규칙, 프로필 검증, 아티팩트 입력 형태가 유효하지 않습니다.

<a id="errorcode-state-version-conflict"></a>
### `STATE_VERSION_CONFLICT`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 공개 최신성 또는 멱등성 충돌이 있습니다. 오래된 `expected_state_version`은 요청 상태 형태입니다.

참고:
- 오래된 `WriteAuthorization.basis_state_version`과 멱등 요청 해시 충돌은 [상태 버전 충돌](error-precedence.md#state-conflict-behavior)에서 다룹니다.

<a id="errorcode-mcp-unavailable"></a>
### `MCP_UNAVAILABLE`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 필요한 Core, MCP, 접점 도달 가능성을 사용할 수 없습니다.

<a id="errorcode-local-access-mismatch"></a>
### `LOCAL_ACCESS_MISMATCH`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 도달 가능한 로컬 접근이 등록된 전송 경로, 세션, 바인딩, 프로젝트, 접점 인스턴스와 맞지 않거나 접근이 철회되었습니다.

<a id="errorcode-no-active-task"></a>
### `NO_ACTIVE_TASK`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- `Task`가 필요하지만 현재 적용 `Task`나 지정된 `Task`가 없습니다.

<a id="errorcode-no-active-change-unit"></a>
### `NO_ACTIVE_CHANGE_UNIT`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 쓰기 가능하거나 닫기와 관련된 동작에 범위가 지정된 현재 적용 Change Unit이 없습니다.

<a id="errorcode-baseline-stale"></a>
### `BASELINE_STALE`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 동작에 필요한 저장소 상태와 기준 상태가 더 이상 맞지 않습니다.

<a id="errorcode-scope-required"></a>
### `SCOPE_REQUIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 요청한 쓰기나 동작 전에 범위 확인이 필요합니다.

<a id="errorcode-scope-violation"></a>
### `SCOPE_VIOLATION`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 의도했거나 관찰된 경로 또는 민감 범주가 현재 적용 범위나 저장된 승인 범위를 넘었습니다.

<a id="errorcode-write-authorization-required"></a>
### `WRITE_AUTHORIZATION_REQUIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 쓰기 가능한 실행 기록에 필요한 `Write Authorization`이 없습니다.

<a id="errorcode-write-authorization-invalid"></a>
### `WRITE_AUTHORIZATION_INVALID`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 제공된 `Write Authorization`이 만료, 철회, 소비, 또는 버전 외 사유로 비호환입니다.

<a id="errorcode-approval-denied"></a>
### `APPROVAL_DENIED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 필요한 민감 동작 승인이 거부되었습니다.

<a id="errorcode-approval-expired"></a>
### `APPROVAL_EXPIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 필요한 민감 동작 승인이 만료되었거나 범위 또는 기준 상태와 달라졌습니다.

<a id="errorcode-approval-required"></a>
### `APPROVAL_REQUIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 진행 전에 민감 동작 승인이 필요합니다.

<a id="errorcode-decision-unresolved"></a>
### `DECISION_UNRESOLVED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 관련 사용자 소유 판단이 대기, 적용 범위 없는 보류, 거부, 차단, 오래됨, 대체됨, 비호환 상태입니다.

<a id="errorcode-autonomy-boundary-exceeded"></a>
### `AUTONOMY_BOUNDARY_EXCEEDED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 의도한 동작이 현재 적용 Change Unit의 자율성 경계를 넘었습니다.

<a id="errorcode-decision-required"></a>
### `DECISION_REQUIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 진행 전에 차단 중인 사용자 소유 판단이 필요합니다.

<a id="errorcode-capability-insufficient"></a>
### `CAPABILITY_INSUFFICIENT`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 접점은 인식되었지만 필요한 접근 등급, 관찰, 캡처, 보장 지원, 지원 동작이 없습니다.

<a id="errorcode-evidence-insufficient"></a>
### `EVIDENCE_INSUFFICIENT`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 필요한 증거 범위가 없거나, 부분적이거나, 오래되었거나, 막혔습니다.

<a id="errorcode-residual-risk-not-visible"></a>
### `RESIDUAL_RISK_NOT_VISIBLE`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 닫기에 영향을 주는 알려진 잔여 위험이 최종 수락이나 닫기 전에 보이지 않았습니다.

<a id="errorcode-acceptance-required"></a>
### `ACCEPTANCE_REQUIRED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 필요한 최종 수락이 대기 중이거나, 거부되었거나, 표시된 결과 근거와 호환되지 않습니다.

<a id="errorcode-projection-stale"></a>
### `PROJECTION_STALE`

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 요청한 읽기용 상태나 보기가 오래되었거나 실패했습니다.

<a id="errorcode-artifact-missing"></a>
### `ARTIFACT_MISSING`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 참조한 지속 아티팩트가 없거나, 사용할 수 없거나, 닫기 근거로 쓸 수 없거나, 무결성/메타데이터 확인에 실패했습니다.

<a id="errorcode-validator-failed"></a>
### `VALIDATOR_FAILED`

사용 위치:
- `ToolRejectedResponse.errors[]`
- 담당 문서가 정의한 결과 경로

조건:
- 필요한 검증기나 차단 사유 확인이 실패했고 더 구체적인 타입 코드가 없을 때 쓰는 대체 코드입니다.
