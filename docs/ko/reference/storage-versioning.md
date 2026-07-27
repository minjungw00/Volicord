# 저장소 버전 관리

이 문서는 현재 Volicord SQLite 저장소 계약을 담당합니다. 여기에는 매니페스트
정체성, 정확한 데이터베이스 열기 검증, 기준 스키마 메타데이터, 프로젝트 상태
시계, 원자적 변경 경계, 멱등성, 정확한 재실행이 포함됩니다.

물리 테이블이나 열 정의, 공개 API 동작, 기록 계열 의미, 메서드별 저장 효과,
아티팩트 생명주기, Runtime Home 배치, 보안 보장은 담당하지 않습니다. 정확한
SQLite DDL은 [저장소 DDL](storage-ddl.md)이 담당합니다.

<a id="surface-stability"></a>
## 표면 안정성

안정성 어휘는
[문서 정책](../maintain/documentation-policy.md#surface-stability-labels)을
확인하세요.

| 표면 | 안정성 | 계약 |
|---|---|---|
| `StorageManifest`, 기준 SQL 정체성, 정확한 열기 검증, `GeneratedSchemaMetadata` | `stable` | 유일하게 허용되는 SQLite 형식을 식별합니다. |
| `project_state.state_version`, 정규 Core UTC 시계, 원자적 권한 커밋, 정확한 재실행 | `stable` | 받아들인 형식 안에서 권한, 최신성, 멱등성 계약으로 유지됩니다. |
| 매니페스트 배치, 생성된 Rust 모듈, 메타데이터 추출 도우미, 질의 구현 | `internal` | 생성된 정확한 사실과 열기 동작을 보존하면서 구현을 바꿀 수 있습니다. |
| 크기가 제한된 저장소 열기 및 손상 진단 | `diagnostic` | 진단 문구는 호환성 정체성이 아니며 원본 담당 상태, SQL 텍스트, 비밀값, 민감한 절대 경로를 노출하면 안 됩니다. |

## 단일 기준 SQLite 계약

Volicord는 SQLite 저장소 형식 하나만 지원합니다. 진실 공급원은 현재 기준 SQL
파일입니다.

- [`registry.sql`](../../../crates/volicord-store/src/schema/registry.sql)
- [`project.sql`](../../../crates/volicord-store/src/schema/project.sql)

새 저장소는 이 원본으로만 만듭니다. `project_state.state_version`은 Core 권한
상태 시계이며 저장소 형식 식별자가 아닙니다. 일반 열기는 정확한 현재 manifest와
물리 schema만 받아들이고 기존 데이터베이스를 바꾸지 않습니다. 개발 데이터는 기준
SQL과 현재 manifest를 사용해 새 위치에 만들 수 있습니다. 일반 열기는 영속 권한
데이터를 암묵적으로 버리거나 다시 만들지 않습니다.

별도 비권한 `diagnostics.sqlite` 데이터베이스도 자체 의미적
`volicord.sqlite.diagnostics` 매니페스트와 SQL inventory에서 파생한 canonical schema
digest를 통해 같은 단일 현재 계약 규칙을 따릅니다. 최종 경로가 없으면 불투명한
호출별 identity를 가진 같은 directory의 staging 데이터베이스 하나에서 전체 schema와
manifest를 초기화하고 검증합니다. 모든 SQLite handle을 닫고 live sidecar가 필요하지
않음을 확인하며 permission을 강화하고 파일을 동기화한 뒤, 기존 대상을 교체하지 않는
원자적 공개 연산 하나로 `diagnostics.sqlite`를 보이게 합니다. 동시 shared writer는
완전히 검증된 승자에 수렴하고 자신이 만든 staging 파일만 정리한 뒤 최종
데이터베이스에 각 session을 기록합니다. 기존 최종 데이터베이스는 정확히 검증하며
초기화하거나 복구하지 않습니다. 이 계약은 숫자 `PRAGMA user_version` dispatch,
추론한 형식, 부분 schema를 받지 않으며 매니페스트는 권한 `StorageManifest`가
아닙니다.

## `StorageManifest`

지원되는 저장소 계약은 아래의 정확한 매니페스트로 식별합니다.

```schema
StorageManifest:
  contract_id: string
  canonical_ddl_digest: string
  integrity_constraints_digest: string
  enabled_capabilities: string[]
```

현재 상수는 다음과 같이 정확합니다.

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

`enabled_capabilities`는 UTF-8 byte 오름차순의 이 완전한 목록이며 직렬화 순서가
달라질 수 있는 집합이 아닙니다. 알 수 없거나 누락된 `contract_id`, 알 수 없거나
누락·중복·재정렬된 capability 목록, strict subset, 현재가 아닌 값은 유효하지
않습니다. Store는 default, alias, conversion, capability 추론을 제공하지 않습니다.

필드 의미:

| 필드 | 계약 |
|---|---|
| `contract_id` | SQLite 저장소 계약의 의미적 정체성입니다. 정확히 비교하며 숫자 revision이 아닙니다. |
| `canonical_ddl_digest` | 생성된 전체 DDL 메타데이터를 결정적으로 기준 인코딩한 값의 다이제스트입니다. |
| `integrity_constraints_digest` | 생성된 모든 무결성 제약을 결정적으로 기준 인코딩한 값의 독립 다이제스트입니다. |
| `enabled_capabilities` | 이 형식이 활성화하는 완전하고 정렬되었으며 중복이 없는 역량 집합입니다. 누락된 역량을 추론하지 않습니다. |

매니페스트 identity는 정확히 비교하고, capability 부재를 의미 있는 값으로 다루며, 숫자 비교, 필드
존재 여부 추론, 디코더 탐색, 대체 경로로 형식을 선택하지 않습니다. 물리
매니페스트 표현과 SQLite 안의 배치는 [저장소 DDL](storage-ddl.md)이 담당합니다.

현재 기준 SQL에서 생성한 매니페스트만 지원합니다. 생산자는 결정적인 기준
매니페스트 인코딩 하나만 출력합니다. `map` 또는 `set` 순회 순서, 호스트 경로 표기,
SQLite 행 순서, 표시 형식은 어느 다이제스트에도 영향을 주면 안 됩니다.

## 데이터베이스 열기 계약

Store는 아래 검사를 모두 통과한 데이터베이스만 받아들입니다.

1. 전체 `StorageManifest`를 읽고 엄격하게 디코드합니다.
2. `contract_id`, 두 다이제스트, 전체 역량 집합을 현재 내장 매니페스트와
   비교합니다.
3. 실제 SQLite 객체와 제약을 검사합니다.
4. 매니페스트를 만들 때 사용한 것과 같은 기준 메타데이터 규칙으로 실제 스키마
   목록을 파생합니다.
5. 영속 매니페스트, 생성 메타데이터, 기준 SQL, 실제 데이터베이스가 정확히
   일치해야 합니다.
6. Store handle을 노출하기 전에 외래 키 집행을 활성화하고 확인합니다.

비교는 누락되거나 예상하지 않은 테이블, 열, 인덱스, 제약을 거절합니다. 기준 SQL이
허용하지 않은 다른 SQLite 객체나 스키마 사실도 거절합니다. 권한 또는 정책 기록을
읽기 전, 그리고 어떤 변경도 가능해지기 전에 검증을 마칩니다.

실패 분류는 [실패 모델](failure-model.md)을 따릅니다.

- 매니페스트가 없거나, 알 수 없거나, 이전 형식이거나, 그 밖에 현재 형식이 아닌
  저장소 계약은 `Corrupt`(`corrupt`)입니다. 현재 매니페스트를 선언했지만 매니페스트
  인코딩, 스키마 객체, 제약, 다이제스트, 타입이 지정된 담당 상태가 현재 계약을
  위반하는 데이터베이스도 같습니다.
- 손상 여부를 판단할 수 없는 채 검사를 막는 I/O, 잠금, 환경 실패는
  `Unavailable`(`unavailable`)입니다.

이 실패는 모두 실패 시 닫힌 상태로 처리합니다. Store는 다른 매니페스트, 디코더,
프로필, SQL 목록을 시도하거나, 누락 필드를 채우거나, 추가 객체를 무시하거나,
일부만 검증한 데이터베이스를 열지 않습니다. 바이트가 바뀌지 않았다면 열기를 반복해도
같은 분류를 냅니다.

새 Runtime Home 초기화는 최종 경로가 없을 때만 허용하는 별도 작업입니다. 같은 상위
directory에 staging directory를 만들고 그 안에 기준 SQL을 적용하며 불투명한 UUID 기반
publication ID 하나를 생성합니다. 이 provenance를 현재 manifest, Runtime Home singleton,
installation metadata와 함께 기록하고 외래 키를 활성화한 뒤 전체 manifest와 물리
schema를 검증합니다. Staging 동기화를 마친 뒤 기존 대상을 교체하지 않는 원자적 rename으로
directory를 공개합니다. 성공한 publisher는 rename 직후 invocation별 guard를 받고 상위
directory 동기화, read-back, manifest 확인까지 이를 유지합니다. `AlreadyExists`를 받은
호출자는 자기 staging만 제거하고 제거 권한 없이 정확한 현재 승자를 관찰합니다.

Setup service는 정규 Runtime Home의 외부 OS 기반 lease로 검사, planning, publication,
Store mutation, 정리, 보고, rollback을 직렬화합니다. 이 lease는 Registry record,
`StorageManifest` capability, schema identity, storage lock이 아닙니다. 지원되는 setup은
no-replace 결과를 관찰했다는 이유로 Store mutation을 계속할 수 없습니다. Lease 보유
중 예상하지 않은 `AlreadyExists`가 발생하면 읽기 전용으로 검사하고 새 plan이 필요한
외부 concurrent modification으로 보고합니다.

공개 뒤 확인이 실패하면 주 확인 오류와 guard 기반 rollback 시도를 typed 실패 하나로
유지합니다. 이 실패는 최종 경로가 present, absent, uncertain 중 무엇으로 관찰되었는지
기록하고 재귀 제거 효과와 상위 directory 내구성을 분리합니다. 상위 directory 동기화가
실패해도 완전한 제거는 terminal이며, 불완전하거나 알 수 없는 효과도 terminal이어서 이후
경로 점유자를 대상으로 재시도할 수 없습니다. 이 lifecycle fact는 다른 storage profile이나
schema를 선택하지 않습니다.

기존 Runtime Home은 setup 변경 전에 읽기 전용 연결로 위의 정확한 열기 검사를 수행합니다.
결과는 `Ready`, `Incompatible`, `Corrupt`이며 불일치는 manifest digest와 물리 relation
사실을 한도 안에서 포함합니다. Store는 호환되지 않거나 손상된 home을 바꾸지 않습니다.

## 기준 SQL과 생성 메타데이터

기준 SQL은 단일 진실 공급원입니다. 결정적인 빌드 시점 또는 테스트 시점 추출은
아래 메타데이터를 정확히 생성합니다.

```schema
GeneratedSchemaMetadata:
  tables: GeneratedTable[]
  columns: GeneratedColumn[]
  indexes: GeneratedIndex[]
  constraints: GeneratedConstraint[]
  canonical_ddl_digest: string
  integrity_constraints_digest: string
```

추출은 고정된 원본 순서와 각 모음 안의 결정적 정렬을 사용합니다. 다이제스트 입력은
다이제스트 필드 자체를 제외합니다. 두 다이제스트는 검증이 소비하는 해당 기준 목록
인코딩에서 계산하며 별도의 수동 목록에서 복사하지 않습니다.

같은 생성 아티팩트를 아래 항목이 공유합니다.

- 런타임 정확한 스키마 검증
- 실행 가능한 DDL 계약 테스트
- `StorageManifest` 생성
- 유지 문서의 스키마 목록
- 질의와 행 디코더에 필요한 Store 스키마 투영
- 저장소 픽스처

어느 소비자도 별도의 권위 있는 테이블, 열, 인덱스, 제약 목록을 유지하지
않습니다. 픽스처나 문서 표는 생성된 사실을 투영할 수 있지만 다시 정의할 수
없습니다. 정확한 SQL 텍스트와 물리 제약 정의는
[저장소 DDL](storage-ddl.md)이 담당합니다.

## 실패 시 닫히는 연결과 원자적 변경

받아들인 모든 SQLite 연결은 아래 설정을 활성화합니다.

```sql
PRAGMA foreign_keys = ON;
```

권한 변경은 최신성, 티켓 호환성, 재실행 정체성, 영속 정규 UTC 하한을 읽기 전에
`BEGIN IMMEDIATE` 또는 동등한 직렬화 쓰기 경계를 사용합니다. Store는 그 하나의
트랜잭션 안에서 계획된 변경이 의존하는 모든 사실을 다시 검증합니다.

성공한 권한 변경은 현재 투영, 변경 불가능한 권한 이벤트 또는 담당 문서가
정의한 이벤트 묶음, 상태 버전 증가, 정규 UTC 하한 갱신, 선택적 재실행 행을
원자적으로 커밋합니다. 메서드가 담당하는 쓰기 티켓, 아티팩트, 증거, 사용자 행동,
생명주기, 닫기 상태 효과도 같은 경계에서 커밋합니다. 트랜잭션이 실패하면 어느 효과도
일부만 보이면 안 됩니다.

타입이 지정된 영속 담당 데이터는 사용하기 전에 완전한 현재 타입으로 디코드하고
검증합니다. 잘못된 JSON, 필수 필드 누락, 알 수 없는 닫힌 variant, 허용되지 않은
추가 필드, 필드 사이 불변식 위반은 `Corrupt`입니다. 빈 값, 기본값, 상태 부재,
다른 저장소 계약으로 바꾸지 않습니다.

<a id="canonical-core-utc-clock"></a>
## 정규 Core UTC 시계

정규 Core UTC 시계는 프로젝트 범위에서 감소하지 않습니다.
`project_state.updated_at`은 영속 하한이며 표시 전용 메타데이터나 두 번째 공개 상태
버전이 아닙니다.

공통 preflight 뒤 준비된 공개 Core 작업은 `operation_now` 샘플 하나를 얻고 모든
현재 시각 판단과 공개 작업 timestamp에 다시 사용합니다. 확인된 timestamp 연산은
정규 RFC 3339 UTC 형태로 표현할 수 있어야 합니다. Overflow나 표현할 수 없는 결과는
저장 효과가 없는 검증 거절입니다.

일반 Core 커밋은 immediate write transaction 안에서 `committed_at` 하나를 고릅니다.
프로덕션 시계에서는 `operation_now`, transaction 안에서 얻은 SQLite 현재 UTC, 영속
하한, 현재 Store handle이 이미 받아들인 더 늦은 샘플의 최댓값입니다. 주입 시계에서는
주입한 실시간 후보가 SQLite 현재 UTC를 대신하지만 영속 하한이나 같은 handle의 하한을
대신할 수 없습니다.

트랜잭션은 같은 `committed_at`을 `project_state.updated_at`, 커밋의 모든 권한
이벤트, 선택적 재실행 행, 해당 커밋에서 생성한 transaction metadata에 씁니다. 의미
있는 작업 또는 관찰 timestamp는 담당 문서가 정한 원천을 보존하며 단지
`committed_at`과 같게 만들기 위해 덮어쓰지 않습니다.

UTC 하한과 `project_state.state_version`은 서로 다른 시계입니다. 담당 문서가 정한
하한 전용 저장 효과는 timestamp를 보존하는 행과 원자적으로 하한을 갱신해야 합니다.
정확한 재실행, 거부된 요청, `dry_run=true` 계획, 읽기 전용 결과, 데이터베이스 열기
검증, 실패한 트랜잭션은 하한을 갱신하지 않습니다.

## 프로젝트 상태 버전과 쓰기 티켓 순서

`project_state.state_version`은 커밋된 Core 권한 상태 변경의 순서를 정하고 공개
충돌 및 최신성 근거를 제공합니다. 담당 문서가 허용한 완전한 권한 변경이 커밋될 때만
정확히 한 번 증가합니다. 거부된 요청, dry-run 응답, 정확한 재실행, 읽기 전용 결과,
초기화, 데이터베이스 열기 검증, 잠금 획득, 실패한 트랜잭션에서는 증가하지 않습니다.

새 권한 변경이 커밋될 때마다 projection과 같은 트랜잭션에서 변경 불가능한
`authority_events` 행을 하나 이상 추가합니다. 담당 문서가 정의한 이벤트 묶음은
하나의 결과 상태 버전을 공유합니다. UTC timestamp가 이 순서를 대신하지 않습니다.

`write_tickets.basis_state_version`은 감사 순서만 기록합니다. 티켓 유효성 좌표가
아니며 관련 없는 상태 버전 증가는 티켓을 무효화하지 않습니다. Core는 계획 전에,
Store는 커밋 트랜잭션 안에서 티켓 호환성과 소비를 다시 검증합니다. 거부, dry-run,
재실행 전용 분기는 티켓을 발급, 재사용, 무효화, 소비하지 않습니다.

## 멱등성과 정확한 재실행

`tool_invocations`는 메서드 담당 문서가 재실행 행을 정의한 커밋된
`dry_run=false` Core `MethodResult` 응답의 재실행 데이터만 저장합니다. 고유 키는
정확히 `(project_id, tool_name, idempotency_key)`이며 `request_hash`는 Core가
담당하는 기준 요청 정체성을 구분합니다.

재실행을 사용하려면 현재 확인된 호출 맥락이 저장된 전체 재실행 맥락과 정확히
일치해야 합니다. 전체 맥락은 `actor_source`, `operation_category`, 정확한 선택적
`verification_basis`, 정확한 선택적 기준 Git 작업 공간 맥락입니다. 선택적 좌표는
값의 유무와 값 자체를 모두 보존합니다. 호출 맥락은 `request_hash`에 암묵적으로
흡수하지 않으며 Core는 요청 해시 호환성보다 호출 맥락 호환성을 먼저 확인합니다.

- 호환 맥락, 같은 키, 같은 해시이면 저장된 원래 커밋 응답을 그대로 반환합니다.
- 호환 맥락과 같은 키에서 해시가 다르면 `STATE_VERSION_CONFLICT`를 반환합니다.
- 맥락이 호환되지 않으면 저장된 응답을 노출하지 않고
  `INVOCATION_CONTEXT_MISMATCH`를 반환합니다.

어느 재실행 경로도 바이트를 반환하기 전에 변경 불가능한 저장 JSON이 모든 깊이에서
디코드 결과가 같은 중복 멤버를 갖지 않고, 저장된 메서드 이름이 고른 현재의 닫힌
`MethodResult`로 직접 엄격하게 디코드되는지 확인해야 합니다. 응답 종류, 효과 종류,
dry-run 플래그, 커밋 상태 버전은 재실행 행과 일치해야 합니다. 형태가 잘못되었거나,
현재 형태가 아니거나, 메서드가 다르거나, 좌표가 일치하지 않는 행은 `Corrupt`이며
재실행에 사용할 수 없습니다. Store는 이 행에서 재실행 byte를 반환하지 않습니다.

재실행은 저장된 응답 본문을 반환합니다. 필드를 다시 계산하거나, 이벤트를 추가하거나,
재실행 행을 하나 더 만들거나, 상태를 바꾸거나, 아티팩트를 승격 또는 연결하거나,
쓰기 티켓을 발급, 무효화, 재사용, 소비하지 않습니다.

<a id="exact-operation-result-retrieval"></a>
### 정확한 동작 결과 조회

조회할 수 있는 모든 `operation_category=agent_workflow` Core 커밋과 정확한 재실행은
변경 불가능한 저장 응답을 가리키는 `OperationResultRef`를 노출합니다. 조회 메서드는
연속된 UTF-8 안전 페이지를 읽습니다. `cursor` 순서로 이어 붙인 결과는 저장 응답과
바이트 단위로 정확히 같아야 합니다. 조회는 어떤 필드도 다시 계산, 정규화, 직렬화,
재분류하지 않습니다.

첫 페이지를 반환하기 전에 Store는 정확한 바이트를 읽어 길이와 SHA-256을 계산하고,
Core는 현재 행위자와 프로젝트 접근을 확인한 뒤 그 사실을 참조와 비교합니다. 재실행과
같은 엄격한 커밋 결과 적격성 검사를 적용합니다. 무결성, 형태, 메서드, 접근, 좌표
검사 가운데 하나라도 실패하면 일부 페이지도 반환하지 않습니다. 조회는 읽기 전용이며
재실행 행, 이벤트, 상태 전이, 상태 버전 증가를 만들지 않습니다.
`operation_category=agent_workflow` 밖의 행은 이 Agent Connection 조회 경로를 사용할
수 없습니다.

`volicord.stage_artifact`는 계속 Core 재실행 트랜잭션 밖에 있습니다. 재실행 행이나
`OperationResultRef`를 만들지 않으며, 스테이징 효과가 일어나기 전에 전체 직렬화
결과가 지원되는 예상 크기 상한을 만족해야 합니다.

## 구조화된 Store 진단

Store 진단은 가능한 경우 SQLite primary/extended 결과 code로 `rusqlite` 실패를
분류합니다. SQLite 메시지 문구는 절대 비교하지 않습니다. 현재 닫힌 code는 다음과
같습니다.

| Code | 원인 조건 |
|---|---|
| `store.sqlite.readonly` | Primary code가 `SQLITE_READONLY`입니다. |
| `store.sqlite.busy` | Primary code가 `SQLITE_BUSY`입니다. |
| `store.sqlite.locked` | Primary code가 `SQLITE_LOCKED`입니다. |
| `store.schema.mismatch` | `SQLITE_SCHEMA`를 포함해 정확한 manifest 또는 물리 schema가 일치하지 않습니다. |
| `store.integrity.corruption_failure` | `SQLITE_CORRUPT`, `SQLITE_NOTADB` 또는 typed integrity 검증 실패입니다. |
| `store.record.missing` | 필수 typed record 또는 query row가 없습니다. |
| `store.transaction.failed` | SQLite가 transaction을 abort하거나 interrupt했습니다. |
| `store.serialization.failed` | Typed 저장 값을 encode 또는 decode할 수 없습니다. |
| `store.constraint.violation` | `SQLITE_CONSTRAINT`입니다. 알 수 있는 경우 extended code에서 안전한 `constraint_kind` 사실을 파생합니다. |

Finding에는 숫자 `sqlite_primary_code`, `sqlite_extended_code`, database kind, entity,
field, I/O 오류 종류를 담을 수 있습니다. 임의 SQLite 메시지, SQL 문구, row body, 환경 값,
비밀값, 파일시스템 내용은 담지 않습니다. 매핑하지 못한 내부 Store 실패는 산문에서
추측하지 않고 `internal.unexpected_failure`를 사용합니다.

권장 동작은 typed code에서 파생합니다. Busy와 locked finding은
`action.store.free_locked_database`를 사용하며 readonly, schema, corruption, 누락 record,
serialization, transaction, constraint finding은 각각의 집중 복구 action을 사용합니다.
이 결정적인 실패에는 일반 host restart를 권하지 않습니다.

## 실패, 재시도, 개발 데이터

커밋 전 실패와 트랜잭션 실패는 저장 효과 일부를 남기지 않습니다. 재시도는 보고된
원인을 해결해야 하며 다른 저장소 계약을 고르거나 열기에 실패한 데이터베이스를
변경하면 안 됩니다.

허용된 식별자 밖의 저장소 계약에는 현재 기준 SQL로 만든 새 Runtime Home 또는 프로젝트
저장소가 필요합니다. 손상된 영속 권한 데이터는 담당 문서가 정의한 명시적 복구 판단이
필요하며 일반 읽기와 쓰기는 이를 복구하지 않습니다. 개발 전용 데이터베이스는
삭제하고 현재 원본으로 다시 만들 수 있습니다.

현재 Registry 계약은 완전한 semantic 좌표마다 불변 integration-verification attempt
하나를 semantic observation policy 및 typed repair/retry field와 함께 저장합니다. 정확한
열기 검증은 이 완전한 현재 relation 및 constraint inventory를 요구하며, Store는 Runtime
Home을 열면서 acquisition evidence나 attempt 상태를 합성하지 않습니다.

## 담당 문서 링크

- 여러 표면에 공통인 실패 범주와 기본값 금지 규칙: [실패 모델](failure-model.md)
- 정확한 SQLite 테이블, 열, 인덱스, 외래 키, 제약:
  [저장소 DDL](storage-ddl.md)
- 영속 기록 계열 의미: [저장소 기록](storage-records.md)
- 메서드별 효과와 효과 없음 분기: [저장 효과](storage-effects.md)
- 공개 상태 충돌과 호출 맥락 오류:
  [API 오류 우선순위](api/error-precedence.md#state-conflict-behavior),
  [API 오류 코드](api/error-codes.md#errorcode-invocation-context-mismatch)
- Runtime Home 배치와 분리: [런타임 경계](runtime-boundaries.md)
