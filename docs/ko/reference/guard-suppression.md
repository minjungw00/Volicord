# Guard 기록 변경 억제

이 문서는 Guard가 이미 기록된 제품 파일 변경을 이후 관찰 경로 집합에서 제거할
수 있는지 판단할 때 사용하는 기준 결과를 담당합니다. 정확한
`SuppressionOutcome` variant와 필드, scan budget, 실패 reason, 보수적 fallback,
진단 및 비차단 Guard projection을 정의합니다.

이 로직은 경로를 관찰한 출처와 독립적입니다. adapter가 관찰 경로 집합을 제공할 수
있지만, 억제 서비스는 정규 프로젝트 경로와 정규 correlation 기록만 받습니다.

## 표면 안정성

결과 variant, 필드, unavailable reason, 경로를 숨기지 않는 불변식과 scan-budget
동작은 안정 계약입니다. Query 배치, cache 전략과 helper module 배치는 내부
세부사항입니다.

## 기준 타입

```yaml
SuppressionOutcome:
  Applied:
    outcome: applied
    remaining_paths: string[]
    suppressions: RecordedChangeSuppression[]
  Unavailable:
    outcome: unavailable
    remaining_paths: string[]
    reason: SuppressionUnavailableReason
    scan_budget: integer
    observed_count: integer

RecordedChangeSuppression:
  paths: string[]
  guard_event_id: string
  write_ticket_id: string
  run_id: string
  path_identity_digest: string

SuppressionUnavailableReason:
  event_window_exceeded
  store_read_failed
  stored_event_corrupt
  correlation_payload_invalid
  run_lookup_failed
  write_ticket_lookup_failed
  path_identity_failed
```

`remaining_paths`와 모든 `paths` 목록은 정규화된 Product Repository 경로를
담고, bytewise 정렬되고 중복이 없으며 절대 경로를 담지 않습니다.
`path_identity_digest`는 비교에 사용한 정규 path identity의 소문자 64-hex
SHA-256입니다. Git 객체 ID나 권한 부여가 아닙니다.

`Applied`는 bounded scan에 필요한 모든 적격 correlation을 읽고 검증했다는
뜻입니다. 빈 `suppressions` 배열은 유효한 성공 결과입니다. 필수 Guard event,
write ticket, Run으로 뒷받침되고 identity가 바뀌지 않았음이 입증된 관찰 경로가
없다는 뜻입니다.

하나의 `RecordedChangeSuppression`은 이름 붙은 모든 기록이 같은 프로젝트와
correlation에 존재하고, 기록된 Run이 이름 붙은 write ticket을 소비했으며, 그
정규 관찰 경로가 `paths`와 같고, 현재 정규 path identity가 저장 digest와 같을
때만 유효합니다. 부분 경로 겹침은 겹치는 부분이나 나머지를 억제하지 않습니다.

## 제한된 scan

현재 scan budget은 적격 과거 Guard event 정확히 512개입니다. Store query는
정확히 512개인 경우와 더 많은 경우를 구분하기 위해 최대 513개 candidate를
관찰합니다. 이 budget은 자원 한계이며 성공 결과를 알리지 않고 자를 권한이
아닙니다.

- 적격 candidate가 0개 이상 512개 이하이면 `Applied`가 될 수 있습니다.
- 513개를 관찰하면 `reason=event_window_exceeded`, `scan_budget=512`,
  `observed_count=513`인 `Unavailable`이 됩니다.
- 이후 budget 변경은 명시적인 계약 및 테스트 변경이며 보고하지 않는 query
  limit가 아닙니다.

다른 unavailable reason의 `observed_count`는 실패를 분류하기 전에 읽거나 처리에
들어간 candidate 수이며 최대 513입니다. 억제된 경로 수가 아니라 진단 수치입니다.

## 보수적인 Unavailable 결과

억제를 진실하게 완료할 수 없으면 `Unavailable.remaining_paths`는 정규화된 전체
입력 관찰 경로 집합과 정확히 같습니다. 어떤 경로도 제거하거나 숨기거나 기록된
것으로 표시하거나 허용된 것으로 취급하지 않습니다. 이 결과에는 `suppressions`
필드가 없으며 빈 목록의 `Applied`로 바꿀 수 없습니다.

