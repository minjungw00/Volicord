# 적합성 참조

## 현재 상태

이 저장소는 문서 전용이며 아직 문서 검토 단계입니다. 여기에는 Harness Server 런타임, Harness Runtime Home, 실행 가능한 픽스처 파일, 적합성 실행기, 생성된 적합성 보고서, 생성된 런타임 아티팩트, 현재 런타임 적합성 결과가 없습니다.

이 문서는 문서 수준의 적합성 의미, 향후 픽스처 후보 형식, 주장 권한 경계, 간결한 시나리오 색인을 담당합니다. API 분기, 저장 효과, 접근 등급, 아티팩트 승격, 보안 보장, 닫기 준비 상태 동작은 이 문서가 정의하지 않습니다.

현재 범위의 기준 설명은 [현재 MVP 범위 참조](active-mvp-scope.md)를 확인하세요. 현재 단계와 인계 상태는 [MVP 계획의 저장소 상태](../build/mvp-plan.md#문서-수락-상태)가 담당합니다.

## Conformance 항목 요약

| 항목 | 현재 상태 | 상세 |
|---|---|---|
| 현재 문서 기준 | 활성 참조 기준 | [현재 문서 기준](#current-documentation-criteria) 참조 |
| 내부 스모크 목표 | 계획 또는 문서화됨 | [내부 스모크 목표](#internal-smoke-target) 참조 |
| 향후 픽스처 형식 | 향후 후보 형식 | [향후 픽스처 형식](#future-fixture-shape) 참조 |
| 향후 실행 가능한 픽스처 | 구현되지 않음 | [향후 실행 가능한 픽스처](#future-executable-fixtures) 참조 |
| 런타임 적합성 보고 | 이후 후보이며 구현되지 않음 | [런타임 적합성 보고](#runtime-conformance-report) 참조 |

<a id="current-documentation-criteria"></a>
### 현재 문서 기준

현재 상태:
- 문서 검토와 향후 계획을 위한 활성 참조 기준입니다.

현재 실행 가능 여부:
- 런타임 실행 기준이 아닙니다. 이 기준은 Harness Server를 실행하거나 런타임 기록을 만들지 않습니다.

담당 문서:
- `docs/ko/reference/conformance.md`

허용되지 않는 것:
- 문서 기준을 런타임 적합성 결과, 수락 증거, 구현 준비 상태로 다루지 않습니다.

<a id="internal-smoke-target"></a>
### 내부 스모크 목표

현재 상태:
- 구현 계획 목표로 계획되었거나 문서화된 항목입니다.

현재 실행 가능 여부:
- 현재 구현 파일이 명시적으로 제공하지 않는 한 실행 가능한 적합성 모음이 아닙니다. 이 저장소에는 그런 구현 파일이 없습니다.

담당 문서:
- [MVP 계획의 첫 내부 스모크 목표](../build/mvp-plan.md#첫-내부-스모크-목표)

허용되지 않는 것:
- 구현된 적합성 모음, 픽스처 명세, 하네스 구현 증명처럼 설명하지 않습니다.

<a id="future-fixture-shape"></a>
### 향후 픽스처 형식

현재 상태:
- 이 참조 문서가 설명하는 향후 후보 형식입니다.

현재 실행 가능 여부:
- 실행 가능하지 않습니다. 이 저장소에는 실행 가능한 픽스처 파일, 픽스처 디렉터리, 픽스처 실행기가 없습니다.

담당 문서:
- `docs/ko/reference/conformance.md`

허용되지 않는 것:
- 후보 형식을 현재 픽스처 파일, 현재 실행기 입력, 구현된 적합성 모음처럼 설명하지 않습니다.

<a id="future-executable-fixtures"></a>
### 향후 실행 가능한 픽스처

현재 상태:
- 구현되지 않았습니다.

현재 실행 가능 여부:
- 실행 가능하지 않습니다. 실행 가능한 픽스처 자료에는 향후 실행기와 담당 문서가 승격한 픽스처가 필요합니다.

담당 문서:
- 향후 실행기 담당 문서와 픽스처를 승격하는 담당 문서입니다. 이 저장소에는 현재 픽스처 실행기나 실행 가능한 픽스처 담당 문서가 없습니다.

허용되지 않는 것:
- 이 문서 저장소에 픽스처 본문, 실행기 출력, 생성된 런타임 객체, 현재 런타임 결과를 추가하지 않습니다.

<a id="runtime-conformance-report"></a>
### 런타임 적합성 보고

현재 상태:
- 이후 후보이며 구현되지 않았습니다.

현재 실행 가능 여부:
- 실행 가능하지 않습니다. 이 저장소에는 적합성 실행기, 생성된 적합성 보고서, 런타임 적합성 결과가 없습니다.

담당 문서:
- [이후 후보 색인](../later/index.md)
- [정책과 적합성: 향후 적합성 실행 진입점](../later/policy-and-conformance.md#future-conformance-run-entrypoint)

허용되지 않는 것:
- 지표, 생성된 글, 렌더링된 보고서, 문서 점검 라벨을 적합성 권한이나 현재 런타임 증명으로 제시하지 않습니다.

이 문서의 "해야 합니다", "필수", "항상"은 현재 문서 기준이나 구현 뒤의 향후 서버/실행기 요건을 뜻합니다. 이 저장소에 실행 가능한 점검이 이미 있다는 뜻이 아닙니다.

## 적합성이 뜻하는 것

향후 서버에서 적합성이란 실행 점검이 담당 문서가 정의한 동작 하나를 담당 문서의 권한 기록과 비교할 수 있다는 뜻입니다. 문서 점검은 링크, 용어, 담당 문서 경계, 현재/이후 문구, 보안 표현, 한영 의미 일치를 보는 별도 유지보수 보조 도구입니다.

향후 런타임 적합성 점검은 담당 문서가 권한 있는 사실로 정한 것만 판단해야 합니다. 특정 담당 문서가 그 사실을 승격하지 않았다면 생성된 글, 에이전트 요약, 렌더링된 보고서, 상태 문구, 문서 점검 라벨, 상태 보기를 권한으로 다루면 안 됩니다.

## 아직 존재하지 않는 것

아래 항목은 향후 구현 작업이며 현재 저장소 내용이 아닙니다.

- Harness Server 런타임 또는 Harness Runtime Home 데이터
- 실행 가능한 픽스처 파일 또는 픽스처 디렉터리
- 적합성 실행기 또는 `harness conformance run` 구현
- 생성된 적합성 보고서, 생성된 런타임 아티팩트, 상태 보기, 운영 파일, 런타임 상태
- 현재 MVP 동작이나 이후 후보에 대한 현재 런타임 결과
- 예방적 차단, OS 권한 제어, 임의 도구 샌드박스, 변조 방지 저장소, 보안 격리, profile-gated `preventive` / `isolated` 보장 주장에 대한 현재 런타임 증명

이 문서의 예시는 계획을 도울 수 있습니다. 하지만 런타임 상태, 수락 증거, 닫기 준비 상태, 잔여 위험 수락, 생성된 보고서, 구현 준비 상태를 만들지 않습니다.

## 픽스처 형식

픽스처 형식은 향후 후보 형식일 뿐 현재 파일을 만들지 않습니다. Harness Server와 실행기가 생긴 뒤 승격된 픽스처는 아래 부분을 담은 작은 구조화 기록이어야 합니다.

| 부분 | 세부사항 |
|---|---|
| `scenario_id` | [`scenario_id`](#fixture-scenario-id) 참고 |
| 권한 맥락 | [권한 맥락](#fixture-authority-context) 참고 |
| 동작 | [동작](#fixture-action) 참고 |
| 기대 주장 | [기대 주장](#fixture-expected-assertions) 참고 |
| 담당 문서 링크 | [담당 문서 링크](#fixture-owner-links) 참고 |

<a id="fixture-scenario-id"></a>
### `scenario_id`

효과:
- 검토할 동작을 안정적으로 식별합니다.

<a id="fixture-authority-context"></a>
### 권한 맥락

효과:
- 동작 전에 필요한 사실을 이름 붙입니다.

조건:
- Task, Change Unit, 상태 버전, 접점, 담당 문서 참조, Core 상태, 저장소 행, `ArtifactRef`, 접점 기능 사실을 담습니다.

<a id="fixture-action"></a>
### 동작

효과:
- 공개 Core, API, 운영자 요청 하나를 설명합니다.

담당 문서 링크:
- 요청은 담당 요청 스키마를 사용해야 합니다.

<a id="fixture-expected-assertions"></a>
### 기대 주장

효과:
- 향후 픽스처가 비교할 수 있는 구조화된 사실을 이름 붙입니다.

조건:
- 응답 사실, 담당 문서가 소유하는 상태 변경 효과, 저장소 또는 아티팩트 사실, 차단 사유 사실, 오류 사실, 보장 표시 사실, 금지된 부작용의 필수 부재를 담습니다.

<a id="fixture-owner-links"></a>
### 담당 문서 링크

효과:
- 정확한 값과 의미를 기준 담당 문서로 보냅니다.

담당 문서 링크:
- API, Core, 저장소, 보안, 에이전트 통합, 아티팩트, 정책 담당 문서.

향후 구체화된 픽스처는 공개 담당 스키마를 사용해야 합니다. 픽스처 전용 enum 값, 가짜 필드, 상태로 쓰는 지역화 표시 라벨, 글로만 된 기대값, 이후 후보 전용 값을 만들면 안 됩니다.

## 주장 권한

주장 권한은 실행 가능한 픽스처가 생긴 뒤 향후 픽스처가 판단할 수 있는 사실의 좁은 범위입니다. 권한은 시나리오 설명이나 생성된 요약이 아니라 담당 문서가 정의한 사실에서 옵니다.

향후 주장은 담당 문서가 정의한 응답 사실, Core 상태, 저장 효과, 아티팩트 사실, 공개 `ErrorCode` 값, 구조화된 차단 사유, 보장 표시 사실, 금지된 부작용의 부재를 참조할 수 있습니다.

정확한 주장 세부사항은 아래 담당 문서에 남습니다.

| 주장 영역 | 담당 문서 |
|---|---|
| API 메서드와 응답 분기 동작 | [MVP API 경로 문서](api/mvp-api.md)와 메서드 담당 문서 |
| 공통 응답 분기와 `dry_run` 미리보기 형태 | [API 코어 스키마](api/schema-core.md) |
| 상태 요약, 차단 사유, 증거, 닫기 준비 상태 구조 | [API 상태 스키마](api/schema-state.md) |
| `ArtifactRef`, `ArtifactInput`, `StagedArtifactHandle` 형태 | [API 아티팩트 스키마](api/schema-artifacts.md) |
| `access_class` 값을 포함한 API 값 집합 | [API 값 집합](api/schema-value-sets.md) |
| 공개 오류와 우선순위 | [API 오류](api/errors.md) |
| 저장 효과, 효과 없음 분기, 상태 버전 효과 | [저장 효과](storage-effects.md) |
| 아티팩트 스테이징, 승격, 지속 저장, 본문 읽기 생명주기 | [아티팩트 저장소](storage-artifacts.md) |
| 보안 비주장과 보장 수준 | [보안](security.md) |
| 런타임 위치와 문서 전용 경계 | [런타임 경계](runtime-boundaries.md) |

## 대표 시나리오 색인

아래 `scenario_id`는 향후 픽스처 계획을 위한 작은 문서 기준입니다. 픽스처 본문, 현재 런타임 결과, 생성된 런타임 객체, 구현 계획이 아닙니다. 정확한 분기, 저장, 접근, 아티팩트, 보안, 닫기 준비 상태 계약은 위 담당 문서 링크를 사용합니다.

| 시나리오 ID | 세부사항 |
|---|---|
| `MVP-ACTIVE-registered-surface-mismatch-blocks-mutation` | [등록된 접점 불일치](#scenario-mvp-active-registered-surface-mismatch-blocks-mutation) 참고 |
| `MVP-ACTIVE-verified-local-surface-allows-owner-mutation` | [확인된 로컬 접점](#scenario-mvp-active-verified-local-surface-allows-owner-mutation) 참고 |
| `MVP-ACTIVE-single-access-class-per-public-request` | [단일 접근 등급](#scenario-mvp-active-single-access-class-per-public-request) 참고 |
| `MVP-ACTIVE-detective-display-capability-gated` | [`detective` 표시](#scenario-mvp-active-detective-display-capability-gated) 참고 |
| `MVP-ACTIVE-shaping-readiness-gap-blocks-or-asks` | [구체화 준비 공백](#scenario-mvp-active-shaping-readiness-gap-blocks-or-asks) 참고 |
| `MVP-ACTIVE-project-state-version-stale-mutation-rejected` | [오래된 상태 변경](#scenario-mvp-active-project-state-version-stale-mutation-rejected) 참고 |
| `MVP-ACTIVE-dry-run-pre-commit-failure-rejected` | [`dry_run` 커밋 전 실패](#scenario-mvp-active-dry-run-pre-commit-failure-rejected) 참고 |
| `MVP-ACTIVE-status-close-blockers-read-only` | [읽기 전용 닫기 차단 사유](#scenario-mvp-active-status-close-blockers-read-only) 참고 |
| `MVP-ACTIVE-sensitive-approval-records-sensitive-action-scope` | [민감 동작 승인 범위](#scenario-mvp-active-sensitive-approval-records-sensitive-action-scope) 참고 |
| `MVP-ACTIVE-prepare-write-requires-compatible-scope-and-approval` | [`prepare_write` 호환성](#scenario-mvp-active-prepare-write-requires-compatible-scope-and-approval) 참고 |
| `MVP-ACTIVE-authorized-attempt-scope-product-file-write-only` | [`AuthorizedAttemptScope`](#scenario-mvp-active-authorized-attempt-scope-product-file-write-only) 참고 |
| `MVP-ACTIVE-record-run-consumes-write-authorization-once` | [1회용 Write Authorization](#scenario-mvp-active-record-run-consumes-write-authorization-once) 참고 |
| `MVP-ACTIVE-stage-artifact-temporary-handle-only` | [임시 스테이징 핸들](#scenario-mvp-active-stage-artifact-temporary-handle-only) 참고 |
| `MVP-ACTIVE-record-run-artifact-input-validation-order` | [아티팩트 입력 검증 순서](#scenario-mvp-active-record-run-artifact-input-validation-order) 참고 |
| `MVP-ACTIVE-record-run-promotes-staged-artifact-to-artifact-ref` | [스테이징된 아티팩트 승격](#scenario-mvp-active-record-run-promotes-staged-artifact-to-artifact-ref) 참고 |
| `MVP-ACTIVE-record-run-rejects-staged-artifact-surface-instance-mismatch` | [스테이징된 아티팩트 불일치](#scenario-mvp-active-record-run-rejects-staged-artifact-surface-instance-mismatch) 참고 |
| `MVP-ACTIVE-record-run-links-existing-artifact-without-registering-bytes` | [기존 아티팩트 연결](#scenario-mvp-active-record-run-links-existing-artifact-without-registering-bytes) 참고 |
| `MVP-ACTIVE-captured-artifact-rejected-in-active-mvp` | [캡처 아티팩트 거절](#scenario-mvp-active-captured-artifact-rejected-in-active-mvp) 참고 |
| `MVP-ACTIVE-close-task-complete-stale-state-version-rejected` | [오래된 닫기 상태](#scenario-mvp-active-close-task-complete-stale-state-version-rejected) 참고 |
| `MVP-ACTIVE-close-task-complete-stale-write-authorization-basis-rejected` | [오래된 Write Authorization 근거](#scenario-mvp-active-close-task-complete-stale-write-authorization-basis-rejected) 참고 |
| `MVP-ACTIVE-close-task-blocks-current-write-compatibility` | [쓰기 호환성 차단](#scenario-mvp-active-close-task-blocks-current-write-compatibility) 참고 |
| `MVP-ACTIVE-close-task-blocks-evidence-insufficient` | [증거 차단](#scenario-mvp-active-close-task-blocks-evidence-insufficient) 참고 |
| `MVP-ACTIVE-close-task-blocks-required-artifact-unavailable` | [아티팩트 가용성 차단](#scenario-mvp-active-close-task-blocks-required-artifact-unavailable) 참고 |
| `MVP-ACTIVE-close-task-blocks-final-acceptance-missing` | [최종 수락 차단](#scenario-mvp-active-close-task-blocks-final-acceptance-missing) 참고 |
| `MVP-ACTIVE-close-task-blocks-visible-unaccepted-residual-risk` | [잔여 위험 차단](#scenario-mvp-active-close-task-blocks-visible-unaccepted-residual-risk) 참고 |
| `MVP-ACTIVE-close-task-check-read-only` | [읽기 전용 닫기 확인](#scenario-mvp-active-close-task-check-read-only) 참고 |
| `MVP-ACTIVE-close-task-state-effecting-dry-run-preview` | [상태 효과가 있는 닫기 dry-run](#scenario-mvp-active-close-task-state-effecting-dry-run-preview) 참고 |
| `MVP-ACTIVE-close-task-supersede-one-state-version` | [`supersede` 상태 버전](#scenario-mvp-active-close-task-supersede-one-state-version) 참고 |

<a id="scenario-mvp-active-registered-surface-mismatch-blocks-mutation"></a>
### `MVP-ACTIVE-registered-surface-mismatch-blocks-mutation`

조건:
- 상태 변경 전 로컬 접점이 등록 정보와 맞지 않습니다.

담당 문서 링크:
- [에이전트 통합](agent-integration.md)
- [API 오류](api/errors.md)
- [보안](security.md)

<a id="scenario-mvp-active-verified-local-surface-allows-owner-mutation"></a>
### `MVP-ACTIVE-verified-local-surface-allows-owner-mutation`

조건:
- 확인된 로컬 접점은 담당 경로의 상태 변경 확인만 허용합니다.

담당 문서 링크:
- [에이전트 통합](agent-integration.md)
- [공통 요청 규칙](api/mvp-api.md#공통-요청-규칙)
- [저장 효과](storage-effects.md)

<a id="scenario-mvp-active-single-access-class-per-public-request"></a>
### `MVP-ACTIVE-single-access-class-per-public-request`

조건:
- 공개 API 요청 하나에는 요청 수준 `access_class` 하나만 있습니다.

담당 문서 링크:
- [API 값 집합](api/schema-value-sets.md)
- [공통 요청 규칙](api/mvp-api.md#공통-요청-규칙)
- [보안](security.md)

<a id="scenario-mvp-active-detective-display-capability-gated"></a>
### `MVP-ACTIVE-detective-display-capability-gated`

조건:
- `detective` 표현은 지원되는 관찰 범위가 있을 때만 가능합니다.

담당 문서 링크:
- [보안](security.md)
- [에이전트 통합](agent-integration.md)

<a id="scenario-mvp-active-shaping-readiness-gap-blocks-or-asks"></a>
### `MVP-ACTIVE-shaping-readiness-gap-blocks-or-asks`

조건:
- 구체화 공백은 별도 계획 아티팩트가 아니라 담당 경로의 차단 사유나 판단 후보로 남습니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [API 상태 스키마](api/schema-state.md)
- [상태 메서드](api/method-status.md)
- [사용자 판단 메서드](api/method-user-judgment.md)

<a id="scenario-mvp-active-project-state-version-stale-mutation-rejected"></a>
### `MVP-ACTIVE-project-state-version-stale-mutation-rejected`

조건:
- 오래된 프로젝트 전체 상태 버전은 커밋 전에 실패합니다.

담당 문서 링크:
- [API 오류](api/errors.md)
- [저장소 버전 관리](storage-versioning.md)
- [저장 효과](storage-effects.md)

<a id="scenario-mvp-active-dry-run-pre-commit-failure-rejected"></a>
### `MVP-ACTIVE-dry-run-pre-commit-failure-rejected`

조건:
- `dry_run`은 검증, 접근, 역량, 오래된 상태 거절을 우회하지 않습니다.

담당 문서 링크:
- [API 코어 스키마](api/schema-core.md)
- [API 오류](api/errors.md)
- [저장 효과](storage-effects.md)

<a id="scenario-mvp-active-status-close-blockers-read-only"></a>
### `MVP-ACTIVE-status-close-blockers-read-only`

조건:
- 상태와 닫기 확인 차단 사유는 저장 변경 없이 읽을 수 있습니다.

담당 문서 링크:
- [상태 메서드](api/method-status.md)
- [Task 닫기 메서드](api/method-close-task.md)
- [API 상태 스키마](api/schema-state.md)
- [저장 효과](storage-effects.md)

<a id="scenario-mvp-active-sensitive-approval-records-sensitive-action-scope"></a>
### `MVP-ACTIVE-sensitive-approval-records-sensitive-action-scope`

조건:
- 민감 동작 승인은 Write Authorization, 최종 수락과 분리됩니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [API 판단 스키마](api/schema-judgment.md)
- [보안](security.md)

<a id="scenario-mvp-active-prepare-write-requires-compatible-scope-and-approval"></a>
### `MVP-ACTIVE-prepare-write-requires-compatible-scope-and-approval`

조건:
- `prepare_write`는 협력형 제품 파일 호환성 경로입니다.

담당 문서 링크:
- [쓰기 준비 메서드](api/method-prepare-write.md)
- [Core 모델](core-model.md)
- [보안](security.md)

<a id="scenario-mvp-active-authorized-attempt-scope-product-file-write-only"></a>
### `MVP-ACTIVE-authorized-attempt-scope-product-file-write-only`

조건:
- `AuthorizedAttemptScope`는 제품 파일 쓰기 범위만 다룹니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [쓰기 준비 메서드](api/method-prepare-write.md)
- [API 판단 스키마](api/schema-judgment.md)

<a id="scenario-mvp-active-record-run-consumes-write-authorization-once"></a>
### `MVP-ACTIVE-record-run-consumes-write-authorization-once`

조건:
- 호환되는 Run 기록은 맞는 Write Authorization을 한 번 소비합니다.

담당 문서 링크:
- [실행 기록 메서드](api/method-record-run.md)
- [저장 효과](storage-effects.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="scenario-mvp-active-stage-artifact-temporary-handle-only"></a>
### `MVP-ACTIVE-stage-artifact-temporary-handle-only`

조건:
- 스테이징은 임시 스테이징 핸들만 만듭니다.

담당 문서 링크:
- [아티팩트 스테이징 메서드](api/method-stage-artifact.md)
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 저장소](storage-artifacts.md)

<a id="scenario-mvp-active-record-run-artifact-input-validation-order"></a>
### `MVP-ACTIVE-record-run-artifact-input-validation-order`

조건:
- Run 아티팩트 입력은 승격이나 연결 전에 검증됩니다.

담당 문서 링크:
- [실행 기록 메서드](api/method-record-run.md)
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 저장소](storage-artifacts.md)

<a id="scenario-mvp-active-record-run-promotes-staged-artifact-to-artifact-ref"></a>
### `MVP-ACTIVE-record-run-promotes-staged-artifact-to-artifact-ref`

조건:
- 호환되는 Run 기록은 스테이징 핸들을 지속 `ArtifactRef`로 승격할 수 있습니다.

담당 문서 링크:
- [아티팩트 저장소](storage-artifacts.md)
- [실행 기록 메서드](api/method-record-run.md)
- [저장 효과](storage-effects.md)

<a id="scenario-mvp-active-record-run-rejects-staged-artifact-surface-instance-mismatch"></a>
### `MVP-ACTIVE-record-run-rejects-staged-artifact-surface-instance-mismatch`

조건:
- 스테이징 핸들의 출처가 맞지 않으면 승격이 거절됩니다.

담당 문서 링크:
- [아티팩트 저장소](storage-artifacts.md)
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [API 오류](api/errors.md)

<a id="scenario-mvp-active-record-run-links-existing-artifact-without-registering-bytes"></a>
### `MVP-ACTIVE-record-run-links-existing-artifact-without-registering-bytes`

조건:
- 이미 지속되는 아티팩트는 새 바이트 등록 없이 연결될 수 있습니다.

담당 문서 링크:
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 저장소](storage-artifacts.md)
- [실행 기록 메서드](api/method-record-run.md)

<a id="scenario-mvp-active-captured-artifact-rejected-in-active-mvp"></a>
### `MVP-ACTIVE-captured-artifact-rejected-in-active-mvp`

조건:
- 접점 자체 캡처 아티팩트 출처는 현재 MVP 아티팩트 권한이 아닙니다.

담당 문서 링크:
- [현재 MVP 범위](active-mvp-scope.md)
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [이후 후보 색인](../later/index.md)

<a id="scenario-mvp-active-close-task-complete-stale-state-version-rejected"></a>
### `MVP-ACTIVE-close-task-complete-stale-state-version-rejected`

조건:
- 오래된 상태는 닫기 준비 상태 평가 전에 실패합니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [API 오류](api/errors.md)
- [저장 효과](storage-effects.md)

<a id="scenario-mvp-active-close-task-complete-stale-write-authorization-basis-rejected"></a>
### `MVP-ACTIVE-close-task-complete-stale-write-authorization-basis-rejected`

조건:
- 닫기 관련 Write Authorization 근거가 오래됐으면 닫기 커밋 전에 실패합니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [API 오류](api/errors.md)
- [저장소 버전 관리](storage-versioning.md)

<a id="scenario-mvp-active-close-task-blocks-current-write-compatibility"></a>
### `MVP-ACTIVE-close-task-blocks-current-write-compatibility`

조건:
- 닫기는 의미적 쓰기 호환성 때문에 막힐 수 있습니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [Task 닫기 메서드](api/method-close-task.md)
- [API 상태 스키마](api/schema-state.md)

<a id="scenario-mvp-active-close-task-blocks-evidence-insufficient"></a>
### `MVP-ACTIVE-close-task-blocks-evidence-insufficient`

조건:
- 닫기는 필수 증거 부족 때문에 막힐 수 있습니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [API 상태 스키마](api/schema-state.md)
- [API 오류](api/errors.md)

<a id="scenario-mvp-active-close-task-blocks-required-artifact-unavailable"></a>
### `MVP-ACTIVE-close-task-blocks-required-artifact-unavailable`

조건:
- 닫기는 필수 아티팩트 가용성 때문에 막힐 수 있습니다.

담당 문서 링크:
- [API 상태 스키마](api/schema-state.md)
- [아티팩트 저장소](storage-artifacts.md)
- [API 오류](api/errors.md)

<a id="scenario-mvp-active-close-task-blocks-final-acceptance-missing"></a>
### `MVP-ACTIVE-close-task-blocks-final-acceptance-missing`

조건:
- 닫기는 호환되는 최종 수락이 없어 막힐 수 있습니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [API 판단 스키마](api/schema-judgment.md)
- [Task 닫기 메서드](api/method-close-task.md)

<a id="scenario-mvp-active-close-task-blocks-visible-unaccepted-residual-risk"></a>
### `MVP-ACTIVE-close-task-blocks-visible-unaccepted-residual-risk`

조건:
- 닫기는 보이는 잔여 위험에 대한 호환되는 수락이 없어 막힐 수 있습니다.

담당 문서 링크:
- [Core 모델](core-model.md)
- [API 판단 스키마](api/schema-judgment.md)
- [API 상태 스키마](api/schema-state.md)

<a id="scenario-mvp-active-close-task-check-read-only"></a>
### `MVP-ACTIVE-close-task-check-read-only`

조건:
- `harness.close_task intent=check`는 읽기 전용입니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [API 코어 스키마](api/schema-core.md)
- [저장 효과](storage-effects.md)

<a id="scenario-mvp-active-close-task-state-effecting-dry-run-preview"></a>
### `MVP-ACTIVE-close-task-state-effecting-dry-run-preview`

조건:
- 상태 효과가 있는 닫기 의도값은 유효하고 미리보기 가능할 때만 `dry_run` 미리보기를 사용합니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [API 코어 스키마](api/schema-core.md)
- [저장 효과](storage-effects.md)

<a id="scenario-mvp-active-close-task-supersede-one-state-version"></a>
### `MVP-ACTIVE-close-task-supersede-one-state-version`

조건:
- `supersede`는 유효할 때 프로젝트 전체 상태 변경 하나를 쓰는 성공 완료가 아닌 종료 경로입니다.

담당 문서 링크:
- [Task 닫기 메서드](api/method-close-task.md)
- [Core 모델](core-model.md)
- [저장 효과](storage-effects.md)

## 향후 항목을 목록으로만 유지하는 경계

향후 픽스처 계열은 [정책과 적합성: 향후 픽스처 계열](../later/policy-and-conformance.md#future-fixture-families)에 둡니다. 이후 후보 색인은 이름만 보존하며, 이 문서는 그 목록을 반복하지 않습니다.

향후 계열 이름은 시나리오 스크립트, 픽스처 본문, 활성 API 페이로드 예시, 실행기 또는 보고 요구사항, 현재 MVP 범위, 구현 작업, 현재 결과, 현재 런타임 증명이 아닙니다. 향후 담당 문서가 좁은 동작을 범위, 대체 동작, 정확한 계약, 증명 경로 기대치와 함께 승격해야 실행 가능한 픽스처 자료가 생깁니다.

## 지표 경계

현재 문서 세트에서 지표는 적합성 권한이 아닙니다. 향후 로컬 지표와 이후 후보인 적합성 보고는 진단이나 계획에 유용할 수 있지만, 담당 문서가 승격하기 전에는 읽기 전용 파생 표시나 이후 후보로 남습니다.

지표는 Core 상태를 만들거나, 증거를 충족하거나, QA 또는 검증을 통과시키거나, 쓰기를 승인하거나, 최종 결과를 수락하거나, 잔여 위험을 수락하거나, 작업을 닫거나, 구현 준비 상태를 증명하거나, 런타임 적합성을 대신하면 안 됩니다. 향후 지표가 승격되면 담당 문서가 원천 기록, 최신성 경계, 표시 문구, 대체 불가 규칙을 정의해야 합니다.
