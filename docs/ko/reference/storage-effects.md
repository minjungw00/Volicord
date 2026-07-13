# 저장 효과

이 문서는 기준 범위에서 메서드와 응답 분기가 만들 수 있는 저장 효과를 정의합니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 읽기 전용, `dry_run`, 거부 응답, 스테이징 생성, Core 커밋, 커밋된 차단 결과의 저장 효과 구분.
- 각 분기가 담당 기록, `authority_events`, 재실행 행, `project_state.state_version`, 스테이징 핸들 생성 또는 소비, 아티팩트 승격, 쓰기 티켓 호환성을 바꿀 수 있는지 여부.
- 차단 사유형 응답 데이터가 지속 저장되는 경계.
- 거부 응답과 유효한 `dry_run` 미리보기의 효과 없음 보장.

이 문서는 담당하지 않습니다.

- 기록 계열 개요: [저장소 기록](storage-records.md)을 봅니다.
- 기준 SQLite DDL, 제약, 인덱스, 외래 키, 기준 SQL 원본: [저장소 DDL](storage-ddl.md)을 봅니다.
- 아티팩트 생명주기 세부사항; [아티팩트 저장소](storage-artifacts.md)를 봅니다.
- 멱등성, 잠금, `state_version` 시계, 이벤트 순서, 호환되지 않는 저장소 처리; [저장소 버전 관리](storage-versioning.md)를 봅니다.
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

비주장: 응답에 이런 값이 있다는 사실만으로 영속 저장, 아티팩트 승격, 스테이징 핸들 소비, 재실행 저장, `close_state` 변경, `project_state.state_version` 증가가 증명되지는 않습니다.

효과는 선택된 메서드 동작과 응답 분기가 정합니다. 아래 표는 각 분기를 짧게 요약하고, 세부 블록은 허용될 수 있는 효과와 허용되지 않는 효과를 나누어 설명합니다.

