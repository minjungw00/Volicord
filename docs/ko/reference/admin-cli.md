# 관리 CLI 참조

이 문서는 지원되는 로컬 관리 명령 표면을 담당합니다. 공개 Core API 메서드 동작은
[API 메서드](api/methods.md), 관리 stdio 동작은 [MCP 전송](mcp-transport.md)이
담당합니다.

<a id="surface-stability"></a>
## 표면 안정성

| 표면 | 안정성 |
|---|---|
| 여기에 나열한 명령과 선택자 | `stable` |
| stable 명령으로 나열하지 않은 pre-1.0 추가 표면 | `beta` |
| 사람용 형식과 진단 문구 | `diagnostic` |
| 생성 Codex 구성 세부사항 | `internal` |

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)을 사용합니다.

## 명령 모델

`volicord`는 로컬 관리/bootstrap 실행 파일이며 장기 실행 네트워크 서비스가 아닙니다.
최초 릴리스는 `host=codex`, `profile=record`, `personal` 또는 `shared` 연결 범위만
받습니다.

지원 명령 그룹은 다음과 같습니다.

```text
volicord init
volicord status
volicord doctor
volicord diagnostics
volicord policy
volicord connection
volicord project
volicord mcp
volicord export authority-bundle
volicord changes reconcile
volicord inbox
```

알 수 없는 명령, 제거된 선택자, 추가 positional, 충돌하는 option은 사용법 오류입니다.
관리 명령 이름은 공개 API 메서드 이름이 아닙니다.

<a id="doctor-diagnostic-states"></a>
## Doctor 진단 상태

`volicord doctor --json`은 missing, invalid, unavailable, corrupt, stale 관찰을
서로 구분합니다. `states.installation_profile` 값은 `present`, `missing`, `invalid`,
`unavailable`, `corrupt`, `not_checked`, `unknown` 중 하나이며
이 조건들을 하나의 값으로 합치지 않습니다. `project_policy_authority` finding은
`authority_missing`,
`authority_corrupt`, `authority_unavailable`,
`managed_file_missing`, `managed_file_invalid`, `managed_file_unavailable`,
`managed_file_stale` 중 하나를 사용합니다. `managed_file_stale`은 두 복사본이 각각
유효하지만 기준 fingerprint가 서로 다르다는 뜻입니다. 복구 동작을 제안할 수는 있지만
doctor가 기본 policy를 대신 넣거나 어느 authority 복사본도 다시 쓰지 않습니다.
제한된 project-policy 감사는 `scan_state: complete` 또는
`scan_state: bounded_incomplete`를 보고합니다. 점검한 page에 finding이 없어도 제한 때문에
감사를 완료하지 못했으면 warning이며 통과로 보고할 수 없습니다.

<a id="runtime-home-selection"></a>
## Runtime Home 선택

`volicord init`과 모든 `volicord connection` 하위 명령은 `--home PATH`를 받습니다.
명시적인 CLI 경로, `VOLICORD_HOME`, 플랫폼 기본값 순서로 선택합니다. 명시적인 상대
경로는 호출자의 현재 작업 디렉터리를 기준으로 해석하며, 선택한 경로는 절대 경로로
보고합니다.

명시적인 경로를 선택하면 환경 변수나 플랫폼 기본 Runtime Home으로 대체하지 않습니다.
Runtime Home 선택과 모든 connection 명령이 수행하는 설치 프로필 검증은 읽기 전용입니다.
선택한 디렉터리나 `registry.sqlite`를 만들거나, Registry 스키마를 초기화하거나
마이그레이션하거나, Registry 상태를 쓰지 않습니다. Registry 스키마 생성은 명시적인
`init` 설정 변경에 속합니다. `init`은 자신이 소유한 설정 변경의 일부로 선택한 홈과
스키마를 만들 수 있지만, connection 명령은 선택한 홈에 현재 installation profile이 있어야
하며 홈이 없거나 사용할 수 없으면 그 정확한 경로를 담아 실패합니다. 선택 뒤에도
`connection list`와 `connection status`는 읽기 전용입니다. 비어 있거나 잘못됐거나 충돌하는
값은 저장소 접근 전에 실패합니다. Product Repository를 Runtime Home으로 사용하지 않습니다.

모든 명령에 같은 경로를 넘기면 `VOLICORD_HOME`을 내보내지 않고도 사용자 지정 홈의
lifecycle을 실행할 수 있습니다.

```sh
volicord init --host codex --repo "<repo>" --profile record --home "/srv/volicord/team-a"
volicord connection status codex --repo "<repo>" --home "/srv/volicord/team-a"
```

<a id="volicord-agent-install"></a>
<a id="agent-host-setup-and-init"></a>
## Codex 설정

```sh
volicord init --host codex --repo "<repo>" --profile record
volicord init --shared --host codex --repo "<repo>" --profile record
```

첫 명령은 개인 연결, `--shared`는 프로젝트 소유 공유 연결을 선택합니다. `init`은
정확한 관리 binding을 계획, 검증, 적용하고 남은 Codex 신뢰, 다시 불러오기, 검증 동작을
보고합니다. `--dry-run`은 파일시스템과 저장소를 변경하지 않습니다.