상위 Guard 결과는 다음과 같이 동작합니다.

- `decision=warn` 또는 담당자가 정의한 같은 비차단 보수 상태와 `allowed=true`로
  계속 진행합니다.
- 모든 `remaining_paths` 항목을 억제가 입증되지 않은 상태로 처리합니다.
- machine-readable 결과에 `suppression_outcome=unavailable`과 정확한 reason을
  노출합니다.
- 억제를 사용할 수 없다는 이유만으로 미기록 변경이 존재한다고 주장하지 않습니다.
- 깨끗하거나 완전히 correlation된 관찰이라고 주장하지 않습니다.

Reason 경계는 다음과 같습니다.

| Reason | 의미 |
|---|---|
| `event_window_exceeded` | 명시적 scan budget보다 많은 candidate가 존재합니다. |
| `store_read_failed` | 필수 Store 읽기를 완료하지 못했고 더 좁은 손상 또는 lookup reason을 확정하지 못했습니다. |
| `stored_event_corrupt` | 저장된 Guard event가 현재 계약을 따른다고 주장하지만 typed 또는 cross-field 규칙을 위반합니다. |
| `correlation_payload_invalid` | correlation payload가 현재 계약에 맞지 않는 문법 또는 구조를 가집니다. |
| `run_lookup_failed` | correlation된 Run을 읽거나 검증할 수 없습니다. |
| `write_ticket_lookup_failed` | correlation된 write ticket을 읽거나 검증할 수 없습니다. |
| `path_identity_failed` | 현재 정규 path identity 계산을 완료할 수 없습니다. |

손상된 영속 데이터는 `Corrupt`로 남습니다. Guard projection은 손상을 성공한 빈
억제로 바꾸지 않고 위 도메인 reason을 사용합니다. 환경 읽기 실패는
`Unavailable`로 남습니다.

## Guard Hook 결과 경계

Guard hook 처리는 서로 추론해서는 안 되는 세 가지 판단을 분리합니다.

- `GuardObservationOutcome`은 호환 event를 commit했는지, 호환되지 않는 event를
  commit했는지, event 영속화를 사용할 수 없는지를 기록합니다.
- `GuardPolicyDecision`은 선택 값이며 정확히 `Continue`, `ContinueWithContext`,
  `ContinueWithWarning`, `Deny` 중 하나입니다. 구조적으로 호환되는 입력이 policy
  평가에 도달했을 때만 존재합니다.
- Host adapter는 결과를 host JSON, context, warning, denial, stderr, process exit
  동작으로 projection합니다. 이는 Core나 Store의 판단이 아닙니다.

`GuardHookOutcome`은 관찰 결과, 선택적 policy 판단, 최대 8개의 typed diagnostic,
안전한 context 또는 warning feedback kind를 담습니다. 따라서 선택한 hook contract와
호환되지 않는 event는 `observation=IncompatibleRecorded`이고 policy 판단은 없습니다.
필수 phase를 충족하지 않지만 자동 `Deny`도 아닙니다.

Codex `record` profile에서 호환 event를 기록하고 denial이 아닌 policy 판단은 계속합니다.
명시적인 `PreToolUse` policy `Deny` 분기만 Codex permission denial로 projection합니다.
호환되지 않는 event와 event 영속화 실패는 제한된 host context, 빈 stderr, process exit
`0`을 냅니다. 영속화 실패만으로 policy denial을 만들지 않습니다. `PostToolUse` warning은
이미 끝난 동작을 설명하며 Guard가 그 동작을 막거나 되돌렸다고 주장하지 않습니다.

Codex adapter만 `hookSpecificOutput`, `permissionDecision`, `additionalContext`, stderr,
exit-code projection을 담당합니다. Core-facing type과 Store record는 Codex process-exit
동작을 encode하지 않습니다.

`PreToolUse`와 `PostToolUse`의 managed hook matcher에는 정확한 Codex native 이름
`mcp__volicord__guard_probe`가 들어갑니다. 이 이름은 정규
`AgentToolId::GUARD_PROBE` wire identity에서 생성하며 독립적으로 유지하는 literal이
아니고 모든 읽기 전용 도구로 matching 범위를 넓히지 않습니다. Prompt capture에는 도구
matcher가 없습니다. 검증 event는 현재 contract digest와 정확한 session, turn, tool-use ID,
tool 이름, `verification_id` 입력이 한도가 있는 run과 일치할 때만 match합니다.

