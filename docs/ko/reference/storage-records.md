# 저장소 기록

이 문서는 최초 릴리스 단일 저장 계약의 의미와 기록 간 불변식을 담당합니다. 정확한
테이블, column, constraint, index, 정규 SQL은 [저장소 DDL](storage-ddl.md)이
담당합니다.

## 저장 위치

| 위치 | 목적 |
|---|---|
| `registry.sqlite` | Runtime Home identity, 설치 profile, 프로젝트, alias, Agent Connection, 명시적 프로젝트 membership, 정규 연결 검증 보고서, 권위 있는 MCP runtime session |
| 프로젝트 `state.sqlite` | 프로젝트 로컬 Core 상태, replay, authority event, UserAction, evidence, artifact, continuity, 프로젝트 Agent Session, Guard 관찰, 조정 |
| artifact store | 영속 artifact row가 참조하는 bytes와 안전 notice |
| `diagnostics.sqlite` | 제한된 비권한 operability counter |

각 프로젝트 상태 데이터베이스는 등록된 정규 Product Repository 하나에 속합니다.
프로젝트를 가로지르는 row, ref, replay, 현재 pointer는 유효하지 않습니다.

### 로컬 Diagnostics 계약

`diagnostics.sqlite`는 현재 SQL의 table, column, index inventory에서 파생한 정확한
canonical schema digest와 `contract_id=volicord.sqlite.diagnostics`로 식별되는 별도
비권한 저장 계약 하나를 사용합니다. 매니페스트에는 singleton row가 정확히 하나 있어야
합니다. 새 diagnostics 저장소는 데이터베이스 경로가 없을 때만 만듭니다. 이미 존재하는
빈 데이터베이스, 빠지거나 추가된 매니페스트 row, 알 수 없는 contract identifier,
현재가 아닌 digest, 빠지거나 변경되거나 예상하지 않은 schema object는 migration,
복구, importer dispatch, 형식 추론 없이 거부합니다.

이 diagnostics 매니페스트는 권한 `StorageManifest`가 아니며 diagnostics 데이터베이스는
숫자 schema version을 compatibility identity로 사용하지 않습니다. 읽기는 이
데이터베이스를 만들지 않습니다. Diagnostics 실패는 Core 또는 User Channel 결과를 바꿀
수 없습니다. 운영 MCP evidence는 이 데이터베이스에서 읽지 않습니다.

## 기록 계열

Registry 기록에는 다음이 포함됩니다.

- Runtime Home identity 하나와 현재 `StorageManifest` carrier
- 설치와 실행 파일 선택
- 프로젝트 등록과 alias
- Agent Connection과 Connection Projects membership
- 프로젝트 범위의 안정적인 Guard 설치 identity와 정규 typed Guard manifest
- Agent Connection마다 최대 하나의 정규 연결 검증 보고서
- MCP runtime session과 그 process, initialization, discovery, 안전 호출, terminal
  failure, graceful close 사실
- runtime/host session 하나를 Connection Project 하나에 결속하는 데이터베이스 간 예약

프로젝트 상태 기록에는 다음이 포함됩니다.

- `project_state`, 프로젝트 workflow policy, Task, acceptance criterion, supplemental
  claim, Change Unit
- 쓰기 티켓, Run, 현재 close basis, blocker, authority event, immutable replay row
- evidence capture intent와 receipt, artifact와 link, evidence summary, observation,
  producer
- `UserActionRequest`, immutable `UserActionResolution`, project continuity
- 조정에 쓰는 expected write, Guard 관찰, prompt 관찰, unrecorded change
- Registry runtime binding보다 먼저 생길 수 있고 host session/thread/latest-turn 상관관계,
  최초/마지막 관찰, 현재 프로젝트 통합 revision을 보관하는 프로젝트 Agent Session

Prompt 관련 Guard 기록은 관찰일 뿐입니다. UserAction resolution, 사용자 답,
verification basis, 권한 출처가 아닙니다.

## 권위 있는 운영 Session

