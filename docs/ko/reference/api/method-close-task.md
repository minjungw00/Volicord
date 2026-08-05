<a id="volicordclose_task"></a>
<a id="volicordcheck_close"></a>

# `volicord.check_close`와 `volicord.close_task` 참조

닫기 준비 상태와 workflow 진행은 별개입니다. ready advisor shaping checkpoint는 advisor
finalization을 선택할 뿐 그 자체가 자문 결과 근거는 아닙니다.
`volicord.finalize_advice`만 checkpoint 기반 advisor 닫기 근거를
만들 수 있습니다. work shaping checkpoint는 implementation close basis가 될 수 없습니다. 차단
사유 자체의 해결 행동은 더 이른 workflow 필수 shaping 행동을 덮어쓰지 않습니다.

## 담당하는 것

이 문서는 닫기 메서드 묶음의 기준 동작을 담당합니다.

- `volicord.check_close`와 `volicord.close_task`의 메서드별 요청 조건, `intent` 처리, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- `volicord.check_close`와 `volicord.close_task` 요청에 적용되는 메서드별 평가 순서
- `CheckCloseResult.blockers`와 `CloseTaskResult.blockers`를 만드는 메서드별
  차단 사유 분기
- 메서드별 `CloseReadinessBlocker.code` 생성 동작
- Task 닫기 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- `ToolEnvelope`, `ToolRejectedResponse`, `ToolDryRunResponse`의 공통 스키마 본문
- 상태, 아티팩트, 판단, 값 집합, 오류의 중첩 스키마 정의
- Core의 닫기 준비 상태 권한 개념
- `CloseReadinessBlocker` 형태나 `CloseReadinessBlocker.category` 값
- 공개 오류 코드 의미, 오류 우선순위, 응답 분기 처리 경로
- 저장소 배치, 저장 효과 세부사항, 보안 보장, 렌더링 문구

## 목적

`volicord.check_close`는 선택된 `Task`의 닫기 준비 상태를 읽기 전용 메서드로 평가합니다. `volicord.close_task`는 요청한 종료 경로가 그 검사를 통과한 뒤 지원되는 종료 상태 변경을 수행합니다.

이 메서드들은 다음 결과를 낼 수 있습니다.

- `volicord.check_close`를 통한 읽기 전용 닫기 준비 상태 관찰 반환
- `intent=complete`, `intent=cancel`, `intent=supersede` 커밋
- `CloseTaskResult.blockers`를 담은 `CloseTaskResult(close_state=blocked)` 반환
- 닫기 준비 상태 평가 전 요청 거절
- 유효한 상태 변경 미리보기에 대한 공통 `dry_run` 미리보기 반환

닫기는 보고서가 아니라 Core 상태 전이입니다. `volicord.close_task`는 `intent=complete`에서 현재 닫기 근거를 평가합니다. 대화, 상태 텍스트, 종료 닫기 요약, 최종 수락만, 잔여 위험 수락만, 증거만, 쓰기 티켓, 렌더링된 보기에서 닫기를 추론하지 않습니다.

## 담당 경계

메서드 담당 블록:

- `volicord.check_close`의 요청 검증
- `volicord.close_task`의 요청 검증과 `intent` 필드 조합
- 이 메서드들이 읽기 전용 확인, 상태 변경, 차단, 거절, `dry_run` 분기에 도달하는 순서
- 유효한 상태 변경 분기가 종료 결과를 커밋하는지, 응답 전용 차단 결과를 반환하는지 여부
- `CheckCloseResult.blockers` 또는 `CloseTaskResult.blockers`에서 생성할 수 있는
  메서드별 차단 사유 코드

Core 담당 블록:

