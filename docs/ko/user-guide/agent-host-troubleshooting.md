# 에이전트 호스트 문제 해결

`host_kind=codex`, `integration_profile=record` 연결을 제한된 범위에서 복구할 때 이 가이드를
사용합니다. 복구 내내 선택한 Product Repository, 연결 범위, Runtime Home을
명시적으로 유지합니다.

## 변경 전 확인

먼저 읽기 전용 진단을 수집합니다.

```sh cli-example
volicord doctor
volicord project current
volicord connection list
volicord connection status codex --repo "<repo>"
volicord connection status codex --repo "<repo>" --json
```

진단을 없애려고 구성, Runtime Home 데이터, 저장소를 삭제하지 않습니다. 재현 가능한
실패를 전달할 때 JSON 출력을 보존하되 자격 증명이나 비공개 payload는 포함하지 않습니다.

`connection status`는 이미 완전한 읽기 전용 평가입니다. 현재 finding을 계산하고 inline 및
영속 원인을 해석하며 Runtime Home에 쓰지 않고 concise, verbose, JSON 출력에서 같은 root를
표시합니다. 활성 executable 및 Store 쓰기 가능성 probe, 일회용 MCP conformance,
diagnostic reconciliation, 보고서 영속화가 필요할 때만 `connection verify`를 사용합니다.
`mcp preflight` 자체는 읽기 전용이며 쓰기 가능성이나 활성 conformance를 성립시키지
않는다고 명시합니다.

## 안정 Finding Code 사용

`volicord doctor --json`에서는 정규화된 전체 최상위 `actions[].code`를 확인합니다.
`actions_required`와 `actions_recommended`는 이 집합을 분할하고
`primary_next_action`은 첫 단계를 식별합니다. Finding 소유 source action은
`findings[].actions[].code`에도 표시되며 최상위 집합에 포함됩니다. Connection status 또는
verification JSON에서는 `root_cause_ids`를 읽고 `findings`에서 같은 ID를 찾은 뒤 최상위
`actions[].code`와
`actions[].root_cause_ids`를 사용합니다. 출력된 finding ID, 값이 있으면 runtime-session ID,
namespaced diagnostic code를 함께 보존합니다. 영어 summary, SQLite 메시지, 경로 문구,
stderr 발췌로 실패를 분류하지 않습니다.

여러 정확한 주체가 영향을 받으면 code 하나가 여러 finding ID에 나타나는 것이 정상입니다.
Code뿐 아니라 `subject.kind`와 `subject.reference`도 비교합니다. Code가 같은 Guard
artifact, phase, repository, managed-config target을 문제 하나로 합치지 않습니다. Opaque
현재 상태 ID는 관리 경로를 그대로 드러내지 않습니다.

현재 상태 ID는 완전한 scope, code, domain, stage, source, typed subject identity에서 파생한
`finding.current.sha256:<64 lowercase hex>`입니다. 사람이 읽을 수 있는 Connection, code,
subject text에서 이 ID를 다시 만들지 말고 출력된 정확한 ID를 보존해 재사용합니다.

Code 계열에 따라 집중 복구 경계를 선택합니다.

| Code 계열 | 복구 경계 |
|---|---|
| `platform.*` | 지원 플랫폼 cell로 옮기거나 필수 플랫폼 관찰을 복구합니다. |
| `runtime_home.*` | Action이 이름 붙인 대로 절대 Runtime Home 또는 경로 경계를 고치고 permission을 복구하거나, 명시적인 새 `--home`으로 지원 `init` 흐름을 실행합니다. Schema object를 제자리에서 복구하지 않습니다. |
| `installation.*` | 완전한 provenance metadata가 있는 실행 가능한 Volicord build를 복구합니다. 있으면 `action.installation.install_build_with_complete_provenance`를 사용합니다. Dirty-source 재현성 finding에는 자동 install action이 없습니다. |
| `managed_config.*` | 같은 지원 `init` 복구를 실행합니다. Finding은 정적 환경 값이나 argument를 노출하지 않습니다. |
| `store.sqlite.busy`, `store.sqlite.locked` | Database transaction을 잡고 있는 프로세스를 끝내거나 중지한 뒤 재시도합니다. |
| `store.schema.mismatch`, `store.integrity.corruption_failure` | 기존 Runtime Home을 보존하고 지원되는 설정에는 명시적 `--home`으로 새 위치를 선택합니다. 검사는 읽기 전용이며 schema table을 직접 편집하지 않습니다. |
| `guard.*` | Guard 설치를 복구하거나 typed action이 이름 붙인 정확한 미관찰 phase를 실행합니다. |
| `trust.repository.not_trusted` | Codex에서 정확한 Product Repository를 승인합니다. |
| `revision.integration.stale` | 이미 적용한 구성 변경 뒤 Codex를 다시 불러옵니다. |
| `revision.observation.mismatch` | 현재 revision에 대해 검증을 다시 실행합니다. |

