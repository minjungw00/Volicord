# Build: MVP 계획

구현 전, 서버 코딩 전에 읽을 단일 Build 진입점입니다. 현재 MVP 경계, 첫 내부 smoke 목표, 요청에서 닫기까지의 경로, 서버 코딩 전 결정, 담당 Reference 문서 링크를 한곳에 둡니다.

Build 문서는 계획 안내만 담당합니다. 정확한 schema, DDL, API request/response shape, storage table, projection template body, fixture format, security guarantee는 정의하지 않습니다. 그런 계약은 아래 담당 Reference 문서에 남습니다.

<a id="문서-수락-상태"></a>
<a id="문서-인계-요약"></a>
<a id="documentation-acceptance-status"></a>
<a id="maintainer-handoff-summary"></a>
<a id="repository-status"></a>
## 저장소 상태

이 저장소는 현재 문서 전용이며 재설계 이후 검토 상태입니다. 문서 수락과 별도의 구현 계획 준비 결정이 끝난 뒤에만 하네스 서버 소스 저장소가 될 예정입니다.

지금은 하네스 서버/런타임 구현, 런타임 상태, 생성된 projection, 생성된 운영 산출물, 실행 가능한 fixture 파일, conformance runner, 하네스 런타임 홈 내용, 제품 코드가 없습니다. 문서 파일은 원천 자료이지 하네스 런타임 기록이 아닙니다.

