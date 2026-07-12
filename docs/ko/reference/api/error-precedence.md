# API 오류 우선순위

이 문서는 공개 오류 후보가 둘 이상일 때 주 공개 오류를 선택하는 규칙을 담당합니다.
오류와 차단 사유의 정식 결정 흐름도 정의합니다. 또한
`STATE_VERSION_CONFLICT`가 다루는 공개 상태 최신성 문제와 멱등성 충돌 동작을
담당합니다.

오류를 담는 분기의 주 공개 오류 코드를 고를 때 이 문서를 사용합니다. 코드 의미, 분기 경로, 스키마, 저장소, 표시 문구는 이웃 담당 문서를 사용합니다.

## 담당 경계

이 문서가 담당합니다.

- 전송 또는 어댑터 실패, Core 거부 응답, `dry_run` 미리보기, 메서드가 담당하는 차단 결과, 커밋된 차단 사유형 결과를 구분하는 정식 결정 흐름.
- 오류를 담는 분기의 주 `errors[0]` 선택 순서.
- `STATE_VERSION_CONFLICT`의 결과 쪽 및 차단 사유 코드 경로 경계.
- 공개 요청의 오래된 `expected_state_version`, 오래된 `WriteTicket.basis_state_version`, 멱등 요청 해시 충돌 동작.

이웃 담당 문서:

