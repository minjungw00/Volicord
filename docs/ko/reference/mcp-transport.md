# MCP 전송 참조

이 문서는 로컬 `volicord mcp --stdio` 프로세스 계약과 로컬 및 Docker
`volicord serve --transport local-http` 프로세스 경계 계약을 담당합니다. 여기에는
프로세스 시작, 프로세스 환경, MCP 프로토콜 버전 협상, 초기화 수명주기, stdio 전송
프레이밍, 로컬 HTTP MCP 요청 처리, JSON-RPC 메시지 검증, Agent Connection에 묶인 시작
검증, MCP에 보이는 도구 탐색, MCP 응답 래핑, 종료와 재연결 동작이 포함됩니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `volicord mcp --stdio` 프로세스 시작과 종료 동작
- `volicord serve --transport local-http` 시작, 로컬 리스너, 전송 경계 인증과 Origin 점검
- 생성된 호스트 설정과 사용자가 관리하는 일반 호스트 설정의 프로세스 설정
- MCP Runtime Home 경로 해석
- MCP 프로토콜 버전 협상과 초기화 수명주기
- stdio JSON-RPC 프레이밍, 메시지 검증, 지원되는 MCP 메서드
- 루프백 전용 로컬 HTTP 전송의 JSON-RPC 요청 처리
- stdio 전송 경계에서 서버가 시작하는 MCP `elicitation`
- 대기 중인 사용자 판단을 위한 로컬 consent URL 대체 경로
- 하나의 내부 Agent Connection 바인딩에 대한 MCP 시작 검증
- 전송 경계에서의 MCP `tools/list`와 `tools/call` 동작
- 내부 래퍼와 호출 메타데이터를 숨기는 MCP 표시 입력·출력 도구 스키마 투영(MCP 전용
  생략 기본값과 입력 예시 포함)
- MCP `tools/call` 응답 래핑과 어댑터 오류 형태
- 프로세스 종료와 재연결 동작

이 문서는 담당하지 않습니다.

- 공개 Volicord 메서드 목록이나 메서드 담당 표: [API 메서드](api/methods.md)
- 공개 Volicord 요청/응답 스키마: [API 코어 스키마](api/schema-core.md)
- Agent Connection, Connection Projects, 프로젝트 선택 의미, 현재 연결 맥락, 행위자 출처:
  [Agent Connection](agent-connection.md)
- Runtime Home 초기 설정, 연결, 프로젝트, 내보내기, 검증 관리 명령: [관리 CLI](admin-cli.md)
- 생성된 호스트 훅 명령 문법, 훅 경로 안전성 진단, 호스트 훅 래퍼 복구:
  [관리 CLI](admin-cli.md#guard-hook-commands)
- 저장소 배치, 스키마 초기화와 검증, 저장 효과: [저장소](storage.md)가 안내하는 저장소 담당 문서

<a id="surface-stability"></a>
## 표면 안정성

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)의
기준 어휘를 따릅니다.

| 표면 | 안정성 | 비고 |
|---|---|---|
| `volicord mcp --stdio`, stdio JSON-RPC 프레이밍, MCP 초기화, 지원되는 MCP 메서드, `tools/list`, `tools/call`, 응답 래핑 | `stable` | 지원되는 메서드 집합을 위한 로컬 프로세스와 MCP 전송 계약입니다. |
| 로컬 HTTP 전송, Docker 호스트 루프백 노출 형태, 로컬 consent URL 대체 경로 | `beta` | 담당 문서가 정한 제한 안에서 지원됩니다. 공개 네트워크 API나 전체 MCP Streamable HTTP 호환성이 아닙니다. |
| 프로세스 바인딩 값, 생성된 호스트 설정 세부사항, 내부 연결·프로젝트 식별 정보, 숨겨진 호출 메타데이터 | `internal` | 로컬 프로세스와 생성된 어댑터를 연결하는 세부사항입니다. 집중 담당 문서가 선택자를 노출하지 않는 한 공개 MCP 도구 스키마에서는 숨깁니다. |
| 시작 진단, `/healthz`, 구조화된 HTTP 오류 보고서, 사람이 읽는 전송 경고 | `diagnostic` | 문서화된 코드와 고지 문구는 보존합니다. 산문 표현은 공개 API 스키마가 아닙니다. |

## 프로세스 모델

`volicord mcp --stdio`는 설치된 `volicord` 실행 파일의 로컬 MCP stdio 프로세스
모드입니다. MCP 호스트는 이를 자식 프로세스로 시작하고 stdin/stdout으로 통신합니다.
MCP TCP 리스너, HTTP MCP 리스너, Unix 도메인 소켓 리스너, 또는 그 밖의 MCP 네트워크
리스너가 아닙니다. 호스트 프롬프트 입력과 채팅 명령 캡처를 사용할 수 없을 때는 대기
중인 사용자 판단을 위해 별도의 루프백 전용 consent 리스너를 시작할 수 있습니다.

`volicord serve --transport local-http`는 Docker와 `localhost` MCP 사용을 위한 별도의
명시적 프로세스 모드입니다. 네이티브 로컬 실행은 루프백 전용 HTTP 리스너를
시작합니다. Docker 실행은 호스트 루프백 포트 노출과 함께 명시적 `--container-listen`
모드만 사용할 수 있습니다. 가능한 곳에서는 stdio와 같은 Agent Connection에 묶인 MCP
어댑터 로직을 재사용합니다. 기본 MCP 전송이 아니며, Docker가 아닌 로컬 호스트 설정
생성에서 사용하지 않고, 일반 Volicord 네트워크 서비스도 아닙니다. 로컬/Docker 전송일 뿐
공개 네트워크 API, SaaS 엔드포인트, 다중 사용자 서버, 보안 경계가 아닙니다. 이 프로세스를
공개 호스트 인터페이스나 원격 HTTP 서비스로 바꾸는 명령 옵션은 없습니다.

현재 로컬 HTTP 전송은 인증을 요구하는 MCP-over-HTTP 부분 구현입니다. MCP 세션 헤더와
베어러 토큰 검사와 함께 HTTP `POST /mcp`로 JSON-RPC를 받고 JSON 응답을 반환합니다.
서버 전송 이벤트 스트림, HTTP `elicitation`, 전체 MCP Streamable HTTP 호환성은 구현하지
않습니다. 문서와 시작 진단은 이 부분 구현만 설명하며 전체 프로토콜 호환성을 주장하지
않습니다.
시작 진단, `/healthz`, 구조화된 HTTP 오류 JSON은 `detective_observation` 공개를
포함합니다. 이는 전송 진단이며 OS 샌드박싱, 네트워크 격리, 악성 코드 방어,
전체 쓰기 방지, 행위자 귀속 증명, 정확성 증명, 테스트 충분성 증명, 인간 검토 대체가
아닙니다.

