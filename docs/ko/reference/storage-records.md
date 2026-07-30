# 저장소 기록

이 문서는 지원되는 저장 계약의 의미와 기록 간 불변식을 담당합니다. 정확한
테이블, column, constraint, index, 정규 SQL은 [저장소 DDL](storage-ddl.md)이
담당합니다.

## 저장 위치

| 위치 | 목적 |
|---|---|
| `registry.sqlite` | Runtime Home identity, 설치 profile, 프로젝트, alias, Agent Connection, 명시적 프로젝트 membership, 정규 연결 검증 보고서, 구조화된 진단 finding, 권위 있는 MCP runtime session |
| 프로젝트 `state.sqlite` | 프로젝트 로컬 Core 상태, replay, authority event, UserAction, evidence, artifact, continuity, 정규화한 host 상관관계, managed MCP project session, Guard 관찰, 조정 |
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
현재가 아닌 digest, 빠지거나 변경되거나 예상하지 않은 schema object는 정확한 열기
검증에 실패합니다.

최종 경로가 없으면 Store는 같은 directory에 불투명하고 고유한 identity를 가진
호출별 staging 파일 하나를 만듭니다. 정규 schema와 manifest 전체를 초기화하고
데이터베이스를 검증한 뒤 닫으며, SQLite journal, WAL, SHM sidecar가 필요하지 않음을
확인한 다음 파일의 permission을 강화해 동기화합니다. 그다음 기존 대상을 교체하지 않는
원자적 연산 하나로 `diagnostics.sqlite`에 공개합니다. 따라서 최종 경로는 완전히
검증된 diagnostics carrier만 가리킵니다. 여러 `SharedWriter` creator가 각각 준비할
수 있지만 하나만 공개하며, 나머지 creator는 자신이 만든 staging 파일만 제거하고
공개된 승자를 검증합니다. 각 호출자는 최종 데이터베이스를 연 뒤에만 자신의
diagnostic session을 삽입합니다.

이미 존재하는 최종 `diagnostics.sqlite`는 현재 read-write diagnostics 경로에서
검증하고 초기화나 복구 없이 permission을 강화합니다. Read-only diagnostics 연산은
그 최종 경로만 검사합니다. 최종 경로가 없으면 빈 결과를 반환하며 staging 파일을
읽거나 만들지 않습니다.

이 diagnostics 매니페스트는 권한 `StorageManifest`가 아니며 diagnostics 데이터베이스는
숫자 schema version을 compatibility identity로 사용하지 않습니다. 읽기는 이
데이터베이스를 만들지 않습니다. Diagnostics 실패는 Core 또는 User Channel 결과를 바꿀
수 없습니다. 운영 MCP evidence는 이 데이터베이스에서 읽지 않습니다.

## 기록 계열

Registry 기록에는 다음이 포함됩니다.

- Runtime Home identity 하나, 불투명 publication ID, 현재 `StorageManifest` carrier
- 설치와 실행 파일 선택
- 프로젝트 등록과 alias
- Agent Connection과 Connection Projects membership
- 프로젝트 범위의 안정적인 Guard 설치 identity와 정규 typed Guard manifest
- Agent Connection마다 최대 하나의 정규 연결 검증 보고서
- 한도가 있는 구조화된 진단 finding과 그 방향성 원인 edge
- MCP runtime session과 그 process, initialization, discovery, 안전 호출, terminal
  finding, graceful close 사실
- runtime/host session 하나를 Connection Project 하나에 결속하는 데이터베이스 간 예약

새 Runtime Home에서는 준비마다 UUID 기반 publication ID 하나를 생성하고 identity row,
최초 installation profile과 함께 staging한 Registry에 삽입한 뒤 최종 directory를
공개합니다. Publication ID는 invocation provenance이며 credential, actor identity,
schema version이 아닙니다. 이 record들이 선택한 Runtime Home 경로에서 접근 가능해지기
전에 정확한 manifest 및 schema 검증을 통과해야 합니다. 기존 Registry는 먼저 읽기
전용으로 검사하며, 호환되지 않는 record 또는 relation 사실을 어떤 Registry record도
다시 쓰지 않고 보고합니다.

Publication guard와 그 rollback 결과는 Registry row가 아니라 process-local typed
lifecycle fact입니다. 확인 실패는 주 오류, rollback 결과, 최종 경로 관찰, 상위 entry
내구성을 함께 유지합니다. 관찰된 완전한 제거는 상위 directory 동기화가 실패해도 완전한
제거로 남으며, 불완전한 효과를 온전히 보존된 상태로 승격하지 않습니다.

정규 Runtime Home별 setup lease와 영속 coordination 파일도 storage record model 밖에
있습니다. Actor identity, publication 권한, recovery 상태, schema version, stale owner
metadata를 담지 않습니다. 활성 OS file lock만 lease 소유권을 나타내며 setup은 정확한
rollback identity를 위해 저장된 publication ID와 process-local publication guard를 계속
사용합니다.

지원하는 모든 Store 변경에는 정확한 정규 Runtime Home을 위한 활성 permit 기반
`RuntimeHomeMutationContext`가 필요합니다. 이 context는 영속 record가 아니라
process-local capability 상태이며 사용자 권한이나 Product Repository 쓰기 허가를
전달하지 않습니다. 일반 writer는 `SharedWriter`에서, setup은 `ExclusiveSetup`에서
context를 만듭니다. Setup과 충돌하면 Registry, project, diagnostic, artifact,
operational-session 기록이 바뀌기 전에 거절합니다.

