# API 상태 스키마

이 문서는 기준 범위의 상태 형태 API 스키마를 담당합니다. `StateSummary`, `StateRecordRef`, API 데이터 형태의 생명주기 상태, 상태 관련 스냅샷, `ProjectContinuityRecord`, `ProjectContinuitySummary`, `ShapingReadiness`, `ChangeUnitEffectContract`, 그리고 `NextActionSummary`, `WriteCheckStateSummary`, `WriteCheckSummary`, `WriteCheckAttemptScope`, `EvidenceSummary`, `EvidenceObservation`, `GuardHealthSummary`, `CurrentCloseBasis`, `ResidualRisk`, `RiskAcceptanceCoverage`, `CloseReadinessBlocker`, `ValidatorResult`, `GuaranteeDisplay` 같은 표시 형태를 정의합니다.

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

상태 스키마는 API 데이터 형태만 설명합니다. 상태처럼 보이는 필드가 있다고 해서 응답 분기가 선택되거나 지속 저장, Core 전이, 재실행 행, `task_events`, 아티팩트 효과, `Write Check` 효과, `state_version` 증가가 생기지는 않습니다.

상태 보기는 계산된 상태를 정직하게 드러내야 합니다.
- `null` 또는 생략된 필드는 메서드가 값을 선택하지 않았거나, 값을 사용할 수 없거나, 담당 스키마가 부재를 명시적으로 허용한다는 뜻입니다. "계산했고 없음"을 암시하는 빈 값으로 바꾸면 안 됩니다.
- `close_blockers: []`나 `risk_acceptance_coverage: []` 같은 빈 배열은 관련 계산을 실행했고 항목이 없었다는 뜻입니다.
- 변경 결과와 `volicord.status` 상태 보기는 겹치는 스키마 영역에서 같은 현재 상태를 설명해야 합니다.
- 계산된 차단 사유는 공유 닫기 준비 상태 엔진과 같은 계산을 사용합니다. 메서드 담당 문서는 분기가 효과를 지속하는지만 결정합니다.

