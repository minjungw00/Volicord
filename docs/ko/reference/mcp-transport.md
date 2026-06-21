# MCP 전송 참조

이 문서는 로컬 `harness-mcp` 프로세스 계약을 담당합니다. 여기에는 프로세스 시작, 프로세스 환경, MCP 프로토콜 버전 협상, 초기화 수명주기, stdio 전송 프레이밍, JSON-RPC 메시지 검증, 통합에 묶인 시작 검증, MCP 응답 래핑, 종료와 재연결 동작이 포함됩니다.

이 문서는 공개 하네스 API 메서드 동작, 공개 요청/응답 스키마, 접근 등급 의미, Agent Integration Profile 의미, 저장소 기록 배치, 보안 보장, Core 권한 의미를 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `harness-mcp` 프로세스 시작과 종료 동작
- 통합에 묶인 시작에 필요한 필수 및 선택 프로세스 설정
- MCP Runtime Home 경로 해석
- MCP 프로토콜 버전 협상과 초기화 수명주기
- stdio JSON-RPC 프레이밍, 메시지 검증, 지원되는 MCP 메서드
- 하나의 `integration_id`에 대한 MCP 시작 검증
- 전송 경계에서의 MCP `tools/list`, `tools/call`, `harness.list_projects` 어댑터 유틸리티 동작
- MCP `tools/call` 응답 래핑
- 프로세스 종료와 재연결 동작

이 문서는 담당하지 않습니다.

