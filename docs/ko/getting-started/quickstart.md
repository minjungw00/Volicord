# 빠른 시작

이 문서는 실제 로컬 에이전트 호스트를 위한 가장 짧은 첫 설정 경로를 담당합니다. `Harness Server` 실행 파일을 빌드했거나 찾을 수 있고, 허용할 `Product Repository`가 있다고 가정합니다.

빌드 세부사항과 실행 파일 탐색 규칙은 [설치](installation.md)를 봅니다. 전체 호스트 설정 옵션, dry-run 미리보기, 저장소 guidance, 제거, 문제 해결은 [에이전트 호스트 설정](../guides/agent-host-setup.md)을 봅니다.

예시는 아래 값을 사용합니다.

| 예시 값 | 의미 |
|---|---|
| `/opt/harness/bin/harness` | 설치된 `harness` 실행 파일 |
| `/opt/harness/bin/harness-mcp` | 설치된 `harness-mcp` 실행 파일 |
| `/Users/alex/.harness` | `Harness Runtime Home` |
| `/work/acme-api` | Product Repository A |
| `acme-api` | Product Repository A의 프로젝트 ID |
| `harness-int-codex-team`, `harness-int-claude-acme` | `integration_id`에서 파생되는 안정적인 호스트 MCP 서버 이름 |

## 1단계: Harness Server 준비

작업 디렉터리: 이 저장소에서 빌드한다면 `Harness Server` 소스 저장소 루트.

```sh
cargo build -p harness-cli -p harness-mcp
```

아래 실행 파일을 사용할 수 있습니다.

- `target/debug/harness`
- `target/debug/harness-mcp`

이 파일들을 절대 경로로 사용하거나, 같은 `harness`와 `harness-mcp` 명령을 제공하는 설치된 실행 파일을 사용합니다.

## 경로 A: Codex 사용자 범위 설정

개인 Codex MCP 항목 하나가 명시적으로 허용된 하나 이상의 `Product Repository` 등록을 처리하게 하려면 이 경로를 사용합니다.

전제 조건:

- Codex가 사용자 `config.toml`을 읽을 수 있습니다.
- `harness-mcp`를 절대 경로로 사용할 수 있습니다.
- Product Repository A는 `/work/acme-api`에 있습니다.
- `/Users/alex/.harness`는 `/work/acme-api`와 분리되어 있습니다.

명령:

```sh
/opt/harness/bin/harness agent install \
  --host codex \
  --scope user \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --default-project-id acme-api \
  --runtime-home /Users/alex/.harness \
  --mcp-command /opt/harness/bin/harness-mcp
```

변경될 수 있는 위치:

| 위치 | 변경 가능 내용 |
|---|---|
| `/Users/alex/.harness` | Runtime Home registry, 통합, 프로젝트, 접점, Host Installation, 프로젝트 상태 기록. |
| 일반적으로 `~/.codex/config.toml` 또는 `CODEX_HOME/config.toml`인 Codex 사용자 설정 | `[mcp_servers.harness-int-codex-team]` 테이블. |
| `/work/acme-api` | 저장소 guidance를 별도로 선택하지 않는 한 파일 변경 없음. |

`--server-name`을 생략했으므로 CLI는 `integration_id`에서 안정적인 호스트 MCP 서버 이름을 파생합니다. 특정 호스트 설정 키를 고정해야 할 때만 `--server-name`을 사용합니다.

예상 결과:

```text
status: complete
integration_id: int-codex-team
host_kind: codex
host_scope: user
server_name: harness-int-codex-team
verification: complete
verification_detail: MCP initialize and tools/list succeeded
```

생성되는 Codex 항목은 아래 형태입니다.

```toml
[mcp_servers.harness-int-codex-team]
command = "/opt/harness/bin/harness-mcp"
args = ["--integration", "int-codex-team"]

[mcp_servers.harness-int-codex-team.env]
HARNESS_HOME = "/Users/alex/.harness"
```

나중에 확인:

```sh
/opt/harness/bin/harness agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness

/opt/harness/bin/harness agent verify \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness
```

성공을 알아보는 기준:

- 설치 또는 verify에서 `status: complete`이면 지속 통합 상태가 있고, 호스트 설정이 설치되었고, 호스트 소유 신뢰나 승인 gate가 충족되었거나 해당하지 않으며, MCP 초기화와 도구 발견이 성공했다는 뜻입니다.
- `harness agent status`는 inventory와 상태 보고입니다. 그 verification 섹션은 호스트 로딩을 증명하지 않는다고 말할 수 있습니다.

## 경로 B: Claude Code 프로젝트 범위 설정

Product Repository A가 팀 공유 Claude Code `.mcp.json` 항목을 갖게 하려면 이 경로를 사용합니다.

전제 조건:

