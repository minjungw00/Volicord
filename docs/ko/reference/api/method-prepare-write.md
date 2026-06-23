<a id="volicordprepare_write"></a>

# `volicord.prepare_write` 참조

## 담당하는 것

이 문서는 기준 범위의 `volicord.prepare_write` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- `PrepareWriteResult` 결정 동작
- 소비 가능한 `Write Authorization` 하나를 만드는 메서드별 처리
- 메서드별 `WriteDecisionReason.code` 생성 동작
- 쓰기 준비 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 판단, 값 집합, 오류의 중첩 스키마 정의
- `Write Authorization`, 일반 쓰기 승인, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 사용자 소유 판단의 Core 의미
- 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장
- 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`volicord.prepare_write`는 제안된 제품 파일 쓰기 하나를 아래 항목과 비교합니다.

- 현재 `Task`
- 현재 적용 Change Unit
- 현재 적용 범위
- 기준선
- 필요한 별도 민감 동작 승인
- 확인된 로컬 접점 역량

확인이 허용되면 소비 가능한 단일 사용 `Write Authorization`(쓰기 권한 부여)을 만듭니다. 확인이 허용되지 않으면 그 `Write Authorization` 경로를 거부하거나 미룹니다.

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
- `intended_paths` 항목은 `Product Repository` API 제품 경로입니다. `Product Repository` 경로 정규화는 [런타임 경계](../runtime-boundaries.md#product-repository-api-path-normalization)가 담당합니다. 이 메서드는 경로 수준 `AuthorizedAttemptScope`를 만들고 비교할 때 정규화된 저장소 상대 경로를 사용합니다.
- `sensitive_categories` 항목은 이 메서드나 프로필 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 불투명 민감 범주 분류 문자열입니다.

## 접근 요구사항

요구사항:

- `access_class=write_authorization`인 서버 파생 `VerifiedSurfaceContext`
- 호환되는 현재 적용 범위
- 호환되는 기준선
- 필요한 사용자 소유 판단
- 필요한 경우 `accepted` 결과의 별도 민감 동작 승인(`sensitive_approval`)
- 의도한 제품 파일 쓰기 확인에 필요한 로컬 접점 역량

별도 민감 동작 승인은 그 판단이 현재 상태이고, `actor_kind=user`로 해결되었으며, `resolution_outcome=accepted`인 선택지를 골랐고, 그 `JudgmentBasis`가 현재 `scope_revision`, 현재 Change Unit, 의도한 동작, 정규화된 `intended_paths`, 민감 범주, `baseline_ref`와 계속 호환될 때만 이 메서드를 만족합니다. 근거 상태가 유효하지 않거나 오래됨, 대체됨, 만료됨, 거절, 연기, 필요한 해결 권한 정보 누락, 비호환인 판단은 민감 동작 승인을 만족할 수 없습니다. 호출자는 승인을 호환되게 만들기 위한 리비전 필드를 제출하지 않습니다.

## 상태 버전 동작

| 결과 | 상태 버전 효과 | `Write Authorization` 효과 |
|---|---|---|
| 커밋된 `decision=allowed` | `project_state.state_version`을 정확히 한 번 올립니다. | `status=active`인 `Write Authorization` 하나를 만듭니다. |
| 커밋된 비허용 결정 | `project_state.state_version`을 정확히 한 번 올립니다. | 소비 가능한 `Write Authorization`을 만들지 않습니다. |
| 커밋 전 거절 또는 `dry_run` | 올리지 않습니다. | 만들지 않습니다. |

## `Write Authorization` 수명과 ID 할당

새로 만들어지는 `Write Authorization` 기록의 기본 수명은 15분입니다. `expires_at`은 표시 전용 메타데이터가 아니라 집행되는 권한 조건입니다. 유효 만료 시점은 저장된 `expires_at`과 `created_at + 15 minutes` 중 더 이른 시점입니다. 이 같은 유효 규칙은 먼 미래 만료 시각을 가진 이력 행도 제한합니다. 만료는 문자열 사전식 비교가 아니라 파싱한 UTC 타임스탬프로 계산합니다.

새로 허용된 커밋 권한은 허용된 상태 변경이 커밋될 때만 지속 `write_authorization_id`를 받습니다. 차단, 승인 필요, 판단 필요, 거절, `dry_run` 경로는 지속 `Write Authorization` ID를 할당하지 않습니다.

## 메서드 결과 필드

`PrepareWriteResult`는 커밋된 쓰기 준비 결정에 대한 메서드별 결과 분기입니다. 이 결과는 `base: ToolResultBase`와 아래 메서드 소유 최상위 필드를 담습니다.

| 필드 | 결과 필드 의미 |
|---|---|
| `base` | 공통 결과 메타데이터입니다. `events`를 포함한 `ToolResultBase` 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당합니다. 커밋된 `PrepareWriteResult` 분기는 `base.response_kind=result`와 `base.effect_kind=core_committed`를 사용합니다. `base.events[].event_kind`가 있을 때 그 값은 불투명한 예시용 분류 문자열입니다. |
| `decision` | 이 쓰기 준비 시도에 대한 메서드 결정입니다. 지원되는 값은 [API 값 집합](schema-value-sets.md#method-local-values)이 담당합니다. |
| `state` | 이 결과가 상태 스냅샷을 포함할 때의 현재 `StateSummary`입니다. `write_authority_summary`를 포함한 중첩 상태 필드는 [API 상태 스키마](schema-state.md)가 담당합니다. |
| `write_authorization_ref` | 허용 결정 결과에 포함되는 소비 가능한 `Write Authorization`의 `StateRecordRef | null`입니다. 새로 커밋된 허용 결정은 이를 만들고, 멱등 재실행은 이 필드를 바꾸지 않은 원래 커밋 응답을 반환합니다. 비허용 결정에서는 `null`입니다. |
| `write_authorization` | 허용 결정 결과에 포함되는 `Write Authorization`의 `WriteAuthorizationSummary | null`입니다. 새로 커밋된 허용 결정은 이를 만들고, 멱등 재실행은 이 필드를 바꾸지 않은 원래 커밋 응답을 반환합니다. 비허용 결정에서는 `null`입니다. |
| `authorization_effect` | `Write Authorization` 경로에 대한 메서드 결과 효과입니다. 지원되는 값은 [API 값 집합](schema-value-sets.md#method-local-values)이 담당합니다. |
| `active_user_judgment_refs` | 쓰기 준비 결정에 적용된 현재 `accepted` 결과의 사용자 소유 판단에 대한 `StateRecordRef[]`입니다. 일치하는 `sensitive_approval` 판단이 있으면 그 판단도 포함합니다. |
| `write_decision_reasons` | 비허용 결정을 설명하는 `WriteDecisionReason[]`입니다. 형태는 [API 상태 스키마](schema-state.md#current-position-display-shapes)가 담당합니다. |
| `user_judgment_candidate` | 메서드가 `Write Authorization`을 만들지 않고 집중된 사용자 소유 판단을 제안할 때의 `UserJudgmentCandidate | null`입니다. 그 밖의 경우에는 `null`입니다. 형태는 [API 판단 스키마](schema-judgment.md#userjudgmentcandidate)가 담당합니다. |
| `guarantee_display` | 메서드의 호환성 표시를 위한 `GuaranteeDisplay | null`입니다. 표시 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당하고, 보안 보장 의미는 [보안](../security.md)이 담당합니다. |

중첩된 `StateRecordRef`, `StateSummary`, `WriteAuthorizationSummary`, `WriteDecisionReason`, `UserJudgmentCandidate`, `GuaranteeDisplay` 필드 본문은 위에 연결된 스키마 담당 문서에 둡니다.

## 성공 결과

`PrepareWriteResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`

`decision=allowed`일 때:

- `write_authorization_ref`는 `null`이 아닙니다.
- `write_authorization`은 `null`이 아닙니다.
- `authorization_effect`는 새로 커밋된 `decision=allowed` 응답에서 `created`입니다.
- 멱등 재실행은 저장된 원래 커밋 `PrepareWriteResult`를 그대로 반환합니다. `authorization_effect`, `base.state_version`, `base.events`나 다른 응답 필드를 다시 계산하거나 재분류하지 않으며, `Write Authorization`을 새로 만들거나 저장 효과를 반복하지 않습니다.
- `Write Authorization`은 정규화된 저장소 상대 `intended_paths`를 사용하는 경로 수준 `AuthorizedAttemptScope`에 묶입니다.
- `active_user_judgment_refs`는 별도 `sensitive_approval`을 포함해 쓰기 선행조건을 만족하는 현재 `accepted` 결과의 사용자 소유 판단을 가리킬 수 있습니다.

## 차단 결과

커밋된 차단 결정은 아래 `decision` 값 중 하나를 가진 `PrepareWriteResult`입니다.

- `decision=blocked`
- `decision=approval_required`
- `decision=decision_required`

결과 데이터:

- `write_authorization_ref`는 `null`입니다.
- `write_authorization`은 `null`입니다.
- `authorization_effect`는 `none`입니다.
- `write_decision_reasons`는 비어 있으면 안 됩니다.
- 유효하게 커밋된 `dry_run=false` 비허용 결과는 구조화된 `write_decision_reasons`를 담은 태스크 이벤트를 하나 추가하고, 멱등성 키가 있으면 재실행 행을 만들며, `project_state.state_version`을 정확히 한 번 증가시킵니다.
- 소비 가능한 `Write Authorization`, 별도 공개 이력 메서드, 새 공개 응답 필드를 만들지 않습니다.
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
| `surface_access_class_mismatch` | `surface_capability` | 확인된 접점의 `access_class`가 `Write Authorization` 경로와 맞지 않습니다. |
| `surface_capability_insufficient` | `surface_capability` | 확인된 접점에 의도한 제품 파일 쓰기 확인에 필요한 역량이 없습니다. |
| `product_write_flag_mismatch` | `write_compatibility` | `product_file_write_intended`가 의도한 동작 또는 경로와 맞지 않습니다. |
| `no_current_change_unit` | `scope` | 쓰기 준비 결정에 사용할 현재 적용 Change Unit을 확인할 수 없습니다. |

비주장:

- 이 코드는 메서드 로컬 `WriteDecisionReason.code` 값입니다. 공개 `ErrorCode` 값, `CloseReadinessBlocker.code` 값, 전역 값 집합 항목이 아닙니다.
- `STATE_VERSION_CONFLICT`는 거절 응답 `ErrorCode`입니다. 메서드 로컬 쓰기 결정 이유로 표현하면 안 됩니다.
- `write_decision_reasons`는 `CloseReadinessBlocker` 값이 아닙니다.
- 쓰기 결정 이유는 닫기 준비 상태를 평가하지 않습니다.
- 소비 가능한 `Write Authorization`은 만들어지지 않습니다.

## 거절 결과

`decision` 평가나 커밋 전에 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 오래된 `expected_state_version`
- 멱등 요청 해시 충돌
- 요청 검증 실패
- 현재 `Task` 또는 현재 적용 Change Unit 없음
- 로컬 접근 실패
- Core 사용 불가
- 오래된 기준선
- 유효하지 않은 요청 보장
- 역량 실패

비주장: `STATE_VERSION_CONFLICT`는 항상 거절 응답 오류이며 메서드 로컬 쓰기 결정 이유가 아닙니다.

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- `Write Authorization`을 만들지 않습니다.
- 쓰기 결정 상태를 지속하지 않습니다.

## 저장 효과

커밋 시 메서드 결과에 따라 `Write Authorization` 또는 쓰기 결정 상태를 지속할 수 있습니다. 정확한 저장 효과는 아래 저장 담당 문서가 담당합니다.

아래 예시는 메서드 안에서만 성립하도록 짧게 구성했습니다. 대표 응답은 해당 `PrepareWriteResult` 분기에 필요한 필드를 보여 주며, 중첩 스키마 본문은 메서드 결과를 분명히 하는 범위에서만 예시합니다.

## 최소 유효 요청

이 예시는 `account_preference_update`를 `sensitive_categories`의 예시 문자열로 사용합니다. 민감 범주의 값 집합을 정의하지 않습니다.

```yaml
method: volicord.prepare_write
params:
  envelope:
    project_id: proj_pref_001
    task_id: task_pref_001
    actor_kind: agent
    surface_id: surface_write
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

`uj_sensitive_pref_001`은 사용자가 `resolution_outcome=accepted`로 해결했고 프로필 환경설정 갱신에 맞는 `SensitiveActionScope`를 가진 현재 `judgment_kind=sensitive_approval`을 나타냅니다. 이는 일반 쓰기 승인, 최종 수락, 잔여 위험 수락, `Write Authorization`이 아닙니다.

이 예시에서 요청은 `expected_state_version: 19`를 담습니다. 허용 커밋은 프로젝트 전체 상태를 `state_version: 20`으로 올리고, 권한 생성 커밋 뒤 결과 버전인 `basis_state_version: 20`을 가진 활성 `Write Authorization`을 만듭니다.

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 20
  events:
    - event_id: evt_pref_001
      event_kind: write_authorization_created
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
  write_authority_summary:
    status: active
    write_authorization_ref:
      record_kind: write_authorization
      record_id: wa_pref_001
      project_id: proj_pref_001
      task_id: task_pref_001
      state_version: 20
    basis_state_version: 20
    intended_paths:
      - src/preferences/profile-save.ts
      - src/preferences/profile-save.test.ts
    guarantee_display:
      level: cooperative
      basis: "Write Authorization is a Volicord compatibility record, not OS permission."
      capability_refs: []
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display:
    level: cooperative
    basis: "Write Authorization is a Volicord compatibility record, not OS permission."
    capability_refs: []
write_authorization_ref:
  record_kind: write_authorization
  record_id: wa_pref_001
  project_id: proj_pref_001
  task_id: task_pref_001
  state_version: 20
write_authorization:
  write_authorization_ref:
    record_kind: write_authorization
    record_id: wa_pref_001
    project_id: proj_pref_001
    task_id: task_pref_001
    state_version: 20
  status: active
  authorized_attempt_scope:
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
  basis_state_version: 20
  expires_at: "<future-expiration-timestamp>"
authorization_effect: created
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
  basis: "Write Authorization is a Volicord compatibility record, not OS permission."
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
write_authorization_ref: null
write_authorization: null
authorization_effect: none
write_decision_reasons:
  - category: sensitive_approval
    code: sensitive_approval_missing
    message: "Profile preference updates require separate sensitive-action approval before Write Authorization."
    related_refs: []
active_user_judgment_refs: []
user_judgment_candidate: null
guarantee_display:
  level: cooperative
  basis: "Write Authorization is a Volicord compatibility record, not OS permission."
  capability_refs: []
```

## 담당 문서 링크

- 요청 래퍼, 공통 결과 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `WriteAuthorizationSummary`, 상태 요약, 참조: [API 상태 스키마](schema-state.md).
- `SensitiveActionScope`와 사용자 소유 승인 형태: [API 판단 스키마](schema-judgment.md).
- `Write Authorization`, 쓰기 승인, 민감 동작 승인, 최종 수락, 잔여 위험 경계: [Core 모델](../core-model.md).
- `Product Repository` 경로 정규화: [런타임 경계](../runtime-boundaries.md#product-repository-api-path-normalization).
- 지원되는 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류, `STATE_VERSION_CONFLICT`, 분기 처리 경로, 차단/`dry_run` 동작: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 저장 효과와 상태 시계: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
