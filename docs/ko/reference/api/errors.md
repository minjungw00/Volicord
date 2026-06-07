# API 오류

## 이 문서로 할 수 있는 일

현재 MVP의 public error code, primary-error precedence, blocked/dry-run 동작, idempotency replay, state conflict 동작, close blocker 동작, 사용자 표시 라벨 지침을 확인할 때 이 참조를 사용합니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 문서 저장소에 MCP server가 구현되어 있다는 뜻이 아닙니다.

## 현재 MVP 보장과 profile-gated 주장 경계

`guarantee_display.level`은 승격된 profile이 profile-gated 표시 값을 명시적으로 지원하지 않는 한 현재 MVP 값인 `cooperative`와 `detective`를 사용합니다. 보안 의미는 [보안 참조: 정직한 guarantee display](../security.md#정직한-guarantee-display)가 담당하고, 정확한 값 집합 경계는 [API Schema Core](schema-core.md#current-mvp-value-sets)가 담당합니다.

지원되지 않는 profile-gated 보장을 요구하거나 표시해 달라는 요청은 주장 경계 오류입니다. 필요한 차단, 격리, 관찰, 증명 지원이 접점에 없으면 `CAPABILITY_INSUFFICIENT`를 사용합니다. 요청한 값이 활성 profile이나 요청 형태에서 유효하지 않으면 `VALIDATION_FAILED`를 사용합니다. 어떤 오류도 더 강한 보장이 존재한다는 증거가 아니며, 문서 전용인 현재 저장소에 런타임 enforcement가 있다는 뜻도 아닙니다.

| Level 또는 이름 | 오류/상태 의미 |
|---|---|
| `cooperative` | 에이전트나 tool이 문서화된 경로를 따를 때 하네스가 확인하고 기록할 수 있습니다. OS 권한, 샌드박스, 변조 방지 저장소, 실행 전 차단이 아닙니다. |
| `detective` | 하네스 또는 연결된 surface가 관찰 가능한 mismatch를 action 중이나 이후에 감지, 기록, 보고할 수 있습니다. 예방이 아닙니다. |
| `preventive` | profile-gated 표시 값 이름입니다. 대상 동작에 대한 승격된 도구 실행 전 차단 지원이 없으면 역량 부족 또는 검증 오류를 반환하고 표시 보장을 낮춥니다. |
| `isolated` | profile-gated 표시 값 이름입니다. 이름 붙은 경계에 대한 승격된 격리 지원이 없으면 역량 부족 또는 검증 오류를 반환하고 표시 보장을 낮춥니다. |

활성 MVP 동작은 기본적으로 협력형 확인입니다. 연결된 surface가 사실을 정직하게 관찰할 수 있을 때만 제한된 사후 확인을 함께 표시합니다. 이런 보안 비주장은 close blocker와 별개입니다. Close blocker는 구조화된 작업 준비 상태 결과이지 `preventive` 수준의 도구 실행 전 차단, `isolated` 수준의 격리, 샌드박스, 변조 방지 저장소의 증거가 아닙니다.

| 조건 | Public path | 에이전트 규칙 |
|---|---|---|
| `core_unavailable` | `MCP_UNAVAILABLE` | 하네스 상태를 만들어 내지 않습니다. Core에 다시 닿거나 사용자가 하네스 밖 진행을 명시적으로 선택하기 전까지 하네스에 의존하는 write와 close를 보류합니다. |
| `local_access_denied` | `LOCAL_ACCESS_MISMATCH` 또는 `CAPABILITY_INSUFFICIENT` | 로컬 파일이나 명령 사실을 추측하지 않습니다. 가능한 local surface를 쓰거나, capability registration을 고치거나, scope를 줄이거나, 입력을 unverified로 표시합니다. |
| `stale_state` | `STATE_CONFLICT`, `BASELINE_STALE`, `PROJECTION_STALE`, stale `WRITE_AUTHORIZATION_INVALID` | 의존하기 전에 current state, baseline, readable view, 쓰기 전 확인을 새로 확인합니다. |
| `unsupported_surface` | `CAPABILITY_INSUFFICIENT` 또는 `VALIDATION_FAILED` | 요청을 줄이거나, 가능한 surface로 옮기거나, blocker를 반환합니다. 지원하지 않는 authority를 prose로 흉내 내지 않습니다. |
| `out_of_scope` | `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `NO_ACTIVE_CHANGE_UNIT`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | 영향을 받는 action을 보류하고, mismatch를 보여 주며, current scope로 줄이거나 specific user-owned scope judgment를 요청합니다. |
| `missing_judgment` | `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, `ACCEPTANCE_REQUIRED` | 집중된 `UserJudgment`를 묻거나 해결합니다. Product, technical, scope, sensitive approval, final acceptance, residual-risk acceptance, QA waiver, verification-risk acceptance, cancellation을 넓은 approval 하나로 합치지 않습니다. |
| `missing_evidence` | `EVIDENCE_INSUFFICIENT`, `ARTIFACT_MISSING` | 영향을 받는 claim, refs, evidence status, 가장 작은 unblocker를 보여줍니다. Test result, artifact integrity, evidence sufficiency를 만들어 내지 않습니다. |
| `close_blocked` | `CloseTaskResponse.close_state=blocked`와 primary `ErrorCode` | Structured blocker와 next action을 반환합니다. Task를 terminal로 표시하지 않습니다. |
| `residual_risk_present` | `RESIDUAL_RISK_NOT_VISIBLE`, `DECISION_REQUIRED`, 또는 `DECISION_UNRESOLVED` | Risk를 보여 주고, active close 또는 acceptance path가 요구할 때만 `judgment_kind=residual_risk_acceptance`를 묻습니다. |

<a id="error-taxonomy"></a>

## 오류 분류

| Code | 의미 |
|---|---|
| `VALIDATION_FAILED` | Payload shape, enum value, activation rule, profile-specific validation이 mutation 전에 실패했습니다. |
| `STATE_CONFLICT` | `expected_state_version`이 stale이거나, state lock ownership이 바뀌었거나, 같은 idempotency key를 다른 canonical request로 다시 사용했습니다. |
| `NO_ACTIVE_TASK` | Task가 필요하지만 active 또는 addressed Task가 없습니다. |
| `NO_ACTIVE_CHANGE_UNIT` | Write-capable 또는 close-relevant operation에 active scoped Change Unit이 없습니다. |
| `SCOPE_REQUIRED` | Requested write나 action 전에 scope confirmation이 필요합니다. |
| `SCOPE_VIOLATION` | Intended 또는 observed paths, tools, commands, network targets, secret access, sensitive categories가 active scope 또는 stored `AuthorizedAttemptScope`를 넘었습니다. |
| `WRITE_AUTHORIZATION_REQUIRED` | Write-capable Run에 `harness.prepare_write`의 required Write Authorization이 없습니다. |
| `WRITE_AUTHORIZATION_INVALID` | Supplied Write Authorization이 missing, expired, stale, revoked, replay 밖에서 consumed, 또는 incompatible입니다. |
| `DECISION_REQUIRED` | Action 전에 blocking user-owned judgment를 요청해야 합니다. |
| `DECISION_UNRESOLVED` | Relevant user judgment가 pending, deferred without coverage, rejected, blocked, stale, superseded, 또는 incompatible입니다. |
| `AUTONOMY_BOUNDARY_EXCEEDED` | Intended operation이 active Change Unit Autonomy Boundary를 넘었습니다. |
| `APPROVAL_REQUIRED` | 진행 전에 민감 동작 승인이 필요합니다. |
| `APPROVAL_DENIED` | 관련 민감 동작 승인이 denied되었습니다. |
| `APPROVAL_EXPIRED` | 관련 민감 동작 승인이 expired되었거나 scope/baseline에서 drift되었습니다. |
| `CAPABILITY_INSUFFICIENT` | Surface는 recognized이지만 required observation, capture, local access, blocking/isolation condition, guarantee claim, active behavior를 충족할 수 없습니다. |
| `MCP_UNAVAILABLE` | 필요한 MCP/Core access를 사용할 수 없거나, stale이거나, unreachable입니다. |
| `LOCAL_ACCESS_MISMATCH` | Reachable local caller/access path가 registered local profile 밖이거나 required local access가 없습니다. |
| `EVIDENCE_INSUFFICIENT` | Required evidence coverage가 absent, partial, stale, blocked입니다. |
| `ACCEPTANCE_REQUIRED` | Required final acceptance가 pending, rejected, 또는 visible result basis와 incompatible입니다. |
| `PROJECTION_STALE` | Requested readable status/view가 stale 또는 failed입니다. Core state가 아니며 그 자체로 close blocker가 아닙니다. |
| `RESIDUAL_RISK_NOT_VISIBLE` | Known close-relevant residual risk가 final acceptance 또는 close 전에 visible하지 않습니다. |
| `ARTIFACT_MISSING` | Referenced artifact가 missing이거나 integrity/metadata check에 실패했습니다. |
| `BASELINE_STALE` | Operation이 요구하는 repository state와 baseline이 더 이상 맞지 않습니다. |
| `VALIDATOR_FAILED` | Required active validator 또는 blocker check가 failed되었고 더 specific한 typed code가 없을 때 쓰는 fallback입니다. |

`ToolError.details.authorization_reason`은 정확히 다음 값만 사용합니다.

```text
missing | expired | stale | revoked | consumed | incompatible
```

Required authorization이 supplied되지 않았으면 `authorization_reason=missing`과 함께 `WRITE_AUTHORIZATION_REQUIRED`를 사용합니다. Existing authorization이 consume될 수 없으면 `WRITE_AUTHORIZATION_INVALID`를 사용합니다.

<a id="primary-error-code-precedence"></a>

## Primary Error Code 우선순위

`ToolResponseBase.errors`가 non-empty이면 method section이 더 좁은 순서를 정의하지 않는 한 `errors[0]`이 아래 순서로 선택된 primary error입니다. Secondary blocker는 method-specific field와 `ToolError.details`에 남을 수 있습니다.

| Precedence | Primary `ErrorCode` |
|---:|---|
| 1 | `VALIDATION_FAILED` |
| 2 | `STATE_CONFLICT` |
| 3 | `MCP_UNAVAILABLE` |
| 4 | `LOCAL_ACCESS_MISMATCH` |
| 5 | `NO_ACTIVE_TASK` |
| 6 | `NO_ACTIVE_CHANGE_UNIT` |
| 7 | `BASELINE_STALE` |
| 8 | `SCOPE_REQUIRED` |
| 9 | `SCOPE_VIOLATION` |
| 10 | `WRITE_AUTHORIZATION_REQUIRED` |
| 11 | `WRITE_AUTHORIZATION_INVALID` |
| 12 | `APPROVAL_DENIED` |
| 13 | `APPROVAL_EXPIRED` |
| 14 | `APPROVAL_REQUIRED` |
| 15 | `DECISION_UNRESOLVED` |
| 16 | `AUTONOMY_BOUNDARY_EXCEEDED` |
| 17 | `DECISION_REQUIRED` |
| 18 | `CAPABILITY_INSUFFICIENT` |
| 19 | `EVIDENCE_INSUFFICIENT` |
| 20 | `RESIDUAL_RISK_NOT_VISIBLE` |
| 21 | `ACCEPTANCE_REQUIRED` |
| 22 | `PROJECTION_STALE` |
| 23 | `ARTIFACT_MISSING` |
| 24 | `VALIDATOR_FAILED` |

<a id="blocked-and-dry-run-behavior"></a>

## Blocked와 dry-run 동작

Blocked response는 pre-commit failure와 다릅니다. 메서드 담당 문서가 blocker recording을 허용하는 경우에만 Core가 blocked response를 commit할 수 있습니다. 커밋된 blocked response는 `blockers`, events, state version, idempotency replay를 업데이트할 수 있지만, blocker가 missing이라고 말하는 authority를 만들면 안 됩니다.

`dry_run=true`는 항상 기준 권한이 아닙니다. Validate하고 diagnostic, candidate blocker, would-change summary를 반환할 수 있지만 current record, event, artifact, evidence summary, consumable Write Authorization, close state, committed replay row를 만들거나 업데이트하면 안 됩니다. 이후 non-dry-run call은 current state를 기준으로 다시 validate해야 합니다.

<a id="idempotency"></a>

## Idempotency

Committed state-changing method는 모두 `idempotency_key`를 요구합니다. Key는 `(project_id, tool_name, idempotency_key)` scope를 가집니다.

`request_hash`는 tool name, schema-normalized request body, 그리고 `request_id`와 `idempotency_key`를 제외한 모든 `ToolEnvelope` field에 대한 canonical JSON에서 계산합니다.

같은 key와 같은 hash를 가진 committed replay row가 있으면 Core는 freshness check를 다시 실행하거나 event append, artifact register, authorization consume, blocker update, replay row 변경을 하지 않고 original committed response를 반환합니다. 같은 key를 다른 hash로 재사용하면 Core는 `STATE_CONFLICT`를 반환하고 original replay row를 보존합니다.

Dry-run call과 pre-commit failure는 replay row를 만들거나 예약하지 않습니다.

<a id="state-conflict-behavior"></a>

## State conflict 동작

Committed replay row가 없는 새 state-changing attempt에서 Core는 freshness check 전에 primary Task를 resolve합니다. Resolution order는 tool-specific `task_id`, `ToolEnvelope.task_id`, active Task 순서입니다.

Task-scoped mutation은 `expected_state_version`을 `tasks.state_version`과 비교합니다. Resolved primary Task가 없는 project-scoped mutation은 `project_state.state_version`과 비교합니다. Mismatch는 `STATE_CONFLICT`를 반환하고 current record, event, artifact, evidence summary, Write Authorization, close state, replay row를 만들지 않습니다.

`STATE_CONFLICT.details`에는 다음 값을 담아야 합니다.

```yaml
scope: task | project
current_state_version: integer
expected_state_version: integer
project_id: string
task_id: string | null
```

`WriteAuthorization.basis_state_version`은 allow decision의 compatibility basis입니다. 반드시 resulting `ToolResponseBase.state_version`과 같지는 않습니다.

<a id="harnessclose_task-close-blockers"></a>

## `harness.close_task` Close Blocker

`CloseTaskResponse.blockers`는 [API Schema Core](schema-core.md#current-position-display-schemas)의 structured `CloseBlocker` object를 사용해야 합니다. Prose-only status text, report text, rendered view, agent summary는 close-blocker result가 아닙니다.

Close blocker는 public error와 매핑될 때 primary-error precedence에 따라 정렬합니다. Evidence blocker는 보통 `EVIDENCE_INSUFFICIENT`를 사용합니다. Artifact availability blocker는 `ARTIFACT_MISSING`을 사용합니다. Unresolved user judgment blocker는 `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED`를 사용합니다. 민감 동작 승인 blocker는 `APPROVAL_*` code를 사용합니다. Scope blocker는 scope와 baseline code를 사용합니다.

알려진 close-relevant risk가 아직 보이지 않으면 `RESIDUAL_RISK_NOT_VISIBLE`를 사용합니다. Visible하지만 accepted되지 않은 close-relevant risk는 이 code 아래 숨기지 않습니다. Residual-risk acceptance가 필요하면 close blocker는 category `residual_risk_acceptance`와 `required_judgment_kind=residual_risk_acceptance`를 사용하고, `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED`를 반환합니다.

`PROJECTION_STALE`은 readable-view freshness 오류입니다. 그 자체로 active close-blocker category가 아닙니다.

## 사용자 표시 라벨 지침

아래 라벨은 표시 지침이지 새 public error code가 아닙니다.

| API condition | 사용자 표시 라벨 | 가장 작은 unblocker |
|---|---|---|
| `VALIDATION_FAILED` | 잘못된 요청 | Retry 전에 payload, enum value, activation rule, field set을 고칩니다. |
| `STATE_CONFLICT` | 상태 충돌 | Current status를 refresh하고 current state version으로 retry하거나 original idempotent request를 replay합니다. |
| `MCP_UNAVAILABLE` | Core 사용 불가 | State change, gate update, write compatibility, close를 주장하기 전에 Core access를 reconnect 또는 diagnose합니다. |
| `LOCAL_ACCESS_MISMATCH` | local access 거부 또는 역량 불일치 | Registered local surface를 사용하거나, local access를 repair하거나, capable surface로 옮깁니다. |
| `CAPABILITY_INSUFFICIENT` | 지원되지 않거나 부족한 surface | Capable surface를 사용하거나, operation을 줄이거나, missing capability가 필요 없는 path를 선택합니다. |
| `NO_ACTIVE_TASK` | active Task 없음 | Task-scoped action 전에 Task를 select 또는 create합니다. |
| `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | scope, boundary, baseline 문제 | Scope를 confirm/narrow하고, Change Unit이나 baseline을 update하거나, 필요한 user judgment를 request합니다. |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | 쓰기 전 범위 확인 없음 또는 오래됨 | Exact operation, current scope, current state로 `harness.prepare_write`를 call 또는 retry합니다. |
| `DECISION_REQUIRED`, `DECISION_UNRESOLVED` | 판단 필요 | Kind, refs, options, consequences와 함께 focused `UserJudgment`를 보여 주거나 resolve합니다. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` | 민감 동작 승인 필요 또는 사용 불가 | `judgment_kind=sensitive_approval` user judgment를 request, resolve, renew합니다. |
| `EVIDENCE_INSUFFICIENT` | evidence 필요 | Missing check를 record/rerun하거나 evidence gap과 가장 작은 unblocker를 보여 줍니다. |
| `ACCEPTANCE_REQUIRED` | final acceptance 필요 | Visible result basis에 대해 `judgment_kind=final_acceptance`를 request 또는 resolve합니다. |
| `RESIDUAL_RISK_NOT_VISIBLE` | residual risk가 보이지 않음 | Final acceptance 또는 close 전에 close-relevant risk를 보여 줍니다. |
| `PROJECTION_STALE` | readable view 오래됨 | 그 view에 의존하기 전에 refresh합니다. Canonical close state로 취급하지 않습니다. |
| `ARTIFACT_MISSING` | artifact 문제 | Missing/failed artifact를 reattach, regenerate, replace한 뒤 의존합니다. |
| `VALIDATOR_FAILED` | check 또는 blocker 실패 | Specific validator 또는 blocker를 보여 줍니다. Typed blocker가 없을 때만 fallback으로 사용합니다. |
