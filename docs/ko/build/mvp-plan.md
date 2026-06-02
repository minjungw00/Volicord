# Build: MVP 계획

## 이 문서로 할 수 있는 일

이 문서는 MVP 범위를 구현 가능한 staged delivery 계획으로 바꿉니다. 코어 권한 조각(v0.1 Core Authority Slice)과 첫 사용자 대상 MVP를 분리해, "MVP"라는 이름을 단순히 권한 루프가 존재하는 단계가 아니라 사용자가 하네스의 가치를 경험할 수 있는 단계에만 사용합니다.

이 문서는 구현 계획 문서입니다. 문서 수락과 별도의 구현 계획 준비 결정 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture, fixture 파일, 런타임 데이터를 만들라는 뜻이 아닙니다. Conformance fixture 문서는 향후 적합성 검증 계획이며, 현재 문서 전용 저장소에는 runnable Harness Server conformance test가 없습니다. 첫 실행 목표는 코어 권한 조각(v0.1 Core Authority Slice)이며, 커널 스모크(Kernel Smoke)는 좁은 future smoke-check 작성 label입니다. 첫 제품 MVP 목표는 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)입니다. 에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)은 agency assurance, operations, handoff 동작을 단계적으로 단단하게 만듭니다. v1+ Expansion은 담당 문서가 승격하고 증명하기 전까지 로드맵 범위에 둡니다.

문서 수락과 별도의 구현 계획 준비 결정 이후 무엇을 만들지 계획할 때 이 문서를 사용합니다. 정확한 contract는 Reference 문서를 사용합니다.

## 읽는 경우

- 첫 내부 권한 증명과 첫 사용자 대상 제품 MVP를 분리해야 할 때.
- 첫 implementation batch를 키우지 않으면서 단계별 전달 범위를 검토해야 할 때.
- 구현 순서를 storage, schema, fixture, template detail과 분리해서 보고 싶을 때.

## 먼저 읽을 것

[구현 개요](implementation-overview.md)와 그 [문서 수락 상태](implementation-overview.md#문서-수락-상태)를 먼저 읽은 뒤 이 단계 계획을 사용합니다. v0.1 구현 순서는 [첫 실행 가능한 조각](first-runnable-slice.md)을, request-to-close runtime path는 [Runtime Walkthrough](runtime-walkthrough.md)를 사용합니다.

정확한 계약은 [Reference 색인](../reference/README.md)에서 지금 필요한 질문의 owner를 골라 확인합니다. v1+ Expansion 후보와 승격 기준은 [로드맵](../roadmap.md)을 사용합니다.

## 핵심 생각

하네스의 가치는 단지 write authority loop가 있다는 데 있지 않습니다. 하네스는 범위, 사용자 소유 판단, 근거, 닫기 준비 상태, 잔여 위험을 로컬 권한 기록에 보존해야 합니다. 그래서 초기 전달에는 두 단계가 있습니다.

- 코어 권한 조각(v0.1 Core Authority Slice)은 가장 작은 내부 Core 권한 루프를 증명합니다.
- 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)는 평범한 요청에서 사용자가 하네스가 작업을 구체화하고, 적절한 절차 규모로 다루고, 필요한 경우 보이는 막힘으로 보류하고, 수락하고, 위험을 설명하는 핵심 가치를 처음 체감하는 제품 MVP입니다.

첫 조각은 의도적으로 좁게 유지합니다. 로컬 프로젝트 등록 하나, Task 하나, 범위가 정해진 작업 경계 하나, `prepare_write` 권한 경로 하나, 한 번만 쓰는 Write Authorization 하나, 기록된 Run 하나, artifact/evidence 참조 하나, 구조화된 막힘/상태 응답 하나를 증명합니다. 이것은 MVP가 아닙니다. 일반적인 작업을 범위, 사용자 소유 판단, 근거, 닫기 준비 상태, 잔여 위험 언어로 바꾸고 민감 동작 승인(Approval), 작업 수락, 잔여 위험 수용을 혼동하지 않게 만드는 단계가 MVP입니다.

읽기용 요약(Projection) 템플릿 다듬기, 상세 보고서, dashboard 또는 hosted workflow UI, index, broad connector ecosystem 또는 marketplace, team workflow, surface-specific connector automation, metric, parallel orchestration, broad automation은 authority record와 user-facing value path가 존재한 뒤 유용해질 수 있습니다. 첫 조각의 요구사항은 아닙니다.

초기 output model은 의도적으로 작게 둡니다.

- v0.1은 Core state에서 오는 최소 상태/막힘 출력만 필요합니다. 전체 읽기용 요약 renderer는 필요하지 않습니다.
- v0.2의 필수 읽기용 요약은 현재 작업 상태, 사용자 판단 요청, 근거 요약, 닫기 준비 상태 / blocker 요약으로 최소화합니다. 작업 수락과 잔여 위험 사실은 관련 있을 때 distinct하게 남지만, 별도 필수 projection kind가 아니라 그 요약 안에 나타납니다.
- Journey Card, Journey Spine, Run Summary, TDD Trace, Module Map, Interface Contract, Export, detailed Evidence Manifest, detailed Eval output은 담당 profile이 명시적으로 승격하지 않는 한 Future/diagnostic projections 또는 다른 later-profile scope로 남습니다.

