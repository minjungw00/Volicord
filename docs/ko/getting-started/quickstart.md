# 빠른 시작

이 튜토리얼은 실제 로컬 에이전트 호스트를 위한 가장 짧은 지원 첫 설정 경로입니다.
[설치](installation.md) 뒤에서 시작하며, 하나의 `Product Repository`를 사용하고,
개인 Codex 사용자 범위 항목과 프로젝트 범위 Claude Code `.mcp.json` 항목 중 하나를
고르게 합니다.

전체 호스트 설정 옵션, dry-run 미리보기, 저장소 지침, 제거, 문제 해결은
[에이전트 호스트 설정](../guides/agent-host-setup.md)을 봅니다.

## 독자, 목표, 완료 상태

독자: 로컬 `volicord`와 `volicord-mcp` 실행 파일을 이미 확인했고 설정을 확장하기 전에
에이전트 호스트 경로 하나를 동작시키려는 첫 사용자 또는 운영자입니다.

목표: 지원되는 호스트 설정 하나를 설치하고, 첫 결과가 `complete`인지
`action_required`인지 알아보며, 선택한 경로에 대해 별도 검증 명령을 실행합니다.

완료 상태: 선택한 경로는 해당 `integration_id`에 대한 `volicord agent verify`가
`status: complete`를 보고하고 선택된 Host Installation이 `final_status: complete`를
보고할 때 완료됩니다. 명령이 `action_required`를 보고하면 이름 붙은 호스트 소유 신뢰,
승인, reload, restart 동작을 완료한 뒤 검증을 다시 실행합니다.

## 시작 상태와 예시 값

명령을 실행하기 전에 아래를 준비합니다.

- POSIX 스타일 셸에서 [설치](installation.md)를 완료합니다.
- `VOLICORD_BIN`이 두 실행 파일이 들어 있는 확인된 절대 디렉터리를 계속 가리키게
  합니다.
- `Volicord Runtime Home`과 같지 않고 그 안이나 위에도 있지 않은 `Product Repository`를
  선택합니다.
- 아래의 모든 예시 경로와 ID를 실제 값으로 바꿉니다.

예시는 아래 값을 사용합니다.

| 예시 값 | 의미 |
|---|---|
| `VOLICORD_BIN="/absolute/path/to/selected/bin"` | `volicord`와 `volicord-mcp`가 함께 들어 있는 선택한 절대 디렉터리. |
| `"$VOLICORD_BIN/volicord"` | `volicord` 관리 CLI 호출. |
| `"$VOLICORD_BIN/volicord-mcp"` | 사용자/로컬 범위 호스트 설정에 쓰는 절대 `volicord-mcp` 명령. |
| `/Users/alex/.volicord` | `Volicord Runtime Home`. |
| `/work/acme-api` | Product Repository A. |
| `acme-api` | Product Repository A에 대해 사용자가 선택하는 안정적인 논리 프로젝트 ID입니다. 디렉터리 이름에서 자동으로 파생되는 값이 아닙니다. |
| `int-codex-team`, `int-claude-acme` | 예시 `integration_id` 값. |
| `volicord-int-codex-team`, `volicord-int-claude-acme` | `--server-name`을 생략했을 때 `integration_id`에서 파생되는 안정적인 호스트 MCP 서버 이름. |

`VOLICORD_BIN`은 튜토리얼용 셸 변수입니다. Volicord는 이를 설정으로 읽지 않습니다.
`VOLICORD_HOME`은 다릅니다. 기본 Runtime Home이 의도한 위치가 아닐 때 관리 명령과 이후
`volicord-mcp` 프로세스 시작에 쓰이는 실제 Runtime Home 선택 입력입니다.

이 튜토리얼에서 설치 인자를 고르는 방법은 아래와 같습니다.

| 인자 선택 | 여기 나타나는 이유 |
|---|---|
| `--host`와 `--scope` | 모든 `volicord agent install` 명령에 필수입니다. |
| `--project-id acme-api`와 `--repo-root /work/acme-api` | 예시가 새 프로젝트 등록을 도입하므로 여기서는 필수입니다. |
| `--integration-id ...` | 선택 사항이지만, 이후 verify, status, 생성 구성, 다중 저장소 예시가 같은 식별자를 가리킬 수 있도록 고정합니다. |
| `--runtime-home /Users/alex/.volicord` 또는 `VOLICORD_HOME=/Users/alex/.volicord` | 일반적으로는 선택 사항이지만, 이 튜토리얼은 환경이나 홈 디렉터리 기본값에 기대지 않고 해당 Runtime Home을 의도적으로 사용하므로 명시합니다. |
| `--mcp-command "$VOLICORD_BIN/volicord-mcp"` | 선택 사항이며, 검증된 절대 실행 파일을 생성되는 Codex 구성에 고정하려는 경로 A에만 유지합니다. 프로젝트 범위는 생략하면 이식 가능한 `volicord-mcp`를 사용하므로 `--mcp-command`를 생략합니다. |
| `--default-project-id` | 생략합니다. 새 통합에서는 선택한 프로젝트가 기본 프로젝트가 됩니다. |
| `--dry-run`, `--output json`, `--allow-repository-write` | `--dry-run`은 선택적인 zero-write 미리보기이고, `--output json`은 선택적 출력 형식이며, `--allow-repository-write`는 `.mcp.json`을 쓰는 실제 프로젝트 범위 적용 명령에만 나타납니다. |

