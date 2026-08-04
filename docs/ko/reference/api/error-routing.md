# API 오류 처리 경로

이 문서는 거부 응답, 차단 결과, `dry_run` 미리보기에 대한 API 응답 분기 경로를 담당합니다.

[오류와 차단 사유의 정식 결정 흐름](error-precedence.md#canonical-error-blocker-decision-flow)이 전송, 어댑터, Core 운영 실패와 Core 메서드 응답을 구분한 뒤 Volicord API 응답 분기를 고를 때 이 문서를 사용합니다. 개별 닫기 차단 사유를 매핑하거나, 차단 사유 범주와 코드를 정의하거나, 메서드별 동작을 결정하는 문서로 사용하지 않습니다.

이 문서가 담당합니다.

- `ToolRejectedResponse.errors[]`, 메서드별 차단 결과, `ToolDryRunResponse` 미리보기 진단 사이의 분기 경계.
- 요청, 선행조건, 상태, 멱등성, 디코딩 뒤 dry-run 실패에 대한 거부 응답 경로.
- `PrepareWriteResult` 차단 판단과 `CloseTaskResult(close_state=blocked)`를 구분하는 차단 결과 분기 경로.
- 일반 결과, 유효한 미리보기, 미리보기 차단 사유, 거부 응답에 대한 `dry_run`
  분기 경로.

이웃 담당 문서:

- 공개 오류 코드 의미: [API 오류 코드](error-codes.md).
- 오류와 차단 사유의 정식 결정 흐름 및 주 공개 오류 선택: [API 오류 우선순위](error-precedence.md#canonical-error-blocker-decision-flow).
- 기계 판독용 오류 세부사항: [API 오류 세부사항](error-details.md).
- `CloseReadinessBlocker`, `WriteDecisionReason`, `PlannedBlocker`, 공통 분기 형태: [API 상태 스키마](schema-state.md), [API 코어 스키마](schema-core.md). 범주와 enum 형태 값은 [API 값 집합](schema-value-sets.md)이 담당합니다.
- 닫기 준비 상태 의미와 대체 불가 규칙: [Core 모델의 닫기 준비 상태](../core-model.md#close_task).
- 닫기 차단 사유와 API 응답 사이의 경계, 공개 오류 코드가 차단 사유로 표현되는 경우의 경계: [API 차단 사유 처리 경로](blocker-routing.md).
- 메서드별 동작: [`volicord.close_task`](method-close-task.md)와 다른 메서드 담당 문서.
- 표시 문구만: [템플릿 본문](../template-bodies.md).
- 제품 전체 실패 범주 의미와 선택 경계: [실패 모델](../failure-model.md).

구현은 이 경로 경계를 명시적으로 유지합니다.
[`method_rejection.rs`](../../../../crates/volicord-core/src/method_rejection.rs)는
메서드 중립적인 거부 응답과 dry-run 응답 구성을 담당하고,
[`error_boundary/`](../../../../crates/volicord-core/src/error_boundary/) 아래의 집중
모듈은 Store 또는 의미 담당자의 typed 실패를 변환합니다.
[`artifact.rs`](../../../../crates/volicord-core/src/artifact.rs),
[`continuity/`](../../../../crates/volicord-core/src/continuity/),
[`close_readiness/`](../../../../crates/volicord-core/src/close_readiness/),
[`write_ticket/`](../../../../crates/volicord-core/src/write_ticket/) 같은 의미
담당자는 typed fact 또는 오류를 반환하며 공개 응답 분기를 구성하지 않습니다.
Write Ticket planning은 envelope, dry-run, 응답 state-version metadata를
포함하지 않은 typed 의미 validation, invariant, Store, UserAction 실패를
구분합니다. `methods/prepare_write.rs`만 이 실패를 현재 메서드 metadata와 결합해
공개 `PlanError` 또는 응답 분기를 선택합니다.
Recording 서비스도 폐쇄형 `RecordingError` variant를 반환하며,
`methods/record_run.rs`만 이 의미 오류를 현재 envelope, dry-run intent,
state version과 결합해 공개 분기를 선택합니다.
메서드 모듈은 메서드별 차단 결과와 최종 응답 구성을 유지합니다. 이 소스 경로는 이
문서가 담당하는 공개 경로 규칙을 구현하지만 다시 정의하지 않습니다.

## 오류와 차단 사유

| 개념 | 공개 형태 | 세부 항목 |
|---|---|---|
| 거부 응답 | `ToolRejectedResponse.errors[]` | [거부 응답](#error-vs-blocker-rejected-response) |
| 차단 결과 | 메서드별 결과 필드 | [차단 결과](#error-vs-blocker-blocked-result) |
| `dry_run` 미리보기 | `ToolDryRunResponse` | [`dry_run` 미리보기](#error-vs-blocker-dry-run-preview) |

<a id="error-vs-blocker-rejected-response"></a>
거부 응답:
- 공개 형태: 필수 `ToolError.category: FailureCategory`와
  `ToolError.code: ErrorCode`를 담은 `ToolRejectedResponse.errors[]`입니다.
- 의미: 메서드가 커밋되는 동작으로 진행하지 않았다는 뜻입니다.
- 조건: 타입이 정해진 Volicord 요청이 Core에 도달했고, 메서드가 담당하는 결과 분기 전에 요청 검증, 최신성, 호출 맥락, `actor_source`, `operation_category`, 그 밖의 선행조건이 실패한 경우입니다.
- 상태 영향: 커밋된 동작이 없고 상태 변경도 없습니다.

Core 실행 전에 일어나는 전송 및 어댑터 실패는 이 분기 밖에 있습니다. [MCP 전송](../mcp-transport.md)이나 해당 전송 또는 어댑터 담당 문서로 보냅니다.
Known MCP tool의 descriptor 유래 인자 issue는 구조화된 MCP invalid-arguments 응답으로
남습니다. Descriptor에는 유효하지만 정확한 Rust request type으로 decode할 수 없는 값은
내부 schema contract 실패입니다. 이를 사용자 field나 다른 union branch로 다시 분류하지
않습니다. 두 경로 모두 `ToolRejectedResponse`를 만들거나 Core에 도달하지 않습니다.
Task 상태 결속 MCP 호출의 메서드 admission과 정확한 메서드별 action form 일치 검사도
Core 실행 전 adapter 실패입니다. 현재 catalog에 없는 메서드는 typed
`WORKFLOW_ACTION_NOT_ALLOWED` MCP 응답을 사용하고, 허용된 메서드에서 form이 없거나
일치하지 않으면 `MCP_ACTION_FORM_STALE`을 사용합니다. 두 응답 모두
`reached_core=false`, `committed=false`, 상태 변경 없음 상태를 보존하고 [MCP
전송](../mcp-transport.md#public-argument-projection)이 담당하는 현재 workflow와 form 복구
사실을 담습니다.
어떤 메서드 결과도 만들 수 없게 하는 typed Core 운영 불가도 이 분기 밖에 있으며 호출
어댑터가 변환합니다.

<a id="error-vs-blocker-blocked-result"></a>
차단 결과:
- 공개 형태: `write_decision_reasons`나 `blockers` 같은 메서드별 결과 필드입니다.
- 의미: 메서드가 동작별 차단 결과를 반환했을 수 있다는 뜻입니다.
- 경계: 차단 결과 데이터는 공개 전송 또는 스키마 오류가 아닙니다.
- 상태 영향: 응답 전용일 수도 있고 커밋될 수도 있습니다. 메서드 담당 문서와 [저장 효과](../storage-effects.md)가 허용할 때만 커밋된 차단 사유형 결과가 가능합니다.

<a id="error-vs-blocker-dry-run-preview"></a>
`dry_run` 미리보기:
- 공개 형태: `DryRunSummary.would_errors[]` 또는 `DryRunSummary.would_blockers[]`를 담은 `ToolDryRunResponse`입니다.
- 의미: 유효한 `dry_run` 요청에서 미리 볼 수 있는 진단입니다.
- 상태 영향: 커밋된 쓰기가 아니며 저장된 차단 사유 상태도 아닙니다.

`ErrorCode` 값은 공개 API 식별자입니다. 닫기 차단 사유와 API 응답 사이의 경계, 공개 오류 코드가 차단 사유로 표현되는 경우의 경계는 [API 차단 사유 처리 경로](blocker-routing.md)가 담당합니다.

표시 문구는 [템플릿 본문](../template-bodies.md)만 담당합니다. API 오류 의미나 차단 사유 의미를 정의하지 않으며, 이 값을 `ErrorCode`, 차단 사유 코드 값, 기계 판독용 `ToolError.details` 키로 사용하면 안 됩니다.

### 실패 범주 분기 경계

| `FailureCategory` | API 분기 규칙 |
|---|---|
| `rejected` | 정책 평가 전의 구조적 요청 또는 필수 맥락 실패는 `ToolRejectedResponse`를 사용합니다. |
| `not_allowed` | 메서드 담당자가 정책 평가 뒤 비허용 결과를 정의하면 같은 조건을 `ToolRejectedResponse`로 만들지 않고 그 메서드별 결과를 사용합니다. 커밋 여부는 메서드와 저장 효과 담당자가 정합니다. |
| `unavailable` | 적용되는 사용 불가 공개 코드가 있는 담당자 정의 메서드 결과는 `ToolRejectedResponse`를 사용합니다. 필수 인프라 의존성이 어떤 메서드 결과도 만들 수 없으면 typed Core 운영 오류 경로로 나가며 API 분기가 없습니다. |
| `degraded` | 핵심 동작을 진실하게 계속하면 성공 메서드 결과에서 담당자가 정의한 진단으로 불완전한 보조 구성 요소를 드러냅니다. 같은 동작을 `ToolError(category=degraded)`로 거절하지 않습니다. |
| `corrupt` | 종속 동작은 `PERSISTED_DATA_CORRUPT`를 담은 `ToolRejectedResponse`를 사용하고 정책이나 효과 전에 닫힌 실패로 중단합니다. |

범주는 필수 기계 판독 분류입니다. 공개 코드나
[API 오류 세부사항](error-details.md#reason)이 담당하는 도메인 `details.reason`을 대신하지
않습니다.

<a id="blocked-and-dry-run-behavior"></a>

## 거부 응답 동작

| 조건 | 세부 항목 |
|---|---|
| 요청 검증이 진행 전에 실패 | [요청 검증 실패](#rejected-request-validation-failure) |
| 영속 담당 데이터 손상 | [선행조건 실패](#rejected-precondition-failure) |
| 선행조건이 커밋 전에 실패 | [선행조건 실패](#rejected-precondition-failure) |
| 상태 또는 멱등성 충돌 | [상태 또는 멱등성 충돌](#rejected-state-or-idempotency-conflict) |
| 디코딩된 `dry_run=true` 요청의 거부 | [디코딩된 `dry_run=true` 요청의 거부](#rejected-dry-run-pre-preview-failure) |

<a id="rejected-request-validation-failure"></a>
### 요청 검증 실패

조건:
- 메서드가 진행되기 전에 요청 형태, 스키마, 프로필, 스테이징된 아티팩트 핸들 검증이 실패합니다.

경계:
- 신뢰할 수 없는 입력의 형태가 잘못된 `category=rejected` 조건입니다. 영속 신뢰 담당
  데이터 손상은 별도의 범주와 코드를 사용합니다.

응답 경로:
- `ToolRejectedResponse.errors[]`.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

결과 경계:
- 메서드별 결과 전용 필드는 이 거부 응답에 포함하지 않습니다.

Descriptor 검증이 Core 전에 실패하면 이 공개 API branch가 아니라 MCP가
`committed=false`, `reached_core=false`, 독립적으로 유효한 routing 좌표로 읽을 수 있는
한도 있는 현재 권한 맥락, 정확한 retry contract를 반환합니다. Checkpoint, UserAction,
Core event, 쓰기 권한, Product Repository 변경은 만들어지지 않습니다. Schema 검증은
성공했지만 Core가 권한 좌표 불일치를 찾으면 이 거부 branch가 typed
`AuthorityBasisMismatch`의 expected/received 값과 효과 없음 사실을 사용합니다. 두 조건
모두 영속 데이터 손상으로 routing하지 않습니다.

<a id="rejected-precondition-failure"></a>
### 선행조건 실패

조건:
- 커밋 전에 호출 맥락, `actor_source`/`operation_category` 호환성, 결정적으로 존재하지
  않는 `Task` 식별자, 영속 담당 데이터 검증, 정확한 계약 선택, 그 밖의 메서드 수준
  선행조건이 거부를 확정합니다.

응답 경로:
- `ToolRejectedResponse.errors[]`.

상태 영향:
- 기록, 재실행 행, 아티팩트, 이벤트, 쓰기 티켓 소비, 닫기 상태 변경, 상태 버전 증가가 없습니다.

<a id="rejected-state-or-idempotency-conflict"></a>
### 상태 또는 멱등성 충돌

조건:
- `expected_state_version`이 오래됐거나 멱등 요청 해시가 충돌합니다. 쓰기 티켓 감사용 `basis_state_version` 불일치는 충돌이 아닙니다.

응답 경로:
- `STATE_VERSION_CONFLICT`를 담은 `ToolRejectedResponse.errors[]`.

상태 영향:
- 커밋되는 동작이 진행되지 않습니다.
- 담당 상태 변경이 발생하지 않습니다.

경로 경계:
- 이 충돌은 차단 사유가 아닙니다.
- 소비 메서드에서 상태 결합 쓰기 티켓이 유효하지 않으면 대신
  `WRITE_TICKET_INVALID`를 반환합니다. 닫기 준비 상태에서 발견한 미해결 무효화
  티켓은 메서드 소유 차단 사유 데이터입니다. 관련 없는 상태 읽기와 쓰기는 티켓을
  무효화하지 않습니다.

<a id="rejected-dry-run-pre-preview-failure"></a>
### 디코딩된 `dry_run=true` 요청의 거부

조건:
- 요청이 정규화된 dry-run 요청 의도로 디코딩된 뒤 메서드 수준 거부에
  도달합니다. 요청된 dry-run 처리를 금지하는 메서드와 결과 또는 미리보기를
  만들기 전의 검증, 상태, 승인, 정책 거부가 여기에 포함됩니다. Core 운영
  불가에는 API 응답 분기가 없습니다.

응답 경로:
- `base.response_kind=rejected`, `base.dry_run=true`인
  `ToolRejectedResponse`.

상태 영향:
- 커밋되는 동작이나 `dry_run` 미리보기가 만들어지지 않습니다.

미리보기 경계:
- 이 거부를 `DryRunSummary.would_errors[]`나 `PlannedBlocker`로 표현하지 않습니다.

거부 응답은 메서드가 커밋되는 동작으로 진행하지 않았다는 뜻입니다. 거부 응답은 차단 결과가 아니며, 요청에 없던 권한, 증거, 수락, 닫기 상태를 만들지 않습니다.

<a id="blocked-result-behavior"></a>

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

실패 범주 경계:
- 이 메서드 정의 정책 평가 뒤 비허용 결과는 `NotAllowed`이며 구조적 `Rejected` 분기가
  아닙니다. 현재 적용 Change Unit이 없으면 더 앞선 `ToolRejectedResponse`에서
  `category=rejected`, `code=NO_ACTIVE_CHANGE_UNIT`,
  `details.reason=current_change_unit_required`를 사용합니다.

응답 경로:
- `write_decision_reasons: WriteDecisionReason[]`.

상태 영향:
- 커밋되는 비허용 효과는 메서드 담당 문서와 [저장 효과](../storage-effects.md)가 정의합니다.

결과 데이터:
- 메서드 담당 판단 사유를 사용합니다.

결과 경계:
- `PrepareWriteResult` 차단 판단은 `CloseReadinessBlocker`를 반환하지 않습니다.

<a id="blocked-close-task-result"></a>
### `CloseTaskResult(close_state=blocked)`

분기 조건:
- `volicord.close_task` 메서드 계약에 따라 유효한 `CloseTaskResult(close_state=blocked)`가 반환되는 경우입니다.

응답 분기:
- 메서드 결과가 `blockers: CloseReadinessBlocker[]`를 담습니다.

상태 영향:
- 차단된 `close_task` 결과가 응답 전용인지 커밋되는지는 `close_task` 메서드 담당 문서와 [저장 효과](../storage-effects.md)가 정의합니다.

결과 데이터 경계:
- 닫기 차단 사유/API 응답 처리 경계와 공개 오류 코드가 차단 사유로 표현되는 경우는 [API 차단 사유 처리 경로](blocker-routing.md)가 담당합니다.
- `CloseReadinessBlocker` 형태와 범주 값은 [API 상태 스키마](schema-state.md)와 [API 값 집합](schema-value-sets.md)에 남습니다.

공개 오류 코드가 차단 사유로 표현되는 경우:
- `CloseTaskResult(close_state=blocked)`는 `STATE_VERSION_CONFLICT`를 사용하지 않습니다.

<a id="blocked-read-only-observation"></a>
### 읽기 전용 관찰

분기 조건:
- 읽기 전용 상태 또는 확인 결과가 닫기 차단 사유 관찰 데이터를 노출합니다.

응답 분기:
- 읽기 전용 `CloseReadinessBlocker` 관찰 데이터.

상태 영향:
- 읽기 때문에 저장된 차단 사유나 상태 버전 증가가 생기지 않습니다.

차단 결과는 메서드가 동작별 차단 결과를 반환했을 수 있다는 뜻입니다. 공개 전송 또는 스키마 오류가 아니며 `ToolRejectedResponse.errors[]`를 사용하지 않습니다. 커밋된 차단 사유형 결과와 상태 영향은 [API 메서드](methods.md)가 안내하는 관련 메서드 담당 문서와 [저장 효과](../storage-effects.md)가 허용해야 합니다.

<a id="dry-run-behavior"></a>

## `dry_run` 동작

`base.dry_run`은 정규화된 요청 의도를 보존합니다. 응답 분기를 판별하는 값이
아니며 실제 분기는 `base.response_kind`나 타입 지정 응답 variant가 선택합니다.

| `dry_run` 경우 | 세부 항목 |
|---|---|
| 유효한 일반 결과 요청 | [유효한 일반 결과 `dry_run=true`](#dry-run-valid-read-only) |
| 유효한 상태 영향 또는 스테이징 미리보기 | [유효한 `dry_run` 미리보기](#dry-run-valid-preview) |
| 미리보기의 예상 차단 사유 | [`dry_run` 미리보기의 예상 차단 사유](#dry-run-expected-blockers) |
| 디코딩 뒤 거부 | [디코딩 뒤 `dry_run=true` 거부](#dry-run-pre-commit-failure) |
| 타입 지정 의도 전 실패 | [타입 지정 dry-run 의도 전 실패](#dry-run-predecode-failure) |

<a id="dry-run-valid-read-only"></a>
### 유효한 일반 결과 `dry_run=true`

조건:
- 메서드 계약이 정규화된 dry-run 요청 의도를 일반 결과 분기로 처리합니다.

응답 경로:
- `base.response_kind=result`, `base.dry_run=true`를 담은 메서드별
  결과입니다. 현재 일반 결과 메서드는 `base.effect_kind=read_only`를
  사용합니다.

분기 경계:
- `dry_run=true`를 `ToolDryRunResponse`의 동의어로 보지 않습니다.

<a id="dry-run-valid-preview"></a>
### 유효한 `dry_run` 미리보기

조건:
- 메서드 계약이 정규화된 dry-run 요청 의도를 미리보기 분기로 매핑합니다.

응답 경로:
- `base.response_kind=dry_run`, `base.dry_run=true`, `DryRunSummary`를
  담은 `ToolDryRunResponse`입니다.

상태 영향:
- `dry_run` 미리보기는 커밋된 쓰기가 아닙니다.

<a id="dry-run-expected-blockers"></a>
### `dry_run` 미리보기의 예상 차단 사유

조건:
- 유효한 `dry_run` 미리보기에 예상 차단 사유가 있습니다.

응답 경로:
- `DryRunSummary.would_blockers: PlannedBlocker[]`.

미리보기 경계:
- 미리보기 차단 사유는 저장된 `CloseReadinessBlocker` 객체가 아닙니다.
- `PlannedBlocker.code`는 `STATE_VERSION_CONFLICT`가 될 수 없습니다.

<a id="dry-run-pre-commit-failure"></a>
### 디코딩 뒤 `dry_run=true` 거부

조건:
- 요청이 정규화된 dry-run 요청 의도로 성공적으로 디코딩된 뒤 어떤 거부
  경로에 도달합니다.

응답 경로:
- `base.response_kind=rejected`, `base.dry_run=true`인
  `ToolRejectedResponse`.

미리보기 경계:
- 실패를 `dry_run` 미리보기 데이터로 표현하지 않습니다.
- 오래된 상태는 미리보기 전에 거부됩니다.

<a id="dry-run-predecode-failure"></a>
### 타입 지정 dry-run 의도 전 실패

조건:
- 타입 지정 dry-run 의도를 얻기 전에 요청 디코딩이 실패합니다.

의도 기본값:
- 정규화된 의도의 기본값은 요청하지 않음입니다. 실패를 API 거부 응답으로
  표현하면 `base.dry_run`은 `false`입니다.
- 경계는 형식이 잘못된 원시 JSON을 검사해 다른 값을 추론하지 않습니다.
- API 응답 분기 밖의 전송 또는 어댑터 운영 실패에는 응답 base가 없습니다.
