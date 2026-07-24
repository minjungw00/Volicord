# MCP 전송 참조

이 문서는 최초 릴리스의 로컬 MCP 프로세스 경계인 관리 stdio 시작, 엄격한 binding,
JSON-RPC lifecycle, 도구 검색, 공개 인자 projection, 응답 wrapping, 종료를 담당합니다.
Core 메서드, Codex 구성, 연결 검증, 저장 효과는 각각의 집중 담당 문서에 남습니다.

<a id="surface-stability"></a>
## 표면 안정성

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)을 사용합니다.

| 표면 | 안정성 |
|---|---|
| `volicord mcp serve`, 초기화, `tools/list`, `tools/call`, 응답 wrapping | `stable` |
| `volicord mcp preflight` 읽기 전용 점검 및 출력 계약 | `stable` |
| 권위 있는 runtime-session lifecycle milestone | `stable` |
| stable 프로세스와 메서드 집합에 나열하지 않은 pre-1.0 추가 표면 | `beta` |
| 숨겨진 관리 launcher, launch lease, 생성 구성 세부사항 | `internal` |
| Host 실행 파일 version, MCP client name/version, best-effort protocol metric | `diagnostic` |

## 프로세스 모델

관리 Codex 구성은 숨겨진 `volicord _host-launch codex` entry를 시작하며, 이 entry는
lease를 소비한 뒤 같은 프로세스에서 stdio adapter로 전환합니다. 공개
`volicord mcp serve` entry는 수동 stdio 표면입니다. 두 경로 모두 stdin과 stdout으로
줄 단위 JSON-RPC를 교환하며 TCP, HTTP, Unix domain socket 또는 그 밖의 네트워크
listener를 열지 않습니다.

```text
volicord mcp serve --connection <connection_id> [--project <project_id>]
volicord mcp serve --discover-repository --host codex
volicord mcp preflight --connection <connection_id> [--project <project_id>] [--verbose | --json]
volicord mcp preflight --discover-repository --host codex [--verbose | --json]
```

`--connection` 프로세스 형태는 현재 수동 실행이나 사전 점검에서 명시적인
`--project` 선택을 받을 수 있습니다. 정규 개인 Codex entry는 `--project` 없이 이
형태를 사용하며, 현재 프로젝트 연결 관계는 Store가 소유하는 Connection Project
membership으로 남습니다. 저장소 검색은 정규 공유 Codex entry 전용이며 정확한 Runtime
Home과 정규 Git 작업 트리에서 Connection과 프로젝트를 해결합니다. cwd만으로
연결을 추론하거나 주변 저장소를 검색하거나 다른 host selector를 받지 않습니다.
`mcp preflight`는 stdio loop에 들어가지 않고 정규 관리 entry를 검증하고 선택한
Registry, Connection, project, protocol profile, 도구 schema, host contract를 읽습니다.
쓰기 가능성 probe를 수행하지 않고 runtime session이나 finding을 만들지 않으며, 읽을 수
있는 read-only SQLite database와 filesystem에서 성공합니다. JSON projection은
`side_effects: []`, 증거 class `read_only_preflight`, 쓰기 가능성 `not_checked`와
`requires_active_verification`을 명시합니다.

## 환경과 시작