전체 필수성, 기본값, 예외는
[관리 CLI 참조](../reference/admin-cli.md#volicord-agent-install)를 사용합니다.

## 호스트 경로 선택

| 경로 | 선택할 때 | 결과 |
|---|---|---|
| 경로 A: Codex `user` 범위 | 개인 Codex MCP 항목 하나가 지금 이 저장소를 처리하고, 나중에 명시적으로 허용된 저장소를 더 처리할 수 있어야 할 때. | 호스트 설정은 Codex 사용자 설정에 있고 절대 `volicord-mcp` 명령 경로와 `VOLICORD_HOME`을 저장합니다. |
| 경로 B: Claude Code `project` 범위 | Product Repository A가 팀 공유 Claude Code `.mcp.json` 항목을 가져야 할 때. | 프로젝트 파일은 이식 가능한 `volicord-mcp`를 사용하고 개인 `VOLICORD_HOME`을 생략하며, `--allow-repository-write`가 필요하고, Claude Code 승인이 끝날 때까지 `action_required`로 남을 수 있습니다. |

다른 호스트나 범위가 필요하면 [에이전트 호스트 설정](../guides/agent-host-setup.md)을
사용합니다. 하나의 사용자 범위 통합이 여러 저장소를 처리해야 하면 첫 저장소에 대해
경로 A를 완료한 뒤 [다중 저장소 에이전트 설정](../guides/multi-repository-agent-setup.md)을
따릅니다. 저장소마다 이 빠른 시작을 기계적으로 반복하지 않습니다.

## 경로 A: Codex 사용자 범위 설정

개인 Codex MCP 항목 하나가 명시적으로 허용된 하나 이상의 `Product Repository` 등록을
처리하게 하려면 이 경로를 사용합니다.

전제 조건:

- Codex가 `CODEX_HOME` 또는 `HOME`을 통해 사용자 `config.toml`을 읽을 수 있습니다.
- 호환성 점검을 위해 관리 명령의 `PATH`에서 `codex` 실행 파일을 사용할 수 있습니다.
- `VOLICORD_BIN`이 설치에서 확인한 절대 실행 파일 디렉터리를 가리킵니다.
- Product Repository A는 `/work/acme-api`에 있습니다.
- `/Users/alex/.volicord`는 `/work/acme-api`와 분리되어 있습니다.

명령:

```sh
"$VOLICORD_BIN/volicord" agent install \
  --host codex \
  --scope user \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --runtime-home /Users/alex/.volicord \
  --mcp-command "$VOLICORD_BIN/volicord-mcp"
```

변경될 수 있는 위치:

| 위치 | 변경 가능 내용 |
|---|---|
| `/Users/alex/.volicord` | Runtime Home registry, 통합, 프로젝트, 접점, Host Installation, 프로젝트 상태 기록. |
| 일반적으로 `~/.codex/config.toml` 또는 `CODEX_HOME/config.toml`인 Codex 사용자 설정 | `[mcp_servers.volicord-int-codex-team]` 테이블. |
| `/work/acme-api` | 저장소 지침을 별도로 선택하지 않는 한 파일 변경 없음. |

`--default-project-id`와 `--server-name`을 생략했으므로 새 통합은 선택한 프로젝트를
기본값으로 사용하고, CLI는 `integration_id`에서 안정적인 호스트 MCP 서버 이름을
파생합니다. 특정 호스트 설정 키를 고정해야 할 때만 `--server-name`을 사용합니다.

첫 예상 결과:

```text
status: complete
integration_id: int-codex-team
host_kind: codex
host_scope: user
server_name: volicord-int-codex-team
verification: complete
verification_detail: MCP initialize and tools/list succeeded
```

생성되는 Codex 항목은 아래 형태입니다.

```toml
[mcp_servers.volicord-int-codex-team]
command = "/absolute/path/to/selected/bin/volicord-mcp"
args = ["--integration", "int-codex-team"]

[mcp_servers.volicord-int-codex-team.env]
VOLICORD_HOME = "/Users/alex/.volicord"
```

실제 `command` 값은 `VOLICORD_BIN`으로 선택한 경로가 셸에서 해석된 절대 경로입니다.
생성된 TOML에는 `VOLICORD_BIN`이 들어가지 않습니다.

독립 완료 점검:

```sh
"$VOLICORD_BIN/volicord" agent verify \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.volicord
```

경로 A는 검증이 `status: complete`를 보고할 때 완료됩니다. 검증이 `action_required`를
보고하면 이름 붙은 동작을 읽습니다. Codex 사용자 범위에서 흔한 원인은 관리 명령의
`PATH`에 `codex`가 없거나 `codex --version`을 실행할 수 없는 경우입니다.

## 경로 B: Claude Code 프로젝트 범위 설정

Product Repository A가 팀 공유 Claude Code `.mcp.json` 항목을 갖게 하려면 이 경로를
사용합니다.

전제 조건:

- `VOLICORD_BIN`이 설치에서 확인한 절대 실행 파일 디렉터리를 가리킵니다.
- Claude Code가 MCP 서버를 시작할 때 사용할 `PATH`에서 `volicord-mcp`를 찾을 수 있어야
  합니다.
- Claude Code가 자체적으로 `/Users/alex/.volicord`를 Runtime Home으로 해석하지 않는다면
  Claude Code 시작 환경이 `VOLICORD_HOME=/Users/alex/.volicord`를 제공해야 합니다.
- Product Repository A는 `/work/acme-api`에 있습니다.
- `/Users/alex/.volicord`는 `/work/acme-api`와 분리되어 있습니다.
- 관리 명령이 `/work/acme-api/.mcp.json`을 쓰는 것을 의도적으로 허용합니다.

프로젝트 파일을 쓰기 전 선택적 dry-run:

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --dry-run \
  --output json
```

설정 적용:

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --allow-repository-write
```

변경될 수 있는 위치:

| 위치 | 변경 가능 내용 |
|---|---|
| `/Users/alex/.volicord` | Runtime Home registry, 통합, 프로젝트, 접점, Host Installation, 프로젝트 상태 기록. |
| `/work/acme-api/.mcp.json` | Claude Code 프로젝트 범위 MCP 서버 항목. |
| Claude Code 사용자 승인 상태 | 사용자가 Claude Code에서 프로젝트 MCP 서버를 승인한 뒤에만 바뀝니다. |

호스트 승인 전 첫 예상 결과:

```text
status: action_required
integration_id: int-claude-acme
host_kind: claude_code
host_scope: project
server_name: volicord-int-claude-acme
verification: action_required
```

출력은 Claude Code 프로젝트 MCP 승인 같은 호스트 소유 후속 조치를 이름 붙여야 합니다.
`action_required`는 성공한 관리 결과이며 명령 실패가 아닙니다.

생성되는 `.mcp.json` 항목은 아래 형태입니다.

```json
{
  "mcpServers": {
    "volicord-int-claude-acme": {
      "command": "volicord-mcp",
      "args": ["--integration", "int-claude-acme"]
    }
  }
}
```

생성되는 `.mcp.json`은 의도적으로 `VOLICORD_HOME`을 생략하고 이식 가능한
`volicord-mcp` 명령을 유지합니다. 이 이식 가능한 명령은 `--mcp-command`를 생략했을
때의 프로젝트 범위 기본값입니다. 설치 명령에 붙은 `VOLICORD_HOME`과 `PATH` 할당은 그
관리 명령 실행에만 적용됩니다. 나중에 Claude Code가 서버를 시작할 때는 Claude Code의
시작 환경이 `PATH`에서 `volicord-mcp`를 찾을 수 있어야 하며, 기본 Runtime Home이
다르다면 `VOLICORD_HOME`을 제공해야 합니다.

호스트 소유 동작을 완료합니다. 의도한 환경에서 `/work/acme-api`의 Claude Code를
시작하거나 재시작하고, 프로젝트 MCP 서버를 검토한 뒤 Claude Code 안에서 승인합니다.

승인 뒤 독립 완료 점검:

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent verify \
  --integration-id int-claude-acme
```

경로 B는 검증이 `status: complete`를 보고하고 `volicord-int-claude-acme` 설치 검증이
`final_status: complete`를 보고할 때 완료됩니다. 검증이 여전히 `action_required`를
보고하면 호스트 소유 승인, reload/restart, 또는 시작 환경이 아직 완료되지 않은
상태입니다.

## 인벤토리, 검증, 실제 호스트 로딩

레지스트리와 Host Installation 인벤토리를 보려면 `volicord agent status`를 사용합니다.

```sh
"$VOLICORD_BIN/volicord" agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.volicord
```

`volicord agent status`는 Codex 또는 Claude Code가 MCP 서버를 로드했다는 증명이 아닙니다.
관리 검증 단계는 `volicord agent verify`로 확인하고, 호스트가 로드 상태를 노출한다면
호스트 자체의 UI, MCP 목록, 승인 흐름에서 확인합니다.

성공한 `volicord-mcp --check --integration <integration_id>`는 MCP 프로세스 시작 검증일
뿐입니다. 그 자체로 완료된 호스트 통합이 아닙니다.

## 설정 상태 의미

| 상태 | 다음 행동 |
|---|---|
| `complete` | 관리 설정, 호스트 소유 확인 단계, MCP 초기화, 도구 발견이 성공했습니다. 호스트를 사용하고, 호스트가 MCP 서버나 도구를 보여 주는 위치에서 서버가 보이는지 확인합니다. |
| `action_required` | 명령은 성공했지만 이름 붙은 호스트 소유 동작이 남았습니다. 그 동작을 완료한 뒤 `volicord agent verify`를 실행합니다. |
| `partial_failure` | 나중 단계가 실패하기 전에 일부 지속 동작이 성공했을 수 있습니다. `effects`와 `residual_effects`를 읽고 이름 붙은 문제만 고친 뒤 같은 명령 또는 verify를 다시 실행합니다. |
| `failed` | 요청한 설정 또는 검증이 사용할 수 있는 지속 통합 상태나 호스트 설정을 만들지 못했습니다. 보고된 오류를 고친 뒤 다시 시도합니다. |

## 실패 경로

| 증상 | 안전한 다음 행동 | 경로 |
|---|---|---|
| 설정 전에 `volicord`, `volicord-mcp`, 또는 `VOLICORD_BIN`이 실패합니다. | 설치로 돌아가 같은 셸에서 실행 파일 점검을 다시 실행합니다. | [설치](installation.md#verify-the-selected-directory) |
| 설정 또는 검증이 `volicord-mcp`를 해석하지 못합니다. | 사용자/로컬 범위에서는 유효한 절대 `--mcp-command`를 사용합니다. 프로젝트 범위에서는 `volicord-mcp`를 이식 가능한 형태로 유지하고 호스트 `PATH`를 고칩니다. | [에이전트 호스트 문제 해결](../guides/agent-host-troubleshooting.md#missing-volicord-mcp) |
| 프로젝트 범위 명령이 `.mcp.json` 또는 `.codex/config.toml` 쓰기를 거부합니다. | 저장소 쓰기가 의도한 동작인지 결정한 뒤 `--allow-repository-write`를 포함해 다시 실행합니다. | [관리 CLI](../reference/admin-cli.md#noninteractive-approval-behavior) |
| 결과가 `action_required`입니다. | 이름 붙은 호스트 소유 신뢰, 승인, reload, restart, 또는 실행 파일 가용성 동작만 완료한 뒤 `volicord agent verify`를 다시 실행합니다. | [에이전트 호스트 문제 해결](../guides/agent-host-troubleshooting.md#status-action_required) |
| 결과가 `partial_failure` 또는 `failed`입니다. | 보고된 `effects`, `residual_effects`, `warnings`, `verification` 세부사항을 읽습니다. 첫 대응으로 Runtime Home, Product Repository, 관련 없는 호스트 항목을 삭제하지 않습니다. | [에이전트 호스트 문제 해결의 partial_failure](../guides/agent-host-troubleshooting.md#status-partial_failure)와 [failed](../guides/agent-host-troubleshooting.md#status-failed) |
| 하나의 통합이 여러 저장소를 처리해야 합니다. | 사용자 범위 통합을 사용하고 명시적 프로젝트 멤버십을 추가합니다. 저장소마다 호스트 항목을 하나씩 추가하지 않습니다. | [다중 저장소 에이전트 설정](../guides/multi-repository-agent-setup.md) |

## 계속 읽기

- 전체 호스트 설정, dry-run 미리보기, 저장소 지침, generic export, 상태, 검증, 안전한 제거:
  [에이전트 호스트 설정](../guides/agent-host-setup.md)
- 하나의 사용자 범위 통합이 여러 저장소를 처리하는 경로:
  [다중 저장소 에이전트 설정](../guides/multi-repository-agent-setup.md)
- 에이전트 작업 흐름: [에이전트 가이드](../guides/agent-workflow.md)
- 정확한 `volicord` agent 명령 동작:
  [관리 CLI](../reference/admin-cli.md#volicord-agent-install)
- 정확한 프로젝트 선택과 지침 경계:
  [에이전트 통합](../reference/agent-integration.md)
- 정확한 `volicord-mcp` 프로세스 동작:
  [MCP 전송](../reference/mcp-transport.md)
- 정확한 런타임 위치 경계:
  [런타임 경계](../reference/runtime-boundaries.md)
