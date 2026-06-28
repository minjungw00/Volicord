# MCP 전송 참조

이 문서는 로컬 `volicord-mcp` 프로세스 계약을 담당합니다. 여기에는 프로세스 시작, 프로세스 환경, MCP 프로토콜 버전 협상, 초기화 수명주기, stdio 전송 프레이밍, JSON-RPC 메시지 검증, Agent Connection에 묶인 시작 검증, MCP 응답 래핑, 종료와 재연결 동작이 포함됩니다.

공개 Volicord API 메서드 동작, 공개 요청/응답 스키마, Agent Connection 의미, 저장소 기록 배치, 보안 보장, Core 권한 의미는 이 문서가 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `volicord-mcp` 프로세스 시작과 종료 동작
- Agent Connection에 묶인 시작에 필요한 필수 및 선택 프로세스 설정
- MCP Runtime Home 경로 해석
- MCP 프로토콜 버전 협상과 초기화 수명주기
- stdio JSON-RPC 프레이밍, 메시지 검증, 지원되는 MCP 메서드
- 하나의 `connection_id`에 대한 MCP 시작 검증
- 전송 경계에서의 MCP `tools/list`, `tools/call`, `volicord.list_projects` 어댑터 유틸리티 동작
- MCP `tools/call` 응답 래핑
- 프로세스 종료와 재연결 동작

이 문서는 담당하지 않습니다.

- 공개 Volicord 메서드 목록이나 메서드 담당 표: [API 메서드](api/methods.md)
- 공개 Volicord 요청/응답 스키마: [API 코어 스키마](api/schema-core.md)
- Agent Connection, Connection Projects, 프로젝트 선택 의미, 현재 연결 맥락, 행위자 출처: [Agent Connection](agent-connection.md)
- 관리 Runtime Home, Agent Connection 설정, 프로젝트 멤버십, 검증 명령: [관리 CLI](admin-cli.md)
- 저장소 배치, 마이그레이션, 저장 효과: [저장소](storage.md)가 안내하는 저장소 담당 문서

## 프로세스 모델

`volicord-mcp`는 로컬 MCP stdio 프로세스입니다. MCP 호스트는 이를 자식 프로세스로 시작하고 stdin/stdout으로 통신합니다. TCP 리스너, HTTP 리스너, Unix-domain socket 리스너, 또는 그 밖의 네트워크 리스너가 아닙니다.

기준 명령줄 동작:

- stdio 루프는 `volicord-mcp --connection <connection_id>`로 시작합니다.
- stdin을 읽지 않는 시작 검증은 `volicord-mcp --check --connection <connection_id>`로 실행합니다.
- 프로젝트별 시작 검증은 `volicord-mcp --check --connection <connection_id> --project <project_id>`로 실행합니다.
- `-h`와 `--help`는 사용법과 환경 요약을 출력한 뒤 종료 코드 `0`으로 끝납니다.
- `-V`와 `--version`은 `volicord-mcp <version>`을 출력한 뒤 종료 코드 `0`으로 끝납니다.
- 인자 없음, `--connection` 없는 `--check`, 알 수 없는 옵션, 결합된 명령줄 모드, 필요한 옵션 값 누락, 잘못된 `--project` 사용, 추가 위치 인자는 사용법 진단을 stderr에 쓰고 종료 코드 `2`로 끝납니다.
- help와 version 처리는 Runtime Home이나 Agent Connection 조회보다 먼저 일어납니다.

종료 코드와 스트림 동작:

- stdin EOF로 정상 종료하면 stdout을 플러시하고 종료 코드 `0`으로 끝납니다.
- 성공한 `--check`는 보고서를 stdout에 쓰고 종료 코드 `0`으로 끝납니다.
- 시작 중 설정, JSON, 저장소 오류는 진단을 stderr에 쓰고 종료 코드 `1`로 끝납니다.
- stdio 루프가 실행 중일 때 잘못된 JSON과 지원하지 않는 JSON-RPC 요청은 응답을 쓸 수 있으면 JSON-RPC 오류를 반환합니다.

<a id="process-environment"></a>
## 프로세스 환경

