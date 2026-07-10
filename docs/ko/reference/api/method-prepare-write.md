<a id="volicordprepare_write"></a>

# `volicord.prepare_write` 참조

## 담당하는 것

이 문서는 기준 범위의 `volicord.prepare_write` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 호출 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- `PrepareWriteResult` 결정 동작
- 열린 쓰기 티켓 권한 기록 하나를 발급하는 메서드별 처리
- 메서드별 `WriteDecisionReason.code` 생성 동작
- 쓰기 준비 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 판단, 값 집합, 오류의 중첩 스키마 정의
- 쓰기 티켓, 일반 쓰기 승인, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 사용자 소유 판단의 Core 의미
- 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장
- 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`volicord.prepare_write`는 제안된 제품 파일 쓰기 하나를 아래 항목과 비교합니다.

- 현재 `Task`
- 현재 적용 Change Unit
- 현재 적용 범위
- 기록된 경우 현재 적용 Change Unit 효과 계약
- 기준선
- 필요한 별도 민감 동작 승인
- 확인된 호출 맥락

확인이 허용되면 열린 쓰기 티켓 하나를 발급합니다. 이 티켓은 현재 `Task`와 Change Unit 안에서 권한 있는 쓰기 의도를 나타내는 Volicord 권한 기록입니다. 파일시스템 집행, OS 권한, 셸 권한, 쓰기가 실제로 일어났다는 증명이 아닙니다. 확인이 허용되지 않으면 쓰기 티켓 경로를 거부하거나 미룹니다.

보안 비주장은 [보안](../security.md)이 담당합니다.

## 필수 입력

- 유효한 `ToolEnvelope`. 커밋되는 `dry_run`이 아닌 요청에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`와 `change_unit_id`. 담당 해석이 현재 `Task`와 현재 적용 Change Unit을 모호하지 않게 사용할 수 있을 때만 `null`을 사용할 수 있습니다.
- `intended_operation`, `intended_paths`, `product_file_write_intended`, `sensitive_categories`, `baseline_ref`.

## 요청 스키마

이 메서드는 아래 최상위 `params` 요청 형태를 담당합니다. `envelope`는 [API 코어 스키마](schema-core.md#tool-envelope)의 공통 `ToolEnvelope`이며, 이 블록은 `ToolEnvelope` 필드를 다시 정의하지 않습니다.

이 메서드 소유 요청 블록에 표시된 모든 필드는 필드 참고가 명시적으로 선택 필드라고 표시하지 않는 한 `params`의 필수 멤버입니다. `T | null`은 멤버가 반드시 있어야 하며 JSON `null`을 담을 수 있다는 뜻입니다.

```yaml
PrepareWriteRequest:
  envelope: ToolEnvelope
  task_id: string | null
  change_unit_id: string | null
  intended_operation: string
  intended_paths: string[]
  product_file_write_intended: boolean
  sensitive_categories: string[]
  baseline_ref: string
