<a id="volicordupdate_scope"></a>

# `volicord.update_scope` 참조

## 담당하는 것

이 문서는 기준 범위의 `volicord.update_scope` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- `volicord.intake` 이후 범위와 Change Unit을 갱신하는 동작
- 범위 갱신 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 아티팩트, 판단, 값 집합, 오류의 중첩 스키마 정의
- 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장, Core 권한 의미
- 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리 경로

## 목적

`volicord.update_scope`는 `volicord.intake` 이후 현재 `Task`와 현재 적용 Change Unit 필드를 갱신합니다.

- 목표 요약
- 범위 경계
- 범위 밖 항목
- 수락 기준
- 자율성 경계
- 기준선 참조
- 현재 적용 Change Unit

이 메서드는 Change Unit을 현재 작업 경계로 기록하며 `work_phase`를 변경하지 않습니다. 따라서 shaping 중인 `work` Task는 `create_current` 또는 `replace_current` 뒤에도 shaping에 머물고, 구현 진입은 `volicord.advance_task`만 수행합니다.

`direct`와 `work` Task에서 커밋된 `create_current` 또는 `replace_current` 작업은 새 Change Unit의 기준선을 확인된 현재
작업 공간 맥락에 결합합니다. Git 기반 Product Repository에서는 공통 Git 디렉터리,
정확한 worktree 식별 정보, 브랜치 또는 detached HEAD 상태, HEAD SHA, 작업 공간 지문을
기록합니다. 이 좌표가 바뀐 뒤 현재 기준선으로 Change Unit을 교체하는 작업이 명시적
대상 변경 또는 재기준 설정 경로입니다. `keep_current`는 기존 Change Unit의 대상을
암묵적으로 바꾸지 않습니다. 현재 Change Unit이 있으면 `keep_current`는 Task
`baseline_ref` 변경을 거절합니다. Task와 Change Unit 기준선을 원자적으로 바꾸려면
호출자가 `replace_current`를 사용해야 합니다.

`advisor` Task에서는 `keep_current`, `create_current`, `replace_current`가 모두 정규
비쓰기 Change Unit 조건을 만족해야 합니다. Change Unit에는 affected/allowed path가 없고,
effect contract는 `artifact_registration`, `user_action_request`, `evidence_update`만 허용하며
sensitive expectation은 없고 `product_file_write`, `run_recording`, `sensitive_action`,
`external_network`, `secret_access`를 명시적으로 금지합니다. Core는 쓰기 가능하거나 그 밖에
호환되지 않는 advisor Change Unit을 만들거나 유지하는 갱신을 scope 효과 커밋 전에
거부합니다. 정규 contract의 expected output, invariant, evidence expectation,
sensitive-action expectation은 모두 비어 있습니다.

현재 MCP action form은 Task, scope 소유 gap에 해당하는 완전한
`related_scope_decision_refs`, 현재 Change Unit 권한이 선택한 정확한
`change_unit.operation`을 고정합니다. 요청의 `baseline_ref`는 현재 nullable 기준선의
복사본이 아니라 에이전트가 작성하는 다음 기준선입니다. 현재 기준선과 범위 리비전은
허용된 form과 예상 상태 버전이 포괄하는 Core 현재 권한으로 남습니다. Scope 내용은
에이전트 작성 slot입니다. 일반 `direct`·`work` create/replace form에서는 Change Unit scope와
effect 내용도 에이전트가 작성합니다. Advisor create/replace form은
`affected_paths=[]`와 완전한 정규 observe-only effect contract를 고정하므로 두 값은
에이전트 작성 값이 아니며 form은 Product Repository 권한을 부여하지 않습니다. 프로젝트와 예상 상태 버전은
adapter가 주입하며 일반 binder는 호출자에게 보이는 고정 값이 바뀌거나 빠지면 Core 전에
거부합니다.

Core는 폐쇄형 action variant `keep_current_change_unit`,
`create_current_change_unit`, `replace_current_change_unit`을 게시하며 각각 대응하는
`ChangeUnitOperation`에 정확히 매핑됩니다. 현재 Change Unit이 없으면 create만 현재
variant입니다. 현재 Change Unit이 있으면 keep과 Core 정책에 호환되는 replace가 현재이고
create는 아닙니다. 현재 shaping application을 stale로 만들 implementation replacement는
게시하지 않습니다. Keep form은 `keep_current`를 고정합니다. 일반 create와 replace form은
각 동작을 고정하고 에이전트가 작성하는 `scope_summary`, `affected_paths`, 다음 기준선을
필수로 하면서 선택적 Change Unit 필드와 effect contract를 유지합니다. Advisor create와
replace는 에이전트가 작성하는 `scope_summary`와 다음 기준선을 요구하지만 빈 affected path와
정규 observe-only effect contract를 고정합니다.

