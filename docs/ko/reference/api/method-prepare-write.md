<a id="harnessprepare_write"></a>

# `harness.prepare_write` 참조

## 담당하는 것

이 문서는 현재 MVP의 `harness.prepare_write` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 계정 데이터 내보내기 확인 예시의 최소 요청과 대표 응답
- 저장 담당 문서가 기록 단위 세부사항을 정의하기 전의 메서드 수준 저장 효과 기대치

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`의 공통 스키마 본문
- 상태, 아티팩트, 사용자 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 아티팩트 생명주기, 보안 보장, Core 제품 의미

## 목적

제안된 제품 파일 쓰기 하나를 아래 항목과 비교합니다.

- 현재 Task.
- 활성 Change Unit.
- 범위.
- 기준선.
- 필요한 별도 민감 동작 승인.
- 확인된 로컬 접점 역량.

결과:

- 허용되면 소비 가능한 단일 사용 쓰기 승인(`Write Authorization`)을 만듭니다.
- 허용되지 않으면 그 쓰기 승인 경로를 거부하거나 미룹니다.

보안 비주장은 [보안](../security.md)이 담당합니다.

## 필수 입력

- `ToolEnvelope`: `dry_run=false` 커밋에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`와 `change_unit_id`. 담당 해석이 활성 Task와 활성 Change Unit을 모호하지 않게 사용할 수 있을 때만 `null`을 사용할 수 있습니다.
- `intended_operation`, `intended_paths`, `product_file_write_intended`, `sensitive_categories`, `baseline_ref`.

## 접근 요구사항

조건:

- `VerifiedSurfaceContext.access_class=write_authorization`입니다.
- `verified=true`입니다.
- 호환되는 활성 범위가 있습니다.
- 기준선이 호환됩니다.
- 필요한 사용자 소유 판단이 처리되어 있습니다.
- 필요한 경우 별도 민감 동작 승인(`sensitive_approval`)이 있습니다.
- 의도한 제품 파일 쓰기 확인에 필요한 로컬 접점 역량이 있습니다.

## 상태 버전 동작

| 결과 | 상태 버전 효과 | 쓰기 승인 효과 |
|---|---|---|
| 커밋된 `decision=allowed` | `project_state.state_version`을 정확히 한 번 올립니다. | 경로 수준 `AuthorizedAttemptScope`에 대한 활성 쓰기 승인 하나를 만듭니다. |
| 커밋된 비허용 판단 | 메서드가 소유한 쓰기 결정 이유 상태에 한해 올릴 수 있습니다. | 소비 가능한 쓰기 승인을 만들지 않습니다. |
| 커밋 전 거절 또는 `dry_run` | 올리지 않습니다. | 만들지 않습니다. |

## 성공 결과

`PrepareWriteResult`를 반환합니다.

- `base.response_kind=result`
- `base.effect_kind=core_committed`

`decision=allowed`일 때:

- `write_authorization_ref`는 `null`이 아닙니다.
- `write_authorization`은 `null`이 아닙니다.
- `authorization_effect`는 새 커밋에서 `created`, 멱등 재실행에서 `returned`입니다.

## 차단 결과

커밋된 차단 결정은 아래 `decision` 값 중 하나를 가진 `PrepareWriteResult`입니다.

- `decision=blocked`
- `decision=approval_required`
- `decision=decision_required`

조건:

- `write_decision_reasons`는 비어 있으면 안 됩니다.

비주장:

- `write_decision_reasons`는 `CloseReadinessBlocker` 값이 아닙니다.
- 쓰기 결정 이유는 닫기 준비 상태를 평가하지 않습니다.
- 소비 가능한 쓰기 승인은 만들어지지 않습니다.

## 거절 결과

`decision` 평가나 커밋 전에 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 오래된 `expected_state_version`.
- 멱등 요청 해시 충돌.
- 요청 검증 실패.
- 활성 Task 또는 Change Unit 없음.
- 로컬 접근 실패.
- Core 사용 불가.
- 기준선이 오래되었습니다.
- 유효하지 않은 요청 보장.
- 역량 실패.