```

필드 참고:
- `intended_paths` 항목은 `Product Repository` API 제품 경로입니다. `Product Repository` 경로 정규화는 [런타임 경계](../runtime-boundaries.md#product-repository-api-path-normalization)가 담당합니다. 이 메서드는 경로 수준 `WriteTicketScope`와 호환성 저장 범위를 만들고 비교할 때 정규화된 저장소 상대 경로를 사용합니다.
- `sensitive_categories` 항목은 이 메서드나 프로필 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 불투명 민감 범주 분류 문자열입니다.

## 접근 요구사항

요구사항:

- `operation_category=agent_workflow`인 확인된 호출 맥락
- 호환되는 현재 적용 범위
- 기록된 경우 제품 파일 쓰기에 대해 호환되는 현재 적용 Change Unit 효과 계약
- 호환되는 기준선
- 필요한 사용자 소유 판단
- 필요한 경우 `accepted` 결과의 별도 민감 동작 승인(`sensitive_approval`)
- 에이전트 워크플로 호출에 호환되는 `actor_source`

별도 민감 동작 승인은 그 판단이 현재 상태이고, `resolved_by_actor_source=local_user`와 호환 User Channel 출처로 해결되었으며, `resolution_outcome=accepted`인 선택지를 골랐고, 그 `JudgmentBasis`가 현재 `scope_revision`, 현재 Change Unit, 의도한 동작, 정규화된 `intended_paths`, 민감 범주, `baseline_ref`와 계속 호환될 때만 이 메서드를 만족합니다. 근거 상태가 유효하지 않거나 오래됨, 대체됨, 만료됨, 거절, 연기, 필요한 해결 권한 정보 누락, 비호환인 판단은 민감 동작 승인을 만족할 수 없습니다. 호출자는 승인을 호환되게 만들기 위한 리비전 필드를 제출하지 않습니다.

## 상태 버전 동작

| 결과 | 상태 버전 효과 | 쓰기 티켓 효과 |
|---|---|---|
| 커밋된 `decision=allowed` | `project_state.state_version`을 정확히 한 번 올립니다. | 열린 쓰기 티켓 하나를 발급합니다. |
| 커밋된 비허용 결정 | `project_state.state_version`을 정확히 한 번 올립니다. | 쓰기 티켓을 발급하지 않습니다. |
| 커밋 전 거절 또는 `dry_run` | 올리지 않습니다. | 만들지 않습니다. |

## 쓰기 티켓 수명과 ID 할당

새로 발급되는 쓰기 티켓의 기본 수명은 15분입니다. `expires_at`은 표시 전용 메타데이터가 아니라 Volicord 호환성 조건이며 OS 수준 쓰기 기한이 아닙니다. 유효 만료 시점은 저장된 `expires_at`과 `created_at + 15 minutes` 중 더 이른 시점입니다. 이 같은 유효 규칙은 먼 미래 만료 시각을 가진 이력 행도 제한합니다. 만료는 문자열 사전식 비교가 아니라 파싱한 UTC 타임스탬프로 계산합니다.

새로 허용되어 커밋된 호출은 허용된 상태 변경이 커밋될 때만 지속 `write_ticket_id`를 받습니다. 차단, 승인 필요, 판단 필요, 거절, `dry_run` 경로는 지속 쓰기 티켓 ID를 할당하지 않습니다.

## 메서드 결과 필드

`PrepareWriteResult`는 커밋된 쓰기 준비 결정에 대한 메서드별 결과 분기입니다. 이 결과는 `base: ToolResultBase`와 아래 메서드 소유 최상위 필드를 담습니다.

| 필드 | 결과 필드 의미 |
|---|---|
| `base` | 공통 결과 메타데이터입니다. `disclosure`와 `events`를 포함한 `ToolResultBase` 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당합니다. 커밋된 `PrepareWriteResult` 분기는 `base.response_kind=result`, `base.effect_kind=core_committed`, `base.disclosure.guarantee_class=authority_record`를 사용합니다. `base.events[].event_kind`가 있을 때 그 값은 불투명한 예시용 분류 문자열입니다. |
| `decision` | 이 쓰기 준비 시도에 대한 메서드 결정입니다. 지원되는 값은 [API 값 집합](schema-value-sets.md#method-local-values)이 담당합니다. |
| `state` | 이 결과가 상태 스냅샷을 포함할 때의 현재 `StateSummary`입니다. `write_ticket_summary`를 포함한 중첩 상태 필드는 [API 상태 스키마](schema-state.md)가 담당합니다. |
| `write_ticket_id` | 허용 결정 결과에서 발급된 쓰기 티켓의 `WriteTicketId | null`입니다. 새로 커밋된 허용 결정은 이를 할당하고, 멱등 재실행은 이 필드를 바꾸지 않은 원래 커밋 응답을 반환합니다. 커밋된 비허용 결정에서는 `null`입니다. |
| `write_ticket_ref` | 발급된 쓰기 티켓을 가리키는 `record_kind=write_ticket`의 `StateRecordRef | null`입니다. 커밋된 비허용 결정에서는 `null`입니다. |
| `write_ticket` | 발급된 쓰기 티켓 권한 기록의 `WriteTicket | null`입니다. 커밋된 비허용 결정에서는 `null`입니다. |
| `write_ticket_effect` | 쓰기 티켓 경로에 대한 메서드 결과 효과입니다. `issued`는 이 커밋 결과가 열린 티켓을 만들었다는 뜻입니다. `none`은 티켓을 발급하지 않았다는 뜻입니다. 지원되는 값은 [API 값 집합](schema-value-sets.md#method-local-values)이 담당합니다. |
| `allowed_path_patterns` | 티켓 결정에서 허용으로 포착한 정규화된 `Product Repository` 경로 패턴입니다. 허용 결과에서는 티켓의 허용 경로 패턴 목록입니다. |
| `denied_path_patterns` | 티켓 결정에서 거부로 포착한 정규화된 `Product Repository` 경로 패턴입니다. 경로 수준 거부가 없으면 `[]`입니다. |
| `control_surface` | 현재 Volicord 제어 표면을 공개하는 `ControlSurfaceSummary | null`입니다. `os_enforced=false`는 티켓이 OS 수준 집행이 아니라는 뜻입니다. |
| `active_user_judgment_refs` | 쓰기 준비 결정에 적용된 현재 `accepted` 결과의 사용자 소유 판단에 대한 `StateRecordRef[]`입니다. 일치하는 `sensitive_approval` 판단이 있으면 그 판단도 포함합니다. |
| `write_decision_reasons` | 비허용 결정을 설명하는 `WriteDecisionReason[]`입니다. 형태는 [API 상태 스키마](schema-state.md#current-position-display-shapes)가 담당합니다. |
| `user_judgment_candidate` | 메서드가 쓰기 티켓을 발급하지 않고 집중된 사용자 소유 판단을 제안할 때의 `UserJudgmentCandidate | null`입니다. 그 밖의 경우에는 `null`입니다. 형태는 [API 판단 스키마](schema-judgment.md#userjudgmentcandidate)가 담당합니다. |
| `guarantee_display` | 메서드의 호환성 표시를 위한 `GuaranteeDisplay | null`입니다. 표시 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당하고, 보안 보장 의미는 [보안](../security.md)이 담당합니다. |

중첩된 `StateRecordRef`, `StateSummary`, `WriteTicket`, `WriteTicketStateSummary`, `ControlSurfaceSummary`, `WriteDecisionReason`, `UserJudgmentCandidate`, `GuaranteeDisplay` 필드 본문은 위에 연결된 스키마 담당 문서에 둡니다.

## 성공 결과

`PrepareWriteResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`

`decision=allowed`일 때:

- `write_ticket_id`, `write_ticket_ref`, `write_ticket`은 `null`이 아닙니다.
- `write_ticket_ref.record_kind`는 `write_ticket`입니다.
- `write_ticket.state`는 `open`입니다.
- `write_ticket_effect`는 새로 커밋된 `decision=allowed` 응답에서 `issued`입니다.
- `write_ticket.path_patterns.allowed`와 최상위 `allowed_path_patterns`는 이 티켓에 허용된 정규화 저장소 상대 `intended_paths`를 담습니다.
- `write_ticket.path_patterns.denied`와 최상위 `denied_path_patterns`는 허용 결과에서 `[]`입니다.
- `write_ticket.observed_paths`는 detective 프로필 hook, watcher 또는 이후 담당 문서가 정의한 관찰 경로가 티켓에 관찰을 연결하기 전까지 `[]`입니다.
- `control_surface`와 `write_ticket.control_surface`는 기준 비집행 모델에서 `os_enforced=false`를 포함해 현재 Volicord 제어 표면을 공개합니다.
- 멱등 재실행은 저장된 원래 커밋 `PrepareWriteResult`를 그대로 반환합니다. `write_ticket_effect`, `base.state_version`, `base.events`나 다른 응답 필드를 다시 계산하거나 재분류하지 않으며, 쓰기 티켓을 새로 만들거나 저장 효과를 반복하지 않습니다.
- 쓰기 티켓은 정규화된 저장소 상대 `intended_paths`를 사용하는 `WriteTicketScope`에 묶입니다.
- `active_user_judgment_refs`는 별도 `sensitive_approval`을 포함해 쓰기 선행조건을 만족하는 현재 `accepted` 결과의 사용자 소유 판단을 가리킬 수 있습니다.

## 차단 결과

커밋된 차단 결정은 아래 `decision` 값 중 하나를 가진 `PrepareWriteResult`입니다.

- `decision=blocked`
- `decision=approval_required`
- `decision=decision_required`

결과 데이터:

- `write_ticket_id`, `write_ticket_ref`, `write_ticket`은 `null`입니다.
- `write_ticket_effect`는 `none`입니다.
- `write_decision_reasons`는 비어 있으면 안 됩니다.
- 유효하게 커밋된 `dry_run=false` 비허용 결과는 구조화된 `write_decision_reasons`를 담은 `authority_events` 행을 하나 추가하고, 멱등성 키가 있으면 재실행 행을 만들며, `project_state.state_version`을 정확히 한 번 증가시킵니다.
- 쓰기 티켓을 발급하지 않고, 별도 공개 이력 메서드를 만들지 않으며, 제품 파일 쓰기 권한 기록을 만들지 않습니다.
- `volicord.status`는 과거 비허용 판단을 노출할 필요가 없습니다.
- 각 항목은 `WriteDecisionReason`입니다.
- `category`는 제어되는 `WriteDecisionReason.category` 값 집합을 사용합니다.
- `code`는 아래에 있는 이 메서드의 로컬 v1 코드 목록을 사용합니다.
- `message`는 자유 형식 표시 문자열입니다.
- `related_refs`는 `StateRecordRef[]`를 사용합니다. 관련 참조가 없으면 `[]`를 사용합니다.

메서드 로컬 `WriteDecisionReason.code` 목록:

아래 생성 의미는 이 메서드가 커밋되는 비허용 `PrepareWriteResult`에 도달했을 때만 적용됩니다. 커밋 전 실패는 여전히 오류 담당 문서에 따라 `ToolRejectedResponse`를 반환합니다.

| 코드 | 범주 | 로컬 생성 의미 |
|---|---|---|
| `scope_not_current` | `scope` | 현재 적용 범위가 요청한 `Task`, Change Unit, 또는 의도한 쓰기 기준과 호환되지 않습니다. |
| `path_out_of_scope` | `scope` | `intended_paths` 중 하나 이상이 현재 적용 범위를 벗어납니다. |
| `sensitive_approval_missing` | `sensitive_approval` | 필요한 별도 `sensitive_approval` 사용자 판단이 없습니다. |
| `user_judgment_unresolved` | `user_judgment` | 쓰기 선행조건에 필요한 사용자 소유 판단이 아직 해결되지 않았습니다. |
| `baseline_mismatch` | `baseline` | `baseline_ref`가 쓰기 호환성 기준과 맞지 않습니다. |
| `effect_contract_forbids_product_file_write` | `effect_contract` | 현재 적용 Change Unit 효과 계약이 제품 파일 쓰기를 명시적으로 금지합니다. |
| `effect_contract_effect_not_allowed` | `effect_contract` | 현재 적용 Change Unit 효과 계약의 비어 있지 않은 허용 효과 목록에 `product_file_write`가 없습니다. |
| `effect_contract_path_not_allowed` | `effect_contract` | 하나 이상의 `intended_paths`가 현재 적용 Change Unit 효과 계약의 `allowed_paths` 밖에 있습니다. |
| `product_write_flag_mismatch` | `write_compatibility` | `product_file_write_intended`가 의도한 동작 또는 경로와 맞지 않습니다. |
| `no_current_change_unit` | `scope` | 쓰기 준비 결정에 사용할 현재 적용 Change Unit을 확인할 수 없습니다. |

비주장:

- 이 코드는 메서드 로컬 `WriteDecisionReason.code` 값입니다. 공개 `ErrorCode` 값, `CloseReadinessBlocker.code` 값, 전역 값 집합 항목이 아닙니다.
- `STATE_VERSION_CONFLICT`는 거절 응답 `ErrorCode`입니다. 메서드 로컬 쓰기 결정 이유로 표현하면 안 됩니다.
- `write_decision_reasons`는 `CloseReadinessBlocker` 값이 아닙니다.
- 쓰기 결정 이유는 닫기 준비 상태를 평가하지 않습니다.
- 효과 계약 결정 사유는 민감 동작 승인, 사용자 소유 판단, 증거, 최종 수락, 닫기 준비 상태, 잔여 위험 수락, 또는 이 메서드가 `decision=allowed`일 때만 만드는 별도 쓰기 티켓을 대신하지 않습니다.
- 쓰기 티켓은 발급되지 않습니다.
- 결과 공개는 OS 샌드박싱, 네트워크 격리, 악성 코드 방어, 전체 쓰기 방지, 정확성 증명, 테스트 충분성 증명, 인간 검토 대체, 행위자 귀속 증명이 아닙니다.

## 거절 결과

`decision` 평가나 커밋 전에 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 오래된 `expected_state_version`
- 멱등 요청 해시 충돌
- 요청 검증 실패
- 현재 `Task` 또는 현재 적용 Change Unit 없음
- 행위자 출처 또는 작업 범주 불일치
- Core 사용 불가
- 오래된 기준선
- 유효하지 않은 요청 보장
- 지원되지 않는 호출 맥락

비주장: `STATE_VERSION_CONFLICT`는 항상 거절 응답 오류이며 메서드 로컬 쓰기 결정 이유가 아닙니다.

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- 커밋된 쓰기 티켓을 발급하지 않습니다.
- 미리보기가 허용될 경우 `dry_run` 요약에 `would_issue` 같은 계획된 `write_ticket` 효과를 설명할 수 있습니다.
- 쓰기 결정 상태를 지속하지 않습니다.

## 저장 효과

커밋 시 메서드 결과에 따라 쓰기 티켓 또는 쓰기 결정 상태를 지속할 수 있습니다. 티켓 기록을 뒷받침하는 물리 테이블을 포함한 정확한 저장 효과는 아래 저장 담당 문서가 담당합니다.

아래 예시는 메서드 안에서만 성립하도록 짧게 구성했습니다. 대표 응답은 해당 `PrepareWriteResult` 분기에 필요한 필드를 보여 주며, 중첩 스키마 본문은 메서드 결과를 분명히 하는 범위에서만 예시합니다.

## 최소 유효 요청

이 예시는 `account_preference_update`를 `sensitive_categories`의 예시 문자열로 사용합니다. 민감 범주의 값 집합을 정의하지 않습니다.

```yaml
method: volicord.prepare_write
params:
  envelope:
    project_id: proj_pref_001
    task_id: task_pref_001
    request_id: req_prepare_pref_001
    idempotency_key: idem_prepare_pref_001
    expected_state_version: 19
    dry_run: false
    locale: en-US
  task_id: task_pref_001
  change_unit_id: cu_pref_001
  intended_operation: "update profile preference save flow"
  intended_paths:
    - src/preferences/profile-save.ts
    - src/preferences/profile-save.test.ts
  product_file_write_intended: true
  sensitive_categories:
    - account_preference_update
  baseline_ref: baseline_pref_001
