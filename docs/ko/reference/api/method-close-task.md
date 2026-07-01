<a id="volicordclose_task"></a>

# `volicord.close_task` 참조

## 담당하는 것

이 문서는 기준 범위의 `volicord.close_task` 메서드 동작을 담당합니다.

- 메서드별 요청 조건, `intent` 처리, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- `volicord.close_task` 요청에 적용되는 메서드별 평가 순서
- `CloseTaskResult.blockers`를 만드는 메서드별 차단 사유 분기
- 메서드별 `CloseReadinessBlocker.code` 생성 동작
- Task 닫기 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- `ToolEnvelope`, `ToolResultBase`, `ToolRejectedResponse`, `ToolDryRunResponse`의 공통 스키마 본문
- 상태, 아티팩트, 판단, 값 집합, 오류의 중첩 스키마 정의
- Core의 닫기 준비 상태 권한 개념
- `CloseReadinessBlocker` 형태나 `CloseReadinessBlocker.category` 값
- 공개 오류 코드 의미, 오류 우선순위, 응답 분기 처리 경로
- 저장소 배치, 저장 효과 세부사항, 보안 보장, 렌더링 문구

## 목적

`volicord.close_task`는 선택된 `Task`의 닫기 준비 상태를 평가하고, 선택한 닫기 의도가 허용할 때 요청된 종료 경로를 수행합니다.

이 메서드는 다음 결과를 낼 수 있습니다.

- 읽기 전용 닫기 준비 상태 관찰 반환
- `intent=complete`, `intent=cancel`, `intent=supersede` 커밋
- `CloseTaskResult.blockers`를 담은 `CloseTaskResult(close_state=blocked)` 반환
- 닫기 준비 상태 평가 전 요청 거절
- 유효한 상태 변경 미리보기에 대한 공통 `dry_run` 미리보기 반환

닫기는 보고서가 아니라 Core 상태 전이입니다. 이 메서드는 `intent=complete`에서 현재 닫기 근거를 평가합니다. 대화, 상태 텍스트, 종료 닫기 요약, 최종 수락만, 잔여 위험 수락만, 증거만, `Write Check`, 렌더링된 보기에서 닫기를 추론하지 않습니다.

## 담당 경계

메서드 담당 블록:

- `volicord.close_task`의 요청 검증과 `intent` 필드 조합
- 이 메서드가 확인, 상태 변경, 차단, 거절, `dry_run` 분기에 도달하는 순서
- 유효한 상태 변경 분기가 종료 결과나 커밋된 차단 결과를 커밋할 수 있는지 여부
- `CloseTaskResult.blockers`에서 생성할 수 있는 메서드별 차단 사유 코드

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
- 요청한 `intent`, `close_reason`, `superseding_task_id` 조합이 유효해야 합니다.
- 확인된 호출 맥락, 작업 범주, 호환 행위자 출처, 종료 경로 선행조건이 요청한 경로를 허용해야 합니다.

상태 변경 조건:

- `dry_run=false`인 상태 변경 `intent`에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- 오래된 `expected_state_version`, 오래된 닫기 관련 `WriteCheck.basis_state_version`, 멱등 요청 해시 충돌은 닫기 준비 상태 평가 전에 거절됩니다.
- 닫기 관련 `WriteCheck.basis_state_version`은 사전 확인 시 현재 `project_state.state_version`과 같지 않을 때 오래된 값입니다.
- 닫기 관련 `Write Check` 최신성 확인은 쓰기 호환성 확인일 뿐입니다. 최종 수락, 잔여 위험 수락, 사용자 소유 판단, 민감 동작 승인, 포괄적 승인을 기록하지 않습니다.

닫기 조건:

