# 빠른 시작

이 튜토리얼은 지원되는 관리 설정으로 Codex 설치 하나를 Product Repository 하나에
연결합니다. 먼저 [설치](installation.md)를 완료해 `volicord`를 `PATH`에서
사용할 수 있게 합니다.

## 1. 연결 초기화

저장소 공유 연결은 다음과 같이 만듭니다.

```sh cli-example
volicord init --shared --host codex --repo "<repo>" --profile record
```

개인 연결은 같은 명령에서 `--shared`를 뺍니다. `<repo>`는 Codex가 작업할 Git
작업 트리입니다. 지원되는 selector 집합은 호스트 `codex`, 프로필 `record`,
연결 의도 `personal` 또는 `shared`입니다.

프로젝트 소유 구성을 커밋하기 전에 보고된 파일 변경을 검토합니다. Volicord가
관리하는 프로젝트 파일에는 `.codex/config.toml`, `.volicord/policy.json`,
`AGENTS.md`의 관리 블록이 포함될 수 있습니다.

## 2. Codex 동작 완료

선택한 저장소에서 Codex를 시작하거나 다시 불러옵니다. Codex가 프로젝트 신뢰 단계를
표시하면 완료하고, 현재 세션이 `volicord.*` 도구를 찾을 수 있는지 확인합니다.
디스크의 구성만으로 이미 실행 중인 세션이 이를 읽었다고 증명할 수는 없습니다.

## 3. 연결 검증

연결을 초기화할 때 사용한 것과 같은 의도 선택자를 사용합니다.

```sh cli-example
volicord connection verify codex --shared --repo "<repo>"
volicord connection status codex --shared --repo "<repo>"
```

`complete` 검증 결과는 연결 준비 체크포인트입니다. Codex가 지침을 따랐음, 저장소
쓰기가 sandbox로 격리됨, Task가 닫기 준비됨을 증명하지 않습니다. 반환된
`action_required` 항목을 처리한 뒤 다시 검증합니다.

## 4. 작업 시작

`volicord.status`로 시작합니다. 반환된 `next_action`을 따르고, 제품 파일 쓰기 전에
쓰기 티켓을 얻고, 작업과 증거를 기록하며, 완료 전에 닫기 준비 상태를 확인합니다.

에이전트가 대기 중인 `UserActionRequest`를 만들었다면 로컬 CLI User Channel만
이를 해결할 수 있습니다.

```sh cli-example
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

MCP 에이전트는 요청을 만들거나 재개하고 나중에 현재 상태를 관찰할 수 있지만 요청을
해결할 수는 없습니다.

## 다음 경로

- 개인/공유 선택, 미리 보기, 검증, 복구, 제거는
  [에이전트 호스트 설정](agent-host-setup.md)을 봅니다.
- 제한된 복구는 [에이전트 호스트 문제 해결](agent-host-troubleshooting.md)을 봅니다.
- 일반 Core 흐름은 [에이전트 작업 흐름](agent-workflow.md)을 봅니다.
- 정확한 지원 경계는 [범위](../reference/scope.md)를 봅니다.
