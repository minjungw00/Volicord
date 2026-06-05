# Build: MVP-1 사용자 작업 루프

## 이 문서가 도와주는 일

이 문서는 첫 사용자 가치 구현 이정표인 MVP-1 사용자 작업 루프를 계획할 때 사용합니다. 서버 코딩을 아직 막는 구현 결정도 이 문서에 중앙화합니다.

내부 엔지니어링 점검이 먼저이며, 그 점검은 내부 Core 권한 루프를 증명합니다. MVP-1 사용자 작업 루프는 그 뒤에 옵니다. 사용자의 평소 작업을 추적하고, 설명하고, 정직하게 막고, 보이는 권한 경계 안에서 닫거나 보류할 수 있음을 증명합니다.

이 문서는 계획 문서입니다. [구현 개요](implementation-overview.md#문서-수락-상태)의 handoff gate가 수락되기 전에는 런타임/서버 구현, 생성된 운영 산출물, 실행 가능한 fixture 파일, 런타임 데이터, 제품 코드, conformance runner를 승인하지 않습니다.

## 이런 때 읽기

- 내부 checkpoint 범위와 첫 사용자 가치 조각을 구분해야 할 때.
- MVP-1에 무엇이 포함되고 제외되는지 알아야 할 때.
- MVP-1 API, storage, security 담당 문서를 exact contract 중복 없이 찾아야 할 때.
- 서버 코딩 전 중앙 결정 기록이 필요할 때.

## 핵심 생각

MVP-1 사용자 작업 루프의 목표는 다음과 같습니다.

> 사용자가 평소 말로 작업을 시작하거나 이어갈 때, 하네스는 범위, 대기 중인 사용자 판단, 증거 요약, 닫기 차단 사유, 다음 안전한 행동, 최종 수락, 잔여 위험 표시의 로컬 기준을 보존한다.

MVP-1은 일부러 좁습니다. 하네스가 prompt pack이나 pre-write wrapper보다 크다는 점을 보여 주기에는 충분해야 합니다. 하지만 full assurance system, QA matrix, evaluation harness, reporting suite, operations suite, dashboard, hosted UI, connector platform은 아닙니다.

활성 MVP-1 surface target은 `surface_id=reference-local-mcp`인 registered reference `capability_profile` 하나입니다. Capability label은 write authority를 부여하지 않습니다. Unsupported field는 guarantee display를 낮추거나 claim을 막으며, product write에는 계속 active scope, `prepare_write`, durable Write Authorization, `record_run`이 필요합니다.

활성 MVP-1 method set은 정확히 다음 일곱 개입니다.

- `harness.status`
- `harness.intake`
- `harness.request_user_judgment`
- `harness.record_user_judgment`
- `harness.prepare_write`
- `harness.record_run`
- `harness.close_task`

활성 MVP-1에는 `harness.next` method가 없습니다. 다음 안전한 행동은 `harness.status.next_actions`로 반환합니다.

활성 작은 출력 세트는 독자별로 나뉩니다.

- 사용자용: `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`
- 에이전트용: `agent-context-packet`

Persisted Journey Card, full Evidence Manifest, Eval report, Manual QA report, TDD Trace, Module Map, Interface Contract, Export report 같은 상세 report는 owner가 좁은 non-required display로 명시적으로 승격하기 전까지 후속/profile 범위입니다.

## MVP-1에 포함되는 것

MVP-1에는 아래가 포함됩니다.

- 추적할 작업을 평소 말로 시작하거나 이어가기
- 작은 직접 변경과 추적 작업을 구분하는 작업 형태 분류
- 범위, 하지 않을 일, 성공 기준 요약
- 사용자에게 다시 묻기 전에 codebase-answerable 또는 state-answerable fact 확인
- 담당 API 경로를 통한 minimal user judgment request와 record
- 관련 경로가 있을 때 제품 판단, 기술 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락을 분리해서 표시
- Core와 `prepare_write`를 통한 협력형 쓰기 전 범위 확인
- `record_run`, 등록된 아티팩트 참조 또는 evidence ref, minimum evidence summary path
- fallback, blocked reason, validator result, guarantee display에 사용하는 reference `capability_profile` 하나
- 최소 state `not_required`, `none`, `partial`, `sufficient`, `stale`, `blocked`를 쓰는 Core-owned `evidence_summary`
- `harness.status.next_actions`를 통한 status와 next-safe-action output
- status와 `prepare_write` output에 보장 수준을 표시하기. Core가 답할 수 없으면 unavailable/capability 상태를 분명히 보여줍니다.
- 증거 요약과 증거 공백 표시
- 필요한 증거가 부족하거나, 필요한 사용자 판단이 unresolved 또는 blocked이거나, 필요한 최종 수락이 없거나, 잔여 위험이 required 상태로 보이지 않거나 수락되지 않았을 때 close blocker summary
- 닫기와 관련된 위험이 있을 때 최종 수락이나 close 전에 잔여 위험 표시
- 작은 활성 MVP 차단 집합을 통해 라우팅되는 design-quality finding: Autonomy Boundary exceeded, unresolved user judgment, missing active scope, missing required evidence, stale context affecting write/close, surface capability insufficient for a claimed guarantee
- MVP-1 path를 위한 Core 기반 작은 출력. 네 가지 사용자용 출력과 에이전트용 패킷 하나로 나누며, 정확한 세트는 [Projection과 Template 참조](../reference/projection-and-templates.md#mvp-1-보기-세트)와 [Template 참조](../reference/templates/README.md#mvp-1-템플릿-세트)가 담당합니다.
- MCP/Core를 사용할 수 없을 때 정직하게 동작하기. Core에 닿을 수 없으면 권한 상태를 만들어 내지 않습니다.

## MVP-1에서 제외되는 것

MVP-1에는 아래 향후 버킷이 포함되지 않습니다.

| 버킷 | MVP-1 밖에 둘 것 |
|---|---|
| 보증 프로필 | 활성 최소 경로를 넘는 검증 강화, full detached verification, detached Eval system, full Manual QA matrix, detailed Evidence Manifest, detailed Eval output, full waiver machinery, full Approval lifecycle hardening, 풍부한 잔여 위험 lifecycle, 위험 검토 hardening, stewardship validator, full TDD trace, full feedback-loop audit, detailed Manual QA policy, full module/interface and domain-language review, broad context-hygiene validator. |
| 운영 프로필 | Full report/export, recover/export suite, release handoff, artifact integrity operations, projection refresh/reconcile suite, doctor/readiness suite, broad operator surface, runtime conformance suite, conformance runner, generated conformance artifact, executable fixture catalog, Export report. |
| 로드맵 | Dashboard, hosted workflow UI, artifact dashboard, rich card expansion, broader connector, connector marketplace, hosted connector registry, team workflow, parallel orchestration, cross-surface orchestration, metrics, automated Browser QA Capture, Cross-Surface Verification automation, hosted/remote workflow, preventive guard expansion, hook, deployment, canary, rollback, production monitoring, 그 밖의 향후 확장 후보. |
| 보안 non-claim | OS-level sandboxing, arbitrary-tool isolation, permission isolation, tamper-proof local storage, default preventive pre-tool blocking. |

유용하지만 제외 버킷에 있는 기능은 담당 문서가 더 좁은 동작을 stage impact와 함께 명시적으로 승격하기 전까지 [보증 프로필](../later/assurance-profile.md), [운영 프로필](../later/operations-profile.md), [향후 Fixtures](../later/future-fixtures.md), [로드맵](../roadmap.md)에 둡니다.

## MVP-1 담당 문서

Build 문서는 exact schema, DDL, API definition을 중복하지 않습니다. 아래 담당 문서를 사용합니다.

| 필요한 것 | 담당 문서 |
|---|---|
| MVP-1 public tool과 resource | [MVP API](../reference/api/mvp-api.md). |
| Shared envelope, ref, staged API value, resource | [API Schema Core](../reference/api/schema-core.md). |
| Error, idempotency, replay, stale-state, state conflict behavior | [API Errors](../reference/api/errors.md). |
| Task, scope, user judgment, `prepare_write`, Write Authorization, `record_run`, evidence gate, blocker, close semantics | [Core Model 참조](../reference/core-model.md). |
| Runtime home layout, minimal storage profile, lock, migration, artifact, later-profile storage boundary | [Storage](../reference/storage.md). |
| MVP-1 보안 보장 표현과 local-access posture | [보안 참조](../reference/security.md). |
| Compact derived view, projection authority boundary, freshness, template ownership | [Projection과 Template 참조](../reference/projection-and-templates.md), [Template 참조](../reference/templates/README.md). |
| Reference surface `capability_profile`과 user-facing surface behavior | [Agent 통합 참조](../reference/agent-integration.md), [Surface Cookbook](../reference/surface-cookbook.md). |
| Future state-assertion conformance example과 smoke authoring | [Conformance Fixtures 참조](../reference/conformance-fixtures.md). |

## MVP-1에 필요한 API 문서

구현자는 아래 순서로 읽는 것이 좋습니다.

1. [MVP API](../reference/api/mvp-api.md): 활성 MVP-1 public tool과 resource.
2. [API Schema Core](../reference/api/schema-core.md): envelope, `ArtifactRef`, shared ref, staged value set, read-only resource.
3. [API Errors](../reference/api/errors.md): public error, idempotency, replay, unavailable Core/MCP behavior, state conflict.
4. [API Schema Later](../reference/api/schema-later.md): 어떤 method나 field가 later/profile-gated라서 MVP-1 밖에 남아야 하는지 확인할 때만 사용합니다.

MVP-1의 next-safe-action output은 `harness.status.next_actions`로 만족해야 합니다. 별도 `harness.next` method는 담당 문서가 승격하기 전까지 later/compatibility material입니다.

MVP-1의 활성 method list는 계속 정확히 `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, `harness.close_task`입니다.

## MVP-1에 필요한 Storage 문서

활성 첫 구현 저장 범위, runtime home layout, artifact storage와 link, lock, storage validation, later/profile storage boundary는 [Storage](../reference/storage.md)를 사용합니다.

MVP-1 계획에서 storage는 `project_state`, reference `surfaces` registration, `tasks`, `task_events`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, minimal `evidence_summaries`, `blockers`, `tool_invocations`에 필요한 owner-approved active record로 제한합니다. Rich Approval lifecycle, full Evidence Manifest table, full Manual QA matrix, full Eval system, projection job, reconcile item, recover/export, broad validator run archive, Journey record, long-term metrics, connector ecosystem table, broad diagnostic을 위한 later-profile storage는 owner가 특정 동작을 승격하기 전까지 MVP-1 exit에 필요하지 않습니다.

## MVP-1 보안 보장

MVP-1은 cooperative plus limited detective wording을 사용합니다.

할 수 있는 일:

- Harness-compatible product write를 기록하기 전에 Core-compatible record를 요구한다.
- Missing scope, missing judgment, missing evidence, stale state, unavailable Core/MCP, close blocker에는 structured blocker를 반환한다.
- 정직한 guarantee status와 evidence/risk gap을 보여 준다.
- 사용자에게 보이는 status와 쓰기 확인 response에 active guarantee level 또는 분명한 unavailable/capability equivalent를 포함한다.
- Harness record/check path를 사용할 수 없거나 맞지 않으면 연결된 agent나 surface가 지시로 보류하도록 요청한다.
- Reference `capability_profile`에 required capability가 없으면 claim을 막거나 낮춘다. Unsupported surface에서 product write가 조용히 진행되면 안 된다.

주장하면 안 되는 일:

- OS-level permission control
- arbitrary-tool sandboxing
- tamper-proof local file
- default pre-tool blocking
- permission isolation이나 security isolation
- future promoted owner profile이 exact covered operation을 증명하기 전의 preventive 또는 isolated behavior

Guarantee level은 [보안 참조](../reference/security.md#단계별-guarantee-level)를 사용하고, unavailable 또는 mismatch behavior의 사용자 표시 방식은 [API Errors](../reference/api/errors.md)를 사용합니다.

## 서버 코딩 전 필요한 구현 결정

이 섹션은 중앙 서버 코딩 전 결정 기록입니다. Review나 첫 runtime-batch planning에서 발견된 큰 구현 결정은 active docs에 흩어진 open marker가 아니라 이곳에 둡니다.

### 문서 기준에서 해소된 MVP-1 결정

아래 결정은 문서 기준선에서 해소되었습니다. 그래도 코딩 전에는 maintainer acceptance가 필요합니다.

| 결정 | 문서 기준선 | 코딩 경계 |
|---|---|---|
| Judgment naming | `UserJudgment` / `user_judgment`, `harness.request_user_judgment`, `harness.record_user_judgment`, `judgment_kind`, `presentation`을 사용하고, 사용자 표시 라벨은 `judgment_kind`와 locale에서 렌더링합니다. | Compatibility alias나 표시 라벨이 추가 authority path를 만들면 안 됩니다. |
| Next action | MVP-1 next-safe-action output은 `harness.status.next_actions`를 사용합니다. | 별도 `harness.next` method는 승격 전까지 later/compatibility입니다. |
| Reference surface scope | `surface_id=reference-local-mcp`인 reference `capability_profile` 하나를 사용합니다. | Broad connector ecosystem, hosted connector registry, cross-surface orchestration은 명시적으로 승격되기 전까지 later/profile입니다. |
| MVP-1 작은 출력 | [Projection과 Template 참조](../reference/projection-and-templates.md#mvp-1-보기-세트)와 [Template 참조](../reference/templates/README.md#mvp-1-템플릿-세트)가 소유한 네 가지 사용자용 출력 `status-card`, `judgment-request`, `run-evidence-summary`, `close-result`와 에이전트용 패킷 `agent-context-packet`만 사용합니다. | 이 출력은 쓰기를 승인하거나 증거를 충족하거나 수락을 기록하거나 위험을 수용하거나 Task를 닫거나 기준 상태가 되지 않습니다. |
| Minimal storage boundary | MVP-1 storage는 user work loop에 필요한 최소 active owner record로 제한합니다. | Later-profile table/record는 owner docs가 승격하기 전까지 제외합니다. |
| Acceptance boundaries | Sensitive action approval, final acceptance, residual-risk acceptance를 분리합니다. | Final acceptance는 Approval이 아니며, residual-risk acceptance는 final acceptance가 아닙니다. |
| Minimal evidence and close contract | Core-owned `evidence_summary`를 사용합니다. Successful close에는 필요한 evidence가 sufficient이고, 필요한 judgment가 resolved이며, 필요한 최종 수락이 기록되어 있고, close-relevant residual risk가 visible해야 합니다. Accepted-risk close에는 명시적 residual-risk acceptance가 필요합니다. | Full Evidence Manifest, detached Eval, full Manual QA, rich residual-risk lifecycle은 owner scope, policy, profile이 활성화하기 전까지 later/profile에 남습니다. |
| Active close assurance boundary | MVP-1 close는 `assurance_level=none` 또는 `self_checked`와 `completed_self_checked`, `completed_with_risk_accepted`, `cancelled`, `superseded` close reason을 사용합니다. | `completed_verified`, `assurance_level=detached_verified`, `profile_required_verification`, verification close blocker, Manual QA close blocker는 later/profile에 남습니다. |
| Design-quality MVP boundary | [설계 품질 정책: 활성 MVP 차단 집합](../reference/design-quality-policies.md#활성-mvp-차단-집합)을 사용합니다. | Full domain language consistency, full module/interface review, full TDD trace, full codebase stewardship suite, full feedback-loop audit, detailed Manual QA policy, detached verification profile은 기본적으로 Routed candidate 또는 Advisory/later입니다. |
| Small direct changes | Small change도 explicit scope, compatible `prepare_write`, `record_run`, required evidence support가 필요합니다. | Small-change label이 authority, user judgment, evidence, risk visibility를 우회하면 안 됩니다. |
| Local access and errors | Local access, unavailable Core/MCP, state conflict, display-safe detail은 API, Operations, Security 담당 계약을 사용합니다. | Build 문서는 새 public error code나 precedence를 정의하지 않습니다. |

### 아직 열려 있는 구현 결정

| 결정 항목 | 현재 상태 | Readiness를 막는 것 |
|---|---|---|
| 구현 준비 판단 | 수락되지 않았습니다. | Readiness criteria가 충족되거나 재분류된 뒤 maintainer가 [구현 개요: 문서 수락 상태](implementation-overview.md#문서-수락-상태)를 갱신해야 합니다. |
| Public API coding acceptance | 코드 작성용으로 수락되지 않았습니다. | 영향을 받는 tool/resource를 코딩하기 전 maintainer가 active MVP-1 surface와 later/profile exclusion을 포함한 API 담당 문서를 수락해야 합니다. |
| Storage/DDL coding acceptance | 코드 작성용으로 수락되지 않았습니다. | DDL이나 runtime data file을 만들기 전 maintainer가 Storage owner profile과 migration을 수락해야 합니다. |
| Core transition acceptance | 코드 작성용으로 수락되지 않았습니다. | 영향을 받는 path를 코딩하기 전 maintainer가 active Core state transition, blocker semantics, close/status behavior를 수락해야 합니다. |
| Security/local-access acceptance | 코드 작성용으로 수락되지 않았습니다. | API/MCP surface를 노출하기 전 maintainer가 local-only posture와 cooperative/detective guarantee wording을 수락해야 합니다. |
| 새 owner conflict | 현재 기록된 항목은 없습니다. | Review에서 schema/design, stage-boundary, guarantee-level, fixture-semantics, storage/API conflict가 실제로 발견되면 owner, stage impact, options, coding 전 필요한 decision을 이곳에 추가합니다. |

결정을 추가할 때는 owner document, affected behavior or field, affected stage, options considered, decision needed, 그리고 documentation acceptance, implementation planning, server coding, later stage 중 무엇을 막는지 적습니다.

## 아직 만들지 않을 이후 프로필

아래 항목은 MVP-1 선결 조건으로 만들지 않습니다.

| 이후 영역 | MVP-1 밖에 둘 것 |
|---|---|
| [보증 프로필](../later/assurance-profile.md) | 검증 강화, detailed Manual QA, detailed evidence, risk review, detailed evaluation output, full Approval lifecycle, stewardship validator, full TDD trace, full feedback-loop audit, full module/interface and domain-language review, stale write/close context를 넘는 context-hygiene validator. |
| [운영 프로필](../later/operations-profile.md) | Export, recovery, handoff, operator readiness, doctor/readiness surface, artifact integrity operations, projection refresh/reconcile operations, conformance runner, broad operator surface. |
| [로드맵](../roadmap.md) | Dashboard, hosted workflow, team workflow, broader connector, hosted connector registry, cross-surface orchestration, automated Browser QA Capture, Cross-Surface Verification, Context Index, metrics, preventive guard expansion, hook, permission, parallel orchestration, deployment, canary, rollback, production monitoring, 그 밖의 확장 향후 후보. |

## 종료 점검

MVP-1 사용자 작업 루프는 사용자가 아래를 볼 수 있을 때만 complete로 볼 수 있습니다.

- Harness internal label을 몰라도 평소 작업을 시작하거나 이어갈 수 있음
- 범위, 하지 않을 일, 성공 기준, work shape
- 필요할 때 choices와 consequences를 포함하는 pending user judgment
- 제품/기술 판단, 민감 동작 승인, 최종 수락, 잔여 위험 수락의 분리 표시
- Core를 통한 compatible 쓰기 전 범위 확인
- Recorded Run과 evidence ref 또는 evidence summary
- 현재 상태, 다음 안전한 행동, 증거 공백, close blocker, 잔여 위험 표시
- 현재 status 또는 쓰기 확인 result에 보장 수준이나 unavailable/capability 상태가 표시됨
- Unsupported behavior에 claim이 의존할 때 reference `capability_profile` limit이 보임
- Required evidence가 `sufficient`가 아니거나, required user judgment가 unresolved 또는 blocked이거나, required final acceptance가 없거나, residual risk가 required 상태로 보이지 않거나 수락되지 않으면 close가 hold됨
- MCP/Core를 사용할 수 없을 때 권한 상태를 만들어 내지 않음
- Core record에서 파생된 네 가지 사용자용 작은 출력과 에이전트용 패킷 하나, 필요한 경우 stale/failed freshness 표시. 이 출력은 conformance proof가 아님

이 checklist 통과는 보증 프로필, 운영 프로필, 로드맵 범위, runtime conformance suite를 수락한다는 뜻이 아닙니다.