`mcp_runtime_sessions`는 Agent Connection 소유 application state입니다. Volicord는 host
thread metadata가 생기기 전 MCP process 시작 시점에 opaque `runtime_session_id`를
만듭니다. `session_source`는 정확히 `managed_host` 또는 `cli_preflight`입니다. CLI row는
검사할 수 있지만 managed-host 운영 evidence 조회를 충족할 수 없습니다.

각 물리 `agent_connections` 행에는 Store가 그 행을 삽입할 때 생성한 고유하고 변경
불가능한 opaque 통합 instance ID가 하나 있습니다. Store는 호환 등록 replay, enabled 상태와
검증 갱신, staged activation과 cleanup 복구, mode 전환에서 이 값을 보존합니다. 물리
삭제는 행과 함께 이 값을 제거하며, 나중에 같은 결정적 Connection identity를 삽입해도 새
값을 받습니다.

Connection 통합 revision은 Connection identity, 변경 불가능한 통합 instance ID, host
kind, intent, scope, mode, server name, configuration target, 정확한
managed-configuration fingerprint, Store 소유 비음수 integration generation을
domain-separated canonical digest로 만든 값입니다. Managed fingerprint에는 현재 server
command와 entry가 포함됩니다. Host와 client version 필드는 제외하며 identity나 allowlist
입력이 아닌 diagnostic 관찰로만 남습니다. 이 fingerprint에는 host executable identity나
provenance 입력이 없습니다. Store는 실제 mode 전환이 성공할 때마다 generation을 정확히 한
번 증가시키고 같은 mode의 no-op에서는 증가시키지 않습니다. Generation은 물리 Connection
instance 하나 안의 revision을 구분하고, 변경 불가능한 instance ID는 물리 삭제와 재생성을
구분합니다. 어느 좌표도 host나 actor를 식별하거나 release를 인증하거나 security
credential로 동작하지 않습니다.

실제 mode 전환은 Connection mode와 generation을 바꾸고 verification report를 비우며,
소유한 모든 엄격한 Guard manifest에서 integration revision만 교체하는 작업을 원자적으로
수행합니다. 변경 전에 완전한 현재 manifest inventory가 Connection Project membership과
일대일로 일치해야 합니다. 누락, 중복, stale, malformed, owner mismatch, 일부 쓰기 실패가
있으면 Connection이나 manifest를 일부만 갱신하지 않고 Registry transaction 전체가
실패합니다.

Milestone timestamp와 사실로 lifecycle 상태를 표현하며 중복 status enum은 저장하지
않습니다. Store는 성공한 `initialize`, initialized notification, 실제 `tools/list`마다의
응답과 required-tool-set 사실, 지정된 안전/읽기 전용 Volicord 호출 성공, terminal
protocol failure, graceful close를 기록합니다. 권위 있는 Store 쓰기가 실패하면 protocol
성공을 내보내지 않습니다. Best-effort diagnostics는 분리되어 있으며 정상적인 도구 결과를
실패시킬 수 없습니다.

프로젝트 `agent_sessions`는 프로젝트 로컬 상관관계 projection입니다. 각 row는 Connection을
이름 붙이고 현재 workflow-policy fingerprint와 Guard ownership pair를 더한 프로젝트 통합
revision을 보관하며, workflow와 Guard 상관관계에 필요한 결정적 revision 범위 session
ID, host session, thread, latest turn, 최초/마지막 관찰을 유지합니다. Store는 Connection
internal ID, 저장된 정확한 프로젝트 통합 revision, 정확한 native host session으로 이 ID를
도출합니다. 프로젝트 revision은 변경할 수 없으며 현재 revision이 바뀌면 이력 row를
갱신하지 않고 새 row를 만듭니다. Guard 관찰은
`runtime_session_id=NULL`인 row를 만들 수 있습니다. 빈 값, sentinel, 조작한 runtime,
CLI-preflight runtime으로 이 상태를 나타내지 않습니다. 복합 프로젝트 foreign key는 하위
Guard row가 session을 다른 Connection과 조합하지 못하게 합니다.

