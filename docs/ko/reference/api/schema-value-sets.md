# API 값 집합

이 문서는 기준 범위의 지원되는 API 값 집합과 enum 형태 공개 값을 담당합니다. 예약된 값이나 지원 범위 밖 값을 이름 붙이는 것만으로 기준 범위가 넓어지지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 지원되는 공개 메서드 이름 값
- 지원되는 행위자 종류 값
- 지원되는 다음 행동 값
- API `response_kind`와 `effect_kind` 값
- 지원되는 operation category 값
- 공유 상태 참조에서 쓰는 기록/참조 판별 값
- 지원되는 생명주기, 닫기 상태, 증거 관찰 출처와 보장 수준, 쓰기 결정 범주, 판단 종류, 표시 형식, 필요 판단 위치, 판단 해결 결과, 아티팩트 가림 처리, 아티팩트 무결성, 아티팩트 가용성 표시, `ValidatorResult.status`, `ValidatorResult.severity`, 보장 표시 등 API 값 집합
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

- 모드 조건부 값은 사용하는 자리에서 connection mode, User Channel, admin-local, 또는 담당 문서가 정의한 조건을 이름 붙여야 합니다.
- 지원 목록 밖의 값은 [범위 참조](../scope.md)와 영향받는 의미 담당 문서가 지원 동작을 정의하기 전까지 기준 범위 API 값이 아닙니다.
- 지원 목록 밖의 이름을 적는 것만으로 기준 범위가 넓어지지 않습니다.
- 화면에 보이는 라벨은 표시 텍스트일 뿐이며, 이 문서의 기준 값을 대신하지 않습니다.
- API 예시는 스키마 담당 문서가 해당 필드를 명시적으로 자유 형식 표시 문자열, 불투명 식별자, 또는 불투명 분류 문자열로 정의하지 않는 한, 이 문서의 지원되는 enum 형태 값을 사용해야 합니다.
- 문자열 형태 필드는 스키마 담당 문서가 이 문서의 값 집합으로 연결할 때만 이 문서가 담당합니다. 불투명 식별자, 불투명 분류 문자열, 자유 형식 표시 문자열은 해당 스키마 또는 메서드 담당 문서에 남습니다.
- 메서드 예시가 불투명 사유 코드나 분류 문자열을 보여 주더라도 그 문자열이 지원되는 전역 값이 되지는 않습니다.

<a id="method-name-values"></a>
## 메서드 이름 값

지원되는 공개 메서드 이름 집합은 아래와 같습니다.

```text
volicord.intake
volicord.update_scope
volicord.status
volicord.prepare_write
volicord.stage_artifact
volicord.record_run
volicord.request_user_judgment
volicord.record_user_judgment
volicord.reconcile_changes
volicord.close_task
```

메서드 동작은 [API 메서드](methods.md)가 안내하는 메서드 담당 문서가 담당합니다. 메서드 이름은 `Task` 생명주기 값이 아닙니다.

<a id="actor-source-values"></a>
<a id="actor-values"></a>
## 행위자 출처 값

`EvidenceObservation.observed_by_actor_source`, `EvidenceObservationInput.observed_by_actor_source`, `UserJudgmentResolution.resolved_by_actor_source` 같은 행위자 출처 필드는 `ActorSource` 값 집합을 사용합니다.

| 값 | 사용하는 곳 | 담당 문서 경로 |
|---|---|---|
| `local_user` | User Channel 호출 출처와 권한을 지니는 사용자 판단 해결. | 호출 의미: [Agent Connection](../agent-connection.md). 해결 형태 담당 문서: [API 판단 스키마](schema-judgment.md). |
| `agent_connection:<connection_id>` | Agent Connection 호출 출처와 에이전트가 만들거나 관찰한 상태. | 호출 의미: [Agent Connection](../agent-connection.md). 중첩 형태 담당 문서가 값이 나타나는 위치를 정의합니다. |
| `system` | 담당 문서가 명시적으로 허용하는 내부 시스템 출처. | 메서드와 저장소 담당 문서가 값이 나타나는 위치를 정의합니다. |

