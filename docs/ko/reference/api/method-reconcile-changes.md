<a id="volicordreconcile_changes"></a>

# `volicord.reconcile_changes` 참조

## 이 문서가 담당하는 것

이 문서는 기준 범위의 `volicord.reconcile_changes` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 현재 프로젝트와 `Task`의 미해결 미기록 Product Repository 변경 찾기 나열
- Core가 결정적으로 검증할 수 있는 찾기 해결
- 사용자 수락이 필요한 찾기에 대해 대기 중인 사용자 소유 판단 생성
- 미해결 Product Repository 변경 찾기에 대한 에이전트 단독 묵살 거부

## 이 문서가 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, dry-run, 거절 응답 스키마 본문
- `UserJudgment`, `StateRecordRef`, `CloseReadinessBlocker`, `GuardHealthSummary`, `NextActionSummary` 필드 정의
- 저장소 테이블 배치, SQLite 제약, 공개 오류 코드 의미, 공개 오류 우선순위, 공통 응답 분기 처리
- 정확성 증명, 테스트 충분성, 리뷰 완료, 최종 수락, 잔여 위험 수락, 보안 보장

## 목적

`volicord.reconcile_changes`는 guarded 미기록 Product Repository 변경 찾기를 복구하는 공개 경로입니다.

이 메서드는 선택된 `Task`의 미해결 찾기를 나열하고, Core가 저장된 Core 또는 guard 기록에서 검증할 수 있는 찾기를 해결하며, 남은 찾기에 사용자 소유 수락 판단이 필요하면 일반 대기 `UserJudgment` 행을 만듭니다. 우회 찾기를 조용히 묵살하면 안 됩니다. Agent Connection이 호환되는 해결된 User Channel 판단 없이 미기록 Product Repository 변경을 수락으로 표시하게 하면 안 됩니다.

미기록 변경 찾기를 해결하면 해당 찾기는 미해결 guard 상태 수와 `unresolved_unrecorded_changes` 닫기 차단 계산에서 빠집니다. 이는 변경된 제품 파일이 정확하거나, 리뷰되었거나, 테스트되었거나, 닫기에 최종 수락되었거나, 잔여 위험으로 수락 가능하다는 증명이 아닙니다.

## 필수 입력

- 유효한 `ToolEnvelope`. 상태를 변경하는 커밋된 non-dry-run 요청은 null이 아닌 `idempotency_key`와 현재 `expected_state_version`을 요구합니다.
- 미해결 찾기를 조정할 `Task`의 `task_id`.
- `accepted_by_user`에 대해 해결된 사용자 판단을 Core에 가리키려는 경우 선택적 `resolution_requests` 항목.

Core는 현재 프로젝트와 `Task`의 미해결 찾기도 함께 스캔합니다. 호출자는 관찰 경로, detection 사실, 행위자 출처, 결정적 증명, 닫기 차단 상태를 제출하지 않습니다.

## 요청 스키마