프로젝트 상태 기록에는 다음이 포함됩니다.

- `project_state`, 프로젝트 workflow policy, Task, acceptance criterion, supplemental
  claim, Change Unit
- 쓰기 티켓, Run, 현재 close basis, blocker, authority event, immutable replay row
- evidence capture intent와 receipt, artifact와 link, evidence summary, observation,
  producer
- `UserActionRequest`, immutable `UserActionResolution`, project continuity
- 조정에 쓰는 expected write, Guard 관찰, prompt 관찰, unrecorded change
- 정규화한 host session, turn, hook tool invocation과 필수 thread를 보관하고 Registry
  runtime binding보다 먼저 생길 수 있는 managed MCP project session

Prompt 관련 Guard 기록은 관찰일 뿐입니다. UserAction resolution, 사용자 답,
verification basis, 권한 출처가 아닙니다.

## 구조화된 진단 Finding

`diagnostic_findings`는 두 lifecycle을 명시적으로 저장하며 `runtime_session_id`에서
lifecycle을 추론하지 않습니다. 모든 row에는 `lifecycle`이 있고, current-state row에는
전체 `current_identity_digest`, `diagnostic_scope_kind`, 완전한
`diagnostic_scope_identity`, 검증된 opaque `current_subject_identity`,
`current_state_status`, 선택적인 `resolved_at`도 있습니다. Subject identity는 정확한
`sha256:<64 lowercase hex>` token을 사용하며 정규 path나 그 밖의 담당자 입력 byte를 저장하지
않습니다. 공유 column에는 namespaced code, domain, stage, severity, source, 한도가 있는 안전한
subject 표시와 안전한 fact JSON, 한도가 있는 action, 적용 가능한 correlation 좌표, 정규 관찰
시각을 저장합니다. Store는 쓰기 transaction을 열기 전에 lifecycle type, subject identity
token, 직렬화 byte 한도를 검증합니다. 환경 dump, 원본 request, 제한 없는 stderr, credential,
정규 subject 입력 byte, 한도 없는 fact object는 저장하지 않습니다.

`insert_occurrence_finding`과 `insert_occurrence_finding_graph`는
`OccurrenceDiagnosticFinding`만 받아 생성된 opaque occurrence ID를 가진 불변 row를
삽입합니다. `upsert_current_snapshot`은 `CurrentDiagnosticFinding`만 받고 완전한 key에서
ID를 파생하며 `current_subject_identity`를 포함한 저장된 identity field 전체를 비교한 뒤
안전한 subject 표시를 포함한 snapshot field만 갱신합니다. 나가는 cause를 원자적으로
교체하고 condition은 항상 active가 됩니다. 검증, 원인 부재, identity, cycle 실패가 발생하면
이전 snapshot을 보존합니다.

`resolve_current_finding`은 `CurrentDiagnosticKey`로 row를 지정해 `resolved_at`을 기록하고
action 및 나가는 cause를 비우며 facts와 마지막 관찰 snapshot data는 유지합니다.
`active_current_findings_for_scope`는 active row만 반환합니다.
`stored_diagnostic_findings_by_ids`와 `stored_diagnostic_finding_by_id`는 완전한
`OccurrenceDiagnosticFinding` 또는 완전한 `CurrentDiagnosticFinding`을 유지하는
`StoredDiagnosticFinding` 값을 반환합니다. 따라서 정확한 current read는
`current_state_status`와 `resolved_at`을 유지합니다. Data를 반환하기 전에 안전한 subject
표시가 아니라 저장된 subject identity에서 각 current key를 복원한 뒤 모든 current digest와
ID를 다시 계산합니다. 잘못된 subject identity나 불일치는 persisted-data corruption입니다.
`reportable_diagnostic_findings_by_ids`는 변경할 수 없는 occurrence와 active current-state
row만 current-report finding으로 projection합니다. Resolved current-state row는
current-report seed에서 제외되지만 정확한 ID 조회로 계속 읽을 수 있습니다. Registry update
trigger도 current identity column과 occurrence row를 보호합니다.

`diagnostic_cause_edges`는 finding에서 원인 finding으로 향하는 edge 하나를 저장합니다.
양쪽 끝은 기존 finding을 가리켜야 하고 composite primary key가 중복을 거부하며, insert
trigger와 Store graph 검증이 cycle을 거부합니다. Store는 immediate transaction 하나에서
모든 finding을 먼저 삽입한 뒤 edge를 삽입하므로 거부된 graph는 일부 finding 집합이나
dangling edge를 남기지 않습니다. 원인 조회는 depth와 finding ID 순서로 정렬하고, 32를
넘는 요청 depth를 거부하며, 서로 다른 finding을 최대 128개 반환하고 선택한 depth 때문에
추가 edge를 자른 경우 이를 표시합니다.
`bounded_stored_diagnostic_graph_from_seeds`는 모든 entry에서 같은
`StoredDiagnosticFinding` lifecycle 형태를 반환하므로 occurrence, active current, resolved
current cause가 각각 정확한 저장 상태를 유지합니다.

