# 저장소

이 문서는 저장소 질문에 맞는 집중 저장소 참조 문서를 찾기 위한 경로 안내입니다. 정확한 저장소 계약은 연결된 저장소 담당 문서에 있습니다.

이 문서는 저장소 기록 배치, SQLite DDL, 저장 효과, 아티팩트 생명주기, 버전 관리, API 형태, 보안 보장, 런타임 위치, Core 권한 의미를 정의하지 않습니다.

## 저장소 경로

| 필요 | 담당 문서 |
|---|---|
| 기록과 저장소 소유 값 | [저장소 기록](storage-records.md) |
| 기준 SQLite 테이블 형태, 인덱스, 외래 키, 제약, canonical SQL 원본 | [저장소 DDL](storage-ddl.md) |
| 메서드나 분기별 저장 효과 | [저장 효과](storage-effects.md) |
| 아티팩트 저장소 생명주기 | [아티팩트 저장소](storage-artifacts.md) |
| 상태 버전 시계, 재실행, 잠금, 호환되지 않는 저장소 처리 | [저장소 버전 관리](storage-versioning.md) |
| 런타임과 제품 저장소 위치 경계 | [런타임 경계](runtime-boundaries.md) |

## 가까운 경로

- API 메서드 동작: [API 메서드](api/methods.md)에서 연결된 메서드 담당 문서.
- API 스키마 형태: [API 코어 스키마](api/schema-core.md)와 같은 API 스키마 담당 문서.
- Core 권한 개념: [Core 모델](core-model.md).
- 보안 표현과 보장 의미: [보안](security.md).
- API 오류 묶음: [API 오류](api/errors.md).
