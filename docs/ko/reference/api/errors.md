# API 오류

이 문서는 API 오류 묶음에서 집중 담당 문서를 찾기 위한 첫 경로입니다. 정확한 기계 판독 담당 문서 경로는 [`docs/doc-index.yaml`](../../../doc-index.yaml)을 사용합니다.

이 문서는 공개 오류 코드 의미, 오류 우선순위, 응답 분기 처리 경로, 닫기 준비 상태 차단 사유와 API 응답 사이의 경계, 기계 판독용 오류 세부사항, 렌더링 라벨, 저장 효과, 메서드별 결과 본문을 정의하지 않습니다.

## 오류 경로

| 질문 | 담당 문서 |
|---|---|
| 공개 `ErrorCode`가 무엇을 뜻하는지 | [API 오류 코드](error-codes.md) |
| 어떤 공개 오류가 선택되는지 | [API 오류 우선순위](error-precedence.md) |
| 어떤 API 응답 분기를 쓰는지 | [API 오류 처리 경로](error-routing.md) |
| 닫기 준비 상태 차단 사유가 API 응답과 만나는 지점 | [API 차단 사유 처리 경로](blocker-routing.md) |
| 오류를 설명하는 기계 판독 필드 | [API 오류 세부사항](error-details.md) |
| `harness.close_task`가 만드는 메서드별 차단 사유 | [Task 닫기 메서드](method-close-task.md) |

## 가까운 경로

- 메서드 동작: [API 메서드](methods.md)에서 연결된 메서드 담당 문서.
- 공통 응답과 오류 래퍼 형태: [API 코어 스키마](schema-core.md).
- 상태와 차단 사유 형태: [API 상태 스키마](schema-state.md), [API 값 집합](schema-value-sets.md).
- 오류가 참조할 수 있는 Core 개념: [Core 모델](../core-model.md).
- 저장소 관심사: [저장소](../storage.md).
- 표시 문구와 렌더링 라벨: [템플릿 본문](../template-bodies.md).
