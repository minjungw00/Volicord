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

`volicord doctor --json`은 공유 `DiagnosticFinding` 형태의 `findings`도 반환합니다.
Registry 조사에서는 임의의 SQLite 메시지를 projection하지 않습니다. SQLite 결과 code,
조사 상태, 한도가 있는 범주형 사실로 finding을 선택하며 산문은 표시 맥락으로만 남습니다.

관리 Codex 구성 finding은 다음 닫힌 code를 사용합니다.

| Code | Typed 관찰 |
|---|---|
| `managed_config.toml.parse_failed` | 구성 문서를 지원 TOML 형태로 parsing할 수 없습니다. |
| `managed_config.entry.missing` | 필수 MCP entry 또는 소유 table이 없습니다. |
| `managed_config.entry.disabled` | 필수 MCP entry에 `enabled = false`가 설정되어 있습니다. |
| `managed_config.command.drift` | 구조화된 command가 다릅니다. |
| `managed_config.arguments.drift` | 구조화된 argument vector가 다릅니다. |
| `managed_config.static_environment.drift` | 정적 환경 이름 또는 값이 다릅니다. 값 자체는 finding에 복사하지 않습니다. |
| `managed_config.forwarded_environment.drift` | 전달하는 환경 이름 집합이 다릅니다. |
| `managed_config.fingerprint.mismatch` | Scope, 소유권 또는 전체 관리 identity가 다릅니다. |
| `managed_config.approval_overlay.malformed` | Typed 도구 승인 overlay가 잘못되었습니다. |
| `managed_config.observation.unavailable` | 구성 target을 조사할 수 없습니다. |

기존 underscore-only `ConnectionCheck.code`는 한도가 있는 check code로 유지합니다. 위
namespace code는 `DiagnosticFinding.code`이며 Connection check가 이를 운반할 때
`details.diagnostic_code`에도 projection합니다. 구성 값, command argument, 전체 환경 값은
diagnostic 사실로 저장하지 않습니다.

다음 단계가 알려졌으면 typed action을 붙입니다. 현재 action code에는
`action.runtime_home.correct_path`, `action.runtime_home.initialize_registry`,
`action.store.free_locked_database`,
`action.installation.reinstall_current_build`,
`action.managed_config.repair`, `action.guard.repair`,
`action.guard.trigger_phase`,
`action.host.reload_after_configuration_change`가 있습니다. Reload action은 구성 변경 뒤
integration revision이 오래된 경우에만 사용합니다. 결정적인 구성 drift, schema mismatch,
permission 실패에는 일반적인 restart action을 붙이지 않습니다.

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
`init` 설정 변경에 속합니다. `init`은 setup 변경 전에 기존 home을 읽기 전용으로 열어
정확한 manifest와 schema를 검증하고 분류합니다. `Incompatible` 또는 `Corrupt` 상태는
보존하며, 기존 home을 유지하고 명시적 `--home`으로 새 위치를 선택하거나 담당자가 정의한
importer가 있는 경우에만 그것을 사용하도록 안내합니다.

최종 경로가 없으면 `init`은 같은 상위 directory의 고유 staging directory에 Registry,
singleton, installation profile을 만들고 singleton에 새 불투명 publication ID를
기록합니다. 기존 대상을 교체하지 않는 원자적 rename에 성공하면 상위 directory 동기화와
read-back 확인 전에 invocation별 소유권을 반환합니다. Setup lease 보유 중
`AlreadyExists`이면 staging을 정리하고 target을 읽기 전용으로 검사한 뒤 오래된 plan을
외부 concurrent modification으로 중단합니다. Connection 명령은 선택한 홈에
현재 installation profile이 있어야 하며 홈이 없거나 사용할 수 없으면 그 정확한 경로를
담아 실패합니다. 선택 뒤에도
`connection list`와 `connection status`는 읽기 전용입니다. 비어 있거나 잘못됐거나 충돌하는
값은 저장소 접근 전에 실패합니다. Product Repository를 Runtime Home으로 사용하지 않습니다.

모든 명령에 같은 경로를 넘기면 `VOLICORD_HOME`을 내보내지 않고도 사용자 지정 홈의
lifecycle을 실행할 수 있습니다.

```sh
volicord init --host codex --repo "<repo>" --profile record --home "/srv/volicord/team-a"
volicord connection status codex --repo "<repo>" --home "/srv/volicord/team-a"
```

모든 변경 관리 명령은 정확히 선택한 Runtime Home을 해석하고 변경에 의존하는 읽기나
planning 전에 `SharedWriter`를 획득합니다. Store 효과, diagnostic 영속화, 결과 구성,
정리가 끝날 때까지 승인을 유지합니다. 여기에는 project use/rename/forget, policy
apply, inbox resolution, change reconciliation, evidence fulfillment, Connection
mode/remove/verification write, managed-launch lease 연산, diagnostic 영속화가 포함됩니다.
순수 status, list, lookup, validation, export 읽기는 writer lease를 획득하지 않습니다.

Setup이 `ExclusiveSetup`을 보유하면 변경 명령은 typed
`runtime_home.mutation.setup_in_progress` 비성공 결과를 반환하고 아무것도 변경하지
않으며 setup 완료 뒤 재시도하도록 안내합니다. Diagnostic에는 정규 Runtime Home,
명령 변경 domain, 요청 mode, 한도가 있는 wait policy, 경과 시간, 재시도 가능 여부가
포함되며 coordination 파일을 복구 대상으로 노출하지 않습니다.

<a id="volicord-agent-install"></a>
<a id="agent-host-setup-and-init"></a>
## Codex 설정

```sh
volicord init --host codex --repo "<repo>" --profile record
volicord init --shared --host codex --repo "<repo>" --profile record
```

첫 명령은 개인 연결, `--shared`는 프로젝트 소유 공유 연결을 선택합니다. `init`은
정확한 관리 binding을 계획, 준비, commit하고 남은 Codex 신뢰, 다시 불러오기, 검증 동작을
보고합니다. `--dry-run`은 파일시스템과 저장소를 변경하지 않습니다.