생성된 호스트 설정과 사용자가 관리하는 일반 호스트 설정은 내부 연결 바인딩으로 stdio
루프를 시작합니다. 설정 항목이 안전하게 프로젝트에 묶이면 선택된 내부 프로젝트
바인딩도 함께 담습니다.

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
```

생성된 프로젝트 범위 Codex 설정은 관리 시작 출처 환경 변수 마커
`VOLICORD_MCP_LAUNCH=managed_host`, `VOLICORD_MCP_HOST=codex`,
`VOLICORD_MCP_CONNECTION_ID=<connection_id>`,
`VOLICORD_MCP_PROJECT_ID=<project_id>`도 설정합니다. Codex 검증은 일치하는
출처 마커 없이 명령과 인자만 있는 항목을 변경된 설정으로 취급하며, 그런 항목은
관리 설정 일치로 볼 수 있기 전에 다시 생성해야 합니다.

`<connection_id>` 프로세스 바인딩 값은 `volicord init` 또는
`volicord connection add`가 만든 저장 `connection_internal_id`에서 옵니다.
선택적 `<project_id>` 프로세스 바인딩 값은
그 연결에 이미 허용된 저장 `project_internal_id`입니다. 일반 사용자가 텍스트 모드
흐름에서 두 값 중 어느 것도 입력할 필요가 없어야 합니다.

기준 명령줄 동작:

- `volicord mcp --stdio --connection <connection_id> [--project <project_id>]`는 stdio
  루프를 시작합니다. `--project`가 있으면 제공한 값은 연결 허용 목록 안에 있어야 하며,
  stdio 프로세스는 도구 요청을 처리하기 전에 그 프로젝트로 좁혀집니다.
- `volicord mcp --check --connection <connection_id>`는 stdin을 읽지 않고 시작 검증을
  실행합니다.
- `volicord mcp --check --connection <connection_id> --project <project_id>`는 같은 시작
  검증을 실행하고 프로젝트 세부 진단을 허용 목록 안의 `project_internal_id` 값 하나로
  제한합니다.
- `-h`와 `--help`는 사용법과 환경 요약을 출력한 뒤 종료 코드 `0`으로 끝납니다.
- `-V`와 `--version`은
  `volicord <package-version> (build_id=<build-id>)`를 출력한 뒤 종료 코드 `0`으로 끝납니다.
- 모드 없음, `--connection` 없는 `--check` 또는 `--stdio`, 알 수 없는 옵션, 결합된
  명령줄 모드, 필요한 옵션 값 누락, 추가 위치 인자는 사용법 진단을 stderr에 쓰고 종료
  코드 `2`로 끝납니다.
- help와 version 처리는 Runtime Home이나 Agent Connection 조회보다 먼저 일어납니다.

성공한 `--check` 출력은 진단 보고서입니다. 이 보고서는 설정 유효성, stdio 전송,
Runtime Home, `connection_id`, `connection.mode`, 연결 활성 상태, 레지스트리 읽기 상태,
선택된 프로젝트 상태 읽기·쓰기 상태, 시작 관찰 상태, 유효 도구 모드,
`tools/list` 스키마 검증, 도구 이름 스타일, 프로젝트 사용 가능
상태, `verification_scope`를 보고합니다.
`registry_read`와 `project_state_read`는 읽기 가능 여부를 보고합니다.
`project_state_write` 진단은 지속 저장되지 않는 SQLite 쓰기 가능성 검사를 사용하며
`passed`, `readonly`, `failed`, `skipped` 중 하나를 보고합니다. 이 검사는 Core 기록,
지속 저장되는 진단 행, 재실행 행, 세션 감시 기록, 도구 호출 기록을 만들거나
`project_state.state_version`을 증가시키면 안 됩니다. `startup_observation`은 사용 가능한
프로젝트가 하나인 일반 stdio 시작이 `recordable`인지, `best_effort_skipped_if_readonly`로
건너뛰어질 것인지, 또는 `skipped_verification_probe`인지를 보고합니다.
`effective_tool_mode`는 `workflow`, `read_only_degraded`, `read_only`, `unavailable` 중
하나이며 같은 연결과 프로젝트 저장 가능성에서의 실제 `tools/list` 동작과 일치해야
합니다. 쓰기 가능성 진단이 `passed`라고 해서 OS 샌드박싱, 호스트 신원, 보안 격리,
앞으로의 쓰기 성공, Product Repository 쓰기 권한이 증명되지는 않습니다.

로컬 HTTP 전송 명령줄 동작:

- `volicord serve --transport local-http`만 지원되는 로컬 HTTP 전송 표기입니다. 다른 전송
  값은 사용법 오류입니다.
- `--listen 127.0.0.1:<port>` 또는 `--listen [::1]:<port>`는 리스너를 선택합니다.
  생략하면 `127.0.0.1:8765`를 사용합니다.
- 네이티브 `--listen`은 루프백 전용입니다. `--listen`으로 `0.0.0.0`, `::`, 공개
  인터페이스, 컨테이너 전체 인터페이스, 또는 루프백이 아닌 주소에 바인딩하려 하면
  거절합니다.
- `--container-listen 0.0.0.0:<port>` 또는 `--container-listen [::]:<port>`는 명시적
  Docker 호스트 루프백 노출 모드입니다. 고정된 0이 아닌 컨테이너 포트가 필요하며
  `-p 127.0.0.1:8765:8765` 같은 호스트 루프백 노출 규칙과 함께 사용해야 합니다.
  네이티브 로컬 실행에는 유효하지 않으며 공개 호스트 인터페이스나 원격 제공 옵션이
  아닙니다.
- `--listen`과 `--container-listen`은 함께 사용할 수 없습니다.
- `--home PATH`는 프로세스의 Runtime Home을 선택합니다. `--home`이 없으면 공통
  `VOLICORD_HOME`과 플랫폼 기본 Runtime Home 해석을 사용합니다.
- `--connection <connection_id>`는 서버를 저장된 Agent Connection 하나에 묶습니다. 이
  옵션이 없으면 선택적 로컬 HTTP 프로젝트 허용 목록과 일치하고 연결 프로젝트가 있는 활성
  Agent Connection이 정확히 하나일 때만 시작이 성공합니다.
- `--project PATH`는 반복할 수 있습니다. 각 경로는 등록된 저장소 루트로 해석되며 로컬 HTTP
  프로세스를 해당 프로젝트 식별 정보들로 좁힙니다. 이렇게 좁힌 집합도 선택된 Agent
  Connection의 연결 프로젝트 허용 목록 안에 있어야 합니다.
- `--token-file PATH`는 UTF-8 로컬 파일에서 베어러 토큰을 읽습니다. 끝의 줄바꿈은
  토큰에 포함하지 않습니다. 로컬 비밀값이 셸 기록이나 로컬 HTTP 프로세스 인자에 직접
  남지 않도록 `--token`보다 `--token-file`을 선호합니다.
- `--token TOKEN`은 베어러 토큰을 명령줄에 직접 제공합니다. 통제된 로컬 사용을 위해
  지원하지만 문서에서 선호하는 형태는 아닙니다.
- `--token-file`과 `--token`을 모두 생략하면 Volicord가 프로세스 로컬 토큰을 생성하고
  시작 중 stderr에 씁니다. 생성 토큰 출력은 그 토큰이 로컬 비밀값이며 엔드포인트를
  호스트 루프백 또는 의도한 Docker 호스트 루프백 경계에만 두어야 한다고 경고합니다.
- `--generate-token`은 토큰 생성 경로를 명시적으로 선택하며 `--token-file` 또는
  `--token`과 함께 사용할 수 없습니다.
- `--allow-origin ORIGIN`은 반복할 수 있으며 정확히 일치하는 Origin 값의 브라우저 가능
  요청을 허용합니다. 이 옵션이 없으면 `Origin` 헤더가 있는 요청은 거절되고 CORS 응답
  헤더를 내지 않습니다.

종료 코드와 스트림 동작:

- stdin EOF로 정상 종료하면 stdout을 플러시하고 종료 코드 `0`으로 끝납니다.
- 성공한 `--check`는 보고서를 stdout에 쓰고 종료 코드 `0`으로 끝납니다.
- 시작 중 설정, JSON, 저장소 오류는 진단을 stderr에 쓰고 종료 코드 `1`로 끝납니다.
- 로컬 HTTP 시작 설정, 리스너, 인증 토큰, Origin, 프로젝트 허용 목록 오류는 진단을
  stderr에 쓰고 종료 코드 `1`로 끝납니다.
- 로컬 HTTP 시작 진단은 로컬 HTTP 전송이 호스트 루프백 또는 의도한 Docker 호스트 루프백
  노출에만 쓰인다고 경고합니다. `--container-listen`은 공개 인터페이스나 원격 호스트를
  위한 옵션이 아니라는 추가 경고를 냅니다.
- stdio 루프가 실행 중일 때 잘못된 JSON과 지원하지 않는 JSON-RPC 요청은 응답을 쓸 수
  있으면 JSON-RPC 오류를 반환합니다.

로컬 HTTP 요청 동작:

- MCP 엔드포인트 경로는 `/mcp`입니다.
- `POST /mcp`에는 `Authorization: Bearer <token>`, `Content-Type: application/json`,
  그리고 `application/json`과 `text/event-stream`을 모두 포함하는 `Accept` 헤더가
  필요합니다.
- 성공한 `initialize`는 `Mcp-Session-Id`를 만듭니다. 이후 JSON-RPC 요청은 그 세션 ID를
  제공해야 합니다.
- `DELETE /mcp`는 베어러 토큰과 세션 ID가 유효할 때 세션을 삭제합니다.
- `GET /mcp`는 `SSE_UNSUPPORTED`를 반환합니다. 서버 전송 이벤트 스트림은 이 로컬 HTTP
  엔드포인트에서 구현하지 않습니다.
- `GET /healthz`는 최소 로컬 상태 확인 엔드포인트이지만 같은 베어러 토큰을 요구합니다.
- `GET /consent`와 `POST /consent`는 로컬 consent URL을 사용할 수 있을 때만 열리는
  엔드포인트입니다. MCP 엔드포인트가 아니며 MCP 베어러 토큰을 사용하지 않습니다.
  프로젝트, 연결, 대기 판단에 묶인 유효한 일회성 consent 토큰이 필요한 루프백 User
  Channel 입력 경로입니다.
- 인증 없이 접근할 수 있는 임의 리소스 엔드포인트는 없습니다.
- 브라우저 대상 요청은 `Origin` 헤더가 있는지로 식별합니다. `Origin`이 있는 MCP
  엔드포인트 요청은 정확한 `--allow-origin` 값과 일치해야 합니다. `Origin`이 있는 로컬
  consent 양식 제출은 consent 엔드포인트 자신의 Origin과 일치해야 합니다.
- CORS 사전 요청은 MCP 엔드포인트에 대해서만, Origin 허용 목록 검증 뒤에만, 그리고 허용된
  Origin이 하나 이상 설정되어 있을 때만 받습니다.
- Local HTTP 응답에는 `Cache-Control: no-store`와 `X-Content-Type-Options: nosniff`가
  포함됩니다. consent HTML 응답은 `Referrer-Policy: no-referrer`와 제한적인
  `Content-Security-Policy`도 포함합니다. CORS 응답 헤더는 명시적으로 허용된 Origin에
  대해서만 냅니다.
- 요청 헤더는 16 KiB, 요청 본문은 1 MiB로 제한됩니다. 더 큰 헤더는
  `HTTP_HEADERS_TOO_LARGE`, 더 큰 본문은 `HTTP_BODY_TOO_LARGE`로 실패합니다.
- 구조화된 HTTP 오류는 인증, Origin, 프로젝트 허용 목록, 지원하지 않는 전송, 지원하지
  않는 메서드, 지원하지 않는 콘텐츠 협상 실패에 안정적인 전송 오류 코드를
  사용합니다.

Docker 노출 동작:

- 지원되는 Docker 노출 형태는 호스트 루프백을 컨테이너 포트에 매핑합니다. 예:
  `-p 127.0.0.1:8765:8765`.
- 이 형태에서 컨테이너 프로세스는 Docker가 호스트 루프백으로 노출한 포트를 컨테이너로
  전달할 수 있도록 `--container-listen 0.0.0.0:8765`를 사용합니다.
- 컨테이너 포트를 `0.0.0.0`, 공개 호스트 인터페이스, 원격 호스트에 노출하는 것은 Local
  HTTP 전송 계약 밖입니다.
- Docker 노출은 위의 전송 경계 베어러 토큰과 Origin 점검을 넘어서는 인증, 인가,
  다중 사용자 격리, 호스트 신뢰, 더 넓은 보안 경계를 추가하지 않습니다.

세션 감시 시작 범위:

- stdio 시작이 `--project <project_id>` 또는 사용할 수 있는 허용 프로젝트가 정확히 하나인
  연결 맥락 때문에 프로젝트에 묶이면, 프로세스는 한정된 스냅샷 생성을 사용할 수 있을 때
  도구 요청을 처리하기 전에 세션 감시 기준선을 만들거나 연결합니다. 관찰 범위 근거는
  `mcp_start`입니다.
- 관리 출처 마커가 검증된 생성 Codex 시작에서는 쓰기 가능한 저장소를 사용할 수 있을 때
  stdio 프로세스가 같은 기준선에 `managed_host_startup`,
  `managed_host_initialize_response`, `managed_host_tools_list`,
  `managed_host_tool_call` 관찰에 대한 관리 생명주기 메타데이터도 추가합니다. 각
  생명주기 이벤트는 선택된 연결과 프로젝트, `host_kind=codex`,
  `launch_origin=managed_host`, 시각, 관찰된 저장 기능, 사용할 수 있을 때의 유효 도구
  모드를 기록합니다.
- 로컬 HTTP 초기화가 `Mcp-Session-Id`를 만들고 선택된 연결·프로젝트 맥락에 사용할 수
  있는 허용 프로젝트가 정확히 하나이면, 서버는 그 세션의 이후 도구 요청을 받기 전에
  같은 `mcp_start` 기준선을 만들거나 연결합니다.
- 세션에 사용할 수 있는 프로젝트가 여전히 둘 이상이면 감시 범위는
  `pending_project_selection`입니다. 도구 요청이 명시적인 `project_selector`를 이름 붙이기
  전에는 전체 탐지 범위를 주장하지 않습니다.
- 프로젝트가 선택된 메서드 요청이 첫 기준선을 만들면 명시적 선택의 근거는
  `first_project_selection`, 단일 프로젝트 메서드 경계 대체 경로의 근거는
  `method_boundary`입니다. 두 경우 모두 더 이른 Product Repository 변경은 감시 범위
  밖이므로 부분 범위를 보고합니다.
- 이런 기준선 생성 시도는 한정된 관찰입니다. 쓰기를 막거나, 파일을 바꾼 행위자를 식별하거나,
  원시 파일 내용을 저장하거나, OS 수준 강제를 만들지 않습니다.

<a id="process-environment"></a>
## 프로세스 환경

MCP 프로세스는 아래처럼 역할이 제한된 환경 입력을 해석합니다.

지원되는 운영자 및 Runtime Home 입력:

- `VOLICORD_HOME`
- `VOLICORD_LOCAL_WEB_CONSENT`
- `VOLICORD_HOME`이 없을 때 사용하는 표준 플랫폼 홈 환경 변수인 `HOME`,
  `USERPROFILE`, `HOMEDRIVE`와 `HOMEPATH` 조합

`VOLICORD_HOME`은 프로세스의 Runtime Home을 선택합니다. 일반 흐름에서 사용자가 직접
입력하는 값이 아니라, 필요할 때 생성된 호스트 설정이 보통 기록하는 값입니다. 이 값은
프로젝트, 연결 의도, 행위자 출처, 작업 범주, 연결 모드, 호스트 신뢰 상태를 선택하지
않습니다. stdio 프로세스와 `--check`는 시작 검증에 들어가기 전에 `VOLICORD_HOME`을
사용합니다. help와 version 모드는 이를 사용하지 않습니다.

`VOLICORD_LOCAL_WEB_CONSENT=0`, `false`, `off`, `disabled`는 stdio local web consent
리스너를 끕니다. 다른 값은 리스너 주소나 토큰 정책을 바꾸지 않습니다.

`VOLICORD_MCP_VERIFICATION=1`은 진단 전용 마커입니다. 관리 명령
`volicord connection verify` 흐름은 자식 MCP handshake에 이 값을 자동으로 설정합니다.
운영자가 직접 설정하는 지원 경로는 한정된
[수동 stdio 생명주기 점검](#manual-stdio-lifecycle-probe)뿐입니다. 이 값은 일반 연결과
프로젝트 시작 점검을 유지하지만 프로세스를 검증 점검으로 분류하므로, 프로세스가
시작 세션 감시 기준선이나 관리 Codex 런타임 관찰을 만들지 않습니다. 일반
호스트 설정에 사용하는 값이 아닙니다.

Volicord가 관리하는 Codex 설정은 다음과 같은 관리 시작 출처 마커를 담습니다.

- `VOLICORD_MCP_LAUNCH=managed_host`
- `VOLICORD_MCP_HOST=codex`
- `VOLICORD_MCP_CONNECTION_ID=<connection_id>`
- 명령에 프로젝트 바인딩이 있을 때의 `VOLICORD_MCP_PROJECT_ID=<project_id>`

이 마커들은 Volicord 관리 설정 식별 정보의 일부이며 일반 운영자 선택자가 아닙니다.
사용자 관리 시작을 관리 시작처럼 보이게 만들려고 직접 추가하거나 바꾸지 말고,
`volicord init` 또는 `volicord connection add`로 관리 설정을 다시 생성합니다. Connection과
선택적 project 값은 대응하는 프로세스 인자와 일치해야 합니다. 관리 마커가 하나도 없는
시작은 수동으로 분류됩니다. 일부만 있거나 값이 맞지 않는 마커 집합은 유효하지 않은 관리
출처이며 관리 생명주기 관찰을 만들지 않습니다. 이 마커는 프로젝트 접근, 호스트
신뢰, 더 넓은 권한을 부여하지 않습니다.

연결 식별 정보는 생성된 호스트 설정이나 사용자가 관리하는 일반 호스트 설정 안의
`--connection <connection_id>`로 제공합니다. 이것은 선택된 Agent Connection에 대한 내부
프로세스 바인딩이며, 사용자가 보통 직접 고르거나 관리하는 값이 아닙니다. 묶인 Agent
Connection과 Runtime Home 레지스트리 상태가 연결 모드, 연결 프로젝트, 어댑터가 파생하는
`actor_source`와 `operation_category`를 제공합니다. 프로젝트 접근은 선택된 Agent
Connection의 연결 프로젝트와 저장소 루트 해석으로 제어됩니다. 그 밖의 Volicord 전용
환경 변수는 지원되는 운영자 설정이 아닙니다.

현재 MCP Runtime Home 경로 해석:

1. `VOLICORD_HOME`이 존재하지만 비어 있으면 오류입니다.
2. 절대 경로 `VOLICORD_HOME`은 제공된 그대로 사용합니다.
3. 상대 경로 `VOLICORD_HOME`은 그 경로가 존재하지 않아도 프로세스의 현재 작업
   디렉터리를 기준으로 해석합니다.
4. `VOLICORD_HOME`이 없으면 플랫폼 홈 환경 변수에서 기본 사용자 홈을 구하고
   `.volicord`를 붙입니다. Windows가 아닌 플랫폼에서는 `HOME`, `USERPROFILE`,
   `HOMEDRIVE`와 `HOMEPATH` 조합 순서로 시도합니다. 네이티브 Windows에서는
   `USERPROFILE`, `HOMEDRIVE`와 `HOMEPATH` 조합, WSL 형식 mount 경로가 아닌 `HOME`
   순서로 시도합니다.
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
Does not prove: public API availability, authentication service status, security boundary, full MCP Streamable HTTP compatibility, OS sandboxing, network isolation, write prevention, actor identity proof, correctness proof, test sufficiency proof, or human review completion
runtime_home: <absolute path>
connection_id: <connection_internal_id process-binding value>
mode: workflow|read_only
enabled: true|false
registry_read: passed
project_state_read: passed|failed
project_state_write: passed|readonly|failed|skipped
startup_observation: recordable|best_effort_skipped_if_readonly|skipped_verification_probe
effective_tool_mode: workflow|read_only_degraded|read_only|unavailable
tools_list_schema_validation: passed|failed
tool_naming_style: dotted_namespace
allowed_projects: <count>
available_projects: <count>
verification_scope: startup_check_only
watcher_status: pending_mcp_start|pending_project_selection|unavailable
watcher_baseline_created_at: <timestamp or empty>
watcher_coverage_start_at: <timestamp or empty>
watcher_coverage_basis: mcp_start|empty
watcher_partial_coverage_warning: <warning or empty>
project[0].project_id: <project_internal_id diagnostic value>
project[0].available: true|false
project[0].state_read: passed|failed
project[0].state_write: passed|readonly|failed|skipped
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
- `Does not prove`는 시작 진단의 비보장을 요약하며, 메서드 호출에 쓰이는 기계 판독 가능
  Core 응답 공개를 바꾸지 않습니다.
- `registry_read`는 시작 검증에서 Runtime Home 레지스트리를 읽을 수 있었는지 보고합니다.
- `project_state_read`는 선택된 프로젝트 상태 집합의 읽기 접근을 요약합니다. 프로젝트별
  `state_read` 줄은 각 세부 블록의 같은 사실을 보고합니다.
- `project_state_write`는 선택된 프로젝트 상태 집합의 유효 쓰기 가능성을 요약합니다.
  프로젝트별 `state_write` 줄은 각 세부 블록의 같은 기능을 보고합니다.
- `startup_observation`은 일반 시작이 한정된 세션 감시 관찰을 기록할 수 있는지,
  읽기 전용 저장소에서는 그 관찰을 건너뛸 것인지, 또는 검증 점검에 그치는지를
  보고합니다.
- `effective_tool_mode`는 같은 연결과 프로젝트 저장 가능성에서 시작 점검이 예상하는
  `tools/list` 모드를 보고합니다.
- `tools_list_schema_validation`은 해당 유효 도구 모드의 MCP 표시 도구 목록이 MCP 도구
  이름, 객체 입력 스키마, 필수 필드, 속성 형태에 대한 Volicord의 클라이언트 호환성
  검사를 통과했는지 보고합니다. 이것은 Volicord 쪽 진단이며 호스트가 도구를 등록하거나
  노출한다는 증명이 아닙니다.
- `tool_naming_style: dotted_namespace`는 해당 Volicord 도구 이름이 `volicord.*`
  점 구분 이름 공간을 사용한다는 것을 보고합니다. 이 줄은 점 없는 별칭을 만들지
  않습니다.
- `allowed_projects`는 Agent Connection 허용 목록 전체를 설명합니다.
- 사용할 수 없는 프로젝트도 모든 프로젝트 세부 키를 출력합니다. `unavailable_reason`은
  사용할 수 없는 프로젝트에서 채워지고 사용할 수 있는 프로젝트에서는 비어 있습니다.
- `verification_scope: startup_check_only`는 시작과 사전 점검에 대한 문장일 뿐이며 전체
  호스트 검증이 아닙니다.
- `--check`는 세션 감시 기준선을 만들지 않습니다. `watcher_status:
  pending_mcp_start`는 향후 프로젝트에 묶인 stdio 또는 HTTP 세션이 `mcp_start` 근거로
  관찰을 시작할 수 있음을 뜻합니다. `pending_project_selection`은 향후 세션이 관찰을
  시작하기 전에 프로젝트를 선택해야 함을 뜻합니다.
- 빈 `watcher_baseline_created_at`, `watcher_coverage_start_at`,
  `watcher_coverage_basis` 값은 이 사전 점검 명령이 기준선을 만들지 않았다는 뜻입니다.
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
  범위에서 지원되는 메시지 객체는 요청과 `notifications/initialized` 알림입니다.
  배열, 원시 JSON 루트, `null`은 유효하지 않은 MCP stdio 메시지입니다.
- JSON-RPC 배치는 지원하지 않습니다. 배열 입력은 배열 요소마다 응답을 내지 않고 Invalid
  Request 응답 하나를 받습니다.
- 메시지는 줄바꿈으로 구분되며 메시지 안에 줄바꿈을 포함하면 안 됩니다.
- 각 출력 줄은 JSON-RPC 응답 객체 하나를 담습니다. 다만 사용자 입력 요청을 사용할 수 있는
  `tools/call`을 처리하는 동안에는 서버가 시작한 `elicitation/create` 요청이 출력될 수
  있습니다. `volicord mcp --stdio`는 `initialize` 전에 준비 완료 메시지를 쓰지 않습니다.
- stdin EOF는 stdout을 플러시한 뒤 프로세스를 끝냅니다.

JSON-RPC 검증 규칙:

- `jsonrpc`는 정확히 `"2.0"`이어야 합니다.
- 요청 `method`는 문자열이어야 합니다.
- 요청 ID는 문자열 또는 정수일 수 있으며 `null`이면 안 됩니다.
- 분류 가능한 알림은 문자열 `method`를 갖고 `id`가 없으며 MCP 메서드 파라미터가
  잘못되었더라도 응답을 받지 않습니다.
- `id`가 없는 객체가 자동으로 유효한 알림이 되는 것은 아닙니다. 그래도 알림 형태를
  만족해야 합니다.
- 지원되는 MCP 요청의 메서드 `params`는 존재할 때 객체여야 합니다. 수명주기
  알림에서는 `params`가 없거나 객체인 경우에만 수명주기에 영향을 줄 수 있습니다.

알림 분류는 MCP 메서드 파라미터 검증보다 먼저 JSON-RPC 래퍼를 기준으로
이루어집니다. 메시지가 알림으로 분류될 수 있으면 잘못된 `params`가 있어도
JSON-RPC 응답을 만들지 않습니다. 그러나 그런 `params`는 수명주기 목적에서는 유효하지
않습니다. 잘못된 `notifications/initialized`는 연결을 준비 상태로 옮기지 않고,
알림으로 받은 요청 전용 메서드는 무시되며 실행하면 안 됩니다.

오류 분류:

| 조건 | MCP 응답 |
|---|---|
| JSON 파싱 실패 | JSON-RPC `-32700` Parse error |
| 배열, 원시 루트, 누락되었거나 잘못된 `jsonrpc`, 잘못된 요청 `id`, 누락되었거나 문자열이 아닌 요청 `method`, 알림이 아닌 잘못된 객체를 포함한 유효하지 않은 JSON-RPC 메시지 구조 | JSON-RPC `-32600` Invalid Request |
| `initialize` 전 요청, `notifications/initialized` 전 `tools/call`, 중복 `initialize`를 포함한 요청의 수명주기 위반 | JSON-RPC `-32600` Invalid Request |
| 알 수 없는 요청 메서드 | JSON-RPC `-32601` Method not found |
| 요청의 잘못된 메서드 파라미터 | JSON-RPC `-32602` Invalid params |
| 구조적으로 유효한 `tools/call` 요청의 알 수 없는 도구 이름 | JSON-RPC `-32602` Invalid params |
| 어댑터 또는 서버 내부 실패 | 적절한 JSON-RPC 내부 오류 응답 |
| 분류 가능한 알림. 잘못된 메서드 파라미터가 있는 경우도 포함 | 응답 없음. 잘못된 파라미터는 수명주기 전환이나 요청 전용 동작을 일으키지 않습니다. |

### 프로토콜 버전과 수명주기

연결에서 첫 번째로 유효한 MCP 요청은 `initialize`입니다. 유효한 `initialize` 요청은
객체 `params` 안에 아래 값을 둡니다.

- 문자열 `protocolVersion`
- 객체 `capabilities`
- 문자열 `name`과 `version` 필드를 포함하는 객체 `clientInfo`

`params.capabilities.elicitation`이 객체이면 어댑터는 MCP 클라이언트가 서버 시작
사용자 입력 요청을 사용할 수 있다고 봅니다. 다른 기능 항목은 그 자체로 Volicord 동작을
만들지 않습니다.

예시는 위에 나열한 필드를 사용합니다. `volicord mcp --stdio`는 2025-11-25 스키마가
허용하는 추가 MCP `Implementation` 메타데이터, 예를 들어 `title`, `description`,
`icons`, `websiteUrl`을 받을 수 있습니다.

성공한 initialize 결과는 `serverInfo.name=volicord-mcp`를 반환하고 상속된 Cargo
패키지 SemVer를 `serverInfo.version`에 유지합니다. `serverInfo`에는 표준 MCP
`Implementation` 필드만 둡니다. 표준 initialize Result의 `_meta` 객체는 Volicord
확장 `_meta["io.volicord/build"]`를 노출하며, 비표준 `serverInfo.buildId` 필드는
사용하지 않습니다. 확장 값은 `volicord doctor --json`에 문서화된 구조화 빌드 객체와
같으며 `build_id`, Git 메타데이터 출처, 타깃, 정확한 프로필 또는 근사 프로필 계열,
최적화 수준, 디버그 상태를 포함합니다. 빌드 시각은 포함하지 않습니다. 알 수 없는 Git
메타데이터는 명시적으로 표현하고, dirty 작업 트리는 수정된 내용을 정확히 식별한다고
주장하지 않은 채 표시합니다.

프로토콜 버전 협상:

- 클라이언트가 `2025-11-25`를 요청하면 `volicord mcp --stdio`는 `2025-11-25`를 반환합니다.
- 클라이언트가 문법적으로 유효한 다른 프로토콜 버전 문자열을 보내면 `volicord mcp --stdio`는
  자신이 지원하는 버전인 `2025-11-25`를 반환합니다.
- 서버 응답은 더 오래된 MCP 프로토콜 버전과 동시에 호환된다고 주장하지 않습니다.

수명주기 상태:

| 연결 지점 | 유효한 클라이언트 메시지 | 결과 |
|---|---|---|
| 성공한 `initialize` 전 | `initialize` 요청 | 성공하면 서버는 `protocolVersion: "2025-11-25"`를 반환하고 `notifications/initialized`를 기다립니다. |
| `notifications/initialized` 대기 중 | `notifications/initialized` 알림, `ping` 요청, `tools/list` 요청 | `notifications/initialized`가 준비 상태 전환을 완료합니다. `ping`은 `initialize`가 성공한 뒤 사용할 수 있으며, 서버가 알림을 기다리는 동안에도 사용할 수 있습니다. `tools/list`는 성공한 `initialize` 응답 뒤 사용할 수 있는 읽기 전용 탐색입니다. |
| 준비 상태 | `ping`, `tools/list`, `tools/call` | 일반 MCP 도구 탐색과 도구 실행을 사용할 수 있습니다. |

`tools/list`는 성공한 `initialize` 응답 뒤 사용할 수 있으며, 서버가
`notifications/initialized`를 기다리는 동안과 준비 상태 전환 뒤에도 계속 사용할 수
있습니다. `tools/call`은 `notifications/initialized`가 준비 상태 전환을 완료한 뒤에만
사용할 수 있습니다. 중복 `initialize` 요청은 유효하지 않습니다. 너무 이르거나 잘못된
`notifications/initialized` 알림은 연결을 준비 상태로 만들지 않습니다.

<a id="manual-stdio-lifecycle-probe"></a>
### 수동 stdio 생명주기 점검

이 점검은 설정된 Agent Connection을 활성 Codex 호스트 프로세스 밖에서 문제를 해결할
때만 사용합니다. `<repo>`, `<connection_id>`, `<project_id>`를 확인하려는 연결의 값으로
바꿉니다. 프로세스 환경이 이미 의도한 Runtime Home을 선택하지 않는다면 `<repo>`에서
실행합니다. 수동 또는 권한 상승 점검은 그 시작 환경에서 MCP 서버가 실행될 수 있음을
증명할 수 있지만, 활성 Codex 세션이 도구를 등록하거나 노출했다는 증명은 아닙니다.
`VOLICORD_MCP_VERIFICATION=1`은 실행을 검증 점검으로 표시합니다. 일반 Agent Connection과
프로젝트 시작 점검은 유지하지만, 이 프로세스를 Codex 호스트 런타임 관찰로 기록하거나
시작 세션 감시 기준선을 만들지는 않습니다. 관리 Codex 출처 마커 없이 실행한
`volicord mcp --stdio`는 호스트 런타임 관찰 목적에서 수동 CLI 시작으로 분류됩니다.

프로세스 명령 형태는 아래와 같습니다.

```sh
VOLICORD_MCP_VERIFICATION=1 volicord mcp --stdio --connection "<connection_id>" --project "<project_id>"
```

`initialize` 뒤 `tools/list`를 보내면 성공한 JSON-RPC 응답과 모드에 맞는 `volicord.*`
도구 목록을 반환해야 합니다.

```sh
cd "<repo>"
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-lifecycle-probe","version":"0.0.0"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | VOLICORD_MCP_VERIFICATION=1 volicord mcp --stdio --connection "<connection_id>" --project "<project_id>"
```

`initialize`, `notifications/initialized`, `tools/list` 순서도 성공해야 합니다.

```sh
cd "<repo>"
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-lifecycle-probe","version":"0.0.0"}}}' \
  '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | VOLICORD_MCP_VERIFICATION=1 volicord mcp --stdio --connection "<connection_id>" --project "<project_id>"