## 진단과 Event Projection

모든 `Unavailable` 결과는 project, Guard event 식별자,
`suppression_outcome=unavailable`, reason, scan budget, observed count와 관찰 시각을
담은 크기 제한 진단을 냅니다. 관련 Guard event의 Store 쓰기를 사용할 수 있으면
같은 machine-readable 필드를 그 event에도 포함합니다.

진단과 event에는 전체 경로 목록, correlation payload, 파일 내용, token 또는
secret을 넣지 않습니다. Store 실패 때문에 영속 진단이나 event 기록도 할 수 없으면
machine-readable Guard 응답은 결과를 계속 담고 진단 영속화를 사용할 수 없다고
보고합니다. 기록을 commit했다고 주장하면 안 됩니다.

Guard 설치와 관찰 진단은 렌더링한 summary가 아니라 닫힌 원인 enum을 사용합니다.

| Code | Typed 조건 |
|---|---|
| `guard.managed_file.missing` | 필수 non-wrapper 관리 파일이 없습니다. |
| `guard.managed_file.integrity_failed` | 관리 content, 소유권, marker, permission 또는 hook contract가 다릅니다. |
| `guard.manifest.mismatch` | 엄격한 manifest 또는 wrapper authority binding이 다릅니다. |
| `guard.hook_wrapper.missing` | 필수 phase wrapper 또는 metadata가 없습니다. |
| `guard.hook_wrapper.not_executable` | 필수 wrapper가 executable 동작을 충족하지 않습니다. |
| `guard.hook_process.failed` | Typed Guard hook 프로세스가 실패했습니다. |
| `guard.phase.required_not_observed` | 현재 필수 phase를 아직 관찰하지 못했습니다. |
| `guard.observation.incompatible` | 현재 event의 hook contract가 호환되지 않습니다. |
| `guard.event.persistence_unavailable` | Guard가 event 관찰을 commit하지 못했습니다. |
| `guard.policy.denied` | 호환되는 pre-tool 입력이 policy에 도달해 거부됐습니다. |
| `guard.host_output.projection_failure` | Host adapter가 typed 결과를 projection하지 못했습니다. |
| `guard.internal.unexpected_failure` | 더 좁은 typed mapping이 없는 예상 밖 Guard 실패입니다. |
| `guard.prompt_capture.unsupported` | Host 경계가 구성된 prompt capture를 지원하지 않습니다. |
| `guard.prompt_capture.unobserved` | 지원되고 구성된 prompt capture를 아직 관찰하지 못했습니다. |

Finding 사실에는 한도가 있는 artifact kind, phase, 범주형 상태, 현재 revision 좌표만 담을
수 있습니다. 관리 파일 내용, prompt text, 임의 event payload, 제한 없는 경로는
projection하지 않습니다. Hook occurrence fact는 사용할 수 있는 contract profile, hook event
kind, 누락 또는 malformed field 범주와 정적 field label, Guard Installation ID, integration
revision, Guard event ID로 제한합니다. 전체 prompt, tool input, tool response, parser prose,
제한 없는 stderr는 절대 포함하지 않습니다. File, manifest, wrapper, 호환되지 않는 관찰 실패에는
`action.guard.repair`를 사용하고, 관찰하지 못한 필수 phase에는
`action.guard.trigger_phase`를 사용합니다. Prompt-capture code는 각 집중 action을
유지합니다. 사람용 summary를 parsing해 action을 고르지 않습니다.

현재 상태 Guard 진단은 정확한 관리 artifact, installation, 필수 phase 또는 호환되지 않는
event를 typed subject로 사용합니다. 안정적인 ID는 Connection scope, code, domain, stage,
source, opaque typed subject identity를 모두 담은 완전한 `CurrentDiagnosticKey`의 고정된 전체
digest입니다. Identity token과 ID에는 관리 경로가 들어가지 않습니다. 별도의 subject kind와
reference는 안전한 snapshot 표시입니다. 따라서 같은 Guard code가 여러 영향받는 artifact나
phase를 충돌 없이 식별할 수 있습니다. 같은 주체를 다시 관찰하면 그 finding의 안전한 subject
표시, facts, 관찰 시각, revision 좌표, cause edge를 갱신하며 오래된 현재 상태 사본을 추가하지
않습니다. Connection 보고서는 현재 check가 참조한 Guard finding과 그 한도가 있는 cause
chain만 포함합니다.