담당 문서 링크:
- 응답 분기 선택: [공통 응답 분기](schema-core.md#common-response)
- 메서드 동작과 효과: [API 메서드](methods.md)와 메서드 담당 문서

<a id="state-references"></a>
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
  effect_contract: ChangeUnitEffectContract | null
  baseline_ref: string | null
  shaping_readiness: ShapingReadiness | null
  pending_user_judgment_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  write_check_summary: WriteCheckStateSummary | null
  evidence_summary: EvidenceSummary | null
  close_state: string | null
  close_blockers: CloseReadinessBlocker[]
  guard_health: GuardHealthSummary | null
  guarantee_display: GuaranteeDisplay | null
```

의미:
- `StateSummary`는 상태 참조, 요약, 닫기 준비 상태 필드를 담는 간결한 응답 형태입니다.
- 메서드 include 플래그는 이 형태의 일부만 선택할 수 있습니다. 메서드 담당 문서가 어떤 상태 보기를 선택하지 않는다고 말하면 `evidence_summary`, `close_state`, `close_blockers`, `guard_health`, `guarantee_display` 같은 include 제어 필드는 null이나 빈 값으로 반환하지 않고 생략합니다. 반환된 빈 배열은 그 상태 보기를 계산했고 비어 있음을 뜻합니다.
- `mode`와 `close_state`는 값이 있을 때 제어 값 문자열입니다.
- `goal_summary`, `scope_summary`, `non_goals`, `acceptance_criteria`, `autonomy_boundary`는 자유 형식 표시 문자열입니다.
- `effect_contract`는 현재 적용 Change Unit의 선택적 추가 효과 계약입니다. `null`은 추가 Change Unit 효과 계약이 기록되어 있지 않다는 뜻입니다. 넓은 안전성이나 제한 없는 실행처럼 설명하면 안 됩니다.
- `baseline_ref`는 불투명 기준선 식별자입니다.
- `pending_user_judgment_refs`는 응답 보기에 관련된 현재 대기 판단을 나열합니다. 대기 판단은 `required_for` 대상, 판단 종류, `Task`, Change Unit, 영향받는 참조, 근거가 해당 작업과 호환될 때만 작업을 차단합니다.

의미하지 않는 것:
- `StateSummary` 필드가 있다는 사실만으로 메서드 커밋 여부가 정의되지 않습니다.

담당 문서 링크:
- `mode`와 `close_state` 값: [`Task` 생명주기 값](schema-value-sets.md#task-lifecycle-values)
- 커밋 결정 분기: [공통 응답 분기](schema-core.md#common-response)
- 메서드별 커밋 동작: [API 메서드](methods.md)가 안내하는 메서드 담당 문서

## Guard health summary

`GuardHealthSummary`는 메서드 담당 문서가 선택했을 때 닫기 준비 상태와 상태 조회 보기가 반환하는 간결한 guard 상태 보기입니다.

```yaml
GuardHealthSummary:
  guard_mode: string
  guard_installation_id: string | null
  guard_installation_status: string
  guard_hook_observed: boolean
  last_guard_observed_at: string | null
  last_guard_event_at: string | null
  prompt_capture_available: boolean
  mcp_connection_healthy: boolean
  mcp_connection_status: string | null
  unresolved_unrecorded_change_count: integer
  missing_or_stale_write_readiness: boolean
```

의미:
- `guard_mode`와 `guard_installation_status`는 제어 값 문자열입니다.
- `guard_installation_id`가 `null`이 아니면 불투명 guard 설치 식별자입니다.
- `guard_hook_observed`는 선택된 guard 설치에 대해 일치하는 호스트 guard hook 관찰이 기록되어 있는지를 보고합니다.
- `last_guard_observed_at`은 가장 최근의 일치하는 guard 설치 관찰 시각이며, 일치하는 관찰이 기록되어 있지 않으면 `null`입니다.
- `last_guard_event_at`은 상태 보기에 사용할 수 있는 최신 guard 이벤트 타임스탬프입니다. 사용할 수 있는 guard 이벤트가 없으면 `null`입니다.
- `prompt_capture_available`은 선택된 guarded 또는 managed 연결에서 prompt capture를 사용할 수 있는지 보고합니다. 프롬프트 텍스트는 포함하지 않습니다.
- `mcp_connection_healthy`와 `mcp_connection_status`는 추적되는 Agent Connection 확인 상태가 있을 때 그 상태를 요약합니다.
- `unresolved_unrecorded_change_count`는 해결되지 않은 미기록 Product Repository 변경 수입니다. 프롬프트 텍스트, 명령 텍스트, 경로 목록은 노출하지 않습니다.
- `missing_or_stale_write_readiness`는 guard 이벤트가 누락되었거나 오래된 쓰기 준비 상태를 감지했는지 보고합니다.

의미하지 않는 것:
- `GuardHealthSummary`는 제품 정확성, 테스트 충분성, OS 강제, 샌드박싱, 보안 격리, 최종 수락의 증거가 아닙니다.
- `active` guard 요약은 증거, 아티팩트 무결성, 사용자 소유 판단, `Write Check`, 최종 수락, 잔여 위험 수락 요구사항을 대체하지 않습니다.
- `mcp_only` 모드는 담당 문서가 정의한 설정이 guarded 또는 managed 동작을 선택하지 않는 한 협력형으로 남습니다.

담당 문서 링크:
- `guard_mode`와 `guard_installation_status` 값: [상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values)
- 닫기 준비 상태 guard 차단 사유와 메서드 로컬 코드: [`volicord.close_task`](method-close-task.md)
- Agent Connection 의미: [Agent Connection](../agent-connection.md)

<a id="project-continuity-shapes"></a>
## 프로젝트 연속성 형태

`ProjectContinuityRecord`는 오래 유지하는 프로젝트 수준 연속성 기록 하나의 전체 API 상태 형태입니다. `ProjectContinuitySummary`는 상태 조회 보기에 쓰는 간결한 형태입니다.

```yaml
ProjectContinuityRecord:
  continuity_record_id: string
  project_id: string
  source_task_id: string
  source_change_unit_id: string | null
  kind: string
  title: string
  summary: string
  rationale: string | null
  applies_to_paths: string[]
  applies_to_refs: StateRecordRef[]
  source_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
  status: string
  supersedes_refs: StateRecordRef[]
  review_triggers: string[]
  created_at: string
  updated_at: string

ProjectContinuitySummary:
  continuity_record_ref: StateRecordRef
  kind: string
  status: string
  title: string
  summary: string
  source_task_ref: StateRecordRef
  source_change_unit_ref: StateRecordRef | null
  review_triggers: string[]
```

의미:
- 프로젝트 연속성 기록은 원천 `Task`가 닫힌 뒤에도 유지해야 하는 결정, 의무, 알려진 한계, 수락된 잔여 위험, 제약 같은 프로젝트 수준 맥락을 보존합니다.
- `source_task_id`와 `source_change_unit_id`는 기록이 어디에서 비롯되었는지를 식별합니다. 원천 `Task`나 Change Unit을 다시 현재 상태로 만들지는 않습니다.
- `applies_to_paths`, `applies_to_refs`, `source_refs`, `artifact_refs`, `supersedes_refs`, `review_triggers`는 이후 검토를 위한 제한된 맥락입니다. 빈 배열은 그 필드에 항목이 없다는 뜻입니다.
- `ProjectContinuitySummary`는 메서드 담당 문서가 선택하는 읽기 보기이며, 전체 지속 기록이 아닙니다.

의미하지 않는 것:
- 프로젝트 연속성 기록은 현재 `Task` 권한, 증거, `Write Check`, 최종 수락, 닫기 준비 상태, 미래 닫기 근거의 잔여 위험 수락, 차단 사유 면제가 아닙니다.
- `status=active`는 그 연속성 기록이 살아 있는 프로젝트 맥락이라는 뜻입니다. 모든 `Task`에 현재 적용된다거나 원천 결정이 새 권한 확인에 충분하다는 뜻은 아닙니다.

담당 문서 링크:
- `kind`와 `status` 값: [프로젝트 연속성 값](schema-value-sets.md#project-continuity-values)
- 저장소 계열과 JSON 배치: [저장소 기록](../storage-records.md)
- 메서드별 생성 효과: [저장 효과](../storage-effects.md)

## `ChangeUnitEffectContract`

`ChangeUnitEffectContract`는 Change Unit에 기록되는 선택적 효과 경계 객체입니다.

```yaml
ChangeUnitEffectContract:
  allowed_effects: string[]
  forbidden_effects: string[]
  allowed_paths: string[]
  expected_outputs: string[]
  invariants: string[]
  evidence_expectations: string[]
  sensitive_action_expectations: string[]
```

의미:
- `allowed_effects`와 `forbidden_effects`는 현재 Change Unit이 Core 상태로 허용하거나 금지하는 효과를 분류합니다.
- `allowed_paths`는 값이 있을 때 제품 파일 쓰기를 더 좁히는 Product Repository 상대 경로 목록입니다.
- `expected_outputs`, `invariants`, `evidence_expectations`, `sensitive_action_expectations`는 구조화된 기대 문자열입니다. 워크플로 엔진을 만들지 않으면서 의도한 출력과 증거 경계를 사용자와 에이전트가 이해하도록 돕습니다.
- 빈 배열은 그 계약 부분이 추가 제한이나 기대를 더하지 않는다는 뜻입니다.

의미하지 않는 것:
- `ChangeUnitEffectContract`는 런타임 샌드박스, 명령 가로채기 장치, 네트워크 차단 장치, 운영체제 권한 체계, 개발 방법론 상태 기계가 아닙니다.
- 사용자 소유 판단, 민감 동작 승인, 증거, `Write Check`, 최종 수락, 닫기 준비 상태, 잔여 위험 수락을 대신하지 않습니다.

담당 문서 링크:
- 효과 값 문자열: [메서드 내부 값](schema-value-sets.md#method-local-values)
- Product Repository 경로 정규화: [런타임 경계](../runtime-boundaries.md#product-repository-api-path-normalization)
- 계약을 기록하는 메서드 동작: [`volicord.update_scope`](method-update-scope.md)
- 제품 파일 쓰기 경계를 적용하는 메서드 동작: [`volicord.prepare_write`](method-prepare-write.md)

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

WriteCheckStateSummary:
  status: string
  write_check_ref: StateRecordRef | null
  basis_state_version: integer | null
  intended_paths: string[]
  consumed_by_run_ref: StateRecordRef | null
  observation_refs: StateRecordRef[]
  guarantee_display: GuaranteeDisplay | null

WriteCheckSummary:
  write_check_ref: StateRecordRef
  status: string
  attempt_scope: WriteCheckAttemptScope
  basis_state_version: integer
  expires_at: string | null

WriteCheckAttemptScope:
  task_id: string
  change_unit_id: string
  intended_operation: string
  intended_paths: string[]
  product_file_write_intended: boolean
  sensitive_categories: string[]
  baseline_ref: string | null

WriteDecisionReason:
  category: string
  code: string
  message: string
  related_refs: StateRecordRef[]
```

의미:
- `NextActionSummary`는 기준 다음 행동 표시 형태입니다. 유효한 필드는 `action_kind`, `owner_method`, `label`, `blocking_question`, `required_refs`입니다.
- 오래된 `action` 또는 `reason` 필드를 쓰는 `next_actions` 항목은 유효한 `NextActionSummary`가 아닙니다.
- `WriteCheckStateSummary.status`와 `WriteCheckSummary.status`는 제어 값 문자열입니다.
- `WriteCheckStateSummary.consumed_by_run_ref`는 요약된 `Write Check`이 기록된 Run에 의해 소비되었을 때만 `null`이 아닙니다.
- `WriteCheckStateSummary.observation_refs`는 사용할 수 있을 때 그 소비 Run이 만든 증거 관찰 참조를 나열합니다. `Write Check`이 소비되지 않았거나 소비 Run이 관찰을 만들지 않았다면 비어 있습니다.
- `WriteCheckAttemptScope`는 `Write Check`이 포착하는 한 번의 시도 경계입니다.
- `WriteCheckAttemptScope`는 일반 쓰기 승인, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 포괄적 사용자 승인이 아닙니다.
- `WriteDecisionReason`은 `PrepareWriteResult.write_decision_reasons`에서 사용합니다.

`NextActionSummary` 필드 분류:

| 필드 | 분류 | 규칙 |
|---|---|---|
| `action_kind` | 제어되는 행동 범주 값. | [다음 행동 값](schema-value-sets.md#next-action-values)의 값 집합을 사용합니다. 메서드 이름 값이 아닙니다. |
| `owner_method` | 담당 메서드 이름 또는 `null`. | 지원되는 공개 메서드 하나가 다음 행동을 담당할 때 그 API 메서드를 이름 붙입니다. 단일 담당 메서드가 없으면 `null`을 사용합니다. |
| `label` | 자유 형식 표시 문자열. | 사람과 에이전트가 읽는 표시 문자열이며 기준 값이 아닙니다. |
| `blocking_question` | 자유 형식 표시 문자열 또는 `null`. | 행동을 진행하기 전에 풀어야 하는 질문입니다. 필요한 질문이 없으면 `null`을 사용합니다. |
| `required_refs` | `StateRecordRef[]`. | 다음 행동에 필요한 기록입니다. 필요한 참조가 없으면 `[]`를 사용합니다. |

`WriteCheckAttemptScope` 필드 분류:

| 필드 | 분류 | 규칙 |
|---|---|---|
| `task_id` | 불투명 식별자. | 포착된 시도 경계의 `Task`를 식별합니다. |
| `change_unit_id` | 불투명 식별자. | 포착된 시도 경계의 Change Unit을 식별합니다. |
| `intended_operation` | 자유 형식 의도 문자열. | 제어 값 집합을 만들지 않고 의도한 작업을 설명합니다. |
| `intended_paths` | 정규화된 Product Repository 경로 문자열. | API 수준 경로 정규화 뒤의 Product Repository 상대 경로입니다. |
| `product_file_write_intended` | Boolean. | 포착된 시도가 제품 파일 쓰기를 의도했는지 나타냅니다. |
| `sensitive_categories` | 불투명 민감 범주 분류 문자열. | 영향받는 메서드나 프로필 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 빠짐없는 공개 enum이 아닙니다. |
| `baseline_ref` | 불투명 기준선 식별자 또는 `null`. | 값이 있을 때 시도 경계에 포착된 기준선 식별자입니다. |

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
- `WriteCheckStateSummary.status`와 `WriteCheckSummary.status` 값: [메서드 내부 값](schema-value-sets.md#method-local-values)
- `WriteDecisionReason.category` 값: [상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values)
- `WriteDecisionReason.code` 값 집합 경계: [불투명 문자열과 메서드 범위 문자열 필드](schema-value-sets.md#opaque-and-method-scoped-string-fields)
- `WriteDecisionReason.code` 생성과 로컬 의미: [`volicord.prepare_write`](method-prepare-write.md)를 포함한 메서드 담당 문서
- `Write Check` 생성 동작: [`volicord.prepare_write`](method-prepare-write.md)
- `Write Check`의 제품 의미와 승인 경계: [Core 모델](../core-model.md)
- 공개 `ErrorCode` 값은 별도입니다: [API 오류 코드](error-codes.md)

<a id="evidence-and-run-snapshot-shapes"></a>
## 증거와 실행 기록 스냅샷 형태

```yaml
EvidenceSummary:
  status: string
  completion_policy: CompletionPolicy
  coverage_items: EvidenceCoverageItem[]
  artifact_refs: ArtifactRef[]
  observation_refs: StateRecordRef[]
  updated_by_run_ref: StateRecordRef | null

CompletionPolicy:
  evidence_required: boolean
  required_claims: string[]

EvidenceCoverageItem:
  claim: string
  required_for_close: boolean
  coverage_state: string
  provenance: EvidenceUpdateProvenance | null
  supporting_refs: StateRecordRef[]
  observation_refs: StateRecordRef[]
  supporting_artifact_refs: ArtifactRef[]
  gap_refs: StateRecordRef[]

EvidenceUpdateProvenance:
  source_kind: string
  assurance_level: string
  observed_at: string | null
  tool_name: string | null
  tool_invocation_id: string | null
  tool_metadata: object
  limitations: string[]

EvidenceObservation:
  observation_id: string
  project_id: string
  task_id: string
  change_unit_id: string | null
  run_ref: StateRecordRef | null
  claim: string
  source_kind: string
  assurance_level: string
  observed_by_actor_source: string | null
  tool_name: string | null
  tool_invocation_id: string | null
  tool_metadata: object
  input_refs: StateRecordRef[]
  output_artifact_refs: ArtifactRef[]
  limitations: string[]
  observed_at: string
  recorded_at: string

EvidenceObservationInput:
  claim: string
  source_kind: string
  assurance_level: string
  observed_by_actor_source: string | null
  tool_name: string | null
  tool_invocation_id: string | null
  tool_metadata: object
  input_refs: StateRecordRef[]
  output_artifact_refs: ArtifactRef[]
  limitations: string[]
  observed_at: string

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
- `EvidenceSummary.status`, `EvidenceCoverageItem.coverage_state`, `EvidenceUpdateProvenance.source_kind`, `EvidenceUpdateProvenance.assurance_level`, `EvidenceObservation.source_kind`, `EvidenceObservation.assurance_level`, `EvidenceObservationInput.source_kind`, `EvidenceObservationInput.assurance_level`, `RunSummary.kind`는 제어 값 문자열입니다.
- `CompletionPolicy.required_claims`, `EvidenceCoverageItem.claim`, `EvidenceObservation.claim`, `EvidenceObservationInput.claim`, `RunSummary.summary`는 자유 형식 주장 또는 표시 문자열입니다.
- `EvidenceCoverageItem.provenance`는 요청 입력에서 선택적으로 사용할 수 있으며, Core가 해당 `EvidenceObservation`을 만들거나 연결한 뒤 커밋된 증거 요약에서는 생략됩니다. 닫기와 관련된 주장을 `supported`로 갱신하려면 같은 주장에 대한 관찰 입력, 사용할 수 있는 관찰 참조, 또는 Core가 관찰을 만들 수 있게 하는 이 출처 객체가 필요합니다.
- `EvidenceSummary.observation_refs`와 `EvidenceCoverageItem.observation_refs`는 Core가 요약이나 주장과 관련지은 커밋된 증거 관찰에 대한 `StateRecordRef` 값을 나열합니다.
- `EvidenceObservation`은 하나의 보고되었거나 관찰된 증거 주장에 대한 지속 출처 기록입니다. 출처, 보장 수준, 관찰자 행위자 출처, 선택적 도구 메타데이터, 입력 참조, 출력 아티팩트 참조, 한계, 관찰 타임스탬프를 기록합니다.
- `EvidenceObservationInput`은 `volicord.record_run`이 받는 요청 측 형태입니다. Core는 커밋할 때 `observation_id`, 프로젝트와 `Task` 좌표, `run_ref`, `recorded_at`을 채웁니다.
- `observed_by_actor_source`는 값이 있으면 `ActorSource` 값이어야 합니다. 관찰 입력에서 null이면 Core가 확인된 호출 맥락에서 값을 채울 수 있습니다.
- `source_kind`와 `assurance_level`은 출처와 관찰 보장 수준을 설명합니다. 그 자체로 제품 정확성을 증명하거나, 사용자 권한을 부여하거나, 최종 수락을 만족하거나, 잔여 위험 수락을 만족하거나, `GuaranteeDisplay.level`을 높이지 않습니다.
- `user_observation`은 사용자 귀속 관찰을 기록하지만 최종 수락이나 다른 권한을 지니는 사용자 판단이 아닙니다.
- `external_tool`과 `external_tool_result`는 외부 도구 결과를 기록합니다. 관련 증거, 아티팩트, 닫기 준비 상태, 보안 담당 문서 없이는 제품 정확성 증명이 아닙니다.
- `unverified_claim`과 `unverified`는 확인된 관찰 없는 주장을 보존하며 그 자체로 충분한 증거가 아닙니다.
- `tool_metadata`는 설명용 메타데이터이며 권한, 승인, 저장 효과로 취급하면 안 됩니다.
- `ObservedChanges.changed_paths`는 경로 문자열입니다.
- `ObservedChanges.sensitive_categories`는 영향받는 메서드나 프로필 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 불투명 민감 범주 분류 문자열입니다.
- `ObservedChanges.baseline_ref`는 불투명 기준선 식별자입니다.

담당 문서 링크:
- `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md)
- 증거, `coverage_state`, 증거 관찰, 실행 종류 값: [상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values), [증거 관찰 값](schema-value-sets.md#evidence-observation-values), [메서드 내부 값](schema-value-sets.md#method-local-values)
- 증거 관찰 행위자 값: [행위자 값](schema-value-sets.md#actor-values)
- 증거 충분성의 의미: [Core 모델의 실행 기록과 증거의 권한](../core-model.md#9-evidence-and-run-authority)
- 메서드 동작: [API 메서드](methods.md)가 안내하는 메서드 담당 문서

<a id="close-readiness-and-validation-shapes"></a>
## 닫기 준비 상태와 검증 형태

```yaml
CurrentCloseBasis:
  close_basis_revision: integer
  scope_revision: integer
  task_id: string
  change_unit_id: string
  baseline_ref: string | null
  result_summary: string
  result_refs: StateRecordRef[]
  evidence_summary_ref: StateRecordRef | null
  residual_risks: ResidualRisk[]
  sensitive_categories: string[]
  sensitive_action_requirements: SensitiveActionRequirement[]
  recovery_constraints: string[]
  source_run_ref: StateRecordRef
  updated_at: string

SensitiveActionRequirement:
  action_kind: string
  normalized_paths: string[]
  sensitive_categories: string[]
  baseline_ref: string | null
  change_unit_id: string
  source_run_ref: StateRecordRef
  source_write_check_ref: StateRecordRef

ResidualRisk:
  risk_id: string
  summary: string
  consequence: string
  acceptance_required: boolean
  source_refs: StateRecordRef[]

RiskAcceptanceCoverage:
  risk_id: string
  accepted: boolean
  accepted_by_judgment_refs: StateRecordRef[]
  missing_reason: string | null

CloseReadinessBlocker:
  category: string
  code: string
  message: string
  can_resolve_in_chat: boolean
  terminal_action_required: boolean
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
- `CurrentCloseBasis`는 닫기 준비 상태 응답이 사용하는 현재 결과와 잔여 위험 상태입니다. 종료 닫기 요약이 아닙니다.
- `close_basis_revision`과 `scope_revision`은 호환성 확인을 위해 드러나는 내부 현재 상태 좌표입니다. 호출자가 선택하는 권한이 아닙니다.
- `ResidualRisk.risk_id`는 Core가 생성한 불투명 식별자입니다. `ResidualRisk.summary`와 `ResidualRisk.consequence`는 표시 문자열이며 텍스트 일치를 권한으로 만들지 않습니다.
- `result_refs`, `source_run_ref`, `source_refs`, `evidence_summary_ref`, `accepted_by_judgment_refs`는 `StateRecordRef`를 사용합니다.
- `sensitive_categories`는 영향받는 메서드나 프로필 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 불투명 민감 범주 분류 문자열입니다.
- `sensitive_action_requirements`는 커밋된 실행 기록과 소비된 `Write Check` 기록에서 Core가 파생한 닫기 요구사항입니다. 범주만 담은 호출자 입력은 이 요구사항을 만들거나 지울 수 없습니다.
- `recovery_constraints`와 `RiskAcceptanceCoverage.missing_reason`은 표시 문자열입니다. 현재 닫기 준비 상태 결과는 필요한 수락이 없으면 `acceptance_required`를 사용하고, 현재 잔여 위험 `risk_id` 값을 덮지 못하는 오래된 잔여 위험 수락이 있으면 `stale_acceptance`를 사용할 수 있습니다.
- `RiskAcceptanceCoverage`는 현재 잔여 위험 요구사항이 호환되는 판단으로 덮였는지를 보고합니다. 증거 충분성이나 최종 수락을 보고하지 않습니다.
- `CloseReadinessBlocker`는 닫기 차단 사유를 표현하는 데이터 형태입니다.
- `CloseReadinessBlocker.category`는 제어 값 문자열입니다.
- `CloseReadinessBlocker.code`는 담당 문서가 정의하는 차단 사유 코드입니다. 차단 사유 또는 메서드 담당 문서가 더 좁은 로컬 목록을 공개하지 않는 한 빠짐없는 전역 공개 enum이 아닙니다.
- `can_resolve_in_chat`은 메서드 담당 문서가 그 경로를 알고 있을 때 차단 사유를 채팅으로 매개되는 사용자 경로에서 해소할 수 있는지를 보고합니다.
- `terminal_action_required`는 다음 행동이 채팅 밖의 터미널, 호스트, 파일시스템, setup 동작을 필요로 하는지를 보고합니다.
- `CloseReadinessBlocker.message`, `ValidatorResult.message`, `GuaranteeDisplay.basis`는 자유 형식 표시 문자열입니다.
- `ValidatorResult.validator_id`는 값 집합 담당 문서가 지원되는 안정 값을 공개하기 전까지 보고용 라벨입니다.
- `ValidatorResult.status`, `ValidatorResult.severity`, `GuaranteeDisplay.level`은 제어 값 문자열입니다.

이 형태들은 닫기 준비 상태 의미, 응답 처리 경로, 지속 동작을 정의하지 않습니다.

닫기 근거 참조 규칙:
- `CurrentCloseBasis.result_refs`나 `ResidualRisk.source_refs`로 받아들일 수 있는 호출자 제공 닫기 평가 참조는 담당 문서가 다른 종류를 명시적으로 추가하지 않는 한 결과/증거 기록 종류인 `run`, `artifact`, `evidence_summary`, `change_unit`으로 제한됩니다.
- 담당 문서가 명시적으로 추가하지 않는 한 `project_state`, `write_check`, `user_judgment`, `blocker`, `task_event`, `task`는 호출자 제공 결과 참조가 아닙니다.
- 받아들인 모든 참조는 존재해야 하고 같은 프로젝트와 `Task`에 속해야 하며 Core가 정규화해야 합니다. Core는 호출자가 보낸 `state_version` 메타데이터를 권한으로 보존하지 않습니다.
- 닫기 증거에 쓰이는 아티팩트 참조는 `Task`에 연결되어 있고 `integrity_status=verified`여야 하며 [아티팩트 저장소](../storage-artifacts.md)에 따라 사용 시점의 현재 바이트 검증을 통과해야 합니다.
- 증거 참조는 현재 `Task` 증거 요약을 식별해야 합니다. 현재 닫기 근거 결과 참조로 쓰이는 실행 기록 참조는 현재 `Task`, 현재 적용 Change Unit, 현재 범위 리비전, 호환되는 기준선, 기록된 상태와 호환되는 기록된 현재 실행 기록을 식별해야 합니다. 이력 실행 기록은 현재 실행 기록이 그 `verified` 아티팩트나 증거를 명시적으로 재사용하고 그 재사용을 기록하지 않는 한 감사 기록입니다.
- Core는 기준 닫기 근거를 구성하면서 현재 실행 기록, 현재 Change Unit, 현재 EvidenceSummary 참조를 추가할 수 있습니다.

보장 표시 규칙:
- `GuaranteeDisplay`는 프로젝트 강제 프로필, 확인된 호출 맥락, 활성화된 강제 메커니즘, 지원되는 기준 범위에서 파생됩니다.
- `capability_refs`는 표시를 정당화하는 참조를 담는 구현 필드 이름입니다. 기준 연결 아키텍처에서는 사용할 수 있으면 호출 바인딩, Agent Connection, 관찰 사실을 인용해야 합니다.
- 협력형 전용 배포는 `detective`를 주장하면 안 됩니다.
- `detective`는 관찰 범위에 대한 지원되는 강제 또는 관찰 사실을 요구하며, 호스트 지침, 연결 모드, 생성된 텍스트만으로는 부족합니다.
- 별도 지원 관찰이 그 표시를 정당화하지 않는 한 협력적 `agent_report` Run이나 관찰을 `detective` 또는 외부 관찰로 표시하지 않습니다.

담당 문서 링크:
- 닫기 준비 상태 의미와 대체 금지 규칙: [Core 모델의 닫기 준비 상태](../core-model.md#close_task)
- 현재 닫기 근거 생성: [`volicord.record_run`](method-record-run.md)
- 판단 호환성과 수락된 위험 입력: [API 판단 스키마](schema-judgment.md)
- 응답 분기 동작, 닫기 준비 상태 평가 순서, 커밋된 차단 결과: [`volicord.close_task`](method-close-task.md)
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
