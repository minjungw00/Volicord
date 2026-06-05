# Build: 구현 개요

## 이 문서가 도와주는 일

이 문서는 Build 문서의 기준점입니다. 현재 저장소 상태, 유지보수자 인계 상태, 향후 구현 경로를 한곳에서 설명합니다. 미래의 하네스 서버 구현자는 여기서 시작한 뒤 정확한 계약은 담당 Reference 문서로 이동합니다.

이 저장소는 현재 문서 전용입니다. 문서 수락과 별도의 구현 계획 준비 결정이 끝난 뒤에만 하네스 서버 소스 저장소가 될 예정입니다. 아직 하네스 서버/런타임 구현, 런타임 상태, 생성된 운영 산출물, 실행 가능한 fixture 파일, 생성된 projection, conformance runner, 제품 코드는 없습니다.

Build 문서는 단계별 구현을 요약합니다. Exact schema, DDL, API request/response shape, storage table, projection template body, fixture format, security guarantee를 정의하지 않습니다. 그런 계약은 아래 연결된 Reference 담당 문서에 남습니다.

## 이런 때 읽기

- 문서가 유지보수자 수락 검토에 준비되었는지 확인할 때.
- 인계 gate가 수락된 뒤 향후 하네스 서버 구현을 계획할 때.
- 내부 엔지니어링 점검과 MVP-1 사용자 가치를 분리해서 보고 싶을 때.

## 핵심 생각

Build에는 네 개의 active page가 있습니다.

| 문서 | 역할 |
|---|---|
| [구현 개요](implementation-overview.md) | 현재 저장소 상태, 구현 접근, 인계 상태, 준비 조건, 읽기 경로. |
| [내부 엔지니어링 점검](engineering-checkpoint.md) | 첫 내부 권한 루프 smoke. 제품 MVP도 아니고 사용자 가치 검증도 아닙니다. |
| [MVP-1 사용자 작업 루프](mvp-user-work-loop.md) | 첫 사용자 가치 구현 계획, MVP-1 포함/제외 범위, 담당 문서 링크, 중앙 서버 코딩 전 결정 기록. |
| [런타임 설계 흐름](runtime-walkthrough.md) | 의도한 request-to-close 동작의 설계 walkthrough. 런타임이 존재한다는 근거가 아닙니다. |

구현 경로는 일부러 단계화되어 있습니다.

1. 문서 수락과 구현 계획 준비 상태를 수락합니다.
2. 내부 엔지니어링 점검을 구현합니다. 범위는 로컬 Core 권한 루프 하나입니다.
3. MVP-1 사용자 작업 루프를 구현합니다. 첫 좁은 사용자 가치입니다.
4. 보증 프로필, 운영 프로필, 로드맵 후보는 담당 문서가 승격할 때까지 미룹니다.