Guard 검증은 typed 관찰의 완전한 집합을 reconcile합니다. 복구한 artifact 또는
installation, 새로 관찰한 필수 phase, 호환되는 현재 event, 지원되는 prompt-capture 경계,
일치하는 integration revision은 이전 condition을 집합에서 빼고 그 active finding을
명시적으로 해소합니다. 해소된 Guard finding은 정확한 ID로 계속 조회할 수 있지만 현재
Connection 보고서에서는 제외됩니다. 각 폐쇄형 Guard diagnostic의 불변 definition은 code,
domain, stage, source, 기본 severity, summary를 담당하며 action 선택은 이 definition, typed
facts, typed check state를 사용합니다.

### Guard 검증 Dependency

Connection 검증은 다음과 같은 명시적인 Guard dependency graph를 사용합니다.

```text
Guard 파일 integrity -> Guard hook 실행 -> Guard phase 관찰 -> 상관관계가 확인된 통합 검증
```

각 check는 정확히 다섯 가지 상태 중 하나입니다. `passed`는 check가 성공적으로 끝났다는
뜻입니다. `pending`은 필요한 외부 관찰 또는 사용자가 일으키는 event가 아직 없고 이를
막는 실패 prerequisite도 없다는 뜻입니다. `failed`는 그 check 자체가 실패를 관찰했다는
뜻입니다. `blocked`는 prerequisite finding이 실패해 실행하거나 관찰할 수 없었다는
뜻입니다. `not_applicable`은 해당 Connection 또는 profile에 check가 적용되지 않는다는
뜻입니다.

Guard 파일 integrity가 실패하면 해당 check는 `failed`가 되고 hook 실행, phase 관찰,
상관관계가 확인된 통합 검증은 해소된 같은 root finding에 의해 `blocked`가 됩니다.
Prerequisite check가 blocked인 동안에는 report가 downstream 관찰을 요청하지 않습니다. Root 선택은 typed finding cause edge를
따르고, 독립 root를 결정적인 순서로 유지하며, summary를 검사하지 않습니다. 전체 check
graph와 report 집계 상태는 [Agent Connection](agent-connection.md)이 담당합니다.

## 필수 테스트

지속 계약 테스트는 다음을 다룹니다.

- 정확히 일치하고 바뀌지 않은 기록 identity의 정상 억제
- 억제 candidate가 없는 `Applied` 결과
- candidate가 정확히 512개인 경우와 512개보다 많은 경우
- 손상된 저장 event와 손상된 correlation payload
- Run과 write-ticket lookup 실패
- path-identity 계산 실패
- Store 읽기 실패가 모든 입력 경로를 보존하는지 여부
- 민감 payload 없이 warning, 진단과 event reason이 projection되는지 여부
- 호환되지 않는 prompt, pre-tool, post-tool event가 policy denial 없이 계속되면서 관찰
  충족에는 사용되지 않는지 여부
- 명시적인 pre-tool denial과 non-blocking post-tool projection
- event 영속화 실패가 제한된 Codex feedback과 함께 계속되는지 여부
- 관련 없는 읽기 전용 도구와 match하지 않는 정확한 정규 Guard probe matcher 생성
- session, turn, tool-use ID, tool 이름, verification ID, policy, revision, hook digest,
  순서, 만료 불일치의 거부

## 인접 담당 문서

- 제품 경로 정규화: [런타임 경계](runtime-boundaries.md)
- 실패 범주 의미: [실패 모델](failure-model.md)
- write-ticket 및 Run 상태 형태: [API 상태 스키마](api/schema-state.md)
- Guard 구현 테스트와 선택적 호스트 smoke: [테스트 전략](../architecture-guide/testing-strategy.md)
- 보안 및 진단 비보장: [보안](security.md)