- 닫기 준비 상태 권한, 정직한 닫기, 최종 수락, 잔여 위험 표시, 잔여 위험 수락, 대체 금지 규칙은 [Core 모델의 닫기 준비 상태](../core-model.md#close_task)가 담당합니다.

API 경계 블록:

- 차단 사유와 API 응답 사이의 처리 경로는 [API 차단 사유 처리 경로](blocker-routing.md)가 담당합니다.
- 오류 우선순위와 `STATE_VERSION_CONFLICT` 선택은 [API 오류 우선순위](error-precedence.md)가 담당합니다.
- 거절, 차단, `dry_run` 응답 분기 처리 경로는 [API 오류 처리 경로](error-routing.md)가 담당합니다.

스키마와 표시 블록:

- `CloseReadinessBlocker`와 상태 형태 데이터는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다.
- 정확한 `intent` 값 이름은 [API 값 집합의 메서드 내부 값](schema-value-sets.md#method-local-values)이 담당합니다.
- 정확한 `close_reason`과 `close_state` 값 이름은 [API 값 집합의 Task 생명주기 값](schema-value-sets.md#task-lifecycle-values)이 담당합니다.
- 정확한 차단 사유 범주 값 이름은 [API 값 집합의 상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values)이 담당합니다.
- 지속 저장 효과는 [저장 효과](../storage-effects.md)가 담당합니다.
- 렌더링 문구는 [템플릿 본문](../template-bodies.md)이 담당합니다.

## 조건

사전 확인 조건:

- 요청 래퍼와 메서드 필드가 유효해야 합니다.
- `params.task_id`는 요청이 선택한 같은 프로젝트의 `Task`를 가리켜야 합니다.
- `volicord.close_task`에서는 요청한 `intent`, `close_reason`, `superseding_task_id` 조합이 유효해야 합니다.
- 확인된 호출 맥락, 작업 범주, 호환 행위자 출처, 종료 경로 선행조건이 요청한 경로를 허용해야 합니다.

읽기 전용 확인 조건:

- `volicord.check_close`에는 `intent`, `close_reason`, `superseding_task_id`, 종료 상태 변경 경로가 없습니다. 현재 닫기 준비 상태 관찰을 반환하며 닫기 상태를 커밋하면 안 됩니다.

상태 변경 조건:

- `dry_run=false`인 상태 변경 `intent`에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- 오래된 `expected_state_version` 또는 멱등 요청 해시 충돌은 닫기 준비 상태 평가 전에 거절됩니다.
- 닫기 관련 쓰기 티켓은 현재 Task, Change Unit, 범위 리비전, 기준선, workspace 맥락, 현재 정규화된 프로젝트 쓰기 권한 결속, 정규 typed 승인 평가, Task 상태, 선택적으로 설정된 idle timeout이라는 명시적 유효성 근거로 한 번 평가합니다. 닫기 준비 상태는 이 평가된 ticket 상태를 소비하며 승인 참조나 resolution ID를 독립적으로 비교하지 않습니다. 감사 전용 `basis_state_version`은 전역 상태 버전과 비교하지 않습니다. 영속 승인 참조의 소유자 불일치나 identity 중복은 Store 손상이며 의미 변경 결과가 될 수 없고, 그 밖에 유효한 승인의 만료는 의미 기반 현재성 결과입니다.
- 쓰기 티켓 유효성 확인은 최종 수락, 잔여 위험 수락, 사용자 소유 판단, 민감 동작 승인, 포괄적 승인을 기록하지 않습니다.

닫기 조건:

- `intent=complete`는 사전 확인이 성공하고, 현재 `CurrentCloseBasis`에 대한 닫기 준비 상태 평가가 유효하며, 현재 닫기 근거 참조가 모드와 호환되는 아티팩트, 실행 기록 또는 shaping-checkpoint lineage 규칙을 만족하고, 닫기 차단 사유가 남아 있지 않을 때만 닫을 수 있습니다.
- `intent=check`와 `intent=complete`의 닫기 준비 상태는 해당 `Task`의 쓰기 티켓이 활성이고 소비되지 않은 채 남아 있을 때 차단됩니다. 무효화되거나 취소됐거나 idle timeout에 따라 유효 상태가 무효화된 티켓 행은 오래된 권한 상태로 계속 표시되지만 그 자체로 닫기를 막지 않습니다. 범위 밖 Product Repository 경로를 포함한 미해결 Unrecorded Change는 별도 차단 사유로 유지됩니다. 고정 티켓 만료는 없습니다.
- 유효한 `effective_control_level=observe` Task에는 쓰기 티켓이나 제품 파일 쓰기 경로가 없습니다. 닫기 준비 상태 결과는 `volicord.prepare_write`를 추천하지 않습니다. `advisor`에서 현재 결과, evidence, 닫기 근거가 없으면 `volicord.finalize_advice`로 안내하며 `volicord.record_run`을 절대 안내하지 않습니다. direct/work에서 호환되는 현재 결과나 닫기 근거 작업은 계속 Run owner가 담당합니다.
- 최종 수락은 Task 모드, 유효 통제 수준, 권위 있는 프로젝트 정책을 따릅니다. `advisor`는 유효 통제가 `observe`여도 항상 호환되는 final-acceptance UserAction을 요구합니다. direct/work에서 `sensitive`와
  `tracked`는 호환되는 최종 수락을 요구하고 `observe`는 `not_required`를 사용합니다.
  `light`는 `policy_dependent`이며 현재 프로젝트 정책이 명시적으로 허용하고 민감 동작,
  미해결 사용자 요구사항, 잔여 위험 수락 요구사항, 필수 Evidence 공백, 미해결
  Unrecorded Change가 없는 등 정책 조건을 모두 계속 만족할 때만
  `not_required`를 사용할 수 있습니다. 정책 강화는 이 판단 전에 유효 통제 수준을
  올리며 절대 낮추지 않습니다. 어떤 정책도 Evidence, 민감 승인, 위험 수락, 다른
  blocker를 면제하지 않습니다.
- 유효 `sensitive` 통제에는 티켓으로 결속된 정확한 민감 동작 근거와 현재 일치하는
  사용자 소유 승인도 필요합니다. 최종 수락은 어느 것도 대신하지 않습니다. 정확한
  근거를 기록하지 않았으면 닫기는 `missing_sensitive_action_basis`, 근거는 있지만
  승인이 더 이상 현재 상태가 아니면 `missing_sensitive_approval`을 보고합니다.
- 미해결 Unrecorded Change는 조정으로 해결될 때까지 닫기를 막습니다. 저장소 관찰을
  사용할 수 없다는 진단은 별도로 남고 `unresolved_unrecorded_changes`를 만들지
  않습니다.
- 현재 수락 기준 중 `evidence_requirement=required`인 항목만 증거 닫기
  요구사항을 만듭니다. 각 항목에는 현재 상태이고 대상이 일치하는 증거 관찰
  출처가 필요합니다. `optional`, `not_required`, 보충 대상, 폐기된 대상은
  닫기를 차단하지 않습니다. 더 강한 출처가 필요한 기준에는 확인되지 않은
  주장, 출처 없는 증거, 오래됨, 반박됨, 부분적임, 뒷받침되지 않음,
  협력적 에이전트 보고만으로 된 증거가 충분하지 않습니다.
  Strong 평가는 현재 바이트 무결성, authority-owned producer 레코드, 정확한
  출력 결합, 현재 Task/scope/baseline 및 대상, supported relevance를 독립적으로
  요구합니다. 재사용 Evidence는 원래 producer 및 relevance 레코드를 모든
  단계에서 재귀 검증합니다.
- `intent=cancel`은 `machine_action=accept`, `resolution_outcome=accepted`, `resolved_by_actor_source=local_user`, 호환 User Channel 출처, `Task`, 현재 범위 리비전, 현재 적용 Change Unit에 묶인 근거를 가진 현재 수락된 취소 판단을 요구합니다. 완료 전용 증거, 최종 수락, 잔여 위험 수락은 필요하지 않습니다.
- `intent=supersede`는 요청한 종료 경로를 평가합니다. 증거 충분성, 최종 수락, 잔여 위험 수락이 아닙니다.

성공한 종료 닫기가 만드는 종료 닫기 요약은 현재 닫기 전 근거가 아니며 `CurrentCloseBasis`의 대체물로 쓰지 않습니다.

## 닫기 의도

`volicord.check_close`에는 `intent` 필드가 없습니다. 지원되는 `volicord.close_task.intent` 값은 [API 값 집합의 메서드 내부 값](schema-value-sets.md#method-local-values)이 담당합니다. 지원되는 `close_reason`과 `close_state` 값은 [API 값 집합의 Task 생명주기 값](schema-value-sets.md#task-lifecycle-values)이 담당합니다.

| `intent` | `close_reason` | `superseding_task_id` | 메서드 규칙 |
|---|---|---|---|
| `complete` | `completed_self_checked` 또는 `completed_with_risk_accepted` | `null` | 완료 경로이며 닫기 준비 상태 평가를 실행합니다. |
| `cancel` | `cancelled` | `null` | 취소 경로이며 호환되는 `accepted` 취소 권한을 요구하고 취소 전용 종료 제약을 평가합니다. |
| `supersede` | `superseded` | `null`이 아닌 같은 프로젝트의 대체 `Task` 참조 | 대체 경로이며 대체 전용 종료 제약을 평가합니다. |

## 필수 입력

모든 `volicord.check_close` 호출에는 아래 입력이 필요합니다.

- `project_id`, `request_id`, `dry_run`을 포함한 메서드 필수 요청 래퍼 필드를 가진 `ToolEnvelope`
- 요청 래퍼가 선택한 요청 맥락과 메서드 params에서 일치하는 `task_id`

모든 `volicord.close_task` 호출에는 아래 입력이 필요합니다.

- `project_id`, `request_id`, `dry_run`을 포함한 메서드 필수 요청 래퍼 필드를 가진 `ToolEnvelope`
- 요청 래퍼가 선택한 요청 맥락과 메서드 params에서 일치하는 `task_id`
- `intent`
- `close_reason`
- `superseding_task_id`
- `user_note`

추가 요구사항:

| 경우 | 필수 입력 규칙 |
|---|---|
| `volicord.check_close` | `idempotency_key`와 `expected_state_version`은 `null`일 수 있습니다. 닫기 의도 필드는 허용되지 않습니다. |
| `intent=complete`, `intent=cancel`, `intent=supersede`와 `dry_run=false` | `idempotency_key`와 `expected_state_version`은 `null`이 아니어야 하며 현재 값이어야 합니다. |
| `intent=supersede` | `superseding_task_id`는 호환되는 같은 프로젝트의 대체 `Task`를 가리켜야 합니다. |

## 요청 스키마

이 문서는 아래 생성 표의 최상위 `params` 요청 필드를 담당합니다. `envelope`는
[API 코어 스키마](schema-core.md#tool-envelope)의 공통 `ToolEnvelope`이며, 표는
`ToolEnvelope` 필드를 다시 정의하지 않습니다. 필수 여부와 Null 허용 여부는 의미
기반 요청 설명자에서 직접 가져옵니다.

<!-- BEGIN GENERATED: contract-structures api.method.check_close.request[params] api.method.close_task.request[params] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `CheckCloseRequest` 필드

| 필드 | 필수 | Null 허용 | 형식 |
|---|---|---|---|
| `envelope` | 예 | 아니요 | `ToolEnvelope` |
| `task_id` | 예 | 아니요 | `TaskId` |

### `CloseTaskRequest` 필드

| 필드 | 필수 | Null 허용 | 형식 |
|---|---|---|---|
| `close_reason` | 예 | 예 | `CloseReason` |
| `envelope` | 예 | 아니요 | `ToolEnvelope` |
| `intent` | 예 | 아니요 | `CloseMutationIntent` |
| `superseding_task_id` | 예 | 예 | `TaskId` |
| `task_id` | 예 | 아니요 | `TaskId` |
| `user_note` | 예 | 예 | `string` |
<!-- END GENERATED: contract-structures api.method.check_close.request[params] api.method.close_task.request[params] -->



중첩 형태 담당 문서:
- `intent` 값은 [API 값 집합의 메서드 내부 값](schema-value-sets.md#method-local-values)이 담당합니다.
- `close_reason` 값은 [API 값 집합의 Task 생명주기 값](schema-value-sets.md#task-lifecycle-values)이 담당합니다.

## 접근 요구사항

| 요청 종류 | 메서드 접근 규칙 |
|---|---|
| `volicord.check_close` | 보호된 닫기 준비 상태 세부정보에는 `operation_category=read`인 확인된 호출 맥락이 필요합니다. |
| 상태 변경 `intent` | `operation_category=agent_workflow`인 확인된 호출 맥락, 호환되는 `Task` 상태, 닫기 관련 담당 기록이 필요합니다. |

이 메서드를 호출할 접근 권한은 사용자 소유 판단, 최종 수락, 잔여 위험 수락, 민감 동작 승인, 쓰기 티켓과 별개입니다.

## 메서드 흐름

구현은 `volicord.check_close`를 아래 순서로 평가합니다.

1. 요청 래퍼, 메서드 필드, 같은 프로젝트의 `Task` 식별자를 검증합니다. 형태 오류, 잘못된 프로젝트 식별자, 읽을 수 없는 `Task` 식별자는 `ToolRejectedResponse`를 반환합니다.
2. 호출 맥락, 작업 범주, 행위자 출처를 확인합니다.
3. [`volicord.status`](method-status.md)에서 증거나 닫기 상태 보기를 선택했을 때와 같은 방식으로 현재 닫기 준비 상태와 기준 `EvidenceGateSummary`를 계산하고 `CheckCloseResult`를 반환합니다. 최상위 gate, `state.evidence_gate`, `summary_card.evidence`는 이 하나의 결과를 재사용합니다.

구현은 `volicord.close_task`를 아래 순서로 평가합니다.

1. 요청 래퍼, 메서드 필드, `intent` 필드 조합, 같은 프로젝트의 `Task` 식별자를 검증합니다. 형태 오류, 잘못된 프로젝트 식별자, 읽을 수 없는 `Task` 식별자는 `ToolRejectedResponse`를 반환합니다.
2. 호출 맥락, 작업 범주, 행위자 출처, 요청한 종료 경로의 선행조건을 확인합니다.
3. `dry_run=false`인 상태 변경 `intent`에서는 `idempotency_key`, 현재 `expected_state_version`, 멱등 요청 해시, 현재 프로젝트 쓰기 권한 결속을 포함한 닫기 관련 각 쓰기 티켓의 명시적인 상태 결합 유효성을 확인합니다. 충돌하는 envelope 버전이나 요청 해시는 `ToolRejectedResponse`를 반환합니다. 무효화되거나 취소됐거나 정책 권한이 stale이거나 idle timeout에 따라 유효 상태가 무효화된 티켓은 계속 표시되지만 그 자체로 닫기 차단 사유를 만들지 않습니다. `basis_state_version`은 감사 전용입니다.
4. 상태 변경 `intent`와 `dry_run=true` 조합은 유효한 사전 확인 뒤 공통 미리보기 분기를 반환합니다.
5. `intent=complete`는 현재 `CurrentCloseBasis`에 대한 닫기 준비 상태 평가를 실행합니다. 차단 사유가 남아 있으면 차단 분기를 반환하고, 없으면 `close_state=closed`, 모드와 호환되는 종료 닫기 결과, 잔여 위험 수락이 필요하지 않은 닫기 근거의 알려진 한계에 대해 메서드가 선택한 프로젝트 연속성 기록을 커밋합니다. 종료 결과는 `Task.mode=advisor`에서 `advice_only`, `Task.mode=direct` 또는 `work`에서 `completed`입니다.
6. `intent=cancel`은 `machine_action=accept`, `resolution_outcome=accepted`, `resolved_by_actor_source=local_user`, 호환 User Channel 출처를 가지며 현재 `Task`, 범위 리비전, Change Unit과 호환되는 현재 수락된 `judgment_kind=cancellation`을 요구합니다. 취소 권한이 없거나 호환되지 않으면 차단 분기를 반환합니다.
7. `intent=cancel` 또는 `intent=supersede`는 요청한 종료 경로만 평가합니다. 종료 경로 차단 사유가 남아 있으면 차단 분기를 반환하고, 없으면 해당 Task의 활성 쓰기 티켓을 `task_closed`로 원자적으로 무효화하고 `close_state=cancelled` 또는 `close_state=superseded`를 커밋합니다.

## 상태 버전 동작

| 경우 | 상태 버전 효과 |
|---|---|
| `volicord.check_close` | `dry_run=true`여도 `project_state.state_version`을 증가시키지 않습니다. |
| 성공한 종료 상태 변경 | `project_state.state_version`을 정확히 한 번 증가시킵니다. |
| 상태 변경 `intent`의 차단 결과 | `project_state.state_version`을 증가시키지 않습니다. 종료 상태 변경, 이벤트, 재실행 행 없이 `base.effect_kind=no_effect`를 반환합니다. |
| 사전 확인 거절 또는 유효한 `dry_run` 미리보기 | 아무것도 증가시키지 않습니다. |

사전 확인 거절에는 오래된 `expected_state_version`과 멱등 요청 해시 충돌이 포함됩니다. 이런 충돌은 오류 담당 문서로 처리되며 닫기 차단 사유가 아닙니다. 쓰기 티켓 무효화는 상태에 묶이고 티켓 권한 상태에 계속 표시되지만 그 자체로 닫기 차단 사유를 만들지 않습니다.

읽기 전용 확인, 그 닫기 준비 상태 계산, 관련 없는 권한 상태 변경은 활성 쓰기
티켓을 소비하거나 무효화하지 않습니다.

## 성공 결과

여기서 성공은 차단되거나 거절되지 않은 결과 분기를 뜻합니다.

`volicord.check_close`는 `CheckCloseResult`, `volicord.close_task`는
`CloseTaskResult`를 반환하며 `base.response_kind=result`를 사용합니다.

| 경우 | 효과 | `close_state` |
|---|---|---|
| `volicord.check_close`이고 현재 차단 사유가 없음 | `base.effect_kind=read_only` | `ready` |
| 성공한 `intent=complete` | `base.effect_kind=core_committed` | `closed` |
| 성공한 `intent=cancel` | `base.effect_kind=core_committed` | `cancelled` |
| 성공한 `intent=supersede` | `base.effect_kind=core_committed` | `superseded` |

성공한 `intent=complete`에서 반환된 `state.lifecycle.result`와 저장된 `Task.result`는 `Task.mode=advisor`일 때 모두 `advice_only`이고, `Task.mode=direct` 또는 `work`일 때 `completed`입니다. 이 결과 매핑은 기존 증거, 최종 수락, 잔여 위험 또는 다른 닫기 준비 상태 정책을 바꾸거나 추론하지 않습니다.

## 메서드 결과 필드

`CheckCloseResult`와 `CloseTaskResult`는 아래 닫기 평가 필드를 공유하지만 서로
다른 공개 결과 형식입니다. `CheckCloseResultBase`는 `read_only`만 허용하고
`CloseTaskResultBase`는 `core_committed`와 `no_effect`만 허용합니다.

<!-- BEGIN GENERATED: contract-structures api.method.check_close.response[response_variants] api.method.check_close.response[result_body] api.method.check_close.response[result_metadata] api.method.check_close.response[rejection] api.method.close_task.response[response_variants] api.method.close_task.response[result_body] api.method.close_task.response[result_metadata] api.method.close_task.response[rejection] api.method.close_task.response[dry_run] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `CheckCloseResult` 성공 필드

| 필드 | 필수 | Null 허용 | 형식 |
|---|---|---|---|
| `artifact_refs` | 예 | 아니요 | `ArtifactRef[]` |
| `authority_receipt` | 예 | 아니요 | `AuthorityReceipt` |
| `base` | 예 | 아니요 | `CheckCloseResultBase` |
| `blockers` | 예 | 아니요 | `CloseReadinessBlocker[]` |
| `close_state` | 예 | 아니요 | `CloseState` |
| `continuity_summary` | 예 | 아니요 | `ProjectContinuitySummary[]` |
| `current_close_basis` | 아니요 | 예 | `CurrentCloseBasis` |
| `evidence_gate` | 예 | 아니요 | `EvidenceGateSummary` |
| `evidence_summary` | 아니요 | 예 | `EvidenceSummary` |
| `pending_user_action_summaries` | 예 | 아니요 | `AgentSafeUserActionRequestSummary[]` |
| `risk_acceptance_coverage` | 예 | 아니요 | `RiskAcceptanceCoverage[]` |
| `state` | 예 | 아니요 | `StateSummary` |
| `summary_card` | 예 | 아니요 | `SummaryCard` |

### `CloseTaskResult` 성공 필드

| 필드 | 필수 | Null 허용 | 형식 |
|---|---|---|---|
| `artifact_refs` | 예 | 아니요 | `ArtifactRef[]` |
| `authority_receipt` | 예 | 아니요 | `AuthorityReceipt` |
| `base` | 예 | 아니요 | `CloseTaskResultBase` |
| `blockers` | 예 | 아니요 | `CloseReadinessBlocker[]` |
| `close_state` | 예 | 아니요 | `CloseState` |
| `continuity_summary` | 예 | 아니요 | `ProjectContinuitySummary[]` |
| `current_close_basis` | 아니요 | 예 | `CurrentCloseBasis` |
| `evidence_gate` | 예 | 아니요 | `EvidenceGateSummary` |
| `evidence_summary` | 아니요 | 예 | `EvidenceSummary` |
| `pending_user_action_summaries` | 예 | 아니요 | `AgentSafeUserActionRequestSummary[]` |
| `risk_acceptance_coverage` | 예 | 아니요 | `RiskAcceptanceCoverage[]` |
| `state` | 예 | 아니요 | `StateSummary` |
| `summary_card` | 예 | 아니요 | `SummaryCard` |

### `결과 Metadata: read_only` 필드

계약: `dry_run`은 정규화된 요청 의도를 보존합니다; `events`는 비어 있어야 합니다(`maxItems: 0`).

| 필드 | 필수 | Null 허용 | 형식 |
|---|---|---|---|
| `disclosure` | 예 | 아니요 | `GuaranteeDisclosure` |
| `dry_run` | 예 | 아니요 | `boolean` |
| `effect_kind` | 예 | 아니요 | `string enum("read_only")` |
| `events` | 예 | 아니요 | `EmptyEventRefs` |
| `response_kind` | 예 | 아니요 | `string enum("result")` |
| `state_version` | 예 | 아니요 | `integer` |

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

### `결과 Metadata: no_effect` 필드

계약: `dry_run`은 `false`입니다; `events`는 비어 있어야 합니다(`maxItems: 0`).

| 필드 | 필수 | Null 허용 | 형식 |
|---|---|---|---|
| `disclosure` | 예 | 아니요 | `GuaranteeDisclosure` |
| `dry_run` | 예 | 아니요 | `boolean enum(false)` |
| `effect_kind` | 예 | 아니요 | `string enum("no_effect")` |
| `events` | 예 | 아니요 | `EmptyEventRefs` |
| `response_kind` | 예 | 아니요 | `string enum("result")` |
| `state_version` | 예 | 아니요 | `integer` |

### `dry_run` 요청 정책

- `volicord.check_close`: `dry_run=true`를 일반 결과 분기로 처리하고 `base.dry_run=true`를 보존하며, 미리보기 응답은 만들지 않습니다. `dry_run=false`이거나 `dry_run`이 생략되면 미리보기 분기를 선택하지 않습니다.
- `volicord.close_task`: `dry_run=true`가 `ToolDryRunResponse` 미리보기 분기를 선택하며, 이 분기의 `base.dry_run`은 `true`입니다. `dry_run=false`이거나 `dry_run`이 생략되면 미리보기 분기를 선택하지 않습니다.


### 공유 응답 구조

각 응답 설명자는 지원하는 응답 분기를 정확한 `anyOf` 분기 union으로 정의합니다. 미리보기를 노출하는 응답 설명자만 해당 분기를 포함합니다. 거절 분기는 생성된 [`ToolRejectedResponse`](schema-core.md#common-response) 구조를 사용합니다. 메서드 동작이 미리보기 분기를 선택할 때는 생성된 [`ToolDryRunResponse`](schema-core.md#common-response) 구조를 사용합니다. 공유 거절 및 미리보기 필드는 위 성공 필드와 구분된 상태로 유지됩니다.
<!-- END GENERATED: contract-structures api.method.check_close.response[response_variants] api.method.check_close.response[result_body] api.method.check_close.response[result_metadata] api.method.check_close.response[rejection] api.method.close_task.response[response_variants] api.method.close_task.response[result_body] api.method.close_task.response[result_metadata] api.method.close_task.response[rejection] api.method.close_task.response[dry_run] -->

각 닫기 차단 사유의 해결 행동은 해당 `CloseReadinessBlocker.next_actions` 목록 안에만 나타나며 [API 상태 스키마](schema-state.md#current-position-display-shapes)의 기준 `NextActionSummary` 형태를 사용합니다. 각 차단 사유는 독립된 fact이고 `summary_card`는 표시 데이터이며, 진행 권한은 tagged `workflow`에 남습니다.

다른 작업을 위한 대기 사용자 행동과 정보성 전용 대기 행동은 더 넓은
`state.pending_user_action_summaries` 상태 보기에 계속 나타날 수 있습니다. 요청한
닫기 경로의 현재 `pending_user_action` 차단 사유가 해당 행동의 정확한
요청을 내부에서 선택하지 않으면 이 행동은 최상위 `pending_user_action_summaries`
목록에 들어가지 않습니다. 이 선택의 공개 출력은 해당 안전 요약뿐이며 request ref나 요청
상세를 담지 않습니다.

이 메서드는 자신이 생성하는 메서드 범위의 `CloseReadinessBlocker.code` 값을 담당합니다. 이런 코드는 공개 `ErrorCode` 값이 아니며 전역 값 집합 항목도 아닙니다.

메서드 로컬 `CloseReadinessBlocker.code` 목록:

아래 생성 의미는 이 메서드가 닫기 준비 상태 관찰 또는 종료 경로 평가에 도달한 뒤에만 적용됩니다. 사전 확인 실패는 여전히 오류 담당 문서에 따라 `ToolRejectedResponse`를 반환합니다.

| 코드 | 범주 | 로컬 생성 의미 |
|---|---|---|
| `task_not_closeable` | `task` | 선택된 `Task` 생명주기나 종료 경로 상태가 요청한 닫기 의도를 받을 수 없습니다. |
| `missing_active_change_unit` | `scope` | 닫기 경로에 현재 적용 Change Unit이 필요하지만 사용할 수 없습니다. |
| `pending_user_action` | `pending_user_action` | 필요한 사용자 소유 행동이 아직 대기 중이거나 해결되지 않았습니다. |
| `missing_sensitive_action_basis` | `sensitive_approval` | 유효 `sensitive` Task에 티켓으로 결속된 정확한 민감 동작 근거가 없습니다. `prepare_write`와 일치하는 `record_run`이 사용자 승인 동작, 범위, 기준선, Change Unit을 보존할 때까지 닫기를 차단합니다. |
| `missing_sensitive_approval` | `sensitive_approval` | 필요한 별도 민감 동작 승인이 없습니다. |
| `missing_cancellation_authority` | `user_action` | `intent=cancel`에 현재 `Task`, 범위 리비전, Change Unit에 묶이고 `resolved_by_actor_source=local_user`, 호환 User Channel 출처를 가진 현재 수락된 취소 `UserActionResolution`이 없습니다. |
| `rejected_cancellation_authority` | `user_action` | 현재 로컬 사용자의 취소 `UserActionResolution`이 `intent=cancel`을 명시적으로 거부했습니다. |
| `stale_cancellation_authority` | `user_action` | 취소 `UserActionResolution`은 있지만 그 `Task`, 범위 리비전, Change Unit 또는 유효 사용자 행동 근거가 더 이상 현재 상태가 아닙니다. |
| `open_write_ticket` | `write_compatibility` | 선택된 `Task`의 쓰기 티켓이 열려 있고 아직 해결되지 않았습니다. |
| `baseline_stale` | `baseline` | 닫기 관련 기준선 근거가 차단 사유 생성 경로에서 오래되었습니다. |
| `unresolved_unrecorded_changes` | `connection_capability` | 미해결 Unrecorded Change는 닫기 전에 조정해야 합니다. 저장소 관찰을 사용할 수 없다는 진단은 이 차단 사유를 만들지 않습니다. 이 차단 사유는 `owner_method=volicord.reconcile_changes`인 `next_actions`를 포함합니다. |
| `evidence_claim_unsupported` | `evidence_claim` | 필요한 닫기 주장이 지원되는 증거 범위를 갖지 못했습니다. |
| `evidence_claim_missing` | `evidence_claim` | 필요한 닫기 주장에 대한 현재 증거 범위 기록이 없습니다. |
| `evidence_provenance_insufficient` | `evidence_provenance` | 필요한 닫기 증거는 있지만 충분한 현재 출처와 보장 수준 출처가 없습니다. |
| `evidence_provenance_stale` | `evidence_provenance` | 증거 관찰 출처가 있지만 현재 Task 범위, Change Unit, 출처 실행 기록, 닫기 근거 증거 요약에 대해 오래되었습니다. |
| `evidence_agent_report_only` | `evidence_provenance` | 더 강한 출처가 필요한데 필요한 닫기 증거가 협력적 에이전트 보고만으로 뒷받침됩니다. |
| `artifact_unavailable` | `artifact_availability` | 닫기 관련 아티팩트가 없거나, 사용할 수 없거나, 사용에 부적합하거나, 무결성에 실패했습니다. |
| `missing_final_acceptance` | `final_acceptance` | 현재 닫기 근거에 필요한 최종 수락이 없습니다. 표시되는 행동은 Agent Connection의 `volicord.request_user_action` 절차와 최종 수락 질문을 식별합니다. |
| `stale_final_acceptance` | `final_acceptance` | 최종 수락은 있지만 현재 `Task`, Change Unit, `scope_revision`, `close_basis_revision`, 기준선, 결과 참조와 호환되지 않거나 오래되었습니다. 표시되는 행동은 현재 근거에 묶인 판단을 요청합니다. |
| `residual_risk_not_visible` | `residual_risk_visibility` | 닫기 관련 잔여 위험이 보이지 않게 남아 있습니다. |
| `missing_residual_risk_acceptance` | `residual_risk_acceptance` | 현재 잔여 위험 요구사항에 필요한 잔여 위험 수락이 없습니다. |
| `stale_residual_risk_acceptance` | `residual_risk_acceptance` | 잔여 위험 수락은 있지만 현재 `close_basis_revision`과 정확한 잔여 위험 `risk_id` 값에 일치하지 않습니다. |
| `recovery_required` | `recovery` | 요청한 닫기 경로를 진행하기 전에 복구 작업이 남아 있습니다. |

이 코드는 메서드 로컬 `CloseReadinessBlocker.code` 값입니다. 공개 `ErrorCode` 값, `WriteDecisionReason.code` 값, 전역 값 집합 항목이 아닙니다.

`pending_user_action`의 경우 차단 사유의 다음 행동은 다음 actor가 User Channel임을 나타낼 수 있고, `pending_user_action_summaries`는 정확한 agent-safe 대기 요약만 담습니다. 공개 닫기 결과에는 resolution form, 캡처 명령, User Channel credential이 들어가지 않습니다. CLI inbox renderer는 별도 내부 Core 경계에서 완전한 typed CLI inbox 항목을 얻습니다. 이 차단 사유는 Agent Connection이 사용자 소유 행동을 해결하도록 권한을 부여하지 않습니다.

대기 중인 최종 수락 요청 없이 `missing_final_acceptance`가 있는 상태는 지원되는 2단계 상태이며 권한 우회가 아닙니다. 읽기 전용 점검이나 차단된 닫기 시도는 요청이나 해결 기록을 만들지 않습니다. 이 상태의 `request_user_action` 행동은 `allowed_operation_categories=[agent_workflow]`이며 Agent Connection이 표시된 질문으로 현재 요청을 만듭니다. 그 커밋 뒤 공개 `pending_user_action` blocker는 `allowed_operation_categories=[user_only]`인 일반 `resolve_user_action` 행동만 보여 줍니다. 사용 가능한 입력 경로는 별도로 검증된 User Channel projection이 사용자에게 제공합니다. Agent Connection은 두 번째 행동을 수행하면 안 됩니다.

## 차단 결과

조건:

- 사전 확인이 성공했습니다.
- 메서드가 닫기 준비 상태 관찰 또는 종료 경로 평가에 도달했습니다.
- 요청한 경로에 하나 이상의 닫기 차단 사유 또는 종료 차단 사유가 있습니다.

결과:

- `volicord.check_close`는 `CheckCloseResult(close_state=blocked)`,
  `volicord.close_task`는 `CloseTaskResult(close_state=blocked)`를 반환할 수
  있으며 둘 다 `blockers: CloseReadinessBlocker[]`를 노출합니다.
- `volicord.check_close`는 차단 사유를 응답 관찰 데이터로 반환하며 차단 사유 행을 만들지
  않습니다.
- `dry_run=false`인 상태 변경 `intent`에 차단 사유가 있으면 `base.effect_kind=no_effect`인 응답 전용 결과를 반환합니다. 닫기 차단 사유 행, 권한 이벤트, 재실행 행, 종료 상태 변경을 저장하지 않고 `project_state.state_version`을 증가시키지 않습니다.

메서드별 차단 사유 분기:

| 분기 | 생성 규칙 |
|---|---|
| `volicord.check_close` | 현재 닫기 준비 상태 차단 사유를 응답 관찰 데이터로 반환합니다. |
| `intent=complete` | 완료 경로가 닫기 준비 상태 평가에 도달했고 담당 문서가 정의한 닫기 요구사항이 해결되지 않았을 때 닫기 차단 사유를 만듭니다. 여기에는 활성이고 소비되지 않은 쓰기 티켓과 미해결 Unrecorded Change가 포함됩니다. 무효화되거나 취소됐거나 idle timeout에 따라 유효 상태가 무효화된 티켓 행은 그 자체로 차단하지 않습니다. |
| `intent=cancel` | 취소 권한 누락이나 비호환을 포함해 취소 전용 종료 제약에 대해서만 차단 사유를 만듭니다. 완료 전용 증거, 최종 수락, 잔여 위험 공백은 그 자체로 취소를 막지 않습니다. |
| `intent=supersede` | 대체 전용 종료 제약에 대해서만 차단 사유를 만듭니다. 완료 전용 증거, 최종 수락, 잔여 위험 공백은 그 자체로 대체를 막지 않습니다. |

닫기 준비 상태의 미기록 변경 규칙:

- 미해결 Unrecorded Change는 계속 보이고 닫기를 막습니다. 저장소 관찰 불가 진단은
  별도로 표시되며 이 차단 사유가 아닙니다.

비주장:

- `CloseReadinessBlocker`가 있다는 사실만으로는 지속 저장을 증명하지 않습니다.
- `STATE_VERSION_CONFLICT`는 절대 `CloseReadinessBlocker.code`가 아닙니다.
- `STATE_VERSION_CONFLICT`는 거절 응답 `ErrorCode`이며 메서드 로컬 차단 사유 코드나 결정 코드가 아닙니다.
- 차단 사유 범주는 사용자 판단, 승인, 증거, 아티팩트 가용성, 최종 수락, 잔여 위험 수락, 복구 상태 자체를 만들지 않습니다.
- 닫기 준비 상태는 정확성 증명, 테스트 충분성 증명, 인간 검토 대체가
  아닙니다. 두 닫기 계열 결과 base 모두 `disclosure.non_guarantees`에
  `NotCorrectnessProof`, `NotTestSufficiencyProof`,
  `NotHumanReviewReplacement`를 포함합니다.
- 미기록 변경은 행위자 귀속, 의도, 정확성, 검토 완료, 테스트 충분성을 세우지 않습니다.
- 확인되지 않은 주장, 출처가 빠진 증거, 오래된 관찰 출처, 협력적 에이전트 보고는 증거 이력으로 기록될 수 있지만, 닫기 경로가 더 강한 출처를 요구할 때 필요한 닫기 증거를 만족하지 않습니다.
- 거절, 연기, 오래됨, 대체됨, 만료됨, 유효하지 않은 근거, 에이전트가 기록함, 출처 누락, 결과 없음 취소 판단은 취소를 허용하지 않습니다.

## 거절 결과

요청이 유효한 닫기 준비 상태 결과나 종료 경로 평가에 도달하기 전에 실패하면 이 메서드는 `ToolRejectedResponse`를 반환합니다.

대표적인 거절 경우:

- 검증 실패
- 행위자 출처 또는 작업 범주 불일치
- 오래된 `expected_state_version`
- 멱등 요청 해시 충돌
- 잘못된 프로젝트 또는 읽을 수 없는 `Task` 식별
- Core 사용 불가
- 지원되지 않는 호출 맥락

거절 응답:

- 닫기 계열 결과나 `blockers`를 반환하지 않습니다.
- 닫기 효과를 만들지 않습니다.
- 쓰기 티켓, 최종 수락, 잔여 위험 수락, 증거, 아티팩트 상태를 만들지 않습니다.

공개 오류 의미, 우선순위, 응답 분기 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`volicord.check_close`와 `dry_run=true`는 `base.effect_kind=read_only`인 읽기
전용 `CheckCloseResult` 분기에 남습니다.

상태 변경 `intent`와 `dry_run=true` 조합은 유효한 사전 확인 뒤 `ToolDryRunResponse`를 사용합니다. 미리보기 차단 사유는 `PlannedBlocker` 데이터이며 저장된 `CloseReadinessBlocker` 객체가 아닙니다.

`dry_run=true` 요청이 미리보기 전에 실패하면 `DryRunSummary.would_errors[]`나 `PlannedBlocker`가 아니라 `ToolRejectedResponse`를 반환합니다.

분기 형태는 [API 코어 스키마](schema-core.md)가 담당합니다. 응답 분기 처리 경로는 [API 오류 처리 경로](error-routing.md)가 담당합니다. 닫기 차단 사유와 API 응답 분기 사이의 경계는 [API 차단 사유 처리 경로](blocker-routing.md)가 담당합니다.

## 저장 효과

`volicord.check_close`는 Core 권한 상태를 저장소에서 변경하지 않습니다. 차단
사유를 반환하거나 `dry_run=true`를 사용해도 마찬가지입니다. 재실행 행을
만들거나, 이벤트를 추가하거나, 닫기 차단 사유 행을 지속 저장하거나,
`close_state`를 변경하거나, 아티팩트 또는 증거를 건드리거나,
`project_state.state_version`을 증가시키지 않습니다.

커밋되는 `dry_run=false` 상태 변경 `intent`는 성공한 종료 결과만 지속 저장합니다. 차단 사유가 있는 상태 변경 `intent`는 응답 전용 `base.effect_kind=no_effect` 결과를 반환하고 종료 상태를 변경하지 않습니다. 성공한 종료 닫기는 닫기 전 준비 상태에 사용한 현재 닫기 근거와 별개인 종료 닫기 요약을 지속 저장할 수 있습니다. 성공한 `intent=complete`는 현재 닫기 근거의 잔여 위험 중 보이지만 잔여 위험 수락이 필요하지 않은 항목에 대해 `kind=known_limit` 프로젝트 연속성 기록도 지속 저장할 수 있습니다. 정확한 저장 효과, 재실행 행, 이벤트, 상태 버전 증가, 프로젝트 연속성 지속 저장은 [저장 효과](../storage-effects.md)와 [저장소 버전 관리](../storage-versioning.md)가 담당합니다.

반환되는 모든 authority receipt는 `completion_claim_allowed`를 도출합니다. 호출자는
`false`를 권한 경계로 취급해야 하며 요약 문구, 성공한 전송, 부분 결과만으로 완료
주장을 출력하면 안 됩니다.

거절 응답과 유효한 상태 변경 `intent`의 `ToolDryRunResponse` 미리보기에는 저장 효과가 없습니다.

## 예시

아래 예시는 의도적으로 작게 유지합니다. 메서드 분기만 보여 주고, 중첩 스키마, 저장소, 표시 세부사항은 각 담당 문서에 남깁니다.

### 최소 유효 요청

```yaml contract=api.method.check_close.request shape=complete_request
method: volicord.check_close
params:
  envelope:
    project_id: proj_close_001
    task_id: task_close_001
    request_id: req_close_check_local_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: ko-KR
  task_id: task_close_001
```

### 대표 차단 확인 응답

`state_version: 72`의 `task_close_001`에 대해, 이 메서드 예시의 응답이 최종 수락 차단 사유 하나를 보고하는 읽기 전용 `CheckCloseResult`:

```schema
base:
  response_kind: result
  effect_kind: read_only
  dry_run: false
  state_version: 72
  events: []
close_state: blocked
current_close_basis: null
risk_acceptance_coverage: []
continuity_summary: []
state:
  project_id: proj_close_001
  state_version: 72
  task_ref:
    record_kind: task
    record_id: task_close_001
    project_id: proj_close_001
    task_id: task_close_001
    produced_at_state_version: 72
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "온보딩 체크리스트 설정을 완료합니다."
  scope_summary: "온보딩 체크리스트 완료."
  non_goals:
    - "계정 생성 방식 변경."
  acceptance_criteria:
    - acceptance_criterion_id: criterion_onboarding_review_001
      statement: "온보딩 체크리스트를 사용자가 검토할 수 있습니다."
      evidence_requirement: not_required
  autonomy_boundary: "온보딩 체크리스트 완료만 다룹니다."
  active_change_unit_ref: null
  baseline_ref: baseline_close_001
  # 현재의 완전한 WorkflowProjection은 이 축약 예시에서 생략했습니다.
  pending_user_action_summaries: []
  blocker_refs: []
  write_ticket_summary: null
  evidence_summary: null
  evidence_gate:
    state: not_required
  close_state: blocked
  close_blockers:
    - category: final_acceptance
      code: missing_final_acceptance
      message: "이 Task를 닫으려면 최종 수락이 필요합니다."
      related_refs: []
      next_actions:
          action_kind: request_user_action
          owner_method: volicord.request_user_action
          allowed_operation_categories: [agent_workflow]
          label: "Agent Connection이 사용자에게 현재 최종 수락 요청을 만들어야 합니다."
          blocking_question: "사용자가 현재 Task 결과와 닫기 근거를 완료된 것으로 수락합니까?"
          expected_state_version: 72
          required_refs:
            - record_kind: task
              record_id: task_close_001
              project_id: proj_close_001
              task_id: task_close_001
              produced_at_state_version: 72
  guarantee_display: null
blockers:
  - category: final_acceptance
    code: missing_final_acceptance
    message: "이 Task를 닫으려면 최종 수락이 필요합니다."
    related_refs: []
    next_actions:
        action_kind: request_user_action
        owner_method: volicord.request_user_action
        allowed_operation_categories: [agent_workflow]
        label: "Agent Connection이 사용자에게 현재 최종 수락 요청을 만들어야 합니다."
        blocking_question: "사용자가 현재 Task 결과와 닫기 근거를 완료된 것으로 수락합니까?"
        expected_state_version: 72
        required_refs:
          - record_kind: task
            record_id: task_close_001
            project_id: proj_close_001
            task_id: task_close_001
            produced_at_state_version: 72
evidence_summary: null
evidence_gate:
  state: not_required
artifact_refs: []
```

## 담당 문서 링크

- 요청 래퍼, 공통 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `CheckCloseResult.blockers`, `CloseTaskResult.blockers`, `CurrentCloseBasis`, `RiskAcceptanceCoverage`, `CloseReadinessBlocker`, `ProjectContinuitySummary`, `EvidenceSummary`, `EvidenceGateSummary`, `StateSummary`, `NextActionSummary` 형태: [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes).
- `ArtifactRef` 형태: [API 아티팩트 스키마](schema-artifacts.md#artifactref).
- `intent` 값: [API 값 집합의 메서드 내부 값](schema-value-sets.md#method-local-values).
- 닫기 상태, 생명주기, 닫기 이유 값: [API 값 집합의 Task 생명주기 값](schema-value-sets.md#task-lifecycle-values).
- 차단 사유 범주 값(`CloseReadinessBlocker.category`): [API 값 집합의 상태와 차단 사유 값](schema-value-sets.md#state-and-blocker-values).
- 닫기 준비 상태 의미와 정직한 닫기: [Core 모델의 닫기 준비 상태](../core-model.md#close_task).
- 공개 `ErrorCode` 의미: [API 오류 코드](error-codes.md).
- 오류 우선순위와 오래된 상태 충돌 선택: [API 오류 우선순위](error-precedence.md).
- 거절, 차단, `dry_run` 응답 분기 처리 경로: [API 오류 처리 경로](error-routing.md).
- 닫기 차단 사유와 API 응답 분기 사이의 처리 경로: [API 차단 사유 처리 경로](blocker-routing.md).
- 지속 저장 효과와 상태 버전 동작: [저장 효과](../storage-effects.md), [저장소 버전 관리](../storage-versioning.md).
- 표시 라벨과 렌더링 문구: [템플릿 본문](../template-bodies.md).
