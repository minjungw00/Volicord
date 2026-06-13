# API 차단 사유 처리 경로

이 문서는 닫기 준비 상태 차단 사유와 API 응답 분기 사이의 처리 경계만 담당합니다.

닫기 관련 발견 사항이 언제 `CloseReadinessBlocker[]`로 표현되는지, 언제 API가 거부 응답이나 미리보기 분기에 남는지, 이웃 계약은 어느 담당 문서가 정의하는지를 설명합니다. `harness.close_task` 메서드 동작, `CloseReadinessBlocker` 형태, 차단 사유 범주 값, Core 닫기 준비 상태 권한, 저장 효과, 공개 `ErrorCode` 의미, 응답 분기 선택, 표시 문구는 정의하지 않습니다.

## 담당 경계

| 관심사 | 담당 문서 |
|---|---|
| 차단 사유/API 응답 처리 경계 | 이 문서 |
| `harness.close_task` 요청 동작, 평가 순서, 결과 분기, 커밋된 차단 결과 | [`harness.close_task`](method-close-task.md) |
| `CloseReadinessBlocker` 필드와 중첩 형태 | [API 상태 스키마](schema-state.md) |
| 정확한 `CloseReadinessBlocker.category` 값과 그 밖의 enum 형태 API 어휘 | [API 값 집합](schema-value-sets.md#state-and-blocker-values) |
| Core 닫기 준비 상태 권한, 최종 수락, 잔여 위험 수락, 대체 불가 규칙 | [Core 모델의 닫기 준비 상태](../core-model.md#close_task) |
| 거부 응답, 차단 결과, `dry_run` 응답 분기 선택 | [API 오류 처리 경로](error-routing.md) |
| 공개 `ErrorCode` 의미와 우선순위 | [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md) |
| 표시 라벨과 렌더링 문구 | [템플릿 본문](../template-bodies.md) |

## API 오류와 차단 사유 경계

| 상황 | 경로 | 경계 |
|---|---|---|
| 유효한 닫기 준비 상태 평가 전 실패 | `ToolRejectedResponse.errors[]`와 `ToolError.code: ErrorCode` | 요청이 유효한 닫기 준비 상태 결과에 도달하지 않았습니다. `CloseReadinessBlocker[]`를 반환하지 않습니다. |
| 유효한 닫기 준비 상태 평가에서 닫기 차단 사유 발견 | 메서드 결과 또는 읽기 전용 상태 결과의 `CloseReadinessBlocker[]` | 데이터는 닫기가 막힌 이유를 설명합니다. 스키마 형태와 정확한 범주 값은 스키마와 값 집합 담당 문서에 남습니다. |
| 유효한 `dry_run` 미리보기에서 차단 사유형 결과 예상 | `DryRunSummary.would_blockers: PlannedBlocker[]` | 미리보기 차단 사유는 저장된 `CloseReadinessBlocker` 객체가 아니며 닫기 준비 상태를 만들지 않습니다. |
| 응답 분기 선택이 질문인 경우 | [API 오류 처리 경로](error-routing.md) | 이 문서는 분기 경계가 식별된 뒤에 적용됩니다. 모든 응답 분기를 선택하지는 않습니다. |

## 범주 처리 경계

`CloseReadinessBlocker.category`는 닫기 준비 상태 차단 사유를 책임지는 담당 문서 묶음을 식별합니다. 정확한 범주 값은 [API 값 집합](schema-value-sets.md#state-and-blocker-values)이 담당합니다. 이 문서는 범주를 가진 차단 사유 데이터를 알맞은 담당 관심사로 보내는 경계만 설명합니다.

| 담당 관심사 | 처리 경로에서의 사용 | 경계 |
|---|---|---|
| Core 상태, 종료 전이, 기준 상태, 복구, 쓰기 호환성 | 범주를 가진 차단 사유는 Core 또는 메서드가 담당하는 상태 요구사항을 가리킬 수 있습니다. | Core 의미는 [Core 모델](../core-model.md)이, 메서드 동작은 [`harness.close_task`](method-close-task.md)가 담당합니다. |
| 범위, 사용자 소유 판단, 민감 동작 승인, 접점 역량 | 범주를 가진 차단 사유는 닫기가 사용자, 범위, 승인, 접점 역량 담당 문서에 의존함을 보여 줄 수 있습니다. | 차단 사유는 사용자 결정, 승인, 범위 변경, 역량 선언을 기록하지 않습니다. |
| 증거와 아티팩트 근거 | 범주를 가진 차단 사유는 닫기가 증거 충분성이나 지속 아티팩트 가용성에 의존함을 보여 줄 수 있습니다. | 증거와 아티팩트 의미는 각 담당 문서에 남습니다. 이 경로는 충분성이나 가용성을 증명하지 않습니다. |
| 최종 수락과 잔여 위험 | 범주를 가진 차단 사유는 닫기가 최종 수락, 잔여 위험 표시, 잔여 위험 수락에 의존함을 보여 줄 수 있습니다. | 차단 사유는 수락이나 위험 수락을 만들지 않습니다. |

## 공개 코드 경계

공개 `ErrorCode` 값은 공개 API 식별자이지 차단 사유 코드가 아닙니다. 어떤 조건이 유효한 닫기 준비 상태 평가 중 발견되고, 적용되는 담당 문서가 그 조건에 대해 지원되는 차단 사유 범주나 차단 사유 코드를 정의할 때만 공개 오류 코드 묶음을 닫기 준비 상태 차단 사유와 관련지을 수 있습니다.

스키마나 메서드 담당 문서가 그 정확한 사용을 명시적으로 허용하지 않는 한 공개 값을 `CloseReadinessBlocker.code`에 복사하지 않습니다.

| 공개 코드와의 관계 | 차단 사유 쪽 경로 | 경계 |
|---|---|---|
| 증거, 아티팩트, 수락, 사용자 판단, 승인, 범위, 자율성 경계, 기준 상태, 역량 묶음 | 담당 문서가 정의한 `CloseReadinessBlocker.category`와 `CloseReadinessBlocker.code`를 통해 보냅니다. | 공개 코드 의미는 [API 오류 코드](error-codes.md)에, 정확한 차단 사유 값은 [API 상태 스키마](schema-state.md)와 [API 값 집합](schema-value-sets.md)에 남습니다. |
| 읽기용 보기 최신성 묶음 | 담당 문서가 허용할 때 관련 진단으로 이름 붙일 수 있습니다. | 최신성 진단만으로는 닫기 준비 상태 차단 사유가 아닙니다. |
| 상태 버전 또는 멱등성 충돌 묶음 | 닫기 준비 상태 차단 사유 표현이 없습니다. | 이 실패는 닫기 준비 상태 평가 전에 거부되며 [API 오류 우선순위](error-precedence.md)에 남습니다. |

<a id="harnessclose_task-close-blockers"></a>
## `harness.close_task` 메서드 경로

메서드별 닫기 동작은 [`harness.close_task`](method-close-task.md)가 담당합니다. 사전 확인 거부, `intent=check`, `intent=complete`, 종료 상태 변경, 유효하지 않은 종료 전이, 상태 버전 동작, 커밋된 차단 결과는 그 메서드 담당 문서로 보냅니다.

이 문서는 그 메서드가 반환하는 차단 사유 데이터와 이웃 API 오류, 스키마, 값 집합, Core, 저장소, 표시 담당 문서 사이의 경계만 정의합니다.

## 권한 경계

차단 사유 처리 경로는 닫기 준비 상태 차단 사유 데이터를 분류합니다. 이 경로는 아래 항목을 만들거나 대신하지 않습니다.

- 최종 수락 또는 잔여 위험 수락
- 사용자 소유 판단, 민감 동작 승인, `Write Authorization`
- 증거 충분성 또는 아티팩트 가용성
- 닫기 완료 또는 종료 `Task` 상태
- 차단 사유 지속 저장 또는 상태 버전 증가
- 렌더링 표시 문구