Finding은 명시적 ID별, runtime occurrence session별, 정확한 active current scope별로 읽을
수 있습니다. Runtime과 상관된 occurrence에는 해당 runtime의 Connection과 integration
revision도 있어야 합니다. 현재 Connection 편의 조회는 정확한 Connection scope를 현재
integration revision으로 걸러냅니다. 이 Registry finding은 `diagnostics.sqlite`의
한도가 있는 비권한 counter와 구분됩니다. 현재 Connection 보고서는 check가 명시적으로
참조한 finding ID에서 시작해 provenance를 담은 overlay로 한도가 있는 cause chain을
해석합니다. Overlay는 현재 평가의 inline finding을 Store 조회보다 먼저 사용한 뒤 명시적인
영속 seed를 occurrence 또는 active current-state row에서 해석합니다. 이런 명시적인 영속
seed가 Store에 없을 때만 missing-record finding이 유효합니다. 독립적인 현재 finding은 해당
보고서 작업이 의도적으로 선택한 경우에만 포함하며, 같은 revision에 저장된 모든 finding을
한꺼번에 읽지 않습니다.

## 권위 있는 운영 Session

`managed_mcp_launch_leases`는 숨겨진 host launcher와 MCP bootstrap 사이의 Registry
evidence-integrity 경계입니다. 각 row는 opaque lease ID, Connection, `codex` host kind,
예상 Connection integration revision, 예상 managed launch fingerprint, 발급 및 만료 시각,
선택적 소비 시각, 정확한 `issued`, `consumed`, `cancelled`, `expired` terminal state를
저장합니다. 수명이 짧은 lease는 해당 `managed_host` runtime을 만드는 같은 transaction에서
한 번만 소비합니다. Replay, 만료, Connection 불일치, revision 불일치, fingerprint 불일치,
현재 상태가 아닌 Connection은 runtime을 만들지 않습니다. Launcher 실패는 사용하지 않은
lease를 취소하고, 한도가 있는 cleanup은 오래된 terminal row를 만료 처리하거나 제거합니다.
마지막 Connection을 명시적으로 제거할 때는 Connection을 지우기 전에 남은 lease inventory도
제거합니다. Lease record는 OS actor credential이나 재사용 가능한 secret이 아닙니다.

`mcp_runtime_sessions`는 Agent Connection 소유 application state입니다. Volicord는 host
thread metadata가 생기기 전 MCP process 시작 시점에 opaque `runtime_session_id`를
만듭니다. `session_source`는 정확히 `managed_host`, `manual_cli`, `cli_preflight`,
`integration_probe` 중 하나입니다. `managed_host`는 lease에 결속된 managed launcher
source이고, `manual_cli`는 공개 stdio 또는 일회용 CLI conformance source이며,
`cli_preflight`와 `integration_probe`는 비관리 diagnostic 분류입니다. 현재 공개
preflight는 읽기 전용이며 runtime row를 만들지 않습니다. 원자적인 launch-lease 소비에
성공한 경우에만 `managed_host`를 만들 수 있으며, 나머지 source는 검사할 수 있지만
managed-host 운영 evidence 조회를 충족하거나 managed call을 승인할 수 없습니다.

각 물리 `agent_connections` 행에는 Store가 그 행을 삽입할 때 생성한 고유하고 변경
불가능한 opaque 통합 instance ID가 하나 있습니다. Store는 호환 등록 replay, enabled 상태와
검증 갱신, staged activation과 cleanup 복구, mode 전환에서 이 값을 보존합니다. 물리
삭제는 행과 함께 이 값을 제거하며, 나중에 같은 결정적 Connection identity를 삽입해도 새
값을 받습니다.

Connection 통합 revision은 Connection identity, 변경 불가능한 통합 instance ID, host
kind, intent, scope, mode, server name, configuration target, 정확한
managed-configuration fingerprint, Store 소유 비음수 integration generation을
domain-separated canonical digest로 만든 값입니다. Managed fingerprint에는 현재 server
command와 entry가 포함되며, setup 담당 경로가 마지막으로 적용에 성공했거나 채택한
Volicord 관리 host configuration을 식별합니다. 명시적인 setup 소유 managed-configuration
쓰기만 이 값을 바꿀 수 있습니다. 이 쓰기가 fingerprint를 바꾸면 같은 Registry
transaction에서 `verification_report_json`을 비우며, fingerprint가 같은 replay는 보고서를
유지할 수 있습니다. Host와 client version 필드는 diagnostic 관찰로 남습니다. 현재 소유자
field와 Store 소유 generation이 lifecycle revision을 파생합니다. Store는 실제 mode 전환이
성공할 때마다 generation을 정확히 한
번 증가시키고 같은 mode의 no-op에서는 증가시키지 않습니다. Generation은 물리 Connection
instance 하나 안의 revision을 구분하고, 변경 불가능한 instance ID는 물리 삭제와 재생성을
구분합니다. 두 값은 Store 소유 로컬 lifecycle 및 상관관계 좌표입니다.

실제 mode 전환은 Connection mode와 generation을 바꾸고 verification report를 비우며,
소유한 모든 엄격한 Guard manifest에서 integration revision만 교체하는 작업을 원자적으로
수행합니다. 변경 전에 완전한 현재 manifest inventory가 Connection Project membership과
일대일로 일치해야 합니다. 누락, 중복, stale, malformed, owner mismatch, 일부 쓰기 실패가
있으면 Connection이나 manifest를 일부만 갱신하지 않고 Registry transaction 전체가
실패합니다.

