# 에이전트 호스트 설정

이 가이드는 Codex, Claude Code, 일반 MCP 호스트를 Volicord에 연결할 때
사용합니다. 일반적인 첫 실행 경로는 `volicord init`, 호스트, Product Repository,
그리고 호스트 capability에 맞는 통합 프로필에서 시작하며, 내부 호스트와 registry 값은
Volicord가 관리합니다.

정확한 CLI 동작은 [관리 CLI 참조](../reference/admin-cli.md)를 보세요.
Agent Connection 의미는 [Agent Connection 참조](../reference/agent-connection.md),
런타임/파일 경계는 [런타임 경계](../reference/runtime-boundaries.md)에 있습니다.

## 설정 순서

먼저 [설치](../user-guide/installation.md)에 따라 `volicord`를 설치한 뒤 호스트
설정 순서를 실행합니다.

```sh
volicord init --host codex --repo "<repo>" --profile record
```

`<repo>`는 에이전트에게 작업을 요청할 Product Repository 경로입니다.
`volicord init`은 필요하면 Runtime Home과 설치 프로필을 만들거나
재사용하고, 해당 저장소 프로젝트를 등록하거나 재사용하며, 저장소 디렉터리에서 보이는
프로젝트 이름을 파생하고, 선택한 호스트의 프로젝트 범위 MCP 설정을 설치하고,
프로젝트 범위 Volicord 지침과 로컬 설정 파일을 쓰고, 통합 상태를 기록하며, 내부
registry 식별 정보를 선택된 `Volicord Runtime Home`에 저장합니다. 생성된 호스트
설정은 `volicord mcp --stdio`를 시작합니다. `--profile record`는 host lifecycle hook
설치나 session watcher를 요구하지 않습니다.

init 뒤에는 터미널 밖에서 호스트가 소유한 후속 동작을 완료합니다.

- Codex 프로젝트 범위 설정에서는 Product Repository에서 Codex를 열거나 restart 또는
  reload하고, Codex가 묻는 경우 프로젝트 설정을 trust 또는 approve합니다.
- Claude Code 프로젝트 범위 설정에서는 Product Repository에서 Claude Code를 열거나
  restart 또는 reload하고, Claude Code가 묻는 경우 프로젝트 MCP 항목, workspace, 또는
  프로젝트 설정을 approve합니다.

저장소 로컬 설정을 썼다는 사실은 이미 실행 중인 호스트가 그 설정을 로드, 신뢰,
승인했다는 증명이 아닙니다. Init은 `.codex/config.toml` 또는 `.mcp.json`,
`.volicord/policy.json`, 관리 `AGENTS.md` 안내를 쓸 수 있지만, reload, restart, trust,
approval은 여전히 호스트가 통제합니다. 로컬 Volicord 상태는 이런 Product Repository 파일과
별도로 Runtime Home에 저장됩니다. CLI MCP preflight 또는 handshake 성공은 Volicord의 MCP
서버가 터미널 쪽 점검 경로에서 시작하고 응답할 수 있다는 뜻입니다. 그 자체만으로 Codex,
Claude Code 또는 다른 호스트가 프로젝트 설정을 로드, 신뢰, 승인했다는 증명은 아닙니다.

공유 저장소 설정에서는 Product Repository의 설정 파일 정책에 따라 저장소 로컬 파일을
commit할지 결정합니다. 다른 기여자나 자동화가 같은 Volicord/Codex 설정을 공유해야 하면
`.codex/config.toml`, `.volicord/policy.json`, 관리 `AGENTS.md` 안내 블록을 commit합니다.
사용자별 설정으로 남겨야 하면 로컬 설정 파일로 둡니다.

### Codex 호스트 검증 개념

Codex 검증은 서로 관련된 개념을 분리해서 보고합니다. `Codex host process`는
Volicord MCP 서버를 시작할 것으로 기대되는 프로세스입니다. 여기에는 Codex CLI/TUI
session, Codex IDE extension session, 비대화식 Codex run, 그 밖의 Codex 호스트 환경이
포함될 수 있습니다.