`init`과 `connection add`는 bootstrap 검사나 plan 구성 전에 선택한 Runtime Home을
canonicalize하고 배타적 OS 기반 setup lease를 획득합니다. 즉시 경합하면 setup planning이나
mutation 전에 typed busy 실패를 반환합니다. Lease 아래의 plan 구성은 읽기 전용이며
정확한 Runtime Home, Store, Codex 구성, repository 관리 파일 mutation을 계산합니다.
Dry run도 일관된 plan 보고까지 같은 lease를 유지합니다. Prepare 단계는 모든 입력
snapshot을 검증하고 각 파일을 target 옆에 staging하며 Store 복구 entry를 준비합니다.
Commit 단계는 소유한 Runtime Home publication guard를 유지하고 Store mutation을 적용한 뒤
repository 파일을 결정적인 경로 순서로 원자 교체하고 Codex 구성을 마지막에 원자 교체한
다음 현재 integration revision을 기록합니다. Lease는 성공 보고와 staging 정리 또는 전체
rollback과 보존 결정을 마친 뒤에만 해제합니다. Rollback은 guard가 정확한 소유권을
다시 검증한 경우에만 새 Runtime Home을 제거할 수 있으며, 소유권 상실이나 managed-host
소비가 있으면 제거를 중단합니다. 재귀 제거 효과와 상위 entry 내구성은 별도 결과로
유지합니다. 제거가 확인되면 상위 directory 동기화가 실패해도 terminal이며, 불완전한
제거도 맹목적으로 재시도하지 않는 terminal 상태입니다. Activation step은 setup이
`committed`일 때만 만듭니다. 이는
여러 파일시스템 전체의 전역 원자성을 뜻하지 않으며, 완전한 사전 준비, 파일별 원자 교체,
한도가 있는 rollback을 뜻합니다.

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
결과, 선택한 저장소와 유효한 mode, `ready`/`blocked`/`waiting`/`failed` check 개수, 대기 관찰보다
앞에 표시하는 현재 문제, 현재 다음 동작을 간결한 사람용 산문으로 보여 줍니다.
실패 보고서의 각 문제는 독립 root finding이며 namespaced code, 한도가 있는 typed summary,
가장 유용한 안전한 actual-versus-expected fact, 영향을 받은 blocked check, finding 또는
runtime-session 식별자를 포함합니다. `Required next steps` 구역 하나가
`IntegrationActivationPlan`의 현재 위상 suffix를 projection하며
`Optional active diagnostics`는 분리합니다. Root finding에 typed remediation이 있으면
일반적인 inspection step을 만들지 않습니다.
사람용 라벨은 표시 문구이며 보고서나 check 상태를 추가하지 않습니다.

정규 check 상태는 `passed`, `pending`, `failed`, `blocked`, `not_applicable`입니다. 각각
성공적으로 완료됨, 실패한 prerequisite가 없는 상태에서 필요한 외부 관찰을 기다림, check
자체에서 실패함, prerequisite finding 실패로 실행할 수 없음, 선택한 Connection 또는
profile에 적용되지 않음을 뜻합니다. Blocked 또는 복구 불가능한 failed check가 있으면
집계 상태는 `failed`입니다. 복구 가능한 failed `correlated_guard_verification`은 failed로
그대로 남으면서 집계에 `action_required`로 기여할 수 있고 pending check도
`action_required`에 기여합니다.

간결한 렌더러는 `activation_state`와 `hook_activation_state`를 항상 이름 붙여
표시합니다. Pending 집중 check는 managed session, capability, Guard 활동으로 묶을 수
있으며 passed, failed, blocked, not-applicable, 부재 check는 `Waiting` 아래에 반복하지
않습니다. 주변 Guard phase는 `ambient_hook_coverage`가 별도로 보고하며
`correlated_guard_verification`을 충족하지 않습니다. 렌더러는 정규 check나 activation
step을 변경, 제거, 재정렬, 영속하지 않습니다. Dry run 산문은 typed
`PlannedConnectionChangeKind`별로 계획 변경 수를 묶으며 target path에서 소유권을
추론하지 않습니다.
따라서 terminal correlated attempt는 다음과 같은 사실로 표시합니다.

```text
Hook installation and ambient execution: passed
Correlated Guard verification: failed
Reason: callable_identity_mismatch
```

적용한 `init`이 프로젝트 hook definition을 만들거나 바꾸면
`hook_activation_state=review_required_by_setup`을 보고하고 다음 host 소유 sequence를
정확히 표시합니다.

1. 해당 저장소에서 Codex를 restart 또는 reload합니다.
2. Codex hook UI 또는 `/hooks`로 현재 프로젝트 hook definition을 review합니다.
3. 새 conversation을 시작하고
   `Run the Volicord integration verification.`을 요청합니다.
4. Agent가 끝나면 현재 connection status를 읽습니다.

이 요청이 `request_integration_verification`입니다. 사용자가 `codex_chat`에서
시작하고 agent가 nested `list_projects` → begin → workflow가 지시한 Guard probe →
workflow가 지시한 status 순서를 실행합니다. `volicord connection verify`는 setup 뒤
선택적으로 쓰는 active diagnostics입니다. CLI 소유 configuration 및 transport fact를
능동적으로 확인하지만 managed-host 또는 상관관계가 확인된 Guard 근거를 대신할 수
없습니다.

Blocked check는 blocked 개수에는 포함하지만 대기
관찰이나 downstream 관찰 action을 만들지 않습니다. Root 선택과 action 중복 제거는
finding ID, cause edge, typed action code만 사용하며 렌더러는 summary 산문으로 분류하지
않습니다. 간결한 개수 표시는 값이 0인 항목을 포함해 네 범주를 항상 모두 표시합니다.

간결한 진단 안내는 작업에 따라 달라집니다. `status` 보고서에 pending, failed 또는 blocked check가
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
작업별 머리말로 시작하고, 적용되는 `Connection`, `Summary`, `Checks`, `Findings`,
`Required next steps`, `Optional active diagnostics`, `Result`, `Planned changes`,
`Report limits` 구역을 상황에 맞게 사용합니다. 모든 check와 상태, 모든 root와 한도가 있는 cause-chain finding, 모든 안전한
typed fact, requested/selected/negotiated protocol revision, 실제 MCP peer `clientInfo`, 별도
PATH executable probe, 한도가 있는 process exit와 stderr fact, Runtime Home과 Connection
correlation, runtime-session ID, integration revision, timestamp, dependency와 blocked-by
관계, 권장 action, report limit을 표시합니다. 알고 있는 세부 필드는 구조화해서 표시하며,
MCP check는 typed `ManagedSessionAttemptDetails`,
`ManagedCapabilityProofDetails`, `RequiredToolsEvidence`,
`VerificationToolEvidence`, `HostExecutableProbeDetails`를 직렬화합니다. Guard check는
`AmbientGuardCoverageEvidence`, `CorrelatedGuardAttemptEvidence`,
`CorrelatedGuardProof`를 직렬화하며 verbose 출력은 attempt 좌표, acquisition stage,
기대·관찰 callable identity, retry policy, timestamp를 표시합니다. 렌더러는 passed
check에서 선택 필드가 없다는 이유로 milestone을 부정값으로 바꾸지 않습니다. 집중 렌더러가
기대하는 타입과 맞지 않는 값이나 알 수 없는 확장 필드는 `Additional
details` 아래에 표시합니다. 렌더러는 summary에서 cause를 재구성하지 않으며 가린 fact는
계속 가립니다.

`--json`은 현재 `DiagnosticReport` schema 하나만 쓰며 정확하고 손실 없는 기계 판독
표현입니다. 허용하는 schema version은 정확히 `2`입니다.
Consumer는 사람용 summary를 parsing하지 않고 구조화된 check, finding, cause ID, action
code, fact object를 사용합니다. `--verbose`와 `--json`은 함께 사용할 수 없는 사용법
옵션입니다.
`volicord connection list`는 별도의 간결한 컬렉션 projection을 유지하며
`--verbose`를 받지 않습니다.

