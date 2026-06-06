# 번역 가이드

영어와 한국어 하네스 문서를 함께 고칠 때 이 번역 가이드를 사용합니다. 이 문서는 살아 있는 이중 언어 편집 규칙입니다. 재설계 이력도 아니고, 런타임 적합성이나 구현 준비 상태 기록도 아닙니다.

## 1. 의미 일치, 줄 단위 번역 아님

영어 문서는 이중 언어 문서 세트의 기준 의미를 정의합니다. 한국어 문서는 그 의미를 보존하되 자연스러운 한국어 기술 문서처럼 읽혀야 합니다.

목표는 문장을 한 줄씩 맞추는 번역이 아니라 의미 일치입니다. 한국어가 더 분명하다면 제목, 문단 나눔, 예시는 영어와 달라도 됩니다. 단, 같은 의미, 담당 문서 경로, active/later 경계, 정확한 식별자는 유지해야 합니다.

의미가 바뀌면 영어와 한국어를 같은 작업 묶음에서 고칩니다. 한국어 편집 중 영어 의미 문제가 보이면 양쪽을 함께 고칩니다.

## 2. 보존할 정확한 식별자

아래 항목은 양쪽 언어에서 정확히 보존합니다.

- 파일 경로와 anchor
- `doc_id` 값
- API 메서드 이름, 도구 이름, 리소스 이름
- schema 이름과 schema field
- enum 값과 상태 값
- error code와 validator ID
- DDL, table 이름, column 이름, storage 식별자
- code identifier, literal marker, placeholder 이름, 코드 형태 문자열

코드 블록, schema, API 예시, 파일 경로, field 목록 안의 정확한 문자열은 번역하지 않습니다. 지역화된 표시 라벨은 렌더링 text이지 기준 식별자가 아닙니다.

## 3. 자연스러운 한국어 규칙

한국어 문서는 자연스러운 한국어 기술 문장으로 씁니다.

- 짧고 분명한 문장을 선호합니다.
- 사용자 대상 문장에서는 한국어 개념을 먼저 둡니다.
- 정확도, 검색, 담당 문서 연결에 필요할 때만 정확한 영어 식별자를 붙입니다.
- 영어 명사에 한국어 조사만 붙인 문장이 대부분인 형태를 피합니다.
- 주변 문장이 완전한 한국어여도 정확한 식별자는 그대로 둡니다.

좋은 한국어 문서는 영어 문장 순서를 바꿀 수 있습니다. 줄 단위 번역처럼 읽히면 다시 씁니다.

## 4. 사용자용 용어

한국어 사용자 대상 문장에서는 아래 표현을 우선합니다.

| 영어 표현 | 한국어 표현 |
|---|---|
| authoring guide | 작성 가이드 |
| translation guide | 번역 가이드 |
| documentation checks | 문서 점검 |
| semantic parity | 의미 일치 |
| not line parity | 줄 단위 번역 아님 |
| owner document | 담당 문서 |
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
| close readiness | 닫기 가능 여부 또는 닫기 준비 상태 |
| close blocker | 닫기 차단 사유 |
| next safe action | 다음 안전한 행동 |
| derived view or projection in user prose | 상태 보기, 요약, 상태 카드 |
| pre-write scope check | 쓰기 전 범위 확인 |
| sensitive-action approval | 민감 동작 승인 |

`Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, `task_events`는 exact 하네스 label이 차단 사유, record, API, template, 담당 문서 링크를 이해하는 데 도움이 될 때만 씁니다.

## 5. 내부 식별자 용어

내부 식별자는 정확히 유지합니다. 필요하면 한국어 문장으로 뜻을 설명하되 식별자 자체를 번역하지 않습니다.

자주 나오는 예시는 아래와 같습니다.

| 식별자 | 필요할 때의 한국어 설명 |
|---|---|
| `user_judgment` | 사용자 판단 기록 |
| `UserJudgment` | 사용자 판단 schema |
| `judgment_kind` | 판단 종류 field |
| `product_decision` | 제품 판단 값 |
| `technical_decision` | 기술 판단 값 |
| `scope_decision` | 범위 판단 값 |
| `sensitive_approval` | 민감 동작 승인 값 |
| `qa_waiver` | QA 면제 판단 값 |
| `verification_risk_acceptance` | 검증 위험 수락 값 |
| `final_acceptance` | 최종 수락 값 |
| `residual_risk_acceptance` | 잔여 위험 수락 값 |
| `presentation` | 표시 형식 field |
| `display_label` | 표시 라벨 field. Schema 값이 아니라 렌더링 표시 text입니다. |
| `prepare_write` | 쓰기 전 범위 확인을 다루는 API/동작 식별자 |
| `record_run` | 실행/확인 기록 API/동작 식별자 |
| `close_task` | 닫기 확인 API/동작 식별자 |
| `ArtifactRef` | 아티팩트 참조 schema |
| `ProjectionKind` | Projection 종류 식별자 |

`제품 판단`, `기술 판단`, `범위 판단` 같은 한국어 라벨은 문장이나 렌더링 예시에 나타날 수 있습니다. 하지만 `product_decision`, `technical_decision`, `scope_decision`, `judgment_kind` 같은 기준 값을 대신하면 안 됩니다.

## 6. 이중 언어 리뷰 체크리스트

- [ ] 한국어 페이지가 영어 페이지와 같은 의미를 보존합니다.
- [ ] 대응 파일이 같은 active file path, 독자 목적, 의미상 섹션 범위, 담당 문서 링크, active/later 경계를 유지합니다.
- [ ] 한국어 문장이 한국어 기술 독자에게 자연스럽게 읽힙니다.
- [ ] 정확한 식별자, path, API/schema 이름, enum 값, error code, table 이름, validator ID가 보존되었습니다.
- [ ] 한국어 표시 라벨이 schema identifier가 아니라 지역화된 display text로 다뤄집니다.
- [ ] 사용자 대상 한국어는 하네스 label이 필요할 때도 자연스러운 한국어를 먼저 둡니다.
- [ ] 담당 문서 밖 중복 contract는 전체 contract 번역이 아니라 요약과 담당 문서 링크로 처리했습니다.
- [ ] 링크 변경은 양쪽 언어에 같은 작업 묶음으로 반영했습니다.
- [ ] 번역 review를 런타임 상태, 증거, QA, 수락, 닫기 준비 상태, 런타임 적합성, 구현 준비 상태처럼 설명하지 않았습니다.