Milestone timestamp와 사실로 lifecycle 상태를 표현하며 중복 status enum은 저장하지
않습니다. `attempted_client_name`, `attempted_client_version`,
`requested_protocol_version`은 한도가 있는 해당 값을 파싱하는 즉시 기록하며, 이후
initialize 검증이 실패한 경우도 포함합니다. `selected_protocol_version`은 initialize가
완료될 때 server가 선택해 반환한 revision이고, `negotiated_protocol_version`은 유효한
initialized notification이 선택 profile의 handshake를 완전히 끝낼 때까지 null입니다.
Store는 initialize 완료, 실제 `tools/list` 시각, 정규 정렬한
`returned_tool_identities_json`, required-tool-set 사실과
`required_tools_validated_at`, verification 도구의 정확한 identity/time 쌍인 `verification_tool_name`과
`verification_tool_observed_at`, terminal 구조화 finding ID 하나, graceful close도 각각
기록합니다. Required-tool 성공에는 list 관찰과 반환 inventory가 필요하고,
verification-tool 성공에는 같은 session의 required-tool validation이 필요합니다. Verification
쌍은 모두 null이거나 모두 있어야 합니다. 이름이 있으면 1~128바이트의 MCP 호환 ASCII
이름이어야 하고 관찰 timestamp는 required-tool validation보다 이르지 않아야
합니다. Store는 현재 enabled `managed_host` runtime과 현재 Connection revision에 대해서만
이 쌍을 받으며 `cli_preflight`는 기록할 수 없습니다. 권위 있는 Store 쓰기가 실패하면
protocol 성공을 내보내지 않습니다.
Best-effort diagnostics는 분리되어 있으며 정상적인 도구 결과를 실패시킬 수 없습니다.

Store는 milestone 관계가 모두 일관된 row만 `McpSessionMilestones`로 변환합니다.
`ManagedCapabilityProof`는 추가로 `session_source=managed_host`이면서 process,
initialize, initialized notification, `tools/list`, required-tool, 정규 verification-tool chain이
그 row 하나에서 모두 완료되어야 합니다. 현재 integration revision 하나에서는 가장 최신
managed row를 `latest_managed_attempt`, 가장 최신 complete row를
`latest_managed_capability_proof`로 선택하며 row를 합치지 않습니다. 선택된 peer의
`clientInfo`가 권위 있는 protocol peer
관찰입니다. 별도로 probe한 PATH executable version은 diagnostic으로 남고 proof 선택에
사용하지 않습니다. 영속 Connection report는 선택된 모든 session ID와 role을 보존하며 한
ID가 두 role을 가지면 중복을 제거합니다.

프로젝트 host 상관관계는 source에 따라 정규화합니다.
`CodexMcpTurnMetadata` decoder는 MCP session/thread/turn 상관관계를 제공하고, 별도
`CodexCommandHooks` decoder는 prompt session/turn 또는 tool
session/turn/tool-use/tool-name 상관관계를 제공합니다. Host-contract 담당자가 두 marker를
검토된 profile ID에 연결합니다. `host_sessions`는 Connection, 정확한 native host session,
변경할 수 없는 프로젝트 integration revision, 최초/마지막 관찰 시각을 저장합니다. Store는
Connection internal ID, 정확한 revision, native session으로 revision 범위 로컬 session ID를
도출합니다.
`host_turns`는 그 로컬 session의 정확한 turn을 기록합니다. `host_tool_invocations`는 정확한
session 및 turn 아래 hook tool-use ID와 정규 tool name을 기록합니다. 같은 tool-use ID를
다른 turn이나 tool name에 다시 쓰면 거부합니다.

호환 event의 `guard_events.correlation_kind`는 `codex_hook_prompt` 또는
`codex_hook_tool`입니다. `prompt_capture`는 session과 turn을 요구하고 tool-use field를
금지합니다. `pre_tool`과 `post_tool`은 session, turn, tool-use ID, 정규 tool name을 모두
요구합니다. Prompt capture는 정확한 host turn을 참조하고, expected write는 정확한 host
tool invocation을 참조하며, 상관관계가 있는 unrecorded change도 같은 invocation을
참조합니다. Rust 입력은 해당 `HostNativeCorrelation` variant를 운반하고, SQL check와 복합
foreign key는 불완전하거나 phase 또는 Connection이 교차된 형태를 거부합니다. 어떤 hook
record에도 host-thread field가 없습니다.

`managed_mcp_sessions`는 별도의 MCP 전용 프로젝트 anchor입니다. 정규화한 host session과
최신 host turn을 참조하고 host thread를 요구하며 선택적인 Registry runtime attachment를
갖습니다. Guard 관찰은 이 row를 만들지 않습니다. 빈 값, sentinel, 조작한 runtime,
CLI-preflight runtime으로 attach되지 않은 MCP anchor를 나타내지 않습니다.

동일한 host session의 첫 실제 managed MCP 도구 호출은 순서가 정해진 네 단계를 사용합니다.
Store는 현재 `managed_host` runtime과 그 Connection revision을 변경 없이 검증합니다. 그다음
immediate 프로젝트 transaction에서 unbound `managed_mcp_sessions` anchor를 만들거나 검증하고
Connection, native session, thread, 변경 불가능한 revision, 기존 runtime 충돌을 모두
거부합니다. 이 commit 뒤에만 immediate Registry transaction이 runtime, Connection,
프로젝트 membership, 현재 프로젝트 identity, 정확한 anchor 좌표를 다시 검증하고
`mcp_runtime_project_session_bindings`를 삽입합니다. 마지막 immediate 프로젝트
transaction은 anchor를 다시 검증하고 runtime 필드가 null이거나 이미 같은 runtime을
가리킬 때만 attach합니다.