```

## 대표 응답

### 허용 분기

별도의 민감 동작 승인이 이미 있을 때 적용되는 분기입니다.

`uj_sensitive_pref_001`은 사용자가 `resolution_outcome=accepted`로 해결했고 프로필 환경설정 갱신에 맞는 `SensitiveActionScope`를 가진 현재 `judgment_kind=sensitive_approval`을 나타냅니다. 이는 일반 쓰기 승인, 최종 수락, 잔여 위험 수락, 쓰기 티켓이 아닙니다.

이 예시에서 요청은 `expected_state_version: 19`를 담습니다. 허용 커밋은 프로젝트 전체 상태를 `state_version: 20`으로 올리고, `basis_state_version: 20`을 가진 열린 쓰기 티켓을 발급합니다.

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 20
  events:
    - event_id: evt_pref_001
      event_kind: write_ticket_issued
decision: allowed
state:
  project_id: proj_pref_001
  state_version: 20
  task_ref:
    record_kind: task
    record_id: task_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    state_version: 20
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Update profile preference save flow."
  scope_summary: "Profile preference save flow update."
  non_goals:
    - "Changing account deletion."
  acceptance_criteria:
    - "Profile preferences save successfully with related tests."
  autonomy_boundary: "Stay within the profile preference save flow."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    state_version: 19
  baseline_ref: baseline_pref_001
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_ticket_summary:
    status: active
    write_ticket_ref:
      record_kind: write_ticket
      record_id: wt_pref_001
      project_id: proj_pref_001
      task_id: task_pref_001
      state_version: 20
    basis_state_version: 20
    intended_paths:
      - src/preferences/profile-save.ts
      - src/preferences/profile-save.test.ts
    guarantee_display:
      level: cooperative
      basis: "Write ticket is a Volicord authority record, not OS permission."
      capability_refs: []
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display:
    level: cooperative
    basis: "Write ticket is a Volicord authority record, not OS permission."
    capability_refs: []
write_ticket_id: wt_pref_001
write_ticket_ref:
  record_kind: write_ticket
  record_id: wt_pref_001
  project_id: proj_pref_001
  task_id: task_pref_001
  state_version: 20
write_ticket:
  write_ticket_id: wt_pref_001
  write_ticket_ref:
    record_kind: write_ticket
    record_id: wt_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    state_version: 20
  state: open
  scope:
    task_id: task_pref_001
    change_unit_id: cu_pref_001
    intended_operation: "update profile preference save flow"
    product_file_write_intended: true
    sensitive_categories:
      - account_preference_update
    baseline_ref: baseline_pref_001
  path_patterns:
    allowed:
      - src/preferences/profile-save.ts
      - src/preferences/profile-save.test.ts
    denied: []
  observed_paths: []
  basis_state_version: 20
  expires_at: "<future-expiration-timestamp>"
  control_surface:
    selected_profile: record
    host_hooks_active: false
    session_watcher_active: false
    cooperative_pre_tool_warning_available: false
    cooperative_pre_tool_denial_available: false
    unrecorded_changes_detectable: false
    actor_identity_provable: false
    os_enforced: false
  guarantee_display:
    level: cooperative
    basis: "Write ticket is a Volicord authority record, not OS permission."
    capability_refs: []
write_ticket_effect: issued
allowed_path_patterns:
  - src/preferences/profile-save.ts
  - src/preferences/profile-save.test.ts
denied_path_patterns: []
control_surface:
  selected_profile: record
  host_hooks_active: false
  session_watcher_active: false
  cooperative_pre_tool_warning_available: false
  cooperative_pre_tool_denial_available: false
  unrecorded_changes_detectable: false
  actor_identity_provable: false
  os_enforced: false
active_user_judgment_refs:
  - record_kind: user_judgment
    record_id: uj_sensitive_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    state_version: 19
write_decision_reasons: []
user_judgment_candidate: null
guarantee_display:
  level: cooperative
  basis: "Write ticket is a Volicord authority record, not OS permission."
  capability_refs: []
```