이 값들은 파생된 호출 출처 또는 지속 행위자 출처를 분류합니다. 이 값만으로 사용자 소유 판단, 승인, 범위 결정 권한, 최종 수락, 잔여 위험 수락, `Write Check`이 생기지는 않습니다. 권한을 지니는 사용자 판단 해결은 [Agent Connection](../agent-connection.md)과 메서드 담당 문서가 정의하는 호환 User Channel 출처와 함께 `resolved_by_actor_source=local_user`를 요구합니다.

<a id="next-action-values"></a>
## 다음 행동 값

`NextActionSummary.action_kind`는 제어되는 행동 범주 값입니다. 지원되는 값은 아래 값 집합뿐입니다.

| `action_kind` 값 | 메서드 하나가 다음 단계를 담당할 때의 `owner_method` |
|---|---|
| `update_scope` | `volicord.update_scope` |
| `prepare_write` | `volicord.prepare_write` |
| `stage_artifact` | `volicord.stage_artifact` |
| `record_run` | `volicord.record_run` |
| `request_user_judgment` | `volicord.request_user_judgment` |
| `record_user_judgment` | `volicord.record_user_judgment` |
| `reconcile_changes` | `volicord.reconcile_changes` |
| `close_task` | `volicord.close_task` |

`action_kind`는 메서드 이름 값이 아닙니다. 지원되는 공개 메서드 하나가 다음 단계를 담당할 때 `NextActionSummary.owner_method`는 [메서드 이름 값 집합](#method-name-values)을 사용하고, 단일 담당 메서드가 없으면 `null`입니다. 다음 단계의 메서드 동작은 [API 메서드](methods.md)가 안내하는 메서드 담당 문서에 둡니다. 전체 `NextActionSummary` 형태는 [API 상태 스키마](schema-state.md#current-position-display-shapes)가 담당합니다.

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
## Operation category 값

메서드 담당 API 호환성 점검은 공개 API 요청 하나마다 요청 수준 operation category 하나를 사용합니다.

| 값 | 어휘 설명 |
|---|---|
| `read` | 읽기 전용 API operation category입니다. `read_only` Agent Connection은 이 category를 실행할 수 있습니다. |
| `agent_workflow` | 에이전트 워크플로 API operation category입니다. `workflow` Agent Connection은 이 category와 `read`를 실행할 수 있습니다. |
| `user_only` | 권한을 지니는 사용자 동작을 위한 User Channel operation category입니다. Agent Connection은 이 category를 실행하지 않습니다. |
| `admin_local` | 로컬 관리 operation category입니다. Agent Connection은 이 category를 실행하지 않습니다. |
| `local_recovery` | `volicord.reconcile_changes` 같은 메서드 담당 복구 경로를 위한 로컬 사용자 복구 operation category입니다. Agent Connection은 이 category를 실행하지 않습니다. |

Operation category는 Volicord API 호환성 분류이지 OS 권한, 파일시스템 ACL, 샌드박스 규칙, 네트워크 정책, 비밀 격리가 아닙니다. 메서드별 동작 요구사항은 [API 메서드](methods.md)가 안내하는 메서드 담당 문서가 담당하고, Agent Connection 호출 검증 동작은 [Agent Connection](../agent-connection.md)과 [보안](../security.md)이 담당합니다.

<a id="record-and-reference-values"></a>
## 기록과 참조 값

`StateRecordRef.record_kind`는 아래 값을 사용합니다.

```text
project_state
task
change_unit
write_check
user_judgment
run
evidence_summary
evidence_observation
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

이 값들은 오래 유지하는 프로젝트 수준 맥락을 분류합니다. 그 자체로 현재 `Task` 권한을 만들거나, 대기 중인 사용자 판단을 만족하거나, 증거를 증명하거나, `Write Check`을 부여하거나, 닫기 준비 상태를 만족하거나, 미래 닫기 근거의 잔여 위험을 수락하지 않습니다.

<a id="task-lifecycle-values"></a>
## `Task` 생명주기 값

`StateSummary.mode`와 확정된 `Task.mode` 필드는 아래 값을 사용합니다.

```text
advisor
direct
work
```

`volicord.intake`의 `requested_mode`는 입력 전용 값으로 `auto`도 받습니다. 출력 `Task.mode` 필드는 `advisor`, `direct`, `work`를 사용합니다. 접수 확정 동작은 [접수 메서드](method-intake.md)가 담당합니다.

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

이 값들은 효과를 Core 상태로 분류합니다. 값 자체가 런타임 샌드박스, 명령 가로채기, 네트워크 차단, 비밀 격리, 사용자 판단, 민감 동작 승인, 증거, `Write Check`, 최종 수락, 닫기 준비 상태, 잔여 위험 수락을 만들지는 않습니다.

`volicord.close_task.intent`는 아래 값을 사용합니다.

```text
check
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

`PrepareWriteResult.write_check_effect`는 아래 값을 사용합니다.

```text
none
would_create
created
```

`WriteCheckStateSummary.status`와 `WriteCheckSummary.status`는 아래 값을 사용합니다.

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

<a id="state-and-blocker-values"></a>
## 상태와 차단 사유 값

`CloseReadinessBlocker` 객체 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다.

이 절은 차단 사유 범주 값인 `CloseReadinessBlocker.category`와 인접 상태/차단 사유 값을 담당합니다.

`PlannedBlocker.source_kind`는 아래 값을 사용합니다.

```text
write_decision
close_readiness
```

`GuardHealthSummary.guard_mode`는 아래 값을 사용합니다.

```text
mcp_only
guarded
managed
```

`managed`는 호스트 지원 plugin, managed 설정 bundle, managed policy 계층처럼 검증된
managed 배포 계약이 뒷받침하는 설치에만 기록되는 guard 모드입니다. 그런 검증된 계약이
없는 호스트는 일반 프로젝트 로컬 guarded 파일을 managed 모드로 기록하는 대신 managed
초기화를 지원하지 않는다고 보고해야 합니다.

`GuardHealthSummary.guard_strength`는 아래 값을 사용합니다.

```text
authority_record_only
detective_watch
host_hook_guarded
managed_guarded
```

`authority_record_only`는 Volicord가 권한 상태는 기록할 수 있지만 선택된 보기에 대해
전체 coverage를 가진 활성 session watcher나 활성 전체 host hook guard를 사용할 수
없다는 뜻입니다. `detective_watch`는 session watcher가 부분 coverage 경고 없이 활성
상태이고 coverage 시작 뒤의 우회 Product Repository 변경을 감지할 수 있지만 쓰기를
사전에 차단할 수 없다는 뜻입니다. `host_hook_guarded`는 선택된
프로젝트 로컬 guarded host hook이 필요한 lifecycle phase에 대해 설정되고 관찰되었다는
뜻입니다. `managed_guarded`는 host-hook guarded 조건을 만족하고 선택된 managed 배포
메타데이터가 검증되었다는 뜻입니다. 이 라벨은 제품 정확성, review 완료, 테스트 충분성,
OS 강제, 샌드박싱, 보안 격리, 최종 수락, 잔여 위험 수락을 증명하지 않습니다.

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

이 값들은 닫기 준비 상태와 상태 조회 보기에 쓰이는 guard 통합 상태를 보고합니다. `guard_installation_status`는 저장된 생명주기 값이고, `guard_configuration_status`는 파일과 required hook 완전성을 도출하며, `guard_observation_status`는 현재 설치에 일치하는 hook 관찰이 있는지를 도출합니다. `effective_guard_status`는 guarded 또는 managed 경로의 닫기 준비 상태 건강 점검에 쓰는 값입니다. 효과적인 `active` 건강 상태에는 guarded 또는 managed 모드, 완전한 required hook 설정, 오래되거나 깨지지 않은 설치, 현재 일치하는 관찰, 일치하는 호스트와 policy 식별 정보가 필요합니다. `prompt_capture_status`는 사용자 소유 판단 채팅 명령을 위한 prompt capture 사용 가능 상태입니다. `unsupported_by_host`는 호스트 기능이 없음을 뜻하고, `not_configured`는 선택된 연결에 prompt-capture 단계가 설정되어 있지 않음을 뜻합니다. `reload_required`는 사용 전에 설치된 설정이나 policy 식별 정보를 다시 읽어야 함을 뜻합니다. `configured`는 prompt-capture 관찰 전에도 검증 코드 채팅 명령을 표시할 수 있음을 뜻하고, `observed`는 일치하는 guard hook이 관찰되었음을 뜻합니다. `active`는 일치하는 prompt-capture hook 관찰이 기록되었음을 뜻하고, `degraded`는 저하된 guard 건강 상태 때문에 prompt capture가 차단됨을 뜻합니다. `session_watch_status`는 탐지용 watcher 사용 가능 상태입니다. `disabled`는 선택된 session-watch baseline을 사용할 수 없다는 뜻이고, `active`는 한정된 스냅샷 비교를 사용할 수 있다는 뜻이며, `degraded`는 watcher 출력이 부분적이거나 운영자 조치가 필요하다는 뜻이고, `unavailable`은 watcher가 선택된 스냅샷 확인을 수행할 수 없었다는 뜻입니다. 이 값들은 제품 정확성, 테스트 충분성, OS 강제, 샌드박싱, 보안 격리, 최종 수락, 잔여 위험 수락을 증명하지 않습니다. `mcp_only`는 활성 session watch가 선택되어 있을 때 watcher가 만든 해결되지 않은 미기록 변경 찾기가 닫기를 막는 경우를 제외하고 협력형으로 남습니다.

`pending_project_selection`은 MCP session에 사용할 수 있는 프로젝트가 둘 이상이고
session-watch baseline을 만들 만큼 프로젝트가 아직 명시적으로 선택되지 않았다는
뜻입니다. `mcp_start`는 프로젝트에 묶인 시작 또는 HTTP session 초기화에서 MCP 도구
처리 전에 watcher coverage가 시작된다는 뜻입니다. `first_project_selection`은 여러
프로젝트 session이 처음 명시적인 `project_selector`를 이름 붙일 때 coverage가
시작된다는 뜻입니다. `method_boundary`는 Core 메서드 경계 fallback에서 coverage가
시작된다는 뜻입니다. `first_project_selection`과 `method_boundary`는 부분 coverage
basis입니다. 기록된 coverage 시작 전의 Product Repository 변경은 watcher coverage
밖에 있습니다.

`UnrecordedChangeFinding.status`는 아래 값을 사용합니다.

```text
unresolved
resolved
```

<a id="unrecorded-change-resolution-basis-values"></a>
`UnrecordedChangeResolutionSummary.resolution_basis`와 저장된 미기록 변경 해결 메타데이터는 아래 값을 사용합니다.

```text
reverted
covered_by_write_readiness
recorded_as_expected_write
accepted_by_user
not_product_change
superseded_by_new_observation
invalid_observation
```

이 값들은 미기록 Product Repository 변경 찾기가 해결된 이유를 분류합니다. 제품 정확성, 증거 충분성, 리뷰 완료, 최종 수락, 잔여 위험 수락, 보안을 증명하지 않습니다. 호출자 사용은 [`volicord.reconcile_changes`](method-reconcile-changes.md)가 제한합니다. basis 이름만으로 에이전트 단독 묵살이 허용되지 않습니다.

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

이 값 집합은 `category`만 제어합니다. `WriteDecisionReason.code`는 전역 닫힌 enum이 아닙니다. 메서드 범위의 불투명 사유 코드이며, 메서드 담당 문서는 예시 코드를 보여 주더라도 전역 지원 목록에 추가하지 않을 수 있습니다. `message`는 자유 형식 표시 문자열이고, `related_refs`는 `StateRecordRef`를 사용합니다.

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

`EvidenceCoverageItem.coverage_state`는 아래 값을 사용합니다.

```text
unsupported
partial
supported
not_applicable
stale
blocked
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

출처 종류 의미:
- `agent_report`는 에이전트 행위자 맥락이 만든 보고를 기록합니다. 그 자체로 외부 도구 결과가 아닙니다.
- `connection_observation`은 등록된 Agent Connection에 귀속된 관찰을 기록합니다. 그 자체로 증명이 아닙니다.
- `external_tool`은 외부 도구의 출력이나 상태를 기록합니다. 그 자체로 제품 정확성 증명이 아닙니다.
- `user_observation`은 사용자 귀속 관찰을 기록합니다. 그 자체로 최종 수락이나 다른 권한을 지니는 판단이 아닙니다.
- `reused_evidence`는 이전 증거나 아티팩트 재사용을 기록합니다. 그 자체로 새 관찰이 아닙니다.
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
- `registered_connection_observed`는 등록된 Agent Connection이 기록된 connection 맥락 안에서 그 주장을 관찰했음을 기록합니다.
- `external_tool_result`는 관찰이 외부 도구 결과에 기반함을 기록합니다.
- `user_observed`는 사용자 귀속 관찰 출처를 기록합니다.
- `unverified`는 확인된 관찰 보장 수준이 없음을 기록합니다.

이 값들은 증거 관찰 출처를 분류합니다. 사용자 권한을 부여하거나, 최종 수락 또는 잔여 위험 수락을 만족하거나, 제품 정확성을 증명하거나, `GuaranteeDisplay.level`을 바꾸지 않습니다.

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

이 기준 범위 값 집합 담당 문서는 지원되는 안정 `ValidatorResult.validator_id` 집합을 공개하지 않습니다. 영향받는 담당 문서가 정확한 안정 값을 이 문서에 공개하고 그 의미를 정의하기 전까지 `validator_id` 문자열은 보고용 라벨입니다.

`GuaranteeDisplay.level`은 기준 범위 지원 값으로 아래를 사용합니다.

```text
cooperative
detective
```

`cooperative`는 기준 대체값입니다. `detective`는 보안 담당 문서가 그 주장을 지원하고, 프로젝트 강제 사실, 확인된 Agent Connection 또는 User Channel 출처, 활성화된 강제 메커니즘, 관찰 범위 사실이 이를 뒷받침할 때만 표시할 수 있습니다. 선언된 connection capability만으로 표시 보장을 높일 수 없습니다.

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

`verified`는 지속 아티팩트 사실이 무결성을 확인할 수 있을 만큼 완전하고 권한 사용 전에 현재 바이트 검증을 수행할 수 있다는 뜻입니다. `corrupt`는 저장된 바이트나 메타데이터가 지속 저장된 무결성 사실과 맞지 않는다고 알려져 있거나 저장된 `verified` 사실 관계가 유효하지 않다는 뜻입니다. 아티팩트를 증거나 닫기에 사용할 때 필요한 현재 바이트 확인은 [아티팩트 저장소](../storage-artifacts.md)가 담당합니다. 본문 바이트가 없거나, 읽을 수 없거나, 사용할 수 없거나, 사용에 부적합한 상태는 아티팩트 무결성 값이 아니라 아티팩트 가용성 값으로 표현합니다.

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
- `accepted`는 판단 종류, 근거, 확인된 행위자 출처, 선택된 선택지, `machine_action=accept`가 모두 호환될 때 권한을 지니는 판단 요구사항을 만족할 수 있는 유일한 결과입니다.
- `rejected`와 `deferred`는 지속되는 사용자 결정이지만 어떤 것도 승인, 수락, 권한 부여, 면제, 닫기를 만들지 않습니다.
- `blocked`는 제품의 다른 차단 결과와 차단 사유 값 집합에서 쓰이지만 `JudgmentResolutionOutcome` 값이 아니며 선택지 해결 결과로 저장할 수 없습니다.
- 기계 판독 가능한 결과가 없으면 절대 `accepted`로 해석하면 안 됩니다.

대기 판단 관련성:
- 대기 판단은 현재 `required_for` 대상이 해당 작업을 포함하고, `judgment_kind`가 그 작업과 관련 있으며, `Task`, Change Unit, 영향받는 참조, 근거가 호환될 때만 작업을 차단합니다.
- 민감 승인 질문은 민감 동작 범위가 현재 민감 동작 요구사항과 겹칠 때만 관련됩니다.
- `informational` 판단은 감사 또는 표시 맥락이며 그 자체로 쓰기, 실행 기록, 닫기를 차단하지 않습니다.

`UserJudgmentOption.option_id`의 범위는 그 판단 안으로 제한되며 전역 값 집합이 아닙니다. 화면에 보이는 선택지 라벨은 기준 값이 아니라 표시 텍스트일 뿐입니다. 현재 공개 `UserJudgmentOption.machine_action`은 위의 권한 선택지 동작 값을 사용합니다. `UserJudgmentOption.resolution_outcome`은 `JudgmentResolutionOutcome`을 사용합니다. 선택지 라벨과 설명 문구가 기계 판독 가능한 동작이나 결과를 뒤집으면 안 됩니다.

## 오류 세부사항 보조 값

`ToolError.details.write_check_reason`과 `ToolError.details.artifact_input_error.reason` 보조 값은 [API 오류 세부사항](error-details.md#error-detail-helper-values)이 담당합니다. 이 값 집합 문서는 기계 판독용 오류 세부사항 의미를 정의하지 않습니다.

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
