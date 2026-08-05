<a id="volicordrecord_run"></a>

# `volicord.record_run` 참조

## 담당하는 것

이 문서는 기준 범위의 `volicord.record_run` 메서드 동작을 담당합니다.

- 메서드별 필수 입력, 접근 요구사항, 상태 버전 동작, 결과 분기, `dry_run` 동작
- 실행 기록, 현재 닫기 근거 갱신, 증거 갱신, 증거 관찰 기록, 차단 사유 갱신, 아티팩트 승격 메서드 동작
- 실행 기록 예시

## 담당하지 않는 것

이 문서는 아래 항목을 담당하지 않습니다.

- 공통 요청 래퍼, 응답 분기, `dry_run`, 거절 응답 스키마 본문
- 상태, 아티팩트, 값 집합, 오류의 중첩 스키마 정의
- Core의 증거 의미, Core 권한 의미, 저장 DDL, 저장 기록 레이아웃, 정확한 저장 효과, 아티팩트 생명주기, 보안 보장
- 공개 오류 코드 의미, 공개 오류 우선순위, 기계 판독용 오류 세부사항, 공통 응답 분기 처리 경로

## 구현 경로

공개 진입점
[`crates/volicord-core/src/methods/record_run.rs`](../../../../crates/volicord-core/src/methods/record_run.rs)는
요청별 pipeline 조율을 담당하고, 공개 요청을 의미 Recording 입력으로 변환하며,
typed 기록 오류를 매핑하고 의미 결과 fact를 공개 메서드 응답으로 변환합니다.

현재 Record Run 구현 책임은 다음 경로로 나뉩니다.

- [`crates/volicord-core/src/recording/context.rs`](../../../../crates/volicord-core/src/recording/context.rs)는
  요청을 정규화하고 typed Task, Change Unit, workflow, control fact를 취득합니다.
- [`crates/volicord-core/src/recording/authority.rs`](../../../../crates/volicord-core/src/recording/authority.rs),
  [`evidence.rs`](../../../../crates/volicord-core/src/recording/evidence.rs),
  [`artifact.rs`](../../../../crates/volicord-core/src/recording/artifact.rs)는
  공유 증거 및 artifact 정책을 통해 캡처 권한을 해석하고 typed 증거 대상, 관찰,
  artifact plan을 만듭니다.
- [`crates/volicord-core/src/write_ticket/approval.rs`](../../../../crates/volicord-core/src/write_ticket/approval.rs)는
  원시 현재 UserAction 권한 fact를 사용하는 유일한 담당 모듈입니다. 정규 Write
  Ticket 승인 요구사항을 담당하고, typed 현재 민감 승인 집합을 비공개로 구성하며,
  의미 근거 평가를 반환합니다.
  [`current_validity.rs`](../../../../crates/volicord-core/src/write_ticket/current_validity.rs)는
  비승인 현재 fact와 이 typed 평가를 받아 active stored candidate를
  `ReusableStoredWriteTicket`으로 변환하고,
  [`admission.rs`](../../../../crates/volicord-core/src/write_ticket/admission.rs)는 이
  모듈이 로컬에서 읽은 원시 승인 권한을 승인 담당자에게 직접 전달한 뒤 reusable
  타입과 일치하는 정확한 attempt 호환성 증명을 결합해
  `AdmissibleStoredWriteTicket`을 반환합니다. Terminal stored 상태는 이 admission
  경로에 들어갈 수 없습니다.
- [`crates/volicord-core/src/close_readiness/recording.rs`](../../../../crates/volicord-core/src/close_readiness/recording.rs)는
  공유 닫기 준비 상태 서비스가 사용할 typed 현재 닫기 근거와 잔여 위험 fact를
  구성합니다.
- [`crates/volicord-core/src/recording/plan.rs`](../../../../crates/volicord-core/src/recording/plan.rs)는
  이 담당 모듈을 조율하고 typed mutation plan을 조립하며 effect와 결과 fact를
  담은 폐쇄형 `RecordRunOperationPlan`을 반환합니다. 보호 대상 mutation
  planning에는 `AdmissibleStoredWriteTicket`만 전달합니다.
  [`state.rs`](../../../../crates/volicord-core/src/recording/state.rs)는 Store를
  사용하는 연산 후 상태 fact를 취득합니다. 공개 진입점은 반환된 fact를 중립
  Core 연산 carrier와 `RecordRunResultFields`로 변환합니다.

정확한 의존성과 트랜잭션 경계는
[Core 아키텍처](../../architecture-guide/architecture.md),
[요청 생명주기](../../architecture-guide/request-lifecycle.md),
[소스 지도](../../architecture-guide/source-map.md)에 설명되어 있습니다.

## 목적

`volicord.record_run`은 실행과 그 증거를 기록합니다. shaping 분석은 `volicord.record_shaping_checkpoint`으로 기록합니다.

Run 내용이나 종류를 검증하기 전에 Core는 정규화된 현재 `WorkflowSnapshot`을 만들고
정규 `WorkflowMachine`에서 정확한 `volicord.record_run/record_run` 전이를
요구합니다. 이 전이가 현재 mode와 phase 가용성을 담당하고 Task, Change Unit,
baseline 좌표를 고정합니다. 전이가 없으면 효과 없는 `TransitionRejection`을 반환하며,
`recovery_action_key`가 있다면 같은 현재 catalog의 전이입니다.

현재 저장된 `Task.mode`, `work_phase`, 요청한 `kind`는 아래 완전한 행렬과
일치해야 합니다.

| 현재 `Task.mode` | 현재 `work_phase` | 허용되는 `RecordRunRequest.kind` |
|---|---|---|
| `direct` | `implementation` | `direct` |
| `work` | `implementation` | `implementation` |

Core는 그 밖의 모드, 단계, 종류 조합을 커밋 전에 거절합니다. Advisor 결과는 정확한
지속 shaping checkpoint에서 `volicord.finalize_advice`로만
최종화합니다. work shaping 결과는 `volicord.advance_task` 전까지 checkpoint 권한으로
남습니다. advisor Run fallback은 없습니다.

현재 workflow catalog에 그 정확한 전이가 없는 Task는 Run planning 전에 typed
`TransitionRejection`을 받습니다. 허용된 뒤 요청 `kind`가 허용된 전이와 호환되지 않으면
수신한 kind와 폐쇄형 허용 kind 집합을 포함한 `RUN_KIND_INCOMPATIBLE`를 반환합니다.

모든 Run은 확인된 현재 Git 작업 공간 맥락이 현재 Change Unit 쓰기 근거와 일치해야
합니다. 이 규칙은 제품 쓰기 Run뿐 아니라 쓰기가 없는 증거와 닫기 평가 Run에도
적용되므로, 명시적인 `replace_current` 재기준 설정 없이 브랜치, HEAD, worktree를 바꿔
Run 권한을 다른 작업 공간으로 옮길 수 없습니다.