`VOLICORD_HOME`은 [런타임 경계](runtime-boundaries.md)에 따라 Runtime Home을
선택합니다. 정규 관리 시작 계약은 개인 구성에 선택한 절대값을 저장하고, 공유
구성에서는 머신 로컬 경로를 내장하지 않은 채 부모 환경 값만 전달합니다. 정확한
생성 형태와 엄격한 parsing은
[Agent Connection](agent-connection.md#managed-mcp-launch-contract)이 담당합니다.

Connection 사전 점검과 CLI stdio 핸드셰이크는 같은 binding 계약에서 공개 프로세스
시작을 파생합니다. 구체화 과정은 상속한 일반 프로세스 환경 변수를 보존하고 명시적인
process 구성만 적용합니다. 전달할 각 이름은
명시적인 검증 입력으로 해석합니다. 따라서 개인 연결 검증은 계약에 이미 들어 있는
정적 Runtime Home을 사용합니다. 공유 연결 검증은 저장소에 보이는 구성을 이식 가능한
형태로 유지하면서 작업이 선택한 Runtime Home을 전달 대상 `VOLICORD_HOME`으로 사용합니다.
공유 저장소 검색은 정규 Product Repository 루트에서 실행하고, 개인 연결 검증은 결속된
식별자를 사용하므로 작업 디렉터리를 통한 저장소 검색에 의존하지 않습니다.

MCP 요청을 읽기 전에 숨겨진 launcher는 정규 관리 구성에서 정확한 등록 Connection을
해결합니다. Connection 활성 상태, 선택한 프로젝트의 현재 membership,
Runtime Home/Product Repository 분리, 현재 `StorageManifest`, 필요한 저장 읽기 가능성을
검증합니다. 엄격한 현재 entry, integration revision, managed fingerprint가 일치한 뒤에만
한도가 있고 한 번 쓰는 launch lease를 만듭니다. 손상된 기록, 모호한 선택, 사용할 수 없는 저장소에는
[실패 모델](failure-model.md)을 적용합니다.

MCP bootstrap은 그 lease를 정확히 한 번 소비하고 같은 Store transaction에서
`managed_host` Registry runtime session을 만듭니다. Replay, 만료, 취소, Connection
불일치, revision 불일치, fingerprint 불일치가 있는 lease는 runtime을 만들지 않습니다.
공개 `mcp serve`는 항상 `manual_cli`를 만들며 공개 flag나 environment variable로
`managed_host`를 선택할 수 없습니다. 전용 integration probe는 `integration_probe`를
만듭니다. 읽기 전용 preflight는 runtime을 만들지 않습니다. Executable path, host
version, client version은
diagnostic으로 남으며, 관리 호출 권한은 아래의 현재 session 및 프로젝트 binding으로
성립합니다.

## MCP wire 동작

비어 있지 않은 stdin 각 줄은 완전한 UTF-8 JSON-RPC 2.0 요청 하나입니다. 잘못된
JSON은 `-32700`, 잘못된 요청은 `-32600`, 알 수 없는 메서드는 `-32601`, 잘못된
인자는 `-32602`, 내부 프로토콜 실패는 `-32603`을 반환합니다. 응답은 요청 `id`를
보존합니다.

`initialize` 요청과 `notifications/initialized` 알림은 각각 독립된 최상위 메시지여야
합니다. `initialize`가 `tools/list`와 `tools/call`보다 먼저 와야 합니다. Initialize 전
호출, 반복 initialize, 잘못된 lifecycle 작업은 Core 전에 실패합니다. 선택한 초기화
profile은 독립된 `notifications/initialized` 단계가 끝나기 전까지 협상 완료 상태가
아닙니다. 이 단계에서 session이 작업을 받을 준비가 될 때까지 최상위 batch는
거절합니다.

관찰 가능한 실패는 사람이 읽는 오류 문구를 분류하지 않고 공유 구조화 finding을
사용합니다. 현재 MCP code 계열은 다음과 같습니다.

| 계열 | 안정적인 code |
|---|---|
| JSON-RPC와 framing | `mcp.json_rpc.parse_error`, `mcp.json_rpc.invalid_request`, `mcp.json_rpc.invalid_id`, `mcp.json_rpc.unknown_method`, `mcp.json_rpc.malformed_response`, `mcp.json_rpc.framing_failure`, `mcp.json_rpc.message_size_exceeded`, `mcp.json_rpc.error_response` |
| Lifecycle | `mcp.lifecycle.initialize_required`, `mcp.lifecycle.duplicate_initialize`, `mcp.lifecycle.initialization_batch_forbidden`, `mcp.lifecycle.initialized_notification_missing`, `mcp.lifecycle.initialized_notification_invalid`, `mcp.lifecycle.operation_before_ready`, `mcp.lifecycle.invalid_shutdown_sequence` |
| Revision과 capability | `mcp.protocol.malformed_version`, `mcp.protocol.unsupported_version`, `mcp.protocol.counter_offer`, `mcp.protocol.counter_offer_rejected`, `mcp.protocol.generation_mismatch`, `mcp.protocol.capability_shape_invalid`, `mcp.protocol.schema_projection_failed` |
| 도구 검색 | `mcp.tools.protocol_error`, `mcp.tools.schema_failure`, `mcp.tools.required_missing`, `mcp.tools.definition_projection_invalid` |
| 도구 호출 | `mcp.tool_call.unknown_tool`, `mcp.tool_call.invalid_arguments`, `mcp.tool_call.protocol_error`, `mcp.tool_call.output_schema_failed`, `mcp.tool_call.response_budget_failed`, `mcp.tool_call.core_execution_failed`, `mcp.tool_call.adapter_execution_failed`, `mcp.tool_call.safe_read_only_failed`, `mcp.tool_call.session_correlation_invalid` |

협상 finding은 제한된 `requested_revision`, `selected_revision`,
`negotiated_revision`, `production_supported_revisions`, 시도한 `clientInfo`
name/version, JSON-RPC 오류 code, 안전한 오류 data, runtime session ID를 서로
다른 사실로 보존합니다. 요청, 선택, 협상 revision을 서로 대신 사용하지 않습니다.
전체 요청, 도구 인자, 환경, 제한 없는 프로세스 출력은 사실에 넣지 않습니다.

<a id="protocol-revision-negotiation"></a>
## Protocol revision 협상

프로덕션에서 지원하는 초기화 revision은 정확히 다음과 같습니다.

- `2024-10-07`
- `2024-11-05`
- `2025-03-26`
- `2025-06-18`
- `2025-11-25`

Revision을 프로덕션에서 지원하려면 명세 manifest가 해당 항목을 릴리스 상태이면서
pre-release 전용이 아닌 것으로 표시하고, schema artifact를 고정하고, 일치하는 프로덕션
protocol profile을 제공해야 합니다. 오프라인 명세 gate는 릴리스 상태이면서
`production_supported=true`인 manifest 항목과 `ProtocolRegistry`의 프로덕션 profile이
정확히 일치하도록 요구합니다. 실행 가능한 런타임 적합성은 이 metadata가 아니라 모든
프로덕션 profile에 registry 기반 protocol 적합성 테스트를 실행하여 독립적으로 확인합니다.
이 결과는 upstream 또는 제3자의 MCP 인증이 아닙니다. Pre-release revision은 추적을 위해
고정했다는 이유만으로 프로덕션 지원 대상이 되지 않습니다.

별도의 conformance coverage boolean이나 conformance revision 배열은 없습니다. 실행
coverage 집합은 `ProtocolRegistry::production().oldest_to_newest()`를 직접 순회한
결과입니다. `xtask`는 manifest 일치 검증을 위해 `volicord-mcp-protocol`을 통해 이
registry를 읽으며 `volicord-mcp` runtime adapter, Core, Store, platform crate에 의존하지
않습니다.

요청의 문자열 `protocolVersion`은 요청 revision입니다. 이 닫힌 집합의 정확한 구성원을
요청하면 같은 profile을 선택하고 initialize 결과도 같은 revision을 반환합니다. 초기화 기반
protocol 형태에 속하지만 이 집합에 없는 다른 문자열에는 서버의 선호 counter-offer인
`2025-11-25`를 반환합니다. 고정된 명세에 따라 client가 반환된 revision을 지원할 수 없으면
연결을 끊어야 합니다. 선택은 문자열이나 날짜 범위 비교가 아니라 정확한 registry membership을
사용하며 지원 집합은 사용자가 구성할 수 없습니다.

`protocolVersion`이 없거나 문자열이 아닌 경우, `capabilities`가 객체가 아닌 경우,
`clientInfo`가 잘못된 경우에는 제한된 error data와 함께 계속 `-32602` invalid params를
반환합니다. 고정된 pre-release `2026-07-28` revision은 discover 기반 generation에
속합니다. 따라서 이를 담은 initialize 요청은 초기화 counter-offer를 받지 않고 typed
method 또는 generation mismatch로 실패합니다.

유효한 인자를 해석한 뒤 활성 MCP 연결은 session 범위의 typed selection 하나를 소유합니다.
이 값은 정확한 요청 문자열, 선택한 profile, exact match 또는 counter-offer 결과, client
capability, 제한 안의 시도된 client name/version, initialized notification 완료 사실을
보관합니다. 선택한 profile에서 initialize 응답의 `protocolVersion`과 capability를 만들고
이후 lifecycle을 검증합니다. 유효한 initialize 요청 뒤 profile이 선택되지만 유효한
initialized notification이 handshake를 완료한 뒤에만 그 revision의 협상이 끝납니다.

Volicord는 선택한 profile이 허용할 때만 `tools` server capability를 광고하며, 지원하는
다섯 profile은 모두 이 필드를 허용합니다. 초기화가 끝난 뒤에는 session 상태에 저장된
profile에 따라 작업 단계의 JSON-RPC batch 동작을 정확히 다음과 같이 결정합니다.

| 선택한 profile | 작업 단계의 batch 요청과 응답 |
|---|---|
| `2024-10-07` | 허용하지 않음 |
| `2024-11-05` | 허용하지 않음 |
| `2025-03-26` | 허용함 |
| `2025-06-18` | 허용하지 않음 |
| `2025-11-25` | 허용하지 않음 |

이 값은 revision 이름의 시간 순서나 batch 내용에서 추론하지 않고 검토된 profile 사실을
사용합니다. 모든 지원 revision에서 초기화 batch를 금지합니다. `initialize` 또는
`notifications/initialized`를 포함한 batch는 항목을 하나도 처리하지 않은 채 거절합니다.
그 밖의 batch도 session이 작업을 받을 준비가 되기 전에 도착하면 선택 또는 협상 revision을
바꾸거나 도구 관찰을 기록하지 않고 거절합니다. 준비가 끝나면 `2025-03-26`은 session에
이미 선택된 profile에 따라 작업 batch를 허용하고, 다른 모든 production profile은 이를
거절합니다. 허용한 batch는 입력 순서대로 하나씩 처리합니다. 알림에는 응답 항목을 만들지
않으며 알림만 있는 batch는 응답을 만들지 않습니다.

## CLI Conformance 및 Host 호환성 Probe

실행 가능한 protocol 적합성 테스트와 CLI server conformance probe는 모두
`ProtocolRegistry`의 프로덕션 profile을 결정론적 순서대로 직접 순회합니다. 따라서
프로덕션 profile을 추가하면 별도 revision 선언 없이 각 일반 matrix에 자동으로 들어갑니다.
집중 실행 case는 독립된 `initialize`, `notifications/initialized`, `tools/list`, 고정 schema
및 필수 도구 검증, 정확한 지정 왕복 도구, revision별 정의와 결과 projection, 초기화 batch
거절, profile에 따른 작업 단계 batch 허용 또는 거절, 잘못된 lifecycle 동작, EOF/종료를
다룹니다. 모든 프로덕션 profile에서 초기화 batching을 거절합니다.

Connection 검증은 새로운 일회용 Runtime Home과 Product Repository 안에서 프로덕션
profile마다 별도 `mcp serve` process와 정확한 요청을 사용합니다.
각 probe는 `initialize`, `notifications/initialized`, `tools/list`, 해당
revision의 고정 schema 검증, 현재 mode의 필수 도구 검증,
`ToolVerificationRole::ManagedHostRoundTrip`이 선택한 도구 호출 정확히 하나, 정상
EOF/종료를 완료합니다. 현재 role 담당 도구는 정확히 `volicord.list_projects`입니다.
Revision별로 요청 및 협상 revision, 반환된 도구, 완료 단계, typed failure를 기록합니다.
모든 프로덕션 profile이 통과해야 집계 server check가 통과하며, profile 하나가 실패해도
나머지 profile probe를 계속 실행합니다.

Host 호환성은 protocol registry projection도 전체 revision 매트릭스의 대체물도 아닌,
host가 독립적으로 소유하는 fixture 목록입니다. 현재 `codex` fixture는
`clientInfo.name`이 `codex-mcp-client`이고 title이
`Codex`이며 현재의 빈 capability 객체를 사용하는 검토된 Codex initialize 요청 형태와,
독립적으로 고정한 revision `2025-06-18`을 사용합니다. 도구 호출 하나에는 유효한
`codex-mcp-turn-metadata` session/thread/turn metadata를 담습니다. 이 fixture ID는
검토한 wire contract를 가리키며 Codex package version identity가 아닙니다. `tools/list`와
`ToolVerificationRole::ManagedHostRoundTrip`이 선택한 도구(현재
`volicord.list_projects`)를 실행하며, 요청 revision을 서버의 선호 또는 최신 profile에서
파생하지 않습니다.

두 matrix 모두 CLI probe 증거입니다. 일회용 `manual_cli` 또는 `integration_probe`
runtime source는 managed check에서 제외되고 검증 fixture와 함께 제거됩니다. Host 호환성 fixture 통과는 검토된 요청 형태가 이
서버에서 동작함을 보여 주지만 관리 Codex process가 실행되었음을 보여 주지는 않습니다.
Launch lease 소비에 성공해 source가 `managed_host`로 생성된 runtime의 lifecycle 관찰만
managed-host 운영 check를 충족할 수 있습니다.

## Semantic Codex Host 계약

관리 Codex wire 입력은 명시적으로 선택한 host contract profile로 decode합니다.
`CodexMcpTurnMetadata` marker는 `codex-mcp-turn-metadata`를 선택합니다. 이 profile은
`tools/call` `_meta`를 소유하며 native session, thread, turn과 최상위 `threadId` 및 중첩
`x-codex-turn-metadata.thread_id`의 동일성을 요구합니다. 별도
`CodexCommandHooks` marker는 `codex-command-hooks`를 선택하며 command-hook envelope를 따로
소유합니다. `UserPromptSubmit` 상관관계는 session과 turn이고, `PreToolUse` 및
`PostToolUse` 상관관계는 session, turn, tool-use ID, 정규 tool name입니다.
Command-hook 상관관계에는 thread 좌표가 없습니다.

두 profile은 payload 모양에서 추론하지 않으며 typed 상관관계를 서로 바꾸어 쓸 수 없습니다.
둘 다 알 수 없는 추가 field를 허용하지만 계약이 소유하는 presentation 값과 tool 값만 한도
안에서 보관하고, 전체 입력을 유지하지 않는 한도 있는 typed failure를 반환합니다. 관리 MCP는
파싱한 session과 thread가 등록된 managed runtime binding과 일치하는지도 요구합니다.
`volicord-host-contract`가 두 marker, 결정적인 profile identity와 digest, source별
correlation type을 담당합니다. `tests/conformance/codex-host/` 아래의 검토된 fixture와
coverage manifest 및
`crates/volicord-host-contract/tests/host_contracts.rs`의 parser/checksum assertion은
고정 계약 입력이며 protocol revision이나 package version 주장이 아닙니다.

MCP 등록은 명시적인 `McpServerKey`를 제공하고 `AgentToolId`는 완전한
`McpRawToolName`을 제공합니다. `McpToolIdentity`는 두 좌표를 보존하며
`CodexMcpCallableNames`는 이를 `codex-mcp-callable-names`의 검증된
`HostCallableIdentity`로 투영합니다. 이 투영은 server key와 완전한 raw name을 각각
정규화하고 Codex separator로 결합하며 64-byte callable 한도를 적용합니다. 서로 다른
source가 이 한도를 넘으면 현재의 결정적 12자리 16진수 SHA-1 source-identity suffix로
이름을 맞춥니다. 이 suffix는 이름 길이 조정 규칙이며 무결성 주장이 아닙니다. 그렇게
투영한 뒤에도 서로 다른 source가 같은 callable이 되면 catalog를 거부합니다. Dotted raw tool name에서
server identity를 추출하지 않습니다. `McpToolCatalog`만 역방향 해석의 근거이므로
underscore나 구두점에서 namespace 경계를 추측하지 않습니다. Adapter는 이 semantic
계약을 직접 선택하며 관찰한 Codex package version은 callable 투영을 제어하지 않습니다.

같은 semantic 담당자가 `HostHookMatcherStrategy`를 정의합니다. 현재 Codex tool
hook에서 검토된 strategy는 native Guard host tool과 server-qualified MCP routing의
합집합입니다. 한도가 있는 callable 표현이 등록된 `McpServerKey` namespace를 보존하면
그 namespace를 사용하고, 그렇지 않으면 같은 정규 catalog에서 exact callable token을
파생합니다. Matcher JSON은 이 typed 값에서만 생성하고 엄격한 구성 검증은 이를 다시
같은 값으로 parsing합니다. Routing은 semantic tool 수락이 아닙니다.
Wrapper에서 `McpToolCatalog`가 계속 정확한 callable 해석을 수행하며, 해석된
`AgentToolId`만 Guard probe 역할을 선택할 수 있습니다. 숫자 host-version branch는
strategy를 바꾸지 않습니다.

## 권위 있는 Lifecycle 기록

`mcp serve` 또는 숨겨진 launcher process는 Agent Connection을 해결한 뒤 thread
metadata를 검증하거나 protocol message를
읽기 전에 Registry runtime session을 만듭니다. 이 row는 Volicord가 생성한 process
launch, Connection, 정확한 `managed_host`, `manual_cli`, `cli_preflight`,
`integration_probe` 중 하나의 source, 현재 Connection 통합 revision, process ID,
process 시작 시각을 식별합니다. 원자적인 launch-lease 소비만 `managed_host`를 만들며
나머지 source는 모두 managed-host 운영 check와 권한에서 제외됩니다.

어댑터는 한도가 있는 `clientInfo.name`, `clientInfo.version`, `protocolVersion`을 파싱하는
즉시 시도된 client와 요청 revision으로 영속 기록하며, 이후 initialize 검증이 실패해도 이
관찰을 유지합니다. Initialize가 성공하면 완료와 server가 선택한 profile revision을 그
revision을 반환하기 전에 기록합니다. 선택 값은 유효한 `notifications/initialized`가
handshake를 완전히 끝낼 때만 협상 revision이 됩니다. 실제 `tools/list` 응답도 반환하기
전에 매번 기록하며 정규 정렬한 정확한 반환 도구 identity를 함께 보관합니다. Discovery
사실은 생성된 그 응답에 현재 Connection mode가 요구하는 모든 도구가 있었는지를 나타내고,
검증에 성공하면 별도 `required_tools_validated_at` milestone을 기록합니다. 중복 initialized
notification은 첫 번째 유효 관찰 뒤 멱등이며 협상한 revision을 바꿀 수 없습니다.

`ToolVerificationRole::ManagedHostRoundTrip`에 결합된 정확한 도구의 `tools/call`이
성공한 경우에만 managed-host 왕복 evidence를 기록할 수 있습니다. 이 role은 컴파일
시점에 `AgentToolId::LIST_PROJECTS`에 결합되고, 그 wire 이름 투영은
`volicord.list_projects`입니다. 호출은 같은 session의 required-tool validation 뒤에
실행되어야 하고, 현재 enabled `managed_host` runtime과 Connection revision에 속하며 유효한
현재 관리 Codex session/thread/turn correlation을 담아야 합니다. 또한
JSON-RPC error나 tool error 없이 완료되어야 합니다. 그러면 Store는 도구 결과를 내보내기
전에 정확한 `verification_tool_name`과 `verification_tool_observed_at` 쌍을 원자적으로
기록합니다. 성공한 `volicord.status`, `volicord.get_operation_result`,
`volicord.check_close` 호출은 이 쌍을 갱신하지 않습니다. 지정 호출이 실패하거나 거부된
경우에도 이 쌍은 없는 상태로 남습니다. 관찰할 수 있는 fatal transport 또는 protocol failure는 한도가 있는 공유
`DiagnosticFinding` 하나를 만들고 그 finding ID를 runtime session의 terminal finding으로
원자적으로 연결합니다. EOF에 따른 graceful close는 이와 함께 있을 수 없는 별도 terminal
사실입니다. 권위 있는 Store 쓰기가 실패하면 해당 protocol 성공을 내보내지 않습니다.
`diagnostics.sqlite`의 제한된 쓰기는 계속 best effort이며 이 사실을 조회할 때 사용하지
않습니다.

Registry를 열기 전에 발생한 실패는 stderr에 한도가 있는
`VOLICORD_DIAGNOSTIC_V1` envelope 하나를 출력합니다. Registry runtime session이
생긴 뒤에는 정확한 Connection, integration revision, runtime-session 좌표와 함께
finding을 영속합니다. Terminal 실패는 두 번째 자유 형식 실패 객체로 저장하지 않고
finding ID로 연결합니다.

연결 검증은 선택한 Runtime Home에 읽기 전용 preflight를 수행한 뒤 일회용 명령별
fixture 안에서만 수동 stdio probe process를 시작합니다. 같은 정규 role 담당 도구,
현재 `volicord.list_projects`를 안전한 읽기 전용 self-test 왕복으로 호출합니다. Fixture
process는 server 표면을 검증하지만 그 lifecycle 사실은 `managed_host` 운영 check를
충족하거나 Connection 호출을 승인할 수 없습니다. 선택한 사용자 Runtime Home에는
session이나 finding을 만들지 않습니다. CLI 검증이 성공해도 관리 `host_session`,
`tools/list`, 도구 왕복 관찰을 꾸며내지 않습니다.

Runtime row는 process launch의 영속 관찰이지 liveness 기록이 아닙니다. Terminal finding 연결이나
graceful close를 기록하기 전에 종료된 process는 열린 것처럼 보이는 row를 남길 수
있습니다. 그 row는 이력 evidence로 남지만 이후 Guard event의 상관관계를 위해 선택하지
않습니다. 여러 managed process가 동시에 존재하면서 서로 다른 host session에 결속할 수
있습니다.

요청 revision은 client에서 받은 값이고, 선택 revision은 server가 반환하거나 선택한 값이며,
협상 revision은 handshake가 완전히 끝난 뒤에만 그 선택 값이 됩니다. 협상한 protocol version만 권위
있는 runtime-session protocol data입니다. `clientInfo` name/version과 관찰한 host 실행
파일 version은 diagnostic 필드입니다. 제한 안의 미래 값도 받아들이며 호환성은 현재 관리
구성과 현재 revision의 `managed_host` runtime 하나에서 관찰한 초기화, 도구 목록, 필수
도구, 안전 호출, Guard 동작으로 판단합니다. 검증은 가장 최신 managed runtime을 현재
health를 나타내는 `latest_managed_attempt`로 선택하고, 같은 session의 전체 chain을 완료한
가장 최신 runtime을 별도로 `latest_managed_capability_proof`로 선택합니다. Runtime 사이의
milestone을 조합하지
않습니다. 실제 protocol peer는 선택된 runtime의 `clientInfo`입니다. 별도로 probe한
executable path와 `codex --version`은 설치 및 수동 호출을 돕는 정보일 뿐 protocol peer의
권위 있는 값이 아닙니다. Peer/PATH version 불일치는 warning evidence가 될 수 있지만 완전한
managed session 자체를 무효화하지 않습니다. 이 기록은 협력적이며 client, host, actor,
human identity를 성립시키지 않습니다.

## 호출별 Session 권한

유효한 Guard 관찰은 정규화한 프로젝트 `host_sessions`와 `host_turns` row를 만듭니다.
Tool phase는 `host_tool_invocations`도 만듭니다. Guard는 `managed_mcp_sessions`를 만들지
않고, 열린 process row에서 runtime을 선택하지 않으며, thread를 합성하지 않습니다. 실제
프로젝트를 선택하기 전 MCP runtime 상태는 정확한 `CodexMcpCorrelation`만 유지하고
Connection만으로 내부 session 좌표를 도출하거나 검색하지 않습니다. 프로젝트를 선택한 뒤
Store가 현재 프로젝트 통합 revision을 결정하고 Connection, 그 정확한 revision, native
session으로 프로젝트 Agent Session ID를 도출합니다.

첫 실제 managed `tools/call`에서 Store는 먼저 현재 managed runtime을 변경 없이 검증하고,
정규화한 host session과 turn을 만든 뒤, 정확한 Connection, native session, thread,
프로젝트 revision, 현재 Guard 소유권, 최신 turn에 맞는 MCP 전용
`managed_mcp_sessions` anchor를 만들거나 검증합니다. 이 프로젝트 transaction이 commit된
뒤에만 Store가 현재 소유자 사실을 다시 검증하고 정확한 Registry
runtime/project/revision/host-session binding을 예약합니다. 마지막 프로젝트 transaction은
같은 anchor에 그 runtime을 붙입니다. 따라서 결정적인 프로젝트 소유권 충돌은 Registry
예약을 만들지 않습니다. 결속되지 않은 MCP anchor는 권한이 아니며, 일치하는 프로젝트
attach가 없는 Registry 예약도 권한이 아닙니다. 마지막 프로젝트 쓰기가 중단되면 소유자
상태가 바뀌지 않은 동일 호출이 예약을 재사용해 attach를 완료합니다. CLI preflight는 이
binding을 수행하지 않습니다.

따라서 프로젝트 Agent Session 검증은 Registry runtime 예약보다 먼저 수행됩니다. 권한에는
완료된 프로젝트 attachment와 정확히 완료된 Registry binding이 모두 필요합니다.

프로젝트 도구의 Core 호출 맥락을 만들기 전에 어댑터는 권위 있는 현재 Registry runtime
session, 정확한 `mcp_runtime_project_session_bindings` row, 그 `host_sessions` 소유자와
join한 프로젝트 `managed_mcp_sessions` row를 검증합니다. Connection이 존재하고 활성
상태여야 하며 프로젝트가 존재하고 Connection
Project로 남아 있어야 합니다. Runtime session은 해당 Connection 소유의 현재
`managed_host` session이어야 합니다. 프로젝트 session은 null이 아닌 runtime binding을
가지고 동일한 runtime, Connection, 프로젝트, host session에 속해야 합니다. Registry
binding revision과 프로젝트 row revision이 서로 같아야 하고, 두 통합 revision은 현재
Connection과 프로젝트 입력에 일치해야 합니다. 현재 Connection mode가 요청한 operation
category를 허용해야 합니다. Hook 전용 host row는 Guard 이력을 보관하지만 도구 호출을
승인할 수 없습니다. 실제 mode 전환마다 Store 소유 Connection
integration generation이 증가하므로, 이전 모든 generation의 runtime session, 프로젝트
Agent Session, Guard event는 나중에 Connection이 같은 mode 값으로 돌아가더라도 이력으로
남습니다.

Core는 직렬화할 수 없는 `ValidatedAgentSession` 하나를 받습니다. Connection ID는
`ActorSource::AgentConnection`과 정확히 같아야 하며 project ID는 모든 프로젝트 범위
호출과 일치해야 합니다. 감사용 `verification_basis`는 로컬에서
`connection:<connection_id>/session:<project_session_id>/revision:<project_integration_revision>`
형태로 만듭니다. 이 값은 감사 event에 기록된 검증된 운영 소유권의 결정적인 로컬
lifecycle 및 상관관계 좌표입니다.

## 도구 검색

| 모드와 저장소 | MCP에 보이는 도구 |
|---|---|
| `workflow`, 쓰기 가능 | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.get_operation_result`, `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_action`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task`, `volicord.list_projects`, `volicord.begin_integration_verification`, `volicord.guard_probe`, `volicord.get_integration_verification` |
| `workflow`, 읽기만 가능 | `volicord.status`, `volicord.get_operation_result`, `volicord.request_user_action`(resume만), `volicord.check_close`, `volicord.list_projects`, `volicord.begin_integration_verification`, `volicord.guard_probe`, `volicord.get_integration_verification` |
| `read_only`, 읽기 가능 | `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects`, `volicord.begin_integration_verification`, `volicord.guard_probe`, `volicord.get_integration_verification` |
| 읽을 수 있는 허용 프로젝트 없음 | `volicord.list_projects` |

Task 상태와 이전 호출은 도구를 동적으로 추가하지 않습니다. 숨긴 mutation은 Core 효과
없이 실패합니다. `volicord.resolve_user_action`은 공개 Core API 메서드이지만 MCP 도구는
아닙니다.

`AgentToolId`는 모든 Agent Connection MCP 도구의 정규 typed identity이자 catalog입니다.
Core 소유 identity는 `MethodName`을 재사용하고, adapter utility와 Connection-integration
도구도 같은 폐쇄형 16개 도구 catalog에 속합니다. 각 identity는 안정적인 MCP wire 이름
투영, category, Connection mode별 가용성, Core method, adapter utility 또는
Connection-integration 소유권, 멱등성, 선택적 운영 verification role을 소유합니다.

정규 도구 registry는 각 정의를 `AgentToolId`로 식별하고 설명, 간결한 입력 schema,
간결한 출력 schema, annotation, 현재 값이 있는 선택적 표시 및 metadata를 제공합니다.
`tools/list`는 선택한 session profile을 통해 identity의 wire 이름을 투영합니다. Volicord는
revision마다 별도 도구 registry나 server 구현을 두지 않습니다. Connection mode와 저장
capability는 위 표에 따라 도구를 숨길 수 있지만 protocol revision은 계속 보이는 도구의
이름을 바꾸거나 다른 도구로 대체하지 않습니다.

`ToolVerificationRole::ManagedHostRoundTrip`은 컴파일 시점에
`AgentToolId::LIST_PROJECTS`에 결합됩니다. MCP runtime, 관리 CLI, Store 관찰, 진단 비교는
이 identity를 함께 사용하고 wire 또는 영속 이름 경계에서만 `volicord.list_projects`를
투영합니다.

| 선택한 profile | 현재 Volicord 도구마다 내보내는 필드 |
|---|---|
| `2024-10-07`, `2024-11-05` | `name`, `description`, `inputSchema` |
| `2025-03-26` | `name`, `description`, `inputSchema`, `annotations` |
| `2025-06-18`, `2025-11-25` | `name`, `description`, `inputSchema`, `outputSchema`, `annotations` |

후기 profile은 `title`과 `_meta`도 허용하고 `2025-11-25`는 `execution`과 `icons`도
허용합니다. 정식 registry가 값이 있는 필드를 소유할 때만 이 필드를 내보냅니다. 현재
Volicord registry는 해당 값을 소유하지 않으므로 값을 꾸며내지 않고 필드를 생략합니다.

## 공개 인자 projection

`tools/call`은 문자열 `params.name`과 선택적인 객체 `params.arguments`를 사용합니다.
공개 schema는 Core envelope, 내부 연결/프로젝트 ID, protocol metadata, idempotency
필드, actor source, operation category, verification basis를 숨깁니다. 숨긴 필드는 Core
전에 거부합니다. 간결한 검색 schema는 담당 문서의 완전한 요청 검증을 느슨하게 하지
않습니다.

<a id="in-chat-integration-verification-schemas"></a>
### 채팅 내 통합 검증 스키마

정규 사용자 수준 activation step은 `request_integration_verification`이며 사용자가
`codex_chat`으로 시작하고 agent가 실행합니다. Nested sequence는
`Run the Volicord integration verification.` 요청에서 시작합니다. Agent는
`volicord.list_projects`로 정확한 프로젝트를 선택한 뒤
`volicord.begin_integration_verification`을 호출합니다. 반환된 tagged `workflow`를
따릅니다. `awaiting_probe`와 `awaiting_observation`은 호출할 정확한 정규 `tool`을
담고, `complete`와 `repair_required`는 tool이 없는 terminal 상태입니다. Begin,
probe, status는 모두 같은 상태 지향 계약을 노출합니다. 현재 Codex semantic host
profile은 status read를 한 번만 허용하는 synchronous observation을 사용하므로 agent는
shell sleep이나 poll loop를 사용하지 않고 같은 turn에 자동으로 다시 시작하지 않습니다.
Guard probe는 내부 nested tool step이며 최상위 사용자 action이 아닙니다. 이 first-party
sequence만 현재 managed MCP와 Guard 상관관계 근거를 만들 수 있습니다.

Volicord tool이 노출되지 않으면 agent는 managed MCP connection이 unavailable이라고
보고합니다. Raw stdio를 시작하거나 Codex `_meta`를 직접 만들거나 `resources/list`,
resource template, CLI preflight, connection status를 managed tool availability의
proof로 취급하지 않습니다. 이 표면은 읽기 전용 diagnostic으로 남습니다. Hook review와
project/configuration trust는 user/host가 소유합니다. `volicord connection verify`는
선택적인 active diagnostics이며 managed-host sequence를 대신하지 않습니다.

이 sequence는 읽기 전용 project discovery와 세 Connection-integration 도구를
결합합니다. 네 도구 모두 MCP adapter 작업이며 Core method나 Task 작업 흐름이 아닙니다.
세 integration 도구는 정확한 현재 managed-host 좌표 안에서 멱등이고 공개 형태는 다음과
같습니다.

| Tool | 정규 annotation | 직접 효과 |
|---|---|---|
| `volicord.list_projects` | `readOnlyHint=true`, `destructiveHint=false`, `idempotentHint=true`, `openWorldHint=false` | Connection project allowlist를 읽으며 쓰지 않습니다. |
| `volicord.begin_integration_verification` | `readOnlyHint=false`, `destructiveHint=false`, `idempotentHint=true`, `openWorldHint=false` | 현재 semantic 좌표에 대한 불변 Registry verification attempt 하나를 만들거나 재개합니다. Core, Task, Product Repository에는 효과가 없습니다. |
| `volicord.guard_probe` | `readOnlyHint=false`, `destructiveHint=false`, `idempotentHint=true`, `openWorldHint=false` | 정확한 active run을 first-write-wins 방식으로 acknowledge하고 현재 공유 workflow 상태를 반환합니다. 정확한 replay는 효과를 반복하지 않고 현재 terminal 또는 nonterminal 상태를 반환합니다. Core, Task, project state, Product Repository에는 효과가 없습니다. |
| `volicord.get_integration_verification` | `readOnlyHint=false`, `destructiveHint=false`, `idempotentHint=true`, `openWorldHint=false` | host policy가 허용하는 bounded status read를 최대 한 번 소비하고 상관관계 phase 상태를 투영하며 terminal typed repair를 영속할 수 있습니다. Core, Task, project state, Product Repository에는 효과가 없습니다. |

이 annotation은 도구 자체를 설명합니다. 일반 호환 Guard event 영속과 뒤따르는 Registry
correlation refresh에는
[저장 효과](storage-effects.md#connection-integration-verification-effects)가 정의한 별도
효과가 그대로 적용됩니다. 네 도구 모두 Codex project trust나 hook review 상태를
변경하지 않습니다.

```yaml
volicord.begin_integration_verification:
  arguments:
    project_selector?: string
  result:
    verification_id: GuardIntegrationVerificationId
    workflow: IntegrationVerificationWorkflowState
    matched_prompt_event_id: GuardEventId

volicord.guard_probe:
  arguments:
    verification_id: GuardIntegrationVerificationId
  result:
    verification_id: GuardIntegrationVerificationId
    workflow: IntegrationVerificationWorkflowState

volicord.get_integration_verification:
  arguments:
    verification_id: GuardIntegrationVerificationId
  result:
    verification_id: GuardIntegrationVerificationId
    workflow: IntegrationVerificationWorkflowState
    guard_phases:
      prompt_capture: pending | matched
      pre_tool: pending | matched
      post_tool: pending | matched
    matched_prompt_event_id?: GuardEventId
    matched_pre_tool_event_id?: GuardEventId
    matched_post_tool_event_id?: GuardEventId

IntegrationVerificationWorkflowState:
  awaiting_probe:
    kind: awaiting_probe
    tool: volicord.guard_probe
  awaiting_observation:
    kind: awaiting_observation
    tool: volicord.get_integration_verification
    acknowledged_at: UtcTimestamp
    remaining_status_reads: u8
  complete:
    kind: complete
    completed_at: UtcTimestamp
  repair_required:
    kind: repair_required
    reason: hook_event_not_observed | hook_payload_incompatible |
      callable_identity_mismatch | verification_id_mismatch |
      session_mismatch | turn_mismatch | tool_use_mismatch |
      integration_revision_changed | hook_definition_changed |
      policy_changed | observation_deadline_exceeded
    retry_policy: no_automatic_retry | new_turn_required |
      host_reload_required | hook_review_required | repair_required
    finding: { code: string, summary: string }
```

`project_selector`는 일반 Connection 프로젝트 선택 규칙을 따르므로 선택이 모호하지 않을
때만 생략할 수 있습니다. Begin은 실제 현재 managed runtime과 native session/turn에
결속하고 현재 호환 prompt-capture event를 요구하며 Connection, project, managed runtime
session, native host session과 turn, integration revision, Guard Installation, semantic
host-contract profile, hook-definition digest, policy digest로 이루어진 좌표에 attempt 하나만
생성하거나 재개합니다. 좌표는 terminal 상태에서도 불변이고 유일하며 시간이 지났다는
이유로 새 attempt를 만들지 않습니다. `manual_cli`, `cli_preflight`,
`integration_probe` runtime 증거는 받지 않습니다. Store가 담당하는 단일 투영은
acknowledge 전 attempt를 `awaiting_probe`, acknowledge된 attempt를
`awaiting_observation`, 성공을 `complete`, acquisition 또는 owner 검사 실패를 별도의
typed reason과 retry policy가 있는 `repair_required`로 변환합니다. Tool reference는
정규 `AgentToolId` wire 투영을 사용하며 임의 tool 문자열을 받지 않습니다.

Probe acknowledgement는 verification ID, Connection, managed runtime session, native host
session, native host turn으로 이루어진 좌표에서 first-write-wins입니다. 적격인 첫 active
호출이 `probe_acknowledged_at`을 기록합니다. 동일한 replay는 권위 있는 acknowledgement
시각을 포함한 같은 `awaiting_observation` 상태를 반환합니다. 완료 또는 repair 뒤의
정확한 replay는 terminal 상태를 그대로 반환합니다. 다른 caller 좌표는 acknowledgement를
노출하지 않고 거부하며, acknowledgement가 없는 terminal attempt에는 뒤늦게 값을 만들 수
없습니다. Probe는 terminal attempt를 다시 active로 만들지 않으며 Core 상태, Task 상태,
Product Repository 파일을 변경하지 않습니다. Get은 semantic
`HookObservationPolicy`를 따릅니다. 검토된 현재 Codex 계약은
`Synchronous { allowed_status_reads: 1 }`입니다. 그 한 번의 read는 완료를 관찰하거나 가장
정확한 `repair_required` acquisition 또는 correlation reason을 영속합니다. 나중에 정의할
Deferred profile은 Codex version threshold가 아니라 명시적인 deadline policy를 가집니다.

Run은 같은 run session과 turn에서 호환 prompt event 뒤에 같은 tool-use ID, 정확히 생성된
host tool 이름 `mcp__volicord__volicord_guard_probe`, 정확한 `verification_id` 입력을 가진
`PreToolUse`와 `PostToolUse`가 있을 때만 통과합니다. Guard Installation, policy hash,
integration revision, hook-contract digest, managed runtime은 현재 상태를 유지해야 하며 event
시각은 prompt가 pre-tool보다 앞서거나 같고 pre-tool이 post-tool보다 앞서야 합니다. 이력,
무관한 관찰, 오래된 관찰, 불일치 관찰은 attempt를 충족할 수 없습니다. Cleanup expiry는
보관 기록의 범위만 제한하며 synchronous observation을 판단하거나 같은 좌표의 retry를
허용하지 않습니다.

<a id="mutation-authority-receipt-projection"></a>
## 응답 wrapping

성공한 모든 Core 호출은 먼저 정식 공개 메서드 결과 객체 하나를 만듭니다. 선택한 profile은
그 객체나 Core 의미를 바꾸지 않고 다음과 같이 MCP carrier만 고릅니다.

| 선택한 profile | 권위 있는 결과 carrier |
|---|---|
| `2024-10-07` | `toolResult`; 이 revision에는 표준 `isError` 필드가 없음 |
| `2024-11-05`, `2025-03-26` | 첫 `content` 항목의 JSON text와 `isError` |
| `2025-06-18`, `2025-11-25` | `structuredContent`, 호환용 `content`, `isError` |

Text 전용 profile에서는 첫 text 항목이 권위 있는 JSON 객체입니다. 그 뒤 text 항목은
호환용 rendering이며 다른 권한 출처가 아닙니다. Structured-content profile에서는 그
객체를 해당 도구가 광고한 정확한 간결한 `outputSchema`로 검증합니다. Core 전 어댑터
거절도 같은 revision carrier를 사용하며 `isError`를 쓸 수 없는 revision에서도 구조화된
오류 코드와 재시도/부작용 flag를 보존합니다.

Mutation은 선택한 `summary`, `workflow`, `full` 공개 projection에 새
`AuthorityReceipt`, 정확한 효과 identity, replay 사실, 제한된 복구 정보를 그대로
담습니다. 응답 크기 계산과 간결한 복구는 실제 선택 profile의 carrier를 사용합니다.
이 계산은 재시도 규칙, Core 효과, 권위 있는 공개 결과 본문을 바꾸지 않습니다.

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
