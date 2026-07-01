# 관리 CLI 참조

이 문서는 로컬 `volicord` 관리/부트스트랩 CLI 계약을 담당합니다. CLI는
`Volicord Runtime Home`을 마련하고, 저장소 루트에서 프로젝트를 등록하며, 사용자가
내부 식별 정보를 다루지 않아도 되도록 Agent Connection을 관리하고, 로컬
`User Channel` 명령 경로를 제공하며, generic MCP 설정을 내보내고, 로컬 guard hook
명령을 제공하며, 설정 또는 연결 진단을 보고합니다. 이 명령들은 공개 Volicord API
메서드가 아닙니다.

이 문서는 공개 API 메서드 동작, API 스키마, 저장소 기록 배치, 보안 보장, Core
권한 의미, MCP stdio 전송 동작을 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `volicord` 명령 이름, 명령줄 인자, 기본값, stdout/stderr 처리, 프로세스 종료 코드
- `init` 또는 `setup` 중 Runtime Home, 설치 프로필, 실행 파일 링크, MCP 명령 선택
- 저장소 루트 프로젝트 감지와 관리 프로젝트 명령
- 지원 호스트 통합을 위한 Agent Connection 명령 동작
- generic MCP 설정 내보내기 동작
- 로컬 serve 명령 이름, 명령줄 인자, 기본값, stdout/stderr 처리, 시작 종료 코드
- 로컬 `volicord guard` lifecycle 명령 이름, 옵션, decision, 출력, 이벤트 기록 동작
- 로컬 `volicord changes` 복구 명령 이름과 출력
- 로컬 `User Channel` 명령 이름과 명령 출력
- 진단 상태, 필요한 사용자 동작, dry-run 동작, JSON 출력, 비대화식 동작
- 관리 명령, 로컬 `User Channel` 명령, 공개 Volicord API 메서드 사이의 경계

이 문서는 담당하지 않습니다.

- 공개 Volicord API 메서드: [API 메서드](api/methods.md)
- Agent Connection, Connection Projects, 연결 모드, 연결 의도, 행위자 출처 의미:
  [Agent Connection](agent-connection.md)
- 런타임 데이터 경계 의미와 `Product Repository` 파일 경계 예외:
  [런타임 경계](runtime-boundaries.md)
- MCP 프로세스 시작, stdio와 HTTP 프레이밍, 와이어 동작, 응답 래핑, 종료:
  [MCP 전송](mcp-transport.md)
- 외부 호스트 hook 프로토콜 스키마와 호스트별 응답 의미
- 저장소 기록 배치, SQLite DDL, 일반 저장소 마이그레이션 정의, Core 권한 의미,
  보안 보장 의미

## 명령 모델

`volicord`는 로컬 관리/부트스트랩 실행 파일입니다. 일반 목적의 장기 실행 서버가
아닙니다. 명시적 `volicord serve` 명령은 [MCP 전송](mcp-transport.md)이 설명하는 로컬
MCP 전송 프로세스로 제한됩니다. `volicord user` 명령군은 선택된 Core 메서드 위에 있는
로컬 `User Channel` CLI 어댑터입니다. 이 명령 이름은 공개 Volicord API 메서드가 아니라
관리 CLI 명령으로 남습니다.

지원되는 기준 명령은 아래와 같습니다.

```text
volicord --help
volicord --version
volicord init --host codex|claude-code --repo PATH [--mode mcp-only|guarded|managed] [--allow-degraded] [--home PATH] [--mcp-command PATH] [--dry-run] [--json]
volicord setup [--home PATH] [--link-bin PATH] [--mcp-command PATH] [--json]
volicord doctor [--json]
volicord connect [HOST] [--repo PATH] [--shared|--global] [--read-only] [--dry-run] [--json]
volicord connections [--repo PATH] [--json]
volicord connection status [HOST] [--repo PATH] [--shared|--global] [--json]
volicord connection verify [HOST] [--repo PATH] [--shared|--global] [--json]
volicord connection mode [HOST] workflow|read-only [--repo PATH] [--shared|--global] [--json]
volicord connection remove [HOST] [--repo PATH] [--shared|--global] [--dry-run] [--json]
volicord project use [PATH] [--json]
volicord project current [--json]
volicord project list [--json]
volicord project rename NAME [--repo PATH] [--json]
volicord project forget [PATH|NAME] [--json]
volicord export mcp-config [--output PATH] [--repo PATH] [--read-only] [--json]
volicord serve --transport streamable-http [--listen 127.0.0.1:8765] [--home PATH] [--connection <connection_id>] [--project PATH]... [--token TOKEN | --generate-token] [--allow-origin ORIGIN] [--allow-nonlocal-listen]
volicord guard session-start [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--guard-mode MODE] [--text]
volicord guard pre-tool [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--guard-mode MODE] [--text]
volicord guard post-tool [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--guard-mode MODE] [--text]
volicord guard prompt-capture [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--guard-mode MODE] [--text]
volicord guard stop [--file PATH] [--repo PATH] [--connection ID] [--session ID] [--guard-installation ID] [--host HOST] [--guard-mode MODE] [--text]
volicord changes reconcile [--repo PATH] [--task active|ID] [--json]
volicord user status [--repo PATH] [--task active|ID] [--json]
volicord user judgments [--repo PATH] [--task active|ID] [--json]
volicord user judgment show INDEX_OR_ID [--repo PATH] [--json]
volicord user judgment answer INDEX_OR_ID OPTION_INDEX_OR_ID [--repo PATH] [--note TEXT] [--json]
```

지원되는 `HOST` 값은 `codex`와 `claude-code`입니다. `HOST`를 생략하면 명령은
모호하지 않은 현재 호스트 맥락을 사용할 수 있습니다. 호스트를 모호하지 않게
식별할 수 없으면 명령은 지원되는 호스트 값을 이름 붙인 진단 동작과 함께
실패합니다.

종료 코드와 스트림 동작:

- 성공한 명령은 성공 출력을 stdout에 쓰고 종료 코드 `0`으로 끝납니다.
- `action_required`는 성공한 관리 결과이며 종료 코드 `0`으로 끝납니다.
- `failed`, 런타임 오류, 저장소 오류, 검증 실패, 충돌은 종료 코드 `1`로
  끝납니다.
- 사용법 오류는 진단을 stderr에 쓰고 종료 코드 `2`로 끝납니다.
- `volicord --version`은 stdout에 `volicord <version>`을 쓰며 Runtime Home 해석을
  요구하지 않습니다.