| 개념 | 의미 | 증명하지 않는 것 |
|---|---|---|
| MCP 설정 일치 | 프로젝트 범위 Codex 설정이 선택된 연결의 Volicord 관리 MCP server 항목과 일치합니다. Codex 도구 승인 정책 overlay가 있어도 이 일치는 바뀌지 않을 수 있습니다. | `Codex host process`가 그 항목을 로드, 신뢰, 승인, 시작했다는 뜻은 아닙니다. |
| Codex 도구 승인 정책 overlay | Codex는 관리 `volicord` 서버 항목 아래에 `tools.<tool>.approval_mode` 하위 table을 추가할 수 있습니다. 이는 Codex 소유 `host policy overlay`이며 보존해야 합니다. | Overlay는 Volicord 설정 drift가 아니며, 호스트 trust, 활성 도구 노출, 실행 중인 session의 승인, 정확성, 테스트 충분성, 사람 검토 완료, sandboxing, 행위자 identity를 증명하지 않습니다. |
| CLI MCP preflight 또는 handshake 통과 | `volicord connection verify`가 터미널 쪽 점검 환경에서 MCP 서버를 직접 시작했고 서버가 응답했습니다. | 활성 Codex session이 같은 서버를 시작했거나 Volicord 도구를 노출했다는 뜻은 아닙니다. |
| MCP 저장소 capability | `volicord mcp --check`는 하나의 프로세스 바인딩에 대해 registry 읽기, project-state 읽기, project-state 쓰기, 시작 관찰, effective tool mode를 보고할 수 있습니다. | 시작 점검은 활성 Codex session이 도구를 노출했다거나 변경 도구를 사용할 수 있다는 증명이 아닙니다. |
| Codex 프로젝트 trust | Codex 사용자 설정이 저장소를 `trusted`, `untrusted`, `unknown`, 또는 그 밖의 미확인 상태로 보고합니다. | `trusted` 항목만으로 실행 중인 Codex 호스트 프로세스가 프로젝트 설정을 reload했다는 증명이 되지 않습니다. |
| 관리 Codex MCP 시작 | Volicord가 이 연결에 대해 관리 Codex 시작 출처로 프로젝트에 묶인 Codex host process가 Volicord MCP 서버를 시작한 것을 관찰했습니다. | CLI preflight, 직접 handshake, 수동 probe는 관리 Codex 시작이 아닙니다. |
| 관리 Codex `tools/list` | Volicord가 이 연결에 대해 관리 Codex host process의 `tools/list` lifecycle event를 관찰했습니다. | 관리 `tools/list` 관찰만으로 활성 session 도구 노출이나 도구 호출 가능성을 증명하지 않습니다. |
| 관리 Codex 도구 호출 | Volicord가 이 연결에 대해 관리 Codex host process가 Volicord 도구를 호출한 것을 관찰했습니다. | 파일 쓰기, 사용자 승인, 정확성, 테스트 충분성, 사람 검토 완료를 증명하지 않습니다. |
| 활성 Codex session의 Volicord 도구 노출 | 활성 Codex session이 호스트의 활성 도구 목록이나 도구 검색 같은 경로에서 선택된 모드의 Volicord MCP 도구를 볼 수 있습니다. | 이후 모델의 도구 선택, 파일 쓰기, 사용자 승인, 정확성, 테스트 충분성, 사람 검토 완료를 증명하지 않습니다. |
| Codex 도구 snapshot 또는 listing 문제 | Codex MCP startup/tool-list log에는 서버 항목이 알려졌거나 시작이 완료되었다고 나오지만 활성 Codex session에는 캐시되었거나 나열된 `volicord.*` 도구가 없습니다. | CLI preflight, 프로젝트 trust, 관리 시작 관찰, 또는 `startup_complete` log만으로 활성 session 도구 등록이 증명되지는 않습니다. |
| 수동 또는 권한 상승 MCP probe | 수동으로 시작했거나 권한을 높인 `volicord mcp --stdio` 프로세스가 그 환경에서 초기화되고 도구를 나열할 수 있습니다. | 활성 Codex session이 그 도구를 등록하거나 노출했다는 뜻은 아닙니다. |
| 호스트 MCP 명령 launch 가능성 | MCP 명령이 MCP 서버를 시작하는 환경에서 실행 가능해야 합니다. `volicord`처럼 `PATH`로 찾는 명령은 `Codex host process`가 보는 PATH에 있어야 합니다. | 로컬 터미널 PATH 점검은 그 터미널 환경만 증명하며 IDE, 비대화식, 원격, executor-backed 호스트 환경을 증명하지 않습니다. |