| 효과 범주 | 응답 또는 분기 | 지속 저장 결과 | 세부사항 |
|---|---|---|---|
| 읽기 전용 | 읽기 전용 `MethodResult` | Core 권한 상태 변경은 없습니다. 메서드가 허용한 세션 감시 진단 기록을 제외하면 응답 데이터만 반환합니다. 재실행 행, 권한 이벤트, 아티팩트 효과, 쓰기 티켓 효과, 닫기 상태 변경, `project_state.state_version` 증가는 없습니다. | [읽기 전용 결과](#read-only-result) |
| 효과 없음 | `ToolRejectedResponse` 또는 `effect_kind=no_effect`인 유효한 `MethodResult` | 요청된 일반 변이가 없고 Core 커밋도 없습니다. 응답이 오류나 차단 사유형 데이터를 담을 수 있지만, 이 분기는 그 값을 지속하지 않습니다. | [`ToolRejectedResponse`](#toolrejectedresponse-effect), [효과가 없는 분기](#no-effect-branches) |
| `dry_run` | 유효한 `ToolDryRunResponse` | 미리보기만 반환합니다. 영속 참조, 재실행 행, 이벤트, 스테이징 핸들, 아티팩트 효과, `project_state.state_version` 증가는 없습니다. | [유효한 `dry_run` 미리보기](#valid-dry-run-preview) |
| 스테이징 생성 | `effect_kind=staging_created`인 `StageArtifactResult` | 저장소 소유 임시 스테이징만 생성합니다. 일반 Core 커밋 트랜잭션이 아닙니다. | [스테이징 생성 아티팩트 결과](#staging-created-artifact-result) |
| Core 커밋 | Core 커밋 `MethodResult` | `CoreProjectStore::commit_mutation`을 통해 메서드 담당 효과를 만듭니다. 상태 버전 증가, 권한 이벤트, 선택적 재실행 행, 메서드가 선택한 `CoreStorageMutation` 값이 포함됩니다. | [Core 커밋 결과](#core-committed-result) |
| 커밋된 차단 사유형 결과 | 메서드 담당 문서가 차단 또는 비허용 지속 저장을 허용한 커밋 `MethodResult` | 명시적으로 허용된 이벤트, 재실행, 상태 버전, 차단 사유 상태 효과만 만듭니다. 차단 사유형 응답만으로는 충분하지 않습니다. | [커밋된 차단 결과](#committed-blocked-result) |

<a id="read-only-result"></a>
### 읽기 전용 결과

저장 효과:

- Core 권한 상태 저장 효과가 없습니다.
- 메서드가 허용한 세션 감시 진단 기록을 제외하면 응답 데이터만 반환합니다.

허용되지 않는 효과:

- 재실행 행
- 권한 이벤트
- Core 현재 기록 변경
- 닫기 상태 변경
- 아티팩트 효과
- 증거 업데이트 또는 증거 관찰
- 쓰기 티켓 효과
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
- 쓰기 티켓 생성 또는 소비
- `project_state.state_version` 증가

<a id="valid-dry-run-preview"></a>
### 유효한 `dry_run` 미리보기

저장 효과:

- 응답 미리보기만 반환합니다.

허용되지 않는 효과:

- 담당 기록 생성 또는 변경
- 생성된 영속 참조
- 재실행 행
- 이벤트
- 스테이징 핸들 생성
- 아티팩트 승격 또는 연결
- `project_state.state_version` 증가

<a id="staging-created-artifact-result"></a>
### 스테이징 생성 아티팩트 결과

허용될 수 있는 효과:

- 저장소 소유 임시 스테이징

이 분기는 일반 Core 커밋 변이와 별개입니다. 저장소가 관리하는 스테이징 표현이나 핸들을 만들 수 있지만, 그 임시 스테이징 쓰기 자체가 Core 현재 기록 변경, 영속 `ArtifactRef`, 아티팩트 연결, 증거 기록은 아닙니다.

허용되지 않는 효과:

- Core 현재 기록
- 재실행 행
- 이벤트
- 영속 `ArtifactRef`
- `project_state.state_version` 증가

<a id="core-committed-result"></a>
### Core 커밋 결과

조건:

- 메서드 담당 문서가 커밋 효과를 허용합니다.

허용될 수 있는 효과:

- 담당 기록 변경
- `authority_events` 추가
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
- 보호된 동작이 진행되기 전의 연결 처리 경로 또는 모드 관문 실패.
- 오래된 `expected_state_version`.
- 오래된 `WriteTicket.basis_state_version`.
- 멱등 요청 해시 충돌.
- 거절된 아티팩트 입력.

효과가 없는 분기는 아래 항목을 만들거나 바꾸면 안 됩니다.

- 담당 기록.
- `authority_events` 추가.
- `tool_invocations.response_json`.
- 재실행 행.
- 증거 요약 또는 증거 관찰.
- `close_state`.
- 쓰기 티켓 호환성 행 생성 또는 소비.
- `artifact_staging.status`.
- `consumed_by_run_id` 또는 `promoted_artifact_id`.
- 아티팩트 승격 또는 연결.
- `project_state.state_version` 증가.

사전 확인에서 `ToolRejectedResponse`가 반환되면 요청된 커밋 동작은 수행되지 않습니다. 이 원칙은 `dry_run` 요청에도 똑같이 적용됩니다. `dry_run`은 검증, 접근, 역량, 오래된 상태 거절을 우회하지 않습니다.

메서드 담당 문서가 응답 전용 차단 분기를 선택하면 유효한 차단 결과도 효과가
없을 수 있습니다. 예를 들어 기준 `volicord.close_task`의 차단된 종료 시도는
`CloseTaskResult` 데이터를 반환하지만 차단 사유 행, 권한 이벤트, 재실행 행,
상태 버전 증가를 커밋하지 않습니다. 이 경로는 커밋되는 비허용
`volicord.prepare_write` 결과와 별개입니다.

## `dry_run` 미리보기 효과

유효한 `dry_run` 미리보기는 `DryRunSummary.would_blockers: PlannedBlocker[]` 또는 계획된 효과를 포함할 수 있습니다. 이런 미리보기 항목은 아래 항목을 만들지 않습니다.

- `authority_events` 추가.
- 재실행 행 또는 `tool_invocations.response_json`.
- 생성된 영속 참조.
- `close_state` 변경.
- 쓰기 티켓 변경.
- 스테이징 핸들 생성 또는 소비.
- 아티팩트 효과.
- 증거 업데이트 또는 증거 관찰.
- `CloseReadinessBlocker` 저장.
- `project_state.state_version` 증가.

## 읽기 전용 효과

읽기 전용 결과에는 Core 권한 상태 저장 효과가 없고 재실행 행도 아닙니다. 아래
메서드 절이 세션에 연결된 Agent Connection의 세션 감시 진단 기록을 명시적으로
허용하지 않는 한, 읽기 전용 결과는 응답으로만 반환됩니다. 허용된 진단 기록은
Core 상태 변경, 재실행 행, 권한 이벤트, 닫기 상태 변경,
`project_state.state_version` 변경이 아닙니다.

응답 계산을 위해 `volicord.status`와 `volicord.check_close`는 메서드 담당 문서가 그 상태 보기를 선택할 때 `CurrentCloseBasis`, 닫기 상태, 위험 수락 범위, 차단 사유, `CloseReadinessBlocker[]`, 증거 요약, 아티팩트 참조, 프로젝트 연속성 요약, 진단, 다음 행동을 계산할 수 있습니다.

저장소는 읽기가 일어났다는 이유만으로 그 계산값을 지속 저장하면 안 됩니다.

읽는 시점의 상태 보기는 계산하지 않음, 사용할 수 없음, 비어 있음, 검증됨 상태를 구분해야 합니다. 저장소는 읽기 경로가 기반 사실을 계산하지 못했다는 이유로 빈 배열, 빈 해시, 0 크기, 만들어 낸 콘텐츠 타입, 더 강한 보장 표시를 쓰면 안 됩니다.

읽는 시점의 아티팩트 확인은 현재 본문을 저장된 사실과 대조해 검증할 수 없을 때 증거, 닫기, 상태 조회 출력용으로 유효한 `missing`, `unavailable`, `integrity_failed` 상태를 계산할 수 있습니다. 그 응답 계산은 별도의 담당 문서가 정의한 상태 변경이 일어나지 않는 한 `artifacts.status`, `artifacts.integrity_status`, 아티팩트 연결, 저장된 생명주기 행을 변경하지 않습니다.

`volicord.status`의 `close_blockers: CloseReadinessBlocker[]`는 읽기 전용 관찰입니다. 이 결과는 아래 항목을 만들지 않습니다.

- `authority_events` 추가
- 재실행 행 또는 `tool_invocations.response_json`
- `close_state` 변경
- 쓰기 티켓 변경
- 스테이징 핸들 소비
- 아티팩트 효과
- 증거 업데이트 또는 증거 관찰
- `project_state.state_version` 증가

`volicord.check_close`의 응답 분기는 [`volicord.check_close`](api/method-close-task.md#volicordcheck_close)가 담당합니다. 이 저장 효과 문서는 `dry_run=true`이거나 `blockers: CloseReadinessBlocker[]`를 포함하더라도 그 점검이 Core 권한 상태와 `project_state.state_version`에 대해 읽기 전용이라는 점을 담당합니다.

세션 감시 진단 기록은 [저장소 기록](storage-records.md)이 설명하는 제한된
스냅샷 메타데이터만 저장합니다. 원본 파일 내용, 민감한 프롬프트 텍스트, 파일
변경에서 추론한 행위자 식별 정보, Volicord가 파일시스템 쓰기를 막았다는 주장을
저장하면 안 됩니다. 읽기 또는 점검 경계가 첫 감시기 기준선을 만들면 기준선
메타데이터는 감시 시작 시각과 `method_boundary` 감시 근거를 부분 감시 경고와
함께 기록합니다. 그 경계 전의 Product Repository 변경은 감시 범위 밖에 있습니다.

## 커밋된 차단 결과의 저장 효과

커밋된 차단 사유형 결과는 거부 응답이나 응답 전용 차단 결과와 다릅니다.

조건: 커밋된 차단 또는 비허용 결과는 관련 메서드 담당 문서가 그 결과에 대해
커밋 분기를 선택할 때만 `MethodResult`입니다.

담당 문서:
- [쓰기 준비 메서드](api/method-prepare-write.md)

<a id="volicordprepare_write-committed-non-allow-decision"></a>
### `volicord.prepare_write`의 커밋된 비허용 판단

조건:

- `dry_run=false`로 커밋되는 호출입니다.
- 결과가 `decision=blocked`, `decision=approval_required`, 또는 `decision=decision_required`입니다.

허용될 수 있는 효과:

- 구조화된 `write_decision_reasons: WriteDecisionReason[]`를 담은 `authority_events` 이벤트를 정확히 하나 추가합니다.
- 멱등성 키가 있으면 재실행 행을 만듭니다.
- `project_state.state_version`을 정확히 한 번 증가시킵니다.
- 메서드가 소유한 판단과 `write_decision_reasons`를 응답과 재실행 페이로드에 기록합니다.

허용되지 않는 효과:

- 쓰기 티켓 발급
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
- 유효하게 커밋된 비허용 판단의 지속 로컬 감사 위치는 커밋된 권한 이벤트와, 키가 있을 때의 재실행 행입니다.

저장된 사유는 아래 항목이 아닙니다.

- 닫기 차단 사유.
- `CloseReadinessBlocker[]`.
- 닫기 차단 사유 기록.

<a id="method-effects"></a>
## 메서드 저장 효과 요약

아래 표는 메서드별 지속 저장 효과를 요약합니다. 메서드 동작과 응답 공용체는 [API 메서드](api/methods.md)가 안내하는 메서드 담당 문서가 담당합니다.

| 메서드 | 주 저장 효과 | 세부사항 |
|---|---|---|
| `volicord.intake` | `Task`와 구체화 기록 생성 | [`volicord.intake`](#volicordintake) |
| `volicord.update_scope` | 현재 적용 범위 기록 갱신 | [`volicord.update_scope`](#volicordupdate_scope) |
| `volicord.status` | 선택적 세션 감시 초기화를 포함하는 읽기형 응답 | [`volicord.status`](#volicordstatus) |
| `volicord.get_operation_result` | 저장 효과 없이 변경 불가능한 과거 재실행 바이트 조회 | [`volicord.get_operation_result`](#volicordget_operation_result) |
| `volicord.prepare_write` | 쓰기 판단 효과 기록 | [`volicord.prepare_write`](#volicordprepare_write) |
| `volicord.prepare_evidence_capture` | 만료되는 불변 capture intent 하나 생성 | [`volicord.prepare_evidence_capture`](#volicordprepare_evidence_capture) |
| `volicord.stage_artifact` | 임시 스테이징만 생성 | [`volicord.stage_artifact`](#volicordstage_artifact) |
| `volicord.record_run` | 실행, 현재 닫기 근거, 증거, 증거 관찰 효과 기록 | [`volicord.record_run`](#volicordrecord_run) |
| `volicord.request_user_judgment` | 대기 중인 판단 요청 생성 | [`volicord.request_user_judgment`](#volicordrequest_user_judgment) |
| `volicord.record_user_judgment` | 사용자 판단 해결 | [`volicord.record_user_judgment`](#volicordrecord_user_judgment) |
| `volicord.record_user_observation` | 대상 결합 User Channel 증거 기록 | [`volicord.record_user_observation`](#volicordrecord_user_observation) |
| `volicord.reconcile_changes` | 미기록 변경 해결, 대기 사용자 판단 생성, 선택적 세션 감시 진단 기록 | [`volicord.reconcile_changes`](#volicordreconcile_changes) |
| `volicord.check_close` | 선택적 세션 감시 진단을 포함하는 닫기 준비 상태 점검 | [`volicord.check_close`](#volicordcheck_close) |
| `volicord.close_task intent=complete` | 성공한 `complete` 종료 효과를 영속 저장하고 차단된 시도는 효과 없는 결과를 반환 | [`volicord.close_task intent=complete`](#volicordclose_task-intentcomplete) |
| `volicord.close_task intent=cancel` | 성공한 취소 종료 효과를 영속 저장하고 차단된 시도는 효과 없는 결과를 반환 | [`volicord.close_task intent=cancel`](#volicordclose_task-intentcancel) |
| `volicord.close_task intent=supersede` | 성공한 대체 종료 효과를 영속 저장하고 차단된 시도는 효과 없는 결과를 반환 | [`volicord.close_task intent=supersede`](#volicordclose_task-intentsupersede) |

<a id="volicordintake"></a>
### `volicord.intake`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- `Task`를 생성합니다.
- 모드, work phase, acceptance policy와 이유, 선택적 predecessor 관계와
  carry-forward disposition을 저장합니다.
- Core가 생성한 identity와 함께 순서가 있는 활성 `acceptance_criteria` 행을 생성합니다.
- 검증된 `initial_source_refs`를 Task 소유자 JSON의 비권위적 Task 맥락으로 보존합니다.
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
- 기준 교체가 null이 아니면 유지한 활성 같은 `Task` 기준 행을 교체 순서대로 갱신하고, null ID에 새 행을 만들며, 빠진 활성 행을 폐기하되 폐기된 identity를 다시 활성화하지 않습니다.
- 메서드 담당 문서가 제공한 효과 계약 JSON을 포함해 현재 적용 `change_units` 행을 만들거나 교체합니다.
- 검증된 선택적 Git workspace context를 Change Unit write basis에 캡처하고 현재
  Change Unit을 만들거나 교체할 때 advisor가 아닌 Task를
  `work_phase=implementation`으로 진행합니다.
- 현재 적용 범위나 현재 적용 Change Unit의 실질적 변경에 대해 `tasks.scope_revision`을 증가시킵니다.
- 실질적 범위 변경에 대해 `tasks.close_basis_json`을 무효화하고 `tasks.close_basis_revision`을 증가시킵니다.
- 담당 문서가 정의한 호환성에 따라 호환되지 않는 판단 근거 행을 오래됨 또는 대체됨으로 표시합니다.
- 메서드 담당 문서가 허용한 차단 사유 또는 오래된 쓰기 티켓 참조를 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run` 미리보기
- 거절된 시도

유효한 `dry_run` 미리보기는 범위, Change Unit, 차단 사유, 오래된 쓰기 티켓 효과만 미리 설명합니다.

의미가 같은 정규화된 갱신은 `tasks.scope_revision`을 증가시키거나 현재 닫기 근거를 무효화하지 않습니다.

담당 문서:

- [`volicord.update_scope` 메서드](api/method-update-scope.md)
- [저장소 기록](storage-records.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="volicordstatus"></a>
### `volicord.status`

읽기 전용 호출은 다음 특성을 가집니다.

- Core 권한 상태 변경 없이 응답 데이터를 반환합니다.
- 재실행 행을 만들지 않습니다.
- `project_continuity_records`를 만들지 않습니다.
- Core 상태를 변경하지 않습니다.
- `project_state.state_version`을 증가시키지 않습니다.

세션에 연결된 Agent Connection의 경우 `volicord.status`는 세션 감시 진단
맥락을 초기화하기 위해 `agent_sessions` 행을 만들 수 있고, 제한된 기준선
스냅샷을 사용할 수 있으면 `session_watch_baselines` 행을 만들 수 있습니다.
감시 비교를 실행하거나, `session_watch_observations`를 만들거나,
`unrecorded_changes`를 만들거나, 권한 이벤트를 추가하거나, 재실행 행을
만들거나, 닫기 상태를 변경하거나, `project_state.state_version`을 증가시키지는
않습니다. 이 상태 조회 경계에서 처음 만들어진 기준선은 `method_boundary` 감시
메타데이터를 사용하고 부분 감시를 보고합니다.

`dry_run=true`도 `ToolDryRunResponse`가 아니라 `effect_kind=read_only`인 `StatusResult`로 유지됩니다.

효과가 없는 분기:

- 거절된 시도

담당 문서:

- [`volicord.status` 메서드](api/method-status.md)

<a id="volicordget_operation_result"></a>
### `volicord.get_operation_result`

성공한 호출은 조회할 수 있는 변경 불가능한 `tool_invocations.response_json` 값
하나를 크기가 제한된 UTF-8 페이지로 읽습니다. 이 읽기는 원래 변경을 재실행하거나
응답을 다시 계산하거나 과거 결과를 현재 권한으로 만들지 않습니다.

이 메서드는 응답 전용이며 아래 항목을 만들거나 변경하면 안 됩니다.

- 재실행 행 또는 `tool_invocations.response_json`
- `authority_events` 또는 Core 현재 행
- Task, Change Unit, 판단, 차단 사유, 연속성 상태
- 스테이징, 아티팩트, 증거, 쓰기 티켓 상태
- 세션 감시 진단 행
- `project_state.state_version`

접근 거절, 잘못된 `cursor`, 사용할 수 없는 행, 무결성 실패에도 같은 무효과
경계를 적용하며 과거 바이트 일부를 반환하면 안 됩니다.

담당 문서:

- [`volicord.get_operation_result` 메서드](api/method-get-operation-result.md)
- [저장소 기록](storage-records.md#exact-operation-result-storage)
- [저장소 버전 관리](storage-versioning.md#exact-operation-result-retrieval)

<a id="volicordprepare_write"></a>
### `volicord.prepare_write`

`decision=allowed`인 재실행이 아닌 원래 커밋된 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 물리 `write_tickets` 테이블에 저장되는 열린 쓰기 티켓 하나를 발급합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

멱등 재실행은 [저장소 버전 관리](storage-versioning.md)에 따라 저장된 원래 응답을 반환하며, 이러한 효과를 반복하지 않습니다.

커밋되는 비허용 판단:

- [`volicord.prepare_write`의 커밋된 비허용 판단](#volicordprepare_write-committed-non-allow-decision)을 따릅니다.
- `authority_events` 행을 정확히 하나 추가하고, 키가 있으면 재실행 행을 만들며, `project_state.state_version`을 정확히 한 번 증가시킵니다.
- 쓰기 티켓, 별도 공개 이력 메서드, 제품 파일 쓰기 권한 기록을 만들지 않습니다.
- `volicord.status`는 과거 비허용 판단을 노출할 필요가 없습니다.

효과가 없는 분기:

- 거절된 시도
- 유효한 `dry_run` 미리보기

이 분기들은 아래 항목을 만들지 않습니다.

- 재실행 행.
- 쓰기 티켓.
- 이벤트.
- `close_state` 변경.
- 아티팩트 또는 증거 효과.
- `project_state.state_version` 증가.

담당 문서:

- [`volicord.prepare_write` 메서드](api/method-prepare-write.md)
- [저장소 기록](storage-records.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="volicordprepare_evidence_capture"></a>
### `volicord.prepare_evidence_capture`

재실행이 아닌 원래 커밋된 `dry_run=false` 호출은 다음을 수행합니다.

- `evidence_capture_intents` 행 하나를 삽입합니다.
- authority event 하나를 추가하고 replay 행 하나를 만듭니다.
- `project_state.state_version`을 정확히 한 번 증가시킵니다.

정확한 멱등 replay는 이 효과를 반복하지 않습니다. 유효한 dry run과 거절된 요청은
intent, receipt, source claim, staging 행이나 바이트, producer, event, replay 행,
state-version 변경을 만들지 않습니다.

등록 source fulfillment는 Core state 커밋 밖의 별도 Store 트랜잭션입니다. Intent와 등록
source를 다시 검증한 뒤 `evidence_capture_receipts` 행 하나, redacted
`artifact_staging` 행 하나, 크기가 제한된 safe JSON bytes, 필요한 모든
`evidence_capture_source_claims` 행을 원자적으로 만듭니다. Command, guard
connection, watcher receipt는 claim 하나를 만들고, tool receipt는 정규화한 host
invocation과 서로 다른 guard event 두 개에 대해 claim 세 개를 만듭니다. 프로젝트
범위 claim key는 정확한 원천 사실이 intent나 producer class를 넘어 재사용되는 것을
거부합니다.
Event나 replay 행을 만들지 않고 `project_state.state_version`도 바꾸지 않습니다.
Source observation은 `intent.created_at <= observed_at < intent.expires_at`을 만족해야
하고, receipt 생성은 `observed_at <= receipt.created_at < intent.expires_at`을
만족해야 하며, staging handle은
정확히 `intent.expires_at`에 만료됩니다. 트랜잭션 실패나 claim 중복 시 receipt와
claim을 rollback하고 새 staging file을 제거합니다. Intent 하나는 최대 한 번
fulfillment할 수 있습니다.

불변 receipt와 source claim은 영속 source-fact 행입니다. Receipt staging handle과
staged safe JSON bytes만 transient이며, 승격은 producer 감사 chain이 사용하는 receipt
행을 삭제하지 않습니다.

담당 문서:

- [`volicord.prepare_evidence_capture` 메서드](api/method-prepare-evidence-capture.md)
- [저장소 기록](storage-records.md)
- [아티팩트 저장소](storage-artifacts.md)

<a id="volicordstage_artifact"></a>
### `volicord.stage_artifact`

성공한 스테이징은 다음을 수행할 수 있습니다.

- `artifact_staging` 또는 동등한 저장소 소유 스테이징 기록을 생성합니다.
- `artifacts/tmp/` 아래에 임시 안전 바이트 또는 알림을 둡니다.

이 분기는 저장소 소유 임시 스테이징만 생성합니다. 일반 Core 커밋 변이 분기가 아니며, 임시 스테이징 디렉터리는 프로젝트 등록 시점이 아니라 스테이징이 일어날 때 만들어질 수 있습니다.

이 분기에는 재실행 행이나 `OperationResultRef`가 없으므로 Core는 스테이징
기록, 스테이징 핸들, 임시 디렉터리, 바이트, 알림을 만들기 전에 전체 직렬화
`StageArtifactResult`가 지원되는 스테이징 결과 상한 안에 들어가는지 증명해야
합니다. 결과가 상한을 넘을 것으로 판단되면 스테이징 효과 없이 거절합니다.
크기 확인을 스테이징 뒤로 미루면 안 됩니다.

아래 항목은 만들지 않습니다.

- Core 현재 기록.
- 영속 `ArtifactRef`.
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
- 호환되는 `write_tickets` 행을 소비합니다.
- 사용할 수 있는 `artifact_staging`을 소비합니다.
- `artifacts`를 승격하거나 연결합니다.
- 새 `Task` 범위 보충 대상에 `evidence_claims` 행을 만들고, 기존 같은 `Task` ID의 불변 문장을 보존합니다.
- `evidence_summaries`를 갱신하거나, Core 기록 입력 참조와 권한 효력이 없는 출처 참조를 분리해 저장하는 `evidence_observations`를 생성하거나, 허용된 `blockers`를 갱신합니다.
- 유효한 capture-intent 관찰마다 safe receipt staging handle을 소비하고 승격하며,
  승격한 artifact를 새 불변 `evidence_producers` 행에 연결하고, 해당 producer와
  일대일 `evidence_observation`을 만듭니다.
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
- 영속 잔여 위험 ID.
- 영속 아티팩트.
- 아티팩트 연결.
- 증거 갱신 또는 증거 관찰.
- 차단 사유 갱신.
- 이벤트.
- 재실행 행.
- 스테이징 핸들 소비.
- 쓰기 티켓 호환성 소비.
- `project_state.state_version` 증가.

거절된 시도는 아래 항목을 바꾸지 않습니다.

- 스테이징 행.
- 아티팩트.
- 수락 기준, 보충 증거 주장, 증거 관찰.
- evidence-capture intent, receipt, producer, receipt staging 행.

제품 파일 쓰기 영속 저장 경계:

- 메서드 담당 문서가 제품 파일 쓰기를 기록하는 커밋된 실행을 허용할 때, 저장소는 같은 커밋에서 호환되는 `write_tickets` 행을 소비할 수 있습니다.
- 테스트 증거 영속 저장은 제품 파일 쓰기 관찰을 뜻하지 않으면서도 스테이징된 아티팩트를 승격하고 증거를 갱신하며 증거 관찰을 기록할 수 있습니다.
- 정확한 실행 분류는 [`volicord.record_run` 메서드](api/method-record-run.md)가 담당합니다.

현재 닫기 근거 영속 저장 경계:

- 커밋된 `volicord.record_run`은 `tasks.close_basis_revision`을 정확히 한 번 증가시킵니다.
- `close_assessment`가 `null`이 아니면 `tasks.close_basis_json`에 새 현재 `CurrentCloseBasis`를 쓰고 Core가 생성한 불투명 잔여 위험 ID를 저장합니다.
- 그 `CurrentCloseBasis`에 저장되는 민감 동작 요구사항은 커밋된 실행 기록과 소비된 쓰기 티켓 호환성 행에서 Core가 파생하며, 동작, 정규화된 경로, 민감 범주, 기준선, Change Unit, 출처 실행 기록 참조, 출처 쓰기 티켓 참조를 닫기까지 보존합니다.
- 범주만 담은 호출자 입력은 민감 동작 요구사항을 만들거나, 만족하거나, 지울 수 없습니다.
- `close_assessment=null`은 커밋된 실행 기록이 현재 닫기 근거를 만들지 않음을 기록합니다. 기존 현재 근거는 오래되거나 없어집니다.
- 실행 기록, 현재 닫기 근거, 증거 요약, 증거 관찰, capture producer, receipt artifact
  승격/연결, 쓰기 티켓 호환성 소비, replay, event, revision 효과는 원자적으로
  커밋됩니다.

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
- 로컬 웹 동의 캡처 경로로 호출된 경우, 판단 해결과 같은 프로젝트 상태 커밋 안에서 일치하는 `local_web_consent_tokens` 행을 `status='consumed'`로 설정합니다.
- 메서드 담당 문서가 선택할 때 수락된 제품, 기술, 범위 결정과 수락된 현재 잔여 위험에 대한 `project_continuity_records`를 생성합니다.
- 종속 차단 사유 또는 다음 행동을 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run` 미리보기
- 거절된 시도

검증 실패, 잘못된 바인딩, 만료, 판단 기록 쓰기 실패를 포함한 거절된 로컬 웹 동의 시도는 토큰을 소비하거나 판단을 해결하면 안 됩니다.

유효한 `dry_run` 미리보기는 아래 항목을 만들지 않습니다.

- 판단 해결.
- 프로젝트 연속성 기록.
- 차단 사유 갱신.
- 이벤트.
- 재실행 행.
- `project_state.state_version` 증가.

사용자 판단 기록은 `tasks.scope_revision`이나 `tasks.close_basis_revision`을 증가시키지 않습니다.

`status='resolved'`는 답변이 기록되었다는 뜻이며 그 자체로 수락이 아닙니다. 현재 해결 행에는 완전한 근거, 선택된 동작, `resolution_outcome`, 해결 요청 본문, 해결 타임스탬프, User Channel 행위자 출처, 검증 근거, 보증 수준, 필요한 행위자 출처가 있어야 합니다. 필요한 해결 권한 정보가 빠진 행은 읽을 수 있는 이력 감사 판단이 아니라 유효하지 않은 저장 상태입니다.

재실행 행은 계속 `operation_category=user_only`입니다. 그 정확한 응답과 자유
형식 비공개 `note`는 `volicord.get_operation_result`를 통한 Agent Connection
조회 대상이 아닙니다.

담당 문서:

- [`volicord.record_user_judgment` 메서드](api/method-record-user-judgment.md#volicordrecord_user_judgment)
- [저장소 기록](storage-records.md)

<a id="volicordrecord_user_observation"></a>
### `volicord.record_user_observation`

커밋된 `dry_run=false`는 다음 효과를 만들 수 있습니다.

- 현재 Task, Change Unit, scope revision, baseline, 대상, relevance, 정확한
  정규 아티팩트 ref, 로컬 사용자 actor, 검증 근거, 요약, 타임스탬프를 담은
  `user_evidence_observations` 행 하나 삽입
- `user_evidence_observation_recorded` 이벤트 추가
- replay 행 생성
- `project_state.state_version` 한 번 증가

Run, EvidenceSummary, EvidenceObservation, UserJudgment, 승인, 아티팩트를 만들지
않습니다. Dry run과 거부는 저장 효과가 없습니다.

담당 문서:

- [`volicord.record_user_observation` 메서드](api/method-record-user-observation.md)
- [저장 레코드](storage-records.md)

<a id="volicordreconcile_changes"></a>
### `volicord.reconcile_changes`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 세션에 연결된 Agent Connection에 대해 제한된 세션 감시 점검을 먼저 실행하고,
  Product Repository 변경이 예상 쓰기 또는 `active` 쓰기 티켓 상관관계로
  결정적으로 포함되지 않을 때
  `agent_sessions`, `session_watch_baselines`, `session_watch_observations`,
  감시기가 만든 `unrecorded_changes`를 만들거나 갱신할 수 있습니다.
- 미해결 `unrecorded_changes` 행을 `status='resolved'`로 설정합니다.
- 해결 근거, 캡처 근거, 해결 메서드, 선택적 연결 사용자 판단 참조를 이름 붙이는 해결 JSON을 저장합니다.
- `resolved_at`과 `resolved_by_actor_source`를 저장합니다.
- 사용자 수락이 필요한 미기록 변경에 대해 대기 `user_judgments` 행을 만듭니다.
- 이벤트를 추가합니다.
- 멱등성 키가 있으면 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

읽기 전용 분기:

- 위에서 허용한 세션 감시 진단 효과가 발생한 뒤에도, 계획된 해결이나 대기
  판단 생성이 없는 유효한 호출은 응답 데이터만 반환하고 조정 효과를 만들지
  않습니다.

효과가 없는 분기:

- 거절된 시도
- 유효한 `dry_run` 미리보기

이 분기들은 미기록 변경을 해결하거나, 대기 판단을 만들거나, 이벤트를 추가하거나, 재실행 행을 만들거나, `project_state.state_version`을 증가시키지 않습니다.

조정 효과는 제품 정확성, 테스트 충분성, 검토 완료, 최종 수락, 잔여 위험 수락, 보안을 증명하지 않습니다. 미기록 변경이 더 이상 미해결이 아닌 이유를 기록하거나, 남은 수락을 위한 대기 사용자 소유 판단을 만들 뿐입니다.

담당 문서:

- [`volicord.reconcile_changes` 메서드](api/method-reconcile-changes.md#volicordreconcile_changes)
- [저장소 기록](storage-records.md)

<a id="volicordcheck_close"></a>
### `volicord.check_close`

읽기 전용 호출에는 Core 권한 상태 저장 효과가 없습니다.

- 계산된 닫기 준비 상태를 반환합니다.
- `volicord.status include.close=true`와 같은 닫기 준비 상태 계산을 사용합니다.
- 재실행 행을 만들지 않습니다.
- 이벤트를 추가하지 않습니다.
- 차단 사유 행을 만들지 않습니다.
- `close_state`를 변경하지 않습니다.
- 아티팩트나 증거를 바꾸지 않습니다.
- `project_state.state_version`을 증가시키지 않습니다.

세션에 연결된 Agent Connection이고 `dry_run=false`이면, 이 점검은 제한된 세션
감시 점검을 먼저 실행하고 Product Repository 변경이 예상 쓰기 또는 `active`
쓰기 티켓 상관 관계로 결정적으로 포함되지 않을 때 `agent_sessions`, `session_watch_baselines`,
`session_watch_observations`, 감시기가 만든 `unrecorded_changes`를 만들거나
갱신할 수 있습니다. 이러한 진단 효과는 Core 권한 상태 저장 효과가 아닙니다.
권한 이벤트를 추가하거나, 차단 사유 행을 만들거나, 닫기 상태를 변경하거나,
아티팩트 또는 증거를 건드리거나, `project_state.state_version`을 증가시키지
않습니다. 이 점검이 첫 감시기 기준선을 만들면 감시 근거는
`method_boundary`이며 더 이른 Product Repository 변경은 감시 범위 밖에
있습니다.

`dry_run=true`도 `effect_kind=read_only`인 `CloseTaskResult`로 유지됩니다.

효과가 없는 분기:

- 거절된 시도

담당 문서:

- [`volicord.check_close` 메서드](api/method-close-task.md#volicordcheck_close)

<a id="volicordclose_task-intentcomplete"></a>
### `volicord.close_task intent=complete`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 세션에 연결된 Agent Connection에 대해 제한된 세션 감시 점검을 먼저 실행하고
  `volicord.check_close`에서 허용한 것과 같은 세션 감시 진단 기록을
  만들거나 갱신할 수 있습니다.
- 메서드가 선택한 완료 종료 효과를 영속 저장합니다.
- 메서드가 선택한 완료 효과가 성공하면 `tasks.close_basis_json`과 별개인 종료 닫기 요약을 영속 저장할 수 있습니다.
- 메서드가 선택한 완료 효과가 성공하면 보이지만 잔여 위험 수락이 필요하지 않은 현재 닫기 근거 잔여 위험에 대해 `kind='known_limit'`인 `project_continuity_records`를 생성합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 응답 전용으로 차단된 `complete` 결과
- 유효한 `dry_run=true`
- 사전 확인 실패

유효한 `dry_run=true`는 `ToolDryRunResponse`를 반환합니다. 사전 확인 실패는 효과가 없는 `ToolRejectedResponse`입니다.

응답 전용으로 차단된 `complete` 결과는 `base.effect_kind=no_effect`를 사용하고 닫기 차단 사유 행, 권한 이벤트, 재실행 행, 종료 상태 변경, 상태 버전 증가를 영속 저장하지 않습니다. 닫기 준비 상태 평가 전에 만든 세션 감시 진단 기록은 차단된 닫기 결과와 별개입니다.

담당 문서:

- [`volicord.close_task` 메서드](api/method-close-task.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="volicordclose_task-intentcancel"></a>
### `volicord.close_task intent=cancel`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 메서드가 선택한 취소 효과를 영속 저장합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 응답 전용으로 차단된 취소 결과
- 유효한 `dry_run=true`
- 사전 확인 실패

유효한 `dry_run=true`는 `ToolDryRunResponse`를 반환합니다.

취소 효과에는 `machine_action=accept`, `resolution_outcome=accepted`, 호환되는 근거, `resolved_by_actor_source=local_user`, 호환 User Channel 출처를 가진 메서드 담당 현재 취소 판단이 필요합니다. 취소 권한이 없거나 호환되지 않으면 응답 전용 차단 결과를 반환하며, 수락이나 완료 전용 닫기 증거를 만들어 내면 안 됩니다.

담당 문서:

- [`volicord.close_task` 메서드](api/method-close-task.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="volicordclose_task-intentsupersede"></a>
### `volicord.close_task intent=supersede`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 메서드가 선택한 대체 효과를 영속 저장합니다.
- 메서드가 선택한 효과에 필요하면 같은 변경에서 `project_state.active_task_id`를 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 응답 전용으로 차단된 대체 결과
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
