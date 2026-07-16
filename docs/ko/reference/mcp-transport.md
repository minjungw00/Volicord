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
- 대기 중인 사용자 행동을 위한 로컬 consent URL 대체 경로
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
리스너가 아닙니다. 대기 중인 사용자 행동을 위해 별도의 루프백 전용 consent
리스너를 시작할 수 있습니다. 리스너 시작 자체는 이 channel을 선택하거나 token
발급을 허용하지 않습니다. 각 tool call은 아래에서 정한 정확히 협상된 모델 비가시적
host capability를 함께 요구합니다. 그렇지 않으면 대기 행동은 CLI inbox에 남습니다.

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

생성된 개인·로컬 호스트 설정과 사용자가 관리하는 일반 호스트 설정은 내부 연결
바인딩으로 stdio 루프를 시작합니다. 로컬 설정 항목이 안전하게 프로젝트에 묶이면 선택된
내부 프로젝트 바인딩도 함께 담을 수 있습니다.

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
```

생성된 공유 프로젝트 설정은 바인딩 ID나 로컬 Runtime Home의 리터럴 경로를 사용하지
않습니다. 명령과 인자는 다음 중 하나입니다.

```text
volicord mcp --stdio --discover-repository --host codex
volicord mcp --stdio --discover-repository --host claude-code
```

공유 명령은 `PATH`로 해석되는 이름 `volicord`여야 합니다. 같은 항목에는 복제본에서
그대로 쓸 수 있는 Runtime Home 전달 설정이 정확히 하나 있어야 합니다.

- Codex `.codex/config.toml`은 `env_vars = ["VOLICORD_HOME"]`을 사용해 호스트가 시작
  환경의 같은 이름 값을 전달하도록 허용합니다.
- Claude Code `.mcp.json`은
  `"env": {"VOLICORD_HOME": "${VOLICORD_HOME}"}`를 사용해 호스트의 프로젝트 설정
  자리표시자로 그 값을 전달합니다.

어느 형태도 Runtime Home 경로를 내장하지 않습니다. 절대 명령, 추가
Connection/프로젝트 인자, Runtime Home의 리터럴 경로, 관리 시작 마커, 비밀값 형태 환경
키, 그 밖의 모든 환경 항목은 유효하지 않습니다. Codex와 Claude Code 검증은 전달 설정이
없거나 정확한 호스트별 프로젝트 기술 정보와 다른 항목을 설정 불일치로 취급합니다.
개인·사용자 범위 Codex 바인딩은 아래에서 설명하는 로컬 관리 시작 마커 계약을
유지합니다.

`<connection_id>` 프로세스 바인딩 값은 `volicord init` 또는
`volicord connection add`가 만든 저장 `connection_internal_id`에서 옵니다.
선택적 `<project_id>` 프로세스 바인딩 값은
그 연결에 이미 허용된 저장 `project_internal_id`입니다. 일반 사용자가 텍스트 모드
흐름에서 두 값 중 어느 것도 입력할 필요가 없어야 합니다.
저장소 발견 모드는 저장소에서 어느 값도 얻지 않으며, 정규화된 현재 Git 복제본을 식별한
뒤 선택된 로컬 Runtime Home에서만 두 값을 해석합니다.

기준 명령줄 동작:

- `volicord mcp --stdio --connection <connection_id> [--project <project_id>]`는 stdio
  루프를 시작합니다. `--project`가 있으면 제공한 값은 연결 허용 목록 안에 있어야 하며,
  stdio 프로세스는 도구 요청을 처리하기 전에 그 프로젝트로 좁혀집니다.
- `volicord mcp --stdio --discover-repository --host codex|claude-code`는 저장소에서
  보이는 이식 가능한 기술 정보로 같은 stdio 루프를 시작합니다. 이 모드는 정확한 호스트
  선택자가 필요하고 `--connection`, `--project`, `--check`, 추가 인자, 알 수 없는
  호스트를 거부합니다.
- `volicord mcp --check --connection <connection_id>`는 stdin을 읽지 않고 시작 검증을
  실행합니다.
- `volicord mcp --check --connection <connection_id> --project <project_id>`는 같은 시작
  검증을 실행하고 프로젝트 세부 진단을 허용 목록 안의 `project_internal_id` 값 하나로
  제한합니다.
- `-h`와 `--help`는 사용법과 환경 요약을 출력한 뒤 종료 코드 `0`으로 끝납니다.
- `-V`와 `--version`은
  `volicord <package-version> (build_id=<build-id>)`를 출력한 뒤 종료 코드 `0`으로 끝납니다.
- 모드 없음, 바인딩 모드에서 `--connection` 없는 `--check` 또는 `--stdio`, `--stdio`와
  지원되는 `--host` 하나가 없는 저장소 발견, 알 수 없는 옵션, 결합된 명령줄 모드,
  필요한 옵션 값 누락, 추가 위치 인자는 사용법 진단을 stderr에 쓰고 종료 코드 `2`로
  끝납니다.
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
- `--home PATH`는 프로세스의 Runtime Home을 선택합니다. 저장소 발견 모드 밖에서
  `--home`을 생략하면 공통 `VOLICORD_HOME`을 사용한 다음 플랫폼 기본 Runtime Home을
  해석합니다. 저장소 발견 모드는 대신 전달된 값이 비어 있지 않은 절대 경로
  `VOLICORD_HOME`일 것을 요구하며 플랫폼 기본값으로 대체하지 않습니다.
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
  프로젝트, 연결, 대기 중인 사용자 행동에 묶인 유효한 일회성 consent 토큰이 필요한 루프백 User
  Channel 입력 경로입니다.
- 인증 없이 접근할 수 있는 임의 리소스 엔드포인트는 없습니다.
- MCP 엔드포인트의 브라우저 요청은 `Origin` 헤더가 있는지로 식별합니다. `Origin`이 있는
  MCP 엔드포인트 요청은 정확한 `--allow-origin` 값과 일치해야 합니다.
- 최상위 `GET /consent` 탐색에는 `Origin`이 필요하지 않습니다. 요청이 `Origin`을 보내면
  헤더 필드가 정확히 하나여야 하며 값은 유효하게 직렬화된 출처로서 consent 엔드포인트
  자신의 `Origin`과 정확히 일치해야 합니다.
- 모든 `POST /consent`에는 consent 엔드포인트 자신의 `Origin`과 정확히 일치하는
  `Origin` 헤더 필드가 정확히 하나 있어야 합니다. 누락, 빈 값, `null`, 잘못된 형식,
  쉼표로 결합된 값, 반복된 헤더, 다른 값은 양식 본문 디코딩·검증, 토큰 조회·소비,
  해결 기록 효과보다 먼저 HTTP 403 `ORIGIN_NOT_ALLOWED`로 실패합니다.
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

- 관리 Codex 세션 결속 대기 경로가 아니라면, 프로젝트에 결속된 stdio 시작은 한정된
  스냅샷 생성을 사용할 수 있을 때 도구 요청을 처리하기 전에 세션 감시 기준선을 만들거나
  연결할 수 있습니다. 관찰 범위 근거는 `mcp_start`입니다.
- 검증된 생성 Codex 기술 정보나 로컬 관리 마커 집합은 관리 시작 출처만 확립합니다.
  진단 세션, 세션 감시 기준선, 관리 생명주기 행, Core 효과, local-web 자격을 만들지
  않습니다. 아래 호출별 결속이 성공할 때까지 프로세스는 대기 상태입니다.
- 처음으로 결속 자격이 있는 Codex `tools/call`이 오면 프로세스는 세션 감시 기준선을
  시작하고 그 프로세스에서 관찰한 한정된 생명주기 사실을 구체화합니다.
  `managed_host_startup`, `managed_host_initialize_response`, 목록 조회가 있었다면
  `managed_host_tools_list`, 그리고 `managed_host_tool_call`이 해당합니다. 관찰 범위는 이
  결속 경계에서 시작하고 명시적으로 부분 범위이며, 이 시작 사실은 기준선 이전의 Product
  Repository 변경을 관찰했다는 주장이 아닙니다.
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
- 저장소 발견 모드가 아니고 `VOLICORD_HOME`이 없을 때 사용하는 표준 플랫폼 홈 환경
  변수인 `HOME`, `USERPROFILE`, `HOMEDRIVE`와 `HOMEPATH` 조합

`VOLICORD_HOME`은 프로세스의 Runtime Home을 선택합니다. 개인·로컬·사용자 전역 호스트
오버레이는 그것을 만든 관리 설정에서 선택한 절대 Runtime Home을 기록합니다. 저장소에서
보이는 공유 항목은 이 로컬 경로를 내장할 수 없습니다. 대신 호스트별 전달 설정이 시작
호스트 프로세스의 `VOLICORD_HOME`을 자식 프로세스에 전달합니다. 저장소 발견 모드는
전달된 값이 존재하고 비어 있지 않은 절대 경로일 것을 요구하며 플랫폼 기본값으로 대체하지
않습니다. 따라서 시작 호스트 환경은 해당 복제본을 초기화할 때 선택한 것과 같은 절대 로컬
Runtime Home을 제공해야 합니다. `VOLICORD_HOME`은 프로젝트, 연결 의도, 행위자 출처, 작업 범주, 연결
모드, 호스트 신뢰 상태를 선택하지 않습니다. stdio 프로세스와 `--check`는 시작 검증 전에
이 값을 사용하며 help와 version 모드는 사용하지 않습니다.

`VOLICORD_LOCAL_WEB_CONSENT=0`, `false`, `off`, `disabled`는 stdio local web consent
리스너를 끕니다. 다른 값은 리스너 주소나 토큰 정책을 바꾸지 않습니다.

`VOLICORD_MCP_VERIFICATION=1`은 진단 전용 마커입니다. 관리 명령
`volicord connection verify` 흐름은 자식 MCP handshake에 이 값을 자동으로 설정합니다.
운영자가 직접 설정하는 지원 경로는 한정된
[수동 stdio 생명주기 점검](#manual-stdio-lifecycle-probe)뿐입니다. 이 값은 일반 연결과
프로젝트 시작 점검을 유지하지만 프로세스를 검증 점검으로 분류하므로, 프로세스가
시작 세션 감시 기준선이나 관리 Codex 런타임 관찰을 만들지 않습니다. 일반
호스트 설정에 사용하는 값이 아닙니다.

Volicord가 관리하는 개인 또는 사용자 범위 Codex 설정은 다음 로컬 관리 시작 출처
마커를 담을 수 있습니다.

- `VOLICORD_MCP_LAUNCH=managed_host`
- `VOLICORD_MCP_HOST=codex`
- `VOLICORD_MCP_CONNECTION_ID=<connection_id>`
- 명령에 프로젝트 바인딩이 있을 때의 `VOLICORD_MCP_PROJECT_ID=<project_id>`

이 마커들은 로컬 Volicord 관리 설정 식별 정보의 일부이며 일반 운영자 선택자가 아닙니다.
사용자 관리 시작을 관리 시작처럼 보이게 만들려고 직접 추가하거나 바꾸지 말고,
`volicord init` 또는 `volicord connection add`로 관리 설정을 다시 생성합니다. Connection과
선택적 project 값은 대응하는 프로세스 인자와 일치해야 합니다. 일부만 있거나 값이 맞지
않는 마커 집합은 유효하지 않은 관리 출처이며 관리 생명주기 관찰을 만들지 않습니다.
저장소 발견 시작은 정확한 형식의 기술 정보와 호스트 선택자를 관리 시작 출처로 사용하며
이 마커를 담으면 안 됩니다. 어느 형태도 Codex native 세션 identity를 제공하지 않습니다.
마커와 기술 정보는 프로젝트 접근, 호스트 신뢰, 세션 결속, 더 넓은 권한을 부여하지
않습니다.

로컬 연결 식별 정보는 개인·로컬 생성 설정이나 사용자가 관리하는 일반 호스트 설정 안의
`--connection <connection_id>`로 제공합니다. 이것은 저장된 `connection_internal_id`를
이름 붙이며 사용자가 보통 직접 고르는 값이 아닙니다. 공유 저장소 설정은 대신
`--discover-repository --host <host>`만 제공합니다. 시작 시 정규화된 현재 Git 루트를
로컬 Runtime Home 등록을 통해 연결 하나와 프로젝트 하나로 해석합니다. 두 형태 모두
해석된 Agent Connection과 Runtime Home 레지스트리 상태가 연결 모드, 연결 프로젝트,
어댑터가 파생하는 `actor_source`와 `operation_category`를 제공합니다. 그 밖의 Volicord
전용 환경 변수는 지원되는 운영자 설정이 아닙니다.

현재 MCP Runtime Home 경로 해석:

1. `VOLICORD_HOME`이 존재하지만 비어 있으면 오류입니다.
2. 저장소 발견 모드에서 `VOLICORD_HOME`이 없거나 상대 경로이면 오류입니다. 위의 빈 값
   규칙과 함께, 플랫폼 기본 Runtime Home 대체, 레지스트리 접근, 저장소 발견보다 먼저
   존재하고 비어 있지 않은 절대 경로를 요구합니다.
3. 절대 경로 `VOLICORD_HOME`은 제공된 그대로 사용합니다.
4. 저장소 발견 모드 밖에서 상대 경로 `VOLICORD_HOME`은 그 경로가 존재하지 않아도
   프로세스의 현재 작업 디렉터리를 기준으로 해석합니다.
5. 저장소 발견 모드 밖에서 `VOLICORD_HOME`이 없으면 플랫폼 홈 환경 변수에서 기본 사용자
   홈을 구하고 `.volicord`를 붙입니다. Windows가 아닌 플랫폼에서는 `HOME`,
   `USERPROFILE`, `HOMEDRIVE`와 `HOMEPATH` 조합 순서로 시도합니다. 네이티브
   Windows에서는 `USERPROFILE`, `HOMEDRIVE`와 `HOMEPATH` 조합, WSL 형식 mount 경로가
   아닌 `HOME` 순서로 시도합니다.
6. 시작 검증 전에 정규화를 요구하지 않습니다.

## 시작 검증

`volicord mcp --stdio`는 stdio 루프에 들어가기 전에 명시적 로컬 Agent Connection
바인딩 또는 저장소 발견 바인딩과 그 바인딩이 의존하는 로컬 레지스트리 기록을
검증합니다.

시작 검증에는 아래 조건이 필요합니다.

- Runtime Home 레지스트리가 존재하고 유효합니다.
- 명시적 바인딩 모드에서는 설정된 `connection_id` 프로세스 인자가 저장된 기존
  `connection_internal_id`를 가리킵니다.
- 연결이 활성화되어 있습니다.
- 연결 모드가 지원됩니다.
- 연결 프로젝트 행이 하나 이상 읽을 수 있습니다.
- 진단에 필요한 MCP 명령 정보를 설치 프로필에서 해석할 수 있습니다.
- 시작에 필요한 레지스트리 JSON과 메타데이터가 유효합니다.

저장소 발견 모드는 위의 공통 검증 전에 다음 단계를 닫힌 방식으로 수행합니다.

1. 명시적으로 전달된, 비어 있지 않은 절대 경로 `VOLICORD_HOME`을 요구합니다. 값이
   없거나 비어 있거나 상대 경로이면 플랫폼 기본값 대체나 레지스트리 접근 전에 시작이
   실패합니다.
2. 프로세스 현재 디렉터리를 정규화하고 상위 경로를 따라가 가장 가까운 유효 Git
   worktree 루트를 찾습니다. 지원되는 gitdir 파일과 연결된 worktree 배치도 포함합니다.
3. 정확히 그 정규화된 루트가 선택된 로컬 Runtime Home에 등록된 프로젝트인지
   요구합니다.
4. `--host`와 호스트가 일치하고, 의도가 `shared`이며, 호스트 범위가 프로젝트이고,
   Connection Projects에 해당 프로젝트를 포함하는 활성 연결만 선택합니다.
5. 일치 항목이 정확히 하나인지 요구하고 프로세스 허용 목록을 해당 프로젝트로
   좁힙니다.

일치 항목 없음은 `REPOSITORY_DISCOVERY_CONNECTION_NOT_FOUND`, 여러 항목은
`REPOSITORY_DISCOVERY_CONNECTION_AMBIGUOUS`, 등록되지 않은 복제본은
`REPOSITORY_DISCOVERY_PROJECT_NOT_REGISTERED`로 실패합니다. 진단은 저장소와 Runtime
Home을 이름 붙이고 해당 `volicord init --shared`, `connection verify`, 또는
`connection list`와 중복 제거 동작을 안내합니다. 어댑터는 모호한 행 하나를 임의로
고르거나 저장소 파일에서 Connection ID나 프로젝트 ID를 읽지 않습니다.

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

<a id="managed-host-session-input"></a>
## 관리 호스트 세션 입력

관리 Codex 시작 출처와 세션 결속은 서로 다른 상태입니다. 검토된 경로는 성공한
`initialize`에서 정확한 `clientInfo.name=codex-mcp-client`와
`clientInfo.version=0.144.4`를 보존하지만 초기화만으로 관리 세션을 결속하지 않습니다.
설치 호스트의 정규 좌표는 정확한 probe 외피 `codex-cli 0.144.4`에서 해석한
`0.144.4`입니다.
이 정확한 버전은 검증과 릴리스 Evidence 좌표로 보존할 뿐 런타임 기능 gate로 사용하지
않습니다. 다른 유효한 버전은 관찰한 그대로 보존합니다. 기능 가용성은 이 검토 좌표와의
동등성이 아니라 [Agent Connection](agent-connection.md#host-feature-support)이 담당하는 현재
capability probe 결과에서 나옵니다.

관리 결속이 `session_watch_baselines` 행을 구체화할 때 기준선 `metadata_json`은 기존의
크기가 제한된 감시 메타데이터와 함께 성공한 initialize의 정확한 정체성만 최상위
`client_name`과 `client_version`으로 보존합니다. 이 문자열은 실제 성공한 initialize
값이며, 어댑터는 호스트 종류, 실행 파일이나 probe 텍스트, 설정, 프로토콜 버전, 상수,
요청 메타데이터, 다른 세션에서 이를 추론하지 않습니다. 실제 호스트 릴리스 기록기는 이 두
필드를 읽을 수 있습니다. 이를 위해 원본 initialize 요청, 그 밖의 파라미터, 원본
프로토콜·세션·thread·turn·tool-call payload를 보존하지 않습니다.
관리 기준선 하나는 클라이언트 쌍 하나만 보존합니다. 같은 initialize 쌍을 다시 관찰하는
것만 멱등입니다. 기존 관리 기준선의 클라이언트 쌍이 없거나 일부만 있거나 다르면 결속
충돌이며 성공한 관리 클라이언트 provenance로 사용하거나 교체하여 복구하면 안 됩니다.

Ready 전환 뒤 처음으로 알려진 도구를 구조적으로 올바르게 호출할 때는
`_meta.threadId`와 객체 `_meta["x-codex-turn-metadata"]` 안의 문자열
`session_id`, `thread_id`, `turn_id`가 있어야 합니다. 바깥 `threadId`는 안쪽
`thread_id`와 정확히 같아야 합니다. 각 native 값은 UTF-8 1바이트 이상 256바이트
이하이고 `[A-Za-z0-9._:-]+`와 일치해야 합니다. JSON-RPC 형태, 알려진 도구 이름,
`arguments` 검증이 모두 성공한 뒤에만 어댑터가 `session_id`에서
`managed_host_session_id`를 파생하고 `thread_id`에서 domain 분리된 메모리 내 thread
결속 다이제스트를 파생합니다. 두 결속은 stdio 프로세스 수명 동안 바뀌지 않습니다.
이후 호출은 둘 다 일치해야 하며 새 turn에서는 `turn_id`가 바뀔 수 있습니다. 원본
세션·thread·turn 값은 검증과 해시 뒤 폐기합니다.

메타데이터가 없거나 형식이 잘못되었거나 일치하지 않으면 진단 세션, 세션 감시 행, 관리
생명주기 이벤트, 도구 호출 행, Core 호출, token, local-web 전달을 만들기 전에
JSON-RPC `-32602`를 반환합니다. 이 숨은 메타데이터는 transport 입력이며 공개 도구
인자, 권한, host attestation이 아닙니다. 환경 변수, 프로세스 ID, 프로세스 조상 관계,
도착 시각, 훅 이벤트와의 시간적 근접성, 최신 세션 조회로 대체하면 안 됩니다. Claude
Code는 담당 문서가 정한 어댑터 입력을 사용합니다. 두 호스트 모두 검증된 native 세션을
[호스트 릴리스 증거](host-release-evidence.md)가 정의한 불투명한
`managed_host_session_id`로 매핑합니다. 원본 세션, thread, turn, 이벤트, 호출,
capture, invocation 식별자는 영속 저장, 로그 기록, 진단, 렌더링, 증거 첨부를 하지
않습니다. 결속이 없거나 잘못됐거나 일치하지 않으면 Strong Evidence 자격이 없으며
대체값을 합성해 고치면 안 됩니다.

`diagnostics.sqlite`는 최선형 운용 저장소이며 결속 권한이 아닙니다. 손상, 쓰기 거부,
기존 진단 좌표와의 충돌은 그 밖에는 유효한 관리 메타데이터를 거부하거나 MCP 결과,
guard 결과, Core 결과, 권한 효력이 있는 결속을 바꿀 수 없습니다. 가능하면 해당 진단
영속화는 건너뛰거나 치명적이지 않은 진단 실패로 기록합니다. 정확한 소유권 충돌은 계속
프로젝트 Agent Session과 등록 연결 상태에서 판단합니다. 이 비권한 진단 경계가 위의
잘못되거나 일치하지 않는 요청 메타데이터에 대한 효과 없는 거부를 약하게 만들지는
않습니다.

## Agent Connection에 묶인 프로세스

`volicord mcp --stdio` 프로세스 하나는 아래 값에 묶입니다.

- 로컬 `connection_id` 프로세스 바인딩 하나 또는 고유한 저장소 발견 결과로 선택된
  저장된 Agent Connection 하나
- 저장소 발견 모드에서는 정규화된 현재 Git worktree에서 선택한 등록 프로젝트 정확히
  하나

Agent Connection이 제공하는 값:

- `workflow` 또는 `read_only` 연결 모드 하나
- `personal`, `shared`, `global` 중 하나의 연결 의도
- 명시적 연결 프로젝트 허용 목록
- 레지스트리를 통한 호스트 설정 인벤토리와 마지막 검증 상태

해석된 프로세스 바인딩은 프로세스 수명 동안 고정됩니다. Agent Connection 식별 정보를 바꾸려면
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
- 문자열 `name`과 `version` 필드를 포함하는 객체 `clientInfo`. 각 필드는 1바이트 이상 256
  UTF-8 바이트 이하이고 공백이 아닌 문자를 하나 이상 포함하며 제어 문자가 없어야 하고,
  그 밖에는 정확한 문자열을 그대로 보존합니다.

`params.capabilities.elicitation`이 객체이면 어댑터는 MCP 클라이언트가 서버 시작
사용자 입력 요청을 사용할 수 있다고 봅니다. 별도 Volicord 확장 capability는 다음과
같습니다.

```json
{
  "capabilities": {
    "experimental": {
      "io.volicord/user-channel": {
        "model_invisible_user_surface": true
      }
    }
  }
}
```

정확한 boolean `true`만 모델 비가시적 local-web handoff에 대한 클라이언트의 협력적
선언을 제공합니다. 이 값은 필요하지만 자격의 충분조건은 아닙니다. 구성원이 없거나
`false`, 잘못된 타입, 잘못된 namespace, 잘못된 중첩 객체이면 initialize 오류가 아니라
capability unavailable입니다. 이 flag는 사용자 권한이나 host 신뢰의 증거가 아닙니다.
Namespaced tool-result `_meta` 값을 사용자 소유 표면에 전달하고 모델 맥락에는 절대
제공하지 않는다는 클라이언트의 약속입니다. 어댑터는 정확한 `clientInfo.name`과
`clientInfo.version`을 검증 입력으로 보존하지만, 클라이언트 text는 신원 증명이 아닙니다.
다른 capability 항목은 그 자체로 Volicord 동작을 만들지 않습니다.

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
주장하지 않은 채 표시합니다. 이 빌드 객체에는 최종 실행 파일 다이제스트나 릴리스 증거
다이제스트가 없으며 릴리스 증거 manifest가 아닙니다. 호스트 역량 평가는 실행 파일 밖의
별도로 검증된 정확한 최종 아티팩트 릴리스 증거 manifest 또는 receipt에서 예상
`evidence_artifact_sha256`을 얻어야 합니다.
Initialize 결과는 클라이언트가 이 선택적 handoff를 협상할 수 있도록
`capabilities.experimental["io.volicord/user-channel"]` 확장도 광고합니다. 서버 광고만으로
클라이언트 capability가 available이 되지는 않습니다.

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
`volicord.request_user_action`을 처리하는 동안 중첩된 `elicitation/create` 요청 하나를
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
| 쓰기 가능한 프로젝트 상태의 `workflow` | `volicord.intake`, `volicord.update_scope`, `volicord.status`, `volicord.get_operation_result`, `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact`, `volicord.record_run`, `volicord.request_user_action`, `volicord.reconcile_changes`, `volicord.check_close`, `volicord.close_task`, `volicord.list_projects` |
| 읽을 수 있지만 쓸 수 없는 프로젝트 상태의 `workflow` | `volicord.status`, `volicord.get_operation_result`, `volicord.request_user_action`(resume만), `volicord.check_close`, `volicord.list_projects` |
| 읽을 수 있는 프로젝트 상태의 `read_only` | `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects` |
| 읽을 수 있는 허용 프로젝트 상태가 없음 | `volicord.list_projects` |

MCP 어댑터는 시작과 탐색 중 프로젝트 상태를 읽기 전용으로 살펴볼 수 있습니다. 현재 MCP
호스트 환경에서 프로젝트 상태를 읽을 수는 있지만 쓸 수 없다면, 저장된 Agent Connection
모드가 `workflow`여도 읽기 호환 메서드 도구만 계속 보이고 워크플로 변경 분기는 숨깁니다.
혼합 `volicord.request_user_action` 도구는 명시적 resume 분기가 기존 요청을 읽을 수 있도록
계속 보이지만 create 분기는 `MCP_UNAVAILABLE`을 반환합니다.
허용 프로젝트 상태를 하나도 읽을 수 없으면 호출자가 프로젝트 사용 가능성을 확인할 수
있도록 `volicord.list_projects`만 보이게 유지합니다.

해석된 연결 모드와 유효 저장 capability 조합 하나에서는 이 도구 집합이 정적입니다. Task
상태, 현재 차단 사유, 쓰기 티켓 상태, 모델의 이전 호출에 따라 도구를 추가하거나 제거하지
않으며 그런 조건은 도구 결과로 보고합니다. JSON-RPC envelope와 요청 ID를 더하기 전 정확히
`{"tools":[...]}` 형태인 compact `tools/list` 결과 객체 전체는 지원되는 모든 모드와 저장
capability 조합에서 직렬화된 UTF-8 기준 35,000바이트 이하여야 합니다.

유효 저장소가 읽기 전용이면 읽기 호환 공개 메서드 도구는 세션 감시 기준선,
`tool_invocations`, `task_events`, 새 `project_state.state_version`을 만들지 않고 실행됩니다.
호스트 쪽 오래된 도구 캐시가 선택된 프로젝트 상태를 쓸 수 없을 때도 공개 워크플로 변경
도구를 호출하면, 어댑터는 Core 쓰기를 시도하지 않고 일반 Volicord 거절 응답을 반환합니다.
이 응답은 `code=MCP_UNAVAILABLE`, `operation_category=agent_workflow`, 메시지
`Volicord project state is not writable in the current MCP host environment.`를 담습니다.

`workflow` 모드의 증거 경로는 다음과 같습니다.

- 등록 command, tool, guard, watcher source가 observation을 제공할 때
  `volicord.prepare_evidence_capture`로 정확한 current-basis intent를 만듭니다.
  Receipt fulfillment는 MCP 도구가 아닙니다.
- 바이트나 안전한 알림이 필요할 때만 `volicord.stage_artifact`로 증거 첨부 입력을
  준비합니다.
- `volicord.record_run`으로 Run 또는 관찰, 대상별 증거 갱신, 관찰 출처, 필요한 첨부
  연결 또는 승격을 기록합니다.

스테이징 핸들만으로는 받아들여진 증거가 아니며 닫기 상태를 만족하지 않습니다.

MCP에 보이는 도구는 공개 Volicord Core API 메서드 목록과 같은 것이 아닙니다.
`volicord.check_close`는 닫기 준비 상태를 확인하는 일급 읽기 전용 Core 메서드에
매핑됩니다. `volicord.close_task`는 워크플로 전용 Core 변경 메서드에 매핑되며
`read_only` 연결에는 나열되지 않습니다.
`volicord.get_operation_result`는 정확한 과거 변경 응답 하나를 크기가 제한된 page로
조회하는 읽기 전용 Core 메서드에 매핑되며 프로젝트 상태를 읽을 수 있을 때 두 연결 모드에
모두 나열됩니다.
`volicord.resolve_user_action`은 모든 User Channel 해결을 위한 공개 Core API
메서드이지만 Agent Connection MCP 도구로 노출되지 않습니다. 공개 메서드 담당 표는 [API
메서드](api/methods.md)를 봅니다.

구조적으로 유효한 `tools/call` 요청은 객체 `params` 안에 아래 값을 둡니다.

- 문자열 `name`
- 선택적 객체 `arguments`

관리 Codex 경로는 [관리 호스트 세션 입력](#managed-host-session-input)에 정의된 숨은 요청측
`_meta` 결속도 사용합니다. 이 값은 `tools/list`에 노출되지 않고 공개 도구 입력
스키마의 일부가 아니며 Core 요청 인자로 복사되지 않습니다.

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
- `volicord.get_operation_result`: `cursor=null`
- `volicord.prepare_write`: `task_id=null`, `change_unit_id=null`,
  `sensitive_categories=[]`
- `volicord.prepare_evidence_capture`: command branch의 `expected_exit_code=0`, tool
  branch의 `expected_success=true`, registered-connection branch의
  `expected_complete=true`. 명시적 null도 같은 뜻입니다.
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
- `volicord.request_user_action`: `request.operation=create` 아래에서 판단 variant는
  `request.change_unit_id=null`, `request.action.sensitive_action_scope=null`,
  `request.action.options=null`, `request.action.affected_refs=[]`,
  `request.expires_at=null`입니다. 관찰 variant에는 호출자 expiry 기본값이 없고,
  `request.operation=resume`에는 create 기본값이 없습니다.

MCP에 보이는 모든 변경 도구는 `detail=summary|workflow|full`도 받습니다. `detail`을
생략하면 `summary`가 기본값입니다. 이는 어댑터 응답 상태 보기 선택이며 Core 요청 필드가
아니고 메서드 담당 요청 멤버를 생략할 권한도 아닙니다.

이 기본값은 MCP 표시 인자 DTO에만 속합니다. 디코딩 뒤 어댑터는 모든 멤버를 갖춘 Core
요청 형태를 구성합니다. 따라서 각 메서드 참조가 담당하는 공개 Core API의 멤버 존재
계약은 바뀌지 않습니다. `volicord.request_user_action`에는 중첩
`request.operation` 판별자가 필수입니다. create variant는 `request.task_id`와 완전한
닫힌 `request.action`을 요구하고, resume variant는
`request.user_action_request_id`만 요구하며 create 필드를 거부합니다.
`volicord.record_run`에서는 각 `evidence_updates` 항목 안의 `target`,
`coverage_state`가 계속 필수이고, 각 `evidence_observations` 항목 안의 `target`,
`source_kind`, `assurance_level`, `observed_at`도 계속 필수입니다. 각 `target`은 API
상태 스키마가 담당하는 엄격한 태그형 수락 기준 또는 보충 주장 합집합입니다. 이 규칙은 그 밖의 필드에 암묵적 값을 만들지
않으며, 정확히 광고한 `required` 배열이 기준입니다.

스키마 생성에는 두 가지 명시적 detail mode가 있습니다.
`ToolSchemaDetail::RuntimeCompact`는 MCP `tools/list`에 사용하며 모든
`inputSchema.examples` 구성원을 생략하고 각 description을 도구의 결과, 권한 또는 쓰기
경계, 호출해야 하는 시점으로 제한합니다. 모드 행렬, 긴 절차, 복구 목록, 예시를 넣지
않습니다. `ToolSchemaDetail::Documentation`은 생성 문서와 스키마 점검에서 쓰는 canonical
예시와 완전한 분기 설명을 유지합니다. 두 mode는 같은 허용 인자 형태, `required` 필드,
닫힌 필드 규칙, output schema, annotation을 사용하며 compact mode는 표시 크기만 바꿉니다.
`advisor`/`shaping_update`, `direct`/`direct`, `work`와 `shaping_update` 또는
`implementation` 같은 mode-to-kind 호환성은 런타임 description이 아니라 Documentation
detail과 메서드 담당 문서에 둡니다.

나열되는 모든 Volicord 도구는 루트 타입이 `object`인 MCP 2025-11-25
`outputSchema`도 노출합니다. 읽기 전용 공개 메서드 도구는 공개 메서드 응답 분기에서 이
스키마를 생성합니다. 변경 도구는 새 `AuthorityReceipt`와 다음 단계에 필요한 메서드 결과를
결합한 summary·workflow 래퍼, 같은 새 receipt와 정확한 공개 메서드 응답을 결합한 full
래퍼, 크기가 제한된 효과 적용 후 복구 분기를 광고합니다.
`volicord.request_user_action`은 복합 `agent_workflow_result`, replay 표시, snapshot에
결속된 현재 상태, nullable `user_channel_resolution` 형태를 사용합니다. user-only 해결이
정확한 요청 결과를 대체하지 않습니다. `volicord.list_projects`는 정확한 어댑터
유틸리티 결과 스키마를 사용합니다. `structuredContent`를 포함하는 서버 결과는 광고한
스키마를 따라야 합니다.

`tools/list`는 다음과 같이 보수적인 MCP `annotations`를 제공합니다.

| 도구 종류 | `readOnlyHint` | `destructiveHint` | `idempotentHint` | `openWorldHint` |
|---|---:|---:|---:|---:|
| `volicord.status`, `volicord.get_operation_result`, `volicord.check_close`, `volicord.list_projects` | `true` | `false` | `true` | `false` |
| `volicord.prepare_write`, `volicord.prepare_evidence_capture`, `volicord.stage_artifact` | `false` | `false` | `false` | `false` |
| `volicord.intake`, `volicord.update_scope`, `volicord.record_run`, `volicord.request_user_action`, `volicord.reconcile_changes`, `volicord.close_task` | `false` | `true` | `false` | `false` |

파괴적이지 않은 변경 도구 행에서 `destructiveHint=false`는 커밋되는 저장 갱신이 기존
권한 상태를 교체하거나 무효화하거나 소비하지 않고 추가된다는 뜻입니다. 호출이 읽기
전용이거나 이후의 별도 MCP 호출을 안전하게 재실행할 수 있다는 뜻은 아닙니다.

`volicord.record_run`은 커밋할 때 호환되는 쓰기 티켓이나 스테이징 입력을 소비하고,
증거와 차단 사유를 갱신하고, `close_basis_revision`을 증가시키고, 현재 판단을
무효화하고, 현재 닫기 근거를 교체하거나 이전 근거를 오래된 상태로 만들 수 있으므로
`destructiveHint=true`를 사용합니다. 커밋된 `volicord.request_user_action`도 대기
사용자 행동을 만들면서 `Task` 생명주기를 바꿀 수 있습니다. 정확한 효과는 메서드와 저장 효과
담당 문서가 정하며, annotation은 이 도구가 기존 권한 상태를 바꿀 수 있음을 MCP
클라이언트에 보수적으로 알립니다.

하나의 annotation이 `volicord.request_user_action`의 두 operation을 모두 다루므로
보수적입니다. `request.operation=create`는 문서화된 파괴적 mutation 효과를 가질 수
있습니다. `request.operation=resume`은 읽기 전용 정확한 과거 replay와 현재 projection일
뿐 효과를 만들지 않지만, tool 수준 annotation은 계속 `readOnlyHint=false`,
`destructiveHint=true`, `idempotentHint=false`입니다.

변경할 수 있는 도구는 서로 다른 create 또는 mutation 호출마다 보통 어댑터가 관리하는 새
요청 식별 정보를 받으므로 `idempotentHint=false`입니다. 하나의 생성된 식별 정보에 대한
Core 재실행 처리는 나중에 보이는 별도 mutation 호출이 같은 결과를 내거나 추가 효과가
없다고 보장하지 않습니다. 명시적 `request_user_action` resume 분기는 동작상 예외입니다.
이미 커밋된 요청을 이름 붙이며 대체 idempotency key나 mutation을 만들지 않습니다.

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

Mutation과 create 분기에서 MCP 어댑터는 Core에 넘기기 전에 Core 래퍼를 생성합니다. 어댑터는 `request_id`, 워크플로
효과에 대한 `idempotency_key`, Core가 최신 상태를 요구하는 경우 선택된 프로젝트의 현재
상태에서 얻은 `expected_state_version`, `dry_run=false`, 기본 로캘, 선택된 내부
프로젝트, 파생된 호출 맥락을 제공합니다. 공개 MCP 인자는 이 사실들을 덮어쓸 수 없습니다.
Request-user-action resume 분기는 대신 읽기 전용 접근 맥락을 파생하고 저장된 원천을
조회하며 mutation envelope, idempotency key, expected state version을 만들지 않습니다.

`volicord.status`는 Core include 행렬을 노출하지 않고 간결한 공개 `detail` 인자를
사용합니다. 지원 값은 `summary`, `workflow`, `full`이며 `detail`을 생략하면 기본값은
`workflow`입니다.

변경 도구의 `detail`은 같은 세 값을 쓰지만 기본값과 효과가 다릅니다. 기본값인
`summary`는 `authority_receipt`와 간결한 `method_result`를 반환합니다. `workflow`는 두
필드에 현재 `next_actions`를 추가합니다. `full`은 `authority_receipt`와 정확한 공개 응답을
`method_result` 아래에 반환합니다. Core/도메인 거절 분기는 모든 detail 값에서 기존 응답
객체를 유지합니다. 어댑터는 Core 진입 전에 이 인자를 검증합니다.

어댑터가 내보내는 모든 작업 흐름 `next_actions` 항목은 null이 아닌 공개
`owner_method`를 가집니다. 끝나지 않은 작업 흐름 상태에 Agent 행동이 필요하면 주 행동은
그 행동을 수행할 공개 `volicord.*` 메서드와 호출에 필요한 크기가 제한된 인자 또는 영속
ref를 제시합니다. Agent가 그 메서드를 찾기 위해 `volicord.status`를 한 번 더 호출하게 하면
안 됩니다. 로컬 정책 복구, 호스트 reload, 연결 설정, 저장소 upgrade 같은 관리 작업은
해당할 때 정확한 CLI 명령을 가진 별도 typed 진단 행동으로 남으며 owner 없는 작업 흐름
next action이 아닙니다.

간결한 `method_result`는 항상 효과 종류, 결과 state version, 커밋된 event ref를
보존합니다. 또한 `volicord.prepare_write`에서는 발급된 쓰기 티켓과 결정을,
`volicord.prepare_evidence_capture`에서는 정확한 capture-intent ref, intent, expiry를,
`volicord.stage_artifact`에서는 스테이징된 handle과 만료 시각을,
`volicord.record_run`에서는 정확한 Run ref, 등록된 `ArtifactRef` 값, 새로 기록된 증거
관찰 ref, null일 수 있는 `close_basis_anchor`를, `volicord.reconcile_changes`에서는 새로
생성한 요청 ref나 form이 없는 찾은 항목별 결과를, `volicord.request_user_action`에서는
정확한 요청 결과, replay 표시,
현재 projection state version/시각, 별도 안전 해결 사실, 해결에서 파생된 ref를
보존합니다. `close_basis_anchor`는 `close_basis_revision`, `scope_revision`,
`source_run_ref`, null일 수 있는 `evidence_summary_ref`를 담습니다. 이는 Task에 저장된
닫기 근거를 가리키는 타입이 지정된 좌표이며 `StateRecordRef`나 별도 닫기 근거 기록이
아닙니다. 해결된 간결한 사용자 행동 결과는 요청 ref 없이 정확한 닫힌 세 필드 요청
요약, 정확한 과거 해결 ref, snapshot에
결속된 상태, 선택한 option ID와 label 또는 증거 관찰 요약, 해당되는 resolution outcome,
공개 해결 파생 ref를 포함하고 자유 형식 사용자 note와 evidence observation summary
텍스트는 제외합니다. `detail=full`은 이러한
다음 단계 필수 handle, ticket, Run·증거 ref, 찾은 항목별 결과, 호스트 고유 선택이 아닌
추가 필드가 필요한 호출자를 위한 값입니다.

Capture-intent compact result는 다른 mutation result와 같은 bounded
summary/workflow 및 post-effect recovery 순서를 사용합니다. Authority receipt는 맞지
않지만 compact result가 맞으면 recovery는 정확한 intent ref, 등록 source에 필요한 전체
intent, expiry를 보존합니다. 이 MCP 호출은 receipt나 producer를 만들지 않으므로 해당
값을 projection하지 않습니다.

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

<a id="mutation-authority-receipt-projection"></a>
### 변경 권한 receipt 상태 보기

변경이 `base.response_kind=result`를 반환하면 어댑터는 도구 결과를 반환하기 전에 같은
선택 프로젝트와 해석된 Task를 대상으로 읽기 전용 `volicord.status`를 새로 실행합니다.
상태 분기가 dry-run이 아닌 읽기 전용 결과이고 `AuthorityReceipt`가 새로 읽은
`base.state_version`, 프로젝트, Task, Task 참조 버전, 현재 상태 보기와 일치할 때만 그
갱신을 받아들입니다. 변경 자체의 Core 효과는 해당 메서드 담당 문서가 정의하며, 이 갱신은
두 번째 변경을 만들지 않습니다.

적용되었거나 replay된 변경 결과 하나에 대해 어댑터는 정확한 메서드 결과, 간결한 메서드
결과, 효과 사실, null일 수 있는 `operation_result_ref`, 새 receipt, 현재 다음 행동을
한 번만 파생하여 하나의 기준 변경 결과로
구성합니다. 정상 `detail` 상태 보기, 응답 바이트 상한 복구, 효과 적용 후 복구, 권한 상태
새로 고침 복구는 모두 이 같은 결과에서 선택합니다. 복구 분기가 서로 다른 간결한 결과를
다시 계산하거나 분기별 보존 순서를 사용하면 안 됩니다.
조회 대상인 커밋 또는 replay agent-workflow Core 변경에서는 ref가 존재하고 모든 정상·복구
상태 보기에 같은 값으로 들어갑니다. Ref는 receipt/result 보존 순서와 독립적이며 다른 후보를
넣기 위해 제거할 수 없습니다. Core 밖 staging과 조회 가능한 영속 행이 없는 결과는
`operation_result_ref=null`을 사용합니다.

받아들인 갱신은 다음과 같이 반환합니다.

- `detail=summary`는 `operation_result_ref`, `authority_receipt`, 간결한 `method_result`를
  `result.structuredContent`에 반환합니다.
- `detail=workflow`는 두 필드에 `next_actions`를 추가해 반환합니다.
- `detail=full`은 `operation_result_ref`, `authority_receipt`, 정확한 공개 메서드 응답을 `method_result` 아래에
  반환합니다. 메서드 응답을 만든 뒤 갱신하기 전에 상태가 바뀌었다면
  `authority_receipt`가 새 권한 보기이고, `method_result` 안의 메서드 소유 상태는 해당
  메서드 호출 결과로 남습니다.
- `result.content[0].text`는 UTF-8 기준 최대 512바이트인 짧은 호환 요약입니다.
  `structuredContent`를 다시 JSON으로 직렬화한 복사본이 아닙니다.
- 간결한 `summary` 또는 `workflow` `CallToolResult`는 최대 65,536바이트입니다. 새로 읽은
  상태 보기가 Core 소유 receipt를 바꾸지 않고 이 상한에 들어갈 수 없으면 어댑터는 권한
  데이터를 잘라 내지 않고 해당 상태 보기를 생략합니다.
- `full` `CallToolResult`는 더 크지만 최대 262,144바이트로 제한됩니다. 상한을 넘으면
  크기 제한이 없거나 잘린 메서드 응답을 반환하지 않고 같은 생략 분기를 사용합니다.

새로 읽은 상태 보기가 바이트 상한을 넘으면 크기가 제한된 효과 적용 후 복구 분기를
`isError=false`로 반환합니다. MCP host가 이미 적용된 동작을 실패한 mutation으로 분류해
자동으로 다시 호출하지 않도록 하기 위함입니다. `structuredContent`에는
`code=MCP_RESPONSE_BUDGET_EXCEEDED`, 메서드 `tool_name`, `requested_detail`,
`retryable=false`, `reached_core`, `committed`, null일 수 있는 `effect_kind`,
`effect_applied`, null일 수 있는 안정적인 `effect_anchor`, null일 수 있는
`operation_result_ref`, null일 수 있는 `authority_receipt`, null일 수 있는 요청 도구의
간결한 `method_result`,
`authoritative_refresh_succeeded=true`, `response_projection_omitted=true`,
`status_read_required=true`, `completion_claim_withheld=true`를 담습니다. `committed`는 새
Core 커밋을 보고하고, `effect_kind`와 `effect_applied`는 생성된 staging handle이나 replay된
적용 효과 같은 Core 밖 효과도 보고합니다. 크기가 제한된 복구는 새 receipt와 간결한 메서드
결과, 새 receipt만, 간결한 메서드 결과만, 효과 사실만의 순서로 시도합니다. receipt와 메서드
결과는 잘라 내지 않습니다. 특히 성공한 `volicord.stage_artifact`의 간결한 결과가 receipt를
제외한 단계에서 들어가면 staging handle과 만료 시각을 보존합니다. 각 보존 단계에서 들어가지
않는 필드는 `null`입니다.

조회 대상인 영속 `operation_result_ref`는 receipt만, 간결한 결과만, 효과 사실만 남는
단계를 포함해 모든 보존 단계에 계속 존재합니다. 정확한 결과가 생략된 호출자는
`volicord.get_operation_result`로 page를 읽어 chunk를 이어 붙인 뒤 현재 권한을
`volicord.status`로 읽습니다. 변경을 다시 제출하면 안 됩니다.

`effect_anchor`는 첫 커밋 authority event, staged handle 또는 결과 state effect를
식별합니다. 이는 효과를 연관 짓는 anchor이지 동작 결과 조회 credential이 아닙니다. 기준
범위에서 `volicord.get_operation_result`가 받는 값은 `operation_result_ref`뿐이며,
`volicord.status`는 정확한 메서드 결과를 복원할 수 없습니다. 이 분기는
`MCP_UNAVAILABLE`을 주장하지 않고 권한 상태 새로 고침 실패로 집계되지
않으며, 일부만 남긴 receipt나 바이트 상한을 넘는 상태 본문을 반환하지 않습니다. 호출자는 새
mutation으로 다시 호출하지 말고, 보존된 모든 필드를 사용한 뒤 행동하기 전에 현재 상태를
읽어야 합니다.

Core가 적용된 결과를 이미 반환한 뒤 후속 어댑터 작업이 정상 래퍼를 만들지 못하면,
어댑터는 같은 검증된 권한 상태 새로 고침을 먼저 수행하고 또 다른 `isError=false`,
`retryable=false` 효과 적용 후 분기를 반환합니다. 대기 중인 사용자 행동을 만든 뒤 호스트 User Channel
어댑터가 실패하면 `code=MCP_POST_EFFECT_ADAPTER_FAILED`, 정상 응답 상태 보기를 만드는 중
실패하면 `code=MCP_RESPONSE_PROJECTION_FAILED`입니다. 두 분기 모두 메서드 `tool_name`,
`requested_detail`, 효과 사실, null일 수 있는 `effect_anchor`, null일 수 있는
`operation_result_ref`, null일 수 있는 `authority_receipt`, null일 수 있는 `method_result`,
`authoritative_refresh_succeeded=true`, `response_projection_omitted=true`,
`status_read_required=true`,
`completion_claim_withheld=true`를 담습니다. 상태 보기 실패 분기는 표현할 수 있을 때 정확한
메서드 결과를 보존하고, 호스트 어댑터 실패 분기는 기준 변경 결과에 메서드 결과가 없을
때나 사용할 수 있는 어떤 결과 형태도 복구 예산에 들어가지 않을 때 이를 `null`로 둘 수
있습니다. 크기가 제한된 복구는 각 값이 있을 때 새 receipt와 정확한 결과, 새 receipt와
간결한 메서드 결과, 새 receipt만, 간결한 메서드 결과만, 효과 사실만의 순서로
시도합니다. 어느 분기도 mutation 재실행을 허용하지 않습니다.

갱신 호출이 실패하거나, 거절 또는 잘못된 형식의 분기를 반환하거나, receipt가 없거나,
최신성 비교가 하나라도 실패하면 같은 성공 계열 `isError=false` 복구 경계를 반환합니다.
크기가 제한된 `structuredContent`에는 `code=MCP_UNAVAILABLE`, 메서드 `tool_name`,
`retryable=false`, `reached_core`, `committed`, null일 수 있는 `effect_kind`,
`effect_applied`, null일 수 있는 안정적인 `effect_anchor`, null일 수 있는
`operation_result_ref`, null일 수 있는 간결한 `method_result`, `status_read_required=true`,
`completion_claim_withheld=true`를 담습니다. 간결한 결과는 write ticket, staging handle,
finding별 reconcile 결과, 선택된 Judgment 결과처럼 다음 단계에 필요한 도구별 데이터를
보존합니다. 이 결과를 잘라 내지는 않으며, 간결한 결과 자체가 고정 복구 예산에 들어가지
않으면 필드는 `null`입니다. 이 분기는 원래의 정확한 성공·완료 본문, 오래된 receipt, 비공개
갱신 오류 본문을 반환하지 않습니다. 호출자는 mutation을 다시 제출하면 안 됩니다.
`operation_result_ref`가 null이 아니면 생략된 정확한 과거 결과를 조회하며, 행동하기 전에
현재 상태도 읽어야 합니다. `effect_anchor`는 위와 같은 연관 확인 용도로만 쓰이며 status는
생략된 정확한 메서드 결과를 복원하지 않습니다. 로컬 세션 진단은 오류 본문을 저장하지 않고
이를 권한 상태 새로 고침 실패로 집계합니다.

Core/도메인 거절 변경 응답은 이 성공 상태 보기 경로에 들어가지 않습니다. 기존 공개 응답
객체와 `isError=false`를 유지하고, 짧은 호환 텍스트로 클라이언트가
`structuredContent`를 보도록 안내합니다.

<a id="user-action-elicitation"></a>
### 사용자 행동 입력 요청

`volicord.request_user_action`은 엄격한 중첩 `request.operation=create` variant를 통해
대기 `UserActionRequest`를 만드는 유일한 Agent Connection tool입니다. 같은 도구의
`request.operation=resume` variant는 직접 만든 요청을 이름 붙여 읽기 전용 연속 작업을
수행합니다. `volicord.resolve_user_action`은 MCP tool로 노출되지 않으며 agent 인자가
User Channel 해결이 될 수 없습니다.

`request.operation=create` 요청 커밋 뒤 어댑터는 Core 소유
`UserActionInboxForm`을 선택된 User Channel renderer 안에서만 소비합니다. Agent
Connection 결과는 요청 ID, `status=pending`, `next_actor=user`만 담은
`AgentSafeUserActionRequestSummary`를 받습니다. 판단 폼은
저장된 `selected_option_id`와 선택적 note만 요청합니다. 증거 관찰 폼은 저장 대상
selector 하나, 저장 아티팩트 ID의 비어 있지 않은 부분집합, `supported` 또는
`contradicted`, 크기가 제한된 summary를 요청합니다. label, description, consequence,
기본 선택 표시는 표시 전용입니다. 대상 statement를 포함한 완전한 저장
`EvidenceTarget` metadata와 `display_name`, `content_type`, `sha256`, `size_bytes`,
`integrity_status`, `redaction_state`, `availability`, `created_by_run_ref`,
`created_by_actor_source`, `storage_ref` 필드를 포함한 완전하고 정확한 `ArtifactRef`
metadata도 표시 전용입니다. 제출하는 값은 선택한 대상 selector와
아티팩트 ID뿐이며 표시 metadata는 후보 권한으로 제출하지 않습니다. MCP elicitation은
자르지 않은 완전한
`elicitation/create` JSON-RPC 요청 객체를 UTF-8 JSON으로 인코딩한 바이트와 끝의 LF
1바이트를 합친 크기가 32 KiB 이하일 때만 사용합니다. 그렇지 않으면 그 경로를 사용
불가로 보고하고 협상된 모델 비가시적 local web, CLI 순서의 가용 경로를 사용합니다.
에이전트 대상 prompt-capture fallback은 완전한 폼 전달 표면이 아닙니다. 폼과 제출은
자르지 않습니다.

별도로 검증된 User Channel 표면을 열기 전에 어댑터는 질문, context summary,
표시되는 모든 `EvidenceTarget`과 `ArtifactRef` metadata 값을 포함한 완전한 닫힌 폼
렌더링에 하나의 보수적인 presentation safety 분류를 적용합니다. 완전한 presentation이
비밀값이나 credential 자료를 나타내 사용자 전용 채널을 요구하면 어댑터는
`elicitation/create` 요청을 보내지 않고, 풍부한 prompt-capture 질문, context, 폼,
검증 코드, resolve 명령 템플릿도 출력하지 않습니다. 사용 가능하면 local web consent로,
그렇지 않으면 CLI inbox로 내려갑니다. 이 사용자 전용 local web 및 CLI 표면은 완전한
canonical 폼을 계속 렌더링합니다. 이 분류는 보수적인 어댑터 경로 선택일 뿐이며 일반
비밀값 scanner, 가림 처리 서비스, 격리 경계, 임의의 비밀값을 탐지한다는 보장이
아닙니다.

유효한 elicitation을 accept하면 어댑터는 파생 `local_user` provenance, 인식된
verification basis, 고유 `channel_submission_id`, `expected_state_version=null`로
user-only 해결 경로를 호출합니다. 어댑터가 만드는 모든 submission identity는 visible
ASCII `0x21..=0x7e` 1~256 bytes이며, 어댑터는 유효하지 않은 값을 잘라내거나 정규화해
이 형태로 만들지 않습니다. Core가 preflight에서 현재 상태를 고정합니다. decline은
거절 선택지가 있는 판단 폼에서만 저장 reject 선택지에 대응합니다. cancel, malformed
content, 알 수 없거나 혼합된 후보, stale 폼, 상태 충돌은 해결을 기록하지 않으며 요청이
계속 현재이고 만료되지 않았으면 유효 pending으로 남깁니다.

MCP 결과는 복합 projection입니다. `agent_workflow_result`는 항상 원래 Agent Connection
호출이 커밋한 byte 단위로 정확한 agent-safe 요청 응답이고 `operation_result_ref`는 그
결과만 가리킵니다. 과거 결과는 완전한 요청이나 폼 없이 만들어졌으므로 presentation
safety 경로 선택이 가리거나 다시 쓸 필요가 없습니다. `agent_workflow_result_replayed`가
create와 명시적 resume을 구분합니다.
Resume은 같은 활성 workflow Agent Connection actor 범위와 허용 project를 요구하고 이후
Git workspace 좌표는 비교하지 않습니다. 다른 connection이나 reconciliation이 만든
요청에는 사용할 수 없습니다. 요청, replay 행, event, token, prompt, resolution,
state version을 만들지 않고 영속 정규 UTC 하한도 갱신하지 않습니다.

별도 nullable `user_channel_resolution`은 변경 불가능한 해결과 현재 compact 사실의
agent-safe 구조화 projection을 담습니다. Core는 이 resolution, `current_status`, 정확한
과거 `derived_refs`를 `current_projection_state_version`과
`current_projection_observed_at`으로 식별하는 한 SQLite snapshot에서 읽습니다. 이후 일반
authority refresh는 더 큰 state version일 수 있으며 projection을 다시 표시하지 않습니다.
즉시 host 해결이 `agent_workflow_result`를 user-only 메서드 결과로 바꾸면 안 됩니다. 자유
형식 note와 관찰 summary는 제외합니다. Resolution ref와 derived ref는 이후 관계없는
commit 뒤에도 원래 `produced_at_state_version`을 유지합니다.

어댑터는 `request.operation=create`를 처리하면서 커밋 후 재조회 결과가 계속
`pending`일 때만 `elicitation/create`를 보냅니다. Resume은 현재 상태가 `pending`이어도
`elicitation/create`를 보내거나 local web, CLI fallback을 실행하지 않고
정확한 과거 `agent_workflow_result`와 현재 안전 projection을 반환합니다. Create에서
요청이 `resolved`, `stale`, `superseded`, `expired`이면 새 prompt 없이 현재 안전
projection을 반환합니다. Create 중 요청을 pending으로 남긴 취소·거절·유효하지 않은 host
입력은 정확한 중첩 resume 안내를 포함하고 두 번째 요청을 만들지 않습니다.

fallback 안내는 Core 권한 밖에 남습니다. 사용할 수 없는 host prompt 입력이 다른 가용
경로를 숨기면 안 됩니다. 중앙 전달 평가기가 관리 stdio 호스트 경로, 준비된 loopback
listener, 정확한 클라이언트 선언, 현재의 정확히 일치하는 호스트 역량 검증을 확인하면
짧게 만료되는 local web token을 정확한 요청, form digest, project, connection,
delivery-surface marker에 결속합니다. 원문 credential을 포함한 URL은
아래의 닫힌 최상위 전달값에만 두며 알 수 없거나 추가된 필드는 허용하지 않습니다.

```json
{
  "_meta": {
    "io.volicord/user-channel": {
      "kind": "local_web_consent",
      "url": "http://127.0.0.1:PORT/consent?...&token=...",
      "expires_at": "RFC3339 UTC timestamp"
    }
  }
}
```

`CallToolResult._meta["io.volicord/user-channel"]`는 공개 tool `outputSchema` 밖에 있습니다.
Agent 대상 content는 요청
ID, pending 상태, next actor, 안전한 연속 작업 안내만 보고합니다. 자격 입력 중 하나라도
사용할 수 없으면 token을 발급하지 않고 `volicord inbox`를 안내합니다. 각 pending
fallback은 두 번째 요청을 만들거나 닫힌 공개 응답 스키마 밖의 구조화된 연속 작업 객체를
추가하지 않습니다. User Channel 완료 뒤 workflow를 계속하는 호출자는 다른 create를
발급하지 않고 정확한 pending summary의 request ID로 공개
`request.operation=resume` 분기를 사용합니다.

<a id="local-web-consent-fallback"></a>
로컬 consent listener는 loopback-only이고 fail closed합니다. `GET /consent`에는
`Origin`이 필요하지 않으며 일회용 token을 검증하고 정확한 canonical 폼을 렌더링합니다.
GET이 `Origin`을 보내면 정확한 동일 출처 헤더 필드가 하나만 있어야 합니다. `POST
/consent`는 그 폼의 필드만 받으며 먼저 유효하게 직렬화된 동일 출처 `Origin` 헤더 필드가
하나만 있는지 요구합니다. 누락, 빈 값, `null`, 잘못된 형식, 쉼표로 결합된 값, 반복된
헤더, 다른 `Origin` 값은 양식 본문 디코딩·검증, 토큰 조회·소비, 해결 기록 효과보다
먼저 HTTP 403 `ORIGIN_NOT_ALLOWED`로 실패합니다. 그다음 project, connection, request,
form digest, expiry, 후보 membership, 토큰 상태를 다시 검증합니다. 성공한 폐쇄형
resolution 본문 삽입과 토큰 소비는 원자적입니다.

Listener 시작만으로는 이 경로를 선택하지 않습니다. Listener context는 공유되는 실시간
준비 상태를 가지며 accept loop는 listener 실패 뒤 종료하기 전에 그 상태를 unavailable로
바꿉니다. 하나의 adapter evaluator가 현재 상태와 정확한 모델 비가시적 client 선언,
관리 stdio 시작 원천, 보존된 `clientInfo`, 현재 영속 호스트 역량 상태를 결합해 invocation
도출, User Channel projection, fallback 선택, 최종 handoff materialization에 사용합니다.
현재 상태가 가리키는 변경 불가능한 검증은 `outcome=passed`이고,
`observed_at <= now < expires_at`을 만족하며, 활성화된 generic이 아닌 연결의 호스트 종류,
관리 지문, 어댑터 프로필·버전, Volicord 빌드, source revision, target, 실행 파일
다이제스트, 클라이언트 이름·버전, 크기가 제한된 실제 호스트 증거 다이제스트와 정확히
일치해야 합니다. 예상 증거 다이제스트에는 [호스트 릴리스 증거](host-release-evidence.md)가
담당하는 정확한 외부 `volicord-host-release-manifest-v3`를 신뢰해 획득하는 운영 경로가
필요합니다. 그 manifest는 같은 역량, 호스트·클라이언트, 어댑터, 빌드, source, target,
실행 파일 다이제스트에 결속됩니다. 행의
`evidence_artifact_sha256`은 그 예상값과 정확히 일치해야 하며 행, 빌드 설명자, 복사한 값이
스스로 예상값을 제공할 수 없습니다. Manifest 입력이 없거나, 알 수 없거나, 잘못됐거나,
검증되지 않았거나, 일치하지 않으면 닫힌 상태로 실패합니다. 현재 어댑터에는 그 manifest를
신뢰해 획득하는 경로가 없고 외부 릴리스 아티팩트 자체도 런타임 신뢰 입력이 아니므로 운영
local-web 선택은 사용할 수 없고 CLI fallback을
반환합니다. 수동 stdio, CLI 검증 probe, Local HTTP transport, generic 연결,
유효하지 않거나 알 수 없는 관리 marker는 자격이 없습니다. 통과하는 source revision은
정확한 소문자 40자리 또는 64자리 16진수이며 `unknown`은 통과할 수 없습니다. 내장 stdio
어댑터에서는 `host_version == client_version == clientInfo.version`이고 같은 버전이 실제
아티팩트의 설치 호스트 버전과 일치해야 합니다. 이 같음을 증명할 수 없으면 통과 행이
아닙니다. 검증 구간은 또한 `observed_at <= created_at`,
`observed_at < expires_at <= observed_at + 86,400 seconds`,
`created_at < expires_at`을 만족해야 합니다. 24시간은 기본 수명이나 attestation 기간이
아니라 최대 최신성 구간이며 게시자는 더 짧은 만료 시각을 선택할 수 있습니다. Token 발급
전에 adapter는
완전한 안전 tool result와 닫힌 `_meta` 전달값이
선택된 65,536 또는 262,144-byte 응답 예산에 맞는지 확인합니다. 그다음 협상된 capability를
같은 evaluator에 전달하고 token 삽입과 handoff 구성이 끝날 때까지 준비된 listener의 공유
발급 lease를 획득합니다. Listener 무효화는 이 lease의 배타 쪽을 획득합니다. 이 지점이
순서를 하나로 정합니다. 무효화가 먼저면 token을 발급하지 않고 `_meta`를 생략하며 일반
CLI fallback을 반환합니다. 발급 lease가 먼저면 그 token은 이미 발급된 것으로 보며 이후
listener가 실패해도 제한된 TTL을 유지합니다. 결과가 예산에 맞지 않을 때도 token 없이
fallback합니다. 예산 거부와 발급보다 먼저 순서화된 준비 상태 무효화는 token을 만들지
않으며 adapter는 URL을 자르지 않습니다. 호스트 역량 상태가 없거나, 통과하지 않았거나,
만료·손상·불일치 상태일 때도 token, `_meta`, 프로젝트 시간 하한 효과가 없습니다. 이 전달값은
resume, pending이 아닌 결과, CLI fallback, token 발급
실패, 지원되지 않거나 잘못된 선언, 없거나 오래되거나 취소되거나 손상되거나 일치하지 않는
검증, listener 시작 실패, 발급보다 먼저 순서화된 저하,
응답 예산 저하에서 없습니다. URL과 token은 MCP `content`,
`structuredContent`, 호환·진단 text, status·close projection, 정확한 Core replay,
operation-result byte, log, template에 나타나면 안 됩니다. Host 선언과 일치하는 크기 제한
검증 기록은 계속 협력적 통합 증거일 뿐 host attestation, host 격리, 사용자 identity,
사용자 권한의 증명이 아닙니다. 이 분리를 보존할 수 없는 host는 capability를 생략하고 CLI
fallback을 받아야 합니다.

Base URL만 받는 기존 공개 programmatic adapter builder는 추적되지 않는 source-compatible
fail-closed shim이며, local web을 available로 만들거나 token을 발급하지 않습니다. 지원되는
stdio와 결합형 local HTTP process entry point는 복제할 수 없는 listener guard를 소유하고
공유 managed-readiness 경로로 adapter를 구성합니다. 향후 외부 embedder가 local web을
사용하려면 별도로 소유되는 공개 listener-lifetime 계약이 필요합니다. 호출자가 제공한 base
URL이나 lifetime assertion은 준비 상태의 증거가 아닙니다.

POST 해결에서 어댑터는 유일하게 허용되는 digest-only `local_web:<sha256>` submission
identity를 위한 Core 소유 도출을 사용합니다. 이 도출은 정확한 프로젝트, 사용자 행동
요청, 원문 bearer-token credential, 예상 Agent Connection, 타입이 지정된 canonical 완료
metadata를 결속합니다. Core는 token을 전달하는 진입점에서 identity를 다시 계산하고
domain-separated token digest, connection, 같은 폐쇄형 metadata도 mutation replay
identity에 결속합니다. 이 전체 binding과 canonical resolution이 같은 중복 제출만 원래의
안전한 완료를 반환합니다. 손으로 만든 identity나 서로 다른 token, connection, metadata,
resolution은 replay를 열지 못하며 두 번째 효과도 만들지 않습니다. 원문 token은 일시적인
입력으로만 남습니다. Token 테이블은 domain-separated hash를 저장하고 resolution·replay
저장소와 응답은 파생 digest 또는 hash만 담습니다. Endpoint는 Runtime Home 또는 Product
Repository 파일, static asset, MCP method, 임의 API를 제공하지 않습니다.

Token 발급은 별도 저장소 transaction에서 프로젝트의 정규 Core UTC 시계를 사용합니다.
Token `created_at`을 저장합니다. 요청 expiry가 있으면 `expires_at`은 요청 expiry와
`created_at + 600 seconds` 중 더 이른 값으로 정확히 파생하고, 요청 expiry가 없으면
정확히 `created_at + 600 seconds`로 파생합니다. 또한 원자적으로 영속 프로젝트 시각
하한을 `created_at` 이상으로 전진시킵니다. 권한 event나 replay 행을 만들지 않고
`state_version`도 증가시키지 않습니다. GET과 POST 검증은 정규 현재 프로젝트 시각을
사용합니다. Token은 반열린 구간 `created_at <= now < expires_at`에서만 유효합니다.
`now < created_at`이면 유효하지 않으며 token을 소비하면 안 됩니다.
`now >= expires_at`이면 만료이며 resolution을 만들거나 소비하면 안 됩니다. 이미 파생된
token `expired` 상태를 영속화하더라도 프로젝트 시각 하한은 전진시키지 않습니다. 전체
하한 계약은 [저장소 버전 관리](storage-versioning.md#canonical-core-utc-clock)가 담당합니다.
Token TTL은 checked timestamp 덧셈을 사용하고 저장 문자열은 canonical RFC 3339 UTC여야
합니다. 저장 `created_at`은 요청의 저장 `requested_at`보다 이를 수 없습니다.
Noncanonical 문자열이나 정확히 파생한 값과 다른 저장 expiry는 손상이며 검증, GET, POST,
expiry 정리, 소비를 모두 무효과로 실패시킵니다. Overflow 또는 표현 불가능한 expiry는
token이나 하한을 삽입하기 전에 실패합니다.

token의 저장 생성 metadata는
`{fallback_kind="local_web_consent", delivery_surface="model_invisible_user_surface", endpoint="/consent", form_digest}` 폐쇄형
객체입니다. 구성원이 빠지거나, 추가되거나, 형식 또는 타입이 잘못되거나, 값이 일치하지
않으면 폼 렌더링이나 resolution 시도 전에 실패합니다. 이 실패는 token을 소비하거나
User Channel 효과를 만들지 않습니다. 이 필수 marker 때문에 보정 전 token도 이전의
Agent-visible 전달 계약을 재사용하지 않고 fail closed합니다.

Volicord까지 도달한 알려진 공개 Volicord 메서드 도구 호출에서 `tools/call`은 MCP 결과
안에 Volicord 응답 JSON을 래핑합니다.

- 읽기 전용 메서드 결과는 Volicord 응답 객체를 `result.structuredContent`로 반환합니다.
  해당 호환 JSON 텍스트를 파싱한 값은 계속 그 객체와 같아야 합니다. 단,
  `volicord.get_operation_result`는 page offset과 완료 여부를 알려 주는 UTF-8 기준 최대
  512 byte의 JSON이 아닌 요약을 사용합니다. `chunk_utf8`를 호환 텍스트에 복제하지
  않습니다. 각 page는 원본 UTF-8 byte를 최대 16,384개 담고 완전한 직렬화
  `CallToolResult`는 최대 65,536 byte입니다.
- 성공한 변경 결과는 위에서 정의한 선택 receipt 상태 보기를 사용합니다.
  `result.content[0].text`는 크기가 제한된 짧은 요약이며 JSON으로 파싱될 필요가 없습니다.
- Core/도메인 거절 변경 결과는 공개 응답 객체를 `result.structuredContent`에 유지하고
  크기가 제한된 짧은 호환 텍스트를 사용합니다.
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
