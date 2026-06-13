# 저장소

이 문서는 저장소 참조 문서 묶음으로 들어가는 경로 문서입니다. 기록 배치, 분기별 저장 효과, 아티팩트 생명주기, 버전 관리, API 형태, 보안 보장, 런타임 위치를 직접 정의하지 않고 담당 문서로 보냅니다.

## 저장소 담당 문서 경로

| 필요 | 담당 문서 |
|---|---|
| 영속 기록 배치와 저장소 소유 값 | [저장소 기록](storage-records.md) |
| 메서드 분기 저장 효과와 API 형태/효과 구분 | [저장 효과](storage-effects.md) |
| 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 | [아티팩트 저장소](storage-artifacts.md) |
| `project_state.state_version`, 멱등성, 재실행, 이벤트, 잠금, 마이그레이션 | [저장소 버전 관리](storage-versioning.md) |
| Product Repository, Harness Server, Runtime Home 위치 | [런타임 경계](runtime-boundaries.md) |

저장소가 아닌 계약은 필요에 따라 [API 메서드](api/methods.md), API 스키마 담당 문서, [Core 모델](core-model.md), [보안](security.md), 또는 관심사별 API 오류 담당 문서로 이동합니다. API 오류 개념은 [API 오류 코드](api/error-codes.md), [API 오류 우선순위](api/error-precedence.md), [API 오류 처리 경로](api/error-routing.md), [API 차단 사유 처리 경로](api/blocker-routing.md), [API 오류 세부사항](api/error-details.md) 중 맞는 담당 문서를 사용합니다. [API 오류 문서 묶음 색인](api/errors.md)은 문서 묶음 탐색에만 사용합니다.
