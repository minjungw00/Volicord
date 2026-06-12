# API 상태 스키마

의미:
- 이 문서는 현재 MVP의 상태 형태 API 스키마를 담당합니다.
- 참조 문서일 뿐입니다.

의미하지 않는 것:
- 런타임 상태, 생성된 상태 보기, 저장소 행, 상태 효과를 만들지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `StateSummary`
- `StateRecordRef`
- API 데이터 형태로서의 Task 생명주기 상태
- 상태 관련 스냅샷과 참조 구조
- `ShapingReadiness`
- `NextActionSummary`, `WriteAuthoritySummary`, `EvidenceSummary`, `CloseReadinessBlocker`, `ValidatorResult`, `GuaranteeDisplay` 같은 현재 위치 표시 스키마
- 상태 형태 데이터와 응답 효과의 경계

이 문서는 담당하지 않습니다.

- 공통 요청 래퍼나 응답 분기: [API 코어 스키마](schema-core.md)
- 활성 enum 형태 값: [API 값 집합](schema-value-sets.md)
- 메서드 동작: [API 메서드](methods.md)와 메서드 담당 문서
- 공개 오류 의미: [API 오류](errors.md)
- Core 생명주기와 닫기 준비 상태의 제품 의미: [Core 모델](../core-model.md)
- 저장소 기록과 지속 효과: [저장소 기록](../storage-records.md), [저장 효과](../storage-effects.md)

## 경계

의미:
- 상태 스키마는 API 데이터 형태를 설명합니다.

의미하지 않는 것:
- 상태처럼 보이는 필드가 있다고 해서 그 자체로 지속 저장이 생기지 않습니다.
- 상태처럼 보이는 필드가 있다고 해서 그 자체로 Core 전이, 재실행 행, `task_events`, 아티팩트 효과, Write Authorization 효과, `state_version` 증가가 생기지 않습니다.

담당 문서 링크:
- 응답 분기 선택: [공통 응답 분기](schema-core.md#common-response)
- 메서드 동작과 효과: [API 메서드](methods.md)와 메서드 담당 문서

## 상태 참조

의미:
- `StateRecordRef`는 API 응답에 나타나는 Core 소유 기록의 공통 공개 참조 형태입니다.

의미하지 않는 것:
- 저장소 행을 그대로 넣은 것이 아닙니다.

```yaml
StateRecordRef:
  record_kind: string
  record_id: string
  project_id: string
  task_id: string | null
  state_version: integer | null
```

담당 문서 링크:
- `record_kind` 값: [기록과 참조 값](schema-value-sets.md#기록과-참조-값)
- 저장소 테이블 이름과 DDL: [저장소 기록](../storage-records.md)

## `StateSummary`

`StateSummary`는 현재 Task 경로를 보여 줘야 하는 활성 메서드가 반환하는 간결한 현재 위치 상태입니다.

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
- `StateSummary`는 저장된 Core 상태, 계산된 읽기 전용 상태, 닫기 준비 상태 관찰을 요약할 수 있습니다.

의미하지 않는 것:
- 어떤 메서드가 커밋했는지를 결정하지 않습니다.

담당 문서 링크:
- 커밋 결정 분기: [공통 응답 분기](schema-core.md#common-response)
- 메서드별 커밋 동작: [API 메서드](methods.md)가 안내하는 메서드 담당 문서

## Task 생명주기 상태

`TaskLifecycleState`는 `StateSummary`나 닫기 결과 안에 나타날 수 있는 Task 생명주기 필드의 API 형태입니다.

```yaml
TaskLifecycleState:
  lifecycle_phase: string
  close_reason: string
  result: string
  closed_at: string | null
```

담당 문서 링크:
- `lifecycle_phase`, `close_reason`, `result`의 활성 값: [Task 생명주기 값](schema-value-sets.md#task-생명주기-값)
- 생명주기 영역의 제품 의미: [Core 모델의 Task 생명주기](../core-model.md#6-task-생명주기)

## `ShapingReadiness`

의미:
- `ShapingReadiness`는 Task, Change Unit, 대기 중인 판단, 증거 요약, 차단 사유, 다음 행동 상태에서 파생되는 API 보기입니다.
- 현재 담당 상태가 다음 안전한 행동에 충분히 구체적인지를 보여 줍니다.

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
- 준비 상태 공백은 차단 사유, 대기 중이거나 후보인 사용자 판단, 범위 갱신 다음 행동으로 이어질 수 있습니다.

의미하지 않는 것:
- 별도 활성 Discovery Brief, Question Queue, Assumption Register, 생성된 계획 아티팩트를 만들지 않습니다.

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
- `WriteDecisionReason`은 `PrepareWriteResult.write_decision_reasons`에서 사용합니다.

의미하지 않는 것:
- 닫기 준비 상태의 차단 사유가 아닙니다.

담당 문서 링크:
- 활성 범주와 사유 값: [상태와 차단 사유 값](schema-value-sets.md#상태와-차단-사유-값)
- 공개 오류 코드의 의미: [API 오류](errors.md)

## 증거와 Run 스냅샷 형태

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

담당 문서 링크:
- `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md)
- 증거 충분성의 의미: [Core 모델의 실행과 증거의 권한](../core-model.md#9-실행과-증거의-권한)
- 메서드 동작: [API 메서드](methods.md)가 안내하는 메서드 담당 문서

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
- `CloseReadinessBlocker`는 닫기 준비 상태 발견 사항을 표현하는 데이터 형태입니다.

의미하지 않는 것:
- 닫기 준비 상태 개념 전체가 아닙니다.
- 그 자체로 지속 저장을 뜻하지 않습니다.

담당 문서 링크:
- 전체 닫기 준비 상태 평가 순서: [Core 모델의 닫기 준비 상태](../core-model.md#close_task)
- 응답 분기 동작과 커밋된 차단 결과: [`harness.close_task`](method-close-task.md)
- 공개 오류 경로: [`close_task` 차단 사유 매핑](errors.md#harnessclose_task-close-blockers)
- 활성 `CloseReadinessBlocker.category`, `ValidatorResult.status`, `ValidatorResult.severity`, `GuaranteeDisplay.level` 값: [API 값 집합](schema-value-sets.md)
- 보안 보장 의미: [보안](../security.md)

## 관련 담당 문서

- [API 코어 스키마](schema-core.md): `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`.
- [API 값 집합](schema-value-sets.md): 상태 필드가 쓰는 정확한 값.
- [API 메서드](methods.md)와 메서드 담당 문서: 이 스키마를 반환하는 메서드.
- [API 아티팩트 스키마](schema-artifacts.md): `ArtifactRef`.
- [API 판단 스키마](schema-judgment.md): `UserJudgmentCandidate`.
- [저장 효과](../storage-effects.md): 지속 저장과 상태 효과.
