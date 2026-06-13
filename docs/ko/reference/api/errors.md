# API 오류

이 문서는 API 오류 참조 색인입니다. 공개 오류 코드, 우선순위, 경로, 기계 판독용 세부사항 질문을 집중 오류 담당 문서로 안내합니다.

렌더링 라벨, 메시지 문구, 템플릿, 저장소 행, 런타임 출력, 메서드별 결과 본문은 정의하지 않습니다.

## 오류 담당 문서

| 질문 | 담당 문서 |
|---|---|
| 공개 `ErrorCode` 식별자, 의미, 발생 위치 요약 | [API 오류 코드](error-codes.md) |
| 주 공개 오류 선택, 우선순위, 오래된 상태 충돌, 멱등성 충돌 동작 | [API 오류 우선순위](error-precedence.md) |
| 거부 응답, 차단 결과, `dry_run` 미리보기, 금지된 차단 사유 코드 사용, `close_task` 차단 사유 매핑 | [API 오류 경로](error-routing.md) |
| `ToolError.details`, 세부 필드, 보조 값, 기계 판독용 세부사항 제약 | [API 오류 세부사항](error-details.md) |

## 관련 담당 문서

- 메서드 요청 본문 스키마, 응답 분기 형태, 공통 요청/응답 래퍼: [API 코어 스키마](schema-core.md), [API 메서드](methods.md)가 안내하는 메서드 담당 문서, API 스키마 담당 문서.
- Core 권한 확인, 사용자 소유 판단 의미, 닫기 준비 상태 의미: [Core 모델](../core-model.md), [사용자 판단 메서드](method-user-judgment.md), [Task 닫기 메서드](method-close-task.md).
- `CloseReadinessBlocker`, `WriteDecisionReason`, `PlannedBlocker`, 값 집합 필드 정의: [API 상태 스키마](schema-state.md), [API 코어 스키마](schema-core.md), [API 값 집합](schema-value-sets.md).
- 저장소 행, 재실행 행, DDL, 잠금, 마이그레이션, 저장 효과: [저장소 기록](../storage-records.md), [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
- 보안 보장 표현과 접근 경계 주장: [보안](../security.md).
- 사용자 표시 라벨, 렌더링 메시지 문구, 템플릿 표현: [템플릿 본문](../template-bodies.md).
