# 에이전트 호스트 설정

이 가이드는 Codex, Claude Code, 일반 MCP 호스트를 Volicord에 연결할 때
사용합니다. 일반적인 첫 실행 경로는 `volicord init`, 호스트, Product Repository,
그리고 호스트 capability에 맞는 통합 모드에서 시작하며, 내부 호스트와 registry 값은
Volicord가 관리합니다.

정확한 CLI 동작은 [관리 CLI 참조](../reference/admin-cli.md)가 담당합니다.
Agent Connection 의미는 [Agent Connection 참조](../reference/agent-connection.md)가,
런타임/파일 경계는 [런타임 경계](../reference/runtime-boundaries.md)가 담당합니다.

## 설정 순서

먼저 [설치](../getting-started/installation.md)에 따라 `volicord`를 설치한 뒤 호스트
설정 순서를 실행합니다.

```sh
volicord init --host codex --repo /path/to/your-product-repo --mode mcp-only
volicord connection status codex --repo /path/to/your-product-repo
```

`/path/to/your-product-repo`는 에이전트에게 작업을 요청할 Product Repository의 경로
예시입니다. `volicord init`은 필요하면 Runtime Home과 설치 프로필을 만들거나
재사용하고, 해당 저장소 프로젝트를 등록하거나 재사용하며, 저장소 디렉터리에서 보이는
프로젝트 이름을 파생하고, 선택한 호스트의 프로젝트 범위 MCP 설정을 설치하고,
Volicord 관리 지침과 policy 메타데이터를 쓰고, guard 설치 상태를 기록하며, 내부
registry 식별 정보를 선택된 `Volicord Runtime Home`에 저장합니다. 생성된 호스트
설정은 `volicord mcp --stdio`를 시작합니다. `--mode mcp-only`는 더 낮은 보장의 설정
경로이며 호스트 lifecycle hook 설치를 요구하지 않습니다.

설치 프로필이 준비된 뒤 personal, global, read-only 동작을 직접 선택하는 등 낮은
수준의 연결 변형이 필요할 때는 `volicord connect`를 사용합니다. 프로세스 현재
디렉터리가 대상 Product Repository가 아닐 때만 `--repo PATH`를 사용합니다.

```sh
volicord connect codex --repo /path/to/your-product-repo
```

## 보호 수준

Guard health는 선택된 연결 또는 session에서 활성인 가장 강한 보호 표면을 보고합니다.

| `guard_strength` | 도달 조건 | 운영상 의미 |
|---|---|---|
| `authority_record_only` | 활성 session watcher나 관찰된 전체 host hook guard 없이 MCP 도구와 권한 기록을 사용할 수 있습니다. | pre-tool 차단은 없습니다. 설정 안내와 policy 메타데이터가 호스트를 유도할 수 있지만 강제하지는 못합니다. |
| `detective_watch` | 선택된 session에 대해 session watcher가 활성 상태입니다. | Product Repository 메타데이터 변경이 조정과 닫기 준비 상태에 쓰이는 찾기를 만들 수 있지만, watcher는 쓰기를 막거나 행위자를 식별하지 않습니다. |
| `host_hook_guarded` | 필수 프로젝트 로컬 호스트 hook phase가 설정되고 관찰되었습니다. | pre-tool 결정, post-tool 상관, prompt capture, guard 상태, 닫기/쓰기 차단 사유가 workflow에 참여할 수 있습니다. |
| `managed_guarded` | `host_hook_guarded`가 활성이고 검증된 managed 배포 출처가 기록되어 있습니다. | 지원되는 호스트 관리 plugin, bundle, policy 배포용입니다. 현재 Codex와 Claude Code 설정은 이 라벨에 도달하지 않습니다. |

## Guard 수명주기

Guarded 모드에서는 설정과 활성화가 분리됩니다. `volicord init`은 MCP 호스트 설정,
Volicord 관리 `AGENTS.md` 안내, `.volicord/policy.json`, 호스트 hook 또는 rule 파일,
guard 설치 상태를 설치하거나 갱신합니다. 그래도 그 파일이 실행되려면 호스트 reload,
restart, trust, 프로젝트 MCP 승인, 또는 다른 호스트 소유 동작이 필요할 수 있습니다.

현재 검증된 guarded adapter는 호스트별로 다릅니다.

- Codex guarded 설정은 프로젝트 MCP 설정, `.codex/hooks.json`,
  `.codex/rules/*.rules`를 씁니다. 생성된 rule과 hook 파일이 실행되려면 호스트에
  프로젝트 trust, hook trust, restart 또는 reload가 필요할 수 있습니다.
- Claude Code guarded 설정은 `.mcp.json`, `.claude/settings.json`,
  `.claude/rules/*.md`를 씁니다. Settings 쓰기는 관련 없는 settings를 보존하고
  Volicord 관리 항목을 병합합니다. 생성된 hook과 rule 파일이 실행되려면 호스트에
  프로젝트 MCP approval, workspace trust, settings reload가 필요할 수 있습니다.

기본 `guarded` init은 모든 필수 호스트 lifecycle hook phase를 설치하고 검증할 수
있어야 합니다. 선택한 Codex 또는 Claude Code 어댑터가 모든 필수 phase에 대해 신뢰할
수 있는 프로젝트 로컬 hook 스키마나 경로를 알지 못하면, init은 `AGENTS.md`나
`.volicord/policy.json`을 집행으로 취급하지 않고 실패합니다. `--allow-degraded`는
degraded 설정 파일을 명시적으로 원하고 필수 hook phase가 누락된 것으로 보고됨을 이해할
때만 사용합니다.

```sh
volicord init --host codex --repo /path/to/your-product-repo --allow-degraded
```

Managed 모드는 프로젝트 로컬 guarded 모드와 별개입니다. Volicord 호스트 계약 데이터에
기록된 검증된 호스트 managed 배포 출처가 필요합니다. 현재 Codex와 Claude Code 계약에는
검증된 plugin, bundle, managed policy 배포 출처가 기록되어 있지 않으므로
`volicord init --mode managed`는 `MANAGED_MODE_UNSUPPORTED`로 실패합니다. 그런 계약이
추가되고 구현되기 전에는 `guarded` 또는 `mcp-only`를 사용합니다.

`volicord connection verify`와 `volicord doctor`는 파일 상태, 필요한 호스트 동작,
관찰된 활성화를 분리해서 다룹니다. Volicord가 기록된 프로젝트, Agent Connection, 호스트
종류, guard 모드, policy hash와 일치하는 guard hook 이벤트를 관찰해야 guard 설치가
활성화됩니다. `AGENTS.md`는 지침 지원이며, 호스트 hook과 rule은 협력적이고 탐지적인
guardrail입니다. OS 샌드박싱, 명령 격리, Volicord를 아는 경로 밖에서 쓰기가 일어날 수
없다는 증명이 아닙니다.

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
