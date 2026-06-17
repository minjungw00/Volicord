# API 상태 스키마

이 문서는 기준 범위의 상태 형태 API 스키마를 담당합니다. `StateSummary`, `StateRecordRef`, API 데이터 형태의 생명주기 상태, 상태 관련 스냅샷, `ShapingReadiness`, 그리고 `NextActionSummary`, `WriteAuthoritySummary`, `EvidenceSummary`, `CloseReadinessBlocker`, `ValidatorResult`, `GuaranteeDisplay` 같은 표시 형태를 정의합니다.

## 담당 경계

이 문서는 상태 형태 API 필드, 중첩 구조, 참조, 요약, 스냅샷, 표시 형태, 그리고 필드 존재와 응답 효과의 경계를 담당합니다. 인접 계약은 아래 담당 문서로 연결합니다.

| 인접 계약 | 담당 문서 |
|---|---|
| 공통 요청 래퍼와 응답 분기 | [API 코어 스키마](schema-core.md) |
| 지원되는 enum 형태 값 | [API 값 집합](schema-value-sets.md) |
| 메서드 동작 | [API 메서드](methods.md)와 메서드 담당 문서 |
| 공개 오류 의미 | [API 오류 코드](error-codes.md), [API 오류 처리 경로](error-routing.md) |
| Core 생명주기와 닫기 준비 상태의 제품 의미 | [Core 모델](../core-model.md) |
| 저장소 기록과 지속 효과 | [저장소 기록](../storage-records.md), [저장 효과](../storage-effects.md) |

## 경계

상태 스키마는 API 데이터 형태만 설명합니다. 상태처럼 보이는 필드가 있다고 해서 응답 분기가 선택되거나 지속 저장, Core 전이, 재실행 행, `task_events`, 아티팩트 효과, `Write Authorization` 효과, `state_version` 증가가 생기지는 않습니다.

