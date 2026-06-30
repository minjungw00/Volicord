# 에이전트 호스트 설정

이 가이드는 Codex, Claude Code, 일반 MCP 호스트를 Volicord에 연결할 때
사용합니다. 일반적인 guarded 경로는 `volicord init`, 호스트, Product Repository에서
시작하며, 내부 호스트와 registry 값은 Volicord가 관리합니다.

정확한 CLI 동작은 [관리 CLI 참조](../reference/admin-cli.md)가 담당합니다.
Agent Connection 의미는 [Agent Connection 참조](../reference/agent-connection.md)가,
런타임/파일 경계는 [런타임 경계](../reference/runtime-boundaries.md)가 담당합니다.

## 설정 순서

먼저 [설치](../getting-started/installation.md)에 따라 `volicord`를 설치한 뒤 호스트
설정 순서를 실행합니다.

```sh
volicord init --host codex --repo /path/to/your-product-repo
volicord connection status codex
```

`/path/to/your-product-repo`는 에이전트에게 작업을 요청할 Product Repository의 경로
예시입니다. `volicord init`은 필요하면 Runtime Home과 설치 프로필을 만들거나
재사용하고, 해당 저장소 프로젝트를 등록하거나 재사용하며, 저장소 디렉터리에서 보이는
프로젝트 이름을 파생하고, 선택한 호스트의 프로젝트 범위 MCP 설정을 설치하고,
Volicord 관리 지침과 guard 통합 파일을 쓰고, guard 설치 상태를 기록하며, 내부
registry 식별 정보를 선택된 `Volicord Runtime Home`에 저장합니다. 생성된 호스트
설정은 `volicord mcp --stdio`를 시작합니다.

설치 프로필이 준비된 뒤 personal, global, read-only 동작을 직접 선택하는 등 낮은
수준의 연결 변형이 필요할 때는 `volicord connect`를 사용합니다. 프로세스 현재
디렉터리가 대상 Product Repository가 아닐 때만 `--repo PATH`를 사용합니다.

```sh
volicord connect codex --repo /path/to/your-product-repo
```

## 연결 의도

연결 의도는 호스트 설정이 어디에 속하는지 설명합니다.

| 의도 | 명령 형태 | 호스트 지원 |
|---|---|---|
| `personal` | `volicord connect codex` 또는 `volicord connect claude-code` | 현재 사용자를 위한 로컬 설정. |
| `shared` | `volicord connect codex --shared` 또는 `volicord connect claude-code --shared` | 호스트가 지원할 때 명시적 통합 파일을 통해 저장되는 프로젝트 공유 설정. |
| `global` | `volicord connect claude-code --global` | 이를 지원하는 호스트의 사용자 전체 호스트 설정. |

`--shared`와 `--global`은 함께 사용할 수 없습니다. 둘 다 없으면 Volicord는
`personal`을 사용합니다.

## Workflow와 Read-Only 모드

기본 모드는 `workflow`입니다. Workflow 도구 대신 읽기 중심 동작을 노출해야 하는
연결에는 `--read-only`를 사용합니다.

```sh
volicord connect codex --read-only
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
volicord connect codex --dry-run
volicord connect claude-code --shared --dry-run
volicord connection remove codex --dry-run
```

공유 호스트 설정을 바꾸기 전이나 제거할 연결의 호스트 대상을 먼저 확인하고 싶을
때 dry run을 사용합니다.

## 조회와 검증

```sh
volicord connections
volicord connection status codex
volicord connection verify codex
```

`shared` 또는 `global` 연결은 선택할 때 쓴 의도 플래그를 함께 넣습니다.

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

## Generic MCP 설정 내보내기

Volicord가 직접 관리하지 않는 MCP 호스트에는 아래처럼 설정을 내보냅니다.

```sh
cd /path/to/your-product-repo
volicord export mcp-config --output /tmp/volicord.mcp.json
```

내보내기는 감지된 Product Repository와 설치 프로필을 사용합니다. 내보낸 설정이
read-only 연결에 묶여야 하면 `--read-only`를 추가합니다. 내보낸 파일은 내보내기 뒤에도
사용자 관리 파일로 남습니다.

## User Channel 경계

Agent Connection은 초점이 맞춰진 판단 필요를 요청하거나 표시할 수 있습니다. 권한을
지니는 사용자 답변은 기록하지 않습니다. Core가 생성한 선택지가 사용자의 기록된
판단이 되어야 하면 로컬 `User Channel` 명령을 사용합니다.

```sh
volicord user judgments
volicord user judgment show 1
volicord user judgment answer 1 1
```

## 제거

선택한 Product Repository를 연결에서 제거합니다.

```sh
volicord connection remove codex --dry-run
volicord connection remove codex
```

제거는 소유권과 안전 점검이 허용할 때 일치하는 관리 호스트 설정만 삭제합니다.
`Product Repository`, Runtime Home, 프로젝트 등록, 프로젝트 상태, Core 기록,
아티팩트 저장소, 관련 없는 호스트 설정은 삭제하지 않습니다.

## 문제 해결 경로

| 증상 | 다음 문서 |
|---|---|
| 설치 프로필, 실행 파일, Product Repository 감지가 준비되지 않았습니다. | [설치](../getting-started/installation.md) |
| 연결이 `action_required` 또는 `failed`를 보고합니다. | [에이전트 호스트 문제 해결](agent-host-troubleshooting.md) |
| 정확한 명령 동작이 불분명합니다. | [관리 CLI 참조](../reference/admin-cli.md) |
| Runtime Home과 Product Repository 경계가 중요합니다. | [런타임 경계](../reference/runtime-boundaries.md) |
