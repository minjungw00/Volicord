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
| 어댑터 모듈, 파일시스템 helper, 숨겨진 launcher, Store lease/query helper | `internal` | 안정된 경계를 보존하지만 공개 표면은 아닙니다. |
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
| 전송 | 숨겨진 host launcher로 시작하는 Volicord 관리형 stdio MCP. 공개 수동 stdio는 `volicord mcp serve` |
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

하나의 typed 관리형 MCP 시작 계약이 숨겨진 launcher 명령과 인자, 정적 및 전달
Runtime Home binding, 개인/공유 구분, 엄격한 시작 형태 검증, 정규 projection,
결정적 managed fingerprint 입력의 정규 출처입니다. 관리 provenance는 one-time launch
lease 소비에 성공한 경우에만 시작됩니다.

개인 연결에는 선택한 정규 절대 Runtime Home과 선택한 절대 `volicord` 실행 파일이
필요합니다. 숨겨진 host 소유 launcher를 호출하고, 프로세스 구성으로는 Runtime Home만
저장하며 부모 환경 이름은 전달하지 않습니다.

```toml
[mcp_servers.volicord]
command = "/absolute/path/to/volicord"
args = ["_host-launch", "codex", "--connection", "<connection_id>"]

[mcp_servers.volicord.env]
VOLICORD_HOME = "/absolute/runtime/home"
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
args = ["_host-launch", "codex", "--discover-repository"]
env_vars = ["VOLICORD_HOME"]
```

공유 시작에는 절대 실행 파일, Runtime Home, Connection ID, project ID 또는 그 밖의
머신 로컬 lifecycle 좌표를 넣지 않습니다. 프로젝트 인자나 프로젝트 환경 marker가
있는 개인 entry, 정적 환경과 전달 환경의 이름 충돌, 비어 있거나 중복된 전달 이름,
불완전한 개인 binding, 개인/공유 인자 또는 환경 형태의 혼합은 유효하지 않습니다.

생성 Codex 구성은 어댑터가 이 계약을 projection한 결과입니다. 이 구성에는 launch
lease, nonce, 재사용 가능한 secret, raw OS handle을 넣지
않습니다. CLI 검증은 같은 binding 사실에서 공개 preflight와 수동 stdio probe 명령을
파생하며, 두 probe 모두 managed-host launch가 아닙니다.

Codex 어댑터는 현재 TOML 형태를 parsing하고 같은 typed 계약을 다시 구성하여 관리
entry를 검증합니다. 알 수 없는 launch key, 잘못된 값, 비정규 형태는 두 번째 허용
형태가 아니라 drift입니다. 어댑터는 유효한
`tools.<known-tool>.approval_mode` overlay만 보존하고 launch identity에서는
제외합니다. Managed fingerprint는 정규 launch projection과 host kind, scope, server
name을 포함합니다. 서식 차이는 fingerprint를 바꾸지 않지만 launch 의미가 달라지면
바뀝니다.

숨겨진 launcher는 현재 정규 entry를 엄격하게 다시 읽고, 활성 Connection과 정확한
integration revision, entry fingerprint와 현재 저장된 managed fingerprint의 일치를
검증합니다. 그 뒤 수명이 짧은 Registry launch lease 하나를 만들고 메모리 안에서 stdio
adapter로 전환합니다. Lease ID는 Codex 구성, 프로세스 인자, 로그, 공개 환경 변수에
기록하지 않습니다. MCP bootstrap은 lease를 원자적으로 한 번 소비하면서
`managed_host` runtime session을 만듭니다. 소비할 때 launcher가 포착한 정확한
Connection, `codex` host kind, integration revision, managed fingerprint가 모두
일치해야 합니다. Lease는 한 번만 사용할 수 있으며 replay, 만료, 불일치, 취소는
fail closed하고, 정상적인 launcher 실패는 아직 쓰지 않은 lease를 terminal state로
전환합니다.

<a id="connection-verification-report"></a>

## `ConnectionVerificationReport`

보고서 하나가 정규 직렬화 Connection 및 activation 상태입니다.

```yaml
ConnectionVerificationReport:
  status: complete | action_required | failed
  activation_state: configured | host_reload_required |
    hook_review_required_or_unknown | mcp_observation_required |
    guard_verification_required | complete | failed
  hook_activation_state: unknown | review_required_by_setup |
    effective_by_observation | managed_by_policy |
    bypassed_for_invocation | disabled
  checked_at: UtcTimestamp
  checks: ConnectionCheck[]
  root_cause_ids: DiagnosticFindingId[]
  activation_plan: IntegrationActivationPlan

ConnectionCheck:
  id: ConnectionCheckKind
  status: passed | pending | failed | blocked | not_applicable
  depends_on: ConnectionCheckKind[]
  cause_finding_ids: DiagnosticFindingId[]
  code?: string
  summary: string
  details?: object
  observed_at?: UtcTimestamp

IntegrationActivationPlan:
  state: IntegrationActivationState
  required_steps: ActivationStep[]
  optional_diagnostics: ActivationStep[]

ActivationStep:
  id: ActivationStepId
  initiator: user | host | volicord | agent
  executor: user | host | volicord | agent
  execution_channel: cli | codex_ui | codex_chat | mcp_tool
  prerequisites: ActivationStepId[]
  completes_checks: ConnectionCheckKind[]
  root_finding_ids: DiagnosticFindingId[]
  instruction: string
  diagnostic_only: boolean
  agent_sequence: AgentSequenceStep[]

AgentSequenceStep:
  tool: AgentToolId
  condition: always | workflow_awaiting_probe |
    workflow_awaiting_observation
```