- `intent=complete`는 사전 확인이 성공하고, 현재 `CurrentCloseBasis`에 대한 닫기 준비 상태 평가가 유효하며, 현재 닫기 근거 참조가 그 아티팩트 및 실행 기록 호환성 규칙을 만족하고, 닫기 차단 사유가 남아 있지 않을 때만 닫을 수 있습니다.
- 확인된 연결이 `guarded` 또는 `managed` 모드이면 닫기 준비 상태는 guard 상태, prompt capture 사용 가능 사실, 해결되지 않은 미기록 Product Repository 변경, guard가 감지한 쓰기 준비 상태 문제도 확인합니다. 이 확인은 guarded 또는 managed 동작에서만 닫기 차단 사유가 됩니다. `mcp_only`는 담당 문서가 정의한 설정이 guarded 또는 managed 동작을 선택하지 않는 한 협력형으로 남습니다.
- 필요한 닫기 증거는 현재 닫기 근거에 맞고 주장과 일치하는 증거 관찰 출처로 뒷받침되어야 합니다. 더 강한 출처가 필요한 닫기 요구사항에는 확인되지 않은 주장, 출처 없는 증거, 오래된 출처, 협력적 에이전트 보고만으로 된 증거가 충분하지 않습니다.
- `intent=cancel`은 `machine_action=accept`, `resolution_outcome=accepted`, `resolved_by_actor_source=local_user`, 호환 User Channel 출처, `Task`, 현재 범위 리비전, 현재 적용 Change Unit에 묶인 근거를 가진 현재 수락된 취소 판단을 요구합니다. 완료 전용 증거, 최종 수락, 잔여 위험 수락은 필요하지 않습니다.
- `intent=supersede`는 요청한 종료 경로를 평가합니다. 증거 충분성, 최종 수락, 잔여 위험 수락이 아닙니다.

성공한 종료 닫기가 만드는 종료 닫기 요약은 현재 닫기 전 근거가 아니며 `CurrentCloseBasis`의 대체물로 쓰지 않습니다.

## 닫기 의도