- Claude Code가 사용할 `PATH`에서 `harness-mcp`를 찾을 수 있습니다.
- Product Repository A는 `/work/acme-api`에 있습니다.
- `/Users/alex/.harness`는 `/work/acme-api`와 분리되어 있습니다.
- Product Repository A에 `.mcp.json`을 쓸 의도가 있습니다.

명령:

```sh
HARNESS_HOME=/Users/alex/.harness \
PATH="/opt/harness/bin:$PATH" \
/opt/harness/bin/harness agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --mcp-command harness-mcp \
  --allow-repository-write
```

변경될 수 있는 위치:

| 위치 | 변경 가능 내용 |
|---|---|
| `/Users/alex/.harness` | Runtime Home registry, 통합, 프로젝트, 접점, Host Installation, 프로젝트 상태 기록. |
| `/work/acme-api/.mcp.json` | Claude Code 프로젝트 범위 MCP 서버 항목. |
| Claude Code 사용자 승인 상태 | 사용자가 Claude Code에서 프로젝트 MCP 서버를 승인한 뒤에만 바뀝니다. |

예상 결과:

```text
status: action_required
verification: action_required
verification_detail: Claude Code requires user approval before project-scoped .mcp.json servers load
```

생성되는 `.mcp.json` 항목은 아래 형태입니다.

```json
{
  "mcpServers": {
    "harness-int-claude-acme": {
      "command": "harness-mcp",
      "args": ["--integration", "int-claude-acme"]
    }
  }
}
```

`action_required`는 설정 실패가 아닙니다. `/work/acme-api`에서 Claude Code를 시작하고 프로젝트 범위 MCP 서버를 검토해 승인한 뒤 아래를 실행합니다.

```sh
HARNESS_HOME=/Users/alex/.harness \
/opt/harness/bin/harness agent verify \
  --integration-id int-claude-acme
```

## 먼저 Dry-run 실행하기

프로젝트 범위 설정이나 저장소 guidance를 쓰기 전에는 `--dry-run --output json`을 사용합니다.

```sh
/opt/harness/bin/harness agent install \
  --host codex \
  --scope user \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --runtime-home /Users/alex/.harness \
  --mcp-command /opt/harness/bin/harness-mcp \
  --dry-run \
  --output json
```

Dry-run 출력은 `status: dry_run`, 계획된 동작, 호스트 대상 경로, 선택한 경우 guidance 대상 경로를 보고합니다. Runtime Home 디렉터리, SQLite 파일이나 행, WAL 또는 SHM 파일, registry 마이그레이션, 호스트 설정, `Product Repository` guidance, generic export 파일을 만들거나 수정하지 않습니다.

## 설정 상태 의미

| 상태 | 다음 행동 |
|---|---|
| `complete` | 관리 설정, 호스트 소유 gate, MCP 검증 경로가 성공했습니다. 호스트를 열어 서버가 MCP UI나 도구 목록에 보이는지 확인합니다. |
| `action_required` | 출력이 이름 붙인 호스트 소유 동작을 완료합니다. 예를 들어 Codex 프로젝트 신뢰나 Claude Code 프로젝트 MCP 승인을 마친 뒤 `harness agent verify`를 실행합니다. |
| `partial_failure` | 나중 단계가 실패하기 전에 일부 지속 동작이 성공했을 수 있습니다. 보고된 문제를 고치고 같은 명령을 다시 실행합니다. |
| `failed` | 요청한 설정이 사용할 수 있는 지속 통합 상태나 호스트 설정을 만들지 못했습니다. 보고된 오류를 고친 뒤 다시 시도합니다. |

성공한 `harness-mcp --check --integration <integration_id>`는 MCP 프로세스 시작 검증일 뿐입니다. 그 자체로 완료된 호스트 통합이 아닙니다. 호스트 설정이 있다는 것과 호스트가 로드하거나 도구를 발견했다는 것은 다릅니다. 도구 발견도 이후 모든 모델 판단이 하네스 도구를 선택한다는 보장이 아닙니다.

## 계속 읽기

- 전체 호스트 설정, dry-run 미리보기, 저장소 guidance, generic export, 상태, 검증, 안전한 제거: [에이전트 호스트 설정](../guides/agent-host-setup.md)
- 하나의 사용자 범위 통합이 여러 저장소를 처리하는 경로: [다중 저장소 에이전트 설정](../guides/multi-repository-agent-setup.md)
- 에이전트 작업 흐름: [에이전트 가이드](../guides/agent-workflow.md)
- 정확한 `harness` agent 명령 동작: [관리 CLI](../reference/admin-cli.md#harness-agent-install)
- 정확한 프로젝트 선택과 guidance 경계: [에이전트 통합](../reference/agent-integration.md)
- 정확한 `harness-mcp` 프로세스 동작: [MCP 전송](../reference/mcp-transport.md)
- 정확한 런타임 위치 경계: [런타임 경계](../reference/runtime-boundaries.md)
