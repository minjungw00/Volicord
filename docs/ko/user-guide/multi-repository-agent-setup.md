# 다중 저장소 에이전트 설정

개인 Codex 연결 하나가 등록된 Product Repository 둘 이상을 명시적으로 처리해야 할
때 이 가이드를 사용합니다. 저장소 하나 또는 프로젝트 공유 연결에는
[에이전트 호스트 설정](agent-host-setup.md)을 사용합니다.

## 토폴로지

저장된 Agent Connection 하나는 명시적인 Connection Projects membership을 가질 수
있습니다. stdio 프로세스는 그 연결에 결속됩니다. 요청은 allowlist에 속한 프로젝트만
선택할 수 있으며 cwd, 디렉터리 검색, 저장소 이름 추측은 membership을 추가하지 않습니다.

## 저장소 연결

첫 저장소를 등록합니다.

```sh cli-example
volicord connection add codex --repo /path/to/acme-api
volicord connection status codex --repo /path/to/acme-api
```

같은 개인 의도로 다른 명시적 membership을 추가합니다.

```sh cli-example
volicord connection add codex --repo /path/to/billing-api
volicord connection status codex --repo /path/to/billing-api
```

결과 연결과 membership을 확인합니다.

```sh cli-example
volicord connection list
volicord connection verify codex
```

## 에이전트 선택

정확한 연결 binding으로 생성된 숨겨진 launcher를 통해 관리 MCP 프로세스를 시작합니다.
에이전트는 `volicord.list_projects`를 호출해 허용된 `project_id` 하나를 선택하고
프로젝트 범위 호출에 그 identity를 전달해야 합니다. 모호하거나 목록에 없는 선택은
fail closed해야 합니다.

각 Product Repository는 자체 Task, scope, 쓰기 티켓, run, evidence, continuity,
UserAction 요청, 닫기 상태를 유지합니다. Membership이 프로젝트 권한을 합치지는
않습니다.

## membership 하나 제거

이름 붙인 저장소 membership을 미리 보고 제거합니다.

```sh cli-example
volicord connection remove codex --repo /path/to/billing-api --dry-run
volicord connection remove codex --repo /path/to/billing-api
```

Codex를 다시 시작하기 전에 남은 membership을 다시 확인합니다. 이 명령은 선택한
membership의 Registry binding과 Guard Installation을 삭제하지만 Agent Connection,
runtime session, 공유 host configuration, 다른 membership의 통합 행은 유지합니다.
Agent Connection과 일치하는 host configuration은 마지막 membership에서만 제거합니다.
다른 Product Repository와 그 프로젝트 로컬 이력은 유지합니다.

## 경계

- 명시적으로 저장된 membership만 선택할 수 있습니다.
- 최초 릴리스는 관리 stdio 위의 Codex `record` 프로필을 사용합니다.
- 공유 연결은 Product Repository 하나의 범위에 남습니다.
- UserAction 해결은 선택한 저장소의 로컬 CLI 동작으로 남습니다.
- 정확한 라우팅은 [Agent Connection](../reference/agent-connection.md)과
  [MCP 전송](../reference/mcp-transport.md)이 담당합니다.
