# MVP-1 API

## 이 문서로 할 수 있는 일

큰 future schema appendix를 읽지 않고 MVP-1에서 활성인 public API surface를 확인할 때 이 짧은 참조를 사용합니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 저장소에는 하네스 runtime이나 server 구현이 없습니다. 현재 저장소 단계와 구현 인계 상태는 [MVP 계획](../../build/mvp-plan.md#문서-수락-상태)가 담당합니다.

## 핵심 생각

MVP-1은 작은 local MCP surface만 노출합니다. 평소 작업 요청을 받아들이고, 현재 상태와 다음 안전한 행동을 보여 주고, 제안된 쓰기가 현재 범위에 맞는지 협력형으로 확인하고, 실행과 증거 ref를 기록하고, 사용자 소유 판단을 요청하고, 사용자의 답을 기록하고, 최소 계약이 허용할 때만 닫습니다.

MVP-1에서는 별도 `harness.next` method를 두지 않습니다. 다음 안전한 행동은 `harness.status.next_actions`에서 읽습니다. 별도 `harness.next`는 [Schema Later](schema-later.md#harnessnext)의 later/compatibility material입니다.

이 API는 OS-level blocking, arbitrary-tool sandboxing, tamper-proof file, pre-tool prevention을 주장하지 않습니다. `harness.prepare_write`는 Core state를 기준으로 하는 협력형 쓰기 전 범위 확인입니다. 반환되는 Write Authorization은 하네스 수준의 기록/확인이지 OS 권한, sandboxing, 변조 방지 enforcement, 사전 차단이 아닙니다. 더 강한 preventive 또는 isolated 주장은 관련 보안/connector 문서에서 owner-promoted profile과 증명이 필요합니다.

활성 MVP-1은 `surface_id=reference-local-mcp`인 registered reference `capability_profile` 하나를 사용합니다. 이 profile은 routing과 capability context이지 write authority가 아니며 Core gate를 대체하지 않습니다. Validator result, blocked reason, fallback behavior, guarantee display에 영향을 줍니다. Requested write나 guarantee claim이 unsupported profile field에 의존하면 API는 display를 낮추거나, `CAPABILITY_INSUFFICIENT` 또는 structured blocker를 반환하고, Write Authorization을 만들지 않아야 합니다.

Status output은 세 부분 모델을 따릅니다. `harness.status.status_card`는 사용자 상태 카드입니다. Agent 접점은 current status와 ref에서 `agent-context-packet`을 만들 수 있습니다. Core 상태가 유일한 운영 기준입니다. 상태 카드, next-action text, 렌더링된 template, 에이전트 패킷, Projection은 read-only view이며 오래된 view는 권한 근거가 아닙니다. 활성 사용자용 작은 출력은 정확히 `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`입니다. 활성 에이전트용 작은 출력은 정확히 `agent-context-packet`입니다. 상세 report surface는 후속/profile 범위에 남습니다.

## MVP-1 method set

| Method | MVP-1 역할 |
|---|---|
| [`harness.status`](#harnessstatus) | 현재 범위, 차단 사유, 대기 중인 판단, 증거 요약, 다음 행동, 닫기 준비 상태를 반환합니다. |
| [`harness.intake`](#harnessintake) | 평소 말로 들어온 작업을 시작하거나 이어가고, advice/read-only, small direct work, tracked work로 분류합니다. |
| [`harness.request_user_judgment`](#harnessrequest_user_judgment) | 집중된 사용자 판단 요청을 만듭니다. |
| [`harness.record_user_judgment`](#harnessrecord_user_judgment) | 대기 중인 사용자 판단에 대한 사용자의 답을 기록합니다. |
| [`harness.prepare_write`](#harnessprepare_write) | 제안된 제품 파일 쓰기를 현재 Task, 범위, baseline, 민감 동작 승인, 사용자 판단 coverage와 비교하는 쓰기 전 범위 확인을 실행합니다. |
| [`harness.record_run`](#harnessrecord_run) | shaping, implementation, direct run과 최소 artifact/evidence ref를 기록합니다. |
| [`harness.close_task`](#harnessclose_task) | 닫기 준비 상태를 확인하고, 차단 사유와 close intent가 허용하는 경우에만 complete, cancel, supersede합니다. |

## MVP-1이 아닌 것

다음 surface는 owner 문서가 승격하기 전까지 later/profile-gated입니다.

- 별도 `harness.next`
- `harness.launch_verify`
- `harness.record_eval`
- `harness.record_manual_qa`
- sensitive-action approval을 `user_judgment`로 다루는 범위를 넘어선 committed Approval record lifecycle
- full Evidence Manifest, detached verification 또는 detached Eval system, full Manual QA matrix, reconcile, export/recover suite, broad operations, detailed diagnostic projections

## 공통 request 규칙

모든 method는 [`ToolEnvelope`](schema-core.md#tool-envelope)와 [`ToolResponseBase`](schema-core.md#common-response)를 사용합니다. State-changing tool은 non-null `idempotency_key`와 current `expected_state_version`을 요구합니다. Read-only tool은 같은 envelope를 tracing에 사용할 수 있고 `expected_state_version`을 `null`로 둘 수 있습니다.

Method가 tool-specific `task_id`와 `ToolEnvelope.task_id`를 모두 가지면 tool-specific `task_id`가 첫 primary Task 후보입니다. Core는 tool-specific `task_id`, envelope `task_id`, active Task resolution 순서로 primary Task를 찾습니다. Primary Task가 없으면 그 mutation은 `expected_state_version`과 `ToolResponseBase.state_version`에 대해 project-scoped mutation입니다.

MVP-1 request validator는 [Schema Core](schema-core.md#stage-specific-active-value-sets)의 활성 schema block과 value-set summary를 사용합니다. Later enum value와 extension branch는 [Schema Later](schema-later.md)에 따로 정의되며, 활성 MVP-1 validator에서 valid하지 않습니다.

Error code, MVP-1 status/error condition name, 사용자 표시 문구 pattern, primary error precedence, idempotency replay, stale-state behavior는 [Errors](errors.md)가 담당합니다. Guarantee level의 보안 의미는 [보안 참조: 정직한 guarantee display](../security.md#정직한-guarantee-display)가 담당합니다. 모든 state-changing tool에서 `dry_run=true`는 기준 권한이 아닙니다. Validation diagnostic 또는 would-change summary를 반환할 수 있지만 current record, `task_events` row, artifact, consumable Write Authorization, projection job, idempotency replay row를 만들지 않습니다.

## 활성 MVP 전이 매트릭스

이 매트릭스는 활성 MVP public method를 Core 소유권, storage side effect, replay behavior, dry-run behavior, public error와 연결합니다. 자세한 request/response schema는 아래의 각 method section과 [Schema Core](schema-core.md)에 있습니다. 자세한 lifecycle 의미는 [Core Model](../core-model.md)이 담당합니다. 물리적 persistence는 [Storage](../storage.md)가 담당합니다. 어떤 method 단락이 활성 MVP path보다 넓게 읽힌다면 이 매트릭스를 기준으로 해석합니다.

이 section에서 "committed"는 `dry_run=false`이고 primary `ToolError`가 없으며 Core가 state mutation을 받아들였다는 뜻입니다. Blocked decision도 아래 row-level side effect가 blocker storage를 명시적으로 허용하면 committed response일 수 있습니다. "Failure"는 validation failure, Core/MCP unavailable, stale state, same-key/different-hash replay, 또는 다른 pre-commit error를 뜻합니다. Failure는 method row가 committed violation/audit 예외를 명시하지 않는 한 기준 상태를 만들지 않습니다.

State-changing method는 `(project_id, tool_name, idempotency_key)` scope의 committed idempotency row를 `tool_invocations`에 사용합니다. Stored `request_hash`는 [Errors: Idempotency](errors.md#idempotency)가 정의하는 canonical request hash입니다. 같은 key와 같은 hash는 새 freshness check나 side effect 전에 original committed response를 반환합니다. 같은 key와 다른 hash는 `STATE_CONFLICT`를 반환합니다. Dry-run call과 pre-commit failure는 replay row를 만들거나 업데이트하지 않고 key를 예약하지도 않습니다. `harness.status`는 read-only이며 committed replay에 참여하지 않습니다.

`UserJudgmentCandidate`는 상태를 바꾸지 않는 candidate/presentation material입니다. `StateRecordRef`가 없고 gate를 충족하지 않습니다. Committed `harness.request_user_judgment`가 pending `user_judgments` row를 만듭니다. Committed `harness.record_user_judgment`가 사용자의 답을 기록합니다. Candidate나 "go ahead" 같은 넓은 prose는 pending `judgment_kind`, affected object, scope, recorded value가 맞지 않는 한 민감 동작 승인, 최종 수락, 잔여 위험 수락, QA 면제, 검증 위험 수락, 취소, scope change, evidence, close, Write Authorization을 만들 수 없습니다.

| Method | Request input | Primary state owner | State version 확인 기준 | Idempotency replay 기준 | Related error codes |
|---|---|---|---|---|---|
| `harness.intake` | `IntakeRequest`: envelope, `user_request`, `requested_mode`, `resume_policy`, `acceptance_criteria`, constraints, `initial_context_refs`. | Core가 소유한 Task와 scope state: `project_state`, `tasks`, write-capable work가 시작될 때 initial `change_units`. | Existing 또는 resumed Task이면 `tasks.state_version`. Resolved primary Task 없이 만들면 `project_state.state_version`. | Committed non-dry-run intake의 `tool_invocations` row. Replay는 같은 `task_id`, resume/create/supersede decision, `change_unit_id`를 반환합니다. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `NO_ACTIVE_TASK`, `VALIDATOR_FAILED`. |
| `harness.status` | `StatusRequest`: envelope와 `include` flags. | 없음. Current Core row에서 만든 read-only derived view입니다. | Mutation check가 없습니다. `expected_state_version`은 `null`일 수 있습니다. Supplied readable context가 stale이면 repair하지 않고 보고합니다. | 없음. `tool_invocations` row를 만들지 않습니다. | `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `NO_ACTIVE_TASK`, requested readable view가 stale 또는 failed이면 `PROJECTION_STALE`. |
| `harness.prepare_write` | `PrepareWriteRequest`: envelope, Task/Change Unit, intended operation, paths, tools, commands/classes, product-file-write intent, network, secret scope, sensitive categories, `baseline_ref`. | Core pre-write compatibility state: allowed committed attempt의 `write_authorizations`, committed write blocker의 `blockers`, input으로 쓰는 Task/Change Unit scope record. | Resolved primary Task이면 `tasks.state_version`. Primary Task가 없으면 `project_state.state_version`. Resulting `WriteAuthorization.basis_state_version`은 compatibility basis이며 response state version과 반드시 같지는 않습니다. | Committed non-dry-run response의 `tool_invocations` row. Committed `decision=allowed`를 exact replay하면 original `write_authorization_ref`를 `authorization_effect=returned`로 반환하며 두 번째 authorization을 만들지 않습니다. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `DECISION_REQUIRED`, `AUTONOMY_BOUNDARY_EXCEEDED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `VALIDATOR_FAILED`. |
| `harness.record_run` | `RecordRunRequest`: envelope, `kind`, Task/Change Unit, optional caller `run_id`, `baseline_ref`, `write_authorization_id`, summary, `artifact_inputs`, matching payload branch. | Core run/evidence state: `runs`, compatible `write_authorizations`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`. | Resolved primary Task이면 `tasks.state_version`. Primary Task가 없으면 `project_state.state_version`. Product-write compatibility는 stored `WriteAuthorization.basis_state_version`과 full `attempt_scope_json`도 current scope, observed facts와 비교합니다. | Committed non-dry-run response의 `tool_invocations` row. Exact replay는 authorization consumption, Run creation, artifact registration, evidence/blocker update, event append, projection enqueue 전에 original Run/evidence response를 반환합니다. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `NO_ACTIVE_CHANGE_UNIT`, `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID`, `SCOPE_VIOLATION`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `BASELINE_STALE`, `ARTIFACT_MISSING`, `EVIDENCE_INSUFFICIENT`, `VALIDATOR_FAILED`. |
| `harness.request_user_judgment` | `RequestUserJudgmentRequest`: envelope, Task/Change Unit, `judgment_kind`, `presentation`, context, question, user/agent boundary text, affected scope/gates/criteria, payload, expiry. | Core user-judgment state: pending `user_judgments`와 affected `blockers`. | Resolved primary Task이면 `tasks.state_version`. Primary Task가 없으면 `project_state.state_version`. | Committed non-dry-run judgment request의 `tool_invocations` row. Replay는 같은 `user_judgment_ref`와 presentation summary를 반환합니다. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `CAPABILITY_INSUFFICIENT`, `VALIDATOR_FAILED`. |
| `harness.record_user_judgment` | `RecordUserJudgmentRequest`: envelope, `user_judgment_id`, matching `judgment_kind`, selected option, judgment payload, note, optional waiver reason, `accepted_risks`. | Core user-judgment state: resolved `user_judgments`, affected `blockers`, stored pending judgment가 명시적으로 cover하는 affected Task/Change Unit decision state. | Stored `user_judgment`를 소유한 Task의 `tasks.state_version`. Task owner가 없으면 `project_state.state_version`. | Committed non-dry-run answer의 `tool_invocations` row. Replay는 같은 resolved judgment response를 반환하며 다시 resolve하거나 accepted-risk ref를 중복하거나 event를 다시 append하면 안 됩니다. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_UNRESOLVED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`. |
| `harness.close_task` | `CloseTaskRequest`: envelope, `task_id`, close `intent`, requested close reason, user note, optional superseding Task. | Core close state: terminal/open `tasks`, current `blockers`, active scope, Run, user judgment, evidence, artifact, final acceptance, residual-risk state에서 파생한 close result. | Target Task의 `tasks.state_version`. | Committed non-dry-run close attempt의 `tool_invocations` row. Successful terminal close를 exact replay하면 같은 terminal response를 반환합니다. Conflicting intent 또는 changed payload는 `STATE_CONFLICT`를 반환합니다. | `VALIDATION_FAILED`, `STATE_CONFLICT`, `NO_ACTIVE_TASK`, `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE`, `CAPABILITY_INSUFFICIENT`, `MCP_UNAVAILABLE`, `LOCAL_ACCESS_MISMATCH`, `VALIDATOR_FAILED`. |

| Method | Rows created | Rows updated | Events appended | Response refs returned | Failure에서 금지되는 side effects | Dry-run에서 금지되는 side effects | 영향받는 close/status blockers |
|---|---|---|---|---|---|---|---|
| `harness.intake` | New `tasks`; write-capable `direct` 또는 `work`의 initial `change_units`; committed initial blocker가 있을 때만 `blockers`; committed replay용 `tool_invocations`. | `project_state.active_task_id` / `project_state.state_version`; resume 또는 supersede 때 existing `tasks`와 `change_units`. | Committed mutation의 Task create/resume/supersede, initial scope/blocker event. | `task_id`, `change_unit_id`, `state`, `next_action`. | Task, Change Unit, blocker, project active-task update, event, artifact, Write Authorization, evidence summary, projection job, replay row를 만들지 않습니다. | Task, Change Unit, blocker, event, state-version advance, projection job, replay row를 만들지 않습니다. | `harness.status`에 보이는 active-task, initial-scope, initial-question blocker를 만들거나 resolve할 수 있습니다. Evidence, acceptance, residual-risk acceptance, close readiness는 만들지 않습니다. |
| `harness.status` | 없음. | 없음. | 없음. | Existing ref만 반환합니다. Pending/active `user_judgments`, evidence refs, blocker refs, write-authority summary refs, residual-risk refs, requested source state refs입니다. | 어떤 mutation도 없습니다. | Read-only behavior와 같습니다. 어떤 mutation도 없습니다. | Current blockers, pending judgments, evidence gaps, guarantee/capability conditions, close-readiness blockers를 반환하지만 바꾸지 않습니다. |
| `harness.prepare_write` | Committed `dry_run=false`이고 `decision=allowed`일 때만 `write_authorizations.status=active`; Core가 non-error blocked decision을 commit하면 `blockers`; committed replay용 `tool_invocations`. | `tasks.state_version` 또는 `project_state.state_version`; affected `blockers`; Core가 affected scope에서 stale로 표시할 때만 older active `write_authorizations`. | Committed decision, blocker, authorization creation/staling event. | `write_authorization_ref`, `write_authorization`, `active_user_judgment_refs`, `blocked_reasons.related_error`, non-mutating `user_judgment_candidate`, `state`, `baseline_ref`. | Validation/MCP/state conflict/pre-commit failure에서는 Write Authorization, blocker row, task event, artifact, evidence summary, projection job, state-version advance, replay row를 만들지 않습니다. Committed blocked decision도 consumable authorization을 만들 수 없습니다. | Write Authorization, blocker/current record, task event, artifact, evidence summary, projection job, state-version advance, replay row를 만들지 않습니다. `authorization_effect=would_create`는 candidate-only입니다. | Write compatibility, missing scope, sensitive action, user judgment, Autonomy Boundary, baseline, capability, design-policy blocker를 열거나 resolve할 수 있습니다. 이 blocker들은 owner record를 통해서만 status와 이후 close에 영향을 줍니다. |
| `harness.record_run` | Compatible committed Run이면 `runs`, 허용된 `artifacts`, `artifact_links`, 없을 때 `evidence_summaries`, recorded gap의 `blockers`, `tool_invocations`. Explicit violation/audit recording은 Core가 after-the-fact observed behavior를 의도적으로 기록할 때만 `runs.status=violation`과 blocker/event row를 만들 수 있습니다. Evidence나 close를 충족하지 않습니다. | Compatible product-write Run은 `write_authorizations.status=active`를 `status=consumed`와 `consumed_by_run_id`로 소비합니다. `evidence_summaries`, `blockers`, artifact availability, affected state version을 업데이트합니다. | Run recording, authorization consumption, artifact/evidence update, blocker/gate update, explicit violation/audit event. | `run_id`, `write_authorization_ref`, `evidence_ref`, `run_summary_ref`, `direct_result_ref`, `registered_artifacts`, `state`. | Pre-commit rejection에서는 Run row, artifact, artifact link, evidence summary, authorization consumption, blocker/gate update, task event, projection job, state-version advance, replay row를 만들지 않습니다. Invalid authorization은 consumed로 표시하면 안 됩니다. | Authorization consumption, Run, artifact, artifact link, evidence summary, blocker/gate update, task event, projection job, state-version advance, replay row를 만들지 않습니다. | Evidence sufficiency, artifact availability, open-run, scope/authorization, capability, recovery blocker를 업데이트합니다. Rejected/invalid write-capable Run은 evidence, final acceptance, residual-risk acceptance, close readiness, later/profile QA 또는 verification requirement를 충족할 수 없습니다. |
| `harness.request_user_judgment` | Pending `user_judgments`; Core가 blocker/request linkage를 commit할 때만 affected `blockers`; committed replay용 `tool_invocations`. | Affected `blockers`와 state version. Request 자체는 Approval, Write Authorization, evidence, acceptance, residual-risk record를 resolve하지 않습니다. | User judgment requested 및 blocker/request-link event. | `user_judgment_ref`, `user_judgment`, `state`; minimum MVP-1에서 `approval_id=null`, `reconcile_item_id=null`. | Pending `user_judgment`, blocker update, Approval, reconcile item, Write Authorization, artifact, evidence summary, acceptance, residual-risk acceptance, close, event, replay row를 만들지 않습니다. | Pending `user_judgment`, blocker update, event, state-version advance, replay row를 만들지 않습니다. Returned presentation이 있더라도 candidate-only이며 committed `StateRecordRef`가 없습니다. | Product, technical, scope, sensitive-action, final-acceptance, residual-risk-acceptance, cancellation blocker를 열거나 계속 보이게 합니다. QA-waiver와 verification-risk blocker는 해당 owner path가 active일 때만 쓰는 later/profile path입니다. Request는 어떤 blocker도 resolve하지 않습니다. |
| `harness.record_user_judgment` | 활성 MVP에는 standalone accepted-risk row가 없습니다. Committed replay용 `tool_invocations`만 만듭니다. | `user_judgments.status`, resolution field, affected `blockers`, 명시적으로 cover된 affected Task/Change Unit decision state, affected state version. Sensitive-action approval은 resolved `user_judgment`를 통해서만 permission을 업데이트합니다. | User judgment resolved/deferred/rejected/blocked/superseded 및 affected blocker/gate recompute event. | `user_judgment_ref`, resolved `user_judgment`, `updated_records`, `accepted_risk_refs`, `state`. | Judgment resolution, blocker update, scope/task update, sensitive-action permission, waiver, final acceptance, residual-risk acceptance, close, event, state-version advance, replay row를 만들지 않습니다. | Judgment resolution, blocker update, permission, waiver, final acceptance, residual-risk acceptance, close, event, state-version advance, replay row를 만들지 않습니다. | 정확한 pending `judgment_kind`에 대해서만 blocker를 resolve하거나 계속 남길 수 있습니다. Final acceptance와 residual-risk acceptance는 분리됩니다. Sensitive approval은 product/technical/scope decision이나 Write Authorization을 대신하지 않습니다. |
| `harness.close_task` | Core가 close blocker를 기록하는 committed blocked close attempt에서는 `blockers`; committed replay용 `tool_invocations`. 별도 `close_readiness` row는 없습니다. | Successful close는 `tasks.lifecycle_phase`, `tasks.result`, `tasks.closed_at`, 필요하면 `project_state.active_task_id`, affected `change_units`, affected `blockers`, state version을 업데이트합니다. Blocked close는 Task를 open 상태로 둡니다. | Close attempt, blocker update, cancellation/supersession, successful close event. Active MVP는 durable projection job storage를 요구하지 않습니다. | `state`, structured `blockers.related_refs`, `evidence_summary`, `residual_risk_state`, `acceptance_state.accepted_by_ref`, `artifact_refs`; later/profile owner가 report를 활성화하지 않으면 `final_report_refs`는 보통 `[]`입니다. | Validation/MCP/state conflict/pre-commit failure에서는 terminal Task update, blocker row, event, projection job, close record, state-version advance, replay row를 만들지 않습니다. Committed blocked close도 Task를 terminal로 표시하면 안 됩니다. | Terminal Task update, blocker row, event, projection job, close record, state-version advance, replay row를 만들지 않습니다. Close result는 would-close diagnostic일 뿐입니다. | Active Task state, open Run, scope, user judgment, sensitive-action permission, active design policy, evidence sufficiency, artifact availability, final acceptance, residual-risk visibility, residual-risk acceptance, cancellation, supersession blocker를 읽고 기록할 수 있습니다. Verification, Manual QA, projection/report freshness, export, operations blocker는 later/profile-only입니다. |

<a id="harnessintake"></a>

## `harness.intake`

작업을 시작하거나, 분류하거나, 이어갈 때 이 method를 사용합니다.

Stage meaning: 내부 엔지니어링 점검에는 필요하지 않습니다. 내부 점검은 owner-valid setup path를 사용할 수 있습니다. MVP-1에서는 평소 말로 시작/이어가기 behavior가 active입니다. MVP-1 요구사항 구체화는 Task, Change Unit, user judgment 경계로 지속됩니다. Committed Shared Design record, full design-support routing, broad planning workflow는 명시적으로 승격되기 전까지 later material입니다.

Allowed actors: `user`, `lead_agent`, `operator`.

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

IntakeResponse:
  base: ToolResponseBase
  task_id: string
  created: boolean
  resumed: boolean
  state: StateSummary
  next_action: string
  change_unit_id: string | null
```

Core는 Task를 만들거나 이어가고, work mode를 설정하며, write-capable direct 또는 tracked work에 initial scoped boundary를 만들 수 있습니다. Idempotent replay는 같은 Task/resume 결정을 반환하고, 같은 key에 다른 payload를 쓰면 `STATE_CONFLICT`를 반환합니다.

Committed `dry_run=false` intake만 `project_state`, `tasks`, `change_units`, `blockers`, `task_events`, `tool_invocations`를 변경합니다. `dry_run=true`는 classification, would-create/would-resume outcome, next action을 반환할 수 있지만 Task, Change Unit, blocker, event, replay row, evidence, artifact, Write Authorization, projection job, acceptance, residual-risk acceptance, close state를 만들지 않습니다.

<a id="harnessstatus"></a>

## `harness.status`

현재 어디에 있고, 무엇이 막고 있고, 다음 안전한 행동이 무엇인지 답할 때 이 method를 사용합니다.

Stage meaning: 내부 엔지니어링 점검에서는 minimal status/blocker output이 active입니다. MVP-1에서는 현재 위치, 대기 중인 사용자 판단, 증거 요약, 닫기 준비 상태, `next_actions`가 active입니다.

Allowed actors: `user`, `lead_agent`, `evaluator`, `operator`.

```yaml
StatusRequest:
  envelope: ToolEnvelope
  include:
    task: boolean
    gates: boolean
    projections: boolean
    pending_user_judgments: boolean
    guarantees: boolean
    user_judgments: boolean
    autonomy_boundary: boolean
    write_authority: boolean
    residual_risk: boolean

StatusResponse:
  base: ToolResponseBase
  active_task: StateSummary | null
  status_card: string
  next_actions: NextActionSummary[]
  pending_user_judgments: StateRecordRef[]
  active_user_judgment_refs: StateRecordRef[]
  autonomy_boundary_summary: AutonomyBoundarySummary | null
  write_authority_summary: WriteAuthoritySummary | null
  residual_risk_summary: ResidualRiskSummary | null
  evidence_summary: EvidenceSummary | null
  evidence_refs: StateRecordRef[]
  blocker_refs: StateRecordRef[]
  projection_freshness:
    status: current | stale | failed | unknown
    stale_refs: StateRecordRef[]
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
```

`status_card`는 current Core state와 ref에서 만든 짧은 읽기용 보기입니다. Compact하게 유지하고 source/freshness 정보를 보여줘야 합니다. 전체 schema, DDL, history, template, projection body, artifact body, log, future catalog를 넣으면 안 됩니다. Core 상태가 아니며 민감 동작 승인, 최종 수락, 잔여 위험 수락, 증거, 닫기 준비 상태, Write Authorization, close를 만들 수 없습니다.

Core가 답할 수 있으면 `StatusResponse`는 항상 `guarantee_display.level`을 보여줘야 합니다. `include.guarantees=false`는 optional note나 확장 capability detail을 줄일 수 있지만, active guarantee level을 숨기면 안 됩니다. Core가 답할 수 없으면 authoritative state mutation claim을 할 수 없다는 분명한 `MCP_UNAVAILABLE`/capability condition을 보여줘야 합니다.

`next_actions`가 MVP-1의 다음 안전한 행동 surface입니다. 사용자에게는 가장 작은 useful next action이나 unblocker를 쉬운 말로 보여 주고, exact enum value는 secondary detail로 둡니다.

`evidence_summary`는 Core가 소유한 compact MVP-1 evidence summary입니다. `evidence_refs`는 active minimal evidence coverage ref를 담습니다. 보통 `StateRecordRef.record_kind=evidence_summary`를 사용하며, nested schema가 허용하는 곳에서는 artifact ref도 함께 둡니다. 이 field들은 full Evidence Manifest table이나 report가 아니며, verification, 수동 QA, 최종 수락, 잔여 위험 수락, close를 대신하지 않습니다.

Status가 Core에 닿지 못하거나, stale state를 보고하거나, unsupported surface를 이름 붙이거나, 범위 밖 작업, 필요한 사용자 판단, 부족한 증거, 닫기 차단 사유, 남은 잔여 위험 같은 blocker를 보여줄 때는 [Errors: MVP-1 guarantee와 상태/error taxonomy](errors.md#mvp-1-guarantee-and-status-taxonomy)의 canonical condition 동작을 사용합니다.

MVP-1 active `NextActionSummary.action_kind` values:

```text
ask_user | prepare_write | implement | request_acceptance | close_task | idle
```

Verification, Eval, Manual QA, reconcile, export/recover, operations next-action kind는 later/profile-gated입니다.

Status는 read-only입니다. State를 만들거나, 제품 파일 쓰기를 compatible하게 만들거나, Write Authorization을 만들거나, gate를 충족하거나, 증거를 만들거나, 민감 동작 승인을 만들거나, 최종 수락을 기록하거나, 잔여 위험을 수락하거나, 닫기 준비 상태를 만들거나, projection repair를 enqueue하거나, Task를 close하면 안 됩니다.

<a id="harnessprepare_write"></a>

## `harness.prepare_write`

에이전트가 제품 파일을 쓰기 전에, 제안된 `AuthorizedAttemptScope`가 현재 Core state에 맞는지 확인할 때 이 method를 사용합니다. 결과는 compatible internal single-use Write Authorization record이거나 structured blocker입니다. 이것은 하네스 수준의 협력형 확인이지 OS 권한, sandboxing, 사전 차단이 아닙니다.

Stage meaning: 내부 엔지니어링 점검과 MVP-1에서 active입니다. MVP-1에서 민감 동작 승인은 `judgment_kind=sensitive_approval`인 compatible `user_judgment`로 표현합니다. Committed Approval record는 later-profile material입니다.

Connected surface `capability_profile`만으로 `decision=allowed`가 되지는 않습니다. Active Task, active Change Unit, current state, compatible `prepare_write`, durable Write Authorization은 계속 Core에서 나옵니다. Recognized surface가 native artifact capture, command observation, network observation, secret-access observation, pre-tool blocking, isolation 같은 required capability를 갖지 못하면 product write가 조용히 진행되면 안 됩니다.

Allowed actors: `lead_agent`, `operator`.

```yaml
PrepareWriteRequest:
  envelope: ToolEnvelope
  task_id: string
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

PrepareWriteResponse:
  base: ToolResponseBase
  decision: allowed | blocked | approval_required | decision_required | state_conflict
  state: StateSummary | null
  change_unit_id: string | null
  baseline_ref: string | null
  write_authorization_ref: StateRecordRef | null
  write_authorization: WriteAuthorizationSummary | null
  authorization_effect: none | would_create | created | returned
  active_user_judgment_refs: StateRecordRef[]
  blocked_reasons:
    - code: string
      message: string
      related_error: ErrorCode
      required_judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation | null
  user_judgment_candidate: UserJudgmentCandidate | null
  guarantee_display:
    level: cooperative | detective | preventive | isolated
    notes: string[]
```

Request fields는 [`AuthorizedAttemptScope`](schema-core.md#evidence-and-pre-write-scope-schemas)의 proposed 부분을 설명합니다. Core는 durable Write Authorization을 만들기 전에 resolved `task_id`, `change_unit_id`, `basis_state_version`, `surface_id`, related user judgment refs, guarantee level을 찍습니다. `WriteAuthorizationSummary.attempt_scope`, `write_authorizations.attempt_scope_json`, `record_run` 비교는 모두 같은 scope를 사용합니다.

`decision=allowed`이고 `dry_run=false`이면 `write_authorization_ref`와 stored `AuthorizedAttemptScope`를 `attempt_scope`로 가진 active `write_authorization`이 있어야 합니다. `dry_run=true`에서는 `authorization_effect=would_create`를 반환할 수 있지만 authorization을 만들지 않습니다. 여기서 `allowed`는 이 API path에서 현재 하네스 기록과 맞는다는 뜻이지 OS 권한이나 실행 전 차단이 아니며, durable Write Authorization lifecycle status도 아닙니다. `decision`이 `allowed`가 아닌 response는 Write Authorization을 포함하면 안 됩니다.

Core가 답할 수 있으면 `PrepareWriteResponse`는 항상 `guarantee_display.level`을 포함해야 합니다. `cooperative` 또는 `detective` level은 접점이 지시로 보류하거나 가능한 경우 사후 탐지를 보고해야 한다는 뜻입니다. 임의 도구를 예방적으로 차단했다는 주장이 아닙니다. Core, 필요한 MCP access, required surface capability를 사용할 수 없으면 response는 [Errors](errors.md)를 따르며, Write Authorization, task event, artifact, projection job, authoritative state-mutation claim을 만들면 안 됩니다. `pre_tool_blocking_supported=false`이면 `preventive` claim을 할 수 없고, `isolation_supported=false`이면 `isolated` claim을 할 수 없습니다.

`user_judgment_candidate`는 상태를 변경하지 않는 [`UserJudgmentCandidate`](schema-core.md#userjudgmentcandidate)입니다. 이것만으로 user judgment, Approval record, Write Authorization, projection을 만들지 않습니다. 민감 동작 승인이 필요하면 MVP-1은 `judgment_kind=sensitive_approval`와 `judgment_payload.approval_scope`를 가진 `user_judgment_candidate`를 반환합니다. 활성 MVP-1에는 `ApprovalRequestCandidate` field나 committed Approval request lifecycle이 없습니다.

Committed `dry_run=false` `decision=allowed` response를 exact idempotent replay하면 original response와 original `write_authorization_ref`를 `authorization_effect=returned`로 반환합니다. 두 번째 Write Authorization을 만들거나 event를 다시 append하면 안 됩니다. 같은 key를 다른 canonical request hash로 replay하면 `STATE_CONFLICT`를 반환합니다.

Public transition summary: `harness.prepare_write`는 envelope를 검증하고, idempotency를 검증하며 exact committed replay가 있으면 새 side effect 전에 반환합니다. Shared request rule에 따라 primary Task를 resolve합니다. Primary Task가 있으면 `tasks.state_version`, 없으면 `project_state.state_version`에 대해 `expected_state_version`을 확인한 뒤 active Change Unit을 resolve합니다. 그다음 candidate `AuthorizedAttemptScope`를 만들고 intended operation/path/tool/command와 command-class/product-file-write/network/secret/sensitive-category compatibility, baseline freshness, 민감 동작 승인, user judgment와 decision-gate coverage, Autonomy Boundary, surface capability, active design-policy precondition을 확인한 뒤 `decision`을 계산합니다. `dry_run=false`이고 `decision=allowed`일 때만 `write_authorizations.status=active`를 만들고 full `attempt_scope_json`을 저장하며, committed `dry_run=false` result는 반환 전에 task event를 append합니다.

<a id="harnessrecord_run"></a>

## `harness.record_run`

Shaping update, direct result, implementation run 뒤에 이 method를 사용합니다. Implementation 또는 direct product-write Run은 `harness.prepare_write`가 반환한 compatible internal Write Authorization record를 소비합니다.

Stage meaning: 내부 엔지니어링 점검에서는 compatible run 하나와 artifact/evidence ref 하나가 active입니다. MVP-1에서는 evidence summary에 active입니다. Verification input, Feedback Loop update, TDD Trace update, full Evidence Manifest behavior는 later/profile-gated입니다.

`record_run`은 active path가 정직하게 지원할 수 있는 내용만 기록합니다. Reference `capability_profile`에서 `artifact_capture_supported=false`, `command_observation_supported=false`, `network_observation_supported=false`, `secret_access_observation_supported=false`이면 native capture, command-observation, network-observation, secret-access claim은 blocked, narrowed, 또는 unverified로 표시해야 합니다. Manual artifact ref는 owner path가 등록한 뒤에만 evidence를 뒷받침할 수 있습니다.

`artifact_inputs`는 `ArtifactInput`이 정의한 source만 받습니다. 즉 Harness staging, approved capture adapter, 이미 commit된 `ArtifactRef`입니다. Caller-supplied arbitrary absolute path, raw secret, token, full sensitive log는 evidence artifact로 등록하면 안 됩니다. Current owner relation, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, `retention_class` metadata가 없는 critical evidence는 `evidence_summary.status=sufficient`를 만들 수 없습니다.

Allowed actors: `lead_agent`, `evaluator`, `operator`.

```yaml
RecordRunRequest:
  envelope: ToolEnvelope
  kind: shaping_update | implementation | direct
  task_id: string
  change_unit_id: string | null
  run_id: string | null
  baseline_ref: string | null
  write_authorization_id: string | null
  summary: string
  artifact_inputs: ArtifactInput[]
  payload: RecordRunPayload

RecordRunPayload:
  kind: shaping_update | implementation | direct
  shaping_update: ShapingUpdatePayload | null
  implementation: ImplementationPayload | null
  direct: DirectPayload | null

RecordRunResponse:
  base: ToolResponseBase
  run_id: string | null
  state: StateSummary
  write_authorization_ref: StateRecordRef | null
  evidence_ref: StateRecordRef | null
  evidence_summary: EvidenceSummary | null
  run_summary_ref: StateRecordRef | null
  direct_result_ref: StateRecordRef | null
  registered_artifacts: ArtifactRef[]
  next_action: string
```

`RecordRunPayload`, `ShapingUpdatePayload`, `ImplementationPayload`, `DirectPayload`는 [Schema Core: Record-run payloads](schema-core.md#record-run-payloads)가 정의합니다. `RecordRunRequest.kind`, `RecordRunPayload.kind`, non-null payload branch는 서로 일대일로 맞아야 합니다. MVP-1은 정확히 `shaping_update`, `implementation`, `direct`만 허용합니다. `verification_input`은 later-profile only입니다.

`kind=shaping_update`일 때 MVP-1은 Discovery와 요구사항 구체화 출력을 active Task update, proposed 또는 active Change Unit update, user judgment 후보 또는 기록으로만 저장합니다. Active API는 `record_kind=shared_design`, committed Shared Design record, required Shared Design projection, Discovery Brief record, Question Queue record, Assumption Register record, First Safe Change Unit Candidate record를 accept하거나 반환하면 안 됩니다.

`evidence_ref`는 active minimal evidence coverage record를 가리킵니다. 보통 `StateRecordRef.record_kind=evidence_summary`를 사용합니다. `evidence_summary`는 Run이 기록된 뒤의 current Core-owned compact summary를 반환합니다. 같은 operation이 반환하는 durable byte는 `registered_artifacts`에 나타납니다. Markdown summary나 projection text는 canonical evidence state가 아닙니다.

Committed `record_run` response를 exact idempotent replay하면 current freshness check, authorization consumption, Run creation, artifact registration, blocker/gate update, projection enqueue, event append 전에 original response를 반환합니다. Write Authorization을 두 번 소비하면 안 됩니다.

Public transition summary: `harness.record_run`은 envelope를 검증하고, idempotency replay를 확인하며 exact committed replay가 있으면 새 side effect 전에 반환합니다. Shared request rule에 따라 primary Task를 resolve합니다. Primary Task가 있으면 `tasks.state_version`, 없으면 `project_state.state_version`에 대해 `expected_state_version`을 확인합니다. 그다음 `kind`를 확인하고 product write를 감지합니다. Product write에는 compatible active Write Authorization을 요구하고, stored `AuthorizedAttemptScope`를 load합니다. Active surface가 관찰하거나 attest할 수 있는 범위에서 observed product-file write, changed paths, tools, commands, command classes, network accesses, secret accesses, sensitive categories, `baseline_ref`, `task_id`, `change_unit_id`, `basis_state_version`, `surface_id`, related user judgment refs, guarantee level을 비교합니다. 그 결과를 compatible observed attempt, missing required authorization, stale authorization, observed attempt outside authorized scope, insufficient surface capability로 분류합니다. Compatible observed attempt일 때만 authorization을 소비하고, Run record를 만들고, 허용된 `ArtifactRef`를 등록하거나 연결하고, evidence summary와 blockers/gates를 업데이트하고, task event를 append한 뒤 committed response를 반환합니다.

이 `record_run` path의 public error mapping은 고정되어 있습니다. Missing required authorization은 `WRITE_AUTHORIZATION_REQUIRED`를 사용합니다. Stale, expired, revoked, consumed, incompatible authorization은 `WRITE_AUTHORIZATION_INVALID`를 사용합니다. Stored `AuthorizedAttemptScope` 또는 active scope 밖의 observed attempt는 `SCOPE_VIOLATION`을 사용합니다. Unsupported observed field 또는 observation/capture/blocking/isolation capability 부족은 `CAPABILITY_INSUFFICIENT`를 사용합니다. Mutation 전 forbidden artifact input shape/source 또는 raw secret payload는 `VALIDATION_FAILED`를 사용합니다. Existing committed artifact ref가 missing이거나 integrity/redaction metadata check에 실패하면 `ARTIFACT_MISSING`를 사용합니다.

이미 등록된 artifact가 missing이거나 required integrity/redaction metadata가 없거나 `hash_mismatch` 같은 diagnostic으로 integrity validation에 실패하면 Core는 related evidence를 `stale` 또는 `blocked`로 표시합니다. Required evidence가 affected이면 replacement, recovery, waiver/risk handling, 또는 다른 owner-approved resolution이 생길 때까지 close path는 blocked로 남습니다.

Core가 write-capable run을 commit 전에 거절하면 `run_id`는 `null`이고 artifact는 등록되지 않으며 response는 Run이 존재한다고 암시하면 안 됩니다. Pre-commit failed `record_run`은 artifact, artifact link, evidence summary, Run row, authorization consumption, blocker/gate update, task event, projection job, state-version advance, replay row를 만들면 안 됩니다. Core는 invalid authorization을 consumed로 표시하면 안 됩니다. Violation/audit Run은 활성 계약의 유일한 예외이며, 제품 쓰기가 이미 관찰된 뒤 Core가 의도적으로 기록할 때만 생길 수 있습니다. Attempted authorization ref는 validator finding, violation payload, event payload에만 나타날 수 있으며 evidence, final acceptance, residual-risk acceptance, close readiness, later/profile QA 또는 verification requirement를 충족하지 않습니다.

<a id="harnessrequest_user_judgment"></a>
<a id="harnessrequest_user_decision"></a>

## `harness.request_user_judgment`

Compatibility alias: `harness.request_user_decision`.

사용자 소유의 제품 판단, 기술 판단, 범위 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락, 취소 판단이 진행이나 close를 막을 때 초점 있는 user judgment request를 만들기 위해 이 method를 사용합니다. QA 면제 판단과 검증 위험 수락은 관련 policy/profile owner가 active일 때만 이 path를 사용합니다. 기본 활성 MVP Manual QA blocker나 verification blocker를 만들지 않습니다.

Stage meaning: 내부 엔지니어링 점검에서는 active가 아닙니다. MVP-1에서 active입니다. Full-format Decision Packet presentation, committed Approval record lifecycle, reconcile, rich residual-risk profile은 명시적으로 active가 되기 전까지 later/profile-gated입니다.

Allowed actors: `lead_agent`, `evaluator`, `operator`.

```yaml
RequestUserJudgmentRequest:
  envelope: ToolEnvelope
  task_id: string
  change_unit_id: string | null
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
  presentation: short | full
  context:
    why_now: string
    source_refs: StateRecordRef[]
    evidence_refs: EvidenceRefs
  state_summary_at_request: StateSummary | null
  question: string
  what_user_is_judging: string
  why_agent_cannot_decide: string
  no_decision_consequence: string
  what_agent_may_decide_without_user: string[]
  affected_scope: UserJudgmentScope
  affected_gates: UserJudgmentGateRef[]
  affected_acceptance_criteria: UserJudgmentCriterionRef[]
  judgment_payload: UserJudgmentPayload
  expires_at: string | null

RequestUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_id: string
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  approval_id: string | null
  reconcile_item_id: string | null
  state: StateSummary
  user_visible_summary: string
```

Minimum MVP-1에서는 `approval_id`가 `null`입니다. Sensitive-action approval judgment는 `harness.record_user_judgment`가 resolve한 뒤에만 범위 있는 승인을 기록합니다. 이것은 Write Authorization이 아니며 제품 판단, 기술 판단, 범위 판단, QA 면제 판단, 검증 위험 수락, 최종 수락, 잔여 위험 수락을 대신하지 않습니다.

`harness.request_user_judgment`는 committed request path입니다. Core가 request를 commit하고 `dry_run=false`일 때만 pending `user_judgments` row를 만듭니다. `harness.prepare_write`, `harness.status`, dry-run validation이 반환하는 `UserJudgmentCandidate`는 `StateRecordRef`가 없는 presentation candidate입니다. Pending judgment가 아니고, 민감 동작 승인을 부여하지 않으며, close/status blocker를 충족하지 않습니다.

Committed judgment request를 exact idempotent replay하면 새 state-version check나 side effect 전에 original `user_judgment_ref`, `user_judgment`, state, user-visible summary를 반환합니다. Same-key/different-hash replay는 `STATE_CONFLICT`를 반환합니다. `dry_run=true`는 question을 validate하고 would-request presentation을 반환할 수 있지만 `user_judgments` row, blocker link, event, replay row, Approval, reconcile item, evidence, Write Authorization, acceptance, residual-risk acceptance, close state를 만들지 않습니다.

<a id="harnessrecord_user_judgment"></a>
<a id="harnessrecord_user_decision"></a>

## `harness.record_user_judgment`

Compatibility alias: `harness.record_user_decision`.

이미 존재하는 canonical `UserJudgment`에 대한 사용자의 답을 기록할 때 이 method를 사용합니다.

Stage meaning: 내부 엔지니어링 점검에서는 active가 아닙니다. MVP-1에서는 사용자 소유 판단, sensitive-action approval judgment resolution, policy가 허용하는 QA waiver/risk path, required verification이 waived된 경우의 verification-risk acceptance, required final acceptance, required residual-risk acceptance, cancellation에 active입니다. Committed Approval update, reconcile outcome, richer residual-risk metadata는 명시적으로 active가 되기 전까지 later/profile-gated입니다.

Allowed actors: `user`, `operator`.

```yaml
RecordUserJudgmentRequest:
  envelope: ToolEnvelope
  user_judgment_id: string
  judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | qa_waiver | verification_risk_acceptance | final_acceptance | residual_risk_acceptance | cancellation
  selected_option_id: string | null
  judgment: RecordUserJudgmentPayload
  note: string
  waiver_reason: string | null
  accepted_risks: AcceptedRiskInput[]

RecordUserJudgmentPayload:
  value: selected | rejected | deferred | granted | denied | expired | waived | accepted | cancelled
  value_note: string | null

RecordUserJudgmentResponse:
  base: ToolResponseBase
  user_judgment_id: string
  user_judgment_ref: StateRecordRef
  user_judgment: UserJudgment
  state: StateSummary
  updated_records: StateRecordRef[]
  accepted_risk_refs: StateRecordRef[]
  next_action: string
```

`judgment_kind`는 저장된 `UserJudgment`와 일치해야 합니다. "yes, do it", "go ahead", "looks good", "진행해" 같은 free-form note는 pending judgment가 그 `judgment_kind`를 명시적으로 묻고, affected object와 scope가 맞으며, 기록된 사용자 intent가 allowed value와 맞을 때만 민감 동작 승인, 최종 수락, 잔여 위험 수락, QA 면제 판단, 검증 위험 수락, 취소 판단, 범위 변경, 쓰기 전 범위 확인 호환성과 연결될 수 있습니다.

MVP-1에서 `accepted_risk_refs`는 해당 close path에서 risk가 보였고 수락됐음을 보여주는 `user_judgment`와 `blocker` ref를 포함합니다. Rich `residual_risk` ref는 later/profile-promoted입니다. 별도 accepted-risk record kind는 없습니다.

`accepted_risks`는 `judgment_kind=residual_risk_acceptance`일 때만 [`AcceptedRiskInput`](schema-core.md#acceptedriskinput)을 사용합니다. 다른 모든 judgment kind에서는 `[]`여야 합니다. Rich residual-risk lifecycle metadata는 later/profile-gated로 남습니다.

Committed answer를 exact idempotent replay하면 새 state-version check, blocker update, gate recompute, event append, replay write 전에 original resolved judgment response를 반환합니다. Same-key/different-hash replay는 `STATE_CONFLICT`를 반환합니다. `dry_run=true`는 answer를 validate하고 would-change effect를 보고하지만 `user_judgment`를 resolve하지 않고, blocker를 update하지 않고, 민감 동작 승인을 부여하지 않고, waiver, final acceptance, residual-risk acceptance를 기록하지 않고, scope/task state를 update하지 않고, event를 append하지 않고, state version을 advance하지 않고, replay row를 만들지 않습니다.

Public transition summary: `harness.record_user_judgment`는 envelope를 validate하고, committed replay를 확인하고, addressed `user_judgment`를 load합니다. Stored `judgment_kind`, affected object, scope, pending status가 request와 맞는지 검증하고, owning Task 또는 project state에 대해 `expected_state_version`을 확인합니다. 그 `judgment_kind`에 맞는 selected value와 payload를 validate한 뒤 covered judgment effect만 적용하고, affected blocker/gate를 update하고, committed event를 append하고, resolved judgment response를 반환합니다. Broad note는 pending judgment contract보다 recorded effect를 넓힐 수 없습니다.

<a id="harnessclose_task"></a>

## `harness.close_task`

Task를 complete, cancel, supersede할 수 있는지 Core에 묻기 위해 이 method를 사용합니다.

Stage meaning: 내부 엔지니어링 점검에서는 optional narrow blocker/status smoke입니다. MVP-1에서는 close-readiness와 blocker response가 active입니다. Detached verification, Manual QA, full assurance, report freshness, export, operations blocker는 later/profile-gated입니다.

Allowed actors: `user`, `lead_agent`, `operator`.

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: complete | cancel | supersede
  requested_close_reason: completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  user_note: string | null
  superseded_by_task_id: string | null

CloseTaskResponse:
  base: ToolResponseBase
  close_state: open | blocked | closed | cancelled | superseded
  closed: boolean
  close_reason: none | completed_self_checked | completed_with_risk_accepted | cancelled | superseded
  assurance_level: none | self_checked
  residual_risk_state: ResidualRiskSummary
  evidence_summary: EvidenceSummary | null
  acceptance_state:
    status: not_required | required | pending | accepted | rejected
    accepted_by_ref: StateRecordRef | null
    required_before_close: boolean
  state: StateSummary
  blockers:
    - code: ErrorCode
      category: task | open_run | scope | user_judgment | sensitive_approval | design_policy | evidence | artifact_availability | final_acceptance | residual_risk_visibility | residual_risk_acceptance | cancellation | supersession
      required_judgment_kind: product_decision | technical_decision | scope_decision | sensitive_approval | final_acceptance | residual_risk_acceptance | cancellation | null
      message: string
      required_next_action: string
      related_refs: StateRecordRef[]
  final_report_refs: StateRecordRef[]
  artifact_refs: ArtifactRef[]
```

MVP-1 close는 active Task, active scope, open Run state, blocker, residual-risk visibility, required final-acceptance state, 아티팩트 가용성, Core가 소유한 `evidence_summary`를 사용합니다. Close readiness는 current record에서 파생됩니다. `completed_verified`, `assurance_level=detached_verified`, `profile_required_verification`, verification blocker, Manual QA blocker, projection/report freshness blocker, operations ref는 [Schema Later](schema-later.md#later-close-and-assurance-extensions)가 소유하는 later/profile-only extension입니다.

`intent=complete`에서 closed response가 되려면 Task state가 close intent와 호환되고, close와 관련해 unresolved active Run이 없고, required user judgment가 unresolved 또는 blocked 상태가 아니며, evidence가 required이면 `evidence_summary.status=sufficient`여야 합니다. Final acceptance가 required이면 `judgment_kind=final_acceptance`가 기록되어야 합니다. Close-relevant residual risk는 visible해야 하며, `completed_with_risk_accepted`에는 명시적인 residual-risk acceptance가 필요합니다. Close-required artifact ref는 여전히 available이어야 하고 required owner relation, `sha256`, `size_bytes`, `content_type`, `redaction_state`, `produced_by`, `retention_class` metadata와 일치해야 합니다. Missing artifact나 `hash_mismatch` 같은 integrity failure는 affected evidence를 stale 또는 blocked로 만듭니다. Stale 또는 blocked Write Authorization fact는 그 영향이 닿는 current Run, scope, artifact, evidence, blocker record를 통해서만 close에 영향을 줍니다. Projection freshness는 display freshness이지 canonical close state가 아닙니다. Caller는 stale projection prose에서 close하면 안 됩니다.

`CloseTaskRequest`는 accepted-risk refs를 싣지 않습니다. `completed_with_risk_accepted`에서는 Core가 close-relevant risk를 보여 주는 blocker와 residual-risk acceptance `user_judgment`의 accepted state를 읽고, 그 상태가 없으면 block합니다. Rich Residual Risk record는 해당 later profile이 active일 때만 필요합니다.

Successful close는 Task를 terminal state로 옮깁니다. Committed blocked close는 Task를 open 상태로 남기고 structured blockers를 반환합니다. Validation, Core/MCP, stale-state, same-key/different-hash failure는 terminal Task update, blocker row, event, projection job, state-version advance, replay row를 만들지 않습니다. 같은 idempotency key의 repeated successful close는 같은 terminal response를 반환하고, conflicting close intent는 `STATE_CONFLICT`를 반환합니다.

`dry_run=true`는 close intent를 확인하고 would-close 또는 would-block diagnostic만 반환합니다. Task를 terminal로 표시하거나 close blocker를 만들거나 업데이트하거나 close event를 append하거나 projection job을 enqueue하거나 state version을 advance하거나 close record 또는 replay row를 만들면 안 됩니다.
