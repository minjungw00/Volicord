# MCP 전송 참조

이 문서는 로컬 `volicord mcp --stdio` 프로세스 계약과 실험적
`volicord serve --transport streamable-http` 프로세스 경계 계약을 담당합니다. 여기에는
프로세스 시작, 프로세스 환경, MCP 프로토콜 버전 협상, 초기화 수명주기, stdio 전송
프레이밍, 로컬 HTTP MCP 요청 처리, JSON-RPC 메시지 검증, Agent Connection에 묶인 시작
검증, MCP에 보이는 도구 탐색, MCP 응답 래핑, 종료와 재연결 동작이 포함됩니다.

공개 Volicord API 메서드 동작, 공개 요청/응답 스키마, Agent Connection 의미, 저장소
기록 배치, 보안 보장, Core 권한 의미는 이 문서가 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `volicord mcp --stdio` 프로세스 시작과 종료 동작
- `volicord serve --transport streamable-http` 시작, 로컬 리스너, 전송 경계 보안 점검
- 생성된 호스트 설정과 내보낸 MCP 설정이 사용하는 프로세스 설정
- MCP Runtime Home 경로 해석
- MCP 프로토콜 버전 협상과 초기화 수명주기
- stdio JSON-RPC 프레이밍, 메시지 검증, 지원되는 MCP 메서드
- 실험적 serve 전송을 위한 로컬 HTTP JSON-RPC 요청 처리
- stdio 전송 경계의 서버 시작 MCP elicitation
- 대기 사용자 판단을 위한 로컬 loopback web consent fallback
- 하나의 내부 Agent Connection 바인딩에 대한 MCP 시작 검증
- 전송 경계에서의 MCP `tools/list`와 `tools/call` 동작
- 내부 래퍼와 호출 메타데이터를 숨기는 MCP 표시 도구 스키마 투영
- MCP `tools/call` 응답 래핑
- 프로세스 종료와 재연결 동작

이 문서는 담당하지 않습니다.

- 공개 Volicord 메서드 목록이나 메서드 담당 표: [API 메서드](api/methods.md)
- 공개 Volicord 요청/응답 스키마: [API 코어 스키마](api/schema-core.md)
- Agent Connection, Connection Projects, 프로젝트 선택 의미, 현재 연결 맥락, 행위자 출처:
  [Agent Connection](agent-connection.md)
- 관리 Runtime Home setup, 연결, 프로젝트, export, 검증 명령: [관리 CLI](admin-cli.md)
- 저장소 배치, 마이그레이션, 저장 효과: [저장소](storage.md)가 안내하는 저장소 담당 문서

## 프로세스 모델

`volicord mcp --stdio`는 설치된 `volicord` 실행 파일의 로컬 MCP stdio 프로세스
모드입니다. MCP 호스트는 이를 자식 프로세스로 시작하고 stdin/stdout으로 통신합니다.
MCP TCP 리스너, HTTP MCP 리스너, Unix-domain socket 리스너, 또는 그 밖의 MCP 네트워크
리스너가 아닙니다. MCP elicitation과 prompt capture를 사용할 수 없을 때는 대기 사용자
판단을 위해 별도의 loopback 전용 local web consent 리스너를 시작할 수 있습니다.

`volicord serve --transport streamable-http`는 Docker와 localhost MCP 사용을 위한 별도의
명시적 프로세스 모드입니다. 이 명령은 로컬 HTTP 리스너를 시작하고, 가능한 곳에서는
stdio와 같은 Agent Connection에 묶인 MCP 어댑터 로직을 재사용합니다. 기본 MCP 전송이
아니며, Docker가 아닌 로컬 호스트 설정 생성에서 사용하지 않고, 인증 없는 일반 Volicord
네트워크 서비스도 아닙니다.

현재 serve 전송은 인증을 요구하는 실험적 Streamable HTTP 스타일 부분 구현입니다. MCP
세션 헤더와 bearer token 검사와 함께 HTTP `POST /mcp`로 JSON-RPC를 받고 JSON 응답을
반환합니다. server-sent event 스트림, HTTP elicitation, 전체 MCP Streamable HTTP
호환성은 구현하지 않습니다. 해당 전송 기능이 구현되고 테스트되기 전에는 문서와 시작
진단이 전체 프로토콜 호환성을 주장하면 안 됩니다.

생성된 호스트 설정과 generic export는 내부 연결 바인딩으로 stdio 루프를 시작할 수
있습니다.

```text
volicord mcp --stdio --connection <connection_id>
```

`<connection_id>` 프로세스 바인딩 값은 `volicord connect` 또는 export 흐름이 만든
저장된 `connection_internal_id`에서 옵니다. 일반 사용자가 텍스트 모드 흐름에서 이를
입력할 필요가 없어야 합니다.

