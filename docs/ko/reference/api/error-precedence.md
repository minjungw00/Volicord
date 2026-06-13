# API 오류 우선순위

이 문서는 공개 오류 후보가 둘 이상 있을 때 주 공개 오류를 선택하는 규칙을 담당합니다. `STATE_VERSION_CONFLICT`의 공개 오래된 상태와 멱등성 충돌 동작도 담당합니다.

공개 `ErrorCode` 값 집합, 응답 분기 경로, 차단 사유 처리 경로, `harness.close_task` 메서드 동작, 기계 판독용 세부 필드, 응답 분기 형태, 저장소 재실행 행, 렌더링 라벨은 정의하지 않습니다.

## 담당 경계

이 문서가 담당합니다.

- 오류를 담는 분기의 주 `errors[0]` 선택 순서.
- `STATE_VERSION_CONFLICT`를 결과 코드와 차단 사유 코드 경로에서 제외하는 규칙.
- 오래된 공개 `expected_state_version`, 오래된 `WriteAuthorization.basis_state_version`, 멱등 요청 해시 충돌 동작.

이 문서는 담당하지 않습니다.

- 우선순위 선택 밖의 공개 오류 코드 의미: [API 오류 코드](error-codes.md).
- API 응답 분기 경로: [API 오류 처리 경로](error-routing.md).
- 닫기 차단 사유와 API 응답 사이의 경계: [API 차단 사유 처리 경로](blocker-routing.md).
- `harness.close_task` 메서드별 차단 동작: [`harness.close_task`](method-close-task.md).
- 기계 판독용 충돌 세부 필드: [API 오류 세부사항](error-details.md#state-conflict-detail-fields).
- 저장소 재실행 행과 상태 시계: [저장소 버전 관리](../storage-versioning.md).

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
- 현재 적용 Change Unit이 없는 경우입니다.

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
- 더 구체적인 지원 코드가 없을 때 쓰는 타입 있는 대체 코드입니다.

<a id="state-version-conflict-precedence-exclusion"></a>
### `STATE_VERSION_CONFLICT` 우선순위 제외

사용 위치:
- `ToolRejectedResponse.errors[]`

조건:
- 오래된 `expected_state_version` 때문에 메서드가 진행될 수 없어 거부 응답이 선택됩니다.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

선택 경계:
- `STATE_VERSION_CONFLICT`를 `MethodResult.base.errors[0]`, `CloseTaskResult(close_state=blocked).errors[0]`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, `PlannedBlocker.code`로 선택하지 않습니다.

관련 충돌 세부사항:
- 오래된 `WriteAuthorization.basis_state_version`과 멱등 요청 해시 충돌은 [상태 버전 충돌](#state-conflict-behavior)에서 다룹니다.

<a id="idempotency"></a>
<a id="state-conflict-behavior"></a>
## 상태 버전 충돌

| 충돌 경우 | 세부 항목 |
|---|---|
| 오래된 `expected_state_version` | [오래된 `expected_state_version`](#state-conflict-expected-state-version) |
| 오래된 `WriteAuthorization.basis_state_version` | [오래된 Write Authorization 근거 버전](#state-conflict-write-authorization-basis) |
| 멱등 요청 해시 충돌 | [멱등 요청 해시 충돌](#state-conflict-idempotency-hash) |

`STATE_VERSION_CONFLICT`의 기준 범위 의미는 하나뿐입니다. 프로젝트 전체의 커밋 전 최신성 또는 멱등성 충돌입니다.

충돌 처리 경계:

| 경계 | 이 문서의 규칙 | 이웃 담당 문서 |
|---|---|---|
| 공개 오류 코드 의미 | 아래 충돌 경우에는 `STATE_VERSION_CONFLICT`를 선택합니다. | 공개 오류 코드 의미: [API 오류 코드](error-codes.md). |
| 응답 경로 | 이 충돌은 `ToolRejectedResponse.errors[]`를 사용합니다. | 응답 분기 경로: [API 오류 처리 경로](error-routing.md). |
| 결과, 차단 사유, 닫기 준비 상태 경계 경로 | `STATE_VERSION_CONFLICT`를 차단 사유 코드, `dry_run` 미리보기, `MethodResult.decision`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, `PlannedBlocker.code`로 사용하지 않습니다. | 경계 처리: [API 차단 사유 처리 경로](blocker-routing.md). 메서드 동작: [`harness.close_task`](method-close-task.md). |
| 세부 필드 | 이 충돌에는 상태 충돌 세부 필드 묶음을 사용합니다. | 기계 판독용 필드: [API 오류 세부사항](error-details.md#state-conflict-detail-fields). |

<a id="state-conflict-expected-state-version"></a>
### 오래된 `expected_state_version`

조건:
- `ToolEnvelope.expected_state_version`이 `project_state.state_version`보다 오래되었습니다.

공개 오류 코드:
- `STATE_VERSION_CONFLICT`

응답 경로:
- `ToolRejectedResponse.errors[]`

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

세부 필드:
- [상태 충돌 세부 필드](error-details.md#state-conflict-detail-fields)를 사용합니다.

<a id="state-conflict-write-authorization-basis"></a>
### 오래된 Write Authorization 근거 버전

조건:
- 소비 전 `WriteAuthorization.basis_state_version`이 오래된 상태입니다.

공개 오류 코드:
- `STATE_VERSION_CONFLICT`

응답 경로:
- `ToolRejectedResponse.errors[]`

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.
- Write Authorization이 소비되지 않습니다.

세부 필드:
- [상태 충돌 세부 필드](error-details.md#state-conflict-detail-fields)를 사용합니다.

<a id="state-conflict-idempotency-hash"></a>
### 멱등 요청 해시 충돌

조건:
- 같은 `idempotency_key`가 다른 요청 해시와 함께 재사용되었습니다.

공개 오류 코드:
- `STATE_VERSION_CONFLICT`

응답 경로:
- `ToolRejectedResponse.errors[]`

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

세부 필드:
- [상태 충돌 세부 필드](error-details.md#state-conflict-detail-fields)를 사용합니다.
