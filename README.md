# Harness Project / 하네스 프로젝트

Harness is a future local work-authority server for AI-assisted product work. Here, work authority means authority over Harness records and state transitions: scope, user-owned judgment, evidence, verification expectations, final acceptance, close readiness, and residual risk. It is not operating-system permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, or security isolation.

하네스는 AI 지원 제품 작업을 위한 향후 로컬 작업 권한 서버입니다. 여기서 작업 권한은 범위, 사용자 소유 판단, 증거, 확인과 검증 기대, 최종 수락, 닫기 가능 여부, 잔여 위험에 대한 하네스 기록과 상태 전이의 권한을 뜻합니다. 운영체제 권한 제어, 임의 도구 샌드박스, 변조 방지 저장소, 기본 도구 실행 전 차단, 보안 격리를 뜻하지 않습니다.

## Repository State / 저장소 상태

This repository is documentation-only today and is in post-redesign review. It contains source documentation for a future local Harness Server. It does not contain a Harness Server implementation.

이 저장소는 현재 문서 전용이며 재설계 이후 검토 상태입니다. 향후 로컬 하네스 서버를 위한 원천 문서를 담고 있을 뿐, 하네스 서버 구현을 담고 있지 않습니다.

It is not the user's Product Repository. It is not a Harness Runtime Home. No runtime state, generated projections, generated operational artifacts, executable fixtures, conformance runner, product implementation code, or server code exists here.

이 저장소는 사용자의 제품 저장소가 아닙니다. 하네스 런타임 홈도 아닙니다. 런타임 상태, 생성된 읽기용 보기, 생성된 운영 산출물, 실행 가능한 fixture, conformance runner, 제품 구현 코드, 서버 코드는 없습니다.

Documentation acceptance, when it happens, is only a maintainer documentation milestone. Server/runtime implementation still requires a separate implementation-planning readiness decision. Documentation files are source material, not Harness runtime state, evidence, QA, acceptance, residual-risk, projection, or close records.

문서가 수락되더라도 그것은 유지보수자의 문서 검토 이정표일 뿐입니다. 서버/런타임 구현에는 별도의 구현 계획 준비 결정이 필요합니다. 문서 파일은 원천 자료이지 하네스 런타임 상태, 증거, QA, 수락, 잔여 위험, 읽기용 보기, 닫기 기록이 아닙니다.

## Start Here / 시작하기

Use [docs/README.md](docs/README.md) for the compact bilingual route.

간결한 이중 언어 경로는 [docs/README.md](docs/README.md)에서 시작합니다.

| Need / 필요 | Start / 시작 |
|---|---|
| Choose a language / 언어 선택 | [docs/README.md](docs/README.md) |
| First-time reader / 처음 읽는 독자 | [English Start](docs/en/start.md) / [한국어 시작하기](docs/ko/start.md) |
| User working with an agent / 에이전트와 작업하는 사용자 | [English User Guide](docs/en/use/user-guide.md) / [한국어 사용자 가이드](docs/ko/use/user-guide.md) |
| Future server implementer / 향후 서버 구현자 | [English Implementation Overview](docs/en/build/implementation-overview.md) / [한국어 구현 개요](docs/ko/build/implementation-overview.md) |
| Exact contract lookup / 정확한 계약 확인 | [English Reference Index](docs/en/reference/README.md) / [한국어 Reference 색인](docs/ko/reference/README.md) |
| Documentation maintainer / 문서 유지보수자 | [English Authoring Guide](docs/en/maintain/authoring-guide.md) / [한국어 문서 작성 가이드](docs/ko/maintain/authoring-guide.md) |
| Manual pre-implementation docs consistency review / 사전 구현 문서 일관성 수동 검토 | [English Documentation Checks](docs/en/maintain/documentation-checks.md) / [한국어 문서 점검표](docs/ko/maintain/documentation-checks.md) |

Documentation Checks is a manual review aid. Its `PASS`, `WARN`, and `FAIL` labels do not decide documentation acceptance, implementation readiness, development readiness, runtime conformance, or permission to start server coding.

문서 점검표는 수동 검토 보조 자료입니다. `PASS`, `WARN`, `FAIL` 라벨은 문서 수락, 구현 준비, 개발 준비, runtime conformance, server coding 시작 허가를 결정하지 않습니다.

## Layer Responsibilities / 문서층 역할

| Layer / 층 | Responsibility / 역할 |
|---|---|
| README | Repository identity and reading paths. It does not claim implementation readiness or treat docs as runtime state. / 저장소 정체성과 읽기 경로를 안내합니다. 구현 준비 상태를 주장하거나 문서를 런타임 상태로 다루지 않습니다. |
| Build | Future implementation sequence, active slice, first proof, active/later boundary, and exclusions. Exact API shapes, schemas, DDL, storage tables, transition tables, fixture bodies, and threat/security contracts stay in Reference. / 향후 구현 순서, 활성 조각, 첫 증명, 활성/이후 경계, 제외 범위를 설명합니다. 정확한 API shape, schema, DDL, storage table, 전이 표, fixture body, threat/security contract는 Reference에 남습니다. |
| Use | User and agent behavior in ordinary language: examples, judgment requests, write checks, evidence summaries, and close flow. It does not define canonical enums, DDL, or full transition tables. / 평소 말로 보는 사용자와 에이전트 동작을 설명합니다. 예시, 판단 요청, 쓰기 전 확인, 증거 요약, 닫기 흐름을 다루며 canonical enum, DDL, 전체 전이 표는 정의하지 않습니다. |
| Reference | Exact contracts: Core transitions, API schemas, Storage/DDL, Security, Agent Integration, Projection/Templates, Conformance, Glossary, and related owner contracts. / Core 전이, API schema, Storage/DDL, Security, Agent Integration, Projection/Templates, Conformance, Glossary와 관련 owner 계약을 정확히 둡니다. |
| Maintain | Documentation authoring, translation, review, drift, and owner-boundary rules. It does not decide runtime readiness or acceptance. / 문서 작성, 번역, 검토, drift, owner 경계를 관리합니다. 런타임 준비나 수락 판단을 결정하지 않습니다. |

## Status Owners / 상태 담당 문서

Current handoff status lives in [Implementation Overview](docs/en/build/implementation-overview.md#maintainer-handoff-summary) / [구현 개요](docs/ko/build/implementation-overview.md#문서-인계-요약). Server-coding decisions live in [MVP-1 User Work Loop](docs/en/build/mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding) / [MVP-1 사용자 작업 루프](docs/ko/build/mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정).

현재 인계 상태는 [Implementation Overview](docs/en/build/implementation-overview.md#maintainer-handoff-summary) / [구현 개요](docs/ko/build/implementation-overview.md#문서-인계-요약)가 담당합니다. 서버 코딩 전 결정은 [MVP-1 User Work Loop](docs/en/build/mvp-user-work-loop.md#implementation-decisions-needed-before-server-coding) / [MVP-1 사용자 작업 루프](docs/ko/build/mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정)에 둡니다.
