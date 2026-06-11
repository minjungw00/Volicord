# 저장 효과 참조

이 문서는 현재 MVP 원천 설계에서 메서드와 응답 분기가 어떤 저장 효과를 만들 수 있는지 담당합니다. 문서 원천 자료일 뿐이며 하네스 런타임 절차를 실행하거나 모의 실행하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 읽기 전용, `dry_run`, 거절 응답, 스테이징 생성, Core 커밋, 커밋된 차단 결과의 저장 효과 구분.
- 각 분기가 담당 기록, `task_events`, 재실행 행, `project_state.state_version`, 스테이징 핸들, 아티팩트 승격, `Write Authorization`을 바꿀 수 있는지 여부.
- 차단 사유형 응답 데이터가 지속 저장되는 경계.
- 거절 응답과 유효한 `dry_run` 미리보기의 효과 없음 보장.

이 문서는 담당하지 않습니다.

| 주제 | 담당 문서 |
|---|---|
| 기록 배치와 DDL | [저장소 기록](storage-records.md) |
| 아티팩트 생명주기 세부사항 | [아티팩트 저장소](storage-artifacts.md) |
| 멱등성, 잠금, `state_version` 시계, 이벤트 순서, 마이그레이션 | [저장소 버전 관리](storage-versioning.md) |
| 공개 응답 분기와 스키마 | [API 코어 스키마](api/schema-core.md) |
| API 메서드 동작 | [MVP API](api/mvp-api.md) |
| 공개 오류 코드 우선순위 | [API 오류](api/errors.md) |

## 저장 효과 분기 요약

응답 형태와 저장 효과는 별개입니다. 효과는 선택된 메서드 동작과 응답 분기가 정합니다.

이 표에서 담당 기록은 각 메서드 담당 문서가 소유하는 현재 기록을 뜻합니다. 이벤트 기록은 `task_events` 추가를 뜻합니다.

| 분기 | 담당 기록 변경 | 이벤트 기록 | `state_version` 증가 | 메모 |
|---|---|---|---|---|
| 읽기 전용 결과 | 없음 | 없음 | 없음 | 응답에만 남습니다. `harness.status`와 `harness.close_task intent=check`가 여기에 속합니다. |
| 거절 응답 (`ToolRejectedResponse`) | 없음 | 없음 | 없음 | 사전 확인 거절은 요청된 커밋 동작을 수행하지 않습니다. |
| `dry_run` 미리보기 (`ToolDryRunResponse`) | 없음 | 없음 | 없음 | 쓰기 효과를 지속하지 않습니다. 계획된 효과와 차단 사유는 미리보기 데이터입니다. |
| 스테이징 생성 (`StageArtifactResult`, `effect_kind=staging_created`) | Core 담당 기록 없음 | 없음 | 없음 | 저장소 소유 임시 스테이징만 만들 수 있습니다. 지속 `ArtifactRef`는 만들지 않습니다. |
| 커밋된 차단 결과 (`MethodResult`) | 메서드 담당 문서가 허용한 기록만 가능 | 허용된 경우만 가능 | 허용된 커밋인 경우만 가능 | 보고한 부족 권한 자체를 만들면 안 됩니다. |
| 성공 결과(상태 변경 커밋) | 메서드 담당 문서가 허용한 기록 가능 | 허용된 경우 가능 | 커밋당 정확히 한 번 | 성공한 상태 변경은 `project_state.state_version`을 올립니다. |

## 효과가 없는 분기

아래 실패는 효과가 없는 분기를 반환합니다.

- 잘못된 요청.
- 커밋 전 검증 실패.
- 보호된 동작이 진행되기 전의 로컬 접근 실패.
- 역량 실패.
- 오래된 `expected_state_version`.
- 오래된 `WriteAuthorization.basis_state_version`.
- 멱등 요청 해시 충돌.
- 거절된 아티팩트 입력.

효과가 없는 분기는 아래 항목을 만들거나 바꾸면 안 됩니다.

- 담당 기록.
- `task_events` 추가.
- `tool_invocations.response_json`.
- 재실행 행.
- 증거 요약.
- `close_state`.
- `Write Authorization` 생성 또는 소비.
- `artifact_staging.status`.
- `consumed_by_run_id` 또는 `promoted_artifact_id`.
- 아티팩트 승격 또는 연결.
- `project_state.state_version` 증가.

