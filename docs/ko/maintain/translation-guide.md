# 번역 가이드

영어와 한국어 하네스 문서를 함께 고칠 때 이 번역 가이드를 사용합니다. 이 문서는 살아 있는 이중 언어 편집 규칙입니다. 재설계 이력도 아니고, 런타임 적합성이나 구현 준비 상태 기록도 아닙니다.

## 1. 의미 일치, 줄 단위 번역 아님

영어와 한국어 문서는 모두 활성 문서입니다. 어느 한쪽도 보관본, 부록, 번역 전용 사본이 아닙니다. 대응 문서는 하나의 의미를 공유해야 하며, 편집 뒤에는 양쪽 언어가 같은 뜻으로 정렬되어 있어야 합니다.

목표는 문장을 한 줄씩 맞추는 번역이 아니라 의미 일치입니다. 한국어가 더 분명하다면 제목, 문단 나눔, 예시는 영어와 달라도 됩니다. 단, 같은 의미, 허용된 담당 경로, active/later 경계, 정확한 식별자는 유지해야 합니다.

의미가 바뀌면 영어와 한국어를 같은 작업 묶음에서 고칩니다. 한국어 편집 중 영어 의미 문제가 보이면 양쪽을 함께 고칩니다.

## 2. 경로와 맥락의 의미 일치

README와 유지보수 문서의 경로 표는 현재 간결 구조와 [doc-index.yaml](../../doc-index.yaml)만 가리켜야 합니다. 깊은 담당 문서 경로를 경로 표에 직접 추가하지 말고 [참조 색인](../reference/README.md)에서 정확한 계약 담당 문서를 고릅니다.

에이전트는 일반 작업에서 하나의 `doc_id`에 한 언어만 불러옵니다. 번역이나 의미 일치 검토에 비교가 꼭 필요할 때만 두 언어를 함께 봅니다. 프롬프트에는 현재 작업, 담당 문서의 필요한 부분, 범위와 범위 밖, 사용자 판단 대기 사항, 차단 사유, 다음 안전한 행동, 증거 공백, 닫기를 막는 이유, 잔여 위험, 하네스가 확인할 수 있는 수준, 출처 최신성만 작게 둡니다.

## 3. 보존할 정확한 식별자

아래 항목은 양쪽 언어에서 정확히 보존합니다.

- 파일 경로와 앵커
- `doc_id` 값
- API 메서드 이름, 도구 이름, 리소스 이름
- 스키마 이름과 스키마 필드
- enum 값과 상태 값
- 오류 코드와 validator ID
- DDL, 테이블 이름, 열 이름, 저장소 식별자
- 템플릿 이름
- 코드 식별자, 리터럴 표시자, 자리 표시자 이름, 코드 형태 문자열

코드 블록, 스키마, API 예시, 파일 경로, 필드 목록 안의 정확한 문자열은 번역하지 않습니다. 지역화된 표시 라벨은 렌더링되는 표시 문자열이지 기준 식별자가 아닙니다.

정확한 식별자는 계약, 경로, 스키마, 저장소, API, 템플릿, 검색에서 그대로 쓰는 기준 문자열입니다. 설명 문장은 그 주변에서 독자에게 의미를 전달하는 문장입니다. 정확한 식별자는 그대로 복사하되, 설명 문장은 자연스러운 한국어로 번역하거나 다시 씁니다.

한국어 사용자 대상 문장에서는 enum 이름과 스키마 값을 그대로 표시 라벨처럼 쓰지 않습니다. 원문 값 자체를 설명하는 문맥일 때만 그대로 쓰고, 보통은 자연스러운 한국어 라벨을 먼저 둔 뒤 계약 정확도나 검색이 필요할 때 정확한 영어 값을 덧붙입니다.

## 4. 자연스러운 한국어 규칙

한국어 문서는 자연스러운 한국어 기술 문장으로 씁니다.