제출된 범위 내용을 검증하기 전에 Core는 정규화된 `WorkflowSnapshot`을 평가하고 선택한
`ChangeUnitOperation`의 정확한 `WorkflowActionKey`를 소비합니다. 메서드 이름만 일치해서는
이 mutation을 admit하지 않습니다. 메서드 의미를 검증하기 전에 descriptor의 고정 권한
좌표와 요청을 대조합니다. 현재 variant가 없으면 효과 없이 `TransitionRejection`을 반환하며,
`recovery_action_key`가 있으면 같은 현재 catalog에도 반드시 있어야 합니다. Implementation
중에는 일반 baseline-operation 불일치보다 권한 무효화를 먼저 거부하고, replace variant가
현재 상태가 아니면 replacement를 제안하지 않습니다.

## 필수 입력

- 유효한 `ToolEnvelope`. 커밋되는 `dry_run`이 아닌 요청에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`.
- 바꿀 범위 필드. 포함/제외 방식으로 범위를 갱신할 때는 `scope_update.include`에 범위에 포함할 제품 작업을, `scope_update.exclude`에 범위에서 제외할 제품 동작을 둡니다. `null`은 기존 값을 유지한다는 뜻이고, 빈 배열은 그 목록을 빈 목록으로 교체합니다.
- `acceptance_criteria=null`은 정규 기준 집합을 그대로 둡니다. null이 아닌
  배열은 전체 교체 집합입니다. 현재 같은 `Task` ID는 기준 identity를
  유지하면서 문장이나 `evidence_requirement`를 갱신할 수 있습니다. 이는 새
  identity 생성이 아니라 같은 기준의 갱신입니다. null ID는 Core가 새 ID를 만들도록 요청하며, 빠진 현재 기준은
  폐기됩니다. 알 수 없거나, 폐기되었거나, 다른 `Task`에 속하거나, 중복된
  ID는 커밋 전에 거절합니다.
- `change_unit.operation`과 그 작업에 필요한 필드. 지원되는 작업 값과 그 의미는 [API 값 집합](schema-value-sets.md#method-local-values)이 담당합니다.
- 일반 `direct`·`work` create/replace에서는 `change_unit.effect_contract`가 선택적입니다.
  Advisor create/replace에서는 action form이 필수 정규 observe-only contract를 권한 고정 값으로
  제공하며 호출자는 이를 작성하거나 생략하지 않습니다.
- `related_scope_decision_refs`는 현재 checkpoint의 accepted 범위 owner gap을 정확히 모두
  포함해야 합니다. 그런 gap이 없으면 제품 전용·기술 전용 진행을 포함해 빈 배열입니다.

범위 갱신이 `scope_decision`을 적용할 때 각 ref는 현재 checkpoint의 accepted 범위 gap에
연결된 정확한 resolution이어야 합니다. 또한 `judgment_kind=scope_decision`,
`status=resolved`, `machine_action=accept`, `resolution_outcome=accepted`,
`resolved_by_actor_source=local_user`, 호환 User Channel 출처,
`basis.coordinates.compatibility_status=current`, 정확한 `required_for=[scope_update]`, 현재
Task, Change Unit, checkpoint, `scope_revision`, baseline, request, 영향받는 ref와 호환되는
근거가 필요합니다. 제품·기술·민감 resolution은 이 필드에서 허용되지 않습니다.

범위 또는 Change Unit 효과를 적용하기 전에 Core는 현재 보류 중인 사용자 행동
요청의 `required_for`에 `scope_update`가 있고 그 행동 종류, Task, 현재 Change Unit,
`scope_revision`, 근거, 영향받는 참조가 이 연산과 일치하면
`DECISION_UNRESOLVED`로 거절합니다. 정보 제공용 요청과 해결됨, 오래됨, 대체됨,
만료됨, 불일치, 행동 종류 비호환 요청은 갱신을 막지 않습니다.

## 요청 스키마

이 메서드는 아래 생성 표의 최상위 `params` 요청 필드를 담당합니다. `envelope`는
[API 코어 스키마](schema-core.md#tool-envelope)의 공통 `ToolEnvelope`이며, 표는
`ToolEnvelope` 필드를 다시 정의하지 않습니다. 필수 여부와 Null 허용 여부는 의미
기반 요청 설명자에서 직접 가져옵니다.

<!-- BEGIN GENERATED: contract-structures api.method.update_scope.request[params] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `UpdateScopeRequest` 필드

| 필드 | 필수 | Null 허용 | 형식 |
|---|---|---|---|
| `acceptance_criteria` | 예 | 예 | `AcceptanceCriterionReplacement[]` |
| `autonomy_boundary` | 예 | 예 | `string` |
| `baseline_ref` | 예 | 예 | `BaselineRef` |
| `change_unit` | 예 | 아니요 | `ChangeUnitUpdate` |
| `envelope` | 예 | 아니요 | `ToolEnvelope` |
| `goal_summary` | 예 | 예 | `string` |
| `non_goals` | 예 | 예 | `string[]` |
| `related_scope_decision_refs` | 예 | 아니요 | `StateRecordRef[]` |
| `scope_boundary` | 예 | 예 | `string` |
| `scope_update` | 예 | 예 | `ScopeUpdate` |
| `task_id` | 예 | 아니요 | `TaskId` |
<!-- END GENERATED: contract-structures api.method.update_scope.request[params] -->



중첩 형태 담당 문서:
- `acceptance_criteria`는 `AcceptanceCriterionReplacement[]`을 사용합니다.
  중첩 형태는 [API 상태 스키마](schema-state.md#evidence-and-run-snapshot-shapes)가 담당합니다.
- `related_scope_decision_refs`는 `StateRecordRef[]`를 사용합니다. 중첩 형태는 [API 상태 스키마](schema-state.md#state-references)가 담당합니다.
- `change_unit.operation` 값은 [API 값 집합의 메서드 내부 값](schema-value-sets.md#method-local-values)이 담당합니다.
- `change_unit.effect_contract`가 있으면 `ChangeUnitEffectContract`를 사용합니다. 중첩 형태는 [API 상태 스키마](schema-state.md#changeuniteffectcontract)가 담당합니다.

## 접근 요구사항

커밋되는 `dry_run`이 아닌 요청에는 아래 조건이 필요합니다.

- `operation_category=agent_workflow`인 확인된 호출 맥락
- Product Repository가 Git 기반이면 확인된 현재 작업 공간 맥락
- 같은 프로젝트의 호환되는 `Task`
- 현재 적용 Change Unit을 만들거나 교체할 때 다음 안전한 행동을 정직하게 만들 만큼 충분한 범위
- `Task.mode=advisor`에서는 정규 비쓰기 advisor 조건을 만족하는 현재 또는 제안 Change Unit

## 상태 버전 동작

커밋된 `dry_run`이 아닌 결과는 `project_state.state_version`을 정확히 한 번 올립니다.

커밋 전에 Core는 권위 있는 프로젝트 정책과 제안된 범위/효과 계약을 기준으로
`Task`의 유효 통제 수준을 다시 평가합니다. `sensitive`를 포함해 수준을 높일 수
있지만 활성 `Task`를 자동으로 낮추지는 않습니다. 따라서 정책 완화는 활성
`Task`를 바꾸지 않고, 강화된 정책이나 새로 드러난 민감 효과는 커밋된
`StateSummary`에 반영됩니다.

기준 문장이나 `evidence_requirement`의 실질적 변경은 `Task` 범위 리비전을
증가시킵니다. 유지된 `AcceptanceCriterionId`가 같더라도 이전 범위에서 기록한
증거 범위는 `stale`로 표시됩니다. 대상 identity가 현재라는 사실만으로 이전
범위 증거가 현재 상태가 되지는 않습니다.

커밋된 갱신이 아래 상태 결속 유효성 좌표 중 하나를 바꾸면 Core는
`status=active`인 쓰기 티켓을 무효화합니다.

- `scope_revision`
- 기준선
- 현재 적용 Change Unit
- 현재 적용 Change Unit에 기록된 작업 공간 결합

저장되는 무효화 사유는 각각 `scope_revision_changed`, `baseline_changed`,
`change_unit_changed`, `workspace_changed`입니다. 정규화된 무효과 갱신,
`scope_revision`을 바꾸지 않는 수락 기준/범위 밖 항목/자율성 경계 편집, 관련
없는 `state_version` 증가는 티켓을 무효화하지 않습니다. 무효화는 티켓을
소비하거나 조용히 재사용하지 않습니다.

범위 결정을 적용하면 결정적인 `ShapingDecisionApplication`을 만들고 결과 scope revision,
baseline, Change Unit에 결합하며 현재 checkpoint에 연결합니다. 같은 transaction에서 선택한
scope gap만 `accepted`에서 `applied`로 바꾸고 scope revision을 증가시키며 결과와 event에
정확한 application ref를 포함합니다. 호환되는 no-op 또는 Change Unit 생성은 scope decision ref 없이
현재 checkpoint를 보존하고 rebase할 수 있습니다. 전이가 checkpoint의 scope 또는
baseline 권한 근거를 실제로 무효화할 때만 checkpoint를 supersede합니다. Scope, baseline,
호환되지 않는 Change Unit 변경은 영향받는 현재 application을 명시적으로 `stale`로
표시하며 row 부재를 무효화로 해석하지 않습니다.
`work/implementation` 중에는 현재 shaping application을 stale로 만들 scope, baseline,
Change Unit 갱신을 mutation 전에 거부합니다. 타입이 지정된 무효과 recovery는 영향받는
application ref를 제시하고 Task가 소유된 close/supersede 전이를 통해 implementation을
벗어나도록 요구합니다. 이 메서드는 Task를 shaping으로 조용히 되돌리지 않습니다.
Core가 거부된 transition에 대해 제출 baseline을 평가하면 서로 독립된 네 compatibility
fact는 `attempt_details.attempt_kind=baseline_transition`과 그 typed
`baseline_compatibility` 값에 들어갑니다.

거부, 보류, 만료, 불일치 상태의 shaping 결정은 scope 권한을 부여하지 않습니다. 이
상태에서는 정규 machine이 시도한 Update Scope action을 생략하고 효과 없는
`TransitionRejection`을 반환합니다. 현재 catalog가 복구를 제공하면
`recovery_action_key`가 정확한 `volicord.record_shaping_checkpoint` semantic variant를
식별합니다. 의미상 no-op인 scope 요청도 이 gate를 우회할 수 없습니다.

## 성공 결과

커밋된 `UpdateScopeResult`는 `base.response_kind=result`와
`base.effect_kind=core_committed`를 사용합니다.

## 메서드 결과 필드

`UpdateScopeResult`는 성공적으로 커밋된 범위 갱신에 대한 메서드별 결과 분기입니다. 이 결과는 결과 효과로 `core_committed`만 허용하는 `base: UpdateScopeResultBase`와 아래 메서드 소유 최상위 필드를 담습니다.

<!-- BEGIN GENERATED: contract-structures api.method.update_scope.response[response_variants] api.method.update_scope.response[result_body] api.method.update_scope.response[result_metadata] api.method.update_scope.response[rejection] api.method.update_scope.response[dry_run] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `UpdateScopeResult` 성공 필드

| 필드 | 필수 | Null 허용 | 형식 |
|---|---|---|---|
| `applied_scope_decision_refs` | 예 | 아니요 | `StateRecordRef[]` |
| `applied_shaping_decision_application_refs` | 예 | 아니요 | `StateRecordRef[]` |
| `applied_shaping_gap_refs` | 예 | 아니요 | `StateRecordRef[]` |
| `base` | 예 | 아니요 | `UpdateScopeResultBase` |
| `blocker_refs` | 예 | 아니요 | `StateRecordRef[]` |
| `change_unit_ref` | 아니요 | 예 | `StateRecordRef` |
| `stale_write_ticket_refs` | 예 | 아니요 | `StateRecordRef[]` |
| `state` | 예 | 아니요 | `StateSummary` |
| `task_ref` | 예 | 아니요 | `StateRecordRef` |

### `결과 Metadata: core_committed` 필드

계약: `dry_run`은 `false`입니다; `events`는 하나 이상의 이벤트를 포함합니다(`minItems: 1`).

| 필드 | 필수 | Null 허용 | 형식 |
|---|---|---|---|
| `disclosure` | 예 | 아니요 | `GuaranteeDisclosure` |
| `dry_run` | 예 | 아니요 | `boolean enum(false)` |
| `effect_kind` | 예 | 아니요 | `string enum("core_committed")` |
| `events` | 예 | 아니요 | `NonEmptyEventRefs` |
| `response_kind` | 예 | 아니요 | `string enum("result")` |
| `state_version` | 예 | 아니요 | `integer` |
| `transition` | 예 | 예 | `TransitionDescriptor` |

### `dry_run` 요청 정책

- `volicord.update_scope`: `dry_run=true`가 `ToolDryRunResponse` 미리보기 분기를 선택하며, 이 분기의 `base.dry_run`은 `true`입니다. `dry_run=false`이거나 `dry_run`이 생략되면 미리보기 분기를 선택하지 않습니다.


### 공유 응답 구조

응답 설명자는 성공, 거절, 미리보기를 정확한 `anyOf` 분기 union으로 정의합니다. 거절 분기는 생성된 [`ToolRejectedResponse`](schema-core.md#common-response) 구조를 사용합니다. 메서드 동작이 미리보기 분기를 선택할 때는 생성된 [`ToolDryRunResponse`](schema-core.md#common-response) 구조를 사용합니다. 공유 거절 및 미리보기 필드는 위 성공 필드와 구분된 상태로 유지됩니다.
<!-- END GENERATED: contract-structures api.method.update_scope.response[response_variants] api.method.update_scope.response[result_body] api.method.update_scope.response[result_metadata] api.method.update_scope.response[rejection] api.method.update_scope.response[dry_run] -->

지원되는 `change_unit.operation` 값은 [API 값 집합](schema-value-sets.md#method-local-values)이 담당합니다. 이 메서드는 각 작업이 `change_unit_ref`, `state.active_change_unit_ref`, 오래된 쓰기 티켓 참조, 차단 사유 참조, 태그 기반 `state.workflow` projection에 어떻게 반영되는지를 담당합니다.

`change_unit.operation=create_current` 또는 `change_unit.operation=replace_current`일 때 일반 form이 제공한 `change_unit.effect_contract`는 값이 있으면 기록하고, Advisor form에서는 고정된 정규 contract를 항상 기록합니다. 효과 계약은 워크플로 엔진을 만들거나 사용자 소유 권한 기록을 대신하지 않으면서 허용 효과, 금지 효과, 허용 Product Repository 경로, 기대 출력, 불변 조건, 증거 기대, 민감 동작 기대를 표현할 수 있습니다. 같은 작업은 이후 쓰기 준비에 사용할 확인된 작업 공간 좌표도 기록합니다. Git 저장소가 아니면 VCS 결합을 기록하지 않고 Git 전용 비교 검사도 적용하지 않습니다.

`applied_shaping_gap_refs`와 `applied_scope_decision_refs`는 이 호출이 적용한 정확한 scope
gap과 resolution만 식별합니다. 제품·기술·민감 gap은 각자의 application owner를 위해
변경하지 않습니다.

## 차단 결과

범위가 아직 준비되지 않았을 때 메서드가 소유한 차단 사유 또는 현재 행 갱신을 커밋할 수 있습니다.

커밋된 차단 범위 결과는 필요한 사용자 소유 판단 범주를 식별해야 합니다.

- `product_decision`
- `technical_decision`
- `scope_decision`
- `sensitive_approval`

허용되지 않는 것:

- 차단된 범위 결과는 필요한 판단을 막연한 모호함 뒤에 숨기면 안 됩니다.

## 거절 결과

커밋 전 실패가 있으면 `ToolRejectedResponse`를 반환합니다. 예시는 아래와 같습니다.

- 오래된 `expected_state_version`
- 유효하지 않은 `Task` 식별
- 유효하지 않은 Change Unit 작업
- 필요한 범위 누락
- 범위 위반
- 미해결 필수 판단
- 자율성 경계 위반
- 오래된 기준선
- keep-current 기준선 retargeting. 효과 없는 typed `AuthorityBasisMismatch`로 반환하며
  재시도 행동에는 `replace_current`가 필요합니다.
- 행위자 출처 또는 작업 범주 불일치
- 검증기 실패

공개 오류 코드 의미, 우선순위, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 상태 효과 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- 범위, Change Unit, 차단 사유, 쓰기 티켓 상태를 만들지 않습니다.

## 저장 효과

커밋 시 범위 담당 현재 상태와 오래된 쓰기 티켓 처리 결과를 지속할 수 있습니다. 정확한 저장 효과는 아래 저장 담당 문서가 담당합니다.

아래 예시는 메서드 안에서만 성립하도록 짧게 구성했습니다. 대표 응답은 범위 갱신 결과 분기, 참조, 상태 버전, 현재 적용 범위, 현재 적용 Change Unit, 생명주기, 다음 행동을 보여 주는 데 필요한 필드로 축약했습니다.

메서드 안의 전제: `task_filter_001`은 `proj_filter_001`에 `state_version: 18`로 이미 있으며, 알맞은 현재 적용 Change Unit이 없습니다. 이 요청은 `cu_filter_001`을 현재 적용 Change Unit으로 만듭니다.

## 최소 유효 요청

```yaml contract=api.method.update_scope.request shape=complete_request
method: volicord.update_scope
params:
  envelope:
    project_id: proj_filter_001
    task_id: task_filter_001
    request_id: req_scope_filter_001
    idempotency_key: idem_scope_filter_001
    expected_state_version: 18
    dry_run: false
    locale: ko-KR
  task_id: task_filter_001
  goal_summary: "저장된 검색 필터를 담당자와 라벨 필드로 제한합니다."
  scope_update:
    include:
      - "저장 필터 편집을 담당자와 라벨 필드로 제한합니다."
      - "저장 필터 검증 테스트를 갱신합니다."
    exclude:
      - "검색 색인 동작."
  scope_boundary: "저장 필터의 담당자·라벨 편집과 관련 테스트."
  non_goals:
    - "검색 색인 동작."
  acceptance_criteria:
    - acceptance_criterion_id: null
      statement: "저장 필터는 담당자와 라벨 이외의 필드 변경을 거부합니다."
      evidence_requirement: required
  autonomy_boundary: "저장 필터 편집 검증과 관련 테스트만 다룹니다."
  baseline_ref: baseline_filter_001
  change_unit:
    operation: create_current
    scope_summary: "저장 필터의 담당자·라벨 편집 검증."
    affected_areas:
      - "저장 필터 편집 화면"
      - "저장 필터 검증 테스트"
    affected_paths:
      - src/search/saved-filter.ts
      - src/search/filter-form.ts
      - tests/saved-filter.test.ts
    constraints:
      - "검색 색인 동작은 범위에서 제외합니다."
  related_scope_decision_refs: []