기준 명령줄 동작:

- `volicord mcp --stdio --connection <connection_id>`는 stdio 루프를 시작합니다.
- `volicord mcp --check --connection <connection_id>`는 stdin을 읽지 않고 시작 검증을
  실행합니다.
- `volicord mcp --check --connection <connection_id> --project <project_id>`는 같은 시작
  검증을 실행하고 프로젝트 세부 진단을 허용 목록 안의 `project_internal_id` 값 하나로
  제한합니다.
- `-h`와 `--help`는 사용법과 환경 요약을 출력한 뒤 종료 코드 `0`으로 끝납니다.
- `-V`와 `--version`은 `volicord <version>`을 출력한 뒤 종료 코드 `0`으로 끝납니다.
- 모드 없음, `--connection` 없는 `--check` 또는 `--stdio`, 알 수 없는 옵션, 결합된
  명령줄 모드, 필요한 옵션 값 누락, 추가 위치 인자는 사용법 진단을 stderr에 쓰고 종료
  코드 `2`로 끝납니다.
- help와 version 처리는 Runtime Home이나 Agent Connection 조회보다 먼저 일어납니다.

실험적 HTTP serve 명령줄 동작:

- `volicord serve --transport streamable-http`만 지원되는 serve 전송 표기입니다. 다른 전송
  값은 사용법 오류입니다.
- `--listen 127.0.0.1:<port>`는 리스너를 선택합니다. 생략하면 `127.0.0.1:8765`를
  사용합니다.
- 기본 리스너는 loopback 전용입니다. `0.0.0.0`, `::`, 또는 다른 non-loopback 주소에
  바인딩하려면 `--allow-nonlocal-listen`이 필요하며 시작할 때 명확한 경고를 씁니다.
- `--home PATH`는 프로세스의 Runtime Home을 선택합니다. `--home`이 없으면 공통
  `VOLICORD_HOME`과 플랫폼 기본 Runtime Home 해석을 사용합니다.
- `--connection <connection_id>`는 서버를 저장된 Agent Connection 하나에 묶습니다. 이
  옵션이 없으면 선택적 serve 프로젝트 허용 목록과 일치하고 연결 프로젝트가 있는 활성
  Agent Connection이 정확히 하나일 때만 시작이 성공합니다.
- `--project PATH`는 반복할 수 있습니다. 각 경로는 등록된 저장소 루트로 해석되며 serve
  프로세스를 해당 프로젝트 식별 정보들로 좁힙니다. 이렇게 좁힌 집합도 선택된 Agent
  Connection의 연결 프로젝트 허용 목록 안에 있어야 합니다.
- `--token TOKEN`은 이 프로세스의 bearer token을 제공합니다. 생략하면 Volicord가 프로세스
  로컬 token을 생성하고 시작 중 stderr에 씁니다. token은 저장소 파일에 저장하지
  않습니다.
- `--allow-origin ORIGIN`은 반복할 수 있으며 정확히 일치하는 Origin 값의 브라우저 가능
  요청을 허용합니다. 이 옵션이 없으면 `Origin` 헤더가 있는 요청은 거절되고 CORS 응답
  헤더를 내지 않습니다.

종료 코드와 스트림 동작:

- stdin EOF로 정상 종료하면 stdout을 플러시하고 종료 코드 `0`으로 끝납니다.
- 성공한 `--check`는 보고서를 stdout에 쓰고 종료 코드 `0`으로 끝납니다.
- 시작 중 설정, JSON, 저장소 오류는 진단을 stderr에 쓰고 종료 코드 `1`로 끝납니다.
- HTTP serve 시작 설정, 리스너, 인증 token, Origin, 프로젝트 허용 목록 오류는 진단을
  stderr에 쓰고 종료 코드 `1`로 끝납니다.
- stdio 루프가 실행 중일 때 잘못된 JSON과 지원하지 않는 JSON-RPC 요청은 응답을 쓸 수
  있으면 JSON-RPC 오류를 반환합니다.

HTTP serve 요청 동작:

- MCP endpoint 경로는 `/mcp`입니다.
- `POST /mcp`에는 `Authorization: Bearer <token>`, `Content-Type: application/json`,
  그리고 `application/json`과 `text/event-stream`을 모두 포함하는 `Accept` 헤더가
  필요합니다.
- 성공한 `initialize`는 `Mcp-Session-Id`를 만듭니다. 이후 JSON-RPC 요청은 그 session ID를
  제공해야 합니다.
- `DELETE /mcp`는 bearer token과 session ID가 유효할 때 session을 삭제합니다.
- `GET /mcp`는 `SSE_UNSUPPORTED`를 반환합니다. server-sent event 스트림은 이 실험적
  endpoint에서 구현하지 않습니다.