비주장: `STATE_VERSION_CONFLICT`는 항상 거절 응답 오류이며 쓰기 결정 이유가 아닙니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 미리보기는 `ToolDryRunResponse`를 반환합니다. 분기 형태는 [API 코어 스키마](schema-core.md)가 담당하고, 저장 효과 없음 의미는 [저장 효과](../storage-effects.md)가 담당합니다.

## 저장 효과

커밋 시 메서드 결과에 따라 쓰기 승인 또는 쓰기 결정 상태를 지속할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.

## 최소 유효 요청

이 예시는 개인정보가 포함될 수 있는 계정 데이터 내보내기에 대해 `personal_data_export`를 `sensitive_categories`의 예시 값으로 사용합니다. 이 메서드 예시는 `personal_data_export`를 새 활성 값으로 정의하지 않고, 민감 범주의 전체 값 집합도 정의하지 않습니다.

```yaml
method: harness.prepare_write
params:
  envelope:
    project_id: proj_123
    task_id: task_456
    actor_kind: agent
    surface_id: surface_local
    request_id: req_prepare_001
    idempotency_key: idem_prepare_001
    expected_state_version: 19
    dry_run: false
    locale: ko-KR
  task_id: task_456
  change_unit_id: cu_001
  intended_operation: "계정 데이터 내보내기 확인 흐름 갱신"
  intended_paths:
    - src/account/export.ts
    - src/account/export-confirmation.ts
    - tests/account-export.test.ts
  product_file_write_intended: true
  sensitive_categories:
    - personal_data_export
  baseline_ref: baseline_account_export_001
```

## 대표 응답

별도의 민감 동작 승인이 이미 있을 때의 허용 분기(`PrepareWriteResult`, `decision=allowed`):

기존 민감 동작 승인은 `state_version: 19`의 `active_user_judgment_refs` 사용자 판단 참조로 표시됩니다. `intended_operation`은 제품 UI 확인 작업을 이름 붙일 뿐이며, 하네스 민감 동작 승인을 대신하지 않습니다.

```yaml
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 20
  events:
    - event_id: evt_1003
      event_kind: write_authorization_created
decision: allowed
state:
  project_id: proj_123
  state_version: 20
  task_ref:
    record_kind: task
    record_id: task_456
    project_id: proj_123
    task_id: task_456
    state_version: 20
write_authorization_ref:
  record_kind: write_authorization
  record_id: wa_001
  project_id: proj_123
  task_id: task_456
  state_version: 20
write_authorization:
  authorization_id: wa_001
  status: active
  basis_state_version: 19
  authorized_paths:
    - src/account/export.ts
    - src/account/export-confirmation.ts
    - tests/account-export.test.ts
authorization_effect: created
active_user_judgment_refs:
  - record_kind: user_judgment
    record_id: uj_sensitive_export_001
    project_id: proj_123
    task_id: task_456
    state_version: 19
write_decision_reasons: []
user_judgment_candidate: null
guarantee_display:
  level: cooperative
  notes:
    - "쓰기 승인(`Write Authorization`)은 하네스 호환성 기록이며 OS 권한이 아닙니다."
```

민감 동작 승인이 없을 때의 승인 필요 분기 발췌:

```yaml
decision: approval_required
write_authorization_ref: null
write_authorization: null
authorization_effect: none
write_decision_reasons:
  - code: sensitive_export_flow
    message: "계정 데이터 내보내기에는 개인정보가 포함될 수 있으므로 Write Authorization 전에 별도의 민감 동작 승인이 필요합니다."
```

## 담당 문서 링크

- 요청 래퍼, 공통 결과 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `WriteAuthorizationSummary`, 상태 요약, 참조: [API 상태 스키마](schema-state.md).
- `SensitiveActionScope`와 사용자 소유 승인 경계: [API 판단 스키마](schema-judgment.md).
- 활성 값과 접근 등급: [API 값 집합](schema-value-sets.md).
- 공개 오류, `STATE_VERSION_CONFLICT`, 차단/`dry_run` 동작: [API 오류](errors.md).
- 저장 효과와 상태 시계: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
