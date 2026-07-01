# 저장 효과

이 문서는 기준 범위 원천 설계에서 메서드와 응답 분기가 어떤 저장 효과를 만들 수 있는지 담당합니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 읽기 전용, `dry_run`, 거부 응답, 스테이징 생성, Core 커밋, 커밋된 차단 결과의 저장 효과 구분.
- 각 분기가 담당 기록, `task_events`, 재실행 행, `project_state.state_version`, 스테이징 핸들 생성 또는 소비, 아티팩트 승격, Write Check를 바꿀 수 있는지 여부.
- 차단 사유형 응답 데이터가 지속 저장되는 경계.
- 거부 응답과 유효한 `dry_run` 미리보기의 효과 없음 보장.

이 문서는 담당하지 않습니다.

- 기록 계열 개요: [저장소 기록](storage-records.md)을 봅니다.
- 기준 SQLite DDL, 제약, 인덱스, 외래 키, 마이그레이션 테이블 형태: [저장소 DDL](storage-ddl.md)을 봅니다.
- 아티팩트 생명주기 세부사항; [아티팩트 저장소](storage-artifacts.md)를 봅니다.
- 멱등성, 잠금, `state_version` 시계, 이벤트 순서, 마이그레이션; [저장소 버전 관리](storage-versioning.md)를 봅니다.
- 공개 응답 분기와 스키마; [API 코어 스키마](api/schema-core.md)를 봅니다.
- API 메서드 동작; [API 메서드](api/methods.md)와 메서드 담당 문서를 봅니다.
- 공개 오류 코드 우선순위; [API 오류 우선순위](api/error-precedence.md)를 봅니다.

## 형태와 효과

응답 형태와 저장 효과는 별개입니다.

API 데이터 형태는 API 스키마 담당 문서가 담당합니다. 차단 사유형 상태 형태는 [API 상태 스키마](api/schema-state.md)가, 아티팩트 형태는 [API 아티팩트 스키마](api/schema-artifacts.md)가 담당합니다. 예시는 아래와 같습니다.

- `CloseReadinessBlocker`
- `WriteDecisionReason`
- `PlannedBlocker`
- `ArtifactRef`
- `StagedArtifactHandle`

비주장: 응답에 이런 값이 있다는 사실만으로 지속 저장, 아티팩트 승격, 스테이징 핸들 소비, 재실행 저장, `close_state` 변경, `project_state.state_version` 증가가 증명되지는 않습니다.

효과는 선택된 메서드 동작과 응답 분기가 정합니다. 아래 표는 각 분기를 짧게 요약하고, 세부 블록은 허용될 수 있는 효과와 허용되지 않는 효과를 나누어 설명합니다.

