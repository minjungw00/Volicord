# Agent Connection 참조

이 문서는 최초 릴리스 Agent Connection 계약을 정의합니다. 정확한
`host_kind=codex` Record 연결 표면, 정규 연결 검증 보고서, 관리 구성 소유권, 통합
revision, Codex 어댑터와 Core 사이의 검증된 운영 session 경계를 담당합니다.

<a id="owns-and-does-not-own"></a>

## 담당 범위

이 문서가 담당합니다.

- 허용하는 `host_kind`, integration profile, 연결 의도, 전송, 사용자 행동 전달 경로,
  mode, 플랫폼 환경 값
- 정규 `ConnectionVerificationReport`, 닫힌 상태 값, 결정적 집계, 엄격한 인코딩,
  보고서 부재 projection
- Connection과 프로젝트 통합 revision
- 권위 있는 managed-host runtime, 협상한 MCP profile, 프로젝트 session 소유권
- `ValidatedAgentSession`과 Core가 이를 소비하기 전에 필요한 검사
- Codex 어댑터의 탐색, 설치, 검증, repair, 제거 책임

이 문서는 아래 항목을 담당하지 않습니다.

- stdio 프레이밍, MCP 초기화, 도구 처리 경로, 종료:
  [MCP 전송](mcp-transport.md)
- 관리 명령 문법, 출력, 종료 코드: [관리 CLI](admin-cli.md)
- 정확한 데이터베이스 테이블이나 저장 효과:
  [저장소 기록](storage-records.md), [저장 효과](storage-effects.md)
- 일반 빌드, 패키지, 플랫폼, 릴리스 검증:
  [검증](../maintain/validation.md)
- 운영체제 배치와 파일시스템 전제 조건: [시스템 요구사항](system-requirements.md)
- Core `UserActionRequest`와 `UserActionResolution` 스키마:
  [API User Action 스키마](api/schema-user-action.md)
- 제품 전체 실패 범주와 보안 의미:
  [실패 모델](failure-model.md), [보안](security.md)

<a id="surface-stability"></a>

## 표면 안정성

