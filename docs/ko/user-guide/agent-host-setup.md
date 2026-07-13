# 에이전트 호스트 설정

Codex나 Claude Code 에이전트 연결을 설정, 검증, 변경, 제거할 때 이 가이드를
사용합니다. 가장 짧은 첫 설정은 [빠른 시작](quickstart.md)에서 시작합니다.

정확한 명령 동작은 [관리 CLI](../reference/admin-cli.md)를 보세요. 에이전트 연결과
런타임의 정확한 경계는 [Agent Connection 참조](../reference/agent-connection.md)와
[런타임 경계](../reference/runtime-boundaries.md)에 있습니다.

## 일반 설정

`volicord`를 설치한 뒤 Product Repository 연결을 초기화합니다.

```sh
volicord init --host codex --repo "<repo>" --profile record
```

Claude Code는 `--host claude-code`를 사용합니다. `<repo>`는 에이전트가 작업할 Git
저장소입니다.

이 명령은 런타임 홈과 설치 프로필을 만들거나 재사용하고, 저장소를 등록하고, 에이전트
연결을 만들며, 프로젝트 범위 MCP 설정과 지침을 씁니다. 생성된 설정은 선택한
연결을 위한 `volicord mcp --stdio`를 시작합니다.

설정 과정에서 Product Repository 안에도 파일을 씁니다. 저장소의 일반 설정 정책에
따라 파일을 검토합니다.

| 호스트 | 일반적인 프로젝트 파일 |
|---|---|
| Codex | `.codex/config.toml`, `.volicord/policy.json`, `AGENTS.md`의 Volicord 관리 블록 |
| Claude Code | `.mcp.json`; 탐지 설정은 `.claude/settings.json`, `.claude/rules/volicord.md`, `.claude/hooks/`도 추가할 수 있습니다. |

다른 기여자나 자동화와 설정을 공유할 때만 이 파일들을 커밋합니다. Product Repository
설정과 `Volicord Runtime Home`에 저장되는 운영 데이터는 서로 다릅니다.

초기화가 끝나면 `Next`에 이름 붙은 호스트 소유 단계를 완료합니다.

- 호스트 재시작 또는 다시 불러오기
- 요청받은 경우 Codex 프로젝트 신뢰
- 요청받은 경우 Claude Code 프로젝트 MCP 항목 승인
- 이름 붙은 경우 `PATH` 또는 명령 가용성 복구

그다음 연결을 검증합니다.

```sh
volicord connection verify codex --shared --repo "<repo>"
volicord connection status codex --shared --repo "<repo>"
```

Claude Code는 `codex` 대신 `claude-code`를 사용합니다.

## 검증이 확인할 수 있는 것

연결 검증은 여러 계층으로 나뉩니다. 앞선 계층의 성공만으로 뒤 계층의 성공을
추론하지 않습니다.

| 계층 | 답하는 질문 |
|---|---|
| 관리 설정 | 선택한 호스트 설정이 Volicord가 관리하는 연결과 일치하는가? |
| 호스트 신뢰 또는 승인 | 호스트가 사용자가 처리할 신뢰, 승인, 대기, 거절 상태를 보고했는가? |
| CLI MCP 점검 | 검증 프로세스가 MCP 프로세스를 시작하고 통신할 수 있는가? |
| 현재 호스트 노출 | 현재 호스트 세션에서 Volicord 도구를 보고 호출할 수 있는가? |
| 저장 역량 | 해당 MCP 프로세스가 레지스트리와 프로젝트 상태를 읽고, 작업 흐름 도구가 필요할 때 프로젝트 상태를 쓸 수 있는가? |

기본 출력에서는 `Status`, `Checks`, `Next`, `Diagnostics`를 읽습니다. 전체 진단이나
자동화에는 `--json`을 사용합니다. 스크립트에서 간결한 사람용 출력을 파싱하면 안
됩니다.

CLI MCP 성공은 현재 호스트의 도구 노출을 증명하지 않습니다. 현재 Codex나 Claude
Code 세션 안에서 도구 가용성을 확인합니다. 도구가 없으면 설정을 직접 고치기 전에
[에이전트 호스트 문제 해결](agent-host-troubleshooting.md)을 따릅니다.

## Codex

일반적인 프로젝트 범위 경로는 아래와 같습니다.

```sh
volicord init --host codex --repo "<repo>" --profile record
volicord connection verify codex --shared --repo "<repo>"
```

초기화 뒤에는 다음을 확인합니다.

1. Product Repository에서 Codex를 열거나 다시 시작합니다.
2. 프로젝트 신뢰 요청을 처리합니다.
3. 현재 세션에 `volicord.*` 도구가 보이는지 확인합니다.
4. 호스트에 `volicord.list_projects`, `volicord.status` 순서로 호출하게 합니다.