## 단계별 전달 계획

| 단계 | 전달 목표 | 증명하는 것 | 아직 증명하지 않는 것 |
|---|---|---|---|
| v0.1 | 코어 권한 조각(v0.1 Core Authority Slice) | 로컬 프로젝트 등록 하나, Task 하나, 범위가 정해진 작업 경계 하나, `prepare_write` 권한 경로 하나, 한 번만 쓰는 Write Authorization 하나, 기록된 Run 하나, artifact/evidence 참조 하나, 구조화된 막힘/상태 응답 하나로 구성된 첫 실행 가능한 내부 Core 권한 루프. | 사용자 대상 MVP 가치, full intake/discovery, profile별 Decision Packet 품질, full Evidence Manifest, 수동 QA, 분리 검증, 잔여 위험 수용 의미, 작업 수락 의미, 여러 projection kind, recover/export, 넓은 operator entrypoint, full conformance suite, future fixture catalog, dashboard/UI behavior. |
| v0.2 | 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP) | 사용자가 하네스가 범위, 사용자 소유 판단, 근거, 닫기 준비 상태, 작업 수락, 잔여 위험 표시를 로컬 권한 기록에 보존한다는 것을 경험합니다. | Full agency assurance hardening, 분리 검증 독립성, 수동 QA matrix, stewardship policy suite, feedback-loop policy, export/recover, release handoff. |
| v0.3 | 에이전시 보증 팩(v0.3 Agency Assurance Pack) | MVP path를 검증, 수동 QA, 잔여 위험, 작업 수락, stewardship profile로 단단하게 만듭니다. | Operator recovery/export completeness, release handoff, broad operations coverage, roadmap automation. |
| v0.4 | 운영과 인계 팩(v0.4 Operations & Handoff Pack) | 같은 Core model로 doctor/readiness, recover/export, artifact integrity, release handoff, 더 넓은 conformance coverage를 지원합니다. | Dashboard, hosted workflow UI, broad connectors, Browser QA Capture automation, Cross-Surface Verification automation, Context Index, team workflow, orchestration. |

```mermaid
flowchart LR
  Core["v0.1 Core Authority Slice"] --> MVP["v0.2 User-Facing Harness MVP"]
  MVP --> Assurance["v0.3 Agency Assurance Pack"]
  Assurance --> Ops["v0.4 Operations & Handoff Pack"]
  Ops -. roadmap .-> Expansion["v1+ Expansion"]
```

커널 스모크(Kernel Smoke)는 코어 권한 조각(v0.1 Core Authority Slice)을 위한 좁은 향후 작성 label로 남습니다. 이 label은 v0.1이 제품 MVP라는 뜻이 아니며, full conformance suite나 future fixture catalog가 있어야 내부 Core 권한 경로를 확인할 수 있다는 뜻도 아닙니다.

Conformance fixture 검증 프로파일은 같은 stage name을 따릅니다. Core Authority Slice fixture 프로파일은 코어 권한 조각(v0.1 Core Authority Slice)에, User-Facing Harness MVP fixture 프로파일은 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)에, Agency Assurance Pack fixture 프로파일은 에이전시 보증 팩(v0.3 Agency Assurance Pack)에 대응합니다. Operations & Handoff Pack fixture 프로파일 또는 승격된 v1+ Expansion 후보 fixture 프로파일은 운영과 인계 팩(v0.4 Operations & Handoff Pack)과 승격된 v1+ Expansion 후보에 대응합니다.

이 fixture profile 이름들이 conformance label로 남습니다. 강화된 로컬 기준 목표(hardened local reference target)는 에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)을 거쳐 도달하는 종합 목표일 뿐, profile name이나 별도 delivery stage가 아닙니다.

### 보안 guarantee 단계 구분