Build 문서의 구현 동사는 모두 readiness gate가 수락된 뒤의 향후 작업을 뜻합니다. [문서 수락 상태](#문서-수락-상태)가 구현 계획 준비 상태를 수락하지 않는 동안 Build는 계획 지침일 뿐입니다.

## 세 구현 층

구현자는 후속/profile 문서나 로드맵을 읽기 전에 아래 세 층을 분리해야 합니다.

| 층 | 활성 구현 범위 | 이 층 밖 |
|---|---|---|
| 첫 실행 가능한 권한 루프 | 내부 엔지니어링 점검입니다. 로컬 project state 하나, registered reference `capability_profile` 하나, 활성 Task 하나, 활성 Change Unit/scope boundary 하나, `harness.prepare_write`, 한 번만 쓰는 활성 쓰기 권한(Write Authorization), `harness.record_run`의 소비, 최소 artifact/evidence 기록, 좁은 status/닫기 차단 사유 확인을 포함합니다. | 평소 말 intake, 저장된 사용자 판단 흐름, full close semantics, full projection renderer, 상세 report, operations, conformance runner, broad connector APIs, hosted connector registry, cross-surface orchestration, later-profile storage. |
| 첫 사용자 작업 루프 | MVP-1 사용자 작업 루프입니다. 평소 말 intake, `harness.status.next_actions`가 있는 status, 같은 reference surface guarantee display 하나, 집중된 사용자 판단 요청/기록, Core-owned evidence summary, close result/blocker, 최종 수락과 잔여 위험 표시, 명시적 잔여 위험 수락이 있는 accepted-risk close, 네 가지 사용자용 작은 출력, 에이전트용 맥락 패킷 하나를 더합니다. | Full Evidence Manifest, detached Eval, full Manual QA matrix, Assurance hardening, operations/export/recover, dashboard, hosted UI, broad connector, hosted connector registry, cross-surface orchestration, automation, detailed report. |
| 후속/profile 범위 | 보증 프로필, 운영 프로필, 로드맵입니다. Full Manual QA matrix, detached Eval system, export/recover suite, dashboard/hosted UI, broad connector ecosystem, hosted connector registry, automated Browser QA Capture, preventive guard expansion, parallel orchestration, cross-surface orchestration, 상세 report projection을 포함합니다. | Owner가 stage impact와 함께 좁은 동작을 명시적으로 승격하기 전까지 내부 엔지니어링 점검이나 최소 MVP-1 종료 조건이 아닙니다. |

활성 MVP-1 method set은 정확히 `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, `harness.close_task`입니다. `harness.next`는 활성 MVP-1 method가 아닙니다. 다음 안전한 행동은 `harness.status.next_actions`로 표현합니다.

활성 MVP-1 작은 출력 세트는 독자별로 나뉩니다. 사용자용은 `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`이고, 에이전트용은 `agent-context-packet`입니다. Persisted Journey Card, full Evidence Manifest, Eval report, Manual QA report, TDD Trace, Module Map, Interface Contract, Export report 같은 상세 report는 owner가 좁은 non-required display로 표시하거나 stage impact와 함께 승격하기 전까지 후속/profile 범위에 남습니다.

## 현재 검토 기준

현재 문서 세트는 재설계 이후 검토 기준이며 문서 수락 후보입니다. 유지보수자가 아래 상태 표를 의도적으로 바꾸기 전까지 최종 수락된 구현 자료가 아닙니다.

현재 사실:

- 문서 검토 상태: 유지보수자 수락 대기.
- 구현 계획 준비 상태: 수락되지 않음.
- 런타임 구현 상태: 시작하지 않음.
- 서버 코딩 전 결정: 코드 작성용으로 수락되지 않음.
- 향후 저장소 역할: 수락과 readiness 이후 하네스 서버 소스 저장소. 사용자의 제품 저장소도 아니고 하네스 런타임 홈도 아닙니다.

검토 중 schema/design, stage boundary, guarantee level, storage/API, fixture semantics와 관련해 코딩을 막는 큰 결정이 드러나면 [MVP-1 사용자 작업 루프: 서버 코딩 전 필요한 구현 결정](mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정)에만 기록합니다. Active docs 곳곳에 큰 결정 표시를 흩어 놓지 않습니다.

## 문서 수락 상태

이 표는 유지보수자가 갱신하는 인계 표시입니다. 주변 설명이나 checklist 완료만 보고 수락을 추론하면 안 됩니다.

| 상태 범주 | 현재 상태 | 경계 |
|---|---|---|
| 문서 검토 상태 | 재설계 이후 검토 상태이며 문서 수락 후보입니다. | 유지보수자 수락은 아직 대기 중입니다. 문서 수락만으로 서버/런타임 구현이 시작되지 않습니다. |
| 구현 계획 준비 상태 | 수락되지 않았습니다. | 유지보수자가 readiness criteria를 수락하거나 blocker를 재분류하기 전까지 첫 런타임 batch planning은 시작할 수 없습니다. |
| 런타임 구현 상태 | 시작하지 않았습니다. | 이 저장소에는 runtime/server code, runtime data, executable fixture, generated projection, conformance result, generated operational artifact가 없습니다. |
| 서버 코딩 전 결정 기록 | 코드 작성용으로 수락되지 않았습니다. | 문서 기준에서 해소된 결정과 아직 열린 결정은 [MVP-1 사용자 작업 루프](mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정)에 중앙화합니다. |

## 문서 인계 요약

이 인계는 현재 문서가 무엇을 정의하고 무엇이 아직 구현 준비를 막는지 말합니다. 런타임 상태, conformance evidence, 구현 허가, 생성된 출력이 아닙니다.

이 문서 기준선이 정의하는 것:

- 제품 명제: 하네스는 범위, 사용자 소유 판단, 근거, 검증 기대, QA 기대, 최종 수락, 잔여 위험, 닫기 준비 상태를 위한 로컬 기준 기록입니다.
- 단계별 Build 경로: 내부 엔지니어링 점검을 먼저 만들고, 그다음 MVP-1 사용자 작업 루프, 이후 보증 프로필과 운영 프로필.
- 담당 문서 경계: Core, API, storage, projection/template, security, operations, conformance, agent integration, glossary, runtime architecture, design quality의 exact contract는 Reference 문서에 있습니다.
- 문서 유지보수 규칙: owner boundary, 이중 언어 의미 일치, status wording, link hygiene, drift routing은 Maintain 문서에 있습니다.

현재 정의하지 않거나 존재하지 않는 것:

- 실행 가능한 하네스 서버/런타임 코드.
- 런타임 상태, 생성된 운영 산출물, 생성된 projection, 하네스 런타임 홈 내용.
- 실행 가능한 fixture 파일, fixture runner, 현재 runtime conformance result.
- 수락된 서버 코딩 결정.

현재 전달 의미:

- 내부 엔지니어링 점검은 가장 작은 로컬 Core 권한 루프를 증명합니다. 내부 구현 신뢰를 위한 점검이지 제품 MVP가 아닙니다.
- MVP-1 사용자 작업 루프는 첫 사용자 가치를 증명합니다. 평소 작업을 추적하고, 범위를 잡고, 설명하고, 정직하게 막고, 근거/판단/위험 경계를 보이면서 닫거나 보류할 수 있어야 합니다.
- 보증 프로필과 운영 프로필은 이후 hardening입니다. 담당 문서가 좁은 항목을 명시적으로 승격하기 전까지 내부 점검이나 최소 MVP-1에 넣지 않습니다.
- 로드맵은 Roadmap 기준과 담당 계약을 통해 승격되기 전까지 향후 범위입니다.

서버 코딩을 시작하기 전에는 유지보수자가 [문서 수락 상태](#문서-수락-상태)를 의도적으로 갱신하고, [하네스 서버 구현 준비 조건](#하네스-서버-구현-준비-조건)을 수락하거나 재분류하며, [MVP-1 사용자 작업 루프](mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정)의 결정 기록을 stage impact와 함께 수락하거나 미뤄야 합니다.

## 하네스 서버 구현 준비 조건

첫 런타임 batch planning이나 서버 코딩 전에 유지보수자는 아래 조건을 수락해야 합니다. 수락하지 않는 조건은 stage impact를 이름 붙여 명시적으로 미뤄야 합니다.

| 조건 | 충족되어야 하는 것 | 담당 경로 |
|---|---|---|
| 저장소 정체성 | 문서가 이 저장소를 현재 문서 전용, 이후 하네스 서버 소스 저장소라고 일관되게 말합니다. | 이 문서, README, Maintain guides. |
| 단계 경계 | 내부 엔지니어링 점검, MVP-1 사용자 작업 루프, 보증 프로필, 운영 프로필, 로드맵이 섞이지 않습니다. | 이 문서, [MVP-1 사용자 작업 루프](mvp-user-work-loop.md), [로드맵](../roadmap.md). |
| MVP-1 범위 | MVP-1은 사용자에게 보이는 범위, 판단, Core-owned evidence summary, 닫기 막힘, 다음 행동, 필요한 최종 수락, 잔여 위험 표시, 명시적 residual-risk acceptance를 통한 accepted-risk close를 포함하고 이후 프로필은 제외합니다. | [MVP-1 사용자 작업 루프](mvp-user-work-loop.md#mvp-1에-포함되는-것). |
| Reference surface scope | 활성 MVP는 reference `capability_profile` 하나를 대상으로 합니다. Capability label은 write authority를 부여하지 않고, unsupported field는 guarantee claim을 낮추거나 막습니다. Broad connector ecosystem, hosted connector registry, cross-surface orchestration은 later/profile에 남습니다. | [Agent 통합 참조](../reference/agent-integration.md#capability-profiles), [Surface Cookbook](../reference/surface-cookbook.md#reference-local-surface). |
| 설계 품질 차단 경계 | 활성 MVP design-quality blocker는 Autonomy Boundary exceeded, unresolved user judgment, missing active scope, missing required evidence, stale context affecting write/close, surface capability insufficient for a claimed guarantee로 제한됩니다. 더 넓은 policy catalog는 기본적으로 Routed candidate 또는 Advisory/later입니다. | [설계 품질 정책](../reference/design-quality-policies.md#활성-mvp-차단-집합). |
| API 담당 문서 합의 | Active MVP-1 API, shared schema, resource, error, idempotency, state conflict behavior는 해당 API 구현을 시작하기 전에 담당 문서 합의가 수락되어 있어야 합니다. | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md). |
| Storage 담당 문서 합의 | Minimal storage profile, runtime home layout, lock, artifact, migration, later-profile storage boundary는 DDL, runtime data, artifact storage 구현 전에 담당 문서 합의가 수락되어 있어야 합니다. | [Storage](../reference/storage.md). |
| Core 담당 문서 합의 | Task, scope, user judgment, `prepare_write`, Write Authorization, `record_run`, blocker, close semantics는 영향을 받는 Core path를 코딩하기 전에 active stage 기준 Core 담당 문서 합의가 수락되어 있어야 합니다. | [Core Model 참조](../reference/core-model.md). |
| 보안 자세 | API/MCP surface를 노출하기 전에 MVP-1 guarantee wording과 local-access posture에 대한 Security 담당 문서 합의가 수락되어 있어야 합니다. 그전까지 wording은 cooperative plus limited detective에 머물러야 하며 OS sandboxing, arbitrary-tool isolation, tamper-proof storage, default pre-tool blocking, permission isolation을 주장하지 않아야 합니다. | [보안 참조](../reference/security.md). |
| Compact output 경계 | 내부 엔지니어링 점검은 status/blocker output을 사용하고, MVP-1은 projection을 authority로 만들지 않는 네 가지 사용자용 작은 출력과 에이전트용 패킷 하나를 사용합니다. | [MVP-1 사용자 작업 루프](mvp-user-work-loop.md#mvp-1에-포함되는-것), [Projection과 Template 참조](../reference/projection-and-templates.md). |
| 향후 conformance 경계 | Fixture 문서는 향후 계획으로 남습니다. 실행 가능한 runner나 pass/fail result가 문서만으로 생긴 것처럼 쓰지 않습니다. | [Conformance Fixtures 참조](../reference/conformance-fixtures.md), [향후 Fixtures](../later/future-fixtures.md). |
| 열린 결정 라우팅 | 큰 구현 결정은 MVP-1 결정 기록에 중앙화하고 active docs에 흩지 않습니다. | [MVP-1 사용자 작업 루프](mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정). |
| 영어/한국어 의미 일치 | 대응 Build 문서는 같은 의미, owner link, active file coverage를 유지하고 한국어는 자연스럽게 읽힙니다. | 영어/한국어 Build docs와 Maintain guides. |

## 구현 접근

Readiness가 수락된 뒤에는 넓은 사용자 경험보다 먼저 Core가 소유한 가장 작은 로컬 권한 경로를 구현합니다.

| 영역 | 내부 엔지니어링 점검 접근 | MVP-1 접근 | Exact owner |
|---|---|---|---|
| 프로세스 | 모듈이 분명한 로컬 하네스 프로세스 또는 서버 하나로 충분합니다. | 같은 로컬 경로에 사용자용 intake/status 동작을 더합니다. | [런타임 아키텍처 참조](../reference/runtime-architecture.md). |
| Core | 권한 루프 하나를 통해 canonical state를 변경합니다. | 두 번째 authority model 없이 사용자에게 보이는 work-loop state와 close/status path를 더합니다. | [Core Model 참조](../reference/core-model.md). |
| API | Minimal status/blocker read, owner-valid setup path, `prepare_write`, `record_run`, artifact/evidence ref path 하나, 좁은 close-blocker check. Natural-language intake는 필요하지 않습니다. | Status/next actions, intake, user judgment, write check, run/evidence, close를 위한 exact MVP-1 public method set을 더합니다. | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md). |
| Surface integration | Reference `capability_profile` 하나를 등록하고 그 한계를 정직하게 표시합니다. | User-visible fallback, blocker, guarantee display behavior에도 같은 profile을 사용합니다. | [Agent 통합 참조](../reference/agent-integration.md), [Surface Cookbook](../reference/surface-cookbook.md). |
| Storage | 권한 루프에 필요한 owner-approved 최소 persistence만 사용합니다. | 현재 기록에서 파생한 작은 출력과 MVP-1 user judgment, artifact와 artifact link, minimal evidence summary, blocker, replay/audit에 필요한 active slice만 더합니다. | [Storage](../reference/storage.md). |
| 보안 | Cooperative plus limited detective. | 같은 baseline 위에서 사용자에게 보이는 blocker와 honest guarantee display를 더 분명히 합니다. | [보안 참조](../reference/security.md). |
| Projection/view | Status/blocker output만 필요하며 full renderer는 필요하지 않습니다. | 네 가지 사용자용 작은 출력과 에이전트용 패킷 하나로 사용자 루프를 만족할 수 있습니다. | [Projection과 Template 참조](../reference/projection-and-templates.md), [Template 참조](../reference/templates/README.md). |
| 운영/conformance | 향후 smoke 작성 계획일 뿐입니다. | Runtime fixture가 실제로 만들어지기 전까지 behavior example로 다룹니다. | [운영과 Conformance 참조](../reference/operations-and-conformance.md), [Conformance Fixtures 참조](../reference/conformance-fixtures.md). |

## 아직 만들지 않는 것

아래 항목은 내부 엔지니어링 점검이나 최소 MVP-1 prerequisite로 만들면 안 됩니다.

- 전체 보증 프로필: detached verification hardening, Manual QA matrix, full Approval lifecycle, full residual-risk lifecycle, stewardship validator, TDD trace policy, feedback-loop policy, broad context-hygiene validator.
- 전체 운영 프로필: doctor/readiness suite, recover/export, artifact integrity operations, release handoff, projection refresh/reconcile operations, conformance runner, broad operator coverage.
- 상세 report projection: persisted Journey Card, full Evidence Manifest, Eval report, Manual QA report, TDD Trace, Module Map, Interface Contract, Export report, 기타 polished report. 명시적으로 non-required display로 승격된 경우는 예외입니다.
- 로드맵 후보: dashboard, hosted workflow UI, Context Index, broad connector marketplace, hosted connector registry, automated Browser QA Capture, Cross-Surface Verification automation, cross-surface orchestration, preventive guard expansion, native hook expansion, Advanced Sidecar Watcher, Local Derived Metrics, team workflow, permission, parallel orchestration, deployment, canary, rollback, production monitoring.

이후 capability는 담당 문서가 exact contract, fallback behavior, fixture/conformance expectation, 필요한 redaction/secret handling, projection-as-canonical이 아님을 정의한 뒤에만 권한 루프를 읽거나 표시하거나 감쌀 수 있습니다.

## Build 읽기 경로

향후 구현자에게 권장하는 순서입니다.

1. [구현 개요](implementation-overview.md): 현재 상태와 인계.
2. [내부 엔지니어링 점검](engineering-checkpoint.md): 첫 내부 권한 루프 smoke.
3. [MVP-1 사용자 작업 루프](mvp-user-work-loop.md): 첫 사용자 가치 계획과 결정 기록.
4. [런타임 설계 흐름](runtime-walkthrough.md): 의도한 request-to-close 동작.
5. [Reference 색인](../reference/README.md): 정확한 담당 계약 찾기.

주요 담당 문서:

- [Core Model 참조](../reference/core-model.md)
- [MVP API](../reference/api/mvp-api.md)
- [API Schema Core](../reference/api/schema-core.md)
- [API Errors](../reference/api/errors.md)
- [Storage](../reference/storage.md)
- [보안 참조](../reference/security.md)
- [Projection과 Template 참조](../reference/projection-and-templates.md)
- [런타임 아키텍처 참조](../reference/runtime-architecture.md)
- [운영과 Conformance 참조](../reference/operations-and-conformance.md)
- [Conformance Fixtures 참조](../reference/conformance-fixtures.md)