일치하는 현재 Agent Connection이 없으면 `init`은 `workflow` mode로 새 연결을
만듭니다. 일치하는 현재 Agent Connection이 이미 있으면 `init` replay와 repair는
host plan, 검증 기대값, 등록에서 기존 `workflow` 또는 `read_only` mode를 정확히
보존합니다. 이때 integration generation은 바뀌지 않습니다. Mode 전환과 generation
증가는 `volicord connection mode`만 수행합니다.

설정은 관련 없는 Codex와 저장소 내용을 보존합니다. 복구는 현재 정규 입력으로 같은
의도를 다시 실행합니다. 같은 `init` 복구 명령은 두 mode 모두에서 소유한 Guard 및
Codex 구성을 복구합니다. 제거는 일치하는 Volicord 관리 내용만 삭제합니다.

<a id="project-commands"></a>
## 프로젝트 명령

```text
volicord project use [PATH]
volicord project current
volicord project list
volicord project rename NAME [--repo PATH]
volicord project forget [PATH|NAME]
```

프로젝트 선택은 등록된 정규 Git 작업 트리를 해결합니다. 모호한 선택은 실패하며 cwd와
표시 이름으로 identity를 조용히 만들지 않습니다.

<a id="project-workflow-policy-commands"></a>
## 정책 명령

```text
volicord policy show --repo PATH
volicord policy validate --file PATH
volicord policy apply --repo PATH --file PATH
```

검증은 효과가 없습니다. Apply는 담당 문서의 plan과 원자적 commit 경계를 사용하며 알 수
없는 필드와 잘못된 값은 commit 전에 실패합니다.

## Agent Connection 명령

```text
volicord connection add [codex] [--repo PATH] [--home PATH] [--shared] [--read-only] [--dry-run] [--verbose | --json]
volicord connection list [--repo PATH] [--home PATH]
volicord connection status [codex] [--repo PATH] [--home PATH] [--shared] [--verbose | --json]
volicord connection verify [codex] [--repo PATH] [--home PATH] [--shared] [--verbose | --json]
volicord connection mode [codex] workflow|read-only [--repo PATH] [--home PATH] [--shared] [--verbose | --json]
volicord connection remove [codex] [--repo PATH] [--home PATH] [--shared] [--dry-run] [--verbose | --json]
```

호스트를 생략하면 현재 맥락이 모호하지 않을 때만 사용합니다. 명시적으로 받는 유일한 값은
`codex`입니다.

`volicord connection add`는 멱등 설정 및 복구 명령입니다. 새 Connection은
`--read-only`가 없으면 `workflow`, 있으면 `read_only`를 사용합니다. 일치하는 현재
Connection이 있으면 이 플래그의 부재를 명시적인 `workflow` 요청으로 해석하지 않으며,
replay와 repair는 저장된 mode를 보존합니다. 현재 Connection이 이미 `read_only`라면
이 플래그는 멱등 요청입니다. 현재 mode가 `workflow`라면 설정을 변경하기 전에 실패하고
`volicord connection mode`를 사용하도록 안내합니다. `connection add`는 mode를 전환하거나
integration generation을 증가시키지 않습니다.

### 선택한 Connection 출력

`volicord init`과 선택한 Connection을 다루는 `add`, `status`, `verify`, `mode`,
`remove` 명령은 출력 선택 방식 하나를 공유합니다. 출력 플래그가 없으면 작업에 맞는
결과, 선택한 저장소와 유효한 mode, `ready`/`waiting`/`failed` check 개수, 대기 관찰보다
앞에 표시하는 현재 문제, 현재 다음 동작을 간결한 사람용 산문으로 보여 줍니다.
이 사람용 라벨은 표시 문구이며 보고서나 check 상태를 추가하지 않습니다.

간결한 렌더러는 `host_session`, `required_tools`, `tool_round_trip`에 대응하는 정규
check가 pending일 때만 해당 활동을 포함합니다. 현재 pending인 일부만 Codex session 또는
도구 활동으로 묶을 수 있으며 passed, failed, 부재 check는 `Waiting` 아래에 반복하지
않습니다. Pending `guard_observation`은 알고 있는 누락 phase와 함께 Guard hook 활동으로
보여 줍니다. 렌더러는 정규 check나 action을 변경, 제거, 재정렬, 영속하지 않습니다.
Dry run 산문은 typed `PlannedConnectionChangeKind`별로 계획 변경 수를 묶으며 target
path에서 소유권을 추론하지 않습니다.

간결한 진단 안내는 작업에 따라 달라집니다. `status` 보고서에 pending이나 failed check가
있으면 같은 읽기 전용 상태 조회를 `--verbose`로 다시 실행할 수 있습니다. `verify`
보고서에 이런 check가 있으면 활성 검증을 verbose로 다시 실행할 수 있습니다. Dry run
역시 같은 dry run을 verbose로 다시 실행할 수 있습니다. `init`이나 `add` 설정을
적용했거나 `mode` 전환에 성공한 뒤 상세 진단이 유용하면 변경 작업을 재실행하라고
안내하지 않고 현재 `connection status ... --verbose` 명령을 안내합니다. 적용된
`remove` 결과에는 재실행 진단을 제안하지 않습니다. 조치할 진단이 없는 `complete`
결과는 이 안내를 생략합니다. 생성하는 모든 connection 후속 명령은 선택한 절대 Runtime
Home을 `--home PATH`로 포함하므로, 다시 실행할 때 호출자의 환경에 의존하지 않습니다.