결정적인 TOML drift, schema mismatch, read-only storage, Runtime Home permission 실패라는
이유만으로 Codex를 재시작하지 않습니다. 먼저 해당 원인을 복구합니다.
`internal.unexpected_failure`는 더 좁은 담당 매핑이 없었다는 뜻이며 산문을 보고 추측할
권한을 주지 않습니다.

## Finding 또는 Runtime Session 하나 조사

Concise, verbose, JSON 출력에 나온 정확한 식별자를 사용합니다.

```sh cli-example
volicord diagnostics show "<finding-id>"
volicord diagnostics show "<finding-id>" --json
volicord diagnostics session "<runtime-session-id>"
volicord diagnostics session "<runtime-session-id>" --json
```

이 명령은 한도가 있는 Registry 조회입니다. 사람용 형태와 JSON 형태는 같은 lookup result,
root ID, lifecycle, current status, 해소 시각, typed fact를 담습니다. Active 또는 terminal
finding의 severity가 `error`여도 record나 session을 찾은 조회는 성공합니다. 식별자가 없으면
요청 ID를 표시한 typed `lookup_status=not_found` 결과를 반환합니다. 빈 finding이나 빈
session을 관찰했다는 뜻이 아닙니다. SQLite를 scan하거나 다른 식별자를 만들지 말고 선택한
Runtime Home과 정확한 ID를 확인합니다.

현재 상태 운영 ID를 지정하면 반복 검증 뒤에도 `diagnostics show`가 그 정확한 주체의 최신
snapshot을 반환하고 `active` 또는 `resolved`로 표시하며, 해소된 snapshot에는
`resolved_at`도 표시합니다. Runtime, process, protocol 발생형 finding은 변경할 수 없는
기록이며 `occurrence`로 표시합니다. 해소된 현재 상태 finding은 현재 Connection 보고서가 더
이상 참조하지 않아도 정확한 ID로 계속 조회할 수 있습니다.

관리 구성, Guard artifact 또는 installation, Product Repository trust, integration revision,
verification-tool 관찰을 복구한 뒤 읽기 전용 status로 현재 상태를 평가합니다. 새로운
executable, preflight, MCP probe evidence가 필요할 때만 활성 검증을 실행합니다. 활성 검증은
영속된 current snapshot을 reconcile할 수 있습니다. 현재 보고서에는 failed 또는 blocked
check가 선택한 finding이 들어갑니다. 영속된 해소 이력이 필요하면 보존한 정확한 ID로
`diagnostics show`를 실행합니다.

`diagnostics.finding_record_missing`은 영속 reference라고 명시된 ID에 Store row가 없다는
뜻입니다. Status가 inline으로 계산한 current finding을 나타내지 않습니다. 실제로 영속
record가 없을 때만 그 storage 복구 action을 따르며, inline 원인을 표시하기 위해 verification을
실행하지 않습니다.

## 명령을 사용할 수 없음

Codex를 시작한 환경의 `PATH`에 정확한 `volicord` 실행 파일이 있는지 확인합니다.
이미 실행 중인 Codex 프로세스는 이전 `PATH`를 유지할 수 있으므로 시작 환경을 고친 뒤
재시작합니다. 그다음 다시 실행합니다.

```sh cli-example
volicord doctor
volicord connection verify codex --repo "<repo>"
```

## 저장소 또는 연결이 모호함

의도한 Git 작업 트리에서 명령을 실행하거나 `--repo`를 명시합니다.
`volicord project current`와 `volicord connection list`로 저장 식별자를 찾습니다.
주변 디렉터리를 검색해 저장소를 선택하지 않습니다.

