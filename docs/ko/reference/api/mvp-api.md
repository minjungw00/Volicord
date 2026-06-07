# 현재 MVP API

## 이 문서로 할 수 있는 일

현재 MVP의 활성 API 표면을 확인할 때 이 참조를 사용합니다. 이 문서는 활성 메서드 목록과 메서드별 요청, 응답, 상태 효과, 저장소 담당 문서, 오류, 보안 경계를 담당합니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 저장소에는 하네스 런타임이나 서버 구현이 없습니다. 향후 API/schema 후보는 활성 API 참조가 아니라 [Later 후보 색인](../../later/index.md)에 둡니다. Storage DDL과 전체 공용 스키마 본문은 이 메서드 참조가 아니라 해당 담당 문서가 담당합니다.

## 핵심 생각

활성 MVP API는 사용자 작업 루프 하나를 위한 작은 로컬 MCP 접점입니다. 작업을 받아들이고, 상태를 보여 주고, 제품 파일 쓰기가 현재 Core 상태와 맞는지 확인하고, 실행과 증거 참조를 기록하고, 사용자 소유 판단을 묻고 기록하며, 활성 차단 사유가 허용할 때만 닫습니다.

이 API는 OS 권한, 임의 도구 샌드박스, 변조 방지 파일, 도구 실행 전 차단, 보안 격리를 제공하지 않습니다. `harness.prepare_write`는 협력형 하네스 기록/확인만 반환합니다.

## 현재 MVP 메서드 집합

활성 공개 메서드 집합은 정확히 아래 일곱 MCP 메서드입니다.

```text
harness.intake
harness.status
harness.prepare_write
harness.record_run
harness.request_user_judgment
harness.record_user_judgment
harness.close_task
```

