# 빠른 시작

이 튜토리얼은 [설치](installation.md)를 통해 `volicord`가 `PATH`에서 사용 가능해진
뒤 작동하는 Agent Connection 하나까지 가는 경로입니다. 에이전트에게 작업을 요청할
Product Repository로 사용할 일반 Git 저장소에 로컬 호스트를 연결한다고 가정합니다.

정확한 명령 계약은 [관리 CLI 참조](../reference/admin-cli.md)를 보세요.
Agent Connection 의미는 [Agent Connection 참조](../reference/agent-connection.md)에
있습니다.

## 빠른 경로

```sh
volicord init --host codex --repo /path/to/repo --profile record
```

`/path/to/repo`는 에이전트에게 작업을 요청할 Product Repository의 경로 예시입니다.
`volicord init`은 첫 실행에서 저장소를 설정하고 호스트를 연결하는 기본 명령입니다.
필요하면 Runtime Home과 설치 프로필을 만들거나 재사용하고, 선택한 저장소를 등록하며,
선택한 호스트의 프로젝트 범위 MCP 설정을 설치하고, 프로젝트 범위 Volicord 지침과 로컬
설정 파일을 쓰고, 통합 상태를 기록합니다. 생성된 호스트
설정은 단일 공개 실행 파일을 프로젝트에 묶인 `volicord mcp --stdio`로 시작합니다.

기본 text 출력은 간결한 온보딩 요약입니다. 실제 경로와 변경 동사는 저장소 상태에 맞게
달라질 수 있지만 형태는 아래처럼 읽으면 됩니다.

```text
Volicord initialized for Codex

Profile:
  record

Repository:
  /path/to/repo

Repo file changes:
  created .codex/config.toml
  created .volicord/policy.json
  updated AGENTS.md

Stored local Volicord state:
  /home/you/.volicord

Next:
  1. Open, restart, or reload Codex in this repository.
  2. Trust or approve the project configuration if Codex asks.
  3. Run volicord connection verify codex --shared --repo /path/to/repo.

Limits:
  The record profile supports cooperative Volicord workflow recording through MCP.
  It does not provide OS sandboxing, network isolation, malware defense,
  full write prevention, actor identity proof, correctness proof, test
  sufficiency proof, or human review completion.
```

이 요약은 Product Repository 안에 쓴 파일과 Runtime Home에 저장한 로컬 Volicord 상태를
구분합니다. 이미 실행 중인 Codex session이 새 설정을 로드하거나 신뢰했다는 뜻은
아닙니다.
사용자 워크플로에서 Record profile은 MCP를 통한 협력적 Volicord 워크플로 기록을
지원합니다. 보안, 정확성, 테스트 충분성, 검토 완료를 보장하지 않습니다.

이 빠른 경로는 host lifecycle hook 설치나 session watcher를 요구하지 않는
Record profile(`--profile record`)을 사용합니다. Detective profile(`--profile detective`)은
모든 필수 host hook phase와 session watcher 관찰에 대한 검증된 지원을 요구합니다. 이
전제조건을 사용할 수 없으면 `--profile record`를 사용하거나, detective를 다시 실행하기 전에
지원되는 호스트, 플랫폼, 저장소 설정을 준비합니다. Detective profile은 협력형 host
decision 신호를 반환하고 watcher coverage 시작 뒤의 미기록 변경을 탐지할 수 있지만
OS 집행, 행위자 증명, 네트워크 격리, 악성 코드 방어, 전체 쓰기 방지, 정확성 증명,
테스트 충분성 증명, 사람 검토 완료, sandbox를 제공하지 않습니다. Native Windows에서는
Windows host hook과 watcher 동작이 구현되고 테스트되기 전까지 detective가 지원되지 않으므로
이 Record profile 빠른 경로를 사용합니다. 정확한 프로젝트 이름, 프로필 동작, 연결 기본값,
내부 식별 정보 동작은
[관리 CLI 참조](../reference/admin-cli.md)를 보세요.

이 `record` 빠른 경로 대신 detective 설정을 선택하면, 생성된 hook 명령은 호스트
session이 저장소 하위 디렉터리에서 시작해도 동작하도록 만들어집니다. Status,
verification, doctor 진단은 `hook_path_safety`를 보고합니다.
`relative_path_unsafe`, `wrapper_missing`, `wrapper_not_executable` 같은 값은 생성된 hook
명령이나 wrapper를 복구하기 전에는 detective host hook이 활성 상태가 아니라는 뜻입니다.

## 설정 확인하기

```sh
volicord doctor
volicord project current
volicord connection status codex --shared --repo /path/to/repo
volicord connection verify codex --shared --repo /path/to/repo
```

