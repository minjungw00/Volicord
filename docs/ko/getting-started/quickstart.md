# Quickstart

이 튜토리얼은 새 소스 체크아웃에서 작동하는 Agent Connection 하나까지 가는
경로입니다. 로컬 호스트를 일반 Git 제품 저장소에 연결한다고 가정합니다.

정확한 명령 계약은 [관리 CLI 참조](../reference/admin-cli.md)가 담당합니다.
Agent Connection 의미는 [Agent Connection 참조](../reference/agent-connection.md)가
담당합니다.

## 빠른 경로

```sh
cargo build --workspace --bins
./target/debug/volicord setup --link-bin ~/.local/bin
cd /work/acme-api
volicord connect codex
```

저장소는 현재 디렉터리에서 감지됩니다. 프로젝트 이름은 저장소 디렉터리에서
나옵니다. 따라서 `/work/acme-api`는 그 이름을 고유하게 만들어야 하는 경우가
아니라면 보이는 프로젝트 이름 `acme-api`가 됩니다. 기본 연결 의도는
`personal`이고 기본 모드는 `workflow`이며, 내부 식별 정보는 Volicord가
관리합니다.

## Setup 확인하기

```sh
volicord doctor
volicord project current
volicord connection status codex
volicord connection verify codex
```

완료 상태: status나 verification이 `complete`를 보고하면 연결 준비가 끝난
것입니다. `action_required`를 보고하면 이름 붙은 호스트 소유 trust, approval,
reload, restart, setup repair 동작을 완료한 뒤 verification을 다시 실행합니다.

## 호스트 의도 선택하기

호스트 설정이 있어야 할 위치에 맞는 가장 짧은 명령을 사용합니다.

| 의도 | 명령 형태 | 사용할 때 |
|---|---|---|
| `personal` | `volicord connect codex` 또는 `volicord connect claude-code` | 현재 사용자의 로컬 호스트 설정을 위한 연결일 때. |
| `shared` | `volicord connect codex --shared` 또는 `volicord connect claude-code --shared` | 저장소가 명시적인 프로젝트 공유 호스트 통합 파일을 가져야 할 때. |
| `global` | `volicord connect claude-code --global` | 선택한 호스트가 사용자 전체 설정을 지원하고, 프로젝트 접근은 계속 Volicord 기록으로 제한되어야 할 때. |

Workflow 도구 대신 읽기 중심 동작만 노출해야 할 때만 `--read-only`를 사용합니다.

```sh
volicord connect codex --read-only
```

현재 디렉터리가 연결하려는 저장소가 아닐 때만 `--repo PATH`를 사용합니다.

```sh
volicord connect codex --repo /work/acme-api
```

## 연결 조회 또는 변경하기

```sh
volicord connections
volicord connection status codex
volicord connection verify codex
volicord connection mode codex read-only
volicord connection mode codex workflow
```

선택한 저장소를 연결에서 제거할 때도 같은 호스트와 의도 선택을 사용합니다.

```sh
volicord connection remove codex --dry-run
volicord connection remove codex
```

`--dry-run`은 지속 변경 없이 계획을 보고합니다.

## Generic MCP 설정 내보내기

Volicord가 직접 관리하지 않는 MCP 호스트에는 호스트 중립 설정을 내보냅니다.

```sh
volicord export mcp-config --output /tmp/volicord.mcp.json
```

Export는 감지된 저장소와 setup 프로필을 사용합니다. 내보낸 파일은 export 뒤에도
사용자가 관리합니다. Volicord는 임의 외부 호스트가 이 파일을 로드하거나
승인했다고 주장하지 않습니다.

## 사용자 판단 기록하기

Agent Connection은 초점이 맞춰진 판단 필요를 요청하거나 보여 줄 수 있지만,
권한을 지니는 사용자 답변은 로컬 `User Channel`을 거칩니다.

```sh
volicord user status
volicord user judgments
volicord user judgment show 1
volicord user judgment answer 1 1
```

현재 저장소가 아닌 다른 저장소에 답해야 할 때만 `--repo PATH`를 사용합니다. 활성
작업이 의도한 작업이 아닐 때는 `--task ID`를 사용합니다.

## 다음 단계

| 필요 | 읽을 문서 |
|---|---|
| 호스트 설정 세부사항 | [에이전트 호스트 Setup](../guides/agent-host-setup.md) |
| `action_required` 또는 `failed` 문제 해결 | [에이전트 호스트 문제 해결](../guides/agent-host-troubleshooting.md) |
| 사용자 작업 흐름과 판단 경계 | [사용자 가이드](../guides/user-workflow.md) |
| 에이전트 작업 흐름 경계 | [에이전트 가이드](../guides/agent-workflow.md) |