Connection 설정, 선택, 복구, 진단 안내는 하나의 명령 표시 규칙을 사용합니다. 논리 인수가
모두 비어 있지 않고 ASCII 영문자, 숫자, `_`, `-`, `.`, `/`, `:`, `=`만 사용하는 이식
가능한 리터럴 토큰일 때만 한 줄 명령으로 표시합니다.
이 보수적인 형태는 POSIX 셸, PowerShell, Command Prompt에서 인수별 인용이 필요하지
않습니다.

저장소나 Runtime Home에 현재 셸에 맞는 인용이 필요하면, 대신 정확한 호스트,
저장소, Runtime Home, 선택적인 `shared` 범위, `--verbose` 출력 요구사항을 라벨과 함께
표시합니다. 이 값들을 모든 셸에서 그대로 복사해 실행할 수 있는 명령처럼 제시하지
않습니다. 사용자는 표시된 정확한 값으로 현재 셸에 맞는 명령을 구성합니다. 제어 문자가
있는 값은 정확한 내용과 값 경계를 모호하지 않게 유지하도록 라벨이 붙은 JSON 문자열
표기법을 사용합니다. 이 표기법은 표시 형식이며 셸 문법이 아닙니다. 예시는 다음과
같습니다.

```text
For detailed current Connection diagnostics, run the verbose status command with:

  Host: codex
  Repository: C:\Work\Product Repo
  Runtime home: C:\Users\Example User\.volicord
  Verbose output: required.
```

선택한 Runtime Home이 없거나 Installation Profile이 없어서 설정을 계속할 수 없을 때는
완전한 명령 대신 정확한 Runtime Home을 라벨이 붙은 필드로 따로 표시할 수 있습니다. 호출자는
`volicord init`을 실행할 때 호스트와 Product Repository를 선택합니다. 안내는 알 수 없는
좌표를 자리표시자 명령에 넣지 않습니다.

`--verbose`는 사람이 진단하는 데 필요한 완전한 보기를 표시합니다. 간결한 출력과 같은
작업별 머리말로 시작하고, 적용되는 `Connection`, `Summary`, `Checks`, `Actions`,
`Result`, `Planned changes`, `Assurance` 구역을 이 순서로 사용합니다. 모든 정규 check와
action, typed result 사실, 계획 operation과 target, 보장 한계를 원시 JSON detail blob 없이
표시합니다. 알고 있는 세부 필드는 구조화해서 표시하며, 집중 렌더러가 기대하는 타입과
맞지 않는 값이나 알 수 없는 확장 필드는 `Additional details` 아래에 표시합니다. 사람이
진단할 때 큰 성공 컬렉션은 개수로 요약할 수 있고, 그 밖의 제한된 컬렉션에서 모든 항목을
표시하지 않을 때는 남은 개수를 명시합니다. 산문 출력은 비어 있지 않은 진단 필드를 조용히
버리지 않습니다. 특히 성공한 MCP 도구 inventory 전체를 산문에 반복하지 않습니다.
각 action은 의미를 나타내는 kind와 지시만 표시합니다. Verbose action 구역은 실행 가능한
명령을 표시하거나 다시 구성하지 않습니다.

`--json`은 완전한 직렬화 `ConnectionCommandReport`를 쓰며 정확하고 손실 없는 기계 판독
표현으로 유지됩니다. 전체 도구 inventory와 원시 중첩 진단 사실은 JSON에서 확인합니다.
`--verbose`와 `--json`은 함께 사용할 수 없는 사용법 옵션입니다.
`volicord connection list`는 별도의 간결한 컬렉션 projection을 유지하며
`--verbose`를 받지 않습니다.

대표적인 간결한 검증 결과는 다음과 같습니다.

```text
Verification completed: 5 ready, 4 waiting.

Repository: /workspace/product
Mode: workflow
Checks: 5 ready, 4 waiting

Waiting
  Codex session and tool activity: initialize, tools/list, and the designated read-only tool call
  Guard hook activity: pre_tool, post_tool, prompt_capture

Next
  Restart or reload Codex, start or resume this repository, and use a read-only Volicord tool.

Rerun active verification with `volicord connection verify codex --repo /workspace/product --home /home/user/.volicord --verbose` for detailed diagnostics.
```

Verbose 보기는 같은 typed 보고서를 구조화된 진단으로 표시합니다.

```text
Verification completed: 1 ready, 1 waiting.

Connection
  ID: connection_1
  Host: codex
  Scope: user
  Profile: record
  Mode: workflow
  Repository: /workspace/product
  Config target: /home/user/.codex/config.toml
  Runtime home: /home/user/.volicord

Summary
  Status: action_required
  Checks: 1 passed, 1 pending, 0 failed

Checks
  [wait] Codex managed session
    Managed host connection use has not been observed
    Code: host_session_not_observed
    Current revision: sha256:current-revision
    Initialize: not observed

  [pass] Managed Codex configuration
    Managed Codex configuration matches the canonical entry
    Target: /home/user/.codex/config.toml
    State: match
    Diagnostic code: managed_config_matches

Actions
  observe_codex
    Restart or reload Codex and use the connection.

Assurance
  Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.
```