생성된 프로젝트 범위 Codex 설정에는 관리 시작 출처 환경 변수 마커인
`VOLICORD_MCP_LAUNCH=managed_host`, `VOLICORD_MCP_HOST=codex`,
`VOLICORD_MCP_CONNECTION_ID=<connection_id>`,
`VOLICORD_MCP_PROJECT_ID=<project_id>`가 있어야 합니다. 명령과 인자만 있고 이 마커가
없다면 관리 설정 일치로 보기 전에 Volicord 관리 설정을 다시 생성합니다.

Codex에서 Volicord가 관리하는 `managed identity`는 `volicord` 서버 이름, 생성 command,
선택된 연결과 프로젝트 ID를 담은 args, Volicord 관리 환경 변수 마커입니다. Codex는 같은
서버 항목 아래에 host policy overlay를 추가할 수 있습니다. 예시는 아래와 같습니다.

```toml
[mcp_servers.volicord.tools."volicord.intake"]
approval_mode = "approve"
```

Codex가 소유한 이 승인 하위 table은 보존합니다. command, args, Volicord 관리 마커가
계속 일치하면 승인 overlay만 있어도 관리 설정입니다. Volicord 관리 마커가 없는
`volicord` 항목은 비관리 항목이며, command, args, 관리 마커 drift는 계속 설정 drift입니다.

이 일반 호스트 프로세스 모델에서의 예시는 아래와 같습니다.

- Codex CLI/TUI: 의도한 실행 파일이 해석되는 셸에서 Codex를 시작합니다.

  ```sh
  command -v volicord
  ```

- Codex IDE extension: extension session에 보이는 PATH나 extension MCP startup log를
  확인합니다.
- 비대화식 Codex run: 시작 환경을 고친 뒤 새 run 또는 session을 시작합니다.
- 원격 또는 executor-backed MCP: 원격 executor 환경에서 명령 가용성을 확인합니다. 로컬
  CLI PATH 점검만으로는 충분하지 않습니다.

호스트 prompt가 요구한 동작을 마친 뒤 터미널 쪽 후속 점검을 실행합니다.

```sh
volicord connection verify codex --shared --repo "<repo>"
```

Claude Code에는 `codex` 대신 `claude-code`를 사용합니다.

설치 프로필이 준비된 뒤 personal, global, read-only 동작을 직접 선택하는 등 낮은
수준의 연결 변형이 필요할 때는 `volicord connection add`를 사용합니다. 프로세스 현재
디렉터리가 대상 Product Repository가 아닐 때만 `--repo PATH`를 사용합니다.

```sh
volicord connection add codex --repo "<repo>"
```

## 통합 프로필

Detective 상태는 선택된 연결 또는 session에 대해 선택된 프로필과 관찰 요약을
보고합니다.

| 프로필 | 도달 조건 | 운영상 의미 |
|---|---|---|
| Record profile(`record`) | Host hook이나 session watcher를 요구하지 않고 MCP를 통한 협력적 Volicord 워크플로 기록을 사용할 수 있습니다. | 생성된 설정 안내가 호스트를 유도할 수 있지만 강제하지는 못합니다. |
| Detective profile(`detective`) | 프로젝트 로컬 host hook에 검증된 생성 설정, cwd-independent 및 subdirectory-safe hook 명령, native host output, 필수 phase, 쓰기 matcher, 일치하는 policy hash, 런타임 관찰, session watcher 관찰이 있습니다. | 협력형 host warning 또는 denial decision 신호, post-tool 상관, 채팅 명령 캡처, detective 상태, 미기록 변경, 닫기/쓰기 차단 사유가 workflow에 참여할 수 있습니다. |

Record profile은 prepare-write workflow를 통해 Volicord 쓰기 티켓을 발급할 수 있습니다.
OS 샌드박싱, 네트워크 격리, 악성 코드 방어, 전체 쓰기 방지, 행위자 identity 증명,
정확성 증명, 테스트 충분성 증명, 사람 검토 완료를 제공하지 않습니다. Detective profile은
쓰기 티켓을 파일시스템 집행, 코드 리뷰 승인, 최종 수락, 쓰기가 실제로 일어났다는
증명으로 바꾸지 않습니다. 대신 지원되는 hook과 watcher 관찰을 더해 나중에 티켓 범위
쓰기 및 미기록 변경과 연결할 수 있습니다.

