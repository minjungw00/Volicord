# API 차단 사유 처리 경로

이 문서는 닫기 차단 사유와 API 응답 분기 사이의 경계를 담당합니다. 이 문서는 경계 경로 문서이며, 메서드 동작이나 스키마 형태의 담당 문서가 아닙니다.

[API 오류 처리 경로](error-routing.md)에서 응답 분기를 먼저 식별한 뒤 이 문서를 사용합니다.

이 문서가 담당합니다.

- 관심사가 API 오류 쪽인지 닫기 차단 사유 쪽인지에 대한 경계.
- 공개 오류 코드 묶음이 담당 문서가 정의한 `CloseReadinessBlocker` 데이터와 관련될 수 있는 방식.
- 닫기 차단 사유와 API 응답 사이의 경계 질문을 보낼 위치.

이웃 담당 문서:

- 메서드별 동작: [`volicord.close_task`](method-close-task.md)와 다른 메서드 담당 문서.
- 데이터 형태와 값: [API 상태 스키마](schema-state.md), [API 값 집합](schema-value-sets.md#state-and-blocker-values).
- 공개 오류 의미와 우선순위: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md).
- Core 닫기 준비 상태 권한: [Core 모델](../core-model.md#close_task).
- 저장 효과: [저장 효과](../storage-effects.md).
- 표시 문구만: [템플릿 본문](../template-bodies.md).

## 오류와 차단 사유의 공통 경계

- 공개 `ErrorCode`는 [API 오류 코드](error-codes.md)가 정의하는 API 오류 조건 식별자입니다. 이 식별자는 자동으로 `CloseReadinessBlocker.category` 값이 되지 않으며 차단 사유 범주도 아닙니다.
- 거부 응답의 오류 코드는 같은 조건이 닫기 준비 상태에 영향을 줄 수 있다는 이유만으로 차단 사유 범주로 사용하지 않습니다. 그 오류 코드는 API 오류 쪽에 남습니다.
- 닫기 차단 사유의 객체 형태는 [API 상태 스키마](schema-state.md)의 `CloseReadinessBlocker`가 담당합니다. 차단 사유 범주 값은 [API 값 집합](schema-value-sets.md#state-and-blocker-values)이 담당합니다.
- 차단 사유 처리 경로는 API 응답 분기 처리 경로가 정해진 뒤에 적용되며 [API 오류 우선순위](error-precedence.md)를 대신하지 않습니다.
- [API 오류 코드](error-codes.md)는 공개 오류 코드 의미를 정의하고, 이 문서는 공개 API 오류와 닫기 차단 사유 데이터 사이의 경계를 정의합니다.

## 오류와 차단 사유의 경계

| 상황 | 경로 | 경계 |
|---|---|---|
| 유효한 닫기 준비 상태 평가 전 실패 | `ToolRejectedResponse.errors[]`와 `ToolError.code: ErrorCode` | 요청이 유효한 닫기 준비 상태 결과에 도달하지 않았습니다. `CloseReadinessBlocker[]`를 반환하지 않습니다. |
| 유효한 닫기 준비 상태 평가에서 닫기 차단 사유 발견 | 메서드 결과 또는 읽기 전용 상태 결과의 `CloseReadinessBlocker[]` | 데이터는 닫기가 막힌 이유를 설명합니다. 객체 형태는 스키마 담당 문서에, 정확한 차단 사유 범주 값은 값 집합 담당 문서에 남습니다. |
| 유효한 `dry_run` 미리보기에서 차단 사유형 결과 예상 | `DryRunSummary.would_blockers: PlannedBlocker[]` | 미리보기 차단 사유는 저장된 `CloseReadinessBlocker` 객체가 아니며 닫기 준비 상태를 만들지 않습니다. |
| 응답 분기 선택이 질문인 경우 | [API 오류 처리 경로](error-routing.md) | 이 문서는 분기 경계가 식별된 뒤에 적용됩니다. 모든 응답 분기를 선택하지는 않습니다. |

## 범주 기반 처리 경계

메서드나 상태 결과가 담당 계약에 따라 닫기 차단 사유 데이터를 반환하면, `CloseReadinessBlocker.category`는 그 데이터의 담당 관심사를 가리킵니다.

정확한 차단 사유 범주 값은 [API 값 집합](schema-value-sets.md#state-and-blocker-values)이 담당합니다. 이 문서는 그 값 자체를 정의하지 않습니다. 차단 사유 범주가 붙은 데이터를 알맞은 담당 관심사로 보내는 경계만 설명합니다.

전체 닫기 차단 사유 분류표, 스키마 필드 표, Task 닫기 평가 순서는 이 문서가 담당하지 않습니다.

| 담당 관심사 | 처리 경로에서의 사용 | 경계 |
|---|---|---|
| Core 상태, 종료 전이, 기준 상태, 복구, 쓰기 호환성 | 차단 사유 범주가 붙은 데이터는 Core 또는 메서드가 담당하는 상태 요구사항을 가리킬 수 있습니다. | Core 의미는 [Core 모델](../core-model.md)이, 메서드 동작은 [`volicord.close_task`](method-close-task.md)가 담당합니다. |
| 범위, 사용자 소유 판단, 민감 동작 승인, 접점 역량 | 차단 사유 범주가 붙은 데이터는 닫기가 사용자, 범위, 민감 동작 승인, 접점 역량 담당 문서에 의존함을 보여 줄 수 있습니다. | 차단 사유는 사용자 결정, 민감 동작 승인, 범위 변경, 역량 선언을 기록하지 않습니다. |
| 증거와 아티팩트 근거 | 차단 사유 범주가 붙은 데이터는 닫기가 증거 충분성이나 지속 아티팩트 가용성에 의존함을 보여 줄 수 있습니다. | 증거와 아티팩트 의미는 각 담당 문서에 남습니다. 차단 사유 처리 경로는 충분성이나 가용성을 증명하지 않습니다. |
| 최종 수락과 잔여 위험 | 차단 사유 범주가 붙은 데이터는 닫기가 최종 수락, 잔여 위험 표시, 잔여 위험 수락에 의존함을 보여 줄 수 있습니다. | 차단 사유는 수락이나 위험 수락을 만들지 않습니다. |

## 공개 오류 코드가 차단 사유로 표현되는 경우

공통 경계 규칙을 먼저 적용한 뒤에만 이 표를 사용합니다.

조건:
- 공개 오류 코드 묶음은 담당 문서가 정의한 닫기 차단 사유 데이터가 있을 때만 닫기 차단 사유와 관련됩니다.
- 스키마나 메서드 담당 문서가 그 정확한 사용을 명시적으로 허용할 때만 공개 `ErrorCode` 값을 `CloseReadinessBlocker.code`에 복사할 수 있습니다.

허용되지 않는 것:
- 그런 담당 문서 허용 없이 공개 `ErrorCode` 값을 `CloseReadinessBlocker.code`에 복사하면 안 됩니다.

| 공개 오류 코드와의 관계 | 차단 사유 쪽 경로 | 경계 |
|---|---|---|
| 증거, 아티팩트, 수락, 사용자 소유 판단, 민감 동작 승인, 범위, 자율성 경계, 기준 상태, 역량 묶음 | 담당 문서가 정의한 `CloseReadinessBlocker.category`와 `CloseReadinessBlocker.code`를 통해 보냅니다. | 공개 오류 코드 의미는 [API 오류 코드](error-codes.md)에 남습니다. 닫기 차단 사유 형태는 [API 상태 스키마](schema-state.md)가 담당합니다. 차단 사유 범주 값은 [API 값 집합](schema-value-sets.md#state-and-blocker-values)이 담당합니다. 메서드별 차단 사유 생성은 [`volicord.close_task`](method-close-task.md)가 담당합니다. |
| 읽기용 보기 최신성 묶음 | 담당 문서가 허용할 때 관련 진단으로 이름 붙일 수 있습니다. | 최신성 진단만으로는 닫기 차단 사유가 아닙니다. |
| 상태 버전 또는 멱등성 충돌 묶음 | 닫기 차단 사유 표현이 없습니다. | 이 실패는 닫기 준비 상태 평가 전에 거부되며 [API 오류 우선순위](error-precedence.md)에 남습니다. |

## `volicord.close_task` 메서드 경로

메서드별 닫기 동작은 [`volicord.close_task`](method-close-task.md)가 담당합니다. 요청 검증, `intent` 처리, 종료 상태 변경, 상태 버전 동작, 커밋된 차단 결과는 그 메서드 담당 문서로 보냅니다.

이 문서는 `volicord.close_task`가 반환하는 닫기 차단 사유 데이터와 이웃 API 오류, 스키마, 값 집합, Core, 저장소, 표시 담당 문서 사이의 경계만 정의합니다.

## 권한 경계

차단 사유 처리 경로는 닫기 차단 사유 데이터를 분류할 뿐입니다. 이 경로는 아래 항목을 만들거나 대신하지 않습니다.

- 최종 수락 또는 잔여 위험 수락
- 사용자 소유 판단, 민감 동작 승인, `Write Authorization`
- 증거 충분성 또는 아티팩트 가용성
- 닫기 완료 또는 종료 `Task` 상태
- 차단 사유의 지속 저장 또는 상태 버전 증가
- 표시 문구
