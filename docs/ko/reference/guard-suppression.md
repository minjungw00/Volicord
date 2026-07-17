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

## 진단과 Event Projection

모든 `Unavailable` 결과는 project, Guard event 식별자,
`suppression_outcome=unavailable`, reason, scan budget, observed count와 관찰 시각을
담은 크기 제한 진단을 냅니다. 관련 Guard event의 Store 쓰기를 사용할 수 있으면
같은 machine-readable 필드를 그 event에도 포함합니다.

진단과 event에는 전체 경로 목록, correlation payload, 파일 내용, token 또는
secret을 넣지 않습니다. Store 실패 때문에 영속 진단이나 event 기록도 할 수 없으면
machine-readable Guard 응답은 결과를 계속 담고 진단 영속화를 사용할 수 없다고
보고합니다. 기록을 commit했다고 주장하면 안 됩니다.

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

## 인접 담당 문서

- 제품 경로 정규화: [런타임 경계](runtime-boundaries.md)
- 실패 범주 의미: [실패 모델](failure-model.md)
- write-ticket 및 Run 상태 형태: [API 상태 스키마](api/schema-state.md)
- Guard 릴리스 시나리오: [호스트 릴리스 증거](host-release-evidence.md)
- 보안 및 진단 비보장: [보안](security.md)