이 메서드는 아래 최상위 `params` 요청 형태를 담당합니다. `envelope`은 공유 [`ToolEnvelope`](schema-core.md#tool-envelope)입니다. 이 블록은 `ToolEnvelope` 필드를 다시 정의하지 않습니다.

```yaml
ReconcileChangesRequest:
  envelope: ToolEnvelope
  task_id: string
  resolution_requests: UnrecordedChangeResolutionRequest[]

UnrecordedChangeResolutionRequest:
  unrecorded_change_id: string
  basis: string
  user_judgment_id: string | null
```

요청 필드 참고:

- `resolution_requests`의 기본값은 `[]`입니다.
- `basis=accepted_by_user`는 미기록 변경 참조에 연결된 기존 해결 판단을 `user_judgment_id`로 요구합니다. 그 판단은 같은 `Task`의 현재 `product_decision`이어야 하고, 호환 User Channel에서 `actor_source=local_user`, `machine_action=accept`, `resolution_outcome=accepted`로 기록되어야 합니다.
- 호출자가 제공한 `reverted`, `covered_by_write_readiness`, `recorded_as_expected_write`, `not_product_change`, `superseded_by_new_observation`, `invalid_observation` 요청은 에이전트가 제공한 시스템 해결 basis로 거부됩니다. Core가 결정적으로 검증할 수 있으면 같은 basis를 Core가 직접 적용할 수는 있습니다.

중첩 담당 문서:

- `UnrecordedChangeFinding`과 `UnrecordedChangeResolutionSummary` 형태: [API 상태 스키마](schema-state.md#unrecorded-change-reconciliation-shapes).
- 해결 basis 값: [API 값 집합](schema-value-sets.md#unrecorded-change-resolution-basis-values).
- 사용자 소유 판단 형태: [API 판단 스키마](schema-judgment.md).

## 접근 요구사항

이 메서드는 아래를 요구합니다.

- `operation_category=agent_workflow`인 검증된 호출 맥락
- `task_id`가 선택한 같은 프로젝트의 호환 `Task`
- MCP를 통해 호출할 때 workflow를 허용하는 Agent Connection

로컬 관리 복구 명령은 `operation_category=agent_workflow`를 유지하면서 로컬 사용자 행위자 출처로 같은 Core 메서드를 호출할 수 있습니다. 이 CLI 경로는 공개 API 메서드가 아니며 CLI가 사용자 판단을 가장하게 하지 않습니다.

## 상태 버전 동작

저장 효과가 계획된 커밋 non-dry-run 결과는 아래 효과를 냅니다.

- `project_state.state_version`을 정확히 한 번 증가시킵니다.
- 하나 이상의 `unrecorded_changes` 행을 해결하고 resolution basis, capture basis, 행위자 출처, 타임스탬프, 선택적 연결 사용자 판단 참조를 저장합니다.
- 그리고/또는 사용자 수락이 필요한 남은 찾기에 대해 대기 중인 `user_judgments` 행을 만듭니다.
- task event 하나를 추가하고, idempotency key가 있으면 replay 행을 만듭니다.
- 해결된 찾기가 더 이상 미해결로 계산되지 않도록 닫기 준비 상태 보기를 갱신합니다.

저장 변경이 없는 유효한 호출은 읽기 전용 결과를 반환하며 replay 행, event, 상태 버전 증가를 만들지 않습니다.

Dry run은 계획된 해결이나 대기 판단을 미리 보여 줄 뿐 ref, event, replay 행, 사용자 판단, 해결 행을 만들지 않습니다. 거절된 시도는 효과를 만들지 않습니다.

## 성공 결과

`ReconcileChangesResult`를 반환합니다.

- `base.response_kind=result`
- 찾기가 해결되거나 판단이 생성되면 `base.effect_kind=core_committed`
- 저장 변경이 필요 없으면 `base.effect_kind=read_only`
- `task_ref`
- `unresolved_changes`
- `resolved_changes`
- `pending_user_judgment_refs`
- `rejected_resolution_requests`
- 현재 `state`
- 계획 후 `close_blockers`
- 계획 후 `guard_health`
- `next_actions`

## 메서드 결과 필드

| 필드 | 결과 필드 의미 |
|---|---|
| `base` | 공통 결과 메타데이터입니다. `ToolResultBase` 형태는 [API 코어 스키마](schema-core.md#common-response)가 담당합니다. |
| `task_ref` | 조정한 `Task`의 `StateRecordRef`입니다. |
| `unresolved_changes` | 이 호출이 선택한 결정적 해결과 사용자 수락 해결을 적용한 뒤에도 남은 미해결 찾기입니다. |
| `resolved_changes` | 이 호출이 해결한 찾기입니다. basis, 행위자 출처, capture basis, 타임스탬프, 선택적 연결 사용자 판단을 포함합니다. |
| `pending_user_judgment_refs` | 이 호출이 미해결 찾기를 위해 만든 판단을 포함해 호출 뒤 `Task`와 관련된 대기 `UserJudgment` 참조입니다. |
| `rejected_resolution_requests` | Core가 거부한 호출자 제공 해결 요청입니다. 이는 성공한 메서드 결과 안의 구조화된 거부이며 공개 `ToolRejectedResponse` 오류가 아닙니다. |
| `state` | 조정 보기 또는 커밋 뒤의 현재 `StateSummary`입니다. |
| `close_blockers` | 계획된 조정 효과 뒤의 닫기 차단 사유 보기입니다. |
| `guard_health` | 검증된 연결에 대해 guard 상태를 사용할 수 있을 때의 guard 상태 보기입니다. |
| `next_actions` | 만들어진 사용자 소유 판단을 기록하거나 조정을 다시 실행하는 등 다음 안전 단계입니다. |

## 해결 동작

Core 소유 결정적 basis:

- `invalid_observation`: 저장된 관찰 데이터를 Product Repository 경로로 해석할 수 없습니다.
- `not_product_change`: 저장된 관찰 데이터에 조정할 Product Repository 경로가 없습니다.
- `recorded_as_expected_write`: 같은 `Task`의 기록된 Run이 관찰된 Product Repository 경로를 이미 덮습니다.
- `covered_by_write_readiness`: 같은 `Task`의 소비된 호환 `Write Check`가 관찰된 Product Repository 경로를 덮습니다.

사용자 소유 basis:

- `accepted_by_user`: 찾기에 연결된 호환 해결 `product_decision` 판단이 로컬 사용자가 해당 관찰 변경을 이 `Task`에서 의도된 변경으로 수락했음을 기록합니다.

`reverted`, `superseded_by_new_observation` 같은 예약 또는 향후 담당자 정의 basis와 그 밖의 나열된 basis는 담당자가 정의한 검증이 구현된 경우에만 저장할 수 있습니다. 이 메서드는 그 검증이 안전하고 담당자가 정의하지 않은 한 파일시스템 되돌리기나 파일시스템 탐색 basis를 구현하면 안 됩니다.

아직 수락이 필요한 찾기에 대해 Core는 이를 수락하지 않고 대기 `UserJudgment` 행을 만듭니다. 기존 User Channel 경로는 이 판단에 답할 수 있습니다. 여기에는 로컬 `volicord user` 명령, 설정된 경우 guarded prompt-capture 명령 처리, 기존 User Channel 통합이 지원하는 경우 MCP elicitation 흐름이 포함됩니다. 사용자 소유 판단이 해결된 뒤 `volicord.reconcile_changes`는 연결된 찾기를 `accepted_by_user`로 해결할 수 있습니다.

## 거절 결과

아래와 같은 커밋 전 실패에는 `ToolRejectedResponse`를 반환합니다.

- 잘못된 요청 형태
- 누락되었거나 호환되지 않는 `Task` 식별 정보
- 행위자 출처 또는 operation category 불일치
- 지원하지 않는 호출 맥락
- 오래된 `expected_state_version`
- idempotency 요청 해시 충돌
- 읽을 수 없는 owner state

개별 찾기에 대한 에이전트 단독 묵살 시도는 보통 전체 메서드 호출을 거절하지 않습니다. 해당 시도는 `rejected_resolution_requests`에 나타나며 찾기는 미해결로 남습니다.

## Dry-run 동작

`dry_run=true`에서 유효한 미리보기는 계획 효과를 담은 `ToolDryRunResponse`를 반환합니다. 찾기를 해결하지 않고, 대기 판단을 만들지 않으며, event, replay 행, `project_state.state_version` 증가를 만들지 않습니다.

## 저장 효과

커밋 시 이 메서드는 미기록 변경 해결 행과 대기 사용자 판단을 저장할 수 있습니다. 정확한 저장 효과는 [저장 효과](../storage-effects.md#volicordreconcile_changes)가 담당합니다.

## 관련 담당 문서

- 정확한 공개 메서드 경로: [API 메서드](methods.md).
- 값 집합: [API 값 집합](schema-value-sets.md#unrecorded-change-resolution-basis-values).
- 상태 형태 응답 필드: [API 상태 스키마](schema-state.md#unrecorded-change-reconciliation-shapes).
- 사용자 판단 권한: [`volicord.record_user_judgment`](method-record-user-judgment.md).
- 닫기 차단 사유 생성: [`volicord.close_task`](method-close-task.md).
- 저장 효과: [저장 효과](../storage-effects.md#volicordreconcile_changes).