분리된 SQLite 데이터베이스 사이에는 foreign key를 만들 수 없으므로 Registry 예약이
uniqueness 경계를 제공합니다. 결정적인 프로젝트 소유권 충돌은 새 예약을 남기지 않습니다.
Registry 예약이 실패하면 유효한 anchor가 unbound로 남을 수 있지만 Core를 승인할 수
없습니다. 마지막 쓰기가 중단되면 프로젝트 attach가 없는 예약이 남을 수 있지만 이 예약도
Core를 승인할 수 없습니다. 소유자 상태가 바뀌지 않은 정확한 replay는 그 예약을 재사용해
attach를 완료합니다. 예약은 프로젝트 row와 같은 정확한 프로젝트 통합 revision을 저장하고
권한 검증은 두 값이 같은지 확인합니다. Runtime, Connection, 프로젝트, revision,
host-session Registry claim이 다르면 실패합니다. 프로젝트의 partial index는 null이 아닌
runtime attach를 유일하게 만들면서 unbound MCP anchor를 여러 개 허용합니다.

현재 Registry binding과 정확히 일치하는 attached `managed_mcp_sessions` row만 Core를
승인할 수 있습니다. Hook 전용 정규화 row는 Guard event와 prompt capture를 보관할 수
있지만 managed 호출을 승인할 수 없습니다. Runtime row 자체는 process의 이력 관찰이므로
crash한 row가 열린 것처럼 남거나 현재 row 여러 개가
동시에 존재해도 Guard 상관관계를 막거나 대신 선택되지 않습니다.

Runtime 권한은 이 현재 기록을 직접 읽습니다. 활성 Connection, 현재 Connection Project
membership, 그 Connection의 `session_source=managed_host` nonterminal runtime session, 같은 runtime
session·Connection·프로젝트 소유의 프로젝트 managed MCP session만 허용합니다. 저장된
Connection과 프로젝트 통합 revision은 현재 담당 입력에서 도출한 revision과 같아야
합니다. Connection mode는 요청한 operation category를 허용해야 합니다.
`cli_preflight` row, diagnostic version 필드, best-effort diagnostics는 이 경계를 충족할 수
없습니다.

Registry 저장소는 위의 row만으로 managed operation에 권한을 부여합니다. 현재가 아닌
권한 schema를 담은 Runtime Home은 다른 `StorageManifest`에 속하므로 정확한 열기
검증에서 거부합니다.

Connection Project 폐기는 이 행들을 Connection 소유 Registry 통합 상태로 다룹니다.
명시적 제거와 replacement 정리는 선택한 membership의
`mcp_runtime_project_session_bindings`, 프로젝트 범위 `guard_installations`,
`connection_projects` 행 순서로 원자적으로 삭제합니다. 여러 프로젝트를 가진 superseded
Connection에 membership이 남으면 Agent Connection, 모든 `mcp_runtime_sessions` 행, 다른
프로젝트의 binding과 Guard Installation을 유지합니다. 명시적으로 마지막 membership을
제거하면 Connection 소유의 남은 binding과 Guard Installation을 모두 삭제한 뒤 runtime
session과 `agent_connections` 행을 삭제합니다.

마지막 프로젝트 replacement 정리는 host 정리와 최종 Registry 재검증이 성공할 때까지 비활성
기존 membership, 그 binding과 Guard Installation, pending-host-cleanup marker를 하나의
재시도 inventory로 유지합니다. 최종 정리는 이 프로젝트 소유 행과 membership을 함께
삭제하고 marker를 지우지만, membership이 없는 비활성 과거 Connection과 그 connection 전체
runtime session은 유지합니다. 프로젝트 등록, installation profile, Runtime Home 기록,
모든 프로젝트 `state.sqlite` 행은 어떤 폐기 집합에도 포함되지 않습니다.

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

Store는 쓰기와 읽기 모두에서 완전한 typed 요청과 resolution을 검증합니다.
`StoredUserActionRequest`, `StoredUserActionResolution`,
`StoredUserActionRecordSet`이 검증된 경계를 전달합니다. 불변 조건을 지닌 field는
비공개이며, 공개 typed accessor는 caller가 모순된 영속 record를 조립할 수 없게
하면서 의미 fact를 제공합니다. Store는 다음을 거부합니다.

- 알 수 없거나 섞인 union tag와 추가 필드
- 빠진 종류별 필드
- 요청 본문과 일치하지 않는 `action_kind`
- 닫힌 요청과 일치하지 않는 basis, `required_for`, expiry 중복 표현
- 저장 후보 밖의 option 또는 evidence 선택
- CLI가 아닌 channel 또는 local-user가 아닌 provenance
- 잘못된 제한, timestamp, ref, submission identity
- 요청 identity, action kind, 프로젝트, Task, 현재 basis가 일치하지 않는 resolution

요청과 resolution을 함께 읽어야 할 때는 검증된 `StoredUserActionRecordSet` 하나를
반환합니다. 일반 공개 Store API는 유효하지 않은 set을 반환하거나 구성할 수
없습니다. Commit 전 정규 typed 메모리 projection에도 같은 Store 담당 일관성
검사를 적용하며 unchecked constructor를 노출하지 않습니다.

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
Volicord managed-file 기대값, 필수 typed hook phase, `host_contract_profile`,
`host_contract_digest`를 담습니다. 현재 Guard profile은 명시적으로 `codex-command-hooks`이며,
audit은 들어온 payload에서 parser를 선택하지 않고 그 profile의 결정적인 검토 digest를
요구합니다. File audit과 필수 phase 관찰은 이 manifest와 현재 소유자가 일치하는 사실에서
현재 Guard check를 파생합니다.

