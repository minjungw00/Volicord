# API 오류

shaping 메서드는 기존 validation, conflict, replay branch로 stale Task, scope, baseline, checkpoint, Change Unit, UserAction resolution 좌표를 거절합니다. Aggregate 실패는 checkpoint, request, event, replay, state version에 부분 효과를 남기지 않습니다.

이 문서는 질문에 맞는 집중 API 오류 참조 문서를 찾는 첫 경로입니다. 계약 담당
문서가 아니라 경로 안내 문서이며, 정확한 오류 계약은 연결된 담당 문서에 있습니다.

이 문서는 아래 담당 문서로 안내합니다.

- 공개 `ErrorCode` 의미, 오류 우선순위, API 응답 분기 처리 경로.
- 제품 전체 실패 범주 의미와 공개 `FailureCategory` 값.
- 닫기 차단 사유와 API 응답 사이의 경계, `ToolError.details`.
- 메서드별 동작, 스키마 데이터 형태, 저장 효과, 표시 문구.

## 오류 경로

| 질문 | 담당 문서 |
|---|---|
| 실패 범주가 무엇을 뜻하는지 | [실패 모델](../failure-model.md) |
| API가 허용하는 정확한 `FailureCategory` 식별자 | [API 값 집합](schema-value-sets.md#failure-category-values) |
| 공개 `ErrorCode`가 무엇을 뜻하는지 | [API 오류 코드](error-codes.md) |
| Workflow 거부에 어떤 typed 현재 fact가 들어가는지 | [API 오류 세부사항](error-details.md#workflow-rejection-detail-fields) |
| 어떤 공개 오류가 선택되는지 | [API 오류 우선순위](error-precedence.md) |
| 어떤 API 응답 분기를 쓰는지 | [API 오류 처리 경로](error-routing.md) |
| 닫기 차단 사유가 API 응답과 만나는 지점 | [API 차단 사유 처리 경로](blocker-routing.md) |
| 오류를 설명하는 기계 판독 필드 | [API 오류 세부사항](error-details.md) |
| Core 전 MCP 인자를 거절하는 descriptor 유래 issue | [MCP 전송](../mcp-transport.md#public-argument-projection) |
| `volicord.close_task`가 만드는 메서드별 차단 사유 | [Task 닫기 메서드](method-close-task.md) |

## 가까운 경로

- 메서드 동작: [API 메서드](methods.md)에서 연결된 메서드 담당 문서.
- 공통 응답과 필수 `ToolError.category` 오류 래퍼 형태: [API 코어 스키마](schema-core.md).
- 상태와 차단 사유 형태: [API 상태 스키마](schema-state.md), [API 값 집합](schema-value-sets.md).
- 오류가 참조할 수 있는 Core 개념: [Core 모델](../core-model.md).
- 저장소 관심사: [저장소](../storage.md).
- 표시 문구만: [템플릿 본문](../template-bodies.md).