담당 문서 링크:
- 응답 분기 선택: [공통 응답 분기](schema-core.md#common-response)
- 메서드 동작과 효과: [API 메서드](methods.md)와 메서드 담당 문서

## 상태 참조

의미:
- `StateRecordRef`는 API 응답에 나타나는 Core 소유 기록의 공통 공개 참조 형태입니다.
- `record_kind`는 제어 값 문자열입니다.
- `record_id`, `project_id`, `task_id`는 불투명 식별자입니다.

이는 공개 참조 형태이며 저장소 행을 그대로 넣은 것이 아닙니다.

```yaml
StateRecordRef:
  record_kind: string
  record_id: string
  project_id: string
  task_id: string | null
  state_version: integer | null
```

담당 문서 링크:
- `record_kind` 값: [기록과 참조 값](schema-value-sets.md#record-and-reference-values)
- 저장소 기록 계열과 값: [저장소 기록](../storage-records.md)
- 저장소 테이블 이름과 DDL: [저장소 DDL](../storage-ddl.md)

## `StateSummary`

`StateSummary`는 지원되는 메서드가 현재 `Task` 경로를 보여 줘야 할 때 반환하는 간결한 현재 위치 상태입니다.

```yaml
StateSummary:
  project_id: string
  state_version: integer
  task_ref: StateRecordRef | null
  mode: string | null
  lifecycle: TaskLifecycleState | null
  goal_summary: string | null
  scope_summary: string | null
  non_goals: string[]
  acceptance_criteria: string[]
  autonomy_boundary: string | null
  active_change_unit_ref: StateRecordRef | null
  baseline_ref: string | null
  shaping_readiness: ShapingReadiness | null
  pending_user_judgment_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  write_authority_summary: WriteAuthoritySummary | null
  evidence_summary: EvidenceSummary | null
  close_state: string | null
  close_blockers: CloseReadinessBlocker[]
  guarantee_display: GuaranteeDisplay | null
```

의미:
- `StateSummary`는 상태 참조, 요약, 닫기 준비 상태 필드를 담는 간결한 응답 형태입니다.
- `mode`와 `close_state`는 값이 있을 때 제어 값 문자열입니다.
- `goal_summary`, `scope_summary`, `non_goals`, `acceptance_criteria`, `autonomy_boundary`는 자유 형식 표시 문자열입니다.
- `baseline_ref`는 불투명 기준선 식별자입니다.

의미하지 않는 것:
- `StateSummary` 필드가 있다는 사실만으로 메서드 커밋 여부가 정의되지 않습니다.

담당 문서 링크:
- `mode`와 `close_state` 값: [`Task` 생명주기 값](schema-value-sets.md#task-lifecycle-values)
- 커밋 결정 분기: [공통 응답 분기](schema-core.md#common-response)
- 메서드별 커밋 동작: [API 메서드](methods.md)가 안내하는 메서드 담당 문서

## `Task` 생명주기 상태

`TaskLifecycleState`는 `StateSummary`나 닫기 결과 안에 나타날 수 있는 `Task` 생명주기 필드의 API 형태입니다.

```yaml
TaskLifecycleState:
  lifecycle_phase: string
  close_reason: string
  result: string
  closed_at: string | null
```

담당 문서 링크:
- `lifecycle_phase`, `close_reason`, `result`의 지원 값: [`Task` 생명주기 값](schema-value-sets.md#task-lifecycle-values)
- 생명주기 영역의 제품 의미: [Core 모델의 `Task` 생명주기](../core-model.md#6-task-lifecycle)

## `ShapingReadiness`

의미:
- `ShapingReadiness`는 `Task`, Change Unit, 대기 중인 판단, 증거 요약, 차단 사유, 다음 행동 필드를 포괄하는 API 보기 형태입니다.
- boolean 필드와 `gaps` 배열은 현재 상태의 준비 상태 형태 데이터를 드러냅니다.

```yaml
ShapingReadiness:
  goal_summary_known: boolean
  scope_boundary_known: boolean
  non_goals_known: boolean
  affected_area_or_paths_known: boolean
  acceptance_criteria_known: boolean
  autonomy_boundary_known: boolean
  first_change_unit_known: boolean
  user_owned_blocker_kind: string | null
  next_safe_action: NextActionSummary | null
  gaps: ShapingGap[]

ShapingGap:
  gap_kind: string
  message: string
  blocker_ref: StateRecordRef | null
  user_judgment_candidate_ref: StateRecordRef | null
```

의미:
- `ShapingGap`은 차단 사유나 사용자 판단 후보를 형태상 참조할 수 있습니다.
- `user_owned_blocker_kind`와 `ShapingGap.gap_kind`는 불투명 준비 상태 분류 문자열입니다. 영향받는 담당 문서가 더 좁은 값을 공개하지 않는 한 빠짐없는 공개 값 집합이 아닙니다.
- `ShapingGap.message`는 자유 형식 표시 문자열입니다.

담당 문서 링크:
- 메서드 동작과 지속 효과: [API 메서드](methods.md)가 안내하는 메서드 담당 문서와 [저장 효과](../storage-effects.md)

<a id="current-position-display-shapes"></a>
## 현재 위치 표시 형태

```yaml
NextActionSummary:
  action_kind: string
  owner_method: string | null
  label: string
  blocking_question: string | null
  required_refs: StateRecordRef[]

WriteAuthoritySummary:
  status: string
  write_authorization_ref: StateRecordRef | null
  basis_state_version: integer | null
  intended_paths: string[]
  guarantee_display: GuaranteeDisplay | null

WriteAuthorizationSummary:
  write_authorization_ref: StateRecordRef
  status: string
  authorized_attempt_scope: object
  basis_state_version: integer
  expires_at: string | null

WriteDecisionReason:
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
```

의미:
- `NextActionSummary`는 기준 다음 행동 표시 형태입니다. 유효한 필드는 `action_kind`, `owner_method`, `label`, `blocking_question`, `required_refs`입니다.
- 오래된 `action` 또는 `reason` 필드를 쓰는 `next_actions` 항목은 유효한 `NextActionSummary`가 아닙니다.
- `WriteAuthoritySummary.status`와 `WriteAuthorizationSummary.status`는 제어 값 문자열입니다.
- `WriteDecisionReason`은 `PrepareWriteResult.write_decision_reasons`에서 사용합니다.

`NextActionSummary` 필드 분류:

| 필드 | 분류 | 규칙 |
|---|---|---|
| `action_kind` | 제어되는 행동 범주 값. | [다음 행동 값](schema-value-sets.md#next-action-values)의 값 집합을 사용합니다. 메서드 이름 값이 아닙니다. |
| `owner_method` | 담당 메서드 이름 또는 `null`. | 지원되는 공개 메서드 하나가 다음 행동을 담당할 때 그 API 메서드를 이름 붙입니다. 단일 담당 메서드가 없으면 `null`을 사용합니다. |
| `label` | 자유 형식 표시 문자열. | 사람과 에이전트가 읽는 표시 문자열이며 기준 값이 아닙니다. |
| `blocking_question` | 자유 형식 표시 문자열 또는 `null`. | 행동을 진행하기 전에 풀어야 하는 질문입니다. 필요한 질문이 없으면 `null`을 사용합니다. |
| `required_refs` | `StateRecordRef[]`. | 다음 행동에 필요한 기록입니다. 필요한 참조가 없으면 `[]`를 사용합니다. |

`WriteDecisionReason` 필드 분류:

| 필드 | 분류 | 규칙 |
|---|---|---|
| `category` | 제어되는 범주 값. | [API 값 집합](schema-value-sets.md#state-and-blocker-values)이 담당하는 `WriteDecisionReason.category` 값 집합을 사용합니다. |
| `code` | 메서드 범위의 불투명 사유 코드. | 전역의 빠짐없는 enum이 아닙니다. 메서드 담당 문서가 로컬 코드를 정의할 수 있지만, 예시 코드는 전역 값이 되지 않습니다. |
| `message` | 자유 형식 표시 문자열. | 사람과 에이전트가 읽는 표시 문자열이며 기준 값이 아닙니다. |
| `related_refs` | `StateRecordRef[]`. | 결정 사유와 관련된 기록입니다. 관련 참조가 없으면 `[]`를 사용합니다. |

`WriteDecisionReason`은 `CloseReadinessBlocker`와 다른 형태입니다.

담당 문서 링크:
- `action_kind` 값: [다음 행동 값](schema-value-sets.md#next-action-values)
- `owner_method` 값: [메서드 이름 값](schema-value-sets.md#method-name-values)
- `WriteAuthoritySummary.status`와 `WriteAuthorizationSummary.status` 값: [메서드 내부 값](schema-value-sets.md#method-local-values)
- `WriteDecisionReason.category` 값: [상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values)
- `WriteDecisionReason.code` 값 집합 경계: [불투명 문자열과 메서드 범위 문자열 필드](schema-value-sets.md#opaque-and-method-scoped-string-fields)
- `WriteDecisionReason.code` 생성과 로컬 의미: [`harness.prepare_write`](method-prepare-write.md)를 포함한 메서드 담당 문서
- 공개 `ErrorCode` 값은 별도입니다: [API 오류 코드](error-codes.md)

<a id="evidence-and-run-snapshot-shapes"></a>
## 증거와 실행 기록 스냅샷 형태

```yaml
EvidenceSummary:
  status: string
  completion_policy: CompletionPolicy
  coverage_items: EvidenceCoverageItem[]
  artifact_refs: ArtifactRef[]
  updated_by_run_ref: StateRecordRef | null

CompletionPolicy:
  evidence_required: boolean
  required_claims: string[]

EvidenceCoverageItem:
  claim: string
  required_for_close: boolean
  coverage_state: string
  supporting_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_refs: StateRecordRef[]

RunSummary:
  run_ref: StateRecordRef
  kind: string
  summary: string
  observed_changes: ObservedChanges
  artifact_refs: ArtifactRef[]

ObservedChanges:
  changed_paths: string[]
  product_file_write_observed: boolean
  sensitive_categories: string[]
  baseline_ref: string | null
```

의미:
- `EvidenceSummary.status`, `EvidenceCoverageItem.coverage_state`, `RunSummary.kind`는 제어 값 문자열입니다.
- `CompletionPolicy.required_claims`, `EvidenceCoverageItem.claim`, `RunSummary.summary`는 자유 형식 주장 또는 표시 문자열입니다.
- `ObservedChanges.changed_paths`는 경로 문자열입니다.
- `ObservedChanges.sensitive_categories`는 영향받는 메서드나 프로필 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 불투명 민감 범주 분류 문자열입니다.
- `ObservedChanges.baseline_ref`는 불투명 기준선 식별자입니다.

담당 문서 링크:
- `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md)
- 증거, `coverage_state`, 실행 종류 값: [상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values), [메서드 내부 값](schema-value-sets.md#method-local-values)
- 증거 충분성의 의미: [Core 모델의 실행 기록과 증거의 권한](../core-model.md#9-evidence-and-run-authority)
- 메서드 동작: [API 메서드](methods.md)가 안내하는 메서드 담당 문서

<a id="close-readiness-and-validation-shapes"></a>
## 닫기 준비 상태와 검증 형태

```yaml
CloseReadinessBlocker:
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
  next_actions: NextActionSummary[]

ValidatorResult:
  validator_id: string
  status: string
  severity: string | null
  message: string
  related_refs: StateRecordRef[]

GuaranteeDisplay:
  level: string
  basis: string
  capability_refs: StateRecordRef[]
```

의미:
- `CloseReadinessBlocker`는 닫기 차단 사유를 표현하는 데이터 형태입니다.
- `CloseReadinessBlocker.category`는 제어 값 문자열입니다.
- `CloseReadinessBlocker.code`는 담당 문서가 정의하는 차단 사유 코드입니다. 차단 사유 또는 메서드 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 빠짐없는 전역 공개 enum이 아닙니다.
- `CloseReadinessBlocker.message`, `ValidatorResult.message`, `GuaranteeDisplay.basis`는 자유 형식 표시 문자열입니다.
- `ValidatorResult.validator_id`는 값 집합 담당 문서가 지원되는 안정 값을 공개하기 전까지 보고용 라벨입니다.
- `ValidatorResult.status`, `ValidatorResult.severity`, `GuaranteeDisplay.level`은 제어 값 문자열입니다.

이 형태는 닫기 준비 상태 의미, 응답 처리 경로, 지속 동작을 정의하지 않습니다.

담당 문서 링크:
- 닫기 준비 상태 의미와 대체 금지 규칙: [Core 모델의 닫기 준비 상태](../core-model.md#close_task)
- 응답 분기 동작, 닫기 준비 상태 평가 순서, 커밋된 차단 결과: [`harness.close_task`](method-close-task.md)
- 닫기 차단 사유와 API 응답 분기 사이의 차단 사유 처리 경로: [API 차단 사유 처리 경로](blocker-routing.md)
- 차단 사유 범주 값(`CloseReadinessBlocker.category`)과 지원되는 `ValidatorResult.status`, `ValidatorResult.severity`, `GuaranteeDisplay.level` 값: [API 값 집합](schema-value-sets.md#state-and-blocker-values)
- 보안 보장 의미: [보안](../security.md)

## 관련 담당 문서

- [API 코어 스키마](schema-core.md): `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`.
- [API 값 집합](schema-value-sets.md#state-and-blocker-values): 차단 사유 범주 값(`CloseReadinessBlocker.category`)과 인접 상태 값.
- [API 메서드](methods.md)와 메서드 담당 문서: 이 스키마를 반환하는 메서드.
- [API 아티팩트 스키마](schema-artifacts.md): `ArtifactRef`.
- [API 판단 스키마](schema-judgment.md): `UserJudgmentCandidate`.
- [저장 효과](../storage-effects.md): 지속 저장과 상태 효과.
