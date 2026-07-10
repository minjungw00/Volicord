# 적합성 참조

## 담당 경계

이 참조 문서는 안정적인 적합성 시나리오 의미와 기준을 정의합니다. 적합성
시나리오는 API, 저장소, 보안, 범위, Core, 아티팩트, Agent Connection 참조
문서가 담당하는 사실을 기준으로 확인할 관찰 가능한 동작 하나에 이름을 붙입니다.

이 문서가 담당합니다.

- `scenario_id` 이름 규칙
- 시나리오 수준의 기대 동작
- 주장 권한 경계
- 기준, 담당 문서, 예시, 튜토리얼, 메서드 안의 API 예시 사이의 관계

API 분기, 요청 형태, 저장 효과, 아티팩트 승격, 보안 보장, 닫기 준비 상태 동작,
메서드 참조 예시, 구현 구조는 정의하지 않습니다. 그 사실은 연결된 담당 문서에
남습니다.

기준 범위는 [범위](scope.md), 공개 용어는 [용어집](glossary.md)을 사용합니다.
완전한 구조화 용어는
[`docs/terminology-map.yaml`](../../terminology-map.yaml)에 있습니다.

<a id="surface-stability"></a>
## 표면 안정성

[기준 안정성 어휘](../maintain/documentation-policy.md#surface-stability-labels)를
사용합니다.

| 표면 | 안정성 | 의미 |
|---|---|---|
| 시나리오 의미, `scenario_id` 규칙, 기대 동작, 주장 권한, 담당 문서 링크 요구사항 | `stable` | 담당 문서가 정의한 동작의 기준입니다. |
| 실행기 요약, 렌더링된 보기, 상태 문구, 유지보수 라벨, 생성된 보고서 | `diagnostic` | 집중 담당 문서가 그 사실을 기준으로 정하지 않는 한 주장 권한이 아닙니다. |

<a id="conformance-item-summary"></a>
<a id="what-conformance-means"></a>
<a id="scenario-semantics"></a>
## 시나리오 의미

시나리오는 기준 동작 하나 또는 분명한 담당 경계 하나에 이름을 붙입니다.
`scenario_id`, 권한 맥락, 동작 하나, 기대 동작, 담당 문서 링크, 주장 경계를
포함합니다.

시나리오는 적합한 결과가 무엇을 보존하고, 거절하고, 드러내고, 바꾸지 않아야
하는지 말할 수 있습니다. 인접 API, 저장소, 보안, 범위, 닫기 준비 상태,
아티팩트, Agent Connection 계약을 다시 정의하면 안 됩니다.

적합성은 담당 문서가 정의한 동작을 담당 문서가 정의한 상태와 비효과에 견주어
판단합니다. 시나리오 설명, 에이전트 요약, 렌더링된 보기, 지표, 상태 보기,
유지보수 라벨 자체에는 주장 권한이 없습니다.

<a id="scenario-id-rules"></a>
### 시나리오 ID 규칙

- 기준 동작에는 `BASELINE-*` ID를 사용합니다.
- 프로젝트 단계, 검토 단계, 대기열, 실행기, 날짜, 구현 상태가 아니라 관찰
  가능한 동작을 이름에 담습니다.
- 기대 동작이 유지되는 동안 ID도 유지합니다.
- 의미가 바뀔 때만 ID를 바꿉니다. 같은 변경에서 페이지 안의 앵커와 링크도
  갱신합니다.

<a id="expected-behavior"></a>
### 기대 동작

기대 동작은 적합한 구현이나 점검이 만족해야 하는 안정적인 결과입니다. 이 문서는
시나리오 수준 결과만 말합니다. 정확한 요청 필드, 응답 분기, 저장 효과, 오류
우선순위, 보장 수준, 닫기 준비 상태 세부사항은 각 담당 문서에 남습니다.

이 문서의 요약과 담당 문서가 충돌하면 담당 문서가 우선합니다. 충돌하는 요약을
구현하지 말고 이 문서를 고칩니다.

<a id="criteria-vs-examples-and-tutorials"></a>
## 기준, 예시, 튜토리얼

예시와 튜토리얼은 독자가 시나리오를 알아보도록 도울 수 있습니다. 권한 기록,
API 분기, 저장 효과, 보안 보장, 닫기 결과, 수락 증거, 잔여 위험 수락을 만들지는
않습니다.

여러 메서드를 잇는 시나리오는 이 문서에서 시나리오 수준 기준으로 다룰 수
있습니다. API 메서드 참조 문서들이 공유하는 페이로드, 픽스처, 예시 축이 되면
안 됩니다. 메서드 예시는 시나리오로 연결할 수 있지만, 어느 쪽도 다른 쪽의
페이로드, 참조, 경로, `state_version`, 아티팩트 참조, 실행 참조, 판단 참조,
차단 사유 참조, 응답 스냅샷을 재사용하도록 요구하지 않습니다.

<a id="scenario-criterion-shape"></a>
## 기준 형식

| 부분 | 필요한 내용 |
|---|---|
| <a id="criterion-scenario-id"></a>`scenario_id` | 위 규칙을 따르는 안정적인 동작 식별자 |
| <a id="criterion-authority-context"></a>권한 맥락 | 동작 전에 필요한 `Task`, Change Unit, 상태 버전, 행위자 출처, 담당 참조, Core 상태, 저장소 행, 아티팩트 참조, 역량 사실 |
| <a id="criterion-action"></a>동작 | 담당 요청 스키마를 사용하는 공개 Core, API, 운영자 요청 하나 |
| <a id="criterion-expected-behavior"></a>기대 동작 | 기준과 관련된 응답, 상태, 저장소, 아티팩트, 차단 사유, 오류, 보장 표시, 금지된 부작용의 부재 |
| <a id="criterion-owner-links"></a>담당 문서 링크 | 각 정확한 사실을 정의하는 API, Core, 저장소, 보안, Agent Connection, 아티팩트, 정책 담당 문서 |
| 주장 경계 | 판단할 수 있는 담당 문서 정의 사실과 필요한 비효과 |

기준은 공개 담당 스키마를 사용합니다. 기준 전용 enum 값, 가짜 필드, 글로만 된
기대값, 상태처럼 쓰는 지역화 라벨, 지원 범위 밖 기능 전용 값을 만들면 안 됩니다.

<a id="assertion-authority"></a>
## 주장 권한

주장 권한은 기준이 판단할 수 있는 담당 문서 정의 사실의 좁은 범위입니다. 응답
사실, Core 상태, 저장 효과, 아티팩트 사실, 공개 `ErrorCode`, 구조화된 차단
사유, 보장 표시, 금지된 부작용의 필수 부재가 여기에 포함됩니다.

| 주장 영역 | 담당 문서 |
|---|---|
| API 메서드와 응답 분기 | [API 메서드](api/methods.md)와 연결된 메서드 담당 문서 |
| 공통 응답 분기와 `dry_run` 미리보기 형태 | [API 코어 스키마](api/schema-core.md) |
| 상태 요약, 차단 사유, 증거, 닫기 준비 상태 구조 | [API 상태 스키마](api/schema-state.md) |
| `ArtifactRef`, `ArtifactInput`, `StagedArtifactHandle` 형태 | [API 아티팩트 스키마](api/schema-artifacts.md) |
| `operation_category`를 포함한 API 값 집합 | [API 값 집합](api/schema-value-sets.md) |
| 공개 오류와 우선순위 | [API 오류 코드](api/error-codes.md), [API 오류 우선순위](api/error-precedence.md) |
| 저장 효과, 효과 없음 분기, 상태 버전 효과 | [저장 효과](storage-effects.md) |
| 아티팩트 스테이징, 승격, 영속 저장, 본문 읽기 | [아티팩트 저장소](storage-artifacts.md) |
| 보안 비주장과 보장 수준 | [보안](security.md) |
| 런타임과 제품 저장소 경계 | [런타임 경계](runtime-boundaries.md) |

<a id="representative-scenario-index"></a>
## 대표 시나리오

아래 ID는 작은 참조 기준입니다. 런타임 결과, 구현 계획, 실행 스크립트, API
예시가 따라야 하는 페이로드가 아닙니다.

| `scenario_id` | 기대 동작 | 담당 문서 |
|---|---|---|
| <a id="scenario-baseline-agent-connection-mismatch-blocks-mutation"></a>`BASELINE-agent-connection-mismatch-blocks-mutation` | Agent Connection 불일치는 상태 변경 전에 요청을 거절합니다. | [Agent Connection](agent-connection.md); [API 오류 코드](api/error-codes.md); [API 오류 처리 경로](api/error-routing.md); [보안](security.md) |
| <a id="scenario-baseline-verified-agent-connection-allows-owner-mutation"></a>`BASELINE-verified-agent-connection-allows-owner-mutation` | 검증된 Agent Connection은 적용되는 담당 계약 안에서만 상태 변경을 허용합니다. | [Agent Connection](agent-connection.md); [API 메서드 담당 경로](api/methods.md#method-owner-routing-table); [저장 효과](storage-effects.md) |
| <a id="scenario-baseline-single-operation-category-per-public-request"></a>`BASELINE-single-operation-category-per-public-request` | 공개 API 요청 하나에는 요청 수준 `operation_category`가 하나만 있습니다. | [API 값 집합](api/schema-value-sets.md); [Agent Connection](agent-connection.md); [보안](security.md) |
| <a id="scenario-baseline-detective-display-capability-gated"></a>`BASELINE-detective-display-capability-gated` | `detective` 표현은 지원되는 관찰 범위가 있을 때만 사용합니다. | [보안](security.md); [Agent Connection](agent-connection.md) |
| <a id="scenario-baseline-shaping-readiness-gap-blocks-or-asks"></a>`BASELINE-shaping-readiness-gap-blocks-or-asks` | 구체화 공백은 별도 계획 아티팩트가 아니라 담당 문서가 정의한 차단 사유나 판단 후보로 남습니다. | [Core 모델](core-model.md); [API 상태 스키마](api/schema-state.md); [상태 메서드](api/method-status.md); [판단 요청](api/method-request-user-judgment.md); [판단 기록](api/method-record-user-judgment.md) |
| <a id="scenario-baseline-project-state-version-stale-mutation-rejected"></a>`BASELINE-project-state-version-stale-mutation-rejected` | 오래된 프로젝트 전체 상태 버전은 커밋 전에 실패합니다. | [상태 버전 충돌](api/error-precedence.md#state-conflict-behavior); [저장소 버전 관리](storage-versioning.md); [저장 효과](storage-effects.md) |
| <a id="scenario-baseline-dry-run-pre-commit-failure-rejected"></a>`BASELINE-dry-run-pre-commit-failure-rejected` | `dry_run`은 검증, 접근, 역량, 오래된 상태 거절을 우회하지 않습니다. | [API 코어 스키마](api/schema-core.md); [`dry_run` 미리보기 전 실패](api/error-routing.md#rejected-dry-run-pre-preview-failure); [저장 효과](storage-effects.md) |
| <a id="scenario-baseline-status-close-blockers-read-only"></a>`BASELINE-status-close-blockers-read-only` | 상태와 닫기 확인 차단 사유는 저장 변경 없이 읽을 수 있습니다. | [상태 메서드](api/method-status.md); [Task 닫기 메서드](api/method-close-task.md); [API 상태 스키마](api/schema-state.md); [저장 효과](storage-effects.md) |
| <a id="scenario-baseline-sensitive-approval-records-sensitive-action-scope"></a>`BASELINE-sensitive-approval-records-sensitive-action-scope` | 민감 동작 승인은 쓰기 티켓, 최종 수락과 분리됩니다. | [Core 모델](core-model.md); [API 판단 스키마](api/schema-judgment.md); [보안](security.md) |
| <a id="scenario-baseline-prepare-write-requires-compatible-scope-and-approval"></a>`BASELINE-prepare-write-requires-compatible-scope-and-approval` | `prepare_write`는 협력형 제품 파일 호환성 경로입니다. | [쓰기 준비 메서드](api/method-prepare-write.md); [Core 모델](core-model.md); [보안](security.md) |
| <a id="scenario-baseline-write-ticket-attempt-scope-product-file-write-only"></a>`BASELINE-write-ticket-attempt-scope-product-file-write-only` | `WriteTicketAttemptScope`는 제품 파일 쓰기만 다룹니다. | [Core 모델](core-model.md); [쓰기 준비 메서드](api/method-prepare-write.md); [API 판단 스키마](api/schema-judgment.md) |
| <a id="scenario-baseline-record-run-consumes-write-ticket-once"></a>`BASELINE-record-run-consumes-write-ticket-once` | 호환되는 실행 기록은 맞는 쓰기 티켓 행을 한 번 소비합니다. | [실행 기록 메서드](api/method-record-run.md); [저장 효과](storage-effects.md); [저장소 버전 관리](storage-versioning.md) |
| <a id="scenario-baseline-stage-artifact-transient-handle-only"></a>`BASELINE-stage-artifact-transient-handle-only` | 스테이징은 임시 스테이징 핸들만 만듭니다. | [아티팩트 스테이징 메서드](api/method-stage-artifact.md); [API 아티팩트 스키마](api/schema-artifacts.md); [아티팩트 저장소](storage-artifacts.md) |
| <a id="scenario-baseline-record-run-artifact-input-validation-order"></a>`BASELINE-record-run-artifact-input-validation-order` | 실행 기록의 아티팩트 입력은 승격이나 연결 전에 검증됩니다. | [실행 기록 메서드](api/method-record-run.md); [API 아티팩트 스키마](api/schema-artifacts.md); [아티팩트 저장소](storage-artifacts.md) |
| <a id="scenario-baseline-record-run-promotes-staged-artifact-to-artifact-ref"></a>`BASELINE-record-run-promotes-staged-artifact-to-artifact-ref` | 호환되는 실행 기록은 스테이징 핸들을 영속 `ArtifactRef`로 승격할 수 있습니다. | [아티팩트 저장소](storage-artifacts.md); [실행 기록 메서드](api/method-record-run.md); [저장 효과](storage-effects.md) |
| <a id="scenario-baseline-record-run-rejects-staged-artifact-actor-source-mismatch"></a>`BASELINE-record-run-rejects-staged-artifact-actor-source-mismatch` | 스테이징 핸들의 출처가 맞지 않으면 승격을 거절합니다. | [아티팩트 저장소](storage-artifacts.md); [API 아티팩트 스키마](api/schema-artifacts.md); [아티팩트 입력 오류 세부사항](api/error-details.md#artifact-input-error-reason) |
| <a id="scenario-baseline-record-run-links-existing-artifact-without-registering-bytes"></a>`BASELINE-record-run-links-existing-artifact-without-registering-bytes` | 기존 영속 아티팩트는 새 바이트 등록 없이 연결할 수 있습니다. | [API 아티팩트 스키마](api/schema-artifacts.md); [아티팩트 저장소](storage-artifacts.md); [실행 기록 메서드](api/method-record-run.md) |
| <a id="scenario-baseline-captured-artifact-rejected-in-baseline-scope"></a>`BASELINE-captured-artifact-rejected-in-baseline-scope` | 호스트 앱에서 자체 캡처한 아티팩트 출처는 기준 범위 아티팩트 권한이 아닙니다. | [범위](scope.md); [API 아티팩트 스키마](api/schema-artifacts.md) |
| <a id="scenario-baseline-close-task-complete-stale-state-version-rejected"></a>`BASELINE-close-task-complete-stale-state-version-rejected` | 오래된 상태는 닫기 준비 상태 평가 전에 실패합니다. | [Task 닫기 메서드](api/method-close-task.md); [상태 버전 충돌](api/error-precedence.md#state-conflict-behavior); [저장 효과](storage-effects.md) |
| <a id="scenario-baseline-close-task-complete-stale-write-ticket-basis-rejected"></a>`BASELINE-close-task-complete-stale-write-ticket-basis-rejected` | 닫기 관련 쓰기 티켓 근거가 오래됐으면 닫기 커밋 전에 실패합니다. | [Task 닫기 메서드](api/method-close-task.md); [상태 버전 충돌](api/error-precedence.md#state-conflict-behavior); [상태 충돌 세부사항](api/error-details.md#state-conflict-detail-fields); [저장소 버전 관리](storage-versioning.md) |
| <a id="scenario-baseline-close-task-blocks-current-write-compatibility"></a>`BASELINE-close-task-blocks-current-write-compatibility` | 의미상 쓰기 호환성이 없으면 닫기가 막힐 수 있습니다. | [Core 모델](core-model.md); [Task 닫기 메서드](api/method-close-task.md); [API 상태 스키마](api/schema-state.md) |
| <a id="scenario-baseline-close-task-blocks-evidence-insufficient"></a>`BASELINE-close-task-blocks-evidence-insufficient` | 필요한 증거가 부족하면 닫기가 막힐 수 있습니다. | [Core 모델](core-model.md); [API 상태 스키마](api/schema-state.md); [Task 닫기 메서드](api/method-close-task.md); [API 차단 사유 처리 경로](api/blocker-routing.md) |
| <a id="scenario-baseline-close-task-blocks-required-artifact-unavailable"></a>`BASELINE-close-task-blocks-required-artifact-unavailable` | 필요한 아티팩트를 사용할 수 없으면 닫기가 막힐 수 있습니다. | [API 상태 스키마](api/schema-state.md); [아티팩트 저장소](storage-artifacts.md); [Task 닫기 메서드](api/method-close-task.md); [API 차단 사유 처리 경로](api/blocker-routing.md) |
| <a id="scenario-baseline-close-task-blocks-final-acceptance-missing"></a>`BASELINE-close-task-blocks-final-acceptance-missing` | 호환되는 최종 수락이 없으면 닫기가 막힐 수 있습니다. | [Core 모델](core-model.md); [API 판단 스키마](api/schema-judgment.md); [Task 닫기 메서드](api/method-close-task.md) |
| <a id="scenario-baseline-close-task-blocks-visible-unaccepted-residual-risk"></a>`BASELINE-close-task-blocks-visible-unaccepted-residual-risk` | 보이는 잔여 위험에 호환되는 수락이 없으면 닫기가 막힐 수 있습니다. | [Core 모델](core-model.md); [API 판단 스키마](api/schema-judgment.md); [API 상태 스키마](api/schema-state.md) |
| <a id="scenario-baseline-check-close-read-only"></a>`BASELINE-check-close-read-only` | `volicord.check_close`는 읽기 전용입니다. | [Task 닫기 메서드](api/method-close-task.md); [API 코어 스키마](api/schema-core.md); [저장 효과](storage-effects.md) |
| <a id="scenario-baseline-close-task-state-effecting-dry-run-preview"></a>`BASELINE-close-task-state-effecting-dry-run-preview` | 상태를 바꾸는 닫기 의도는 유효하고 미리 볼 수 있을 때만 `dry_run` 미리보기를 사용합니다. | [Task 닫기 메서드](api/method-close-task.md); [API 코어 스키마](api/schema-core.md); [저장 효과](storage-effects.md) |
| <a id="scenario-baseline-close-task-supersede-one-state-version"></a>`BASELINE-close-task-supersede-one-state-version` | `supersede`는 유효할 때 프로젝트 전체 상태 변경 하나를 쓰는 성공 완료가 아닌 종료 경로입니다. | [Task 닫기 메서드](api/method-close-task.md); [Core 모델](core-model.md); [저장 효과](storage-effects.md) |

## 목록 경계

기준 범위 밖 시나리오 계열 이름은 [범위](scope.md)가 담당합니다. 이런 이름은
시나리오 스크립트, 지원 API 페이로드, 실행기 요구사항, 구현 작업, 런타임 결과,
런타임 증명이 아닙니다.

## 지표 경계

지표에는 적합성 권한이 없습니다. 담당 문서가 원천 기록, 최신성 경계, 표시 문구,
대체 금지 규칙을 정의할 때만 지표가 기준에 영향을 줍니다.

지표는 Core 상태를 만들거나, 증거를 충족하거나, QA 또는 검증을 통과시키거나,
쓰기를 승인하거나, 결과나 잔여 위험을 수락하거나, 작업을 닫거나, 구현 구조를
증명하거나, 런타임 적합성을 대신할 수 없습니다.