Build staging 자체가 security guarantee를 올려 주지는 않습니다. Security wording은 [보안 위협 모델의 단계별 guarantee level](../reference/security-threat-model.md#단계별-guarantee-level)을 따릅니다.

| 단계 | 계획할 guarantee posture |
|---|---|
| v0.1 Core Authority Slice | 지시 기반/협력적 behavior에 제한된 탐지 가능 behavior가 더해진 수준입니다. Core는 invalid state change를 거부하고 구조화된 막힘을 반환할 수 있지만, reference path가 기본으로 arbitrary local process를 멈추거나 tool을 격리하지는 않습니다. |
| v0.2 User-Facing Harness MVP | 사용자에게 보이는 막힘, MCP availability, evidence gap, close readiness, 정직한 보장 표시를 갖춘 cooperative/detective behavior입니다. |
| v0.3 Agency Assurance Pack | Verification, 수동 QA, residual risk, 작업 수락, Approval, stewardship 주변의 더 강한 분리와 탐지 가능 assurance입니다. |
| v0.4 Operations & Handoff Pack | Doctor/readiness, recover/export, artifact integrity, projection freshness, release handoff 주변의 탐지 가능 operations입니다. |
| v1+ Expansion | Owner docs가 exact covered operation 또는 real isolation boundary를 구현하고 증명한 뒤의 preventive 또는 isolated candidate만 포함합니다. |

### Stage별 API surface

MCP API reference는 문서화한 모든 method의 정확한 schema를 정의합니다. Staged delivery는 method/profile이 언제 active인지 결정합니다. Later-profile field는 해당 profile에서 exact하게 남지만 더 이른 stage exit에 들어가지 않습니다.

| Stage | Active API surface | Stage exit에 넣지 않을 later-profile fields |
|---|---|---|
| v0.1 Core Authority Slice | Minimal `harness.status` status/blocker read, `harness.prepare_write`, `harness.record_run`, owner-valid Task/scope setup path 하나, optional minimal `harness.next` 또는 narrow `harness.close_task` blocker smoke. | Full natural-language intake, Decision Packet storage, Evidence Manifest, Manual QA, Eval, 작업 수락 의미, 잔여 위험 수용, projection rendering, reconcile, export/recover, broad operations. |
| v0.2 User-Facing Harness MVP | `harness.status.next_actions`와 optional `harness.next`, user-facing `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.record_run`을 통한 evidence summaries, `harness.close_task`를 통한 close readiness/blockers. | Detached verification independence, full Manual QA matrix, Approval hardening, full residual-risk accepted close, stewardship validators, export/recover, broad operations. |
| v0.3 Agency Assurance Pack | `harness.launch_verify`, `harness.record_eval`, `harness.record_manual_qa`, judgment method의 assurance/waiver/approval/risk profiles, `harness.record_run`의 evidence/feedback/TDD profiles, ValidatorResult-emitting assurance paths. | Operator recover/export completeness, broad projection/reconcile operations, release handoff. |
| v0.4 Operations & Handoff Pack | API response의 projection freshness, reconcile judgment profile, Operations가 담당하는 operator readiness/recover/export/artifact-integrity/conformance surfaces. | Dashboard, hosted workflow UI, broad connectors, automation, team workflow, orchestration은 later promotion 전까지 제외합니다. |

### Stage별 read-only MCP resources

MCP resource는 읽기 전용이며 public tool과 같은 staged delivery boundary를 따릅니다. Resource를 읽는 행위는 Task record, decision, projection job, reconcile item을 만들거나 상태 변경을 일으키면 안 됩니다.

| Stage | Stage 범위의 resource | Stage exit에 넣지 않을 것 |
|---|---|---|
| v0.1 Core Authority Slice | Current state, blocker, write authority, 최소 Run/artifact/evidence ref를 위한 `harness://project/current`, `harness://task/active`, `harness://task/{task_id}`, optional `harness://task/{task_id}/summary` / `harness://status/card`. | Journey, Spine, Decision Packet storage, Evidence Manifest, bundle, report, design/domain map, module map, interface contract, projection job, full projection rendering. |
| v0.2 User-Facing Harness MVP | v0.1 resource에 더해 사용자 판단 표시를 위한 `harness://task/{task_id}/decision-packets`와 `harness://task/{task_id}/judgment-context`. Evidence summary, close readiness, 작업 수락 상태, 잔여 위험 표시는 status/card 또는 task summary output 안에 나타날 수 있습니다. | Detailed Evidence Manifest resource, detached verification/QA resource, report, bundle, Journey/Spine polish, design map, module map, interface contract, export/recover. |
| v0.3 Agency Assurance Pack | Evidence/assurance support가 켜졌을 때 `harness://policy/sensitive-categories`, `harness://task/{task_id}/evidence-manifest` 같은 profile-gated assurance read. | Operator report/export completeness와 넓은 operations resource. |
| v0.4 Operations & Handoff Pack | Connector freshness, report, export, recover, handoff profile이 범위에 있을 때 broad `harness://project/surfaces`, `harness://task/{task_id}/reports/latest`, `harness://task/{task_id}/bundle/current` 같은 operations read. | Dashboard, hosted workflow UI, broad connector automation, later promotion 전 roadmap resource. |
| Future/diagnostic | Owner가 승격한 `harness://task/{task_id}/spine`, `harness://task/{task_id}/journey`, `harness://task/{task_id}/change-unit-dag`, `harness://design/domain-language`, `harness://design/module-map`, `harness://design/interface-contracts` 같은 read. | Diagnostic resource를 v0.1 또는 minimum v0.2 요구사항처럼 취급하는 것. |

### 단계별 운영자 surface

Operator command는 예시적인 구현 선택지입니다. Stage boundary는 최종 command spelling이 아니라 동작입니다.

| Stage | 범위에 들어오는 운영자 동작 | Stage 밖에 남기는 운영자 동작 |
|---|---|---|
| v0.1 Core Authority Slice | 최소 local connect/register, 기본 상태 또는 진단 읽기, 첫 조각이 그 boundary를 요구할 때만 local API/MCP exposure. | Projection refresh, reconcile, recover, export, artifacts check, full conformance run, release handoff, broad doctor/readiness. |
| v0.2 User-Facing Harness MVP | 같은 최소 surface에 더해 현재 작업, 사용자 판단, 근거 상태, close blocker를 위한 user-facing status/next diagnostic입니다. 작업 수락과 잔여 위험 사실은 관련 있을 때 그 안에 나타납니다. | Assurance operations, recover/export, release handoff, broad projection/reconcile operations, full conformance run, broad operations coverage. |
| v0.3 Agency Assurance Pack | Owner path를 통한 verification, Manual QA, residual-risk, 작업 수락, stewardship, context-hygiene behavior의 assurance-profile support. | Operator recover/export completeness, release handoff, broad projection/reconcile operations, full operations conformance. |
| v0.4 Operations & Handoff Pack | Full local operations support입니다. Doctor/readiness, projection refresh, reconcile, recover, export, artifacts check, 담당 문서가 정의한 release handoff, runtime suite가 materialized된 뒤 conformance run을 포함합니다. | Remote/shared operations, dashboard, hosted workflow UI, broad connector automation, team workflow, orchestration은 later promotion 전까지 제외합니다. |
| v1+ Expansion | Owner docs가 exact contract, guarantee level, fixture, fallback behavior를 정의한 뒤 승격한 roadmap operations만 포함합니다. | 승격되지 않은 roadmap candidate는 staged delivery 밖에 남습니다. |

### 단계별 전달 이후의 경계: v1+ Expansion

v1+ Expansion은 로드맵 범위이며 Build가 소유하는 staged delivery phase가 아닙니다. Dashboard, hosted workflow UI, Browser QA Capture automation, Cross-Surface Verification automation, Context Index, broader connectors, metrics, team workflow, orchestration 같은 후보는 담당 문서가 future item을 명시적으로 승격하고 증명하기 전까지 v0.1부터 v0.4 밖에 둡니다.

## 코어 권한 조각(v0.1 Core Authority Slice)

v0.1은 구현자 확신을 위한 내부 구현 조각입니다. 하네스가 chat memory나 generated Markdown이 아니라 로컬 권한 기록임을 보여 주는 가장 작은 coherent loop만 증명해야 합니다. 사용자 가치 검증 단계가 아니며 첫 제품 MVP라고 부르면 안 됩니다.

v0.1은 다음을 증명해야 합니다.

- local project registration 하나
- Core가 소유한 상태 안의 Task 하나
- intended change를 위한 범위가 정해진 작업 경계 하나. Reference 계약상 필요한 경우에만 Change Unit 소유자 형태로 표현된다.
- `prepare_write` allow/structured-blocker path 하나
- 지속적이며 한 번만 쓸 수 있는 Write Authorization 하나
- 그 authorization을 consume하는 `record_run` 하나
- Core/API contract가 소유하는 registered `ArtifactRef` 또는 equivalent evidence reference 하나
- missing scope, missing write authority, 또는 missing artifact/evidence support를 위한 구조화된 막힘/상태 응답 하나

이에 맞는 storage profile은 [Storage와 DDL: Core Authority Slice schema](../reference/storage-and-ddl.md#core-authority-slice-schema)입니다. 이 profile이 v0.1 minimum입니다. User-facing Decision Packet table, Approval record, Evidence Manifest, Manual QA, Eval, residual-risk acceptance record, projection job, reconcile item, validator run, Journey record, diagnostic/stewardship table은 profile owner가 명시적으로 승격하기 전까지 later-profile storage로 남습니다.

v0.1은 full natural-language intake, full Discovery, profile별 Decision Packet 품질, full Evidence Manifest, 수동 QA, 분리 검증, 잔여 위험 수용 의미, 작업 수락 의미, product/UX judgment와 architecture judgment의 presentation, stewardship, feedback-loop policy, 여러 projection kind, full projection rendering, export/recover, 넓은 operator entrypoint, full conformance suite, future fixture catalog, full dashboard/UI behavior, release handoff를 증명하면 안 됩니다. 이것들은 이후 단계 또는 roadmap 범위입니다.

v0.1 Kernel Smoke candidate는 Core state, 그 루프에 필요한 owner record, artifact/evidence refs, structured blocker를 통해 minimal authority loop만 확인해야 합니다. 읽기용 요약 다듬기, detailed template, renderer output, 넓은 fixture catalog는 first-slice conformance truth가 아닙니다.

이 시점에 implementer는 Core가 최소 상태를 소유하고, scoped write가 허용되거나 구조화된 막힘으로 거부되며, authorization 하나가 한 번 소비되고, artifact/evidence ref가 기록된 Run에 연결되며, 상태/막힘 출력이 구조화된 막힘을 반환할 수 있음을 관찰할 수 있습니다. 이것은 구현자 확신이지 사용자가 Harness 가치를 경험했다는 증명이 아닙니다.

### 계약 필드 단계 구분

Reference schema에는 관련 capability가 범위에 들어올 때만 필요한 field도 포함됩니다. Build는 field requiredness를 다시 정의하지 않습니다. 어떤 capability가 어느 stage에 들어오는지만 말합니다. Field는 owner contract와 active stage를 함께 보고 읽습니다.

| Stage | Build 읽기 규칙 | 적용할 owner contract |
|---|---|---|
| v0.1 Core Authority Slice | 좁은 authority loop와 [Core Authority Slice schema](../reference/storage-and-ddl.md#core-authority-slice-schema)를 증명하는 데 필요한 owner-defined field만 사용합니다. 넓은 checklist를 만족하려고 future-profile record를 만들지 않습니다. Minimal seeded blocker가 owner ref를 사용한다면, profile별 user-facing Decision Packet 품질이 아니라 그 owner path의 valid shape만 적용합니다. | [커널 참조](../reference/kernel.md), [MCP API와 스키마](../reference/mcp-api-and-schemas.md), [Storage와 DDL](../reference/storage-and-ddl.md), [Conformance Fixtures 참조](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue). |
| v0.2 User-Facing Harness MVP | 사용자가 대기 중인 사용자 판단 맥락, 근거, 닫기 막힘을 이해하는 데 필요한 field와 display summary를 추가합니다. 작업 수락과 잔여 위험 사실은 관련 있을 때 distinct하게 남지만 최소 요약 안에 들어갑니다. | [MCP API와 스키마](../reference/mcp-api-and-schemas.md), [커널 참조](../reference/kernel.md), [읽기용 요약(Projection) 참조](../reference/document-projection.md), [Template 참조](../reference/templates/README.md). |
| 에이전시 보증 팩(v0.3 Agency Assurance Pack) / 운영과 인계 팩(v0.4 Operations & Handoff Pack) | Verification, QA, 잔여 위험, 작업 수락, stewardship, projection/reconcile, operations, export/recover, artifact-integrity, release-handoff profile은 담당 문서가 정의한 곳에서만 추가합니다. | [설계 품질 정책](../reference/design-quality-policies.md), [운영과 Conformance](../reference/operations-and-conformance.md), [Conformance Fixtures 참조](../reference/conformance-fixtures.md), [향후 Fixture Catalog](../reference/future-fixture-catalog.md), [Storage와 DDL](../reference/storage-and-ddl.md). |

따라서 API schema에서 required라는 말은 해당 tool call, record, profile이 구현되거나 사용될 때 required라는 뜻입니다. 그 자체로 future-profile field가 가장 작은 runnable slice의 일부가 되지는 않습니다.

### 서버 코딩 전 필요한 구현 결정

이 섹션은 maintainer review나 첫 runtime batch planning에서 발견되는 구현 시작 전 결정 기록의 단일 위치입니다. 큰 구현 선택을 흩어진 `TODO_DECISION`이나 막연한 follow-up으로 남기지 않습니다.

| 결정 기록 항목 | 현재 상태 | 결정 조건 |
|---|---|---|
| 확인된 server-coding decision-log 항목 | 현재 기준에서는 기록된 항목이 없습니다. 서버/런타임 구현 결정은 코드 작성용으로 공식 수락되지 않았으며, 이것은 남은 결정이 없다는 증명이 아닙니다. | Maintainer review나 첫 runtime batch planning에서 schema/design decision, stage boundary decision, 그 밖의 server-coding decision이 발견되면 server code나 DDL을 바꾸기 전에 이곳에 추가합니다. |
| 구현 준비 판단 | 수락되지 않았습니다. | Maintainer가 구현 준비 조건이 충족되었거나 남은 blocker가 재분류되었다고 판단한 뒤 [구현 개요: 문서 수락 상태](implementation-overview.md#문서-수락-상태)를 의도적으로 갱신해야 합니다. |
| 문서 drift | 기본적으로 server-coding decision이 아닙니다. | Docs-maintenance finding이 실제 owner-contract decision이나 stage blocker를 드러내면 stage impact와 함께 이 기록으로 승격합니다. 그렇지 않으면 문서 작성 가이드 tracker로 routing합니다. |

확인된 결정이 추가되면 다음을 기록합니다.

- 담당 문서 또는 담당 section
- 영향을 받는 behavior, field, table, fixture semantics, guarantee level, stage boundary
- 영향을 받는 stage
- 검토한 option
- server code나 DDL 변경 전에 필요한 결정
- 이 항목이 문서 수락, 구현 계획, 서버 코딩, 또는 이후 stage만 막는지

### 코어 권한 조각 흐름

```mermaid
flowchart LR
  Register["프로젝트 등록"] --> Task["Task"]
  Task --> Scope["범위"]
  Scope --> Check["쓰기 확인"]
  Check -->|허용| Authorization["쓰기 권한"]
  Authorization --> Run["Run 기록"]
  Run --> Evidence["ArtifactRef"]
  Check -->|허용 안 됨| Blocker["구조화된 막힘"]
  Evidence --> Status["상태 / 다음 행동<br/>또는 막힘"]
  Blocker --> Status
```

정확한 state와 blocker behavior는 [커널 참조](../reference/kernel.md)가, public tool shape는 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)가, future fixture semantics는 [Conformance Fixtures 참조](../reference/conformance-fixtures.md#conformance-fixture-format)가 담당합니다. 이 흐름은 pack gate, projection renderer requirement, fixture body requirement를 추가하지 않습니다.

향후 smoke 작성 순서는 [커널 스모크(Kernel Smoke) Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue)를 사용합니다. 이 queue는 candidate check를 이 내부 조각에 매핑하지만 executable fixture file이 이미 존재하거나 v0.1에 full conformance suite가 필요하다고 암시하지 않습니다.

## 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)

v0.2는 사용자가 하네스의 핵심 가치를 처음 체감하는 제품 MVP입니다. 더 긴 구성 요소 점검 목록이 아니라 사용자가 경험하는 가치로 정의합니다.

MVP는 다음을 보여야 합니다.

- 평범한 사용자 요청이 범위, 사용자 소유 판단, 근거, 닫기 준비 상태 언어로 정리된다
- product/UX judgment와 기술 구조 판단이 서로 분리되고, 민감 동작 승인, 작업 수락, 잔여 위험 수용과도 분리되어 제시될 수 있다
- 작은 변경과 tracked work가 서로 다른 procedural budget을 가지되, small-change label이 authority를 우회하지 않는다
- status와 next-action output이 현재 scope, 누락된 judgments, 근거 상태, close blockers, 안전한 다음 행동을 설명한다
- 필요한 근거가 없거나 필요한 사용자 판단이 missing이면 close가 막힘을 보고한다
- 작업 수락과 close 전에 잔여 위험을 표시할 수 있다
- 사용자의 작업 수락이 sensitive-action Approval, 잔여 위험 수용과 구분된다
- readable summary 또는 card가 현재 작업 상태, 사용자 판단 요청, 근거 요약, 닫기 준비 상태/blocker를 보여 주지만, template polish가 source of truth가 되지는 않는다. 작업 수락과 잔여 위험 사실은 관련 있을 때 이 요약 안에서 distinct하게 남는다
- prose나 renderer output만이 아니라 Core state, events, artifacts, projection/freshness facts, structured errors로 conformance를 증명할 수 있다

근거 기록, 읽기 쉬운 요약, projection 최신성은 이 경험을 지원합니다. 이것들이 단계의 정체성은 아니며, 이 사용자 읽기 경로를 넘어서는 projection polish는 범위 밖에 둡니다.

v0.2는 특정 사용자 대상 MVP scenario가 최소 표시 또는 blocker hook을 요구하지 않는 한 분리 검증, full 수동 QA 정책 매트릭스, stewardship validators, feedback-loop policy, export/recover, release handoff, Journey Card/Spine polish, Run Summary, TDD Trace, Module Map, Interface Contract, detailed Evidence Manifest, detailed Eval, Export projection을 staged profile로 남겨 둡니다. Browser QA Capture, Cross-Surface Verification automation, dashboard, broad connectors, Context Index, metrics, team workflow, orchestration은 MVP 밖에 둡니다.

v0.2를 통과했다는 것은 사용자가 하네스가 authorization wrapper 이상임을 볼 수 있다는 뜻입니다. Work의 scope, decision, 근거, 작업 수락, 위험 경계가 로컬에서 inspectable하게 유지됩니다.

## 에이전시 보증 팩(v0.3 Agency Assurance Pack)

v0.3은 MVP path를 강화하여 로컬 reference path가 검증, QA, 잔여 위험, 작업 수락, stewardship을 정직한 경계 안에서 route할 수 있게 합니다.

중점:

- profile별 Decision Packet 품질과 user-judgment routing
- sensitive-action Approval, Decision Packet, Write Authorization, 작업 수락, 잔여 위험 수용 분리
- same-session verification guard behavior를 포함한 분리 검증 독립성
- 수동 QA 정책 매트릭스, 수동 QA 막힘 조건, 유효한 QA 면제 판단
- 잔여 위험 수용 close의 전체 의미
- stewardship validators와 codebase stewardship coverage
- policy가 요구하는 TDD trace behavior
- policy가 요구하는 feedback-loop policy
- context-hygiene validators와 현재 상태/오래된 context 경계
- Core state, events, artifacts, projection/freshness facts, errors를 통해 judgment, QA, verification, 잔여 위험, 작업 수락의 분리를 증명하는 Agency Assurance Pack conformance fixtures

이 pack을 통과하면 user-facing MVP path가 agency-preserving하고 policy-aware하며 검증, QA, 잔여 위험, 작업 수락, stewardship 경계를 정직하게 다룬다는 뜻입니다. v1+ Expansion automation을 staged delivery로 승격하지는 않습니다.

## 운영과 인계 팩(v0.4 Operations & Handoff Pack)

v0.4는 같은 Core state model 위에서 로컬 운영 증명을 완성합니다.

중점:

- 하네스 런타임 홈, project state, artifact store, reference surface, MCP availability, projections, reconcile, validators/checks, agency/stewardship/context에 대한 doctor/readiness categories
- interrupted 또는 drifted operational state에 대한 recover handling
- state snapshots, report projection snapshots, artifact refs, redaction status, omitted-secret notes, retained/expired/unavailable artifact status에 대한 export behavior
- artifact integrity checks
- 담당 문서가 정의하는 release handoff report/export profile
- v0.4 operations profile에 대한 operator smoke. 여기에는 connect, doctor, serve MCP, 읽기용 요약 refresh, reconcile, recover, export, artifacts check, conformance run이 포함되며, 초기 단계는 더 작은 subset만 유지합니다
- export/recover, artifact integrity, release handoff, operator readiness, 그리고 담당 문서가 정의하고 증명한 higher guarantee level에 대한 operations/future fixture coverage
- 별도로 증명하고 승격하기 전까지 roadmap item을 v1+ Expansion에 두는 later-boundary checks

Operator command를 위한 두 번째 state model을 만들면 안 됩니다. Operator는 같은 Core state model 위에서 diagnose, repair, export, fixture run을 수행합니다.

Docs-maintenance는 별도의 읽기 전용 문서 profile로 남습니다. Documentation drift를 보고할 수 있지만 코어 권한 조각(v0.1 Core Authority Slice)도, 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)도, Agency Assurance Pack 또는 operations runtime conformance도, 구현 준비 상태 신호도 아닙니다.

## Roadmap 범위의 v1+ Expansion 후보

아래 항목은 향후 계획이 담당 문서를 통해 [로드맵 단계 승격 조건](../roadmap.md#단계-승격-조건)을 만족시켜 승격하기 전까지 staged delivery 밖에 둡니다. 승격하려면 사용자 소유 판단을 보존하고, Core 권한을 우회하지 않으며, 단계에 맞는 보안 보장 표현을 사용하고, 근거/검증/QA/작업 수락/잔여 위험에 미치는 영향을 밝히며, v0.1부터 v0.4까지의 범위를 부풀리지 않아야 합니다. 또한 필요한 능력 프로필, 정확한 계약, redaction/secret/PII 정책, 런타임 접점 캡처 시 아티팩트 보존 규칙과 test environment 규칙, fixture 또는 적합성 목표, fallback 동작, 읽기용 요약을 기준 상태로 삼지 않는다는 조건을 담당 문서가 정의해야 합니다.

| 후보 | 단계 경계 |
|---|---|
| 대시보드, 호스팅된 작업 UI, 아티팩트 대시보드, 풍부한 카드 확장 | 상태를 표시할 수는 있지만 권한, 구현 준비 상태, 닫기 준비 상태, 작업 수락, 잔여 위험 수용이 되면 안 됩니다. |
| 넓은 커넥터 시장 또는 접점 생태계 | 나중에 접점을 확장할 수 있지만 첫 Core 권한 루프 증명을 대체하거나 MCP 노출을 기본적으로 넓히면 안 됩니다. |
| 브라우저 QA 캡처 자동화 | 승격 뒤 수동 QA를 보조할 수 있지만 사람의 QA 판단, 작업 수락, 분리 검증을 대체하면 안 됩니다. |
| 여러 접점 검증 자동화 | 승격 뒤 evaluator routing을 자동화할 수 있지만 Core 소유 반환 기록 없이 Eval 또는 assurance를 충족하면 안 됩니다. |
| 예방적 가드 확장, 네이티브 후크, 고급 사이드카 워처 | 증명된 pre-tool blocking 또는 관찰 경로가 있을 때 접점을 강화할 수 있지만 label만으로 주장하면 안 됩니다. |
| 맥락 색인, 로컬 파생 지표, 장기 지표 | 읽기 전용 검색이나 진단을 제공할 수 있지만 write를 authorize하거나, gate를 충족하거나, 읽기용 요약을 refresh하거나, Task를 close하면 안 됩니다. |
| 팀 작업 흐름, 권한, 오케스트레이션, 병렬 lane | 향후 작업을 조율할 수 있지만 staged delivery나 single-project local authority의 필수 요소가 되면 안 됩니다. |
| 배포, canary, rollback, merge, production monitoring | 향후 통합 작업이 될 수 있습니다. Release handoff는 담당 문서가 더 많은 권한을 승격하기 전까지 report/export boundary로 남습니다. |

구현 중 향후 기능이 유용해 보이더라도 담당 문서가 권한 경로를 정의하고 증명하기 전까지는 읽기 전용 표시, 메타데이터, 아티팩트 후보, fixture 후보로 유지합니다. Build 문서는 단계별 전달을 소유하고, 로드맵은 후보 예시만 추적합니다.

## 단계별 종료 기준

문서 수락과 별도의 구현 계획 준비 결정 이후 향후 런타임 계획을 위한 구현자가 읽을 수 있는 점검 목록으로 사용합니다. 이들은 staged exit을 다시 말할 뿐이며 schema, fixture, DDL, new runtime requirement를 추가하지 않습니다. [문서 수락 상태](implementation-overview.md#문서-수락-상태)가 첫 런타임 배치 계획을 막고 있는 동안 구현을 허가하지 않습니다.

### 코어 권한 조각(v0.1 Core Authority Slice) 종료 점검 목록

- local project 하나가 등록된다.
- Task 하나가 Core-owned state 안에 존재한다.
- 범위가 정해진 작업 경계 하나가 intended change boundary를 이름 붙인다.
- Compatible scope 없는 product write는 Core가 구조화된 막힘으로 거부합니다. 이것은 기본 도구 실행 전 보안 차단이 아닙니다.
- Out-of-scope intended write는 Core가 구조화된 막힘으로 거부합니다. 이것은 기본 도구 실행 전 보안 차단이 아닙니다.
- 허용된 `prepare_write`는 지속적이며 한 번만 쓸 수 있는 Write Authorization을 만든다.
- Compatible `record_run`은 authorization을 한 번 consume한다.
- 두 번째 distinct product-write Run은 consumed authorization을 재사용할 수 없다.
- Artifact/evidence ref 하나가 등록되어 Run 또는 minimal owner relation에 연결된다.
- 상태/막힘 출력이 상태를 변경하지 않고 현재 상태 또는 blocker를 반환한다.
- Structured blocker/status response가 missing scope, missing write authority, 또는 missing artifact/evidence support를 보고한다.

### 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP) 종료 점검 목록

- 평범한 사용자 언어가 Harness vocabulary를 요구하지 않고 tracked work를 시작하거나 resume할 수 있다.
- User-facing path가 scope, non-goals, acceptance criteria, evidence expectations, close readiness, judgment boundaries를 clarify한다.
- Product/UX judgment와 기술 구조 판단을 서로 분리하고, 민감 동작 승인, 작업 수락, 잔여 위험 수용과도 분리해 제시할 수 있다.
- Small direct changes와 tracked work가 write authority, evidence, 필요한 사용자 판단을 우회하지 않으면서 서로 다른 procedural budget을 사용한다.
- Status/next output이 현재 scope, missing decisions, 근거 상태, 잔여 위험 표시, close blockers, 안전한 다음 행동을 설명한다.
- Required 근거가 없으면 close가 막힘을 보고한다.
- 필요한 사용자 판단이 missing 또는 unresolved이면 close가 막힘을 보고한다.
- 알려진 닫기 관련 위험이 있으면 작업 수락 또는 close 전에 잔여 위험이 보인다.
- 사용자의 작업 수락이 sensitive-action Approval과 잔여 위험 수용과 별도로 기록되거나 표현된다.
- 잔여 위험 수용을 지원하는 경우, 이것이 작업 수락과 뚜렷하게 구분되어 보인다.
- 사용자에게 보이는 readable summary 또는 card는 Core record에서 파생되며, template polish를 기준 권한으로 만들지 않고 MVP path에 충분하다.

### 에이전시 보증 팩(v0.3 Agency Assurance Pack) 종료 점검 목록

- Decision Packet quality와 user-judgment routing이 fixture로 증명된다.
- Sensitive-action Approval이 Decision Packet, Write Authorization, 수동 QA, verification, 작업 수락, 잔여 위험 수용을 대체하지 않는다.
- 분리 검증 독립성와 same-session verification guard behavior가 fixture로 증명된다.
- Policy가 요구하는 곳에서 수동 QA 정책 매트릭스와 QA blocker가 fixture로 증명된다.
- 위험 수용 close는 담당 semantics에 따라 accepted Residual Risk refs를 인용한다.
- Policy가 요구하는 곳에서 stewardship validators, feedback-loop policy, TDD trace behavior, context-hygiene behavior가 cover된다.
- Agency conformance가 Journey visibility, user-judgment routing, Autonomy Boundary respect, distinct judgment categories/routes, 잔여 위험 처리를 증명한다.

### 운영과 인계 팩(v0.4 Operations & Handoff Pack) 종료 점검 목록

- Doctor/readiness가 하네스 런타임 홈, project state, artifact store, reference surface, MCP availability, projections, reconcile, validators/checks, agency/stewardship/context category를 보고한다.
- Recover는 recovery artifact를 successful completion proof로 취급하지 않으면서 interrupted 또는 drifted 운영 상태를 처리한다.
- Export는 state snapshot, report projection snapshot, artifact refs, redaction status, omitted-secret notes, retained/expired/unavailable artifact status를 포함한다.
- Artifact integrity check는 missing 또는 mismatched artifact를 기존 diagnostics로 보고한다.
- Release handoff report/export behavior는 deployment, merge, rollback, production authority를 가져오지 않고 담당 profile을 따른다.
- Operations/future fixture coverage가 export/recover, artifact integrity, release handoff, operator readiness, 승격된 higher guarantee level을 prose가 아니라 exact-shape fixture로 증명한다.
- 후속 경계 확인은 담당 문서가 승격하고 증명하기 전까지 v1+ Expansion item을 staged delivery 밖에 둔다.

## 단계별 관찰 가능 항목

| 단계 | 사용자 또는 operator가 볼 수 있는 것 |
|---|---|
| 코어 권한 조각(v0.1 Core Authority Slice) | Implementer는 로컬 Task 하나가 scoped work boundary, `prepare_write`, Write Authorization, `record_run`, artifact/evidence ref, 구조화된 상태/막힘 출력을 통과하는 것을 볼 수 있습니다. |
| 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP) | 사용자는 평범한 작업이 범위, 사용자 소유 판단, 근거, 닫기 준비 상태, 작업 수락, 잔여 위험 언어로 정리되고 근거 또는 필요한 사용자 판단이 없으면 close가 막힘을 보고하는 것을 볼 수 있습니다. |
| 에이전시 보증 팩(v0.3 Agency Assurance Pack) | Local path가 verification, 수동 QA, 잔여 위험 수용, 작업 수락, stewardship, TDD, feedback, context hygiene, close behavior를 Core record와 fixture로 설명합니다. |
| 운영과 인계 팩(v0.4 Operations & Handoff Pack) | Operator는 같은 Core state 위에서 diagnose, recover, reconcile, export, artifact check, conformance run, release handoff 준비를 수행할 수 있습니다. |

단계별 전달 이후에는 promoted roadmap item이 담당 문서가 exact contract와 fixture coverage를 정의한 뒤에만 authority loop를 읽고, 표시하고, 감싸고, 확장할 수 있습니다.