범위 선택자를 일관되게 유지합니다. 공유 연결은 `init`, `status`, `verify`,
`remove`에 `--shared`를 사용하고 개인 연결은 이를 뺍니다.

## Activation 상태부터 읽기

먼저 `activation_state`로 단계를 확인한 뒤 현재
`activation_plan.required_steps` suffix를 보고된 순서대로 실행합니다. 근거 부재를 trust
주장으로 바꾸지 않으면서 `hook_activation_state`를 읽습니다.

| 상태 또는 조건 | 현재 semantic step |
|---|---|
| `host_reload_required` | `reload_codex`와 뒤이어 보고된 suffix |
| Setup 변경이 있는 `hook_review_required_or_unknown` | `review_project_hooks`와 뒤이어 보고된 suffix |
| 명시적인 disabled/incompatible hook 근거 | `repair_hook_contract` |
| `mcp_observation_required` 또는 `guard_verification_required` | `request_integration_verification` |
| 최신 runtime attempt 실패 | `read_connection_status` |
| Managed configuration 실패 | `repair_managed_configuration` |

`unknown`은 hook activation이 성립하지 않았다는 뜻입니다. Untrusted 또는 disabled를
뜻하지 않습니다. `project_trust`는 별도 check이며 host가 그 관심사를 노출할 때만
적용됩니다. `latest_managed_attempt`, `latest_managed_capability_proof`,
`guard_verification_attempt`, `guard_verification_proof`는 role로 비교하고 관련
verification ID를 모두 보존하며 실제 MCP peer 정보와 PATH executable probe를 분리해
보고합니다.

`request_integration_verification`은 사용자가 Codex chat에서 시작하고 agent가 실행합니다.
Nested sequence는 project discovery, begin, workflow가 지시한 Guard probe, workflow가
지시한 status 순서입니다. Guard probe를 최상위 복구로 실행하지 않습니다. 상관 attempt가
`repair_required`를 보고하면 보고된 repair step을 따릅니다. `volicord connection
verify`는 선택적인 active diagnostics이며 activation workflow가 아닙니다.

## `action_required`

`action_required`는 구조화된 다음 단계이며 설명 없는 성공이나 치명적 실패가 아닙니다.
보고된 required step만 완료하고 terminal workflow 상태를 따른 뒤 connection status를
읽습니다. Shell sleep이나 poll loop를 사용하거나 같은 turn에 workflow를 자동으로 다시
시작하지 않습니다.

```sh cli-example
volicord connection status codex --shared --repo "<repo>"
```

상태 결과를 편집하거나 구성 파일만 보고 준비 상태를 추론하지 않습니다.

활성 검증은 별도의 선택적 진단입니다. 최신 실행 파일, Store, protocol, host probe 근거가
필요할 때만 실행합니다.

```sh cli-example
volicord connection verify codex --shared --repo "<repo>"
```