Schema 2의 최상위 형태는 다음과 같습니다.

```yaml
DiagnosticReport:
  schema_version: 2
  operation: init | add | status | verify | mode | remove
  status: complete | action_required | failed
  activation_state: configured | host_reload_required |
    hook_review_required_or_unknown | mcp_observation_required |
    guard_verification_required | complete | failed
  hook_activation_state: unknown | review_required_by_setup |
    effective_by_observation | managed_by_policy |
    bypassed_for_invocation | disabled
  generated_at: timestamp
  connection: DiagnosticConnectionContext | null
  checks: ConnectionCheck[]
  findings: DiagnosticFinding[]
  root_cause_ids: DiagnosticFindingId[]
  activation_plan: IntegrationActivationPlan
  operation_details: object
  limits: string[]
```

`connection`에는 Runtime Home, 선택한 Connection 좌표, 선택적인 repository와 config
target, 현재 integration revision, 한도가 있는 `runtime_session_ids`, role을 보존하는
`runtime_sessions`, 관련 `verification_ids`가 들어갑니다. 각 role 항목은 `id` 하나와
`latest_managed_attempt`, `latest_managed_capability_proof`,
`guard_verification_attempt`, `guard_verification_proof`에서 가져온 정규 `roles`를
가집니다. Finding correlation이 없어도 check evidence에서 ID를 수집합니다. Session 하나가
여러 role을 가지면 한 번만 나타내고 정규 순서의 role을 함께 표시하며 verbose human
출력도 같은 배정을 표시합니다. 각
check에는 상태, 정규 dependency, typed detail, 관찰 시각, cause-finding ID가 들어갑니다.
각 finding에는 안전한 typed fact, cause ID, action, correlation, redaction metadata,
truncation metadata가 들어갑니다. 관찰 부재는 필드 부재 또는 명시적인 담당자 fact
`observation_state=absent`, 관찰된 빈 collection은 `[]`, 관찰 실패는 finding이 있는
`failed` check, 차단된 관찰은 root ID가 있는 `blocked` check로 나타냅니다. 이 상태들을
같은 빈 값으로 합치지 않습니다.

대표적인 간결한 protocol mismatch 결과는 다음과 같습니다.

```text
Verification completed: 1 blocked, 2 failed.

Repository: /workspace/product
Mode: workflow
Checks: 0 ready, 1 blocked, 0 waiting, 2 failed

Problems
  mcp.protocol.counter_offer_rejected: the protocol counter-offer was rejected or disconnected
    Actual MCP client: codex 0.42.0
    Requested protocol: 2025-01-15
    Supported protocols: 2024-10-07, 2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25
    Blocked checks: tool_round_trip
    Runtime session: runtime_session_01
    Finding: finding.runtime_session_01.protocol

Required next steps
  action.mcp.use_supported_protocol_revision: Configure the client to request a production-supported protocol revision

Rerun active verification with `volicord connection verify codex --repo /workspace/product --home /home/user/.volicord --verbose` for detailed diagnostics.
```

Verbose 보기는 같은 root ID와 typed 관찰을 표시합니다.

```text
Verification completed: 1 blocked, 2 failed.

Connection
  ID: connection_1
  Host: codex
  Scope: user
  Profile: record
  Mode: workflow
  Repository: /workspace/product
  Config target: /home/user/.codex/config.toml
  Runtime home: /home/user/.volicord
  Integration revision: sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  Runtime sessions: runtime_session_01 (latest_managed_attempt)

Summary
  Status: failed
  Checks: 0 passed, 1 blocked, 0 pending, 2 failed, 0 not applicable

Checks
  [fail] Codex managed session
    MCP client rejected or disconnected before accepting the selected counter-offer
    Code: host_session_protocol_mismatch
    Depends on: process_startup
    Root findings: finding.runtime_session_01.protocol
    Evidence role: latest_managed_attempt
    Runtime session: runtime_session_01
    PATH executable: /opt/codex
    PATH executable version: 0.42.0
    Actual MCP peer: codex
    Actual MCP peer version: 0.42.0
    Requested protocol: 2025-01-15
    Selected protocol: 2025-11-25
    Initialize: failed

  [fail] Codex required tools
    No current-revision managed-host session completed same-session required-tool validation
    Root findings: finding.runtime_session_01.protocol

  [blocked] Read-only tool round trip
    Depends on: required_tools
    Blocked by: required_tools
    Root findings: finding.runtime_session_01.protocol

Findings
  [root] finding.runtime_session_01.protocol
    Code: mcp.protocol.counter_offer_rejected
    Runtime session: runtime_session_01
    Bounded typed facts
      Attempted client name: codex
      Attempted client version: 0.42.0
      Requested revision: 2025-01-15
      Selected revision: 2025-11-25
      Production supported revisions: 2024-10-07, 2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25

Actions
  action.mcp.use_supported_protocol_revision
    Configure the client to request a production-supported protocol revision
    Root findings: finding.runtime_session_01.protocol

Report limits
  Diagnostic cause traversal is bounded to 32 edges and 128 findings.
  Diagnostic fact strings are bounded to 1024 bytes, collections to 32 items, and sensitive fields remain redacted.
  Volicord reports cooperative local configuration and observed behavior; it does not prove OS enforcement, actor identity, correctness, test sufficiency, or human review completion.
```

### Connection 목록 투영

`volicord connection list`는 읽기 전용 컬렉션 목록입니다. 선택한 Connection
하나나 운영 결과 하나를 다루지 않으므로 선택한 Connection용 `DiagnosticReport`
projection을 사용하지 않습니다. JSON 문서의 최상위 구성원은 정확히 다음과 같습니다.

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
step도 내보내지 않습니다. 실제 전환이 성공해도 host configuration이나 Product Repository
file을 다시 쓰지 않고, 기존 managed host를 새 revision에 맞춰 reload해야 하므로 정확히 하나의
`reload_codex` step을 내보냅니다. 이전 runtime session, 프로젝트 Agent Session, Guard
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

선택한 Connection의 설정 및 생명주기 명령은 모두 위에서 정의한 현재 schema 2
`DiagnosticReport` 하나를 직렬화합니다. 여기에는 `volicord init`과 Connection의
`add`, `status`, `verify`, `mode`, `remove` 명령이 포함됩니다. Operation별 fact는
`operation_details` 아래에 둡니다.