사전 확인에서 `ToolRejectedResponse`가 반환되면 요청된 커밋 동작은 수행되지 않습니다. 이 원칙은 `dry_run` 요청에도 똑같이 적용됩니다. `dry_run`은 검증, 접근, 역량, 오래된 상태 거절을 우회하지 않습니다.

## dry-run 미리보기 효과

유효한 `dry_run` 미리보기는 저장 효과가 아니라 응답 미리보기입니다.

`ToolDryRunResponse`는 `DryRunSummary.would_blockers: PlannedBlocker[]` 또는 계획된 효과를 포함할 수 있습니다. 이 값들은 커밋된 담당 기록, 저장된 닫기 차단 사유, 지속 아티팩트, 재실행 저장을 뜻하지 않습니다.

유효한 `dry_run` 미리보기는 아래 항목을 만들지 않습니다.

- `task_event` 또는 `task_events` 추가.
- 재실행 행 또는 `tool_invocations.response_json`.
- 생성된 지속 참조.
- `close_state` 변경.
- `Write Authorization` 변경.
- 스테이징 핸들 생성 또는 소비.
- 아티팩트 승격, 연결, 또는 그 밖의 아티팩트 효과.
- 증거 업데이트.
- `CloseReadinessBlocker` 저장.
- `project_state.state_version` 증가.

## 커밋된 차단 결과의 저장 효과

커밋된 차단 결과는 거절 응답과 다릅니다. `harness.prepare_write` 또는 `harness.close_task`의 커밋된 차단 결과는 [MVP API](api/mvp-api.md)가 차단 커밋을 허용할 때만 `MethodResult`입니다.

`harness.prepare_write`에서 커밋된 비허용 판단은 메서드 상태 효과 계약이 허용한 경우에만 저장 효과를 가질 수 있습니다.

| 조건 | 허용될 수 있는 효과 | 허용되지 않는 효과 |
|---|---|---|
| 커밋된 `dry_run=false` `PrepareWriteResult`이고 `decision=blocked`, `decision=approval_required`, 또는 `decision=decision_required`인 경우 | 응답과 재실행 페이로드의 `write_decision_reasons: WriteDecisionReason[]`. 단, 메서드 계약이 그 판단 커밋을 허용할 때만 가능합니다. | 소비 가능한 `Write Authorization` 생성, `close_state` 변경, 닫기 준비 상태 평가, `CloseReadinessBlocker` 저장, 증거 업데이트, 아티팩트 변경, 스테이징 핸들 소비, `close_task` 효과. |

이 사유는 `prepare_write` 판단 사유입니다. 아래 항목이 아닙니다.

- 닫기 준비 상태 평가 결과.
- `CloseReadinessBlocker[]`.
- 닫기 차단 사유 기록.

`harness.close_task`에서 `CloseTaskResult(close_state=blocked)`는 아래 조건을 모두 만족할 때만 저장 효과가 있습니다.

- 닫기 준비 상태 평가가 실행되었습니다.
- `harness.close_task` 메서드 계약이 차단 결과 커밋을 허용합니다.

허용된 경우에도 만들 수 있는 효과는 API/저장소 계약이 명시한 아래 항목뿐입니다.

- 차단 사유 상태.
- `task_events`.
- 재실행 행.
- `project_state.state_version` 증가.

Task는 열린 상태로 남습니다.

`STATE_VERSION_CONFLICT`에는 커밋된 차단 결과 분기를 사용하면 안 됩니다. 이 오류는 사전 확인의 `ToolRejectedResponse` 분기에 속하며 재실행으로 저장하지 않습니다.

<a id="메서드별-저장-효과"></a>
## 메서드 저장 효과 요약

아래 표는 메서드별 지속 저장 효과를 요약합니다. 메서드 동작과 응답 공용체는 [MVP API](api/mvp-api.md)가 담당합니다.

