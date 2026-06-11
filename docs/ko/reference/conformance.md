# 적합성 참조

## 1. 현재 상태

이 저장소는 문서 전용이며 아직 문서 검토 단계입니다. 여기에는 Harness Server 런타임, Harness Runtime Home, 실행 가능한 fixture 파일, 적합성 실행기, 생성된 적합성 보고서, 생성된 런타임 아티팩트, 현재 런타임 적합성 결과가 없습니다.

이 문서는 문서 수준의 적합성 의미, 향후 fixture 후보 형식, 주장 권한 경계, 간결한 시나리오 색인을 담당합니다. API 분기, 저장 효과, 접근 등급, 아티팩트 승격, 보안 보장, 닫기 준비 상태 동작은 이 문서가 정의하지 않습니다.

현재 범위의 기준 설명은 [현재 MVP 범위 참조](active-mvp-scope.md)를 확인하세요. 현재 단계와 인계 상태는 [MVP 계획의 저장소 상태](../build/mvp-plan.md#문서-수락-상태)가 담당합니다.

| 항목 | 현재 상태 | 담당 | 실행 가능 여부 |
|---|---|---|---|
| 현재 문서 기준 | 활성 참조 기준 | `docs/ko/reference/conformance.md` | 아니요. 런타임 실행 기준이 아닙니다. |
| 계획된 내부 스모크 목표 | 계획/문서화됨 | [MVP 계획의 첫 내부 스모크 목표](../build/mvp-plan.md#첫-내부-스모크-목표) | 아니요. |
| 향후 fixture 형식 | 후보 형식 | 이 문서 | 아니요. |
| 향후 실행 가능한 fixture | 미구현 | 향후 실행기와 담당 문서가 승격한 fixture | 아니요. |
| 이후 적합성 보고 | 이후 후보 | [이후 후보 색인](../later/index.md) | 아니요. |

이 문서의 "해야 합니다", "필수", "항상"은 현재 문서 기준이나 구현 뒤의 향후 서버/실행기 요건을 뜻합니다. 이 저장소에 실행 가능한 점검이 이미 있다는 뜻이 아닙니다.

## 2. 적합성이 뜻하는 것

향후 서버에서 적합성이란 실행 점검이 담당 문서가 정의한 동작 하나를 담당 문서의 권한 기록과 비교할 수 있다는 뜻입니다. 문서 점검은 링크, 용어, 담당 문서 경계, 현재/이후 문구, 보안 표현, 한영 의미 일치를 보는 별도 유지보수 보조 도구입니다.

향후 런타임 적합성 점검은 담당 문서가 권한 있는 사실로 정한 것만 판단해야 합니다. 특정 담당 문서가 그 사실을 승격하지 않았다면 생성된 글, 에이전트 요약, 렌더링된 보고서, 상태 문구, 문서 점검 라벨, 상태 보기를 권한으로 다루면 안 됩니다.

## 3. 아직 존재하지 않는 것

아래 항목은 향후 구현 작업이며 현재 저장소 내용이 아닙니다.

- Harness Server 런타임 또는 Harness Runtime Home 데이터
- 실행 가능한 fixture 파일 또는 fixture 디렉터리
- 적합성 실행기 또는 `harness conformance run` 구현
- 생성된 적합성 보고서, 생성된 런타임 아티팩트, 상태 보기, 운영 파일, 런타임 상태
- 현재 MVP 동작이나 이후 후보에 대한 현재 런타임 결과
- 예방적 차단, OS 권한 제어, 임의 도구 샌드박스, 변조 방지 저장소, 보안 격리, profile-gated `preventive` / `isolated` 보장 주장에 대한 현재 런타임 증명

이 문서의 예시는 계획을 도울 수 있습니다. 하지만 런타임 상태, 수락 증거, 닫기 준비 상태, 잔여 위험 수락, 생성된 보고서, 구현 준비 상태를 만들지 않습니다.

## 4. fixture 형식

fixture 형식은 향후 후보 형식일 뿐 현재 파일을 만들지 않습니다. Harness Server와 실행기가 생긴 뒤 승격된 fixture는 아래 부분을 담은 작은 구조화 기록이어야 합니다.

| 부분 | 목적 |
|---|---|
| `scenario_id` | 검토할 동작의 안정적인 식별자입니다. |
| 권한 맥락 | 동작 전에 필요한 Task, Change Unit, 상태 버전, 접점, 담당 문서 참조, Core 상태, 저장소 행, `ArtifactRef`, 접점 기능 사실입니다. |
| 동작 | 담당 요청 스키마를 사용하는 공개 Core, API, 운영자 요청 하나입니다. |
| 기대 주장 | 구조화된 응답 사실, 담당 문서가 소유하는 상태 변경 효과, 저장소 또는 아티팩트 사실, 차단 사유 사실, 오류 사실, 보장 표시 사실, 금지된 부작용의 필수 부재입니다. |
| 담당 문서 링크 | 정확한 값과 의미를 정의하는 API, Core, 저장소, 보안, 에이전트 통합, 아티팩트, 정책 담당 문서입니다. |

향후 구체화된 fixture는 공개 담당 스키마를 사용해야 합니다. fixture 전용 enum 값, 가짜 필드, 상태로 쓰는 지역화 표시 라벨, 글로만 된 기대값, 이후 후보 전용 값을 만들면 안 됩니다.

## 5. 주장 권한

주장 권한은 실행 가능한 fixture가 생긴 뒤 향후 fixture가 판단할 수 있는 사실의 좁은 범위입니다. 권한은 시나리오 설명이나 생성된 요약이 아니라 담당 문서가 정의한 사실에서 옵니다.

향후 주장은 담당 문서가 정의한 응답 사실, Core 상태, 저장 효과, 아티팩트 사실, 공개 `ErrorCode` 값, 구조화된 차단 사유, 보장 표시 사실, 금지된 부작용의 부재를 참조할 수 있습니다.

정확한 주장 세부사항은 아래 담당 문서에 남습니다.

| 주장 영역 | 담당 문서 |
|---|---|
| API 메서드와 응답 분기 동작 | [MVP API](api/mvp-api.md) |
| 공통 응답 분기와 `dry_run` 미리보기 형태 | [API 코어 스키마](api/schema-core.md) |
| 상태 요약, 차단 사유, 증거, 닫기 준비 상태 구조 | [API 상태 스키마](api/schema-state.md) |
| `ArtifactRef`, `ArtifactInput`, `StagedArtifactHandle` 형태 | [API 아티팩트 스키마](api/schema-artifacts.md) |
| `access_class` 값을 포함한 API 값 집합 | [API 값 집합](api/schema-value-sets.md) |
| 공개 오류와 우선순위 | [API 오류](api/errors.md) |
| 저장 효과, 효과 없음 분기, 상태 버전 효과 | [저장 효과](storage-effects.md) |
| 아티팩트 스테이징, 승격, 지속 저장, 본문 읽기 생명주기 | [아티팩트 저장소](storage-artifacts.md) |
| 보안 비주장과 보장 수준 | [보안](security.md) |
| 런타임 위치와 문서 전용 경계 | [런타임 경계](runtime-boundaries.md) |

## 6. 대표 시나리오 색인

아래 `scenario_id`는 향후 fixture 계획을 위한 작은 문서 기준입니다. fixture 본문, 현재 런타임 결과, 생성된 런타임 객체, 구현 계획이 아닙니다. 정확한 분기, 저장, 접근, 아티팩트, 보안, 닫기 준비 상태 계약은 위 담당 문서 링크를 사용합니다.

| 시나리오 ID | 초점 | 주 담당 문서 |
|---|---|---|
| `MVP-ACTIVE-registered-surface-mismatch-blocks-mutation` | 상태 변경 전 로컬 접점 불일치. | [에이전트 통합](agent-integration.md), [API 오류](api/errors.md), [보안](security.md) |
| `MVP-ACTIVE-verified-local-surface-allows-owner-mutation` | 확인된 로컬 접점은 담당 경로의 상태 변경 확인만 허용합니다. | [에이전트 통합](agent-integration.md), [MVP API](api/mvp-api.md), [저장 효과](storage-effects.md) |
| `MVP-ACTIVE-single-access-class-per-public-request` | 공개 API 요청 하나에는 요청 수준 `access_class` 하나만 있습니다. | [API 값 집합](api/schema-value-sets.md), [MVP API](api/mvp-api.md), [보안](security.md) |
| `MVP-ACTIVE-detective-display-capability-gated` | `detective` 표현은 지원되는 관찰 범위가 있을 때만 가능합니다. | [보안](security.md), [에이전트 통합](agent-integration.md) |
| `MVP-ACTIVE-shaping-readiness-gap-blocks-or-asks` | 구체화 공백은 별도 계획 아티팩트가 아니라 담당 경로의 차단 사유나 판단 후보로 남습니다. | [Core 모델](core-model.md), [API 상태 스키마](api/schema-state.md), [MVP API](api/mvp-api.md) |
| `MVP-ACTIVE-project-state-version-stale-mutation-rejected` | 오래된 프로젝트 전체 상태 버전은 커밋 전에 실패합니다. | [API 오류](api/errors.md), [저장소 버전 관리](storage-versioning.md), [저장 효과](storage-effects.md) |
| `MVP-ACTIVE-dry-run-pre-commit-failure-rejected` | `dry_run`은 검증, 접근, 역량, 오래된 상태 거절을 우회하지 않습니다. | [API 코어 스키마](api/schema-core.md), [API 오류](api/errors.md), [저장 효과](storage-effects.md) |
| `MVP-ACTIVE-status-close-blockers-read-only` | 상태와 닫기 확인 차단 사유는 저장 변경 없이 읽을 수 있습니다. | [MVP API](api/mvp-api.md), [API 상태 스키마](api/schema-state.md), [저장 효과](storage-effects.md) |
| `MVP-ACTIVE-sensitive-approval-records-sensitive-action-scope` | 민감 동작 승인은 Write Authorization, 최종 수락과 분리됩니다. | [Core 모델](core-model.md), [API 판단 스키마](api/schema-judgment.md), [보안](security.md) |
| `MVP-ACTIVE-prepare-write-requires-compatible-scope-and-approval` | `prepare_write`는 협력형 제품 파일 호환성 경로입니다. | [MVP API](api/mvp-api.md), [Core 모델](core-model.md), [보안](security.md) |
| `MVP-ACTIVE-authorized-attempt-scope-product-file-write-only` | `AuthorizedAttemptScope`는 제품 파일 쓰기 범위만 다룹니다. | [Core 모델](core-model.md), [MVP API](api/mvp-api.md), [API 판단 스키마](api/schema-judgment.md) |
| `MVP-ACTIVE-record-run-consumes-write-authorization-once` | 호환되는 Run 기록은 맞는 Write Authorization을 한 번 소비합니다. | [MVP API](api/mvp-api.md), [저장 효과](storage-effects.md), [저장소 버전 관리](storage-versioning.md) |
| `MVP-ACTIVE-stage-artifact-temporary-handle-only` | 스테이징은 임시 스테이징 핸들만 만듭니다. | [MVP API](api/mvp-api.md), [API 아티팩트 스키마](api/schema-artifacts.md), [아티팩트 저장소](storage-artifacts.md) |
| `MVP-ACTIVE-record-run-artifact-input-validation-order` | Run 아티팩트 입력은 승격이나 연결 전에 검증됩니다. | [MVP API](api/mvp-api.md), [API 아티팩트 스키마](api/schema-artifacts.md), [아티팩트 저장소](storage-artifacts.md) |
| `MVP-ACTIVE-record-run-promotes-staged-artifact-to-artifact-ref` | 호환되는 Run 기록은 스테이징 핸들을 지속 `ArtifactRef`로 승격할 수 있습니다. | [아티팩트 저장소](storage-artifacts.md), [MVP API](api/mvp-api.md), [저장 효과](storage-effects.md) |
| `MVP-ACTIVE-record-run-rejects-staged-artifact-surface-instance-mismatch` | 스테이징 핸들의 출처가 맞지 않으면 승격이 거절됩니다. | [아티팩트 저장소](storage-artifacts.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 오류](api/errors.md) |
| `MVP-ACTIVE-record-run-links-existing-artifact-without-registering-bytes` | 이미 지속되는 아티팩트는 새 바이트 등록 없이 연결될 수 있습니다. | [API 아티팩트 스키마](api/schema-artifacts.md), [아티팩트 저장소](storage-artifacts.md), [MVP API](api/mvp-api.md) |
| `MVP-ACTIVE-captured-artifact-rejected-in-active-mvp` | 접점 자체 캡처 아티팩트 출처는 현재 MVP 아티팩트 권한이 아닙니다. | [현재 MVP 범위](active-mvp-scope.md), [API 아티팩트 스키마](api/schema-artifacts.md), [이후 후보 색인](../later/index.md) |
| `MVP-ACTIVE-close-task-complete-stale-state-version-rejected` | 오래된 상태는 닫기 준비 상태 평가 전에 실패합니다. | [MVP API](api/mvp-api.md), [API 오류](api/errors.md), [저장 효과](storage-effects.md) |
| `MVP-ACTIVE-close-task-complete-stale-write-authorization-basis-rejected` | 닫기 관련 Write Authorization 근거가 오래됐으면 닫기 커밋 전에 실패합니다. | [MVP API](api/mvp-api.md), [API 오류](api/errors.md), [저장소 버전 관리](storage-versioning.md) |
| `MVP-ACTIVE-close-task-blocks-current-write-compatibility` | 닫기는 의미적 쓰기 호환성 때문에 막힐 수 있습니다. | [Core 모델](core-model.md), [MVP API](api/mvp-api.md), [API 상태 스키마](api/schema-state.md) |
| `MVP-ACTIVE-close-task-blocks-evidence-insufficient` | 닫기는 필수 증거 부족 때문에 막힐 수 있습니다. | [Core 모델](core-model.md), [API 상태 스키마](api/schema-state.md), [API 오류](api/errors.md) |
| `MVP-ACTIVE-close-task-blocks-required-artifact-unavailable` | 닫기는 필수 아티팩트 가용성 때문에 막힐 수 있습니다. | [API 상태 스키마](api/schema-state.md), [아티팩트 저장소](storage-artifacts.md), [API 오류](api/errors.md) |
| `MVP-ACTIVE-close-task-blocks-final-acceptance-missing` | 닫기는 호환되는 최종 수락이 없어 막힐 수 있습니다. | [Core 모델](core-model.md), [API 판단 스키마](api/schema-judgment.md), [MVP API](api/mvp-api.md) |
| `MVP-ACTIVE-close-task-blocks-visible-unaccepted-residual-risk` | 닫기는 보이는 잔여 위험에 대한 호환되는 수락이 없어 막힐 수 있습니다. | [Core 모델](core-model.md), [API 판단 스키마](api/schema-judgment.md), [API 상태 스키마](api/schema-state.md) |
| `MVP-ACTIVE-close-task-check-read-only` | `harness.close_task intent=check`는 읽기 전용입니다. | [MVP API](api/mvp-api.md), [API 코어 스키마](api/schema-core.md), [저장 효과](storage-effects.md) |
| `MVP-ACTIVE-close-task-state-effecting-dry-run-preview` | 상태 효과가 있는 닫기 intent는 유효하고 미리보기 가능할 때만 `dry_run` 미리보기를 사용합니다. | [MVP API](api/mvp-api.md), [API 코어 스키마](api/schema-core.md), [저장 효과](storage-effects.md) |
| `MVP-ACTIVE-close-task-supersede-one-state-version` | supersede는 유효할 때 프로젝트 전체 상태 변경 하나를 쓰는 성공 완료가 아닌 종료 경로입니다. | [MVP API](api/mvp-api.md), [Core 모델](core-model.md), [저장 효과](storage-effects.md) |

## 7. 향후 항목을 목록으로만 유지하는 경계

향후 fixture 계열은 [정책과 적합성: 향후 fixture 계열](../later/policy-and-conformance.md#future-fixture-families)에 둡니다. 이후 후보 색인은 이름만 보존하며, 이 문서는 그 목록을 반복하지 않습니다.

향후 계열 이름은 시나리오 스크립트, fixture 본문, 활성 API 페이로드 예시, 실행기 또는 보고 요구사항, 현재 MVP 범위, 구현 작업, 현재 결과, 현재 런타임 증명이 아닙니다. 향후 담당 문서가 좁은 동작을 범위, 대체 동작, 정확한 계약, 증명 경로 기대치와 함께 승격해야 실행 가능한 fixture 자료가 생깁니다.

## 8. 지표 경계

현재 문서 세트에서 지표는 적합성 권한이 아닙니다. 향후 로컬 지표와 이후 후보인 적합성 보고는 진단이나 계획에 유용할 수 있지만, 담당 문서가 승격하기 전에는 읽기 전용 파생 표시나 이후 후보로 남습니다.

지표는 Core 상태를 만들거나, 증거를 충족하거나, QA 또는 검증을 통과시키거나, 쓰기를 승인하거나, 최종 결과를 수락하거나, 잔여 위험을 수락하거나, 작업을 닫거나, 구현 준비 상태를 증명하거나, 런타임 적합성을 대신하면 안 됩니다. 향후 지표가 승격되면 담당 문서가 원천 기록, 최신성 경계, 표시 문구, 대체 불가 규칙을 정의해야 합니다.