Policy command와 runtime command는 의도적으로 서로 다른 projection입니다. 정규 policy
command에는 `--policy-hash`가 없고, 그 policy를 hash한 뒤 runtime command에
`--policy-hash <exact-hash>`를 추가합니다. Hook wrapper와 Guard manifest는 같은 typed
runtime command를 사용합니다. Audit은 공유 소유자 field와 command segment를 개별적으로
비교하며 두 전체 command 객체의 동등성을 비교하지 않습니다.

프로젝트 `guard_events`는 모든 관찰을 Guard 설치, policy hash, integration revision,
typed hook phase, contract status에 결속합니다. `UserPromptSubmit`은 session/turn
상관관계를 가진 `prompt_capture`가 됩니다. `PreToolUse`와 `PostToolUse`는
session/turn/tool-use/tool-name 상관관계를 가진 `pre_tool`과 `post_tool`이 됩니다. 어떤
경우에도 thread 좌표를 요구하거나 저장하지 않습니다. 정규 tool name을 포함한 같은 typed
tool 상관관계가 관련 pre/post record에서 일치해야 합니다. 현재의 compatible event만
`guard_observation`을 도출하며 이전 hash나 revision은 이력을 유지하되 현재 check를
충족하지 못합니다. 현재 malformed 또는 incompatible event가 있으면 이 check는
실패합니다. Prompt capture는 별도 설치 상태가 아니라 같은 관찰 summary의 사실입니다.
Routing된 MCP hook event는 hash와 한도가 있는 정규 correlation을 저장하지만 제한 없는
raw event, tool input, tool result는 저장하지 않습니다.

Registry의 `guard_integration_verification_runs`는 Core 또는 Task record가 아니라 영속적이고
한도가 있는 Connection-integration record입니다. 각 row는 불투명 verification ID와
Connection, project, managed runtime session, native host session과 turn, integration
revision, Guard Installation, host-contract profile, hook-definition digest, policy digest로
이루어진 불변 semantic 좌표를 저장합니다. 무조건 unique constraint가 terminal row를
포함해 좌표마다 row 하나만 허용합니다. Prompt event도 해당 turn의 attempt 하나만
소유합니다. 따라서 정확한 begin replay는 같은 ID와 현재 상태를 반환하며 시간이 지나도
이를 교체하지 않습니다.

Row는 예상 typed probe와 host callable, semantic observation-policy kind, 선택적인 deferred
deadline, 허용 및 소비한 status read 수, 생성 및 cleanup 시각, acknowledgement와 완료,
일치한 prompt/pre/post event, typed terminal repair reason과 retry policy, code, summary도
저장합니다. 상태는 정확히 `awaiting_probe`, `awaiting_observation`, `complete`,
`repair_required`입니다. SQL check는 상태마다 정확한 nullable field 조합을 강제합니다.
좌표 갱신, 두 번째 probe acknowledgement, terminal 변경, terminal-to-active 전이는
거부합니다. Cleanup 시각은 보관 범위만 제한합니다.

Registry의 `guard_probe_observations`는 acquisition 경계를 상관관계가 확인된 완료와
분리해 기록합니다. 폐쇄형 `stage` 값은 `probe_acknowledged`,
`unrelated_routed_tool`, `hook_event_not_observed`, `hook_payload_incompatible`,
`callable_identity_unknown`, `callable_identity_mismatch`,
`verification_id_mismatch`, `session_mismatch`, `turn_mismatch`,
`tool_use_mismatch`, `pre_tool_matched`, `post_tool_matched`입니다. 각 row는 예상
`volicord.guard_probe` agent-tool identity와 예상 host callable, 선택적인 한도 내 관찰
callable, 선택적인 hook kind, verification ID의 존재 및 일치 boolean, Guard
Installation, integration revision, 관찰 시각을 저장합니다. Prompt, 전체 payload, tool
input, tool output은 저장하지 않습니다. 따라서 event 부재는 입증된 routing 원인을
주장하지 않고 `hook_event_not_observed`만 기록합니다.
`unrelated_routed_tool`은 workflow control과 그 밖의 known routed tool, 그리고 정확한
현재 verification ID를 주장하지 않는 알 수 없는 same-server callable을 위한 nonterminal
bounded trace입니다. 이 stage는 proof를 제공하거나 status-read budget을 소비하거나
acknowledgement, repair, retry, root finding을 선택할 수 없습니다.

Store는 이 권위 있는 row 사실을 공유 tagged
`IntegrationVerificationWorkflowState`로 한 번만 투영합니다. 네 저장 상태를 직접
대응시키고 두 nonterminal 상태에만 정규 `AgentToolId`를 노출합니다. Repair reason은
`no_automatic_retry`, `new_turn_required`, `host_reload_required`,
`hook_review_required`, `repair_required`와 분리됩니다. Begin, probe, get/status, adapter,
renderer는 이 상태를 소비합니다. 영속 계층은 사용자 대상 다음 행동 산문이나 renderer
문구를 만들지 않습니다.

Connection report는 가장 최신 현재 revision Guard run을
`guard_verification_attempt`로 조회하고 가장 최신 완료 현재 revision run을 별도로
`guard_verification_proof`로 조회합니다. 최신 run만 correlated check를 결정합니다.
Active는 pending, complete는 passed, `repair_required`는 failed입니다. 더 오래된 완료
row는 proof evidence로 남지만 더 최신 failed attempt를 덮지 못합니다. Report context는
managed MCP role과 같은 runtime session을 중복 제거하고 관련 verification ID를 모두
보존합니다.