라벨은 [문서 정책](../maintain/documentation-policy.md#surface-stability-labels)의 기준
어휘를 따릅니다.

| 표면 | 안정성 | 계약 |
|---|---|---|
| 최초 릴리스 값 집합, `ConnectionVerificationReport`, 통합 revision, 권위 있는 운영 session, `ValidatedAgentSession` | `stable` | 정확한 경계 계약입니다. |
| Codex 탐색, 관리 설치, 검증, repair, 제거, drift 결과 의미 | `stable` | 관찰 가능한 계약을 유지하면서 구현을 바꿀 수 있습니다. |
| 어댑터 모듈, 파일시스템 helper, 생성된 시작 marker, Store query helper | `internal` | 안정된 경계를 보존하지만 공개 표면은 아닙니다. |
| 사람이 읽는 검증 안내와 client/host version 관찰 | `diagnostic` | Machine-readable 범주, 사유, typed 필드가 권위 있는 값입니다. |

<a id="first-release-surface"></a>

## 최초 릴리스 표면

최초 릴리스는 아래 Agent Connection 표면만 허용합니다.

| 차원 | 정확한 값 |
|---|---|
| 호스트 | `host_kind=codex` |
| Integration profile | `integration_profile=record` |
| 연결 의도 | `personal` 또는 `shared` |
| Connection mode | `read_only` 또는 `workflow` |
| 전송 | `volicord mcp --stdio`로 시작하는 Volicord 관리형 stdio MCP |
| 프로덕션 MCP revision | `2024-10-07`, `2024-11-05`, `2025-03-26`, `2025-06-18`, `2025-11-25` 중 하나 |
| 사용자 소유 행동 전달 | CLI inbox |
| 플랫폼 환경 | `linux`, `macos`, `native_windows`, `wsl2` |

`personal` 연결은 사용자 소유 로컬 Codex 구성을 설치합니다. `shared` 연결은 선택한
`Product Repository` 안에 프로젝트 소유 Codex 구성을 설치합니다. 개인 관리 시작은
등록된 Connection 하나를 식별하며, 이 Connection의 허용 프로젝트는 Store가 소유하는
권위 있는 membership으로 남습니다. 공유 관리 시작은 저장소 검색을 통해 Connection과
프로젝트를 해석합니다.

Connection은 host kind, scope, mode, managed-configuration fingerprint, 프로젝트
membership, 변경 불가능한 Store 소유 integration-instance ID, Store 소유 integration
generation을 담당합니다. 이 현재 소유자 사실에서 파생한 Connection 및 프로젝트 integration
revision은 로컬 lifecycle과 상관관계 좌표입니다.

완전히 새로운 Agent Connection의 기본 mode는 `workflow`이며,
`volicord connection add --read-only`로 만드는 새 Connection은 `read_only`로
시작합니다. 일치하는 현재 Agent Connection을 다시 설정하거나 복구할 때는 저장된 mode를
정확히 사용합니다. 특히 `connection add`에서 `--read-only`를 생략해도 `workflow` 전환
요청이 아니며, 기존 `workflow` Connection에 이 플래그를 지정해도 암묵적인 전환 권한이
생기지 않습니다. 설정은 Record profile에서 mode를 추론하지 않습니다. 기존 mode를 바꾸고
integration generation을 증가시키는 명령은 `volicord connection mode`뿐입니다.

Agent Connection은 `Volicord Runtime Home`에 저장하는 로컬 통합 기록입니다. 운영체제
권한을 부여하거나 사용자 identity를 성립시키거나 Codex가 관리 entry를 불러왔음을
증명하지 않습니다. 관리 stdio MCP 프로세스 하나는 현재 Agent Connection 하나에
결속됩니다.

사용자 소유 행동은 CLI inbox로 전달합니다. MCP 에이전트는 담당 문서가 정의한 행동을
요청할 수 있지만 로컬 사용자 채널로 동작하거나 사용자를 대신해 해결할 수 없습니다.

프로덕션 revision 집합에서는 정확한 `protocolVersion` 요청이 같은 session profile을
선택합니다. 초기화 기반 요청 형태의 다른 문자열에는 서버가 선호하는 `2025-11-25`
counter-offer를 반환합니다. 날짜 순서로 지원 여부를 추론하거나 사용자가 지원 집합을
구성하지 않습니다. 고정된 pre-release `2026-07-28` revision은 discover 기반
generation에 속하므로 initialize 협상에 들어가지 않습니다. 정확한 인자와 응답 동작은
[MCP 전송](mcp-transport.md#protocol-revision-negotiation)이 담당합니다.

<a id="managed-mcp-launch-contract"></a>

## 관리형 MCP 시작 계약

하나의 typed 관리형 MCP 시작 계약이 실행 명령, stdio 인자, 정적 및 전달 환경
binding, 개인/공유 구분, 관리 provenance, 엄격한 시작 형태 검증, 정규 projection,
결정적 managed fingerprint 입력의 정규 출처입니다.

개인 연결에는 선택한 정규 절대 Runtime Home과 선택한 절대 `volicord` 실행 파일이
필요합니다. Runtime Home과 관리 host, launch, Connection marker를 정적 환경 값으로
저장하며 부모 환경 이름은 전달하지 않습니다.

```toml
[mcp_servers.volicord]
command = "/absolute/path/to/volicord"
args = ["mcp", "--stdio", "--connection", "<connection_id>"]

[mcp_servers.volicord.env]
VOLICORD_HOME = "/absolute/runtime/home"
VOLICORD_MCP_CONNECTION_ID = "<connection_id>"
VOLICORD_MCP_HOST = "codex"
VOLICORD_MCP_LAUNCH = "managed_host"
```

개인 entry는 프로젝트 선택자를 담지 않습니다. 인자에는 `--project`가 없고 정적
환경에는 프로젝트 marker가 없습니다. Agent Connection의 권위 있는 Product Repository
연결 관계는 프로세스 시작 상태가 아니라 Store가 소유하는 Connection Project
membership으로 남습니다.

공유 연결에는 clone에 이식 가능한 저장소 탐색 시작만 들어갑니다. `PATH`의
`volicord`를 사용하고 `VOLICORD_HOME`만 전달하며 정적 환경 table은 두지 않습니다.

```toml
[mcp_servers.volicord]
command = "volicord"
args = ["mcp", "--stdio", "--discover-repository", "--host", "codex"]
env_vars = ["VOLICORD_HOME"]
```

공유 시작에는 절대 실행 파일, Runtime Home, Connection ID, project ID 또는 그 밖의
머신 로컬 lifecycle 좌표를 넣지 않습니다. 프로젝트 인자나 프로젝트 환경 marker가
있는 개인 entry, 정적 환경과 전달 환경의 이름 충돌, 비어 있거나 중복된 전달 이름,
불완전한 개인 binding, 개인/공유 인자 또는 환경 형태의 혼합은 유효하지 않습니다.

생성 Codex 구성은 어댑터가 이 계약을 projection한 결과입니다. CLI 검증의 사전
점검과 stdio 자체 검사는 모두 같은 계약을 구체화합니다. 어느 소비 경로도 프로젝트
선택자, 플랫폼 identity, WSL 전용 필드를 추가하지 않습니다.

Codex 어댑터는 현재 TOML 형태를 parsing하고 같은 typed 계약을 다시 구성하여 관리
entry를 검증합니다. 알 수 없는 launch key, 잘못된 값, 비정규 형태는 두 번째 허용
형태가 아니라 drift입니다. 어댑터는 유효한
`tools.<known-tool>.approval_mode` overlay만 보존하고 launch identity에서는
제외합니다. Managed fingerprint는 정규 launch projection과 host kind, scope, server
name을 포함합니다. 서식 차이는 fingerprint를 바꾸지 않지만 launch 의미가 달라지면
바뀝니다.

<a id="connection-verification-report"></a>

## `ConnectionVerificationReport`

작은 보고서 하나가 정규 직렬화 연결 검증 상태입니다.

```yaml
ConnectionVerificationReport:
  status: complete | action_required | failed
  checked_at: UtcTimestamp
  checks: ConnectionCheck[]
  root_cause_ids: DiagnosticFindingId[]
  actions: ConnectionAction[]

ConnectionCheck:
  id: ConnectionCheckKind
  status: passed | pending | failed | blocked | not_applicable
  depends_on: ConnectionCheckKind[]
  cause_finding_ids: DiagnosticFindingId[]
  code?: string
  summary: string
  details?: object
  observed_at?: UtcTimestamp

ConnectionAction:
  id: ConnectionActionKind
  instruction: string
```

`status`, `checked_at`, `checks`, `root_cause_ids`, `actions`, 각 check 또는 action의 선택 사항이 아닌
구성원은 필수입니다. 선택적인 `code`, `details`, `observed_at` 값이 없으면 null로
직렬화하지 않고 구성원을 생략합니다. 알 수 없는 구성원, 중복 JSON key, 중복 check kind,
중복 action kind, 비정규 순서, 선택 구성원의 명시적 null, 알 수 없는 상태, check kind,
action kind 값은 유효하지 않습니다. Null이 아닌 check code는 ASCII 1~128 byte이고
`[a-z][a-z0-9_]*`와 일치해야 합니다. `summary`와 `instruction`은 UTF-8 1~4,096
byte이고 NUL을 포함하지 않습니다. Null이 아닌 `details`는 직렬화 형태가 최대 16 KiB인
JSON 객체입니다. Check 하나에는 dependency edge를 최대 16개, root finding reference를
최대 32개 둘 수 있습니다. 보고서는 check를 최대 64개, action을 최대 32개 포함하며
직렬화 형태는 최대 64 KiB입니다.

Check는 `ConnectionCheckKind`의 안정적인 snake-case 표기를 기준으로 UTF-8 byte
오름차순 정렬합니다. Action도 `ConnectionActionKind`의 안정적인 snake-case 표기를
기준으로 같은 순서를 사용합니다. 엄격한 decoding은 다른 순서를 조용히 정규화하지 않고
거부합니다. Enum 선언 순서는 wire 순서 계약이 아닙니다.

`ConnectionCheckKind`는 현재 제품의 닫힌 어휘입니다. 정확한 값은
`connection_removal`, `diagnostic_lookup`, `guard_files`, `guard_hook_execution`,
`guard_observation`, `host_executable`, `host_session`, `managed_config`, `mcp_server`,
`mode_transition`, `process_startup`, `project_trust`, `required_tools`,
`runtime_session_lookup`, `setup_plan`, `tool_round_trip`, `verification_not_run`입니다. 운영 검증은 아래 표에서
적용되는 check를 사용합니다.
보고서 부재와 관리 명령 계획은 나머지 이름 붙은 kind를 사용합니다.
`diagnostic_lookup`과 `runtime_session_lookup`은 한도가 있는 해당 관리 diagnostic
operation에서만 사용하며, 어댑터가 임의로 정한 check ID는 받지 않습니다.

`ConnectionActionKind`는 현재 제품의 닫힌 어휘입니다. 정확한 값은
`apply_removal`, `apply_setup`, `host_trust_required`,
`inspect_codex_protocol`, `install_or_repair_codex`, `observe_codex`,
`reload_host`, `repair_guard`, `repair_managed_config`, `repair_mcp_server`,
`run_verification`입니다. `HostPlan`, `HostEffect`, 검증 보고서, 명령 보고서는 정규
`ConnectionAction` 계약을 직접 사용합니다. 이 직접 계약에서 `observe_codex`와
`inspect_codex_protocol`은 `reload_host`와 서로 다른 행동으로 유지됩니다.

Connection action은 안정적인 kind와 사용자 지시로 의미 있는 작업을 표현하며 실행 가능한
셸 텍스트를 포함하지 않습니다. JSON 소비자는 action 내용을 실행하지 않고 action ID와
지시를 보고서 사실로 사용합니다. 완전한 현재 selector 좌표가 있으면 사람용 렌더러가 typed
현재 CLI 호출 맥락에서 실행 안내를 구성합니다. 렌더러가 담당하는 이 안내는 action JSON이나
영속 보고서에 복사하지 않습니다.

보고서의 모든 check는 그 보고서에 필수입니다. 최상위 상태는 check에서 파생됩니다.

1. `failed` 또는 `blocked` check가 하나라도 있으면 `status=failed`입니다.
2. 그렇지 않고 `pending` check가 하나라도 있으면 `status=action_required`입니다.
3. 그렇지 않으면 `passed`와 `not_applicable` check만 있으므로
   `status=complete`입니다.

다섯 check 상태의 의미는 정확히 다음과 같습니다.

- `passed`: check가 성공적으로 완료되었습니다.
- `pending`: 필요한 외부 관찰이나 사용자 유발 event가 아직 없고, 현재 이를 막는 실패한
  prerequisite도 없습니다.
- `failed`: check 자체가 실패를 관찰했습니다.
- `blocked`: prerequisite check가 실패하여 이 check를 실행하거나 관찰할 수 없습니다.
- `not_applicable`: 이 Connection 또는 profile에는 check가 적용되지 않습니다.

`depends_on`은 check kind별 정규 명시 dependency edge 집합입니다. 운영 검증은 다음 chain을
사용합니다.

```text
managed_config -> process_startup -> host_session -> required_tools -> tool_round_trip
managed_config -> mcp_server
guard_files -> guard_hook_execution -> guard_observation
```

`host_session`은 managed host의 `initialize` check이고, `required_tools`는 managed host의
`tools/list` check이며, `tool_round_trip`은 정규 verification role 도구 호출 check입니다.
`ToolVerificationRole::ManagedHostRoundTrip`의 정규 담당 도구는 정확히 하나이며 현재
`volicord.list_projects`입니다.
Managed-host 시도가 한 번도 없으면 `process_startup`부터 `tool_round_trip`까지 네 check는
`pending`입니다. Initialize가 실패하면 `host_session`은 `failed`이고,
`required_tools`와 `tool_round_trip`은 같은 root finding 때문에 `blocked`입니다. Managed
configuration이 실패하면 `mcp_server`와 process/protocol chain을 모두 막습니다. Guard
file integrity가 실패하면 hook 실행과 phase 관찰을 막습니다.

`failed`와 `blocked` check만 `cause_finding_ids`를 가질 수 있습니다. `blocked` check는
실패했거나 blocked인 prerequisite의 독립 root finding ID를 정규 정렬하고 합친 집합을
담습니다. `root_cause_ids`는 전체 check graph에서 얻은 정렬·중복 제거 합집합입니다.
Blocked check의 원인이 실패한 prerequisite와 일치하지 않거나 dependency cycle 또는
비정규 dependency edge가 있으면 보고서는 유효하지 않습니다.

현재 Codex 연결 보고서에는 다음 운영 check가 들어갑니다.

| Check ID | 성공 관찰 | 대기 또는 적용 규칙 | 자체 실패 |
|---|---|---|---|
| `managed_config` | 선택한 대상에 정규 managed entry가 있습니다. | 모든 managed Connection에 적용됩니다. | 필수 entry가 없거나 malformed이거나 다른 entry가 소유하거나 변경되었거나 조사할 수 없습니다. |
| `host_executable` | `PATH`에서 `codex`를 찾았고 version 명령이 성공했습니다. | 읽기 전용 status 경로에 이전 active probe가 없으면 기다립니다. | 탐색 또는 version 명령이 실패했습니다. |
| `mcp_server` | CLI self-test가 preflight와 전체 MCP exchange를 통과했습니다. | Active verification을 기다리며 managed configuration 실패가 막을 수 있습니다. | Self-test 자체가 process, Store 또는 protocol 실패를 관찰했습니다. |
| `process_startup` | 현재 managed host가 구성된 MCP process를 시작했습니다. | Managed-host 사용을 기다리며 managed configuration 실패가 막을 수 있습니다. | Typed host 관찰이 없으면 managed-host startup 실패를 주장하지 않으며, 관찰 부재는 대기로 남습니다. |
| `host_session` | 현재 상태이고 host-version이 fresh인 managed-host session이 `initialize`와 initialized notification을 완료했습니다. | 조건을 충족하는 시도를 기다리며 `process_startup` 실패가 막을 수 있습니다. | 현재 시도가 initialization 또는 protocol 실패를 관찰했습니다. |
| `required_tools` | 조건을 충족하는 `tools/list` 관찰에 모든 필수 도구가 있습니다. | 도구 검색을 기다리며 `host_session` 실패가 막을 수 있습니다. | 도구 검색이 완료됐지만 필수 도구가 없거나 유효하지 않습니다. |
| `tool_round_trip` | 조건을 충족하는 현재 상태이고 host-version이 fresh인 session이 `verification_tool_name=volicord.list_projects`와 `verification_tool_observed_at`을 모두 기록했습니다. | 정규 role 담당 도구 호출을 기다리며 `required_tools` 실패가 막을 수 있습니다. | 호출 자체가 protocol 또는 contract 실패를 관찰했거나 기록한 도구 이름이 현재 정규 담당 도구와 다릅니다. |
| `project_trust` | Project trust가 충족되었습니다. | 일반 trust 또는 reload action은 `pending`이고, 별도 trust check가 없는 scope는 `not_applicable`입니다. | Trust configuration이 malformed 또는 모순 상태입니다. |
| `guard_files` | 현재 Guard manifest의 모든 file 기대값이 일치합니다. | Guard가 Connection profile에 포함될 때 적용됩니다. | Managed file, manifest, wrapper, ownership 또는 executable integrity check가 실패했습니다. |
| `guard_hook_execution` | 현재 managed Guard hook이 실행되었습니다. | 현재 hook 활동을 기다리며 `guard_files` 실패가 막을 수 있습니다. | Hook 실행 자체가 실패를 기록했습니다. |
| `guard_observation` | 현재 필수 typed hook phase를 모두 관찰했습니다. | 남은 phase를 기다리며 `guard_hook_execution` 실패가 막을 수 있습니다. | 현재 event가 incompatible hook contract를 보고했습니다. |

CLI MCP self-test는 `session_source=cli_preflight`만 만듭니다. 따라서
`process_startup`, `host_session`, `required_tools`, `tool_round_trip`을 충족할 수 없습니다.
Guard는 최상위 운영 check로 `guard_files`, `guard_hook_execution`,
`guard_observation`을 사용합니다. 엄격한 Guard manifest는
현재 policy hash, integration revision, typed runtime command, 전체 Volicord 관리 artifact
기대값, 필수 hook phase를 담당합니다. Policy command와 runtime command는 typed invocation
하나에서 나온 서로 다른 projection입니다. Audit은 각 관리 artifact를 정규 현재 기대값과
비교하고, Guard 관찰은 모든 필수 phase의 호환되는 현재 event를 요구합니다.

기록한 verification 도구 이름이 다르면 `tool_round_trip`은 절대 통과하지 않습니다.
Check는 `tool_round_trip_designation_mismatch`로 실패하고 활성 검증은
`mcp.tool_verification.designation_mismatch`를 영속합니다. 제한된 facts에는 정확한
`expected_tool_name`과 `observed_tool_name`을 노출하며 JSON check detail과 verbose 출력도
정확한 기대 이름과 관찰 이름을 표시합니다. 이전 revision, CLI preflight row, timestamp
부재, stale host-version 관찰은 현재의 정확한 쌍을 대신할 수 없습니다.

제한 안의 모든 Codex version은 이 동작 check를 거칩니다. Version이 바뀌면 Codex를
reload하고 운영 동작을 다시 관찰할 때까지 현재 host 관찰이 pending이 됩니다.

`dry_run`은 작업 mode이며 연결 상태나 check 상태가 아닙니다. 구성 일치, 실행 파일
가용성, protocol/host version, capability 관찰, 관찰 timestamp는 check 사실에 두며
별도 공개 또는 영속 상태 enum을 만들지 않습니다.

사용자 지시는 이 보고서의 `actions`에만 둡니다. Root finding과 현재 check 상태에서
만들고 안정적인 ID 순서로 정렬해 중복을 제거합니다. Blocked downstream check는 관찰
action을 만들지 않으며 blocker의 repair action을 먼저 제공합니다. 여러 symptom에서 나온
동등한 action은 안정적인 action 하나로 합칩니다. 다시 불러오기와 최초 사용 action은
실제 Codex 활동을 관찰해야 한다고 명시합니다. `guard_files` check가 통과했다면 Guard
파일 재설치를 요청하지 않습니다. Registry 저장소는 독립된 검증 상태나 action 배열을
저장하지 않습니다. 완료된 영속 보고서가 없는 연결은
`verification_not_run` pending check 하나와 검증 action 하나를 포함하는 합성
`status=action_required` 보고서로 projection합니다. 읽었다는 이유로 이를 저장하지
않습니다.

관리 CLI는 init, add, status, verify, mode, remove를 현재 schema 2
`DiagnosticReport`로 projection합니다. 정규 check, Store API로 읽은 한도 있는 finding과
cause edge, 계산한 root ID, root마다 중복 제거한 typed action 하나, Connection context,
operation별 result detail, report limit을 담습니다. Concise, verbose, JSON 출력은 같은
report의 projection이며 같은 root를 식별합니다. 렌더러는 summary 산문에서 cause나
remediation category를 만들지 않고 `DiagnosticFinding`이 가린 fact를 다시 노출하지
않습니다.

현재 Connection 보고서는 `failed` 및 `blocked` check가 참조한 정확한 ID에서 finding을
선택하고 그 finding의 한도가 있는 cause chain만 읽습니다. 독립적인 현재 finding은 작업이
의도적으로 선택한 경우에만 나타나며, 같은 integration revision에 저장된 모든 finding을
현재 상태로 취급하지 않습니다. CLI 소유 현재 상태 운영 finding은 typed subject로 정확한
managed-config target, Product Repository, Guard 관리 artifact, Guard phase 또는 event,
Guard Installation, runtime session을 식별합니다. 안정적인 ID는 해당 정규 subject의
한도가 있는 digest를 포함하므로 artifact나 phase가 둘이면 diagnostic code가 같아도 finding
둘을 유지하고, 주체 하나를 다시 관찰하면 그 snapshot만 갱신합니다.

JSON projection은 `generated_at`을 report 시각으로 담고, 값이 있으면 Connection
context에 정확한 현재 integration revision을 담습니다. 영속 verification의 `checked_at`은
해당 verification 관찰 시각으로 남으며 경쟁하는 두 번째 최상위 시각으로 반복하지
않습니다. Status는 저장된 active-probe fact와 현재 관찰에서 메모리 안의 최신 projection을
다시 만들 수 있지만, 이 읽기는 projection을 영속하거나 timestamp를 바꾸지 않습니다.
참조된 finding row가 없으면 typed `diagnostics.finding_record_missing` 관찰로 표시하고 현재
관찰을 다시 만들도록 안내합니다. 렌더링 과정에서 누락된 domain fact를 꾸며 내지 않습니다.

활성 검증은 plan이나 probe를 시작하기 전에 정확한 typed Connection integration revision을
확보합니다. Store는 같은 revision이 여전히 현재 상태일 때만 비교와 보고서 교체를 immediate
Registry transaction 하나에서 수행합니다. 이 쓰기는 `verification_report_json`과 일반 row
갱신 timestamp만 바꿉니다. Revision 충돌이 나면 기존 보고서와 모든 소유자 field를 그대로
두고 검증을 다시 실행하도록 요구합니다. 검증은 관리 configuration을 관찰할 뿐 새로 계획한
managed fingerprint를 적용하거나 채택하거나 기록하지 않습니다.

영속 action에 `id`와 `instruction` 이외의 구성원이 있으면 현재 보고서 형태가 잘못된
것이며 엄격한 읽기에서 거부합니다. 활성 검증은 같은 revision 보호 교체 경계를 통해 이런
잘못된 보고서를 교체할 수 있지만 관련 없는 Connection 소유자 상태는 바꾸지 않습니다.
JSON을 제자리에서 다시 쓰거나 다른 decoder를 사용하는 경로는 없습니다.

운영 호환성은 현재 관리 구성과 어댑터가 실제로 관찰한 protocol, 도구 목록, 필수 도구,
안전 호출, Guard 동작으로 판단합니다. `complete`는 적용되는 모든 필수 check가 통과하고
나머지 check가 모두 `not_applicable`임을 뜻합니다. 운영체제 집행, actor 또는 human identity, correctness, 미래 동작,
조작 방지 기록을 성립시키지는 않습니다. Core 호출 권한은 각 관리 MCP 호출에서 별도로
평가합니다.

## 통합 Revision과 운영 Session

현재 Connection 통합 revision은 타입이 지정되고 domain-separated된 canonical SHA-256
digest입니다. Basis는 Agent Connection identity, 변경 불가능한 Store 소유 통합 instance
ID, host kind, intent, scope, mode, server name, configuration target, 현재의 정확한
managed-configuration fingerprint, Store 소유 비음수 integration generation입니다. 이
fingerprint는 관리 server command와 entry를 포함하며, setup 담당 경로가 마지막으로 적용에
성공했거나 채택한 Volicord 관리 host configuration을 식별합니다. Setup, repair, staged
activation 또는 다른 명시적 managed-configuration 변경만 이 값을 교체할 수 있습니다.
교체하면 integration revision이 바뀌고 이전 verification report를 같은 transaction에서
원자적으로 비웁니다. Fingerprint가 같은 호환 replay는 그 보고서를 유지할 수 있습니다.

Store는 새 물리 `agent_connections` 행을 삽입할 때만 새 opaque 통합 instance ID를
생성합니다. 호환 등록 replay, enabled 상태와 검증 갱신, staged activation과 cleanup 복구,
mode 전환은 이 값을 보존합니다. 물리 삭제는 행과 함께 instance를 제거합니다. 따라서
호출자에게 보이는 target과 configuration 입력이 모두 같고 결정적 Connection identity를
재사용하더라도 다시 만든 Connection은 새 통합 instance ID를 받습니다.

Integration generation은 해당 물리 instance 안에서 0으로 시작하고 실제 mode 전환이
성공할 때마다 정확히 한 번 증가하며, 같은 mode를 지정한 no-op에서는 그대로 유지됩니다.
그러므로 generation은 물리 instance 하나 안의 revision을 구분하고, 통합 instance ID는
삭제와 재생성을 구분합니다. 이전에 사용한 mode로 돌아가더라도 새 revision이 만들어지고
그 이전 mode generation의 evidence는 다시 현재 상태가 될 수 없습니다.

관찰한 executable path, host version, MCP client name/version은 diagnostic 사실로
남습니다. Host version이 바뀌면 운영 관찰을 갱신하며, 권한은 현재 Connection, revision,
권위 있는 runtime/project session binding을 사용합니다.

실제 MCP peer는 해당 runtime session에서 관찰한 제한된 `clientInfo.name`과
`clientInfo.version`입니다. PATH probe는 별도로 관찰한 Codex executable path와
version입니다. 보고서와 finding은 둘을 서로 대신 사용하지 않습니다. 두 version이 모두
있고 서로 다르면 `host.codex.peer_version_differs_from_path_probe`가 두 사실 객체를 담은
warning evidence를 기록하며, 이 불일치만으로 Connection을 치명적 실패로 만들지 않습니다.
잘못된 native metadata, 일관되지 않은 session/thread/turn 좌표, 등록 session 불일치,
managed marker 불일치는 각각 `host.codex.metadata_malformed`,
`host.codex.session_thread_turn_inconsistent`,
`host.codex.registered_session_correlation_mismatch`,
`host.codex.managed_marker_mismatch`를 사용합니다.

각 MCP process 시작은 host thread metadata가 생기기 전에 불투명 Registry runtime
session ID를 만듭니다. `session_source`는 정확히 `managed_host` 또는
`cli_preflight`입니다. `managed_host`만 Agent Connection 호출을 승인할 수 있습니다.
Runtime session은 소유 Connection과 Connection 통합 revision을 보관합니다.

유효한 initialize 요청 뒤 해당 runtime은 session 범위의 typed MCP selection 하나를
소유합니다. 이 값은 요청 protocol 문자열, 선택한 프로덕션 profile, exact match 또는 서버
counter-offer 결과, client capability, 제한 안의 시도된 client name/version, initialized
notification의 handshake 완료 여부를 보관합니다. 선택한 profile에서 initialize 결과
revision과 capability를 만들고 이후 lifecycle을 검증합니다. 선택은 협상 완료가 아닙니다.
필수 initialized notification이 유효하게 도착한 뒤에만 profile의 협상이 끝나며 그 revision을
권위 있는 runtime-session protocol 관찰로 기록합니다. 다시 연결하면 새 runtime과 새
selection을 만들며 process 사이에서 profile을 공유하거나 상속하지 않습니다.

프로젝트 통합 revision은 Connection revision에 현재 프로젝트 workflow-policy
fingerprint와 현재 Guard installation identity/policy hash 또는 Guard ownership의 명시적
부재를 더합니다. 프로젝트 Agent Session은 이 revision, 결정적인 revision 범위 session
ID, Connection, host session/thread/latest turn, 최초/마지막 관찰 시각을 보관합니다.
Store는 프로젝트를 결정하고 현재 Guard ownership을 검증한 뒤에만 Connection internal ID,
정확한 프로젝트 통합 revision, 정확한 host-native session ID에 domain-separated digest를
적용해 내부 ID를 도출합니다. 호출자는 완성된 내부 ID를 제공할 수 없습니다. 저장된
프로젝트 revision은 변경할 수 없습니다. 이후 Connection mode generation, 물리 Connection
재생성, 프로젝트 정책 revision, Guard ownership revision이 생기면 같은 native session에도
서로 다른 프로젝트 Agent Session row를 만들고 이전 row는 이력으로 남깁니다. Guard
관찰은 runtime binding이 null인 session을 만들 수 있습니다. 해당 host session의 첫 실제
managed MCP 도구 호출은 먼저 현재 managed runtime을 변경 없이 검증하고, 정확한 프로젝트
Agent Session anchor를 만들거나 검증한 뒤, Registry에서 정확한 데이터베이스 간 binding을
예약하면서 현재 소유자 입력을 다시 검증합니다. 마지막 프로젝트 transaction에서만 runtime을
붙입니다. 프로젝트 anchor에서 확인한 Connection, 프로젝트, Guard Installation, revision,
native session, thread, 기존 runtime 충돌은 새 Registry binding을 남기지 않습니다. 이후
Registry 예약이 실패하면 프로젝트 anchor가 unbound로 남을 수 있지만 권한은 아닙니다.
마지막 attach 전 중단으로 남은 Registry 예약도 권한이 아니며, 소유자 상태가 바뀌지 않은
정확한 replay가 그 예약을 재사용해 attach를 완료합니다. 붙인 session은 다른 runtime
session, Connection, 프로젝트, host session, host thread에 다시 결속할 수 없습니다.

Runtime row는 lease나 liveness 주장이 아니라 process의 이력 관찰입니다. Crash한 process는
열린 것처럼 보이는 row를 남길 수 있고 여러 협력적 Codex process가 동시에 현재 상태일 수
있습니다. 어느 경우도 Guard 상관관계를 막지 않습니다. 열린 row에서 runtime을 추측하지
않고 서로 다른 host session은 독립적으로 결속합니다.

실제 Connection mode 전환은 Connection과 소유한 모든 Guard Installation의 엄격한
manifest를 아우르는 Registry transaction 하나입니다. 이 transaction은 mode, integration
generation, 저장된 verification report, manifest integration revision, 영향을 받은 Registry
timestamp만 바꿉니다. Guard command, managed-file inventory, policy hash, host
configuration, Product Repository file은 바꾸지 않습니다. 모든 후보가 완전하고 현재 상태이며
소유자가 일치해야 쓰기를 시작하고, 그렇지 않으면 전환을 commit하지 않습니다.

이 기록은 현재 구성 아래 로컬에서 관찰한 협력적 protocol/session 소유권을 성립시킵니다.
Client, host, actor, 운영체제 사용자, human identity를 성립시키지는 않습니다. MCP client
name/version과 관찰한 host executable version은 제한 안의 임의 미래 값을 받고 diagnostic으로만
남습니다.

Integration instance ID, integration generation, 파생 integration revision은 로컬 lifecycle과
상관관계 좌표입니다. Store가 lifecycle 입력을 소유하며 호출자는 이를 선택할 수 없습니다.

<a id="validated-agent-session"></a>

## `ValidatedAgentSession`

Core는 직렬화할 수 없는 아래 typed 경계로만 Agent Connection 호출 권한을 받습니다.

```rust
struct ValidatedAgentSession {
    connection_id: AgentConnectionId,
    project_id: ProjectId,
    runtime_session_id: AgentRuntimeSessionId,
    project_session_id: AgentSessionId,
    integration_revision: IntegrationRevision,
}
```

다음 현재 사실을 모두 검증한 뒤에만 이 값을 만듭니다.

1. Agent Connection이 존재하고 활성 상태입니다.
2. 프로젝트가 존재하고 현재 Connection Project입니다.
3. Runtime session이 해당 Connection에 속합니다.
4. 프로젝트 session에 null이 아닌 runtime binding이 있습니다.
5. Registry binding이 runtime, Connection, 프로젝트, 프로젝트 session, host session과
   정확히 일치합니다.
6. 프로젝트 session이 해당 runtime session, Connection, 프로젝트에 속합니다.
7. Runtime과 프로젝트 session revision이 현재 Connection/프로젝트 통합 revision과
   일치합니다.
8. Connection mode가 요청한 operation category를 허용합니다.
9. `ActorSource::AgentConnection`이 검증된 Connection을 정확히 이름 붙입니다.
10. 프로젝트 범위 operation이 검증된 프로젝트를 정확히 이름 붙입니다.
11. Runtime session의 `session_source=managed_host`이며 `cli_preflight`가 아닙니다.

어댑터는 프로젝트 도구를 호출할 때마다 Core 호출 맥락을 만들기 전에 권위 있는 runtime
및 프로젝트 row를 검증합니다. Executable path, host version, client version은 이 권한
판정 밖의 diagnostic으로 남습니다.

Core는 감사 basis를 결정적으로 만듭니다.

```text
connection:<connection_id>/session:<project_session_id>/revision:<project_integration_revision>
```

이 basis는 감사 event에 기록된 검증된 운영 소유권의 결정적인 로컬 lifecycle 및
상관관계 좌표입니다.

## Codex 어댑터 책임

Codex 어댑터는 host별 구성 조사와 변경을 담당합니다.

- Codex configuration target 탐색
- 현재 Connection 입력이 선택한 관리 entry만 설치
- 정규 관리형 MCP 시작 계약을 Codex TOML로 projection하고 같은 계약으로 엄격하게
  다시 parsing
- 누락되거나 변경되거나 추가된 관리 구성을 drift로 탐지
- executable 가용성과 제한된 host version diagnostic 보고
- 현재 정규 입력으로 담당 문서가 정의한 관리 상태 repair
- 일치하는 Volicord 관리 상태만 제거

Codex 어댑터는 네이티브 Linux와 WSL2를 분류하지 않고 프로세스 target이나
파일시스템 제한을 검증하지 않습니다. 해당 관찰과 검사는
[시스템 요구사항](system-requirements.md)에 따른 플랫폼 파일시스템 경계가 담당합니다.

Setup과 repair는 먼저 plan하고 검증한 뒤 host configuration을 적용하거나 채택하고, 그 결과인
managed fingerprint 및 소유자와 일관된 Registry/Guard 상태를 commit합니다. 검증은 이 최종
Connection record에서만 시작하며 해당 record의 정확한 integration revision을 기준으로
보고서를 영속합니다. 보고서 영속은 fingerprint를 두 번째로 갱신하지 않습니다.

Runtime 권한은 현재 활성 Connection, 프로젝트 membership, 허용 mode, 관리 runtime
session, revision 범위 프로젝트 session, 정확한 Registry/프로젝트 binding을 검증합니다.
Command name, executable path, version string, 환경 값, 로컬 session metadata는 diagnostic
또는 routing 사실이며 actor나 human identity를 성립시키지 않습니다.

Repair는 관련 없는 Codex 구성을 덮어쓰거나 선택한 프로젝트, Connection, intent,
profile, 플랫폼 환경을 암묵적으로 바꾸지 않습니다. `workflow`와 `read_only` 모두에서
누락된 소유 Codex 구성, Guard 파일, 현재 Guard Installation을 복구하면서 Connection
mode와 integration generation을 보존합니다. Repair가 다른 정규 관리 configuration을
적용하면 새 managed fingerprint를 기록하고 integration revision을 바꾸며, 새 revision에서
검증하기 전에 이전 verification report를 무효화합니다. Fingerprint가 같으면 현재 revision을
보존합니다. 제거는 현재 관리 identity가 Volicord 소유와 계속 일치하는 내용만 삭제합니다.

명시적인 connection 제거 명령은 Connection 소유 Registry 통합 상태도 폐기합니다.
Membership 하나를 제거하면 해당 Registry project-session binding과 Guard
Installation을 삭제합니다. 여러 프로젝트를 가진 personal Connection은 마지막
membership이 제거될 때까지 connection 전체 runtime session과 다른 프로젝트 소유 행을
유지합니다. 마지막 membership 제거는 남은 Registry project-session binding, Guard
Installation, runtime session, Agent Connection을 모두 삭제합니다. 프로젝트 로컬 Agent
Session, Guard 관찰, workflow 이력, evidence와 그 밖의 권한 기록은 Product Repository의
과거 상태로 유지됩니다. 현재 Registry membership과 현재 검증된 runtime/project
session이 없으면 이 과거 기록은 이후 호출에 권한을 부여할 수 없습니다.

Connection migration은 같은 프로젝트 범위 Registry 폐기 순서를 사용합니다. 여러
프로젝트를 가진 superseded Connection에서는 원자적 replacement 활성화 transaction 안에서
선택한 프로젝트의 runtime/project binding, Guard Installation, membership만 제거합니다.
마지막 프로젝트를 가진 superseded Connection에서는 외부 정리가 성공할 때까지 기존
Connection을 비활성화하고 그 프로젝트의 완전한 inventory와 pending-host-cleanup marker를
유지합니다. 최종 Registry transaction은 replacement와 정확한 유지 inventory를 다시
검증한 뒤 binding, Guard Installation, membership을 삭제하고 marker를 지웁니다. 정리나
재검증이 실패하면 inventory를 온전히 유지하여 재시도할 수 있으며, 정리가 성공해도
membership이 없는 비활성 과거 Connection과 그 connection 전체 runtime session은 유지합니다.

## 위협 모델

신뢰 대상:

- 동일 운영체제 사용자 계정
- 해당 계정이 소유한 `Volicord Runtime Home`
- 해당 계정의 Store 쓰기 권한

비신뢰 대상:

- 외부 host/client 입력
- CLI-preflight, 오래됨, 닫힘, revision 불일치 session
- 다른 프로젝트, runtime, Connection의 session
- 수동으로 변경한 구성
- identity 주장으로 쓰는 client/host version과 process metadata

동일 사용자 권한으로 실행되는 악성 프로세스의 Runtime Home 변조는 최초 릴리스 위협
범위 밖입니다. 따라서 로컬 기록은 협력적이며 같은 계정 접근 권한을 가진 다른
프로세스에 대해 변조 방지를 보장하지 않습니다.

## 인접 담당 문서

- 관리 stdio MCP 동작: [MCP 전송](mcp-transport.md)
- 설치, 검증, repair, 제거 명령: [관리 CLI](admin-cli.md)
- 플랫폼 셀과 WSL2 배치: [시스템 요구사항](system-requirements.md)
- 일반 빌드, 패키지, 플랫폼, 릴리스 검증:
  [검증](../maintain/validation.md)
- Runtime Home 및 Product Repository 경계: [런타임 경계](runtime-boundaries.md)
- 보안 보장과 비보장: [보안](security.md)