| 효과 범주 | 응답 또는 분기 | 지속 저장 결과 | 세부사항 |
|---|---|---|---|
| 읽기 전용 | 읽기 전용 `MethodResult` | 응답만 반환합니다. 재실행 행, 이벤트, 아티팩트 효과, Write Check 효과, `project_state.state_version` 증가는 없습니다. | [읽기 전용 결과](#read-only-result) |
| 효과 없음 | `ToolRejectedResponse` 또는 `effect_kind=no_effect`인 유효한 `MethodResult` | 요청된 일반 변이가 없고 Core 커밋도 없습니다. 응답이 오류나 차단 사유형 데이터를 담을 수 있지만, 이 분기는 그 값을 지속하지 않습니다. | [`ToolRejectedResponse`](#toolrejectedresponse-effect), [효과가 없는 분기](#no-effect-branches) |
| `dry_run` | 유효한 `ToolDryRunResponse` | 미리보기만 반환합니다. 지속 참조, 재실행 행, 이벤트, 스테이징 핸들, 아티팩트 효과, `project_state.state_version` 증가는 없습니다. | [유효한 `dry_run` 미리보기](#valid-dry-run-preview) |
| 스테이징 생성 | `effect_kind=staging_created`인 `StageArtifactResult` | 저장소 소유 임시 스테이징만 생성합니다. 일반 Core 커밋 트랜잭션이 아닙니다. | [스테이징 생성 아티팩트 결과](#staging-created-artifact-result) |
| Core 커밋 | Core 커밋 `MethodResult` | `CoreProjectStore::commit_mutation`을 통해 메서드 담당 효과를 만듭니다. 상태 버전 증가, Task 이벤트, 선택적 재실행 행, 메서드가 선택한 `CoreStorageMutation` 값이 포함됩니다. | [Core 커밋 결과](#core-committed-result) |
| 커밋된 차단 사유형 결과 | 메서드 담당 문서가 차단 또는 비허용 지속 저장을 허용한 커밋 `MethodResult` | 명시적으로 허용된 이벤트, 재실행, 상태 버전, 차단 사유 상태 효과만 만듭니다. 차단 사유형 응답만으로는 충분하지 않습니다. | [커밋된 차단 결과](#committed-blocked-result) |

<a id="read-only-result"></a>
### 읽기 전용 결과

저장 효과:

- 응답만 반환합니다.

허용되지 않는 효과:

- 재실행 행
- 이벤트
- 담당 기록 변경
- 아티팩트 효과
- Write Check 효과
- `project_state.state_version` 증가

<a id="toolrejectedresponse-effect"></a>
### `ToolRejectedResponse`

저장 효과:

- 없습니다.

허용되지 않는 효과:

- 담당 기록 생성 또는 변경
- 재실행 행
- 이벤트
- 아티팩트 효과
- Write Check 생성 또는 소비
- `project_state.state_version` 증가

<a id="valid-dry-run-preview"></a>
### 유효한 `dry_run` 미리보기

저장 효과:

- 응답 미리보기만 반환합니다.

허용되지 않는 효과:

- 담당 기록 생성 또는 변경
- 생성된 지속 참조
- 재실행 행
- 이벤트
- 스테이징 핸들 생성
- 아티팩트 승격 또는 연결
- `project_state.state_version` 증가

<a id="staging-created-artifact-result"></a>
### 스테이징 생성 아티팩트 결과

허용될 수 있는 효과:

- 저장소 소유 임시 스테이징

이 분기는 일반 Core 커밋 변이와 별개입니다. 저장소가 관리하는 스테이징 표현이나 핸들을 만들 수 있지만, 그 임시 스테이징 쓰기 자체가 Core 현재 기록 변경, 지속 `ArtifactRef`, 아티팩트 연결, 증거 기록은 아닙니다.

허용되지 않는 효과:

- Core 현재 기록
- 재실행 행
- 이벤트
- 지속 `ArtifactRef`
- `project_state.state_version` 증가

<a id="core-committed-result"></a>
### Core 커밋 결과

조건:

- 메서드 담당 문서가 커밋 효과를 허용합니다.

허용될 수 있는 효과:

- 담당 기록 변경
- `task_events` 추가
- 재실행 행 생성
- `project_state.state_version` 정확히 한 번 증가

아티팩트 승격과 `artifact_links` 생성은 메서드 담당 문서가 그런 아티팩트 효과를 명시적으로 포함하는 커밋 변이 분기를 선택할 때만 일어납니다. 앞선 스테이징만으로 자동 발생하지 않습니다.

<a id="committed-blocked-result"></a>
### 커밋된 차단 결과

조건:

- 메서드 담당 문서가 차단 결과 커밋을 허용합니다.

허용될 수 있는 효과:

- 명시적으로 허용된 차단 사유 상태 효과
- 명시적으로 허용된 이벤트 효과
- 명시적으로 허용된 재실행 행 효과
- 명시적으로 허용된 `project_state.state_version` 효과

허용되지 않는 효과:

- 그 분기가 보고하는 부족한 권한이나 근거 생성

<a id="no-effect-branches"></a>
## 효과가 없는 분기

효과가 없는 분기에는 거부 응답과, 메서드가 요청된 동작에 대해 지속 변이를
선택하지 않은 유효한 메서드 결과가 포함됩니다.

아래 실패는 효과가 없는 분기를 반환합니다.

- 잘못된 요청.
- 커밋 전 검증 실패.
- 보호된 동작이 진행되기 전의 연결 라우팅 또는 모드 게이트 실패.
- 오래된 `expected_state_version`.
- 오래된 `WriteCheck.basis_state_version`.
- 멱등 요청 해시 충돌.
- 거절된 아티팩트 입력.

효과가 없는 분기는 아래 항목을 만들거나 바꾸면 안 됩니다.

- 담당 기록.
- `task_events` 추가.
- `tool_invocations.response_json`.
- 재실행 행.
- 증거 요약 또는 증거 관찰.
- `close_state`.
- Write Check 생성 또는 소비.
- `artifact_staging.status`.
- `consumed_by_run_id` 또는 `promoted_artifact_id`.
- 아티팩트 승격 또는 연결.
- `project_state.state_version` 증가.

사전 확인에서 `ToolRejectedResponse`가 반환되면 요청된 커밋 동작은 수행되지 않습니다. 이 원칙은 `dry_run` 요청에도 똑같이 적용됩니다. `dry_run`은 검증, 접근, 역량, 오래된 상태 거절을 우회하지 않습니다.

메서드 담당 문서가 응답 전용 차단 분기를 선택하면 유효한 차단 결과도 효과가
없을 수 있습니다. 예를 들어 기준 `volicord.close_task`의 차단된 종료 시도는
`CloseTaskResult` 데이터를 반환하지만 차단 사유 행, Task 이벤트, 재실행 행,
상태 버전 증가를 커밋하지 않습니다. 이 경로는 커밋되는 비허용
`volicord.prepare_write` 결과와 별개입니다.

## `dry_run` 미리보기 효과

유효한 `dry_run` 미리보기는 `DryRunSummary.would_blockers: PlannedBlocker[]` 또는 계획된 효과를 포함할 수 있습니다. 이런 미리보기 항목은 아래 항목을 만들지 않습니다.

- `task_event` 또는 `task_events` 추가.
- 재실행 행 또는 `tool_invocations.response_json`.
- 생성된 지속 참조.
- `close_state` 변경.
- Write Check 변경.
- 스테이징 핸들 생성 또는 소비.
- 아티팩트 효과.
- 증거 업데이트 또는 증거 관찰.
- `CloseReadinessBlocker` 저장.
- `project_state.state_version` 증가.

## 읽기 전용 효과

읽기 전용 결과는 응답으로만 반환되며 재실행 행이 아닙니다.

응답 계산을 위해 `volicord.status`와 `volicord.close_task intent=check`는 메서드 담당 문서가 그 상태 보기를 선택할 때 `CurrentCloseBasis`, 닫기 상태, 위험 수락 범위, 차단 사유, `CloseReadinessBlocker[]`, 증거 요약, 아티팩트 참조, 프로젝트 연속성 요약, 진단, 다음 행동을 계산할 수 있습니다.

저장소는 읽기가 일어났다는 이유만으로 그 계산값을 지속 저장하면 안 됩니다.

읽는 시점의 상태 보기는 계산하지 않음, 사용할 수 없음, 비어 있음, 검증됨 상태를 구분해야 합니다. 저장소는 읽기 경로가 기반 사실을 계산하지 못했다는 이유로 빈 배열, 빈 해시, 0 크기, 만들어 낸 콘텐츠 타입, 더 강한 보장 표시를 쓰면 안 됩니다.

읽는 시점의 아티팩트 확인은 현재 본문을 저장된 사실과 대조해 검증할 수 없을 때 증거, 닫기, 상태 조회 출력용으로 유효한 missing, unavailable, integrity-failed 상태를 계산할 수 있습니다. 그 응답 계산은 별도의 담당 문서가 정의한 상태 변경이 일어나지 않는 한 `artifacts.status`, `artifacts.integrity_status`, 아티팩트 연결, 저장된 생명주기 행을 변경하지 않습니다.

`volicord.status`의 `close_blockers: CloseReadinessBlocker[]`는 읽기 전용 관찰입니다. 이 결과는 아래 항목을 만들지 않습니다.

- `task_event` 또는 `task_events` 추가
- 재실행 행 또는 `tool_invocations.response_json`
- `close_state` 변경
- Write Check 변경
- 스테이징 핸들 소비
- 아티팩트 효과
- 증거 업데이트 또는 증거 관찰
- `project_state.state_version` 증가

`volicord.close_task intent=check`의 응답 분기는 [`volicord.close_task`](api/method-close-task.md)가 담당합니다. 이 저장 효과 문서는 `dry_run=true`이거나 `blockers: CloseReadinessBlocker[]`를 포함하더라도 그 점검이 읽기 전용이라는 점만 담당합니다.

## 커밋된 차단 결과의 저장 효과

커밋된 차단 사유형 결과는 거부 응답이나 응답 전용 차단 결과와 다릅니다.

조건: 커밋된 차단 또는 비허용 결과는 관련 메서드 담당 문서가 그 결과에 대해
커밋 분기를 선택할 때만 `MethodResult`입니다.

담당 문서:
- [쓰기 준비 메서드](api/method-prepare-write.md)
- [Task 닫기 메서드](api/method-close-task.md)

<a id="volicordprepare_write-committed-non-allow-decision"></a>
### `volicord.prepare_write`의 커밋된 비허용 판단

조건:

- `dry_run=false`로 커밋되는 호출입니다.
- 결과가 `decision=blocked`, `decision=approval_required`, 또는 `decision=decision_required`입니다.

허용될 수 있는 효과:

- 구조화된 `write_decision_reasons: WriteDecisionReason[]`를 담은 `task_events` 이벤트를 정확히 하나 추가합니다.
- 멱등성 키가 있으면 재실행 행을 만듭니다.
- `project_state.state_version`을 정확히 한 번 증가시킵니다.
- 메서드가 소유한 판단과 `write_decision_reasons`를 응답과 재실행 페이로드에 기록합니다.

허용되지 않는 효과:

- 소비 가능한 Write Check 생성
- 별도 공개 이력 메서드 생성
- 과거 비허용 판단용 새 공개 응답 필드 추가
- `volicord.status`가 과거 비허용 판단을 노출해야 한다는 요구
- `close_state` 변경
- 닫기 준비 상태 평가
- `CloseReadinessBlocker` 저장
- 증거 업데이트 또는 증거 관찰
- 아티팩트 변경
- 스테이징 핸들 소비
- `close_task` 효과 적용

지속 저장 경계:

- 요청 측 `volicord.prepare_write` 페이로드 필드는 [`volicord.prepare_write` 참조](api/method-prepare-write.md)가 담당합니다.
- 저장된 `write_decision_reasons`는 `volicord.prepare_write` 판단 사유로 남습니다.
- 유효하게 커밋된 비허용 판단의 지속 감사 위치는 커밋된 태스크 이벤트와, 키가 있을 때의 재실행 행입니다.

저장된 사유는 아래 항목이 아닙니다.

- 닫기 차단 사유.
- `CloseReadinessBlocker[]`.
- 닫기 차단 사유 기록.

<a id="volicordclose_task-committed-blocked-result"></a>
### `volicord.close_task`의 커밋된 차단 결과

조건:

- 닫기 준비 상태 평가가 실행되었습니다.
- `volicord.close_task` 메서드 계약이 차단 결과 커밋을 허용합니다.

허용될 수 있는 효과:

- 차단 사유 상태.
- `task_events`.
- 재실행 행.
- `project_state.state_version` 증가.

이 결과에서도 `Task`는 열린 상태로 남습니다.

허용되지 않는 사용:

- 이 분기를 `STATE_VERSION_CONFLICT`에 사용
- `STATE_VERSION_CONFLICT`를 재실행으로 저장

`STATE_VERSION_CONFLICT`는 사전 확인의 `ToolRejectedResponse` 분기에 속합니다.

<a id="method-effects"></a>
## 메서드 저장 효과 요약

아래 표는 메서드별 지속 저장 효과를 요약합니다. 메서드 동작과 응답 공용체는 [API 메서드](api/methods.md)가 안내하는 메서드 담당 문서가 담당합니다.

| 메서드 | 주 저장 효과 | 세부사항 |
|---|---|---|
| `volicord.intake` | `Task`와 구체화 기록 생성 | [`volicord.intake`](#volicordintake) |
| `volicord.update_scope` | 현재 적용 범위 기록 갱신 | [`volicord.update_scope`](#volicordupdate_scope) |
| `volicord.status` | 읽기 전용 응답 | [`volicord.status`](#volicordstatus) |
| `volicord.prepare_write` | 쓰기 판단 효과 기록 | [`volicord.prepare_write`](#volicordprepare_write) |
| `volicord.stage_artifact` | 임시 스테이징만 생성 | [`volicord.stage_artifact`](#volicordstage_artifact) |
| `volicord.record_run` | 실행, 현재 닫기 근거, 증거, 증거 관찰 효과 기록 | [`volicord.record_run`](#volicordrecord_run) |
| `volicord.request_user_judgment` | 대기 중인 판단 요청 생성 | [`volicord.request_user_judgment`](#volicordrequest_user_judgment) |
| `volicord.record_user_judgment` | 사용자 판단 해결 | [`volicord.record_user_judgment`](#volicordrecord_user_judgment) |
| `volicord.reconcile_changes` | 미기록 변경 찾기를 해결하거나 대기 사용자 판단 생성 | [`volicord.reconcile_changes`](#volicordreconcile_changes) |
| `volicord.close_task intent=check` | 읽기 전용 닫기 준비 상태 점검 | [`volicord.close_task intent=check`](#volicordclose_task-intentcheck) |
| `volicord.close_task intent=complete` | 메서드가 선택한 `complete` 종료 또는 차단 효과 지속 | [`volicord.close_task intent=complete`](#volicordclose_task-intentcomplete) |
| `volicord.close_task intent=cancel` | 메서드가 선택한 취소 종료 또는 차단 효과 지속 | [`volicord.close_task intent=cancel`](#volicordclose_task-intentcancel) |
| `volicord.close_task intent=supersede` | 메서드가 선택한 대체 종료 또는 차단 효과 지속 | [`volicord.close_task intent=supersede`](#volicordclose_task-intentsupersede) |

<a id="volicordintake"></a>
### `volicord.intake`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- `Task`를 생성합니다.
- 선택적 Change Unit을 생성합니다.
- 구체화 기록을 생성합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run=true`
- 거절된 시도

이 분기는 `Task`, 참조, 이벤트, 재실행 행, `state_version` 증가를 만들지 않습니다.

담당 문서:

- [`volicord.intake` 메서드](api/method-intake.md)
- [저장소 기록](storage-records.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="volicordupdate_scope"></a>
### `volicord.update_scope`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 현재 적용 `Task` 범위 필드를 갱신합니다.
- 메서드 담당 문서가 제공한 효과 계약 JSON을 포함해 현재 적용 `change_units` 행을 만들거나 교체합니다.
- 현재 적용 범위나 현재 적용 Change Unit의 실질적 변경에 대해 `tasks.scope_revision`을 증가시킵니다.
- 실질적 범위 변경에 대해 `tasks.close_basis_json`을 무효화하고 `tasks.close_basis_revision`을 증가시킵니다.
- 담당 문서가 정의한 호환성에 따라 호환되지 않는 판단 근거 행을 오래됨 또는 대체됨으로 표시합니다.
- 메서드 담당 문서가 허용한 차단 사유 또는 오래된 Write Check 참조를 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run` 미리보기
- 거절된 시도

유효한 `dry_run` 미리보기는 범위, Change Unit, 차단 사유, 오래된 `Write Check` 효과만 미리 설명합니다.

의미가 같은 정규화된 갱신은 `tasks.scope_revision`을 증가시키거나 현재 닫기 근거를 무효화하지 않습니다.

담당 문서:

- [`volicord.update_scope` 메서드](api/method-update-scope.md)
- [저장소 기록](storage-records.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="volicordstatus"></a>
### `volicord.status`

읽기 전용 호출은 다음 특성을 가집니다.

- 응답 데이터만 반환합니다.
- 재실행 행을 만들지 않습니다.
- `project_continuity_records`를 만들지 않습니다.
- 저장소를 변경하지 않습니다.
- `project_state.state_version`을 증가시키지 않습니다.

`dry_run=true`도 `ToolDryRunResponse`가 아니라 `effect_kind=read_only`인 `StatusResult`로 유지됩니다.

효과가 없는 분기:

- 거절된 시도

담당 문서:

- [`volicord.status` 메서드](api/method-status.md)

<a id="volicordprepare_write"></a>
### `volicord.prepare_write`

`decision=allowed`인 재실행이 아닌 원래 커밋된 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 호환되는 `status=active` Write Check를 만듭니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

멱등 재실행은 [저장소 버전 관리](storage-versioning.md)에 따라 저장된 원래 응답을 반환하며, 이러한 효과를 반복하지 않습니다.

커밋되는 비허용 판단:

- [`volicord.prepare_write`의 커밋된 비허용 판단](#volicordprepare_write-committed-non-allow-decision)을 따릅니다.
- 태스크 이벤트를 정확히 하나 추가하고, 키가 있으면 재실행 행을 만들며, `project_state.state_version`을 정확히 한 번 증가시킵니다.
- 소비 가능한 Write Check, 별도 공개 이력 메서드, 새 공개 응답 필드를 만들지 않습니다.
- `volicord.status`는 과거 비허용 판단을 노출할 필요가 없습니다.

효과가 없는 분기:

- 거절된 시도
- 유효한 `dry_run` 미리보기

이 분기들은 아래 항목을 만들지 않습니다.

- 재실행 행.
- Write Check.
- 이벤트.
- `close_state` 변경.
- 아티팩트 또는 증거 효과.
- `project_state.state_version` 증가.

담당 문서:

- [`volicord.prepare_write` 메서드](api/method-prepare-write.md)
- [저장소 기록](storage-records.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="volicordstage_artifact"></a>
### `volicord.stage_artifact`

성공한 스테이징은 다음을 수행할 수 있습니다.

- `artifact_staging` 또는 동등한 저장소 소유 스테이징 기록을 생성합니다.
- `artifacts/tmp/` 아래에 임시 안전 바이트 또는 알림을 둡니다.

이 분기는 저장소 소유 임시 스테이징만 생성합니다. 일반 Core 커밋 변이 분기가 아니며, 임시 스테이징 디렉터리는 프로젝트 등록 시점이 아니라 스테이징이 일어날 때 만들어질 수 있습니다.

아래 항목은 만들지 않습니다.

- Core 현재 기록.
- 지속 `ArtifactRef`.
- 재실행 행.
- `project_state.state_version` 증가.

효과가 없는 분기:

- 유효한 `dry_run=true`
- 잘못된 스테이징 요청

유효한 `dry_run=true`는 아래 항목을 만들지 않습니다.

- 바이트.
- 스테이징 기록.
- `StagedArtifactHandle`.
- 재실행 행.
- `project_state.state_version` 증가.

담당 문서:

- [`volicord.stage_artifact` 메서드](api/method-stage-artifact.md)
- [아티팩트 저장소](storage-artifacts.md)

<a id="volicordrecord_run"></a>
### `volicord.record_run`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- `runs`를 생성합니다.
- 호환되는 `write_checks`를 소비합니다.
- 사용할 수 있는 `artifact_staging`을 소비합니다.
- `artifacts`를 승격하거나 연결합니다.
- `evidence_summaries`를 갱신하거나, `evidence_observations`를 생성하거나, 허용된 `blockers`를 갱신합니다.
- `close_assessment`에 따라 `tasks.close_basis_revision`과 `tasks.close_basis_json`을 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run` 미리보기
- 거절된 시도
- 커밋 전의 잘못된 스테이징 핸들

유효한 `dry_run` 미리보기는 아래 항목을 만들지 않습니다.

- `run_summary`.
- 현재 닫기 근거.
- 지속 잔여 위험 ID.
- 지속 아티팩트.
- 아티팩트 연결.
- 증거 갱신 또는 증거 관찰.
- 차단 사유 갱신.
- 이벤트.
- 재실행 행.
- 스테이징 핸들 소비.
- Write Check 소비.
- `project_state.state_version` 증가.

거절된 시도는 아래 항목을 바꾸지 않습니다.

- 스테이징 행.
- 아티팩트.

제품 파일 쓰기 지속 저장 경계:

- 메서드 담당 문서가 제품 파일 쓰기를 기록하는 커밋된 실행을 허용할 때, 저장소는 같은 커밋에서 호환되는 `write_checks` 행을 소비할 수 있습니다.
- 테스트 증거 지속 저장은 제품 파일 쓰기 관찰을 뜻하지 않으면서도 스테이징된 아티팩트를 승격하고 증거를 갱신하며 증거 관찰을 기록할 수 있습니다.
- 정확한 실행 분류는 [`volicord.record_run` 메서드](api/method-record-run.md)가 담당합니다.

현재 닫기 근거 지속 저장 경계:

- 커밋된 `volicord.record_run`은 `tasks.close_basis_revision`을 정확히 한 번 증가시킵니다.
- `close_assessment`가 `null`이 아니면 `tasks.close_basis_json`에 새 현재 `CurrentCloseBasis`를 쓰고 Core가 생성한 불투명 잔여 위험 ID를 저장합니다.
- 그 `CurrentCloseBasis`에 저장되는 민감 동작 요구사항은 커밋된 실행 기록과 소비된 Write Check에서 Core가 파생하며, 동작, 정규화된 경로, 민감 범주, 기준선, Change Unit, 출처 실행 기록 참조, 출처 Write Check 참조를 닫기까지 보존합니다.
- 범주만 담은 호출자 입력은 민감 동작 요구사항을 만들거나, 만족하거나, 지울 수 없습니다.
- `close_assessment=null`은 커밋된 실행 기록이 현재 닫기 근거를 만들지 않음을 기록합니다. 기존 현재 근거는 오래되거나 없어집니다.
- 실행 기록, 현재 닫기 근거, 증거 요약, 증거 관찰, 아티팩트, `Write Check` 소비, 재실행, 이벤트, 리비전 효과는 원자적으로 커밋됩니다.

담당 문서:

- [`volicord.record_run` 메서드](api/method-record-run.md)
- [아티팩트 저장소](storage-artifacts.md)
- [저장소 기록](storage-records.md)

<a id="volicordrequest_user_judgment"></a>
### `volicord.request_user_judgment`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 대기 중인 `user_judgments` 행을 생성합니다.
- Core가 파생한 판단 근거에 대해 `basis_json`과 `basis_status='current'`를 저장합니다.
- 영향받은 차단 사유를 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run` 미리보기
- 거절된 시도

유효한 `dry_run` 미리보기는 아래 항목을 만들지 않습니다.

- 실제 `user_judgment_ref`.
- 대기 중인 판단.
- 차단 사유 갱신.
- 이벤트.
- 재실행 행.
- `project_state.state_version` 증가.

담당 문서:

- [`volicord.request_user_judgment` 메서드](api/method-request-user-judgment.md#volicordrequest_user_judgment)
- [저장소 기록](storage-records.md)

<a id="volicordrecord_user_judgment"></a>
### `volicord.record_user_judgment`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- `user_judgments` 행을 `status='resolved'`로 설정합니다.
- 선택된 선택지, `resolution_machine_action`, `resolution_outcome`, 파생된 해결 행위자 출처, 답변 본문, 설명용 판단 이유 메타데이터, 근거 상태를 메서드 담당 문서가 허용한 대로 저장합니다.
- 메서드 담당 문서가 선택할 때 수락된 제품, 기술, 범위 결정과 수락된 현재 잔여 위험에 대한 `project_continuity_records`를 생성합니다.
- 종속 차단 사유 또는 다음 행동을 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run` 미리보기
- 거절된 시도

유효한 `dry_run` 미리보기는 아래 항목을 만들지 않습니다.

- 판단 해결.
- 프로젝트 연속성 기록.
- 차단 사유 갱신.
- 이벤트.
- 재실행 행.
- `project_state.state_version` 증가.

사용자 판단 기록은 `tasks.scope_revision`이나 `tasks.close_basis_revision`을 증가시키지 않습니다.

`status='resolved'`는 답변이 기록되었다는 뜻이며 그 자체로 수락이 아닙니다. 현재 해결 행에는 완전한 근거, 선택된 동작, `resolution_outcome`, 해결 요청 본문, 해결 타임스탬프, User Channel 행위자 출처, 검증 근거, 보증 수준, 필요한 행위자 출처가 있어야 합니다. 필요한 해결 권한 정보가 빠진 행은 읽을 수 있는 이력 감사 판단이 아니라 유효하지 않은 저장 상태입니다.

담당 문서:

- [`volicord.record_user_judgment` 메서드](api/method-record-user-judgment.md#volicordrecord_user_judgment)
- [저장소 기록](storage-records.md)

<a id="volicordreconcile_changes"></a>
### `volicord.reconcile_changes`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 미해결 `unrecorded_changes` 행을 `status='resolved'`로 설정합니다.
- resolution basis, capture basis, 해결 메서드, 선택적 연결 사용자 판단 참조를 이름 붙이는 resolution JSON을 저장합니다.
- `resolved_at`과 `resolved_by_actor_source`를 저장합니다.
- 사용자 수락이 필요한 찾기에 대해 대기 `user_judgments` 행을 만듭니다.
- 이벤트를 추가합니다.
- idempotency key가 있으면 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

읽기 전용 분기:

- 계획된 해결이나 대기 판단 생성이 없는 유효한 호출은 응답 데이터만 반환합니다.

효과가 없는 분기:

- 거절된 시도
- 유효한 `dry_run` 미리보기

이 분기들은 찾기를 해결하거나, 대기 판단을 만들거나, 이벤트를 추가하거나, 재실행 행을 만들거나, `project_state.state_version`을 증가시키지 않습니다.

조정 효과는 제품 정확성, 테스트 충분성, 리뷰 완료, 최종 수락, 잔여 위험 수락, 보안을 증명하지 않습니다. 미기록 변경 찾기가 더 이상 미해결이 아닌 이유를 기록하거나, 남은 수락을 위한 대기 사용자 소유 판단을 만들 뿐입니다.

담당 문서:

- [`volicord.reconcile_changes` 메서드](api/method-reconcile-changes.md#volicordreconcile_changes)
- [저장소 기록](storage-records.md)

<a id="volicordclose_task-intentcheck"></a>
### `volicord.close_task intent=check`

읽기 전용 호출은 다음 특성을 가집니다.

- 계산된 닫기 준비 상태를 반환합니다.
- `volicord.status include.close=true`와 같은 닫기 준비 상태 계산을 사용합니다.
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

- [`volicord.close_task` 메서드](api/method-close-task.md)

<a id="volicordclose_task-intentcomplete"></a>
### `volicord.close_task intent=complete`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 메서드가 선택한 완료 종료 효과를 지속합니다.
- 메서드가 선택한 완료 효과가 성공하면 `tasks.close_basis_json`과 별개인 종료 닫기 요약을 지속할 수 있습니다.
- 메서드가 선택한 완료 효과가 성공하면 보이지만 잔여 위험 수락이 필요하지 않은 현재 닫기 근거 잔여 위험에 대해 `kind='known_limit'`인 `project_continuity_records`를 생성합니다.
- `Task`를 열린 상태로 둔 채 담당 문서가 허용한 `complete` 차단 효과를 지속합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run=true`
- 사전 확인 실패

유효한 `dry_run=true`는 `ToolDryRunResponse`를 반환합니다. 사전 확인 실패는 효과가 없는 `ToolRejectedResponse`입니다.

담당 문서:

- [`volicord.close_task` 메서드](api/method-close-task.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="volicordclose_task-intentcancel"></a>
### `volicord.close_task intent=cancel`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 메서드가 선택한 취소 효과를 지속합니다.
- `Task`를 열린 상태로 둔 채 담당 문서가 허용한 취소 차단 효과를 지속합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run=true`
- 사전 확인 실패

유효한 `dry_run=true`는 `ToolDryRunResponse`를 반환합니다.

취소 효과에는 `machine_action=accept`, `resolution_outcome=accepted`, 호환되는 근거, `resolved_by_actor_source=local_user`, 호환 User Channel 출처를 가진 메서드 담당 현재 취소 판단이 필요합니다. 취소 권한이 없거나 호환되지 않으면 담당 문서가 허용한 차단된 취소 효과를 만들 수 있지만, 수락이나 완료 전용 닫기 증거를 만들어 내면 안 됩니다.

담당 문서:

- [`volicord.close_task` 메서드](api/method-close-task.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="volicordclose_task-intentsupersede"></a>
### `volicord.close_task intent=supersede`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 메서드가 선택한 대체 효과를 지속합니다.
- 메서드가 선택한 효과에 필요하면 같은 변경에서 `project_state.active_task_id`를 갱신합니다.
- 담당 문서가 허용한 대체 차단 효과를 지속합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run=true`
- 사전 확인 실패

유효한 `dry_run=true`는 `ToolDryRunResponse`를 반환합니다.

담당 문서:

- [`volicord.close_task` 메서드](api/method-close-task.md)
- [저장소 버전 관리](storage-versioning.md)

## 관련 담당 문서

- [API 메서드](api/methods.md)와 메서드 담당 문서: 선택된 메서드 동작과 응답 공용체.
- [API 오류 처리 경로](api/error-routing.md), [API 오류 코드](api/error-codes.md): 거부 응답의 공개 오류.
- [저장소 기록](storage-records.md): 저장 효과가 건드릴 수 있는 기록.
- [아티팩트 저장소](storage-artifacts.md): 스테이징 핸들과 아티팩트 생명주기 세부사항.
- [저장소 버전 관리](storage-versioning.md): `state_version` 시계와 재실행/멱등성 의미.