- MCP JSON-RPC 오류, `CallToolResult.isError`, `tools/call` 래핑: [MCP 전송](../mcp-transport.md).
- Agent Connection 프로젝트 선택, 모드, 현재 연결 맥락: [Agent Connection](../agent-connection.md).
- 우선순위 선택 밖의 공개 오류 코드 의미: [API 오류 코드](error-codes.md).
- API 응답 분기 경로: [API 오류 처리 경로](error-routing.md).
- 닫기 차단 사유와 API 응답 사이의 경계: [API 차단 사유 처리 경로](blocker-routing.md).
- 메서드별 동작: [`volicord.close_task`](method-close-task.md)와 다른 메서드 담당 문서.
- 기계 판독용 충돌 세부 필드: [API 오류 세부사항](error-details.md#state-conflict-detail-fields).
- 커밋된 결과의 저장 효과: [저장 효과](../storage-effects.md).
- 저장소 재실행 행과 상태 시계: [저장소 버전 관리](../storage-versioning.md).
- 표시 문구만: [템플릿 본문](../template-bodies.md).

<a id="canonical-error-blocker-decision-flow"></a>

## 오류와 차단 사유의 정식 결정 흐름

[주 오류 코드 우선순위](#primary-error-code-precedence)를 적용하기 전에 이 흐름을 사용합니다. 이 흐름은 먼저 응답 계열을 고릅니다. 아래 우선순위 표는 호출이 `ToolRejectedResponse.errors[]`를 가진 Volicord 거부 응답이 된 뒤에만 적용됩니다.

| 순서 | 경계 | 실행 지점 | 공개 형태 | 처리 규칙 |
|---:|---|---|---|---|
| 1 | 공개 Volicord 요청이 생기기 전에 전송, JSON-RPC, 또는 어댑터 메시지 형태가 실패합니다. | Core 실행 전 전송 또는 어댑터 계층. | JSON-RPC `error`, 프로세스 종료 진단, 또는 전송 담당 실패 형태입니다. Volicord `ErrorCode`는 선택하지 않습니다. | [MCP 전송](../mcp-transport.md)이나 해당 전송 또는 어댑터 담당 문서로 보냅니다. 요청이 Core 전에 실패한 뒤 이를 `ToolRejectedResponse.errors[]`로 바꾸지 않습니다. |
| 2 | 알려진 MCP 도구 호출이 Core 실행 전에 MCP 어댑터에서 거절됩니다. 여기에는 Agent Connection 모드 거절, 프로젝트 선택 실패, 프로젝트 허용 목록 실패, 호출자 소유 호출 필드, 알려진 도구 인자 디코딩/스키마 거절이 포함됩니다. | Core 실행 전 MCP 어댑터 계층. | 그 조건이 [MCP 전송](../mcp-transport.md)이 담당하는 JSON-RPC 프로토콜 또는 파라미터 오류가 아닌 한, 텍스트 콘텐츠와 `isError: true`를 담은 MCP `CallToolResult`입니다. | Volicord 메서드 결과가 아니며 `ToolRejectedResponse.errors[]`가 없습니다. 다시 시도하기 전에 MCP 호출, 연결 모드, 프로젝트 선택자를 고칩니다. |
| 3 | 타입이 정해진 Volicord 요청이 Core에 도달했지만 메서드가 담당하는 결과 분기 전에 요청 검증, Core 사전 확인, 호출 호환성, `Task` 조회, 최신성, 멱등성이 실패합니다. | Core 안, 커밋되는 메서드 실행 전. | 공개 `ErrorCode` 값을 담은 `ToolRejectedResponse.errors[]`입니다. | 거부 분기는 [API 오류 처리 경로](error-routing.md)를 사용하고, `errors[0]`에는 이 문서의 우선순위 표를 사용합니다. 커밋되는 동작은 진행되지 않습니다. |
| 4 | 유효한 `dry_run` 요청이 사전 확인 뒤 미리보기 분기에 도달합니다. | Core 안, 검증과 사전 확인 뒤, 커밋 전. | 메서드가 정의한 경우 `DryRunSummary.would_errors[]` 또는 `DryRunSummary.would_blockers[]`를 담은 `ToolDryRunResponse`입니다. | 미리보기 분기 동작은 [API 오류 처리 경로](error-routing.md#dry-run-behavior)로 보냅니다. 미리보기 차단 사유는 저장된 `CloseReadinessBlocker` 객체가 아니라 `PlannedBlocker`입니다. |
| 5 | 유효한 메서드 평가가 커밋된 차단 효과를 선택하지 않고 차단 결과를 반환합니다. | Core 안, 메서드별 평가 뒤. | `CloseTaskResult(close_state=blocked)` 또는 메서드가 담당하는 판단 필드 같은 메서드별 `MethodResult` 필드입니다. `errors[]` 분기는 없습니다. | 분기 선택은 [API 오류 처리 경로](error-routing.md#blocked-result-behavior)로, 닫기 차단 사유 경계는 [API 차단 사유 처리 경로](blocker-routing.md)로, 메서드 세부사항은 메서드 담당 문서로 보냅니다. 이는 전송 오류나 스키마 오류가 아닙니다. |
| 6 | 유효한 메서드 평가가 커밋되는 차단 사유형 또는 비허용 결과를 선택합니다. | Core 안, 메서드별 커밋 분기. | 메서드와 저장 효과 담당 문서가 허용할 때 커밋 효과를 가진 메서드별 `MethodResult`입니다. 예를 들면 커밋된 `PrepareWriteResult` 비허용 결정입니다. | 실패한 전송 호출이 아니라 지속 상태일 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)로, 정확한 결과 필드는 메서드 담당 문서로 보냅니다. `ToolRejectedResponse.errors[]` 분기가 없으므로 공개 오류 우선순위는 적용되지 않습니다. |

MCP `tools/call`에서 MCP 전송이 성공하면 Volicord 도메인 수준 `ToolRejectedResponse`를 포함해 Volicord 응답은 `isError: false`로 래핑됩니다. 호출자는 `base.response_kind`, `errors`, 메서드 결과 필드를 확인하려면 `result.content[0].text`를 JSON으로 파싱해야 합니다.

<a id="primary-error-code-precedence"></a>

## 오류 우선순위

오류를 담는 분기의 `errors`가 비어 있지 않으면 메서드 담당 문서가 더 좁은 메서드별 순서를 정의하지 않는 한 아래 순서로 `errors[0]` 공개 주 오류를 고릅니다. 이 표는 순서만 정의합니다. 공개 오류 코드 의미는 [API 오류 코드](error-codes.md)에 남습니다.

| 우선순위 | 주 `ErrorCode` | 의미 담당 문서 |
|---:|---|---|
| <a id="precedence-validation-failed"></a>1 | `VALIDATION_FAILED` | [`VALIDATION_FAILED`](error-codes.md#errorcode-validation-failed) |
| 2 | `STATE_VERSION_CONFLICT` | [`STATE_VERSION_CONFLICT`](error-codes.md#errorcode-state-version-conflict) |
| <a id="precedence-mcp-unavailable"></a>3 | `MCP_UNAVAILABLE` | [`MCP_UNAVAILABLE`](error-codes.md#errorcode-mcp-unavailable) |
| <a id="precedence-invocation-context-mismatch"></a>4 | `INVOCATION_CONTEXT_MISMATCH` | [`INVOCATION_CONTEXT_MISMATCH`](error-codes.md#errorcode-invocation-context-mismatch) |
| <a id="precedence-no-active-task"></a>5 | `NO_ACTIVE_TASK` | [`NO_ACTIVE_TASK`](error-codes.md#errorcode-no-active-task) |
| <a id="precedence-no-active-change-unit"></a>6 | `NO_ACTIVE_CHANGE_UNIT` | [`NO_ACTIVE_CHANGE_UNIT`](error-codes.md#errorcode-no-active-change-unit) |
| <a id="precedence-baseline-stale"></a>7 | `BASELINE_STALE` | [`BASELINE_STALE`](error-codes.md#errorcode-baseline-stale) |
| <a id="precedence-scope-required"></a>8 | `SCOPE_REQUIRED` | [`SCOPE_REQUIRED`](error-codes.md#errorcode-scope-required) |
| <a id="precedence-scope-violation"></a>9 | `SCOPE_VIOLATION` | [`SCOPE_VIOLATION`](error-codes.md#errorcode-scope-violation) |
| <a id="precedence-write-ticket-required"></a>10 | `WRITE_TICKET_REQUIRED` | [`WRITE_TICKET_REQUIRED`](error-codes.md#errorcode-write-ticket-required) |
| <a id="precedence-write-ticket-invalid"></a>11 | `WRITE_TICKET_INVALID` | [`WRITE_TICKET_INVALID`](error-codes.md#errorcode-write-ticket-invalid) |
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
| <a id="precedence-operation-result-unavailable"></a>25 | `OPERATION_RESULT_UNAVAILABLE` | [`OPERATION_RESULT_UNAVAILABLE`](error-codes.md#errorcode-operation-result-unavailable) |

`volicord.get_operation_result`에서는 접근 맥락 불일치가 메서드 범위 결과 없음 분기보다
먼저 `INVOCATION_CONTEXT_MISMATCH`를 선택합니다. 그 밖에 적용되는 전역 순서는
바뀌지 않습니다.

<a id="state-version-conflict-precedence-exclusion"></a>
### `STATE_VERSION_CONFLICT` 선택 경계

선택 조건:
- 오래된 `expected_state_version`, 오래된 `WriteTicket.basis_state_version`, 멱등 요청 해시 충돌 때문에 메서드가 진행될 수 없으면 거부 응답에서 `STATE_VERSION_CONFLICT`가 선택됩니다.

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
| 오래된 `WriteTicket.basis_state_version` | [오래된 쓰기 티켓 근거 버전](#state-conflict-write-ticket-basis) |
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

<a id="state-conflict-write-ticket-basis"></a>
### 오래된 쓰기 티켓 근거 버전

조건:
- 소비 전에 `WriteTicket.basis_state_version`이 현재 `project_state.state_version`과 같지 않습니다.

공개 오류 코드:
- `STATE_VERSION_CONFLICT`

응답 경로:
- `ToolRejectedResponse.errors[]`

소비 경계:
- 오래된 쓰기 티켓은 소비되지 않습니다.
- 거절된 시도는 소비 쪽 상태 변경을 만들지 않습니다.

세부 필드:
- [상태 충돌 세부 필드](error-details.md#state-conflict-detail-fields)를 사용합니다.

### 만료된 쓰기 티켓

조건:
- 소비 전에 쓰기 티켓이 [`volicord.record_run`](method-record-run.md)과 [`volicord.prepare_write`](method-prepare-write.md)가 담당하는 유효 만료 규칙에 따라 만료되었고, `WriteTicket.basis_state_version`은 오래되지 않았습니다.

공개 오류 코드:
- `WRITE_TICKET_INVALID`

응답 경로:
- `ToolRejectedResponse.errors[]`

우선순위 경계:
- `WriteTicket.basis_state_version`이 오래되었으면 만료로 인한 무효 처리 대신 `STATE_VERSION_CONFLICT`를 선택합니다.
- 만료는 결과 쪽 판단, 차단 사유 코드, 닫기 준비 상태 차단 사유 코드, 미리보기 차단 사유 코드로 모델링하지 않습니다.

세부 필드:
- `ToolError.details.write_ticket_reason=expired`를 사용합니다.

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