동일한 host session의 첫 실제 managed MCP 도구 호출은 Registry
`mcp_runtime_project_session_bindings`를 예약한 다음 runtime을 프로젝트 row에 붙입니다.
분리된 SQLite 데이터베이스 사이에는 foreign key를 만들 수 없으므로 Registry 예약이
uniqueness 경계를 제공합니다. 예약 뒤 attach 전에 중단된 경우를 포함해 정확한 replay는
멱등입니다. 예약은 프로젝트 row와 같은 정확한 프로젝트 통합 revision을 저장하고 권한
검증은 두 값이 같은지 확인합니다. Runtime, Connection, 프로젝트, revision, host-session
claim이 다르면 실패합니다. 프로젝트의
partial index는 null이 아닌 runtime attach를 유일하게 만들면서 Guard-first unbound session은
여러 개 허용합니다.

현재 Registry binding과 정확히 일치하는 attached 프로젝트 row만 Core를 승인할 수
있습니다. Unbound row는 Guard event와 prompt capture를 보관할 수 있습니다. Runtime row
자체는 process의 이력 관찰이므로 crash한 row가 열린 것처럼 남거나 현재 row 여러 개가
동시에 존재해도 Guard 상관관계를 막거나 대신 선택되지 않습니다.

Runtime 권한은 이 현재 기록을 직접 읽습니다. 활성 Connection, 현재 Connection Project
membership, 그 Connection의 `session_source=managed_host` runtime session, 같은 runtime
session·Connection·프로젝트 소유의 프로젝트 Agent Session만 허용합니다. 저장된
Connection과 프로젝트 통합 revision은 현재 담당 입력에서 도출한 revision과 같아야
합니다. Connection mode는 요청한 operation category를 허용해야 합니다.
`cli_preflight` row, diagnostic version 필드, best-effort diagnostics는 이 경계를 충족할 수
없습니다.

Registry 저장소는 위의 row만으로 managed operation에 권한을 부여합니다. 현재가 아닌
권한 schema를 담은 Runtime Home은 다른 `StorageManifest`에 속하므로 migration 없이
거부합니다.

명시적인 Connection Project 제거는 이 행들을 Connection 소유 Registry 통합 상태로
다룹니다. Store는 선택한 membership의 `mcp_runtime_project_session_bindings`, 프로젝트
범위 `guard_installations`, `connection_projects` 행을 원자적으로 삭제합니다. 다른
membership이 남으면 Agent Connection, 모든 `mcp_runtime_sessions` 행, 다른 프로젝트의
binding과 Guard Installation을 유지합니다. Membership이 하나도 남지 않으면 Connection
소유의 남은 binding과 Guard Installation을 모두 삭제한 뒤 runtime session과
`agent_connections` 행을 삭제합니다. 프로젝트 등록, installation profile, Runtime Home
기록, 모든 프로젝트 `state.sqlite` 행은 이 삭제 집합에 포함되지 않습니다.

유지된 프로젝트 로컬 Agent Session과 Guard 또는 workflow 이력은 이후 권한이 되지
않습니다. Runtime 권한은 계속 위에서 설명한 현재 Registry membership, runtime session,
project-session 검증을 요구합니다. 실제 mode 전환 뒤 또는 물리 Connection 삭제와 재생성
뒤에 같은 native host session을 다시 사용하면 유지된 이력과 충돌하지 않고 새로운 revision
범위 프로젝트 row를 선택합니다.

## Identity와 소유권

저장 식별자는 정확하고 비어 있지 않은 담당자 값입니다. Store는 표시 text에서 식별자를
trim, 추측, 재할당하거나 대체 식별자를 도출하지 않습니다. 모든 Task 범위 row는 소유
row와 같은 프로젝트와 Task를 이름 붙입니다. 모든 Change Unit, evidence target, Run,
artifact link, blocker, continuity ref는 소유 좌표에 맞춰 검증합니다.

현재 pointer는 같은 프로젝트의 현재 record를 참조해야 합니다. 현재 상태가 전진한 뒤에도
immutable history가 남을 수 있지만 timestamp 비교나 record 이름 순서로 현재가 되지는
않습니다.

