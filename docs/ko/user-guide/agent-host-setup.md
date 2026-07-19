# 에이전트 호스트 설정

관리 Codex Agent Connection을 설치, 검증, 복구, 제거할 때 이 가이드를 사용합니다.
정확한 명령 계약은 [관리 CLI](../reference/admin-cli.md), 관리 운영 session 경계는
[Agent Connection](../reference/agent-connection.md)이 담당합니다.

## 지원 설정

최초 릴리스에는 관리 호스트 하나와 프로필 하나가 있습니다.

- `host_kind=codex`
- `profile=record`
- `scope=personal` 또는 `scope=shared`
- 새 연결은 `volicord connection add --read-only`를 선택하지 않으면 `workflow`로
  시작하며, 이후 설정은 기존 `workflow` 또는 `read_only` mode를 보존
- 관리 `volicord mcp --stdio` 전송

공유 설정을 만들거나 복구합니다.

```sh
volicord init --shared --host codex --repo "<repo>" --profile record
```

개인 설정은 `--shared`를 뺍니다. 이후 상태, 검증, 복구, 제거 명령에도 같은 선택자를
사용합니다.

## 설정 출력 읽기

출력 플래그를 지정하지 않으면 `init`과 선택한 Connection의 생명주기 명령은 터미널에
간결한 산문을 표시합니다. 새 설정을 적용했지만 관리 호스트 활동이 더 필요한 경우에는
다음과 같은 대표 출력이 나타납니다.

```text
Volicord setup was applied and needs one more step.

Repository: <repo>
Mode: workflow
Checks: 5 ready, 4 waiting

Waiting
  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call
  Guard hook activity: pre_tool, post_tool, prompt_capture

Next
  Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool.

Run again with --verbose for detailed diagnostics.
```

개수와 구역은 현재 보고서에 따라 달라집니다. 모든 check, 지원용 식별자, 정확한 계획
`target`, 보장 한계가 필요하면 `--verbose`를 사용합니다. 완전한 기계 판독 보고서가 필요하면
`--json`을 사용합니다. 두 플래그는 함께 사용할 수 없습니다. 정확한 출력과 종료 동작은
[관리 CLI](../reference/admin-cli.md#agent-connection-result-states)가 담당합니다.

## 관리 변경 검토

설정을 수락하기 전에 구조화된 결과와 모든 관리 파일을 검토합니다. 프로젝트 소유
구성에는 `.codex/config.toml`, `.volicord/policy.json`, `AGENTS.md`의 Volicord
관리 블록이 포함될 수 있습니다. 설정은 관련 없는 사용자 내용을 덮어쓰면 안 됩니다.

하위 수준 연결 변경은 dry run으로 먼저 확인합니다.

```sh
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

```sh
volicord connection verify codex --shared --repo "<repo>"
volicord connection status codex --shared --repo "<repo>"
volicord connection list --repo "<repo>"
```

`verify`는 선택한 관리 구성을 검사하고 현재 3상태 보고서를 반환합니다. 런타임 권한을
발급하지 않습니다. 권한은 MCP project 호출마다 현재 Connection, membership, mode,
권위 있는 관리 runtime/project session을 기준으로 검증합니다.

직접 프로세스 사전 점검에는 정확한 저장 식별자를 사용합니다.

```sh
volicord mcp --check --connection "<connection_id>" --project "<project_id>"
```

일반 관리 동작에서는 생성된 Codex 구성이 launch 맥락을 제공해
`volicord mcp --stdio`를 시작합니다. Marker는 협력적 routing 입력일 뿐 credential이
아닙니다. cwd에서 personal Connection을 추론하거나 주변 저장소를 검색하지 않습니다.

## UserAction 경계

MCP 에이전트는 `volicord.request_user_action`으로 대기 요청을 만들거나 읽기 전용
재개 동작을 사용할 수 있습니다. 사람은 CLI inbox로만 이를 해결합니다.

```sh
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

```sh
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