```yaml
operation_details:
  dry_run: bool
  result?: SetupResult | ModeTransitionResult | RemovalResult
  planned_changes?: PlannedConnectionChange[] # dry-run에만 사용

SetupResult:
  kind: setup
  disposition: planned | committed | rolled_back | preserved |
    partially_rolled_back
  setup_lease: acquired
  runtime_home_publication: not_published | existing_ready |
    published_by_this_invocation |
    owned_publication_rolled_back | owned_publication_removal_incomplete |
    owned_publication_preserved |
    ownership_lost_during_rollback
  runtime_home_rollback?:
    outcome: removed
    durability: parent_synchronized | parent_synchronization_failed |
      not_applicable
    failure_phase?: target_inspection | recursive_removal |
      post_removal_inspection | parent_directory_synchronization
  # 또는
  runtime_home_rollback?:
    outcome: removal_incomplete
    effect: not_removed | partially_removed_or_unknown
    phase: target_inspection | recursive_removal |
      post_removal_inspection | parent_directory_synchronization
    final_path: present | absent | unknown
  # 또는
  runtime_home_rollback?:
    outcome: preserved | ownership_lost
    reason: string

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
  kind: runtime_home_initialization | project_registration |
    managed_host_configuration | hook_definition | guard_managed_file |
    guard_registry_setup | connection_membership
  operation: create | update | remove | register | rebind
  target: string
```

이 보고서에는 집계 상태 하나와 check/finding/action graph 하나만 있습니다. 선택적인
tagged `operation_details.result`에는 작업별 사실만 두며 두 번째 상태를 만들지 않습니다.
설정 result는 `kind=setup`, mode result는 `kind=mode_transition`, 적용에 성공한 제거
result는 `kind=removal`을 사용합니다. Status와 verify는 보통 `result`를 생략하고, 제거
dry run은 아직 발생하지 않은 결과를 생략합니다.

`SetupResult.disposition`은 setup transaction과 운영 검증을 구분합니다. Init 또는
add가 성공하면 뒤의 로컬 또는 운영 check 때문에 `status=failed`가 되더라도
`committed`를 보고합니다. Dry run은 `planned`와 `planned_changes`를 보고하며
`status=dry_run`을 직렬화하지
않습니다. 계획 변경이나 host action이 남으면 `action_required`, 둘 다 없으면
`complete`입니다.

`SetupResult.setup_lease=acquired`는 검사, planning, mutation, report 구성, 정리와
rollback이 정규 Runtime Home lease 안에서 실행됐음을 나타냅니다. 획득 결과가 busy이면
`SetupResult`를 합성하지 않습니다. 대신 실패한 schema 2 보고서가
`operation_details.setup_lease.outcome=busy`, 정규 Runtime Home, 요청 operation,
`wait_policy=immediate`, 한도 있는 elapsed time, 실패한 `setup_plan` check code
`setup_lease_busy`, finding code `setup.lease_busy`, action
`action.setup.wait_for_current_transaction`을 담습니다. Action은 활성 setup이 끝날 때까지
기다렸다가 다시 실행하도록 안내하며 coordination 파일을 노출하거나 삭제하라고 권하지
않습니다.

`SetupResult.runtime_home_publication`은 독립적인 publication 상태를 보고합니다.
`published_by_this_invocation`만 기존 대상을 교체하지 않는 rename 성공과 rollback 권한을
함께 뜻합니다. Rollback 결과는 소유한 publication을 제거했는지, setup 또는 managed-host
소비 정책 때문에 보존했는지, 제거가 불완전한지, 정확한 소유권 재검증을 통과하지 못해
제거할 수 없는지를 구분합니다. Lease를 보유한 채 no-replace publication에서 예상하지
않은 기존 최종 경로를 만나면 setup은 staging을 정리하고 target을 읽기 전용으로 검사한
뒤 `setup.concurrent_modification`을 반환합니다. 그 오래된 plan의 Store나 관리 파일
mutation은 수행하지 않습니다.

Setup이 소유한 Runtime Home rollback을 시도하면
`SetupResult.runtime_home_rollback`이 존재합니다. `outcome=removed`는 최종 경로가 부재로
관찰되었다는 뜻이며, `durability`는 상위 entry를 동기화했는지 또는 현재 플랫폼에서 그런
연산을 제공하지 않는지를 별도로 나타냅니다. `parent_synchronization_failed`여도 결과는
`removed`이고 setup disposition은 `partially_rolled_back`이 될 수 있습니다.
`outcome=removal_incomplete`는 재귀 제거 효과, 실패 단계, 정확한 경로 관찰을 유지합니다.
이 관찰은 이후 경로 재생성을 막는다는 보장이 아니며 terminal guard 상태는 대체 경로를
제거된 publication으로 취급하는 재시도를 막습니다.

Commit 전에 실패하면 `preserved`를 보고합니다. Commit한 교체 가능 상태를 복원했으면
`rolled_back`을 보고합니다. 이후 외부 변경을 안전하게 교체할 수 없어 복원이
완료되지 않으면 `partially_rolled_back`을 보고하며, 외부 bytes를 보존하고 실패한
`setup_plan` check에 rollback 개수와 오류를 기록합니다. 오래된 입력은 더 새로운
bytes를 덮어쓰지 않고 `SETUP_CONCURRENT_MODIFICATION`으로 실패합니다. 이런 실패
disposition은 host activation step을 내보내거나 setup이 적용됐다고 주장하지 않습니다.

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

