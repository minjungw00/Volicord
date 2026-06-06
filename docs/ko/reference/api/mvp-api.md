# 현재 MVP API

## 이 문서로 할 수 있는 일

현재 MVP의 활성 API surface를 확인할 때 이 참조를 사용합니다. 이 문서는 활성 method 목록과 method별 요청, 응답, 상태 효과, 저장소 담당 문서, 오류, 보안 경계를 담당합니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 저장소에는 하네스 런타임이나 서버 구현이 없습니다. 향후 API/schema 후보는 활성 API 참조가 아니라 [Later 후보 색인](../../later/index.md)에 둡니다.

## 핵심 생각

활성 MVP API는 사용자 작업 루프 하나를 위한 작은 local MCP surface입니다. 작업을 받아들이고, 상태를 보여 주고, 제품 파일 쓰기가 현재 Core 상태와 맞는지 확인하고, 실행과 증거 ref를 기록하고, 사용자 소유 판단을 묻고 기록하며, 활성 차단 사유가 허용할 때만 닫습니다.

이 API는 OS 권한, 임의 도구 sandboxing, 변조 방지 파일, 사전 도구 차단, 보안 격리를 제공하지 않습니다. `harness.prepare_write`는 협력형 하네스 기록/확인만 반환합니다.

## 현재 MVP method set

활성 method set은 정확히 다음과 같습니다.

```text
harness.intake
harness.status
harness.prepare_write
harness.record_run
harness.request_user_judgment
harness.record_user_judgment
harness.close_task
```