### 승인 필요 분기

대응하는 민감 동작 승인이 없을 때 적용되는 분기입니다.

아래의 `code: sensitive_approval_missing` 값은 이 메서드의 로컬 이유 코드 중 하나입니다. 공개 `ErrorCode` 값이 아닙니다.

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 20
  events: []
decision: approval_required
write_ticket_id: null
write_ticket_ref: null
write_ticket: null
write_ticket_effect: none
allowed_path_patterns:
  - src/preferences/profile-save.ts
  - src/preferences/profile-save.test.ts
denied_path_patterns: []
control_surface:
  selected_profile: record
  host_hooks_active: false
  session_watcher_active: false
  cooperative_pre_tool_warning_available: false
  cooperative_pre_tool_denial_available: false
  unrecorded_changes_detectable: false
  actor_identity_provable: false
  os_enforced: false
write_decision_reasons:
  - category: sensitive_approval
    code: sensitive_approval_missing
    message: "Profile preference updates require separate sensitive-action approval before write ticket issuance."
    related_refs: []
active_user_judgment_refs: []
user_judgment_candidate: null
guarantee_display:
  level: cooperative
  basis: "Write ticket is a Volicord authority record, not OS permission."
  capability_refs: []
```

## 담당 문서 링크

- 요청 래퍼, 공통 결과 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `WriteTicket`, `WriteTicketStateSummary`, 상태 요약, 참조: [API 상태 스키마](schema-state.md).
- `SensitiveActionScope`와 사용자 소유 승인 형태: [API 판단 스키마](schema-judgment.md).
- 쓰기 티켓, 쓰기 승인, 민감 동작 승인, 최종 수락, 잔여 위험 경계: [Core 모델](../core-model.md).
- `Product Repository` 경로 정규화: [런타임 경계](../runtime-boundaries.md#product-repository-api-path-normalization).
- 지원되는 값과 operation category: [API 값 집합](schema-value-sets.md#operation-category-values).
- 공개 오류, `STATE_VERSION_CONFLICT`, 분기 처리 경로, 차단/`dry_run` 동작: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 저장 효과와 상태 시계: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
