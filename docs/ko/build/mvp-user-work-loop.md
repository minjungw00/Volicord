# Build: MVP-1 사용자 작업 루프

## 이 문서가 도와주는 일

이 문서는 첫 사용자 가치 구현 이정표인 MVP-1 사용자 작업 루프를 계획할 때 사용합니다. 서버 코딩을 아직 막는 구현 결정도 이 문서에 중앙화합니다.

내부 엔지니어링 점검이 먼저이며, 그 점검은 내부 Core 권한 루프를 증명합니다. MVP-1 사용자 작업 루프는 그 뒤에 옵니다. 사용자의 평소 작업을 추적하고, 설명하고, 정직하게 막고, 보이는 권한 경계 안에서 닫거나 보류할 수 있음을 증명합니다.

이 문서는 계획 문서입니다. [구현 개요](implementation-overview.md#문서-수락-상태)의 handoff gate가 수락되기 전에는 런타임/서버 구현, 생성된 운영 산출물, 실행 가능한 fixture 파일, 런타임 데이터, 제품 코드, conformance runner를 허가하지 않습니다.

## 이런 때 읽기

- 내부 checkpoint 범위와 첫 사용자 가치 조각을 구분해야 할 때.
- MVP-1에 무엇이 포함되고 제외되는지 알아야 할 때.
- MVP-1 API, storage, security 담당 문서를 exact contract 중복 없이 찾아야 할 때.
- 서버 코딩 전 중앙 결정 기록이 필요할 때.

## 핵심 생각

MVP-1 사용자 작업 루프의 목표는 다음과 같습니다.

> 사용자가 평소 말로 작업을 시작하거나 이어갈 때, 하네스는 범위, 대기 중인 사용자 판단, 근거 요약, 닫기 막힘, 다음 안전한 행동, 작업 수락, 잔여 위험 표시의 로컬 근거를 보존한다.

MVP-1은 일부러 좁습니다. 하네스가 prompt pack이나 pre-write wrapper보다 크다는 점을 보여 주기에는 충분해야 합니다. 하지만 full assurance system, QA matrix, evaluation harness, reporting suite, operations suite, dashboard, hosted UI, connector platform은 아닙니다.

## MVP-1에 포함되는 것

MVP-1에는 아래가 포함됩니다.

- 추적할 작업을 평소 말로 시작하거나 이어가기
- 작은 직접 변경과 추적 작업을 구분하는 작업 형태 분류
- 범위, 하지 않을 일, 성공 기준 요약
- 사용자에게 다시 묻기 전에 codebase-answerable 또는 state-answerable fact 확인
- 담당 API 경로를 통한 minimal user judgment request와 record
- 관련 경로가 있을 때 제품/UX 판단, 기술 판단, 민감 동작 승인, 작업 수락, 잔여 위험 수용을 분리해서 표시
- Core와 `prepare_write`를 통한 협력형 쓰기 전 범위 확인
- `record_run`과 registered artifact/evidence ref 또는 minimum evidence summary path
- `status`와 next-safe-action output
- 근거 요약과 근거 gap 표시
- 필요한 근거나 사용자 판단이 없을 때 close blocker summary
- 닫기와 관련된 위험이 있을 때 작업 수락이나 close 전에 잔여 위험 표시
- MVP-1 path를 위한 compact Core-derived view. 정확한 view set은 [Projection과 Template 참조](../reference/projection-and-templates.md#mvp-1-보기-세트)와 [Template 참조](../reference/templates/README.md#mvp-1-템플릿-세트)가 담당합니다.
- MCP/Core를 사용할 수 없을 때 정직하게 동작하기. Core에 닿을 수 없으면 권한 상태를 만들어 내지 않습니다.

## MVP-1에서 제외되는 것

MVP-1에는 아래 향후 버킷이 포함되지 않습니다.

| 버킷 | MVP-1 밖에 둘 것 |
|---|---|
| 보증 프로필 | 활성 최소 경로를 넘는 검증 강화, full detached verification, full Manual QA matrix, detailed Evidence Manifest, detailed Eval output, full waiver machinery, full Approval lifecycle hardening, 풍부한 잔여 위험 lifecycle, 위험 검토 hardening, stewardship validator, TDD trace, feedback-loop policy, broad context-hygiene validator. |
| 운영 프로필 | Full report/export, recover/export, release handoff, artifact integrity operations, projection refresh/reconcile suite, doctor/readiness suite, broad operator surface, runtime conformance suite, conformance runner, generated conformance artifact, executable fixture catalog. |
| 로드맵 | Dashboard, hosted workflow UI, artifact dashboard, rich card expansion, broader connector, connector marketplace, team workflow, orchestration, metrics, Browser QA Capture, Cross-Surface Verification automation, hosted/remote workflow, preventive guard expansion, hook, deployment, canary, rollback, production monitoring, 그 밖의 향후 확장 후보. |
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
| Connector capability profile과 user-facing surface behavior | [Agent 통합 참조](../reference/agent-integration.md), [Surface Cookbook](../reference/surface-cookbook.md). |
| Future conformance model과 future smoke authoring | [Conformance Fixtures 참조](../reference/conformance-fixtures.md). |

## MVP-1에 필요한 API 문서

구현자는 아래 순서로 읽는 것이 좋습니다.

1. [MVP API](../reference/api/mvp-api.md): 활성 MVP-1 public tool과 resource.
2. [API Schema Core](../reference/api/schema-core.md): envelope, `ArtifactRef`, shared ref, staged value set, read-only resource.
3. [API Errors](../reference/api/errors.md): public error, idempotency, replay, unavailable Core/MCP behavior, state conflict.
4. [API Schema Later](../reference/api/schema-later.md): 어떤 method나 field가 later/profile-gated라서 MVP-1 밖에 남아야 하는지 확인할 때만 사용합니다.

MVP-1의 next-safe-action output은 `harness.status.next_actions`로 만족해야 합니다. 별도 `harness.next` method는 담당 문서가 승격하기 전까지 later/compatibility material입니다.

## MVP-1에 필요한 Storage 문서

Exact DDL, runtime home layout, artifact storage, lock, migration, staged storage profile은 [Storage](../reference/storage.md)를 사용합니다.

MVP-1 계획에서 storage는 local project state, Task/scope state, user judgment, write authorization, run, evidence ref 또는 evidence summary, blocker, 최소 replay/audit support에 필요한 owner-approved record로 제한합니다. Rich Approval lifecycle, detailed Evidence Manifest, Manual QA, Eval, projection job, reconcile item, recover/export, validator run, Journey record, broad diagnostic을 위한 later-profile storage는 owner가 특정 동작을 승격하기 전까지 MVP-1 exit에 필요하지 않습니다.

## MVP-1 보안 보장

MVP-1은 cooperative plus limited detective wording을 사용합니다.

할 수 있는 일:

- Harness-compatible product write를 기록하기 전에 Core-compatible record를 요구한다.
- Missing scope, missing judgment, missing evidence, stale state, unavailable Core/MCP, close blocker에는 structured blocker를 반환한다.
- 정직한 guarantee status와 evidence/risk gap을 보여 준다.
- Harness record/check path를 사용할 수 없거나 맞지 않으면 연결된 agent나 surface가 지시로 보류하도록 요청한다.

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
| Judgment naming | `UserJudgment` / `user_judgment`, `harness.request_user_judgment`, `harness.record_user_judgment`, `judgment_type`, `presentation`, `display_label`을 사용합니다. | Compatibility alias가 추가 authority path를 만들면 안 됩니다. |
| Next action | MVP-1 next-safe-action output은 `harness.status.next_actions`를 사용합니다. | 별도 `harness.next` method는 승격 전까지 later/compatibility입니다. |
| MVP-1 compact views | [Projection과 Template 참조](../reference/projection-and-templates.md#mvp-1-보기-세트)와 [Template 참조](../reference/templates/README.md#mvp-1-템플릿-세트)가 소유한 compact Core-derived view set을 사용합니다. | 이 view는 쓰기를 허가하거나 근거를 충족하거나 수락을 기록하거나 위험을 수용하거나 task를 close하거나 canonical state가 되지 않습니다. |
| Minimal storage boundary | MVP-1 storage는 user work loop에 필요한 최소 active owner record로 제한합니다. | Later-profile table/record는 owner docs가 승격하기 전까지 제외합니다. |
| Acceptance boundaries | Sensitive action approval, work acceptance, residual-risk acceptance를 분리합니다. | Work acceptance는 Approval이 아니며, residual-risk acceptance는 work acceptance가 아닙니다. |
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

아래 항목은 MVP-1 prerequisite로 만들지 않습니다.

| 이후 영역 | MVP-1 밖에 둘 것 |
|---|---|
| [보증 프로필](../later/assurance-profile.md) | 검증 강화, 수동 QA, 상세 근거, 위험 검토, 상세 평가 출력, full Approval lifecycle, stewardship validator, TDD trace, feedback-loop policy, context-hygiene validator. |
| [운영 프로필](../later/operations-profile.md) | Export, recovery, handoff, operator readiness, doctor/readiness surface, artifact integrity operations, projection refresh/reconcile operations, conformance runner, broad operator surface. |
| [로드맵](../roadmap.md) | Dashboard, hosted workflow, team workflow, broader connector, Browser QA Capture, Cross-Surface Verification, Context Index, metrics, preventive guard expansion, hook, permission, orchestration, deployment, canary, rollback, production monitoring, 그 밖의 확장 향후 후보. |

## 종료 점검

MVP-1 사용자 작업 루프는 사용자가 아래를 볼 수 있을 때만 complete로 볼 수 있습니다.

- Harness internal label을 몰라도 평소 작업을 시작하거나 이어갈 수 있음
- 범위, 하지 않을 일, 성공 기준, work shape
- 필요할 때 choices와 consequences를 포함하는 pending user judgment
- 제품/기술 판단, 민감 동작 승인, 작업 수락, 잔여 위험 수용의 분리 표시
- Core를 통한 compatible 쓰기 전 범위 확인
- Recorded Run과 evidence ref 또는 evidence summary
- 현재 상태, 다음 안전한 행동, 근거 공백, close blocker, 잔여 위험 표시
- Required evidence나 required user judgment가 없으면 close가 hold됨
- MCP/Core를 사용할 수 없을 때 권한 상태를 만들어 내지 않음
- Core record에서 파생된 compact view와, 필요한 경우 stale/failed freshness 표시

이 checklist 통과는 보증 프로필, 운영 프로필, 로드맵 범위, runtime conformance suite를 수락한다는 뜻이 아닙니다.