관찰 요약은 host hook과 session watcher가 활성인지, 협력형 pre-tool warning이나
denial이 사용 가능한지, 미기록 변경을 탐지할 수 있는지, 행위자 identity를 증명할 수 있는지,
OS 집행이 제공되는지를 보고합니다. 현재 Volicord 출력은 행위자 identity 증명과 OS 집행을
제공하지 않는다고 보고합니다.

## Detective 수명주기

Detective 프로필에서는 설정과 활성화가 분리됩니다. `volicord init`은 MCP 호스트 설정,
Volicord 관리 `AGENTS.md` 안내, `.volicord/policy.json`, 호스트 hook 또는 rule 파일,
host-hook 관찰을 위한 detective 설치 상태를 설치하거나 갱신합니다. 그래도 그 파일이
실행되려면 호스트 reload, restart, trust, 프로젝트 MCP 승인, 또는 다른 호스트 소유
동작이 필요할 수 있습니다.

현재 검증된 detective adapter는 호스트별로 다릅니다.

- Codex detective 설정은 프로젝트 MCP 설정, `.codex/hooks/` 아래의 Volicord 관리
  POSIX `sh` wrapper script, `.codex/hooks.json`, `.codex/rules/*.rules`를 씁니다.
  pre-tool 및 post-tool matcher는 `Bash`, `apply_patch`, `Edit`, `Write`,
  `mcp__.*__(write|edit|create|update|delete|remove|move|patch).*` tool 이름을
  대상으로 합니다. 생성된 rule과 hook 파일이 실행되려면 호스트에 프로젝트 trust,
  hook trust, restart 또는 reload가 필요할 수 있습니다.
- Claude Code detective 설정은 `.mcp.json`, `.claude/hooks/` 아래의 Volicord 관리
  POSIX `sh` wrapper script, `.claude/settings.json`, `.claude/rules/*.md`를 씁니다.
  pre-tool 및 post-tool matcher는 `Bash`, `Edit`, `Write`, `MultiEdit`,
  `mcp__.*__(write|edit|create|update|delete|remove|move|patch).*` tool 이름을
  대상으로 합니다. Settings 쓰기는 관련 없는 settings를 보존하고 Volicord 관리
  항목을 병합합니다. 생성된 hook과 rule 파일이 실행되려면 호스트에 프로젝트 MCP
  approval, workspace trust, settings reload가 필요할 수 있습니다.

검증이 `hook_path_safety=ok`를 보고할 때 생성된 hook 명령은 cwd-independent이고
subdirectory-safe입니다. Codex hook 항목은 bare `.codex/hooks/...` 경로를 실행하지
않습니다. POSIX `sh` 명령으로 `git rev-parse --show-toplevel`을 실행해 Git work-tree
root를 해석한 뒤 그 root 아래의 Volicord 관리 dispatch wrapper를 `exec`합니다. Dispatch
wrapper는 phase wrapper가 존재하고 실행 가능한지 확인한 뒤 실행합니다. Claude Code hook
항목은 `${CLAUDE_PROJECT_DIR}/.claude/hooks/volicord-pre-tool.sh`처럼
`${CLAUDE_PROJECT_DIR}`를 기준으로 하는 exec-form 명령과 빈 args를 사용합니다. 생성된
명령을 bare `.codex/hooks/...` 또는 `.claude/hooks/...` 상대 경로로 바꾸면 안 됩니다.
그런 경로는 호스트 session cwd에 의존하므로 `relative_path_unsafe`로 보고됩니다.

생성된 hook 설정은 wrapper script를 `--host-output codex` 또는
`--host-output claude-code`로 호출하므로 hook stdout은 host-native JSON/context이거나
빈 출력이며 Volicord wrapper JSON이 아닙니다.

