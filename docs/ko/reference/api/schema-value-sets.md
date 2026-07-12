# API 값 집합

이 문서는 기준 범위의 지원되는 API 값 집합과 enum 형태 공개 값을 담당합니다. 예약된 값이나 지원 범위 밖 값을 이름 붙이는 것만으로 기준 범위가 넓어지지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 지원되는 공개 메서드 이름 값
- 지원되는 행위자 출처 값
- 지원되는 다음 행동 값
- API `response_kind`와 `effect_kind` 값
- 지원되는 작업 범주(`operation_category`) 값
- 공유 상태 참조에서 쓰는 기록/참조 판별 값
- 지원되는 생명주기, 닫기 상태, 증거 관찰 출처와 보장 수준, 쓰기 결정 범주, 판단 종류, 표시 형식, 필요 판단 위치, 판단 결과, 아티팩트 가림 처리, 아티팩트 무결성, 아티팩트 가용성 표시, `ValidatorResult.status`, `ValidatorResult.severity`, 보장 표시 등 API 값 집합
- 지원되는 `change_unit.operation` 값
- 지원되는 공개 `ValidatorResult.validator_id` 값의 경계
- 메서드 범위 사유 코드와 불투명 분류 문자열에 대한 값 집합 경계
- 지원되는 스키마 해석에 영향을 주는 모드 조건부 또는 예약 값 경계
- 렌더링된 라벨이 기준 스키마 값이 아니라는 규칙

이 문서는 담당하지 않습니다.

- 공개 `ErrorCode` 값과 우선순위: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md)
- 차단 사유 처리 경로: [API 차단 사유 처리 경로](blocker-routing.md). 이 문서는 차단 사유 범주 값만 담당합니다.
- 이 값을 쓰는 필드 형태: [API 코어 스키마](schema-core.md), [API 상태 스키마](schema-state.md), [API 아티팩트 스키마](schema-artifacts.md), [API 판단 스키마](schema-judgment.md)
- 메서드 동작: [API 메서드](methods.md)와 메서드 담당 문서
- 보안 보장 의미: [보안](../security.md)
- 지원 범위 밖 기능 승격: [범위 참조](../scope.md)

## 경계

이 문서가 지원 값으로 둔 값만 지원되는 API 값입니다.

- 모드 조건부 값은 사용하는 자리에서 연결 모드, User Channel, 로컬 관리, 또는 담당 문서가 정의한 조건을 이름 붙여야 합니다.
- 지원 목록 밖의 값은 기준 범위 API 값이 아닙니다.
- 지원 목록 밖의 이름을 적는 것만으로 기준 범위가 넓어지지 않습니다.
- 화면에 보이는 라벨은 표시 텍스트일 뿐이며, 이 문서의 기준 값을 대신하지 않습니다.
- API 예시는 스키마 담당 문서가 해당 필드를 명시적으로 자유 형식 표시 문자열, 불투명 식별자, 또는 불투명 분류 문자열로 정의하지 않는 한, 이 문서의 지원되는 enum 형태 값을 사용해야 합니다.
- 문자열 형태 필드는 스키마 담당 문서가 이 문서의 값 집합으로 연결할 때만 이 문서가 담당합니다. 불투명 식별자, 불투명 분류 문자열, 자유 형식 표시 문자열은 해당 스키마 또는 메서드 담당 문서에 남습니다.
- 메서드 예시가 불투명 사유 코드나 분류 문자열을 보여 주더라도 그 문자열이 지원되는 전역 값이 되지는 않습니다.

## 값 집합 찾기

