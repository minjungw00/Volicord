# API 오류 경로

이 문서는 거부 응답, 차단 결과, `dry_run` 미리보기, 금지된 차단 사유 코드 사용, `close_task` 차단 사유 매핑에서 API 오류와 차단 사유의 경계를 담당합니다.

공개 `ErrorCode` 의미, 주 코드 우선순위, `ToolError.details`, 응답 분기 형태, 표시 라벨, 닫기 준비 상태 의미는 정의하지 않습니다.

## 담당 경계

이 문서가 담당합니다.

- `ToolRejectedResponse.errors[]`, 메서드별 차단 결과, `ToolDryRunResponse` 미리보기 진단 사이의 경계.
- 커밋 전 공개 오류가 차단 사유 코드 배열로 들어가지 않게 하는 규칙.
- 필요한 경우 닫기 준비 상태 발견 사항과 공개 오류 코드 묶음을 연결하는 `close_task` 매핑.

이 문서는 담당하지 않습니다.

- 공개 코드 의미: [API 오류 코드](error-codes.md).
- 주 공개 오류 선택: [API 오류 우선순위](error-precedence.md).
- 기계 판독용 오류 세부사항: [API 오류 세부사항](error-details.md).
- `CloseReadinessBlocker`, `WriteDecisionReason`, `PlannedBlocker`, 공통 분기 형태: [API 상태 스키마](schema-state.md), [API 값 집합](schema-value-sets.md), [API 코어 스키마](schema-core.md).
- 닫기 준비 상태 의미와 대체 불가 규칙: [Core 모델의 닫기 준비 상태](../core-model.md#close_task).

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
| 잔여 위험 수락 누락 | [잔여 위험 수락 누락](#close-mapping-unaccepted-residual-risk) |
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
### 잔여 위험 수락 누락

조건:
- 잔여 위험은 보였지만 수락 기록이 없습니다.

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