`status`, `activation_state`, `hook_activation_state`, `checked_at`, `checks`,
`root_cause_ids`, `activation_plan`과 위에 표시한 plan 및 step 구성원은 모두
필수입니다. 선택적인 check 구성원 `code`, `details`, `observed_at` 값이 없으면 null로
직렬화하지 않고 생략합니다. 알 수 없는 구성원, 중복 JSON key, 중복 check kind, 중복
activation step ID, 선택 구성원의 명시적 null, 알 수 없는 enum 값은 유효하지 않습니다.
Null이 아닌 check code는 ASCII 1~128 byte이고 `[a-z][a-z0-9_]*`와 일치해야 합니다.
`summary`와 `instruction`은 UTF-8 1~4,096 byte이고 NUL을 포함하지 않습니다. Null이
아닌 `details`는 직렬화 형태가 최대 16 KiB인 JSON 객체입니다. Check 또는 activation
step 하나에는 dependency edge를 최대 16개, root finding reference를 최대 32개 둘 수
있습니다. 보고서는 check를 최대 64개 포함하고 activation plan은 필수 및 선택 step을
합쳐 최대 32개 포함합니다. 직렬화한 보고서는 최대 64 KiB입니다.

Check는 `ConnectionCheckKind`의 안정적인 snake-case 표기를 기준으로 UTF-8 byte
오름차순 정렬합니다. Activation step은 `prerequisites`의 결정적 위상 순서를 사용하고,
서로 독립인 step은 현재 workflow 순서로 정합니다. 직렬화 ID 표기는 정렬 규칙이
아닙니다. 엄격한 decoding은 비정규 위상 순서를 조용히 정규화하지 않고 거부합니다.
Plan 생성은 cycle, 알 수 없는 prerequisite, 중복 step ID, 최상위에 노출된 nested
agent tool, `required_steps` 안의 diagnostic-only step을 거부합니다.

`ConnectionCheckKind`는 현재 제품의 닫힌 어휘입니다. 정확한 값은
`connection_removal`, `diagnostic_lookup`, `guard_files`, `ambient_hook_coverage`,
`guard_observation`, `correlated_guard_verification`, `hook_source_activation`,
`host_executable`, `host_reload`, `host_session`, `managed_capability_proof`,
`managed_config`, `managed_session_health`, `mcp_server`, `mode_transition`,
`process_startup`, `project_trust`, `required_tools`,
`runtime_session_lookup`, `setup_plan`, `tool_round_trip`,
`verification_not_run`입니다. 운영 검증은 아래 표에서 적용되는 check를 사용합니다.
보고서 부재와 관리 명령 계획은 나머지 이름 붙은 kind를 사용합니다.
`diagnostic_lookup`과 `runtime_session_lookup`은 한도가 있는 해당 관리 diagnostic
operation에서만 사용하며, 어댑터가 임의로 정한 check ID는 받지 않습니다.

`ActivationStepId`는 현재 제품의 닫힌 어휘입니다. 정확한 값은 `reload_codex`,
`review_project_hooks`, `request_integration_verification`,
`read_connection_status`, `run_optional_active_diagnostics`,
`repair_hook_contract`, `repair_managed_configuration`입니다.
검증 보고서의 `IntegrationActivationPlan`이 유일한 activation plan 소유자이며,
`HostPlan`과 `HostEffect`에는 별도 step 목록이 없습니다.

Activation step은 안정적인 ID, 서로 구분된 initiator와 executor, 실행 channel, step
prerequisite, 완료 의도 check, root finding, 한도 있는 instruction, diagnostic 분류,
선택적인 nested agent sequence로 의미 있는 작업을 표현합니다. 실행 가능한 셸 텍스트는
포함하지 않습니다. JSON 소비자는 instruction 내용을 실행하지 않고 이 구성원을 보고서
사실로 사용합니다.
완전한 현재 selector 좌표가 있으면 사람용 렌더러가 typed
현재 CLI 호출 맥락에서 실행 안내를 구성합니다. 렌더러가 담당하는 이 안내는 step JSON이나
영속 보고서에 복사하지 않습니다.

### Hook activation 근거

`HookActivationState`는 Volicord가 근거를 특정할 수 있는 상태만 보고합니다.

| Variant | Wire 값 | 필요한 근거 |
|---|---|---|
| `Unknown` | `unknown` | 권위 있는 host 상태도 없고 현재 hook definition에 맞는 호환 event도 없습니다. 관찰 부재는 trust 판정이 아닙니다. |
| `ReviewRequiredBySetup` | `review_required_by_setup` | Setup이 프로젝트 로컬 hook definition을 만들거나 바꿨습니다. 이전 definition이 실행된 적이 있어도 host review를 다시 해야 합니다. |
| `EffectiveByObservation` | `effective_by_observation` | 현재 설치 definition, policy hash, integration revision, installation 경계에 맞는 호환 Guard event가 있습니다. |
| `ManagedByPolicy` | `managed_by_policy` | Host가 현재 hook activation이 policy로 관리된다고 명시적으로 보고합니다. |
| `BypassedForInvocation` | `bypassed_for_invocation` | Host가 호출 한 번에 한정된 bypass를 명시적으로 보고합니다. 지속적인 activation이 아닙니다. |
| `Disabled` | `disabled` | Host가 hook source가 disabled라고 명시적으로 보고합니다. |