유지보수자가 [서버 코딩 전 결정](#서버-코딩-전-결정)의 열린 결정을 해결하거나 수락하거나 stage impact와 함께 명시적으로 미루기 전까지 서버 코딩은 시작하면 안 됩니다.

## 현재 있는 것

- Start, Use, Build, Reference, Later, Maintain, Roadmap 독자를 위한 이중 언어 계획 문서.
- Core, API, storage, security, projection/templates, agent integration, runtime architecture, operations/conformance, design quality, glossary를 담당하는 Reference 문서.
- 현재 MVP 밖의 future assurance, operations, fixture, roadmap 자료를 보존하는 Later/Profile 문서.
- 향후 서버 코드 작성 전 구현 계획 경로를 압축한 이 Build 계획.

## 현재 없는 것

- 서버/런타임 구현 코드.
- 런타임 상태, 생성된 운영 파일, 생성된 projection, runtime artifact, 하네스 런타임 홈 데이터.
- 실행 가능한 fixture 파일, conformance runner, 생성된 conformance artifact, 현재 runtime conformance result.
- dashboard, hosted UI, connector marketplace, hosted connector registry, operations suite, 배포/운영 자동화.
- 제품 저장소 코드나 제품 구현 변경.

<a id="핵심-생각"></a>
<a id="mvp-1에-포함되는-것"></a>
<a id="main-idea"></a>
<a id="mvp-1-included"></a>
## 현재 MVP 활성 조각

현재 MVP는 하네스가 범위, 사용자 소유 판단, 증거, 닫기 가능 여부, 잔여 위험 표시를 위한 로컬 기준 기록임을 보여주는 가장 작은 사용자 작업 루프입니다.

포함 범위는 다음과 같습니다.

- 평소 말로 추적되는 작업을 시작하거나 이어가기
- 작업 형태 분류, 범위, 제외 범위, 성공 기준 요약
- 담당 API 경로를 통한 최소 사용자 판단 처리. 필요할 때 민감 동작 승인, 최종 수락, 잔여 위험 수락을 구분해 표시
- `surface_id=reference-local-mcp`용 reference `capability_profile` 하나와 정직한 fallback, 보장 수준 표시
- `prepare_write`를 통한 협력형 쓰기 전 범위 확인
- Core 담당 문서가 요구하는 지속되고 한 번만 쓰이는 Write Authorization 동작
- `record_run`과 등록된 artifact/evidence ref 또는 최소 증거 요약 경로
- Full Evidence Manifest가 아닌, Core가 소유한 compact evidence summary
- 담당 status surface를 통한 현재 상태와 다음 안전한 행동 표시
- 필수 증거가 충분하지 않거나, 필수 사용자 판단이 해결되지 않았거나, 필요한 최종 수락이 없거나, 닫기에 영향을 주는 잔여 위험이 필요 수준으로 보이지 않거나 수락되지 않았을 때의 닫기 차단 사유
- 닫기 전에 close-relevant risk가 있을 때 잔여 위험 표시
- 현재 작업 루프용, Core 기록에서 파생된 compact output. Projection은 기준 권한이 아니라 파생 읽기입니다.

현재 MVP 표현은 cooperative에 제한된 detective behavior를 더한 수준입니다. OS permission control, arbitrary-tool sandboxing, tamper-proof storage, default pre-tool blocking, security isolation을 암시하면 안 됩니다.

<a id="mvp-1에서-제외되는-것"></a>
<a id="아직-만들지-않을-이후-프로필"></a>
<a id="mvp-1-excluded"></a>
<a id="later-profiles-not-to-build-yet"></a>
## 제외되는 이후 자료

담당 문서가 scope, fallback behavior, proof expectation과 함께 좁은 동작을 명시적으로 승격하기 전까지 아래 자료는 현재 MVP 밖에 둡니다.

- full Evidence Manifest behavior, detailed evidence catalog, persisted Journey Card, detailed run report, TDD Trace, Module Map, Interface Contract, Domain Language, Export report, later-profile template
- detached Eval, detached verification hardening, full Manual QA matrix, full waiver machinery, rich Approval lifecycle, rich residual-risk lifecycle, broad stewardship 또는 context-hygiene validator
- dashboard, hosted workflow UI, artifact dashboard, hosted connector registry, connector marketplace, broad connector ecosystem, cross-surface orchestration, team workflow, metrics, automation candidate
- doctor/readiness suite, recover/export, artifact integrity operation, release handoff, projection refresh/reconcile operation, broad operator coverage, conformance runner, executable fixture catalog, generated conformance artifact 같은 운영 프로필 자료
- preventive guard expansion, native hook expansion, broad isolated execution, permission isolation, deployment, canary, rollback, production monitoring

Reference schema에 있다는 사실만으로 현재 MVP가 커지지 않습니다. Required field는 owning tool, record, profile이 active이거나 실제로 사용될 때만 적용됩니다.

<a id="first-internal-smoke-target"></a>
## 첫 내부 smoke 목표

첫 내부 smoke 목표는 제품 MVP가 아닙니다. 사용자용 작업 루프를 넓히기 전에 가장 작은 Core 권한 루프를 증명합니다.

보여야 하는 것은 다음과 같습니다.

- local project registration 하나와 reference `capability_profile` 하나
- active Task 하나와 active Change Unit 또는 담당 문서가 인정한 scope boundary 하나
- 담당 경로를 통한 `prepare_write` compatible, blocked, dry-run, replay 동작
- Write Authorization을 한 번 소비하는 compatible `record_run` 하나
- missing, stale, consumed, observed-outside-authorized-scope attempt가 완료 증거를 만들지 않고 막히는 처리
- artifact/evidence ref 하나와 compact evidence coverage/gap read
- Core 상태를 변경하지 않고 읽는 status/blocker output
- missing evidence, unresolved judgment, visible residual risk를 보여줄 수 있지만 full assurance close semantics는 구현하지 않는 좁은 close-blocker check

이 smoke 목표는 평소 말 intake 대신 담당 문서가 인정한 setup 또는 seed path를 사용할 수 있습니다. Full projection renderer, detailed template, dashboard, hosted UI, operations suite, conformance runner, broad connector platform은 필요하지 않습니다.

<a id="user-work-loop"></a>
## 사용자 작업 루프

사용자 작업 루프는 사용자가 하네스 내부 라벨을 몰라도 평소 작업을 시작하거나 이어가게 합니다. 먼저 사용자가 원하는 것, 저장소나 하네스 상태에서 확인할 수 있는 것, 아직 불확실한 것, 사용자가 직접 판단해야 하는 것을 구체화해야 합니다.

MVP의 요구사항 구체화는 active Task, scope/Change Unit, user judgment 담당 경로를 통해서만 지속됩니다. 별도의 committed Discovery Brief, Shared Design record, Question Queue, Assumption Register, evidence record, Write Authorization, final acceptance, residual-risk acceptance, close record가 아닙니다.

다음 안전한 행동은 계속 보여야 합니다. Core, MCP, reference surface가 어떤 주장을 뒷받침할 수 없다면 status가 그 사실을 말해야 합니다. 권한 상태를 지어내면 안 됩니다.

<a id="request-to-close-path"></a>
## 요청에서 닫기까지의 경로

1. 사용자가 평소 말로 작업을 요청합니다.
2. 하네스는 Task를 만들거나 이어가고, 범위와 제외 범위를 요약하며, 사용자가 소유한 판단이 있을 때만 최소 판단을 요청합니다.
3. 제품 쓰기 전에는 agent 또는 surface가 `prepare_write`를 호출합니다. 호환되는 작업은 담당 문서가 정의한 Write Authorization 결과를 받고, 호환되지 않는 작업은 blocker 또는 담당 문서가 정의한 error를 받습니다.
4. 쓰기 또는 direct work 뒤에는 `record_run`이 실제로 일어난 일을 기록하고 등록된 artifact/evidence ref 또는 compact evidence summary 경로와 연결합니다.
5. Status와 compact output은 현재 범위, 미해결 판단, evidence gap, blocker, 다음 안전한 행동, guarantee level, 잔여 위험 표시를 Core 기록에서 파생해 보여줍니다.
6. `close_task`는 담당 문서가 정의한 활성 경로로 닫거나 닫기 차단 사유를 반환합니다. MVP close는 최종 수락, 잔여 위험 수락, 검증, QA, evidence sufficiency를 서로 구분해야 합니다.

`compatible`, `blocked`, `allowed`는 하네스 권한 결과입니다. 향후 승격된 profile이 정확한 mechanism을 증명하지 않는 한 물리적인 OS blocking, arbitrary-tool prevention, sandbox isolation, permission isolation을 뜻하지 않습니다.

<a id="서버-코딩-전-필요한-구현-결정"></a>
<a id="아직-열려-있는-구현-결정"></a>
<a id="implementation-decisions-before-server-coding"></a>
<a id="implementation-decisions-needed-before-server-coding"></a>
<a id="implementation-decisions-still-open"></a>
## 서버 코딩 전 결정

유지보수자가 각 행을 수락하거나 해결하거나 명시적인 stage impact와 함께 미루기 전까지 서버 코딩은 시작하면 안 됩니다.

| 결정 항목 | 현재 상태 | 코딩 전에 결정할 것 |
|---|---|---|
| 구현 계획 준비 판단 | 수락되지 않았습니다. | 문서 계획 기준이 첫 runtime-batch planning에 충분한지 유지보수자가 수락하거나, 남은 blocker와 affected stage를 이름 붙여야 합니다. |
| Public API 코딩 수락 | 코딩용으로 수락되지 않았습니다. | 영향을 받는 tool/resource를 코딩하기 전에 API 담당 문서에서 active MVP method set, shared schema, resource, error, idempotency/replay behavior, unavailable Core/MCP behavior, later/profile exclusion을 수락해야 합니다. |
| Storage/DDL 코딩 수락 | 코딩용으로 수락되지 않았습니다. | DDL, runtime data file, artifact storage를 만들기 전에 minimal storage profile, runtime home layout, lock, artifact, migration, replay/audit 필요 범위, later-profile storage boundary를 수락해야 합니다. |
| Core transition 수락 | 코딩용으로 수락되지 않았습니다. | Active Task/scope, `user_judgment`, `prepare_write`, Write Authorization, `record_run`, blocker, status, evidence summary, `close_task` semantics를 현재 MVP path 기준으로 수락해야 합니다. |
| Security와 local-access 수락 | 코딩용으로 수락되지 않았습니다. | API/MCP surface를 노출하기 전에 local-only posture와 cooperative/limited-detective guarantee wording을 수락해야 합니다. MVP는 OS sandboxing, arbitrary-tool isolation, tamper-proof storage, default pre-tool blocking, permission isolation, security isolation을 주장하면 안 됩니다. |
| Surface와 compact-output 경계 | 코딩용으로 수락되지 않았습니다. | 표시 코드를 구현하기 전에 reference `capability_profile` 하나, compact user-facing view, compact agent-facing packet, freshness/unavailable behavior, projection-as-derived-read boundary를 수락해야 합니다. |
| 새로 발견된 owner conflict | 현재 기록된 항목은 없습니다. | Review에서 실제 schema/design, stage-boundary, guarantee-level, fixture-semantics, storage/API conflict가 나오면 owner, stage impact, option, 필요한 결정을 이곳에 추가해야 합니다. |

<a id="mvp-1-담당-문서"></a>
<a id="mvp-1에-필요한-api-문서"></a>
<a id="mvp-1에-필요한-storage-문서"></a>
<a id="mvp-1-security-guarantee"></a>
<a id="reference-owners"></a>
<a id="mvp-1-owner-docs"></a>
<a id="api-docs-needed-for-mvp-1"></a>
<a id="storage-docs-needed-for-mvp-1"></a>
<a id="security-guarantees-for-mvp-1"></a>
## 담당 Reference 문서

Build는 순서와 범위만 요약합니다. 정확한 contract는 아래 담당 Reference 문서를 봅니다.

| 필요 | 담당 문서 |
|---|---|
| 현재 MVP public tool과 resource | [MVP API](../reference/api/mvp-api.md). |
| Shared envelope, ref, staged API value, resource, active schema shape | [API Schema Core](../reference/api/schema-core.md). |
| Public error, idempotency, replay, stale-state, state conflict behavior | [API Errors](../reference/api/errors.md). |
| Task, scope, user judgment, `prepare_write`, Write Authorization, `record_run`, evidence gate, blocker, status, close semantics | [Core Model 참조](../reference/core-model.md). |
| Runtime home layout, minimal storage profile, lock, migration, artifact, later-profile storage boundary | [Storage](../reference/storage.md). |
| MVP security guarantee wording과 local-access posture | [보안 참조](../reference/security.md). |
| Compact derived view, projection authority boundary, freshness, template ownership | [Projection과 Template 참조](../reference/projection-and-templates.md)와 [Template 참조](../reference/templates/README.md). |
| Reference surface `capability_profile`과 사용자용 surface behavior | [Agent 통합 참조](../reference/agent-integration.md)와 [Surface Cookbook](../reference/surface-cookbook.md). |
| Runtime architecture와 local Core placement | [런타임 아키텍처 참조](../reference/runtime-architecture.md). |
| Active design-quality blocking boundary | [설계 품질 정책](../reference/design-quality-policies.md#활성-mvp-차단-집합). |
| Future fixture/conformance와 operations material | [Conformance Fixtures 참조](../reference/conformance-fixtures.md), [향후 Fixtures](../later/future-fixtures.md), [운영과 Conformance 참조](../reference/operations-and-conformance.md). |

<a id="하네스-서버-구현-준비-조건"></a>
<a id="종료-체크리스트"></a>
<a id="implementation-readiness-criteria"></a>
<a id="exit-checklist"></a>
<a id="exit-criteria-for-documentation-planning"></a>
## 문서 계획 종료 기준

문서 계획은 유지보수자가 아래 항목을 명시적으로 확인한 뒤에만 종료할 수 있습니다.

- 이 단일 Build 계획이 active Build entry point이고 예전 Build route가 retired 되었습니다.
- 현재 MVP boundary와 later/profile exclusion이 수락되었거나 남은 boundary issue가 stage impact와 함께 재분류되었습니다.
- 위 서버 코딩 전 결정이 해결되었거나 수락되었거나 named stage impact와 함께 미뤄졌습니다.
- 현재 MVP에 필요한 active API, Core, Storage, Security, projection/template, surface boundary에 대해 담당 Reference 문서가 서로 맞습니다.
- 영어와 한국어 Build 문서가 같은 구현 결정과 현재 MVP 경계를 보존합니다.
- Later/profile material이 현재 MVP requirement처럼 보이지 않습니다.
- 문서는 원천 자료로 남아 있으며 server/runtime code, generated runtime state, executable fixture, conformance result, generated operational artifact, product implementation output을 만들지 않았습니다.

이 문서 계획 기준을 통과해도 하네스가 구현되거나 runtime conformance가 증명되거나 향후 제품 작업이 닫히는 것은 아닙니다.
