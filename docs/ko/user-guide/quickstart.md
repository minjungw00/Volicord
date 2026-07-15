# 빠른 시작

이 튜토리얼은 내장 관리 설정을 사용해 Codex 또는 Claude Code 호스트 하나를 Product
Repository 하나에 연결합니다. [설치](installation.md)를 따라 `volicord`를 `PATH`에서
사용할 수 있게 만든 뒤 시작합니다.

정확한 명령 동작은 [관리 CLI](../reference/admin-cli.md), 정확한 연결 의미는
[Agent Connection 참조](../reference/agent-connection.md)를 보세요.

## 1. 저장소 연결 초기화

Codex는 아래 명령을 실행합니다.

```sh
volicord init --shared --host codex --repo "<repo>" --profile record
```

`<repo>`는 에이전트가 작업할 Git 저장소입니다. Claude Code는
`--host claude-code`를 사용합니다.

이 명령은 로컬 Volicord 상태를 만들거나 재사용하고, 저장소를 등록하며, 프로젝트
범위 MCP 설정과 지침을 씁니다. 생성된 호스트 설정은 이 연결을 위한
`volicord mcp --stdio`를 시작합니다.

이 경로는 공유 저장소 설정을 만들므로 init이 선택한 것과 같은 비어 있지 않은 절대
경로 `VOLICORD_HOME`을 제공하는 환경에서 호스트를 시작해야 합니다. 생성된 항목은
호스트의 값을 전달하며 머신 로컬 Runtime Home 경로를 내장하지 않습니다. 값이 없을 때
플랫폼 기본값으로 대체하지도 않습니다.

명령 출력의 `Next` 섹션을 읽습니다. 호스트 재시작, 프로젝트 신뢰, MCP 항목 승인처럼
남은 호스트 소유 동작을 알려 줍니다. 설정 파일을 썼다는 사실만으로 실행 중인
호스트가 그 설정을 불러왔다고 단정할 수 없습니다.

첫 연결에는 기록 프로필을 사용합니다. 이 프로필은 호스트 생명주기 hook이나 세션
감시기를 요구하지 않습니다. 탐지 프로필의 추가 호스트, 플랫폼, 저장소 전제 조건은
[에이전트 호스트 설정](agent-host-setup.md#integration-profiles)을 보세요.

## 2. 호스트 동작 완료

설정 뒤에는 선택한 호스트를 Product Repository에서 열거나 다시 시작합니다.

| 호스트 | 호스트에서 확인할 것 |
|---|---|
| Codex | 프로젝트 신뢰 요청을 처리한 뒤 현재 세션에 Volicord 도구가 보이는지 확인합니다. |
| Claude Code | 프로젝트 MCP 승인 요청을 처리하고 `/mcp`를 확인한 뒤 현재 세션에 Volicord 도구가 보이는지 확인합니다. |

이미 실행 중인 호스트는 이전 `PATH`나 설정 사본을 계속 사용할 수 있습니다. 호스트가
`volicord`를 시작하지 못하면 명령을 찾을 수 있는 환경에서 다시 시작합니다.

## 3. 연결 검증

위에서 만든 Codex 연결은 아래처럼 검증합니다.

```sh
volicord connection verify codex --shared --repo "<repo>"
volicord connection status codex --shared --repo "<repo>"
```

Claude Code는 `codex` 대신 `claude-code`를 사용합니다.

사람이 읽는 출력에서는 `Status`, `Checks`, `Next`, `Diagnostics`를 확인합니다.

- `complete`: 이 설정 경로가 요구하는 점검이 준비되었습니다.
- `action_required`: 이름 붙은 로컬 또는 호스트 동작이 남았습니다. 해당 동작을
  완료하고 검증을 다시 실행합니다.
- `failed`: 필수 점검이 성공하지 않았습니다.

`complete` 결과는 이 설정 경로의 준비 상태만 성립시키며 모든 호스트 기능의 지원을
뜻하지 않습니다. JSON의 `states.host_feature_support`는 여섯 기능을 각각 `verified`,
`implemented_unverified`, `unsupported_by_host`, `temporarily_unavailable` 중 하나로
보고합니다. 현재 기능 지원을 주장할 수 있는 상태는 `verified`뿐입니다. 정확한 계약은
[호스트 기능 지원 상태](../reference/agent-connection.md#host-feature-support-state)를
보세요.

자동화나 전체 진단에는 `--json`을 사용합니다. 간결한 사람용 출력을 파싱하지
않습니다. 정확한 결과 상태 의미는
[관리 CLI](../reference/admin-cli.md#agent-connection-result-states)에 있습니다.

CLI 검증은 점검 환경에서 MCP 프로세스를 시작하고 통신할 수 있습니다. 그 결과만으로
현재 호스트 세션에 도구가 보인다고 단정할 수 없습니다. 현재 호스트 세션에서 다음
읽기 전용 호출을 요청합니다.

1. `volicord.list_projects`
2. `volicord.status`

이 호출은 `Task`를 만들지 않고 도구 노출, 프로젝트 선택, 프로젝트 상태 읽기를
확인합니다.

읽기 호환 도구만 보이거나 Volicord 도구가 전혀 보이지 않으면
[에이전트 호스트 문제 해결](agent-host-troubleshooting.md)을 사용합니다. 이 문서는
호스트 신뢰, 명령 가용성, 현재 세션의 도구 노출, 런타임 홈 쓰기 역량을 구분합니다.

## 4. 일반 작업 시작

평소 말로 작업을 요청합니다.

```text
현재 인증 흐름을 확인하고 요청한 잠금 안내 문구를 추가해줘. 집중 점검도 실행하고 아직 닫기를 막는 것을 알려줘.
```

에이전트는 현재 작업, 범위, 증거, 대기 중인 사용자 행동, 닫기 상태를 보이게 유지해야
합니다. 사용자 소유 행동을 기록해야 하면 Volicord가 보여 주는 사용자 채널 경로를
사용합니다. 안정적인 CLI 대체 경로는 아래와 같습니다.

```sh
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

## 빠른 경로로 충분하지 않을 때

개인, 전역, 읽기 전용 연결이나 여러 저장소 운영, 명시적 제거가 필요할 때만 낮은
수준의 연결 명령을 사용합니다. 자세한 선택지는
[에이전트 호스트 설정](agent-host-setup.md)과
[여러 저장소 에이전트 설정](multi-repository-agent-setup.md)에 있습니다.

| 필요 | 읽을 문서 |
|---|---|
| 호스트별 설정과 제거 | [에이전트 호스트 설정](agent-host-setup.md) |
| `action_required`, `failed`, 도구 누락 | [에이전트 호스트 문제 해결](agent-host-troubleshooting.md) |
| 사용자 작업 흐름과 판단 경계 | [사용자 작업 흐름](user-workflow.md) |
| 에이전트 작업 지침 | [에이전트 가이드](agent-workflow.md) |