Expected-write와 unrecorded-change 기록은 프로젝트 로컬입니다. Guard suppression은
제한된 정규 correlation 데이터만 읽고 정확한 `SuppressionOutcome`을 반환합니다. Store
읽기 실패, 손상된 기록, budget 소진, 유효하지 않은 correlation은 `Unavailable`이며
관찰 경로를 숨기지 않습니다.

Prompt 관찰은 제한된 관찰 schema 아래에서만 저장할 수 있습니다. 사용자 choice,
resolution 본문, 비공개 resolution form, credential을 담지 않습니다.

## 현재 close basis와 continuity

현재 close basis는 Task 소유이며 terminal close history와 구분됩니다. 없음은 생성된 빈
basis가 아니라 없음으로 표현합니다. Evidence와 acceptance ref는 담당 문서 아래에서
정확하고 현재 상태여야 합니다.

Project continuity record는 오래 유지되는 맥락이며 면제가 아닙니다. Typed cursor와
ordering은 status 메서드가 담당합니다. Carry-forward는 현재 scope, baseline, 쓰기
티켓, evidence, UserAction, close 검사를 우회하지 않습니다.

## Store 소유 구조화 값

Store가 소유하는 모든 구조화된 권한 필드는 닫힌 typed schema, digest가 byte에
의존할 때의 정규 encoding, 명시적 크기 제한을 사용합니다. 알 수 없거나 빠지거나
중복되거나 타입이 잘못되거나 noncanonical이거나 담당자 불변식과 맞지 않는 member는
유효하지 않은 입력이며 영속 데이터 손상입니다.

물리 row 형태, 직렬화한 JSON, 닫힌 `TEXT` 값, 영속 timestamp parsing은 Store
내부에만 둡니다. Store는 Task, Change Unit, workflow policy, Write Ticket, Run,
evidence, artifact, project state, replay identity, reconciliation observation, project
continuity, `StoredRecordRef`, UserAction 담당 상태를 public Store-to-Core 또는
Store-to-service interface 밖으로 내보내기 전에 decode합니다. 결과 record는 닫힌
typed 값, actor, timestamp, JSON object, Product Repository 경로, workflow policy,
Write Ticket validity basis와 attempt scope를 전달합니다. Mutation interface도 대응하는
typed 값을 받으며 Store가 SQLite 경계에서 한 번만 물리 형식으로 직렬화합니다.

형식이 잘못된 JSON 값, 알 수 없는 닫힌 문자열, 유효하지 않은 timestamp, 유효하지 않은
Product Repository 경로, 중복 물리 column 사이의 모순은 Store 소유 영속 데이터
`Corrupt` failure를 만듭니다. 빈 값, 기본값, 다른 column을 통해 다시 해석하지
않습니다. Store 손상 constructor는 Store 내부에만 있습니다. Core와 의미 서비스는
검증된 field를 직접 사용하며 영속 표현을 parse하거나 Store 손상을 구성하지 않습니다.
하나의 영속 aggregate 안에서 여러 field 사이 관계가 어긋난 경우에도 Store
corruption이며, 임의의 column 대신 해당 aggregate의 폐쇄형 invariant identity를
사용합니다. 완전히 검증된 typed fact와 현재 operation 또는 policy fact 사이의 의미
모순은 이를 소비하는 Core 또는 서비스 policy가 담당합니다.

### Write Ticket 물리 소유권

Write Ticket Store aggregate만 물리 `write_tickets` 테이블과 column, row
projection, JSON 및 폐쇄형 값 decoding, timestamp decoding, Product Repository
경로 decoding, 영속 필드 간 불변 조건을 담당합니다. 일반 읽기와 기존 Store
transaction 안의 읽기는 정규 row projection, decoder, invariant validator 하나를
공유합니다. 활성 ticket filtering은 선택한 각 row가 검증된 typed Write Ticket이 된
뒤에만 수행합니다. `StoredWriteTicket`의 모든 field는 aggregate 밖에서
비공개입니다. Consumer는 읽기 전용 의미 accessor만 사용하며 영속 record를 구성,
갱신, destructure할 수 없습니다. 공개 호환 alias도 없습니다.

Decode된 `WriteTicketValidityBasis.approval_basis_refs` 모음은 전용
`UserActionResolutionRef` 계약을 사용합니다. 타입이 있는 JSON decoding은
`user_action_resolution`으로 고정된 kind, 완전한 프로젝트, `Task`, resolution
identity, 구체적인 produced state version을 요구합니다. 이어지는 aggregate
검증은 모든 참조의 프로젝트와 `Task`가 물리 ticket 소유자와 같은지 확인하고,
완전한 프로젝트/`Task`/resolution identity 중복을 거부합니다.

다른 Store 모듈은 완전한 typed Write Ticket 또는 그 record에서 만든 집중된 typed
view를 받습니다. 특히 workflow policy 영속화는 typed authority-binding view를
사용하며 `write_tickets`를 query하거나 `validity_basis_json`을 parse하거나 ticket
corruption을 구성하지 않습니다. 검증된 binding과 현재 workflow policy authority의
비교는 의미 policy 판단입니다.