```

`notifications/initialized` 전에 `tools/call`을 보내면 JSON-RPC Invalid Request로 실패해야
합니다. 초기화 완료 알림 전에는 도구 실행이 준비되지 않았기 때문입니다.

```sh
cd "<repo>"
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"volicord-lifecycle-probe","version":"0.0.0"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"volicord.status","arguments":{}}}' \
  | VOLICORD_MCP_VERIFICATION=1 volicord mcp --stdio --connection "<connection_id>" --project "<project_id>"
```

`notifications/initialized` 뒤에는 프로젝트 상태를 읽을 수 있을 때 읽기 전용
`volicord.status` 호출이 성공할 수 있습니다. 유효 저장소가 읽기 전용이면 워크플로
변경 호출은 `tools/list`에 없을 수 있습니다. 호스트 쪽 오래된 캐시가 그래도 그
도구를 호출하면 MCP 결과는 쓰기 준비 상태를 증명하지 않고 Volicord `MCP_UNAVAILABLE`
거절을 래핑합니다.

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

지원되는 수명주기 알림은 `notifications/initialized`입니다.

<a id="tool-discovery-and-toolscall-response-wrapping"></a>
## 도구 탐색과 `tools/call` 응답 래핑

성공한 `initialize` 응답 뒤 `tools/list`는 현재 저장된 Agent Connection 모드와 선택된
허용 프로젝트의 유효 저장소 읽기·쓰기 가능 여부에 따라 도구를 노출합니다.

| 모드와 저장소 읽기·쓰기 가능 여부 | MCP에 보이는 도구 |
|---|---|
| 쓰기 가능한 프로젝트 상태의 `workflow` | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.prepare_write`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task`, `volicord.list_projects` |
| 읽을 수 있지만 쓸 수 없는 프로젝트 상태의 `workflow` | `volicord.status`, `volicord.check_close`, `volicord.list_projects` |
| 읽을 수 있는 프로젝트 상태의 `read_only` | `volicord.status`, `volicord.check_close`, `volicord.list_projects` |
| 읽을 수 있는 허용 프로젝트 상태가 없음 | `volicord.list_projects` |

MCP 어댑터는 시작과 탐색 중 프로젝트 상태를 읽기 전용으로 살펴볼 수 있습니다. 현재 MCP
호스트 환경에서 프로젝트 상태를 읽을 수는 있지만 쓸 수 없다면, 저장된 Agent Connection
모드가 `workflow`여도 읽기 호환 메서드 도구만 계속 보이고 워크플로 변경 도구는 숨깁니다.
허용 프로젝트 상태를 하나도 읽을 수 없으면 호출자가 프로젝트 사용 가능성을 확인할 수
있도록 `volicord.list_projects`만 보이게 유지합니다.

유효 저장소가 읽기 전용이면 읽기 호환 공개 메서드 도구는 세션 감시 기준선,
`tool_invocations`, `task_events`, 새 `project_state.state_version`을 만들지 않고 실행됩니다.
호스트 쪽 오래된 도구 캐시가 선택된 프로젝트 상태를 쓸 수 없을 때도 공개 워크플로 변경
도구를 호출하면, 어댑터는 Core 쓰기를 시도하지 않고 일반 Volicord 거절 응답을 반환합니다.
이 응답은 `code=MCP_UNAVAILABLE`, `operation_category=agent_workflow`, 메시지
`Volicord project state is not writable in the current MCP host environment.`를 담습니다.

`workflow` 모드의 증거 경로는 다음과 같습니다.

- 바이트나 안전한 알림이 필요할 때만 `volicord.stage_artifact`로 증거 첨부 입력을
  준비합니다.
- `volicord.record_run`으로 Run 또는 관찰, 대상별 증거 갱신, 관찰 출처, 필요한 첨부
  연결 또는 승격을 기록합니다.

스테이징 핸들만으로는 받아들여진 증거가 아니며 닫기 상태를 만족하지 않습니다.

MCP에 보이는 도구는 공개 Volicord Core API 메서드 목록과 같은 것이 아닙니다.
`volicord.check_close`는 닫기 준비 상태를 확인하는 일급 읽기 전용 Core 메서드에
매핑됩니다. `volicord.close_task`는 워크플로 전용 Core 변경 메서드에 매핑되며
`read_only` 연결에는 나열되지 않습니다.
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

MCP 인자 투영은 생략이 기존에 허용하던 명시적 `null` 또는 빈 배열과 정확히 같은 의미인
경우에만 다음 생략 기본값을 적용합니다.

- `volicord.intake`: `initial_context_refs=[]`, `initial_source_refs=[]`
- `volicord.update_scope`: `goal_summary=null`, `scope_update=null`,
  `scope_boundary=null`, `non_goals=null`, `acceptance_criteria=null`,
  `autonomy_boundary=null`, `baseline_ref=null`,
  `related_scope_decision_refs=[]`
- `volicord.prepare_write`: `task_id=null`, `change_unit_id=null`,
  `sensitive_categories=[]`
- `volicord.stage_artifact`: `expected_sha256=null`,
  `expected_size_bytes=null`, `relation_hint=null`
- `volicord.record_run`: `run_id=null`, `write_ticket_id=null`,
  `artifact_inputs=[]`, `evidence_updates=[]`, `evidence_observations=[]`,
  `close_assessment=null`; 각 `evidence_updates` 항목 안에서는
  `supporting_run_refs=[]`, `observation_refs=[]`,
  `supporting_artifact_refs=[]`, `gap_refs=[]`; 각 `evidence_observations`
  항목 안에서는 `observed_by_actor_source=null`, `tool_name=null`,
  `tool_invocation_id=null`, `tool_metadata={}`, `input_refs=[]`,
  `source_refs=[]`, `output_artifact_refs=[]`, `limitations=[]`
- `volicord.request_user_judgment`: `change_unit_id=null`,
  `sensitive_action_scope=null`, `options=null`, `affected_refs=[]`,
  `expires_at=null`

이 기본값은 MCP 표시 인자 DTO에만 속합니다. 디코딩 뒤 어댑터는 모든 멤버를 갖춘 Core
요청 형태를 구성합니다. 따라서 각 메서드 참조가 담당하는 공개 Core API의 멤버 존재
계약은 바뀌지 않습니다. `volicord.request_user_judgment`의 `task_id`,
`judgment_kind`, `presentation`, `question`, `context`, `required_for`는 계속 필수 MCP
인자입니다. `volicord.record_run`에서는 각 `evidence_updates` 항목 안의 `target`,
`coverage_state`가 계속 필수이고, 각 `evidence_observations` 항목 안의 `target`,
`source_kind`, `assurance_level`, `observed_at`도 계속 필수입니다. 각 `target`은 API
상태 스키마가 담당하는 엄격한 태그형 수락 기준 또는 보충 주장 합집합입니다. 이 규칙은 그 밖의 필드에 암묵적 값을 만들지
않으며, 정확히 광고한 `required` 배열이 기준입니다.

도구 description에는 짧은 목적과 핵심 경계만 둡니다. `volicord.record_run.kind`의
호환성은 MCP에 보이는 다른 인자가 아니라 현재 저장된 Task에 따라 달라지므로, 이 도구의
description은 완전한 모드와 실행 종류 호환 행렬을 포함합니다. `advisor`는 `shaping_update`,
`direct`는 `direct`, `work`는 `shaping_update` 또는 `implementation`을 사용합니다.
자주 쓰는 인자 형태 예시는 `inputSchema.examples` 값으로 광고합니다. 여기에는 intake의
생성·재개·대체·활성 Task 거절, update-scope의 유지·생성·교체, status의 세 detail 수준,
prepare-write, stage-artifact, Product Repository 파일 쓰기가 없는 `advisor`의 `shaping_update`,
증거를 포함한 work `implementation`, request-judgment, reconcile, check-close, close의
완료·취소·대체 분기가 포함됩니다. 광고한 각 예시는 호출에 쓰는 동일한 `inputSchema`와
MCP 인자 DTO를 따릅니다. 예시는 지원하는 인자 분기를 보여 줄 뿐이며, 일치하는 프로젝트
상태나 권한, 전제조건, 성공적인 Core 결과를 주장하지 않습니다.

나열되는 모든 Volicord 도구는 루트 타입이 `object`인 MCP 2025-11-25
`outputSchema`도 노출합니다. 공개 메서드 도구는 공개 메서드 응답 분기에서 이 스키마를
생성합니다. `volicord.request_user_judgment` 출력 스키마는 원래 도구 호출이 끝나기 전에
호스트 elicitation이 대기 판단을 기록했을 때 반환되는 User Channel 응답도 포함합니다.
`volicord.list_projects`는 정확한 어댑터 유틸리티 결과 스키마를 사용합니다.
`structuredContent`를 포함하는 서버 결과는 광고한 스키마를 따라야 합니다.

`tools/list`는 다음과 같이 보수적인 MCP `annotations`를 제공합니다.

| 도구 종류 | `readOnlyHint` | `destructiveHint` | `idempotentHint` | `openWorldHint` |
|---|---:|---:|---:|---:|
| `volicord.status`, `volicord.check_close`, `volicord.list_projects` | `true` | `false` | `true` | `false` |
| `volicord.prepare_write`, `volicord.stage_artifact` | `false` | `false` | `false` | `false` |
| `volicord.intake`, `volicord.update_scope`, `volicord.record_run`, `volicord.request_user_judgment`, `volicord.reconcile_changes`, `volicord.close_task` | `false` | `true` | `false` | `false` |

파괴적이지 않은 변경 도구 행에서 `destructiveHint=false`는 커밋되는 저장 갱신이 기존
권한 상태를 교체하거나 무효화하거나 소비하지 않고 추가된다는 뜻입니다. 호출이 읽기
전용이거나 이후의 별도 MCP 호출을 안전하게 재실행할 수 있다는 뜻은 아닙니다.

`volicord.record_run`은 커밋할 때 호환되는 쓰기 티켓이나 스테이징 입력을 소비하고,
증거와 차단 사유를 갱신하고, `close_basis_revision`을 증가시키고, 현재 판단을
무효화하고, 현재 닫기 근거를 교체하거나 이전 근거를 오래된 상태로 만들 수 있으므로
`destructiveHint=true`를 사용합니다. 커밋된 `volicord.request_user_judgment`도 대기
판단을 만들면서 `Task` 생명주기를 바꿀 수 있습니다. 정확한 효과는 메서드와 저장 효과
담당 문서가 정하며, annotation은 이 도구가 기존 권한 상태를 바꿀 수 있음을 MCP
클라이언트에 보수적으로 알립니다.

모든 변경 도구는 MCP에 보이는 각 호출마다 어댑터가 관리하는 새 요청 식별 정보를
받으므로 `idempotentHint=false`입니다. 하나의 생성된 식별 정보에 대한 Core 재실행 처리는
나중에 보이는 별도 MCP 호출이 같은 결과를 내거나 추가 효과가 없다고 보장하지 않습니다.

이 값은 클라이언트 힌트이며 신뢰할 수 있는 권한 부여 사실이 아닙니다. Agent Connection
권한을 부여하거나, 호스트 신뢰·승인을 우회하거나, 호스트 안전 검토를 생략하거나, 광고한
표면 밖의 멱등 저장 동작을 증명하거나, 선택된 연결과 프로젝트보다 접근 범위를 넓히지
않습니다. 호스트는 자체 신뢰, 승인, 샌드박스 정책을 계속 적용해야 합니다.

프로젝트 선택은 Agent Connection 맥락에서 해석합니다. 사용 가능한 연결 프로젝트가
정확히 하나이면 공개 메서드 도구의 프로젝트 선택을 생략할 수 있습니다. 여러 프로젝트가
연결된 경우에는 `volicord.list_projects`가 반환한 `project_selector` 값이 필요합니다.
그렇지 않으면 어댑터는 수행 가능한 모호성 오류 문구로 호출을 거절합니다. 에이전트는 폴더
이름, 현재 작업 디렉터리, MCP roots, 호스트 라벨, 저장소 라벨, 기억에서 프로젝트 식별
정보를 추론하면 안 됩니다.

`volicord.list_projects`는 선택된 연결 바인딩, 모드, 프로젝트 선택자, 프로젝트 사용
가능성, 저장소 루트 표시 경로, 현재 MCP 세션의 세션 감시 범위 필드를 반환합니다. 범위
필드는 `watcher_status`, `watcher_baseline_created_at`,
`watcher_coverage_start_at`, `watcher_coverage_basis`,
`watcher_partial_coverage_warning`입니다. 명시적 프로젝트 선택이 아직 없는 여러 프로젝트
세션에서는 `watcher_status=pending_project_selection`이고 범위 시각과 근거는 `null`입니다.
경고는 관찰이 아직 시작되지 않았음을 말합니다. 명시적 프로젝트 선택으로 기준선이 만들어진
뒤의 `volicord.list_projects` 출력은 저장된 관찰 시작 시각과 근거를 보고합니다.

MCP 어댑터는 Core에 넘기기 전에 Core 래퍼를 생성합니다. 어댑터는 `request_id`, 워크플로
효과에 대한 `idempotency_key`, Core가 최신 상태를 요구하는 경우 선택된 프로젝트의 현재
상태에서 얻은 `expected_state_version`, `dry_run=false`, 기본 로캘, 선택된 내부
프로젝트, 파생된 호출 맥락을 제공합니다. 공개 MCP 인자는 이 사실들을 덮어쓸 수 없습니다.

`volicord.status`는 Core include 행렬을 노출하지 않고 간결한 공개 `detail` 인자를
사용합니다. 지원 값은 `summary`, `workflow`, `full`이며 `detail`을 생략하면 기본값은
`workflow`입니다.

알려진 도구에서 어댑터는 프로젝트 선택, 세션 감시 설정, 생성 Core 래퍼 작성, Core 메서드
진입보다 먼저 객체 `arguments`를 정확히 광고한 `inputSchema`로 검증합니다. 경계가 정해진
사전 검증기는 지원하는 구조 키워드인 로컬 `$ref`, `type`, `enum`, `required`,
`properties`, `additionalProperties`, 배열 `items`, `allOf`/`anyOf`/`oneOf` 분기를
검사합니다. 서로 독립적으로 평가할 수 있으면 여러 루트·중첩 필드 누락, 알 수 없는 필드,
타입 불일치, enum 불일치, 배열 항목 오류를 한 결과에 함께 담습니다. 지원하지 않는 JSON
Schema 키워드는 타입 디코더나 이후 담당 계층에 맡기며, 사전 검증기가 이를 이유로 유효한
입력을 거절하면 안 됩니다. 타입 디코더에서만 발견되는 나머지 실패는 구조화된 디코드 issue
하나로 변환합니다.

고정된 `MAX_VALIDATION_ISSUES` 값은 `32`입니다. 유효하지 않은 집계 입력을 더 탐색하면
이 오류 항목 허용량을 넘게 되는 시점에 검증 탐색을 중단합니다. 각 조합 분기는 독립된 상한
안에서 평가합니다. 앞선 유효하지 않은 `anyOf` 또는 `oneOf` 분기가 상한에 도달해도 뒤의 분기
평가를 막으면 안 되며, 뒤의 유효한 분기가 조합을 계속 허용할 수 있어야 합니다. 반환하는
`issues` 항목의 경로와 메시지는 각각 UTF-8 기준 `256`바이트와 `512`바이트 이하입니다. 경로를
줄여도 RFC 6901 JSON Pointer 문법을 유지하며, enum과 수신 값 미리보기는 메시지에 넣기 전에
길이를 제한합니다. JSON 이스케이프와 호환 텍스트 래핑을 적용한 뒤 알려진 도구 오류
`CallToolResult` 전체를 공백 없는 JSON으로 직렬화한 크기는 `65536`바이트 이하입니다. 이 최종
바이트 상한을 지키기 위해 원래 보고할 수 있던 `issues` 항목을 생략할 수 있습니다.

입력 검증은 어댑터 전제조건 검사보다 먼저 수행합니다. 유효한 입력을 디코딩한 다음 공개
메서드 도구 호출은 [Agent Connection](agent-connection.md#current-connection-context)이
담당하는 결정적 저장소 루트 프로젝트 선택과 프로젝트별 검증을 수행합니다. 모호하거나
사용할 수 없는 프로젝트 선택은 Core 실행 전에 거절하며, 해당될 때 메시지는 상태를 고칠
`volicord project use` 또는 `volicord connection add` 명령을 이름 붙여야 합니다.

알려진 도구의 입력과 어댑터 전제조건 실패는 `isError: true`인 `CallToolResult`를
반환합니다. `result.structuredContent`는 다음 필드를 가진 객체입니다.

- `code`: `MCP_INVALID_ARGUMENTS` 또는 `MCP_ADAPTER_PRECONDITION_FAILED`
- `tool_name`: 요청한 MCP 도구 이름
- `retryable`: 인자를 고쳐 다시 호출할 수 있으면 `true`, 연결·프로젝트·모드·환경 수리가
  필요한 어댑터 전제조건이면 `false`
- `reached_core: false`
- `committed: false`
- `reported_issue_count`: 반환된 `issues` 배열의 길이와 정확히 같은 값
- `truncated`: 오류 항목, 필드, 전체 결과 상한 때문에 추가 검증 탐색이나 원래 반환할 수 있던
  경로, 메시지, `issues` 세부사항을 줄였으면 정확히 `true`, 그렇지 않으면 `false`
- 비어 있지 않은 `issues`: 각 항목은 RFC 6901 JSON Pointer `path`, 안정적인 `code`, 사람이
  읽는 `message`를 가집니다.

안정적인 issue 코드는 `MCP_ARGUMENT_REQUIRED`, `MCP_ARGUMENT_UNKNOWN`,
`MCP_ARGUMENT_TYPE_MISMATCH`, `MCP_ARGUMENT_ENUM_VALUE`,
`MCP_ARGUMENT_DECODE_FAILED`, `MCP_ADAPTER_PRECONDITION_FAILED`입니다. 루트 포인터는 빈
문자열입니다. `result.content[0].text`는 같은 객체를 JSON으로 직렬화한 값이며 이를 파싱한
결과는 `result.structuredContent`와 같아야 합니다.
어댑터 전제조건 오류와 타입 디코더에서만 발견된 나머지 오류에도 같은
`reported_issue_count`, `truncated`, 필드 크기, 전체 결과 규칙을 적용합니다.

이 실패는 Core 메서드에 진입하거나 Core 메서드 상태를 커밋하거나 프로젝트 aggregate
state version을 올리거나 Core 메서드 이벤트를 만들지 않습니다. 전송 소유 진단 수명주기
관찰은 별도 경계입니다. 잘못된 JSON-RPC 요청 래퍼, 객체가 아닌 `tools/call.arguments`, 알
수 없는 도구 이름은 기존 JSON-RPC 오류 동작을 유지합니다. Core/도메인 거절 응답도 일반
Volicord 응답 형태와 전송 의미 `isError: false`를 유지합니다.

`volicord mcp --stdio`는 MCP 태스크 보강 도구 실행을 광고하거나 구현하지 않습니다. `tools/call`
요청은 `CreateTaskResult`를 반환하지 않으며, `task` 파라미터는 지원되는 기준 기능이
아닙니다.

<a id="user-judgment-elicitation"></a>
### 사용자 판단 입력 요청

`volicord.request_user_judgment`는 Core에 구체적인 대기 `UserJudgment` 생성을 요청하는
유일한 Agent Connection 도구로 남습니다. MCP 어댑터는 `volicord.record_user_judgment`를
Agent Connection 도구로 노출하지 않으며, 에이전트가 넣은 답변 필드를 사용자 입력의
대체물로 받지 않습니다.

`workflow` 연결이 `volicord.request_user_judgment`를 호출하고 Core가 대기 판단을 커밋하면
다음 규칙을 적용합니다.

- 초기화된 클라이언트가 `capabilities.elicitation`을 선언했다면 어댑터는 원래
  `tools/call` 응답을 반환하기 전에 `elicitation/create`를 보낼 수 있습니다. 요청
  스키마는 Core가 만든 선택지 ID에서 가져온 필수 `selected_option_id`와 선택적 `note`를
  담은 평평한 객체입니다. 이 스키마는 비밀값, 자격 증명, 토큰, 개인 키 또는 그 밖의
  비공개 비밀 자료를 요청하지 않습니다.
- `elicitation` 응답이 `action=accept`이면 어댑터는 `content.selected_option_id`를 대기
  판단 선택지와 대조해 검증합니다. 유효한 응답은 Core의 User Channel 메서드를 통해
  `actor_source=local_user`, `operation_category=user_only`,
  `resolved_verification_basis=mcp_elicitation_user_channel`로 기록합니다. 반환되는
  `tools/call` 결과에는 그 결과 Volicord 응답이 `structuredContent`와 JSON 텍스트로 모두
  들어갑니다.
- `elicitation` 응답이 `action=decline`이고 대기 판단에 Core 거절 선택지가 있으면
  어댑터는 같은 User Channel 경로로 그 거절 선택지를 기록합니다. 거절 선택지가 없으면
  판단은 대기 상태로 남습니다.
- `elicitation` 응답이 `action=cancel`이거나, 유효하지 않거나, 형식이 잘못되었거나, 대기
  판단과 맞출 수 없으면 어댑터는 답변을 기록하지 않으며 대기 판단은 대기 상태로 남습니다.
- 클라이언트가 해당 기능을 선언하지 않아 호스트 프롬프트 입력을 사용할 수 없으면 어댑터는
  답변을 기록하지 않고 대기 `RequestUserJudgmentResult`와 추가 텍스트를 반환합니다.
  채팅 명령 캡처 사용 가능 상태가 `configured`, `observed`, `active`이면 그 텍스트에
  프롬프트 제출 훅 경로와 호환되고 현재 검증 코드를 포함한 정확한 채팅 명령이 들어갈 수
  있습니다.
- 채팅 명령 캡처를 사용할 수 없고 로컬 consent URL을 사용할 수 있으면 어댑터는 짧게
  만료되는 일회성 토큰을 만들고 루프백 consent URL과 구조화된 대체 JSON을 반환합니다.
  URL에는 프로젝트 선택자와 토큰만 들어갑니다. Runtime Home 경로, 저장소 경로,
  프롬프트 본문, 답변, 임의 API 매개변수는 포함하지 않습니다.
- 로컬 consent URL 경로가 비활성화되었거나, 안전하게 바인딩할 수 없거나, 토큰을 만들 수
  없으면 대체 안내는 `volicord inbox` CLI 받은편지함 경로를 가리킵니다.

모든 분기에서 `result.structuredContent`는 Volicord 응답 객체이고
`result.content[0].text`는 하위 호환성을 위해 같은 객체를 JSON으로 직렬화한 문자열로
남습니다. 추가 `content[]` 텍스트가 있으면 대체 안내나 `elicitation` 취소·무효 설명 같은
어댑터 안내입니다. 그 추가 텍스트는 `structuredContent`의 일부가 아니며 Core 권한, 공개
API 응답 필드, 사용자 판단 기록도 아닙니다.

<a id="local-web-consent-fallback"></a>
로컬 consent 리스너는 기본적으로 `127.0.0.1`에 바인딩합니다. 안전하게 바인딩할 수
없으면 요청을 허용하지 않고 실패해야 합니다. stdio 모드에서는 임시 루프백 포트를
사용합니다. `volicord serve --transport local-http`에서는 같은 루프백 전용 로컬 HTTP
리스너에서만 consent 경로를 제공합니다.

로컬 consent 엔드포인트 동작:

- `GET /consent?project=<project_id>&token=<token>`은 일회성 토큰을 현재 프로젝트와
  연결에 대해 검증합니다. 만료되었거나, 사용되었거나, 유효하지 않거나, 프로젝트 또는
  연결이 다른 토큰은 안전한 HTML 오류 페이지로 거절합니다. 유효한 요청에는 판단 문구,
  선택지, 검증 정보, 양식이 있는 최소 HTML 페이지를 렌더링합니다. 페이지는 선택지와 그
  의미, 프로젝트 이름 또는 식별자, 사용할 수 있을 때 등록된 저장소 경로, 연결 식별자,
  판단 ID, 토큰 만료 시각, 대체 CLI 명령을 보여 줍니다. 또한 사용자가 사용자 소유 판단을
  기록한다는 점과 에이전트가 사용자를 대신해 이를 기록할 수 없다는 점을 밝힙니다. 이
  판단은 정확성, 테스트 충분성, 배포 성공,
  검토 완료, 보안 강제, 닫기 준비 상태를 증명하지 않는다는 점을 명시합니다.
- `POST /consent`는 토큰, 선택한 Core 선택지 ID, 선택적 메모가 들어 있는
  `application/x-www-form-urlencoded` 양식 제출만 받습니다. `Origin` 헤더가 있으면 동의
  엔드포인트의 Origin 값과 일치해야 합니다. 알 수 없는 선택지 ID를 포함한 검증 실패는
  토큰을 사용 처리하지 않습니다.
- 성공한 제출은 Core를 통해 `actor_source=local_user`, `operation_category=user_only`,
  `resolved_verification_basis=local_user_local_web`으로 답변을 기록하고, 같은 프로젝트 상태
  트랜잭션 또는 동등한 원자적 작업 안에서 토큰을 `consumed`로 표시합니다.
- 만료, 다른 프로젝트·연결·판단에 묶인 토큰, 사용된 토큰의 재사용은 다른 답변을 기록하기
  전에 거절합니다. 성공한 제출 뒤 중복 제출은 이미 사용된 토큰 결과를 결정적으로 반환하며
  기록된 판단을 바꾸지 않습니다. 판단 기록 중 쓰기 실패가 발생하면 대기 판단이 여전히
  현재 상태인 동안 토큰은 만료 전까지 `pending`으로 남습니다.
- 로컬 consent URL은 사람 사용자의 답변을 캡처합니다. Agent Connection이 사용자 소유
  판단에 답하도록 하는 경로로 사용하면 안 됩니다.
- 엔드포인트는 Runtime Home 파일, Product Repository 파일, 정적 자산, MCP 메서드, 임의
  API를 제공하지 않습니다.

Volicord까지 도달한 알려진 공개 Volicord 메서드 도구 호출에서 `tools/call`은 MCP 결과
안에 Volicord 응답 JSON을 래핑합니다.

- Volicord 응답 객체는 `result.structuredContent`로 반환됩니다.
- 같은 객체는 구조화 도구 결과를 소비하지 않는 클라이언트를 위해
  `result.content[0].text`에 JSON으로 직렬화됩니다. 이 텍스트를 파싱한 값은
  `result.structuredContent`와 같아야 합니다.
- 클라이언트는 `structuredContent`를 도구가 광고한 `outputSchema`로 검증할 수 있습니다.
- 성공한 MCP 전송은 Volicord 도메인 수준 거절 응답을 포함해 `isError: false`를
  반환합니다.
- Volicord 도메인 성공 또는 거절은 파싱한 Volicord 응답, 특히 `base.response_kind`와
  `errors`에서 판단합니다.
- 파싱한 공개 메서드 응답은 API 스키마 담당 문서가 정의한 안정적인
  `guarantee_class`와 `non_guarantees` 값을 담은 `base.disclosure`를 포함합니다.
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