우선순위는 명시적인 disabled 근거, setup definition 변경, policy 관리, 호출별 bypass,
현재 definition 관찰, `unknown` 순입니다. 의도적으로 `trusted` hook 상태는 두지 않습니다.
Project 또는 configuration trust는 host/user가 소유하는 별도 관심사이며
`project_trust`로 표현합니다. Hook activation에서 trust를 추론하지 않고 trust로 hook
activation을 증명하지도 않습니다.

Guard 관찰은 현재 installation definition 경계 이후에 발생하고 policy hash,
integration revision, installation이 일치할 때만 current입니다. Byte가 같은 definition
내용을 다시 적용하면 이 경계를 유지합니다. 관리 definition 내용이 바뀌면 경계를
전진시키므로 이전 event가 새 definition의 효과를 증명할 수 없습니다. 보고서는 현재 hook
definition content hash와 주변 prompt/pre/post phase detail을 분리해 보여 줍니다.

### Connection activation 진행 상태

`IntegrationActivationState`의 정확한 variant와 안정적인 wire 값은 다음과 같습니다.

| Variant | Wire 값 | 의미 |
|---|---|---|
| `Configured` | `configured` | Managed configuration은 있지만 아직 이후 activation 단계가 확정되지 않았습니다. |
| `HostReloadRequired` | `host_reload_required` | Managed host가 현재 configuration을 다시 불러와야 합니다. |
| `HookReviewRequiredOrUnknown` | `hook_review_required_or_unknown` | 현재 hook review가 필요하거나 hook-source activation이 unknown으로 남아 있습니다. |
| `McpObservationRequired` | `mcp_observation_required` | 현재 managed-host session 및 capability 근거가 불완전합니다. |
| `GuardVerificationRequired` | `guard_verification_required` | 상관관계가 확인된 first-party Guard verification이 불완전합니다. |
| `Complete` | `complete` | 현재 activation check가 모두 완료됐습니다. |
| `Failed` | `failed` | 필수 activation 또는 diagnostic check가 실패했습니다. |

상태는 다음 순서로 파생합니다.

1. 필수 check가 failed 또는 blocked이면 `failed`
2. `managed_config`가 미완료이면 `configured`
3. `host_reload`가 미완료이면 `host_reload_required`
4. hook 상태가 `effective_by_observation` 또는 `managed_by_policy`가 아니면
   `hook_review_required_or_unknown`
5. `managed_session_health` 또는 `managed_capability_proof`가 미완료이면
   `mcp_observation_required`
6. `correlated_guard_verification`이 미완료이면 `guard_verification_required`
7. 그 밖에는 `complete`

보고서의 모든 check는 그 보고서에 필수입니다. 최상위 상태는 check에서 파생됩니다.

1. `blocked` check 또는 복구 불가능한 `failed` check가 하나라도 있으면
   `status=failed`입니다.