Detective init은 모든 필수 호스트 lifecycle hook phase를 설치하고 검증할 수
있어야 합니다. 선택한 Codex 또는 Claude Code 어댑터가 모든 필수 phase에 대해 신뢰할
수 있는 프로젝트 로컬 hook 스키마나 경로를 알지 못하면, init은 `AGENTS.md`나
`.volicord/policy.json`을 집행으로 취급하지 않고 실패합니다. Native Windows에서는
Windows host-hook wrapper와 session watcher 동작이 구현되고 테스트되지 않았으므로
detective init이 `DETECTIVE_WINDOWS_UNSUPPORTED`로 실패합니다. Native Windows에서는
`--profile record`를 사용합니다. Detective 전제조건을 사용할 수 없으면
`--profile record`를 사용하거나, init을 다시 실행하기 전에 지원되는 호스트, 플랫폼,
저장소 설정을 준비합니다.

```sh
volicord init --host codex --repo "<repo>" --profile record
```

`volicord connection verify`와 `volicord doctor`는 파일 상태, 필요한 호스트 동작,
관찰된 활성화, 관찰 사실을 분리해서 다룹니다. Volicord가 기록된 프로젝트,
Agent Connection, 호스트 종류, 통합 프로필, policy hash와 일치하는 host-hook event를
관찰해야 detective 설치 상태가 활성화됩니다. Hook 경로 안전성은 호스트 trust, reload,
restart, approval을 대신하지 않습니다. `AGENTS.md`는 지침 지원이며, 호스트 hook과 rule은
협력적이고 탐지적인 guardrail입니다. OS 샌드박싱, 명령 격리, 네트워크 격리, 행위자
증명, Volicord를 아는 경로 밖에서 쓰기가 일어날 수 없다는 증명이 아닙니다.

## 연결 의도

연결 의도는 호스트 설정이 어디에 속하는지 설명합니다.

| 의도 | 명령 형태 | 호스트 지원 |
|---|---|---|
| `personal` | `volicord connection add codex` 또는 `volicord connection add claude-code` | 현재 사용자를 위한 로컬 설정. |
| `shared` | `volicord connection add codex --shared` 또는 `volicord connection add claude-code --shared` | 호스트가 지원할 때 명시적 통합 파일을 통해 저장되는 프로젝트 공유 설정. |
| `global` | `volicord connection add claude-code --global` | 이를 지원하는 호스트의 사용자 전체 호스트 설정. |

`--shared`와 `--global`은 함께 사용할 수 없습니다. 둘 다 없으면 Volicord는
`personal`을 사용합니다.

## Workflow와 Read-Only 모드

기본 모드는 `workflow`입니다. Workflow 도구 대신 읽기 중심 동작을 노출해야 하는
연결에는 `--read-only`를 사용합니다.

```sh
volicord connection add codex --read-only
```

기존 연결 모드는 아래처럼 바꿉니다.

```sh
volicord connection mode codex read-only
volicord connection mode codex workflow
```

모드를 바꾼 뒤에는 호스트 reload 또는 restart가 필요할 수 있습니다.

유효한 도구 노출은 MCP 호스트 환경의 저장소 capability에도 좌우됩니다.

| 조건 | effective tool mode | 예상되는 도구 탐색 |
|---|---|---|
| `workflow` 연결이고 프로젝트 상태를 쓸 수 있음 | `workflow` | Workflow 도구와 읽기/프로젝트 탐색 도구. |
| `workflow` 연결이지만 프로젝트 상태를 읽을 수만 있고 쓸 수 없음 | `read_only_degraded` | `volicord.status`, `volicord.check_close`, `volicord.list_projects` 같은 읽기 호환 도구. |
| `read-only` 연결이고 프로젝트 상태를 읽을 수 있음 | `read_only` | 읽기 호환 도구. |
| 읽을 수 있는 허용 프로젝트 상태가 없음 | `unavailable` | 프로젝트 탐색만 보이거나 바인딩에 따라 시작이 실패합니다. |

기존 프로세스 바인딩에 대한 시작 및 저장소 진단은 아래 명령으로 확인합니다.

```sh
volicord mcp --check --connection "<connection_id>" --project "<project_id>"
```