지원되는 `intent` 값은 [API 값 집합의 메서드 내부 값](schema-value-sets.md#method-local-values)이 담당합니다. 지원되는 `close_reason`과 `close_state` 값은 [API 값 집합의 Task 생명주기 값](schema-value-sets.md#task-lifecycle-values)이 담당합니다.

| `intent` | `close_reason` | `superseding_task_id` | 메서드 규칙 |
|---|---|---|---|
| `check` | `null` | `null` | 읽기 전용 닫기 준비 상태 관찰입니다. |
| `complete` | `completed_self_checked` 또는 `completed_with_risk_accepted` | `null` | 완료 경로이며 닫기 준비 상태 평가를 실행합니다. |
| `cancel` | `cancelled` | `null` | 취소 경로이며 호환되는 `accepted` 취소 권한을 요구하고 취소 전용 종료 제약을 평가합니다. |
| `supersede` | `superseded` | `null`이 아닌 같은 프로젝트의 대체 `Task` 참조 | 대체 경로이며 대체 전용 종료 제약을 평가합니다. |

## 필수 입력

모든 호출에는 아래 입력이 필요합니다.

- `project_id`, `request_id`, `dry_run`을 포함한 메서드 필수 요청 래퍼 필드를 가진 `ToolEnvelope`
- 요청 래퍼가 선택한 요청 맥락과 메서드 params에서 일치하는 `task_id`
- `intent`
- `close_reason`
- `superseding_task_id`
- `user_note`

추가 요구사항:

| 경우 | 필수 입력 규칙 |
|---|---|
| `intent=check` | `idempotency_key`와 `expected_state_version`은 `null`일 수 있습니다. `close_reason`과 `superseding_task_id`는 `null`이어야 합니다. |
| `intent=complete`, `intent=cancel`, `intent=supersede`와 `dry_run=false` | `idempotency_key`와 `expected_state_version`은 `null`이 아니어야 하며 현재 값이어야 합니다. |
| `intent=supersede` | `superseding_task_id`는 호환되는 같은 프로젝트의 대체 `Task`를 가리켜야 합니다. |

## 요청 스키마

이 메서드는 아래 최상위 `params` 요청 형태를 담당합니다. `envelope`는 [API 코어 스키마](schema-core.md#tool-envelope)의 공통 `ToolEnvelope`이며, 이 블록은 `ToolEnvelope` 필드를 다시 정의하지 않습니다.

이 메서드 소유 요청 블록에 표시된 모든 필드는 필드 참고가 명시적으로 선택 필드라고 표시하지 않는 한 `params`의 필수 멤버입니다. `T | null`은 멤버가 반드시 있어야 하며 JSON `null`을 담을 수 있다는 뜻입니다.

```yaml
CloseTaskRequest:
  envelope: ToolEnvelope
  task_id: string
  intent: string
  close_reason: string | null
  superseding_task_id: string | null
  user_note: string | null
```

중첩 형태 담당 문서:
- `intent` 값은 [API 값 집합의 메서드 내부 값](schema-value-sets.md#method-local-values)이 담당합니다.
- `close_reason` 값은 [API 값 집합의 Task 생명주기 값](schema-value-sets.md#task-lifecycle-values)이 담당합니다.

## 접근 요구사항

| 요청 종류 | 메서드 접근 규칙 |
|---|---|
| `intent=check` | 보호된 닫기 준비 상태 세부정보에는 `operation_category=read`인 확인된 호출 맥락이 필요합니다. |
| 상태 변경 `intent` | `operation_category=agent_workflow`인 확인된 호출 맥락, 호환되는 `Task` 상태, 닫기 관련 담당 기록이 필요합니다. |

이 메서드를 호출할 접근 권한은 사용자 소유 판단, 최종 수락, 잔여 위험 수락, 민감 동작 승인, `Write Check`과 별개입니다.

## 메서드 흐름

구현은 `volicord.close_task`를 아래 순서로 평가합니다.

1. 요청 래퍼, 메서드 필드, `intent` 필드 조합, 같은 프로젝트의 `Task` 식별자를 검증합니다. 형태 오류, 잘못된 프로젝트 식별자, 읽을 수 없는 `Task` 식별자는 `ToolRejectedResponse`를 반환합니다.
2. 호출 맥락, 작업 범주, 행위자 출처, 요청한 종료 경로의 선행조건을 확인합니다.
3. `dry_run=false`인 상태 변경 `intent`에서는 `idempotency_key`, 현재 `expected_state_version`, 멱등 요청 해시, 닫기 관련 `WriteCheck.basis_state_version`을 확인합니다. 오래되었거나 충돌하는 값은 `ToolRejectedResponse`를 반환합니다.
4. `intent=check`는 선택된 guard 상태 사실을 포함해 [`volicord.status`](method-status.md)의 `include.close=true`와 같은 계산으로 현재 닫기 준비 상태를 계산하고 읽기 전용 `CloseTaskResult`를 반환합니다.
5. 상태 변경 `intent`와 `dry_run=true` 조합은 유효한 사전 확인 뒤 공통 미리보기 분기를 반환합니다.
6. `intent=complete`는 현재 `CurrentCloseBasis`에 대한 닫기 준비 상태 평가를 실행합니다. 차단 사유가 남아 있으면 차단 분기를 반환하고, 없으면 `close_state=closed`, 종료 닫기 결과, 잔여 위험 수락이 필요하지 않은 닫기 근거의 알려진 한계에 대해 메서드가 선택한 프로젝트 연속성 기록을 커밋합니다.
7. `intent=cancel`은 `machine_action=accept`, `resolution_outcome=accepted`, `resolved_by_actor_source=local_user`, 호환 User Channel 출처를 가지며 현재 `Task`, 범위 리비전, Change Unit과 호환되는 현재 수락된 `judgment_kind=cancellation`을 요구합니다. 취소 권한이 없거나 호환되지 않으면 차단 분기를 반환합니다.
8. `intent=cancel` 또는 `intent=supersede`는 요청한 종료 경로만 평가합니다. 종료 경로 차단 사유가 남아 있으면 차단 분기를 반환하고, 없으면 `close_state=cancelled` 또는 `close_state=superseded`를 커밋합니다.

## 상태 버전 동작

| 경우 | 상태 버전 효과 |
|---|---|
| `intent=check` | `dry_run=true`여도 항상 읽기 전용이며 상태를 증가시키지 않습니다. |
| 성공한 종료 상태 변경 | `project_state.state_version`을 정확히 한 번 증가시킵니다. |
| 상태 변경 `intent`의 커밋된 차단 결과 | 이 메서드와 저장 효과 담당 문서가 그 커밋된 차단 결과를 허용할 때 `project_state.state_version`을 정확히 한 번 증가시킵니다. |
| 사전 확인 거절 또는 유효한 `dry_run` 미리보기 | 아무것도 증가시키지 않습니다. |

사전 확인 거절에는 오래된 `expected_state_version`, 오래된 닫기 관련 `WriteCheck.basis_state_version`, 멱등 요청 해시 충돌이 포함됩니다. 이런 충돌은 오류 담당 문서로 처리되며 닫기 차단 사유가 아닙니다.

## 성공 결과

여기서 성공은 차단되거나 거절되지 않은 결과 분기를 뜻합니다.

`base.response_kind=result`인 `CloseTaskResult`를 반환합니다.

| 경우 | 효과 | `close_state` |
|---|---|---|
| `intent=check`이고 현재 차단 사유가 없음 | `base.effect_kind=read_only` | `ready` |
| 성공한 `intent=complete` | `base.effect_kind=core_committed` | `closed` |
| 성공한 `intent=cancel` | `base.effect_kind=core_committed` | `cancelled` |
| 성공한 `intent=supersede` | `base.effect_kind=core_committed` | `superseded` |

## 메서드 결과 필드

`CloseTaskResult`는 유효한 닫기 확인 또는 종료 닫기 시도를 위한 메서드별 결과 분기입니다. 이 결과는 `base: ToolResultBase`와 아래 메서드 담당 최상위 필드를 담습니다.

| 필드 | 결과 필드 의미 |
|---|---|
| `base` | 공통 결과 메타데이터입니다. `events`를 포함한 `ToolResultBase` 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당합니다. 유효한 `CloseTaskResult` 분기는 `base.response_kind=result`를 사용합니다. 이 메서드는 `intent=check`에는 `base.effect_kind=read_only`를, 커밋된 종료 결과 또는 담당 문서가 허용한 커밋된 차단 결과에는 `base.effect_kind=core_committed`를 선택합니다. |
| `close_state` | 요청한 경로에 대한 메서드 결과 닫기 상태입니다. 지원 값은 [API 값 집합](schema-value-sets.md#task-lifecycle-values)이 담당합니다. `close_state=blocked`는 유효한 닫기 또는 종료 경로 평가 뒤의 메서드 결과이지 `ToolRejectedResponse`가 아닙니다. |
| `state` | 확인, 종료 상태 변경, 또는 담당 문서가 허용한 차단 결과 뒤 선택된 Task의 `StateSummary`입니다. `close_blockers`를 포함한 중첩 상태 필드는 [API 상태 스키마](schema-state.md)가 담당합니다. |
| `current_close_basis` | 결과에 선택된 닫기 준비 상태에 사용한 `CurrentCloseBasis | null`입니다. `null`은 이 결과에 사용할 현재 닫기 근거가 없다는 뜻입니다. 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다. |
| `risk_acceptance_coverage` | 닫기 준비 상태 결과에서 현재 잔여 위험 수락 범위를 나타내는 `RiskAcceptanceCoverage[]`입니다. 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다. |
| `continuity_summary` | 이 닫기 결과로 관련성이 생긴 프로젝트 연속성 기록의 `ProjectContinuitySummary[]`입니다. 성공한 `intent=complete`에서는 잔여 위험 수락이 필요하지 않은 닫기 근거의 알려진 한계를 Core가 이어 가는 연속성 기록이 여기에 포함됩니다. 빈 배열은 이 결과에 대해 계산이 실행됐고 이어 갈 기록이 없었다는 뜻입니다. 형태는 [API 상태 스키마](schema-state.md#project-continuity-shapes)가 담당합니다. |
| `blockers` | 요청한 경로에 닫기 차단 사유 또는 종료 차단 사유가 있을 때 반환되는 `CloseReadinessBlocker[]`입니다. 형태와 중첩은 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당하며, `category` 값은 [API 값 집합](schema-value-sets.md#state-and-blocker-values)이 담당합니다. |
| `guard_health` | 닫기 준비 상태 결과에 선택된 guard 상태 사실의 `GuardHealthSummary | null`입니다. 형태는 [API 상태 스키마](schema-state.md#guard-health-summary)가 담당합니다. |
| `evidence_summary` | 결과에 선택된 닫기 근거의 `EvidenceSummary | null`입니다. 결과에 증거 요약이 선택되지 않으면 `null`입니다. 형태는 [API 상태 스키마](schema-state.md#evidence-and-run-snapshot-shapes)가 담당합니다. |
| `artifact_refs` | 결과에 선택된 닫기 관련 아티팩트의 `ArtifactRef[]`입니다. `ArtifactRef` 형태는 [API 아티팩트 스키마](schema-artifacts.md#artifactref)가 담당합니다. |

`CloseTaskResult`에는 최상위 `next_actions` 필드가 없습니다. 닫기 차단 사유의 다음 동작은 `CloseReadinessBlocker.next_actions` 안에 나타나며 [API 상태 스키마](schema-state.md#current-position-display-shapes)의 기준 `NextActionSummary` 형태를 사용합니다.

이 메서드는 자신이 생성하는 메서드 범위의 `CloseReadinessBlocker.code` 값을 담당합니다. 이런 코드는 공개 `ErrorCode` 값이 아니며 전역 값 집합 항목도 아닙니다.

메서드 로컬 `CloseReadinessBlocker.code` 목록:

아래 생성 의미는 이 메서드가 닫기 준비 상태 관찰 또는 종료 경로 평가에 도달한 뒤에만 적용됩니다. 사전 확인 실패는 여전히 오류 담당 문서에 따라 `ToolRejectedResponse`를 반환합니다.

| 코드 | 범주 | 로컬 생성 의미 |
|---|---|---|
| `task_not_closeable` | `task` | 선택된 `Task` 생명주기나 종료 경로 상태가 요청한 닫기 의도를 받을 수 없습니다. |
| `missing_active_change_unit` | `scope` | 닫기 경로에 현재 적용 Change Unit이 필요하지만 사용할 수 없습니다. |
| `pending_user_judgment` | `pending_user_judgment` | 필요한 사용자 소유 판단이 아직 대기 중이거나 해결되지 않았습니다. |
| `missing_sensitive_approval` | `sensitive_approval` | 필요한 별도 민감 동작 승인이 없습니다. |
| `missing_cancellation_authority` | `user_judgment` | `intent=cancel`에 현재 `Task`, 범위 리비전, Change Unit에 묶이고 `resolved_by_actor_source=local_user`, 호환 User Channel 출처를 가진 현재 수락된 사용자 취소 판단이 없습니다. |
| `write_check_stale` | `write_compatibility` | 닫기 관련 `Write Check`이 `STATE_VERSION_CONFLICT`로 처리되지 않는 최신성 사유로 사용할 수 없습니다. |
| `baseline_stale` | `baseline` | 닫기 관련 기준선 근거가 차단 사유 생성 경로에서 오래되었습니다. |
| `guard_not_installed` | `connection_capability` | guarded 또는 managed 닫기 경로에 확인된 연결에 대해 사용할 수 있는 guard 설치가 기록되어 있지 않습니다. |
| `guard_reload_required` | `connection_capability` | guard 파일은 설치되어 있지만, Volicord가 설정된 hook을 관찰하기 전에 호스트를 restart 또는 reload해야 합니다. |
| `guard_not_observed` | `connection_capability` | guarded 또는 managed 닫기 경로에 guard 파일은 설정되어 있지만 일치하는 호스트 guard hook 관찰이 기록되어 있지 않습니다. |
| `guard_stale` | `connection_capability` | guarded 또는 managed 닫기 경로에 기록 상태가 `stale`인 guard 설치가 있습니다. |
| `guard_broken` | `connection_capability` | guarded 또는 managed 닫기 경로에 기록 상태가 `broken`인 guard 설치가 있습니다. |
| `guard_degraded` | `connection_capability` | guarded 또는 managed 닫기 경로에 기록 상태가 `degraded`인 guard 설치가 있고 현재 guard policy가 degraded 상태에서 닫기를 차단합니다. |
| `guard_connection_unhealthy` | `connection_capability` | guarded 또는 managed 닫기 경로에 건강하지 않은 Agent Connection 상태 사실이 있습니다. |
| `unresolved_unrecorded_changes` | `connection_capability` | guard 기록에 닫기 전에 기록하거나 조정해야 하는 해결되지 않은 미기록 Product Repository 변경이 있습니다. |
| `guard_write_readiness_missing_or_stale` | `write_compatibility` | guard 이벤트가 닫기 경로에 누락되었거나 오래된 쓰기 준비 상태를 감지했습니다. |
| `evidence_claim_unsupported` | `evidence_claim` | 필요한 닫기 주장이 지원되는 증거 범위를 갖지 못했습니다. |
| `evidence_claim_missing` | `evidence_claim` | 필요한 닫기 주장에 대한 현재 증거 범위 기록이 없습니다. |
| `evidence_provenance_insufficient` | `evidence_provenance` | 필요한 닫기 증거는 있지만 충분한 현재 출처와 보장 수준 출처가 없습니다. |
| `evidence_provenance_stale` | `evidence_provenance` | 증거 관찰 출처가 있지만 현재 Task 범위, Change Unit, 출처 실행 기록, 닫기 근거 증거 요약에 대해 오래되었습니다. |
| `evidence_agent_report_only` | `evidence_provenance` | 더 강한 출처가 필요한데 필요한 닫기 증거가 협력적 에이전트 보고만으로 뒷받침됩니다. |
| `artifact_unavailable` | `artifact_availability` | 닫기 관련 아티팩트가 없거나, 사용할 수 없거나, 사용에 부적합하거나, 무결성에 실패했습니다. |
| `missing_final_acceptance` | `final_acceptance` | 현재 닫기 근거에 필요한 최종 수락이 없습니다. |
| `stale_final_acceptance` | `final_acceptance` | 최종 수락은 있지만 현재 `Task`, Change Unit, `scope_revision`, `close_basis_revision`, 기준선, 결과 참조와 호환되지 않거나 오래되었습니다. |
| `residual_risk_not_visible` | `residual_risk_visibility` | 닫기 관련 잔여 위험이 보이지 않게 남아 있습니다. |
| `missing_residual_risk_acceptance` | `residual_risk_acceptance` | 현재 잔여 위험 요구사항에 필요한 잔여 위험 수락이 없습니다. |
| `stale_residual_risk_acceptance` | `residual_risk_acceptance` | 잔여 위험 수락은 있지만 현재 `close_basis_revision`과 정확한 잔여 위험 `risk_id` 값에 일치하지 않습니다. |
| `recovery_required` | `recovery` | 요청한 닫기 경로를 진행하기 전에 복구 작업이 남아 있습니다. |

이 코드는 메서드 로컬 `CloseReadinessBlocker.code` 값입니다. 공개 `ErrorCode` 값, `WriteDecisionReason.code` 값, 전역 값 집합 항목이 아닙니다.

`pending_user_judgment`의 경우 차단 사유의 다음 행동은 사용할 수 있는 User Channel 답변 경로를 가리킬 수 있습니다. 여기에는 사용할 수 있을 때 MCP elicitation, prompt-capture 채팅 명령, 로컬 사용자 명령이 포함됩니다. 이 차단 사유는 Agent Connection이 사용자 소유 판단에 답하도록 권한을 부여하지 않습니다.

## 차단 결과

조건:

- 사전 확인이 성공했습니다.
- 메서드가 읽기 전용 닫기 준비 상태 관찰 또는 종료 경로 평가에 도달했습니다.
- 요청한 경로에 하나 이상의 닫기 차단 사유 또는 종료 차단 사유가 있습니다.

결과:

- `blockers: CloseReadinessBlocker[]`를 담은 `CloseTaskResult(close_state=blocked)`를 반환할 수 있습니다.
- `intent=check`는 차단 사유를 읽기 전용 관찰 데이터로 반환하며 차단 사유 행을 만들지 않습니다.
- `dry_run=false`인 상태 변경 `intent`는 이 메서드와 [저장 효과](../storage-effects.md)가 그 효과를 허용할 때만 차단 결과를 커밋할 수 있습니다.

메서드별 차단 사유 분기:

| 분기 | 생성 규칙 |
|---|---|
| `intent=check` | 현재 닫기 준비 상태 차단 사유를 읽기 전용 관찰 데이터로 반환합니다. |
| `intent=complete` | 완료 경로가 닫기 준비 상태 평가에 도달했고 담당 문서가 정의한 닫기 요구사항이 해결되지 않았을 때 닫기 차단 사유를 만듭니다. `guarded` 또는 `managed` 모드에서는 guard 상태, 해결되지 않은 미기록 변경, guard가 감지한 쓰기 준비 상태 차단 사유도 포함합니다. |
| `intent=cancel` | 취소 권한 누락이나 비호환을 포함해 취소 전용 종료 제약에 대해서만 차단 사유를 만듭니다. 완료 전용 증거, 최종 수락, 잔여 위험 공백은 그 자체로 취소를 막지 않습니다. |
| `intent=supersede` | 대체 전용 종료 제약에 대해서만 차단 사유를 만듭니다. 완료 전용 증거, 최종 수락, 잔여 위험 공백은 그 자체로 대체를 막지 않습니다. |

비주장:

- `CloseReadinessBlocker`가 있다는 사실만으로는 지속 저장을 증명하지 않습니다.
- `STATE_VERSION_CONFLICT`는 절대 `CloseReadinessBlocker.code`가 아닙니다.
- `STATE_VERSION_CONFLICT`는 거절 응답 `ErrorCode`이며 메서드 로컬 차단 사유 코드나 결정 코드가 아닙니다.
- 차단 사유 범주는 사용자 판단, 승인, 증거, 아티팩트 가용성, 최종 수락, 잔여 위험 수락, 복구 상태 자체를 만들지 않습니다.
- 확인되지 않은 주장, 출처가 빠진 증거, 오래된 관찰 출처, 협력적 에이전트 보고는 증거 이력으로 기록될 수 있지만, 닫기 경로가 더 강한 출처를 요구할 때 필요한 닫기 증거를 만족하지 않습니다.
- 거절, 연기, 오래됨, 대체됨, 만료됨, 유효하지 않은 근거, 에이전트가 기록함, 출처 누락, 결과 없음 취소 판단은 취소를 허용하지 않습니다.

## 거절 결과

요청이 유효한 닫기 준비 상태 결과나 종료 경로 평가에 도달하기 전에 실패하면 이 메서드는 `ToolRejectedResponse`를 반환합니다.

대표적인 거절 경우:

- 검증 실패
- 행위자 출처 또는 작업 범주 불일치
- 오래된 `expected_state_version`
- 오래된 닫기 관련 `WriteCheck.basis_state_version`
- 멱등 요청 해시 충돌
- 잘못된 프로젝트 또는 읽을 수 없는 `Task` 식별
- Core 사용 불가
- 지원되지 않는 호출 맥락

거절 응답:

- `CloseTaskResult.blockers`를 반환하지 않습니다.
- 닫기 효과를 만들지 않습니다.
- `Write Check`, 최종 수락, 잔여 위험 수락, 증거, 아티팩트 상태를 만들지 않습니다.

공개 오류 의미, 우선순위, 응답 분기 처리 경로는 아래 오류 담당 문서가 담당합니다.

## `dry_run` 동작

`intent=check`와 `dry_run=true`는 `base.effect_kind=read_only`인 읽기 전용 `CloseTaskResult` 분기에 남습니다.

상태 변경 `intent`와 `dry_run=true` 조합은 유효한 사전 확인 뒤 `ToolDryRunResponse`를 사용합니다. 미리보기 차단 사유는 `PlannedBlocker` 데이터이며 저장된 `CloseReadinessBlocker` 객체가 아닙니다.

`dry_run=true` 요청이 미리보기 전에 실패하면 `DryRunSummary.would_errors[]`나 `PlannedBlocker`가 아니라 `ToolRejectedResponse`를 반환합니다.

분기 형태는 [API 코어 스키마](schema-core.md)가 담당합니다. 응답 분기 처리 경로는 [API 오류 처리 경로](error-routing.md)가 담당합니다. 닫기 차단 사유와 API 응답 분기 사이의 경계는 [API 차단 사유 처리 경로](blocker-routing.md)가 담당합니다.

## 저장 효과

`intent=check`에는 저장 효과가 없습니다. 차단 사유를 반환하거나 `dry_run=true`를 사용해도 마찬가지입니다.

커밋되는 `dry_run=false` 상태 변경 `intent`는 메서드 결과에 따라 종료 결과나 차단 결과를 지속 저장할 수 있습니다. 성공한 종료 닫기는 닫기 전 준비 상태에 사용한 현재 닫기 근거와 별개인 종료 닫기 요약을 지속 저장할 수 있습니다. 성공한 `intent=complete`는 현재 닫기 근거의 잔여 위험 중 보이지만 잔여 위험 수락이 필요하지 않은 항목에 대해 `kind=known_limit` 프로젝트 연속성 기록도 지속 저장할 수 있습니다. 정확한 저장 효과, 재실행 행, 이벤트, 상태 버전 증가, 프로젝트 연속성 지속 저장, 차단 사유 지속 저장 규칙은 [저장 효과](../storage-effects.md)와 [저장소 버전 관리](../storage-versioning.md)가 담당합니다.

거절 응답과 유효한 `dry_run` 미리보기에는 저장 효과가 없습니다.

## 예시

아래 예시는 의도적으로 작게 유지합니다. 메서드 분기만 보여 주고, 중첩 스키마, 저장소, 표시 세부사항은 각 담당 문서에 남깁니다.

### 최소 유효 요청

```yaml
method: volicord.close_task
params:
  envelope:
    project_id: proj_close_001
    task_id: task_close_001
    request_id: req_close_check_local_001
    idempotency_key: null
    expected_state_version: null
    dry_run: false
    locale: en-US
  task_id: task_close_001
  intent: check
  close_reason: null
  superseding_task_id: null
  user_note: null
```

### 대표 차단 확인 응답

`state_version: 72`의 `task_close_001`에 대해, 이 메서드 예시의 응답이 최종 수락 차단 사유 하나를 보고하는 읽기 전용 `CloseTaskResult`:

```yaml
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
    state_version: 72
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "Complete onboarding checklist setup."
  scope_summary: "Onboarding checklist completion."
  non_goals:
    - "Changing account creation."
  acceptance_criteria:
    - "The onboarding checklist is ready for user review."
  autonomy_boundary: "Stay within onboarding checklist completion."
  active_change_unit_ref: null
  baseline_ref: baseline_close_001
  shaping_readiness: null
  pending_user_judgment_refs: []
  blocker_refs: []
  write_check_summary: null
  evidence_summary: null
  close_state: blocked
  close_blockers:
    - category: final_acceptance
      code: missing_final_acceptance
      message: "Final acceptance is still required before this Task can close."
      can_resolve_in_chat: false
      terminal_action_required: false
      related_refs: []
      next_actions:
        - action_kind: request_user_judgment
          owner_method: volicord.request_user_judgment
          label: "Request final acceptance from the user."
          blocking_question: "Has the user given final acceptance for the completed Task?"
          required_refs:
            - record_kind: task
              record_id: task_close_001
              project_id: proj_close_001
              task_id: task_close_001
              state_version: 72
  guarantee_display: null
blockers:
  - category: final_acceptance
    code: missing_final_acceptance
    message: "Final acceptance is still required before this Task can close."
    can_resolve_in_chat: false
    terminal_action_required: false
    related_refs: []
    next_actions:
      - action_kind: request_user_judgment
        owner_method: volicord.request_user_judgment
        label: "Request final acceptance from the user."
        blocking_question: "Has the user given final acceptance for the completed Task?"
        required_refs:
          - record_kind: task
            record_id: task_close_001
            project_id: proj_close_001
            task_id: task_close_001
            state_version: 72
evidence_summary: null
artifact_refs: []
```

## 담당 문서 링크

- 요청 래퍼, 공통 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `CloseTaskResult.blockers`, `CurrentCloseBasis`, `RiskAcceptanceCoverage`, `CloseReadinessBlocker`, `ProjectContinuitySummary`, `EvidenceSummary`, `StateSummary`, `NextActionSummary` 형태: [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes).
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
