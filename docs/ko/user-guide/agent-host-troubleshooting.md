# 에이전트 호스트 문제 해결

`volicord init`, `volicord connection add`, `volicord connection ...`이 호스트 설정 문제를
보고할 때 이 가이드를 사용합니다. 이 가이드는 Volicord가 Product Repository를 감지하고
내부 식별 정보를 관리하는 단순화된 명령 모델을 전제로 합니다.

정확한 setup, doctor, 연결 결과 상태 의미를 확인하려면
[관리 CLI 참조](../reference/admin-cli.md#runtime-home-selection)와
[연결 결과 상태](../reference/admin-cli.md#agent-connection-result-states)를 보세요.

## 변경 전에

현재 로컬 상태를 모읍니다.

```sh
volicord doctor
volicord project current
volicord connection list
```

명령을 의도한 Product Repository 밖에서 실행하고 있다면 그 저장소로 `cd`하거나,
확인하려는 project, connection, inbox 명령에 `--repo PATH`를 추가합니다.

`volicord init`과 `volicord doctor`는 서로 다른 상태 질문에 답합니다. Init은 첫 저장소
설정과 호스트 연결에 사용자 또는 호스트 동작이 아직 필요한지를 보고합니다. Doctor는
저장된 설치 프로필을 사용할 수 있는지를 보고합니다. 따라서 프로필을 사용할 수 있으면
doctor가 `complete`를 보고하면서도 이후 셸이나 에이전트 호스트를 위한 명령 가용성
경고 또는 권장 `PATH` 동작을 함께 보여 줄 수 있습니다.

`volicord init`에서는 먼저 간결한 온보딩 출력을 읽습니다. 제목, profile, repository,
repo file changes, 저장된 Runtime Home 경로를 확인한 뒤 `Next:` 체크리스트를 따릅니다.
이 체크리스트가 호스트 reload 또는 restart, 프로젝트 trust 또는 approval, 후속
`volicord connection verify ...` 명령으로 이어지는 설정 흐름입니다.

진단 명령이 모두 같은 text 형태를 쓰지는 않습니다. `volicord connection status`와
`volicord connection verify`에서는 먼저 `Status`, `Checks`, `Next`, `Diagnostics` 섹션을
읽습니다. `Status`와 `Checks`는 연결 상태와 시도된 점검을 보여 주고, `Next`는 호스트
소유 또는 로컬 후속 동작을 이름 붙이며, `Diagnostics`는 JSON 진단 명령을 가리킵니다.
다른 보기에서는 연결 형태를 가정하지 말고 명령이 출력한 결과 줄 label을 따라갑니다.

안정적인 자동화 표면이나 전체 진단 필드가 필요하면 `--json`을 사용합니다. 간결한 사람용
text는 대화형 복구를 위한 것이며 스크립트가 파싱하면 안 됩니다.

MCP 시작이나 도구 탐색이 증상이고 JSON 진단 또는 생성된 호스트 설정에서 프로세스
바인딩 값을 확인할 수 있다면 시작 저장소 capability를 직접 확인합니다.

```sh
volicord mcp --check --connection "<connection_id>" --project "<project_id>"
```

`registry_read`, `project_state_read`, `project_state_write`,
`startup_observation`, `effective_tool_mode`를 함께 읽습니다. 성공한 시작 점검은 전체
호스트 검증이 아니며 활성 호스트 도구 노출을 증명하지 않습니다.

## 설치 프로필이 없음

관찰 증상: 일반 project, connection, MCP, inbox workflow가 `SETUP_REQUIRED`를
보고하거나 선택된 `Volicord Runtime Home`에 설치 프로필이 없다고 말합니다.

제한된 복구:

`volicord`를 이미 사용할 수 있다면:

```sh
volicord init --host codex --repo "<repo>" --profile record
volicord doctor
```

`volicord`를 사용할 수 없다면 [설치](../user-guide/installation.md)의 릴리스
바이너리 경로를 다시 실행합니다. 의도적으로 개발용 소스 체크아웃에서 작업 중이라면:

```sh
cargo build --workspace --bins
./target/debug/volicord init --host codex --repo "<repo>" --profile record
```

Init이 `volicord`를 사용할 수 있게 만드는 방법, 호스트 trust, reload를
`action_required`로 보고하면 그 안내를 따릅니다. 셸 시작 파일을 직접 갱신했다면 새
셸을 열거나 에이전트 호스트를 restart 또는 reload한 뒤 다시 확인합니다.

```sh
volicord doctor
```

Runtime Home 파일을 직접 만들지 않습니다. Registry와 설치 프로필이 함께 만들어지도록
init을 사용합니다.

## 명령이 PATH에 없음

관찰 증상: init 또는 doctor가 이후 터미널이나 에이전트 호스트에서 `volicord`를
`PATH`로 사용할 수 없다고 보고합니다.

제한된 복구:

설치된 `volicord` 바이너리를 사용자가 제어하는 명령 디렉터리에 두거나 링크한 뒤, 그
디렉터리가 `PATH`에 보이도록 합니다. Volicord는 현재 부모 셸 환경을 직접 바꿀 수
없습니다. 이미 실행 중인 에이전트 호스트는 새 명령 디렉터리를 보려면 restart, reload,
resume, 또는 새 session이 필요할 수 있습니다. 나중에 호스트 안에서 연 터미널만이 아니라
호스트를 시작한 환경을 확인합니다.

## 저장소가 감지되지 않음

관찰 증상: project 또는 connection 명령이 Git 저장소 루트를 찾지 못했다고 말합니다.

제한된 복구:

```sh
cd "<repo>"
volicord project current
volicord project use
```

또는 Product Repository를 명시적으로 선택합니다.

```sh
volicord init --host codex --repo "<repo>" --profile record
```

`<repo>`는 에이전트에게 작업을 요청할 Product Repository 경로입니다. 사용자에게 보이는
프로젝트 이름은 저장소 디렉터리에서 나옵니다. 내부
프로젝트 식별 정보는 복구 입력이 아닙니다.

## Windows 경로가 거부됨

관찰 증상: native Windows 설정이 Runtime Home 또는 Product Repository 경로가 UNC 경로,
WSL UNC 경로, WSL 스타일 `/mnt/<drive>` 경로이기 때문에 유효하지 않다고 보고합니다.

제한된 복구:

- `--repo`와 명시적인 `VOLICORD_HOME` 또는 `--home` 값에는
  `C:\Users\you\product-repo` 같은 native 로컬 drive-letter 경로를 사용합니다.
- Product Repository가 WSL2 안에 있다면 그 WSL 경로를 native Windows `volicord.exe`에
  전달하지 말고 WSL2 환경 안에서 Linux Volicord 바이너리를 실행합니다.
- Native Windows 설정에서 Runtime Home 또는 Product Repository로 network share를 사용하지
  않습니다.

## 호스트를 선택할 수 없음

관찰 증상: `volicord connection add` 또는 `volicord connection ...`이 호스트를 추론하지
못하거나 호스트 값이 지원되지 않습니다.

제한된 복구: lifecycle hook 설치 없는 일반 첫 실행 설정에는 호스트, 저장소,
`record` 프로필을 init에 명시적으로 전달합니다.

```sh
volicord init --host codex --repo "<repo>" --profile record
```

Detective 설정에는 [관리 CLI
참조](../reference/admin-cli.md#agent-host-setup-and-init)의 전체 init 계약을 사용합니다.
host hook 또는 session watcher 지원이 빠져 있으면 실행 가능한 진단과 함께 실패해야
합니다. Detective 전제 조건을 사용할 수 없으면 `--profile record`를 사용합니다.

Native Windows에서는 detective 설정이 지원되지 않습니다. Init이
`DETECTIVE_WINDOWS_UNSUPPORTED`를 보고하면 record 프로필로 다시 실행합니다.

```powershell
volicord init --host codex --repo "<repo>" --profile record
```

Detective는 선택한 host hook과 session watcher 계약이 지원되고 테스트된 WSL2, Linux,
macOS에서만 사용합니다.

하위 수준 연결 복구에는 호스트와 저장소를 connect에 명시적으로 전달합니다.

```sh
volicord connection add codex --repo "<repo>"
volicord connection status codex --repo "<repo>"
```

연결에 사용한 의도 선택자도 함께 사용합니다.

```sh
volicord connection status codex --shared
volicord connection verify claude-code --global
```

Codex는 personal과 shared 연결 의도를 지원합니다. Claude Code는 personal, shared,
global 연결 의도를 지원합니다.

## `action_required`

관찰 증상: connection status 또는 verification이 text 또는 JSON 출력에서
`action_required`를 보고합니다.

제한된 복구:

```sh
volicord connection status codex --shared --repo "<repo>"
volicord connection verify codex --shared --repo "<repo>"
```

먼저 `Status`, `Checks`, `Next`, `Diagnostics` 섹션을 읽습니다. 이름 붙은 호스트 소유
또는 로컬 단계만 완료합니다. 흔한 동작에는 호스트 항목 신뢰, 프로젝트 MCP 항목 승인,
호스트 로그인, 호스트 reload, 호스트 restart, 설치 프로필 복구가 있습니다. `Next`가
`volicord connection verify ...` 명령을 보여 주면 호스트 쪽 단계를 완료한 뒤 그 명령을
실행합니다.

`action_required`를 치명적 실패로 다루지 않습니다. 오래 유지되는 Volicord 쪽 상태가
이미 있을 수 있습니다.

다른 실행 가능한 `Next` 줄은 선택된 작업 흐름 안에서 해석합니다. 출력이
`volicord inbox`를 이름 붙이면 터미널에서 대기 중인 사용자 판단을 확인하거나
답합니다. local consent URL이 없다고 하면 표시된 CLI 답변 명령이나 MCP Judgment
Inbox 항목에 이미 표시된 URL을 사용합니다. selector가 모호하거나 잘못된 저장소가
선택되었다면 `--repo PATH`와 `--shared` 또는 `--global` 같은 일치하는 intent flag를
붙여 다시 실행합니다.

## 읽기 전용 호스트 저장소

관찰 증상: MCP 호스트 환경이 Volicord 설정을 읽거나 `volicord`를 시작할 수는 있지만,
직접 sandbox 실행은 SQLite가 읽기 전용 database에 쓰려고 했다는 오류처럼 저장소 오류로
실패합니다. 권한을 높였거나 제한이 더 적은 진단 실행에서는 `initialize`, `tools/list`,
읽기 전용 status 호출이 동작할 수 있습니다.

예상 동작:

- Runtime Home registry와 프로젝트 상태를 읽을 수 있으면 MCP 시작과 `tools/list`는 읽기
  전용 상태를 견딜 수 있습니다.
- Mutation 도구는 선택된 `Volicord Runtime Home`의 프로젝트 상태를 쓸 수 있어야 합니다.
  읽기 전용 저장소에서는 `tools/list`에서 빠지거나 구조화된 `MCP_UNAVAILABLE` 거절을
  반환할 수 있습니다.
- 읽을 수 있지만 쓸 수 없는 프로젝트 상태를 가진 `workflow` 연결은 도구 탐색에서
  `read_only_degraded`로 동작합니다.
- 프로젝트 상태를 읽을 수 있으면 `volicord.status`, `volicord.check_close`,
  `volicord.list_projects` 같은 읽기 호환 도구는 계속 보일 수 있습니다.

제한된 복구:

```sh
volicord mcp --check --connection "<connection_id>" --project "<project_id>"
```

`project_state_write`가 `readonly`이고 `effective_tool_mode`가 `read_only_degraded`라면
workflow 변경 도구가 필요할 때 선택된 Runtime Home과 프로젝트 상태를 쓸 수 있도록 MCP
호스트 환경을 고칩니다. 의도한 호스트 통합이 read-only라면 연결을 read-only 모드로 두고
workflow 도구를 기대하지 않습니다. 권한을 높인 실행은 성공하지만 일반 호스트 sandbox가
실패한다면, 이를 활성 호스트 session이 같은 도구를 로드하거나 노출했다는 증명이 아니라
저장소 capability 진단으로 봅니다.

## Claude Code 설정은 있지만 도구가 노출되지 않음

관찰 증상: `volicord connection status claude-code` 또는
`volicord connection verify claude-code`가 설정 일치, connected 호스트 상태, 또는
`action_required`를 보고하지만 활성 Claude Code session에 `volicord.*` 도구가 보이지
않습니다.

이는 Volicord가 관리 Claude Code 설정이나 `claude mcp get` 상태를 검사할 수 있다는
뜻입니다. 그러나 활성 Claude Code session 도구 노출이 증명된 것은 아닙니다. 현재 Claude
Code verification만으로는 실행 중인 Claude Code session 안의 관리 lifecycle 시작, 관리
`tools/list`, 관리 도구 호출 증거, 저장소 capability가 증명되지 않습니다.

제한된 복구:

1. 호스트 설정과 approval 상태를 확인합니다.

   ```sh
   claude mcp list
   claude mcp get volicord
   ```

2. Shared 프로젝트 설정에서는 프로젝트 `.mcp.json`을 확인하고 프로젝트 서버 항목이
   approval 대기인지, rejected 상태인지 확인합니다.
3. 활성 Claude Code session에서 `/mcp`를 확인하고, Volicord 도구에 대한 permissions가
   호스트 policy에 맞게 allow, ask, deny 중 무엇인지 확인합니다.
4. Claude Code를 시작하거나 resume하기 전에 호스트 시작 환경을 확인합니다.

   ```sh
   command -v volicord
   ```

5. 프로세스 바인딩의 Volicord 시작과 저장소 capability를 확인합니다.

   ```sh
   volicord mcp --check --connection "<connection_id>" --project "<project_id>"
   ```

6. Claude Code에서 `volicord.list_projects`와 `volicord.status`부터 읽기 전용 활성 도구
   호출을 시도합니다.
7. 호스트 쪽 동작 뒤 Volicord verification을 다시 실행합니다.

   ```sh
   volicord connection verify claude-code --repo "<repo>"
   volicord connection status claude-code --repo "<repo>"
   ```

Claude Code approval 또는 permission overlay를 일반 복구 방법으로 삭제하지 않습니다.
이름 붙은 호스트 approval, pending 상태, 시작 환경, 관리 설정 불일치, 저장소 capability
문제만 고칩니다.

<a id="trusted-codex-project-but-host-runtime-is-not-observed"></a>
## Codex 프로젝트가 trusted이고 CLI handshake도 통과했지만 도구가 노출되지 않음

관찰 증상: 연결 상태 또는 검증이 아래 사실을 함께 보고합니다.

- `Codex project trust: trusted`
- `MCP configuration: match` 또는 `Current MCP configuration: match`
- `CLI MCP preflight: passed` 또는 `MCP preflight: passed`
- `CLI MCP handshake: passed` 또는 `MCP handshake: passed`
- 활성 Codex session에 `volicord.*` 도구가 노출되지 않습니다.

다른 줄은 `Managed Codex MCP startup: not observed`,
`Managed Codex MCP startup: unknown`, 또는
`Host MCP command: uses volicord from the Codex host PATH`를 보여 줄 수 있습니다.
Codex MCP startup/tool-list log에는 시작이 완료되었거나 `startup_complete` 항목이
보이지만 활성 session에 대해 캐시된 tool snapshot 또는 나열된 `volicord.*` 도구가 없을
수도 있습니다.

이는 저장소 로컬 MCP 설정이 일치하고 터미널 쪽 verification도 성공했지만, 활성 Codex
session이 Volicord 도구를 등록했다는 증명은 아니라는 뜻입니다. Codex가 MCP 서버의 존재를
알거나 서버를 시작했더라도 활성 session에는 tool snapshot 또는 tool listing이 없을 수
있습니다. Volicord는 Codex host log 없이는 Codex 내부 도구 등록을 완전히 진단할 수
없습니다.

제한된 복구:

설정을 바꾸기 전에 아래 분기를 확인합니다.

- 먼저 JSON 진단을 확인합니다.

  ```sh
  volicord connection status codex --shared --repo "<repo>" --json
  volicord connection verify codex --shared --repo "<repo>" --json
  ```

  `checks[]`, `actions[]`, `verification.project_trust`,
  `verification.host_runtime`, `verification.active_tool_exposure`,
  `verification.host_mcp_command`를 별도 사실로 읽습니다. CLI MCP handshake 성공을
  활성 session 도구 노출로 합치지 않습니다.
- 활성 Codex session이 MCP 서버를 시작하지 않았습니다. 해당 호스트 환경에서 명령 시작
  가능성을 확인한 뒤 Product Repository에서 Codex session을 restart, reload, resume 또는
  새로 시작합니다.
- Codex startup 또는 tool-list log가 server launch, `initialize`, `tools/list`, 도구 등록
  실패를 보여 줍니다. 먼저 호스트 log의 실패를 따라갑니다.
- 호스트는 `volicord`를 시작할 수 있지만 프로젝트 상태가 읽기 전용입니다.
  `volicord mcp --check` 시작 진단을 실행하고 `project_state_write`와
  `effective_tool_mode`를 확인합니다.
- CLI 쪽 preflight 또는 handshake는 성공하지만 활성 호스트 session은 여전히
  `volicord.*` 도구를 나열하지 않습니다. CLI 결과는 터미널 쪽 MCP 검증으로만 다룹니다.
- 권한을 높인 실행은 성공하지만 sandbox 실행은 실패합니다. 실제 MCP 호스트 환경에서
  Runtime Home과 project-state 쓰기 capability를 비교합니다.

먼저 활성 Codex session의 도구 검색 또는 도구 목록에서 `volicord.*` 도구를 확인합니다.
그런 다음 Codex MCP startup/tool-list log에서 서버 launch, `initialize`, `tools/list`,
캐시된 tool snapshot, 도구 등록 항목을 확인합니다. 로그가 startup complete를 보여 주지만
tool snapshot이나 나열된 `volicord.*` 도구가 없다면 Product Repository에서 Codex session을
restart, reload, resume 또는 새로 시작하고 도구 노출이 달라지는지 비교합니다.

셸에서 Codex를 시작한다면 Codex를 시작하거나 resume하기 전에 같은 셸 환경을 확인합니다.

```sh
command -v volicord
```

Codex IDE extension에서는 extension session에 보이는 PATH 또는 MCP startup log를 확인합니다.
비대화식 Codex run에서는 시작 환경을 고친 뒤 새 run을 시작합니다. 원격 또는
executor-backed MCP 시작에서는 해당 executor에서 명령 가용성을 확인합니다. 로컬 CLI
PATH만으로는 원격 명령 launch 가능성을 증명하지 않습니다.

설정 일치가 의심되면 생성된 `<repo>/.codex/config.toml` 항목을 확인합니다. Volicord가
관리하는 프로젝트 범위 Codex 항목에는 일치하는 명령과 인자와 함께
`VOLICORD_MCP_LAUNCH=managed_host`, `VOLICORD_MCP_HOST=codex`,
`VOLICORD_MCP_CONNECTION_ID=<connection_id>`,
`VOLICORD_MCP_PROJECT_ID=<project_id>` 관리 시작 마커가 있어야 합니다. 명령과 인자는
있지만 이 마커가 없다면 Volicord setup 또는 connection 관리 명령을 다시 실행해 관리
항목을 다시 생성합니다.

호스트 쪽 변경 뒤에는 터미널 쪽 verification을 다시 실행합니다.

```sh
volicord connection verify codex --shared --repo "<repo>"
```

Codex 밖에서 MCP 수명주기를 직접 확인하려면
[MCP 전송](../reference/mcp-transport.md#manual-stdio-lifecycle-probe)의 수동 또는 권한 상승
`VOLICORD_MCP_VERIFICATION=1` probe를 사용합니다. 프로세스 명령 형태는 아래와 같습니다.

```sh
VOLICORD_MCP_VERIFICATION=1 volicord mcp --stdio --connection "<connection_id>" --project "<project_id>"
```

참조 문서의 JSON-RPC 예시를 이 프로세스에 pipe합니다. 예상 차이는 아래와 같습니다.

- `initialize`와 `tools/list`는 도구 탐색을 확인하며 변경 준비 상태로 해석하지 않습니다.
- `notifications/initialized` 전 `tools/call`은 JSON-RPC Invalid Request로 실패해야
  합니다.
- 초기화 뒤 read-only status 호출은 프로젝트 상태를 읽을 수 있으면 성공할 수 있습니다.
- 읽기 전용 저장소에서 mutation 호출은 도구 탐색에 없거나 구조화된 unavailable 응답을
  반환할 수 있습니다.

이 probe는 probe를 시작한 환경에서 MCP 서버가 실행될 수 있다는 사실만 증명합니다. 활성
Codex session 도구 노출을 증명하지는 않습니다.

사용 중인 build가 smoke 또는 schema 진단을 보고한다면 그 역시 진단으로만 사용합니다.
예를 들어 `tools_list_schema_validation: passed`는 유효 모드의 MCP 표시 도구 스키마가
Volicord 쪽에서 유효하다는 뜻이며, 활성 Codex session이 그 도구를 등록했다는 증명은
아닙니다.

고급 진단: 사용하는 Codex 호스트 설정 형식이 MCP 서버의 `required = true`를 지원한다면
진단 실행에 사용해 그 호스트에서 MCP 시작 실패를 더 잘 보이게 할 수 있습니다. 하지만 서버를
사용할 수 없을 때 session 시작이나 resume을 막을 수도 있습니다. `required = true`를
Volicord `record` 프로필의 기본 동작이나 도구 노출의 증명으로 다루지 않습니다.

<a id="codex-approval-overlay-reported-as-mcp-configuration-changed"></a>
## 도구 승인 뒤 connection verify가 MCP 설정 변경을 보고함

관찰 증상: Codex에서 하나 이상의 Volicord 도구를 승인한 뒤 연결 상태 또는 검증이
`MCP configuration: changed`, `Current MCP configuration: changed`, 또는
`mcp_config_changed` 다음 동작을 보고합니다.

먼저 생성된 Codex 프로젝트 설정을 확인합니다.

```sh
volicord connection status codex --shared --repo "<repo>" --json
```

그다음 `<repo>/.codex/config.toml`을 확인합니다. Codex 승인 overlay 형태는 아래와
같습니다.

```toml
[mcp_servers.volicord.tools."volicord.intake"]
approval_mode = "approve"
```

제한된 복구:

1. 차이가 `[mcp_servers.volicord.tools."<tool>"]` table 하나 이상과 `approval_mode`뿐이면
   그 항목을 보존하고 verification을 다시 실행합니다.
2. Overlay만 있는 설정이 여전히 changed로 보고되면 Codex 도구 승인 정책 overlay를
   허용하는 Volicord build를 사용한 뒤 verification을 다시 실행합니다.
3. command, args, Volicord 관리 환경 변수 마커가 바뀌었다면 관리 항목을 복구합니다.

   ```sh
   volicord init --host codex --repo "<repo>"
   ```

4. `volicord` 서버 항목에 Volicord 관리 마커가 없다면 비관리 호스트 설정으로 다룹니다.
   무작정 덮어쓰거나 삭제하지 않습니다. 그 사용자 관리 항목을 유지할지, 명시적인
   `volicord init --host codex --repo "<repo>"` 설정 경로로 교체할지 결정합니다.

Overlay만 있는 승인 정책은 Codex 소유 `host policy overlay`입니다. 승인 하위 table을
삭제해야 하는 일반 이유가 아니며, 활성 Codex session이 Volicord 도구를 로드하거나
노출했다는 증명도 아닙니다.

## `failed`

관찰 증상: setup, connect, export, verification이 `failed`를 보고하거나 런타임 오류로
종료합니다.

제한된 복구:

설치 프로필을 확인합니다.

```sh
volicord doctor
```

그다음 계속 진행합니다.

1. 이 명령이 이름 붙인 첫 실패 setup 또는 실행 파일 점검을 고칩니다.
2. 원래 명령이 지원한다면 `--dry-run`으로 다시 실행합니다.
3. Dry-run 계획이 기대한 호스트와 Product Repository를 이름 붙인 뒤에만 실제 명령을 다시
   실행합니다.

정확한 실패 문구를 사용해 다음 동작을 고릅니다. 참조 문서나 인간 운영자가 의도한
복구라고 식별하지 않은 한 Runtime Home 상태나 호스트 설정을 직접 삭제하지 않습니다.

## MCP 명령을 사용할 수 없음

관찰 증상: init 또는 verification이 `volicord mcp --stdio`를 찾거나 시작하거나 초기화할 수
없다고 보고합니다.

제한된 복구:

설치된 릴리스 바이너리로 init을 다시 실행합니다.

```sh
volicord init --host codex --repo "<repo>" --profile record
```

의도적으로 개발용 소스 체크아웃에서 작업 중이라면:

```sh
cargo build --workspace --bins
./target/debug/volicord init --host codex --repo "<repo>" --profile record
```

`action_required`가 이름 붙인 명령 가용성 또는 호스트 단계를 완료한 뒤 설치와
연결을 다시 확인합니다.

```sh
volicord doctor
volicord connection verify codex --shared --repo "<repo>"
```

Init은 관리 호스트 설정이 사용할 MCP 명령을 기록합니다. 일반 `connection add` 명령은
사용자가 MCP 명령 경로를 전달하도록 요구하지 않습니다. 실행 파일이 sibling 조회나
`PATH`로 찾을 수 없는 위치에 설치되어 있다면 `--mcp-command PATH`로 init을 다시
실행합니다.

<a id="guard-hook-path-or-wrapper-is-unsafe"></a>
## Hook 경로 또는 wrapper가 안전하지 않음

관찰 증상: `volicord doctor`, 연결 status, 연결 verification이 `hook_path_safety`를
`ok`가 아닌 값으로 보고합니다. 예를 들면 `relative_path_unsafe`,
`wrapper_missing`, `wrapper_not_executable`, `absolute_path_stale`,
`host_output_mismatch`, `policy_hash_mismatch`입니다.

제한된 복구:

```sh
volicord doctor
volicord connection status codex --shared --repo "<repo>"
volicord init --host codex --repo "<repo>"
volicord connection verify codex --shared --repo "<repo>"
```

영향받은 연결과 같은 호스트와 의도 선택자를 사용합니다. Claude Code에서는 `codex`를
`claude-code`로 바꾸고, 선택된 연결이 그렇다면 `--global` 또는 `--shared`를 함께
넣습니다.

진단 의미와 복구:

- `relative_path_unsafe`: 호스트 hook 설정이 호스트 session cwd에 의존하는 bare
  `.codex/hooks/...`, `./.codex/hooks/...`, `.claude/hooks/...`, 또는
  `./.claude/hooks/...` 경로를 사용합니다. Hook 명령을 손으로 고치지 말고
  `volicord init --host HOST --repo PATH`를 다시 실행합니다.
- `wrapper_missing` 또는 `dispatch_missing`: 생성된 wrapper 또는 Codex dispatch wrapper가
  없습니다. 선택된 Product Repository에 대해 init을 다시 실행합니다.
- `wrapper_not_executable`: 생성된 wrapper는 있지만 지원되는 Unix 계열 플랫폼에서 실행
  가능하지 않습니다. Init을 다시 실행해 관리 wrapper와 실행 비트를 복구합니다.
- `absolute_path_stale`: 생성된 명령이 이전 프로젝트 root를 가리킵니다. Product
  Repository를 옮긴 뒤 흔히 발생합니다. 현재 `--repo PATH`로 init을 다시 실행하고,
  필요하면 호스트를 reload 또는 restart합니다.
- `host_output_mismatch`, `policy_hash_mismatch`, `authority_mismatch`: 생성된 wrapper
  메타데이터가 기대하는 host-output mode, policy hash, 연결, detective 설치와 맞지 않습니다.
  관리 파일과 registry 상태가 일치하도록 init을 다시 실행합니다.
- `metadata_missing` 또는 `placeholder_unsupported`: 생성된 설정이 현재 검증되는 형태가
  아닙니다. Init을 다시 실행하고 생성 명령을 지원되지 않는 placeholder로 바꾸지
  않습니다.

Codex detective host hook 명령에는 선택된 Product Repository가 Git work tree여야 합니다. Wrapper
stderr가 Git root를 해석할 수 없다고 말하거나, 호스트 session이 하위 디렉터리에서
시작할 때만 hook이 실패한다면, session이 의도한 Git work tree 안에 있고 호스트
프로세스가 `git`을 사용할 수 있는지 확인한 뒤 그 저장소에 대해 init을 다시 실행합니다.
Claude Code detective host hook 명령은 `${CLAUDE_PROJECT_DIR}`를 기준으로 합니다. 호스트가 그
프로젝트 디렉터리를 제공하지 않는다면 호스트 자체의 trust와 project-selection 흐름으로
호스트 설정을 reload하거나 복구합니다.

안전하지 않은 hook 경로는 detective host hook을 inactive로 유지합니다. Watcher 사용 가능
여부는 관찰 요약에서 별도로 보고됩니다. 경로 복구는 여전히 호스트 trust,
approval, restart, reload와 별개입니다. 보고된 호스트 소유 동작을 완료하고 복구 뒤
verification을 다시 실행합니다.

## Shared 연결에 호스트 승인이 필요함

관찰 증상: shared 연결이 프로젝트 통합 파일을 쓰거나 갱신했지만 호스트가 여전히
Volicord 도구를 로드하지 않습니다.

제한된 복구:

```sh
volicord connection status codex --shared
volicord connection verify codex --shared
```

명령이 이름 붙인 호스트 소유 프로젝트 승인 또는 reload 동작을 완료합니다.
`Product Repository` 통합 파일은 Volicord 권한이 아니며, 호스트가 MCP 서버를 로드,
신뢰, 승인, 노출했다는 증거도 아닙니다.

## Generic 호스트에 Volicord 도구가 보이지 않음

관찰 증상: 사용자 관리 설정을 사용하는 외부 MCP 호스트에 Volicord 도구가 보이지 않습니다.

제한된 복구:

```sh
volicord doctor
volicord connection status codex --repo "<repo>"
```

그다음 외부 호스트의 자체 설정 절차를 확인합니다. 해당 항목은 기존 Agent Connection에
대해 `volicord mcp --stdio --connection <connection_id> [--project <project_id>]`를
시작해야 합니다. 외부 설정은 사용자 관리 설정입니다.

## 제거가 일부만 완료됨

관찰 증상: `volicord connection remove ...`가 호스트 설정을 제거하지 못했다고
보고하거나, 다른 Product Repository에 대한 연결이 계속 보입니다.

제한된 복구:

```sh
volicord connection remove codex --dry-run
volicord connection status codex
volicord connection list
```

제거는 먼저 선택된 Product Repository 멤버십을 제거합니다. 소유 멤버십이 남지 않고 안전 점검이
허용할 때만 Agent Connection과 관리 호스트 설정을 제거합니다. `Product Repository`,
프로젝트 상태, Volicord 기록, 증거 첨부 저장소, 관련 없는 호스트 항목을 제거하면 안
됩니다.

## 보안 한계

Volicord setup과 verification은 로컬 진단입니다. 외부 호스트가 안전하다거나, 모델이
Volicord 도구를 사용할 것이라거나, 파일 쓰기가 안전하다는 증명이 아닙니다. 정확한
보안 표현은 [보안](../reference/security.md)을 사용합니다.
