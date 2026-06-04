# API Errors

## 이 문서로 할 수 있는 일

Public API error code, primary error precedence, idempotency replay, stale-state behavior를 확인할 때 이 참조를 사용합니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 문서 저장소에 MCP server가 구현되어 있다는 뜻이 아닙니다.

## Error taxonomy

| Code | Meaning |
|---|---|
| `VALIDATION_FAILED` | Request payload, enum value, activation rule, profile-specific schema validation이 mutation 전에 실패했습니다. |
| `STATE_CONFLICT` | `expected_state_version`이 stale이거나, lock ownership이 바뀌었거나, 같은 idempotency key를 다른 payload로 다시 사용했습니다. |
| `NO_ACTIVE_TASK` | Task가 필요하지만 active 또는 addressed Task가 없습니다. |
| `NO_ACTIVE_CHANGE_UNIT` | Write-capable operation에 active scoped Change Unit이 없습니다. |
| `SCOPE_REQUIRED` | Requested write가 진행되기 전에 scope confirmation이 필요합니다. |
| `SCOPE_VIOLATION` | Intended paths, tools, commands, network, secrets, categories가 scope를 넘었습니다. |
| `WRITE_AUTHORIZATION_REQUIRED` | Write-capable run에 `harness.prepare_write`의 required Write Authorization이 없습니다. |
| `WRITE_AUTHORIZATION_INVALID` | Supplied Write Authorization이 absent, expired, stale, revoked, already consumed outside idempotent replay, 또는 incompatible입니다. |
| `DECISION_REQUIRED` | Requested action 전에 blocking user-owned judgment request가 필요합니다. |
| `DECISION_UNRESOLVED` | Relevant user judgment가 pending, deferred without coverage, rejected, blocked, stale, 또는 incompatible입니다. |
| `AUTONOMY_BOUNDARY_EXCEEDED` | Intended operation이 active Change Unit Autonomy Boundary를 넘었습니다. |
| `APPROVAL_REQUIRED` | Sensitive action을 진행하기 전에 sensitive-action permission이 필요합니다. |
| `APPROVAL_DENIED` | Relevant sensitive-action permission / Approval이 denied되었습니다. |
| `APPROVAL_EXPIRED` | Sensitive-action permission / Approval이 expired되었거나 baseline/scope에서 drift되었습니다. |
| `CAPABILITY_INSUFFICIENT` | Connected surface는 valid하지만 required validator, feature, enforcement condition을 충족할 수 없습니다. |
| `MCP_UNAVAILABLE` | Required MCP access가 unavailable, stale, unreachable입니다. |
| `LOCAL_ACCESS_MISMATCH` | Core 또는 operator가 caller의 local access mode를 registered local profile 밖으로 분류할 수 있습니다. |
| `EVIDENCE_INSUFFICIENT` | Required evidence coverage가 absent, partial, stale, blocked입니다. |
| `VERIFY_NOT_DETACHED` | Verification이 detached verification으로 인정될 수 없습니다. |
| `QA_REQUIRED` | Required Manual QA가 pending, failed, missing입니다. |
| `ACCEPTANCE_REQUIRED` | Required work acceptance가 pending 또는 rejected입니다. |
| `PROJECTION_STALE` | Requested action에 필요한 projection freshness가 stale 또는 failed입니다. |
| `RECONCILE_REQUIRED` | Human-editable 또는 managed-block drift에 reconcile이 필요합니다. |
| `RESIDUAL_RISK_NOT_VISIBLE` | Known close-relevant residual risk가 work acceptance 또는 close 전에 visible하지 않습니다. |
| `ARTIFACT_MISSING` | Referenced artifact file이 missing이거나 integrity check가 failed입니다. |
| `BASELINE_STALE` | Operation이 요구하는 repository state와 baseline이 더 이상 맞지 않습니다. |
| `VALIDATOR_FAILED` | Required validator 또는 close/blocker check가 failed되었고 더 구체적인 typed code가 없을 때 쓰는 fallback입니다. |

