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
| 읽기 전용 | 읽기 전용 `MethodResult` | Core 권한 상태 변경은 없습니다. 응답 데이터만 반환합니다. 재실행 행, 권한 이벤트, 아티팩트 효과, 쓰기 티켓 효과, 닫기 상태 변경, `project_state.state_version` 증가는 없습니다. | [읽기 전용 결과](#read-only-result) |
| 효과 없음 | `ToolRejectedResponse` 또는 `effect_kind=no_effect`인 유효한 `MethodResult` | 요청된 일반 변이가 없고 Core 커밋도 없습니다. 응답이 오류나 차단 사유형 데이터를 담을 수 있지만, 이 분기는 그 값을 지속하지 않습니다. | [`ToolRejectedResponse`](#toolrejectedresponse-effect), [효과가 없는 분기](#no-effect-branches) |
| `dry_run` | 유효한 `ToolDryRunResponse` | 미리보기만 반환합니다. 영속 참조, 재실행 행, 이벤트, 스테이징 핸들, 아티팩트 효과, `project_state.state_version` 증가는 없습니다. | [유효한 `dry_run` 미리보기](#valid-dry-run-preview) |
| 스테이징 생성 | `effect_kind=staging_created`인 `StageArtifactResult` | 저장소 소유 임시 스테이징과 영속 정규 UTC 하한의 원자적이고 감소하지 않는 전진만 만듭니다. 일반 Core 커밋 트랜잭션이 아닙니다. | [스테이징 생성 아티팩트 결과](#staging-created-artifact-result) |
| Core 커밋 | Core 커밋 `MethodResult` | `CoreProjectStore::commit_mutation`을 통해 메서드 담당 효과를 만듭니다. 상태 버전 증가, 권한 이벤트, 선택적 재실행 행, 메서드가 선택한 `CoreStorageMutation` 값, 정규 커밋 timestamp 하나가 포함됩니다. | [Core 커밋 결과](#core-committed-result) |
| 커밋된 차단 사유형 결과 | 메서드 담당 문서가 차단 또는 비허용 지속 저장을 허용한 커밋 `MethodResult` | 명시적으로 허용된 이벤트, 재실행, 상태 버전, 차단 사유 상태 효과만 만듭니다. 차단 사유형 응답만으로는 충분하지 않습니다. | [커밋된 차단 결과](#committed-blocked-result) |

정확한 재실행, 거부된 요청, 유효한 dry run, 읽기 전용 결과는 영속 정규 Core UTC
하한을 갱신하지 않습니다. 저장소 소유 staging, 등록된 evidence-capture receipt 이행,
로컬 User Channel token 발급은 아래에서 정의하는 하한 전용 예외입니다. 전체 시계
계약은 [저장소 버전 관리](storage-versioning.md#canonical-core-utc-clock)가 담당합니다.

<a id="read-only-result"></a>
### 읽기 전용 결과

저장 효과:

- Core 권한 상태 저장 효과가 없습니다.
- 응답 데이터만 반환합니다.

허용되지 않는 효과:

- 재실행 행
- 권한 이벤트
- Core 현재 기록 변경
- 닫기 상태 변경
- 아티팩트 효과
- 증거 업데이트 또는 증거 관찰
- 쓰기 티켓 효과
- `project_state.state_version` 증가
- 영속 정규 UTC 하한 갱신

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
- 영속 정규 UTC 하한 갱신

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
- 영속 정규 UTC 하한 갱신

<a id="staging-created-artifact-result"></a>
### 스테이징 생성 아티팩트 결과

허용될 수 있는 효과:

- 저장소 소유 임시 스테이징
- 원자적인 `project_state.updated_at >= artifact_staging.created_at` 하한 갱신

이 분기는 일반 Core 커밋 변이와 별개입니다. 저장소가 관리하는 스테이징 표현이나 핸들을 만들 수 있지만, 그 임시 스테이징 쓰기 자체가 Core 현재 기록 변경, 영속 `ArtifactRef`, 아티팩트 연결, 증거 기록은 아닙니다.

허용되지 않는 효과:

- 물리 프로젝트 시각 하한을 제외한 Core 권한 현재 기록
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

커밋은 준비된 동작 시각 샘플보다 이르지 않은 정규 `committed_at` 하나를 선택합니다.
정확히 같은 값을 `project_state.updated_at`, 추가한 모든
`authority_events.created_at`, 선택적 재실행 행의 `tool_invocations.created_at`,
mutation application이 생성하는 적용 가능한 Store transaction metadata인
`created_at`, `updated_at`, `retired_at`, `promoted_at`에 씁니다. 의미 있는 동작
시각인 `requested_at`, `resolved_at`, `closed_at`, `recorded_at`, `consumed_at`과
입력·관찰 담당 사실인 `observed_at`, `started_at`은 담당 문서가 정의한 동작 샘플
또는 검증된 원천 의미를 유지하며 커밋 timestamp로 바꾸지 않습니다.

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
- 소비 시도에서 유효하지 않거나 소비됨, 철회됨, 상태 결합 비호환인 쓰기 티켓.
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

읽기 전용 결과에는 Core 권한 상태 저장 효과가 없고 재실행 행도 아니며 응답으로만
반환됩니다.

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

프로젝트 정책 적용은 권위 있는 `project_workflow_policies` 정규 복사본, 단조 증가
버전, 기준 JSON, 지문, source를 원자적으로 쓰고 정규화된 쓰기 권한 fingerprint도
파생합니다. 이 fingerprint가 바뀌면 같은 트랜잭션에서 저장 결속이 없거나 다른 모든
활성 티켓을 `explicit_revoke`로 무효화하고, 현재 통제 수준과 수락 수준이 이미 새
최솟값을 충족해도 활성 Task 재평가 표시를 만들거나 갱신하며, 표시된 Task의 모든
활성 티켓도 `explicit_revoke`로 무효화합니다. 유효 통제 수준을
조용히 낮추지 않습니다. Canonical 정책이 같거나 변경된 정책의 정규화된 쓰기 권한
fingerprint가 같으면 정책 적용을 실행했다는 이유만으로 티켓을 무효화하지 않습니다.
정확한 명령, 파일, host 동작은 관리 담당 문서의 관심사입니다.

Workflow metric 쓰기는 집계 counter, duration, 직렬화 tool byte 수, 범주형
outcome만 저장합니다. 이 기록은 prompt, file, answer, command 본문을 저장하지
않습니다.

## 관리 Connection Setup과 검증

승인된 setup, repair 또는 staged managed-configuration 적용은 host configuration을 먼저 쓴
뒤 setup 소유 Store 경로에서 그 결과인 `managed_fingerprint`를 commit합니다. 기존
Connection의 fingerprint가 바뀌면 같은 immediate Registry transaction에서
`verification_report_json`을 비웁니다. Fingerprint가 같은 호환 replay는 보고서를 유지할 수
있습니다. Host 쓰기가 성공하고 뒤의 관련 없는 작업이 실패한 경우에는 관리 CLI의 부분 적용
보고 및 재시도 계약을 따릅니다.

활성 연결 검증은 자신이 검증하는 정확한 typed Connection integration revision을
확보합니다. 보고서 영속은 immediate Registry transaction 하나에서 현재 revision을 비교하고
정확히 일치할 때만 `verification_report_json`과 일반 row 갱신 timestamp를 교체합니다.
`managed_fingerprint`, integration instance 또는 generation, mode, metadata, membership,
Guard manifest, runtime session, 프로젝트 Agent Session은 쓰지 않습니다. Revision 충돌은
Registry 효과가 없습니다. `volicord connection status`는 계속 읽기 전용이며 이 변경을
사용하지 않습니다.

활성 검증은 conformance 전에 항상 rollback하는 한도가 있는 SQLite transaction으로
Registry와 선택한 project의 쓰기 가능성을 probe합니다. 이 probe는 write lock을 얻을 수
있으므로 활성 저장 작업이지만 schema object나 row를 남기지 않습니다. Protocol 및 host
호환성 conformance는 새로운 일회용 명령별 Runtime Home에만 `manual_cli` session과 가능한
finding을 만들며 fixture disposal이 이를 제거합니다. 선택한 사용자 Runtime Home에는
conformance session이나 finding을 만들지 않습니다.

`volicord mcp preflight`는 선택한 Registry와 project database를 read-only로 엽니다. 쓰기
가능성을 probe하거나 runtime session을 생성·갱신하거나 finding을 영속하거나 diagnostic을
reconcile하거나 Runtime Home 또는 Product Repository에 쓰지 않습니다. 따라서 JSON
`side_effects`는 빈 배열입니다.

<a id="connection-integration-verification-effects"></a>
## Connection-Integration 검증 효과

`volicord.begin_integration_verification`은 immediate Registry transaction 하나보다 먼저
현재 managed runtime, native session/turn, 선택한 Connection Project, 현재 Agent Session,
Guard Installation, policy, revision, hook contract, prompt 관찰을 검증합니다. 정확한
semantic 좌표의 기존 row는 terminal row를 포함해 그대로 반환하고, 실제로 달라진 적격
좌표에만 `guard_integration_verification_runs` row 하나를 삽입합니다. 이전 nonterminal
좌표가 대체되면 begin은 typed terminal repair를 기록한 다음 retry policy를 적용합니다.
Cleanup은 오래 보관된 record만 제거하며 새 ID를 만들지 않습니다. 거부된 호출, 수동 호출,
preflight 호출, 모호한 호출, prompt가 없는 호출, retry가 허용되지 않는 호출은 새 run
효과가 없습니다.

`volicord.guard_probe`는 immediate Registry transaction 하나를 사용합니다. 정확한 run을
읽고 완전한 caller 좌표를 검증하며 현재 유효 상태를 계산한 뒤, 기존
`probe_acknowledged_at`이 있으면 갱신 없이 반환합니다. 이 필드가 없을 때는 적격인 active
run만 조건부로 값을 설정할 수 있습니다. Store는 `guard_probe_observations`에
`probe_acknowledged`를 기록하고 pre-tool acquisition이 아직 도착하지 않았으면
`hook_event_not_observed`도 기록한 뒤 commit 전에 권위 있는 timestamp와 상태를 다시
읽습니다. 따라서 동시에 실행한 동일한 첫 호출은 timestamp 하나로 수렴합니다. `complete`
또는 `repair_required` 뒤의 정확한 replay는 완료 정보와 일치한 event를 바꾸지 않고
원래 acknowledgement를 반환합니다. 다른 caller 좌표는 값을 노출하지 않고 거부하며
acknowledgement가 없는 terminal run에는 뒤늦게 값을 만들 수 없습니다. 프로젝트
`state.sqlite`, Core 작업 흐름, Task, Product Repository에는 효과가 없습니다.

`volicord.get_integration_verification`은 immediate Registry transaction 하나를
사용합니다. Caller와 현재 owner 좌표를 검증하고 저장된 host policy가 허용한 status read
수를 넘지 않으며 기존 terminal 상태는 변경하지 않고 반환합니다. 현재 synchronous Codex
policy는 read 한 번을 허용합니다. 정확한 event correlation이 attempt를 이미 완료하지
않았다면 이 read가 가장 구체적인 acquisition reason과 별도 retry policy가 있는
`repair_required`를 영속합니다. 호환 Guard event 영속은 일반적인 프로젝트 로컬 효과를
유지하며 뒤따르는 Registry acquisition write는 bounded stage를 기록할 수 있습니다.
일치한 pre/post stage는 `complete`를 원자적으로 확정할 수 있습니다. 어떤 분기도 없는
Guard event를 꾸며내거나 cleanup expiry를 기다리거나 terminal attempt를 다시
활성화하거나 MCP trust 상태를 변경하지 않습니다.

## Managed Runtime 프로젝트 Session 결속

실제 managed MCP 프로젝트 호출은 다음 순서로 저장 효과를 만듭니다.

1. Runtime, Connection revision, 프로젝트 membership, 관찰 시각, 현재 프로젝트
   identity 검증이 실패하면 저장 효과가 없습니다.
2. Immediate 프로젝트 transaction 하나가 정확한 unbound Agent Session anchor를 만들거나
   검증하고 담당 문서가 정의한 관찰 갱신만 적용합니다.
3. Immediate Registry transaction 하나가 현재 소유자 사실을 다시 검증하고 일치하는
   `mcp_runtime_project_session_bindings` 예약을 삽입하거나 정확히 재사용합니다.
4. 마지막 immediate 프로젝트 transaction 하나가 정확한 anchor에 runtime을 붙이거나 이미
   같은 attach가 있으면 replay로 받아들입니다.

결정적인 Connection, 프로젝트, Guard Installation, native session, thread, 변경 불가능한
revision, 기존 runtime 소유권 충돌은 첫 두 단계에서 거부되며 Registry 예약을 만들지
않습니다. Registry uniqueness 실패는 검증된 프로젝트 anchor를 unbound로 남길 수 있지만
그 row는 상관관계 상태일 뿐입니다. Registry 예약 뒤 중단되면 정확한 예약만 있고 프로젝트
attach가 없는 상태가 남을 수 있으며 이 예약도 권한이 아닙니다. 소유자 상태가 바뀌지 않은
정확한 replay는 예약을 재사용해 마지막 attach를 완료합니다. 어떤 실패 경로도 다른 runtime의
유효한 예약을 보상 삭제하지 않습니다.

## 관리 Connection Project 폐기

승인된 `volicord connection remove` 적용은 immediate Registry transaction 하나를
사용합니다. Agent Connection과 선택한 membership을 검증하고 pending-host-cleanup
충돌을 거부한 뒤, 선택한 membership의 Registry project-session binding과
integration-verification run을 먼저 삭제하고 Guard Installation, membership 순서로 삭제한
뒤 commit 전에 남은 membership 수를 계산합니다.
Membership이 남으면 connection 전체 runtime session과 다른 프로젝트 행은 바꾸지
않습니다. Membership이 남지 않으면 Connection 소유의 남은 binding,
integration-verification run, Guard Installation, 모든 Connection 소유 MCP runtime session,
Agent Connection도 삭제합니다.

Connection migration도 같은 소유자 순서의 프로젝트 폐기를 사용합니다. 여러 프로젝트를
가진 superseded Connection에서는 replacement membership, Guard Installation,
Connection을 활성화하는 같은 Registry transaction 안에서 선택한 프로젝트의 binding,
integration-verification run, Guard Installation, membership만 제거합니다. 다른 프로젝트
행과 connection 전체 runtime session은 유지합니다.

마지막 프로젝트를 가진 superseded Connection은 외부 정리가 대기 중이거나 실패하는 동안
membership, binding, Guard Installation, pending-host-cleanup marker의 완전한 inventory와
함께 비활성 상태로 남습니다. 정리가 성공한 뒤 최종 Registry transaction 하나에서 정확한
replacement, marker, membership inventory를 다시 검증하고 프로젝트 소유 행을 membership보다
먼저 폐기한 뒤 marker를 지웁니다. 재검증이 실패하면 완전한 Registry inventory를 바꾸지 않아
재시도할 수 있습니다. 정리가 성공한 뒤에도 membership이 없는 비활성 과거 Connection과 그
connection 전체 runtime session은 유지합니다.

거절되거나 실패한 Store transaction은 Registry에 효과를 남기지 않습니다. Dry run은
Registry, host configuration, Product Repository에 효과가 없습니다. 프로젝트 로컬 Agent
Session, Guard 및 workflow 이력, evidence, authority event, replay와 그 밖의 프로젝트
권한 행은 이 관리 폐기에 포함되지 않습니다. 유지된 과거 행은 현재 Registry 소유권이 없으면
현재 호출에 권한을 부여할 수 없습니다.

<a id="method-effects"></a>
## 메서드 저장 효과 요약

아래 표는 메서드별 지속 저장 효과를 요약합니다. 메서드 동작과 응답 공용체는 [API 메서드](api/methods.md)가 안내하는 메서드 담당 문서가 담당합니다.

| 메서드 | 주 저장 효과 | 세부사항 |
|---|---|---|
| `volicord.intake` | `Task`와 구체화 기록 생성 | [`volicord.intake`](#volicordintake) |
| `volicord.update_scope` | 현재 적용 범위 기록 갱신 | [`volicord.update_scope`](#volicordupdate_scope) |
| `volicord.status` | 읽기형 응답 | [`volicord.status`](#volicordstatus) |
| `volicord.get_operation_result` | 저장 효과 없이 변경 불가능한 과거 재실행 바이트 조회 | [`volicord.get_operation_result`](#volicordget_operation_result) |
| `volicord.prepare_write` | 쓰기 판단 효과 기록 | [`volicord.prepare_write`](#volicordprepare_write) |
| `volicord.prepare_evidence_capture` | 만료되는 불변 capture intent 하나 생성 | [`volicord.prepare_evidence_capture`](#volicordprepare_evidence_capture) |
| `volicord.stage_artifact` | 임시 스테이징만 생성 | [`volicord.stage_artifact`](#volicordstage_artifact) |
| `volicord.record_run` | 실행, 현재 닫기 근거, 증거, 증거 관찰 효과 기록 | [`volicord.record_run`](#volicordrecord_run) |
| `volicord.request_user_action` | 대기 사용자 행동 요청과 canonical 캡처 폼 하나 생성 | [`volicord.request_user_action`](#volicordrequest_user_action) |
| `volicord.resolve_user_action` | 변경 불가능한 User Channel 해결 하나 삽입 | [`volicord.resolve_user_action`](#volicordresolve_user_action) |
| `volicord.reconcile_changes` | 미기록 변경 해결과 대기 사용자 행동 생성 | [`volicord.reconcile_changes`](#volicordreconcile_changes) |
| `volicord.check_close` | 읽기 전용 닫기 준비 상태 점검 | [`volicord.check_close`](#volicordcheck_close) |
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
- 담당 문서가 정의한 호환성에 따라 호환되지 않는 사용자 행동 근거 행을 오래됨 또는 대체됨으로 표시합니다.
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
- Task, Change Unit, 사용자 행동, 차단 사유, 연속성 상태
- 스테이징, 아티팩트, 증거, 쓰기 티켓 상태
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

- 물리 `write_tickets` 테이블에 활성 쓰기 티켓 하나를 발급하거나 호환되는 활성 미소비 티켓 하나를 재사용합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

발급은 행 하나를 삽입합니다. 재사용은 티켓을 삽입하지 않고 식별자를 보존하며
이벤트/재실행/상태 버전 효과는 정확히 한 번 발생합니다. 이 증가나 관련 없는 Core
변경은 티켓을 무효화하지 않습니다. 비허용 판단은 관련 없는 활성 티켓을 철회하지 않습니다.

발급은 현재 정규화된 프로젝트 쓰기 권한 fingerprint를 `validity_basis_json`에
저장합니다. 재사용에는 null이 아닌 현재 fingerprint의 정확한 일치가 필요합니다.
커밋된 모든 non-dry-run 판단에서는 결속이 없거나 달라 오래된 티켓으로 선택된 모든
활성 티켓을 원자적으로
`status=invalidated,invalidation_reason=explicit_revoke`로 바꿉니다. 허용 판단은 새
현재 티켓을 발급하기 전에 무효화하고, 커밋된 비허용 판단은 교체 티켓을 발급하지 않은 채
무효화를 영속합니다. 거절 및 dry-run 경로는 해당 무효 행을 바꾸지 않으며 그 행은 동적으로
사용할 수 없는 상태로 남습니다.

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

등록 source fulfillment는 Core state 커밋 밖의 별도 Store 트랜잭션입니다. Intent의
source selector와 명시적으로 선택한 등록 source를 다시 검증한 뒤
`evidence_capture_receipts` 행 하나, redacted
`artifact_staging` 행 하나, 크기가 제한된 safe JSON bytes, 필요한 모든
`evidence_capture_source_claims` 행을 원자적으로 만듭니다. Command, guard
connection, watcher receipt는 claim 하나를 만들고, tool receipt는 정규화한 host
invocation과 서로 다른 guard event 두 개에 대해 claim 세 개를 만듭니다. 프로젝트
범위 claim key는 정확한 원천 사실이 intent나 producer class를 넘어 재사용되는 것을
거부합니다.
Event나 replay 행을 만들지 않고 `project_state.state_version`도 바꾸지 않습니다.
같은 transaction에서 `project_state.updated_at`을 `receipt.created_at` 이상으로
전진시킵니다. 다른 동시 writer가 이미 더 늦은 하한을 만들었을 수 있으므로 두 값이
정확히 같을 필요는 없습니다.
등록 connection observation에서 intent는 intent 전 selector만 결합합니다. Source 소유
receipt가 선택 source identifier, observation 시각, raw-event 또는 snapshot/selection
digest를 확정합니다. Guard event는 선택한 event kind와 일치해야 합니다. Watcher
observation은 정확한 connection과 session의 유일한 현재 active baseline에 속해야
합니다. 명시적 source 좌표가 없거나 여러 개인 경우, intent 전 source, incomplete 또는
degraded source, receipt/source 불일치는 모두 효과 없이 실패합니다.
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
- staging 행과 원자적으로 `project_state.updated_at`을 그 행의 `created_at` 이상으로
  전진시킵니다.

이 분기는 저장소 소유 임시 스테이징만 생성합니다. 일반 Core 커밋 변이 분기가 아니며, 임시 스테이징 디렉터리는 프로젝트 등록 시점이 아니라 스테이징이 일어날 때 만들어질 수 있습니다.

이 분기에는 재실행 행이나 `OperationResultRef`가 없으므로 Core는 스테이징
기록, 스테이징 핸들, 임시 디렉터리, 바이트, 알림을 만들기 전에 전체 직렬화
`StageArtifactResult`가 지원되는 스테이징 결과 상한 안에 들어가는지 증명해야
합니다. 결과가 상한을 넘을 것으로 판단되면 스테이징 효과 없이 거절합니다.
크기 확인을 스테이징 뒤로 미루면 안 됩니다.

아래 항목은 만들지 않습니다.

- 물리 프로젝트 시각 하한을 제외한 Core 권한 현재 기록.
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
- Run이 실제 Product Repository 파일 쓰기를 기록하거나 유효 `sensitive` Task가 해당
  티켓에 결속된 정확한 승인 비제품 동작을 기록할 때 호환되는 `write_tickets` 행을
  소비합니다.
- 사용할 수 있는 `artifact_staging`을 소비합니다.
- `artifacts`를 승격하거나 연결합니다.
- 새 `Task` 범위 보충 대상에 `evidence_claims` 행을 만들고, 기존 같은 `Task` ID의 불변 문장을 보존합니다.
- `evidence_summaries`를 삽입하거나 갱신하면서 `produced_at_state_version`을 transaction의 결과 `project_state.state_version`으로 설정하거나, Core 기록 입력 참조와 권한 효력이 없는 출처 참조를 분리해 저장하는 `evidence_observations`를 생성하거나, 허용된 `blockers`를 갱신합니다.
- 유효한 capture-intent 관찰마다 safe receipt staging handle을 소비하고 승격하며,
  승격한 artifact를 새 불변 `evidence_producers` 행에 연결하고, 해당 producer와
  일대일 `evidence_observation`을 만듭니다.
- `close_assessment`에 따라 `tasks.close_basis_revision`과 `tasks.close_basis_json`을 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

제품 파일 쓰기가 없는 비민감 Run은 호환 티켓을 재사용할 수 있도록 활성 상태로
둡니다. 반면 유효 `sensitive` 비제품 Run은 정확한 승인 결속 티켓을 요구하고 소비하여
Core가 파생한 민감 동작 근거를 닫기까지 보존합니다.
소비, Run 삽입, 모든 증거/아티팩트 효과는 하나의 원자적 커밋입니다. 거절은 소비를
기록하지 않습니다. `basis_state_version`은 감사 전용이며 유효성은 저장된
Task/Change Unit/범위/기준선/workspace/현재 프로젝트 쓰기 권한/승인 근거, 상태, 선택적
idle timeout을 사용합니다.

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

쓰기 티켓 소비 경계:

- 메서드 담당 문서가 제품 파일 쓰기를 기록하는 커밋된 실행 또는 유효 `sensitive`
  Run의 정확한 승인 비제품 동작 기록을 허용할 때 저장소는 같은 커밋에서 호환되는
  `write_tickets` 행을 소비할 수 있습니다.
- Core는 소비를 계획하기 전에 현재 정규화된 쓰기 권한 fingerprint를 다시 읽고
  검증합니다. 커밋 트랜잭션 안에서 Store는 정책을 다시 읽어 활성 티켓의 영속 결속과
  계획 시 예상 결속이 모두 일치하는지 확인합니다. 계획과 소비 사이에 정책 권한이
  바뀌면 트랜잭션 전체를 롤백하여 티켓을 소비하지 않고 Run, 증거, 아티팩트, 이벤트,
  재실행 행, 상태 버전 효과를 만들지 않습니다.
- 테스트 증거 영속 저장은 제품 파일 쓰기 관찰을 뜻하지 않으면서도 스테이징된 아티팩트를 승격하고 증거를 갱신하며 증거 관찰을 기록할 수 있습니다.
- 정확한 실행 분류는 [`volicord.record_run` 메서드](api/method-record-run.md)가 담당합니다.

현재 닫기 근거 영속 저장 경계:

- 커밋된 `volicord.record_run`은 `tasks.close_basis_revision`을 정확히 한 번 증가시킵니다.
- `close_assessment`가 `null`이 아니면 `tasks.close_basis_json`에 새 현재 `CurrentCloseBasis`를 쓰고 Core가 생성한 불투명 잔여 위험 ID를 저장합니다.
- 그 `CurrentCloseBasis`에 저장되는 민감 동작 요구사항은 커밋된 실행 기록과 소비된 쓰기 티켓 호환성 행에서 Core가 파생하며, 동작, 정규화된 경로, 민감 범주, 기준선, Change Unit, 출처 실행 기록 참조, 출처 쓰기 티켓 참조를 닫기까지 보존합니다.
- 범주만 담은 호출자 입력은 민감 동작 요구사항을 만들거나, 만족하거나, 지울 수 없습니다.
- `close_assessment=null`은 커밋된 실행 기록이 현재 닫기 근거를 만들지 않음을 기록합니다. 기존 현재 근거는 오래되거나 없어집니다.
- Evidence Summary 최신성은 `created_at`, `updated_at`, 불투명 record ID가 아니라
  `produced_at_state_version`으로 결정합니다. 정규 UTC 시계는 권한 커밋 순서를
  대신하지 않습니다.
- 실행 기록, 현재 닫기 근거, 증거 요약, 증거 관찰, capture producer, receipt artifact
  승격/연결, 쓰기 티켓 호환성 소비, replay, event, revision 효과는 원자적으로
  커밋됩니다.

담당 문서:

- [`volicord.record_run` 메서드](api/method-record-run.md)
- [아티팩트 저장소](storage-artifacts.md)
- [저장소 기록](storage-records.md)

<a id="volicordrequest_user_action"></a>
### `volicord.request_user_action`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- `user_action_requests` 행 하나를 생성합니다.
- Core가 canonical 캡처 폼을 도출하는 닫힌 요청과 Core 파생 근거, 현재 근거 상태,
  required-for 대상, 후보, 만료, 정확한 원천 메서드/idempotency 관계를 저장합니다.
- 영향받은 차단 사유를 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

`source_method=volicord.request_user_action`이면 직접 원천
`(project_id, source_idempotency_key)`는 고유합니다. MCP
`request.operation=resume` 분기는 같은 Agent Connection 접근 범위에서 그 행과 불변 원래
replay 응답 전체가 현재의 닫힌 agent-safe 결과 형태로 strict decode된 뒤에만 읽습니다.
그 계약을 위반하는 저장 replay 행은 다시 쓰지 않고
`PERSISTED_DATA_CORRUPT`로 닫힌 상태로 실패합니다. Resume은 요청, event, replay 행, resolution, blocker 갱신, state version을 만들지 않고 영속 정규 UTC 하한도 갱신하지 않습니다.

효과가 없는 분기:

- 유효한 `dry_run` 미리보기
- 거절된 시도

유효한 `dry_run` 미리보기는 아래 항목을 만들지 않습니다.

- 실제 `user_action_request_ref`.
- 대기 사용자 행동.
- 차단 사유 갱신.
- 이벤트.
- 재실행 행.
- `project_state.state_version` 증가.
- 영속 정규 UTC 하한 갱신.

담당 문서:

- [`volicord.request_user_action` 메서드](api/method-request-user-action.md#volicordrequest_user_action)
- [저장소 기록](storage-records.md)

<a id="volicordresolve_user_action"></a>
### `volicord.resolve_user_action`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 변경 불가능한 일대일 `user_action_resolutions` 행 하나를 삽입해 Core 유효 상태 evaluator가 `resolved`를 반환하게 합니다.
- 일치하는 폐쇄형 resolution 본문, channel kind와 submission ID, 파생 local-user 출처, verification basis, assurance level, Core 캡처 시각을 저장합니다. 본문은 선택지에서 도출한 choice 사실 또는 전체 Evidence 관찰 detail을 담습니다.
- 메서드 담당 문서가 선택할 때 수락된 제품, 기술, 범위 결정과 수락된 현재 잔여 위험에 대한 `project_continuity_records`를 생성합니다.
- 종속 차단 사유 또는 다음 행동을 갱신합니다.
- 이벤트를 추가합니다.
- 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

효과가 없는 분기:

- 유효한 `dry_run` 미리보기
- 거절된 시도

검증 실패, 잘못된 binding, 만료, 상태 race, 해결 쓰기 실패를 포함한 거절된 CLI
시도는 resolution을 삽입하거나 영속 정규 UTC 하한을 갱신하면 안 됩니다.

유효한 `dry_run` 미리보기는 아래 항목을 만들지 않습니다.

- 사용자 행동 해결 또는 관찰 detail.
- 프로젝트 연속성 기록.
- 차단 사유 갱신.
- 이벤트.
- 재실행 행.
- `project_state.state_version` 증가.
- 영속 정규 UTC 하한 갱신.

사용자 행동 해결은 `tasks.scope_revision`이나 `tasks.close_basis_revision`을 증가시키지 않습니다.

유효 `status=resolved`는 변경 불가능한 해결이 있다는 뜻이며 그 자체로 수락이나 증거 뒷받침이 아닙니다. Choice 해결에는 저장 선택지에서 파생한 action/outcome이 필요하고 관찰 해결에는 요청이 저장한 정확한 artifact ref를 보존하면서 현재 대상/아티팩트 detail을 검증해야 합니다. 종류별 권한 사실이 빠지면 유효하지 않은 담당 상태입니다.

재실행 행은 계속 `operation_category=user_only`입니다. 그 정확한 응답과 자유
형식 비공개 `note`는 `volicord.get_operation_result`를 통한 Agent Connection
조회 대상이 아닙니다.

담당 문서:

- [`volicord.resolve_user_action` 메서드](api/method-resolve-user-action.md#volicordresolve_user_action)
- [저장소 기록](storage-records.md)

<a id="volicordreconcile_changes"></a>
### `volicord.reconcile_changes`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

- 미해결 `unrecorded_changes` 행을 `status='resolved'`로 설정합니다.
- 해결 근거, 캡처 근거, 해결 메서드, 선택적 연결 사용자 행동 참조를 이름 붙이는 해결 JSON을 저장합니다.
- `resolved_at`과 `resolved_by_actor_source`를 저장합니다.
- 사용자 수락이 필요한 미기록 변경에 대해 `source_method=volicord.reconcile_changes`와 reconciliation idempotency key를 담은 대기 `user_action_requests` 행을 만듭니다.
- 이벤트를 추가합니다.
- 멱등성 키가 있으면 재실행 행을 생성합니다.
- `project_state.state_version`을 한 번 증가시킵니다.

읽기 전용 분기:

- 계획된 해결이나 대기 사용자 행동 생성이 없는 유효한 호출은 응답 데이터만
  반환하고 조정 효과를 만들지 않습니다.

효과가 없는 분기:

- 거절된 시도
- 유효한 `dry_run` 미리보기

이 분기들은 미기록 변경을 해결하거나, 대기 사용자 행동을 만들거나, 이벤트를 추가하거나, 재실행 행을 만들거나, `project_state.state_version`을 증가시키지 않습니다.

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

`dry_run=true`도 `effect_kind=read_only`인 `CloseTaskResult`로 유지됩니다.

효과가 없는 분기:

- 거절된 시도

담당 문서:

- [`volicord.check_close` 메서드](api/method-close-task.md#volicordcheck_close)

<a id="volicordclose_task-intentcomplete"></a>
### `volicord.close_task intent=complete`

커밋되는 `dry_run=false` 호출은 다음을 수행할 수 있습니다.

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

응답 전용으로 차단된 `complete` 결과는 `base.effect_kind=no_effect`를 사용하고 닫기 차단 사유 행, 권한 이벤트, 재실행 행, 종료 상태 변경, 상태 버전 증가를 영속 저장하지 않습니다.

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