| 메서드 | 주 저장 효과 | 세부사항 |
|---|---|---|
| `harness.intake` | Task와 구체화 기록 생성 | [`harness.intake`](#harnessintake) |
| `harness.update_scope` | 활성 범위 기록 갱신 | [`harness.update_scope`](#harnessupdate_scope) |
| `harness.status` | 읽기 전용 응답 | [`harness.status`](#harnessstatus) |
| `harness.prepare_write` | 쓰기 판단 효과 기록 | [`harness.prepare_write`](#harnessprepare_write) |
| `harness.stage_artifact` | 임시 스테이징만 생성 | [`harness.stage_artifact`](#harnessstage_artifact) |
| `harness.record_run` | 실행/증거 효과 기록 | [`harness.record_run`](#harnessrecord_run) |
| `harness.request_user_judgment` | 대기 중인 판단 요청 생성 | [`harness.request_user_judgment`](#harnessrequest_user_judgment) |
| `harness.record_user_judgment` | 사용자 판단 해결 | [`harness.record_user_judgment`](#harnessrecord_user_judgment) |
| `harness.close_task intent=check` | 읽기 전용 닫기 준비 상태 점검 | [`harness.close_task intent=check`](#harnessclose_task-intentcheck) |
| `harness.close_task intent=complete` | 닫기 또는 차단된 `complete` 결과 기록 | [`harness.close_task intent=complete`](#harnessclose_task-intentcomplete) |
| `harness.close_task intent=cancel` | 취소 또는 차단된 취소 기록 | [`harness.close_task intent=cancel`](#harnessclose_task-intentcancel) |
| `harness.close_task intent=supersede` | 대체 또는 차단된 대체 기록 | [`harness.close_task intent=supersede`](#harnessclose_task-intentsupersede) |

### `harness.intake`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- Task를 생성합니다.
- 선택적 Change Unit을 생성합니다.
- 구체화 기록을 생성합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run=true`
- 거절된 시도

이 분기는 Task, 참조, 이벤트, 재실행 행, `state_version` 증가를 만들지 않습니다.

담당 문서:

- [MVP API의 `harness.intake`](api/mvp-api.md#harnessintake)
- [저장소 기록](storage-records.md)
- [저장소 버전 관리](storage-versioning.md)

### `harness.update_scope`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 활성 Task 범위 필드를 갱신합니다.
- 활성 `change_units`를 만들거나 교체합니다.
- 메서드 담당 문서가 허용한 차단 사유 또는 오래된 `Write Authorization` 참조를 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 dry-run 미리보기
- 거절된 시도

유효한 dry-run 미리보기는 범위, Change Unit, 차단 사유, 오래된 승인 효과만 미리 설명합니다.

담당 문서:

- [MVP API의 `harness.update_scope`](api/mvp-api.md#harnessupdate_scope)
- [저장소 기록](storage-records.md)
- [저장소 버전 관리](storage-versioning.md)

### `harness.status`

읽기 전용 호출은 다음 특성을 가집니다.

- 응답 데이터만 반환합니다.
- 재실행 행을 만들지 않습니다.
- 저장소를 변경하지 않습니다.
- `project_state.state_version`을 증가시키지 않습니다.

`dry_run=true`도 `ToolDryRunResponse`가 아니라 `effect_kind=read_only`인 `StatusResult`로 유지됩니다.

효과가 없는 분기:

- 거절된 시도

담당 문서:

- [MVP API의 `harness.status`](api/mvp-api.md#harnessstatus)

### `harness.prepare_write`

`decision=allowed`로 커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 호환되는 활성 `Write Authorization`을 만들거나 반환합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

커밋되는 비허용 판단은 허용된 판단 상태와 재실행 효과만 지속할 수 있습니다.

효과가 없는 분기:

- 거절된 시도
- 유효한 dry-run 미리보기

이 분기는 재실행 행, `Write Authorization`, 이벤트, `close_state` 변경, 아티팩트/증거 효과, `state_version` 증가를 만들지 않습니다.

담당 문서:

- [MVP API의 `harness.prepare_write`](api/mvp-api.md#harnessprepare_write)
- [저장소 기록](storage-records.md)
- [저장소 버전 관리](storage-versioning.md)

### `harness.stage_artifact`

성공한 스테이징은 다음을 수행할 수 있습니다.

- `artifact_staging` 또는 동등한 저장소 소유 스테이징 매니페스트를 생성합니다.
- `artifacts/tmp/` 아래에 임시 안전 바이트 또는 알림을 둡니다.

이 분기는 저장소 소유 임시 스테이징만 생성합니다. Core 현재 기록, 지속 `ArtifactRef`, 재실행 행, `state_version` 증가는 만들지 않습니다.

효과가 없는 분기:

- 유효한 `dry_run=true`
- 잘못된 스테이징 요청

유효한 `dry_run=true`는 바이트, 스테이징 매니페스트, `StagedArtifactHandle`, 재실행 행, `state_version` 증가를 만들지 않습니다.

담당 문서:

- [MVP API의 `harness.stage_artifact`](api/mvp-api.md#harnessstage_artifact)
- [아티팩트 저장소](storage-artifacts.md)

### `harness.record_run`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- `runs`를 생성합니다.
- 호환되는 `write_authorizations`를 소비합니다.
- 사용할 수 있는 `artifact_staging`을 소비합니다.
- `artifacts`를 승격하거나 연결합니다.
- `evidence_summaries` 또는 허용된 `blockers`를 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 dry-run 미리보기
- 거절된 시도
- 커밋 전의 잘못된 스테이징 핸들

유효한 dry-run 미리보기는 `run_summary`, 지속 아티팩트, 아티팩트 연결, 증거 갱신, 차단 사유 갱신, 이벤트, 재실행 행, 스테이징 핸들 소비, `Write Authorization` 소비, `state_version` 증가를 만들지 않습니다. 거절된 시도는 스테이징 행이나 아티팩트를 바꾸지 않습니다.

담당 문서:

- [MVP API의 `harness.record_run`](api/mvp-api.md#harnessrecord_run)
- [아티팩트 저장소](storage-artifacts.md)
- [저장소 기록](storage-records.md)

### `harness.request_user_judgment`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 대기 중인 `user_judgments` 행을 생성합니다.
- 영향받은 차단 사유를 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 dry-run 미리보기
- 거절된 시도

유효한 dry-run 미리보기는 실제 `user_judgment_ref`, 대기 중인 판단, 차단 사유 갱신, 이벤트, 재실행 행, `state_version` 증가를 만들지 않습니다.

담당 문서:

- [MVP API의 `harness.request_user_judgment`](api/mvp-api.md#harnessrequest_user_judgment)
- [저장소 기록](storage-records.md)

### `harness.record_user_judgment`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- `user_judgments` 행을 해결합니다.
- 종속 차단 사유 또는 다음 행동을 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 dry-run 미리보기
- 거절된 시도

유효한 dry-run 미리보기는 판단 해결, 차단 사유 갱신, 이벤트, 재실행 행, `state_version` 증가를 만들지 않습니다.

담당 문서:

- [MVP API의 `harness.record_user_judgment`](api/mvp-api.md#harnessrecord_user_judgment)
- [저장소 기록](storage-records.md)

### `harness.close_task intent=check`

읽기 전용 호출은 다음 특성을 가집니다.

- 계산된 닫기 준비 상태를 반환합니다.
- 재실행 행을 만들지 않습니다.
- 이벤트를 추가하지 않습니다.
- 차단 사유 행을 만들지 않습니다.
- `close_state`를 변경하지 않습니다.
- 아티팩트나 증거를 바꾸지 않습니다.
- `project_state.state_version`을 증가시키지 않습니다.

`dry_run=true`도 `effect_kind=read_only`인 `CloseTaskResult`로 유지됩니다.

효과가 없는 분기:

- 거절된 시도

담당 문서:

- [MVP API의 `harness.close_task`](api/mvp-api.md#harnessclose_task)

### `harness.close_task intent=complete`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 차단 사유가 허용할 때 Task를 닫습니다.
- Task를 열린 상태로 둔 채 허용된 `complete` 차단 효과를 커밋합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run=true`
- 사전 확인 실패

유효한 `dry_run=true`는 `ToolDryRunResponse`를 반환합니다. 사전 확인 실패는 효과가 없는 `ToolRejectedResponse`입니다.

담당 문서:

- [MVP API의 `harness.close_task`](api/mvp-api.md#harnessclose_task)
- [저장소 버전 관리](storage-versioning.md)

### `harness.close_task intent=cancel`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- Task를 취소합니다.
- Task를 열린 상태로 둔 채 취소 자체를 무효화하는 차단 사유를 커밋합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

취소는 증거 충분성이 아닙니다.

효과가 없는 분기:

- 유효한 `dry_run=true`
- 사전 확인 실패

유효한 `dry_run=true`는 `ToolDryRunResponse`를 반환합니다.

담당 문서:

- [MVP API의 `harness.close_task`](api/mvp-api.md#harnessclose_task)
- [저장소 버전 관리](storage-versioning.md)

### `harness.close_task intent=supersede`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- Task를 대체합니다.
- 같은 변경에서 `project_state.active_task_id`를 갱신합니다.
- 대체 자체를 무효화하는 차단 사유를 커밋합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

대체는 증거 충분성이 아닙니다.

효과가 없는 분기:

- 유효한 `dry_run=true`
- 사전 확인 실패

유효한 `dry_run=true`는 `ToolDryRunResponse`를 반환합니다.

담당 문서:

- [MVP API의 `harness.close_task`](api/mvp-api.md#harnessclose_task)
- [저장소 버전 관리](storage-versioning.md)

## `state_version` 영향

`project_state.state_version`은 상태 효과가 허용된 커밋에서만 증가합니다. 성공한 상태 변경은 커밋당 정확히 한 번 증가하고, 커밋된 차단 결과는 메서드 담당 문서가 `state_version` 효과를 허용할 때만 증가할 수 있습니다.

| 분기 | `project_state.state_version` 영향 |
|---|---|
| 읽기 전용 결과 | 증가하지 않습니다. |
| `ToolRejectedResponse` | 증가하지 않습니다. |
| 유효한 `ToolDryRunResponse` | 증가하지 않습니다. |
| `StageArtifactResult`, `effect_kind=staging_created` | 증가하지 않습니다. 임시 스테이징은 Core 상태 변경이 아닙니다. |
| 커밋된 차단 결과 | 메서드 담당 문서가 차단 결과 커밋과 `state_version` 효과를 허용할 때만 증가할 수 있습니다. |
| 성공한 상태 변경 | 커밋당 정확히 한 번 증가합니다. |

`state_version`이 오래된 경우는 커밋된 차단 결과가 아닙니다. 오래된 `expected_state_version` 또는 오래된 `WriteAuthorization.basis_state_version`은 사전 확인의 `ToolRejectedResponse` 분기에 속하며 저장 효과가 없습니다.

## 저장 효과가 아닌 것

아래 값이 응답에 있다는 사실만으로 저장 효과가 증명되지 않습니다.

- `CloseReadinessBlocker`.
- `WriteDecisionReason`.
- `PlannedBlocker`.
- `ArtifactRef`.
- `StagedArtifactHandle`.
- `DryRunSummary.would_blockers`.
- 계획된 효과 설명.

특히 아래 추론은 하지 않습니다.

- 응답에 `CloseReadinessBlocker[]`가 있으므로 닫기 차단 사유 기록이 저장되었다.
- 응답에 `ArtifactRef`가 있으므로 아티팩트가 승격되었다.
- 응답에 `StagedArtifactHandle`이 있으므로 스테이징 핸들이 소비되었다.
- `dry_run` 응답에 계획된 효과가 있으므로 담당 기록이 바뀌었다.
- 읽기 전용 결과가 차단 사유나 증거 요약을 계산했으므로 그 계산값이 지속 저장되었다.

읽기 전용 결과는 응답을 위해 차단 사유, `CloseReadinessBlocker[]`, 증거 요약, 아티팩트 참조, 진단, 다음 행동을 계산할 수 있습니다. 읽기가 일어났다는 이유만으로 그 계산값을 저장하면 안 됩니다.

`harness.status`의 `close_blockers: CloseReadinessBlocker[]`는 읽기 전용 관찰입니다. 이 결과는 `task_events`, 재실행 행, `tool_invocations.response_json`, `close_state`, `Write Authorization`, 스테이징 핸들, 아티팩트, 증거 요약, `project_state.state_version`을 바꾸지 않습니다.

`harness.close_task intent=check`의 응답 분기는 [`harness.close_task`](api/mvp-api.md#harnessclose_task)가 담당합니다. 이 문서는 `dry_run=true`이거나 `blockers: CloseReadinessBlocker[]`를 포함하더라도 그 점검이 읽기 전용이라는 점만 담당합니다.

## 관련 담당 문서

- [MVP API](api/mvp-api.md): 선택된 메서드 동작과 응답 공용체.
- [API 오류](api/errors.md): 거절 응답의 공개 오류.
- [저장소 기록](storage-records.md): 저장 효과가 건드릴 수 있는 기록.
- [아티팩트 저장소](storage-artifacts.md): 스테이징 핸들과 아티팩트 생명주기 세부사항.
- [저장소 버전 관리](storage-versioning.md): `state_version` 시계와 재실행/멱등성 의미.