- 공개 하네스 메서드 목록이나 메서드 담당 표: [API 메서드](api/methods.md)
- 공개 하네스 요청/응답 스키마: [API 코어 스키마](api/schema-core.md)
- 접근 등급 값 의미: [API 값 집합](api/schema-value-sets.md#access-class-values)
- Agent Integration Profile, Host Installation, 프로젝트 선택 의미, 확인된 접점 맥락, 행위자 출처: [에이전트 통합](agent-integration.md)
- 관리 Runtime Home, 에이전트 설치, 프로젝트 멤버십, 검증 명령: [관리 CLI](admin-cli.md)
- 저장소 배치, 마이그레이션, 저장 효과: [저장소](storage.md)가 안내하는 저장소 담당 문서

## 프로세스 모델

`harness-mcp`는 로컬 MCP stdio 프로세스입니다. MCP 호스트는 이를 자식 프로세스로 시작하고 stdin/stdout으로 통신합니다. TCP 리스너, HTTP 리스너, Unix-domain socket 리스너, 또는 그 밖의 네트워크 리스너가 아닙니다.

기준 명령줄 동작:

- stdio 루프는 `harness-mcp --integration <integration_id>`로 시작합니다.
- stdin을 읽지 않는 시작 검증은 `harness-mcp --check --integration <integration_id>`로 실행합니다.
- `-h`와 `--help`는 사용법과 환경 요약을 출력한 뒤 종료 코드 `0`으로 끝납니다.
- `-V`와 `--version`은 `harness-mcp <version>`을 출력한 뒤 종료 코드 `0`으로 끝납니다.
- 알 수 없는 옵션, 결합된 명령줄 모드, 필요한 옵션 값 누락, 추가 위치 인자는 사용법 진단을 stderr에 쓰고 종료 코드 `2`로 끝납니다.
- help와 version 처리는 Runtime Home이나 통합 조회보다 먼저 일어납니다.

종료 코드와 스트림 동작:

- stdin EOF로 정상 종료하면 stdout을 플러시하고 종료 코드 `0`으로 끝납니다.
- 성공한 `--check`는 보고서를 stdout에 쓰고 종료 코드 `0`으로 끝납니다.
- 시작 중 설정, JSON, 저장소 오류는 진단을 stderr에 쓰고 종료 코드 `1`로 끝납니다.
- stdio 루프가 실행 중일 때 잘못된 JSON과 지원하지 않는 JSON-RPC 요청은 응답을 쓸 수 있으면 JSON-RPC 오류를 반환합니다.

호환성:

- `HARNESS_PROJECT_ID`, `HARNESS_SURFACE_ID`, `HARNESS_SURFACE_INSTANCE_ID`를 사용하는 레거시 고정 프로젝트 시작 모드는 호환 경로로만 남길 수 있습니다.
- 새 설정, 새 예시, 기준 Host Installation 기록은 레거시 고정 프로젝트 시작 형식을 생성하면 안 됩니다.

<a id="process-environment"></a>
## 프로세스 환경

선택:

- `HARNESS_HOME`

stdio 프로세스와 `--check`는 시작 검증에 들어가기 전에 `HARNESS_HOME`을 사용합니다. help와 version 모드는 이를 사용하지 않습니다.

새 기준 호스트 설정은 아래 값을 요구하면 안 됩니다.

- `HARNESS_PROJECT_ID`
- `HARNESS_SURFACE_ID`
- `HARNESS_SURFACE_INSTANCE_ID`

선택된 Agent Integration Profile이 접점과 접점 인스턴스 바인딩을 제공합니다. 선택 프로젝트는 공개 MCP 도구 호출마다 결정됩니다.

현재 MCP Runtime Home 경로 해석:

1. `HARNESS_HOME`이 존재하지만 비어 있으면 오류입니다.
2. 절대 경로 `HARNESS_HOME`은 제공된 그대로 사용합니다.
3. 상대 경로 `HARNESS_HOME`은 그 경로가 존재하지 않아도 프로세스의 현재 작업 디렉터리를 기준으로 해석합니다.
4. `HARNESS_HOME`이 없으면 `HOME`, `USERPROFILE`, `HOMEDRIVE`와 `HOMEPATH` 결합 순서로 첫 번째 비어 있지 않은 홈 소스를 사용합니다.
5. 선택한 사용자 홈에 `.harness`를 붙입니다.
6. 선택한 홈이 상대 경로이면 프로세스의 현재 작업 디렉터리를 기준으로 해석합니다.
7. 시작 검증 전에 정규화를 요구하지 않습니다.

## 시작 검증

`harness-mcp`는 stdio 루프에 들어가기 전에 통합 바인딩과 그 바인딩이 의존하는 로컬 레지스트리 기록을 검증합니다.

시작 검증에는 아래 조건이 필요합니다.

- Runtime Home 레지스트리가 존재하고 유효합니다.
- 설정된 `integration_id`가 존재합니다.
- 통합이 활성화되어 있습니다.
- 통합 역할을 인식할 수 있고 MCP 에이전트 통합에 유효합니다.
- 통합 `surface_id`와 `surface_instance_id`가 있습니다.
- 통합 프로젝트 멤버십 행을 읽을 수 있습니다.
- `default_project_id`가 있으면 그 값도 멤버십 행입니다.
- 시작에 필요한 레지스트리 JSON과 메타데이터가 유효합니다.

시작 검증은 모든 호출에 쓸 프로젝트를 선택하지 않습니다. 프로젝트 가용성, 프로젝트 상태, 경로 분리, 접점 등록, 접근 등급 허용은 [에이전트 통합](agent-integration.md#current-surface-context)이 정의한 대로 호출마다 검증합니다.

## 통합에 묶인 프로세스

`harness-mcp` 프로세스 하나는 아래 값에 묶입니다.

- 하나의 `integration_id`

통합이 제공하는 값:

- 하나의 `surface_id`
- 하나의 `surface_instance_id`
- 명시적 프로젝트 멤버십 허용 목록
- 선택적 `default_project_id`

프로세스 바인딩은 프로세스 수명 동안 고정됩니다. 통합을 바꾸려면 다른 프로세스나 호스트 설정 갱신이 필요합니다. 프로젝트 멤버십 변경이나 통합 비활성화는 호스트 설정을 다시 쓰지 않아도 레지스트리 상태를 통해 효력을 가집니다.

<a id="configuration-preflight"></a>
## 설정 사전 점검

`harness-mcp --check --integration <integration_id>`는 stdio 루프에 들어가기 전에 쓰는 것과 같은 Runtime Home, 통합, 멤버십, 레지스트리 형태 시작 검증을 실행합니다. stdin은 읽지 않습니다.

성공하면 `--check`는 stdout에 아래 줄들을 이 순서로 씁니다.

```text
configuration: valid
transport: stdio
runtime_home: <absolute path>
integration_id: <value>
interaction_role: agent
surface_id: <value>
surface_instance_id: <value>
enabled: true
allowed_projects: <count>
available_projects: <count>
default_project_id: <value or empty>
verification_scope: startup_check_only
```

시작 검증 실패:

- 프로세스 진입점을 통해 stderr에 진단을 씁니다.
- 종료 코드 `1`로 끝납니다.
- stdio 루프에 들어가지 않으며 stdin을 기다리지 않습니다.

성공한 `--check`는 전체 호스트 통합 결과가 아닙니다. 전체 호스트 통합에는 [관리 CLI](admin-cli.md#agent-setup-result-states)가 정의한 오래 유지되는 통합 상태, 호스트 설정 설치, 성공한 MCP 초기화, 성공한 도구 탐색이 필요합니다.

## MCP 와이어 동작

`harness-mcp`는 stdio 위에서 MCP 프로토콜 버전 `2025-11-25`를 지원합니다. 더 오래된 MCP 프로토콜 버전과 동시에 호환된다고 광고하지 않습니다. 새 프로세스나 stdio 연결마다 새 MCP 수명주기가 시작되며, 각 연결은 자체 초기화 순서를 완료해야 합니다.

서버 초기화 응답에는 MCP 서버 지침이 들어갑니다. 이 지침은 하네스 도구 선택, 결정적 프로젝트 라우팅, 제한을 설명할 수 있지만 안내일 뿐이며 접근 통제나 모델 동작 보장이 아닙니다.

### 프레이밍과 JSON-RPC 검증

프레이밍 규칙:

- 비어 있지 않은 각 stdin 줄은 UTF-8 JSON-RPC 메시지 객체 하나를 정확히 담습니다.
- JSON 루트는 JSON-RPC 메시지 객체 하나여야 합니다. 하네스의 클라이언트-서버 기준 범위에서 지원되는 메시지 객체는 요청과 `notifications/initialized` notification입니다. 배열, 원시 JSON 루트, `null`은 유효하지 않은 MCP stdio 메시지입니다.
- JSON-RPC 배치는 지원하지 않습니다. 배열 입력은 배열 요소마다 응답을 내지 않고 Invalid Request 응답 하나를 받습니다.
- 메시지는 줄바꿈으로 구분되며 메시지 안에 줄바꿈을 포함하면 안 됩니다.
- 각 출력 줄은 JSON-RPC 응답 객체 하나를 담습니다. `harness-mcp`는 `initialize` 전에 준비 완료 메시지를 쓰지 않습니다.
- stdin EOF는 stdout을 플러시한 뒤 프로세스를 끝냅니다.

JSON-RPC 검증 규칙:

- `jsonrpc`는 정확히 `"2.0"`이어야 합니다.
- 요청 `method`는 문자열이어야 합니다.
- 요청 ID는 문자열 또는 정수일 수 있으며 `null`이면 안 됩니다.
- 구조적으로 유효한 notification은 문자열 `method`를 갖고 `id`가 없으며 응답을 받지 않습니다.
- `id`가 없는 객체가 자동으로 유효한 notification이 되는 것은 아닙니다. 그래도 notification 형태를 만족해야 합니다.
- 지원되는 MCP 메서드의 `params`는 존재할 때 객체여야 합니다.

오류 분류:

| 조건 | MCP 응답 |
|---|---|
| JSON 파싱 실패 | JSON-RPC `-32700` Parse error |
| 배열, 원시 루트, 누락되었거나 잘못된 `jsonrpc`, 잘못된 요청 `id`, 누락되었거나 문자열이 아닌 요청 `method`, 잘못된 non-notification 객체를 포함한 유효하지 않은 JSON-RPC 메시지 구조 | JSON-RPC `-32600` Invalid Request |
| `initialize` 전 요청, 준비 상태 전 `tools/list`나 `tools/call`, 중복 `initialize`를 포함한 요청의 수명주기 위반 | JSON-RPC `-32600` Invalid Request |
| 알 수 없는 요청 메서드 | JSON-RPC `-32601` Method not found |
| 잘못된 메서드 파라미터 | JSON-RPC `-32602` Invalid params |
| 구조적으로 유효한 `tools/call` 요청의 알 수 없는 도구 이름 | JSON-RPC `-32602` Invalid params |
| 어댑터 또는 서버 내부 실패 | 적절한 JSON-RPC 내부 오류 응답 |
| 구조적으로 유효한 notification | 응답 없음. `notifications/initialized`가 너무 이르거나 다른 이유로 수명주기에 맞지 않으면 연결을 준비 상태로 옮기지 않습니다. |

### 프로토콜 버전과 수명주기

연결에서 첫 번째로 유효한 MCP 요청은 `initialize`입니다. 유효한 `initialize` 요청은 객체 `params` 안에 아래 값을 둡니다.

- 문자열 `protocolVersion`
- 객체 `capabilities`
- 문자열 `name`과 `version` 필드를 포함하는 객체 `clientInfo`

2025-11-25 스키마가 허용하는 추가 MCP `Implementation` 메타데이터, 예를 들어 `title`, `description`, `icons`, `websiteUrl`은 받을 수 있지만 예시에 필수는 아닙니다.

프로토콜 버전 협상:

- 클라이언트가 `2025-11-25`를 요청하면 `harness-mcp`는 `2025-11-25`를 반환합니다.
- 클라이언트가 문법적으로 유효한 다른 프로토콜 버전 문자열을 보내면 `harness-mcp`는 자신이 지원하는 버전인 `2025-11-25`를 반환합니다.
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

연결이 준비 상태가 된 뒤 `tools/list`는 아래를 노출합니다.

- [API 메서드](api/methods.md)가 담당하는 공개 하네스 도구 아홉 개
- 읽기 전용 MCP 어댑터 유틸리티 도구 `harness.list_projects`

이 문서는 두 번째 독립 공개 Core 메서드 목록을 만들지 않습니다. `harness.list_projects`는 공개 하네스 Core API 메서드가 아닙니다.

구조적으로 유효한 `tools/call` 요청은 객체 `params` 안에 아래 값을 둡니다.

- 문자열 `name`
- 선택적 객체 `arguments`

`arguments`가 없으면 빈 객체로 취급합니다. `arguments: null`과 객체가 아닌 `arguments`는 잘못된 메서드 파라미터이며 JSON-RPC `-32602`를 반환합니다. 알 수 없는 도구 이름은 프로토콜 오류이며 JSON-RPC `-32602`를 반환합니다.

알려진 공개 하네스 도구에서 객체 `arguments`가 도구 입력 스키마를 통과하지 못하면 `isError: true`와 실행 가능한 text content를 담은 `CallToolResult`를 반환합니다. 이는 JSON-RPC 프로토콜 오류가 아니라 도구 실행 오류입니다.

`harness.list_projects`에 대해 어댑터는 묶인 통합만을 위한 읽기 전용 프로젝트 목록을 반환합니다. 이 도구는 Core에 들어가거나, 저장 효과를 만들거나, 프로젝트 멤버십을 바꾸거나, 통합 허용 목록 밖의 프로젝트를 노출하면 안 됩니다.

공개 하네스 도구 호출에 대해 어댑터는 먼저 [에이전트 통합](agent-integration.md#current-surface-context)이 담당하는 결정적 프로젝트 선택과 프로젝트별 검증을 수행합니다. 모호한 프로젝트 선택은 Core 실행 전에 거절하고, 실행 가능한 텍스트는 에이전트에게 `harness.list_projects`를 호출하라고 안내해야 합니다.

`harness-mcp`는 MCP 태스크 보강 도구 실행을 광고하거나 구현하지 않습니다. `tools/call` 요청은 `CreateTaskResult`를 반환하지 않으며, `task` 파라미터는 지원되는 기준 기능이 아닙니다.

하네스까지 도달한 알려진 공개 하네스 도구 호출에서 `tools/call`은 MCP 결과 안에 하네스 응답 JSON을 래핑합니다.

- 하네스 응답 JSON은 `result.content[0].text`의 문자열로 직렬화됩니다.
- 클라이언트는 하네스 응답을 검사하려면 그 문자열을 JSON으로 파싱해야 합니다.
- 성공한 MCP 전송은 하네스 도메인 수준 거절 응답을 포함해 `isError: false`를 반환합니다.
- 하네스 도메인 성공 또는 거절은 파싱한 하네스 응답, 특히 `base.response_kind`와 `errors`에서 판단합니다.
- JSON-RPC `error`는 프로토콜, 잘못된 파라미터, 어댑터/내부 실패에만 사용합니다. 하네스 도메인 수준 거절에는 사용하지 않습니다.

하네스 응답 분기 형태와 오류 의미는 각 담당 문서에 둡니다.

- 공통 응답 분기: [API 코어 스키마](api/schema-core.md#common-response)
- 응답 분기 처리 경로: [API 오류 처리 경로](api/error-routing.md)
- 공개 오류 코드: [API 오류 코드](api/error-codes.md)
- 기계 판독용 오류 세부사항: [API 오류 세부사항](api/error-details.md)

## 종료와 재연결

stdin을 닫거나 자식 프로세스를 종료하면 MCP 세션이 끝납니다.

종료와 재연결 규칙:

- SQLite 상태는 Runtime Home에 남습니다.
- 같은 `integration_id`로 다시 시작하면 같은 통합과 현재 레지스트리 상태에 다시 연결합니다.
- 통합을 바꾸려면 새 프로세스나 호스트 설정 갱신이 필요합니다.

런타임 데이터 위치 경계는 [런타임 경계](runtime-boundaries.md)가 담당하고, 저장소 기록 세부사항은 [저장소](storage.md)가 안내하는 저장소 담당 문서가 담당합니다.