- `--json`은 stdout에 JSON 문서 정확히 하나를 쓰며 사람용 설명을 섞지 않습니다.
- `volicord guard`는 기본적으로 JSON을 씁니다. `deny` decision은 종료 코드 `1`로
  끝나며, `allow`, `warn`, `inject_context`는 종료 코드 `0`으로 끝납니다.
- 오류는 CLI 종료 코드 모델에 따라 stderr 진단으로 남습니다.
- `volicord serve --transport streamable-http`는 명시적 장기 실행 MCP 전송 프로세스입니다.
  기본 리스너는 loopback으로 유지하고 bearer 인증을 요구하며, HTTP 와이어 동작과 전송
  보안 점검은 [MCP 전송](mcp-transport.md)에 맡깁니다.

지원하지 않는 것:

- CLI에는 일반 목적의 `server` 또는 daemon 명령이 없습니다.
- `volicord serve`는 공개 Volicord API 서비스나 인증 없는 네트워크 서비스로 취급하면
  안 됩니다.
- 관리 명령은 공개 Volicord API 메서드가 아니며 공개 메서드 목록에 추가되면
  안 됩니다.
- Guard 명령은 협력적이고 탐지적인 hook 명령이며 OS 수준 sandboxing이나 보안 집행
  증명이 아닙니다.
- 텍스트 모드 사용자 흐름은 `project_internal_id`, `connection_internal_id`,
  호스트 설정 키, 프로토콜 래퍼, 저장된 registry 필드를 사용자가 입력하도록 요구하면
  안 됩니다.

<a id="runtime-home-selection"></a>
## setup과 Runtime Home

`volicord setup`은 저장소를 연결하지 않고 로컬 설치 프로필을 마련하거나 복구합니다.
선택된 Runtime Home을 만들거나 검증하고, 이후 관리 명령, Agent Connection, export,
MCP 프로세스 흐름이 사용할 명령 경로를 저장합니다. Setup은 독립 설치 프로필 명령이지
일반적인 guarded 첫 실행 저장소 경로가 아닙니다. `volicord init`은 기본 첫 실행 경로이며
저장소 설정과 호스트 연결을 수행하면서 Runtime Home 경로나 MCP 시작 명령도 선택할 수
있습니다. Setup은 `volicord`를 `PATH`에서 사용할 수 있게 돕지만 부모 셸의 현재 환경을
바꿀 수는 없습니다.

텍스트 모드에서 `volicord setup`은 stdin과 stdout이 대화형 터미널이고 `--json`이
없으며 `--link-bin`도 없을 때만 프롬프트를 표시할 수 있습니다. 선택된 명령 경로가
`PATH`에 준비되어 있지 않을 때만 프롬프트를 표시합니다. 비대화식 조건, JSON 모드,
명시적 `--link-bin` 모드에서는 프롬프트 대신 동작을 보고해야 합니다.

setup의 최상위 상태는 설치 프로필 준비에 이름 붙은 사용자 동작이 아직 필요한지를
답합니다. Runtime Home과 설치 프로필을 저장한 뒤에도 선택된 명령을 이후 셸이나
에이전트 호스트가 `PATH`로 찾을 준비가 되어 있지 않으면 setup은
`action_required`를 보고할 수 있습니다. setup 출력은 명령 가용성 세부사항과 필요한
동작을 명시적으로 보여 줘야 합니다.

인자:

| 인자 | 의미 |
|---|---|
| `--home PATH` | `Volicord Runtime Home`을 선택합니다. 생략하면 플랫폼 기본 로컬 런타임 위치를 사용합니다. 선택한 경로는 프로젝트 상태를 사용하기 전에 Runtime Home/Product Repository 분리 계약을 만족해야 합니다. |
| `--link-bin PATH` | 필요하면 디렉터리를 만들고 쓰기 가능 여부를 확인한 뒤, 가능할 때 그곳에 `volicord` 명령 링크를 만들거나 갱신합니다. 명령은 대상 경로를 보고하고 안전하지 않은 교체를 거절하며, 이 옵션 자체가 셸 시작 파일이나 부모 셸 `PATH`를 편집하지는 않습니다. |
| `--mcp-command PATH` | 관리 호스트 설정과 generic export가 `mcp --stdio --connection <connection_id>` 인자를 붙여 사용할 정확한 `volicord` 명령을 저장합니다. 생략하면 setup이 선택한 실행 중인 `volicord` 실행 파일을 사용합니다. |
| `--json` | 기계 판독 비대화식 출력을 선택합니다. JSON 모드에서는 setup이 프롬프트를 표시하지 않습니다. |

Setup 효과:

- Runtime Home registry를 만들거나 검증합니다.
- Runtime Home 식별 정보와 설치 프로필 메타데이터를 기록합니다.
- 이후 `init`, `connect`, `doctor`, export, MCP 시작 흐름이 사용할 `volicord` 명령 위치와 MCP
  시작 명령을 기록합니다.
- 선택된 명령 경로가 현재 프로세스의 `PATH`로 해석되는지 검사합니다.
- 대화형 텍스트 모드에서는 안전한 명령 가용성 선택지를 물어볼 수 있습니다.
  선택지는 쓰기 가능성이 확인된 기존 setup 제안 디렉터리에 명령 링크 생성,
  `HOME` 아래의 없는 관례적 사용자 명령 디렉터리(예: `~/.local/bin`)를 setup이
  안전하게 만들 수 있고 생성 뒤 쓰기 가능성을 확인할 수 있을 때 그곳에 링크 생성,
  승인된 셸 시작 `PATH` 블록 쓰기, 셸 명령 출력, 링크 건너뛰기입니다.
- `--link-bin`으로 지정했거나 대화형 프롬프트에서 선택한 `volicord` 명령 링크를
  갱신할 수 있습니다.
- 명시적 대화형 승인을 받은 뒤에만 관리되는 셸 시작 `PATH` 블록을 쓸 수 있습니다.
- 링크 디렉터리가 현재 프로세스의 `PATH`에 보이지 않으면 `PATH` 동작을 보고합니다.
  기존 셸과 에이전트 호스트 프로세스에는 restart 또는 reload가 필요할 수 있습니다.
- 임의의 존재하지 않는 경로를 자동 대화형 명령 링크 선택지로 제안하지 않습니다.
  명시적으로 없는 디렉터리를 사용하려면 `--link-bin PATH`를 사용합니다.
- 별도의 프로젝트 또는 연결 명령이 저장소를 선택하기 전에는 프로젝트를 등록하지 않습니다.
- 공개 Volicord API 메서드를 만들거나 사용자 소유 판단을 기록하지 않습니다.

