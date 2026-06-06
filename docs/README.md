# Harness Documentation / 하네스 문서

This directory contains bilingual design documentation for a future local Harness Server. The repository is documentation-only today, in post-redesign review, and not a running Harness instance.

이 디렉터리는 향후 로컬 하네스 서버를 위한 이중 언어 설계 문서를 담고 있습니다. 이 저장소는 현재 문서 전용이고 재설계 이후 검토 상태이며, 실행 중인 하네스 인스턴스가 아닙니다.

Harness authority means authority over Harness records and state transitions: scope, user-owned judgment, evidence, verification expectations, final acceptance, close readiness, and residual risk. It does not mean operating-system permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, or security isolation.

하네스 권한은 범위, 사용자 소유 판단, 증거, 확인과 검증 기대, 최종 수락, 닫기 가능 여부, 잔여 위험에 대한 하네스 기록과 상태 전이의 권한을 뜻합니다. 운영체제 권한 제어, 임의 도구 샌드박스, 변조 방지 저장소, 기본 도구 실행 전 차단, 보안 격리를 뜻하지 않습니다.

It is not the user's Product Repository and not a Harness Runtime Home. Documentation files are source material, not runtime state, generated projections, evidence, QA, final acceptance, residual-risk, or close records. Documentation acceptance does not by itself authorize server/runtime implementation.

사용자의 제품 저장소도, 하네스 런타임 홈도 아닙니다. 문서 파일은 원천 자료이지 런타임 상태, 생성된 읽기용 보기, 증거, QA, 최종 수락, 잔여 위험, 닫기 기록이 아닙니다. 문서 수락만으로 서버/런타임 구현이 승인되지 않습니다.

## Choose A Language / 언어 선택

| Language / 언어 | Entry point / 진입점 |
|---|---|
| English | [en/README.md](en/README.md) |
| 한국어 | [ko/README.md](ko/README.md) |

## Minimal First-Read Path / 최소 첫 읽기 경로

| Step / 순서 | English | 한국어 |
|---|---|---|
| 1 | [Start](en/start.md) | [시작하기](ko/start.md) |
| 2 | [User Guide](en/use/user-guide.md) | [사용자 가이드](ko/use/user-guide.md) |
| Implementers only / 구현자만 | [MVP Plan](en/build/mvp-plan.md) | [MVP 계획](ko/build/mvp-plan.md) |
| Lookup only / 찾아볼 때만 | [Reference Index](en/reference/README.md) | [Reference 색인](ko/reference/README.md) |

## Reader Paths / 독자별 경로

| Reader / 독자 | English path | 한국어 경로 |
|---|---|---|
| General user / 일반 사용자 | [Start](en/start.md) -> [User Guide](en/use/user-guide.md). | [시작하기](ko/start.md) -> [사용자 가이드](ko/use/user-guide.md). |
| Agent instruction writer / 에이전트 지침 작성자 | [Agent Guide](en/use/agent-guide.md); use [Agent Integration Reference](en/reference/agent-integration.md) only for exact connector/context contracts. | [에이전트 가이드](ko/use/agent-guide.md); 정확한 connector/context 계약이 필요할 때만 [Agent 통합 참조](ko/reference/agent-integration.md). |
| Future server implementer / 향후 서버 구현자 | [MVP Plan](en/build/mvp-plan.md) -> [Reference Index](en/reference/README.md). | [MVP 계획](ko/build/mvp-plan.md) -> [Reference 색인](ko/reference/README.md). |
| Documentation maintainer / 문서 유지보수자 | [Authoring Guide](en/maintain/authoring-guide.md) -> [Translation Guide](en/maintain/translation-guide.md) -> [Documentation Checks](en/maintain/documentation-checks.md). | [문서 작성 가이드](ko/maintain/authoring-guide.md) -> [번역 가이드](ko/maintain/translation-guide.md) -> [문서 점검표](ko/maintain/documentation-checks.md). |
| Later/profile reader / 이후 프로필 독자 | [Assurance Profile](en/later/assurance-profile.md), [Operations Profile](en/later/operations-profile.md), [Future Fixtures](en/later/future-fixtures.md), and [Roadmap](en/roadmap.md). | [보증 프로필](ko/later/assurance-profile.md), [운영 프로필](ko/later/operations-profile.md), [향후 Fixtures](ko/later/future-fixtures.md), [로드맵](ko/roadmap.md). |

