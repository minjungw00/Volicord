# 저장소

이 문서는 저장소 질문에 맞는 집중 저장소 참조 문서를 찾기 위한 경로 안내입니다. 정확한 저장소 계약은 연결된 저장소 담당 문서에 있습니다.

아래에서 owner routing을 위해 현재 canonical manifest identity를 반복하는 것 외에는
저장 레코드 배치, SQLite DDL, 저장 효과, 아티팩트 생명주기, API 형태, 보안 보장,
runtime 위치, Core 권한 의미를 정의하지 않습니다.

## 현재 canonical manifest identity

```schema
contract_id: volicord.sqlite.canonical
enabled_capabilities:
  - artifact_storage
  - authority_event_chain
  - exact_operation_result
  - guard_reconciliation
  - managed_codex_connection
  - operational_mcp_sessions
  - project_continuity
  - user_action_cli_resolution
```

값과 순서는 정확합니다. 알 수 없거나 누락·재정렬된 값, subset, 현재가 아닌 값은
유효하지 않으며 default나 conversion은 없습니다.
[Storage Versioning](storage-versioning.md#storagemanifest)이 전체 형태, digest, 검증,
실패 분류를 소유합니다.

## 저장소 경로

| 필요 | 담당 문서 |
|---|---|
| 기록과 저장소 소유 값 | [저장소 기록](storage-records.md) |
| 기준 SQLite 테이블 형태, 인덱스, 외래 키, 제약, 기준 SQL 원본 | [저장소 DDL](storage-ddl.md) |
| 메서드나 분기별 저장 효과 | [저장 효과](storage-effects.md) |
| 아티팩트 저장소 생명주기 | [아티팩트 저장소](storage-artifacts.md) |
| 상태 버전 시계, 재실행, 잠금, 호환되지 않는 저장소 처리 | [저장소 버전 관리](storage-versioning.md) |
| 단일 기준 SQLite 계약, 정확한 매니페스트 검증, 지원하지 않는 형식 거절 | [저장소 버전 관리](storage-versioning.md) |
| 프로젝트 정책 복사본, 통제 수준 필드, 상태 결합 티켓 기록, 별도의 비권한 workflow metric | [저장소 기록](storage-records.md), [저장소 DDL](storage-ddl.md) |
| 런타임과 제품 저장소 위치 경계 | [런타임 경계](runtime-boundaries.md) |

## 가까운 경로

- API 메서드 동작: [API 메서드](api/methods.md)에서 연결된 메서드 담당 문서.
- API 스키마 형태: [API 코어 스키마](api/schema-core.md)와 같은 API 스키마 담당 문서.
- Core 권한 개념: [Core 모델](core-model.md).
- 보안 표현과 보장 의미: [보안](security.md).
- API 오류 묶음: [API 오류](api/errors.md).
- 정책 적용을 위한 관리 명령, 파일, 호스트 통합: [관리 CLI](admin-cli.md).