| 값 계열 | 시작할 절 |
|---|---|
| 메서드, 행위자 출처, 다음 행동, 응답 분기, 작업 범주 | [메서드 이름 값](#method-name-values), [행위자 출처 값](#actor-source-values), [다음 행동 값](#next-action-values), [응답과 효과 값](#response-and-effect-values), [작업 범주 값](#operation-category-values) |
| 기록 참조, 프로젝트 연속성, `Task` 생명주기 | [기록과 참조 값](#record-and-reference-values), [프로젝트 연속성 값](#project-continuity-values), [`Task` 생명주기 값](#task-lifecycle-values) |
| 메서드별 요청과 결과 값 | [메서드 내부 값](#method-local-values) |
| 관찰 상태, 증거 상태, 차단 사유 범주 | [상태와 차단 사유 값](#state-and-blocker-values) |
| 증거 출처와 보장 수준 | [증거 관찰 값](#evidence-observation-values) |
| 아티팩트와 판단 값 | [아티팩트 값](#artifact-values), [판단 값](#judgment-values) |
| 오류 세부사항 보조 값과 기준 범위 밖의 값 | [오류 세부사항 보조 값](#error-detail-helper-values), [프로필 조건부 및 예약 값](#profile-gated-and-reserved-values) |

<a id="method-name-values"></a>
## 메서드 이름 값

지원되는 공개 메서드 이름 집합은 아래와 같습니다.

```text
volicord.intake
volicord.update_scope
volicord.status
volicord.get_operation_result
volicord.check_close
volicord.prepare_write
volicord.stage_artifact
volicord.record_run
volicord.request_user_judgment
volicord.record_user_judgment
volicord.record_user_observation
volicord.reconcile_changes
volicord.close_task
```

메서드 동작은 [API 메서드](methods.md)가 안내하는 메서드 담당 문서가 담당합니다. 메서드 이름은 `Task` 생명주기 값이 아닙니다.

<a id="actor-source-values"></a>
<a id="actor-values"></a>
## 행위자 출처 값

`EvidenceObservation.observed_by_actor_source`, `UserEvidenceObservation.observed_by_actor_source`, `EvidenceObservationInput.observed_by_actor_source`, `UserJudgmentResolution.resolved_by_actor_source` 같은 행위자 출처 필드는 `ActorSource` 값 집합을 사용합니다.

| 값 | 사용하는 곳 | 담당 문서 경로 |
|---|---|---|
| `local_user` | User Channel 호출 출처와 권한 효력이 있는 사용자 판단 결과 기록. | 호출 의미: [Agent Connection](../agent-connection.md). 결과 형태 담당 문서: [API 판단 스키마](schema-judgment.md). |
| `agent_connection:<connection_id>` | Agent Connection 호출 출처와 에이전트가 만들거나 관찰한 상태. | 호출 의미: [Agent Connection](../agent-connection.md). 중첩 형태 담당 문서가 값이 나타나는 위치를 정의합니다. |
| `system` | 담당 문서가 명시적으로 허용하는 내부 시스템 출처. | 메서드와 저장소 담당 문서가 값이 나타나는 위치를 정의합니다. |

이 값들은 파생된 호출 출처 또는 지속 행위자 출처를 분류합니다. 이 값만으로 사용자
소유 판단, 승인, 범위 결정 권한, 최종 수락, 잔여 위험 수락, 쓰기 티켓이 생기지는
않습니다. 사용자 판단을 권한 효력이 있는 결과로 기록하려면 [Agent
Connection](../agent-connection.md)과 메서드 담당 문서가 정의하는 호환 User
Channel 출처와 함께 `resolved_by_actor_source=local_user`가 필요합니다.

<a id="next-action-values"></a>
## 다음 행동 값

`NextActionSummary.presentation_role`은 아래의 제어 값만 사용합니다.

| 값 | 의미 |
|---|---|
| `primary` | 담당 문서가 구성한 행동 모음에서 주된 다음 행동으로 선택된 단일 행동입니다. |
| `additional` | 같은 모음에 함께 표시되는 다른 행동입니다. 여전히 필수일 수 있으며 선택 사항이라는 뜻이 아닙니다. |

`NextActionSummary.action_kind`는 제어되는 행동 범주 값입니다. 지원되는 값과 담당 문서가 지원하는 호출 범주는 아래와 같습니다.

| `action_kind` 값 | 메서드 하나가 다음 단계를 담당할 때의 `owner_method` | `allowed_operation_categories` |
|---|---|---|
| `update_scope` | `volicord.update_scope` | `agent_workflow` |
| `prepare_write` | `volicord.prepare_write` | `agent_workflow` |
| `stage_artifact` | `volicord.stage_artifact` | `agent_workflow` |
| `record_run` | `volicord.record_run` | `agent_workflow` |
| `request_user_judgment` | `volicord.request_user_judgment` | `agent_workflow` |
| `record_user_judgment` | `volicord.record_user_judgment` | `user_only` |
| `reconcile_changes` | `volicord.reconcile_changes` | `agent_workflow`, `local_recovery` |
| `close_task` | `volicord.close_task` | `agent_workflow` |

`action_kind`는 메서드 이름 값이 아닙니다. 지원되는 공개 메서드 하나가 다음 단계를 담당할 때 `NextActionSummary.owner_method`는 [메서드 이름 값 집합](#method-name-values)을 사용하고, 단일 담당 메서드가 없으면 `null`입니다. `owner_method=null`인 행동은 `allowed_operation_categories=[]`를 사용하며, 라벨과 포함하는 담당 문서가 필요한 호스트, 터미널, 파일시스템, 설정 작업을 식별합니다. 작업 범주 목록은 담당 문서가 지원하는 호출 경로를 설명할 뿐 현재 연결에서 사용할 수 있거나 권한이 부여됐음을 뜻하지 않습니다. 다음 단계의 메서드 동작은 [API 메서드](methods.md)가 안내하는 메서드 담당 문서에 둡니다. 전체 `NextActionSummary` 형태는 [API 상태 스키마](schema-state.md#current-position-display-shapes)가 담당합니다.

<a id="response-and-effect-values"></a>
## 응답과 효과 값

`ToolResultBase.response_kind`는 아래 값을 사용합니다.

```text
result
rejected
dry_run
```

`ToolResultBase.effect_kind`는 아래 값을 사용합니다.

```text
read_only
core_committed
staging_created
no_effect
```

`response_kind`와 `effect_kind`는 분기 메타데이터 값입니다. 공통 분기 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당하고, 메서드별 효과는 메서드 담당 문서가 담당합니다. 거절 분기의 공개 오류 의미는 [API 오류 코드](error-codes.md)와 [API 오류 처리 경로](error-routing.md)가 담당합니다.

<a id="opaque-and-method-scoped-string-fields"></a>
## 불투명 문자열과 메서드 범위 문자열 필드

아래 필드는 의도적으로 전역 닫힌 값 집합이 아닙니다.

| 필드 | 분류 | 담당 문서 경로 |
|---|---|---|
| `EventRef.event_kind` | 불투명 이벤트 분류 문자열입니다. 메서드 예시가 `event_kind` 문자열을 보여 줄 수 있지만, 이 문서는 빠짐없는 공개 `event_kind` 값 집합을 공개하지 않습니다. | 형태 담당 문서: [API 코어 스키마](schema-core.md#shared-support-shapes). 이벤트를 만드는 동작: 메서드 담당 문서. |
| `WriteDecisionReason.code` | 메서드 범위의 불투명 사유 코드입니다. 메서드 담당 문서는 전역의 빠짐없는 코드 목록을 만들지 않고 예시 코드를 보여 줄 수 있습니다. | 형태 담당 문서: [API 상태 스키마](schema-state.md#current-position-display-shapes). 생성과 로컬 의미: [`volicord.prepare_write`](method-prepare-write.md)와 영향받는 메서드 담당 문서. |

공개 `ErrorCode` 값은 별도이며 [API 오류 코드](error-codes.md)가 담당합니다.

<a id="operation-category-values"></a>
## 작업 범주 값

메서드별 API 호환성 점검은 공개 API 요청 하나마다 요청 수준 작업 범주
(`operation_category`) 하나를 사용합니다.

| 값 | 어휘 설명 |
|---|---|
| `read` | 읽기 전용 API 작업 범주입니다. `read_only` Agent Connection은 이 범주를 실행할 수 있습니다. |
| `agent_workflow` | 에이전트 작업 흐름용 API 작업 범주입니다. `workflow` Agent Connection은 이 범주와 `read`를 실행할 수 있습니다. |
| `user_only` | 권한 효력이 있는 사용자 동작을 위한 User Channel 작업 범주입니다. Agent Connection은 이 범주를 실행하지 않습니다. |
| `admin_local` | 로컬 관리 작업 범주입니다. Agent Connection은 이 범주를 실행하지 않습니다. |
| `local_recovery` | `volicord.reconcile_changes` 같은 메서드별 복구 경로를 위한 로컬 사용자 복구 작업 범주입니다. Agent Connection은 이 범주를 실행하지 않습니다. |

작업 범주는 Volicord API 호환성 분류입니다. OS 권한, 파일시스템 ACL,
샌드박스 규칙, 네트워크 정책, 비밀 격리가 아닙니다. 메서드별 동작 요구사항은
[API 메서드](methods.md)가 안내하는 메서드 담당 문서가 담당합니다. Agent
Connection 호출 검증 동작은 [Agent Connection](../agent-connection.md)과
[보안](../security.md)이 담당합니다.

<a id="record-and-reference-values"></a>
## 기록과 참조 값

`StateRecordRef.record_kind`는 아래 값을 사용합니다.

```text
project_state
task
change_unit
write_ticket
user_judgment
run
evidence_summary
evidence_observation
user_evidence_observation
artifact
blocker
task_event
agent_connection
project_continuity_record
unrecorded_change
```

이 값들은 API 참조 종류를 식별합니다. 저장소 테이블 이름, DDL, Core 권한 의미, 메서드별 담당 규칙을 대신하지 않습니다.

<a id="project-continuity-values"></a>
## 프로젝트 연속성 값

`ProjectContinuityRecord.kind`와 `ProjectContinuitySummary.kind`는 아래 값을 사용합니다.

```text
decision
obligation
known_limit
accepted_risk
constraint
```

`ProjectContinuityRecord.status`와 `ProjectContinuitySummary.status`는 아래 값을 사용합니다.

```text
active
superseded
closed
```

이 값들은 오래 유지하는 프로젝트 수준 맥락을 분류합니다. 그 자체로 현재 `Task` 권한을 만들거나, 대기 중인 사용자 판단을 만족하거나, 증거를 증명하거나, 쓰기 티켓 권한을 부여하거나, 닫기 준비 상태를 만족하거나, 미래 닫기 근거의 잔여 위험을 수락하지 않습니다.

<a id="task-lifecycle-values"></a>
## `Task` 생명주기 값

`StateSummary.mode`와 확정된 `Task.mode` 필드는 아래 값을 사용합니다.

```text
advisor
direct
work
```

`volicord.intake`의 `requested_mode`는 입력 전용 값으로 `auto`도 받습니다. 출력 `Task.mode` 필드는 `advisor`, `direct`, `work`를 사용합니다. 접수 확정 동작은 [접수 메서드](method-intake.md)가 담당합니다.

모드와 `work_phase`가 함께 Run 종류 호환성을 제한합니다.

| `Task.mode` | `work_phase` | 허용되는 `RunKind` | 성공한 `intent=complete` 결과 |
|---|---|---|---|
| `advisor` | `shaping` | `shaping_update` | `advice_only` |
| `direct` | `implementation` | `direct` | `completed` |
| `work` | `shaping` | `shaping_update` | `completed` |
| `work` | `implementation` | `implementation` | `completed` |

`advisor`는 Product Repository 파일 효과에 대해 읽기 전용인 자문 작업입니다. `prepare_write`나 쓰기 티켓을 사용하지 않으며, 호환되는 실행 기록은 `product_file_write_observed=false`, 빈 `changed_paths` 목록, `write_ticket_id=null`을 가집니다. 호환되는 `shaping_update`는 `record_run`이 Run과 메서드 소유 Core 증거 상태를 커밋하는 것을 허용합니다.

`StateSummary.work_phase`와 `TaskFlowItem.work_phase`는 아래 값을 사용합니다.

```text
shaping
implementation
```

이 단계는 한 Task의 장기 outcome을 유지하면서 분석과 쓰기 가능한 실행을
구분합니다. `lifecycle_phase`와는 독립된 필드입니다.

`StateSummary.acceptance_policy`는 아래 값을 사용합니다.

```text
required
not_required
policy_dependent
```

정책과 이유는 intake에서 선택합니다. `not_required`는 advisor Task에만 사용할 수
있고 `policy_dependent`는 닫기 담당 규칙이 평가하며 에이전트가 고르는 면제가
아닙니다.

`TaskLineageSummary.relation`은 아래 값을 사용합니다.

```text
continues
derived_from
split_from
replaces
implements_advice_from
```

`CarryForwardDisposition.kind`는 아래 값을 사용합니다.

```text
scope
non_goals
user_decisions
source_refs
context_refs
known_limitations
unresolved_obligations
residual_risks
baseline
```

`status`는 `applied` 또는 `reference_only`입니다. 적용한 material은 새 Task
입력으로 다시 검증합니다. Reference-only 맥락은 predecessor 범위, 판단,
Evidence, 수락, 위험 수락, 쓰기 권한을 다시 현재 상태로 만들지 않습니다.

`WorkspaceContext.vcs`는 현재 `git`만 사용하고 `branch_ref=null`은 detached
HEAD를 뜻합니다. `AuthorityReceipt.next_actor`는 `agent`, `user`, `none`을
사용합니다.

`Task.lifecycle_phase`는 아래 값을 사용합니다.

```text
shaping
ready
executing
waiting_user
blocked
completed
cancelled
superseded
```

사용자 소유 판단 대기의 생명주기 의미:

- `waiting_user`는 현재 종료되지 않은 `Task`에 현재 호환되는 근거 상태와 정보성이 아닌 작업 대상을 가진 대기 사용자 판단이 하나 이상 있고 사용자 답변이 아직 필요하다는 뜻입니다. 여기에는 `product_decision`과 `technical_decision`도 포함되며 Core가 만든 권한 선택지를 사용하는 판단 종류로 한정되지 않습니다.
- 정보성 판단과 근거 상태가 오래됐거나 대체된 대기 판단은 `waiting_user`를 만들거나 유지하지 않습니다.
- 마지막 해당 대기 판단을 해결하면 현재 적용 Change Unit이 있을 때 `ready`로, 없을 때 `shaping`으로 돌아갑니다. 여러 해당 판단 중 하나만 해결하면 `waiting_user`를 유지합니다.
- `completed`, `cancelled`, `superseded`는 종료 상태이며 판단 생명주기 유지 작업으로 바뀌지 않습니다.

`CloseTaskResult.close_state`는 아래 값을 사용합니다.

```text
ready
blocked
closed
cancelled
superseded
```

`StatusResult.close_state`는 현재 닫기 상태가 없을 때 `none`도 허용합니다.

`Task.close_reason`은 아래 값을 사용합니다.

```text
none
completed_self_checked
completed_with_risk_accepted
cancelled
superseded
```

`Task.result`는 아래 값을 사용합니다.

```text
none
advice_only
completed
cancelled
superseded
```

실행 실패, 위반, 차단된 닫기, 증거 공백은 종료 `Task.result` 값이 아닙니다.

<a id="method-local-values"></a>
## 메서드 내부 값

MCP mutation 인자 `detail`은 아래 값을 사용합니다.

```text
summary
workflow
full
```

기본값 `summary`는 새 authority receipt와 간결한 메서드 결과를 결합한 래퍼이고,
`workflow`는 정규 다음 행동을 추가하며, `full`은 새 receipt와 크기가 제한된 정확한 메서드
결과를 결합합니다. Transport는 호환 text member를 유지하지만 전체 JSON을 중복한 문서가
아니라 크기가 제한된 요약을 사용합니다.

`volicord.intake`의 `resume_policy`는 아래 값을 사용합니다.

```text
resume_active
create_new
supersede_active
reject_if_active
```

`change_unit.operation`은 아래 값을 사용합니다.

```text
keep_current
create_current
replace_current
```

값 의미:
- `keep_current`는 현재 적용 Change Unit을 바꾸지 않고 범위 관련 `Task` 필드를 갱신합니다.
- `create_current`는 알맞은 현재 적용 Change Unit이 없을 때 현재 적용 Change Unit을 만듭니다.
- `replace_current`는 현재 적용 Change Unit을 새 작업 경계로 교체합니다.

각 `operation` 값의 메서드 동작은 [`volicord.update_scope`](method-update-scope.md)가 담당합니다. API 예시와 스키마 독자가 하나의 기준 값 담당 문서를 볼 수 있도록 지원 값 집합은 이 문서에 둡니다.

`ChangeUnitEffectContract.allowed_effects`와 `ChangeUnitEffectContract.forbidden_effects`는 아래 값을 사용합니다.

```text
product_file_write
artifact_registration
run_recording
user_judgment_request
evidence_update
sensitive_action
external_network
secret_access
```

이 값들은 효과를 Core 상태로 분류합니다. 값 자체가 런타임 샌드박스, 명령 가로채기, 네트워크 차단, 비밀 격리, 사용자 판단, 민감 동작 승인, 증거, 쓰기 티켓, 최종 수락, 닫기 준비 상태, 잔여 위험 수락을 만들지는 않습니다.

`volicord.check_close`에는 `intent` 필드가 없습니다. `volicord.close_task.intent`는 아래 값을 사용합니다.

```text
complete
cancel
supersede
```

`PrepareWriteResult.decision`은 아래 값을 사용합니다.

```text
allowed
blocked
approval_required
decision_required
```

`PrepareWriteResult.write_ticket_effect`는 아래 값을 사용합니다.

```text
none
would_issue
issued
```

`issued`는 커밋된 `decision=allowed` 결과가 열린 쓰기 티켓 권한 기록 하나를 발급했다는 뜻입니다. `would_issue`는 미리보기나 계획 설명에서만 쓰이며 커밋된 티켓을 만들지 않습니다.

`WriteTicket.state`는 아래 값을 사용합니다.

```text
open
observed
reconciled
closed
expired
revoked
```

이 상태는 Volicord 티켓 권한과 관찰 생명주기를 설명합니다. 파일시스템 ACL, OS 수준 집행, 셸 권한, 명령 승인, 쓰기가 실제로 일어났다는 증명을 뜻하지 않습니다.

`WriteTicketStateSummary.status`는 아래 값을 사용합니다.

```text
active
consumed
expired
stale
revoked
```

`RecordRunRequest.kind`와 `RunSummary.kind`는 아래 값을 사용합니다.

```text
shaping_update
implementation
direct
```

위 Task 모드 호환성 행렬은 완전한 목록입니다. 이 값에는 호환 별칭이 없으며, 호환되지 않는 모드와 종류 조합은 기록되지 않습니다.

<a id="state-and-blocker-values"></a>
## 상태와 차단 사유 값

`CloseReadinessBlocker` 객체 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다.

이 절은 차단 사유 범주 값인 `CloseReadinessBlocker.category`와 인접 상태/차단 사유 값을 담당합니다.

`PlannedBlocker.source_kind`는 아래 값을 사용합니다.

```text
write_decision
close_readiness
```

`IntegrationProfile`과 `GuardHealthSummary.selected_profile`은 아래 값을 사용합니다.

| 값 | 의미 |
|---|---|
| `record` | 호스트 훅이나 세션 감시기 관찰을 요구하지 않고 권한 상태를 기록하며 MCP 도구 작업 흐름을 노출합니다. Core가 발급한 권한 쓰기 티켓도 포함합니다. |
| `detective` | 쓰기 티켓 범위와 연결할 수 있는 지원되는 호스트 훅과 세션 감시기 관찰을 추가합니다. 협력형 호스트 경고나 거부를 반환하고, 감시 범위가 시작된 뒤의 미기록 Product Repository 변경을 탐지할 수 있습니다. 행위자 신원을 증명하거나, OS 강제를 제공하거나, 네트워크를 격리하거나, 도구를 샌드박스에 격리하지는 않습니다. |

`GuardHealthSummary.hook_path_safety`는 아래 값을 사용합니다.

```text
ok
not_recorded
metadata_missing
authority_mismatch
policy_hash_mismatch
host_output_mismatch
relative_path_unsafe
absolute_path_stale
placeholder_unsupported
dispatch_missing
wrapper_missing
wrapper_not_executable
```

`ok`는 모든 필수 호스트 훅 명령이 현재 작업 디렉터리와 무관하고 하위
디렉터리에서도 안전하게 기록되어 있으며, 예상한 관리 래퍼 경로로 해석된다는
뜻입니다. 실패 값은 이 조건을 만족하지 못한 주된 이유를 보고합니다. 여기에는
세션의 현재 작업 디렉터리에 의존하는 상대 명령, 오래된 절대 프로젝트 루트,
지원되지 않는 자리표시자, 누락된 디스패치 또는 래퍼 스크립트, 지원되는 Unix 계열
플랫폼에서 실행할 수 없는 래퍼 스크립트, 생성된 래퍼 메타데이터 불일치, 누락된
검증 메타데이터가 포함됩니다.

`relative_path_unsafe`에는 호스트 세션의 현재 작업 디렉터리를 기준으로 해석되는
`.codex/hooks/...`, `./.codex/hooks/...`, `.claude/hooks/...`,
`./.claude/hooks/...` 명령이 포함됩니다. `ok`가 아닌 `hook_path_safety` 값은
`detective` 호스트 훅을 `inactive`로 유지합니다.

`GuardHealthSummary.guard_installation_status`는 아래 값을 사용합니다.

```text
absent
configured
reload_required
active
degraded
stale
broken
```

`GuardHealthSummary.guard_configuration_status`는 아래 값을 사용합니다.

```text
absent
configured
reload_required
degraded
stale
broken
```

`GuardHealthSummary.guard_observation_status`는 아래 값을 사용합니다.

```text
not_observed
observed
stale_observation
```

`GuardHealthSummary.effective_guard_status`는 아래 값을 사용합니다.

```text
inactive
action_required
active
degraded
broken
```

`GuardHealthSummary.prompt_capture_status`는 아래 값을 사용합니다.

```text
unavailable
unsupported_by_host
not_configured
reload_required
configured
observed
active
degraded
```

`GuardHealthSummary.session_watch_status`는 아래 값을 사용합니다.

```text
disabled
active
degraded
unavailable
pending_project_selection
```

`GuardHealthSummary.session_watch_coverage_basis`는 아래 값을 사용합니다.

```text
mcp_start
first_project_selection
method_boundary
```

`CoverageSummary.host_hook_state`는 아래 값을 사용합니다.

```text
observed
not_observed
unsupported
degraded
```

`CoverageSummary.session_watcher_state`는 아래 값을 사용합니다.

```text
active
inactive
unsupported
degraded
```

아래 필드는 `detective` 호스트 훅의 설정, 관찰, 유효한 닫기 준비 상태를
구분합니다.

- `guard_installation_status`는 저장된 설치 생명주기 값입니다.
- `guard_configuration_status`는 파일과 필수 훅 설정이 완전한지를 나타냅니다.
- `guard_observation_status`는 현재 설치에 일치하는 훅 관찰이 있는지를 나타냅니다.
- `effective_guard_status`는 `detective` 경로의 닫기 준비 상태에 쓰는 값입니다. `active`가 되려면 `detective` 프로필, 완전한 필수 훅 설정, 오래되거나 깨지지 않은 설치, 현재 일치하는 관찰, 일치하는 호스트와 정책 식별 정보가 필요합니다.

`prompt_capture_status`는 사용자 소유 판단 채팅 명령을 사용할 수 있는지를
보고합니다.

- `unsupported_by_host`: 호스트 기능이 없습니다.
- `not_configured`: 선택된 연결에 프롬프트 캡처 단계가 설정되지 않았습니다.
- `reload_required`: 사용 전에 설치 설정이나 정책 식별 정보를 다시 읽어야 합니다.
- `configured`: 프롬프트 캡처 관찰 전에도 검증 코드 채팅 명령을 표시할 수 있습니다.
- `observed`: 일치하는 호스트 훅이 관찰되었습니다.
- `active`: 일치하는 프롬프트 캡처 훅 관찰이 기록되었습니다.
- `degraded`: 저하된 `detective` 호스트 훅 상태 때문에 프롬프트 캡처가 차단됩니다.

`session_watch_status`는 `detective` 세션 감시기의 사용 가능 상태를 보고합니다.

- `disabled`: 선택된 세션 감시 기준선을 사용할 수 없습니다.
- `active`: 한정된 스냅샷 비교를 사용할 수 있습니다.
- `degraded`: 감시 결과가 부분적이거나 운영자 조치가 필요합니다.
- `unavailable`: 감시기가 선택된 스냅샷 확인을 수행할 수 없었습니다.

`CoverageSummary.host_hook_state`와
`CoverageSummary.session_watcher_state`는 사람이 읽는 상태 조회와 닫기 준비 상태
출력을 위한 간결한 파생 상태입니다. 자세한 `GuardHealthSummary` 필드를 대신하지
않습니다.

이 값들은 제품 정확성, 테스트 충분성, OS 강제, 샌드박싱, 보안 격리, 최종 수락,
잔여 위험 수락, 행위자 귀속, 전체 파일시스템 감시, 완전한 쓰기 방지를 증명하지
않습니다. `record` 프로필은 협력형입니다. 닫기 준비 상태가 해결되지 않은 미기록
변경을 보고하면 그 변경은 닫기를 막습니다.

관찰 범위 시점 값의 의미는 다음과 같습니다.

- `pending_project_selection`: MCP 세션에 사용할 수 있는 프로젝트가 둘 이상이고, 세션 감시 기준선을 만들 만큼 프로젝트를 명시적으로 선택하지 않은 상태입니다.
- `mcp_start`: 프로젝트에 연결된 시작 또는 HTTP 세션 초기화에서 MCP 도구 처리 전에 감시 범위가 시작됩니다.
- `first_project_selection`: 여러 프로젝트를 다루는 세션이 명시적인 `project_selector`를 처음 지정할 때 감시 범위가 시작됩니다.
- `method_boundary`: Core 메서드 경계의 대체 지점에서 감시 범위가 시작됩니다.

`first_project_selection`과 `method_boundary`는 부분 관찰 범위 근거입니다. 기록된
관찰 시작 전의 Product Repository 변경은 감시 범위 밖에 있습니다.

`UnrecordedChangeFinding.status`는 아래 값을 사용합니다.

```text
unresolved
resolved
```

<a id="unrecorded-change-resolution-basis-values"></a>
`UnrecordedChangeResolutionSummary.resolution_basis`와 저장된 미기록 변경 해결 메타데이터는 아래 값을 사용합니다.

```text
reverted
covered_by_write_ticket
recorded_as_expected_write
accepted_by_user
not_product_change
superseded_by_new_observation
invalid_observation
```

이 값들은 미기록 Product Repository 변경이 해결된 이유를 분류합니다. 제품 정확성,
증거 충분성, 검토 완료, 최종 수락, 잔여 위험 수락, 보안을 증명하지 않습니다.
호출자 사용은 [`volicord.reconcile_changes`](method-reconcile-changes.md)가
제한합니다. 해결 근거의 이름만으로 에이전트가 변경을 단독으로 무시할 수는
없습니다.

`WriteDecisionReason.category`는 제어되는 범주 값입니다. 지원되는 값은 아래 값 집합뿐입니다.

| 값 | 범주 계열 |
|---|---|
| `scope` | 범위 호환성 또는 범위 경계 사유. |
| `user_judgment` | 필요한 사용자 소유 판단 사유. |
| `sensitive_approval` | 필요한 별도 민감 동작 승인 사유. |
| `write_compatibility` | 쓰기 호환성 사유. |
| `baseline` | 기준선 호환성 사유. |
| `effect_contract` | Change Unit 효과 계약 호환성 사유. |
| `connection_capability` | Agent Connection 호환성 또는 모드 지원 사유. |

이 범주는 `volicord.prepare_write` 결정 사유를 분류합니다. `CloseReadinessBlocker` 객체가 아니며 닫기 준비 상태를 평가하지 않습니다. 메서드별 결정 동작과 사유 생성은 [`volicord.prepare_write`](method-prepare-write.md)에 둡니다.

이 값 집합은 `category`만 제어합니다. `WriteDecisionReason.code`는 전역에서 쓰는
고정 값 집합이 아닙니다. 메서드 범위의 불투명 사유 코드이므로, 메서드 담당 문서가
예시 코드를 보여 주더라도 전역 지원 목록에 추가되는 것은 아닙니다. `message`는
자유 형식 표시 문자열이고, `related_refs`는 `StateRecordRef`를 사용합니다.

`CloseReadinessBlocker.category`는 아래 값을 사용합니다.

```text
task
open_run
scope
user_judgment
pending_user_judgment
sensitive_approval
write_compatibility
baseline
connection_capability
evidence
evidence_claim
evidence_provenance
artifact_availability
final_acceptance
residual_risk_visibility
residual_risk_acceptance
recovery
```

`EvidenceSummary.status`는 아래 값을 사용합니다.

```text
unknown
insufficient
sufficient
blocked
```

`StageArtifactResult.evidence_state`와 `EvidenceSummary.evidence_state`는 해당 필드가 있을 때 아래 증거 첨부 표시 상태 값을 사용합니다.

```text
prepared
attached
accepted_for_close
```

이 값들은 사용자에게 보이는 표시 상태입니다. `accepted_for_close`는 증거가 현재 닫기 준비 상태 계산에 사용될 수 있다는 뜻입니다. 정확성 증명, 테스트 충분성 증명, QA 결과, 최종 수락, 잔여 위험 수락이 아닙니다.

<a id="evidence-gate-values"></a>
### 증거 gate 값

`EvidenceGateSummary.state`와 선택된 `SummaryCard.evidence`는 정확히 아래 값을 사용합니다.

```text
not_required
optional_none
required_missing
partial
sufficient
stale
blocked
```

| 값 | 의미 |
|---|---|
| `not_required` | 활성 기준 중 증거가 `required` 또는 `optional`인 기준이 없습니다. 기준이 없거나 모두 `not_required`인 집합이 이 값을 사용합니다. |
| `optional_none` | 활성 기준 중 하나 이상이 `optional`이고 `required`는 없으며, 선택적 기준에 기록된 증거 지원이 없습니다. |
| `required_missing` | 활성 기준 중 하나 이상이 `required`이고, 필요한 기준 어느 것에도 기록된 증거 지원이 없습니다. |
| `partial` | 필요한 증거 지원이 일부 있지만 필요한 집합이 충분하지 않거나, 선택적 기준만 있는 상태에서 기록된 증거 항목이 모두 `supported`는 아닙니다. |
| `sufficient` | 모든 필요한 기준이 정확히 `supported`이고 증거 주장, 출처, 아티팩트 가용성 차단 사유가 없습니다. 선택적 기준만 있으면 기록된 지원이 있는 선택적 항목이 모두 `supported`입니다. |
| `stale` | 필요한 증거나 그 출처가 현재 닫기 근거에 비해 오래되었고 더 높은 우선순위의 증거 차단 상태는 없습니다. |
| `blocked` | 필요한 기준이 모순되었거나, 오래됨 이외의 증거 또는 출처 조건이 증거 gate를 차단하거나, 사용할 수 없는 아티팩트 차단 사유가 필요한 기준을 뒷받침하는 아티팩트를 가리킵니다. |

Core는 활성 기준의 요구 수준과 범위, 기준 증거 관찰의 최신성과 출처, 필요한 기준을 뒷받침하는 기준 아티팩트의 가용성, 증거 관련 닫기 차단 사유를 사용해 이 파생 상태 보기를 한 번 계산합니다. `blocked`가 `stale`보다 우선하고, 그 다음 필요한 범위에 따라 `sufficient`, `partial`, `required_missing`을 선택합니다. `optional`과 `not_required` 기준은 닫기 차단 사유를 만들지 않으며 충분한 필수 gate를 낮추지 않습니다. 필요한 기준을 뒷받침하지 않는 닫기 근거 결과 아티팩트의 가용성 차단 사유를 포함해 증거와 무관한 닫기 차단 사유는 증거 gate를 바꾸지 않습니다. 계산 결과는 status와 close 결과, `StateSummary.evidence_gate`, `SummaryCard.evidence`에 복사되며 첨부 표시 상태가 별도 gate 계산이 되지 않습니다.

`AcceptanceCriterion.evidence_requirement`, intake 기준 입력, update-scope
기준 교체 입력은 아래 값을 사용합니다.

```text
required
optional
not_required
```

현재 기준 중 `required`만 증거 닫기 차단 사유를 만들 수 있습니다.

`EvidenceTarget.target_kind`는 아래 값을 사용합니다.

```text
acceptance_criterion
supplemental_claim
```

`EvidenceCoverageUpdate.coverage_state`는 아래 값을 사용합니다.

```text
unsupported
partial
supported
contradicted
not_applicable
```

커밋된 `EvidenceCoverageItem.coverage_state`는 같은 값과 아래 값을 추가로
사용할 수 있습니다.

```text
stale
```

<a id="evidence-observation-values"></a>
## 증거 관찰 값

`EvidenceUpdateProvenance.source_kind`, `EvidenceObservation.source_kind`, `EvidenceObservationInput.source_kind`는 아래 값을 사용합니다.

```text
agent_report
connection_observation
external_tool
user_observation
reused_evidence
unverified_claim
```

`EvidenceUpdateProvenance`와 `EvidenceObservationInput`에서 이 값은 요청한 출처
분류입니다. 커밋된 `EvidenceObservation`에서는 Core가 파생한 분류입니다. 유효한
요청 조합 자체가 보장 수준을 부여하지는 않습니다.

출처 종류 의미:
- `agent_report`는 에이전트 행위자 맥락이 만든 보고를 기록합니다. 그 자체로 외부 도구 결과가 아닙니다.
- `connection_observation`은 대상별 확인된 연결 관찰 기록으로 뒷받침되는 관찰을 이름 붙입니다. 기준 범위의 직접 `record_run` 경로에는 이런 기록이 없으므로 요청된 값을 `agent_report`로 강등합니다.
- `external_tool`은 정확한 출력과 결합된 authority-owned verified tool 또는
  command producer 레코드를 요구합니다. 기준 구현에는 아직 해당 producer
  전이가 없으므로 직접 요청은 강등됩니다. 검증된 아티팩트 바이트만으로는
  충분하지 않습니다.
- `user_observation`은 `volicord.record_user_observation`이 만든 현재의 대상 결합
  `UserEvidenceObservation`으로 뒷받침되는 관찰입니다. 앵커 없는 직접 선택은
  강등되며 최종 수락이나 다른 권한 효력이 있는 판단은 아닙니다.
- `reused_evidence`는 Core가 검증한 이전 강한 관찰의 재사용을 기록합니다. 호출자가 직접 선택한 값은 강등되며 검증된 재사용 자체도 새 관찰은 아닙니다.
- `unverified_claim`은 확인된 관찰 없는 주장을 보존합니다. 그 자체로 충분한 증거가 아닙니다.

`EvidenceUpdateProvenance.assurance_level`, `EvidenceObservation.assurance_level`, `EvidenceObservationInput.assurance_level`은 아래 값을 사용합니다.

```text
cooperative_report
registered_connection_observed
external_tool_result
user_observed
unverified
```

보장 수준 의미:
- `cooperative_report`는 제출 행위자 맥락의 협력형 보고입니다.
- `registered_connection_observed`에는 대상별 확인된 연결 관찰 앵커가 필요합니다. 현재 Agent Connection 호출만으로 파생되지 않습니다.
- `external_tool_result`에는 authority-owned producer 레코드, 정확한 정규 출력
  결합, 현재 바이트, 분리된 supported relevance 평가가 필요합니다.
- `user_observed`에는 현재의 대상별 User Channel 관찰, 정확한 출력,
  `relevance_status=supported`가 필요합니다.
- `unverified`는 확인된 관찰 보장 수준이 없음을 기록합니다.

Core는 필요한 앵커가 없는 요청된 강한 조합을 `agent_report` /
`cooperative_report`로 강등합니다. `reused_evidence`에서는 원래 identity, 대상,
`Task`와 Change Unit, 출처 실행 기록, 범위 리비전, 기준선, 승계한 보장 수준,
정확한 출력, producer 앵커, relevance 평가를 각 재귀 단계에서 다시
확인합니다. 이 값들은 사용자 권한을 부여하거나, 최종 수락 또는 잔여 위험 수락을
만족하거나, 제품 정확성을 증명하거나, `GuaranteeDisplay.level`을 바꾸지 않습니다.

`EvidenceProducerAnchor.producer_kind`는 다음 값을 사용합니다.

```text
unverified_caller
user_channel_observation
registered_connection_observation
verified_tool_invocation
verified_command_execution
reused_evidence
```

기준 구현에서 authority-owned producer 경로가 있는 값은
`user_channel_observation`과 재귀 검증된 `reused_evidence`뿐입니다. connection,
tool, command 값은 이후 담당 문서가 정의할 producer 전이를 위한 정규 분류를
예약합니다. 호출자 입력이나 raw guard payload로 이 값을 만들 수 없습니다.

`EvidenceRelevanceAssessment.status`와
`UserEvidenceObservation.relevance_status`는 `unassessed`, `supported`,
`contradicted`를 사용합니다. User Channel 메서드는 `supported` 또는
`contradicted`만 받습니다. Strong evidence에는 분리된 현재 `supported` 평가가
필요합니다.

<a id="source-ref-values"></a>
### 출처 참조 값

`SourceRef.source_kind`는 아래 값을 사용합니다.

```text
repository_file
git_commit
git_diff
command
external_uri
user_context
```

이 값들은 구조가 서로 다른 권한 효력이 없는 출처 본문 하나를 선택합니다. 맥락이나
출처만 분류하며 증거 보장 수준, 사용자 권한, 범위, 닫기 권한을 선택하지 않습니다.

`ValidatorResult.status`는 아래 값을 사용합니다.

```text
passed
warning
failed
blocked
```

`ValidatorResult.severity`는 아래 값을 사용합니다.

```text
info
warning
error
blocking
```

이 기준 범위 값 집합 담당 문서는 지원되는 안정 `ValidatorResult.validator_id` 집합을 공개하지 않습니다. `validator_id` 문자열은 보고용 라벨이며 안정된 제어 값이 아닙니다.

`GuaranteeDisplay.level`은 기준 범위 지원 값으로 아래를 사용합니다.

```text
cooperative
detective
```

`cooperative`는 다른 근거가 없을 때 사용하는 기준 값입니다. `detective`는 보안 담당 문서가 그 주장을
지원하고, 프로젝트 강제 사실, 확인된 Agent Connection 또는 User Channel 출처,
활성화된 강제 메커니즘, 관찰 범위 사실이 이를 뒷받침할 때만 표시할 수 있습니다.
선언된 연결 역량만으로 표시 보장을 높일 수 없습니다.

`GuaranteeDisclosure.guarantee_class`는 아래 값을 사용합니다.

```text
authority_record
cooperative_host_decision
detective_observation
user_judgment_record
```

값 의미:
- `authority_record`는 결과가 문서화된 메서드 계약 안에서 Core 권한 상태, 응답 분기 메타데이터, 메서드별 결과 필드를 보고한다는 뜻입니다.
- `cooperative_host_decision`은 결과가 관찰된 호스트 이벤트에 대해 협력형 호스트 훅으로 반환한 결정을 보고한다는 뜻입니다.
- `detective_observation`은 결과가 Volicord가 검사할 수 있었던 로컬 진단, 검증, 관찰, 전송 상태 사실을 보고한다는 뜻입니다.
- `user_judgment_record`는 결과가 지원되는 `User Channel` 경로로 받은 사용자 소유 판단을 기록한다는 뜻입니다.

`GuaranteeDisclosure.non_guarantees`는 아래 값을 사용합니다.

```text
NotOsSandbox
NotNetworkIsolation
NotMalwareDefense
NotTamperProofAuditLog
NotCorrectnessProof
NotTestSufficiencyProof
NotHumanReviewReplacement
NotFullWritePrevention
NotFullFilesystemMonitoring
NotActorAttributionProof
NotIntentProof
```

이 값들은 안정적인 비주장입니다. 결과를 OS 샌드박싱, 네트워크 격리, 악성 코드 방어, 변조 불가능 감사 로그, 제품 정확성 증명, 테스트 충분성 증명, 인간 검토 대체, 전체 쓰기 방지, 전체 파일시스템 감시, 행위자 귀속 증명, 의도 증명으로 해석하면 안 된다는 뜻입니다.

<a id="artifact-values"></a>
## 아티팩트 값

`ArtifactInput.source_kind`는 아래 값을 사용합니다.

```text
staged_artifact
existing_artifact
```

값 의미:
- `staged_artifact`는 `ArtifactInput.staged_artifact_handle`과 짝을 이룹니다.
- `existing_artifact`는 `ArtifactInput.existing_artifact_ref`와 짝을 이룹니다.

선택된 출처 값은 어느 `ArtifactInput` 출처 필드가 적용되는지 정합니다. 정확한 형태 불변조건은 [API 아티팩트 스키마](schema-artifacts.md#artifactinput)가 담당합니다.

이 목록 밖의 값은 지원되는 출처 값이 아닙니다. 새 출처 어휘의 동작을 지원된다고 설명하려면 이 문서의 지원 값과 영향받는 의미 담당 문서가 모두 필요합니다.

`redaction_state`는 아래 값을 사용합니다.

```text
none
redacted
secret_omitted
blocked
```

아티팩트 가용성 표시 값은 아래를 사용합니다.

```text
available
unavailable
missing
integrity_failed
blocked
unusable
```

`ArtifactIntegrityStatus`는 아래 값을 사용합니다.

```text
verified
corrupt
```

`verified`는 영속 아티팩트의 무결성 사실이 충분하며, 권한 근거로 사용하기 전에 현재 바이트를 검증할 수 있다는 뜻입니다. `corrupt`는 저장된 바이트나 메타데이터가 저장된 무결성 사실과 맞지 않는다고 알려져 있거나, 저장된 `verified` 사실 관계가 유효하지 않다는 뜻입니다. 아티팩트를 증거나 닫기에 사용할 때 필요한 현재 바이트 확인은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다. 본문 바이트가 없거나, 읽을 수 없거나, 사용할 수 없거나, 사용에 부적합한 상태는 아티팩트 무결성 값이 아니라 아티팩트 가용성 값으로 표현합니다.

아티팩트 저장소 생명주기와 본문 읽기 자격은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다.

<a id="judgment-values"></a>
## 판단 값

`judgment_kind`는 아래 값을 사용합니다.

```text
product_decision
technical_decision
scope_decision
sensitive_approval
final_acceptance
residual_risk_acceptance
cancellation
```

`presentation`은 아래 값을 사용합니다.

```text
short
```

`required_for`는 아래 작업 대상 값을 사용합니다.

```text
scope_update
prepare_write
record_run
close_complete
close_cancel
close_supersede
informational
```

`UserJudgment.status`는 아래 값을 사용합니다.

```text
pending
resolved
stale
superseded
expired
```

상태 값은 판단 생명주기를 설명합니다. `resolved`는 답변이 기록되었다는 뜻이며, 그 자체로 승인, 수락, 권한 부여를 뜻하지 않습니다.

`JudgmentResolutionOutcome`은 아래 값을 사용합니다.

```text
accepted
rejected
deferred
```

`JudgmentBasis.compatibility_status`는 아래 값을 사용합니다.

```text
current
stale
superseded
```

의미:
- `current`는 근거가 현재 만족할 수 있는 요구사항과 지금 일치한다는 뜻입니다.
- `stale`은 저장된 근거가 더 이상 현재 상태와 일치하지 않는다는 뜻입니다. 해결된 행은 감사용으로 남을 수 있지만 현재 요구사항에는 사용할 수 없습니다.
- `superseded`는 대기 판단이 더 새 질문이나 근거로 대체되어 성공적으로 답할 수 없다는 뜻입니다.

권한 선택지 동작 값:
- `accept`는 `accepted`로 매핑됩니다.
- `reject`는 `rejected`로 매핑됩니다.
- `defer`는 메서드나 의미 담당 문서가 연기를 허용하는 곳에서만 `deferred`로 매핑됩니다.

해결 결과 의미:
- `accepted`는 판단 종류, 근거, 확인된 행위자 출처, 선택된 선택지, `machine_action=accept`가 모두 호환될 때 권한 요구사항을 만족할 수 있는 유일한 결과입니다.
- `rejected`와 `deferred`는 지속되는 사용자 결정이지만 어떤 것도 승인, 수락, 권한 부여, 면제, 닫기를 만들지 않습니다.
- `blocked`는 제품의 다른 차단 결과와 차단 사유 값 집합에서 쓰이지만 `JudgmentResolutionOutcome` 값이 아니며 선택지 해결 결과로 저장할 수 없습니다.
- 기계 판독 가능한 결과가 없으면 절대 `accepted`로 해석하면 안 됩니다.

대기 판단 관련성:
- 대기 판단은 현재 `required_for` 대상이 해당 작업을 포함하고, `judgment_kind`가 그 작업과 관련 있으며, `Task`, Change Unit, 영향받는 참조, 근거가 호환될 때만 작업을 차단합니다.
- 민감 승인 질문은 민감 동작 범위가 현재 민감 동작 요구사항과 겹칠 때만 관련됩니다.
- `informational` 판단은 감사 또는 표시 맥락이며 그 자체로 쓰기, 실행 기록, 닫기를 차단하지 않습니다.

`UserJudgmentOption.option_id`의 범위는 그 판단 안으로 제한되며 전역 값 집합이
아닙니다. 화면에 보이는 선택지 라벨은 기준 값이 아니라 표시 텍스트일 뿐입니다.
공개 API의 `UserJudgmentOption.machine_action`은 위의 권한 선택지 동작 값을
사용합니다. `UserJudgmentOption.resolution_outcome`은
`JudgmentResolutionOutcome`을 사용합니다. 선택지 라벨과 설명 문구가 기계 판독
가능한 동작이나 결과를 뒤집으면 안 됩니다.

<a id="error-detail-helper-values"></a>
## 오류 세부사항 보조 값

`ToolError.details.write_ticket_reason`과 `ToolError.details.artifact_input_error.reason` 보조 값은 [API 오류 세부사항](error-details.md#error-detail-helper-values)이 담당합니다. 이 값 집합 문서는 기계 판독용 오류 세부사항 의미를 정의하지 않습니다.

<a id="profile-gated-and-reserved-values"></a>
## 프로필 조건부 및 예약 값

예약된 값이나 프로필 조건부 값은 기준 범위의 기본 지원 값이 아닙니다. 이 문서는 지원되지 않는 값 이름을 지원되는 값 집합으로 공개하지 않습니다.

경계:
- 지원 목록 밖의 이름은 메모, 예시, 경로 문서, 렌더링된 라벨에 나온다는 이유만으로 기준 범위 동작이 되지 않습니다.
- 예약된 값이나 프로필 조건부 값의 동작을 지원된다고 설명하려면 [범위 참조](../scope.md) 경계와 영향받는 의미 담당 문서가 먼저 필요합니다.

## 관련 담당 문서

- [기준 범위](../scope.md): 값이 기준 범위에 속하는지 판단.
- [API 오류 코드](error-codes.md): 공개 오류 코드 의미.
- [API 오류 우선순위](error-precedence.md): 공개 오류 우선순위.
- [API 차단 사유 처리 경로](blocker-routing.md): 닫기 차단 사유와 API 응답 분기 사이의 처리 경계.
- [API 오류 세부사항](error-details.md): 기계 판독용 오류 세부사항 보조 값.
- [API 코어 스키마](schema-core.md), [API 상태 스키마](schema-state.md), [API 아티팩트 스키마](schema-artifacts.md), [API 판단 스키마](schema-judgment.md): 이 값을 쓰는 필드.
- [API 메서드](methods.md)와 메서드 담당 문서: 이 값을 사용하는 메서드 동작.
- [범위 참조](../scope.md): 예약된 값과 프로필 조건부 값의 경계.
