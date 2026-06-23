# API 오류 우선순위

이 문서는 공개 오류 후보가 둘 이상 있을 때 주 공개 오류를 선택하는 규칙을 담당합니다. `STATE_VERSION_CONFLICT`의 공개 오래된 상태와 멱등성 충돌 동작도 담당합니다.

오류를 담는 분기의 주 공개 오류 코드를 고를 때 이 문서를 사용합니다. 코드 의미, 분기 경로, 스키마, 저장소, 표시 문구는 이웃 담당 문서를 사용합니다.

## 담당 경계

이 문서가 담당합니다.

- 오류를 담는 분기의 주 `errors[0]` 선택 순서.
- `STATE_VERSION_CONFLICT`의 결과 쪽 및 차단 사유 코드 경로 경계.
- 오래된 공개 `expected_state_version`, 오래된 `WriteAuthorization.basis_state_version`, 멱등 요청 해시 충돌 동작.

이웃 담당 문서:

- 우선순위 선택 밖의 공개 오류 코드 의미: [API 오류 코드](error-codes.md).
- API 응답 분기 경로: [API 오류 처리 경로](error-routing.md).
- 닫기 차단 사유와 API 응답 사이의 경계: [API 차단 사유 처리 경로](blocker-routing.md).
- 메서드별 동작: [`volicord.close_task`](method-close-task.md)와 다른 메서드 담당 문서.
- 기계 판독용 충돌 세부 필드: [API 오류 세부사항](error-details.md#state-conflict-detail-fields).
- 저장소 재실행 행과 상태 시계: [저장소 버전 관리](../storage-versioning.md).
- 표시 문구만: [템플릿 본문](../template-bodies.md).

<a id="primary-error-code-precedence"></a>

## 오류 우선순위

오류를 담는 분기의 `errors`가 비어 있지 않으면 메서드 담당 문서가 더 좁은 메서드별 순서를 정의하지 않는 한 아래 순서로 `errors[0]` 공개 주 오류를 고릅니다. 이 표는 순서만 정의합니다. 공개 오류 코드 의미는 [API 오류 코드](error-codes.md)에 남습니다.

| 우선순위 | 주 `ErrorCode` | 의미 담당 문서 |
|---:|---|---|
| <a id="precedence-validation-failed"></a>1 | `VALIDATION_FAILED` | [`VALIDATION_FAILED`](error-codes.md#errorcode-validation-failed) |
| 2 | `STATE_VERSION_CONFLICT` | [`STATE_VERSION_CONFLICT`](error-codes.md#errorcode-state-version-conflict) |
| <a id="precedence-mcp-unavailable"></a>3 | `MCP_UNAVAILABLE` | [`MCP_UNAVAILABLE`](error-codes.md#errorcode-mcp-unavailable) |
| <a id="precedence-local-access-mismatch"></a>4 | `LOCAL_ACCESS_MISMATCH` | [`LOCAL_ACCESS_MISMATCH`](error-codes.md#errorcode-local-access-mismatch) |
| <a id="precedence-no-active-task"></a>5 | `NO_ACTIVE_TASK` | [`NO_ACTIVE_TASK`](error-codes.md#errorcode-no-active-task) |
| <a id="precedence-no-active-change-unit"></a>6 | `NO_ACTIVE_CHANGE_UNIT` | [`NO_ACTIVE_CHANGE_UNIT`](error-codes.md#errorcode-no-active-change-unit) |
| <a id="precedence-baseline-stale"></a>7 | `BASELINE_STALE` | [`BASELINE_STALE`](error-codes.md#errorcode-baseline-stale) |
| <a id="precedence-scope-required"></a>8 | `SCOPE_REQUIRED` | [`SCOPE_REQUIRED`](error-codes.md#errorcode-scope-required) |
| <a id="precedence-scope-violation"></a>9 | `SCOPE_VIOLATION` | [`SCOPE_VIOLATION`](error-codes.md#errorcode-scope-violation) |
| <a id="precedence-write-authorization-required"></a>10 | `WRITE_AUTHORIZATION_REQUIRED` | [`WRITE_AUTHORIZATION_REQUIRED`](error-codes.md#errorcode-write-authorization-required) |
| <a id="precedence-write-authorization-invalid"></a>11 | `WRITE_AUTHORIZATION_INVALID` | [`WRITE_AUTHORIZATION_INVALID`](error-codes.md#errorcode-write-authorization-invalid) |
| <a id="precedence-approval-denied"></a>12 | `APPROVAL_DENIED` | [`APPROVAL_DENIED`](error-codes.md#errorcode-approval-denied) |
| <a id="precedence-approval-expired"></a>13 | `APPROVAL_EXPIRED` | [`APPROVAL_EXPIRED`](error-codes.md#errorcode-approval-expired) |
| <a id="precedence-approval-required"></a>14 | `APPROVAL_REQUIRED` | [`APPROVAL_REQUIRED`](error-codes.md#errorcode-approval-required) |
| <a id="precedence-decision-unresolved"></a>15 | `DECISION_UNRESOLVED` | [`DECISION_UNRESOLVED`](error-codes.md#errorcode-decision-unresolved) |
| <a id="precedence-autonomy-boundary-exceeded"></a>16 | `AUTONOMY_BOUNDARY_EXCEEDED` | [`AUTONOMY_BOUNDARY_EXCEEDED`](error-codes.md#errorcode-autonomy-boundary-exceeded) |
| <a id="precedence-decision-required"></a>17 | `DECISION_REQUIRED` | [`DECISION_REQUIRED`](error-codes.md#errorcode-decision-required) |
| <a id="precedence-capability-insufficient"></a>18 | `CAPABILITY_INSUFFICIENT` | [`CAPABILITY_INSUFFICIENT`](error-codes.md#errorcode-capability-insufficient) |
| <a id="precedence-evidence-insufficient"></a>19 | `EVIDENCE_INSUFFICIENT` | [`EVIDENCE_INSUFFICIENT`](error-codes.md#errorcode-evidence-insufficient) |
| <a id="precedence-residual-risk-not-visible"></a>20 | `RESIDUAL_RISK_NOT_VISIBLE` | [`RESIDUAL_RISK_NOT_VISIBLE`](error-codes.md#errorcode-residual-risk-not-visible) |
| <a id="precedence-acceptance-required"></a>21 | `ACCEPTANCE_REQUIRED` | [`ACCEPTANCE_REQUIRED`](error-codes.md#errorcode-acceptance-required) |
| <a id="precedence-projection-stale"></a>22 | `PROJECTION_STALE` | [`PROJECTION_STALE`](error-codes.md#errorcode-projection-stale) |
| <a id="precedence-artifact-missing"></a>23 | `ARTIFACT_MISSING` | [`ARTIFACT_MISSING`](error-codes.md#errorcode-artifact-missing) |
| <a id="precedence-validator-failed"></a>24 | `VALIDATOR_FAILED` | [`VALIDATOR_FAILED`](error-codes.md#errorcode-validator-failed) |

<a id="state-version-conflict-precedence-exclusion"></a>
### `STATE_VERSION_CONFLICT` 선택 경계

선택 조건:
- 오래된 `expected_state_version`, 오래된 `WriteAuthorization.basis_state_version`, 멱등 요청 해시 충돌 때문에 메서드가 진행될 수 없으면 거부 응답에서 `STATE_VERSION_CONFLICT`가 선택됩니다.

선택 경계:
- 이 충돌은 `ToolRejectedResponse.errors[]`로 표현하며, `MethodResult`나 `CloseTaskResult(close_state=blocked)` 분기를 만들지 않습니다. `STATE_VERSION_CONFLICT`를 결과 쪽 판단, 차단 사유 코드, 닫기 차단 사유 코드, 미리보기 차단 사유 코드로 모델링하지 않으며, 여기에는 `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, `PlannedBlocker.code`가 포함됩니다.

관련 담당 문서:
- 이 충돌의 기계 판독용 필드는 [API 오류 세부사항](error-details.md#state-conflict-detail-fields)이 담당합니다.

<a id="idempotency"></a>
<a id="state-conflict-behavior"></a>
## 상태 버전 충돌

| 충돌 경우 | 세부 항목 |
|---|---|
| 오래된 `expected_state_version` | [오래된 `expected_state_version`](#state-conflict-expected-state-version) |
| 오래된 `WriteAuthorization.basis_state_version` | [오래된 `Write Authorization` 근거 버전](#state-conflict-write-authorization-basis) |
| 멱등 요청 해시 충돌 | [멱등 요청 해시 충돌](#state-conflict-idempotency-hash) |

우선순위에서 아래 충돌 경우는 프로젝트 전체의 커밋 전 최신성 또는 멱등성 충돌로 `STATE_VERSION_CONFLICT`를 선택합니다.

충돌 처리 경계:

| 경계 | 이 문서의 규칙 | 이웃 담당 문서 |
|---|---|---|
| 충돌 선택 | 아래 충돌 경우에는 `STATE_VERSION_CONFLICT`를 선택합니다. | 공개 오류 코드 의미: [API 오류 코드](error-codes.md). |
| 응답 경로 | 이 충돌은 `ToolRejectedResponse.errors[]`를 사용합니다. | 응답 분기 경로: [API 오류 처리 경로](error-routing.md). |
| 결과, 차단 사유, 닫기 준비 상태 경계 경로 | `STATE_VERSION_CONFLICT`를 차단 사유 코드, `dry_run` 미리보기, `MethodResult.decision`, `WriteDecisionReason.code`, `CloseReadinessBlocker.code`, `PlannedBlocker.code`로 사용하지 않습니다. | 경계 처리: [API 차단 사유 처리 경로](blocker-routing.md). 메서드 동작: [`volicord.close_task`](method-close-task.md). |
| 세부 필드 | 이 충돌에는 상태 충돌 세부 필드 묶음을 사용합니다. | 기계 판독용 필드: [API 오류 세부사항](error-details.md#state-conflict-detail-fields). |

<a id="state-conflict-expected-state-version"></a>
### 오래된 `expected_state_version`

조건:
- `ToolEnvelope.expected_state_version`이 `project_state.state_version`보다 오래되었습니다.

공개 오류 코드:
- `STATE_VERSION_CONFLICT`

응답 경로:
- `ToolRejectedResponse.errors[]`

세부 필드:
- [상태 충돌 세부 필드](error-details.md#state-conflict-detail-fields)를 사용합니다.

<a id="state-conflict-write-authorization-basis"></a>
### 오래된 `Write Authorization` 근거 버전

조건:
- 소비 전에 `WriteAuthorization.basis_state_version`이 현재 `project_state.state_version`과 같지 않습니다.

공개 오류 코드:
- `STATE_VERSION_CONFLICT`

응답 경로:
- `ToolRejectedResponse.errors[]`

소비 경계:
- 오래된 `Write Authorization`은 소비되지 않습니다.
- 거절된 시도는 소비 쪽 상태 변경을 만들지 않습니다.

세부 필드:
- [상태 충돌 세부 필드](error-details.md#state-conflict-detail-fields)를 사용합니다.

### 만료된 `Write Authorization`

조건:
- 소비 전에 권한이 [`volicord.record_run`](method-record-run.md)과 [`volicord.prepare_write`](method-prepare-write.md)가 담당하는 유효 만료 규칙에 따라 만료되었고, `WriteAuthorization.basis_state_version`은 오래되지 않았습니다.

공개 오류 코드:
- `WRITE_AUTHORIZATION_INVALID`

응답 경로:
- `ToolRejectedResponse.errors[]`

우선순위 경계:
- `WriteAuthorization.basis_state_version`이 오래되었으면 만료 무효가 아니라 `STATE_VERSION_CONFLICT`를 선택합니다.
- 만료는 결과 쪽 판단, 차단 사유 코드, 닫기 준비 상태 차단 사유 코드, 미리보기 차단 사유 코드로 모델링하지 않습니다.

세부 필드:
- `ToolError.details.authorization_reason=expired`를 사용합니다.

<a id="state-conflict-idempotency-hash"></a>
### 멱등 요청 해시 충돌

조건:
- 같은 `idempotency_key`가 다른 요청 해시와 함께 재사용되었습니다.

공개 오류 코드:
- `STATE_VERSION_CONFLICT`

응답 경로:
- `ToolRejectedResponse.errors[]`

세부 필드:
- [상태 충돌 세부 필드](error-details.md#state-conflict-detail-fields)를 사용합니다.