`registry_read`, `project_state_read`, `project_state_write`,
`startup_observation`, `effective_tool_mode` 줄을 보고 쓰기 가능한 workflow 경로와
읽기 전용 degraded mode를 구분합니다. 이 필드들은 시작 진단이며 활성 호스트 도구 노출을
증명하지 않습니다. 정확한 출력 의미는 [MCP 전송](../reference/mcp-transport.md#configuration-preflight)이
담당합니다.

## 적용 전 dry-run

dry-run은 지속 변경 없이 계획을 보고합니다.

```sh
volicord connection add codex --dry-run
volicord connection add claude-code --shared --dry-run
volicord connection remove codex --dry-run
```

공유 호스트 설정을 바꾸기 전이나 제거할 연결의 호스트 대상을 먼저 확인하고 싶을
때 dry run을 사용합니다.

## 조회와 검증

```sh
volicord connection list
volicord connection status codex --shared --repo "<repo>"
volicord connection verify codex --shared --repo "<repo>"
```

기본 text 출력은 대화형 설정 작업을 위한 간결한 사람용 요약입니다. 연결 상태와
검증에서는 먼저 `Status`, `Checks`, `Next`, `Diagnostics`를 읽습니다. 자동화와 전체
진단에는 `--json`을 사용합니다. 스크립트는 간결한 text를 파싱하면 안 됩니다. 자세한
guard state, hook 진단, MCP handshake 세부사항, 호스트 관찰은 JSON 진단에 둡니다.

`volicord connection status codex --shared --repo <repo>`는 아래와
같은 간결한 형태입니다.

```text
Agent Connection status for Codex

Status:
  Connection: enabled
  Mode: workflow
  Last verification: action required

Profile:
  record

Repository:
  <repo>

Checks:
  Stored connection: enabled, mode workflow, last verification action required
  Current MCP configuration: match
  Codex project trust: trusted
  Last CLI MCP preflight: passed
  Last CLI MCP handshake: passed
  Managed Codex MCP startup: not observed
  Managed Codex tools/list: not observed
  Managed Codex tool call: not observed
  Active Codex tool exposure: unconfirmed
  Host MCP command: uses volicord from the Codex host PATH
  Host follow-up: action required

Next:
  1. Confirm Volicord tools are exposed in the active Codex session.
  2. If tools are not exposed, check managed Codex MCP startup/tool-list logs and Volicord storage read/write capability.
  3. Run:
     volicord connection verify codex --shared --repo <repo>

Limits:
  The record profile supports cooperative Volicord workflow recording through MCP.
  It does not provide OS sandboxing, network isolation, malware defense,
  full write prevention, actor identity proof, correctness proof, test
  sufficiency proof, or human review completion.

Diagnostics:
  Run:
    volicord connection status codex --shared --repo <repo> --json
```

`volicord connection verify codex --shared --repo <repo>`는 같은 섹션
모델을 사용하되 새 검증 점검을 보여 줍니다.

```text
Agent Connection checked for Codex

Status:
  Verification: action required
  Connection: enabled
  Mode: workflow

Profile:
  record

Repository:
  <repo>

Checks:
  MCP configuration: match
  Codex project trust: trusted
  CLI MCP preflight: passed
  CLI MCP handshake: passed
  Managed Codex MCP startup: not observed
  Managed Codex tools/list: not observed
  Managed Codex tool call: not observed
  Active Codex tool exposure: unconfirmed
  Host MCP command: uses volicord from the Codex host PATH
  Host follow-up: action required

Next:
  1. Confirm Volicord tools are exposed in the active Codex session.
  2. If tools are not exposed, check managed Codex MCP startup/tool-list logs and Volicord storage read/write capability.
  3. Run:
     volicord connection verify codex --shared --repo <repo>

Limits:
  The record profile supports cooperative Volicord workflow recording through MCP.
  It does not provide OS sandboxing, network isolation, malware defense,
  full write prevention, actor identity proof, correctness proof, test
  sufficiency proof, or human review completion.

Diagnostics:
  Run:
    volicord connection status codex --shared --repo <repo> --json
```

같은 호스트와 저장소에 둘 이상의 연결이 일치하면 선택할 때 쓴 의도 플래그를 함께
넣습니다.

```sh
volicord connection status codex --shared
volicord connection verify claude-code --global
```

결과 상태:

| 상태 | 설정 가이드에서의 의미 |
|---|---|
| `complete` | Volicord 쪽 상태, 관리 호스트 설정, 필요한 호스트 시작 가능성과 신뢰 게이트, CLI MCP 시작과 초기화, 그리고 Codex 검증이 적용되는 경우 활성 Codex 도구 노출 증거가 준비되었습니다. |
| `action_required` | Volicord 쪽 상태는 있지만 이름 붙은 사용자 통제 호스트 동작이 남아 있습니다. 그 자체로 치명적인 CLI 오류는 아닙니다. |
| `failed` | 필요한 로컬 전제 조건, 호스트 설정 단계, 검증 단계가 성공하지 못했습니다. |
| `dry_run` | 명령이 지속 변경 없이 계획된 동작을 보고했습니다. |

Codex에서는 프로젝트 trust가 `trusted`이고 CLI MCP preflight와 handshake가 통과했더라도
`action_required`가 나타날 수 있습니다. 이때 남은 단계는 보통 활성 session 또는 관리
host runtime 문제입니다. 먼저 활성 Codex session에 Volicord 도구가 노출되는지 확인하고,
도구가 보이지 않으면 관리 Codex MCP 시작, 관리 `tools/list`, 관리 도구 호출 증거,
Codex startup/tool-list log, Volicord storage read/write capability를 확인합니다. Host MCP
command launchability는 명령 점검이 그 위험을 보고할 때 별도의 시작 환경 진단으로
다룹니다.

## 읽기 전용 및 워크플로 간단 점검

호스트에 워크플로 상태 생성을 요청하기 전에는 읽기 전용 연결 점검을 사용합니다.

1. 선택한 호스트와 저장소에 대해 `volicord connection verify`를 실행합니다.
2. 활성 MCP 호스트 session에서 `volicord.list_projects`를 호출합니다.
3. 같은 session에서 `volicord.status`를 호출합니다.

이 경로는 설정, 활성 도구 표시, 프로젝트 선택, 읽을 수 있는 프로젝트 상태를 확인합니다.
Volicord `Task` 생성을 요구하지 않아야 합니다.

Volicord 상태를 만들어도 될 때만 워크플로 쓰기 경로 간단 점검을 사용합니다. 최소
워크플로 경로는 `volicord.intake`, `volicord.update_scope`, `volicord.record_run`, 닫기가
필요할 때 최종 수락을 위한 `volicord.request_user_judgment`, 그리고
`volicord.check_close`를 사용할 수 있습니다. 이 경로는 사용자가 지원되는 `User Channel`을
통해 최종 판단을 기록할 때까지 `Task`를 `missing_final_acceptance`로 막힌 상태에 둘 수
있습니다.

## Generic MCP 호스트 설정

Volicord가 직접 관리하지 않는 MCP 호스트에는 먼저 지원되는 Agent Connection을 만든 뒤,
외부 호스트의 자체 설정에서
`volicord mcp --stdio --connection <connection_id> [--project <project_id>]`를 시작하도록
구성합니다. 연결이 read-only여야 하면 `volicord connection mode`를 사용합니다. 외부
호스트 설정은 사용자 관리 설정으로 남습니다.

## User Channel 경계

Agent Connection은 초점이 맞춰진 판단 필요를 요청하거나 표시할 수 있습니다. 권한을
지니는 사용자 답변은 기록하지 않습니다. 표시된 선택지가 사용자의 기록된
판단이 되어야 하면 로컬 `User Channel` 명령을 사용합니다.

```sh
volicord inbox
volicord inbox answer JUDGMENT_ID --choice CHOICE_ID
```

## 제거

선택한 Product Repository를 연결에서 제거합니다.

```sh
volicord connection remove codex --dry-run
volicord connection remove codex
```

제거는 소유권과 안전 점검이 허용할 때 일치하는 관리 호스트 설정만 삭제합니다.
`Product Repository`, Runtime Home, 프로젝트 등록, 프로젝트 상태, Volicord 기록,
증거 첨부 저장소, 관련 없는 호스트 설정은 삭제하지 않습니다.

## 문제 해결 경로

| 증상 | 다음 문서 |
|---|---|
| 설치 프로필, 실행 파일, Product Repository 감지가 준비되지 않았습니다. | [설치](../user-guide/installation.md) |
| 연결이 `action_required` 또는 `failed`를 보고합니다. | [에이전트 호스트 문제 해결](agent-host-troubleshooting.md) |
| 정확한 명령 동작이 불분명합니다. | [관리 CLI 참조](../reference/admin-cli.md) |
| Runtime Home과 Product Repository 경계가 중요합니다. | [런타임 경계](../reference/runtime-boundaries.md) |