2. 그렇지 않고 `pending` check 또는 복구 가능한 failed
   `correlated_guard_verification`이 있으면 `status=action_required`입니다.
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
managed_config -> host_reload -> hook_source_activation
host_reload -> managed_session_health -> managed_capability_proof
hook_source_activation -> ambient_hook_coverage -> correlated_guard_verification
```

`project_trust`는 독립적으로 평가합니다. Host 소유 prerequisite를 설명할 수는 있지만
`hook_source_activation`에 합치지 않습니다. `managed_session_health`는 고정된
`latest_managed_attempt` role, 즉 현재 revision의 `managed_host` runtime 가운데 가장 최신 항목을
사용합니다. `managed_capability_proof`는 별도 `latest_managed_capability_proof` role, 즉 runtime
하나에서 initialize, initialized notification, `tools/list`, required-tool validation,
지정된 safe tool call을 모두 완료한 가장 최신 항목을 사용합니다. 서로 다른 session의
milestone을 조합하지 않습니다. 최신 attempt가 terminal failure이면 현재 session health는
실패합니다. 더 오래된 complete proof는 별도 role로 보이지만 현재 실패를 숨길 수 없습니다.
실제 MCP peer `clientInfo`와 별도로 probe한 PATH executable/version도 서로 다른 보고서
사실로 유지합니다.

`failed`와 `blocked` check만 `cause_finding_ids`를 가질 수 있습니다. `blocked` check는
실패했거나 blocked인 prerequisite의 독립 root finding ID를 정규 정렬하고 합친 집합을
담습니다. `root_cause_ids`는 전체 check graph에서 얻은 정렬·중복 제거 합집합입니다.
Blocked check의 원인이 실패한 prerequisite와 일치하지 않거나 dependency cycle 또는
비정규 dependency edge가 있으면 보고서는 유효하지 않습니다.

현재 Codex 연결 보고서에는 다음 운영 check가 들어갑니다.

| Check ID | 현재 역할 |
|---|---|
| `managed_config` | 선택한 대상에 정규 managed entry가 있습니다. |
| `host_reload` | 현재 revision의 managed-host attempt가 host가 현재 configuration을 읽었음을 보여 줍니다. |
| `hook_source_activation` | Typed hook activation 상태와 현재 definition hash를 담고 주변 phase 관찰은 detail로 유지합니다. |
| `managed_session_health` | Terminal protocol failure를 포함해 `latest_managed_attempt`를 보고합니다. |
| `managed_capability_proof` | 별도 `latest_managed_capability_proof`와 같은 session의 capability milestone을 보고합니다. |
| `ambient_hook_coverage` | 현재 definition 실행, managed-file integrity, 일반 host 활동의 구성된 prompt/pre/post phase coverage만 보고합니다. |
| `correlated_guard_verification` | 최신 현재 verification attempt와 최신 완료 현재 proof를 서로 다른 근거로 보고합니다. |
| `project_trust` | Hook 상태를 바꾸지 않고 별도로 확인할 수 있는 project/configuration trust 적용 여부를 보고합니다. |
| `host_executable` | 별도로 probe한 PATH executable과 version을 diagnostic으로 보고합니다. |
| `mcp_server` | CLI가 소유한 MCP preflight/self-test 사실을 diagnostic으로만 보고합니다. |

CLI MCP preflight는 읽기 전용이며 runtime session을 만들지 않습니다. 수동 stdio
self-test는 일회용 명령별 Runtime Home에만 `session_source=manual_cli`를 만듭니다.
따라서 preflight와 이 일회용 증거는 `managed_session_health`,
`managed_capability_proof`, `correlated_guard_verification`을 충족할 수 없습니다. MCP resource와
resource template도 tool 노출을 증명하지 않습니다. Guard는 집중 activation check로
`ambient_hook_coverage`와 `correlated_guard_verification`을 사용합니다. 엄격한 Guard manifest는
현재 policy hash, integration revision, typed runtime command, 전체 Volicord 관리 artifact
기대값, 필수 hook phase를 담당합니다. 또한 정확한 `host_contract_profile`과 결정적인
`host_contract_digest`를 지정하며, 현재 값은 `codex-command-hooks`와 검토된 계약 identity를
선택합니다. Policy command와 runtime command는 typed invocation 하나에서 나온 서로 다른
projection입니다. Audit은 profile이나 digest가 이 정확한 선택과 다르면 manifest를 거부하고,
각 관리 artifact를 정규 현재 기대값과 비교하며, 모든 필수 phase의 호환되는 현재 event를
요구합니다.

기록한 verification 도구 이름이 다르면 `managed_capability_proof`는 절대 통과하지
않습니다. 한도 있는 fact에는 정확한 기대 이름과 관찰 이름을 노출합니다. 이전 revision,
non-managed runtime row, milestone 부재, 서로 다른 session에 나뉜 milestone은 대신할 수
없습니다.

제한 안의 모든 Codex version은 이 동작 check를 거칩니다. PATH executable version은
managed runtime session을 선택하거나 제외하는 기준이 아닙니다.

`dry_run`은 작업 mode이며 연결 상태나 check 상태가 아닙니다. 구성 일치, 실행 파일
가용성, protocol/host version, capability 관찰, 관찰 timestamp는 check 사실에 두며
별도 공개 또는 영속 상태 enum을 만들지 않습니다.

각 step ID의 actor, channel, diagnostic 분류, agent sequence, 완료 의도 check
metadata는 고정되어 있습니다.

| ID | Initiator / executor / channel | 완료 의도 check |
|---|---|---|
| `reload_codex` | `user` / `host` / `codex_ui` | `host_reload` |
| `review_project_hooks` | `user` / `user` / `codex_ui` | `hook_source_activation` |
| `request_integration_verification` | `user` / `agent` / `codex_chat` | `managed_session_health`, `managed_capability_proof`, `ambient_hook_coverage`, `correlated_guard_verification` |
| `read_connection_status` | `user` / `volicord` / `cli` | 없음 |
| `run_optional_active_diagnostics` | `user` / `volicord` / `cli` | active diagnostic check, 선택 사항만 해당 |
| `repair_hook_contract` | `user` / `user` / `codex_ui` | `hook_source_activation`, `ambient_hook_coverage` |
| `repair_managed_configuration` | `user` / `volicord` / `cli` | `managed_config` |

`request_integration_verification`에는 `volicord.list_projects`,
`volicord.begin_integration_verification`, workflow가 지시한
`volicord.guard_probe`, workflow가 지시한
`volicord.get_integration_verification` 순서가 중첩됩니다. 사용자가 Codex chat 요청을
시작하고 agent가 tool을 실행합니다. Guard probe는 최상위 형제 step이 아닙니다.
`awaiting_probe`는 probe를 한 번 허용하고 `awaiting_observation`은 status tool을 한 번
허용하며, `repair_required`와 `complete`는 tool 실행 및 같은 turn의 재시작을
중단합니다.

엄격한 생성과 decoding은 ID와 맞지 않는 step metadata를 거부합니다.
`root_finding_ids`는 step을 현재 독립 원인과 연결합니다. 필수 step은 root finding과
현재 check 상태에서 만듭니다. Blocked downstream check는 관찰 step을 만들지 않고
blocker의 repair step을 먼저 제공합니다. 여러 symptom에서 나온 같은 step은 하나로
합칩니다. Repair가 필요한 상관 attempt는 무조건 Guard probe를 다시 요청하지 않고
상황에 맞는 `repair_hook_contract` 또는 `repair_managed_configuration`을 projection합니다.
`run_optional_active_diagnostics`는 필수 activation과 분리합니다. Registry 저장소는
독립된 검증 상태나 activation 배열을 저장하지 않습니다. 완료된 영속 보고서가 없는
연결은 `verification_not_run` pending check 하나, 필수
`request_integration_verification` step 하나, 선택적인
`run_optional_active_diagnostics` step 하나를 포함하는 합성
`status=action_required` 보고서로 projection합니다. 읽었다는 이유로 이를 저장하지
않습니다.

관리 CLI는 init, add, status, verify, mode, remove를 현재 schema 2
`DiagnosticReport`로 projection합니다. 정규 check, 현재 평가 overlay와 Store API에서
해석한 한도 있는 finding과 cause edge, 계산한 root ID, 같은 root-scoped
`IntegrationActivationPlan`, Connection context,
operation별 result detail, report limit을 담습니다. Concise, verbose, JSON 출력은 같은
report의 projection이며 같은 root를 식별합니다. 렌더러는 summary 산문에서 cause나
remediation category를 만들지 않고 `DiagnosticFinding`이 가린 fact를 다시 노출하지
않습니다.

현재 Connection 보고서는 `failed` 및 `blocked` check가 참조한 정확한 ID에서 finding을
선택하고 그 finding의 한도가 있는 cause chain만 해석합니다. 각 reference의 provenance를
명시적으로 유지하면서 현재 평가의 inline finding을 먼저 사용하고, 그다음 명시적인 영속
Store seed를 사용합니다. 결합한 graph에는 inline current finding, 영속된 불변 occurrence,
영속된 active current-state finding을 함께 담을 수 있습니다. 독립적인 현재 finding은 작업이
의도적으로 선택한 경우에만 나타나며, 같은 integration revision에 저장된 모든 finding을
현재 상태로 취급하지 않습니다. CLI 소유 현재 상태 운영 finding은 typed subject로 정확한
managed-config target, Product Repository trust, Guard 관리 artifact, Guard phase, Guard
Installation, Guard event, integration revision, verification tool을 식별하고, 각 폐쇄형
diagnostic 값을 불변 definition 하나에 연결합니다. 각 subject는 scope, typed versioned 정규
identity encoding과 opaque subject identity, 별도의 안전한 표시 projection을 담당합니다. Path를
담는 subject는 filesystem alias를 정규화한 뒤 opaque identity를 파생하며 정규 path byte를
저장하지 않습니다. 각 `CurrentDiagnosticKey`에는 완전한 Connection scope, 전체 code, domain,
stage, source, opaque subject identity가 들어갑니다. 안정적인 ID는 이 완전한 key의 고정된 전체
digest이므로 artifact나 phase가 둘이면 diagnostic code가 같아도 finding 둘을 유지하고, 주체
하나를 다시 관찰하면 안전한 표시 projection을 포함한 그 snapshot만 갱신합니다.

활성 검증은 CLI 담당자별 완전한 관찰 집합을 reconcile합니다. 계속 관찰된 condition은
활성화하거나 갱신하고, 복구 성공, 호환 revision, 새로운 관찰 뒤 집합에서 빠진 이전 active
condition은 명시적으로 해소합니다. 해소된 current finding은 정확한 ID로 계속 조회할 수
있지만 보고 가능한 current finding이 아니며 failed 또는 blocked check의 현재 projection에
다시 나타나지 않습니다.

JSON projection은 `generated_at`을 report 시각으로 담고, 값이 있으면 Connection
context에 정확한 현재 integration revision을 담습니다. 영속 verification의 `checked_at`은
해당 verification 관찰 시각으로 남으며 경쟁하는 두 번째 최상위 시각으로 반복하지
않습니다. Status는 저장된 active-probe fact와 현재 관찰에서 메모리 안의 완전한 현재 평가를
만들지만, 이 읽기는 projection을 영속하거나 timestamp를 바꾸지 않습니다. Inline 원인을
보고 가능하게 만드는 데 verification 실행이 필요하지 않습니다. 영속 reference라고
명시됐지만 Store row가 없을 때만 typed `diagnostics.finding_record_missing` 관찰로
표시합니다. 렌더링 과정에서 누락된 domain fact를 꾸며 내지 않습니다. Inline finding은
실제 원인으로 반환하며 missing-record 치환이나
`action.diagnostics.rebuild_current_observations` 안내를 만들지 않습니다.

선택적인 활성 검증은 plan이나 probe를 시작하기 전에 정확한 typed Connection integration
revision을 확보합니다. Store는 같은 revision이 여전히 현재 상태일 때만 비교와 보고서 교체를 immediate
Registry transaction 하나에서 수행합니다. 이 쓰기는 `verification_report_json`과 일반 row
갱신 timestamp만 바꿉니다. Revision 충돌이 나면 기존 보고서와 모든 소유자 field를 그대로
두고 검증을 다시 실행하도록 요구합니다. 검증은 관리 configuration을 관찰할 뿐 새로 계획한
managed fingerprint를 적용하거나 채택하거나 기록하지 않습니다.

영속 action에 필수 typed 구성원이 없거나, 알 수 없는 구성원이 있거나, reference 순서가
비정규이거나, metadata가 ID와 일치하지 않으면 현재 보고서 형태가 잘못된 것이며 엄격한
읽기에서 거부합니다. 활성 검증은 같은 revision 보호 교체 경계를 통해 이런 보고서를
교체할 수 있지만 관련 없는 Connection 소유자 상태는 바꾸지 않습니다. JSON을 제자리에서
다시 쓰거나 다른 decoder를 사용하는 경로는 없습니다.

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
남습니다. 권한은 현재 Connection, revision, 권위 있는 runtime/project session binding을
사용합니다.

실제 MCP peer는 해당 runtime session에서 관찰한 제한된 `clientInfo.name`과
`clientInfo.version`입니다. PATH probe는 별도로 관찰한 Codex executable path와
version입니다. 보고서와 finding은 둘을 서로 대신 사용하지 않습니다. 두 version이 모두
있고 서로 다르면 `host.codex.peer_version_differs_from_path_probe`가 두 사실 객체를 담은
warning evidence를 기록하며, 이 불일치만으로 Connection을 치명적 실패로 만들지 않습니다.
명시적으로 선택한 `codex-mcp-turn-metadata` profile이 MCP session/thread/turn
metadata를 소유합니다. 잘못된 MCP metadata, 중첩 및 최상위 thread 좌표 불일치, 등록
session 불일치는 각각 `host.codex.metadata_malformed`,
`host.codex.session_thread_turn_inconsistent`,
`host.codex.registered_session_correlation_mismatch`를 사용합니다.

각 MCP process 시작은 host thread metadata가 생기기 전에 불투명 Registry runtime
session ID를 만듭니다. `session_source`는 정확히 `managed_host`, `manual_cli`,
`cli_preflight`, `integration_probe` 중 하나입니다. Launch lease를 원자적으로 소비한
경우에만 `managed_host`를 만들 수 있고, `managed_host`만 Agent Connection 호출을
승인할 수 있습니다. 공개 `volicord mcp serve`는 항상 `manual_cli`를 기록하고
preflight는 session을 만들지 않으며 integration probe는 managed-host 활동으로 계산하지
않습니다. Runtime
session은 소유 Connection과 Connection 통합 revision을 보관합니다.

유효한 initialize 요청 뒤 해당 runtime은 session 범위의 typed MCP selection 하나를
소유합니다. `McpSessionMilestones`는 runtime, source, Connection, integration revision,
process start, 실제 peer `clientInfo`, requested/selected/negotiated protocol revision,
initialize와 initialized-notification 완료, `tools/list` 시각과 결정적으로 정렬한 정확한
반환 도구 identity, required-tool validation 시각, 정규 verification-tool identity/시각,
terminal finding, 마지막 관찰 시각을 보관합니다. Negotiation에는 완료된 initialization이,
required-tool 성공에는 실제 list 관찰이, verification-tool 성공에는 같은 session의
required-tool validation이 필요합니다. Managed capability proof는
`session_source=managed_host`인 경우에만 만들 수 있으며, 그 밖의 조합은 거부합니다. 다시
연결하면 새 runtime과 새 milestone을 만들고 process 사이에서 공유하거나 상속하지 않습니다.

Connection report context는 finding correlation뿐 아니라 check evidence에서도 session ID를
수집합니다. 각 항목은 폐쇄형 role `latest_managed_attempt`,
`latest_managed_capability_proof`, `guard_verification_attempt`,
`guard_verification_proof`를 보존합니다. Session 하나가 여러 role을 가지면 한 번만
표시하고 정규 순서의 role 목록을 둡니다. Context는 관련 Guard verification ID도
보존합니다. Human과 JSON projection은 같은 role 배정을 표시합니다.

프로젝트 통합 revision은 Connection revision에 현재 프로젝트 workflow-policy
fingerprint와 현재 Guard installation identity/policy hash 또는 Guard ownership의 명시적
부재를 더합니다. `host_sessions`는 revision 범위 로컬 session ID, Connection, 정확한
native session, 관찰 시각을 보관합니다. `host_turns`는 두 계약 source가 함께 쓰는 turn을
보관하고, `host_tool_invocations`는 hook tool-use ID와 정규 tool name을 보관합니다. Store는
프로젝트를 결정하고 현재 Guard ownership을 검증한 뒤에만 Connection internal ID, 정확한
프로젝트 통합 revision, 정확한 native session에 domain-separated digest를 적용해 로컬
session ID를 도출합니다. 호출자는 완성된 로컬 ID를 제공할 수 없고 저장된 프로젝트
revision은 변경할 수 없습니다.

`CodexCommandHooks` marker가 `codex-command-hooks` parser를 선택합니다. 이 parser는
`UserPromptSubmit`에 `CodexHookPromptCorrelation`을, `PreToolUse`와 `PostToolUse`에
`CodexHookToolCorrelation`을 만듭니다. Prompt 상관관계에는 session과 turn만 필요합니다.
Tool 상관관계에는 tool-use ID와 정규 tool name도 필요합니다. 어떤 hook phase에도 thread
좌표가 없습니다. 별도 `CodexMcpTurnMetadata` marker는
`codex-mcp-turn-metadata` parser를 선택하며, 이 parser는 session, thread, turn이 모두
필요한 `CodexMcpCorrelation`을 만듭니다. `HostNativeCorrelation`은 범용으로 교환 가능한
좌표를 제공하지 않고 이 source variant를 보존합니다. Store phase check와 SQL
discriminator는 source가 교차되거나 불완전한 조합을 거부합니다.

Connection에 등록된 `server_name`은 `McpServerKey`로 decode하며 각 도구의 완전한
`McpRawToolName`과 구분해 유지합니다. `CodexMcpCallableNames` 계약은 이 좌표를
`codex-mcp-callable-names`의 `HostCallableIdentity` 하나로 투영합니다. Raw tool
name에서 마침표로 구분한 일부를 server key로 취급하지 않습니다. 생성한 hook, MCP
preflight diagnostic, Guard 검증은 충돌 검사를 마친 같은 `McpToolCatalog`를 사용하며,
역방향 해석은 구두점을 분석하지 않고 catalog에서 정확히 조회합니다. Adapter는 이
semantic 계약을 직접 선택하며 관찰한 Codex package version에서 host 동작을 도출하지
않습니다.

같은 semantic command-hook 계약이 typed tool-routing strategy도 소유합니다. Tool
phase에서는 검토된 native host-tool 집합과 server-qualified MCP callable을
routing합니다. Callable 투영이 등록된 namespace를 보존하면 그 namespace를 사용하고,
그렇지 않으면 `McpToolCatalog`에서 파생한 exact token을 사용합니다. 생성 matcher는
acquisition 경계일 뿐입니다. Event가 전달된 뒤 wrapper는
관찰한 callable을 `McpToolCatalog`로 해석하며, 현재 verification ID, session, turn,
tool-use chain이 정확히 일치하는 `AgentToolId::GuardProbe`만 검증을 완료할 수
있습니다. Routing된 다른 tool, 알 수 없는 same-server callable, 호환되지 않는 payload,
각 coordinate mismatch는 서로 다른 acquisition stage로 남습니다.

영속 stage는 `ProbeAcknowledged`, `HookEventNotObserved`,
`HookPayloadIncompatible`, `CallableIdentityUnknown`,
`CallableIdentityMismatch`, `VerificationIdMismatch`, `SessionMismatch`,
`TurnMismatch`, `ToolUseMismatch`, `PreToolMatched`, `PostToolMatched`입니다.
각 stage는 한도가 있는 callable 및 범주형 상관관계 fact만 보관합니다. 특히
`HookEventNotObserved`는 Volicord가 event를 받지 못했다는 뜻이며 matcher 오류나 host
emission 오류라고 진단하지 않습니다.

Guard 상관관계와 Guard policy는 별도 단계입니다. 호환되는 hook 상관관계는 policy에 도달해
`Continue`, `ContinueWithContext`, `ContinueWithWarning`, `Deny` 중 하나를 낼 수 있습니다.
반면 호환되지 않는 hook contract는 가능한 경우 관찰 실패를 기록하고 policy 판단을 만들지
않으며 phase를 충족하지 않습니다. Codex `record` profile에서 adapter는 제한된 context와
exit `0`으로 해당 host 동작을 계속하며, 관찰 실패를 denial로 바꾸지 않습니다. Event
영속화를 사용할 수 없는 경우에도 합성 denial을 만들지 않는 같은 규칙을 따릅니다. 호환되는
명시적 `PreToolUse` policy `Deny`만 Codex permission denial이 됩니다. `PostToolUse` output은
이미 끝난 동작에 대해 warning이나 reconciliation 필요성을 알릴 수 있지만 동작을 막았다고
주장할 수 없습니다.

Host-neutral 경계는 관찰 결과, 선택적 policy 판단, 제한된 diagnostic, 안전한 feedback을
담는 `GuardHookOutcome`입니다. Core나 Store가 아니라 Codex adapter가 stdout hook JSON,
stderr, process exit, context, warning, denial projection을 담당합니다.

Guard 관찰은 정규화한 host, turn, tool row를 만들 수 있지만 MCP 전용
`managed_mcp_sessions` row는 만들지 않습니다. 해당 host session의 첫 실제 managed MCP
도구 호출은 먼저 현재 managed runtime을 변경 없이 검증하고, 정확한 MCP anchor를 만들거나
검증한 뒤, Registry에서 데이터베이스 간 binding을 예약하면서 현재 소유자 입력을 다시
검증합니다. 마지막 프로젝트 transaction에서만 runtime을 붙입니다. MCP anchor에서 확인한
Connection, 프로젝트, Guard Installation, revision, native session, thread, 기존 runtime
충돌은 새 Registry binding을 남기지 않습니다. 이후 Registry 예약이 실패하면 anchor가
unbound로 남을 수 있지만 권한은 아닙니다. 마지막 attach 전 중단으로 남은 Registry 예약도
권한이 아니며, 소유자 상태가 바뀌지 않은 정확한 replay가 그 예약을 재사용해 attach를
완료합니다. 붙인 MCP session은 다른 runtime session, Connection, 프로젝트, host session,
host thread에 다시 결속할 수 없습니다.

Runtime row는 launch lease나 liveness 주장이 아니라 process의 이력 관찰입니다. Launch
lease는 bootstrap 전환 하나만 승인하며 runtime row를 liveness 기록으로 바꾸지 않습니다.
Crash한 process는
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

### 관리되는 채팅 내 통합 검증

`GuardIntegrationVerificationRun`은 first-party 채팅 내 작업 흐름의 영속 attempt입니다.
불변 semantic 좌표는 Connection, project, managed MCP runtime session, native host
session과 turn, integration revision, Guard Installation, host-contract profile,
hook-definition digest, policy digest로 구성됩니다. Terminal 상태가 된 뒤에도 이 좌표에는
verification ID가 정확히 하나만 존재합니다. Row는 semantic observation policy, bounded
status-read 횟수, cleanup 경계, first-write probe acknowledgement, 상관관계가 확인된
prompt/pre/post event ID, 완료 정보, terminal finding도 기록합니다. Cleanup 시각은
attempt identity나 workflow 상태를 바꾸지 않습니다.

실제 현재 `managed_host` 호출만 run을 시작하거나 probe하거나 읽을 수 있습니다. 수동 stdio,
CLI preflight, integration probe는 성공을 만들 수 없습니다.
`IntegrationVerificationWorkflowState`는 begin, probe, status가 공유하는 단일 공개
투영입니다. Tagged alternative는 정규 Guard probe tool을 담은 `awaiting_probe`, 정규
status tool과 권위 있는 acknowledgement 시각 및 남은 bounded read 수를 담은
`awaiting_observation`, 완료 시각을 담은 `complete`, typed repair reason과 별도의 retry
policy 및 bounded finding을 담은 `repair_required`입니다. Tool 필드는 임의 문자열이
아니라 typed 정규 `AgentToolId` 투영입니다. `complete`와 `repair_required`는 불변 terminal
상태입니다.

Host contract는 `HookObservationPolicy`를 semantic하게
`Synchronous { allowed_status_reads }` 또는
`Deferred { deadline, allowed_status_reads }`로 선택합니다. 검토된 현재 Codex
command-hook profile은 status read 한 번의 synchronous observation을 선택하며 package
version이나 numeric profile generation으로 이 동작을 고르지 않습니다. 결정적 순서는
begin 또는 resume, 요청된 경우 GuardProbe 한 번 호출, policy에 따른 status 호출, terminal
상태에서 중단입니다. Sleep loop와 자동 same-turn retry는 없습니다.

Probe acknowledgement는 정확한 verification ID, Connection, managed runtime session,
native host session, native host turn 좌표에서 first-write-wins입니다. 적격인 첫 active
호출이 timestamp를 기록합니다. Replay는 `awaiting_observation`과 원래 timestamp를
유지합니다. 완료 또는 repair 뒤 정확한 replay는 같은 terminal 상태를 반환합니다. 어떤
replay도 완료 정보나 일치한 event를 바꾸지 않습니다. 다른 caller 좌표는
acknowledgement를 노출하지 않고 거부하며, acknowledgement가 없는 terminal attempt에는
뒤늦게 값을 만들 수 없습니다.

통과하려면 prompt, pre-tool,
post-tool 기록이 같은 run session과 turn에 속해야 하고, pre/post는 tool-use ID, 생성된 정확한
probe 이름, verification-ID 입력을 공유해야 합니다. 현재 Guard Installation, policy hash,
integration revision, hook-contract digest, managed runtime도 계속 일치해야 합니다. Prompt는
pre-tool보다 늦을 수 없고 pre-tool은 post-tool보다 앞서야 합니다. 이력 event 검색이나 서로
다른 attempt의 phase 조합은 허용하지 않습니다. Synchronous read를 모두 사용하면 누락
event, 비호환 payload, callable, verification ID, session, turn, tool-use 불일치를 구분한
가장 정확한 typed repair reason을 만듭니다. Owner drift도 integration revision, hook
definition, policy 변경을 구분합니다. Retry eligibility는
`no_automatic_retry`, `new_turn_required`, `host_reload_required`,
`hook_review_required`, `repair_required` 중 하나로만 표현하며 새 attempt에는 여전히
실제로 달라진 적격 좌표가 필요합니다.

`ambient_hook_coverage`와 `correlated_guard_verification`은 하나의 boolean을 공유하지
않습니다. `AmbientGuardCoverageEvidence`는 현재 hook definition 실행과 구성된 모든
prompt/pre/post phase의 일반 coverage만 증명합니다.
`CorrelatedGuardAttemptEvidence`는 verification, runtime session, host session과 turn,
event, 기대·관찰 callable, acquisition stage, repair, retry, timestamp 사실을 포함한 최신
현재 attempt를 보존합니다. `CorrelatedGuardProof`는 최신 완료 현재 proof를 보존합니다.

Attempt가 없거나 deferred host policy에서 실제로 진행 중인 attempt는 `pending`입니다.
`complete`는 `passed`이고 `repair_required`는 typed recoverability와 action을 가진
`failed`로 항상 유지합니다. 복구 가능한 failure가 집계를 `action_required`로 만들 수는
있지만 check나 attempt 상태를 pending으로 바꾸지는 않습니다. 더 오래된 완료 proof는 더
최신 attempt가 실패했을 때 이력 capability 근거로만 남고 현재 check를 통과시키지
못합니다. Diagnostic code는 summary 문구가 아니라 typed repair reason과 acquisition
stage에서 직접 선택합니다. Run은 협력적 로컬 증거일
뿐입니다. Codex 프로젝트 trust를 자동화하거나 우회하지 않고, MCP trust configuration을
변경하지 않으며, 사용자 또는 host identity나 Core 작업 흐름 권한을 만들지 않습니다.

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
11. Runtime session의 `session_source=managed_host`이며 `manual_cli`, `cli_preflight`,
    `integration_probe`가 아닙니다.

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
- one-time launch lease를 발급하고 관리 stdio에 들어가기 전에 그 정확한 현재 entry를
  다시 검증
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
Command name, executable path, version string, Runtime Home 구성, 로컬 session metadata는
diagnostic 또는 routing 사실이며 actor나 human identity를 성립시키지 않습니다. Launch
lease는 evidence integrity를 위한 전환 좌표이지 OS actor credential이 아닙니다.

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
- 수동, integration-probe, 오래됨, 닫힘, revision 불일치 session
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