[Documentation Checks](en/maintain/documentation-checks.md) / [문서 점검표](ko/maintain/documentation-checks.md)는 사전 구현 문서 일관성을 수동으로 검토하기 위한 보조 자료입니다. `PASS`, `WARN`, `FAIL` 라벨은 문서 수락, 구현 준비, 개발 준비, runtime conformance, server coding 시작 허가를 결정하지 않습니다.

## Layer Responsibilities / 문서층 역할

| Family / 문서군 | Purpose / 목적 | Boundary / 경계 |
|---|---|---|
| Start / 첫 읽기 | Why Harness exists, one ordinary task, first concepts, and the current guarantee boundary. / 하네스가 왜 필요한지, 평소 작업 하나, 첫 개념, 현재 보장 경계를 설명합니다. | No strict schemas, gates, DDL, implementation sequence, or fixture mechanics. / 엄격한 schema, gate, DDL, 구현 순서, fixture mechanics를 정의하지 않습니다. |
| Use / 사용 | User and agent usage in ordinary language, including judgment requests, write checks, evidence summaries, and close flow. / 평소 말로 보는 사용자와 에이전트 사용 방식. 판단 요청, 쓰기 전 확인, 증거 요약, 닫기 흐름을 다룹니다. | No canonical enum definitions, DDL, or full transition tables. / canonical enum definition, DDL, 전체 전이 표를 두지 않습니다. |
| Build / 구현 | Future implementation sequence, active slice, first proof, active/later boundary, reading path, and exclusions. / 향후 구현 순서, 활성 조각, 첫 증명, 활성/이후 경계, 읽기 경로, 제외 범위. | Exact API shapes, schemas, DDL, storage tables, fixture bodies, transition tables, security guarantees, and full threat catalogs are linked, not duplicated. / 정확한 API shape, schema, DDL, storage table, fixture body, 전이 표, 보안 보장, 전체 threat catalog는 중복하지 않고 링크합니다. |
| Reference / 참조 | Exact owner contracts: Core transition, API schema, Storage/DDL, Security, Agent Integration, Projection/Templates, Conformance, Glossary, and related owners. / Core 전이, API schema, Storage/DDL, Security, Agent Integration, Projection/Templates, Conformance, Glossary와 관련 owner의 정확한 계약. | Not a tutorial or staged implementation plan. / 튜토리얼이나 단계별 구현 계획이 아닙니다. |
| Later / 이후 | Assurance, operations, future fixture, and roadmap material outside the active MVP path. / 활성 MVP 경로 밖에 두는 보증, 운영, 향후 fixture, 로드맵 자료. | Not active delivery unless promoted by an owner. / 담당 문서가 승격하기 전까지 활성 전달 범위가 아닙니다. |
| Maintain / 유지보수 | Documentation writing, translation, review, drift, owner-boundary, and link rules. / 문서 작성, 번역, 검토, drift, owner 경계, 링크 규칙. | No runtime readiness decision or acceptance judgment. / 런타임 준비 결정이나 수락 판단을 하지 않습니다. |

## Status And Handoff / 상태와 인계

Detailed status belongs in the Build plan and Maintain owner docs:

- [English maintainer handoff summary](en/build/mvp-plan.md#maintainer-handoff-summary)
- [한국어 문서 인계 요약](ko/build/mvp-plan.md#문서-인계-요약)
- [English implementation decisions before server coding](en/build/mvp-plan.md#implementation-decisions-needed-before-server-coding)
- [한국어 서버 코딩 전 필요한 구현 결정](ko/build/mvp-plan.md#서버-코딩-전-필요한-구현-결정)
- [English Authoring Guide](en/maintain/authoring-guide.md)
- [한국어 문서 작성 가이드](ko/maintain/authoring-guide.md)
- [English manual pre-implementation consistency checklist](en/maintain/documentation-checks.md#final-pre-implementation-consistency-checklist)
- [한국어 최종 사전 구현 일관성 점검표](ko/maintain/documentation-checks.md#최종-사전-구현-일관성-점검표)

상세 상태는 Build와 Maintain 담당 문서가 관리합니다. 이 경로들은 문서 검토를 돕기 위한 것이며 런타임 conformance, 최종 수락, close readiness, implementation readiness를 만들지 않습니다.
