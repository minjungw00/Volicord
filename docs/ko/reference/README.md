# 참조 문서 색인

이 색인은 "이 질문의 담당 문서는 어디인가?"에 답하기 위한 문서입니다. 이 README는 담당 문서로 안내할 뿐, API 계약, 스키마, 저장 효과, 보안 보장, 현재 MVP 범위를 정의하지 않습니다.

이 문서들은 하네스 서버를 위한 참조 자료입니다. 이 저장소에 런타임 구현, 런타임 상태, 생성된 아티팩트, 상태 보기, 증거 기록, QA 기록, 수락 기록, 닫기 기록, 적합성 결과가 있다는 뜻이 아닙니다.

## 읽기 규칙

- 먼저 답해야 할 질문을 고르고, 그 질문에 맞는 담당 문서만 엽니다.
- 계약 세부사항은 담당 문서에 둡니다. 이 색인에 필드 목록, 응답 분기, DDL, 값 집합, 보장 수준을 길게 적고 싶어진다면 담당 문서로 옮기고 여기에는 경로만 남깁니다.
- 한영 문서나 용어 의미가 바뀌는 편집은 같은 작업 묶음에서 영어/한국어 담당 문서를 함께 맞춥니다.
- 번역이나 의미 일치 검토가 아니라면 같은 `doc_id`의 영어/한국어 대응 문서를 한 프롬프트에 함께 넣지 않습니다.
- 정확한 식별자는 백틱으로 보존하고, 의미는 담당 문서가 정하게 둡니다.

## 구현 담당자 읽기 경로

제품 경계에서 정확한 담당 문서로 들어갈 때는 아래 순서를 사용합니다.

