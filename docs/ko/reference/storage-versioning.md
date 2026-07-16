# 저장소 버전 관리

이 문서는 현재 Volicord SQLite 저장소의 기준 버전 관리 규칙과 유일하게 지원되는 오프라인 v6-to-v7 복사 변환을 정의합니다. 공개 API 동작, Core 권한 의미, 보안 보장, 관리 명령 문법, 정책 파일 탐색, host 통합은 정의하지 않습니다.

## 저장소 프로필

현재 기준 저장소 프로필은 `baseline_sqlite_v7`입니다.

기준 저장소는 기준 SQL 원본인 [`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql)과 [`project.sql`](../../../crates/volicord-store/src/schema/project.sql)을 사용합니다. Runtime Home을 초기화할 때 이 원본을 빈 SQLite 데이터베이스에 적용합니다. `schema_migrations`, `schema_version`, `migration_version`, `storage_version` 같은 저장소 버전 필드나 테이블은 만들지 않습니다.

데이터베이스를 사용하려면 테이블, 열, 인덱스, 외래 키, 제약, 저장된 `storage_profile`이 현재 기준과 일치해야 합니다. 다음 조건은 저장소 또는 런타임을 사용할 수 없는 상태입니다.

- 이전 스키마 이력을 나타내는 알 수 없는 테이블이 있습니다.
- 필수 테이블이 없습니다.
- 금지된 저장소 버전 열이 있습니다.
- 저장소 프로필이 일치하지 않습니다.
- 필수 기록의 형식이 잘못되었습니다.

일반 Store open은 기록 의미를 추측하거나, 데이터를 알리지 않고 다시 쓰거나, 지원하지 않는 저장소를 변환하면 안 됩니다. v7은 v6를 호환되지 않는 것으로 거절합니다. 아래에서 명시적으로 실행하는 오프라인 복사 변환만 예외이며 v6 source를 변경 가능하게 열지 않습니다.

기준 `registry.sqlite`에는 Runtime Home 식별 정보, 설치 프로필, 저장소 루트 기반 프로젝트 등록, 프로젝트 별칭, Agent Connection, `connection_projects`, 변경 불가능한 호스트 역량 검증 이력, 현재 호스트 역량 포인터, `guard_installations`가 들어갑니다. 기준 프로젝트 `state.sqlite`에는 Core 상태 보기 기록, `authority_events`, 재실행 행, 스테이징·영속 아티팩트, 증거, evidence capture intent, receipt, 배타적 source claim, 불변 evidence producer, 사용자 행동 요청, 변경 불가능한 사용자 행동 해결, 요청 결속 로컬 채널 token, 실행 기록, 차단 사유, `write_tickets`, 호스트 관찰 기록, 세션 감시 기록이 들어갑니다.

`baseline_sqlite_v7`은 요청/유효 통제 수준 필드, 권위 있는
`volicord-policy-v2` 데이터베이스 복사본과 지문, 안정된 무효화 사유와 선택적 idle
timeout을 가진 재사용 가능한 상태 결합 쓰기 티켓, 미기록 변경 confidence,
`completion_claim_allowed`를 가진 session-end 권한 receipt를 추가합니다. 개인정보를
제한한 workflow metric은 별도의 비권한 진단 schema에 남으며 프로젝트 저장 profile
권한이 아닙니다.

`baseline_sqlite_v6`는 credential 전달 자격이 클라이언트 선언이나 변경 가능한 설정 검증
JSON이 아니라 변경 불가능하고 만료되며 정확한 프로필에 결속된 실제 호스트 증거에
의존하도록 레지스트리에 `host_capability_verifications`와 `host_capability_state`를
추가합니다. `baseline_sqlite_v5`에서 제자리 변환하는 경로는 제공하지 않습니다. v5 Runtime
Home은 호환되지 않는 형태이므로 다시 만들어야 합니다. Store는 기존 연결 상태를 다른
profile로 표시하거나, 통과 검증으로 추론하거나, 이력을 합성하면 안 됩니다.
V6/0.9.0 호스트 역량 형태에는 정확한 UTF-8 바이트 제약이 포함됩니다. 일반 자유 형식
이력·현재 포인터 좌표는 1~1,024바이트이고 관리 MCP `client_name`과 `client_version`은
1~256바이트입니다. V6 batch 안에서 이 제약을 완성해도 v7 전이를 만들지 않습니다. 정규
제약 없이 v6로 표시된 데이터베이스는 호환되지 않으며 다시 만들어야 합니다. Store는 그
값을 trim, truncate, 복구하거나 legacy 형태로 해석하면 안 됩니다.

이전 `baseline_sqlite_v5` 프로필은 v4의 판단 및 직접 사용자 관찰 family를
`user_action_requests`, 닫힌 tagged 관찰 해결 detail을 담는 변경 불가능한 일대일
`user_action_resolutions`, 요청 결속 로컬 채널 token으로 교체합니다. 기준 구현은
`baseline_sqlite_v4`에서 제자리 변환을 제공하지 않습니다. v4 Runtime Home은
호환되지 않는 형태이므로 다시 만들어야 하며 Store는 profile을 바꾸어 표시하거나
의미를 추측해 변환해서는 안 됩니다.
`project_state.state_version`은 계속 Core 상태 clock이며 storage-profile version이
아닙니다.

pre-major v5 계약은 등록 connection capture의 폐쇄형 source selector와 Core가
파생한 canonical selector digest를 intent에 저장합니다. 구체적인 event/watcher-observation
identity, observation time, raw-event 또는 snapshot/selection digest는 receipt 소유
사실입니다. 이 보정은 기준 SQL table, column, index, foreign key, constraint를 바꾸지
않고 `baseline_sqlite_v5` / `0.8.0` batch 안에서 완료되었습니다. 따라서 별도
storage-profile 또는 package-version 전이를 만들지 않습니다. Store는 제거된 호출자 제공
미래-observation-digest capture 형태를 legacy alias나 fallback으로 decode하지 않으며,
필수 record 형태가 잘못되면 닫힌 상태로 실패합니다.

<a id="canonical-core-utc-clock"></a>
## 정규 Core UTC 시계

정규 Core UTC 시계는 시간 권한 판단에 사용하는 프로젝트 범위의 감소하지 않는
UTC 시계입니다. `project_state.updated_at`은 이 시계의 영속 하한입니다. 물리 열
이름이 `updated_at`이더라도 `project_state` 행에서 이 값은 단순 표시용 metadata가
아닙니다.

현재 프로젝트 시각 샘플은 다음 모든 값보다 이르면 안 됩니다.

- `SystemClock`에서는 SQLite 현재 UTC인 구성된 실시간 시각 후보
- 영속 `project_state.updated_at` 하한
- 현재 Store handle이 해당 프로젝트에 대해 이미 받아들인 더 늦은 시각 샘플

기본 `SystemClock`은 SQLite 현재 UTC를 실시간 시각 원천으로 사용합니다. 주입 또는
custom Clock은 제어된 실행이나 테스트에서 이 실시간 원천을 대신할 수 있지만 영속
하한이나 같은 handle이 받아들인 샘플을 대신할 수는 없습니다. `CoreService` 시계 경계는
정규 프로젝트 시각을 노출하기 전에 모든 후보와 이 하한들의 최댓값을 취해 합성해야
합니다. 이 합성은 저장 행 timestamp를 현재 시각으로 다시 쓰지 않습니다. 미래 시각
행은 해당 timestamp 담당자가 그 값을 invalid로 정의한 경우에만 닫힌 상태로 실패합니다.
시계는 그 값을 정규화하지 않으며 다른 담당자에 새 거부 규칙을 추가하지 않습니다.

영속 하한과 여기에 비교하는 모든 timestamp에는 [저장소 기록](storage-records.md)이
정의한 정규 UTC timestamp 형식을 사용합니다. 저장된 하한의 형식이 잘못되면 담당
상태가 손상된 것입니다. Store는 값을 복구하거나 바꾸거나 되감지 말고 닫힌 상태로
실패해야 합니다.

각 공개 Core 동작은 공통 preflight를 마친 뒤 `operation_now` 샘플을 정확히 한 번
가져옵니다. 계획 단계는 동작 안의 모든 현재 시각 판단과 공개 동작 timestamp에 이
샘플을 다시 사용해야 합니다. 여기에는 만료와 유효 상태 판단, 파생 만료 시각 계산,
UserAction의 `created_at`, `requested_at`, `resolved_at` 값이 포함됩니다. 같은 동작
안에서 결과가 달라질 수 있는 별도 시계 샘플을 가져오면 안 됩니다.

담당 문서가 정의한 모든 TTL 또는 파생 expiry는 checked timestamp 덧셈을 사용하고 정규
RFC 3339 UTC 형식으로 표현할 수 있어야 합니다. 산술 overflow나 표현 불가능한 결과는
커밋 전 제어된 검증 거부이며 저장 효과가 없습니다. Store는 timestamp 열에 쓰기 전에
정규 형식을 다시 검증합니다. Core나 어댑터가 타입이 지정된 timestamp를 제공해도 이
저장소 경계를 우회하지 않습니다.

새 일반 Core 커밋은 immediate 쓰기 transaction 안에서 `committed_at` 하나를
선택합니다. 후보 집합은 구성된 Clock에 따라 달라집니다.

- Production `SystemClock`에서는 `operation_now`, transaction 안에서 샘플링한 SQLite
  현재 UTC, 영속 프로젝트 하한, 현재 Store handle이 이미 받아들인 더 늦은 프로젝트
  시각 샘플의 최댓값을 `committed_at`으로 선택합니다.
- 주입 또는 custom Clock에서는 `operation_now`, 해당 Clock이 주입한 실시간 시각 후보,
  영속 프로젝트 하한, 같은 handle이 받아들인 더 늦은 샘플의 최댓값을
  `committed_at`으로 선택합니다. 주입 후보는 transaction의 SQLite 실시간 시각 후보를
  대신하며 SQLite 현재 UTC를 두 번째 실시간 후보로 추가하지 않습니다.

어느 분기도 영속 하한이나 같은 handle 하한을 우회할 수 없습니다. Transaction은 정확히
같은 `committed_at` 값을 다음 위치에 씁니다.

- `project_state.updated_at`
- 커밋한 이벤트 또는 이벤트 묶음의 모든 `authority_events.created_at` 행
- 커밋이 재실행 행을 만들 때 `tool_invocations.created_at`
- 해당 커밋에서 mutation application 자체가 생성하는 모든 Store transaction
  metadata timestamp. 적용되는 `created_at`, `updated_at`, `retired_at`,
  `promoted_at` 값을 포함합니다.

`committed_at`은 `operation_now`보다 늦을 수 있습니다. 메서드 담당 문서가 준비
시각 샘플에서 파생하도록 정의한 공개 timestamp는 계속 `operation_now`입니다. 선택된
UTC 값이 같으면 여러 상태 버전이 같은 timestamp를 공유할 수 있습니다. 이 시계는
감소하지 않아야 하지만 모든 커밋마다 반드시 엄격하게 증가할 필요는 없습니다.
UTC 값은 시간 경계와 담당자가 정의한 시각을 나타내며 권한 커밋 순서나 최신 기록
선택을 대신하는 값으로 사용하면 안 됩니다.

의미 있는 동작 시각과 입력 또는 관찰 담당 사실은 자동 Store transaction metadata가
아닙니다. 담당 문서가 정의한 `requested_at`, `resolved_at`, `closed_at`,
`recorded_at`, `consumed_at`은 메서드 담당 규칙에 따라 준비된 `operation_now` 샘플
하나 또는 담당자가 검증한 관찰 시각을 보존합니다. `observed_at`, `started_at`도 원천이
보고한 관찰 또는 활동 시각을 보존합니다. Core 커밋은 단지 `committed_at`과 같게
만들기 위해 이 값을 덮어쓰면 안 됩니다.

`project_state.state_version`과 영속 UTC 하한은 서로 다른 시계입니다.

- `state_version`은 커밋된 권한 상태 전이의 순서를 정하고 공개 충돌 및 최신성
  근거를 제공합니다.
- UTC 하한은 이후의 시간 권한 판단이 프로젝트가 이미 받아들인 시각보다 이른
  프로젝트 시각을 관찰하지 못하게 합니다.

두 시계는 서로를 대신하지 않습니다. 일반 Core 권한 커밋은 `state_version`을
증가시키고 하한을 하나의 transaction에서 갱신합니다. 다음 저장소 소유 효과는
자신의 `created_at` 이상으로 하한을 갱신하지만 `state_version`을 증가시키거나 권한
이벤트 또는 재실행 행을 만들지 않습니다.

- 요청 결속 로컬 User Channel token 발급
- artifact staging 행 생성
- evidence capture receipt와 그 staging 및 source-claim 행 이행

이러한 하한 전용 효과는 보존할 timestamp를 가진 행 또는 행 집합과 원자적으로
커밋해야 합니다. 정확한 재실행, 거부된 요청, `dry_run=true` 계획, 읽기 전용 결과,
실패한 transaction은 영속 하한을 갱신하지 않습니다. 읽기는 더 늦은 현재 프로젝트
시각을 관찰할 수 있지만 그 값 자체를 영속화하지 않습니다.

새 프로젝트 등록은 저장소 엔진의 현재 UTC 시각으로 `project_state.created_at`과
`project_state.updated_at`을 초기화합니다. 기존 프로젝트를 다시 등록할 때는 담당
문서가 허용한 등록 metadata만 갱신하면서 기존 `updated_at` 값을 검증하고 정확히
보존해야 합니다. 저장 값이 현재 실시간 샘플보다 늦더라도 하한을 호스트 시각이나
저장소 시각으로 초기화하면 안 됩니다.

## 프로젝트 상태 버전

`project_state.state_version`은 커밋된 권한 상태 변경을 위한 프로젝트 전체 Core 상태 시계입니다. 스키마 버전, 마이그레이션 버전, 저장소 버전, 호환성 표시가 아닙니다.

담당 문서가 허용한 상태 변경 트랜잭션이 모두 커밋될 때만 증가합니다. 거부된 요청, `dry_run` 응답, 읽기 전용 결과, 시작 점검, 호스트 검증, 스키마 초기화, 저장소 프로필 검증, 잠금 획득, 상태 보기, 렌더링된 보고서, 실패한 트랜잭션에서는 증가하지 않습니다.

새 권한 변경이 커밋되면 현재 상태 보기 기록을 갱신하는 트랜잭션에서 영속 `authority_events` 행을 하나 이상 추가해야 합니다. 일반 변경은 권한 이벤트 하나를 추가합니다. 담당 문서가 이벤트 묶음을 명시하면 묶음의 모든 행이 해당 상태 전이로 생긴 하나의 `project_state.state_version`을 공유합니다.

`tasks.state_version`은 기준 권한 필드가 아닙니다. 기준에 없는 `tasks.state_version` 열은 잘못된 저장소 형태입니다. 충돌, 최신성, 잠금, 쓰기 티켓의 근거로 사용하면 안 됩니다.

관련 필드:

- `write_tickets.basis_state_version`은 발급 또는 재사용의 감사 순서를 저장합니다. 고유하지 않고 유효성 좌표도 아닙니다. 티켓 유효성은 명시적 Task, Change Unit, 범위 리비전, 기준선, workspace, 승인 근거, 소비/철회 상태, 선택적 idle timeout을 사용합니다.
- `evidence_summaries.produced_at_state_version`은 해당 요약을 가장 최근에 삽입하거나
  갱신한 커밋의 결과 `project_state.state_version`을 저장합니다. 현재 Evidence
  Summary를 선택할 때 이 필드를 내림차순으로 정렬하며 timestamp나 불투명 record
  ID를 tie-break 또는 대체값으로 사용하지 않습니다.
- `tool_invocations.basis_state_version`은 변경이 커밋되기 직전에 관찰한 프로젝트 전체 상태 버전을 저장합니다.
- `authority_events.state_version`은 권한 이벤트 또는 이벤트 묶음이 커밋된 뒤의 프로젝트 전체 상태 버전을 저장합니다.

## 쓰기 티켓

쓰기 티켓은 호환되는 승인 product-file 쓰기 의도에 대해 소비 전까지 재사용 가능한 Volicord 권한입니다. OS 권한, OS 샌드박스, 파일시스템 ACL, 네트워크 정책, 비밀값 격리, 전역 파일시스템 가로채기, 실제 쓰기가 일어났다는 증거가 아닙니다.

쓰기 티켓 발급과 호환되는 소비에는 일반 상태 버전 규칙이 적용됩니다.

- 담당 문서가 정의한 메서드 분기만 쓰기 티켓 발급을 커밋할 수 있습니다.
- prepare-write는 모든 유효성 좌표가 일치하고 기존 allowed prefix가 요청 prefix를 포함하며 denied prefix가 계속 적용되고 민감 권한이 같거나 더 강할 때 활성 미소비 티켓을 재사용할 수 있습니다.
- 실제 product-file 쓰기에서만 행이 활성, 호환, 미소비, 미철회, 미무효화이고 선택적으로 설정된 idle 경계 안일 때 소비를 커밋할 수 있습니다.
- 관련 없는 상태 버전 증가는 티켓을 무효화하지 않습니다. 명시적 무효화 사유는 `scope_revision_changed`, `change_unit_changed`, `baseline_changed`, `workspace_changed`, `approval_basis_changed`, `idle_timeout`, `task_closed`, `explicit_revoke`입니다.
- 거부, `dry_run`, 재실행 전용 분기에서는 발급하거나 소비하지 않습니다.

## 멱등성과 재실행

`tool_invocations`는 메서드 담당 문서가 재실행 행 생성을 허용한, 커밋된 `dry_run=false` Core `MethodResult` 응답만 그대로 저장합니다.

저장소 고유 키는 정확히 `(project_id, tool_name, idempotency_key)`입니다.
`request_hash`는 Core 소유 canonical 요청 identity의 충돌을 구분합니다. 일반적으로 이는
공개 요청 payload입니다. Token을 전달하는 local-web
`volicord.resolve_user_action`은 hashing 전에 domain-separated token digest, 예상 Agent
Connection, 타입이 지정된 canonical 완료 metadata도 결속합니다. 원문 token과 그 내부
binding 객체는 `tool_invocations`에 저장하지 않으며 결과 request hash와 응답만
영속적입니다. `actor_source`, `operation_category`, `verification_basis` 같은 다른 호출
맥락을 이 hash에 조용히 흡수하지 않습니다. Local-web binding의 예상 connection은
메서드가 의도적으로 소유하는 credential 맥락이며 별도의 검증된 replay-context 점검을
대체하지 않습니다.

새 재실행 행은 검증된 호출의 `actor_source`, `operation_category`, 정확한 선택적
`verification_basis`, 정확한 선택적 정규 Git 작업 공간 맥락을 저장합니다. 뒤의 두
좌표는 값뿐 아니라 값의 유무도 보존합니다. 현재 재실행 행은 저장된 네 좌표가 현재
검증된 재실행 맥락과 모두 정확히 일치할 때만 사용할 수 있습니다. 필수 재실행 식별
정보가 빠진 행은 호환성 상태 보기가 아니라 유효하지 않은 저장 상태입니다.

재실행 조건:

- 현재 호출의 맥락을 검증하기 전에는 저장된 응답을 반환하면 안 됩니다.
- Core는 요청 해시보다 호출 맥락의 호환성을 먼저 확인합니다.
- `verification_basis` 또는 Git 작업 공간 맥락이 달라지거나 한쪽에만 있는 경우를
  포함해 호출 맥락이 호환되지 않으면 `INVOCATION_CONTEXT_MISMATCH`를 반환하고 저장된
  응답을 노출하지 않습니다.
- 호출 맥락, `idempotency_key`, `request_hash`가 모두 같으면 처음 커밋해 저장한 응답을 그대로 반환합니다.
- 호출 맥락과 `idempotency_key`는 같지만 `request_hash`가 다르면 `STATE_VERSION_CONFLICT`를 반환합니다.

저장된 모든 공개 메서드 결과에는 preflight replay, commit transaction 안에서
발견한 replay, MCP resume 전에 전체 응답을 확인하는 추가 조건이 있습니다. 변경
불가능한 원문 JSON은 서로 다른 JSON escape 표기가 같은 이름으로 decode되는 경우를
포함해 어떤 중첩 단계에도 중복 object member가 없어야 하며, 일반 JSON tree로
normalize하기 전에 저장된 `method_name`이 선택하는 현재의 구체적이고 닫힌
`MethodResult` 타입으로 직접 strict decode되어야 합니다. Base는
`response_kind=result`, `effect_kind=core_committed`, `dry_run=false`, 정확한
`state_version=tool_invocations.committed_state_version`을 사용해야 합니다. Replay 행을
만들지 않는 메서드는 무조건 사용할 수 없습니다. 이 검사는 모든 중첩 `StateSummary`도
포함합니다. 요청 결과는 정확한 닫힌 세 필드
`AgentSafeUserActionRequestSummary`를 담아야 합니다. 닫기 결과는 현재
`pending_user_action_summaries` 필드를 담고 기존
`pending_user_action_inbox_items` 필드를 담지 않아야 합니다. 필수 필드가 없거나
잘못됐거나, 중복 member, 기존 full-form 필드, 알 수 없는 추가 필드, 일반 rejected 또는
dry-run 분기, 잘못된 메서드 형태, commit 좌표 불일치, 이전·새 형태 혼합이 있으면 그 행을
사용할 수 없습니다. 모든 replay 경로는 타입이 지정된 owner 상태 경계에서
`MCP_UNAVAILABLE`로 닫힌 상태로 실패하며 저장 byte를 반환하지 않습니다. Core는 기존
replay 행을 다시 쓰거나, redact하거나, upgrade하지 않습니다.

재실행은 저장된 응답 본문을 사용합니다. `write_ticket_effect`, `base.state_version`, `base.events`나 다른 응답 필드를 다시 계산하거나 분류하지 않습니다. 이벤트나 재실행 행을 추가하지 않고, 아티팩트를 승격하거나 연결하지 않으며, 쓰기 티켓을 발급하거나 소비하지 않고, 상태를 다시 변경하지 않습니다.

<a id="exact-operation-result-retrieval"></a>
### 정확한 동작 결과 조회

조회할 수 있는 모든 `operation_category=agent_workflow` Core 커밋과 정확한
재실행은 원래 커밋이 이미 저장한 변경 불가능한
`tool_invocations.response_json`을 가리키는 `OperationResultRef`를 노출합니다.
`volicord.get_operation_result`는 그 값을 연속된 UTF-8 안전 페이지로 읽습니다.
`cursor` 순서대로 페이지를 이어 붙이면 저장 응답과 바이트 단위로 정확히 같아야
하며, 조회 중 어떤 필드도 다시 계산, 정규화,
재직렬화, 재분류하면 안 됩니다.

페이지를 하나라도 반환하기 전에 Store는 정확한 저장 byte를 읽고 byte 길이와
SHA-256을 계산하며, Core는 그 사실을 `OperationResultRef`와 대조합니다. 현재 검증된
행위자와 프로젝트 접근은 보안 및 메서드 담당
문서에 따라 별도로 확인하며 참조는 베어러 자격 증명이 아닙니다.
모든 저장 메서드에 대해 Core는 접근 맥락 검증 뒤 첫 page를 반환하기 전에 원문 전체
응답의 committed-result 조건도 적용합니다. 사용할 수 없는 중복 member, non-result,
기존 형태, 잘못된 메서드, commit 좌표 불일치 또는 혼합 형태 행은
`OPERATION_RESULT_UNAVAILABLE`을 반환하며 일부 page도 반환하지 않습니다.
`volicord.resolve_user_action`을 포함한 `operation_category=user_only` 행은
Agent Connection 조회 대상이 아닙니다. 조회한 응답은 과거 결과이며 현재
`volicord.status` 조회를 대신하지 않습니다.

이 경로는 기존 재실행 행과 변경 불가능한 `response_json`을 재사용합니다. 새
테이블, 열, 재실행 형태, 스키마 이력, 저장소 프로필, 마이그레이션을 추가하지
않습니다. 조회 자체는 읽기 전용이며 재실행 행, 이벤트, 잠금에서 보이는 상태
전이, `project_state.state_version` 증가를 만들지 않습니다.

`volicord.stage_artifact`는 계속 Core 재실행 트랜잭션 밖에 있습니다. 재실행
행이나 `OperationResultRef`를 만들지 않으며, 스테이징 효과가 일어나기 전에 전체
직렬화 결과가 지원되는 예상 크기 상한을 만족해야 합니다.

## 오프라인 v6-to-v7 복사 변환

일반 v7 open은 v6를 거절합니다. 오프라인 converter는 v6 Runtime Home과 모든 source
database를 읽기 전용으로 열고 전체 v6 형태와 타입이 지정된 owner 상태를 검증한 뒤,
기준 DDL로 별도의 빈 v7 destination을 만들고 transaction 안에서 변환한 기록을
복사합니다. Source를 relabel하거나 바꾸지 않고, table을 in-place 갱신하지 않으며,
변환의 일부로 destination을 활성화하지 않습니다.

변환은 project, Task, Change Unit, Run, judgment, Evidence, artifact, blocker, event,
replay 식별자, 사용자 권한, 잔여 위험 결정, 기준 Evidence/judgment hash, 영속 관계를
보존합니다. 기존 `advisor`는 `observe`, `direct`와 `work`는 보수적으로 `tracked`로
매핑합니다. 기존 acceptance outcome은 보존합니다. 초기 v2 정책 복사본은 보수적인
tracked 기본값을 사용합니다. v6 사실이 변경을 결정적으로 확정할 때만 관찰 기반
confidence를 두 영역에서 따로 변환합니다. 복사한 `unrecorded_changes` 행은 v6 사실이
제품 변경을 결정적으로 확정할 때만 `UnrecordedChangeConfidence::Confirmed`, 그 밖에는
`Suspected`를 사용합니다. 복사한 기존 Detective 평가의 `guard_events.result_json`은 v6
source 사실이 해당 수준을 입증할 때만 `ObservationConfidence::Confirmed` 또는
`Structured`를 사용하고 그 밖에는 `Heuristic`으로 표시합니다. 두 영역은 서로의 값
집합을 빌려 쓰지 않습니다. 활성 v6 쓰기 티켓은
모두 `invalidation_reason=explicit_revoke`인 revoked 상태로 복사하고 소비된 티켓/Run
연결은 보존합니다.

성공을 보고하기 전에 converter는 foreign key, table과 대상 row count, 식별자 보존,
기준 JSON, 정책/record fingerprint, Evidence/judgment hash, 티켓 변환, source 불변성을
검증하고 크기가 제한된 변환 report를 만듭니다. 실패하면 source는 그대로이고
destination은 수락되지 않습니다. 일부 output을 성공한 v7 store로 취급하면 안 됩니다.
활성화는 별도 관리 동작입니다.

## 실패와 재시도

커밋 전 실패에는 저장 효과가 없습니다. 트랜잭션 실패는 상태 버전 증가, 정규 하한 갱신, 이벤트, 재실행 행, 쓰기 티켓 변경, 아티팩트 효과, 증거 갱신, 사용자 행동 요청 또는 해결 효과, 닫기 효과, 생명주기 효과, 스테이징 핸들 소비 중 일부만 남기면 안 됩니다.

예:

- 오래된 `expected_state_version`
- 소비 시도에서 유효하지 않거나 상태 결합 비호환인 쓰기 티켓
- 검증 실패
- 잘못된 요청
- 타입이 지정된 담당 상태의 손상
- 멱등성 요청 해시 충돌
- 호출 맥락 불일치
- 호환되지 않는 기존 저장소 형태

재시도 방법은 거부 사유에 따라 정합니다. 상태 버전이 오래됐으면 상태를 새로 읽습니다. 입력 검증에 실패했으면 입력을 고칩니다. 대기 사용자 행동이 있으면 User Channel을 사용합니다. 쓰기 호환성이 필요하면 정해진 쓰기 티켓 절차를 따릅니다. 저장소가 호환되지 않으면 Runtime Home을 다시 만듭니다.

## 담당 문서

- 기록 계열 개요와 저장소 소유 값: [저장소 기록](storage-records.md)
- SQLite DDL, 제약, 인덱스, 외래 키: [저장소 DDL](storage-ddl.md)
- 메서드별 저장 효과: [저장 효과](storage-effects.md)
- 공개 충돌 동작: [API 오류 우선순위](api/error-precedence.md#state-conflict-behavior)
- 공개 호출 맥락 불일치 코드: [API 오류 코드](api/error-codes.md#errorcode-invocation-context-mismatch)
- Runtime Home 분리: [런타임 경계](runtime-boundaries.md)
