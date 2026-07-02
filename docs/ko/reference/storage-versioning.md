# 저장소 버전 관리

이 문서는 현재 Volicord SQLite 저장소의 기준 저장소 버전 관리 규칙을 담당합니다. 공개 API 동작, Core 권한 의미, 보안 보장, 지원 기준 밖 migration 동작을 정의하지 않습니다.

## 저장소 프로필

현재 기준 저장소 프로필은 `baseline_sqlite_v3`입니다.

Registry 저장소와 project-state 저장소는 각각 migration ledger 행을 기록합니다. 데이터베이스는 schema version, migration name, database kind, storage profile이 컴파일된 기준과 일치할 때만 current입니다. 알 수 없는 더 최신 version, 누락된 migration 행, 부분 ledger, migration name mismatch, storage-profile mismatch는 storage/runtime unavailable 조건입니다. Store 코드는 기록 의미를 추측하거나, 데이터를 조용히 다시 쓰거나, 지원되지 않는 profile을 변환하면 안 됩니다.

기준 registry 저장소는 Runtime Home 식별 정보, 설치 프로필 기록, 저장소 루트 기반 프로젝트
등록, 프로젝트 alias, Agent Connection 기록, `connection_projects`, `guard_installations`를
포함합니다. 기준 project-state 저장소는 Core 상태 projection 기록, `authority_events`,
replay 행, staged artifact, persistent artifact, evidence, user judgment,
`local_web_consent_tokens`, run, blocker, `write_tickets`, host-observation 기록,
session-watch 기록을 포함합니다.

## Project State Version

`project_state.state_version`은 커밋된 권한 상태 변경을 위한 project-wide Core state clock입니다.

완전한 owner-allowed state-changing transaction이 commit될 때만 증가합니다. rejected request, dry-run response, read-only result, startup check, host verification, migration metadata, lock acquisition, status projection, rendered report, failed transaction에서는 증가하지 않습니다.

새로 커밋된 권한 mutation은 현재 projection update와 같은 트랜잭션 안에서 영속 `authority_events` 행을 적어도 하나 추가합니다. 일반 커밋 mutation은 권한 이벤트를 정확히 하나 추가합니다. 담당 문서가 이벤트 배치를 명시적으로 정의하면, 그 배치의 모든 행은 해당 커밋 상태 전이의 단일 결과 `project_state.state_version`을 공유합니다.

`tasks.state_version`은 기준 권한 필드가 아닙니다. 기준 밖 `tasks.state_version` 열은 무시되는 metadata일 뿐이며 conflict, freshness, lock, 쓰기 티켓 basis로 사용하면 안 됩니다.

관련 필드:

- `write_tickets.basis_state_version`은 쓰기 티켓 발급 commit 뒤 결과 `project_state.state_version`을 저장합니다. Core는 이를 나중의 쓰기 티켓 소비 freshness basis로 사용합니다.
- `tool_invocations.basis_state_version`은 commit된 mutation 전에 관찰한 project-wide state version을 저장합니다.
- `authority_events.state_version`은 commit된 권한 이벤트 또는 이벤트 배치 뒤 결과 project-wide version을 저장합니다.

## 쓰기 티켓

쓰기 티켓은 제안된 제품 파일 쓰기 시도 하나에 대한 권한 있는 쓰기 의도를 나타내는 Volicord 권한입니다. OS 권한, OS 샌드박싱, 파일시스템 ACL, 네트워크 정책, 비밀 격리, 전역 파일시스템 가로채기, 쓰기가 실제로 일어났다는 증명이 아닙니다.

쓰기 티켓 발급과 호환성 소비는 일반 state-version 규칙을 따릅니다.

- 발급은 owner-defined method branch를 통해서만 commit될 수 있습니다.
- 소비는 저장된 물리 `write_tickets` 행이 해당 쓰기 티켓에 대해 active, compatible, unexpired, unconsumed이고 project state basis에 대해 current일 때만 commit될 수 있습니다.
- 오래된 `WriteTicket.basis_state_version`은 소비 전에 거절됩니다.
- rejected, dry-run, replay-only branch에서는 발급이나 소비가 일어나지 않습니다.