정규 Product Repository 경로는
[런타임 경계](runtime-boundaries.md#product-repository-api-path-normalization)를 따릅니다.
Git object ID는 공유된 정확한 소문자 16진수 40자리 또는 64자리 계약을 사용합니다.
다른 길이와 16진수가 아닌 값은 쓰기에서 유효하지 않고 읽기에서 손상입니다.

## 저장 UserAction 엄격 검증

`user_action_requests`는 닫힌 요청 본문 하나, Core 파생 typed basis, `required_for`,
source method/idempotency identity, actor, expiry를 저장합니다.
`user_action_resolutions`는 최대 하나의 닫힌 종류 일치 resolution,
`channel_kind=cli`, 제한된 visible-ASCII submission identity,
`resolved_by_actor_source=local_user`, verification basis, assurance, Core capture
time을 저장합니다.

Store는 쓰기와 읽기 모두에서 완전한 typed 요청과 resolution을 검증합니다. 다음을
거부합니다.

- 알 수 없거나 섞인 union tag와 추가 필드
- 빠진 종류별 필드
- 요청 본문과 일치하지 않는 `action_kind`
- 저장 후보 밖의 option 또는 evidence 선택
- CLI가 아닌 channel 또는 local-user가 아닌 provenance
- 잘못된 제한, timestamp, ref, submission identity
- 요청, 프로젝트, Task, 현재 basis가 일치하지 않는 resolution

잘못된 저장 값은 영속 데이터 기계 판독 code를 가진 `Corrupt`입니다. 기본값을 넣거나
조용히 건너뛰거나 다른 column에서 복구하거나 부분적으로 유효한 객체로 반환하지 않습니다.
CLI inbox는 fail closed하며 MCP는 안전한 실패만 노출할 수 있고 row를 해결하지 않습니다.

<a id="exact-operation-result-storage"></a>
## Replay와 효과

커밋된 non-dry-run Core mutation 하나는 적격 응답을 method, project, actor, operation
category, idempotency identity, request hash, state version, 선택적 검증 workspace 좌표와
함께 정확히 저장합니다. 정확한 retry는 원래 bytes를 반환하고 같은 identity에 다른 정규
입력을 쓰면 conflict입니다.

User-only resolution replay는 Agent Connection이 접근할 수 없습니다.
Request-user-action resume은 원래 agent-safe 요청 결과와 별도로 새로 읽은 안전한 현재
projection만 읽을 수 있습니다.

## Guard와 조정 기록

Registry의 각 `guard_installations` row는 안정적인 설치/소유자 identity, 정규
`manifest_json`, 생성/갱신 timestamp만 유지합니다. Manifest는 엄격하고 소유자에
결속되며 정확한 policy hash, integration revision, typed runtime command, 전체
Volicord managed-file 기대값, 필수 typed hook phase를 담습니다. 이 객체는 host capability
인증서도 설치 status 상태 기계도 아닙니다.

Policy command와 runtime command는 의도적으로 서로 다른 projection입니다. 정규 policy
command에는 `--policy-hash`가 없고, 그 policy를 hash한 뒤 runtime command에
`--policy-hash <exact-hash>`를 추가합니다. Hook wrapper와 Guard manifest는 같은 typed
runtime command를 사용합니다. Audit은 공유 소유자 field와 command segment를 개별적으로
비교하며 두 전체 command 객체의 동등성을 비교하지 않습니다.

프로젝트 `guard_events`는 모든 관찰을 Guard 설치, policy hash, integration revision,
typed hook phase, contract status에 결속합니다. 현재의 compatible event만
`guard_observation`을 도출하며 이전 hash나 revision은 이력을 유지하되 현재 check를
충족하지 못합니다. 현재 malformed 또는 incompatible event가 있으면 이 check는
실패합니다. Prompt capture는 별도 설치 상태가 아니라 같은 관찰 summary의 사실입니다.

Expected-write와 unrecorded-change 기록은 프로젝트 로컬입니다. Guard suppression은
제한된 정규 correlation 데이터만 읽고 정확한 `SuppressionOutcome`을 반환합니다. Store
읽기 실패, 손상된 기록, budget 소진, 유효하지 않은 correlation은 `Unavailable`이며
관찰 경로를 숨기지 않습니다.

Prompt 관찰은 제한된 관찰 schema 아래에서만 저장할 수 있습니다. 사용자 choice,
resolution 본문, 비공개 inbox form, credential을 담지 않습니다.

## 현재 close basis와 continuity

현재 close basis는 Task 소유이며 terminal close history와 구분됩니다. 없음은 생성된 빈
basis가 아니라 없음으로 표현합니다. Evidence와 acceptance ref는 담당 문서 아래에서
정확하고 현재 상태여야 합니다.

Project continuity record는 오래 유지되는 맥락이며 면제가 아닙니다. Typed cursor와
ordering은 status 메서드가 담당합니다. Carry-forward는 현재 scope, baseline, 쓰기
티켓, evidence, UserAction, close 검사를 우회하지 않습니다.

## 저장소 소유 JSON

권한과 관련된 모든 JSON 필드는 닫힌 typed schema, digest가 bytes에 의존할 때의 정규
encoding, 명시적 크기 제한을 사용합니다. 알 수 없거나 빠지거나 중복되거나 타입이
잘못되거나 noncanonical이거나 담당자 불변식과 맞지 않는 member는 유효하지 않은 입력이며
영속 데이터 손상입니다.

명시적으로 비권한으로 분류한 metadata는 계속 비권한입니다. 사용자 판단, evidence
assurance, acceptance, 쓰기 티켓 권한, 닫기 준비 상태를 만들 수 없습니다.

### Agent Connection 검증 보고서

`agent_connections.verification_report_json`은 유일한 영속 연결 검증 상태입니다.
Null이 아닌 값은 완전하고 엄격한 `ConnectionVerificationReport` 하나이며 파생 상태,
check, 사용자 action을 독립적으로 저장하거나 변경할 수 없습니다. SQL null은 완료된
보고서가 없다는 뜻입니다. 읽기 경로는 Registry 저장소를 바꾸지 않고 그 부재를 Agent
Connection 담당 문서의 합성 `verification_not_run` 보고서로 projection합니다.

Store는 쓰기 전과 읽은 뒤 공유 보고서 type을 검증합니다. 닫힌 값, 상한, 결정적 순서,
중복 거부, 파생 집계를 모두 확인합니다. 형식이 잘못되었거나 비정규인 보고서 JSON은
영속 담당 상태 손상입니다. 보고서 부재로 해석하거나 다른 column에서 복구하지 않습니다.

선택적인 check 또는 action 구성원 값이 없으면 정규 JSON에 명시적 null을 저장하지 않고
그 구성원을 생략합니다. 이 영속 보고서만 저장 check/action 상태를 소유합니다. CLI 명령
출력은 해당 구성원을 최상위에 projection하며 두 번째 명령 출력 트리를 영속하지 않습니다.

<a id="authority-bundle-export"></a>
## 권한 번들 내보내기

비변경 권한 번들은 담당 문서가 정의한 일관된 snapshot을 읽습니다. Diagnostics,
credential, 비공개 UserAction note, prompt, 대화 기록, runtime log, export 담당자가
선택하지 않은 artifact bytes는 포함하지 않습니다. 내보내기는 프로젝트 상태를 바꾸지
않습니다.

내보내는 record table 집합은 별도로 유지하는 table 목록이 아니라 정규 프로젝트 상태
`GeneratedSchemaMetadata`에서 투영합니다. `acceptance_criteria`, `authority_events`,
`evidence_claims`, `project_workflow_policies`를 포함한 모든 정규 table relation을
포함합니다. 정규 프로젝트 상태 스키마는 파생 호환 relation이 아니라 record table로
구성됩니다. Content redaction은 field 의미를 따릅니다. 예를 들어 `prompt_captures` row는
`prompt_text`를 내보내지 않고 user-only replay row는 response 본문을 내보내지 않습니다.

## 관련 담당 문서

- [저장소](storage.md)
- [저장소 DDL](storage-ddl.md)
- [저장 효과](storage-effects.md)
- [저장소 버전 관리](storage-versioning.md)
- [API UserAction 스키마](api/schema-user-action.md)
- [실패 모델](failure-model.md)
