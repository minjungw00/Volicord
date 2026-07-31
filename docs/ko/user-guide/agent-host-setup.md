# 에이전트 호스트 설정

관리 Codex Agent Connection을 설치, 검증, 복구, 제거할 때 이 가이드를 사용합니다.
정확한 명령 계약은 [관리 CLI](../reference/admin-cli.md), 관리 운영 session 경계는
[Agent Connection](../reference/agent-connection.md)이 담당합니다.

## 지원 설정

지원되는 설정은 다음과 같은 닫힌 값 집합을 사용합니다.

- `host_kind=codex`
- `profile=record`
- `scope=personal` 또는 `scope=shared`
- 새 연결은 `volicord connection add --read-only`를 선택하지 않으면 `workflow`로
  시작하며, 이후 설정은 기존 `workflow` 또는 `read_only` mode를 보존
- 생성된 숨겨진 launcher를 통한 관리 stdio MCP 전송

공유 설정을 만들거나 복구합니다.

```sh cli-example
volicord init --shared --host codex --repo "<repo>" --profile record
```

개인 설정은 `--shared`를 뺍니다. 이후 상태, 검증, 복구, 제거 명령에도 같은 선택자를
사용합니다.

<a id="read-setup-output"></a>
## 설정 출력 읽기

출력 플래그를 지정하지 않으면 `init`과 선택한 Connection의 생명주기 명령은 터미널에
현재 보고서를 운영자가 읽을 수 있게 요약합니다. 먼저 설정 변경이 commit되었는지와 현재
결과에 주의가 필요한지를 확인하고, 계산된 호스트 소유 activation 단계가 얼마나 남았는지
봅니다. 선택한 Product Repository와 Connection mode를 확인하고, integration activation
상태와 프로젝트 hook activation 상태를 서로 구분해 읽습니다.

보고서는 passed, blocked, pending, failed check 개수를 각각 계산합니다. 이 개수로 전체
진행 상황을 파악한 뒤 보고된 실행 순서대로 필수 호스트 소유 단계를 완료합니다. 실행 파일,
Store 쓰기 가능성, protocol conformance, host compatibility에 관한 최신 probe 근거가 필요할
때만 선택적인 active diagnostics를 실행합니다.

현재 보고서를 더 자세히 진단할 때는 `--verbose`를 사용합니다.

```sh cli-example
volicord connection verify codex --shared --repo "<repo>" --verbose
```

[관리 CLI](../reference/admin-cli.md#agent-connection-result-states)는 정확한 concise 및
verbose 출력, JSON 필드, 안정적인 enum 철자, check 상태 정의, 종료 동작, 현재 상태를 사용할
수 없을 때의 동작을 규정하는 기준 문서입니다.

## 관리 변경 검토

설정을 수락하기 전에 구조화된 결과와 모든 관리 파일을 검토합니다. 프로젝트 소유
구성에는 `.codex/config.toml`, `.volicord/policy.json`, `AGENTS.md`의 Volicord
관리 블록이 포함될 수 있습니다. 설정은 관련 없는 사용자 내용을 덮어쓰면 안 됩니다.

하위 수준 연결 변경은 dry run으로 먼저 확인합니다.

```sh cli-example
volicord connection add codex --repo "<repo>" --dry-run
volicord connection add codex --repo "<repo>" --read-only --dry-run
volicord connection remove codex --repo "<repo>" --dry-run
```

기본 dry-run 출력은 소유권 `kind`별로 계획 변경을 묶습니다. 적용 전에 정확한 `operation`과
`target`을 모두 확인하려면 `--verbose`를 추가합니다.

일치하는 현재 Connection에서는 일반 `connection add`와 이미 `read_only`인 Connection에
`--read-only`를 지정한 `connection add` 모두 replay 또는 repair로 동작합니다. 플래그를
생략해도 `workflow`를 요청하는 것이 아닙니다. 기존 mode를 바꿀 때는 항상
`volicord connection mode`를 사용합니다.

## 검증

Codex가 구성을 읽고 필요한 신뢰 동작을 마친 뒤 실행합니다.

```sh cli-example
volicord connection verify codex --shared --repo "<repo>"
volicord connection status codex --shared --repo "<repo>"
volicord connection list --repo "<repo>"
```

`verify`는 설정과 준비 상태를 진단하는 명령으로 사용합니다. 현재 Connection과 session의
권한 의미는 [검증된 에이전트 세션](../reference/agent-connection.md#validated-agent-session)이
담당합니다.

직접 프로세스 사전 점검에는 정확한 저장 식별자를 사용합니다.

```sh cli-example
volicord mcp preflight --connection "<connection_id>" --project "<project_id>"
```

일반 관리 동작에서는 생성된 Codex 구성이 launch 맥락을 제공해
숨겨진 launcher를 통해 관리 MCP를 시작합니다. Marker는 협력적 routing 입력일 뿐 credential이
아닙니다. cwd에서 personal Connection을 추론하거나 주변 저장소를 검색하지 않습니다.

## UserAction 경계

MCP 에이전트는 `volicord.request_user_action`으로 대기 요청을 만들거나 읽기 전용
재개 동작을 사용할 수 있습니다. 사람은 CLI inbox로만 이를 해결합니다.

```sh cli-example
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

Guard의 prompt 관련 관찰은 진단 입력일 뿐입니다. UserAction resolution을 만들지
않으며 명시적인 CLI 명령을 대신하지 않습니다.

## 복구

`volicord doctor`를 실행한 뒤 정확히 같은 연결 의도의 `init` 명령을 다시 실행합니다.
diff를 다시 검토하고 안내된 경우 Codex를 재시작하거나 다시 불러옵니다. 이 복구는
`workflow`와 `read_only` mode 모두에서 기존 mode를 유지합니다. Mode를 전환하려면
`volicord connection mode`를 사용합니다. 복구는 관련 없는 구성과 제품 데이터를
보존해야 합니다.

## 제거

먼저 미리 보고 같은 의도를 제거합니다.

```sh cli-example
volicord connection remove codex --shared --repo "<repo>" --dry-run
volicord connection remove codex --shared --repo "<repo>"
```

제거는 결과가 이름 붙인 Volicord 관리 통합 자료만 삭제합니다. 여기에는 선택한
membership의 Registry binding과 Guard Installation이 포함됩니다. 다른 저장소
membership이 남아 있는 동안 Agent Connection과 connection 전체 runtime session은
유지되며, 마지막 membership은 일치하는 host configuration을 제거한 뒤 이 행들도
삭제합니다. 프로젝트 로컬 Agent Session, Guard 및 workflow 이력, evidence와 그 밖의
권한 기록은 유지합니다. Product Repository, 다른 저장소, 관련 없는 Codex 구성은
보존합니다.

## 관련 가이드

- [빠른 시작](quickstart.md)
- [에이전트 호스트 문제 해결](agent-host-troubleshooting.md)
- [다중 저장소 에이전트 설정](multi-repository-agent-setup.md)
- [시스템 요구사항](../reference/system-requirements.md)