Unix에서 대화형 셸 시작 파일 갱신은 setup이 `HOME`과 `SHELL`을 식별할 수 있을 때
`bash`, `zsh`, `sh`에 대해 지원됩니다. 대상 파일은 각각 `~/.bashrc`,
`~/.zshrc`, `~/.profile`입니다. Setup은 사용자가 정확한 블록을 승인한 뒤 그 파일의
Volicord 관리 블록을 쓰거나 갱신합니다. 지원되지 않는 셸, 지원되지 않는 플랫폼,
누락된 환경 변수, 쓰기 실패는 수동 `PATH` 동작으로 남습니다.

`volicord doctor`는 설치 프로필을 위한 읽기 중심 진단 명령입니다. 최상위 상태는
현재 설치 프로필을 사용할 수 있는지를 답합니다. Runtime Home 접근, registry 스키마,
설치 프로필 존재 여부, 저장된 명령 준비 상태, `PATH`를 통한 명령 가용성, 링크
메타데이터가 있을 때의 명령 링크 또는 shim 준비 상태를 확인합니다. 저장된 명령
경로가 실행 가능하면 doctor는 `complete`를 보고하면서도 이후 셸이나 에이전트
호스트를 위한 명령 가용성 경고와 `actions_recommended`를 함께 보고할 수 있습니다.
`PATH` 또는 명령 링크 권장 동작은 기존 에이전트 호스트에 restart 또는 reload가
필요할 수 있는 때를 말해야 합니다. 지원 호스트 감지는 연결 검증이 보고할 문제로
표시합니다. guard 설치 기록이 있으면 doctor는 guard 파일 설치, 설정 건강 상태,
런타임 hook 관찰 건강 상태, 효과적인 guard 건강 상태, 호스트 reload 필요도 진단으로
보고할 수 있습니다. 이 guard 진단은 로컬 setup 및 관찰 점검이며 OS 강제,
sandboxing, 쓰기 방지, 제품 정확성,
닫기 준비 상태의 증명이 아닙니다. Doctor는 `guard_strength`와 그 라벨의 근거가 되는
기능 boolean도 보고합니다. session watcher 우회 감지나 local web consent 같은 런타임
전용 기능은 보고 프로세스가 실제로 그 런타임 상태를 소유하지 않는 한 사용할 수 없음으로
보고합니다. 프로젝트를 만들거나, 호스트 설정을 설치하거나, 연결 모드를 바꾸거나,
사용자 판단에 답하지 않습니다.

<a id="project-commands"></a>
## 프로젝트 명령

프로젝트 명령은 저장소 루트를 사용자 대상 프로젝트 식별자로 사용합니다. 내부 프로젝트
식별 정보는 저장소와 출처 데이터이며 텍스트 모드 명령은 이를 요구하지 않습니다.

저장소 루트 감지:

- `--repo PATH`와 `PATH` 인자는 프로젝트 조회 전에 해석됩니다.
- 경로가 제공되지 않으면 명령은 프로세스의 현재 작업 디렉터리를 사용합니다.
- 감지된 저장소 루트는 선택한 경로를 포함하는 가장 가까운 지원 저장소 루트입니다.
  루트를 감지할 수 없으면 프로젝트가 필요한 명령은 `volicord project use PATH`를
  이름 붙인 진단 동작과 함께 실패합니다.