| 메서드 | 활성 역할 |
|---|---|
| [`harness.intake`](#harnessintake) | 평소 사용자 작업을 시작, 재개, 분류합니다. |
| [`harness.status`](#harnessstatus) | 현재 상태 요약, 차단 사유, 대기 중인 판단, 증거 요약, 닫기 상태, 다음 안전한 행동을 반환합니다. |
| [`harness.prepare_write`](#harnessprepare_write) | 제안된 제품 쓰기를 현재 범위, 상태, 민감 동작 승인, baseline, 접점 역량과 비교합니다. |
| [`harness.record_run`](#harnessrecord_run) | shaping, direct, implementation 작업과 간결한 증거/아티팩트 참조를 기록합니다. |
| [`harness.request_user_judgment`](#harnessrequest_user_judgment) | 대기 중인 사용자 소유 판단 요청 하나를 만듭니다. |
| [`harness.record_user_judgment`](#harnessrecord_user_judgment) | 기존 pending `UserJudgment`에 대한 사용자의 답을 기록합니다. |
| [`harness.close_task`](#harnessclose_task) | 닫기 준비 상태를 확인하고, 차단 사유가 허용할 때만 close, cancel, supersede합니다. |

## 공통 요청 규칙

모든 메서드는 [`ToolEnvelope`](schema-core.md#tool-envelope)와 [`ToolResponseBase`](schema-core.md#common-response)를 사용합니다. 상태를 바꾸는 메서드는 non-null `idempotency_key`와 현재 `expected_state_version`을 요구합니다. `harness.status`는 read-only이며 `expected_state_version: null`을 사용할 수 있습니다.

메서드에 도구별 `task_id`가 있으면 Core는 도구별 `task_id`, `ToolEnvelope.task_id`, active Task 순서로 primary Task를 찾습니다. Task 범위 변경은 `expected_state_version`을 `tasks.state_version`과 비교합니다. 선택된 Task가 없는 project-scoped mutation은 `project_state.state_version`과 비교합니다.

`dry_run=true`는 기준 권한이 아닙니다. 진단이나 would-change 결과를 반환할 수 있지만 현재 기록, `task_events` 행, 아티팩트, 소비 가능한 Write Authorization, 증거 요약, 닫기 상태, 멱등 재실행 행을 만들지 않습니다.

오류 코드, 기본 오류 우선순위, 멱등성, stale-state 동작, 닫기 차단 사유 순서, 사용자 표시 오류 라벨은 [API Errors](errors.md)가 담당합니다. 공용 스키마와 활성 값 집합은 [API Schema Core](schema-core.md)가 담당합니다.

<a id="harnessintake"></a>

## `harness.intake`

- **담당:** Task 시작/재개/분류와 쓰기 가능한 작업의 초기 active scope boundary.
- **담당하지 않음:** 제품 쓰기, 증거 충분성, 사용자 판단 해결, Write Authorization, 최종 수락, 잔여 위험 수락, 닫기.
- **호출 시점:** 평소 작업을 시작할 때, 또는 기존 active Task를 resume, supersede, reject해야 할 때.
- **요청:**

```yaml
IntakeRequest:
  envelope: ToolEnvelope
  user_request: string
  requested_mode: advisor | direct | work | auto
  resume_policy: resume_active | create_new | supersede_active | reject_if_active
  acceptance_criteria: string[]
  constraints:
    allowed_paths: string[]
    non_goals: string[]
    sensitive_categories: string[]
  initial_context_refs: StateRecordRef[]
```

- **응답:**

```yaml
IntakeResponse:
  base: ToolResponseBase
  task_ref: StateRecordRef
  change_unit_ref: StateRecordRef | null
  state: StateSummary
  next_actions: NextActionSummary[]
```

- **상태 효과:** 커밋된 non-dry-run call은 `tasks`를 만들거나 재개하고, `project_state.active_task_id`를 설정하며, 쓰기 가능한 `direct` 또는 `work`에 초기 `change_units` row를 만들고, 차단 사유를 업데이트하고, event와 committed idempotency row를 만들 수 있습니다. Dry-run과 커밋 전 실패는 이를 만들지 않습니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `NO_ACTIVE_TASK`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `project_state`, `tasks`, `change_units`, `blockers`, `task_events`, `tool_invocations`.
- **보안 경계:** Intake는 범위와 `mode`를 기록합니다. 로컬 접근, 민감 동작, 제품 쓰기, 더 강한 guarantee level을 승인하지 않습니다.

<a id="harnessstatus"></a>

## `harness.status`

- **담당:** Core 상태와 참조를 읽어 만든 read-only 현재 위치 출력.
- **담당하지 않음:** 상태 변경, 읽기용 보기 복구, 쓰기 호환성, 증거 생성, 사용자 판단 해결, 최종 수락, 잔여 위험 수락, 닫기.
- **호출 시점:** 다음 행동을 정하기 전, 상태를 바꾸는 호출 이후, 또는 차단 사유, 대기 중인 판단, 증거 요약, 쓰기 권한 요약, 닫기 상태, 보장 표시가 필요할 때.
- **요청:**

```yaml
StatusRequest:
  envelope: ToolEnvelope
  include:
    task: boolean
    pending_user_judgments: boolean
    write_authority: boolean
    evidence: boolean
    close: boolean
    guarantees: boolean
```

- **응답:**

```yaml
StatusResponse:
  base: ToolResponseBase
  active_task: StateSummary | null
  status_card: string
  next_actions: NextActionSummary[]
  pending_user_judgments: StateRecordRef[]
  write_authority_summary: WriteAuthoritySummary | null
  evidence_summary: EvidenceSummary | null
  blocker_refs: StateRecordRef[]
  close_state: ready | blocked | closed | cancelled | superseded | none
  close_blockers: CloseBlocker[]
  guarantee_display: GuaranteeDisplay
```

- **상태 효과:** 없습니다. `harness.status`는 `tool_invocations` 재실행 행을 만들지 않습니다.
- **오류:** `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `NO_ACTIVE_TASK`, 요청한 읽기용 보기가 stale 또는 failed이면 `PROJECTION_STALE`.
- **저장소 담당 문서:** `project_state`, `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `evidence_summaries`, `artifacts`, `artifact_links`, `blockers`를 read-only로 읽습니다.
- **보안 경계:** 승격된 profile이 없으면 status는 현재 MVP `GuaranteeDisplay.level` 값인 `cooperative` 또는 `detective`만 표시합니다. `preventive`와 `isolated`는 schema와 security 담당 문서가 뒷받침하는 profile-gated 표시 값으로만 나타날 수 있습니다. 최신이 아닌 상태 텍스트, 대화, 렌더링된 보기, 캐시된 요약은 권한 근거가 아닙니다.

<a id="harnessprepare_write"></a>

## `harness.prepare_write`

- **담당:** 협력형 쓰기 전 범위 확인과 proposed attempt가 compatible할 때 오래 남는 1회용 Write Authorization.
- **담당하지 않음:** OS 권한, 샌드박스, 변조 방지 강제, 도구 실행 전 차단, 사용자 판단 생성, 증거 충분성, Run 기록, 닫기.
- **호출 시점:** 제품 파일 쓰기 또는 쓰기 가능한 동작 직전에, 현재 Task, Change Unit, baseline, 민감 동작 승인, 접점 역량과 맞는지 확인해야 할 때.
- **요청:**

```yaml
PrepareWriteRequest:
  envelope: ToolEnvelope
  task_id: string | null
  change_unit_id: string | null
  intended_operation: string
  intended_paths: string[]
  intended_tools: string[]
  intended_commands:
    - command: string
      command_class: string
      writes_product_files: boolean
  product_file_write_intended: boolean
  intended_network:
    - target: string
      direction: read | write
  intended_secret_scope:
    - secret_handle: string
      access_kind: read | write
  sensitive_categories: string[]
  baseline_ref: string | null
```

- **응답:**

```yaml
PrepareWriteResponse:
  base: ToolResponseBase
  decision: allowed | blocked | approval_required | decision_required | state_conflict
  state: StateSummary | null
  write_authorization_ref: StateRecordRef | null
  write_authorization: WriteAuthorizationSummary | null
  authorization_effect: none | would_create | created | returned
  active_user_judgment_refs: StateRecordRef[]
  blocked_reasons: CloseBlocker[]
  user_judgment_candidate: UserJudgmentCandidate | null
  guarantee_display: GuaranteeDisplay
```

- **상태 효과:** 커밋된 non-dry-run `decision=allowed`는 `write_authorizations.status=active` 행 하나와 재실행 행을 만듭니다. 커밋된 blocked 응답은 차단 사유를 업데이트할 수 있지만 소비 가능한 Write Authorization을 만들면 안 됩니다. Dry-run과 커밋 전 실패는 현재 기록, Write Authorization, blocker 행, 이벤트, 아티팩트, 증거 요약, 재실행 행을 만들지 않습니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `write_authorizations`, `blockers`, `tasks` 또는 `project_state` version clock, `task_events`, `tool_invocations`.
- **보안 경계:** `decision=allowed`는 이 attempt가 하네스 기록과 compatible하다는 뜻입니다. 운영체제가 incompatible write를 막거나 임의 도구가 격리된다는 뜻이 아닙니다.

<a id="harnessrecord_run"></a>

## `harness.record_run`

- **담당:** Run 기록, compatible Write Authorization 소비, artifact 등록, 간결한 evidence-summary 업데이트, Run 관련 차단 사유.
- **담당하지 않음:** 새 범위, 사용자 판단 해결, 최종 수락, 잔여 위험 수락, 별도 보증 기록, 닫기.
- **호출 시점:** Shaping work, direct answer/result, implementation work가 끝난 뒤. 제품 쓰기 Run은 `harness.prepare_write`가 반환한 compatible active Write Authorization을 제공해야 합니다.
- **요청:**

```yaml
RecordRunRequest:
  envelope: ToolEnvelope
  task_id: string | null
  change_unit_id: string | null
  kind: shaping_update | implementation | direct
  run_id: string | null
  baseline_ref: string | null
  write_authorization_id: string | null
  summary: string
  observed_changes: ObservedChanges
  artifact_inputs: ArtifactInput[]
  evidence_updates: EvidenceCoverageItem[]
```

- **응답:**

```yaml
RecordRunResponse:
  base: ToolResponseBase
  run_summary: RunSummary
  registered_artifacts: ArtifactRef[]
  evidence_summary: EvidenceSummary | null
  blocker_refs: StateRecordRef[]
  state: StateSummary
```

- **상태 효과:** 호환되는 커밋 호출은 `runs`, `artifacts`, `artifact_links`, `evidence_summaries`를 만들고, 차단 사유를 업데이트하고, `write_authorizations.status=active`를 소비하고, 이벤트와 커밋된 재실행 행을 만들 수 있습니다. 거부된 호출은 Run 생성, 아티팩트 등록, 증거 업데이트, 유효하지 않은 Write Authorization 소비를 하면 안 됩니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `SCOPE_VIOLATION`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `runs`, `write_authorizations`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, `tool_invocations`.
- **보안 경계:** Run은 접점이 관찰한 사실을 기록할 수 있습니다. 접점이 경로, 명령, 네트워크, 비밀 접근, 아티팩트 캡처, 차단, 격리 사실을 관찰할 수 없으면 API는 그 사실을 검증됨으로 표시하면 안 됩니다.

<a id="harnessrequest_user_judgment"></a>

## `harness.request_user_judgment`

- **담당:** 하나의 집중된 사용자 소유 판단에 대한 pending `UserJudgment` 생성.
- **담당하지 않음:** 사용자의 답, 민감 동작 승인, Write Authorization, 증거, 최종 수락, 잔여 위험 수락, 닫기.
- **호출 시점:** 진행, 쓰기 호환성, 수락, 위험 처리, 닫기가 기존 기록에서 추론할 수 없는 사용자 소유 판단에 의존할 때.
- **요청:**

```yaml
RequestUserJudgmentRequest:
  envelope: ToolEnvelope
  task_id: string | null
  change_unit_id: string | null
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short
  question: string
  options: UserJudgmentOption[]
  context: UserJudgmentContext
  affected_refs: StateRecordRef[]
  required_for: next_action | write | run | close | acceptance | risk
  expires_at: string | null
```

- **응답:**

```yaml
RequestUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  blocker_refs: StateRecordRef[]
  state: StateSummary
```

- **상태 효과:** 커밋된 non-dry-run 호출은 pending `user_judgments` 행 하나를 만들고, 영향을 받는 차단 사유를 연결하거나 업데이트할 수 있으며, 이벤트와 재실행 행을 만듭니다. 다른 메서드가 반환한 후보는 이 메서드가 커밋되기 전까지 대기 중인 판단이 아닙니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `user_judgments`, `blockers`, `task_events`, `tool_invocations`.
- **보안 경계:** 이 요청은 질문을 표시합니다. `harness.record_user_judgment`가 맞는 답변을 기록하기 전에는 권한을 부여하거나 gate를 만족하지 않습니다.

<a id="harnessrecord_user_judgment"></a>

## `harness.record_user_judgment`

- **담당:** 기존 pending `UserJudgment`를 해결, 거절, 유예, 차단 상태로 기록.
- **담당하지 않음:** Pending `judgment_kind`보다 넓은 결정, 제품 쓰기, 증거, Write Authorization, 닫기, 명시적으로 묻지 않은 다른 판단.
- **호출 시점:** 사용자가 특정 pending `UserJudgment`에 답한 뒤.
- **요청:**

```yaml
RecordUserJudgmentRequest:
  envelope: ToolEnvelope
  user_judgment_id: string
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
  selected_option_id: string
  answer: RecordUserJudgmentPayload
  note: string | null
  accepted_risks: AcceptedRiskInput[]
```

- **응답:**

```yaml
RecordUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  updated_refs: StateRecordRef[]
  state: StateSummary
```

- **상태 효과:** 커밋된 non-dry-run 호출은 `user_judgments.status`를 업데이트하고, 답변을 기록하고, 관련 차단 사유와 영향받은 상태만 업데이트하며, 이벤트와 재실행 행을 만듭니다. 활성 MVP에서는 독립 accepted-risk 행을 만들지 않습니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_UNRESOLVED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `user_judgments`, `blockers`, 영향을 받는 `tasks` 또는 `change_units`, `task_events`, `tool_invocations`.
- **보안 경계:** "go ahead"나 "looks good" 같은 넓은 말은 대기 중인 판단이 그 종류를 명시적으로 묻고 기록된 답변이 맞을 때만 제품 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 취소, QA 면제 판단, 범위 확장으로 작동합니다.

<a id="harnessclose_task"></a>

## `harness.close_task`

- **담당:** 활성 닫기 준비 상태 확인과 차단 사유가 허용할 때 terminal Task close/cancel/supersede.
- **담당하지 않음:** 증거 생성, 사용자 판단 생성, 최종 수락 생성, 잔여 위험 수락 생성, export, release handoff, Projection/report freshness, active blocker 밖의 implementation validation.
- **호출 시점:** 작업을 닫을 수 있는지 확인해야 하거나, 사용자가 active Task를 complete, cancel, supersede하려 할 때.
- **요청:**

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: check | complete | cancel | supersede
  close_reason: completed_self_checked | completed_with_risk_accepted | cancelled | superseded | null
  superseding_task_id: string | null
  user_note: string | null
```

- **응답:**

```yaml
CloseTaskResponse:
  base: ToolResponseBase
  close_state: ready | blocked | closed | cancelled | superseded
  state: StateSummary
  blockers: CloseBlocker[]
  evidence_summary: EvidenceSummary | null
  artifact_refs: ArtifactRef[]
  next_actions: NextActionSummary[]
```

- **상태 효과:** `intent=check`는 read-only입니다. 커밋된 non-dry-run 최종 닫기는 `tasks.lifecycle_phase`, `tasks.result`, `tasks.closed_at`, 영향을 받는 `change_units`, 차단 사유, 필요한 경우 project active-task 상태, 이벤트, 재실행을 업데이트합니다. 차단된 닫기는 차단 사유를 기록할 수 있지만 Task를 열린 상태로 둬야 합니다. Dry-run은 닫기 상태나 재실행 행을 만들지 않습니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `tasks`, `change_units`, `blockers`, `runs`, `evidence_summaries`, `artifacts`, `artifact_links`, `user_judgments`, `task_events`, `tool_invocations`.
- **보안 경계:** Close는 Core 상태 전이이며 보고서가 아닙니다. 대화, 상태 텍스트, 최종 수락만 있는 상태, 잔여 위험 수락만 있는 상태, 증거만 있는 상태, 렌더링된 보기에서 추론하면 안 됩니다.