완료 상태: status나 verification이 `complete`를 보고하면 연결 준비가 끝난 것입니다.
`action_required`를 보고하면 이름 붙은 호스트 소유 동작이나 로컬 복구 동작을 완료한
뒤 verification을 다시 실행합니다. 정확한 결과 상태 의미는 [관리 CLI
참조](../reference/admin-cli.md#agent-connection-result-states)를 보세요.
Detective host hook 경로 복구 안내는
[에이전트 호스트 문제 해결](../user-guide/agent-host-troubleshooting.md#guard-hook-path-or-wrapper-is-unsafe)이
정리합니다.
CLI verification은 Volicord의 MCP 서버가 터미널 쪽 점검 경로에서 시작하고 응답할 수
있음을 확인할 수 있습니다. 그 자체만으로 Codex, Claude Code 또는 다른 호스트가 프로젝트
설정을 로드하고 신뢰했다는 증명은 아닙니다.

## 호스트 의도 선택하기

personal, global, read-only 변형을 직접 써야 할 때만 낮은 수준의
`volicord connection add` 명령을 사용합니다. 일반 `init` 흐름 없이 `volicord connection add`로
프로젝트 공유 통합 파일을 관리할 때만 `--shared`를 추가하고, `--global`은 사용자
전체 설정을 지원하는 호스트 경로에만 사용합니다. 정확한 의도 의미는
[관리 CLI 참조](../reference/admin-cli.md#connection-intents-and-hosts)를 보세요.
호스트 가용성 요구사항은 [시스템
요구사항](../reference/system-requirements.md#host-configuration-requirements)에 있습니다.

읽기 중심 동작만 노출해야 할 때만 `--read-only`를 사용합니다.

```sh
volicord connection add codex --read-only
```

낮은 수준의 연결 관리에서는 현재 디렉터리가 연결 대상 Product Repository가 아닐 때
`--repo PATH`를 사용합니다.

```sh
volicord connection add codex --repo /path/to/your-product-repo
```

`volicord connection add`는 personal, shared, global, read-only 변형을 위한 낮은 수준의
연결 관리 명령으로 계속 지원됩니다. 일반적인 첫 실행 경로에서는
`volicord init --host HOST --repo PATH --profile record`를 우선 사용합니다.

## 연결 조회 또는 변경하기

```sh
volicord connection list
volicord connection status codex --shared --repo /path/to/repo
volicord connection verify codex --shared --repo /path/to/repo
volicord connection mode codex read-only
volicord connection mode codex workflow
```

선택한 Product Repository를 연결에서 제거할 때도 같은 호스트와 의도 선택을 사용합니다.

```sh
volicord connection remove codex --dry-run
volicord connection remove codex
```

`--dry-run`은 지속 변경 없이 계획을 보고합니다.

## Generic MCP 호스트 사용하기

Volicord가 직접 관리하지 않는 MCP 호스트에는 먼저 지원되는 Agent Connection을 만든 뒤,
외부 호스트의 자체 설정에서
`volicord mcp --stdio --connection <connection_id> [--project <project_id>]`를 시작하도록
구성합니다. 그 설정은 사용자 관리 설정으로 남습니다. Volicord는 임의 외부 호스트가
이 설정을 로드하거나 승인했다고 주장하지 않습니다.

## 사용자 판단 기록하기

Agent Connection은 초점이 맞춰진 판단 필요를 요청하거나 보여 줄 수 있지만,
권한을 지니는 사용자 답변은 로컬 `User Channel`을 거칩니다.

호스트와 클라이언트가 지원하면 MCP 어댑터는 대기 판단에 호스트 프롬프트를 사용할 수
있습니다. detective 상태가 채팅 명령 캡처를 `configured`, `observed`, `active`로 보고할 때
채팅 경로는 `Volicord: answer J-3 1 #AB7K` 같은 엄격한 prompt 명령입니다. 호스트
프롬프트 입력과 채팅 명령 캡처를 사용할 수 없고 adapter가 fallback을 안전하게 노출할 수
있으면 Volicord는 짧게 만료되는 일회성 token이 있는 loopback local consent URL을 반환할
수 있습니다. 다른 User Channel 입력 방법을 사용할 수 없거나 수동 점검이 필요할 때는
아래 터미널 명령을 안정적인 CLI inbox 경로로 사용합니다.

```sh
volicord inbox
volicord inbox answer JUDGMENT_ID --choice CHOICE_ID
```

현재 Product Repository와 다른 Product Repository에 답해야 할 때만 `--repo PATH`를
사용합니다. 활성 작업이 의도한 작업이 아닐 때는 `--task ID`를 사용합니다.

## 다음 단계

| 필요 | 읽을 문서 |
|---|---|
| 호스트 설정 세부사항 | [에이전트 호스트 설정](../user-guide/agent-host-setup.md) |
| `action_required` 또는 `failed` 문제 해결 | [에이전트 호스트 문제 해결](../user-guide/agent-host-troubleshooting.md) |
| 사용자 작업 흐름과 판단 경계 | [사용자 가이드](../user-guide/user-workflow.md) |
| 에이전트 작업 흐름 경계 | [에이전트 가이드](../user-guide/agent-workflow.md) |