| 단계 | 담당 문서 경로 |
|---|---|
| 현재 MVP 범위 | `scope.md` |
| API 메서드 목록 | `api/methods.md` |
| API 메서드 동작 | [API 메서드 담당 문서](#api-메서드-담당-문서) |
| 스키마 형태 | [API와 스키마 담당 문서](#api와-스키마-담당-문서) |
| 저장 효과 | `storage-effects.md` |

이 경로는 정확한 담당 문서가 필요한 구현 담당자와 검토자를 위한 것입니다. 처음 읽는 사용자와 작업 중인 사용자는 [시작하기](../start.md)와 [사용자 가이드](../use/user-guide.md)에서 시작합니다.

## 현재 범위

| 질문 | 담당 문서 |
|---|---|
| 현재 MVP 포함 범위는 어디서 정의되는가? | `scope.md` |
| 현재 MVP 제외 범위는 어디서 정의되는가? | `scope.md` |
| 기능이 현재 활성인가, 프로필 조건부인가, 예약된 값인가, 범위 밖인가? | `scope.md` |
| 현재 MVP에서 `isolated`가 활성인가? | `scope.md`, `security.md` |
| 구현 경로는 어디서 설명하는가? | `../build/implementation-guide.md` |
| 문서 경계는 어디서 정의되는가? | `runtime-boundaries.md`, `scope.md` |

## 담당 문서 찾기

| 질문 | 담당 문서 |
|---|---|
| Core 권한은 어디서 정의되는가? | `core-model.md` |
| 활성 API 메서드 목록은 어디에 있는가? | `api/methods.md` |
| 공통 API 요청 래퍼는 어디서 정의되는가? | `api/schema-core.md` |
| 응답 분기는 어디서 정의되는가? | `api/schema-core.md` |
| 공개 오류 코드는 어디서 정의되는가? | `api/errors.md` |
| 저장소 기록은 어디서 정의되는가? | `storage-records.md` |
| 저장 효과는 어디서 정의되는가? | `storage-effects.md` |
| 메서드별 저장 효과는 어디서 정의되는가? | `storage-effects.md` |
| 보안 주장은 어디서 정의되는가? | `security.md` |
| 제품 용어는 어디서 정의되는가? | `glossary.md`, `../../terminology-map.yaml` |
| 상태 보기 권한은 어디서 정의되는가? | `projection-and-templates.md` |
| 템플릿 본문은 어디서 정의되는가? | `template-bodies.md` |

## API와 스키마 담당 문서

| 질문 | 담당 문서 |
|---|---|
| API 예시는 어떤 시나리오를 쓰는가? | `api/methods.md`, `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| API 예시가 문서 유지보수를 시나리오로 써도 되는가? | `../maintain/authoring-guide.md` |
| API 예시 점검은 어디서 정의되는가? | `../maintain/checks.md`, `../maintain/authoring-guide.md` |
| API 예시 정합성 점검은 어디서 정의되는가? | `../maintain/checks.md`, `../maintain/authoring-guide.md` |
| API 예시 필드 이름 점검은 어디서 정의되는가? | `../maintain/checks.md`, `../maintain/authoring-guide.md` |
| 활성 API 메서드 목록은 어디에 있는가? | `api/methods.md` |
| API 메서드 이름 값은 어디서 정의되는가? | `api/schema-value-sets.md` |
| 메서드 페이로드 필드는 어디서 정의되는가? | [API 메서드 담당 문서](#api-메서드-담당-문서) |
| 공통 페이로드 스키마는 어디서 정의되는가? | `api/schema-core.md` |
| `harness.status`의 `state_version` 예시 규칙은 어디에 있는가? | `api/method-status.md`, `../maintain/checks.md` |
| `harness.prepare_write`는 무엇을 반환하는가? | `api/method-prepare-write.md` |
| `harness.prepare_write` 응답 분기는 어디서 정의되는가? | `api/schema-core.md` |
| `harness.prepare_write` 상태 형태는 어디서 정의되는가? | `api/schema-state.md` |
| `harness.prepare_write` 판단 형태는 어디서 정의되는가? | `api/schema-judgment.md` |
| `harness.prepare_write` 민감 동작 승인은 어디서 정의되는가? | `api/method-prepare-write.md` |
| `ToolRejectedResponse`는 어디서 정의되는가? | `api/schema-core.md` |
| `STATE_VERSION_CONFLICT`는 차단 사유 코드인가? | `api/errors.md` |
| `dry_run=true`에서 `harness.close_task`는 무엇을 반환하는가? | `api/method-close-task.md` |
| enum 형태 API 값은 어디서 정의되는가? | `api/schema-value-sets.md` |
| `complete`는 enum 값인가, 산문인가? | `../../terminology-map.yaml`, `glossary.md` |
| 접근 등급은 어디서 정의되는가? | `api/schema-value-sets.md` |
| `DryRunSummary`, `PlannedEffect`, `PlannedBlocker`는 어디서 정의되는가? | `api/schema-core.md` |
| 보장 라벨 값은 어디서 정의되는가? | `api/schema-value-sets.md` |
| 보장 의미는 어디서 정의되는가? | `security.md` |
| `isolated` 값은 어디서 정의되는가? | `api/schema-value-sets.md` |
| `isolated` 보장 의미는 어디서 정의되는가? | `security.md` |
| 상태 요약 형태는 어디서 정의되는가? | `api/schema-state.md` |
| 아티팩트 참조 형태는 어디서 정의되는가? | `api/schema-artifacts.md` |
| 판단 입력 형태는 어디서 정의되는가? | `api/schema-judgment.md` |

<a id="api-메서드-담당-문서"></a>

## API 메서드 담당 문서

| 질문 | 담당 문서 |
|---|---|
| `harness.intake` 메서드 동작은 어디서 정의되는가? | `api/method-intake.md` |
| `harness.update_scope` 메서드 동작은 어디서 정의되는가? | `api/method-update-scope.md` |
| `harness.status` 메서드 동작은 어디서 정의되는가? | `api/method-status.md` |
| `harness.prepare_write` 메서드 동작은 어디서 정의되는가? | `api/method-prepare-write.md` |
| `harness.stage_artifact` 메서드 동작은 어디서 정의되는가? | `api/method-stage-artifact.md` |
| `harness.record_run` 메서드 동작은 어디서 정의되는가? | `api/method-record-run.md` |
| `harness.record_run` 증거 메서드 동작은 어디서 정의되는가? | `api/method-record-run.md`, `storage-effects.md` |
| `harness.record_run` 저장 효과는 어디서 정의되는가? | `storage-effects.md` |
| `harness.request_user_judgment` 메서드 동작은 어디서 정의되는가? | `api/method-user-judgment.md` |
| `harness.record_user_judgment` 메서드 동작은 어디서 정의되는가? | `api/method-user-judgment.md` |
| `harness.close_task` 메서드 동작은 어디서 정의되는가? | `api/method-close-task.md` |

## 저장소 담당 문서

| 질문 | 담당 문서 |
|---|---|
| 저장소 문서 묶음은 어디서 시작하는가? | `storage.md` |
| Harness Runtime Home 분리는 어디서 정의되는가? | `runtime-boundaries.md` |
| 로컬 저장소 가정은 어디서 정의되는가? | `storage-records.md` |
| 저장소 기록 값은 어디서 정의되는가? | `storage-records.md` |
| `CloseReadinessBlocker`는 저장소 행인가? | `storage-records.md` |
| 아티팩트 스테이징은 증거를 만드는가? | `storage-artifacts.md`, `storage-effects.md` |
| 아티팩트 스테이징과 승격은 어디서 정의되는가? | `storage-artifacts.md` |
| 아티팩트 참조 스키마는 어디서 정의되는가? | `api/schema-artifacts.md` |
| 스테이징 핸들 검증은 어디서 정의되는가? | `storage-artifacts.md` |
| 상태 시계와 마이그레이션은 어디서 정의되는가? | `storage-versioning.md` |

## 보안과 런타임 담당 문서

| 질문 | 담당 문서 |
|---|---|
| 현재 MVP는 OS 수준 샌드박싱을 제공하는가? | `security.md` |
| `isolated` 보장 의미는 어디서 정의되는가? | `security.md` |
| 보장 의미는 어디서 정의되는가? | `security.md` |
| 런타임 분리는 어디서 정의되는가? | `runtime-boundaries.md` |
| 로컬 커넥터 동작은 어디서 정의되는가? | `agent-integration.md` |
| 확인된 접점 맥락은 어디서 정의되는가? | `agent-integration.md` |
| 확인된 보장 경계는 어디서 정의되는가? | `security.md` |
| 접점별 레시피는 어디서 정의되는가? | `../use/surface-recipes.md` |
| 보안 오류 경로는 어디서 정의되는가? | `api/errors.md` |

## 사용자 판단과 닫기 준비 상태 담당 문서

| 질문 | 담당 문서 |
|---|---|
| 사용자 소유 판단 의미는 어디서 정의되는가? | `core-model.md` |
| 사용자 판단 프롬프트 동작은 어디서 정의되는가? | `api/method-user-judgment.md`, `core-model.md` |
| 사용자 판단 스키마는 어디서 정의되는가? | `api/schema-judgment.md` |
| 민감 동작 승인 의미는 어디서 정의되는가? | `core-model.md` |
| 민감 동작 승인 스키마는 어디서 정의되는가? | `api/schema-judgment.md` |
| 민감 동작 승인 보안 의미는 어디서 정의되는가? | `security.md` |
| 닫기 준비 상태 의미는 어디서 정의되는가? | `core-model.md` |
| `harness.close_task` 동작은 어디서 정의되는가? | `api/method-close-task.md` |
| 닫기 차단 사유 형태는 어디서 정의되는가? | `api/schema-state.md` |
| 닫기 오류 경로는 어디서 정의되는가? | `api/errors.md` |
| 수락과 잔여 위험 경계는 어디서 정의되는가? | `core-model.md` |
| 수락된 위험 스키마는 어디서 정의되는가? | `api/schema-judgment.md` |
| 수락된 위험 값은 어디서 정의되는가? | `api/schema-value-sets.md` |
| 압축된 증거 요약 의미는 어디서 정의되는가? | `core-model.md` |
| 압축된 증거 요약 형태는 어디서 정의되는가? | `api/schema-state.md` |

## 범위와 유지보수 문서

| 질문 | 담당 문서 |
|---|---|
| 예약된 값, 프로필 조건부 값, 범위 밖 기능은 활성인가? | `scope.md` |
| 현재 범위 제외 항목은 어디서 정의되는가? | `scope.md` |
| 승격 시점의 담당 문서 갱신은 무엇인가? | `glossary.md`, `scope.md` |
| 범위 밖 기능이 활성화되려면 무엇이 바뀌어야 하는가? | `scope.md`, 영향받는 담당 문서 |
| "Full close-readiness evaluation order"의 한국어 표현은 어디서 확인하는가? | `glossary.md`, `../maintain/translation-guide.md` |
| "close readiness"의 한국어 표현은 어디서 확인하는가? | `../../terminology-map.yaml`, `glossary.md`, `../maintain/translation-guide.md` |
| 한국어 표현과 번역 지침은 어디서 관리되는가? | `../maintain/translation-guide.md`, `../../terminology-map.yaml`, `glossary.md` |
| 문서 작성 규칙은 어디에 있는가? | `../maintain/authoring-guide.md` |
| 큰 Markdown 표 규칙은 어디서 정의되는가? | `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| 긴 Markdown 표는 언제 나누는가? | `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| 촘촘한 참조 문단은 언제 나누는가? | `../maintain/authoring-guide.md`, `../maintain/checks.md` |
| 문서 점검은 어디에 있는가? | `../maintain/checks.md` |
| 검색과 경로 메타데이터는 어디서 관리되는가? | `../../doc-index.yaml` |
| 에이전트가 먼저 읽어야 할 문서는 무엇인가? | `../../../AGENTS.md`, `../../doc-index.yaml` |