```

## 대표 응답

축약한 결과 분기(`UpdateScopeResult`, 커밋됨):

```schema
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 19
  events:
    - event_id: evt_filter_001
      event_kind: scope_updated
task_ref:
  record_kind: task
  record_id: task_filter_001
  project_id: proj_filter_001
  task_id: task_filter_001
  produced_at_state_version: 19
change_unit_ref:
  record_kind: change_unit
  record_id: cu_filter_001
  project_id: proj_filter_001
  task_id: task_filter_001
  produced_at_state_version: 19
applied_shaping_gap_refs: []
applied_scope_decision_refs: []
applied_shaping_decision_application_refs: []
stale_write_ticket_refs: []
blocker_refs: []
state:
  project_id: proj_filter_001
  state_version: 19
  task_ref:
    record_kind: task
    record_id: task_filter_001
    project_id: proj_filter_001
    task_id: task_filter_001
    produced_at_state_version: 19
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  work_phase: shaping
  goal_summary: "저장된 검색 필터를 담당자와 라벨 필드로 제한합니다."
  scope_summary: "저장 필터의 담당자·라벨 편집 검증."
  non_goals:
    - "검색 색인 동작."
  acceptance_criteria:
    - acceptance_criterion_id: criterion_filter_001
      statement: "저장 필터는 담당자와 라벨 이외의 필드 변경을 거부합니다."
      evidence_requirement: required
  autonomy_boundary: "저장 필터 편집 검증과 관련 테스트만 다룹니다."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_filter_001
    project_id: proj_filter_001
    task_id: task_filter_001
    produced_at_state_version: 19
  baseline_ref: baseline_filter_001
  workspace_context:
    vcs: git
    git_common_dir: "/work/search/.git"
    worktree_id: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
    branch_ref: "refs/heads/filter-scope"
    head_sha: "0123456789abcdef0123456789abcdef01234567"
    workspace_fingerprint: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
  # 현재의 완전한 WorkflowProjection은 이 축약 예시에서 생략했습니다.
  pending_user_action_summaries: []
  blocker_refs: []
  write_ticket_summary: null
  evidence_summary: null
  close_state: null
  close_blockers: []
  guarantee_display: null
```

## 담당 문서 링크

- 요청 래퍼와 응답 분기: [API 코어 스키마](schema-core.md).
- 상태 참조, `StateSummary`, 태그 기반 워크플로 진행 상태, 차단 사유: [API 상태 스키마](schema-state.md).
- 범위 관련 사용자 판단 형태: [API 판단 스키마](schema-judgment.md).
- 지원되는 값 집합, `change_unit.operation` 의미, 작업 범주: [API 값 집합](schema-value-sets.md#operation-category-values).
- 공개 오류, 우선순위, 거절 응답 처리 경로: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md).
- 저장 효과와 오래된 쓰기 티켓 동작: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