터미널 쪽 점검은 통과하지만 도구가 없다면 나중에 연 터미널만 보지 말고 Codex를
시작한 환경을 확인합니다.

```sh
command -v volicord
```

IDE, 원격 실행기, 비대화형 실행에서는 해당 호스트의 MCP 시작 환경과 로그를
확인합니다. [Codex 문제 해결 경로](agent-host-troubleshooting.md#trusted-codex-project-but-host-runtime-is-not-observed)를
보세요.

Codex 소유 도구 승인 설정은 Volicord 관리 MCP 설정과 함께 있을 수 있습니다.
`volicord` 서버 항목 아래에 있다는 이유만으로 승인 오버레이를 삭제하지 않습니다.
검증이 불일치를 보고하면
[설정 불일치 문제 해결 경로](agent-host-troubleshooting.md#codex-approval-overlay-reported-as-mcp-configuration-changed)를
사용합니다.

## Claude Code

일반적인 프로젝트 범위 경로는 아래와 같습니다.

```sh
volicord init --host claude-code --repo "<repo>" --profile record
volicord connection verify claude-code --shared --repo "<repo>"
```

초기화 뒤에는 다음을 확인합니다.

1. Product Repository에서 Claude Code를 열거나 다시 시작합니다.
2. 프로젝트 MCP 승인 요청을 처리합니다.
3. 호스트 연결 상태를 확인합니다.

   ```sh
   claude mcp list
   claude mcp get volicord
   ```

4. 현재 Claude Code 세션에서 `/mcp`를 확인합니다.
5. 호스트에 `volicord.list_projects`, `volicord.status` 순서로 호출하게 합니다.

`.mcp.json`이나 `claude mcp get` 출력이 일치한다는 사실만으로 현재 세션의 도구
노출을 증명할 수 없습니다. 현재 세션에 Volicord 도구가 없다면
[Claude Code 문제 해결 경로](agent-host-troubleshooting.md#claude-code-configuration-exists-but-tools-are-not-exposed)를
사용합니다.

<a id="integration-profiles"></a>
## 통합 프로필

| 프로필 | 사용할 때 | 추가되는 것 |
|---|---|---|
| 기록 프로필(`record`) | 호스트 생명주기 훅이나 세션 감시기 없이 일반 MCP 작업 흐름을 사용할 때 | 협력적 작업 기록과 프로젝트 지침 |
| 탐지 프로필(`detective`) | 선택한 호스트, 플랫폼, 저장소가 모든 필수 훅과 감시기 전제 조건을 충족할 때 | 미기록 변경 신호를 포함한 지원되는 호스트 훅과 감시기 관찰 |

어느 프로필도 OS 샌드박스, 네트워크 격리, 행위자 증명, 정확성 증명, 전체 쓰기
방지를 제공하지 않습니다. 탐지 관찰은 신호이며 쓰기 티켓을 파일시스템 강제로
바꾸지 않습니다.

Windows 네이티브 환경에서는 `--profile record`를 사용합니다. 탐지 호스트 훅 래퍼와 세션
감시기는 지원되지 않습니다. 정확한 프로필 동작과 실패 조건은
[관리 CLI](../reference/admin-cli.md#agent-host-setup-and-init)에 있습니다.

탐지 프로필 설정이 안전하지 않거나 빠진 훅 경로를 보고하면 생성된 래퍼를 직접 고치지
말고 같은 호스트와 저장소에 대해 탐지 초기화를 다시 실행합니다.

```sh
volicord init --host codex --repo "<repo>" --profile detective
```

이후 호스트 재시작, 신뢰, 승인 단계를 완료하고 검증을 다시 실행합니다. 각 진단 값은
[훅 경로 또는 래퍼가 안전하지 않음](agent-host-troubleshooting.md#guard-hook-path-or-wrapper-is-unsafe)을
보세요.

## 낮은 수준의 연결 선택

`volicord init`이 기본 첫 실행 경로입니다. 연결 의도나 모드를 직접 골라야 할 때
`volicord connection add`를 사용합니다.

### 연결 의도

| 의도 | 명령 형태 | 용도 |
|---|---|---|
| `personal` | `volicord connection add codex --repo "<repo>"` | 현재 사용자 소유 호스트 설정 |
| `shared` | `volicord connection add codex --shared --repo "<repo>"` | Product Repository 안의 프로젝트 공유 설정 |
| `global` | `volicord connection add claude-code --global --repo "<repo>"` | 명시적으로 연결된 프로젝트를 사용하는 사용자 전체 Claude Code 설정 |

Codex는 `personal`, `shared` 의도를 지원합니다. Claude Code는 `personal`, `shared`,
`global` 의도를 지원합니다. `--shared`와 `--global`을 함께 사용할 수 없습니다.

호스트 수준 연결 하나가 여러 저장소를 처리해야 하면
[여러 저장소 에이전트 설정](multi-repository-agent-setup.md)을 사용합니다.

### 연결 모드

기본은 작업 흐름 모드입니다. 호스트가 작업 흐름 변경 도구 없이 프로젝트와 상태만
확인해야 하면 읽기 전용 모드를 사용합니다.

```sh
volicord connection add codex --repo "<repo>" --read-only
volicord connection mode codex read-only --repo "<repo>"
volicord connection mode codex workflow --repo "<repo>"
```

작업 흐름 연결이라도 MCP 호스트가 프로젝트 상태를 읽을 수만 있고 쓸 수 없다면 읽기
호환 도구만 노출할 수 있습니다. 이는 새 연결 모드가 아니라 저장 역량 문제입니다.
[읽기 전용 호스트 저장소](agent-host-troubleshooting.md#read-only-host-storage)에서
진단합니다.

## 변경 미리 보기와 조회

관리 설정을 바꾸기 전에 변경 계획을 확인합니다.

```sh
volicord connection add codex --repo "<repo>" --dry-run
volicord connection remove codex --repo "<repo>" --dry-run
```

기존 연결은 아래처럼 확인합니다.

```sh
volicord connection list --repo "<repo>"
volicord connection status codex --repo "<repo>"
volicord connection verify codex --repo "<repo>"
```

호스트와 저장소에 둘 이상의 연결이 일치하면 생성할 때 사용한 `--shared`나
`--global` 같은 의도 플래그를 추가합니다.

## 간단 점검

작업 흐름 상태를 만들기 전에 읽기 전용 점검을 사용합니다.

1. 선택한 호스트와 저장소에 `volicord connection verify`를 실행합니다.
2. 현재 호스트에서 `volicord.list_projects`를 호출합니다.
3. 의도한 프로젝트에 `volicord.status`를 호출합니다.

이 순서는 설정, 도구 노출, 프로젝트 선택, 프로젝트 상태 읽기를 점검합니다. `Task`를
만들지 않아야 합니다.

Volicord 상태를 만들어도 될 때만 작업 흐름 변경 호출을 사용합니다. 정확한 공개
메서드 순서는 [API 메서드](../reference/api/methods.md)와 각 집중 담당 문서에 있습니다.

## 일반 MCP 호스트

Volicord가 관리하지 않는 호스트에는 먼저 지원되는 에이전트 연결을 만듭니다.
그다음 외부 호스트의 자체 설정에서 아래 프로세스를 시작하게 합니다.

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
```

외부 설정은 사용자 관리 상태로 남습니다. Volicord는 임의의 호스트가 설정을
불러오거나 승인했다고 주장하지 않습니다. 정확한 프로세스와 프로젝트 선택 동작은
[MCP 전송](../reference/mcp-transport.md)을 보세요.

## 사용자 채널 경계

에이전트 연결은 초점이 맞춰진 사용자 행동을 요청하거나 보여 줄 수 있습니다.
사용자 resolution을 대신 기록하지는 않습니다. Volicord가 대기 행동을 보여 주면 함께
제공된 사용자 채널 경로를 사용합니다. 안정적인 CLI 경로는 저장된 양식을 먼저 나열한
뒤 해결합니다. 선택 양식은 다음과 같습니다.

```sh
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

Evidence 관찰 양식은 표시된 수락 기준 또는 보충 주장, 아티팩트 ID, summary, 선택적
contradicted 플래그를 사용합니다. 자세한 내용은
[사용자 작업 흐름](user-workflow.md#use-evidence-without-replacing-judgment)을 보세요.

## 제거

선택한 저장소 멤버십을 미리 보고 제거합니다.

```sh
volicord connection remove codex --repo "<repo>" --dry-run
volicord connection remove codex --repo "<repo>"
```

소유권과 안전 점검이 허용할 때만 일치하는 관리 호스트 설정을 제거합니다.
`Product Repository`, 프로젝트 상태, Volicord 기록, 증거 첨부, 관련 없는 호스트
설정은 삭제하지 않습니다.

## 문제 해결 경로

| 증상 | 읽을 문서 |
|---|---|
| 실행 파일이나 설치 프로필이 없음 | [설치](installation.md) |
| 설정이 `action_required` 또는 `failed`를 보고함 | [에이전트 호스트 문제 해결](agent-host-troubleshooting.md) |
| 정확한 명령 동작이 불분명함 | [관리 CLI](../reference/admin-cli.md) |
| 런타임 홈과 Product Repository 분리가 중요함 | [런타임 경계](../reference/runtime-boundaries.md) |
