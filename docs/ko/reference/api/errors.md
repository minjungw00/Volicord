# API Errors

## 이 문서로 할 수 있는 일

Public API error code, primary error precedence, idempotency replay, stale-state behavior를 확인할 때 이 참조를 사용합니다.

이 문서는 향후 하네스 서버 동작을 계획하고 검토하기 위한 참조입니다. 현재 문서 저장소에 MCP server가 구현되어 있다는 뜻이 아닙니다.

<a id="mvp-1-guarantee-and-status-taxonomy"></a>

## MVP-1 guarantee와 상태/error taxonomy

이 섹션은 MVP-1 public status/error condition name, 사용자에게 보이는 문구 pattern, 에이전트 행동의 단일 owner입니다. 아래 condition name은 display와 routing을 위한 이름입니다. `Public API path` 열이 code를 이름 붙이는 경우가 아니라면 새 `ErrorCode` enum value가 아닙니다. Guarantee level value의 보안 의미는 [보안 참조: 정직한 guarantee display](../security.md#정직한-guarantee-display)가 담당합니다.

`guarantee_display.level`은 exact value인 `cooperative`, `detective`, `preventive`, `isolated`를 사용합니다.

| Level | MVP-1 표시 의미 | 에이전트 규칙 |
|---|---|---|
| `cooperative` | Agent나 tool이 문서화된 경로를 따를 때 하네스가 확인하고 기록할 수 있습니다. OS 권한, sandboxing, 변조 방지 저장소, 실행 전 차단이 아닙니다. | 하네스 확인을 사용하고, 맞지 않는 쓰기는 지시로 보류하며, 한계를 정직하게 보여줍니다. |
| `detective` | 하네스 또는 연결된 접점이 mismatch를 관찰할 수 있을 때 감지, 기록, 보고할 수 있습니다. 예방이 아닙니다. | 무엇을 감지했는지와 아직 증명되지 않은 것을 보고합니다. 실행 전에 막았다고 말하지 않습니다. |
| `preventive` | 승격된 profile이 covered operation을 실행 전에 막는 증명된 control을 가지고 있습니다. | Exact covered operation과 proof가 이름 붙은 경우에만 이 label을 사용합니다. |
| `isolated` | 승격된 profile이 해당 주장에 맞는 문서화되고 증명된 separation boundary를 가지고 있습니다. | Boundary를 이름 붙입니다. Isolation만으로 민감 동작 승인, 근거, 작업 수락, 잔여 위험 수용, close, 더 강한 authority를 추론하지 않습니다. |

MVP-1의 기본값은 협력형 행동이며, active surface가 mismatch를 관찰할 수 있을 때 제한된 사후 확인을 함께 표시할 수 있습니다. 더 강한 label은 exact operation 또는 boundary에 대해 owner-promoted profile 문서와 proof가 필요합니다.

| Condition | Public API path | 짧은 뜻 | 사용자 표시 문구 pattern | 에이전트 행동 | 막는 대상: next / write / close | 에이전트가 만들어 내면 안 되는 것 |
|---|---|---|---|---|---|---|
| `core_unavailable` | `MCP_UNAVAILABLE`; 알 수 있을 때 diagnostic `MCP_SERVER_UNAVAILABLE` 또는 `SURFACE_MCP_UNAVAILABLE` | Harness/Core authority에 닿을 수 없습니다. | "하네스 기준 상태에 접근할 수 없습니다. 그래서 현재 하네스 상태를 확인했다고 말할 수 없습니다. 다시 연결하거나 진단할 수 있습니다. 하네스 밖에서 계속하려면 사용자가 그 방식을 명시적으로 선택해야 합니다." | Authority는 fail closed로 다룹니다. 하네스에 의존하는 쓰기와 닫기를 보류합니다. 다시 연결하거나, 진단하거나, 가능한 접점으로 옮깁니다. 사용자가 명시적으로 선택한 경우에만 하네스 밖에서 진행합니다. | 하네스 authority가 필요한 행동은 예 / 예 / 예. | Task 상태, 민감 동작 승인, 사용자 판단, 근거, 작업 수락, 잔여 위험 수용, gate update, projection 최신성, 닫기 준비 상태. |
| `local_access_denied` | Off-profile local access에는 `LOCAL_ACCESS_MISMATCH`; 현재 접점에 required local access가 없으면 `CAPABILITY_INSUFFICIENT` | 로컬 파일 또는 시스템 접근을 사용할 수 없거나, 거부되었거나, registered local profile 밖입니다. | "로컬 접근이 거부되었거나 사용할 수 없습니다. 이 접점에서는 요청한 로컬 경로를 확인하거나 변경할 수 없습니다." | 로컬 상태를 추측하지 않습니다. 가능한 접점을 사용하거나, local profile을 고치거나, 접근 가능한 path로 줄이거나, 검증되지 않은 입력이라고 표시하고 계속합니다. | 접근이 필요하면 예 / 예 / close가 그 접근이나 근거에 의존하면 예. | 파일 내용, 명령 결과, artifact byte, 근거 충분성, 성공한 local change. |
| `stale_state` | `STATE_CONFLICT`, `BASELINE_STALE`, `PROJECTION_STALE`, 또는 stale `WRITE_AUTHORIZATION_INVALID` | 현재 상태, baseline, authorization, 읽기용 보기가 오래됐을 수 있습니다. | "현재 하네스 상태나 상태 보기가 오래됐을 수 있습니다. 이 행동에 의존하기 전에 새로 확인해야 합니다." | Current status/state, baseline, projection, 쓰기 전 범위 확인을 새로 확인합니다. Stale context는 refresh 또는 reconcile 전까지 pull-only input으로 다룹니다. | 상태 의존 next action / 예 / close가 stale fact에 의존하면 예. | Current state, freshness, valid Write Authorization, 근거 충분성, 작업 수락, 잔여 위험 상태, 닫기 준비 상태. |
| `unsupported_surface` | `CAPABILITY_INSUFFICIENT`; active가 아닌 stage/profile branch를 요청하면 `VALIDATION_FAILED` | 요청한 동작이 current stage, profile, connected surface capability 밖입니다. | "현재 단계나 접점에서 지원하지 않는 동작입니다. 여기서는 사용할 수 있는 기능처럼 취급할 수 없습니다." | 지원되는 fallback을 제시하거나, 요청을 줄이거나, 가능한/profile-promoted surface로 옮깁니다. Prose로 later-profile authority를 흉내 내지 않습니다. | 그 동작이 필요하면 예 / write에 필요하면 예 / close에 필요하면 예. | Active stage support, surface capability, stronger guarantee level, projection/job existence, 근거, QA, 작업 수락, 잔여 위험 수용, close support. |
| `out_of_scope` | `SCOPE_REQUIRED`, `SCOPE_VIOLATION`, `NO_ACTIVE_CHANGE_UNIT`, `AUTONOMY_BOUNDARY_EXCEEDED`, 또는 `BASELINE_STALE` | 제안된 행동이나 쓰기가 현재 범위 밖이거나 compatible scoped work boundary가 없습니다. | "현재 범위 밖입니다. 행동을 줄이거나 사용자에게 범위를 업데이트할지 물어볼 수 있습니다." | 영향을 받는 행동을 보류합니다. Mismatch를 보여 주고, 현재 범위로 줄이거나, 구체적인 사용자 소유 scope judgment를 요청합니다. | 영향을 받는 next action / 예 / unresolved scope가 close에 영향을 주면 예. | Scope expansion, non-goal removal, Write Authorization, 민감 동작 permission, user judgment. |
| `missing_judgment` | `DECISION_REQUIRED`, `DECISION_UNRESOLVED`, `APPROVAL_REQUIRED`, `APPROVAL_DENIED`, `APPROVAL_EXPIRED`, 또는 `ACCEPTANCE_REQUIRED` | 사용자 소유 판단이 필요하거나 기존 판단을 사용할 수 없습니다. | "사용자 판단이 필요합니다. 이 판단 없이는 계속할 수 없습니다." | 선택지, 결과, 불확실성, 영향을 받는 ref를 담은 집중된 판단 요청을 합니다. 제품/UX 판단, 기술 판단, 민감 동작 승인, 작업 수락, 잔여 위험 수용을 분리합니다. | 의존하는 next action / write가 이에 의존하면 예 / close가 이에 의존하면 예. | 사용자의 결정, 민감 동작 승인, 작업 수락, 잔여 위험 수용, waiver, 모호한 문구에서 나온 broad consent. |
| `missing_evidence` | `EVIDENCE_INSUFFICIENT`, `VERIFY_NOT_DETACHED`, `QA_REQUIRED`, 또는 `ARTIFACT_MISSING` | 필요한 근거, 분리 검증, 수동 QA, artifact support가 없거나, 오래됐거나, 막혔거나, 부족합니다. | "근거가 부족합니다. 그 주장을 뒷받침하려면 추가 확인이나 기록이 필요합니다." | Agent가 할 수 있으면 빠진 확인을 실행하거나 기록합니다. 그렇지 않으면 gap, 영향을 받는 claim, 가장 작은 해소 방법을 보여줍니다. | 근거 의존 next action / evidence가 write precondition이면 예 / close가 evidence에 의존하면 예. | 근거, test result, QA, artifact integrity, verification independence, sufficiency, 닫기 준비 상태. |
| `close_blocked` | `CloseTaskResponse.close_state=blocked`와 precedence로 선택된 primary `ErrorCode` | 현재 계약에서는 작업을 닫을 수 없습니다. | "닫기가 막혀 있습니다. 현재 계약에서 닫으려면 먼저 해소해야 할 일이 있습니다." | Blocker, 관련 ref, 가장 작은 해소 방법을 보여줍니다. 근거, QA, 작업 수락, 잔여 위험 표시, 잔여 위험 수용을 하나의 claim으로 합치지 않습니다. | Next action은 unblocker / blocker가 write를 요구할 때만 예 / 예. | Closed terminal state, 닫기 준비 상태, 작업 수락, 잔여 위험 수용, verification, QA, final report authority. |
| `residual_risk_present` | Status condition; acceptance 또는 close를 막을 때 `RESIDUAL_RISK_NOT_VISIBLE`, `DECISION_REQUIRED`, 또는 `DECISION_UNRESOLVED` | 알려진 잔여 위험이 있으며 보여줘야 합니다. 어떤 context에서는 명시적 잔여 위험 수용이 필요합니다. | "잔여 위험이 남아 있습니다. 이를 명시적으로 보여드리겠습니다. 닫기 전에는 별도의 수용 판단이 필요할 수 있습니다." | Risk, impact, ref, acceptance 필요 여부를 보여줍니다. Close 또는 acceptance path가 요구할 때만 잔여 위험 수용을 묻습니다. | 관련 risk-sensitive next action / risk가 scope나 safety를 바꾸면 예 / 보이지 않거나 required acceptance가 없으면 예. | No-risk status, 숨겨진 risk, accepted risk, 작업 수락, 닫기 준비 상태. |

Core unavailable rule: Harness/Core authority를 사용할 수 없으면 agent는 Task 상태, 민감 동작 승인, 사용자 판단, 근거, 작업 수락, 잔여 위험 수용, 닫기 준비 상태를 만들어 내면 안 됩니다. Authority를 사용할 수 없다고 보고할 수만 있으며, 사용자가 그 방식을 명시적으로 선택한 경우에만 하네스 밖에서 진행할 수 있습니다.

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
| `CAPABILITY_INSUFFICIENT` | Connected surface는 valid하지만 required validator, feature, enforcement condition, MVP-1 behavior를 충족할 수 없습니다. |
| `MCP_UNAVAILABLE` | Required MCP/Core access가 unavailable, stale, unreachable입니다. |
| `LOCAL_ACCESS_MISMATCH` | Core 또는 operator가 caller의 local access mode를 registered local profile 밖으로 분류할 수 있거나, required local access가 그 profile에서 denied됩니다. |
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

`WRITE_AUTHORIZATION_REQUIRED`와 `WRITE_AUTHORIZATION_INVALID`는 missing 또는 invalid Write Authorization record에만 사용합니다. Observed paths, tools, commands, network targets, secrets, sensitive categories가 Write Authorization record 또는 active scope를 넘으면 scope problem은 여전히 `SCOPE_VIOLATION`을 사용합니다.

MCP availability, local access/profile mismatch, capability insufficiency는 서로 다릅니다.

- `MCP_UNAVAILABLE`: Core에 닿을 수 없거나 required MCP access가 stale/unusable입니다.
- `LOCAL_ACCESS_MISMATCH`: reachable local endpoint 또는 caller path가 off-profile, stale, weak, forwarded/tunneled, cross-user, unauthorized, 또는 mismatched입니다.
- `CAPABILITY_INSUFFICIENT`: caller는 recognized surface/profile에 있지만 required capability, validator, enforcement condition을 충족할 수 없습니다.

## 사용자 표시 라벨

아래 label은 display guidance이지 새 public error code가 아닙니다.

| API condition | User-facing label | Smallest unblocker language |
|---|---|---|
| `VALIDATION_FAILED` | invalid request | Retry 전에 payload, enum value, activation rule, profile-specific field set을 고칩니다. |
| `STATE_CONFLICT` | state conflict | Current status를 refresh한 뒤 current state version으로 retry하거나 original idempotent request를 replay합니다. |
| `MCP_UNAVAILABLE` | Core unavailable | State change, gate update, projection repair, 쓰기 전 범위 확인 호환성, close를 주장하기 전에 Core access를 reconnect 또는 diagnose합니다. |
| `LOCAL_ACCESS_MISMATCH` | local access denied or off-profile | Registered local surface/profile로 reconnect하거나, local binding/profile을 repair하거나, 필요한 local access가 있는 surface를 사용합니다. |
| `CAPABILITY_INSUFFICIENT` | unsupported or insufficient surface | Capable surface/profile을 사용하거나, operation을 줄이거나, missing capability가 필요 없는 path를 선택합니다. |
| `NO_ACTIVE_TASK` | no active Task | Task-scoped action을 사용하기 전에 Task를 select 또는 create합니다. |
| `WRITE_AUTHORIZATION_REQUIRED`, `WRITE_AUTHORIZATION_INVALID` | 쓰기 전 범위 확인 없음 또는 오래됨 | Exact intended operation, current scope, current state로 `harness.prepare_write`를 call 또는 retry합니다. |
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

Visible-but-unaccepted close-relevant risk는 `RESIDUAL_RISK_NOT_VISIBLE`로 반환하지 않습니다. Requested close path가 risk acceptance를 요구하면, residual-risk acceptance user judgment를 새로 request해야 할 때 public close/API response는 primary `DECISION_REQUIRED`를 사용합니다. Relevant residual-risk acceptance user judgment가 있지만 pending, rejected, blocked, stale, deferred without coverage, incompatible이면 `DECISION_UNRESOLVED`를 사용합니다. Structured close blocker category는 `residual_risk_acceptance`여야 하며, MVP-1에서는 relevant `blocker`와 `user_judgment` record refs를 포함합니다. Rich `residual_risk` ref는 later/profile-promoted입니다.

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