이 메서드는 현재 닫기 근거와 대상별 간결한 증거 범위를 갱신하고, 안정적인
수락 기준 또는 보충 주장 대상에 대한 증거 관찰을 기록하고, 제품 쓰기를
기록하거나 유효 `sensitive` Task의 정확한 승인 비제품 동작을 기록할 때 호환되는
쓰기 티켓을 소비하며, 기존 증거 첨부를 연결하고,
허용되는 경우 적격 스테이징 첨부 입력을 지속 `ArtifactRef`로 승격할 수도
있습니다. 입력 전용 또는 스테이징 전용 항목은 받아들여진 증거가 아니며,
이 메서드가 아래 증거 규칙에 따라 대상, 출처, 첨부 연결 또는 승격을
기록하기 전에는 닫기 준비 상태를 만들지 않습니다.

## 필수 입력

- 유효한 `ToolEnvelope`. 커밋되는 `dry_run`이 아닌 요청에는 `null`이 아닌 `idempotency_key`와 현재 `expected_state_version`이 필요합니다.
- `task_id`, `change_unit_id`, `kind`, `run_id`, `baseline_ref`, `write_ticket_id`, `performed_operation`, `summary`, `observed_changes`, `artifact_inputs`, `evidence_updates`, `evidence_observations`, `close_assessment`.
- `performed_operation`은 필수-null 허용 필드입니다. JSON `null`은 수행 동작이
  없음을 뜻하며, 생략은 유효하지 않습니다. 모든 유효 `sensitive` 비제품 Run은
  바깥쪽 공백을 제거한 뒤 소비할 쓰기 티켓에 저장된
  동작과 정확히 일치하는 비어 있지 않은 값을 제공해야 합니다. 일반 제품 쓰기
  Run은 이 필드를 생략할 수 있습니다.
- 제품 쓰기 Run과 모든 유효 `sensitive` Run은 `volicord.prepare_write`가 발급한
  호환 `status=active` 쓰기 티켓이 필요합니다. 제품 파일 쓰기가 없는 비민감 Run에는
  필요하지 않습니다.
- 새 아티팩트 바이트는 이미 유효한 `StagedArtifactHandle`로 표현되어 있어야 합니다. `volicord.record_run`은 새 바이트를 스테이징하지 않습니다. 이 핸들은 커밋된 실행 결과에서 받아들여지기 전까지 증거 첨부 입력으로 남습니다.
- `supported` 증거 갱신은 대상이 일치하는 `EvidenceObservationInput`, 사용할
  수 있는 대상 일치 증거 관찰 참조, 또는 Core가 증거 관찰을 만들 수 있는
  `EvidenceCoverageUpdate.provenance`로 뒷받침되어야 합니다. 요청 측
  `source_kind`와 `assurance_level`은 주장하는 출처 조합을 선택하며 Core는
  확인된 앵커에서 커밋할 조합을 파생합니다.
- 수락 기준 대상은 이 `Task`의 현재 기준을 식별해야 합니다. 보충 대상은
  호출자가 부여한 `Task` 범위 `EvidenceClaimId`를 사용하며 처음 커밋된 사용
  뒤에는 문장이 바뀌지 않습니다. 필요한 기준은
  `coverage_state=not_applicable`을 거부합니다.

Run 생성, 증거나 닫기 근거 상태 변경, 아티팩트 승격, 쓰기 티켓 소비 전에 Core는
현재 보류 중인 사용자 행동 요청의 `required_for`에 `record_run`이 있고 그 행동
종류, Task, 현재 Change Unit, `scope_revision`, 근거, 영향받는 참조가 이 연산과
일치하면 `DECISION_UNRESOLVED`로 거절합니다. 보류 중인 `sensitive_approval`은
경계가 지정된 행동 범위가 검증된 쓰기 티켓의 연산, 이 Run의 실제 정규화 변경
경로, 민감 범주, 기준선과 겹칠 때만 일치합니다. 정보 제공용 요청과 해결됨,
오래됨, 대체됨, 만료됨, 불일치, 행동 종류 비호환 요청은 Run 기록을 막지
않습니다.

## 요청 스키마