지원되는 선택 환경 입력:

- `VOLICORD_HOME`

`VOLICORD_HOME`은 지원되는 MCP 프로세스 환경 입력의 전부입니다. 이 값은 프로세스의 Runtime Home을 선택하지만 프로젝트, 연결, 행위자 출처, 작업 범주, 연결 모드를 선택하지 않습니다. stdio 프로세스와 `--check`는 시작 검증에 들어가기 전에 `VOLICORD_HOME`을 사용합니다. help와 version 모드는 이를 사용하지 않습니다.

연결 식별 정보는 `--connection <connection_id>`로 제공합니다. 묶인 Agent Connection과 Runtime Home 레지스트리 상태가 연결 모드, 연결 프로젝트, 어댑터가 파생하는 `actor_source`와 `operation_category`를 제공합니다. 프로젝트 접근은 Runtime Home 레지스트리 상태에 있는 선택된 Agent Connection의 연결 프로젝트로 제어됩니다. 선택 프로젝트는 공개 MCP 도구 호출마다 결정됩니다. MCP 프로세스는 그 밖의 프로세스 환경 입력을 해석하지 않습니다.

현재 MCP Runtime Home 경로 해석:

1. `VOLICORD_HOME`이 존재하지만 비어 있으면 오류입니다.
2. 절대 경로 `VOLICORD_HOME`은 제공된 그대로 사용합니다.
3. 상대 경로 `VOLICORD_HOME`은 그 경로가 존재하지 않아도 프로세스의 현재 작업 디렉터리를 기준으로 해석합니다.
4. `VOLICORD_HOME`이 없으면 `HOME`, `USERPROFILE`, `HOMEDRIVE`와 `HOMEPATH` 결합 순서로 첫 번째 비어 있지 않은 홈 소스를 사용합니다.
5. 선택한 사용자 홈에 `.volicord`를 붙입니다.
6. 선택한 홈이 상대 경로이면 프로세스의 현재 작업 디렉터리를 기준으로 해석합니다.
7. 시작 검증 전에 정규화를 요구하지 않습니다.

## 시작 검증

`volicord-mcp`는 stdio 루프에 들어가기 전에 Agent Connection 바인딩과 그 바인딩이 의존하는 로컬 레지스트리 기록을 검증합니다.

시작 검증에는 아래 조건이 필요합니다.

- Runtime Home 레지스트리가 존재하고 유효합니다.
- 설정된 `connection_id`가 존재합니다.
- 연결이 활성화되어 있습니다.
- 연결 모드가 지원됩니다.
- 연결 프로젝트 행을 읽을 수 있습니다.
- 시작에 필요한 레지스트리 JSON과 메타데이터가 유효합니다.

