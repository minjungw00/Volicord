<a id="harnessprepare_write"></a>

# `harness.prepare_write` 참조

## 담당하는 것

이 문서는 기준 범위의 `harness.prepare_write` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- `PrepareWriteResult` 결정 동작
- 소비 가능한 `Write Authorization` 하나를 만드는 메서드별 처리
- prepare-write 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 판단, 값 집합, 오류의 중첩 스키마 정의
- `Write Authorization`, 일반 쓰기 승인, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 사용자 소유 판단의 Core 의미
- 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장
- 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`harness.prepare_write`는 제안된 제품 파일 쓰기 하나를 아래 항목과 비교합니다.

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

## 접근 요구사항

요구사항:

- `VerifiedSurfaceContext.access_class=write_authorization`
- `verified=true`
- 호환되는 현재 적용 범위
- 호환되는 기준선
- 필요한 사용자 소유 판단
- 필요한 경우 별도 민감 동작 승인(`sensitive_approval`)
- 의도한 제품 파일 쓰기 확인에 필요한 로컬 접점 역량

## 상태 버전 동작

| 결과 | 상태 버전 효과 | `Write Authorization` 효과 |
|---|---|---|
| 커밋된 `decision=allowed` | `project_state.state_version`을 정확히 한 번 올립니다. | `status=active`인 `Write Authorization` 하나를 만듭니다. |
| 커밋된 비허용 판단 | 메서드가 소유한 쓰기 결정 이유 상태에 한해 올릴 수 있습니다. | 소비 가능한 `Write Authorization`을 만들지 않습니다. |
| 커밋 전 거절 또는 `dry_run` | 올리지 않습니다. | 만들지 않습니다. |

## 성공 결과

`PrepareWriteResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`

`decision=allowed`일 때:

- `write_authorization_ref`는 `null`이 아닙니다.
- `write_authorization`은 `null`이 아닙니다.
- `authorization_effect`는 새 커밋에서 `created`, 멱등 재실행에서 `returned`입니다.
- 권한 부여는 경로 수준 `AuthorizedAttemptScope`에 묶입니다.
- `active_user_judgment_refs`는 별도 `sensitive_approval`을 포함해 쓰기 선행조건을 만족하는 해결된 사용자 소유 판단을 가리킬 수 있습니다.

## 차단 결과

커밋된 차단 결정은 아래 `decision` 값 중 하나를 가진 `PrepareWriteResult`입니다.

- `decision=blocked`
- `decision=approval_required`
- `decision=decision_required`

결과 데이터:

- `write_decision_reasons`는 비어 있으면 안 됩니다.

비주장:

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

비주장: `STATE_VERSION_CONFLICT`는 항상 거부 응답 오류이며 쓰기 결정 이유가 아닙니다.

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- `Write Authorization`을 만들지 않습니다.
- 쓰기 결정 상태를 지속하지 않습니다.

## 저장 효과

커밋 시 메서드 결과에 따라 `Write Authorization` 또는 쓰기 결정 상태를 지속할 수 있습니다. 정확한 저장 효과는 아래 저장 담당 문서가 담당합니다.

## 최소 유효 요청

이 예시는 `account_preference_update`를 `sensitive_categories`의 예시 문자열로 사용합니다. 민감 범주의 값 집합을 정의하지 않습니다.

```yaml
method: harness.prepare_write
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

`uj_sensitive_pref_001`은 프로필 환경설정 갱신에 맞는 `SensitiveActionScope`를 가진 기존의 해결된 `judgment_kind=sensitive_approval`을 나타냅니다. 이는 일반 쓰기 승인, 최종 수락, 잔여 위험 수락, `Write Authorization`이 아닙니다.

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
write_authorization_ref:
  record_kind: write_authorization
  record_id: wa_pref_001
  project_id: proj_pref_001
  task_id: task_pref_001
  state_version: 20
write_authorization:
  authorization_id: wa_pref_001
  status: active
  basis_state_version: 19
  authorized_paths:
    - src/preferences/profile-save.ts
    - src/preferences/profile-save.test.ts
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
  basis: "Write Authorization is a Harness compatibility record, not OS permission."
  capability_refs: []
```

### 승인 필요 분기 발췌

대응하는 민감 동작 승인이 없을 때 적용되는 분기입니다.

```yaml
decision: approval_required
write_authorization_ref: null
write_authorization: null
authorization_effect: none
write_decision_reasons:
  - code: sensitive_account_preference
    message: "Profile preference updates require separate sensitive-action approval before Write Authorization."
```

## 담당 문서 링크

- 요청 래퍼, 공통 결과 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `WriteAuthorizationSummary`, 상태 요약, 참조: [API 상태 스키마](schema-state.md).
- `SensitiveActionScope`와 사용자 소유 승인 형태: [API 판단 스키마](schema-judgment.md).
- `Write Authorization`, 쓰기 승인, 민감 동작 승인, 최종 수락, 잔여 위험 경계: [Core 모델](../core-model.md).
- 지원되는 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류, `STATE_VERSION_CONFLICT`, 분기 처리 경로, 차단/`dry_run` 동작: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 저장 효과와 상태 시계: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