`WRITE_AUTHORIZATION_REQUIRED`와 `WRITE_AUTHORIZATION_INVALID`는 missing 또는 invalid Write Authorization에만 사용합니다. Observed paths, tools, commands, network targets, secrets, sensitive categories가 authorized 또는 active scope를 넘으면 scope problem은 여전히 `SCOPE_VIOLATION`을 사용합니다.

MCP availability, local access/profile mismatch, capability insufficiency는 서로 다릅니다.

- `MCP_UNAVAILABLE`: Core에 닿을 수 없거나 required MCP access가 stale/unusable입니다.
- `LOCAL_ACCESS_MISMATCH`: reachable local endpoint 또는 caller path가 off-profile, stale, weak, forwarded/tunneled, cross-user, unauthorized, 또는 mismatched입니다.
- `CAPABILITY_INSUFFICIENT`: caller는 recognized surface/profile에 있지만 required capability, validator, enforcement condition을 충족할 수 없습니다.

## 사용자-facing display labels

아래 label은 display guidance이지 새 public error code가 아닙니다.

| API condition | User-facing label | Smallest unblocker language |
|---|---|---|
| `VALIDATION_FAILED` | invalid request | Retry 전에 payload, enum value, activation rule, profile-specific field set을 고칩니다. |
| `STATE_CONFLICT` | state conflict | Current status를 refresh한 뒤 current state version으로 retry하거나 original idempotent request를 replay합니다. |
| `MCP_UNAVAILABLE` | MCP unavailable | State change, gate update, projection repair, write authority, close를 주장하기 전에 Core access를 reconnect 또는 diagnose합니다. |
| `LOCAL_ACCESS_MISMATCH` | local access profile mismatch | Registered local surface/profile로 reconnect하거나 local binding/profile을 repair합니다. |
| `CAPABILITY_INSUFFICIENT` | capability insufficient | Capable surface/profile을 사용하거나, operation을 줄이거나, missing capability가 필요 없는 path를 선택합니다. |
| `NO_ACTIVE_TASK` | no active Task | Task-scoped action을 사용하기 전에 Task를 select 또는 create합니다. |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | missing or stale write authority | Exact intended operation, current scope, current state로 `harness.prepare_write`를 call 또는 retry합니다. |
| `NO_ACTIVE_CHANGE_UNIT`, `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `AUTONOMY_BOUNDARY_EXCEEDED`, `BASELINE_STALE` | scope, boundary, or baseline issue | Scope를 confirm/narrow하고, Change Unit이나 baseline을 update하거나, 필요한 user judgment를 request합니다. |
| `DECISION_REQUIRED`, `DECISION_UNRESOLVED` | judgment needed | Relevant user judgment prompt 또는 pending outcome을 refs와 consequences와 함께 보여 줍니다. |
| `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED` | sensitive-action permission needed or not usable | Minimum MVP-1에서는 sensitive-action approval user judgment를 request, resolve, renew합니다. Committed Approval record는 later-profile입니다. |
| `EVIDENCE_INSUFFICIENT`, `VERIFY_NOT_DETACHED`, `QA_REQUIRED`, `ACCEPTANCE_REQUIRED`, `RESIDUAL_RISK_NOT_VISIBLE` | evidence, verification, QA, work acceptance, or risk visibility needed | Missing check를 record/rerun하고, residual risk를 보여 주고, work acceptance를 request하거나 valid owner waiver path를 사용합니다. |
| `PROJECTION_STALE` | stale status view | 그 readable view에 의존하기 전에 projection을 refresh 또는 reconcile합니다. |
| `RECONCILE_REQUIRED` | reconcile needed | Affected projection 또는 close path를 사용하기 전에 human-editable 또는 managed-block drift를 reconcile합니다. |
| `ARTIFACT_MISSING` | artifact issue | Artifact를 evidence로 쓰기 전에 missing/failed artifact를 reattach, regenerate, replace합니다. |
| `VALIDATOR_FAILED` | check or blocker failed | Available한 specific validator 또는 blocker를 보여 줍니다. Typed blocker가 없을 때만 fallback으로 사용합니다. |

## Primary Error Code Precedence

Core가 여러 blocker를 관찰해도 public tool response는 하나의 primary `ToolError.code`를 가집니다. `ToolResponseBase.errors`가 non-empty이면 `errors[0]`이 primary error입니다. Tool subsection이 더 좁은 순서를 정의하지 않는 한 아래 precedence를 사용합니다. Secondary blocker는 tool-specific field, validator result, `ToolError.details`, state summary에 남을 수 있습니다.

| Precedence | Primary `ErrorCode` | Selection note |
|---:|---|---|
| 1 | `VALIDATION_FAILED` | Request payload, enum, activation, profile-specific field validation이 mutation 전에 실패했습니다. |
| 2 | `STATE_CONFLICT` | Stale `expected_state_version`, state lock conflict, 또는 같은 idempotency key를 다른 payload로 재사용했습니다. |
| 3 | `MCP_UNAVAILABLE` | Core/operator classification 뒤 required MCP access가 unavailable, stale, unreachable입니다. |
| 4 | `LOCAL_ACCESS_MISMATCH` | Reachable local caller/access mode가 registered local profile에서 off-profile 또는 unauthorized입니다. |
| 5 | `NO_ACTIVE_TASK` | Operation에 Task가 필요하지만 active/addressed Task가 없습니다. |
| 6 | `NO_ACTIVE_CHANGE_UNIT` | Operation이 write-capable 또는 close-relevant인데 active scoped Change Unit이 없습니다. |
| 7 | `BASELINE_STALE` | Requested operation이 stale baseline에 의존합니다. |
| 8 | `SCOPE_REQUIRED` | Requested operation 전에 scope confirmation이 필요합니다. |
| 9 | `SCOPE_VIOLATION` | Intended 또는 observed paths, tools, commands, network, secrets, categories가 scope를 넘었습니다. |
| 10 | `WRITE_AUTHORIZATION_REQUIRED` | Write-capable Run에 required Write Authorization이 없습니다. |
| 11 | `WRITE_AUTHORIZATION_INVALID` | Supplied Write Authorization이 stale, expired, revoked, consumed outside replay, 또는 incompatible입니다. |
| 12 | `APPROVAL_DENIED` | Relevant sensitive-action permission이 denied되었습니다. |
| 13 | `APPROVAL_EXPIRED` | Relevant sensitive-action permission이 expired 또는 drift되었습니다. |
| 14 | `APPROVAL_REQUIRED` | Sensitive change에 sensitive-action permission이 필요하고 compatible grant가 없습니다. |
| 15 | `DECISION_UNRESOLVED` | Existing relevant user judgment가 pending, rejected, stale, 또는 incompatible입니다. |
| 16 | `AUTONOMY_BOUNDARY_EXCEEDED` | Intended operation이 active Autonomy Boundary를 넘었습니다. |
| 17 | `DECISION_REQUIRED` | Blocking user-owned judgment에 user judgment request가 필요합니다. |
| 18 | `CAPABILITY_INSUFFICIENT` | Connected surface가 required capability 또는 enforcement condition을 충족할 수 없습니다. |
| 19 | `EVIDENCE_INSUFFICIENT` | Required evidence coverage가 absent, partial, stale, blocked입니다. |
| 20 | `VERIFY_NOT_DETACHED` | Verification이 detached verification으로 인정될 수 없습니다. |
| 21 | `QA_REQUIRED` | Required Manual QA가 pending, failed, missing, 또는 validly waived가 아닙니다. |
| 22 | `RESIDUAL_RISK_NOT_VISIBLE` | Known close-relevant residual risk가 visible하지 않습니다. |
| 23 | `ACCEPTANCE_REQUIRED` | Residual-risk visibility가 충족된 뒤 required work acceptance가 pending 또는 rejected입니다. |
| 24 | `PROJECTION_STALE` | Requested action에 필요한 projection freshness가 stale 또는 failed입니다. |
| 25 | `RECONCILE_REQUIRED` | Human-editable 또는 managed-block drift에 reconcile이 필요합니다. |
| 26 | `ARTIFACT_MISSING` | Referenced artifact file이 missing이거나 integrity check가 failed입니다. |
| 27 | `VALIDATOR_FAILED` | 더 specific한 typed blocker가 없을 때만 generic validator fallback으로 선택합니다. |

## `harness.close_task` Close Blockers

`harness.close_task`는 여러 close blocker를 반환할 수 있습니다. `CloseTaskResponse.base.errors`의 primary `ToolError`는 위 precedence를 사용하고, `CloseTaskResponse.blockers`는 observed close blocker를 같은 상대 순서의 structured result로 포함해야 합니다. Prose-only status text, report, Journey view, agent summary는 close-blocker result가 아닙니다.

Visible-but-unaccepted close-relevant risk는 `RESIDUAL_RISK_NOT_VISIBLE`로 반환하지 않습니다. Requested close path가 risk acceptance를 요구하면, residual-risk acceptance user judgment를 새로 request해야 할 때 public close/API response는 primary `DECISION_REQUIRED`를 사용합니다. Relevant residual-risk acceptance user judgment가 있지만 pending, rejected, blocked, stale, deferred without coverage, incompatible이면 `DECISION_UNRESOLVED`를 사용합니다. Structured close blocker category는 `residual_risk_acceptance`여야 하며, relevant `residual_risk` 또는 `user_judgment` record refs를 포함해야 합니다.

## Idempotency

Idempotency key는 `(project_id, tool_name, idempotency_key)` scope를 가집니다. 같은 key와 같은 payload를 반복하면 original committed response를 반환합니다. 같은 key를 다른 payload로 재사용하면 `STATE_CONFLICT`를 반환합니다.

`request_hash`는 UTF-8 canonical JSON에서 계산합니다. Canonical input은 `tool_name`, schema-normalized request body, 그리고 `request_id`와 `idempotency_key`를 제외한 모든 `ToolEnvelope` field를 포함합니다.

State-changing tool에서 Core는 call을 new mutation attempt로 다루기 전에 existing committed replay row를 확인합니다. Matching hash는 current freshness check를 다시 실행하거나, event를 append하거나, artifact를 register하거나, projection을 enqueue하거나, replay row를 update하지 않고 original committed response를 반환합니다. Different hash는 `STATE_CONFLICT`를 반환하고 original replay row를 보존합니다.

Key가 different canonical request payload로 재사용되면 `ToolError.details`는 idempotency scope, stored/received request hash 또는 equivalent opaque comparison, caller가 original request를 replay하거나 fresh key로 retry해야 한다는 사실을 포함할 수 있습니다. Details는 sensitive request body를 노출하면 안 됩니다.

## State conflict behavior

Supplied idempotency scope에 committed replay row가 없는 state-changing tool에서 Core는 mutation 전에 `expected_state_version`을 current project/task state와 비교합니다. Mismatch는 `STATE_CONFLICT`를 반환합니다. 그 conflicting new attempt에 대해 current records, events, artifacts, projection jobs, replay rows를 만들지 않습니다.

Core는 먼저 `ToolEnvelope.task_id`, tool-specific `task_id`, active Task resolution에서 primary addressed Task를 resolve합니다. Task-scoped tool은 `tasks.state_version`과 비교하고, resolved primary Task가 없는 project-scoped tool은 `project_state.state_version`과 비교합니다.

`STATE_CONFLICT.details` should include:

```yaml
scope: task | project
current_state_version: integer
expected_state_version: integer
project_id: string
task_id: string | null
```

Stale `expected_state_version`은 concurrency drift이지 caller identity의 증명이 아닙니다. Caller는 retry 전에 refresh해야 합니다. Core는 caller가 older Task 또는 project view를 제공했다는 이유만으로 이를 accept하면 안 됩니다.
