# 이후: 보증 프로필

이 문서는 보증 프로필의 향후 hardening 내용을 MVP 구현 경로에 섞지 않고 찾아가도록 돕습니다.

향후 하네스 동작을 위한 계획 및 길잡이 문서입니다. 상태: MVP-1 요구사항 아님, 구현된 런타임 아님. 이 저장소에서 런타임/서버 구현, 생성된 운영 파일, 실행 가능한 fixture, 런타임 데이터, 제품 코드를 허가하지 않습니다.

## 이런 때 읽기

- MVP-1 사용자 작업 루프 이후에 무엇이 속하는지 확인할 때.
- 검증 강화, 수동 QA, 상세 근거, 위험 검토, 상세 평가 출력을 MVP-1과 구분해야 할 때.
- 보증 관련 계약의 담당 문서를 찾아야 할 때.

## 버킷 경계

보증 프로필은 MVP-1 이후 범위입니다. 사용자 가치 루프의 보증 동작을 더 단단하게 만들 수 있지만, 첫 사용자 가치 경로가 아니며 운영/export/recover 프로필도 아닙니다.

| 보증 버킷 | 여기에 속하는 것 | 승격 전까지 밖에 둘 것 |
|---|---|---|
| 검증 강화 | 분리 검증 정책, 독립성 표시, 검증 면제 라우팅, 검증 공백, owner 기록으로 뒷받침되는 더 강한 보증 주장. | 여러 접점 검증 자동화와 evaluator orchestration은 승격 전까지 로드맵 향후 후보입니다. |
| 수동 QA | 전체 수동 QA 기대치, QA 면제 상세, QA 근거 참조, QA의 닫기 영향. | Browser QA Capture 자동화와 QA dashboard는 승격 전까지 로드맵 향후 후보입니다. |
| 상세 근거 | 상세 Evidence Manifest 동작, 아티팩트 참조, 근거 충분성 상세, redaction/omission 표시, 근거 공백. | 전체 export bundle과 release handoff packaging은 운영 프로필에 속합니다. |
| 위험 검토 | 풍부한 잔여 위험 lifecycle, 작업 수락 또는 닫기 전 표시, 잔여 위험 수용 라우팅, 위험 검토 요약. | Team risk workflow, policy dashboard, hosted review flow는 로드맵 향후 후보입니다. |
| 상세 평가 출력 | Eval result 상세, Verification Result Card 표시, 상세 `EVAL` projection 출력, Eval owner path가 active일 때의 assurance-level 설명. | Eval을 orchestration으로 취급하는 metrics product, analytics, automation은 로드맵 향후 후보입니다. |

Design-quality, stewardship, TDD trace, feedback-loop, context-hygiene 내용은 위 보증 버킷 중 하나를 지원할 때만 여기에 속합니다. Dashboard, hosted workflow, team workflow, broader connector, orchestration, preventive security, isolation은 owner가 구체적인 mechanism을 승격하고 증명하기 전까지 [로드맵](../roadmap.md) 향후 후보로 남습니다.

## 읽는 경로

먼저 [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md)에서 MVP 경계를 확인합니다. 그다음 필요한 질문의 담당 문서만 엽니다.

| 필요한 것 | 담당 문서 |
|---|---|
| Core gate, 사용자 판단, 닫기, waiver, 작업 수락, 잔여 위험 의미 | [Core Model 참조](../reference/core-model.md) |
| 이후/profile-gated API method와 schema material | [API Schema Later](../reference/api/schema-later.md) |
| 설계 품질 정책, validator ID, severity composition, waiver 영향 | [설계 품질 정책](../reference/design-quality-policies.md) |
| Fixture mechanics와 profile 증명 모델 | [Conformance Fixtures 참조](../reference/conformance-fixtures.md) |
| 향후 보증 scenario 후보 | [향후 Fixtures](future-fixtures.md) |
| 보증 report의 읽기용 표시 경계 | [Projection과 Template 참조](../reference/projection-and-templates.md)와 [Template 참조](../reference/templates/README.md) |

## 경계

보증 프로필은 report text만으로 권한을 만들지 않습니다. 검증, 수동 QA, 상세 근거, 위험 검토, 상세 Eval 표시는 각각 별도의 owner 기록, 참조, 파생 보기입니다. 어느 것도 작업 수락, 잔여 위험 수용, 닫기 준비 상태, Core state를 대신하지 않습니다.

여기에 이름이 있다고 해서 MVP-1 요구사항이 되거나, 구현된 런타임 동작이 되거나, 실행 가능한 conformance가 되지는 않습니다. 향후 fixture row는 owner가 정확한 동작을 승격하고 exact-shape fixture를 materialize하기 전까지 [향후 Fixtures](future-fixtures.md)에 남습니다.
