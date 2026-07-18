# MCP 전송 참조

이 문서는 최초 릴리스의 로컬 MCP 프로세스 경계인 관리 stdio 시작, 엄격한 binding,
JSON-RPC lifecycle, 도구 검색, 공개 인자 projection, 응답 wrapping, 종료를 담당합니다.
Core 메서드, Codex 구성, 연결 검증, 저장 효과는 각각의 집중 담당 문서에 남습니다.

<a id="surface-stability"></a>
## 표면 안정성

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)을 사용합니다.

| 표면 | 안정성 |
|---|---|
| `volicord mcp --stdio`, 초기화, `tools/list`, `tools/call`, 응답 wrapping | `stable` |
| 권위 있는 runtime-session lifecycle milestone | `stable` |
| stable 프로세스와 메서드 집합에 나열하지 않은 pre-1.0 추가 표면 | `beta` |
| 관리 시작 marker와 생성 구성 세부사항 | `internal` |
| Host 실행 파일 version, MCP client name/version, best-effort protocol metric | `diagnostic` |

## 프로세스 모델

`volicord mcp --stdio`는 관리 Codex 구성이 시작하는 자식 프로세스입니다. stdin과
stdout으로 줄 단위 JSON-RPC를 교환하며 TCP, HTTP, Unix domain socket 또는 그 밖의
네트워크 listener를 열지 않습니다.

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
volicord mcp --stdio --discover-repository --host codex
volicord mcp --check --connection <connection_id> [--project <project_id>]
```

결속 형태는 생성된 관리 entry의 정확한 저장 식별자를 사용합니다. 저장소 검색은 정규
공유 Codex entry 전용이며 정확한 Runtime Home과 정규 Git 작업 트리에서 Connection과
프로젝트를 해결합니다. cwd만으로
연결을 추론하거나 주변 저장소를 검색하거나 다른 host selector를 받지 않습니다.
`--check`는 stdio loop에 들어가지 않고 사전 점검만 수행합니다.

## 환경과 시작

`VOLICORD_HOME`은 [런타임 경계](runtime-boundaries.md)에 따라 Runtime Home을
선택합니다. 공유 구성은 머신 로컬 경로를 내장하지 않고 값을 전달합니다.

MCP 요청을 읽기 전에 어댑터는 Volicord가 생성한 관리 시작/구성 맥락에서 정확한 등록
Connection을 해결합니다. Connection 활성 상태, 선택한 프로젝트의 현재 membership,
Runtime Home/Product Repository 분리, 현재 `StorageManifest`, 필요한 저장 읽기 가능성을
검증합니다. 관리 시작 marker는 협력적인 process source를 분류하지만 client, host,
actor, human identity를 증명하지 않습니다. 손상된 기록, 모호한 선택, 사용할 수 없는
저장소에는 [실패 모델](failure-model.md)을 적용합니다.

시작 경로는 parent executable을 hash하거나, 정확한 호스트 allowlist를 조회하거나,
플랫폼 실행 파일 identity를 도출하거나, 실행 파일 attestation을 발급 또는 읽거나,
client/host version을 권한 입력으로 사용하지 않습니다.

## MCP wire 동작

비어 있지 않은 stdin 각 줄은 완전한 UTF-8 JSON-RPC 2.0 요청 하나입니다. 잘못된
JSON은 `-32700`, 잘못된 요청은 `-32600`, 알 수 없는 메서드는 `-32601`, 잘못된
인자는 `-32602`, 내부 프로토콜 실패는 `-32603`을 반환합니다. 응답은 요청 `id`를
보존합니다.

`initialize`가 `tools/list`와 `tools/call`보다 먼저 와야 합니다. 프로세스는 지원 MCP
protocol version만 협상하고 `notifications/initialized`를 받습니다. 초기화 전 호출,
반복 initialize, batch 입력, 지원하지 않는 version은 Core 전에 실패합니다.

## 권위 있는 Lifecycle 기록

프로세스는 Agent Connection을 해결한 뒤 thread metadata를 검증하거나 protocol message를
읽기 전에 Registry runtime session을 만듭니다. 이 row는 Volicord가 생성한 process
launch, Connection, `managed_host` 또는 `cli_preflight` source, 현재 Connection 통합
revision, process ID, process 시작 시각을 식별합니다. CLI preflight row는
managed-host 운영 check를 충족하지 않습니다.

어댑터는 성공한 `initialize`를 응답보다 먼저 영속 기록하고, 유효한
`notifications/initialized`를 ready 상태 진입 전에 기록하며, 실제 `tools/list` 응답을
반환하기 전에 매번 기록합니다. Discovery 사실은 생성된 그 응답에 현재 Connection
mode가 요구하는 모든 도구가 있었는지를 나타냅니다. 중복 initialized notification은
첫 번째 유효 관찰 뒤 멱등입니다.

성공한 `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`,
`volicord.list_projects` 완료는 도구 결과를 내보내기 전에 안전/읽기 전용 milestone을
갱신합니다. 관찰할 수 있는 fatal transport failure와 EOF에 따른 graceful close는 각
terminal 사실을 기록합니다. 권위 있는 Store 쓰기가 실패하면 해당 protocol 성공을
내보내지 않습니다. `diagnostics.sqlite`의 제한된 쓰기는 계속 best effort이며 이 사실을
조회할 때 사용하지 않습니다.

연결 검증은 별도의 `cli_preflight` process를 시작하고 지정된 안전한 읽기 전용 round
trip으로 `volicord.list_projects`를 호출합니다. 이 process는 server 표면을 검증하지만
그 lifecycle 사실은 `managed_host` 운영 check를 충족하거나 Connection 호출을 승인할 수
없습니다.

Runtime row는 process launch의 영속 관찰이지 liveness 기록이 아닙니다. Terminal failure나
graceful close를 기록하기 전에 종료된 process는 열린 것처럼 보이는 row를 남길 수
있습니다. 그 row는 이력 evidence로 남지만 이후 Guard event의 상관관계를 위해 선택하지
않습니다. 여러 managed process가 동시에 존재하면서 서로 다른 host session에 결속할 수
있습니다.

협상한 protocol version은 권위 있는 protocol data입니다. `clientInfo` name/version과
관찰한 host 실행 파일 version은 diagnostic 필드입니다. 제한 안의 미래 값도 받아들이며
client identity, host identity, compatibility, allowlist membership을 증명하지 않습니다.
Session은 실제로 기록한 협력적 protocol 동작만 증명합니다.

## 호출별 Session 권한

유효한 Guard 관찰은 MCP runtime을 아직 모르는 상태에서 프로젝트 `agent_sessions` row를
생성하거나 갱신할 수 있습니다. 이 row에는 조작한 값이나 sentinel runtime 좌표를 저장하지
않습니다. 동일한 Connection-bound host session identity를 운반한 첫 실제 managed
`tools/call`이 정확한 Registry runtime/project/host-session binding을 예약하고 그 runtime을
프로젝트 row에 붙입니다. 예약 뒤 프로젝트 쓰기가 중단되어도 동일한 호출을 재실행하면
attach를 안전하게 완료할 수 있습니다. CLI preflight는 이 binding을 수행하지 않습니다.

프로젝트 도구의 Core 호출 맥락을 만들기 전에 어댑터는 권위 있는 현재 Registry runtime
session, 정확한 `mcp_runtime_project_session_bindings` row, 프로젝트 `agent_sessions` row를
검증합니다. Connection이 존재하고 활성 상태여야 하며 프로젝트가 존재하고 Connection
Project로 남아 있어야 합니다. Runtime session은 해당 Connection 소유의 현재
`managed_host` session이어야 합니다. 프로젝트 session은 null이 아닌 runtime binding을
가지고 동일한 runtime, Connection, 프로젝트, host session에 속해야 합니다. 두 통합
revision은 현재 Connection과 프로젝트 입력에 일치해야 하며 현재 Connection mode가 요청한
operation category를 허용해야 합니다. 결속되지 않은 Guard-only session은 Guard 이력을
보관하지만 도구 호출을 승인할 수 없습니다.

Core는 직렬화할 수 없는 `ValidatedAgentSession` 하나를 받습니다. Connection ID는
`ActorSource::AgentConnection`과 정확히 같아야 하며 project ID는 모든 프로젝트 범위
호출과 일치해야 합니다. 감사용 `verification_basis`는 로컬에서
`connection:<connection_id>/session:<project_session_id>/revision:<project_integration_revision>`
형태로 만듭니다. 이 값은 운영 소유권 기록이며 certificate, receipt, identity proof,
trusted host digest가 아닙니다. 이전 권한 기록으로 fallback하지 않습니다.

## 도구 검색

| 모드와 저장소 | MCP에 보이는 도구 |
|---|---|
| `workflow`, 쓰기 가능 | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.get_operation_result`, `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_action`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task`, `volicord.list_projects` |
| `workflow`, 읽기만 가능 | `volicord.status`, `volicord.get_operation_result`, `volicord.request_user_action`(resume만), `volicord.check_close`, `volicord.list_projects` |
| `read_only`, 읽기 가능 | `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects` |
| 읽을 수 있는 허용 프로젝트 없음 | `volicord.list_projects` |

Task 상태와 이전 호출은 도구를 동적으로 추가하지 않습니다. 숨긴 mutation은 Core 효과
없이 실패합니다. `volicord.resolve_user_action`은 공개 Core API 메서드이지만 MCP 도구는
아닙니다.

## 공개 인자 projection

`tools/call`은 문자열 `params.name`과 선택적인 객체 `params.arguments`를 사용합니다.
공개 schema는 Core envelope, 내부 연결/프로젝트 ID, protocol metadata, idempotency
필드, actor source, operation category, verification basis를 숨깁니다. 숨긴 필드는 Core
전에 거부합니다. 간결한 검색 schema는 담당 문서의 완전한 요청 검증을 느슨하게 하지
않습니다.

<a id="mutation-authority-receipt-projection"></a>
## 응답 wrapping

읽기 전용 도구는 공개 메서드 결과를 structured content로 반환합니다. Mutation은
선택한 `summary`, `workflow`, `full` projection에 새 `AuthorityReceipt`, 정확한 효과
identity, replay 사실, 제한된 복구 정보를 담습니다. Text는 사람용 rendering이며 다른
권한 출처가 아닙니다.

Core 효과를 커밋한 뒤 전달이 실패하면 operation-result 좌표를 보존합니다. 응답 직렬화나
전송이 실패했다는 이유로 mutation을 다시 시도하지 않습니다.

## UserAction 요청

MCP 에이전트는 `volicord.request_user_action`으로 대기 요청을 만들거나 명시적인 읽기
전용 resume 분기를 사용할 수 있습니다. 나중에 현재 상태와 불변 CLI resolution
identity의 안전한 snapshot을 관찰할 수 있습니다. 비공개 inbox form, note, submission
identity, credential은 받지 않습니다.

어댑터는 요청에 답하거나 해결하지 않고 서버가 시작하는 resolution 요청도 보내지
않습니다. 사용자는 `volicord inbox resolve`로만 해결합니다. Guard prompt 관찰이
있다면 권한이 아닌 관찰로 남습니다.

## 종료와 재연결

EOF는 처리 중인 응답 뒤 loop를 닫고 graceful close를 기록합니다. 새 프로세스는 시작 검증과 MCP 초기화를 다시
수행하며 이전 프로세스의 연결, 프로젝트, session 권한, 현재 상태를 상속하지 않습니다.

## 관련 담당 문서

- [Agent Connection](agent-connection.md)
- [관리 CLI](admin-cli.md)
- [API 메서드](api/methods.md)
- [API UserAction 스키마](api/schema-user-action.md)
- [저장 효과](storage-effects.md)
- [보안](security.md)