- `GET /healthz`는 최소 로컬 health endpoint이지만 같은 bearer token을 요구합니다.
- `GET /consent`와 `POST /consent`는 local web consent를 사용할 수 있을 때만 열리는
  endpoint입니다. MCP endpoint가 아니며 MCP bearer token을 사용하지 않습니다. 프로젝트,
  연결, 대기 판단에 묶인 유효한 일회성 consent token이 필요합니다.
- 인증 없는 임의 resource endpoint는 없습니다.
- CORS preflight는 MCP endpoint에 대해서만, Origin 허용 목록 검증 뒤에만, 그리고 허용된
  Origin이 하나 이상 설정되어 있을 때만 받습니다.
- 구조화된 HTTP 오류는 인증, Origin, 프로젝트 허용 목록, 지원하지 않는 전송, 지원하지
  않는 메서드, 지원하지 않는 content negotiation 실패에 안정적인 전송 오류 코드를
  사용합니다.

<a id="process-environment"></a>
## 프로세스 환경

지원되는 선택 환경 입력:

- `VOLICORD_HOME`
- `VOLICORD_LOCAL_WEB_CONSENT`

`VOLICORD_HOME`은 프로세스의 Runtime Home을 선택합니다. 일반 흐름에서 사용자가 직접
입력하는 값이 아니라, 필요할 때 생성된 호스트 설정이 보통 기록하는 값입니다. 이 값은
프로젝트, 연결 의도, 행위자 출처, 작업 범주, 연결 모드, 호스트 신뢰 상태를 선택하지
않습니다. stdio 프로세스와 `--check`는 시작 검증에 들어가기 전에 `VOLICORD_HOME`을
사용합니다. help와 version 모드는 이를 사용하지 않습니다.

`VOLICORD_LOCAL_WEB_CONSENT=0`, `false`, `off`, `disabled`는 stdio local web consent
리스너를 끕니다. 다른 값은 리스너 주소나 token 정책을 바꾸지 않습니다.

연결 식별 정보는 생성된 호스트 설정이나 generic export 출력 안의
`--connection <connection_id>`로 제공합니다. 이것은 선택된 Agent Connection에 대한 내부
프로세스 바인딩이며, 사용자가 보통 직접 고르거나 관리하는 값이 아닙니다. 묶인 Agent
Connection과 Runtime Home 레지스트리 상태가 연결 모드, 연결 프로젝트, 어댑터가 파생하는
`actor_source`와 `operation_category`를 제공합니다. 프로젝트 접근은 선택된 Agent
Connection의 연결 프로젝트와 저장소 루트 해석으로 제어됩니다. MCP 프로세스는 그 밖의
프로세스 환경 입력을 해석하지 않습니다.

현재 MCP Runtime Home 경로 해석:

1. `VOLICORD_HOME`이 존재하지만 비어 있으면 오류입니다.
2. 절대 경로 `VOLICORD_HOME`은 제공된 그대로 사용합니다.
3. 상대 경로 `VOLICORD_HOME`은 그 경로가 존재하지 않아도 프로세스의 현재 작업
   디렉터리를 기준으로 해석합니다.
4. `VOLICORD_HOME`이 없으면 `volicord init` 또는 `volicord setup`이 마련한 Runtime Home,
   또는 플랫폼 기본 로컬 런타임 위치를 사용합니다.
5. 시작 검증 전에 정규화를 요구하지 않습니다.

## 시작 검증

`volicord mcp --stdio`는 stdio 루프에 들어가기 전에 Agent Connection 바인딩과 그
바인딩이 의존하는 로컬 레지스트리 기록을 검증합니다.

시작 검증에는 아래 조건이 필요합니다.

- Runtime Home 레지스트리가 존재하고 유효합니다.
- 설정된 `connection_id` 프로세스 인자가 저장된 기존 `connection_internal_id`를
  가리킵니다.
- 연결이 활성화되어 있습니다.
- 연결 모드가 지원됩니다.
- 연결 프로젝트 행이 하나 이상 읽을 수 있습니다.
- 진단에 필요한 MCP 명령 정보를 설치 프로필에서 해석할 수 있습니다.
- 시작에 필요한 레지스트리 JSON과 메타데이터가 유효합니다.