- Runtime Home과 `Product Repository` 경로는 [Runtime Home/Product Repository
  분리 계약](runtime-boundaries.md#runtime-home-product-repository-separation)을
  만족해야 합니다.

`volicord project use [PATH]`는 감지된 저장소 루트를 등록하거나 재사용합니다.
등록은 `project_internal_id`, 사용자 대상 프로젝트 이름, Runtime Home 아래의
프로젝트 홈, 필요한 프로젝트별 상태를 만듭니다. 기본 프로젝트 이름은 저장소 디렉터리에서
파생하고 Runtime Home 안에서 필요하면 고유하게 만듭니다.

`volicord project current`는 현재 작업 디렉터리에서 감지된 프로젝트를 보고합니다.
프로젝트 등록을 만들지 않습니다.

`volicord project list`는 등록 프로젝트를 사용자 대상 이름, 저장소 루트, 상태,
진단 가용성과 함께 나열합니다.

`volicord project rename NAME [--repo PATH]`는 선택된 저장소의 사용자 대상 프로젝트
이름을 바꿉니다. `project_internal_id`, 저장소 루트, 프로젝트 홈, Core 상태는
바꾸지 않습니다.

`volicord project forget [PATH|NAME]`은 active Agent Connection 멤버십이나 담당
문서가 계속 주소 지정 가능해야 한다고 요구하는 프로젝트 상태를 고아로 만들지 않을
때만 선택된 프로젝트 등록을 제거합니다. 프로젝트를 잊는 동작은 `Product Repository`,
관련 없는 Runtime Home 데이터, 호스트 설정, 남아 있는 다른 등록이 소유하는
아티팩트 저장소, 보존되어야 하는 Core 권한 행을 삭제하면 안 됩니다.

<a id="connection-intents-and-hosts"></a>
## 연결 의도와 호스트

Agent Connection 설정은 낮은 수준의 호스트 설정 범위 이름 대신 연결 의도를
사용합니다.

| 의도 | 선택 방법 | 의미 |
|---|---|---|
| `personal` | 기본값 | 현재 사용자의 일반 로컬 흐름을 위한 사용자 소유 호스트 설정입니다. |
| `shared` | `--shared` | 선택된 `Product Repository` 안의 명시적 통합 파일로 저장되는 프로젝트 소유 또는 프로젝트 공유 호스트 설정입니다. |
| `global` | `--global` | 선택된 호스트의 사용자 전역 호스트 설정입니다. 프로젝트 접근은 계속 등록된 저장소 루트와 Connection Projects로 제한됩니다. |

`--shared`와 `--global`은 함께 사용할 수 없습니다. 둘 다 없으면 의도는
`personal`입니다.

연결 모드:

- `workflow`가 기본 모드입니다.
- `read-only`는 명시적으로 선택하며 Agent Connection을 통해 읽기와 프로젝트 탐색
  동작만 노출합니다.
- `volicord connection mode ... workflow|read-only`는 사용자가 호스트 설정을 직접
  편집하지 않아도 선택된 연결의 저장 모드를 바꿉니다.

내부 호스트 설정 키 `server_name`의 기본값은 `volicord`입니다. 일반 CLI 흐름은
서버 이름 옵션을 노출하지 않습니다. 생성된 호스트 설정에는 호스트가
`volicord mcp --stdio`를 시작할 수 있도록 저장된 `connection_internal_id`에서 파생된
`connection_id` 프로세스 바인딩 값, 서버 이름, 명령 인자가 들어갈 수 있습니다. 이
값들은 저장된 프로세스 바인딩 세부사항이며 사용자 권한 토큰이 아닙니다. 텍스트 모드
명령 입력은 선택된 호스트, 의도, 저장소 루트를 사용합니다.

일반 `volicord connect` 명령은 MCP 명령 경로나 Runtime Home 경로를 다시 묻지 않고
해석된 Runtime Home에 저장된 프로필을 사용합니다. 개인, 로컬, 사용자 전체 호스트
설정은 그 Runtime Home을 `VOLICORD_HOME`으로 담을 수 있습니다. shared 프로젝트
호스트 설정은 개인 Runtime Home 경로를 포함하면 안 되며, 미래의 호스트 환경이
`PATH`로 해석해야 하는 명령 이름 `volicord`와 `mcp --stdio --connection <connection_id>`
인자를 사용합니다.

<a id="agent-host-setup-and-init"></a>
`volicord init --host codex --repo PATH --mode mcp-only`와
`volicord init --host claude-code --repo PATH --mode mcp-only`는 필수 hook 지원을 설치하지
않는 chat-first 사용을 위한 더 낮은 보장의 첫 실행 저장소 설정 및 호스트 연결
예시입니다. Init은 shared 프로젝트 범위 호스트 레이아웃을 사용하므로 생성된 호스트 MCP
설정은 `PATH`를 통해 `volicord mcp --stdio`를 시작하고 개인 Runtime Home 경로를
포함하지 않습니다.

`--mode`는 guard 통합 수준을 선택합니다.

- `mcp-only`는 MCP 설정, 관리되는 `AGENTS.md` 안내 블록, guard 명령이 비활성화된
  policy 메타데이터를 씁니다. Guard 활성화를 요구하지 않는 guard 설치 상태를
  기록합니다.
- `guarded`가 기본값입니다. MCP 설정, 관리되는 `AGENTS.md` 안내 블록,
  `.volicord/policy.json` guard 명령 policy, 그리고 지원되는 프로젝트 로컬 호스트
  hook 파일과 rule 파일을 씁니다.
- `managed`는 일반 프로젝트 로컬 설정과 구별되는 검증된 managed 배포 출처를
  요구합니다. 예를 들어 Volicord 호스트 계약 데이터에 기록된 호스트 지원 plugin,
  managed 설정 bundle, managed policy 계층이어야 합니다. 선택한 호스트에 검증된
  managed 배포 계약이 없으면 init은 `MANAGED_MODE_UNSUPPORTED`로 실패하며 프로젝트
  로컬 guarded 파일을 managed 대체물로 생성하지 않습니다.

guard를 인식하는 setup, status, verification, doctor 출력은 도출된 현재 보호 라벨인
`guard_strength`를 보고합니다.

| `guard_strength` | 의미 |
|---|---|
| `authority_record_only` | Volicord가 권한 상태는 기록할 수 있지만 선택된 보기에 대해 활성 session watcher나 관찰된 전체 host hook guard를 사용할 수 없습니다. |
| `detective_watch` | session watcher가 활성 상태라 Product Repository 변경에서 미기록 변경 찾기를 만들 수 있습니다. 쓰기를 사전에 차단하거나 행위자를 식별할 수는 없습니다. |
| `host_hook_guarded` | 선택된 프로젝트 로컬 guarded host hook이 모든 필수 lifecycle phase에 대해 설정되고 관찰되었습니다. |
| `managed_guarded` | host-hook guarded 조건을 만족하고 선택된 managed 배포 메타데이터가 검증되었습니다. 현재 Codex와 Claude Code 설정은 향후 검증된 managed 배포 계약 없이는 이 라벨에 도달하지 않습니다. |

완전한 `guarded` 초기화에는 선택한 호스트 어댑터가 모든 필수 lifecycle hook인
`session-start`, `pre-tool`, `post-tool`, `prompt-capture`, `stop` 지원을 선언하고
검증할 수 있어야 합니다. `AGENTS.md`와 `.volicord/policy.json`은 호스트 hook 설정이
아닙니다. 어댑터가 모든 필수 phase에 대해 신뢰할 수 있는 프로젝트 로컬 hook 스키마나
경로를 알지 못하면 init은 호출자가 `--allow-degraded`를 전달하지 않는 한
`GUARDED_HOOKS_UNSUPPORTED`로 실패합니다. 명시적인 degraded opt-in은 MCP 설정, 안내,
policy, 지원되는 hook 또는 rule 파일을 쓸 수 있지만, degraded guard 상태를 기록하고
사람용 출력과 JSON 출력에 누락된 필수 hook phase를 보고합니다. `mcp-only`는 hook 설치를
요구하지 않습니다.

Managed 초기화는 guarded hook 요구사항과 별도의 managed 배포 요구사항을 모두 만족해야
합니다. 검증된 managed 계약이 없는 호스트에서 `--allow-degraded`는 적용되지 않았다고
보고되며, `managed`를 조용히 `guarded`나 `mcp-only`로 바꾸지 않습니다.

`guarded`에서 init은 생성된 guard hook을 로드하려면 호스트 restart 또는 reload가 아직
필요할 때 `reload_required`를 기록하고, 파일은 설치되었지만 일치하는 guard hook이 아직
관찰되지 않았을 때 `configured`를 기록합니다. 파일을 썼다는 이유만으로 guard 설치를
`active`로 표시하지 않습니다.

`--home PATH`는 이 초기화에 사용할 Runtime Home을 선택합니다. `--mcp-command PATH`는
init이 설치 프로필을 만들거나 갱신해야 할 때 정확한 명령 경로를 설치 프로필에
저장합니다. 프로젝트 범위 호스트 MCP 설정은 그래도 `PATH`의 `volicord`를 사용합니다.

dry-run이 아닌 `volicord init`은 다음을 수행합니다.

- Runtime Home이 없으면 초기화합니다.
- 필요하면 설치 프로필을 만들거나 갱신합니다.
- 선택한 `Product Repository`를 등록하거나 재사용합니다.
- 일치하는 Agent Connection과 Connection Projects 멤버십을 만들거나 갱신합니다.
- `volicord mcp --stdio --connection <connection_id>`를 사용하는 프로젝트 범위 Codex
  `.codex/config.toml` 또는 Claude Code `.mcp.json`을 씁니다.
- `AGENTS.md` 안의 Volicord 관리 블록만 쓰거나 갱신합니다.
- `volicord guard`를 호출하는 guard 명령을 담은 `.volicord/policy.json`을 씁니다.
- `.codex/hooks.json` 또는 `.claude/settings.json` 같은 지원 호스트 hook 파일을
  씁니다.
- `.codex/rules/*.rules` 또는 `.claude/rules/volicord.md` 같은 지원 호스트 rule 파일을
  씁니다.
- Runtime Home registry에 guard 설치 상태를 기록합니다.
- 필수 호스트 hook 설정이 없을 때 `--allow-degraded`가 명시적으로 제공되지 않았다면
  `mcp-only`가 아닌 guarded 초기화를 거부합니다.
- 호스트가 새 MCP 또는 guard 설정을 로드해야 할 때 필요한 restart, reload, trust,
  approval 동작을 보고합니다.

init 재실행은 일치하는 Volicord 관리 내용에 대해 idempotent입니다. 관리 블록, policy
파일, 호스트 MCP 항목, guard 설치 행을 중복 없이 갱신합니다. 기존 대상에 Volicord가
소유 마커나 관리 지문을 요구하는 위치의 비관리 내용이 있으면 init은 이를 덮어쓰지
않고 충돌로 보고해야 합니다.

<a id="volicord-agent-install"></a>
## Agent Connection 명령

연결 선택은 호스트, 의도, 저장소 루트를 사용합니다. 의도 플래그가 없고 저장소가
선택되어 있으면 status, verify, mode, remove는 그 호스트와 저장소에 대해 의도를
가로질러 하나만 일치하는 연결을 선택합니다. 둘 이상의 연결이 일치하면 명령은
모호한 선택자를 보고하고 호출자는 일치하는 의도 플래그를 추가해야 합니다. 명령은
내부 연결 식별자를 파생하거나 조회합니다.

| 명령 | Runtime Home registry 효과 | 호스트 설정 효과 | 검증 효과 |
|---|---|---|---|
| `volicord init` | 필요하면 Runtime Home과 설치 프로필을 초기화하고, 선택된 저장소 프로젝트를 등록하거나 재사용하며, shared 프로젝트 범위 Agent Connection을 만들거나 갱신하고, Connection Projects 멤버십을 보장하며, guard 설치 상태를 기록합니다. | `codex` 또는 `claude-code`를 위한 관리 프로젝트 로컬 MCP 설정, `AGENTS.md` 안내, `.volicord/policy.json`, 지원 호스트 hook 파일과 rule 파일을 설치하거나 갱신합니다. | 관찰 가능한 곳에서 호스트 설정, MCP 시작, 초기화, `tools/list` 점검을 실행한 뒤 필요한 host reload, restart, trust, approval 동작을 보고합니다. |
| `volicord connect` | 선택된 저장소 프로젝트를 등록하거나 재사용하고, 일치하는 Agent Connection을 만들거나 갱신하며, 연결 의도와 모드를 기록하고, 프로젝트가 Connection Projects에 들어 있음을 보장합니다. | 선택된 의도에 따라 `codex` 또는 `claude-code`용 관리 호스트 설정을 설치하거나 갱신합니다. | 관찰 가능한 곳에서 호스트 설정, MCP 시작, 초기화, `tools/list` 점검을 실행합니다. |
| `volicord connections` | 일치하는 Agent Connection과 연결 프로젝트를 읽습니다. | 호스트를 시작하지 않고 호스트 설정을 다시 쓰지 않습니다. | 저장된 검증 상태와 진단 검증 상태를 호스트 점검 없이 보고합니다. |
| `volicord connection status` | 선택된 Agent Connection 하나를 읽습니다. | 호스트를 시작하지 않고 호스트 설정을 다시 쓰지 않습니다. | 저장된 전체 검증 상태와 필요한 사용자 동작을 보고합니다. |
| `volicord connection verify` | 선택된 Agent Connection을 읽고 마지막으로 알려진 검증 상태를 갱신합니다. | 호스트 통합이 관찰 가능한 대상을 소유하면 관리 대상을 검사합니다. | 관찰 가능한 점검을 실행하고 결과 검증 상태를 저장합니다. |
| `volicord connection mode` | 선택된 연결 모드를 갱신합니다. | 모드를 반영하려면 호스트 항목을 다시 생성해야 하는 경우가 아니면 호스트 설정을 다시 쓰지 않습니다. | 모드 변경 뒤 진단을 보고합니다. |
| `volicord connection remove` | 선택된 Connection Projects 멤버십을 제거하고 소유 멤버십이 남지 않으면 Agent Connection을 제거합니다. | 소유권과 안전 점검이 허용할 때 일치하는 관리 호스트 설정만 제거합니다. | 프로젝트, Core 상태, Runtime Home, 아티팩트 저장소, 관련 없는 호스트 설정을 삭제하지 않습니다. |

규칙:

- `volicord connect`는 기본적으로 Runtime Home의 모든 프로젝트를 연결하면 안 됩니다.
- 선택 프로젝트는 항상 저장소 루트에서 해석되며 명령이 지속 프로젝트 등록을 필요로
  하면 자동 등록됩니다.
- shared 의도는 [런타임 경계](runtime-boundaries.md#explicit-integration-files-in-product-repositories)가
  허용하는 명시적 통합 파일만 쓸 수 있습니다.
- 같은 생성 호스트 대상의 기존 비관리 호스트 설정은 충돌입니다. 일치하는
  Volicord 관리 내용은 소유 명령으로만 갱신하거나 제거할 수 있습니다.
- 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, restart, reload, 그 밖의
  호스트 통제 동작은 계속 사용자 통제 호스트 동작입니다.

<a id="agent-connection-result-states"></a>
<a id="agent-setup-result-states"></a>
## 연결 결과 상태

Agent Connection 명령은 아래 결과 상태를 사용합니다.

| 상태 | 의미 |
|---|---|
| `not_verified` | 선택된 Agent Connection에 현재 기록된 검증 결과가 없습니다. 호스트가 실패했다는 증거가 아닙니다. |
| `complete` | 오래 유지되는 Agent Connection 상태가 있고, 관리 호스트 설정이 존재하며 예상 관리 지문과 일치하고, 필요한 호스트 로드 가능성 및 신뢰 게이트가 충족되고, MCP 시작이 성공하고, MCP 초기화가 성공하며, `tools/list`가 모드에 필요한 도구를 노출합니다. |
| `action_required` | 오래 유지되는 Agent Connection 상태와 호스트 설정은 있지만 호스트 신뢰, 프로젝트 승인, OAuth, reload, restart, 명령 링크 복구, 설치 프로필 복구, 또는 그와 비슷한 사용자 통제 동작이 남아 있습니다. |
| `failed` | 요청한 명령이나 검증이 사용할 수 있는 오래 유지되는 Agent Connection 상태, 사용할 수 있는 호스트 설정, 또는 필요한 로컬 전제 조건을 만들지 못했습니다. |
| `dry_run` | 명령이 영속 변경 없이 계획된 동작을 보고했습니다. |

검증 출력은 점검과 사용자 동작을 일급 진단으로 만들어야 합니다. Text 출력은 전체
상태, 시도되었거나 차단된 각 점검, 필요한 경우 다음 사용자 동작을 보여 줘야 합니다.
JSON 출력은 진단 소비자를 위해 최상위 `status`, `checks`, `actions` 필드를 포함해야
합니다. 연결 상태 및 검증 출력은 guard 파일 설치, 설정 건강 상태, 런타임 hook 관찰
건강 상태, 효과적인 guard 건강 상태, 호스트 reload 필요, prompt-capture 가용성, 알
수 있을 때의 최근 guard event를 별도 진단으로 유지해야 합니다. 또한 `guard_strength`,
pre-tool 차단 가용성, post-tool 상관 가용성, 우회 감지 가용성, prompt-capture 가용성,
local web consent 가용성, managed 배포 검증을 별도 필드로 보고해야 합니다. 파일이
설치되었거나 설정되었다는 사실을 활성 관찰된 guard hook이나 일치하는 관찰 전의
host-hook guarded 강도로 보고하면 안 됩니다.

성공한 `volicord mcp --check` 시작 점검만으로는 Agent Connection을 `complete`로 설명하면
안 됩니다. 이는 MCP 프로세스의 시작 검증일 뿐입니다.

<a id="generic-mcp-config-export"></a>
## generic MCP 설정 내보내기

`volicord export mcp-config [--output PATH] [--repo PATH] [--read-only] [--json]`는
호스트 중립 MCP 설정을 내보냅니다. 이는 별도 export 흐름이며 일반 호스트 연결
의도가 아닙니다.

규칙:

- 명령은 선택된 저장소 프로젝트를 루트 기준으로 해석하거나 등록합니다.
- setup이 유효하지 않으면 `action_required` setup 진단을 보고하고, 유효하면 설치
  프로필의 저장된 MCP 명령을 사용합니다.
- 내보낸 명령이 묶인 `volicord mcp --stdio` 프로세스를 시작하는 데 필요한 내부 registry
  상태를 만들거나 갱신할 수 있습니다.
- `--read-only`를 생략하면 workflow 모드를 사용합니다.
- `--output PATH`가 있으면 설정을 그 정확한 출력 파일에 씁니다. `--output`을
  생략하면 해석된 저장소 맥락의 기본 MCP 설정 파일을 쓰며, 기본 파일 이름은
  `volicord.mcp.json`입니다.
- 내보낸 설정은 export 뒤에도 사용자 관리 설정으로 남습니다. Volicord는 임의 외부
  호스트가 이를 로드, 신뢰, 승인, 초기화, 노출했다고 주장하면 안 됩니다.

## Guard hook 명령

`volicord guard` 명령은 agent lifecycle event 때 명령을 실행할 수 있는 호스트를
위한 로컬 hook 진입점입니다. 이 명령은 등록된 프로젝트 상태를 검사하고
guarded-operation 이벤트를 기록하며 기계 판독 가능한 로컬 decision을 반환합니다.
Core 메서드, 사용자 소유 판단, `Write Check`, 닫기 준비 상태 점검, 호스트 신뢰,
셸 승인, OS 수준 sandboxing을 대체하지 않습니다.

각 guard 명령은 기본적으로 stdin에서 JSON hook event 하나를 읽습니다. `--file PATH`는
테스트나 이벤트를 파일에 준비하는 호스트 통합을 위해 그 파일에서 JSON event를
읽습니다. 기본 출력은 JSON이며 `decision`, `allowed`, `guard_event_id`, 선택적
`session_id`, 명령별 `result`를 포함합니다. `--text`는 사람이 읽기 쉬운 짧은 한 줄
출력을 선택합니다. 지원되는 decision은 `allow`, `deny`, `warn`, `inject_context`입니다.

프로젝트 선택은 `--repo PATH`, event에 있는 프로젝트나 저장소 필드, 또는 현재 작업
디렉터리를 사용합니다. Hook event에 `connection_id`가 없으면 `--connection ID`로
Agent Connection 식별 정보를 제공합니다. `--session ID`, `--guard-installation ID`,
`--host HOST`, `--guard-mode MODE`로 기록되는 세션, 설치, 호스트 종류, guard 모드를
고정할 수 있습니다. 호스트 종류는 `codex`, `claude_code`, `generic` 같은 저장소 값을
사용합니다. Guard 모드는 `mcp_only`, `guarded`, `managed`입니다.

`mcp_only`가 아닌 guard 명령이 기록된 프로젝트, Agent Connection, guard 설치,
호스트 종류, guard 모드, policy hash, 알려진 hook 단계와 일치하는 유효한 event를
받으면 Volicord는 관찰 메타데이터를 기록합니다. 필요한 hook 설정이 완전하고 설치가
degraded, stale, broken 상태가 아닐 때만 그 관찰이 guard 설치를 `active`로 승격할 수
있습니다. 프로젝트, 연결, 호스트 종류, guard 모드, policy hash, hook 단계 데이터가
맞지 않으면 설치를 활성화하지 않습니다. `active`는 Volicord가 현재 사용할 수 있는
guard 설정에 대해 일치하는 hook event를 관찰했다는 뜻입니다. OS 수준 집행,
sandboxing, 쓰기 방지를 주장하지 않습니다.

입력 event 계약은 호스트 중립입니다. Guard 파서는 호스트 종류, 세션, 도구 이름,
명령, prompt, 결과, 변경 경로의 일반적인 필드 위치를 관대하게 읽고, 알 수 없는
필드는 저장되는 guard event의 redacted subject에 보존합니다. Prompt 형태 필드는
기본적으로 hash하거나 생략합니다. Prompt capture 기록은 이후 담당 문서가 별도
정책을 정의하기 전까지 prompt hash를 저장하고 prompt text는 생략합니다.

Lifecycle 동작:

- `session-start`는 Agent Session을 기록하거나 재사용하고, 호스트 세션 주입용으로
  간결한 프로젝트, active task, `Write Check`, 대기 판단, blocker, 미해결 변경
  맥락과 함께 `inject_context`를 반환합니다.
- `pre-tool`은 읽기 전용, 명확한 변경, 불확실한 도구 시도를 분류합니다. 읽기와 상태
  명령은 blocker를 만들지 않고 허용됩니다. 제품 파일 쓰기 시도는 active task가
  없거나, 현재 active `Write Check`가 없거나, 시도 대상이 선택된 `Product Repository`
  밖에 있거나, policy가 명확한 변경 shell 명령을 차단할 때 `deny` 또는 `warn`을
  반환할 수 있습니다. 불확실한 shell 명령은 guard policy가 `deny`를 요구하지 않으면
  기본적으로 `warn`입니다. Pre-tool이 구체적인 저장소 내부 경로 집합, active task,
  현재 쓰기 준비 상태, 호환되는 프로젝트 범위를 가진 명확한 제품 파일 쓰기를 허용하면
  expected-write 상관 행을 기록합니다. 이 행은 프로젝트, 연결, 세션, 선택적 호스트
  invocation 식별 정보, 도구 종류, 정확한 경로 정책, Task/Change Unit/Write Check
  근거, 타임스탬프 메타데이터를 담습니다. 읽기 전용 명령과 불확실한 명령은
  expected-write 행을 만들지 않습니다.
- `post-tool`은 관찰된 도구 결과를 기록합니다. Event가 변경된 `Product Repository`
  경로를 제공하면 먼저 같은 프로젝트, 연결, 세션, 제한된 시간 창, 정확한 경로 정책의
  이전 expected-write 행과 맞춰 봅니다. 호스트가 invocation 식별 정보를 제공하면 그
  식별 정보를 사용합니다. 매칭된 범위 안 쓰기는 미해결 unrecorded-change 행을 만들지
  않습니다. 매칭되지 않았거나, 범위 밖이거나, 모호한 관찰된 Product Repository 변경은
  미해결 unrecorded-change 행을 기록하고 `warn`을 반환합니다. Post-tool 관찰과 매칭은
  guarded-operation 기록이지 제품 정확성 증명이 아닙니다. 변경을 찾기 위해 신뢰할 수
  없는 명령을 실행하지 않습니다.
- `prompt-capture`는 현재 host, project, connection의 prompt-capture 사용 가능
  상태가 `configured`, `observed`, `active`일 때만 prompt-capture 메타데이터를
  기록하고 엄격한 chat judgment 명령을 인식합니다. prompt에는
  `Volicord: answer J-3 1 #AB7K`, `Volicord: answer J-3 reject #AB7K`,
  `Volicord: answer J-3 defer #AB7K`, `Volicord: note J-3 "text" #AB7K` 같은 명시적
  줄이 있어야 합니다. 지원되지 않거나, 설정되지 않았거나, 다시 읽어야 하거나,
  저하된 prompt capture는 `prompt_capture_unsupported`,
  `prompt_capture_not_configured`, `prompt_capture_reload_required` 같은 구조화된
  비기록 출력을 하나의 다음 행동과 함께 반환합니다. 명령이 아닌 prompt는 prompt
  capture를 사용할 수 있을 때만 정상적으로 진행됩니다. 형식이 잘못되었거나,
  모호하거나, 알 수 없거나, 코드가 없거나, 코드가 틀렸거나, 오래되었거나, 이미
  답했거나, 프로젝트나 연결이 맞지 않는 판단 명령은 판단을 기록하지 않고 `deny`를
  반환합니다. 유효한 명령은 로컬 `User Channel`을 통해 지정된 대기 판단을
  `actor_source=local_user`와
  `resolved_verification_basis=user_prompt_submit_hook`으로
  기록하고, prompt-capture 저장소에는 전체 prompt text를 생략하며, 그 명령을 일반
  agent 지시로 다루지 않고 모델에 보이는 기록 완료 맥락을 반환합니다.
- `stop`은 active task를 완료로 다뤄도 되는지 점검합니다. 닫기 준비 상태 blocker가
  남아 있거나, 사용자 소유 판단이 대기 중이거나, 미해결 unrecorded change가 남아
  있으면 `deny`를 반환하고, 그렇지 않으면 `allow`를 반환합니다.

## 변경 조정 명령

`volicord changes reconcile [--repo PATH] [--task active|ID] [--json]`는 unresolved guarded 미기록 Product Repository 변경 찾기를 위한 로컬 복구 명령입니다.

이 명령은 `--repo PATH` 또는 현재 작업 디렉터리에서 선택 프로젝트를 해석하고 기본적으로 active `Task`를 선택합니다. `actor_source=local_user`, `operation_category=local_recovery`로 공개 `volicord.reconcile_changes` Core 메서드를 호출하고, 해결된 찾기 수, 대기 사용자 판단 수, 남은 미해결 찾기 수를 출력하며 일반 CLI 종료 코드 모델을 따릅니다. 거절된 Core 응답은 성공한 조정 요약으로 바꾸지 않고 거절된 CLI 결과로 유지합니다.

이 명령은 결정적 찾기를 해결하거나 대기 사용자 소유 판단을 만들 수 있습니다. 사용자 답변을 기록하지 않고, 사용자를 대신해 변경을 수락하지 않으며, 정확성, 리뷰나 테스트 충분성, 닫기 준비 완료를 증명하지 않습니다. 대기 판단이 만들어졌다면 사용자는 기존 `User Channel` 경로로 판단을 기록한 뒤 `volicord changes reconcile`을 다시 실행합니다.

## User Channel 명령

<a id="user-channel-commands"></a>
<a id="user-interaction-commands"></a>

`volicord user` 명령은 사람이 로컬 CLI에서 `User Channel`을 통해 작업 상태를
확인하고 대기 중인 사용자 판단에 답할 수 있는 경로를 제공합니다. 이 명령은 Agent
Connection을 만들거나, MCP 호스트 설정을 설치하거나, Agent Connection이 사용자처럼
동작할 수 있게 하지 않습니다.

초기화된 MCP 클라이언트가 elicitation 지원을 선언하면 MCP elicitation은
`volicord.request_user_judgment`로 만들어진 대기 판단의 선호 대화형 경로입니다.
elicitation을 사용할 수 없고 prompt-capture 사용 가능 상태가 `configured`, `observed`,
`active`이면 fallback 안내가 현재 검증 코드가 포함된
`Volicord: answer J-3 1 #AB7K` 같은 정확한 채팅 명령을 보여 줄 수 있습니다.
elicitation과 prompt capture를 모두 사용할 수 없고 adapter가 local web consent를 안전하게
노출할 수 있으면 fallback 안내가 짧게 만료되는 일회성 token을 쓰는 loopback consent
URL을 보여 줄 수 있습니다. 터미널의 `volicord user` 명령은 elicitation, prompt capture,
local web consent를 사용할 수 없거나, 비활성화, 저하, 또는 작업 흐름에 부적합할 때 쓰는
로컬 복구와 수동 점검 경로로 남습니다.

프로젝트 선택은 `--repo PATH` 또는 현재 작업 디렉터리의 저장소 루트를 사용합니다.
작업 선택은 기본적으로 active 작업을 사용합니다. `--task active`는 이를 명시하고,
`--task ID`는 이름 붙은 작업을 선택합니다.

일반 텍스트 모드 판단 흐름은 `volicord user judgments`와
`volicord user judgment show`가 출력하는 번호 인덱스를 사용합니다. 저장된 판단
식별자와 선택지 식별자는 참조와 JSON 세부사항입니다.

명령:

- `volicord user status`는 `actor_source=local_user`, `operation_category=read`,
  User Channel 출처로 `volicord.status`를 통해 사용자 중심 작업 상태를 보여 줍니다.
- `volicord user judgments`는 선택된 작업의 대기 판단을 나열하고 현재 출력에 안정적인
  표시 인덱스를 제공합니다.
- `volicord user judgment show INDEX_OR_ID`는 대기 중이거나 과거의 판단 하나, 맥락
  요약, Core 생성 선택지를 표시합니다.
- `volicord user judgment answer INDEX_OR_ID OPTION_INDEX_OR_ID`는 `actor_source=local_user`,
  `operation_category=user_only`, 호환 User Channel 출처, 선택된 선택지의 저장된 기계
  동작과 결과로 `volicord.record_user_judgment`를 통해 Core 생성 선택지 하나를
  기록합니다. `--note`는 메모로만 저장됩니다.

판단 하나를 기록하는 것은 그 판단만 기록합니다. 최종 수락과 잔여 위험 수락은 별개의
판단 종류와 동작으로 남아야 하며, 이 명령이 둘을 하나로 합치면 안 됩니다.

상태, 판단 목록, show 출력은 사용자의 다음 행동을 위해 선택된 담당 상태를 보여
줍니다. 이 출력은 증거, 최종 수락, 잔여 위험 수락, 닫기 준비 상태를 만들지 않습니다.
`volicord user judgment answer`만 대기 중인 해당 판단을 변경하며, 그것도 선택된 Core
생성 선택지를 통해서만 변경합니다.

<a id="dry-run"></a>
## Dry-run과 JSON 출력

`--dry-run`은 영속 변경 없이 계획, 검증, 충돌 감지, 호스트 대상 렌더링, 출력 형태
만들기를 수행합니다.

Dry-run이 하지 않는 것:

- `Volicord Runtime Home` 생성
- SQLite 데이터베이스 생성 또는 수정
- SQLite WAL 또는 SHM 파일 생성
- registry 또는 프로젝트 상태 마이그레이션 적용
- 프로젝트, Agent Connection, Connection Projects, 설치 프로필 행, 검증 상태 행
  또는 guard 설치 행 등록 또는 갱신
- 호스트 설정 파일 생성, 수정, 제거
- `Product Repository` 파일이나 디렉터리 생성, 수정, 제거
- generic export 파일 생성, 수정, 제거
- MCP 시작 점검, MCP 초기화, 도구 탐색 호출

Text 출력은 사람이 읽을 수 있어야 하며 각 리소스 작업을 `created`, `reused`,
`updated`, `removed`, `skipped`, `conflict`, `planned` 중 하나로 식별해야 합니다.

<a id="setup-output"></a>
JSON 출력은 관리 CLI 출력이지 공개 Volicord API 응답 스키마가 아닙니다. setup, 연결,
export, 프로젝트, User Channel 상태를 보고하는 명령은 비대화식 운영자가 성공한
setup과 필요한 사용자 동작을 구분할 수 있을 만큼 구조화된 상태를 포함해야 합니다.

필수 진단 JSON 값:

- `status`: `complete`, `action_required`, `failed`, `not_verified`, 또는 `dry_run`
- `checks[]`: 안정적인 점검 ID, 상태, 요약, 선택 세부사항이 있는 순서 있는 진단 점검
- `actions[]`: 필요하거나 제안되는 사용자 동작. 사용할 수 있을 때 안정적인 동작 ID와
  사람이 읽을 수 있는 명령 또는 안내를 포함합니다.
- guard를 인식하는 setup, doctor, 연결 상태, 연결 검증 JSON은 guard 진단을 보고하는
  곳에서 `guard_strength`와 `pre_tool_blocking_available`,
  `post_tool_correlation_available`, `bypass_detection_active`,
  `prompt_capture_available`, `local_web_consent_available`,
  `managed_distribution_verified`를 노출해야 합니다.

setup과 doctor JSON은 진단 소비자가 setup 동작 상태와 설치 프로필 상태를 구분할 수
있도록 `status_meaning`을 포함해야 합니다. doctor JSON은 최상위 상태가
`complete`로 남는 경고 전용 후속 동작을 `actions_recommended[]`에, 차단하는 로컬
복구 동작을 `actions_required[]`에 구분해야 합니다.

<a id="noninteractive-approval-behavior"></a>
## 비대화식 동작

비대화식 명령은 누락된 사용자 입력이나 호스트 통제 동작을 위해 프롬프트를 표시하면
안 됩니다. 누락 상태는 일반 결과 모델로 보고해야 합니다. 복구 가능한 사용자 또는
호스트 동작은 `action_required`, 사용법 오류는 종료 코드 `2`, 충돌이나 런타임
실패는 종료 코드 `1`로 보고합니다.

규칙:

- shared 의도 `Product Repository` 쓰기는 명시적 `--shared` 명령 경로로 승인되며, 그
  명령이 미리 보여 주는 관리 통합 파일로 제한됩니다.
- 기존 비관리 내용은 충돌입니다. CLI는 관련 없는 호스트 설정이나 제품 파일을 조용히
  교체하면 안 됩니다.
- 포괄적 셸 승인, 쓰기 승인, 호스트 신뢰 결정, 민감 동작 승인, `Write Check`는 이
  관리 계약이 요구하는 명시적 CLI 명령 경로를 대신하지 않습니다.
- 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, restart, reload 동작은 계속
  사용자 통제 호스트 동작이며 CLI가 대신 제공할 수 없습니다.

## 관리 경계

관리 CLI는 로컬 리소스를 초기화, 등록, 연결, 내보내기, 진단할 수 있습니다. 그 자체로
공개 Volicord API 메서드를 만들지 않으며 Core 권한, Write Check 호환성, 증거 충분성,
닫기 준비 상태, 사용자 소유 판단, 수락, 잔여 위험 수락, 아티팩트 권한, 보안 보장을
만들지 않습니다.

담당 문서 경로:

- 공개 메서드 목록과 메서드 경로: [API 메서드](api/methods.md).
- 공통 요청/응답 스키마: [API 코어 스키마](api/schema-core.md).
- Agent Connection, Connection Projects, 행위자 맥락 의미:
  [Agent Connection](agent-connection.md).
- MCP 프로세스 동작: [MCP 전송](mcp-transport.md).
- 런타임 위치와 저장소 쓰기 경계: [런타임 경계](runtime-boundaries.md).