- 짧고 분명한 문장을 선호합니다.
- 사용자 대상 문장에서는 한국어 개념을 먼저 둡니다.
- 정확도, 검색, 담당 문서 연결에 필요할 때만 정확한 영어 식별자를 붙입니다.
- 같은 개념은 파일이 달라도 같은 한국어 표현으로 씁니다. 새 한국어 용어가 필요하면 여러 변형을 흩뿌리기 전에 용어집이나 이 가이드를 먼저 갱신합니다.
- 영어 원문에 있었다는 이유만으로 한국어 문장에 영어 명사구를 남기지 않습니다. 정확한 식별자나 의도적인 하네스 라벨이 아니라면 설명 성격의 영어 명사구는 한국어 개념으로 옮깁니다.
- 사용자 대상 문장에서는 enum 값이나 상태 값을 그대로 쓰기보다 자연스러운 한국어 표시 라벨을 우선합니다. 정확한 원문 값을 설명하는 문맥에서만 그 값을 주어로 둡니다.
- 사용자 대상 템플릿에서는 스키마와 enum 개념을 자연스러운 표시 문구로 보여 줍니다. 정확한 계약 값 자체를 설명할 때만 원문 값을 노출합니다.
- 영어의 부정 병렬 구조를 한국어에서 압축해 의미가 뒤집히게 만들지 않습니다. 조건의 뜻이 "보이지 않거나, 요구될 때 수락되지 않은 경우"라면 각 부정을 드러내고, 앞 요구사항의 부정을 생략한 표현은 쓰지 않습니다.
- 영어 명사에 한국어 조사만 붙인 문장이 대부분인 형태를 피합니다.
- 주변 문장이 완전한 한국어여도 정확한 식별자는 그대로 둡니다.

좋은 한국어 문서는 영어 문장 순서를 바꾸고, 절을 나누거나 합치고, 읽기 좋은 문단 흐름으로 재구성할 수 있습니다. 줄 단위 번역처럼 읽히면 다시 씁니다.

## 5. 사용자용 용어

한국어 사용자 대상 문장에서는 아래 표현을 우선합니다.

한 개념에는 한 가지 한국어 표현을 일관되게 씁니다. 스키마 필드, 메서드 이름, enum 값, 코드 식별자는 스키마와 코드 형태 예시에서 정확한 영어로 유지하고, 아래 한국어 표현은 산문과 렌더링 라벨에 씁니다.

| 영어 표현 | 한국어 표현 |
|---|---|
| bilingual active maintenance | 한영 문서 동시 유지 |
| authoring guide | 작성 가이드 |
| translation guide | 번역 가이드 |
| documentation checks | 문서 점검 |
| semantic parity | 의미 일치 |
| not line parity | 줄 단위 번역 아님 |
| no duplicate agent injection | 에이전트 중복 주입 금지 |
| owner document | 담당 문서 |
| current MVP | 현재 MVP |
| profile-gated value | profile-gated 값 |
| active/later boundary | active/later 경계 |
| stale content deletion rule | 오래된 내용 삭제 규칙 |
| work | 작업 |
| scope | 범위 |
| out of scope | 범위 밖 |
| judgment | 판단 |
| user-owned judgment | 사용자 소유 판단 |
| judgment request | 판단 요청 |
| evidence | 증거 |
| detailed evidence list | 증거 목록 |
| check | 확인 |
| verification | 검증 |
| Manual QA | 수동 QA |
| final acceptance | 최종 수락 |
| residual risk | 잔여 위험 |
| residual risk acceptance | 잔여 위험 수락 |
| close readiness | 닫기 가능 여부 또는 닫기 준비 상태 |
| close blocker in user-facing display | 닫기를 막는 이유 |
| `lifecycle_phase` in user-facing display | 현재 단계 |
| Autonomy Boundary in user-facing display | 에이전트가 스스로 판단해도 되는 범위 |
| `guarantee_level` in user-facing display | 하네스가 확인할 수 있는 수준 |
| Change Unit in user-facing display | 이번에 바꿀 가장 작은 작업 단위 |
| EvidenceSummary in user-facing display | 확인 근거 요약 또는 확인한 것 |
| next safe action | 다음 안전한 행동 |
| derived view or projection in user prose | 상태 보기, 요약, 상태 카드 |
| pre-write scope check | 쓰기 전 범위 확인 |
| sensitive-action approval | 민감 동작 승인 |
| verified surface context | 확인된 접점 맥락 |
| local surface registration | 로컬 접점 등록 |
| sensitive action scope | 민감 동작 범위 |
| staged artifact handle | 스테이징된 아티팩트 핸들 |
| completion policy | 완료 정책 |
| shaping readiness | 구체화 준비 상태 |
| project-wide state_version | 프로젝트 전체 `state_version` |
| task-scoped state_version | Task 범위 `state_version` |
| public conflict clock | 공개 충돌 시계 |
| artifact input | 아티팩트 입력 |
| evidence coverage item | 증거 범위 항목 |
| cooperative guarantee | 협력형 보장 |
| detective guarantee | 탐지형 보장 |
| surface identifier in user prose | 접점 식별자. 권한 증거처럼 쓰지 않음 |
| Discovery Brief as a persistent artifact | 영속 아티팩트로서의 Discovery Brief |
| Question Queue | 질문 큐 |
| Assumption Register | 가정 기록부 |
| persistent projection job | 지속 저장되는 상태 보기 작업 |
| projection reconcile | 상태 보기 조정 |
| managed block drift repair | 관리 블록 불일치 복구 |
| native artifact capture | 접점 자체 아티팩트 캡처 |
| task-scoped state clock | Task 범위 상태 시계 |
| `captured_artifact` | `captured_artifact` 값 이름. 산문에서는 이후 전용 캡처된 아티팩트 값이라고 설명 |

`Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, `task_events`는 정확한 하네스 라벨이 내부 차단 사유, 기록, API, 템플릿, 담당 문서 경로를 이해하는 데 도움이 될 때만 씁니다. 사용자용 카드와 예시에는 위의 평이한 표시 문구를 우선합니다.

## 6. 내부 식별자 용어

내부 식별자는 정확히 유지합니다. 필요하면 한국어 문장으로 뜻을 설명하되 식별자 자체를 번역하지 않습니다.

자주 나오는 예시는 아래와 같습니다.

| 식별자 | 필요할 때의 한국어 설명 |
|---|---|
| `user_judgment` | 사용자 판단 기록 |
| `UserJudgment` | 사용자 판단 스키마 |
| `judgment_kind` | 판단 종류 필드 |
| `product_decision` | 제품 판단 값 |
| `technical_decision` | 기술 판단 값 |
| `scope_decision` | 범위 판단 값 |
| `sensitive_approval` | 민감 동작 승인 값 |
| `qa_waiver` | later/reserved QA 면제 판단 값 |
| `verification_risk_acceptance` | later/reserved 검증 위험 수락 값 |
| `final_acceptance` | 최종 수락 값 |
| `residual_risk_acceptance` | 잔여 위험 수락 값 |
| `presentation` | 표시 형식 필드 |
| `display_label` | 표시 라벨 필드. 스키마 값이 아니라 렌더링 표시 문자열입니다. |
| `prepare_write` | 쓰기 전 범위 확인을 다루는 API/동작 식별자 |
| `record_run` | 실행/확인 기록 API/동작 식별자 |
| `close_task` | 닫기 확인 API/동작 식별자 |
| `ArtifactRef` | 아티팩트 참조 스키마 |
| `ArtifactInput` | 아티팩트 입력 스키마 |
| `StagedArtifactHandle` | 스테이징된 아티팩트 핸들 |
| `EvidenceCoverageItem` | 증거 범위 항목 |
| `CompletionPolicy` | 완료 정책 스키마 |
| `ShapingReadiness` | 구체화 준비 상태 파생 보기 |
| `LocalSurfaceRegistration` | 로컬 접점 등록 사실 |
| `VerifiedSurfaceContext` | 확인된 접점 맥락 |
| `SensitiveActionScope` | 민감 동작 범위 스키마 |
| `AuthorizedAttemptScope` | 제품 파일 쓰기 시도 범위 스키마 |
| `surface_id` | 접점 식별자 값. 권한, 접근, 바인딩, 역량의 증거가 아닙니다. |
| `state_version` | 상태 버전 필드명. 공개 충돌 기준은 담당 문서가 정합니다. |
| `project_state.state_version` | 프로젝트 전체 상태 시계 |
| `tasks.state_version` | Task 범위 상태 시계. 현재 MVP 공개 충돌 기준으로 쓰지 않습니다. |
| `ProjectionKind` | 상태 보기 종류 식별자 |
| `detective` | 보장 수준 값. 산문에서는 탐지형 보장이라고 설명합니다. |
| `EvidenceSummary` | 확인 근거 요약 스키마 또는 표시 개념 |

`제품 판단`, `기술 판단`, `범위 판단` 같은 한국어 라벨은 문장이나 렌더링 예시에 나타날 수 있습니다. 하지만 `product_decision`, `technical_decision`, `scope_decision`, `judgment_kind` 같은 기준 값을 대신하면 안 됩니다.

## 7. 경계 용어

active/later, 보안, 판단 경계를 더 강한 주장으로 번역하지 않습니다.

- "profile-gated"는 이름 붙은 프로필, 기능, 연결 모드, 향후 설정에서만 가능하다는 뜻입니다. 기본 현재 MVP 값이 아닙니다.
- "later candidate"는 미뤄 둔 후보 자료라는 뜻입니다. 담당 문서가 범위와 증명 기대를 함께 승격하기 전까지 활성 요구사항이 아닙니다.
- 협력형 또는 탐지형 보안 표현을 한국어에서 예방형, 격리형, 샌드박스형, 변조 방지형, 기본 도구 차단형 표현으로 바꾸지 않습니다.
- 넓은 승인, 최종 수락, 잔여 위험 수락은 서로 구분합니다. Later/reserved QA 면제 판단과 검증 위험 수락 용어는 승격 전까지 이후 후보 색인에 속하며 현재 MVP의 활성 판단 종류 목록에 포함되지 않습니다.
- `surface_id`는 식별자일 뿐 권한, 로컬 접근, 바인딩, 역량의 증거가 아닙니다. 이미 허가나 검증이 끝난 것처럼 들리는 한국어 표현으로 옮기지 않습니다.
- `captured_artifact`는 승격 전까지 이후 전용 캡처된 아티팩트 값입니다. 한국어에서 활성 아티팩트 입력 경로처럼 설명하지 않습니다.
- `sensitive_approval` / `SensitiveActionScope`는 제품 파일 쓰기의 `AuthorizedAttemptScope`와 Write Authorization과 별개입니다.
- `detective`는 대상 관찰 범위에 대한 역량 확인이 통과했을 때만 쓸 수 있습니다. 그 확인이 없으면 협력형 표현이나 역량 제한 표현을 씁니다.
- 현재 MVP의 공개 충돌 표현은 담당 문서가 다른 시계를 승격하지 않는 한 프로젝트 전체 `project_state.state_version`을 기준으로 합니다. Task 범위와 프로젝트 범위 `state_version`을 둘 다 공개 충돌 시계처럼 노출하지 않습니다.
- projection reconcile과 `reconcile`은 승격 전까지 이후 전용입니다. 번역 과정에서 Core 상태 변경 경로처럼 만들지 않습니다.
- 최종 수락과 잔여 위험 수락은 빠진 필수 증거를 대신하지 않습니다.
- 사용자 대상 템플릿은 `EvidenceSummary`, `CloseReadinessBlocker.category`, `judgment_kind`, `guarantee_level` 같은 내부 enum이나 스키마 용어를 계약 값 자체를 설명하는 경우가 아니면 노출하지 않습니다.

## 8. 이중 언어 리뷰 체크리스트

- [ ] 영어와 한국어 페이지가 같은 의미를 보존합니다.
- [ ] 대응 파일이 같은 active 파일 경로, 독자 목적, 의미상 섹션 범위, 담당 문서 경로, active/later 경계를 유지합니다.
- [ ] 경로 표가 현재 간결 구조와 `docs/doc-index.yaml`만 가리킵니다.
- [ ] 한국어 문장이 한국어 기술 독자에게 자연스럽게 읽힙니다.
- [ ] 정확한 식별자, 경로, API/스키마 이름, enum 값, 오류 코드, 테이블 이름, validator ID, 템플릿 이름, `doc_id` 값이 보존되었습니다.
- [ ] 정확한 식별자는 그대로 복사했고, 설명 문장은 자연스러운 한국어로 번역하거나 다시 썼습니다.
- [ ] 한국어 표시 라벨이 스키마 식별자가 아니라 지역화된 표시 문자열로 다뤄집니다.
- [ ] 사용자 대상 한국어는 하네스 라벨이 필요할 때도 자연스러운 한국어를 먼저 둡니다.
- [ ] 정확한 식별자나 의도적인 하네스 라벨이 아닌 영어 명사구가 한국어 문장에 그대로 남아 있지 않습니다.
- [ ] 자연스러운 한국어 설명이 필요한 자리에서 영어 어순이나 줄 단위 번역투를 보존하지 않았습니다.
- [ ] 한국어 번역이 부정 병렬 구조를 압축해 의미를 뒤집지 않았습니다.
- [ ] 담당 문서 밖 중복 계약은 전체 계약 번역이 아니라 요약과 허용된 담당 경로로 처리했습니다.
- [ ] 이후 전용 개념은 이후 전용으로 표시했고, 참조 문서가 이후 전용 기능을 현재 MVP 요구사항처럼 보이게 만들지 않았습니다.
- [ ] 사용자 대상 템플릿은 불필요한 내부 enum이나 스키마 용어 대신 자연스러운 표시 문구를 씁니다.
- [ ] 링크 변경은 양쪽 언어에 같은 작업 묶음으로 반영했습니다.
- [ ] 번역 검토를 런타임 상태, 증거, QA, 수락, 닫기 준비 상태, 런타임 적합성, 구현 준비 상태처럼 설명하지 않았습니다.