시작 검증은 호스트 신뢰를 부여하지 않고 사용자 소유 판단을 기록하지 않습니다. 프로젝트
가용성, 프로젝트 상태, 경로 분리, 저장소 루트 대조, 모드 호환성은
[Agent Connection](agent-connection.md#current-connection-context)이 정의한 대로 호출마다
검증합니다.

Agent Connection은 연결 프로젝트가 하나도 없는 상태가 된 뒤에도 저장된 채 남을 수
있습니다. 이 지속 상태는 시작 가능성을 뜻하지 않습니다. 연결 프로젝트가 없으면 새 stdio
프로세스와 시작 점검은 실패합니다.

이미 실행 중인 프로세스는 새 프로세스와 다릅니다. 하나 이상의 프로젝트가 연결된 상태에서
시작 검증을 통과한 프로세스는 프로젝트 라우팅 때 레지스트리 상태를 새로 읽습니다. 마지막
멤버십이 제거된 뒤 프로젝트 탐색은 사용 가능한 프로젝트가 없다고 보고할 수 있으며,
프로젝트 라우팅이 필요한 공개 도구는 연결 프로젝트가 남아 있지 않으므로 거절됩니다.

## Agent Connection에 묶인 프로세스

`volicord mcp --stdio` 프로세스 하나는 아래 값에 묶입니다.

- 저장된 Agent Connection을 위한 하나의 `connection_id` 프로세스 바인딩

Agent Connection이 제공하는 값:

- `workflow` 또는 `read_only` 연결 모드 하나
- `personal`, `shared`, `global` 중 하나의 연결 의도
- 명시적 연결 프로젝트 허용 목록
- 레지스트리를 통한 호스트 설정 인벤토리와 마지막 검증 상태

프로세스 바인딩은 프로세스 수명 동안 고정됩니다. Agent Connection 식별 정보를 바꾸려면
다른 프로세스나 호스트 설정 갱신이 필요합니다. 프로젝트 멤버십, 모드, 활성화 상태, 검증
상태 변경은 레지스트리 상태를 통해 효력을 가지며, 새 프로세스는 시작할 때마다 현재
레지스트리 상태로 시작 검증을 다시 실행합니다.

MCP 호출 인자와 다른 MCP 요청 본문은 `connection_internal_id`, `project_internal_id`,
`actor_source`, `operation_category`, 연결 의도, 연결 모드를 설정할 수 없습니다. 관리
연결 상태 출력은 `volicord` CLI에 속하고, MCP 시작 진단은 `volicord mcp --check`에
속합니다. 공개 MCP 도구 인자는 아래에서 설명하는 `project_selector` 동작을 사용합니다.

<a id="configuration-preflight"></a>
## 설정 사전 점검

`volicord mcp --check --connection <connection_id>`는 stdio 루프에 들어가기 전에 쓰는
것과 같은 Runtime Home, Agent Connection, 멤버십, 레지스트리 형태 시작 검증을
실행합니다. stdin을 읽지 않으며 전체 호스트 검증을 수행하지 않습니다.

성공하면 `--check`는 고정 요약 줄을 먼저 쓰고, 이어 연결된 각 프로젝트마다 반복되는
프로젝트 세부 블록을 아래 순서로 stdout에 씁니다.

```text
configuration: valid
transport: stdio
runtime_home: <absolute path>
connection_id: <connection_internal_id process-binding value>
mode: workflow|read_only
enabled: true|false
allowed_projects: <count>
available_projects: <count>
verification_scope: startup_check_only
project[0].project_id: <project_internal_id diagnostic value>
project[0].available: true|false
project[0].unavailable_reason: <value or empty>
project[0].repo_root: <path>
```

프로젝트 세부 규칙:

- 세부 인덱스는 0에서 시작합니다.
- `--project`가 없으면 안정적인 저장소 루트 순서대로 허용 프로젝트마다 세부 블록 하나를
  냅니다.
- `--project <project_id>`를 사용하면 제공한 값은 연결 허용 목록 안에 있어야 하며, 그
  프로젝트의 세부 블록만 출력합니다.
- `connection_id`는 저장된 Agent Connection을 위한 프로세스 바인딩입니다.
- `allowed_projects`는 Agent Connection 허용 목록 전체를 설명합니다.
- 사용할 수 없는 프로젝트도 모든 프로젝트 세부 키를 출력합니다. `unavailable_reason`은
  사용할 수 없는 프로젝트에서 채워지고 사용할 수 있는 프로젝트에서는 비어 있습니다.
- `verification_scope: startup_check_only`는 시작과 사전 점검에 대한 문장일 뿐이며 전체
  호스트 검증이 아닙니다.
- `--check` 출력에는 연결 존재 여부, 연결 프로젝트 수, 프로젝트 표시 이름을 나타내는
  관리 상태 필드가 포함되지 않습니다.

시작 검증 실패:

- 프로세스 진입점을 통해 stderr에 진단을 씁니다.
- 종료 코드 `1`로 끝납니다.
- stdio 루프에 들어가지 않으며 stdin을 기다리지 않습니다.

성공한 `--check`는 전체 호스트 연결 결과가 아닙니다. 전체 호스트 검증에는
[관리 CLI](admin-cli.md#agent-connection-result-states)가 정의한 오래 유지되는 Agent
Connection 상태, 호스트 설정 설치, 관찰 가능한 경우 충족된 호스트 소유 게이트, 성공한
MCP 초기화, 성공한 도구 탐색이 필요합니다.

## MCP 와이어 동작

`volicord mcp --stdio`는 stdio 위에서 MCP 프로토콜 버전 `2025-11-25`를 지원합니다. 더
오래된 MCP 프로토콜 버전과 동시에 호환된다고 광고하지 않습니다. 새 프로세스나 stdio
연결마다 새 MCP 수명주기가 시작되며, 각 연결은 자체 초기화 순서를 완료해야 합니다.

서버 초기화 응답에는 MCP 서버 지침이 들어갑니다. 이 지침은 Volicord 도구 선택, 저장소
루트 프로젝트 라우팅, 제한을 설명할 수 있지만 안내일 뿐이며 접근 통제나 모델 동작
보장이 아닙니다.

### 프레이밍과 JSON-RPC 검증

프레이밍 규칙:

- 비어 있지 않은 각 stdin 줄은 UTF-8 JSON-RPC 메시지 객체 하나를 정확히 담습니다.
- JSON 루트는 JSON-RPC 메시지 객체 하나여야 합니다. Volicord의 클라이언트-서버 기준
  범위에서 지원되는 메시지 객체는 요청과 `notifications/initialized` notification입니다.
  배열, 원시 JSON 루트, `null`은 유효하지 않은 MCP stdio 메시지입니다.
- JSON-RPC 배치는 지원하지 않습니다. 배열 입력은 배열 요소마다 응답을 내지 않고 Invalid
  Request 응답 하나를 받습니다.
- 메시지는 줄바꿈으로 구분되며 메시지 안에 줄바꿈을 포함하면 안 됩니다.
- 각 출력 줄은 JSON-RPC 응답 객체 하나를 담습니다. 다만 elicitation을 사용할 수 있는
  `tools/call`을 처리하는 동안에는 서버가 시작한 `elicitation/create` 요청이 출력될 수
  있습니다. `volicord mcp --stdio`는 `initialize` 전에 준비 완료 메시지를 쓰지 않습니다.
- stdin EOF는 stdout을 플러시한 뒤 프로세스를 끝냅니다.

JSON-RPC 검증 규칙:

- `jsonrpc`는 정확히 `"2.0"`이어야 합니다.
- 요청 `method`는 문자열이어야 합니다.
- 요청 ID는 문자열 또는 정수일 수 있으며 `null`이면 안 됩니다.
- 분류 가능한 notification은 문자열 `method`를 갖고 `id`가 없으며 MCP 메서드 파라미터가
  잘못되었더라도 응답을 받지 않습니다.
- `id`가 없는 객체가 자동으로 유효한 notification이 되는 것은 아닙니다. 그래도
  notification 형태를 만족해야 합니다.
- 지원되는 MCP 요청의 메서드 `params`는 존재할 때 객체여야 합니다. 수명주기
  notification에서는 `params`가 없거나 객체인 경우에만 수명주기에 영향을 줄 수 있습니다.

notification 분류는 MCP 메서드 파라미터 검증보다 먼저 JSON-RPC envelope를 기준으로
이루어집니다. 메시지가 notification으로 분류될 수 있으면 잘못된 `params`가 있어도
JSON-RPC 응답을 만들지 않습니다. 그러나 그런 `params`는 수명주기 목적에서는 유효하지
않습니다. 잘못된 `notifications/initialized`는 연결을 준비 상태로 옮기지 않고,
notification으로 받은 요청 전용 메서드는 무시되며 실행하면 안 됩니다.

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

연결에서 첫 번째로 유효한 MCP 요청은 `initialize`입니다. 유효한 `initialize` 요청은
객체 `params` 안에 아래 값을 둡니다.

- 문자열 `protocolVersion`
- 객체 `capabilities`
- 문자열 `name`과 `version` 필드를 포함하는 객체 `clientInfo`

`params.capabilities.elicitation`이 객체이면 어댑터는 MCP 클라이언트가 서버 시작
elicitation을 사용할 수 있다고 봅니다. 다른 capability 항목은 그 자체로 Volicord 동작을
만들지 않습니다.

예시는 위에 나열한 필드를 사용합니다. `volicord mcp --stdio`는 2025-11-25 스키마가
허용하는 추가 MCP `Implementation` 메타데이터, 예를 들어 `title`, `description`,
`icons`, `websiteUrl`을 받을 수 있습니다.

프로토콜 버전 협상:

- 클라이언트가 `2025-11-25`를 요청하면 `volicord mcp --stdio`는 `2025-11-25`를 반환합니다.
- 클라이언트가 문법적으로 유효한 다른 프로토콜 버전 문자열을 보내면 `volicord mcp --stdio`는
  자신이 지원하는 버전인 `2025-11-25`를 반환합니다.
- 서버 응답은 더 오래된 MCP 프로토콜 버전과 동시에 호환된다고 주장하지 않습니다.

수명주기 상태:

| 연결 지점 | 유효한 클라이언트 메시지 | 결과 |
|---|---|---|
| 성공한 `initialize` 전 | `initialize` 요청 | 성공하면 서버는 `protocolVersion: "2025-11-25"`를 반환하고 `notifications/initialized`를 기다립니다. |
| `notifications/initialized` 대기 중 | `notifications/initialized` notification, `ping` 요청 | `notifications/initialized`가 준비 상태 전환을 완료합니다. `ping`은 `initialize`가 성공한 뒤 사용할 수 있으며, 서버가 notification을 기다리는 동안에도 사용할 수 있습니다. |
| 준비 상태 | `ping`, `tools/list`, `tools/call` | 일반 MCP 도구 탐색과 도구 실행을 사용할 수 있습니다. |

`tools/list`와 `tools/call`은 `notifications/initialized`가 준비 상태 전환을 완료한 뒤에만
사용할 수 있습니다. 중복 `initialize` 요청은 유효하지 않습니다. 너무 이르거나 잘못된
`notifications/initialized` notification은 연결을 준비 상태로 만들지 않습니다.

지원되는 MCP 요청 메서드:

- `initialize`
- `ping`
- `tools/list`
- `tools/call`

초기화된 클라이언트가 `capabilities.elicitation`을 선언했다면 서버는
`volicord.request_user_judgment`를 처리하는 동안 중첩된 `elicitation/create` 요청 하나를
보낼 수 있습니다. 이 요청은 서버가 시작한 MCP 프로토콜 트래픽이며 Agent Connection
도구가 아닙니다. 서버는 User Channel 기록을 시도하기 전에 그 서버 요청에 대한 클라이언트
응답을 검증합니다.

지원되는 수명주기 notification은 `notifications/initialized`입니다.

<a id="tool-discovery-and-toolscall-response-wrapping"></a>
## 도구 탐색과 `tools/call` 응답 래핑

연결이 준비 상태가 된 뒤 `tools/list`는 현재 저장된 Agent Connection 모드에 따라
도구를 노출합니다.

| 모드 | MCP에 보이는 도구 |
|---|---|
| `workflow` | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.prepare_write`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task`, `volicord.list_projects` |
| `read_only` | `volicord.status`, `volicord.check_close`, `volicord.list_projects` |

MCP에 보이는 도구는 공개 Volicord Core API 메서드 목록과 같은 것이 아닙니다.
`volicord.check_close`는 닫기 준비 상태를 확인하는 읽기 전용 MCP 도구이며 내부적으로
Core 닫기 준비 상태 확인 경로를 호출합니다. `volicord.close_task`는 워크플로 전용
MCP 변경 도구이며 `read_only` 연결에는 나열되지 않습니다.
`volicord.record_user_judgment`는 User Channel 경로를 위한 공개 Core API 메서드이지만
Agent Connection MCP 도구로 노출되지 않습니다. 공개 메서드 담당 표는 [API
메서드](api/methods.md)를 봅니다.

구조적으로 유효한 `tools/call` 요청은 객체 `params` 안에 아래 값을 둡니다.

- 문자열 `name`
- 선택적 객체 `arguments`

`arguments`가 없으면 빈 객체로 취급합니다. `arguments: null`과 객체가 아닌
`arguments`는 잘못된 메서드 파라미터이며 JSON-RPC `-32602`를 반환합니다. 알 수 없는
도구 이름은 프로토콜 오류이며 JSON-RPC `-32602`를 반환합니다.

공개 Volicord 메서드 도구에서 `tools/list`는 Core 요청 래퍼가 아니라 워크플로 도메인
인자를 담은 MCP 표시 입력 스키마를 노출합니다. 보이는 스키마는 선택적
`project_selector`를 노출하며 내부 요청 래퍼, 프로토콜 메타데이터, `project_id`,
`connection_id`, `request_id`, `idempotency_key`, `expected_state_version`, `dry_run`,
`locale`, `actor_source`, `operation_category`, 검증 근거 필드를 숨겨야 합니다. 숨겨진
필드는 공개 MCP 도구 인자로 필요하지도 허용되지도 않습니다. 원시 공개 메서드 도구
인자가 이런 필드를 포함하면 어댑터는 Core 실행 전에 호출을 거절합니다.

프로젝트 선택은 Agent Connection 맥락에서 해석합니다. 사용 가능한 연결 프로젝트가
정확히 하나이면 공개 메서드 도구의 프로젝트 선택을 생략할 수 있습니다. 여러 프로젝트가
연결된 경우에는 `volicord.list_projects`가 반환한 `project_selector` 값이 필요합니다.
그렇지 않으면 어댑터는 수행 가능한 모호성 오류 문구로 호출을 거절합니다. 에이전트는 폴더
이름, 현재 작업 디렉터리, MCP roots, 호스트 라벨, 저장소 라벨, 기억에서 프로젝트 식별
정보를 추론하면 안 됩니다.

MCP 어댑터는 Core에 넘기기 전에 Core 래퍼를 생성합니다. 어댑터는 `request_id`, 워크플로
효과에 대한 `idempotency_key`, Core freshness가 요구하는 경우 선택된 프로젝트의 현재
상태에서 얻은 `expected_state_version`, `dry_run=false`, 기본 locale, 선택된 내부
프로젝트, 파생된 호출 맥락을 제공합니다. 공개 MCP 인자는 이 사실들을 덮어쓸 수 없습니다.

`volicord.status`는 Core include 행렬을 노출하지 않고 간결한 공개 `detail` 인자를
사용합니다. 지원 값은 `summary`, `workflow`, `full`이며 `detail`을 생략하면 기본값은
`workflow`입니다.

알려진 공개 Volicord 메서드 도구에서 객체 `arguments`가 도구 입력 스키마를 통과하지
못하면 `isError: true`와 실행 가능한 text content를 담은 `CallToolResult`를 반환합니다.
이는 JSON-RPC 프로토콜 오류가 아니라 도구 실행 오류입니다.

공개 Volicord 메서드 도구 호출에 대해 어댑터는 먼저
[Agent Connection](agent-connection.md#current-connection-context)이 담당하는 결정적 저장소
루트 프로젝트 선택과 프로젝트별 검증을 수행합니다. 모호하거나 사용할 수 없는 프로젝트
선택은 Core 실행 전에 거절하고, 실행 가능한 텍스트는 상태를 고칠
`volicord project use` 또는 `volicord connect` 명령을 이름 붙여야 합니다.

`volicord mcp --stdio`는 MCP 태스크 보강 도구 실행을 광고하거나 구현하지 않습니다. `tools/call`
요청은 `CreateTaskResult`를 반환하지 않으며, `task` 파라미터는 지원되는 기준 기능이
아닙니다.

<a id="user-judgment-elicitation"></a>
### 사용자 판단 elicitation

`volicord.request_user_judgment`는 Core에 집중된 대기 `UserJudgment` 생성을 요청하는
유일한 Agent Connection 도구로 남습니다. MCP 어댑터는 `volicord.record_user_judgment`를
Agent Connection 도구로 노출하지 않으며, 에이전트가 넣은 답변 필드를 사용자 입력의
대체물로 받지 않습니다.

`workflow` 연결이 `volicord.request_user_judgment`를 호출하고 Core가 대기 판단을 커밋하면
다음 규칙을 적용합니다.

- 초기화된 클라이언트가 `capabilities.elicitation`을 선언했다면 어댑터는 원래
  `tools/call` 응답을 반환하기 전에 `elicitation/create`를 보낼 수 있습니다. 요청
  스키마는 Core가 만든 선택지 ID에서 가져온 필수 `selected_option_id`와 선택적 `note`를
  담은 평평한 객체입니다. 이 스키마는 secret, credential, token, private key 또는 그 밖의
  비공개 secret 자료를 요청하지 않습니다.
- elicitation 응답이 `action=accept`이면 어댑터는 `content.selected_option_id`를 대기
  판단 선택지와 대조해 검증합니다. 유효한 응답은 Core의 User Channel 메서드를 통해
  `actor_source=local_user`, `operation_category=user_only`,
  `resolved_verification_basis=mcp_elicitation_user_channel`로 기록합니다. 반환되는
  `tools/call` content에는 그 결과 Volicord 응답 JSON이 들어갑니다.
- elicitation 응답이 `action=decline`이고 대기 판단에 Core reject 선택지가 있으면
  어댑터는 같은 User Channel 경로로 그 reject 선택지를 기록합니다. reject 선택지가 없으면
  판단은 대기 상태로 남습니다.
- elicitation 응답이 `action=cancel`이거나, 유효하지 않거나, 형식이 잘못되었거나, 대기
  판단과 맞출 수 없으면 어댑터는 답변을 기록하지 않으며 대기 판단은 대기 상태로 남습니다.
- 클라이언트가 capability를 선언하지 않아 elicitation을 사용할 수 없으면 어댑터는 답변을
  기록하지 않고 대기 `RequestUserJudgmentResult`와 추가 text content를 반환합니다.
  prompt-capture 사용 가능 상태가 `configured`, `observed`, `active`이면 그 text에
  prompt-submit hook 경로와 호환되고 현재 검증 코드를 포함한 정확한 채팅
  prompt-capture 명령이 들어갈 수 있습니다.
- prompt capture를 사용할 수 없고 local web consent를 사용할 수 있으면 어댑터는 짧게
  만료되는 일회성 token을 만들고 loopback consent URL과 구조화된 fallback JSON을
  반환합니다. URL에는 프로젝트 selector와 token만 들어갑니다. Runtime Home 경로, 저장소
  경로, prompt 본문, 답변, 임의 API 매개변수는 포함하지 않습니다.
- local web consent가 비활성화되었거나, 안전하게 bind할 수 없거나, token을 만들 수 없으면
  fallback text는 `volicord user` 로컬 CLI 복구 경로를 안내합니다.

모든 분기에서 `result.content[0].text`는 Volicord 응답 JSON 문자열로 남습니다. 추가
`content[]` text가 있으면 fallback 안내나 elicitation 취소/무효 설명 같은 어댑터
안내입니다. 그 추가 text는 Core 권한, 공개 API 응답 필드, 사용자 판단 기록이 아닙니다.

Local web consent 리스너는 기본적으로 `127.0.0.1`에 bind하며, 안전하게 bind할 수 없으면
fail closed해야 합니다. stdio 모드에서는 임시 loopback port를 사용합니다.
`volicord serve --transport streamable-http`에서는 실제 serve 리스너가 loopback일 때만
local web consent를 사용할 수 있습니다. 명시적으로 non-local serve 리스너를 연 경우
consent form을 노출하면 안 됩니다.

Local web consent endpoint 동작:

- `GET /consent?project=<project_id>&token=<token>`은 일회성 token을 현재 프로젝트와
  연결에 대해 검증합니다. 만료됨, 소비됨, 유효하지 않음, wrong-project, wrong-connection
  token은 안전한 HTML 오류 페이지로 거절하고, 그 밖에는 판단 text, 선택지, 검증 정보,
  form을 담은 최소 HTML page를 렌더링합니다.
- `POST /consent`는 token, 선택한 Core option ID, 선택적 note가 들어 있는
  `application/x-www-form-urlencoded` form 제출만 받습니다. `Origin` header가 있으면
  consent endpoint origin과 일치해야 합니다.
- 성공한 post는 token을 정확히 한 번 소비하고 Core를 통해 `actor_source=local_user`,
  `operation_category=user_only`, `resolved_verification_basis=local_user_local_web`으로
  답변을 기록합니다.
- replay, 만료, 소비된 token 재사용, wrong project, wrong connection은 다른 답변을
  기록하기 전에 거절합니다.
- endpoint는 Runtime Home 파일, Product Repository 파일, 정적 asset, MCP 메서드, 임의
  API를 제공하지 않습니다.

Volicord까지 도달한 알려진 공개 Volicord 메서드 도구 호출에서 `tools/call`은 MCP 결과
안에 Volicord 응답 JSON을 래핑합니다.

- Volicord 응답 JSON은 `result.content[0].text`의 문자열로 직렬화됩니다.
- 클라이언트는 Volicord 응답을 검사하려면 그 문자열을 JSON으로 파싱해야 합니다.
- 성공한 MCP 전송은 Volicord 도메인 수준 거절 응답을 포함해 `isError: false`를
  반환합니다.
- Volicord 도메인 성공 또는 거절은 파싱한 Volicord 응답, 특히 `base.response_kind`와
  `errors`에서 판단합니다.
- JSON-RPC `error`는 프로토콜, 잘못된 파라미터, 어댑터/내부 실패에만 사용합니다.
  Volicord 도메인 수준 거절에는 사용하지 않습니다.

Volicord 응답 분기 형태와 오류 의미는 각 담당 문서에 둡니다.

- 공통 응답 분기: [API 코어 스키마](api/schema-core.md#common-response)
- 응답 분기 처리 경로: [API 오류 처리 경로](api/error-routing.md)
- 공개 오류 코드: [API 오류 코드](api/error-codes.md)
- 기계 판독용 오류 세부사항: [API 오류 세부사항](api/error-details.md)

## 종료와 재연결

stdin을 닫거나 자식 프로세스를 종료하면 MCP 세션이 끝납니다.

종료와 재연결 규칙:

- SQLite 상태는 Runtime Home에 남습니다.
- 같은 `connection_id` 프로세스 바인딩으로 다시 시작하면 같은 Agent Connection과 현재
  레지스트리 상태에 다시 연결합니다.
- 연결을 바꾸려면 새 프로세스나 호스트 설정 갱신이 필요합니다.

런타임 데이터 위치 경계는 [런타임 경계](runtime-boundaries.md)가 담당하고, 저장소 기록
세부사항은 [저장소](storage.md)가 안내하는 저장소 담당 문서가 담당합니다.