모든 `action_required` 결과 뒤에 필요한 명령은 아니며 현재 plan의 managed-host, session,
hook 또는 Codex 대화에서 수행해야 하는 Guard 단계를 대신하지 않습니다. 정확한 효과는
[관리 CLI](../reference/admin-cli.md#agent-connection-result-states)가 담당합니다.

## Guard Hook이 Warning을 내고 계속함

Codex `record` hook은 hook payload가 호환되지 않거나 Guard event를 영속화할 수 없을 때
의도적으로 exit `0`으로 계속합니다. 제한된 `additionalContext`와 안정적인 finding code를
확인합니다. Stderr가 비었다는 사실을 관찰 성공의 증거로 사용하지 않습니다.

- `guard.observation.incompatible`은 event를 호환되지 않는 관찰로 기록했고 해당 phase를
  충족하지 않았다는 뜻입니다. 이름 붙은 contract profile, hook event kind, 누락 또는
  malformed field 범주를 확인한 뒤 관리 Guard integration을 복구하거나 다시 불러옵니다.
- `guard.event.persistence_unavailable`은 Guard가 event를 commit하지 못했다는 뜻입니다.
  선택한 Runtime Home 또는 project Store를 복구하고 해당 phase를 다시 일으킵니다.
- `guard.policy.denied`는 다릅니다. 호환되는 `PreToolUse` 입력이 policy에 도달했고 Codex가
  명시적인 permission denial을 받았습니다. Parser를 복구하는 대신 현재 Write Ticket 준비
  같은 policy reason을 따릅니다.

Post-tool warning은 이미 끝난 동작을 설명합니다. 보고된 repository 변경을 조정하고 Guard가
그 변경을 막거나 되돌렸다는 증거로 warning을 해석하지 않습니다. 원래 prompt, tool input,
tool response, raw stderr를 diagnostic에 복사하지 않습니다.

## MCP 사전 점검 실패

정확한 저장 연결과 프로젝트 식별자를 사용합니다.

```sh cli-example
volicord mcp preflight --connection "<connection_id>" --project "<project_id>"
```

실패는 구조, binding, 저장 읽기, 외부 계약 문제를 식별해야 합니다. 그 문제를 고친 뒤
사전 점검을 다시 실행합니다. 통과 결과도 쓰기 가능성을 `not_checked`로 보고하므로
쓰기 가능성이나 활성 conformance 결과가 필요하면 `connection verify`를 사용합니다. 다른
전송을 시작하거나 연결 binding을 우회하지 않습니다.

## MCP 자체 검사 실패

JSON 출력으로 활성 검증을 다시 실행하고 root ID부터 확인합니다.

```sh cli-example
volicord connection verify codex --repo "<repo>" --json
```

`root_cause_ids`와 같은 `findings[].id`를 찾은 뒤 해당 finding의 `code`, typed
`facts.data`, `causes`, correlation, action을 확인합니다. 적용되는 경우 실패한 check에는
`details.last_active_verification.protocol_conformance[]` 아래에
`diagnostic_code`, `failure_stage`, `finding_id` 같은 stage별 detail도 남습니다. 종료
code, timeout, 누락 도구, stderr 발췌와 같은 제한된 Registry 사실을 확인하거나 전달할
때 finding ID를 함께 보존합니다.

`connection.runtime_sessions`에서 `latest_managed_attempt`는 현재 managed-session health로,
`latest_managed_capability_proof`는 initialize, `tools/list`, required-tool validation, 정규 verification
호출을 session 하나에서 완료한 가장 최신 항목으로 읽습니다. 두 role의 ID가 다를 수
있습니다. 더 최신인 partial attempt가 오래된 proof를 지우지 않고, 오래된 proof도 더 최신인
terminal failure를 숨기지 않습니다. Complete proof가 없으면 최신 attempt에 따라 readiness를
pending 또는 failed로 유지하고 여러 session entry의 milestone을 조합하지 않습니다.

`mcp.protocol.unsupported_version`이면 `requested_revision`과
`production_supported_revisions`를 비교합니다.
서로 다른 `selected_revision`이 관찰되거나 추적 중인 비프로덕션 revision인 경우에도
정확한 선택이 지원되지 않은 것으로 처리하며, 서버는 다른 profile로 대체하지 않습니다.
`negotiated_revision`이 없다는 것은 handshake가 완료되지 않았다는 뜻입니다.
`attempted_client_name`과 `attempted_client_version`을 보존하고
`action.mcp.use_supported_protocol_revision`을 사용합니다. 더 오래된 complete proof가
없다면 일반 concise 출력에는 적용되는 제한된 사실, failed
`managed_session_health`, blocked `managed_capability_proof` check가 나옵니다. Verbose
출력에서는 requested, selected, negotiated revision을 구분하고 실제 MCP peer
`clientInfo`와 PATH executable probe도 구분합니다.

`stderr`는 제한된 맥락으로만 취급합니다. 자식 프로세스 문구에서 기계 판독 사유를
추론하거나 자격 증명을 보고서에 복사하지 않습니다. 안정적인 `process.*`, `mcp.*`,
`host.codex.*` code가 후속 산문 parsing 없이 원인을 식별합니다. 정확한 진단 참조 필드와
프로세스 제한은 [관리 CLI](../reference/admin-cli.md), MCP code 의미와 안전한 협상 사실은
[MCP 전송](../reference/mcp-transport.md)이 담당합니다.

`mcp.tool_verification.designation_mismatch`이면
`facts.data.expected_tool_name`과 `facts.data.observed_tool_name`만 비교합니다. 그런 다음
현재 관리 Codex 연결을 통해 정규 in-chat sequence를 실행합니다. 첫 tool은
`volicord.list_projects`이며 `volicord.status`, `volicord.get_operation_result`,
`volicord.check_close` 같은 다른 읽기 전용 도구를 따로 호출해 성공해도 managed-host
capability와 Guard verification을 충족하지 않습니다.

`managed_peer.client_info.version`과 `host_executable_probe.version`이 다르면 먼저
활성 Codex 프로세스와 PATH를 확인합니다. 이 warning은 유용한 evidence지만 그 자체로
치명적 결과는 아닙니다. 보고할 때 한 version을 다른 version으로 바꾸지 않습니다.

## 채팅 내 Guard 검증이 대기 중인 경우

먼저 Guard check 둘을 모두 확인합니다. `ambient_hook_coverage=passed`는 현재 hook
definition과 configured phase를 일반적으로 관찰했다는 뜻일 뿐 correlated attempt를
증명하지 않습니다. Terminal attempt는 concise 출력에서 waiting이 아니라
`Correlated Guard verification: failed`와 typed `Reason`으로 표시됩니다. Verbose 또는
JSON 출력에서는 verification ID, runtime 및 host session, turn, event ID, attempt state,
acquisition stage, 기대·관찰 callable, retry policy, timestamp를 보존합니다.

`latest_attempt.attempt_state=repair_required`이면 중단합니다. Typed recovery action과
안정적인 `guard.probe.*` root finding을 따릅니다. 더 오래된
`latest_completed_proof`가 있다는 이유로 retry하거나 summary에서 reason을 분류하지
않습니다. Attempt 부재는 pending입니다. 현재 Codex 계약에서는 acknowledgement 뒤
status read 한 번으로 attempt가 `complete` 또는 `repair_required`가 되며 pending으로
남지 않습니다.

같은 현재 managed Codex 채팅과 native turn을 유지합니다.

1. `Run the Volicord integration verification.` 요청에서는
   `volicord.list_projects`를 호출해 정확한 프로젝트를 선택합니다.
2. `volicord.begin_integration_verification`을 호출합니다. Connection에 적격 프로젝트가
   둘 이상일 때만 `project_selector`를 제공합니다.
3. 반환된 tagged `workflow`를 정확히 따릅니다.
   - `awaiting_probe`이면 반환된 `verification_id`를 반환된
     `volicord.guard_probe` 도구에 넣습니다.
   - `awaiting_observation`이면 같은 ID로 반환된
     `volicord.get_integration_verification` 도구를 호출합니다.
   - `complete` 또는 `repair_required`이면 중단합니다.
4. 나중에 실제로 달라진 좌표로 attempt를 시작하기 전에 `repair_required`의 typed
   `retry_policy`를 따릅니다.

Project discovery는 읽기 전용입니다. Begin, probe, status는 멱등이고 비파괴적인
integration record update이며 Core 또는 Task 상태, Product Repository file, project
trust, hook review 상태를 바꾸지 않습니다. 정확한 annotation과 저장 효과는
[MCP 전송](../reference/mcp-transport.md#in-chat-integration-verification-schemas)을
봅니다.
정확한 probe replay는 상관관계가 성립해 완료된 뒤에도 현재 공유 workflow 상태인
`complete`를 반환하며 완료 또는 일치한 event 효과를 반복하지 않습니다. Repair replay는
`repair_required`로 유지됩니다. 다른 session이나 turn은 원래
acknowledgement를 읽을 수 없습니다.

통과 결과는 `workflow.kind=complete`이며 일치한 prompt, pre-tool, post-tool phase를
표시합니다. 현재 Codex 계약에서는 요청된 경우 GuardProbe를 한 번 호출하고, 이어서
요청된 경우 status를 한 번 호출합니다. Sleep, polling, 같은 turn의 새 attempt는 하지
않습니다. `repair_required`이면 중단하고 typed retry policy를 충족합니다. Cleanup 시각은
retry eligibility가 아닙니다. 다른 session이나 turn의 ID를 재사용하거나 다른 읽기 전용
도구로 대체하거나 오래된 Guard event를 성공으로 취급하지 않습니다.

Status tool 자체의 Pre/Post hook은 예상된 routed control traffic이며 Guard probe
evidence가 아닙니다. 이 hook은 nonterminal trace로만 남고 추가 status read를 소비하지
않습니다. 요청된 status 호출 한 번 전까지 Guard probe hook이 도착하지 않았다면 결과는
`hook_event_not_observed`로 유지됩니다. `callable_identity_mismatch`는 event가 Guard
probe로 해석됐지만 예상 callable과 달랐거나, 알 수 없는 same-server callable이 정확한
현재 verification ID를 명시적으로 주장했음을 뜻합니다.

Codex가 도구를 노출하지 않거나 프로젝트를 trust하지 않은 상태라면 먼저 host 소유 상태를
해결합니다. Volicord는 trust 요구 사항을 보고하지만 Codex trust control을 클릭·편집·승인·
자동화·우회하지 않습니다. 이 check를 강제로 통과시키려고 MCP trust configuration을
변경하지 않습니다.

Volicord tool이 없으면 managed MCP connection이 unavailable이라고 보고합니다. Raw
stdio를 시작하거나 Codex `_meta`를 직접 만들거나 MCP resource, resource template,
`connection verify`, CLI preflight를 managed host가 tool을 노출했다는 proof로 취급하지
않습니다.

## Codex에서 도구가 보이지 않음

별도 `project_trust` check가 적용되면 그 host 소유 action을 완료합니다. 모든 경우에
Codex가 현재 `.codex/config.toml`을 다시 읽었는지, 관리 명령이 의도한 `volicord` 실행
파일과 Runtime Home을 가리키는지 확인합니다. 그런 다음 새 conversation에서 정규 검증
요청을 보냅니다. 다른 읽기 전용 도구는 연결 진단에는 도움이 되지만 완전한 managed 및
Guard evidence를 만들지 않습니다.

구성이 있다고 활성 도구 검색이 증명되지는 않습니다. 현재 세션에 도구가 계속 없다면
진단을 보존하고 같은 관리 연결에서 새 Codex 세션을 시작합니다.

## 대기 중인 UserAction

에이전트는 대기 요청을 만들거나 재개할 수 있지만 답할 수 없습니다. 로컬 CLI User
Channel로만 해결합니다.

```sh cli-example
volicord inbox --repo "<repo>"
volicord inbox resolve USER_ACTION_REQUEST_ID --choice CHOICE_ID --repo "<repo>"
```

CLI가 저장 요청이나 resolution을 손상으로 거부하면 데이터베이스를 편집하거나 답을
추측해 대신하지 않습니다. 기계 판독 실패를 보존하고 필요한 경우 폐기 가능한 개발
상태를 다시 만듭니다.

## 기록되지 않은 변경

Unrecorded Change는 제한된 관찰이며 actor 귀속이 아닙니다. 반환된 조정 동작을
따릅니다. 완전한 호출 범위 저장소 delta에서 일치하지 않는 부분만 담습니다. 사용할 수
없는 관찰은 별도 진단으로 남겨야 하며 빈 delta나 경로 finding으로 취급하면 안 됩니다.

## 관리 구성 복구

같은 지원 설정 의도를 다시 실행합니다.

```sh cli-example
volicord init --shared --host codex --repo "<repo>" --profile record
volicord connection status codex --shared --repo "<repo>"
```

변경된 모든 파일을 검토하고 반환된 `activation_plan.required_steps`를 완료한 뒤 상태를
다시 읽습니다. 복구는 관련 없는 Codex 설정과 저장소 내용을 보존해야 합니다. 복구 뒤에
활성 검증을 일상적으로 실행하지 말고 최신 probe 근거가 필요할 때만 사용합니다.

## 일부만 제거된 것처럼 보임

정확한 의도를 미리 보고 결과를 확인합니다.

```sh cli-example
volicord connection remove codex --shared --repo "<repo>" --dry-run
volicord connection remove codex --shared --repo "<repo>"
volicord connection list --repo "<repo>"
```

제거는 결과가 이름 붙인 Volicord 관리 경로에 대해서만 성공합니다. 명령 계약이 제거
대상으로 정하지 않은 권한 기록이나 관련 없는 구성의 보존은 일부 실패가 아닙니다.

## 보안 경계

Volicord는 협력적 로컬 권한 상태입니다. 쓰기 티켓은 파일시스템 권한이 아니며 연결
검증은 모델 준수 증명이 아니고 닫기 상태는 정확성, 배포, 사람 검토 증명이 아닙니다.
[보안](../reference/security.md)을 봅니다.