이 메서드는 아래 생성 표의 최상위 `params` 요청 필드를 담당합니다. `envelope`는
[API 코어 스키마](schema-core.md#tool-envelope)의 공통 `ToolEnvelope`이며, 표는
`ToolEnvelope` 필드를 다시 정의하지 않습니다. 필수 여부와 Null 허용 여부는 의미
기반 요청 설명자에서 직접 가져옵니다.

<!-- BEGIN GENERATED: contract-structures api.method.record_run.request[params] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `RecordRunRequest` 필드

| 필드 | 필수 | Null 허용 | 형식 |
|---|---|---|---|
| `artifact_inputs` | 예 | 아니요 | `ArtifactInput[]` |
| `baseline_ref` | 예 | 아니요 | `BaselineRef` |
| `change_unit_id` | 예 | 아니요 | `ChangeUnitId` |
| `close_assessment` | 예 | 예 | `CloseAssessmentInput` |
| `envelope` | 예 | 아니요 | `ToolEnvelope` |
| `evidence_observations` | 예 | 아니요 | `EvidenceObservationInput[]` |
| `evidence_updates` | 예 | 아니요 | `EvidenceCoverageUpdate[]` |
| `kind` | 예 | 아니요 | `RunKind` |
| `observed_changes` | 예 | 아니요 | `ObservedChanges` |
| `performed_operation` | 예 | 예 | `string` |
| `run_id` | 예 | 예 | `RunId` |
| `summary` | 예 | 아니요 | `string` |
| `task_id` | 예 | 아니요 | `TaskId` |
| `write_ticket_id` | 예 | 예 | `WriteTicketId` |
<!-- END GENERATED: contract-structures api.method.record_run.request[params] -->



중첩 형태 담당 문서:
- `observed_changes`, `evidence_updates`, `evidence_observations`는
  `ObservedChanges`, `EvidenceCoverageUpdate`, `EvidenceObservationInput`을
  사용합니다. 이 형태는 [API 상태 스키마](schema-state.md#evidence-and-run-snapshot-shapes)가 담당합니다.
- `close_assessment.result_refs`와 `ResidualRiskInput.source_refs`는 [API 상태 스키마](schema-state.md#state-references)가 담당하는 `StateRecordRef`를 사용합니다.
- `CurrentCloseBasis`와 커밋된 `ResidualRisk` 출력 형태는 [API 상태 스키마](schema-state.md#close-readiness-and-validation-shapes)가 담당합니다. `ResidualRiskInput`에는 호출자 권한의 `risk_id`가 없습니다. Core는 새 현재 닫기 근거를 커밋할 때 불투명 `risk_id` 값을 생성합니다.
- `artifact_inputs`는 `ArtifactInput[]`을 사용합니다. `ArtifactInput`, `StagedArtifactHandle`, `ArtifactRef` 형태는 [API 아티팩트 스키마](schema-artifacts.md#artifactinput)가 담당합니다.
- `kind`, 아티팩트 출처 값, `redaction_state`, 증거 범위 값은 [API 값 집합](schema-value-sets.md)이 담당합니다.

경로와 접근 참고:
- `null`이 아닌 `performed_operation`은 바깥쪽 공백만 제거해 정규화합니다.
  Core는 대소문자 변환, 의미 기반 일치, `summary` 값 대체를 하지 않습니다.
  티켓을 소비하는 Run이 이 필드를 제공하면 정규화한 값이 티켓의 정규화된
  `WriteTicketAttemptScope.intended_operation`과 정확히 같아야 하며, 유효
  `sensitive` 비제품 Run은 이 필드를 생략할 수 없습니다.
- `observed_changes.changed_paths` 항목은 `Product Repository` API 제품 경로입니다. `Product Repository` 경로 정규화는 [런타임 경계](../runtime-boundaries.md#product-repository-api-path-normalization)가 담당합니다.
- `ArtifactInput[]`와 스테이징 핸들은 두 번째 요청 수준 작업 범주나 행위자 출처를 만들지 않습니다. 호출은 확인된 호출 맥락의 값으로 유지됩니다.
- `ArtifactInput[]` 멤버는 증거 첨부 입력입니다. 선택적
  `evidence_target`은 범위와 관찰이 쓰는 것과 같은 태그 대상 identity를
  사용합니다. 이 메서드가 그 입력을 대상별 증거나 관찰에 연결할 때만
  증거를 뒷받침합니다. 요청 안에 있다는 사실만으로 증거가 충분해지지는
  않습니다.
- `EvidenceObservationInput.source_refs`와 `EvidenceUpdateProvenance.source_refs`는 구조가 검증된 권한 효력이 없는 출처를 보존합니다. Core는 이 참조를 위해 파일을 읽거나, Git 객체를 해석하거나, 명령을 실행하거나, URI를 가져오거나, 메시지를 조회하지 않습니다. 선택적인 명령 또는 Git diff 아티팩트 참조는 이 프로젝트와 Task가 소유하는 기존 기준 아티팩트와 일치해야 합니다. 출처 참조는 증거 충분성이나 닫기 권한을 만들지 않습니다.
- `EvidenceObservationInput.observed_by_actor_source`는 권한 효력이 있는 입력이
  아닙니다. Core는 검증된 producer 레코드가 있으면 그 레코드에서 커밋된
  관찰자를 파생하고, 그렇지 않으면 확인된 호출 맥락에서 파생합니다.
- capture-backed 관찰은 `input_refs`에 현재
  `record_kind=evidence_capture_intent` ref를 정확히 하나 둡니다.
  `observed_by_actor_source`, `tool_name`, `tool_invocation_id`는 null로 두고,
  `tool_metadata`, `source_refs`, `output_artifact_refs`, `limitations`는 비워 둡니다.
  문법상 유효한 `observed_at`은 계속 제공하지만 capture-backed input에서는 Core가
  그 호출자 값을 무시하고, 앞에서 열거한 필드와 함께 저장된 receipt fact로
  대체합니다. Command와 tool capture는 `external_tool` /
  `external_tool_result`를 요청하고 registered connection capture는
  `connection_observation` / `registered_connection_observed`를 요청합니다.

닫기 평가 참조 규칙:
- 호출자가 제공한 `close_assessment.result_refs`와 `ResidualRiskInput.source_refs`는 담당 문서가 다른 종류를 명시적으로 추가하지 않는 한 `record_kind=run`, `artifact`, `evidence_summary`, `change_unit`으로 제한됩니다.
- 담당 문서가 명시적으로 추가하지 않는 한 이 메서드는 호출자가 제공한 `project_state`, `write_ticket`, `user_action_request`, `user_action_resolution`, `blocker`, `task_event`, `task` 참조를 닫기 근거에서 거절하거나 제외합니다.
- 받아들인 모든 참조는 존재해야 하고 같은 프로젝트와 `Task`에 속해야 합니다. 아티팩트 참조는 `Task`에 연결되어 있고 `integrity_status=verified`로 현재 바이트 검증을 통과해야 합니다. 증거 참조는 현재 `Task` 증거 요약을 식별해야 합니다. 현재 닫기 근거 결과 참조로 쓰이는 실행 기록 참조는 현재 `Task`, 현재 적용 Change Unit, 현재 범위 리비전, 호환되는 기준선, 기록된 상태와 호환되는 기록된 현재 실행 기록을 식별해야 합니다.
- 이력 실행 기록 참조는 이 새 현재 실행 기록이 이력의 `verified` 아티팩트나 증거를 명시적으로 재사용하고 그 재사용을 커밋된 증거나 닫기 평가에 기록하지 않는 한 닫기 근거 용도에서는 감사 기록입니다.
- Core는 `CurrentCloseBasis`에 기준 참조를 저장하며 호출자가 보낸 `produced_at_state_version` 메타데이터를 권한이나 동시성 입력으로 취급하지 않습니다.
- Core는 기준 닫기 근거를 만들면서 현재 실행 기록, 현재 Change Unit, 현재 EvidenceSummary 참조를 추가할 수 있습니다.

증거 갱신 출처 규칙:
- `coverage_state=supported`는 범위에 대한 주장이지 그 자체로 충분한 출처가 아닙니다.
- `supported` 항목에 `EvidenceCoverageUpdate.provenance`가 제공되고 대상이
  일치하는 명시적 관찰 입력이 없으면 Core는 현재 실행 기록에 대한
  `EvidenceObservation`을 만들고 그 참조를 커밋된 증거 요약에 연결합니다.
- 요청 측 `source_kind`와 `assurance_level`은 유효한 조합이어야 하지만 그
  조합만으로 더 강한 출처를 스스로 선언할 수 없습니다. Core는 커밋할 조합을
  다음과 같이 파생합니다.
  - 정규 아티팩트는 바이트 identity와 현재 무결성만 증명합니다. 누가 그
    바이트를 만들었는지 또는 해당 대상을 뒷받침하는지는 증명하지 않습니다.
  - `user_observation` / `user_observed`는 `input_refs`가
    [`volicord.resolve_user_action`](method-resolve-user-action.md)이 만든
    현재의 대상 결합 `evidence_observation UserActionResolution`을 식별하고 정확한 출력
    아티팩트가 일치할 때만 유지됩니다. Core는 로컬 사용자 actor, 검증 근거,
    relevance, Task, Change Unit, scope, baseline, 대상, 현재 바이트를 다시
    확인합니다. Resolution에 저장된 정확한 `supported` 또는 `contradicted` relevance를
    커밋된 `relevance_assessment`에 보존하고, 호출자가 제공한
    `EvidenceObservationInput.observed_at`을 바깥 resolution의 `resolved_at`으로
    대체합니다. 두 relevance 상태 모두 로컬 사용자 producer provenance를 유지합니다.
    `contradicted`는 부정적 relevance로 남으며 supported coverage나 증거 충분성을
    세울 수 없습니다.
  - 현재 capture intent와 일치하는 완전한 receipt가 있으면 Core가 verified
    command, verified tool invocation, registered connection observation producer를
    finalization할 수 있습니다. 그 정확한 intent ref가 없으면 직접
    `external_tool`과 `connection_observation` 요청은 연결된 아티팩트를 사용할 수
    있고 무결성이 검증되어도 계속 강등됩니다. 설명용 tool 필드, `SourceRef`,
    staging 메타데이터, raw guard payload는 저장된 receipt를 대신할 수 없습니다.
  - 호출자가 직접 제출한 `reused_evidence`는 검증된 재사용 경로가 아닙니다.
  - 입증되지 않은 강한 주장은 `agent_report` / `cooperative_report`로 커밋됩니다.
    `unverified_claim` / `unverified`는 확인되지 않은 상태로 유지됩니다.
- `supported` 갱신이 강하고 사용할 수 있으며 대상이 일치하는
  `observation_refs`에만 의존하면 Core는 현재 실행 기록에
  `source_kind=reused_evidence` 관찰을 기록합니다. 그 단일 `input_ref`는 원래 관찰
  참조를 보존하므로 이력 관찰을 현재 관찰로 다시 이름 붙이지 않고 출처 입력으로
  유지합니다. 재사용 관찰은 원래 관찰의 정확한 정규 아티팩트 출력과 재사용
  한계를 담습니다. 현재 갱신이 승계된 producer chain에 다른 바이트를 대입할
  수 없습니다.
- 이 재사용 관찰을 만들기 전에 Core는 원래 관찰의 identity와 대상, `Task`와
  Change Unit 소유권, 출처 실행 기록, 현재 범위 리비전과 기준선, 승계한 보장
  수준, producer 앵커, 정확한 출력, 분리된 relevance 평가를 다시 검증합니다.
  각 재귀 단계는 저장된 권한 메타데이터를 엄격히 decode하고 같은 현재 앵커가
  있는 보장 수준으로 이어져야 합니다. stale, 누락, 모순, 손상, 불일치, 출력
  대체, 순환 체인은 거부됩니다.
- 위 거부 목록의 `contradicted`는 supported 재사용 규칙이며 producer provenance
  강등 규칙이 아닙니다. `contradicted` relevance를 가진 현재의 정확한 User Channel
  관찰은 `user_observation` / `user_observed`로 남지만 `supported`를 세우는 검증된
  재사용 자격을 얻을 수 없습니다.
- 위의 모든 strong `user_observation` 및 검증된 재사용 검사에서 정확한 출력 집합은
  비어 있지 않고 `artifact_id` 값이 서로 달라야 합니다. Core는 중복 아티팩트 ID를
  중복 제거하지 않고 거부합니다. 각 이력 출력은 현재 정규 `ArtifactRef`의
  `artifact_id`, `project_id`, `task_id`, `display_name`, `content_type`, `sha256`,
  `size_bytes`, `integrity_status`, `redaction_state`, `availability`,
  `created_by_run_ref` 존재 여부와 identity(`record_kind`, `record_id`, `project_id`,
  `task_id`), `created_by_actor_source`, `storage_ref`가 모두 일치해야 합니다. 이력 ref와
  현재 정규 상태 보기를 비교할 때 허용되는 유일한 정규화는 중첩
  `created_by_run_ref.produced_at_state_version`을 새 상태 보기 기준으로 바꾸는 것입니다.
  이 필드는 상태 보기 최신성만 나타내며 권한이나 동시성을 부여하지 않습니다. 그 밖의
  typed 필드는 무시하거나 새 기준으로 바꾸거나 대체할 수 없습니다. 중복이나 불일치는
  커밋 전에 요청을 거부합니다. `dry_run`도 같은 검증을 수행하며 어느 분기도 strong
  provenance나 다른 효과를 기록하지 않습니다.
- `supported`가 아닌 상태에서는 대상이 일치하는 현재 협력적 또는 확인되지 않은
  관찰 참조를 설명용 뒷받침으로 보존할 수 있습니다. 강한 재사용 요구는 참조로
  `supported`를 세울 때만 적용됩니다.
- 커밋된 `source_kind`와 `assurance_level`은 호출자가 부여한 보장이 아니라
  Core가 파생한 출처 분류를 담습니다.
- `unverified_claim`, `unverified`, 협력적 `agent_report` 관찰은 증거 관찰로 기록될 수 있지만, 더 강한 출처가 필요할 때 닫기 준비 상태에서는 약한 출처로 평가됩니다.
- 증거 관찰은 사용자 소유 판단, 최종 수락, 잔여 위험 수락, 닫기 준비 상태를 대신하지 않습니다.

Capture-backed 관찰 규칙:

- Core는 intent와 그 불변 receipt 하나를 직접 읽고 현재 프로젝트, Task, Change
  Unit, scope revision, baseline, target, workspace, connection/actor, 만료, 정확한
  digest, receipt 바이트, 완전성, redaction 상태를 다시 검증합니다. 누락, stale,
  만료, 이미 소비됨, cross-scope, 손상된 intent 또는 receipt는 커밋 전에
  거부합니다.
- Core는 크기가 제한된 안전한 receipt staging handle을 자동으로 승격하고, 생성된
  artifact를 새 `EvidenceProducer`에 연결하며, producer와 일대일
  `EvidenceObservation`을 저장된 source fact 및 Run과 함께 원자적으로 만듭니다.
  호출자가 제공한 output ref나 메타데이터로 이 fact를 대체할 수 없습니다.
- 관찰 outcome이 intent expectation과 같으면 강한 producer provenance를 만들고
  `relevance_assessment.status=unassessed`로 기록합니다. 등록된 실행이나 관찰은 그
  결과가 선택한 대상을 뒷받침한다고 판단하지 않으므로 capture-backed 관찰만으로
  필요한 기준을 충분하게 만들 수 없습니다. `supported`에는 별도의 담당 문서가
  정의한 relevance 권한이 필요합니다. 완전하지만 저장된 expectation과 일치하지 않는
  outcome은 `contradicted`로 보존하며 협력적 성공 주장으로 조용히 바꾸지 않습니다.
  두 capture 분류 모두에서 `assessment_ref`는 분류 근거인 불변 capture intent를
  가리키고 `assessed_by_actor_source=null`입니다. 이 ref는 독립 relevance 권한이
  아니며 `unassessed`를 `supported`로 바꿀 수 없습니다.
- capture-intent ref가 없는 input은 기존 unanchored downgrade와 검증된 재사용
  규칙을 그대로 사용합니다.

## 접근 요구사항

요구사항:

- `operation_category=agent_workflow`인 확인된 호출 맥락

`ArtifactInput.source_kind=staged_artifact`인 경우:

- 현재 확인된 `actor_source`가 스테이징 핸들의 기록된 출처와 일치해야 합니다.

기록된 출처는 스테이징 시점의 확인된 호출 맥락에서 캡처된 것입니다. 이 메서드는 호출자가 제출한 출처를 권한 근거로 받아들이지 않고, 그 기록된 출처를 현재 확인된 맥락과 비교합니다.

비주장:

- `ArtifactInput[]`는 `artifact_registration`을 추가하지 않습니다.
- 행위자 사이의 스테이징 핸들 전달은 기준 범위 밖입니다.

## 상태 버전 동작

호환되는 커밋 결과는 `project_state.state_version`을 정확히 한 번 올립니다.

호환되는 커밋 결과는 선택된 `Task.close_basis_revision`을 정확히 한 번 증가시킵니다. `close_assessment`가 `null`이 아니면 커밋은 커밋된 현재 실행 기록, 평가 필드, 생성된 잔여 위험 ID, 현재 Task, 현재 적용 Change Unit, 선택된 현재 범위 리비전, 호환되는 기준선에서 새 `CurrentCloseBasis`를 만듭니다. `close_assessment=null`이면 커밋된 실행 기록이 현재 닫기 근거를 만들지 않음을 명시하며, 기존 현재 닫기 근거는 오래되거나 없어집니다.

빈 `close_assessment.residual_risks` 목록은 현재 결과에 식별된 잔여 위험이 없다는 명시적 의미입니다. Core는 커밋된 `null`이 아닌 평가에 대해서만 불투명 `risk_id` 값을 생성합니다. `dry_run`은 지속 `risk_id` 값을 예약하지 않습니다.

결과 `CurrentCloseBasis` 안의 민감 동작 요구사항은 커밋된 실행 기록과 소비된 쓰기 티켓에서 Core가 파생합니다. `close_assessment.sensitive_categories` 안의 범주만 담은 호출자 입력은 표시 맥락에는 기여할 수 있지만 민감 승인 요구사항을 만들거나, 만족하거나, 지울 수 없습니다.

유효 통제 수준이 `sensitive`인 Task는 Run이 제품 파일 쓰기를 기록하지 않아도 호환
티켓이 필요합니다. 이 티켓은 정확한 동작과 승인 근거이며
`product_file_write_intended=false`, Run의 빈 제품 경로 관찰, 현재 사용자 소유 민감
승인에 일치해야 합니다. 일치하는 Run은 티켓을 소비하고 동작, Change Unit, 범위,
기준선, 승인에 결속된 민감 동작 요구사항을 닫기까지 보존합니다. 제품 파일 쓰기가
없는 일반 비민감 Run에는 계속 티켓이 필요하지 않습니다.

범주만 담은 `observed_changes.sensitive_categories`는 Core가 확인한 승인 근거가 아니라 호출자 보고입니다. 이 입력만으로 Task의 유효 통제 수준을 높이거나 민감 동작 승인 권한을 만들지는 않습니다. 대신 Task의 수락 정책을 같은 트랜잭션에서 `required`로 강화하므로 정책 의존 `light` 자동 닫기가 이 신호를 소모할 수 없고 현재 최종 수락은 계속 필수입니다. Core가 확인한 `sensitive` 통제 근거에는 일치하는 사용자 승인과 최종 수락이 모두 필요하며, 범주만 담은 입력은 어느 쪽도 제공할 수 없습니다.

성공한 Run, 그 닫기 평가, 또는 이후 최종 수락을 기록해도 누락된 쓰기 전 승인이
보완되지 않으며 쓰기에 소급해 권한을 부여하지 않습니다. 현재 정책이 `sensitive`
통제와 민감 동작 승인을 요구하면 `record_run`은 Run을 만들기 전에 이미 승인되었고
현재 정책에 결속된 티켓을 요구합니다.

실행 기록, 현재 닫기 근거, 증거 갱신, 증거 관찰, 아티팩트 연결 또는 승격, 쓰기 티켓 소비, 리비전 변경은 결과가 커밋될 때 원자적으로 커밋됩니다.

티켓 결속 실행 기록이 쓰기 티켓을 소비하려면 아래 조건을 모두 만족해야 합니다.

- 티켓이 `status=active`이고 이미 소비되거나 철회되지 않았습니다.
- `WriteTicketValidityBasis`가 현재 `task_id`, `change_unit_id`,
  `scope_revision`, 기준선, workspace digest와 계속 일치합니다. Store가 검증한
  승인 근거는 정규 typed 승인 평가에서 현재 또는 불필요 결과를 받아야 합니다.
  Admission은 원시 UserAction 권한을 로컬에서 읽어 승인 담당자에게 직접 전달하고
  현재 유효성 fact에는 보관하지 않습니다.
- null이 아닌 `write_authority_fingerprint`가 현재 권위 프로젝트 정책에서 독립적으로
  다시 읽어 계산한 fingerprint와 정확히 일치합니다. Store는 티켓 소비 트랜잭션
  안에서 같은 결속을 다시 확인합니다.
- 프로젝트 정책이 선택한 선택적 `idle_expires_at` 경계를 지나지 않았습니다.
  기본 유휴 제한 시간은 `null`입니다.
- 티켓과 그 `WriteTicketAttemptScope`가 기록하려는 Run과 같은 `task_id`와 `change_unit_id`를 식별합니다.
- `WriteTicketAttemptScope`가 포착한 시도의 `product_file_write_intended`가 Run의 제품
  파일 쓰기 관찰 여부와 정확히 일치합니다. 비제품 `sensitive` Run은 `false`를 사용합니다.
- `WriteTicketAttemptScope`가 포착한 시도의 `baseline_ref`가 Run의 `baseline_ref`와 일치합니다.
- 제공된 `performed_operation`이 포착한 시도의 정규화된 `intended_operation`과
  정확히 일치합니다. 유효 `sensitive` 비제품 Run에는 이 필드가 필수입니다.
- 확인된 현재 Git 작업 공간 맥락이 티켓 발급 시 포착한 현재 Change Unit 쓰기 근거와
  계속 정확히 일치합니다. 발급 뒤 브랜치, HEAD, worktree, 지문이 바뀌면 소비를 거절합니다.
- 관찰된 민감 범주가 포착한 시도의 정규화된 `sensitive_categories`와 일치합니다.
- 제품 파일 쓰기라면 `Product Repository` 경로 정규화 뒤의 관찰된 변경 경로가
  포착한 시도와 호환됩니다. 비제품 민감 Run은 제품 변경 경로를 기록하지 않습니다.

티켓은 상태/닫기 확인, 증거 기록, 진단, 과거 operation 결과 조회, 관련 없는
사용자 행동, 커밋된 비허용 쓰기 준비 결정을 포함한 관련 없는 `state_version`
변경 뒤에도 유효합니다. `WriteTicket.basis_state_version`은 발급 순서만 기록합니다.

형식이 올바른 승인이 만료되거나 더 이상 현재 상태가 아니면 의미 기반
`approval_basis_changed` 결과가 됩니다. 영속 승인 참조의 소유자 불일치, 필수
참조 metadata 누락, 완전한 resolution identity 중복은 Store 손상이며 이 승인
policy까지 도달할 수 없습니다. 평가는 승인이 새로 필요한 경우, 현재 resolution
부재, 승인 범위 변경, 영속 근거 resolution이 더 이상 현재 상태가 아닌 경우를
구분합니다. 정규 담당자는 Record Run admission에 원시 UserAction 권한 fact를
노출하지 않고 typed 평가를 반환합니다. Record Run admission은 승인 참조 identity를
독립적으로 다시 구성하거나 비교하지 않습니다.

오래된 `expected_state_version`은 일반 요청 충돌 우선순위에 따라 거절합니다.
티켓 근거는 별도로 검증하며 전역 상태 버전 차이만으로 티켓에
`STATE_VERSION_CONFLICT`를 만들지 않습니다.

선택적 유휴 경계가 설정되면 문자열 사전식 비교가 아니라 파싱한 UTC
타임스탬프로 계산합니다. 이 경계로 무효화된 티켓은 소비하지 않고
`ToolError.details.write_ticket_reason=idle_timeout`과 함께
`WRITE_TICKET_INVALID`를 반환합니다.

저장된 무효화는 저장 사유 `scope_revision_changed`, `change_unit_changed`,
`baseline_changed`, `workspace_changed`, `approval_basis_changed`,
`idle_timeout`, `task_closed`, `explicit_revoke`와 함께 `WRITE_TICKET_INVALID`를
사용합니다. 시도 호환성 불일치는 `task_mismatch`, `change_unit_mismatch`,
`product_write_flag_mismatch`, `baseline_mismatch`,
`operation_mismatch`, `workspace_context_mismatch`, `sensitive_category_mismatch`, `path_mismatch`
같은 메서드 로컬 세부 값을 사용합니다.

## 메서드 결과 필드

`RecordRunResult`는 커밋된 실행 기록 작업에 대한 메서드별 결과 분기입니다. 이 결과는 결과 효과로 `core_committed`만 허용하는 `base: RecordRunResultBase`와 아래 메서드 소유 최상위 필드를 담습니다.

<!-- BEGIN GENERATED: contract-structures api.method.record_run.response[response_variants] api.method.record_run.response[result_body] api.method.record_run.response[result_metadata] api.method.record_run.response[rejection] api.method.record_run.response[dry_run] -->
<!-- Generated by `cargo run -p xtask -- docs-sync`; do not edit this region. -->

### `RecordRunResult` 성공 필드

| 필드 | 필수 | Null 허용 | 형식 |
|---|---|---|---|
| `base` | 예 | 아니요 | `RecordRunResultBase` |
| `blocker_refs` | 예 | 아니요 | `StateRecordRef[]` |
| `current_close_basis` | 아니요 | 예 | `CurrentCloseBasis` |
| `evidence_observations` | 예 | 아니요 | `EvidenceObservation[]` |
| `evidence_producers` | 예 | 아니요 | `EvidenceProducer[]` |
| `evidence_summary` | 아니요 | 예 | `EvidenceSummary` |
| `registered_artifacts` | 예 | 아니요 | `ArtifactRef[]` |
| `run_summary` | 예 | 아니요 | `RunSummary` |
| `state` | 예 | 아니요 | `StateSummary` |

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

- `volicord.record_run`: `dry_run=true`가 `ToolDryRunResponse` 미리보기 분기를 선택하며, 이 분기의 `base.dry_run`은 `true`입니다. `dry_run=false`이거나 `dry_run`이 생략되면 미리보기 분기를 선택하지 않습니다.


### 공유 응답 구조

응답 설명자는 성공, 거절, 미리보기를 정확한 `anyOf` 분기 union으로 정의합니다. 거절 분기는 생성된 [`ToolRejectedResponse`](schema-core.md#common-response) 구조를 사용합니다. 메서드 동작이 미리보기 분기를 선택할 때는 생성된 [`ToolDryRunResponse`](schema-core.md#common-response) 구조를 사용합니다. 공유 거절 및 미리보기 필드는 위 성공 필드와 구분된 상태로 유지됩니다.
<!-- END GENERATED: contract-structures api.method.record_run.response[response_variants] api.method.record_run.response[result_body] api.method.record_run.response[result_metadata] api.method.record_run.response[rejection] api.method.record_run.response[dry_run] -->

MCP compact 결과는 `evidence_observation_refs`와 함께
`evidence_producer_refs`를 보존하며 full detail은 정확한 producer 본문을 담습니다.
응답 budget 때문에 full detail이 생략되면 영속 operation-result 경로로 정확한
`RecordRunResult`를 복구합니다.

중첩된 `StateRecordRef`, `RunSummary`, `ObservedChanges`, `EvidenceSummary`, `EvidenceCoverageItem`, `EvidenceObservation`, `EvidenceProducer`, `StateSummary`, `ArtifactRef` 필드 본문은 위에 연결된 스키마 담당 문서에 둡니다. 스테이징 핸들 소비, 아티팩트 승격, 증거 갱신, 증거 관찰 기록, 재실행 행, 쓰기 티켓 소비를 포함한 정확한 지속 효과는 [저장 효과](../storage-effects.md)와 [아티팩트 저장소](../storage-artifacts.md)에 둡니다.

## 성공 결과

커밋된 `RecordRunResult`는 `base.response_kind=result`와
`base.effect_kind=core_committed`를 사용합니다. 아티팩트가 존재한다는 사실만으로
증거 충분성이 성립하지 않으며, null이 아닌 닫기 근거는 이 Run이 현재 닫기 근거를
만들었다는 뜻입니다.

## 차단 결과

실행 자체는 기록 가능하지만 결과가 증거 공백 같은 차단 사유를 만들거나 유지할 때 호환되는 실행 관련 차단 사유 상태를 커밋할 수 있습니다.

허용되지 않는 것:

- 커밋된 차단 결과는 유효하지 않은 스테이징 핸들, 누락되거나 무효화된 쓰기
  티켓, 오래된 요청 상태, 호출 맥락 실패를 숨기면 안 됩니다.

위 경우는 커밋 전에 거절됩니다.

## 거절 결과

아래 경우는 `ToolRejectedResponse`를 반환합니다.

- work Task가 아직 shaping이면 `TASK_PHASE_TRANSITION_REQUIRED`
- 그 밖의 `kind`, mode, phase 불일치에는 `RUN_KIND_INCOMPATIBLE`
- 오래된 `expected_state_version`
- 무효화된 쓰기 티켓 유효성 근거
- 누락되거나 일치하지 않는 쓰기 티켓 정책 권한 결속
- 오래되었거나 일치하지 않는 현재 Git 작업 공간 맥락
- 제품 쓰기에 필요한 쓰기 티켓 누락 또는 무효
- 선택적 유휴 제한 시간으로 무효화된 쓰기 티켓
- 쓰기 티켓 동작, 경로, 기준선, 제품 쓰기 플래그, 민감 범주, Task, Change Unit 비호환
- 유효하지 않은 스테이징 핸들
- 스테이징 핸들 출처 불일치
- 누락, 만료, 이미 소비됨, stale, cross-scope, 손상된 evidence-capture intent 또는 receipt
- capture intent/receipt의 source, digest, 바이트, 완전성, redaction, outcome,
  target, connection 불일치
- 필요한 관찰 출처가 없는 `supported` 증거 갱신
- 누락된 아티팩트
- 범위 위반
- 오래된 기준선
- 행위자 출처 또는 작업 범주 불일치
- 지원되지 않는 호출 맥락
- 검증기 실패

비주장: 유효하지 않은 스테이징 핸들은 [API 오류 세부사항](error-details.md#artifact-input-error-reason)이 담당하는 아티팩트 입력 세부정보가 있는 검증 실패입니다. 요청 호출 자체가 실패한 경우가 아니라면 호출 맥락 불일치가 아닙니다.

공개 오류 코드 의미, 우선순위, 세부사항, 거절 응답 처리 경로는 아래 오류 담당 문서가 담당합니다.

무효화된 쓰기 티켓 근거에서는 소비 전에 거절되며 Run, 증거 갱신, 증거 관찰, 아티팩트 연결, 아티팩트 승격, 이벤트, 재실행 행, `project_state.state_version` 증가를 만들지 않습니다.

그 밖에는 활성인 티켓의 정책 권한 결속이 없거나 일치하지 않으면
`ToolError.details.write_ticket_reason=policy_authority_mismatch`와 함께
`WRITE_TICKET_INVALID`를 반환합니다. 티켓을 소비하거나 권한 있는 Run을 기록하지
않으며, 이 검사는 Guard가 쓰기를 관찰하거나 거부했는지에 의존하지 않습니다. 일반
정책 적용은 먼저 `status=invalidated,invalidation_reason=explicit_revoke`를 영속하므로,
이미 무효화된 그 행을 나중에 사용하려는 시도는 상태 우선순위에 따라
`explicit_revoke`를 보고합니다.

필수 `performed_operation`이 누락되거나 값이 일치하지 않아도 티켓 소비 전에
거절되며 위 효과를 만들지 않습니다.

유휴 제한 시간으로 무효화된 쓰기 티켓에서는 소비 전에 거절되며 Run, 이벤트, 재실행 행, 아티팩트 승격, 증거 갱신, 증거 관찰, 쓰기 티켓 소비, `project_state.state_version` 증가를 만들지 않습니다.

mode, phase, Run-kind workflow 거절도 Run, 닫기 근거 리비전, 증거 갱신, 증거 관찰, 아티팩트 연결이나 승격, 이벤트, 재실행 행, 쓰기 티켓 효과, 상태 버전 증가를 만들지 않습니다.

## `dry_run` 동작

`dry_run=true`에서 유효한 미리보기:

- `ToolDryRunResponse`를 반환합니다.
- Run, 현재 닫기 근거, 잔여 위험 ID, 증거 갱신, 증거 관찰, 차단 사유 갱신, 아티팩트 연결, 아티팩트 승격, 쓰기 티켓 소비를 만들지 않습니다.

## 저장 효과

커밋 시 실행, 현재 닫기 근거, 증거 요약, 증거 관찰, 차단 사유, 쓰기 티켓 소비,
아티팩트 연결 결과를 지속할 수 있습니다. capture-backed 관찰은 같은 트랜잭션에서
receipt artifact도 승격하고 불변 producer를 만듭니다. 정확한 저장 효과와 아티팩트
승격 세부사항은 아래 저장 담당 문서가 담당합니다.

아래 예시는 메서드 안에서만 성립하도록 짧게 구성했습니다. 대표 응답은 커밋된 실행, 승격된 아티팩트 참조, 갱신된 증거 요약, 증거 관찰, 차단 사유 참조, 상태 버전, 현재 상태 스냅샷을 보여 주는 데 필요한 필드로 축약했습니다.


## 최소 유효 요청

이 예시는 이 메서드 문서 안에서 전제로 둔 스테이징된 핸들의 검증 출력을 기록합니다. 메서드 안의 전제: `staged_runprobe_001`은 만료되지 않았고 소비되지 않았으며 `proj_runprobe_001` / `task_runprobe_001`에 속합니다. 스테이징 시점에 캡처된 기록된 행위자 출처는 `agent_connection:conn_run_probe`입니다. 대상과 연결된 스테이징 아티팩트는 바이트 무결성을 설정하지만 요청에 capture-intent producer 앵커가 없으므로 요청한 외부 도구 분류는 협력적 agent report로 커밋됩니다. 요청은 `observed_by_actor_source=null`로 두며 응답은 확인된 호출에서 파생된 행위자 출처를 보여 줍니다. 이 전제는 이 문서의 예시 안에서만 성립하며 다른 메서드 예시를 재사용하지 않습니다.

```yaml contract=api.method.record_run.request shape=complete_request
method: volicord.record_run
params:
  envelope:
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    request_id: req_runprobe_001
    idempotency_key: idem_runprobe_001
    expected_state_version: 31
    dry_run: false
    locale: ko-KR
  task_id: task_runprobe_001
  change_unit_id: cu_runprobe_001
  kind: implementation
  run_id: null
  baseline_ref: baseline_runprobe_001
  write_ticket_id: null
  performed_operation: null
  summary: "검색 결과 수 검증을 통과했습니다."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_runprobe_001
  artifact_inputs:
    - artifact_input_id: artifact_input_runprobe_001
      source_kind: staged_artifact
      staged_artifact_handle:
        handle_id: staged_runprobe_001
        project_id: proj_runprobe_001
        task_id: task_runprobe_001
        created_by_actor_source: agent_connection:conn_run_probe
        content_type: application/json
        sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
        size_bytes: 96
        redaction_state: none
        expires_at: "2030-01-01T00:00:00Z"
        consumed: false
      existing_artifact_ref: null
      relation_hint: "validation_report"
      evidence_target:
        target_kind: acceptance_criterion
        acceptance_criterion_id: criterion_runprobe_count_001
      expected_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
      expected_size_bytes: 96
      redaction_state: none
  evidence_updates:
    - target:
        target_kind: acceptance_criterion
        acceptance_criterion_id: criterion_runprobe_count_001
      coverage_state: supported
      supporting_run_refs: []
      observation_refs: []
      supporting_artifact_refs: []
      gap_refs: []
  evidence_observations:
    - target:
        target_kind: acceptance_criterion
        acceptance_criterion_id: criterion_runprobe_count_001
      source_kind: external_tool
      assurance_level: external_tool_result
      observed_by_actor_source: null
      tool_name: "search-count-validator"
      tool_invocation_id: null
      tool_metadata:
        validator: "search-count"
      input_refs: []
      source_refs: []
      output_artifact_refs: []
      limitations: []
      observed_at: "2026-07-28T11:59:00Z"
  close_assessment:
    result_summary: "검색 결과 수 검증을 통과했습니다."
    result_refs: []
    residual_risks: []
    sensitive_categories: []
    recovery_constraints: []
```

## 대표 응답

결과 분기(`RecordRunResult`, 커밋됨):

```schema
base:
  response_kind: result
  effect_kind: core_committed
  dry_run: false
  state_version: 32
  events:
    - event_id: evt_runprobe_001
      event_kind: run_recorded
run_summary:
  run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    produced_at_state_version: 32
  kind: implementation
  summary: "검색 결과 수 검증을 통과했습니다."
  observed_changes:
    changed_paths: []
    product_file_write_observed: false
    sensitive_categories: []
    baseline_ref: baseline_runprobe_001
  artifact_refs:
    - artifact_id: artifact_runprobe_report_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      display_name: "search-result-count-validation.json"
      content_type: application/json
      sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
      size_bytes: 96
      integrity_status: verified
      redaction_state: none
      availability: available
      created_by_run_ref:
        record_kind: run
        record_id: run_runprobe_001
        project_id: proj_runprobe_001
        task_id: task_runprobe_001
        produced_at_state_version: 32
      created_by_actor_source: agent_connection:conn_run_probe
      storage_ref: "artifact-storage://search-result-count-validation"
registered_artifacts:
  - artifact_id: artifact_runprobe_report_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    display_name: "search-result-count-validation.json"
    content_type: application/json
    sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
    size_bytes: 96
    integrity_status: verified
    redaction_state: none
    availability: available
    created_by_run_ref:
      record_kind: run
      record_id: run_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      produced_at_state_version: 32
    created_by_actor_source: agent_connection:conn_run_probe
    storage_ref: "artifact-storage://search-result-count-validation"
evidence_summary:
  evidence_state: accepted_for_close
  status: sufficient
  coverage_items:
    - target:
        target_kind: acceptance_criterion
        acceptance_criterion_id: criterion_runprobe_count_001
      coverage_state: supported
      supporting_run_refs:
        - record_kind: run
          record_id: run_runprobe_001
          project_id: proj_runprobe_001
          task_id: task_runprobe_001
          produced_at_state_version: 32
      observation_refs:
        - record_kind: evidence_observation
          record_id: evidence_observation_runprobe_001
          project_id: proj_runprobe_001
          task_id: task_runprobe_001
          produced_at_state_version: 32
      supporting_artifact_refs:
        - artifact_id: artifact_runprobe_report_001
          project_id: proj_runprobe_001
          task_id: task_runprobe_001
          display_name: "search-result-count-validation.json"
          content_type: application/json
          sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
          size_bytes: 96
          integrity_status: verified
          redaction_state: none
          availability: available
          created_by_run_ref:
            record_kind: run
            record_id: run_runprobe_001
            project_id: proj_runprobe_001
            task_id: task_runprobe_001
            produced_at_state_version: 32
          created_by_actor_source: agent_connection:conn_run_probe
          storage_ref: "artifact-storage://search-result-count-validation"
      gap_refs: []
  artifact_refs:
    - artifact_id: artifact_runprobe_report_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      display_name: "search-result-count-validation.json"
      content_type: application/json
      sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
      size_bytes: 96
      integrity_status: verified
      redaction_state: none
      availability: available
      created_by_run_ref:
        record_kind: run
        record_id: run_runprobe_001
        project_id: proj_runprobe_001
        task_id: task_runprobe_001
        produced_at_state_version: 32
      created_by_actor_source: agent_connection:conn_run_probe
      storage_ref: "artifact-storage://search-result-count-validation"
  observation_refs:
    - record_kind: evidence_observation
      record_id: evidence_observation_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      produced_at_state_version: 32
  updated_by_run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    produced_at_state_version: 32
evidence_observations:
  - observation_id: evidence_observation_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    change_unit_id: cu_runprobe_001
    run_ref:
      record_kind: run
      record_id: run_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      produced_at_state_version: 32
    target:
      target_kind: acceptance_criterion
      acceptance_criterion_id: criterion_runprobe_count_001
    source_kind: agent_report
    assurance_level: cooperative_report
    observed_by_actor_source: agent_connection:conn_run_probe
    tool_name: "search-count-validator"
    tool_invocation_id: null
    tool_metadata:
      validator: "search-count"
    input_refs: []
    source_refs: []
    output_artifact_refs:
      - &runprobe_output
        artifact_id: artifact_runprobe_report_001
        project_id: proj_runprobe_001
        task_id: task_runprobe_001
        display_name: "search-result-count-validation.json"
        content_type: application/json
        sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
        size_bytes: 96
        integrity_status: verified
        redaction_state: none
        availability: available
        created_by_run_ref:
          record_kind: run
          record_id: run_runprobe_001
          project_id: proj_runprobe_001
          task_id: task_runprobe_001
          produced_at_state_version: 32
        created_by_actor_source: agent_connection:conn_run_probe
        storage_ref: "artifact-storage://search-result-count-validation"
    producer_anchor:
      producer_kind: unverified_caller
      producer_ref: null
      output_artifact_refs:
        - *runprobe_output
      verification_basis: null
    relevance_assessment:
      status: unassessed
      assessment_ref: null
      assessed_by_actor_source: null
    limitations: []
    observed_at: "<example-observed-at>"
    recorded_at: "<example-recorded-at>"
evidence_producers: []
current_close_basis:
  close_basis_revision: 4
  scope_revision: 2
  task_id: task_runprobe_001
  change_unit_id: cu_runprobe_001
  baseline_ref: baseline_runprobe_001
  result_summary: "검색 결과 수 검증을 통과했습니다."
  result_refs:
    - record_kind: run
      record_id: run_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      produced_at_state_version: 32
    - record_kind: change_unit
      record_id: cu_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      produced_at_state_version: 32
    - record_kind: evidence_summary
      record_id: evidence_summary_runprobe_001
      project_id: proj_runprobe_001
      task_id: task_runprobe_001
      produced_at_state_version: 32
  evidence_summary_ref:
    record_kind: evidence_summary
    record_id: evidence_summary_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    produced_at_state_version: 32
  residual_risks: []
  sensitive_categories: []
  sensitive_action_requirements: []
  recovery_constraints: []
  source_run_ref:
    record_kind: run
    record_id: run_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    produced_at_state_version: 32
  updated_at: "<example-updated-at>"
blocker_refs: []
state:
  project_id: proj_runprobe_001
  state_version: 32
  task_ref:
    record_kind: task
    record_id: task_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    produced_at_state_version: 32
  mode: work
  lifecycle:
    lifecycle_phase: ready
    close_reason: none
    result: none
    closed_at: null
  goal_summary: "검색 결과 수 표시를 검증합니다."
  scope_summary: "검색 결과 수 검증."
  non_goals:
    - "검색 순위 변경."
  acceptance_criteria:
    - acceptance_criterion_id: criterion_runprobe_count_001
      statement: "검색 결과에 예상한 개수가 표시됩니다."
      evidence_requirement: required
  autonomy_boundary: "검색 결과 수 검증 기록만 다룹니다."
  active_change_unit_ref:
    record_kind: change_unit
    record_id: cu_runprobe_001
    project_id: proj_runprobe_001
    task_id: task_runprobe_001
    produced_at_state_version: 32
  baseline_ref: baseline_runprobe_001
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

- 요청 래퍼, 응답 분기, `dry_run` 요약: [API 코어 스키마](schema-core.md).
- `RunSummary`, `EvidenceSummary`, `EvidenceCoverageItem`, `EvidenceObservation`, `CurrentCloseBasis`, `ResidualRisk`, `StateSummary`, 참조: [API 상태 스키마](schema-state.md).
- `ArtifactInput`, `StagedArtifactHandle`, `ArtifactRef`: [API 아티팩트 스키마](schema-artifacts.md).
- 쓰기 티켓과 닫기 관련 증거 경계: [Core 모델](../core-model.md).
- `Product Repository` 경로 정규화: [런타임 경계](../runtime-boundaries.md#product-repository-api-path-normalization).
- 지원되는 값과 작업 범주: [API 값 집합](schema-value-sets.md#operation-category-values).
- 공개 오류, 우선순위, 응답 처리 경로, 아티팩트 입력 세부 값: [API 오류 코드](error-codes.md), [API 오류 우선순위](error-precedence.md), [API 오류 처리 경로](error-routing.md), [아티팩트 입력 오류 세부사항](error-details.md#artifact-input-error-reason).
- 저장 효과와 아티팩트 승격: [저장 효과](../storage-effects.md), [아티팩트 저장소](../storage-artifacts.md).
