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
volicord init --host codex --repo /path/to/your-product-repo --profile record
volicord connection status codex --repo /path/to/your-product-repo
```

`/path/to/your-product-repo`는 에이전트에게 작업을 요청할 Product Repository의 경로
예시입니다. `volicord init`은 필요하면 Runtime Home과 설치 프로필을 만들거나
재사용하고, 해당 저장소 프로젝트를 등록하거나 재사용하며, 저장소 디렉터리에서 보이는
프로젝트 이름을 파생하고, 선택한 호스트의 프로젝트 범위 MCP 설정을 설치하고,
프로젝트 범위 Volicord 지침과 로컬 설정 파일을 쓰고, 통합 상태를 기록하며, 내부
registry 식별 정보를 선택된 `Volicord Runtime Home`에 저장합니다. 생성된 호스트
설정은 `volicord mcp --stdio`를 시작합니다. `--profile record`는 host lifecycle hook
설치나 session watcher를 요구하지 않습니다.

설치 프로필이 준비된 뒤 personal, global, read-only 동작을 직접 선택하는 등 낮은
수준의 연결 변형이 필요할 때는 `volicord connection add`를 사용합니다. 프로세스 현재
디렉터리가 대상 Product Repository가 아닐 때만 `--repo PATH`를 사용합니다.

```sh
volicord connection add codex --repo /path/to/your-product-repo
```

## 통합 프로필

Detective 상태는 선택된 연결 또는 session에 대해 선택된 프로필과 관찰 요약을
보고합니다.

| 프로필 | 도달 조건 | 운영상 의미 |
|---|---|---|
| Record profile(`record`) | Host hook이나 session watcher를 요구하지 않고 MCP 도구와 권한 기록을 사용할 수 있습니다. | 생성된 설정 안내가 호스트를 유도할 수 있지만 강제하지는 못합니다. |
| Detective profile(`detective`) | 프로젝트 로컬 host hook에 검증된 생성 설정, cwd-independent 및 subdirectory-safe hook 명령, native host output, 필수 phase, 쓰기 matcher, 일치하는 policy hash, 런타임 관찰, session watcher 관찰이 있습니다. | 협력형 host warning 또는 denial decision 신호, post-tool 상관, 채팅 명령 캡처, detective 상태, 미기록 변경, 닫기/쓰기 차단 사유가 workflow에 참여할 수 있습니다. |

Record profile은 prepare-write workflow를 통해 Volicord 쓰기 티켓을 발급할 수 있습니다.
Detective profile은 쓰기 티켓을 파일시스템 집행, 코드 리뷰 승인, 최종 수락, 쓰기가
실제로 일어났다는 증명으로 바꾸지 않습니다. 대신 지원되는 hook과 watcher 관찰을 더해
나중에 티켓 범위 쓰기 및 미기록 변경과 연결할 수 있습니다.

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
volicord init --host codex --repo /path/to/your-product-repo --profile record
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
volicord connection status codex --repo /path/to/your-product-repo
volicord connection verify codex --repo /path/to/your-product-repo
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
| `complete` | Volicord 쪽 상태, 관리 호스트 설정, 관찰 가능한 MCP 시작, 초기화, 기대 도구 노출이 준비되었습니다. |
| `action_required` | Volicord 쪽 상태는 있지만 이름 붙은 사용자 통제 호스트 동작이 남아 있습니다. |
| `failed` | 필요한 로컬 전제 조건, 호스트 설정 단계, 검증 단계가 성공하지 못했습니다. |
| `dry_run` | 명령이 지속 변경 없이 계획된 동작을 보고했습니다. |

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