| Method | 활성 역할 |
|---|---|
| [`harness.intake`](#harnessintake) | 평소 사용자 작업을 시작, 재개, 분류합니다. |
| [`harness.status`](#harnessstatus) | 현재 상태 요약, 차단 사유, 대기 중인 판단, 증거 요약, 닫기 상태, 다음 안전한 행동을 반환합니다. |
| [`harness.prepare_write`](#harnessprepare_write) | 제안된 제품 쓰기를 현재 범위, 상태, 민감 동작 승인, baseline, 접점 역량과 비교합니다. |
| [`harness.record_run`](#harnessrecord_run) | shaping, direct, implementation 작업과 compact evidence/artifact ref를 기록합니다. |
| [`harness.request_user_judgment`](#harnessrequest_user_judgment) | 대기 중인 사용자 소유 판단 요청 하나를 만듭니다. |
| [`harness.record_user_judgment`](#harnessrecord_user_judgment) | 기존 pending `UserJudgment`에 대한 사용자의 답을 기록합니다. |
| [`harness.close_task`](#harnessclose_task) | 닫기 준비 상태를 확인하고, 차단 사유가 허용할 때만 close, cancel, supersede합니다. |

## 공통 request 규칙

모든 method는 [`ToolEnvelope`](schema-core.md#tool-envelope)와 [`ToolResponseBase`](schema-core.md#common-response)를 사용합니다. 상태를 바꾸는 method는 non-null `idempotency_key`와 current `expected_state_version`을 요구합니다. `harness.status`는 read-only이며 `expected_state_version: null`을 사용할 수 있습니다.

Method에 tool-specific `task_id`가 있으면 Core는 tool-specific `task_id`, `ToolEnvelope.task_id`, active Task 순서로 primary Task를 찾습니다. Task-scoped mutation은 `expected_state_version`을 `tasks.state_version`과 비교합니다. Resolved Task가 없는 project-scoped mutation은 `project_state.state_version`과 비교합니다.

`dry_run=true`는 기준 권한이 아닙니다. Diagnostic이나 would-change 결과를 반환할 수 있지만 current record, `task_events` row, artifact, consumable Write Authorization, evidence summary, close state, idempotency replay row를 만들지 않습니다.

Error code, primary error precedence, idempotency, stale-state behavior, close blocker ordering, 사용자 표시 오류 label은 [API Errors](errors.md)가 담당합니다. Shared schema와 활성 value set은 [API Schema Core](schema-core.md)가 담당합니다.

<a id="harnessintake"></a>

## `harness.intake`

- **담당:** Task 시작/재개/분류와 write-capable work의 초기 active scope boundary.
- **담당하지 않음:** 제품 쓰기, 증거 충분성, 사용자 판단 해결, Write Authorization, 최종 수락, 잔여 위험 수락, close.
- **호출 시점:** 평소 작업을 시작할 때, 또는 기존 active Task를 resume, supersede, reject해야 할 때.
- **Request:**

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

- **Response:**

```yaml
IntakeResponse:
  base: ToolResponseBase
  task_ref: StateRecordRef
  change_unit_ref: StateRecordRef | null
  state: StateSummary
  next_actions: NextActionSummary[]
```

- **상태 효과:** Committed non-dry-run call은 `tasks`를 만들거나 재개하고, `project_state.active_task_id`를 설정하며, write-capable `direct` 또는 `work`에 초기 `change_units` row를 만들고, blocker를 업데이트하고, event와 committed idempotency row를 만들 수 있습니다. Dry-run과 pre-commit failure는 이를 만들지 않습니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `NO_ACTIVE_TASK`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `project_state`, `tasks`, `change_units`, `blockers`, `task_events`, `tool_invocations`.
- **보안 경계:** Intake는 범위와 mode를 기록합니다. Local access, 민감 동작, 제품 쓰기, 더 강한 guarantee level을 승인하지 않습니다.

<a id="harnessstatus"></a>

## `harness.status`

- **담당:** Core 상태와 ref 위의 read-only current-position output.
- **담당하지 않음:** 상태 변경, 읽기용 보기 복구, 쓰기 호환성, 증거 생성, 사용자 판단 해결, 최종 수락, 잔여 위험 수락, 닫기.
- **호출 시점:** 다음 행동을 정하기 전, 상태를 바꾸는 call 이후, 또는 blocker, pending judgment, evidence summary, write-authority summary, close status, guarantee display가 필요할 때.
- **Request:**

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

- **Response:**

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

- **상태 효과:** 없습니다. `harness.status`는 `tool_invocations` replay row를 만들지 않습니다.
- **오류:** `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `NO_ACTIVE_TASK`, 요청한 readable view가 stale 또는 failed이면 `PROJECTION_STALE`.
- **저장소 담당 문서:** `project_state`, `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `evidence_summaries`, `artifacts`, `artifact_links`, `blockers`를 read-only로 읽습니다.
- **보안 경계:** 승격된 profile이 없으면 status는 현재 MVP `GuaranteeDisplay.level` 값인 `cooperative` 또는 `detective`만 표시합니다. `preventive`와 `isolated`는 schema와 security 담당 문서가 뒷받침하는 profile-gated 표시 값으로만 나타날 수 있습니다. 최신이 아닌 상태 text, chat, rendered view, cached summary는 권한 근거가 아닙니다.

<a id="harnessprepare_write"></a>

## `harness.prepare_write`

- **담당:** 협력형 쓰기 전 범위 확인과 proposed attempt가 compatible할 때 durable single-use Write Authorization.
- **담당하지 않음:** OS 권한, sandboxing, 변조 방지 enforcement, 사전 도구 차단, 사용자 판단 생성, 증거 충분성, run recording, close.
- **호출 시점:** 제품 파일 쓰기 또는 쓰기 가능한 동작 직전에, 현재 Task, Change Unit, baseline, 민감 동작 승인, 접점 역량과 맞는지 확인해야 할 때.
- **Request:**

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

- **Response:**

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

- **상태 효과:** Committed non-dry-run `decision=allowed`는 `write_authorizations.status=active` row 하나와 replay row를 만듭니다. Committed blocked response는 blocker를 업데이트할 수 있지만 consumable authorization을 만들면 안 됩니다. Dry-run과 pre-commit failure는 current record, authorization, blocker row, event, artifact, evidence summary, replay row를 만들지 않습니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `write_authorizations`, `blockers`, `tasks` 또는 `project_state` version clock, `task_events`, `tool_invocations`.
- **보안 경계:** `decision=allowed`는 이 attempt가 하네스 기록과 compatible하다는 뜻입니다. 운영체제가 incompatible write를 막거나 임의 도구가 격리된다는 뜻이 아닙니다.

<a id="harnessrecord_run"></a>

## `harness.record_run`

- **담당:** Run recording, compatible Write Authorization consumption, artifact registration, compact evidence-summary update, run-related blocker.
- **담당하지 않음:** 새 scope, 사용자 판단 해결, 최종 수락, 잔여 위험 수락, 별도 보증 기록, 닫기.
- **호출 시점:** Shaping work, direct answer/result, implementation work 이후. Product-write run은 `harness.prepare_write`가 반환한 compatible active Write Authorization을 제공해야 합니다.
- **Request:**

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

- **Response:**

```yaml
RecordRunResponse:
  base: ToolResponseBase
  run_summary: RunSummary
  registered_artifacts: ArtifactRef[]
  evidence_summary: EvidenceSummary | null
  blocker_refs: StateRecordRef[]
  state: StateSummary
```

- **상태 효과:** Compatible committed call은 `runs`, `artifacts`, `artifact_links`, `evidence_summaries`를 만들고, blocker를 업데이트하고, `write_authorizations.status=active`를 consume하고, event와 committed replay row를 만들 수 있습니다. Rejected call은 Run 생성, artifact 등록, evidence update, invalid authorization consumption을 하면 안 됩니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `SCOPE_VIOLATION`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `runs`, `write_authorizations`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, `tool_invocations`.
- **보안 경계:** Run은 surface가 관찰한 사실을 기록할 수 있습니다. Surface가 paths, commands, network, secret access, artifact capture, blocking, isolation fact를 관찰할 수 없으면 API는 그 사실을 verified로 표시하면 안 됩니다.

<a id="harnessrequest_user_judgment"></a>

## `harness.request_user_judgment`

- **담당:** 하나의 집중된 사용자 소유 판단에 대한 pending `UserJudgment` 생성.
- **담당하지 않음:** 사용자의 답, 민감 동작 승인, Write Authorization, evidence, final acceptance, residual-risk acceptance, close.
- **호출 시점:** 진행, write compatibility, acceptance, risk handling, close가 기존 기록에서 추론할 수 없는 사용자 소유 판단에 의존할 때.
- **Request:**

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

- **Response:**

```yaml
RequestUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  blocker_refs: StateRecordRef[]
  state: StateSummary
```

- **상태 효과:** Committed non-dry-run call은 pending `user_judgments` row 하나를 만들고, affected blocker를 link/update할 수 있으며, event와 replay row를 만듭니다. 다른 method가 반환한 candidate는 이 method가 commit되기 전까지 pending judgment가 아닙니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `user_judgments`, `blockers`, `task_events`, `tool_invocations`.
- **보안 경계:** 이 요청은 질문을 표시합니다. `harness.record_user_judgment`가 matching answer를 기록하기 전에는 permission을 부여하거나 gate를 만족하지 않습니다.

<a id="harnessrecord_user_judgment"></a>

## `harness.record_user_judgment`

- **담당:** 기존 pending `UserJudgment`의 resolve, reject, defer, block.
- **담당하지 않음:** Pending `judgment_kind`보다 넓은 결정, 제품 쓰기, evidence, Write Authorization, close, 명시적으로 묻지 않은 다른 judgment.
- **호출 시점:** 사용자가 특정 pending `UserJudgment`에 답한 뒤.
- **Request:**

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

- **Response:**

```yaml
RecordUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  updated_refs: StateRecordRef[]
  state: StateSummary
```

- **상태 효과:** Committed non-dry-run call은 `user_judgments.status`를 업데이트하고, answer를 기록하고, covered blocker와 affected state만 업데이트하며, event와 replay row를 만듭니다. 활성 MVP에서는 standalone accepted-risk row를 만들지 않습니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_UNRESOLVED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `user_judgments`, `blockers`, affected `tasks` 또는 `change_units`, `task_events`, `tool_invocations`.
- **보안 경계:** "go ahead"나 "looks good" 같은 넓은 말은 pending judgment가 그 kind를 명시적으로 묻고 recorded answer가 맞을 때만 product decision, sensitive-action approval, final acceptance, residual-risk acceptance, cancellation, QA waiver, scope expansion으로 작동합니다.

<a id="harnessclose_task"></a>

## `harness.close_task`

- **담당:** 활성 close-readiness check와 blocker가 허용할 때 terminal Task close/cancel/supersede.
- **담당하지 않음:** Evidence creation, user judgment creation, final acceptance creation, residual-risk acceptance creation, export, release handoff, projection/report freshness, active blocker 밖의 implementation validation.
- **호출 시점:** 작업을 닫을 수 있는지 확인해야 하거나, 사용자가 active Task를 complete, cancel, supersede하려 할 때.
- **Request:**

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: check | complete | cancel | supersede
  close_reason: completed_self_checked | completed_with_risk_accepted | cancelled | superseded | null
  superseding_task_id: string | null
  user_note: string | null
```

- **Response:**

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

- **상태 효과:** `intent=check`는 read-only입니다. Committed non-dry-run terminal close는 `tasks.lifecycle_phase`, `tasks.result`, `tasks.closed_at`, affected `change_units`, blockers, 필요한 경우 project active-task state, events, replay를 업데이트합니다. Blocked close는 blocker를 기록할 수 있지만 Task를 open으로 둬야 합니다. Dry-run은 close state나 replay row를 만들지 않습니다.
- **오류:** `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`.
- **저장소 담당 문서:** `tasks`, `change_units`, `blockers`, `runs`, `evidence_summaries`, `artifacts`, `artifact_links`, `user_judgments`, `task_events`, `tool_invocations`.
- **보안 경계:** Close는 Core 상태 전이이며 report가 아닙니다. Chat, status text, final acceptance alone, residual-risk acceptance alone, evidence alone, rendered view에서 추론하면 안 됩니다.
