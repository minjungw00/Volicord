# 저장소 버전 관리

이 문서는 현재 Volicord SQLite 저장소의 기준 버전 관리 규칙을 정의합니다. 공개 API 동작, Core 권한 의미, 보안 보장, 스키마 변환 절차, 이전 Runtime Home의 호환성 변환은 정의하지 않습니다.

## 저장소 프로필

현재 기준 저장소 프로필은 `baseline_sqlite_v3`입니다.

기준 저장소는 기준 SQL 원본인 [`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql)과 [`project.sql`](../../../crates/volicord-store/src/schema/project.sql)을 사용합니다. Runtime Home을 초기화할 때 이 원본을 빈 SQLite 데이터베이스에 적용합니다. `schema_migrations`, `schema_version`, `migration_version`, `storage_version` 같은 저장소 버전 필드나 테이블은 만들지 않습니다.

데이터베이스를 사용하려면 테이블, 열, 인덱스, 외래 키, 제약, 저장된 `storage_profile`이 현재 기준과 일치해야 합니다. 다음 조건은 저장소 또는 런타임을 사용할 수 없는 상태입니다.

- 이전 스키마 이력을 나타내는 알 수 없는 테이블이 있습니다.
- 필수 테이블이 없습니다.
- 금지된 저장소 버전 열이 있습니다.
- 저장소 프로필이 일치하지 않습니다.
- 필수 기록의 형식이 잘못되었습니다.

저장소 코드는 기록의 의미를 추측하거나, 데이터를 알리지 않고 다시 쓰거나, 지원하지 않는 저장소를 변환하면 안 됩니다. 기존 Runtime Home의 저장소가 호환되지 않으면 명확한 오류를 반환하고 Runtime Home을 다시 만들도록 요구해야 합니다.

기준 `registry.sqlite`에는 Runtime Home 식별 정보, 설치 프로필, 저장소 루트 기반 프로젝트 등록, 프로젝트 별칭, Agent Connection, `connection_projects`, `guard_installations`가 들어갑니다. 기준 프로젝트 `state.sqlite`에는 Core 상태 보기 기록, `authority_events`, 재실행 행, 스테이징·영속 아티팩트, 증거, 사용자 판단, `local_web_consent_tokens`, 실행 기록, 차단 사유, `write_tickets`, 호스트 관찰 기록, 세션 감시 기록이 들어갑니다.

## 프로젝트 상태 버전

`project_state.state_version`은 커밋된 권한 상태 변경을 위한 프로젝트 전체 Core 상태 시계입니다. 스키마 버전, 마이그레이션 버전, 저장소 버전, 호환성 표시가 아닙니다.

담당 문서가 허용한 상태 변경 트랜잭션이 모두 커밋될 때만 증가합니다. 거부된 요청, `dry_run` 응답, 읽기 전용 결과, 시작 점검, 호스트 검증, 스키마 초기화, 저장소 프로필 검증, 잠금 획득, 상태 보기, 렌더링된 보고서, 실패한 트랜잭션에서는 증가하지 않습니다.

새 권한 변경이 커밋되면 현재 상태 보기 기록을 갱신하는 트랜잭션에서 영속 `authority_events` 행을 하나 이상 추가해야 합니다. 일반 변경은 권한 이벤트 하나를 추가합니다. 담당 문서가 이벤트 묶음을 명시하면 묶음의 모든 행이 해당 상태 전이로 생긴 하나의 `project_state.state_version`을 공유합니다.

`tasks.state_version`은 기준 권한 필드가 아닙니다. 기준에 없는 `tasks.state_version` 열은 잘못된 저장소 형태입니다. 충돌, 최신성, 잠금, 쓰기 티켓의 근거로 사용하면 안 됩니다.

관련 필드:

- `write_tickets.basis_state_version`은 쓰기 티켓 발급이 커밋된 뒤의 `project_state.state_version`을 저장합니다. Core는 나중에 쓰기 티켓을 소비할 때 이 값을 최신성 근거로 사용합니다.
- `tool_invocations.basis_state_version`은 변경이 커밋되기 직전에 관찰한 프로젝트 전체 상태 버전을 저장합니다.
- `authority_events.state_version`은 권한 이벤트 또는 이벤트 묶음이 커밋된 뒤의 프로젝트 전체 상태 버전을 저장합니다.

## 쓰기 티켓

쓰기 티켓은 제안된 제품 파일 쓰기 시도 하나에 대한 권한 있는 쓰기 의도를 기록하는 Volicord 권한입니다. OS 권한, OS 샌드박스, 파일시스템 ACL, 네트워크 정책, 비밀값 격리, 전역 파일시스템 가로채기, 실제 쓰기가 일어났다는 증거가 아닙니다.

쓰기 티켓 발급과 호환되는 소비에는 일반 상태 버전 규칙이 적용됩니다.

- 담당 문서가 정의한 메서드 분기만 쓰기 티켓 발급을 커밋할 수 있습니다.
- 저장된 `write_tickets` 행이 `active` 상태이고, 호환되며, 만료되거나 소비되지 않았고, 현재 프로젝트 상태 근거와 맞을 때만 소비를 커밋할 수 있습니다.
- 오래된 `WriteTicket.basis_state_version`은 소비 전에 거부합니다.
- 거부, `dry_run`, 재실행 전용 분기에서는 발급하거나 소비하지 않습니다.

## 멱등성과 재실행

`tool_invocations`는 메서드 담당 문서가 재실행 행 생성을 허용한, 커밋된 `dry_run=false` Core `MethodResult` 응답만 그대로 저장합니다.

저장소 고유 키는 `(project_id, tool_name, idempotency_key)`입니다. `request_hash`는 공개 요청 본문의 충돌을 구분합니다. `actor_source`, `operation_category`, `connection_id`, `verification_basis` 같은 호출 맥락은 해시에 포함하지 않습니다.

새 재실행 행은 검증된 호출 맥락의 `actor_source`와 `operation_category`를 빠짐없이 `NULL`이 아닌 값으로 저장합니다. 현재 재실행 행을 사용하려면 두 값이 모두 현재 호출과 일치해야 합니다. 필수 재실행 식별 정보가 빠진 행은 호환성 상태 보기가 아니라 유효하지 않은 저장 상태입니다.

재실행 조건:

- 현재 호출의 맥락을 검증하기 전에는 저장된 응답을 반환하면 안 됩니다.
- Core는 요청 해시보다 호출 맥락의 호환성을 먼저 확인합니다.
- 호출 맥락이 호환되지 않으면 `INVOCATION_CONTEXT_MISMATCH`를 반환하고 저장된 응답을 노출하지 않습니다.
- 호출 맥락, `idempotency_key`, `request_hash`가 모두 같으면 처음 커밋해 저장한 응답을 그대로 반환합니다.
- 호출 맥락과 `idempotency_key`는 같지만 `request_hash`가 다르면 `STATE_VERSION_CONFLICT`를 반환합니다.

재실행은 저장된 응답 본문을 사용합니다. `write_ticket_effect`, `base.state_version`, `base.events`나 다른 응답 필드를 다시 계산하거나 분류하지 않습니다. 이벤트나 재실행 행을 추가하지 않고, 아티팩트를 승격하거나 연결하지 않으며, 쓰기 티켓을 발급하거나 소비하지 않고, 상태를 다시 변경하지 않습니다.

## 실패와 재시도

커밋 전 실패에는 저장 효과가 없습니다. 트랜잭션 실패는 상태 버전 증가, 이벤트, 재실행 행, 쓰기 티켓 변경, 아티팩트 효과, 증거 갱신, 판단 효과, 닫기 효과, 생명주기 효과, 스테이징 핸들 소비 중 일부만 남기면 안 됩니다.

예:

- 오래된 `expected_state_version`
- 오래된 `WriteTicket.basis_state_version`
- 검증 실패
- 잘못된 요청
- 손상된 타입 지정 담당 상태
- 멱등성 요청 해시 충돌
- 호출 맥락 불일치
- 호환되지 않는 기존 저장소 형태

재시도 방법은 거부 사유에 따라 정합니다. 상태 버전이 오래됐으면 상태를 새로 읽습니다. 입력 검증에 실패했으면 입력을 고칩니다. 사용자 판단이 없으면 User Channel을 사용합니다. 쓰기 호환성이 필요하면 정해진 쓰기 티켓 절차를 따릅니다. 저장소가 호환되지 않으면 Runtime Home을 다시 만듭니다.

## 담당 문서

- 기록 계열 개요와 저장소 소유 값: [저장소 기록](storage-records.md)
- SQLite DDL, 제약, 인덱스, 외래 키: [저장소 DDL](storage-ddl.md)
- 메서드별 저장 효과: [저장 효과](storage-effects.md)
- 공개 충돌 동작: [API 오류 우선순위](api/error-precedence.md#state-conflict-behavior)
- 공개 호출 맥락 불일치 코드: [API 오류 코드](api/error-codes.md#errorcode-invocation-context-mismatch)
- Runtime Home 분리: [런타임 경계](runtime-boundaries.md)
