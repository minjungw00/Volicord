# 적합성 참조

## 경계

이 참조 문서는 안정적인 적합성 시나리오 의미와 참조 기준을 정의합니다.

적합성 시나리오는 이름 붙은 동작 기대치입니다. API, 저장소, 보안, 범위, Core, 아티팩트, 접점 담당 문서가 권한 있는 사실로 정한 내용만 기준으로 평가할 수 있습니다.

이 문서가 담당하는 항목은 아래와 같습니다.

- `scenario_id` 이름 규칙
- 적합성 시나리오 의미
- 시나리오 색인의 기대 동작 요약
- 적합성 기준의 주장 권한 경계
- 적합성 기준, 담당 문서, 예시, 튜토리얼 사이의 관계

API 분기, 저장 효과, 접근 등급, 아티팩트 승격, 보안 보장, 닫기 준비 상태 동작, 구현 경로는 이 문서가 정의하지 않습니다.

기준 범위의 기준 설명은 [범위](scope.md)를 확인하세요. 제품 용어는 [용어집](glossary.md)을 확인하세요.

## 적합성 항목 요약

| 항목 | 경계 | 세부사항 |
|---|---|---|
| 시나리오 의미 | 안정적인 동작 기대치 | [세부사항](#scenario-semantics) |
| 시나리오 ID | 안정적인 식별자 규칙 | [세부사항](#scenario-id-rules) |
| 기대 동작 | 담당 문서로 이어지는 기준 | [세부사항](#expected-behavior) |
| 주장 권한 | 담당 문서가 소유한 사실 | [세부사항](#assertion-authority) |
| 예시와 튜토리얼 | 권한이 아닌 설명 자료 | [세부사항](#criteria-vs-examples-and-tutorials) |

<a id="scenario-semantics"></a>
### 시나리오 의미

정의:
- 적합성 시나리오는 기준 범위 또는 분명한 담당 경계에 속하는 동작 기대치 하나에 이름을 붙입니다.

필수 부분:
- `scenario_id`
- 기대 동작
- 담당 문서 링크
- 주장 경계

허용되는 효과:
- 시나리오는 적합한 결과가 보존하거나, 거절하거나, 노출하거나, 바꾸지 않아야 하는 결과를 요약할 수 있습니다.

허용되지 않는 것:
- 시나리오는 자신이 인용하는 API, 저장소, 보안, 범위, 닫기 준비 상태, 아티팩트, 접점 계약을 다시 정의하면 안 됩니다.

<a id="scenario-id-rules"></a>
### 시나리오 ID 규칙

정의:
- `scenario_id`는 검토할 동작의 안정적인 식별자입니다.

규칙:
- 기준 범위 동작에는 `BASELINE-*` ID를 사용합니다.
- 프로젝트 단계, 검토 단계, 작업 대기열, 구현 상태가 아니라 관찰 가능한 동작을 이름에 담습니다.
- 기대 동작이 안정적으로 유지되면 ID도 안정적으로 유지합니다.
- 시나리오 의미가 바뀔 때만 ID를 바꾸고, 같은 작업 묶음에서 같은 페이지의 앵커와 내부 링크를 함께 갱신합니다.

허용되지 않는 것:
- 오래 유지되지 않는 상태 라벨, 날짜 라벨, 실행기 이름, 유지보수 작업 흐름 라벨을 시나리오 ID로 쓰지 않습니다.

<a id="expected-behavior"></a>
### 기대 동작

정의:
- 기대 동작은 그 시나리오에서 적합한 구현이나 점검이 만족해야 하는 안정적인 기준입니다.

담당 문서 관계:
- 이 페이지는 시나리오 수준의 결과를 말할 수 있습니다.
- 정확한 요청 필드, 응답 분기, 저장 효과, 오류 우선순위, 보장 수준, 닫기 준비 상태 세부사항은 기준 담당 문서에 남습니다.

충돌 규칙:
- 시나리오 요약과 기준 담당 문서가 어긋나면 기준 담당 문서가 우선하며, 이 페이지를 고쳐야 합니다.

허용되지 않는 것:
- 시나리오 설명, 요약, 렌더링된 보기, 지표, 유지보수 점검 라벨을 담당 문서가 정의하지 않은 사실의 권한으로 취급하지 않습니다.

## 적합성이 뜻하는 것

적합성이란 구현이나 점검이 담당 문서가 정의한 동작 하나를 담당 문서가 정의한 권한 기록과 비효과에 견주어 비교할 수 있다는 뜻입니다.

적합성 기준은 담당 문서가 권한 있는 사실로 정한 것만 판단합니다. 특정 담당 문서가 그 사실을 권한으로 정의하지 않았다면 시나리오 설명, 에이전트 요약, 렌더링된 보기, 상태 문구, 유지보수 점검 라벨, 상태 보기를 권한으로 다루면 안 됩니다.

이 문서의 "해야 합니다", "필수", "항상"은 적합성 기준이나 담당 문서로 이어지는 요건을 뜻합니다. 이웃 계약을 다시 정의한다는 뜻이 아닙니다.

<a id="criteria-vs-examples-and-tutorials"></a>
## 기준과 예시, 튜토리얼의 구분

적합성 기준은 참조 기대치입니다. 예시와 튜토리얼은 독자가 시나리오를 알아보도록 도울 수 있지만, 권한 기록, API 분기, 저장 효과, 보안 보장, 닫기 준비 상태 결과, 수락 증거, 잔여 위험 수락을 만들지 않습니다.

참조 시나리오는 안정적인 동작 설명을 사용해야 합니다. 문서 유지보수, 경로 정리, 마이그레이션 작업, 넓은 검토 단계, 오래 유지되지 않는 프로젝트 상태를 점검 대상 동작으로 삼으면 안 됩니다.

## 시나리오 기준 형식

적합성 시나리오 기준은 아래의 간결한 구조를 사용합니다.

| 부분 | 세부사항 |
|---|---|
| `scenario_id` | [`scenario_id`](#criterion-scenario-id) 참고 |
| 권한 맥락 | [권한 맥락](#criterion-authority-context) 참고 |
| 동작 | [동작](#criterion-action) 참고 |
| 기대 동작 | [기대 동작](#criterion-expected-behavior) 참고 |
| 담당 문서 링크 | [담당 문서 링크](#criterion-owner-links) 참고 |

<a id="criterion-scenario-id"></a>
### `scenario_id`

목적:
- 검토할 동작을 안정적으로 식별합니다.

<a id="criterion-authority-context"></a>
### 권한 맥락

목적:
- 동작 전에 필요한 사실을 이름 붙입니다.

예상 내용:
- `Task`, Change Unit, 상태 버전, 접점, 담당 문서 참조, Core 상태, 저장소 행, `ArtifactRef`, 접점 기능 사실을 담습니다.

<a id="criterion-action"></a>
### 동작

목적:
- 공개 Core, API, 운영자 요청 하나를 설명합니다.

담당 문서 링크:
- 요청은 담당 요청 스키마를 사용해야 합니다.

<a id="criterion-expected-behavior"></a>
### 기대 동작

목적:
- 적합한 결과가 만족해야 하는 안정적인 결과를 이름 붙입니다.

예상 내용:
- 응답 사실, 담당 문서가 소유하는 상태 변경 효과, 저장소 또는 아티팩트 사실, 차단 사유 사실, 오류 사실, 보장 표시 사실, 금지된 부작용의 필수 부재를 담습니다.

<a id="criterion-owner-links"></a>
### 담당 문서 링크

목적:
- 정확한 값과 의미를 기준 담당 문서로 보냅니다.

담당 문서 링크:
- API, Core, 저장소, 보안, 에이전트 통합, 아티팩트, 정책 담당 문서.

적합성 기준은 공개 담당 스키마를 사용해야 합니다. 기준 전용 enum 값, 가짜 필드, 상태로 쓰는 지역화 표시 라벨, 글로만 된 기대값, 지원 범위 밖 기능 전용 값을 만들면 안 됩니다.

<a id="assertion-authority"></a>
## 주장 권한

주장 권한은 적합성 기준이 판단할 수 있는 사실의 좁은 범위입니다. 권한은 시나리오 설명이나 생성된 요약이 아니라 담당 문서가 정의한 사실에서 옵니다.

적합성 주장은 담당 문서가 정의한 응답 사실, Core 상태, 저장 효과, 아티팩트 사실, 공개 `ErrorCode` 값, 구조화된 차단 사유, 보장 표시 사실, 금지된 부작용의 부재를 참조할 수 있습니다.

정확한 주장 세부사항은 아래 담당 문서에 남습니다.

| 주장 영역 | 담당 문서 |
|---|---|
| API 메서드와 응답 분기 동작 | [API 메서드](api/methods.md)와 메서드 담당 문서 |
| 공통 응답 분기와 `dry_run` 미리보기 형태 | [API 코어 스키마](api/schema-core.md) |
| 상태 요약, 차단 사유, 증거, 닫기 준비 상태 구조 | [API 상태 스키마](api/schema-state.md) |
| `ArtifactRef`, `ArtifactInput`, `StagedArtifactHandle` 형태 | [API 아티팩트 스키마](api/schema-artifacts.md) |
| `access_class` 값을 포함한 API 값 집합 | [API 값 집합](api/schema-value-sets.md) |
| 공개 오류와 우선순위 | [API 오류 코드](api/error-codes.md), [API 오류 우선순위](api/error-precedence.md) |
| 저장 효과, 효과 없음 분기, 상태 버전 효과 | [저장 효과](storage-effects.md) |
| 아티팩트 스테이징, 승격, 지속 저장, 본문 읽기 생명주기 | [아티팩트 저장소](storage-artifacts.md) |
| 보안 비주장과 보장 수준 | [보안](security.md) |
| 런타임 위치와 문서 경계 | [런타임 경계](runtime-boundaries.md) |

## 대표 시나리오 색인

아래 `scenario_id`는 작은 참조 기준입니다. 예시, 튜토리얼, 런타임 결과, 구현 계획이 아닙니다. 정확한 분기, 저장, 접근, 아티팩트, 보안, 닫기 준비 상태 계약은 위 담당 문서 링크를 사용합니다.

- `BASELINE-registered-surface-mismatch-blocks-mutation`
  [등록된 접점 불일치](#scenario-baseline-registered-surface-mismatch-blocks-mutation)를 참고합니다.
- `BASELINE-verified-local-surface-allows-owner-mutation`
  [확인된 로컬 접점](#scenario-baseline-verified-local-surface-allows-owner-mutation)을 참고합니다.
- `BASELINE-single-access-class-per-public-request`
  [단일 접근 등급](#scenario-baseline-single-access-class-per-public-request)을 참고합니다.
- `BASELINE-detective-display-capability-gated`
  [`detective` 표시](#scenario-baseline-detective-display-capability-gated)를 참고합니다.
- `BASELINE-shaping-readiness-gap-blocks-or-asks`
  [구체화 준비 공백](#scenario-baseline-shaping-readiness-gap-blocks-or-asks)을 참고합니다.
- `BASELINE-project-state-version-stale-mutation-rejected`
  [오래된 상태 변경](#scenario-baseline-project-state-version-stale-mutation-rejected)을 참고합니다.
- `BASELINE-dry-run-pre-commit-failure-rejected`
  [`dry_run` 커밋 전 실패](#scenario-baseline-dry-run-pre-commit-failure-rejected)를 참고합니다.
- `BASELINE-status-close-blockers-read-only`
  [읽기 전용 닫기 차단 사유](#scenario-baseline-status-close-blockers-read-only)를 참고합니다.
- `BASELINE-sensitive-approval-records-sensitive-action-scope`
  [민감 동작 승인 범위](#scenario-baseline-sensitive-approval-records-sensitive-action-scope)를 참고합니다.
- `BASELINE-prepare-write-requires-compatible-scope-and-approval`
  [`prepare_write` 호환성](#scenario-baseline-prepare-write-requires-compatible-scope-and-approval)을 참고합니다.
- `BASELINE-authorized-attempt-scope-product-file-write-only`
  [`AuthorizedAttemptScope`](#scenario-baseline-authorized-attempt-scope-product-file-write-only)를 참고합니다.
- `BASELINE-record-run-consumes-write-authorization-once`
  [1회용 Write Authorization](#scenario-baseline-record-run-consumes-write-authorization-once)을 참고합니다.
- `BASELINE-stage-artifact-transient-handle-only`
  [임시 스테이징 핸들](#scenario-baseline-stage-artifact-transient-handle-only)을 참고합니다.
- `BASELINE-record-run-artifact-input-validation-order`
  [아티팩트 입력 검증 순서](#scenario-baseline-record-run-artifact-input-validation-order)를 참고합니다.
- `BASELINE-record-run-promotes-staged-artifact-to-artifact-ref`
  [스테이징된 아티팩트 승격](#scenario-baseline-record-run-promotes-staged-artifact-to-artifact-ref)을 참고합니다.
- `BASELINE-record-run-rejects-staged-artifact-surface-instance-mismatch`
  [스테이징된 아티팩트 불일치](#scenario-baseline-record-run-rejects-staged-artifact-surface-instance-mismatch)를 참고합니다.
- `BASELINE-record-run-links-existing-artifact-without-registering-bytes`
  [기존 아티팩트 연결](#scenario-baseline-record-run-links-existing-artifact-without-registering-bytes)을 참고합니다.
- `BASELINE-captured-artifact-rejected-in-baseline-scope`
  [캡처 아티팩트 거절](#scenario-baseline-captured-artifact-rejected-in-baseline-scope)을 참고합니다.
- `BASELINE-close-task-complete-stale-state-version-rejected`
  [오래된 닫기 상태](#scenario-baseline-close-task-complete-stale-state-version-rejected)를 참고합니다.
- `BASELINE-close-task-complete-stale-write-authorization-basis-rejected`
  [오래된 Write Authorization 근거](#scenario-baseline-close-task-complete-stale-write-authorization-basis-rejected)를 참고합니다.
- `BASELINE-close-task-blocks-current-write-compatibility`
  [쓰기 호환성 차단](#scenario-baseline-close-task-blocks-current-write-compatibility)을 참고합니다.
- `BASELINE-close-task-blocks-evidence-insufficient`
  [증거 차단](#scenario-baseline-close-task-blocks-evidence-insufficient)을 참고합니다.
- `BASELINE-close-task-blocks-required-artifact-unavailable`
  [아티팩트 가용성 차단](#scenario-baseline-close-task-blocks-required-artifact-unavailable)을 참고합니다.
- `BASELINE-close-task-blocks-final-acceptance-missing`
  [최종 수락 차단](#scenario-baseline-close-task-blocks-final-acceptance-missing)을 참고합니다.
- `BASELINE-close-task-blocks-visible-unaccepted-residual-risk`
  [잔여 위험 차단](#scenario-baseline-close-task-blocks-visible-unaccepted-residual-risk)을 참고합니다.
- `BASELINE-close-task-check-read-only`
  [읽기 전용 닫기 확인](#scenario-baseline-close-task-check-read-only)을 참고합니다.
- `BASELINE-close-task-state-effecting-dry-run-preview`
  [상태 효과가 있는 닫기 `dry_run`](#scenario-baseline-close-task-state-effecting-dry-run-preview)을 참고합니다.
- `BASELINE-close-task-supersede-one-state-version`
  [`supersede` 상태 버전](#scenario-baseline-close-task-supersede-one-state-version)을 참고합니다.

<a id="scenario-baseline-registered-surface-mismatch-blocks-mutation"></a>
### `BASELINE-registered-surface-mismatch-blocks-mutation`

기대 동작:
- 상태 변경 전 로컬 접점이 등록 정보와 맞지 않습니다.

담당 문서 링크:
- [에이전트 통합](agent-integration.md)
- [API 오류 코드](api/error-codes.md)
- [API 오류 처리 경로](api/error-routing.md)
- [보안](security.md)

<a id="scenario-baseline-verified-local-surface-allows-owner-mutation"></a>
### `BASELINE-verified-local-surface-allows-owner-mutation`

기대 동작:
- 확인된 로컬 접점은 담당 메서드 계약 안에서만 상태 변경 확인을 허용합니다.

담당 문서 링크:
- [에이전트 통합](agent-integration.md)
- [API 메서드 담당 문서 경로](api/methods.md#method-owner-routing-table)
- [저장 효과](storage-effects.md)

<a id="scenario-baseline-single-access-class-per-public-request"></a>
### `BASELINE-single-access-class-per-public-request`

기대 동작:
- 공개 API 요청 하나에는 요청 수준 `access_class` 하나만 있습니다.

담당 문서 링크:
- [API 값 집합](api/schema-value-sets.md)
- [에이전트 통합](agent-integration.md)
- [보안](security.md)

<a id="scenario-baseline-detective-display-capability-gated"></a>
### `BASELINE-detective-display-capability-gated`

기대 동작:
- `detective` 표현은 지원되는 관찰 범위가 있을 때만 가능합니다.

담당 문서 링크:
- [보안](security.md)
- [에이전트 통합](agent-integration.md)

<a id="scenario-baseline-shaping-readiness-gap-blocks-or-asks"></a>
### `BASELINE-shaping-readiness-gap-blocks-or-asks`

기대 동작:
- 구체화 공백은 별도 계획 아티팩트가 아니라 관련 계약이 정의한 차단 사유나 판단 후보로 남습니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [API 상태 스키마](api/schema-state.md)
- [상태 메서드](api/method-status.md)
- [사용자 판단 메서드](api/method-user-judgment.md)

<a id="scenario-baseline-project-state-version-stale-mutation-rejected"></a>
### `BASELINE-project-state-version-stale-mutation-rejected`

기대 동작:
- 오래된 프로젝트 전체 상태 버전은 커밋 전에 실패합니다.

담당 문서 링크:
- [상태 버전 충돌](api/error-precedence.md#state-conflict-behavior)
- [저장소 버전 관리](storage-versioning.md)
- [저장 효과](storage-effects.md)

<a id="scenario-baseline-dry-run-pre-commit-failure-rejected"></a>
### `BASELINE-dry-run-pre-commit-failure-rejected`

기대 동작:
- `dry_run`은 검증, 접근, 역량, 오래된 상태 거절을 우회하지 않습니다.

담당 문서 링크:
- [API 코어 스키마](api/schema-core.md)
- [`dry_run` 미리보기 전 거절](api/error-routing.md#rejected-dry-run-pre-preview-failure)
- [저장 효과](storage-effects.md)

<a id="scenario-baseline-status-close-blockers-read-only"></a>
### `BASELINE-status-close-blockers-read-only`

기대 동작:
- 상태와 닫기 확인 차단 사유는 저장 변경 없이 읽을 수 있습니다.

담당 문서 링크:
- [상태 메서드](api/method-status.md)
- [Task 닫기 메서드](api/method-close-task.md)
- [API 상태 스키마](api/schema-state.md)
- [저장 효과](storage-effects.md)

<a id="scenario-baseline-sensitive-approval-records-sensitive-action-scope"></a>
### `BASELINE-sensitive-approval-records-sensitive-action-scope`

기대 동작:
- 민감 동작 승인은 Write Authorization, 최종 수락과 분리됩니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [API 판단 스키마](api/schema-judgment.md)
- [보안](security.md)

<a id="scenario-baseline-prepare-write-requires-compatible-scope-and-approval"></a>
### `BASELINE-prepare-write-requires-compatible-scope-and-approval`

기대 동작:
- `prepare_write`는 협력형 제품 파일 호환성 경로입니다.

담당 문서 링크:
- [쓰기 준비 메서드](api/method-prepare-write.md)
- [Core 모델](core-model.md)
- [보안](security.md)

<a id="scenario-baseline-authorized-attempt-scope-product-file-write-only"></a>
### `BASELINE-authorized-attempt-scope-product-file-write-only`

기대 동작:
- `AuthorizedAttemptScope`는 제품 파일 쓰기 범위만 다룹니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [쓰기 준비 메서드](api/method-prepare-write.md)
- [API 판단 스키마](api/schema-judgment.md)

<a id="scenario-baseline-record-run-consumes-write-authorization-once"></a>
### `BASELINE-record-run-consumes-write-authorization-once`

기대 동작:
- 호환되는 실행 기록은 맞는 `Write Authorization`을 한 번 소비합니다.

담당 문서 링크:
- [실행 기록 메서드](api/method-record-run.md)
- [저장 효과](storage-effects.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="scenario-baseline-stage-artifact-transient-handle-only"></a>
### `BASELINE-stage-artifact-transient-handle-only`

기대 동작:
- 스테이징은 임시 스테이징 핸들만 만듭니다.

담당 문서 링크:
- [아티팩트 스테이징 메서드](api/method-stage-artifact.md)
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 저장소](storage-artifacts.md)

<a id="scenario-baseline-record-run-artifact-input-validation-order"></a>
### `BASELINE-record-run-artifact-input-validation-order`

기대 동작:
- 실행 기록의 아티팩트 입력은 승격이나 연결 전에 검증됩니다.

담당 문서 링크:
- [실행 기록 메서드](api/method-record-run.md)
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 저장소](storage-artifacts.md)

<a id="scenario-baseline-record-run-promotes-staged-artifact-to-artifact-ref"></a>
### `BASELINE-record-run-promotes-staged-artifact-to-artifact-ref`

기대 동작:
- 호환되는 실행 기록은 스테이징 핸들을 지속 `ArtifactRef`로 승격할 수 있습니다.

담당 문서 링크:
- [아티팩트 저장소](storage-artifacts.md)
- [실행 기록 메서드](api/method-record-run.md)
- [저장 효과](storage-effects.md)

<a id="scenario-baseline-record-run-rejects-staged-artifact-surface-instance-mismatch"></a>
### `BASELINE-record-run-rejects-staged-artifact-surface-instance-mismatch`

기대 동작:
- 스테이징 핸들의 출처가 맞지 않으면 승격이 거절됩니다.

담당 문서 링크:
- [아티팩트 저장소](storage-artifacts.md)
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 입력 오류 세부사항](api/error-details.md#artifact-input-error-reason)

<a id="scenario-baseline-record-run-links-existing-artifact-without-registering-bytes"></a>
### `BASELINE-record-run-links-existing-artifact-without-registering-bytes`

기대 동작:
- 이미 지속되는 아티팩트는 새 바이트 등록 없이 연결될 수 있습니다.

담당 문서 링크:
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 저장소](storage-artifacts.md)
- [실행 기록 메서드](api/method-record-run.md)

<a id="scenario-baseline-captured-artifact-rejected-in-baseline-scope"></a>
### `BASELINE-captured-artifact-rejected-in-baseline-scope`

기대 동작:
- 접점 자체 캡처 아티팩트 출처는 기준 범위 아티팩트 권한이 아닙니다.

담당 문서 링크:
- [기준 범위](scope.md)
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [범위 참조](scope.md)

<a id="scenario-baseline-close-task-complete-stale-state-version-rejected"></a>
### `BASELINE-close-task-complete-stale-state-version-rejected`

기대 동작:
- 오래된 상태는 닫기 준비 상태 평가 전에 실패합니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [상태 버전 충돌](api/error-precedence.md#state-conflict-behavior)
- [저장 효과](storage-effects.md)

<a id="scenario-baseline-close-task-complete-stale-write-authorization-basis-rejected"></a>
### `BASELINE-close-task-complete-stale-write-authorization-basis-rejected`

기대 동작:
- 닫기 관련 Write Authorization 근거가 오래됐으면 닫기 커밋 전에 실패합니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [상태 버전 충돌](api/error-precedence.md#state-conflict-behavior)
- [상태 충돌 세부 필드](api/error-details.md#state-conflict-detail-fields)
- [저장소 버전 관리](storage-versioning.md)

<a id="scenario-baseline-close-task-blocks-current-write-compatibility"></a>
### `BASELINE-close-task-blocks-current-write-compatibility`

기대 동작:
- 닫기는 의미적 쓰기 호환성 때문에 막힐 수 있습니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [Task 닫기 메서드](api/method-close-task.md)
- [API 상태 스키마](api/schema-state.md)

<a id="scenario-baseline-close-task-blocks-evidence-insufficient"></a>
### `BASELINE-close-task-blocks-evidence-insufficient`

기대 동작:
- 닫기는 필수 증거 부족 때문에 막힐 수 있습니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [API 상태 스키마](api/schema-state.md)
- [Task 닫기 메서드](api/method-close-task.md)
- [API 차단 사유 처리 경로](api/blocker-routing.md)

<a id="scenario-baseline-close-task-blocks-required-artifact-unavailable"></a>
### `BASELINE-close-task-blocks-required-artifact-unavailable`

기대 동작:
- 닫기는 필수 아티팩트 가용성 때문에 막힐 수 있습니다.

담당 문서 링크:
- [API 상태 스키마](api/schema-state.md)
- [아티팩트 저장소](storage-artifacts.md)
- [Task 닫기 메서드](api/method-close-task.md)
- [API 차단 사유 처리 경로](api/blocker-routing.md)

<a id="scenario-baseline-close-task-blocks-final-acceptance-missing"></a>
### `BASELINE-close-task-blocks-final-acceptance-missing`

기대 동작:
- 닫기는 호환되는 최종 수락이 없어 막힐 수 있습니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [API 판단 스키마](api/schema-judgment.md)
- [Task 닫기 메서드](api/method-close-task.md)

<a id="scenario-baseline-close-task-blocks-visible-unaccepted-residual-risk"></a>
### `BASELINE-close-task-blocks-visible-unaccepted-residual-risk`

기대 동작:
- 닫기는 보이는 잔여 위험에 대한 호환되는 수락이 없어 막힐 수 있습니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [API 판단 스키마](api/schema-judgment.md)
- [API 상태 스키마](api/schema-state.md)

<a id="scenario-baseline-close-task-check-read-only"></a>
### `BASELINE-close-task-check-read-only`

기대 동작:
- `harness.close_task intent=check`는 읽기 전용입니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [API 코어 스키마](api/schema-core.md)
- [저장 효과](storage-effects.md)

<a id="scenario-baseline-close-task-state-effecting-dry-run-preview"></a>
### `BASELINE-close-task-state-effecting-dry-run-preview`

기대 동작:
- 상태 효과가 있는 닫기 의도값은 유효하고 미리보기 가능할 때만 `dry_run` 미리보기를 사용합니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [API 코어 스키마](api/schema-core.md)
- [저장 효과](storage-effects.md)

<a id="scenario-baseline-close-task-supersede-one-state-version"></a>
### `BASELINE-close-task-supersede-one-state-version`

기대 동작:
- `supersede`는 유효할 때 프로젝트 전체 상태 변경 하나를 쓰는 성공 완료가 아닌 종료 경로입니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [Core 모델](core-model.md)
- [저장 효과](storage-effects.md)

## 목록 경계

기준 범위 밖 시나리오 계열 이름은 [범위](scope.md)가 담당합니다. 범위 담당 문서는 그 이름을 지원 범위 밖 기능으로만 둘 수 있으며, 이 페이지는 그 목록을 반복하지 않습니다.

지원 범위 밖 계열 이름은 아래 항목이 아닙니다.

- 시나리오 스크립트
- 지원되는 API 페이로드 예시
- 실행기 또는 보고 요구사항
- 기준 범위
- 구현 작업
- 런타임 결과
- 런타임 증명

## 지표 경계

지표는 적합성 권한이 아닙니다. 지표는 기준 담당 문서가 원천 기록, 최신성 경계, 표시 문구, 대체 불가 규칙을 정의할 때만 적합성 기준에 영향을 줄 수 있습니다.

지표는 아래 항목을 하면 안 됩니다.

- Core 상태 만들기
- 증거 충족
- QA 또는 검증 통과
- 쓰기 승인
- 최종 결과 수락
- 잔여 위험 수락
- 작업 닫기
- 구현 경로 상태 증명
- 런타임 적합성 대체
