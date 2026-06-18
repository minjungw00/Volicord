# 저장 효과

이 문서는 기준 범위 원천 설계에서 메서드와 응답 분기가 어떤 저장 효과를 만들 수 있는지 담당합니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 읽기 전용, `dry_run`, 거부 응답, 스테이징 생성, Core 커밋, 커밋된 차단 결과의 저장 효과 구분.
- 각 분기가 담당 기록, `task_events`, 재실행 행, `project_state.state_version`, 스테이징 핸들 생성 또는 소비, 아티팩트 승격, `Write Authorization`을 바꿀 수 있는지 여부.
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

| 분기 | 요약 | 세부사항 |
|---|---|---|
| 읽기 전용 `MethodResult` | 응답만 반환 | [읽기 전용 결과](#read-only-result) |
| `ToolRejectedResponse` | 저장 효과 없음 | [`ToolRejectedResponse`](#toolrejectedresponse-effect) |
| 유효한 `ToolDryRunResponse` | 미리보기만 반환 | [유효한 `dry_run` 미리보기](#valid-dry-run-preview) |
| `StageArtifactResult`, `effect_kind=staging_created` | 임시 스테이징만 생성 | [스테이징 생성 아티팩트 결과](#staging-created-artifact-result) |
| Core 커밋 `MethodResult` | 메서드 담당 커밋 효과 | [Core 커밋 결과](#core-committed-result) |
| 커밋된 차단 결과 `MethodResult` | 명시적으로 허용된 차단 효과만 | [커밋된 차단 결과](#committed-blocked-result) |

<a id="read-only-result"></a>
### 읽기 전용 결과

저장 효과:

- 응답만 반환합니다.

허용되지 않는 효과:

- 재실행 행
- 이벤트
- 담당 기록 변경
- 아티팩트 효과
- `Write Authorization` 효과
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
- `Write Authorization` 생성 또는 소비
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

## `dry_run` 미리보기 효과

유효한 `dry_run` 미리보기는 `DryRunSummary.would_blockers: PlannedBlocker[]` 또는 계획된 효과를 포함할 수 있습니다. 이런 미리보기 항목은 아래 항목을 만들지 않습니다.

- `task_event` 또는 `task_events` 추가.
- 재실행 행 또는 `tool_invocations.response_json`.
- 생성된 지속 참조.
- `close_state` 변경.
- `Write Authorization` 변경.
- 스테이징 핸들 생성 또는 소비.
- 아티팩트 효과.
- 증거 업데이트.
- `CloseReadinessBlocker` 저장.
- `project_state.state_version` 증가.

## 읽기 전용 효과

읽기 전용 결과는 응답으로만 반환되며 재실행 행이 아닙니다.

응답 계산을 위해 `harness.status`와 `harness.close_task intent=check`는 차단 사유, `CloseReadinessBlocker[]`, 증거 요약, 아티팩트 참조, 진단, 다음 행동을 계산할 수 있습니다.

저장소는 읽기가 일어났다는 이유만으로 그 계산값을 지속 저장하면 안 됩니다.

`harness.status`의 `close_blockers: CloseReadinessBlocker[]`는 읽기 전용 관찰입니다. 이 결과는 아래 항목을 만들지 않습니다.

- `task_event` 또는 `task_events` 추가
- 재실행 행 또는 `tool_invocations.response_json`
- `close_state` 변경
- `Write Authorization` 변경
- 스테이징 핸들 소비
- 아티팩트 효과
- 증거 업데이트
- `project_state.state_version` 증가

`harness.close_task intent=check`의 응답 분기는 [`harness.close_task`](api/method-close-task.md)가 담당합니다. 이 저장 효과 문서는 `dry_run=true`이거나 `blockers: CloseReadinessBlocker[]`를 포함하더라도 그 점검이 읽기 전용이라는 점만 담당합니다.

## 커밋된 차단 결과의 저장 효과

커밋된 차단 결과는 거부 응답과 다릅니다.

조건: `harness.prepare_write` 또는 `harness.close_task`의 커밋된 차단 결과는 관련 메서드 담당 문서가 차단 커밋을 허용할 때만 `MethodResult`입니다.

담당 문서:
- [쓰기 준비 메서드](api/method-prepare-write.md)
- [Task 닫기 메서드](api/method-close-task.md)

<a id="harnessprepare_write-committed-non-allow-decision"></a>
### `harness.prepare_write`의 커밋된 비허용 판단

조건:

- `dry_run=false`로 커밋되는 호출입니다.
- 결과가 `decision=blocked`, `decision=approval_required`, 또는 `decision=decision_required`입니다.

허용될 수 있는 효과:

- 구조화된 `write_decision_reasons: WriteDecisionReason[]`를 담은 `task_events` 이벤트를 정확히 하나 추가합니다.
- 멱등성 키가 있으면 재실행 행을 만듭니다.
- `project_state.state_version`을 정확히 한 번 증가시킵니다.
- 메서드가 소유한 판단과 `write_decision_reasons`를 응답과 재실행 페이로드에 기록합니다.

허용되지 않는 효과:

- 소비 가능한 `Write Authorization` 생성
- 별도 공개 이력 메서드 생성
- 과거 비허용 판단용 새 공개 응답 필드 추가
- `harness.status`가 과거 비허용 판단을 노출해야 한다는 요구
- `close_state` 변경
- 닫기 준비 상태 평가
- `CloseReadinessBlocker` 저장
- 증거 업데이트
- 아티팩트 변경
- 스테이징 핸들 소비
- `close_task` 효과 적용

지속 저장 경계:

- 요청 측 `harness.prepare_write` 페이로드 필드는 [`harness.prepare_write` 참조](api/method-prepare-write.md)가 담당합니다.
- 저장된 `write_decision_reasons`는 `harness.prepare_write` 판단 사유로 남습니다.
- 유효하게 커밋된 비허용 판단의 지속 감사 위치는 커밋된 태스크 이벤트와, 키가 있을 때의 재실행 행입니다.

저장된 사유는 아래 항목이 아닙니다.

- 닫기 차단 사유.
- `CloseReadinessBlocker[]`.
- 닫기 차단 사유 기록.

<a id="harnessclose_task-committed-blocked-result"></a>
### `harness.close_task`의 커밋된 차단 결과

조건:

- 닫기 준비 상태 평가가 실행되었습니다.
- `harness.close_task` 메서드 계약이 차단 결과 커밋을 허용합니다.

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
| `harness.intake` | `Task`와 구체화 기록 생성 | [`harness.intake`](#harnessintake) |
| `harness.update_scope` | 현재 적용 범위 기록 갱신 | [`harness.update_scope`](#harnessupdate_scope) |
| `harness.status` | 읽기 전용 응답 | [`harness.status`](#harnessstatus) |
| `harness.prepare_write` | 쓰기 판단 효과 기록 | [`harness.prepare_write`](#harnessprepare_write) |
| `harness.stage_artifact` | 임시 스테이징만 생성 | [`harness.stage_artifact`](#harnessstage_artifact) |
| `harness.record_run` | 실행과 증거 효과 기록 | [`harness.record_run`](#harnessrecord_run) |
| `harness.request_user_judgment` | 대기 중인 판단 요청 생성 | [`harness.request_user_judgment`](#harnessrequest_user_judgment) |
| `harness.record_user_judgment` | 사용자 판단 해결 | [`harness.record_user_judgment`](#harnessrecord_user_judgment) |
| `harness.close_task intent=check` | 읽기 전용 닫기 준비 상태 점검 | [`harness.close_task intent=check`](#harnessclose_task-intentcheck) |
| `harness.close_task intent=complete` | 메서드가 선택한 `complete` 종료 또는 차단 효과 지속 | [`harness.close_task intent=complete`](#harnessclose_task-intentcomplete) |
| `harness.close_task intent=cancel` | 메서드가 선택한 취소 종료 또는 차단 효과 지속 | [`harness.close_task intent=cancel`](#harnessclose_task-intentcancel) |
| `harness.close_task intent=supersede` | 메서드가 선택한 대체 종료 또는 차단 효과 지속 | [`harness.close_task intent=supersede`](#harnessclose_task-intentsupersede) |

<a id="harnessintake"></a>
### `harness.intake`

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

- [`harness.intake` 메서드](api/method-intake.md)
- [저장소 기록](storage-records.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="harnessupdate_scope"></a>
### `harness.update_scope`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 현재 적용 `Task` 범위 필드를 갱신합니다.
- 현재 적용 `change_units` 행을 만들거나 교체합니다.
- 메서드 담당 문서가 허용한 차단 사유 또는 오래된 `Write Authorization` 참조를 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run` 미리보기
- 거절된 시도

유효한 `dry_run` 미리보기는 범위, Change Unit, 차단 사유, 오래된 승인 효과만 미리 설명합니다.

담당 문서:

- [`harness.update_scope` 메서드](api/method-update-scope.md)
- [저장소 기록](storage-records.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="harnessstatus"></a>
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

- [`harness.status` 메서드](api/method-status.md)

<a id="harnessprepare_write"></a>
### `harness.prepare_write`

`decision=allowed`인 재실행이 아닌 원래 커밋된 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 호환되는 `status=active` `Write Authorization`을 만듭니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

멱등 재실행은 [저장소 버전 관리](storage-versioning.md)에 따라 저장된 원래 응답을 반환하며, 이러한 효과를 반복하지 않습니다.

커밋되는 비허용 판단:

- [`harness.prepare_write`의 커밋된 비허용 판단](#harnessprepare_write-committed-non-allow-decision)을 따릅니다.
- 태스크 이벤트를 정확히 하나 추가하고, 키가 있으면 재실행 행을 만들며, `project_state.state_version`을 정확히 한 번 증가시킵니다.
- 소비 가능한 `Write Authorization`, 별도 공개 이력 메서드, 새 공개 응답 필드를 만들지 않습니다.
- `harness.status`는 과거 비허용 판단을 노출할 필요가 없습니다.

효과가 없는 분기:

- 거절된 시도
- 유효한 `dry_run` 미리보기

이 분기들은 아래 항목을 만들지 않습니다.

- 재실행 행.
- `Write Authorization`.
- 이벤트.
- `close_state` 변경.
- 아티팩트 또는 증거 효과.
- `project_state.state_version` 증가.

담당 문서:

- [`harness.prepare_write` 메서드](api/method-prepare-write.md)
- [저장소 기록](storage-records.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="harnessstage_artifact"></a>
### `harness.stage_artifact`

성공한 스테이징은 다음을 수행할 수 있습니다.

- `artifact_staging` 또는 동등한 저장소 소유 스테이징 기록을 생성합니다.
- `artifacts/tmp/` 아래에 임시 안전 바이트 또는 알림을 둡니다.

이 분기는 저장소 소유 임시 스테이징만 생성합니다.

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

- [`harness.stage_artifact` 메서드](api/method-stage-artifact.md)
- [아티팩트 저장소](storage-artifacts.md)

<a id="harnessrecord_run"></a>
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

- 유효한 `dry_run` 미리보기
- 거절된 시도
- 커밋 전의 잘못된 스테이징 핸들

유효한 `dry_run` 미리보기는 아래 항목을 만들지 않습니다.

- `run_summary`.
- 지속 아티팩트.
- 아티팩트 연결.
- 증거 갱신.
- 차단 사유 갱신.
- 이벤트.
- 재실행 행.
- 스테이징 핸들 소비.
- `Write Authorization` 소비.
- `project_state.state_version` 증가.

거절된 시도는 아래 항목을 바꾸지 않습니다.

- 스테이징 행.
- 아티팩트.

제품 파일 쓰기 지속 저장 경계:

- 메서드 담당 문서가 제품 파일 쓰기를 기록하는 커밋된 실행을 허용할 때, 저장소는 같은 커밋에서 호환되는 `write_authorizations` 행을 소비할 수 있습니다.
- 테스트 증거 지속 저장은 제품 파일 쓰기 관찰을 뜻하지 않으면서도 스테이징된 아티팩트를 승격하고 증거를 갱신할 수 있습니다.
- 정확한 실행 분류는 [`harness.record_run` 메서드](api/method-record-run.md)가 담당합니다.

담당 문서:

- [`harness.record_run` 메서드](api/method-record-run.md)
- [아티팩트 저장소](storage-artifacts.md)
- [저장소 기록](storage-records.md)

<a id="harnessrequest_user_judgment"></a>
### `harness.request_user_judgment`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 대기 중인 `user_judgments` 행을 생성합니다.
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

- [`harness.request_user_judgment` 메서드](api/method-request-user-judgment.md#harnessrequest_user_judgment)
- [저장소 기록](storage-records.md)

<a id="harnessrecord_user_judgment"></a>
### `harness.record_user_judgment`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- `user_judgments` 행을 해결합니다.
- 종속 차단 사유 또는 다음 행동을 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run` 미리보기
- 거절된 시도

유효한 `dry_run` 미리보기는 아래 항목을 만들지 않습니다.

- 판단 해결.
- 차단 사유 갱신.
- 이벤트.
- 재실행 행.
- `project_state.state_version` 증가.

담당 문서:

- [`harness.record_user_judgment` 메서드](api/method-record-user-judgment.md#harnessrecord_user_judgment)
- [저장소 기록](storage-records.md)

<a id="harnessclose_task-intentcheck"></a>
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

- [`harness.close_task` 메서드](api/method-close-task.md)

<a id="harnessclose_task-intentcomplete"></a>
### `harness.close_task intent=complete`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 메서드가 선택한 완료 종료 효과를 지속합니다.
- `Task`를 열린 상태로 둔 채 담당 문서가 허용한 `complete` 차단 효과를 지속합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run=true`
- 사전 확인 실패

유효한 `dry_run=true`는 `ToolDryRunResponse`를 반환합니다. 사전 확인 실패는 효과가 없는 `ToolRejectedResponse`입니다.

담당 문서:

- [`harness.close_task` 메서드](api/method-close-task.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="harnessclose_task-intentcancel"></a>
### `harness.close_task intent=cancel`

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

담당 문서:

- [`harness.close_task` 메서드](api/method-close-task.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="harnessclose_task-intentsupersede"></a>
### `harness.close_task intent=supersede`

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

- [`harness.close_task` 메서드](api/method-close-task.md)
- [저장소 버전 관리](storage-versioning.md)

## 관련 담당 문서

- [API 메서드](api/methods.md)와 메서드 담당 문서: 선택된 메서드 동작과 응답 공용체.
- [API 오류 처리 경로](api/error-routing.md), [API 오류 코드](api/error-codes.md): 거부 응답의 공개 오류.
- [저장소 기록](storage-records.md): 저장 효과가 건드릴 수 있는 기록.
- [아티팩트 저장소](storage-artifacts.md): 스테이징 핸들과 아티팩트 생명주기 세부사항.
- [저장소 버전 관리](storage-versioning.md): `state_version` 시계와 재실행/멱등성 의미.