### Connection 목록 투영

`volicord connection list`는 읽기 전용 컬렉션 목록입니다. 선택한 Connection
하나나 운영 결과 하나를 다루지 않으므로 `ConnectionCommandReport`를 사용하지 않습니다.
JSON 문서의 최상위 구성원은 정확히 다음과 같습니다.

```yaml
ConnectionListReport:
  connections: ConnectionListEntry[]
  limits:
    - "Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion."

ConnectionListEntry:
  connection_id: string
  host_kind: codex
  connection_intent: personal | shared
  host_scope: user | project
  mode: read_only | workflow
  enabled: bool
  connected_projects: string[]
  connected_repositories: string[]
  verification_report: ConnectionVerificationReport | null
  issues: ConnectionListIssue[]
  server_name: string
  config_target: string

ConnectionListIssue:
  kind: metadata_corrupt | verification_report_corrupt
  summary: string
```

각 행의 `verification_report`는 정규
[`ConnectionVerificationReport`](agent-connection.md#connection-verification-report)입니다.
영속한 보고서가 없으면 목록은 `verification_not_run`을 포함하는 담당 문서 정의의 합성
`action_required` 보고서를 사용하며, 읽는 것만으로 영속하지 않습니다. 영속 보고서가
손상되어 디코딩할 수 없을 때만 `verification_report`가 `null`이고, 해당 행에
`verification_report_corrupt` 문제가 하나 생깁니다. 영속 등록 메타데이터가
유효하지 않으면 행을 숨기거나 검증 및 mode 명령의 엄격한 거부를 약화하지 않고
`metadata_corrupt` 문제 하나를 추가합니다.

문제 종류는 닫힌 snake-case 어휘입니다. 행에서는 종류 기준으로 정렬하고 중복을 제거하므로
`metadata_corrupt`가 `verification_report_corrupt`보다 앞섭니다. 문제 요약은 제한된
진단 문구이며 손상된 영속 JSON을 노출하지 않습니다. 행 문제는 영속 상태 손상을 보고할
뿐 Connection 운영 상태가 아니며 목록 집계 상태를 만들지 않습니다.

사람용 출력은 다음의 정확한 탭 구분 머리글을 유지합니다.

```text
host	intent	mode	enabled	connected_repositories	verification_status	issues	target
```

보고서가 있으면 정규 검증 상태를 표시하고 손상된 보고서를 사용할 수 없으면 `-`를
표시합니다. 문제 목록이 비어 있어도 `-`, 문제가 있으면 정렬된 종류를 표시합니다.
저장소 필터링은 일치하는 각 Connection의 결정적인 전체 membership 필드를 보존합니다.
일치 항목이 없어도 유효한 빈 목록입니다. 행 범위 문제를 보고할 때도 열거는 Registry나
파일시스템을 쓰지 않고 종료 코드 `0`으로 성공합니다. Store 접근, 선택, 직렬화 실패는
런타임 오류 채널을 사용합니다.

`volicord connection mode`는 Connection mode와 소유한 모든 프로젝트 범위 Guard
manifest를 revision 전환 하나로 다룹니다. CLI는 변경 전에 모든 Connection Project마다
현재 상태의 엄격히 유효한 Guard Installation이 정확히 하나씩 있는지 확인하고, 각 후보
manifest에서 Connection integration revision만 교체합니다. Inventory가 누락되거나,
중복되거나, stale이거나, malformed이거나, 소유자가 일치하지 않으면 담당 `volicord init`
명령을 다시 실행하라는 복구 안내와 함께 실패합니다. 그다음 Store는 Registry transaction
하나에서 Connection mode를 변경하고 integration generation을 한 번 증가시키며, 저장된
검증 보고서를 비우고, 모든 후보 manifest를 다시 결속합니다. 충돌이나 쓰기 실패가 하나라도
발생하면 여러 프로젝트를 가진 personal Connection을 포함해 전환 전체를 rollback합니다.

현재 mode를 다시 선택하면 정확한 no-op입니다. Registry row, timestamp, report,
generation, manifest, host configuration, Product Repository file을 전혀 바꾸지 않으며 reload
action도 내보내지 않습니다. 실제 전환이 성공해도 host configuration이나 Product Repository
file을 다시 쓰지 않고, 기존 managed host를 새 revision에 맞춰 reload해야 하므로 정확히 하나의
`reload_host` action을 내보냅니다. 이전 runtime session, 프로젝트 Agent Session, Guard
event는 이력으로 남지만 현재 check를 충족하지 못하며, 나중에 이전에 사용한 mode로 돌아가도
다시 현재 상태가 되지 않습니다.

`volicord init`이 선택한 프로젝트의 Connection을 교체할 때 migration은 superseded
membership보다 먼저 그 프로젝트의 Registry project-session binding과 Guard
Installation을 폐기합니다. Superseded Connection에 다른 프로젝트가 있으면 이 순서의
폐기, replacement membership과 Guard Installation, replacement 활성화를 Registry
transaction 하나에서 commit합니다. 기존 Connection, 다른 membership과 그 child row,
connection 전체 runtime session은 유지합니다. Superseded Connection의 마지막
프로젝트라면 기존 Connection을 비활성화하고 host 정리가 성공할 때까지 membership,
binding, Guard Installation, 정확한 pending-host-cleanup marker를 유지합니다. Host 정리가
실패하면 이 완전한 재시도 inventory를 바꾸지 않은 채 `partial_application`을 보고합니다.
Host 정리 뒤의 최종 Registry transaction은 replacement, marker, membership inventory를
다시 검증하고 기존 프로젝트 소유 행과 membership을 폐기한 뒤 marker를 지웁니다. 소유한
host entry가 이미 없으면 no-op이므로 replay는 두 Connection의 현재 행을 중복 생성하지
않고 Registry 정리를 마칠 수 있습니다.

`volicord connection remove`는 선택한 Connection Project membership과 그 프로젝트
범위 Registry binding 및 Guard Installation을 Store transaction 하나에서 제거합니다.
다른 membership이 남으면 Agent Connection, connection 전체 runtime session, 다른
membership과 그 Registry 행을 유지하며 공유 host configuration은 바꾸지 않습니다.
마지막 membership이면 CLI가 plan을 검증하고 일치하는 관리 host entry를 먼저 제거한 뒤,
남은 binding, Guard Installation, runtime session, membership, Agent Connection을
제거하는 Registry transaction을 commit합니다. 소유한 host entry가 이미 없으면 no-op으로
처리하므로 재시도에서 Registry 정리를 마칠 수 있습니다.

Host 제거 실패는 Registry 변경 전에 발생합니다. 그 뒤 Registry 실패가 발생하면 Registry
transaction 전체를 rollback하여, host 제거가 이미 성공했더라도 membership과 Agent
Connection을 재시도에서 다시 선택할 수 있게 유지합니다. `--dry-run`은 Registry 상태,
host configuration, Product Repository 내용을 모두 바꾸지 않습니다. 제거 출력은
`membership_removed`, `connection_removed`, `remaining_project_count`를 포함하여
membership만 제거한 경우와 Agent Connection을 완전히 제거한 경우를 구분합니다.

<a id="agent-connection-result-states"></a>
### 연결 결과 상태

| 상태 | 의미 |
|---|---|
| `complete` | 선택한 동작이 끝나고 담당 문서가 요구하는 모든 현재 검사를 통과했습니다. |
| `action_required` | 오래 유지되는 설정이 있을 수 있지만 이름 붙은 사용자 또는 Codex 동작이 남았습니다. |
| `failed` | 동작이 실패했고 기계 판독 원인을 보고합니다. |

`complete`는 해당 명령 보고서의 모든 필수 check가 통과했음을 뜻합니다. Core 호출
권한은 각 관리 MCP 호출에서 별도로 평가합니다.

선택한 Connection의 설정 및 생명주기 명령은 모두
`ConnectionCommandReport` 하나를 직렬화합니다. 여기에는 `volicord init`과
Connection의 `add`, `status`, `verify`, `mode`, `remove` 명령이 포함됩니다.

```yaml
ConnectionCommandReport:
  operation: init | add | status | verify | mode | remove
  dry_run: bool
  status: complete | action_required | failed
  runtime_home: string
  connection:
    id: string
    host: codex
    scope: user | project
    profile: record
    mode: read_only | workflow
    repository: string
    config_target: string
  checks: ConnectionCheck[]
  actions: ConnectionAction[]
  result?: SetupResult | ModeTransitionResult | RemovalResult
  planned_changes?: PlannedConnectionChange[] # dry-run에만 사용
  limits: string[]

SetupResult:
  kind: setup
  applied: bool

ModeTransitionResult:
  kind: mode_transition
  changed: bool
  previous_mode: read_only | workflow
  current_mode: read_only | workflow
  previous_integration_revision: string
  current_integration_revision: string
  rebound_guard_installation_ids: string[]

RemovalResult:
  kind: removal
  membership_removed: bool
  connection_removed: bool
  remaining_project_count: integer

PlannedConnectionChange:
  kind: runtime_home_initialization | project_registration | managed_host_configuration | guard_managed_file | guard_registry_setup | connection_membership
  operation: create | update | remove | register | rebind
  target: string

ConnectionAction:
  id: ConnectionActionKind
  instruction: string
```

이 보고서에는 집계 상태 하나와 check/action 트리 하나만 있습니다. 선택적인 tagged
`result`에는 작업별 사실만 두며 두 번째 상태를 만들지 않습니다. 설정 보고서는
`kind=setup`, mode 보고서는 `kind=mode_transition`, 적용에 성공한 제거 보고서는
`kind=removal`을 사용합니다. Status와 verify는 보통 `result`를 생략하고, 제거 dry
run은 아직 발생하지 않은 결과를 생략합니다. JSON은 적용되지 않는 선택 필드를
생략합니다. `limits`에는 협력적 보장 한계를 한 번만 둡니다.

`SetupResult.applied`는 설정 변경과 운영 검증을 구분합니다. Init 또는 add 적용이
성공하면 뒤의 로컬 또는 운영 check 때문에 `status=failed`가 되더라도
`applied=true`입니다. Dry run은 `applied=false`와 `planned_changes`를 보고하며
`status=dry_run`을 직렬화하지
않습니다. 계획 변경이나 host action이 남으면 `action_required`, 둘 다 없으면
`complete`입니다.

Migration의 일부만 적용된 뒤 설정이 실패하면 `status=failed`와 `applied=false`를
보고합니다. 실패한 `setup_plan` check의 details에는 관찰한 Registry 전환, cleanup,
이전 Connection의 disposition, 재시도 인자를 기록합니다. 두 번째 migration 상태를
보고하거나 관찰하지 않은 단계가 성공했다고 암시하지 않습니다.

명령 보고서는 필수 check가 하나라도 실패하면 `failed`, 실패 없이 필수 check가
pending이거나 typed action이 남으면 `action_required`, 그 밖에는 `complete`로
집계합니다. 따라서 완료된 mode 전환은 두 번째 전환 상태를 만들지 않고 통과한 전환
check와 현재 필요한 reload/use action 하나를 함께 보고할 수 있습니다.

계획 단계는 각 변경에 닫힌 소유권 `kind`, 타입이 지정된 `operation`, 정규 target을 지정합니다.
No-op 항목은 내보내지 않으며 안정적인 `kind` 표기, `operation`, `target` 순서로 정렬하고
중복을 제거합니다. 관리 구성과 Guard check는 `kind`를 사용합니다. Target 경로가 바뀌어도
계획 변경의 의미는 바뀌지 않습니다. JSON은 위의 세 필드를 포함합니다. 간결한 사람용
출력은 `kind`별 개수를 묶고, verbose 사람용 출력은 각 항목을 `Kind`, `Operation`,
`Target` 라벨이 있는 색인 블록으로 표시합니다.

`checks`와 `actions`는 정규
[`ConnectionVerificationReport`](agent-connection.md#connection-verification-report)의
구성원 type과 순서를 사용합니다. JSON과 사람용 출력은 같은 typed command report를
표시합니다. 각 JSON action은 정확히 `id`와 `instruction`만 포함합니다. 사람용 출력은
check를 묶어 보여 줄 수 있지만 상태나 action을 다시 계산하지 않습니다. Action은 의미를
나타내는 보고서 사실입니다. 작업별 실행 안내는 현재 typed 호스트, 저장소, Runtime Home,
범위, 출력 선택 좌표에서 별도로 생성합니다.

Mode no-op은 `changed=false`, 같은 이전/현재 mode와 revision, 빈 Guard Installation
재결속 ID, 통과한 `mode_transition` check, 빈 action, `status=complete`를 보고합니다.
실제 전환은 `changed=true`, 정확한 이전/현재 mode와 revision, 재결속한 Guard
Installation ID, 통과한 `mode_transition` check 하나, 현재 `reload_host` action 정확히
하나, `status=action_required`를 보고합니다.

적용에 성공한 제거는 통과한 `connection_removal` check, `status=complete`, 정확한
`membership_removed`, `connection_removed`, `remaining_project_count` 사실을
`RemovalResult` 안에 보고합니다. 제거 dry run은 실제 제거 계획이 있을 때만 pending
제거 check와 `apply_removal` action을 사용하며 typed `planned_changes`로 계획을
보고하고 아무것도 변경하지 않습니다.

`volicord connection status`는 읽기 전용입니다. 현재 관리 구성, 신뢰, Guard audit,
통합 revision, managed-host session 관찰을 마지막 활성 executable/MCP server probe와
함께 projection합니다. Process를 시작하지 않으며 파일, timestamp, 보고서, action,
관찰, 데이터베이스 row를 바꾸지 않습니다.

`volicord connection verify`는 `codex`를 활성 탐색하고 version 명령을 실행한 뒤
`volicord mcp --check`와 CLI 전용 MCP self-test를 실행합니다. Self-test는
`initialize`, `tools/list`, 필수 도구 검증, 안전한 읽기 전용
`volicord.list_projects` 호출을 수행합니다. 사전 점검과 자체 검사의 프로세스 시작 구체화는 모두
점검 대상 호스트 구성에 사용된 정규 관리 시작 계약에서 파생합니다. 개인 연결 검증은 그
계약의 정적 절대 `VOLICORD_HOME`을 사용합니다. 공유 연결 검증은 연결 작업이 선택한
Runtime Home으로 전달 대상 `VOLICORD_HOME`을 해석하고 정규 Product Repository 루트에서
저장소 검색을 실행합니다. CLI 전용 검증 표식은 호출에만 적용하는 진단 값이며 생성 호스트
구성에는 포함되지 않습니다.

stdio 자체 검사가 실패하면 현재 단계별 check code를 유지하고 JSON 진단의
`checks[id=mcp_server].details.self_test.failure`에 실패 객체를 추가합니다. 자체 검사
진단은 완료된 점검 관찰을 직접 기록합니다.

```yaml
McpSelfTestProgress:
  status: passed | failed | pending
  code: string
  diagnostic: string
  initialize: boolean
  tools_list_observed: boolean
  tools_list?: string[]
  required_tools_validated: boolean
  safe_read_only_tool: volicord.list_projects
  safe_read_only_tool_completed: boolean
  shutdown_completed: boolean
  failure?: McpSelfTestFailure
```

`tools/list`를 관찰했으면 빈 결과를 관찰한 경우의 빈 배열을 포함하여 반환된 이름
그대로 `tools_list`에 나타납니다. 유효한 도구 목록을 관찰하지 못했으면 이 필드를
생략합니다. 이후의 안전한 호출이나 종료가 실패해도 관찰한 도구 목록과 앞서 성공한
모든 완료 사실을 보존합니다. 사람이 읽는 상세 출력은 각 완료 사실이 참일 때만 해당
점검 단계를 통과로 보고하며 정상 종료 결과를 별도로 표시합니다.

현재 실패 객체 형태는 다음과 같습니다.

```yaml
McpSelfTestFailure:
  kind: spawn | exited_before_response | timeout | read | write | protocol | wait | cleanup | shutdown
  stage: startup | initialize | tools_list | safe_tool_call | shutdown
  exit_code?: integer | null
  timeout_ms?: integer
  stderr?: BoundedDiagnosticText
  protocol_detail?: BoundedDiagnosticText
  missing_tools?: string[]
  io_detail?: BoundedDiagnosticText

BoundedDiagnosticText:
  text: string
  truncated: bool
  omitted_bytes: integer
```

타입이 지정된 실패에 필요한 필드만 나타납니다. `exit_code=null`은 시그널 종료를
포함하여 종료된 프로세스에 숫자 상태 코드가 없다는 뜻입니다. 검증기는 stderr 파이프를
동시에 비우면서 자식 stderr를 최대 2 KiB까지 `stderr`에 보존합니다.
`protocol_detail`은 2 KiB, stdout 프로토콜 줄 하나는 64 KiB로 제한하며 자체 검사
프로세스 하나에서 stdout 프로토콜 메시지를 최대 16개까지 받습니다. 단조 증가 시계의
생명주기 기한 하나가 프로세스 진행 전체를 제어합니다. 프로세스 트리 종료, 직접 자식
프로세스 회수, 파이프 완료에는 한도가 있는 정리 여유 시간을 사용합니다. `cleanup`
실패는 이 한도 안에 정리를 끝내지 못했음을 나타내며 제한된 `io_detail`을 포함합니다.
잘린 텍스트 끝에는 생략한 바이트 수를 밝히는 결정적 표식이 붙으며 같은 수가
`omitted_bytes`에도 남습니다. 검증기는 타입이 지정된 단계에서 직접
`startup` 또는 `shutdown`을 `mcp_server_process_failed`, `initialize`를
`mcp_server_initialize_failed`, `tools_list`를 `mcp_server_tools_list_failed`,
`safe_tool_call`을 `mcp_server_safe_call_failed`로 보고합니다. `stderr`는 진단 맥락이지
기계 판독 가능한 자식 프로세스 사유가 아니며 CLI는 임의의 자식 산문을 분류하지 않습니다.
`missing_tools`는 구조화된 `tools/list` 응답에서만 만들고, JSON-RPC 오류 상세
정보에는 임의의 오류 산문이나 데이터를 복사하지 않은 채 구조화된 숫자 `code`만 포함합니다.

그런 다음 현재 managed-host 관찰을 읽고 정규 보고서 하나만 영속합니다. Plan 전에 선택한
Connection의 정확한 typed integration revision을 확보하고, immediate Registry transaction
하나에서 그 revision을 비교해 보고서만 교체합니다. 검증 중 Connection이 바뀌면 stale
보고서를 저장하지 않고 명령 재실행을 요구합니다. 관찰한 Host Plan fingerprint는
diagnostic으로만 남습니다. 검증은 이를 적용하거나 채택하지 않으며
`managed_fingerprint`를 바꾸지 않습니다. CLI 자체 검사가 성공해도 관리 호스트 관찰이
아니며 `host_session`, `required_tools`, `tool_round_trip` 증거를 만들지 않습니다.

`volicord init`과 `volicord connection add`는 뒤의 운영 check가 실패하더라도 이미 쓴
유효한 설정을 유지합니다. Codex를 사용할 수 없거나 self-test가 실패했다는 이유로 관리
구성을 rollback하지 않습니다. Managed-host 관찰이 아직 없는 새 유효 설정은
`action_required`이며, 관찰을 얻는 데 필요한 typed reload/first-use action을 담습니다.
이 setup 명령은 host configuration을 적용하거나 채택한 뒤 managed fingerprint를 commit하고,
소유자와 일관된 Registry/Guard 상태를 완성하며, 최종 Connection revision을 파생한 다음에만
현재 구성과 관찰한 host 동작을 검증하고 조건부로 보고서를 영속합니다.

Action은 pending 및 failed check에서 직접 만드는 정렬되고 중복 제거된 목록입니다. 다시
불러오기와 최초 사용 지시는 실제 Codex 활동을 관찰해야 한다고 명시합니다.
`guard_files` check가 통과했다면 Guard 파일 재설치를 지시하지 않습니다.

<a id="external-host-configuration"></a>
## 관리 Codex 구성

개인 연결은 사용자 소유 관리 Codex 구성만 씁니다. 그 entry는 선택한 정규 절대
Runtime Home을 정적 `VOLICORD_HOME`으로 결속하고 프로젝트 선택자를 담지 않으며 환경
이름을 전달하지 않습니다. 공유 연결은 지원되는 프로젝트 소유 Codex entry를 쓰고
`VOLICORD_HOME`만 전달하며 머신 로컬 경로나 lifecycle 좌표를 내장하지 않습니다. 생성,
엄격한 검증, fingerprinting은 같은 정규 관리 시작 계약을 projection합니다. 정확한 형태,
drift, 복구, launch 맥락, uninstall 경계는
[Agent Connection](agent-connection.md#managed-mcp-launch-contract)이 담당합니다. 구성
marker는 협력적 launch 경로를 선택할 뿐 credential이나 identity 증거가 아닙니다.

## MCP 명령

```text
volicord mcp --stdio --connection <connection_id> [--project <project_id>]
volicord mcp --stdio --discover-repository --host codex
volicord mcp --check --connection <connection_id> [--project <project_id>]
```

이 명령은 관리 stdio만 노출합니다. 정확한 framing, lifecycle, 도구 목록, 응답
projection은 [MCP 전송](mcp-transport.md)이 담당합니다.

<a id="diagnostics"></a>
## Diagnostics

```text
volicord diagnostics session [--session SESSION_ID] [--json]
volicord diagnostics workflow-metrics --repo PATH --json
```

Diagnostics 출력은 제한된 비권한 operability 데이터입니다. JSON 보고서는 현재
diagnostics SQL에서 파생한 정확한 `canonical_schema_digest`와
`contract_id=volicord.sqlite.diagnostics`로 로컬 저장소를 식별합니다. 숫자 schema
version을 노출하거나 그 값으로 dispatch하지 않습니다. Diagnostics 읽기는 저장소를
만들거나 프로젝트 권한 상태를 열거나 state version을 전진시키거나 evidence 또는
assurance, 닫기 준비 상태를 바꾸거나 UserAction을 해결하지 않습니다.

<a id="authority-bundle-export"></a>
## 권한 번들 내보내기

```text
volicord export authority-bundle --output "<path>" --repo "<repo>"
```

내보내기는 담당 문서가 정의한 권한 번들을 새 경로나 명시적으로 허용된 출력 경로에
씁니다. 프로젝트 권한 상태를 바꾸지 않습니다.

## 변경 조정

```text
volicord changes reconcile --repo "<repo>"
```

조정은 공개 `volicord.reconcile_changes` 동작을 로컬 관리 흐름에 투영합니다. Guard
suppression 실패는 명시적으로 남으며 `Unavailable` 결과를 비어 있는 성공으로 표시하지
않습니다.

<a id="user-channel-commands"></a>
## User Channel 명령

```sh
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

`inbox`는 최초 릴리스의 유일한 UserAction 해결 채널입니다. 로컬 사용자 소유 표면에
저장 typed form을 표시하고 명시적인 choice 또는 evidence observation 하나를
`volicord.resolve_user_action`에 제출합니다. MCP 에이전트는 대기 요청을 만들거나
재개할 수 있지만 이 해결 경로를 실행할 수 없습니다.

저장 요청과 resolution은 엄격한 typed record입니다. 손상되거나 알 수 없거나 섞이거나
잘못된 저장 값은 영속 데이터 실패 분류로 실패하며 CLI가 기본값을 넣거나 복구하거나
답을 추측하지 않습니다.

## 출력과 종료 상태

선택한 Connection 명령 보고서의 기본 간결한 산문과 `--verbose` 진단은 사람용이며
자동화에서 파싱하면 안 됩니다. `--json`은 stdout에 완전한 JSON 문서 하나만 씁니다. 두
플래그는 사용법 parsing 단계에서 충돌합니다. `complete`, `action_required`, 유효한 모든
dry run은 `0`으로 종료합니다. Typed `failed` 운영 보고서는 `1`, 사용법 오류는 `2`로
종료합니다. 실패한 JSON 운영 보고서는 stdout 문서 하나만 쓰고 stderr는 비워 둡니다.
실패한 사람용 운영 보고서는 stdout에 표시합니다. 예상하지 못한 런타임 또는 직렬화 오류는
stderr를 사용하고 `1`로 종료합니다. 종료 상태는 표시 문자열이나 다시 parsing한 JSON이
아니라 typed report 상태로 선택합니다.

<a id="noninteractive-approval-behavior"></a>
## 비대화형 동작

비대화형 실행은 프로젝트 신뢰를 수락하거나 UserAction을 해결하거나 민감 동작을
승인하거나 호스트가 표시한 질문에 답하지 않습니다. 구조화된 다음 동작을 반환하고 판단을
사용자에게 남깁니다.

## 관련 담당 문서

- [Agent Connection](agent-connection.md)
- [MCP 전송](mcp-transport.md)
- [런타임 경계](runtime-boundaries.md)
- [API UserAction 스키마](api/schema-user-action.md)
- [저장 효과](storage-effects.md)