영속화 전 ticket 제안은 합성 `StoredWriteTicket`이 아니라 Core가 소유하는 별도
`PlannedWriteTicket`입니다. Core는 projection 전에 의미 identity, scope,
authority, timestamp 관계와 불변 조건을 지키는 `WriteTicketPathScope` 하나를
검증합니다. 같은 planned 값이 그 정확한 path scope를 포함한 projection 대상 ticket
fact와 완전히 typed인 `WriteTicketInsert`를 모두 제공하며, Store만 해당 삽입 입력을
물리 column과 JSON으로 매핑합니다. Store는 물리 허용 및 거부 column에서 검증된
`WriteTicketPathScope` 하나를 다시 구성한 뒤 opaque stored ticket을 반환합니다.
Dry-run 발급 분기에는 영속 ticket identity가 없고 Store 삽입 입력을 만들 수
없습니다. 재사용은 이미 검증된 stored ticket을 읽고 불변 path-scope view를
노출합니다.

한 물리 Write Ticket field에 국한된 corruption은 그 정확한 field를 식별합니다.
Owner identity, scope revision, baseline, timestamp 순서, path set, path coverage,
write intent, status lifecycle처럼 여러 field 사이 관계가 어긋나면 Write Ticket
aggregate와 폐쇄형 invariant code를 식별합니다. 승인 소유자 불일치와 승인
resolution identity 중복은 JSON column에 귀속하지 않고 aggregate invariant
failure로 처리합니다. 영속 ticket 자체가 내부적으로
유효하다면 operation 시각을 기준으로 한 expiry, 현재 policy와의 incompatibility,
요청 operation coverage 부족은 의미 거절로 유지됩니다.

커밋된 메서드 응답의 정확한 byte는 의도적인 의미 담당자 예외입니다. Store는 영속 replay
carrier와 typed replay identity를 검증하고 `response_json`을 정확히 보존하지만 메서드
결과를 다시 해석하지 않습니다. Replay 또는 operation-result 동작에 필요할 때 Core가
그 정확한 결과를 담당 메서드 계약에 맞춰 검증하고 decode합니다. Core 소유 event payload
byte는 해당 Core event 계약을 따르며 typed Store 권한 record field로 노출하지 않습니다.

명시적으로 비권한으로 분류한 metadata는 계속 비권한입니다. 사용자 판단, evidence
assurance, acceptance, 쓰기 티켓 권한, 닫기 준비 상태를 만들 수 없습니다.

### Agent Connection 검증 보고서

`agent_connections.verification_report_json`은 유일한 영속 연결 검증 상태입니다.
Null이 아닌 값은 완전하고 엄격한 `ConnectionVerificationReport` 하나이며 파생 상태,
check, 사용자 action을 독립적으로 저장하거나 변경할 수 없습니다. SQL null은 완료된
보고서가 없다는 뜻입니다. 읽기 경로는 Registry 저장소를 바꾸지 않고 그 부재를 Agent
Connection 담당 문서의 합성 `verification_not_run` 보고서로 projection합니다.

보고서의 `mcp_server` check 안에서 `preflight`와
`last_active_verification`은 서로 같은 계층의 분리된 증거 record입니다.
`preflight.evidence`는 생성 뒤 불변이고 쓰기 가능성을 항상 `not_checked`로 유지하며
side-effect 배열은 항상 비어 있습니다. `last_active_verification`은 활성 증거가 없으면
null이고, 있으면 마지막 활성 Registry/프로젝트 쓰기 결과, 일회용 conformance 결과,
timestamp, source, side effect를 담습니다. Store는 결합된 preflight/write 결과를 받지
않으며 결합 형태는 유효하지 않은 영속 담당 상태입니다.

보고서 교체는 호출자가 준 fingerprint가 아니라 정확한 예상 typed Connection integration
revision을 받습니다. Store는 immediate Registry transaction 하나에서 revision 소유자 field를
읽고 검증하며 현재 revision을 비교한 뒤 `verification_report_json`과 일반 row 갱신
timestamp만 바꿉니다. 불일치는 쓰기 효과가 없는 충돌입니다. 이 경계는 malformed인 저장
보고서의 명시적 교체는 허용하지만 malformed metadata나 다른 소유자 상태를 복구하지
않습니다. Action의 typed 구성원이 누락되거나 알 수 없는 구성원이 있거나, check/root
reference가 비정규이거나, owner/channel/check metadata가 ID와 일치하지 않으면 이런
malformed 보고서에 해당합니다. 엄격한 읽기는 이를 거부하지만 활성 검증은 revision이
바뀌지 않았을 때 보고서 전체를 교체할 수 있습니다. 따라서 검증은 관리 configuration을
채택하거나 Connection integration revision을 바꿀 수 없습니다.

Store는 쓰기 전과 읽은 뒤 공유 보고서 type을 검증합니다. 닫힌 값, 상한, 결정적 순서,
중복 거부, 파생 집계를 모두 확인합니다. 형식이 잘못되었거나 비정규인 보고서 JSON은
영속 담당 상태 손상입니다. 보고서 부재로 해석하거나 다른 column에서 복구하지 않습니다.

선택적인 check 구성원 값이 없으면 정규 JSON에 명시적 null을 저장하지 않고 그 구성원을
생략합니다. 각 action은 의미를 나타내는 `id`와 사용자 `instruction`만 포함하며 실행 호출을
영속하지 않습니다. 이 보고서만 저장 check/action 상태를 소유합니다. CLI 명령 출력은 해당
구성원을 최상위에 projection하며 두 번째 명령 출력 트리를 영속하지 않습니다.

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