`checks`는 정규 [`ConnectionVerificationReport`](agent-connection.md#connection-verification-report)의
구성원 type과 순서를 사용합니다. `findings`와 `root_cause_ids`는 공유 failure-model
계약을 사용하고 schema 2 `activation_plan`은 connection 보고서의 정확한
`IntegrationActivationPlan`을 재사용합니다. 사람용 출력은 pending check를 묶어 보여
줄 수 있지만 산문에서 cause나 step을 다시 계산하지 않습니다. 작업별 실행 안내는 현재 typed 호스트,
저장소, Runtime Home, 범위, 출력 선택 좌표에서 별도로 생성합니다.

Mode no-op은 `changed=false`, 같은 이전/현재 mode와 revision, 빈 Guard Installation
재결속 ID, 통과한 `mode_transition` check, 빈 required step, `status=complete`를 보고합니다.
실제 전환은 `changed=true`, 정확한 이전/현재 mode와 revision, 재결속한 Guard
Installation ID, 통과한 `mode_transition` check 하나, 현재 `reload_codex` step 정확히
하나, `status=action_required`를 보고합니다.

적용에 성공한 제거는 통과한 `connection_removal` check, `status=complete`, 정확한
`membership_removed`, `connection_removed`, `remaining_project_count` 사실을
`RemovalResult` 안에 보고합니다. 제거 dry run은 실제 제거 계획이 있을 때만 pending
제거 check를 사용하며 typed `planned_changes`로 계획을 보고하고 아무것도 변경하지
않습니다. 제거 적용 안내는 operation별 안내이며 connection activation step이 아닙니다.

`volicord connection status`는 완전한 읽기 전용 평가입니다. 현재 관리 구성, 신뢰,
Guard audit, 통합 revision, managed-host session 관찰을 마지막 활성 executable/MCP
server probe와 함께 평가하고 check, finding, root, activation plan을 메모리에서 조립합니다.
Process를 시작하지 않으며 파일, timestamp, 보고서, activation step, 관찰, 데이터베이스 row를
바꾸지 않습니다. 이 status 보고서를 구체화하거나 설명하는 데 활성 검증은 필요하지
않습니다.

`volicord connection verify`는 선택적으로 실행하는 활성 검증 작업입니다. `codex`를
탐색하고 version 명령을 실행한 뒤 선택한 Registry 및 project store에 한도가 있는
rollback 전용 쓰기 가능성 probe를 수행하고, `volicord mcp preflight`와 CLI 전용 MCP
self-test를 실행합니다. 서버 conformance matrix는 프로덕션 지원 protocol profile마다
독립된 `volicord mcp serve` process 하나를 실행합니다. 각 revision에서 `initialize`,
initialized notification, `tools/list`, 고정 schema와 필수 도구 검증,
`ToolVerificationRole::ManagedHostRoundTrip`에 결합된 도구 호출 정확히 하나, 계약에
정한 정상 EOF/종료 순서를 수행합니다. 모든 revision probe가 통과해야 집계
`mcp_server` check가 통과합니다.

Preflight 시작 구체화는 점검 대상 호스트 구성에 사용된 정규 관리 시작 계약에서
파생하지만, preflight는 구성, Registry 및 project 상태, protocol profile, 도구 schema,
host contract만 읽습니다. 쓰기 가능성을 probe하거나 runtime session 또는 finding을
만들지 않습니다. 각 conformance 및 host 호환성 process는 해당 검증 명령이 소유하는
새로운 일회용 Runtime Home과 Product Repository를 사용합니다. 이 process는
`manual_cli`로 남고 일회용 fixture와 함께 제거되며, 선택한 사용자 Runtime Home에
session이나 finding을 추가하거나 managed-host 관찰을 만들지 않습니다.

`ToolVerificationRole::ManagedHostRoundTrip`은 컴파일 시점에
`AgentToolId::LIST_PROJECTS`에 결합되며 wire 이름 투영은
`volicord.list_projects`입니다. CLI는 첫 번째 읽기 전용 도구를 선택하거나 독립적인 지정
도구 문자열을 유지하지 않습니다. Self-test는 이와 별도로 독립적으로 고정한 현재 host
호환성 fixture를 실행합니다. `codex` fixture는 검토된 Codex initialize `clientInfo` 및
capability 형태를 사용하고, 정확한 revision `2025-06-18`을 요청하며,
`volicord.list_projects` 호출 하나에 유효한 native session correlation metadata를
보냅니다. 이 fixture는 서버의 선호 profile에서 revision을 선택하지 않습니다. Identity는
검토된 semantic wire contract `codex-mcp-turn-metadata`를 가리키며 Codex package
version으로 fixture를 선택하지 않습니다.

stdio probe가 실패하면 현재 단계별 check code를 유지합니다. 해당 matrix entry에는
정확한 revision 또는 host fixture와 완료 관찰을 유지합니다. 실패 식별값은 안정적인
diagnostic code, 실패 단계, 영속 finding 참조로 projection하며 verification report에
두 번째 terminal 실패 객체를 저장하지 않습니다. 현재 self-test 진단 형태는 다음과
같습니다.

```yaml
McpSelfTestProgress:
  status: passed | failed | pending
  code: string
  diagnostic: string
  production_supported_revisions: string[]
  conformance: McpRevisionProbeResult[]
  host_compatibility_profiles: string[]
  host_compatibility: McpHostProbeResult[]
  tools_list?: string[]
  safe_read_only_tool: volicord.list_projects
  diagnostic_code?: string
  failure_stage?: startup | initialize | tools_list | safe_tool_call | shutdown
  finding_id?: string

McpRevisionProbeResult:
  revision: string
  status: passed | failed
  requested_revision: string
  negotiated_revision: string | null
  initialize: boolean
  initialized_notification: boolean
  pinned_schema_validated: boolean
  tools_list_observed: boolean
  tools_returned: integer | null
  required_tools_validated: boolean
  safe_read_only_tool: volicord.list_projects
  safe_read_only_tool_completed: boolean
  shutdown_completed: boolean
  diagnostic_code?: string
  failure_stage?: startup | initialize | tools_list | safe_tool_call | shutdown
  finding_id?: string

McpHostProbeResult:
  profile: codex
  fixture: string
  # 나머지는 McpRevisionProbeResult와 같은 progress/diagnostic 참조 필드이며,
  # revision은 requested/negotiated 필드로 나타냅니다.
```

집계 `tools_list`에는 빈 결과를 관찰한 경우의 빈 배열을 포함하여 관찰한 서버 inventory의
정확한 반환 이름을 담고, 각 probe는 자체 `tools_list_observed` 사실과 `tools_returned`
개수를 기록합니다. 유효한 도구 목록을 관찰하지 못했으면 집계 inventory를 생략합니다.
이후의 안전한 호출이나 종료가 실패해도 관찰한 도구 목록과 앞서 성공한 모든
완료 사실을 보존합니다. Initialize 응답이 요청한 fixture revision을 선택하기 전까지
`negotiated_revision`은 `null`입니다. 사람이 읽는 상세 출력은 반환된 도구 수와 정상 종료를
포함해 revision과 host fixture를 각각 독립적으로 보고합니다.

참조된 `DiagnosticFinding`이 제한된 실패 사실을 담당합니다. 프로세스 code에는
`process.spawn.failed`, `process.pipe_acquisition.failed`,
`process.pipe.read_failed`, `process.pipe.write_failed`,
`process.startup.timeout`, `process.initialize.timeout`, `process.tools_list.timeout`,
`process.safe_tool_call.timeout`, `process.child.exited`,
`process.shutdown.timeout`, `process.child.signaled`, `process.child.wait_failed`,
`process.cleanup.failed`, `process.preflight.report_invalid`가 있습니다. MCP 응답 실패에는
[MCP 전송](mcp-transport.md)이 담당하는 안정적인 `mcp.*` code를 사용합니다.

검증기는 stderr를 동시에 비우며 공유 사실 projection의 문자열별 제한을 적용하기 전에
최대 2 KiB를 보존합니다. Finding에는 명시적인 잘림 여부와 생략 바이트 수를 남깁니다.
stdout protocol 줄 하나는 64 KiB, 자체 검사 프로세스 하나의 stdout protocol
message는 최대 16개로 제한합니다. 단조 증가 시계의 lifecycle 기한 하나가 프로세스
진행 전체를 제어하며 프로세스 트리 종료, 직접 자식 회수, 파이프 완료에는 한도가 있는
정리 여유 시간을 사용합니다. Stderr와 제한된 I/O 상세는 맥락일 뿐입니다. 진단 식별은
임의의 자식 산문이 아니라 닫힌 프로세스 또는 protocol variant에서 나옵니다. 전체 요청,
도구 인자, 환경, 제한 없는 stderr는 영속하지 않습니다.

그런 다음 현재 managed-host 관찰을 읽고 정규 보고서 하나만 영속합니다. Plan 전에 선택한
Connection의 정확한 typed integration revision을 확보하고, immediate Registry transaction
하나에서 그 revision을 비교해 보고서만 교체합니다. 검증 중 Connection이 바뀌면 stale
보고서를 저장하지 않고 명령 재실행을 요구합니다. 관찰한 Host Plan fingerprint는
diagnostic으로만 남습니다. 검증은 이를 적용하거나 채택하지 않으며
`managed_fingerprint`를 바꾸지 않습니다. 서버 conformance와 host 호환성 결과는 CLI
probe 증거일 뿐입니다. `codex` 호환성 fixture가 통과해도 실제 관리 Codex process를
관찰한 것이 아니며 `managed_session_health` 또는 `managed_capability_proof` 근거를
만들지 않습니다. Source가 `managed_host`인 runtime session만 해당 관찰을 제공할 수
있습니다. 현재 revision에서 가장 최신인 managed session을 session health의 고정
`latest_managed_attempt`로 선택합니다. Capability proof는 initialize, initialized notification,
`tools/list`, required-tool validation, 지정된 safe call을 같은 session에서 모두 완료한
가장 최신 `latest_managed_capability_proof`에서만 통과합니다. 더 최신인 partial/failed attempt와
더 오래된 proof는 서로 다른 role evidence로 남고 서로의 milestone을 채우지 않습니다.

`volicord init`과 `volicord connection add`는 뒤의 운영 check가 실패하더라도 commit한
유효한 설정을 유지합니다. Codex를 사용할 수 없거나 self-test가 실패했다는 이유로 관리
구성을 rollback하지 않습니다. Managed-host 관찰이 아직 없는 새 유효 설정은
`action_required`이며 typed `reload_codex`, `review_project_hooks`,
`request_integration_verification`, `read_connection_status` step을 담습니다.
이 setup 명령은 계획한 관리 파일과 소유자와 일관된 Store 상태를 commit하고 최종
Connection revision을 기록한 다음에만
현재 구성과 관찰한 host 동작을 검증하고 조건부로 보고서를 영속합니다.

Required step은 pending 및 failed check에서 만드는 위상 정렬되고 중복 제거된
plan입니다. 각 step은 고정 initiator, executor, execution channel, step prerequisite,
완료 의도 check, 현재 root finding ID를 담습니다. 내부 Guard probe는 nested agent
sequence에만 있습니다. `ambient_hook_coverage` check가 통과했다면 managed
configuration repair step을 만들지 않습니다.

<a id="external-host-configuration"></a>
## 관리 Codex 구성

개인 연결은 사용자 소유 관리 Codex 구성만 씁니다. 그 entry는 선택한 정규 절대
Runtime Home을 정적 `VOLICORD_HOME`으로 결속하고
`volicord _host-launch codex --connection <connection_id>`를 호출하며 프로젝트 선택자를
담지 않고 환경 이름을 전달하지 않습니다. 공유 연결은 지원되는 프로젝트 소유 Codex
entry를 쓰고 `VOLICORD_HOME`만 전달하며 그 숨겨진 launcher의 저장소 검색 형태를
호출하고 머신 로컬 경로나 lifecycle 좌표를 내장하지 않습니다. 생성,
엄격한 검증, fingerprinting은 같은 정규 관리 시작 계약을 projection합니다. 정확한 형태,
drift, 복구, launch 맥락, uninstall 경계는
[Agent Connection](agent-connection.md#managed-mcp-launch-contract)이 담당합니다. 숨겨진
launcher는 정확한 현재 entry를 엄격하게 검증하고 한도가 있는 one-time Registry launch
lease를 만듭니다. 정적 구성에는 lease, nonce, 재사용 가능한 secret을 넣지 않습니다.
Lease는 협력적인 evidence integrity 상태이지 credential이나 identity
주장이 아닙니다.

## 관리 Guard Hook 명령

생성된 Guard wrapper는 등록된 repository, Connection, Guard Installation, policy hash,
`record` profile, Codex host output 선택과 함께 내부 `volicord hook prompt-capture`,
`pre-tool`, `post-tool` 명령을 호출합니다. 이는 관리 adapter 진입점이며 별도의 공개 workflow
API가 아닙니다.

명령은 렌더링 전에 host-neutral `GuardHookOutcome` 하나를 만듭니다. 호환되는 입력은
`CompatibleRecorded`와 policy 판단을 기록합니다. 호환되지 않는 Codex payload는
`IncompatibleRecorded`를 기록하고 policy 판단이 없으며, 해당 Guard phase를 충족하지 않은 채
계속합니다. Event Store 실패는 `PersistenceUnavailable`을 보고하며 그 실패만으로 host 동작을
거부하지 않습니다.

Codex host output에서 adapter는 유효한 hook JSON만 stdout에 쓰고 stderr를 비우며, 호환되는
계속, 호환되지 않는 관찰, persistence-unavailable feedback, 명시적인 pre-tool denial 모두
exit `0`을 사용합니다. 호환되는 명시적 `PreToolUse` policy-denial 분기만
`permissionDecision=deny`를 담습니다. Prompt context와 warning은 `additionalContext`를
사용하고 post-tool feedback은 동작이 이미 끝났다고 밝힙니다. 이 host 전용 exit와 JSON
계약은 아래의 일반 관리 명령 exit 규칙을 바꾸지 않습니다.

## MCP 명령

```text
volicord mcp preflight --connection <connection_id> [--project <project_id>] [--verbose | --json]
volicord mcp preflight --discover-repository --host codex [--verbose | --json]
volicord mcp serve --connection <connection_id> [--project <project_id>]
volicord mcp serve --discover-repository --host codex
```

`mcp preflight`는 읽기 전용 점검입니다. JSON projection은 `side_effects: []`, 증거
class `read_only_preflight`, 쓰기 가능성 `not_checked`와
`requires_active_verification`을 명시합니다. 현재 시작 구성과 선택한 읽기 표면을 점검할
수 있음만 입증하며 Store 쓰기 가능성, 활성 protocol conformance, managed-host 동작,
Agent Connection 권한은 입증하지 않습니다.

`mcp serve`는 공개 수동 stdio 표면입니다. 이 명령이 만드는 모든 runtime source는
`manual_cli`이며 어떤 flag나 environment variable로도 `managed_host`로 분류할 수
없습니다. 선택한 Runtime Home에 runtime session과 lifecycle 관찰을 만들거나 갱신하고
terminal finding을 만들 수 있습니다. 정확한 framing, lifecycle, 도구 목록, 응답
projection은 [MCP 전송](mcp-transport.md)이 담당합니다. 생성 Codex entry는 유일한 관리
시작 표면인 숨겨진 `_host-launch` 경로를 사용합니다.

폐쇄 runtime-source 집합에서 `managed_host`에는 one-time launch lease 소비가 필요하고,
`manual_cli`는 공개 stdio 또는 일회용 CLI conformance를 식별하며, `cli_preflight`와
`integration_probe`는 비관리 diagnostic 분류로 남습니다. 현재 `mcp preflight` 명령은
runtime row를 만들지 않습니다. 뒤의 세 값은 argument, environment marker, 통과한
probe로 managed evidence로 승격할 수 없습니다.

`connection verify`의 human 및 JSON 출력은 활성 작업, 증거 class
`active_verification`, 가능한 효과를 명시합니다. 가능한 효과는 rollback 전용 Store
쓰기 가능성 probe, 일회용 protocol 및 host 호환성 conformance session, diagnostic
reconciliation, 검증 보고서 영속화입니다. 활성 검증도 관리 host가 실제로 실행되었음,
미래 시작의 지속적인 가용성, 확인한 계약 밖의 Product Repository 동작 정확성은
입증하지 않습니다.

`mcp_server` check는 불변 읽기 전용 증거를 `preflight` 아래에, 마지막 활성 증거를 같은
계층의 `last_active_verification` 아래에 투영합니다. 활성 실행 전에는 후자가 null입니다.
Preflight 쓰기 가능성은 항상 `not_checked`로 남으며 활성 Registry 및 프로젝트 쓰기 결과가
이를 교체하지 않습니다. 활성 구성원이 없으면 사람용 출력은
`Storage writeability: not checked`라고 표시합니다. 활성 구성원이 있으면 verbose와 JSON
출력은 활성 증거의 별도 `observed_at`, `source=connection_verify`, 쓰기 결과, conformance
결과, side effect를 표시합니다. 결합된 결과는 유효하지 않으며 schema-version 또는
host-version 분기로 evidence 형태를 선택하지 않습니다.

`volicord connection verify codex`는 `ambient_hook_coverage`와
`correlated_guard_verification`을 분리해 보고합니다. Concise 출력은 결과 둘과 terminal
attempt의 typed repair reason을 표시합니다. Verbose 출력은 한도가 있는 전체 attempt
좌표와 현재 proof를 표시하고 JSON은 같은 typed object, runtime-session role,
verification ID를 보존합니다. CLI가 채팅 내 작업 흐름을 합성하거나 실행하지는
않습니다. Pending check는 현재 managed Codex 채팅 안에서
`volicord.begin_integration_verification`을 사용한 뒤 반환된 tagged `workflow`와 정확한
typed `tool`을 따르도록 안내합니다. `awaiting_probe`는 Guard probe를 호출하고,
`awaiting_observation`은 semantic host policy에 따라 status를 읽습니다. `complete`와
`repair_required`는 verification tool을 호출하지 않습니다. 현재 Codex 계약은
synchronous status read 한 번만 허용하며 sleep, 반복 poll, same-turn retry는 없습니다.
Begin, probe, status는 같은 workflow 상태를 보고합니다. CLI preflight, 수동 stdio
self-test, 이력 Guard 활동은 이 check를 완료할 수 없습니다. 이 명령은 Codex trust 상태를
관찰할 뿐 프로젝트 trust를 자동화·승인·우회하거나 MCP trust configuration을 변경하지
않습니다.

생성 Codex entry는 숨겨진 `_host-launch` 명령을 사용합니다. `_host-launch`는 host
소유이고 일반 help에 표시하지 않으며 별도의 관리 또는 공개 API 표면이 아닙니다.

<a id="diagnostics"></a>
## Diagnostics

```text
volicord diagnostics show FINDING_ID [--json]
volicord diagnostics session RUNTIME_SESSION_ID [--json]
volicord diagnostics workflow-metrics --repo PATH --json
```

`diagnostics show`는 영속 finding 하나와 한도가 있는 typed cause chain을 읽습니다.
`diagnostics session`은 authoritative MCP runtime session 하나와 여기에 correlation된 한도
안의 finding을 읽습니다. 두 조회는 Registry Store API를 사용하고 무제한 history를
scan하지 않습니다. JSON은 선택한 Connection 보고서가 아니라 별도의 schema 1
`DiagnosticLookupReport`를 사용합니다.

```yaml
DiagnosticLookupReport:
  schema_version: 1
  operation: diagnostics_show | diagnostics_session
  lookup_status: found | not_found
  requested_id: string
  root: StoredDiagnosticFinding | RuntimeSessionLookupRoot | null
  cause_graph: StoredDiagnosticGraph
  context: DiagnosticConnectionContext | null
  limits: string[]

StoredDiagnosticFinding:
  lifecycle: occurrence | current_state
  current_state_status?: active | resolved
  resolved_at?: timestamp | null
  finding: DiagnosticFinding
```

Finding 조회에서는 root와 cause graph의 모든 entry가 같은 lifecycle-aware 저장 finding
형태를 사용합니다. Runtime-session root는 authoritative session data와 명시적인
`terminal_condition`을 담고, 한도가 있는 graph는 lifecycle을 보존한 occurrence record를
유지합니다. `--json`이 없으면 사람용 projection은 같은 lookup result, 요청 ID와 root ID,
lifecycle, 해당하는 current state와 resolution 시각, severity, code, 안전한 subject, 관찰
시각, cause record, action, limit을 표시합니다. Finding이나 session이 없으면
`lookup_status=not_found`로 `requested_id`를 식별하고 synthetic failed finding이나
Connection check를 만들지 않습니다.

`diagnostics workflow-metrics`는 별도의 한도 있는 비권한 operability 보고서입니다. 이
JSON은 현재 diagnostics SQL에서 파생한 정확한 `canonical_schema_digest`와
`contract_id=volicord.sqlite.diagnostics`로 로컬 diagnostics 저장소를 식별하며
어느 보고서의 schema version도 사용하지 않습니다. Diagnostics 읽기는 저장소를
만들거나 프로젝트 권한 상태를 전진시키거나 evidence 또는 assurance, 닫기 준비 상태를
바꾸거나 UserAction을 해결하지 않습니다.

활성 Connection 검증은 관리 구성, Guard 파일과 관찰, 저장소 신뢰, revision freshness에
대한 CLI 소유 finding을 영속화합니다. 현재 안정 code는
`trust.repository.not_trusted`, `revision.integration.stale`,
`revision.observation.mismatch`입니다. 현재 runtime에 기록된 verification 도구 이름이
정규 role 담당 도구와 다르면 `mcp.tool_verification.designation_mismatch`를 만들며 도구
identity facts로는 `expected_tool_name`과 `observed_tool_name`만 담습니다. JSON check
detail과 verbose 출력도 같은 정확한 이름을 노출합니다. 한도 안의 임의 미래 Codex version 문구는 계속
diagnostic으로 받아들입니다. 집중 host 담당 문서는 지원하지 않는 현재 host revision
code를 정의하지 않으므로 CLI도 이를 만들어 내지 않습니다.

Terminal Guard attempt는 typed repair reason에서
`guard.probe.hook_event_not_observed`, `guard.probe.payload_incompatible`,
`guard.probe.callable_mismatch`, `guard.probe.verification_id_mismatch`,
`guard.probe.session_mismatch`, `guard.probe.turn_mismatch`,
`guard.probe.tool_use_mismatch`, `guard.probe.current_contract_changed` 중 하나로 직접
대응합니다. Fact는 acquisition stage와 retry policy를 보존합니다. Diagnostic lookup과
renderer는 code를 고르기 위해 attempt summary를 parsing하지 않습니다.
Nonterminal `UnrelatedRoutedTool` trace는 해당하는 경우에만 한도 있는 attempt detail로
노출되며 concise root cause나 terminal repair finding이 되지 않습니다. 특히 begin/status
control tool의 routed hook은 누락된 Guard probe event를
`guard.probe.callable_mismatch`로 바꾸지 않습니다. JSON, concise, verbose status는 모두
`guard.probe.hook_event_not_observed`를 root로 유지합니다.

이 CLI 소유 운영 finding은 현재 상태 snapshot입니다. `CurrentDiagnosticKey`에는 완전한
Connection scope, code, domain, stage, source, opaque typed subject identity가 들어가며 opaque
ID는 이 key의 고정된 전체 digest입니다. 안전한 subject kind와 reference는 교체 가능한
snapshot 표시로 남습니다. 따라서 관리 artifact나 Guard phase 두 곳에서 같은 code가 실패해도
서로 다른 ID를 가집니다. 같은 주체에 활성 검증을 반복하면 ID를 보존하면서 안전한 subject
표시, facts, 관찰 시각, revision 좌표, 나가는 cause edge를 원자적으로 갱신합니다. Runtime,
process, protocol 발생형 finding은 insert-only로 남으며 이 현재 상태 경로로 덮어쓸 수
없습니다.

각 폐쇄형 운영 diagnostic 값은 code, domain, stage, source, 기본 severity, summary를 담는
불변 definition 하나를 가집니다. 각 subject type은 scope, typed versioned 정규 identity
encoding과 opaque subject identity, 별도의 안전한 표시 projection을 담당합니다. Path를 담는
subject는 filesystem alias를 정규화한 뒤 그 identity를 파생하며 정규 path byte를 저장하지
않습니다. Action은 렌더링한 산문이 아니라 definition, typed facts, typed check state에서
선택합니다.

활성 검증은 CLI 담당자마다 완전한 관찰 집합을 reconcile합니다. 관찰한 모든 current
condition은 활성화하거나 갱신하고, 복구 또는 새로운 성공 관찰 뒤 집합에서 빠진 담당자
소유의 이전 active condition은 명시적으로 해소합니다. 같은 condition이 다시 나타나면 같은
key와 ID를 재활성화합니다.

Connection status와 verification은 같은 finding overlay와 report assembler를 사용합니다.
Overlay는 현재 평가가 계산한 finding을 ID로 보관하고, 명시적인 영속 finding seed를
식별하며, 각 reference의 provenance를 기록합니다. 해석할 때 inline finding을 먼저 확인한
뒤 Store를 확인합니다. 따라서 한도가 있는 graph 하나에 inline current finding, 영속된
불변 occurrence, 영속된 active current-state finding을 함께 담을 수 있습니다. 영속
reference라고 명시됐지만 Store row가 없을 때만 `diagnostics.finding_record_missing`이
됩니다. 계산한 inline finding은 이 diagnostic이나
`action.diagnostics.rebuild_current_observations`를 만들지 않습니다.

보고서는 check가 명시적으로 참조한 finding ID, 그 한도가 있는 cause chain, 작업이
의도적으로 선택한 독립 현재 finding만 선택합니다. 같은 revision에 저장되어 있어도
해소됐거나 관련 없는 finding은 현재 보고서에 다시 나타나지 않습니다. 정확한 ID를 지정한
`diagnostics show`는 occurrence 또는 최신 current-state
snapshot을 반환하며 `active` 또는 `resolved` 상태와 `resolved_at`을 포함합니다.
`diagnostics session`은 변경할 수 없는 runtime 발생 관찰 조회를 유지합니다. 찾은 record는
severity나 terminal condition과 관계없이 성공합니다.

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
자동화에서 파싱하면 안 됩니다. `--json`은 stdout에 schema 2 `DiagnosticReport` JSON
문서 하나만 씁니다. 두
플래그는 사용법 parsing 단계에서 충돌합니다. `complete`, `action_required`, 유효한 모든
dry run은 `0`으로 종료합니다. Typed `failed` 운영 보고서는 `1`, 사용법 오류는 `2`로
종료합니다. 실패한 JSON 운영 보고서는 stdout 문서 하나만 쓰고 stderr는 비워 둡니다.
실패한 사람용 운영 보고서는 stdout에 표시합니다. 예상하지 못한 런타임 또는 직렬화 오류는
stderr를 사용하고 `1`로 종료합니다. 종료 상태는 표시 문자열이나 다시 parsing한 JSON이
아니라 typed report 상태로 선택합니다.

`diagnostics show`와 `diagnostics session`은 finding severity가 아니라 lookup status로
종료 상태를 정합니다. Active finding이나 terminal occurrence의 severity가 `error`여도 찾은
record 또는 session은 `0`으로 종료합니다. Typed `not_found` lookup은 lookup report를
stdout에 쓰고 `1`로 종료합니다. 잘못된 finding ID는 stderr의 사용법 오류이며 `2`, Store
corruption 또는 그 밖의 read failure는 stderr의 runtime 오류이며 `1`로 종료합니다.
Workflow metrics는 별도 report와 종료 계약을 유지합니다.

<a id="noninteractive-approval-behavior"></a>
## 비대화형 동작

비대화형 실행은 프로젝트 신뢰를 수락하거나 UserAction을 해결하거나 민감 동작을
승인하거나 호스트가 표시한 질문에 답하지 않습니다. 구조화된 다음 동작을 반환하고 판단을
사용자에게 남깁니다.

## 실제 바이너리 릴리스 검증

게시하지 않는 `tests/release-smoke` 패키지는 전달받은 정확한 `volicord` 실행 파일을 이
문서가 담당하는 관리 명령 표면과 [MCP 전송](mcp-transport.md)이 담당하는 수동 전송을
통해 검증합니다. 안정적인 테스트 소유 Codex fixture를 제공하고 한도가 있는 자식 실행과
정리는 `volicord-test-process`에 위임합니다.

`.github/actions/volicord-release-smoke`는 재사용 workflow 호출 경계입니다. 일반 CI는
debug 바이너리를 빌드한 뒤 정확히 한 번 호출하고, 네이티브 릴리스 target마다 artifact
staging 전에 정확히 한 번 호출합니다. Release-integrity 테스트는 완전한 shell 명령
형식을 고정하지 않고 action, input, 순서, 호출 수의 의미를 검증합니다.

## 관련 담당 문서

- [Agent Connection](agent-connection.md)
- [MCP 전송](mcp-transport.md)
- [런타임 경계](runtime-boundaries.md)
- [API UserAction 스키마](api/schema-user-action.md)
- [저장 효과](storage-effects.md)