시작 검증은 모든 호출에 쓸 프로젝트 하나를 선택하지 않습니다. 프로젝트 가용성, 프로젝트 상태, 경로 분리, 모드 호환성은 [Agent Connection](agent-connection.md#current-connection-context)이 정의한 대로 호출마다 검증합니다.

Agent Connection은 연결 프로젝트가 하나도 없는 상태가 된 뒤에도 저장된 채 남을 수 있습니다. 이 지속 상태는 시작 가능성을 뜻하지 않습니다. 연결 프로젝트가 없으면 새 stdio 프로세스와 `volicord-mcp --check --connection <connection_id>`는 시작 검증에 실패합니다.

이미 실행 중인 프로세스는 새 프로세스와 다릅니다. 하나 이상의 프로젝트가 연결된 상태에서 시작 검증을 통과한 프로세스는 `volicord.list_projects`와 프로젝트 라우팅 때 레지스트리 상태를 새로 읽습니다. 마지막 멤버십이 제거된 뒤 `volicord.list_projects`는 빈 프로젝트 목록을 반환할 수 있지만, 프로젝트 라우팅이 필요한 공개 도구는 연결 프로젝트가 남아 있지 않으므로 거절됩니다.

## Agent Connection에 묶인 프로세스

`volicord-mcp` 프로세스 하나는 아래 값에 묶입니다.

- 하나의 `connection_id`

Agent Connection이 제공하는 값:

- `read_only` 또는 `workflow` 연결 모드 하나
- 명시적 연결 프로젝트 허용 목록
- 레지스트리를 통한 호스트 설정 인벤토리와 마지막 검증 상태

프로세스 바인딩은 프로세스 수명 동안 고정됩니다. Agent Connection 식별 정보를 바꾸려면 다른 프로세스나 호스트 설정 갱신이 필요합니다. 프로젝트 멤버십, 모드, 활성화 상태 변경은 레지스트리 상태를 통해 효력을 가지며, 새 프로세스는 시작할 때마다 현재 레지스트리 상태로 시작 검증을 다시 실행합니다.

MCP 호출 인자와 다른 MCP 요청 본문은 연결 식별 정보, `actor_source`, `operation_category`, 연결 모드를 설정할 수 없습니다.

<a id="configuration-preflight"></a>
## 설정 사전 점검

`volicord-mcp --check --connection <connection_id>`는 stdio 루프에 들어가기 전에 쓰는 것과 같은 Runtime Home, Agent Connection, 멤버십, 레지스트리 형태 시작 검증을 실행합니다. `volicord-mcp --check --connection <connection_id> --project <project_id>`는 프로젝트 세부 구간을 프로젝트 하나로 제한하고, 선택된 연결의 허용 목록 밖 프로젝트를 거절합니다. 두 형식 모두 stdin을 읽지 않습니다.

성공하면 `--check`는 고정 요약 줄을 먼저 쓰고, 이어 선택된 각 프로젝트마다 반복되는 프로젝트 세부 블록을 아래 순서로 stdout에 씁니다.

```text
configuration: valid
transport: stdio
runtime_home: <absolute path>
connection_id: <value>
mode: read_only|workflow
enabled: true
allowed_projects: <count>
available_projects: <count>
verification_scope: startup_check_only
project[0].project_id: <value>
project[0].available: true|false
project[0].unavailable_reason: <value or empty>
project[0].repo_root: <path>
```

프로젝트 세부 규칙:

- 세부 인덱스는 0에서 시작합니다.
- `--project`가 없으면 안정적인 `project_id` 순서대로 연결 프로젝트마다 세부 블록 하나를 냅니다.
- `--project <project_id>`는 Agent Connection에 연결되지 않은 프로젝트를 거절하고 세부 블록 선택을 그 프로젝트 하나로 제한합니다.
- `allowed_projects`는 Agent Connection 전체를 설명합니다. `--project`를 쓰면 `available_projects`는 출력된 세부 선택을 설명하므로 `0` 또는 `1`입니다.
- 사용할 수 없는 프로젝트도 모든 프로젝트 세부 키를 출력합니다. `unavailable_reason`은 사용할 수 없는 프로젝트에서 채워지고 사용할 수 있는 프로젝트에서는 비어 있습니다.
- `verification_scope: startup_check_only`는 시작과 사전 점검에 대한 문장일 뿐이며 전체 호스트 검증이 아닙니다.

시작 검증 실패:

- 프로세스 진입점을 통해 stderr에 진단을 씁니다.
- 종료 코드 `1`로 끝납니다.
- stdio 루프에 들어가지 않으며 stdin을 기다리지 않습니다.

성공한 `--check`는 전체 호스트 연결 결과가 아닙니다. 전체 호스트 검증에는 [관리 CLI](admin-cli.md#agent-connection-result-states)가 정의한 오래 유지되는 Agent Connection 상태, 호스트 설정 설치, 성공한 MCP 초기화, 성공한 도구 탐색이 필요합니다.

## MCP 와이어 동작

`volicord-mcp`는 stdio 위에서 MCP 프로토콜 버전 `2025-11-25`를 지원합니다. 더 오래된 MCP 프로토콜 버전과 동시에 호환된다고 광고하지 않습니다. 새 프로세스나 stdio 연결마다 새 MCP 수명주기가 시작되며, 각 연결은 자체 초기화 순서를 완료해야 합니다.

서버 초기화 응답에는 MCP 서버 지침이 들어갑니다. 이 지침은 Volicord 도구 선택, 결정적 프로젝트 라우팅, 제한을 설명할 수 있지만 안내일 뿐이며 접근 통제나 모델 동작 보장이 아닙니다.

### 프레이밍과 JSON-RPC 검증

프레이밍 규칙:

- 비어 있지 않은 각 stdin 줄은 UTF-8 JSON-RPC 메시지 객체 하나를 정확히 담습니다.
- JSON 루트는 JSON-RPC 메시지 객체 하나여야 합니다. Volicord의 클라이언트-서버 기준 범위에서 지원되는 메시지 객체는 요청과 `notifications/initialized` notification입니다. 배열, 원시 JSON 루트, `null`은 유효하지 않은 MCP stdio 메시지입니다.
- JSON-RPC 배치는 지원하지 않습니다. 배열 입력은 배열 요소마다 응답을 내지 않고 Invalid Request 응답 하나를 받습니다.
- 메시지는 줄바꿈으로 구분되며 메시지 안에 줄바꿈을 포함하면 안 됩니다.
- 각 출력 줄은 JSON-RPC 응답 객체 하나를 담습니다. `volicord-mcp`는 `initialize` 전에 준비 완료 메시지를 쓰지 않습니다.
- stdin EOF는 stdout을 플러시한 뒤 프로세스를 끝냅니다.

JSON-RPC 검증 규칙:

- `jsonrpc`는 정확히 `"2.0"`이어야 합니다.
- 요청 `method`는 문자열이어야 합니다.
- 요청 ID는 문자열 또는 정수일 수 있으며 `null`이면 안 됩니다.
- 분류 가능한 notification은 문자열 `method`를 갖고 `id`가 없으며 MCP 메서드 파라미터가 잘못되었더라도 응답을 받지 않습니다.
- `id`가 없는 객체가 자동으로 유효한 notification이 되는 것은 아닙니다. 그래도 notification 형태를 만족해야 합니다.
- 지원되는 MCP 요청의 메서드 `params`는 존재할 때 객체여야 합니다. 수명주기 notification에서는 `params`가 없거나 객체인 경우에만 수명주기에 영향을 줄 수 있습니다.

notification 분류는 MCP 메서드 파라미터 검증보다 먼저 JSON-RPC envelope를 기준으로 이루어집니다. 메시지가 notification으로 분류될 수 있으면 잘못된 `params`가 있어도 JSON-RPC 응답을 만들지 않습니다. 그러나 그런 `params`는 수명주기 목적에서는 유효하지 않습니다. 잘못된 `notifications/initialized`는 연결을 준비 상태로 옮기지 않고, notification으로 받은 요청 전용 메서드는 무시되며 실행하면 안 됩니다.

오류 분류:

| 조건 | MCP 응답 |
|---|---|
| JSON 파싱 실패 | JSON-RPC `-32700` Parse error |
| 배열, 원시 루트, 누락되었거나 잘못된 `jsonrpc`, 잘못된 요청 `id`, 누락되었거나 문자열이 아닌 요청 `method`, 잘못된 non-notification 객체를 포함한 유효하지 않은 JSON-RPC 메시지 구조 | JSON-RPC `-32600` Invalid Request |
| `initialize` 전 요청, 준비 상태 전 `tools/list`나 `tools/call`, 중복 `initialize`를 포함한 요청의 수명주기 위반 | JSON-RPC `-32600` Invalid Request |
| 알 수 없는 요청 메서드 | JSON-RPC `-32601` Method not found |
| 요청의 잘못된 메서드 파라미터 | JSON-RPC `-32602` Invalid params |
| 구조적으로 유효한 `tools/call` 요청의 알 수 없는 도구 이름 | JSON-RPC `-32602` Invalid params |
| 어댑터 또는 서버 내부 실패 | 적절한 JSON-RPC 내부 오류 응답 |
| 분류 가능한 notification. 잘못된 메서드 파라미터가 있는 경우도 포함 | 응답 없음. 잘못된 파라미터는 수명주기 전환이나 요청 전용 동작을 일으키지 않습니다. |

### 프로토콜 버전과 수명주기

연결에서 첫 번째로 유효한 MCP 요청은 `initialize`입니다. 유효한 `initialize` 요청은 객체 `params` 안에 아래 값을 둡니다.

- 문자열 `protocolVersion`
- 객체 `capabilities`
- 문자열 `name`과 `version` 필드를 포함하는 객체 `clientInfo`

2025-11-25 스키마가 허용하는 추가 MCP `Implementation` 메타데이터, 예를 들어 `title`, `description`, `icons`, `websiteUrl`은 받을 수 있지만 예시에 필수는 아닙니다.

프로토콜 버전 협상:

- 클라이언트가 `2025-11-25`를 요청하면 `volicord-mcp`는 `2025-11-25`를 반환합니다.
- 클라이언트가 문법적으로 유효한 다른 프로토콜 버전 문자열을 보내면 `volicord-mcp`는 자신이 지원하는 버전인 `2025-11-25`를 반환합니다.
- 서버 응답은 더 오래된 MCP 프로토콜 버전과 동시에 호환된다고 주장하지 않습니다.

수명주기 상태:

| 연결 지점 | 유효한 클라이언트 메시지 | 결과 |
|---|---|---|
| 성공한 `initialize` 전 | `initialize` 요청 | 성공하면 서버는 `protocolVersion: "2025-11-25"`를 반환하고 `notifications/initialized`를 기다립니다. |
| `notifications/initialized` 대기 중 | `notifications/initialized` notification, `ping` 요청 | `notifications/initialized`가 준비 상태 전환을 완료합니다. `ping`은 `initialize`가 성공한 뒤 사용할 수 있으며, 서버가 notification을 기다리는 동안에도 사용할 수 있습니다. |
| 준비 상태 | `ping`, `tools/list`, `tools/call` | 일반 MCP 도구 탐색과 도구 실행을 사용할 수 있습니다. |

`tools/list`와 `tools/call`은 `notifications/initialized`가 준비 상태 전환을 완료한 뒤에만 사용할 수 있습니다. 중복 `initialize` 요청은 유효하지 않습니다. 너무 이르거나 잘못된 `notifications/initialized` notification은 연결을 준비 상태로 만들지 않습니다.

지원되는 MCP 요청 메서드:

- `initialize`
- `ping`
- `tools/list`
- `tools/call`

지원되는 수명주기 notification은 `notifications/initialized`입니다.

## 도구 탐색과 `tools/call` 응답 래핑

연결이 준비 상태가 된 뒤 `tools/list`는 묶인 Agent Connection 모드에 따라 도구를 노출합니다.

| 모드 | MCP 메서드 도구 | MCP 어댑터 유틸리티 도구 |
|---|---|---|
| `read_only` | 2개: `volicord.status`, `volicord.close_task` | `volicord.list_projects` |
| `workflow` | 8개: `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.prepare_write`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.close_task` | `volicord.list_projects` |

위 MCP 메서드 도구 수는 공개 Volicord Core API 메서드 목록과 같은 것이 아닙니다. `volicord.list_projects`는 MCP 어댑터 유틸리티이며 공개 Volicord Core API 메서드가 아닙니다. `volicord.record_user_judgment`는 User Channel 경로를 위한 공개 Core API 메서드이지만 Agent Connection MCP 도구로 노출되지 않습니다. 공개 메서드 담당 표는 [API 메서드](api/methods.md)를 봅니다.

구조적으로 유효한 `tools/call` 요청은 객체 `params` 안에 아래 값을 둡니다.

- 문자열 `name`
- 선택적 객체 `arguments`

`arguments`가 없으면 빈 객체로 취급합니다. `arguments: null`과 객체가 아닌 `arguments`는 잘못된 메서드 파라미터이며 JSON-RPC `-32602`를 반환합니다. 알 수 없는 도구 이름은 프로토콜 오류이며 JSON-RPC `-32602`를 반환합니다.

공개 Volicord 메서드 도구에서 `tools/list`는 공유 Volicord 요청 스키마에 Agent Connection 바인딩을 적용해 만든 MCP에 보이는 입력 스키마를 노출합니다. `envelope.project_id`는 호출자가 선택할 수 있는 선택적 선택자로 남습니다. `envelope.actor_source`, `envelope.operation_category`, `envelope.connection_id`, `envelope.verification_basis`는 MCP에 보이는 스키마에 노출되지 않으며 원시 `tools/call` `arguments`에서도 허용되지 않습니다. 원시 공개 메서드 도구 `arguments`의 최상위나 `envelope` 안에 호출자 소유 호출 필드가 들어 있으면 어댑터는 Core 실행 전에 호출을 거절합니다.

알려진 공개 Volicord 메서드 도구에서 객체 `arguments`가 도구 입력 스키마를 통과하지 못하면 `isError: true`와 실행 가능한 text content를 담은 `CallToolResult`를 반환합니다. 이는 JSON-RPC 프로토콜 오류가 아니라 도구 실행 오류입니다.

`volicord.list_projects`에 대해 어댑터는 묶인 Agent Connection만을 위한 읽기 전용 프로젝트 목록을 반환합니다. 이 도구는 Core에 들어가거나, 저장 효과를 만들거나, 프로젝트 멤버십을 바꾸거나, 연결 허용 목록 밖의 프로젝트를 노출하면 안 됩니다. 연결 프로젝트의 현재 등록이 유효하지 않으면 어댑터는 그 프로젝트를 정상 available 또는 unavailable 항목으로 반환하지 않고 유틸리티 호출을 실패시킵니다.

공개 Volicord 메서드 도구 호출에 대해 어댑터는 먼저 [Agent Connection](agent-connection.md#current-connection-context)이 담당하는 결정적 프로젝트 선택과 프로젝트별 검증을 수행합니다. 모호한 프로젝트 선택은 Core 실행 전에 거절하고, 실행 가능한 텍스트는 에이전트에게 `volicord.list_projects`를 호출하라고 안내해야 합니다.

`volicord-mcp`는 MCP 태스크 보강 도구 실행을 광고하거나 구현하지 않습니다. `tools/call` 요청은 `CreateTaskResult`를 반환하지 않으며, `task` 파라미터는 지원되는 기준 기능이 아닙니다.

Volicord까지 도달한 알려진 공개 Volicord 메서드 도구 호출에서 `tools/call`은 MCP 결과 안에 Volicord 응답 JSON을 래핑합니다.

- Volicord 응답 JSON은 `result.content[0].text`의 문자열로 직렬화됩니다.
- 클라이언트는 Volicord 응답을 검사하려면 그 문자열을 JSON으로 파싱해야 합니다.
- 성공한 MCP 전송은 Volicord 도메인 수준 거절 응답을 포함해 `isError: false`를 반환합니다.
- Volicord 도메인 성공 또는 거절은 파싱한 Volicord 응답, 특히 `base.response_kind`와 `errors`에서 판단합니다.
- JSON-RPC `error`는 프로토콜, 잘못된 파라미터, 어댑터/내부 실패에만 사용합니다. Volicord 도메인 수준 거절에는 사용하지 않습니다.

Volicord 응답 분기 형태와 오류 의미는 각 담당 문서에 둡니다.

- 공통 응답 분기: [API 코어 스키마](api/schema-core.md#common-response)
- 응답 분기 처리 경로: [API 오류 처리 경로](api/error-routing.md)
- 공개 오류 코드: [API 오류 코드](api/error-codes.md)
- 기계 판독용 오류 세부사항: [API 오류 세부사항](api/error-details.md)

## 종료와 재연결

stdin을 닫거나 자식 프로세스를 종료하면 MCP 세션이 끝납니다.

종료와 재연결 규칙:

- SQLite 상태는 Runtime Home에 남습니다.
- 같은 `connection_id`로 다시 시작하면 같은 Agent Connection과 현재 레지스트리 상태에 다시 연결합니다.
- 연결을 바꾸려면 새 프로세스나 호스트 설정 갱신이 필요합니다.

런타임 데이터 위치 경계는 [런타임 경계](runtime-boundaries.md)가 담당하고, 저장소 기록 세부사항은 [저장소](storage.md)가 안내하는 저장소 담당 문서가 담당합니다.