## Idempotency And Replay

`tool_invocations`는 method owner가 replay row를 만드는, commit된 `dry_run=false` Core `MethodResult` 응답의 정확한 replay만 저장합니다.

저장소 고유 키는 정확히 `(project_id, tool_name, idempotency_key)`입니다. `request_hash`는 공개 request payload의 conflict discriminator입니다. `actor_source`, `operation_category`, `connection_id`, `verification_basis` 같은 invocation context를 흡수하지 않습니다.

새 replay row는 확인된 호출 맥락에서 온 완전하고 null이 아닌 `actor_source`와 `operation_category`를 저장합니다. current replay row는 완전히 일치하는 `actor_source`와 `operation_category`를 요구합니다. 필요한 replay identity가 빠진 행은 compatibility projection이 아니라 invalid stored state입니다.

Replay eligibility:

- 현재 호출에 verified invocation context가 생기기 전에는 stored response를 반환하면 안 됩니다.
- Core는 request-hash compatibility보다 invocation-context compatibility를 먼저 확인합니다.
- context가 호환되지 않으면 `INVOCATION_CONTEXT_MISMATCH`를 반환하고 stored response를 노출하면 안 됩니다.
- 호환되는 context와 같은 `idempotency_key`, 같은 `request_hash`는 저장된 원래 commit response를 그대로 반환합니다.
- 호환되는 context와 같은 `idempotency_key`, 다른 `request_hash`는 `STATE_VERSION_CONFLICT`를 반환합니다.

Replay는 stored response body를 사용합니다. `write_ticket_effect`, `base.state_version`, `base.events`나 다른 response field를 다시 계산하거나 재분류하지 않습니다. Replay는 event를 추가하거나, artifact를 promote/link하거나, 쓰기 티켓을 발급하거나 소비하거나, 다른 replay row를 만들거나, state를 다시 변경하지 않습니다.

## Failure And Retry

Pre-commit failure에는 storage effect가 없습니다. Transaction failure는 state-version increment, event, replay row, 쓰기 티켓 change, artifact effect, evidence update, judgment effect, close effect, lifecycle effect, staged-handle consumption의 부분 상태를 남기면 안 됩니다.

예:

- 오래된 `expected_state_version`
- 오래된 `WriteTicket.basis_state_version`
- validation failure
- malformed request
- corrupt typed owner state
- idempotency request-hash conflict
- invocation-context mismatch

Retry는 rejected reason을 따릅니다. 오래된 version conflict는 state를 refresh하고, validation failure는 input을 고치며, 빠진 user judgment는 User Channel을 사용하고, write compatibility가 여전히 필요하면 필요한 쓰기 티켓 흐름을 사용합니다.

## Migration Boundary

Migration semantics는 지원되는 storage profile 또는 schema-version 변경이 Core authority record를 어떻게 보존하는지 설명합니다. 지원되는 migration execution은 [범위](scope.md), [저장소 기록](storage-records.md), [저장소 DDL](storage-ddl.md), 이 문서가 version, storage profile, validation, preservation, repair, retry, metadata-advance behavior를 정의할 때만 존재합니다.

Migration은 focused owner가 명시적으로 그 effect를 정의하지 않는 한 공개 `project_state.state_version` increment, Core event, replay record, public method effect를 만들지 않습니다.

## 담당 문서 링크

- 기록 계열 overview와 저장소 소유 값: [저장소 기록](storage-records.md)
- SQLite DDL, constraint, index, foreign key, migration table shape: [저장소 DDL](storage-ddl.md)
- Method storage effect: [저장 효과](storage-effects.md)
- 공개 conflict 동작: [API 오류 우선순위](api/error-precedence.md#state-conflict-behavior)
- 공개 invocation-context mismatch 코드: [API 오류 코드](api/error-codes.md#errorcode-invocation-context-mismatch)
- Runtime Home 분리: [런타임 경계](runtime-boundaries.md)
