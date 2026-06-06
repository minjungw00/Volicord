# Conformance Fixtures 참조

## 이 문서로 할 수 있는 일

하네스 conformance 자료는 문서 점검, 구조화된 활성 fixture 초안, 향후 runtime conformance라는 세 층으로 나누어 봐야 합니다. 이 참조 문서는 향후 conformance가 무엇을 증명하는지, 활성 Kernel Smoke와 MVP-1 user-loop, artifact/evidence 초안 묶음, canonical active fixture value 규칙, exact structured fixture draft shape, 향후 runner 실행 동작, fixture assertion semantics, 현재 단계 상태, 향후 fixture catalog와의 경계를 설명합니다.

이 문서는 conformance 작성자, 구현자, 유지보수 담당자를 위한 조회용 문서입니다. 운영자 절차 문서가 아니므로 운영자 entrypoint와 `harness conformance run` 개요는 [운영과 Conformance 참조](operations-and-conformance.md)를 사용합니다.

이 문서는 향후 conformance 작업을 위한 참조 문서입니다. 현재 저장소는 문서 전용이며 실행 가능한 하네스 서버 conformance test를 담고 있지 않습니다. 현재 단계와 인계 상태는 [MVP 계획](../build/mvp-plan.md#문서-수락-상태)에 있습니다.

## 이런 때 읽기

- 향후 fixture 기반 conformance 설계를 작성하거나 리뷰할 때.
- 정확한 fixture body field, canonical active value boundary, `request.payload` public-schema rule, 향후 runner isolation behavior를 확인해야 할 때.
- response fact, Core state, storage row, event, artifact, blocker, error, forbidden side effect, 그리고 승격된 경우 projection fact에 적용되는 fixture assertion mode가 필요할 때.
- 활성 Kernel Smoke, MVP-1 사용자 작업 루프, artifact/evidence fixture draft, 또는 이 draft와 향후 fixture catalog 사이의 경계를 확인해야 할 때.

## 읽기 전에

`harness conformance run` entrypoint, suite-selection overview, docs-maintenance profile boundary, 운영자 절차는 [운영과 Conformance 참조](operations-and-conformance.md#conformance-run)에서 확인합니다. Public request/response schema는 [MVP API](api/mvp-api.md)와 [API Schema Core](api/schema-core.md)를, storage layout과 seed-loader owner value는 [Storage](storage.md)를 봅니다. State transition과 stable event의 의미는 [Core Model 참조](core-model.md)를, projection freshness는 [Projection과 Template 참조](projection-and-templates.md)를, policy validator behavior는 [설계 품질 정책](design-quality-policies.md)을, connector conformance overview는 [Agent 통합 참조](agent-integration.md)를 사용합니다.

## 핵심 생각

현재 이 문서는 실행 가능한 테스트 모음이 아니라 향후 runtime conformance 설계입니다. 이후 구현 계획에서 사용할 동작 예시 ID와 필요한 동작을 정의할 뿐이며, fixture file, runner code, generated output, runtime state, 실행 가능한 하네스 서버 conformance suite를 만들지 않습니다. 문서 전용 단계에서는 이 예시를 바탕으로 실제 fixture 파일을 만들지 않습니다.

세 층을 항상 구분합니다.

- 문서 점검은 Markdown 문서에 대한 읽기 전용 편집 점검입니다. Link integrity, terminology consistency, stage boundary, security wording, user-language check, owner-boundary drift, 영어/한국어 의미 일치를 봅니다. Markdown drift를 보고할 수는 있지만 fixture action을 실행하거나, `task_events`를 append하거나, artifact를 만들거나, projection을 refresh하거나, QA 또는 acceptance state를 만들거나, close readiness에 영향을 주거나, implementation readiness 또는 runtime result를 만들지 않습니다.
- Active MVP fixture draft는 내부 엔지니어링 점검과 MVP-1을 위한 작은 structured 설계 초안입니다. Assertion field로 기대 동작을 설명하지만 아직 실행 가능한 fixture가 아니며 generated runtime artifact도 아닙니다.
- runtime conformance는 향후 하네스 서버 구현 작업입니다. 구현된 Core/API/storage/surface behavior에 적용되며, documentation prose가 아니라 실행 가능한 fixture와 structured assertion으로 판단합니다. Server implementation과 fixture materialization이 있은 뒤에만 exact-shape fixture가 Core 또는 operator entrypoint를 실행하고 runtime pass/fail result를 만듭니다.
- Active MVP fixture 본문에는 public API, schema, Core, storage, error owner 문서와 같은 기준 활성 값을 사용합니다. `fixture-only shorthand`, fixture-local enum value, pseudo-field, 상태값으로 쓰는 표시 라벨, later/profile-only value를 쓰면 안 됩니다.

핵심 모델과 작은 active MVP fixture draft는 이 파일에 둡니다. 자세한 later scenario는 [향후 Fixtures](../later/future-fixtures.md)에 둡니다. 이렇게 하면 내부 엔지니어링 점검 Kernel Smoke와 MVP-1 사용자 가치를 설명하면서도 later catalog coverage를 early implementation requirement로 오해하지 않게 됩니다.

구현이 시작된 뒤 conformance는 실행 가능한 fixture로 하네스 동작을 증명합니다. Runtime fixture가 pass하려면 Core 또는 operator request를 실행하고 captured response fact, Core state, storage row, event, artifact, blocker, error, forbidden side effect를 structured expectation과 비교해야 합니다.

단언(assertion)의 권한은 층위가 있습니다.

- Prose scenario description, comment, rendered Markdown, Journey Card prose, status text, close report prose, agent summary는 설명일 뿐 권한이 아닙니다.
- Captured response fact, Core state, storage row, `task_events`, validator result, returned primary error, structured blocker field, forbidden-side-effect check는 fixture pass/fail을 판단하는 권위 있는 단언입니다.
- Artifact ref, owner link, `sha256`, `size_bytes`, `content_type`, `redaction_state`, relation owner, retention, availability, file-integrity 단언은 scenario가 artifact 또는 증거 바이트에 의존할 때 권위 있는 단언입니다.
- Projection output으로는 projection support가 범위에 있을 때 freshness, source-state-version 표시, readability, availability를 확인할 수 있습니다. 하지만 renderer output이 Core state를 대체하거나, evidence를 충족하거나, write를 authorize하거나, close를 수행하거나, final acceptance 또는 residual-risk acceptance를 만들거나, conformance truth의 source가 되면 안 됩니다. 내부 엔지니어링 점검은 empty 또는 "no projection requirement" field를 넘는 projection assertion을 요구하지 않습니다.

## 참조 범위

이 문서는 다음 항목을 담당합니다.

- conformance fixture body shape
- active 내부 엔지니어링 점검 / MVP-1 path의 canonical active value boundary
- active fixture body에서 `request.payload`에 적용되는 public-schema requirement
- 테스트 위생을 위한 isolated fixture execution behavior. 이는 `isolated` 보안 보장이 아닙니다.
- fixture assertion semantics와 comparison mode
- suite catalog metadata boundary
- 검증 프로파일별 증명 동작, 축소된 내부 엔지니어링 점검 / MVP-1 structured draft, 축소된 Kernel Smoke 작성 순서
- 현재 단계 상태와 runtime conformance/docs-maintenance check 사이의 경계
- 향후 catalog scenario를 내부 엔지니어링 점검 또는 MVP-1 requirement로 만들지 않는 link boundary

## 여기서 다루지 않는 것

이 참조 문서는 operator command procedure, docs-maintenance reporting, public MCP schema, SQLite DDL, projection template body, policy contract, 간결한 향후 scenario 목록을 담당하지 않습니다. 그 내용은 각 owner Reference 문서에 남습니다. 여기의 suite metadata, example, catalog row는 fixture-body field, public request field, storage row, projection kind, runtime implementation readiness를 추가하지 않습니다.

## Conformance 탐색 지도

| 찾는 것 | 볼 곳 |
|---|---|
| 정확한 fixture body field | [Conformance Fixture Format](#conformance-fixture-format) |
| 향후 runner가 load, seed, execute, capture, compare하는 방식 | [Conformance Execution](#conformance-execution) |
| `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, `forbidden_side_effects`에 적용되는 기본 comparison mode | [Fixture Assertion Semantics](#fixture-assertion-semantics) |
| 활성 structured fixture draft | [Active Structured Fixture Drafts](#engineering-checkpoint-behavior-examples) |
| Suite intent와 작성 순서 | [Conformance staging](operations-and-conformance.md#conformance-staging), [Kernel Smoke Authoring Queue](#kernel-smoke-authoring-queue), [향후 Fixtures: Fixture Suites](../later/future-fixtures.md#fixture-suites) |
| 핵심 모델과 현재 단계 경계 | [핵심 적합성 모델](#핵심-적합성-모델)과 [Fixture 현재 단계 상태](#fixture-현재-단계-상태) |
| concern별 향후 scenario 목록 | [향후 Fixtures](../later/future-fixtures.md) |

## 핵심 적합성 모델

핵심 적합성 모델은 향후 runtime conformance가 무엇을 증명하고 assertion authority가 어디에 있는지 정의합니다. Fixture가 pass하려면 하나의 Core 또는 operator request를 실행하고 captured response fact, Core state, storage row, event, artifact, blocker, error, forbidden side effect를 fixture expectation과 비교해 동작을 증명해야 합니다. Prose, 생성된 Markdown, Journey Card text, status prose, close prose, agent summary를 맞추는 것만으로는 동작을 증명하지 않습니다.

Assertion type은 의도적으로 작게 유지합니다.

- State와 storage assertion은 Core-owned record, storage row effect, `task_events`, validator result, returned primary error, structured blocker, owner ref, state-version behavior를 비교합니다.
- Artifact assertion은 scenario가 증거 바이트에 의존할 때 등록된 아티팩트 식별 정보, owner link, `sha256`, `size_bytes`, `content_type`, `redaction_state`, relation owner, retention class, availability, file-integrity fact를 비교합니다.
- Projection assertion은 projection support가 범위에 있을 때만 freshness, enqueue 또는 job status, source-state-version display, readability, availability를 비교합니다. Core state를 대체하거나 authority, evidence, close, final acceptance, residual-risk acceptance를 충족하지 않습니다.
- Error assertion은 public schema precedence에 따른 API-owned primary `ErrorCode`와 optional details를 비교합니다.

State와 storage assertion은 "request 이후 Core가 무엇을 소유했고 어떤 durable row effect가 발생했는가?"에 답합니다. Artifact assertion은 "어떤 증거 바이트 또는 metadata가 안전하게 등록되고 link되었는가?"에 답합니다. Projection assertion은 "derived readable view가 current, stale, available, failed, queued 중 무엇인가?"에 답합니다. 이 assertion 위치들은 서로 분리되어 있으며 projection output이 state나 artifact proof를 대신하면 안 됩니다.

## 검증 프로파일별 증명 동작

검증 프로파일은 rendered output의 완성도가 아니라 무엇을 증명하는지에 따라 묶습니다. Profile 이름은 fixture-body field를 추가하지 않고, renderer를 권위 있게 만들지 않으며, 현재 문서 전용 저장소에 fixture file이 존재한다는 뜻도 아닙니다.

강화된 로컬 기준 목표(hardened local reference target)는 보증 프로필과 운영 프로필을 통해 도달하는 종합 목표입니다. 다섯 번째 fixture profile로 취급하거나 suite name으로 쓰면 안 됩니다.

| 프로파일 | 단계 이름 | 증명하는 동작 | 해당 프로파일 밖의 범위 |
|---|---|---|---|
| 내부 엔지니어링 점검 fixtures, 작성 label은 Kernel Smoke | 내부 엔지니어링 점검 | 첫 실행 가능한 권한 루프를 증명합니다. No-active-Task status, owner-valid setup/intake가 active Task 하나를 만드는 동작, active Change Unit requirement, in-scope/out-of-scope `prepare_write`, dry-run과 replay, single-use Write Authorization, `record_run` consumption과 invalid-authorization blocker, 최소 artifact metadata, evidence summary, close blocker, residual-risk visibility, 정직한 cooperative/detective guarantee display가 포함됩니다. | Ordinary natural-language intake 품질, full user-loop judgment UX, full Evidence Manifest, projection renderer support, final-acceptance 또는 residual-risk acceptance 성공 의미, later/profile 보증 확인, export/recover, release handoff, full conformance runner, broad future catalog coverage, hosted connector registry, cross-surface orchestration, preventive guard expansion, broad operations. |
| MVP-1 사용자 작업 루프 fixtures | MVP-1 사용자 작업 루프 | 평소 요청이 Harness vocabulary 없이 tracked work가 되는지, focused user judgment와 status next safe action이 보이는지, broad approval text, 민감 동작 승인, final acceptance, residual-risk acceptance, evidence가 서로를 대신하지 않는 경계가 유지되는지, active MVP가 later/profile 보증 상태를 만들어내지 않는다는 증명이 Core state와 structured response에 드러나는지 확인합니다. | Full agency assurance hardening 세부 사항, stewardship policy suite, full TDD/module/interface/domain-language catalog, full feedback-loop audit, export/recover, release handoff, broad connector ecosystem, hosted connector registry, cross-surface orchestration, MVP-1 사용자 가치 경로 밖의 automation. |
| 보증 프로필 fixtures | 보증 프로필 | User-owned judgment, 민감 동작 승인(Approval), Write Authorization, 수동 QA, verification, 최종 수락, 잔여 위험 수락, stewardship, design-quality, context-hygiene, TDD, feedback-loop boundary가 Core record를 통해 분리되어 있고 fixture로 증명된 상태임을 확인합니다. | Operator recovery/export completeness, release handoff, broad operations coverage, dashboard/hosted workflow UI, broad connector automation, 증명되지 않은 preventive 또는 isolated guarantee claim. |
| 운영 프로필 / 승격된 로드맵 fixtures | 운영 프로필 및 로드맵 | Export/recover, artifact integrity, release handoff, operator readiness, reconcile, broader conformance coverage, 향후 승격된 더 높은 guarantee level 또는 automation profile을 증명합니다. | Owner 문서가 mechanism을 정의하고 fixture가 covered behavior를 증명하기 전의 stronger security, isolation, preventive guard, browser-capture, remote/shared MCP, automation claim. |

## 활성 MVP Fixture 초안 묶음

이 초안 묶음은 내부 엔지니어링 점검과 MVP-1을 위한 활성 향후 작성 target입니다. 아직 실행 가능한 fixture가 아니며 generated runtime artifact도 아니고 현재 pass/fail 기준도 아닙니다. 아래 structured draft body는 활성 `scenario_id`, 증명 의도, public request owner, 예상 Core/storage effect, owner link를 보존합니다. 향후 구현에서 명시적으로 materialize하기 전까지는 문서 초안입니다.

### Canonical Active Fixture Values

Active MVP fixture 본문에는 public owner 문서와 같은 기준 활성 값을 사용합니다. `fixture-only shorthand`, alternate enum value, compact pseudo-field, 상태값으로 쓰는 표시 라벨, pseudo event name, pseudo storage row, later/profile-only value를 만들면 안 됩니다. 그래야 향후 runner가 별도 fixture dialect 없이 public contract로 fixture를 검증할 수 있습니다.

#### Active Fixture Value Owners

Conformance fixture draft는 active contract를 사용하며, 이 문서는 active contract를 다시 정의하지 않습니다. 아래 표는 영어와 한국어 문서 tree에서 fixture value 영역을 owner 문서에 연결합니다. Active fixture draft는 enum value, table shape, request field, blocker category, error code를 만들면 안 됩니다. Fixture가 새 value를 필요로 하는 것처럼 보이면 먼저 owner document를 명확히 해야 하며, fixture document에서 이를 암묵적으로 만들면 안 됩니다. Later/profile-only fixture material은 active MVP fixture set 밖에 둡니다.

| Fixture value 영역 | Active owner contract | Fixture 작성 규칙 |
|---|---|---|
| API request shape | [MVP API](api/mvp-api.md) (`docs/*/reference/api/mvp-api.md`) | `request.tool`과 `request.payload`는 public method request shape를 사용합니다. Fixture-only request field를 추가하지 않습니다. |
| Active schema values | [API Schema Core](api/schema-core.md) (`docs/*/reference/api/schema-core.md`) | Active enum value, shared ref, response field, schema-owned value set은 active schema owner에서 가져옵니다. |
| Core lifecycle and state transitions | [Core Model 참조](core-model.md) (`docs/*/reference/core-model.md`) | `lifecycle_phase`, gate effect, Core-owned state change, transition outcome은 Core owner value를 사용합니다. |
| Storage row shape | [Storage](storage.md) (`docs/*/reference/storage.md`) | Table, column, JSON `TEXT` shape, row effect, storage hardening value는 Storage에서 가져옵니다. |
| Error codes | [API Errors](api/errors.md) (`docs/*/reference/api/errors.md`) | `ErrorCode` value, primary-error precedence, error detail은 API error owner를 따릅니다. |
| Blocker categories | [API Schema Core](api/schema-core.md) (`docs/*/reference/api/schema-core.md`)와 [Core Model 참조](core-model.md) (`docs/*/reference/core-model.md`) | Blocker category, `required_judgment_kind`, related ref, owner-state blocker fact는 schema와 Core owner value를 사용합니다. |
| Close semantics | [MVP API](api/mvp-api.md) (`docs/*/reference/api/mvp-api.md`)와 [Core Model 참조](core-model.md) (`docs/*/reference/core-model.md`) | `close_task` request/response shape와 close state effect는 API와 Core owner를 따릅니다. Fixture-local close state를 만들지 않습니다. |
| Artifact and evidence summary shape | [API Schema Core](api/schema-core.md) (`docs/*/reference/api/schema-core.md`)와 [Storage](storage.md) (`docs/*/reference/storage.md`) | `ArtifactRef`, `ArtifactInput`, artifact relation value, evidence-summary row 또는 JSON shape는 schema와 Storage owner value를 사용합니다. |
| Later/profile-only fixture material | [API Schema Later](api/schema-later.md) (`docs/*/reference/api/schema-later.md`)와 [향후 Fixtures](../later/future-fixtures.md) 같은 later docs | Later/profile-only value, method, ref, fixture branch, catalog material은 owner가 승격하기 전까지 active MVP fixture body 밖에 둡니다. |

내부 엔지니어링 점검과 MVP-1의 활성 fixture 본문에는 다음 규칙이 적용됩니다.

- `request.payload`는 `request.tool`에 해당하는 public request object여야 합니다. [MVP API](api/mvp-api.md)와 [API Schema Core](api/schema-core.md)의 method request schema가 요구하는 `envelope: ToolEnvelope`와 모든 required field를 포함해야 합니다. 요약하면 `request.payload`는 해당 public method request schema와 일치해야 하며, fixture를 위한 더 좁거나 느슨한 payload dialect는 따로 두지 않습니다. Suite metadata는 작성자가 deterministic envelope value를 고르는 데 도움을 줄 수는 있습니다. 하지만 materialized active fixture 본문은 validation, canonical request hashing, Core execution 전에 public request를 body 안에 확장해 담아야 합니다.
- `expected_state_changes`는 [Core Model 참조](core-model.md), [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md)가 정의한 active Core-owned field와 value를 assert해야 합니다. `tasks.lifecycle_phase`를 assert할 때 active fixture 본문은 `intake`, `shaping`, `ready`, `executing`, `waiting_user`, `blocked`, `completed`, `cancelled`만 사용합니다. Lifecycle value로 `active`, `open`, `terminal` 같은 status word를 쓰면 안 됩니다.
- `expected_storage_rows`는 [Storage](storage.md)의 active table, column, JSON payload shape, owner-bound value set을 assert해야 합니다. [Storage Validation And Enum Hardening](storage.md#canonical-enum-hardening)의 hardening map도 따라야 합니다.
- Active `expected_storage_rows`는 Storage가 소유한 active record 영역만 사용할 수 있습니다. 범위는 `project_state`, `surfaces` 또는 같은 역할의 reference-surface registration record, `tasks`, `task_events`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries` 또는 같은 역할의 minimal evidence coverage record, `blockers`, `tool_invocations`입니다. 다른 table family는 later/profile material이며 Storage와 owning profile이 승격하기 전까지 active fixture body 밖에 둡니다.
- Requirements-shaping fixture assertion은 shaping output을 active owner row에만 저장합니다. 해당 row는 필요에 따라 `tasks`, `change_units`, committed judgment request가 있을 때의 `user_judgments`, Core가 blocker를 commit할 때의 `blockers`, minimal evidence coverage effect가 있을 때의 `evidence_summaries`, 그리고 committed shaping run을 위한 `runs.kind=shaping_update`입니다. Shaping fixture가 active owner set 밖의 별도 design, clarification catalog, candidate-record storage를 요구하면 안 됩니다.
- `expected_storage_rows.write_authorizations`는 active `AuthorizedAttemptScope`를 `attempt_scope_json` 아래 보존해야 합니다. 포함되는 값은 `task_id`, `change_unit_id`, `basis_state_version`, `surface_id`, intended operation, intended paths/tools/commands와 command classes, product-file-write intent, intended network targets, intended secret handles/scope, sensitive categories, `baseline_ref`, related user judgment refs, `guarantee_level`입니다. Committed non-dry-run `prepare_write.decision=allowed` fixture는 `request.payload`의 proposed attempt-scope field, `expected_response.write_authorization.attempt_scope`, `expected_storage_rows.write_authorizations.attempt_scope_json`이 같은 resolved `AuthorizedAttemptScope`를 가리킨다고 assert해야 합니다. Request나 증명 claim에 command, tool, network, secret, baseline, sensitive category, surface, guarantee fact가 포함된다면 path만 assert해서는 안 됩니다. Committed non-dry-run `decision=allowed`만 `write_authorizations.status=active`를 만듭니다. `blocked`, `approval_required`, `decision_required`, `state_conflict`는 response/blocker/error outcome이지 authorization row가 아닙니다.
- `expected_storage_rows.runs`는 committed active run shape를 assert해야 합니다. `runs.kind`는 `shaping_update`, `implementation`, `direct`만 사용하고, `runs.status`는 Storage의 값을 사용합니다. 모든 committed Run row는 active `RecordRunPayload` branch와 comparison outcome을 담은 `observed_attempt_json` 또는 active `ObservedChanges`를 담은 `observed_changes_json` 중 하나를 포함해야 합니다. Product-write `implementation`과 `direct` Run은 둘 다 assert하고, compatible `write_authorization_id`, stored `AuthorizedAttemptScope`와의 비교, consumed authorization effect도 함께 assert합니다. Pre-commit rejected `record_run` fixture는 owner-defined violation/audit 예외를 명시적으로 다루지 않는 한 Run row, artifact/link/evidence mutation, authorization consumption, replay row가 없음을 assert합니다.
- `expected_storage_rows.user_judgments`는 active user-judgment schema가 정의한 `judgment_kind`, `presentation`, `status`, owner ref, payload JSON을 사용해야 합니다. `display_label`과 지역화된 라벨은 렌더링되는 표시 문구일 뿐입니다. Storage column, 기준 row identity, validator input이나 key, state-compatibility assertion, blocker key, gate key, compatibility input, close aggregation key로 나타나면 안 됩니다.
- `UserJudgmentCandidate`에 대한 assertion은 read, validation, dry-run, compatibility path가 반환하는 candidate-only output으로 다뤄야 합니다. Candidate는 committed `StateRecordRef`가 없고, `user_judgments` row를 만들지 않으며, blocker, gate, 민감 동작 승인, 최종 수락, 잔여 위험 수락, close, Write Authorization, evidence assertion을 충족하지 않습니다. 향후 fixture는 committed `dry_run=false` `harness.request_user_judgment` call이 그 active request를 기록한 뒤에만 pending `user_judgments` row를 assert할 수 있습니다. 같은 stored `judgment_kind`에 대해 `harness.record_user_judgment`가 사용자의 답을 기록한 뒤에만 resolved judgment effect를 assert할 수 있습니다.
- `expected_storage_rows.artifacts`와 `expected_storage_rows.artifact_links`는 fixture가 Storage와 `ArtifactRef`가 지원하는 registered artifact 또는 safe metadata notice를 commit할 때만 active입니다. Rejected raw-secret artifact branch는 `artifacts`, `artifact_links`, evidence-sufficiency mutation이 모두 없음을 assert합니다. Committed notice branch는 `redaction_state=blocked` 또는 `secret_omitted`인 safe artifact/link/evidence effect만 assert합니다.
- `expected_storage_rows.tool_invocations`는 committed replayable non-dry-run response에만 적용됩니다. Dry run, pre-commit state conflict, mutation 전 validation failure, pre-commit rejected `record_run` path는 replay row가 없음을 assert합니다.
- `expected_events`는 Core owner가 stable event fact를 승격한 뒤에만 그 값을 이름 붙입니다. `owner-promoted Run recording event` 같은 사람이 읽기 위한 label은 authoring note이지 active event value가 아닙니다.
- `expected_artifacts`는 [API Schema Core: ArtifactRef](api/schema-core.md#artifactref), [ArtifactInput](api/schema-core.md#artifactinput), [Storage](storage.md)가 정의한 active `ArtifactRef`, `ArtifactInput`, relation owner, redaction, retention, artifact status value를 사용해야 합니다.
- `ArtifactInput`, `ArtifactRef`, `expected_artifacts`, `expected_storage_rows.artifacts`의 active `redaction_state` 값은 정확히 `none`, `redacted`, `secret_omitted`, `blocked`입니다. `none`은 redaction 없이 저장해도 되는 byte에만 사용합니다. `redacted`는 storage 전에 content를 제거한 경우, `secret_omitted`는 secret 또는 PII material을 생략하거나 handle로 대체한 경우, `blocked`는 raw payload storage 또는 exposure가 blocked된 경우에 사용합니다. `visible`, `hidden`, `safe`, `unsafe`는 redaction state가 아닙니다.
- `expected_blockers`와 `expected_response.blockers`는 [MVP API: harness.close_task](api/mvp-api.md#harnessclose_task), [API Errors: harness.close_task Close Blockers](api/errors.md#harnessclose_task-close-blockers), Core/storage owner가 정의한 active blocker category, `required_judgment_kind`, related ref, close-blocker shape를 사용해야 합니다. Active close/status blocker assertion은 Schema Core가 MVP-1에서 제외한 category나 response field를 쓰면 안 됩니다.
- 민감 동작 승인 expectation은 active `user_judgment` / `judgment_kind=sensitive_approval`, `approval_scope`, `approval_gate`, active `sensitive_approval` blocker category, 또는 API owner가 선택한 `APPROVAL_REQUIRED` / `APPROVAL_DENIED` / `APPROVAL_EXPIRED` code를 사용해야 합니다. Broad permission text나 별도 permission-record lifecycle을 assert하면 안 됩니다. `decision_required` / `DECISION_REQUIRED`는 민감 동작 승인이 아닌 user-owned judgment에 남겨 두며 민감 동작 승인과 같은 뜻으로 쓰면 안 됩니다.
- `harness.close_task` fixture body는 `CloseTaskRequest.intent`에 `complete`, `cancel`, `supersede`만 사용해야 합니다. 일반 완료와 잔여 위험을 수락한 완료는 모두 `intent=complete`를 사용합니다. Accepted risk는 `intent`를 바꾸는 방식이 아니라 `requested_close_reason=completed_with_risk_accepted`와 compatible active Core state로 표현합니다. 취소는 `intent=cancel`과 `requested_close_reason=cancelled`를 사용합니다. Supersession은 `intent=supersede`, `requested_close_reason=superseded`, 필요하면 API가 소유한 supersession field를 사용합니다. Active fixture body는 close reason이나 later/profile 보증 value를 intent value로 쓰면 안 됩니다.
- `expected_errors`는 [API Errors](api/errors.md)의 active public `ErrorCode`와 primary-error precedence를 사용해야 합니다. Validator ID나 policy finding code는 owner-defined validator/state assertion 아래에 둡니다. Public API owner가 그 code를 선택하지 않는 한 primary `expected_errors[].code`로 쓰지 않습니다.
- `harness.record_run` error fixture는 [API Errors](api/errors.md#error-taxonomy)의 active mapping을 사용해야 합니다. Missing required authorization은 details에 reason을 assert할 때 `authorization_reason=missing`이 있는 `WRITE_AUTHORIZATION_REQUIRED`를 사용합니다. Stale, expired, revoked, consumed, incompatible authorization은 matching `authorization_reason`이 있는 `WRITE_AUTHORIZATION_INVALID`를 사용합니다. Stored `AuthorizedAttemptScope` 밖의 observed work는 `SCOPE_VIOLATION`을 사용합니다. Required comparison에 필요한 observation이 unsupported이거나 surface capability가 부족하면 `CAPABILITY_INSUFFICIENT`를 사용합니다. Forbidden secret 또는 artifact handling은 owner mapping에 따라 `VALIDATION_FAILED`, `SCOPE_VIOLATION`, 또는 `ARTIFACT_MISSING`을 사용합니다.
- `forbidden_side_effects`는 documentation draft에서는 읽기 쉬운 문장일 수 있습니다. 하지만 materialized executable fixture에서는 가능한 곳마다 owner-record absence, row effect, artifact, event, derived-view, generated-output assertion으로 확장해야 합니다. Failed operation에서는 `expected_storage_rows`와 `forbidden_side_effects`가 서로 맞아야 합니다. Fixture가 Run, artifact, replay row, evidence mutation, authorization consumption, derived-view job, non-active record를 금지하면서 동시에 그 row나 effect를 기대하면 안 됩니다. Absence 자체가 증명 대상이면 `expected_storage_rows` table-effect assertion과 compatible exact-mode metadata 또는 명시적인 negative side-effect assertion을 사용합니다.
- `harness.record_run` fixture body는 `RecordRunRequest.kind`, `RecordRunPayload.kind`, non-null `RecordRunPayload` branch 하나를 정확히 맞춰야 합니다. Active body는 `shaping_update`, `implementation`, `direct`만 사용할 수 있습니다. Discovery와 요구사항 구체화 update는 `shaping_update`를 씁니다. 구현 쓰기와 구현 시도는 `implementation`을 씁니다. 쓰기가 없는 직접 관찰과 제품 변경이 아닌 작업은 `direct`를 씁니다. Legacy 또는 shorthand run-kind value, schema에 없는 payload branch name, 여러 non-null payload branch는 invalid입니다.
- Later/profile-only value, branch, method, ref, table family, status value, error는 active MVP fixture 본문에 나타나면 안 됩니다. Owner가 더 좁은 path를 승격하기 전까지 [Schema Later](api/schema-later.md), promoted later/profile owner docs, [향후 Fixtures](../later/future-fixtures.md)에 남습니다.

`task-fixture-001` 같은 deterministic ID는 valid owner record와 matching ref 안의 일반 string ID로만 쓸 수 있습니다. Symbolic ID가 required record, omitted request field, unsupported schema branch, fixture-local status value, unexpanded artifact ref를 대신하면 안 됩니다.

<a id="engineering-checkpoint-behavior-examples"></a>
<a id="mvp-1-user-work-loop-behavior-examples"></a>
<a id="security-and-capability-behavior-examples"></a>
<a id="artifact-and-evidence-behavior-examples"></a>

### Active Structured Fixture Drafts

활성 초안은 하나의 공통 shape를 사용합니다. Review하기에 너무 커지지 않게 유지하되, 향후 materialized fixture가 확장하고 검증해야 할 public request field, active storage row, public error code, blocker category를 이름 붙입니다. 이 fenced body는 구조화된 source draft로 남아야 하며, 전체가 YAML로 파싱되어야 합니다. Markdown이나 YAML indicator로 시작하는 scalar를 따옴표로 감싸는 작업은 fixture 본문 유효성 정리일 뿐입니다. 새 fixture contract나 향후 runner 지원을 주장하는 말이 아닙니다. Evidence summary family는 두 request path를 둡니다. Insufficient 상태는 read/close-visible state이고, sufficient 상태는 committed active `record_run`으로 만들어지기 때문입니다.

```yaml
scenario_id: MVP-ACTIVE-task-change-unit-setup
purpose: Active Task / Change Unit setup을 만든다.
initial_state:
  project_state:
    project_id: PROJ-001
    state_version: 1
    active_task_id: null
    default_surface_id: reference-local-mcp
  surfaces:
    - surface_id: reference-local-mcp
      guarantee_level: cooperative
      status: active
request:
  tool: harness.intake
  payload:
    envelope:
      request_id: REQ-001
      idempotency_key: IDEMP-001
      expected_state_version: 1
      project_id: PROJ-001
      task_id: null
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: user
      dry_run: false
    user_request: "Implement the narrow settings copy change."
    requested_mode: work
    resume_policy: create_new
    acceptance_criteria:
      - "Settings copy is updated in the allowed path."
    constraints:
      allowed_paths: ["app/settings/page.tsx"]
      non_goals: ["No settings behavior change"]
      sensitive_categories: []
    initial_context_refs: []
expected_response:
  base:
    errors: []
  task_id: TASK-001
  created: true
  resumed: false
  change_unit_id: CU-001
  state:
    mode: work
    lifecycle_phase: ready
    result: none
    close_reason: none
expected_state_changes:
  project_state:
    active_task_id: TASK-001
  tasks:
    TASK-001:
      mode: work
      lifecycle_phase: ready
      active_change_unit_id: CU-001
  change_units:
    CU-001:
      task_id: TASK-001
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
expected_storage_rows:
  project_state:
    updated:
      rows:
        - project_id: PROJ-001
          active_task_id: TASK-001
  tasks:
    inserted:
      rows:
        - task_id: TASK-001
          mode: work
          lifecycle_phase: ready
          active_change_unit_id: CU-001
  change_units:
    inserted:
      rows:
        - change_unit_id: CU-001
          task_id: TASK-001
          status: active
  write_authorizations:
    inserted:
      count: 0
  runs:
    inserted:
      count: 0
  artifacts:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - No Write Authorization, Run, artifact, evidence summary, final acceptance, residual-risk acceptance, close state, or non-active row/effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessintake
  schema: docs/*/reference/api/schema-core.md
  core: docs/*/reference/core-model.md
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md
```

```yaml
scenario_id: MVP-ACTIVE-shaping-update-persists
purpose: Shaping update를 active state에 저장한다.
initial_state:
  project_state:
    project_id: PROJ-001
    state_version: 2
    active_task_id: TASK-001
    default_surface_id: reference-local-mcp
  tasks:
    - task_id: TASK-001
      mode: work
      lifecycle_phase: shaping
      active_change_unit_id: CU-001
      state_version: 2
  change_units:
    - change_unit_id: CU-001
      task_id: TASK-001
      status: active
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-002
      idempotency_key: IDEMP-002
      expected_state_version: 2
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: shaping_update
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: null
    summary: "Clarified the current goal and first allowed path."
    artifact_inputs: []
    payload:
      kind: shaping_update
      shaping_update:
        shaping_kind: scope
        task_update:
          title: null
          original_user_request: null
          current_goal_summary: "Update the settings copy only."
          mode: work
          success_criteria: ["Settings copy is updated."]
          non_goals: ["No behavior change"]
          affected_areas: ["settings"]
          affected_path_candidates: ["app/settings/page.tsx"]
          constraints:
            allowed_paths: ["app/settings/page.tsx"]
            sensitive_categories: []
        change_unit_update:
          change_unit_id: CU-001
          operation: update
          scope_summary: "Settings copy update."
          affected_areas: ["settings"]
          affected_path_candidates: ["app/settings/page.tsx"]
          allowed_paths: ["app/settings/page.tsx"]
          denied_paths: []
          non_goals: ["No behavior change"]
          success_criteria: ["Settings copy is updated."]
          sensitive_categories: []
          baseline_ref: BASE-001
          autonomy_boundary: null
        user_judgment_candidates: []
        confirmed_facts: ["The requested file is inside the active scope."]
        remaining_uncertainties: []
        blocking_question: null
        useful_non_blocking_questions: []
        next_safe_action: "Run prepare_write for the settings copy change."
        source_refs:
          - record_kind: task
            record_id: TASK-001
        evidence_refs:
          state_refs: []
          artifact_refs: []
      implementation: null
      direct: null
expected_response:
  base:
    errors: []
  run_id: RUN-001
  state:
    mode: work
    lifecycle_phase: shaping
  write_authorization_ref: null
  registered_artifacts: []
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: shaping
      current_goal_summary: "Update the settings copy only."
      next_safe_action: "Run prepare_write for the settings copy change."
  change_units:
    CU-001:
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
  runs:
    RUN-001:
      kind: shaping_update
      status: completed
      product_write: false
expected_storage_rows:
  tasks:
    updated:
      rows:
        - task_id: TASK-001
          lifecycle_phase: shaping
          current_goal_summary: "Update the settings copy only."
  change_units:
    updated:
      rows:
        - change_unit_id: CU-001
          status: active
  runs:
    inserted:
      rows:
        - run_id: RUN-001
          kind: shaping_update
          status: completed
          product_write: false
          write_authorization_id: null
  write_authorizations:
    inserted:
      count: 0
  artifacts:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - No product-write Run, Write Authorization, non-active row/effect, final acceptance, or residual-risk acceptance is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#record-run-payloads
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-allowed-authorization
purpose: prepare_write allowed 결과로 Write Authorization을 생성한다.
initial_state:
  project_state:
    project_id: PROJ-001
    active_task_id: TASK-001
    default_surface_id: reference-local-mcp
  tasks:
    - task_id: TASK-001
      mode: work
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 3
  change_units:
    - change_unit_id: CU-001
      task_id: TASK-001
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
      baseline_ref: BASE-001
request:
  tool: harness.prepare_write
  payload:
    envelope:
      request_id: REQ-003
      idempotency_key: IDEMP-003
      expected_state_version: 3
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    change_unit_id: CU-001
    intended_operation: "Update settings copy."
    intended_paths: ["app/settings/page.tsx"]
    intended_tools: ["edit"]
    intended_commands: []
    product_file_write_intended: true
    intended_network: []
    intended_secret_scope: []
    sensitive_categories: []
    baseline_ref: BASE-001
expected_response:
  base:
    errors: []
  decision: allowed
  state:
    lifecycle_phase: ready
  change_unit_id: CU-001
  baseline_ref: BASE-001
  write_authorization_ref:
    record_kind: write_authorization
    record_id: WA-001
  write_authorization:
    write_authorization_id: WA-001
    status: active
    attempt_scope:
      task_id: TASK-001
      change_unit_id: CU-001
      basis_state_version: 3
      surface_id: reference-local-mcp
      intended_operation: "Update settings copy."
      intended_paths: ["app/settings/page.tsx"]
      intended_tools: ["edit"]
      intended_commands: []
      product_file_write_intended: true
      intended_network: []
      intended_secret_scope: []
      sensitive_categories: []
      baseline_ref: BASE-001
      related_user_judgment_refs: []
      guarantee_level: cooperative
  authorization_effect: created
  active_user_judgment_refs: []
  blocked_reasons: []
expected_state_changes:
  write_authorizations:
    WA-001:
      status: active
      basis_state_version: 3
      consumed_by_run_id: null
  tasks:
    TASK-001:
      lifecycle_phase: ready
expected_storage_rows:
  write_authorizations:
    inserted:
      rows:
        - write_authorization_id: WA-001
          task_id: TASK-001
          change_unit_id: CU-001
          surface_id: reference-local-mcp
          status: active
          basis_state_version: 3
          attempt_scope_json:
            task_id: TASK-001
            change_unit_id: CU-001
            basis_state_version: 3
            surface_id: reference-local-mcp
            intended_operation: "Update settings copy."
            intended_paths: ["app/settings/page.tsx"]
            intended_tools: ["edit"]
            intended_commands: []
            product_file_write_intended: true
            intended_network: []
            intended_secret_scope: []
            sensitive_categories: []
            baseline_ref: BASE-001
            related_user_judgment_refs: []
            guarantee_level: cooperative
  tool_invocations:
    inserted:
      rows:
        - tool_name: harness.prepare_write
          idempotency_key: IDEMP-003
          task_id: TASK-001
          basis_state_version: 3
          status: committed
  runs:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - No OS permission, sandboxing, tamper-proof enforcement, preventive blocking, isolated guarantee, Run, artifact, evidence sufficiency, close state, final acceptance, or residual-risk acceptance is claimed or created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessprepare_write
  schema: docs/*/reference/api/schema-core.md#evidence-and-pre-write-scope-schemas
  core: docs/*/reference/core-model.md#prepare_write
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-blocked-no-authorization
purpose: prepare_write blocked 결과에서는 Write Authorization을 생성하지 않는다.
initial_state:
  project_state:
    project_id: PROJ-001
    active_task_id: TASK-001
    default_surface_id: reference-local-mcp
  tasks:
    - task_id: TASK-001
      mode: work
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 4
  change_units:
    - change_unit_id: CU-001
      task_id: TASK-001
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
request:
  tool: harness.prepare_write
  payload:
    envelope:
      request_id: REQ-004
      idempotency_key: IDEMP-004
      expected_state_version: 4
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    change_unit_id: CU-001
    intended_operation: "Update billing copy outside the active scope."
    intended_paths: ["app/billing/page.tsx"]
    intended_tools: ["edit"]
    intended_commands: []
    product_file_write_intended: true
    intended_network: []
    intended_secret_scope: []
    sensitive_categories: []
    baseline_ref: BASE-001
expected_response:
  base:
    errors: []
  decision: blocked
  write_authorization_ref: null
  write_authorization: null
  authorization_effect: none
  blocked_reasons:
    - code: out_of_scope
      related_error: SCOPE_VIOLATION
      required_judgment_kind: scope_decision
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: blocked
  blockers:
    - task_id: TASK-001
      blocked_action: prepare_write
      blocker_kind: scope
      status: open
expected_storage_rows:
  blockers:
    inserted:
      rows:
        - task_id: TASK-001
          blocked_action: prepare_write
          blocker_kind: scope
          status: open
  write_authorizations:
    inserted:
      count: 0
  runs:
    inserted:
      count: 0
  artifacts:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers:
  - code: SCOPE_VIOLATION
    blocker_kind: scope
    required_judgment_kind: scope_decision
expected_errors: []
forbidden_side_effects:
  - No consumable Write Authorization row, Run, artifact, evidence summary, non-active effect, close state, final acceptance, or residual-risk acceptance is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessprepare_write
  schema: docs/*/reference/api/schema-core.md#evidence-and-pre-write-scope-schemas
  core: docs/*/reference/core-model.md#prepare_write
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-idempotent-replay
purpose: Idempotent replay는 original committed prepare_write response를 반환한다.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 5
  write_authorizations:
    - write_authorization_id: WA-001
      task_id: TASK-001
      change_unit_id: CU-001
      surface_id: reference-local-mcp
      status: active
      basis_state_version: 5
      attempt_scope_json:
        task_id: TASK-001
        change_unit_id: CU-001
        basis_state_version: 5
        surface_id: reference-local-mcp
        intended_operation: "Update settings copy."
        intended_paths: ["app/settings/page.tsx"]
        intended_tools: ["edit"]
        intended_commands: []
        product_file_write_intended: true
        intended_network: []
        intended_secret_scope: []
        sensitive_categories: []
        baseline_ref: BASE-001
        related_user_judgment_refs: []
        guarantee_level: cooperative
  tool_invocations:
    - tool_name: harness.prepare_write
      idempotency_key: IDEMP-005
      request_hash: HASH-ORIGINAL
      task_id: TASK-001
      basis_state_version: 5
      status: committed
      response_json:
        decision: allowed
        write_authorization_ref:
          record_kind: write_authorization
          record_id: WA-001
request:
  tool: harness.prepare_write
  payload:
    envelope:
      request_id: REQ-005-REPLAY
      idempotency_key: IDEMP-005
      expected_state_version: 5
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    change_unit_id: CU-001
    intended_operation: "Update settings copy."
    intended_paths: ["app/settings/page.tsx"]
    intended_tools: ["edit"]
    intended_commands: []
    product_file_write_intended: true
    intended_network: []
    intended_secret_scope: []
    sensitive_categories: []
    baseline_ref: BASE-001
expected_response:
  base:
    errors: []
  decision: allowed
  write_authorization_ref:
    record_kind: write_authorization
    record_id: WA-001
  authorization_effect: returned
expected_state_changes: {}
expected_storage_rows:
  write_authorizations:
    inserted:
      count: 0
    updated:
      count: 0
  tool_invocations:
    inserted:
      count: 0
    updated:
      count: 0
    unchanged:
      rows:
        - tool_name: harness.prepare_write
          idempotency_key: IDEMP-005
          request_hash: HASH-ORIGINAL
          status: committed
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - No duplicate Write Authorization, event, artifact, replay-row update, non-active effect, state-version increment, Run, evidence, close, final acceptance, or residual-risk acceptance is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessprepare_write
  schema: docs/*/reference/api/schema-core.md#tool-envelope
  core: docs/*/reference/core-model.md#prepare_write
  storage: docs/*/reference/storage.md#event-and-idempotency-semantics
  errors: docs/*/reference/api/errors.md#idempotency
```

```yaml
scenario_id: MVP-ACTIVE-idempotency-key-hash-conflict
purpose: 같은 idempotency key와 다른 canonical request hash는 conflict를 반환한다.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 6
  tool_invocations:
    - tool_name: harness.prepare_write
      idempotency_key: IDEMP-006
      request_hash: HASH-ORIGINAL
      task_id: TASK-001
      basis_state_version: 6
      status: committed
request:
  tool: harness.prepare_write
  payload:
    envelope:
      request_id: REQ-006-CONFLICT
      idempotency_key: IDEMP-006
      expected_state_version: 6
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    change_unit_id: CU-001
    intended_operation: "Update a different path with the reused key."
    intended_paths: ["app/account/page.tsx"]
    intended_tools: ["edit"]
    intended_commands: []
    product_file_write_intended: true
    intended_network: []
    intended_secret_scope: []
    sensitive_categories: []
    baseline_ref: BASE-001
expected_response:
  base:
    errors:
      - code: STATE_CONFLICT
        retryable: true
        details:
          stored_request_hash: HASH-ORIGINAL
          received_request_hash: HASH-DIFFERENT
expected_state_changes: {}
expected_storage_rows:
  tool_invocations:
    inserted:
      count: 0
    updated:
      count: 0
    unchanged:
      rows:
        - tool_name: harness.prepare_write
          idempotency_key: IDEMP-006
          request_hash: HASH-ORIGINAL
          status: committed
  write_authorizations:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors:
  - code: STATE_CONFLICT
forbidden_side_effects:
  - No merged response, new Write Authorization, event, artifact, non-active effect, owner relation, replay-row update, Run, evidence, close, final acceptance, or residual-risk acceptance is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessprepare_write
  schema: docs/*/reference/api/schema-core.md#tool-envelope
  core: docs/*/reference/core-model.md#prepare_write
  storage: docs/*/reference/storage.md#event-and-idempotency-semantics
  errors: docs/*/reference/api/errors.md#idempotency
```

```yaml
scenario_id: MVP-ACTIVE-record-run-consumes-authorization
purpose: record_run은 compatible Write Authorization을 소비한다.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 7
  change_units:
    - change_unit_id: CU-001
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
  write_authorizations:
    - write_authorization_id: WA-007
      task_id: TASK-001
      change_unit_id: CU-001
      surface_id: reference-local-mcp
      status: active
      basis_state_version: 7
      attempt_scope_json:
        task_id: TASK-001
        change_unit_id: CU-001
        basis_state_version: 7
        surface_id: reference-local-mcp
        intended_operation: "Update settings copy."
        intended_paths: ["app/settings/page.tsx"]
        intended_tools: ["edit"]
        intended_commands: []
        product_file_write_intended: true
        intended_network: []
        intended_secret_scope: []
        sensitive_categories: []
        baseline_ref: BASE-001
        related_user_judgment_refs: []
        guarantee_level: cooperative
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-007
      idempotency_key: IDEMP-007
      expected_state_version: 7
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: implementation
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: WA-007
    summary: "Updated settings copy."
    artifact_inputs:
      - input_id: ARTIN-007-DIFF
        source_kind: staged_file
        existing_artifact_ref: null
        staged:
          staged_uri: harness-staging://PROJ-001/RUN-007/settings.diff
          display_name: settings.diff
          content_type: text/x-diff
          expected_sha256: SHA256-DIFF-007
          expected_size_bytes: 2048
        capture: null
        kind: diff
        redaction_state: none
        produced_by: lead_agent
        retention_class: task
        relation:
          task_id: TASK-001
          run_id: null
          record_kind: run
          record_id_hint: RUN-007
        description: "Diff for settings copy."
    payload:
      kind: implementation
      shaping_update: null
      implementation:
        outcome: completed
        product_write: true
        observed_changes:
          changed_paths:
            - path: app/settings/page.tsx
              change_kind: modified
              product_file: true
              within_change_unit: true
              before_sha256: SHA256-BEFORE-007
              after_sha256: SHA256-AFTER-007
          diff_artifact_input_ids: ["ARTIN-007-DIFF"]
          no_product_changes: false
        command_results: []
        tool_invocations:
          - tool_name: edit
            purpose: "Apply settings copy change."
            status: succeeded
            artifact_input_ids: ["ARTIN-007-DIFF"]
            summary: "Changed one scoped file."
        network_accesses: []
        secret_accesses: []
        evidence_updates:
          coverage_updates:
            - claim_or_criterion: "Settings copy is updated."
              coverage_state: supported
              supporting_state_refs: []
              supporting_artifact_input_ids: ["ARTIN-007-DIFF"]
              note: "Diff supports the copy update."
          gap_blocker_refs: []
          summary: "Implementation evidence recorded."
        implementation_notes: []
        follow_up_needed: []
      direct: null
expected_response:
  base:
    errors: []
  run_id: RUN-007
  state:
    lifecycle_phase: executing
  write_authorization_ref:
    record_kind: write_authorization
    record_id: WA-007
  registered_artifacts:
    - artifact_id: ART-007-DIFF
      kind: diff
      redaction_state: none
      retention_class: task
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: executing
  write_authorizations:
    WA-007:
      status: consumed
      consumed_by_run_id: RUN-007
  runs:
    RUN-007:
      kind: implementation
      status: completed
      product_write: true
expected_storage_rows:
  runs:
    inserted:
      rows:
        - run_id: RUN-007
          task_id: TASK-001
          change_unit_id: CU-001
          write_authorization_id: WA-007
          kind: implementation
          status: completed
          product_write: true
  write_authorizations:
    updated:
      rows:
        - write_authorization_id: WA-007
          status: consumed
          consumed_by_run_id: RUN-007
  artifacts:
    inserted:
      rows:
        - artifact_id: ART-007-DIFF
          kind: diff
          redaction_state: none
          retention_class: task
          status: available
  artifact_links:
    inserted:
      rows:
        - artifact_id: ART-007-DIFF
          task_id: TASK-001
          owner_record_kind: run
          owner_record_id: RUN-007
  tool_invocations:
    inserted:
      rows:
        - tool_name: harness.record_run
          idempotency_key: IDEMP-007
          status: committed
expected_events: []
expected_artifacts:
  - artifact_id: ART-007-DIFF
    kind: diff
    redaction_state: none
    relation_owner:
      record_kind: run
      record_id: RUN-007
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - The Write Authorization is not consumed twice.
  - No final acceptance, residual-risk acceptance, non-active assurance state, or close state is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#record-run-payloads
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-record-run-missing-authorization-blocked
purpose: record_run은 authorization 없는 product write를 거부한다.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 8
  change_units:
    - change_unit_id: CU-001
      status: active
      allowed_paths_json: ["app/settings/page.tsx"]
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-008
      idempotency_key: IDEMP-008
      expected_state_version: 8
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: implementation
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: null
    summary: "Product file was changed without a pre-write scope check."
    artifact_inputs: []
    payload:
      kind: implementation
      shaping_update: null
      implementation:
        outcome: completed
        product_write: true
        observed_changes:
          changed_paths:
            - path: app/settings/page.tsx
              change_kind: modified
              product_file: true
              within_change_unit: true
              before_sha256: SHA256-BEFORE-008
              after_sha256: SHA256-AFTER-008
          diff_artifact_input_ids: []
          no_product_changes: false
        command_results: []
        tool_invocations: []
        network_accesses: []
        secret_accesses: []
        evidence_updates:
          coverage_updates: []
          gap_blocker_refs: []
          summary: "Rejected before evidence mutation."
        implementation_notes: []
        follow_up_needed: []
      direct: null
expected_response:
  base:
    errors:
      - code: WRITE_AUTHORIZATION_REQUIRED
        details:
          authorization_reason: missing
  run_id: null
  state:
    lifecycle_phase: ready
  write_authorization_ref: null
  registered_artifacts: []
expected_state_changes: {}
expected_storage_rows:
  runs:
    inserted:
      count: 0
  write_authorizations:
    updated:
      count: 0
  artifacts:
    inserted:
      count: 0
  artifact_links:
    inserted:
      count: 0
  evidence_summaries:
    inserted:
      count: 0
    updated:
      count: 0
  tool_invocations:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors:
  - code: WRITE_AUTHORIZATION_REQUIRED
    details:
      authorization_reason: missing
forbidden_side_effects:
  - No Run, artifact, artifact link, evidence summary mutation, authorization consumption, blocker/gate update, task event, non-active effect, state-version advance, replay row, completion evidence, final acceptance, residual-risk acceptance, or close state is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#record-run-payloads
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-record-run-observed-out-of-scope
purpose: Authorized scope 밖에서 관찰된 attempt를 거부한다.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 9
  write_authorizations:
    - write_authorization_id: WA-009
      task_id: TASK-001
      change_unit_id: CU-001
      surface_id: reference-local-mcp
      status: active
      basis_state_version: 9
      attempt_scope_json:
        task_id: TASK-001
        change_unit_id: CU-001
        basis_state_version: 9
        surface_id: reference-local-mcp
        intended_operation: "Update settings copy."
        intended_paths: ["app/settings/page.tsx"]
        intended_tools: ["edit"]
        intended_commands: []
        product_file_write_intended: true
        intended_network: []
        intended_secret_scope: []
        sensitive_categories: []
        baseline_ref: BASE-001
        related_user_judgment_refs: []
        guarantee_level: cooperative
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-009
      idempotency_key: IDEMP-009
      expected_state_version: 9
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: implementation
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: WA-009
    summary: "Observed change includes a path outside the authorized scope."
    artifact_inputs: []
    payload:
      kind: implementation
      shaping_update: null
      implementation:
        outcome: completed
        product_write: true
        observed_changes:
          changed_paths:
            - path: app/billing/page.tsx
              change_kind: modified
              product_file: true
              within_change_unit: false
              before_sha256: SHA256-BEFORE-009
              after_sha256: SHA256-AFTER-009
          diff_artifact_input_ids: []
          no_product_changes: false
        command_results: []
        tool_invocations: []
        network_accesses: []
        secret_accesses: []
        evidence_updates:
          coverage_updates: []
          gap_blocker_refs: []
          summary: "Rejected before evidence mutation."
        implementation_notes: []
        follow_up_needed: []
      direct: null
expected_response:
  base:
    errors:
      - code: SCOPE_VIOLATION
  run_id: null
  state:
    lifecycle_phase: ready
  write_authorization_ref:
    record_kind: write_authorization
    record_id: WA-009
  registered_artifacts: []
expected_state_changes: {}
expected_storage_rows:
  runs:
    inserted:
      count: 0
  write_authorizations:
    unchanged:
      rows:
        - write_authorization_id: WA-009
          status: active
          consumed_by_run_id: null
  artifacts:
    inserted:
      count: 0
  artifact_links:
    inserted:
      count: 0
  evidence_summaries:
    inserted:
      count: 0
    updated:
      count: 0
  tool_invocations:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors:
  - code: SCOPE_VIOLATION
forbidden_side_effects:
  - The active Write Authorization is not consumed as success.
  - No Run, artifact, artifact link, evidence mutation, replay row, close readiness, completion evidence, final acceptance, residual-risk acceptance, or non-active effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#record-run-payloads
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-raw-secret-artifact-blocked
purpose: Raw secret artifact storage를 mutation 전에 차단한다.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: executing
      active_change_unit_id: CU-001
      state_version: 10
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-010
      idempotency_key: IDEMP-010
      expected_state_version: 10
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: direct
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: null
    summary: "Attempt to register a staged artifact classified as raw secret material."
    artifact_inputs:
      - input_id: ARTIN-010-SECRET
        source_kind: staged_file
        existing_artifact_ref: null
        staged:
          staged_uri: harness-staging://PROJ-001/RUN-010/raw-secret.log
          display_name: raw-secret.log
          content_type: text/plain
          expected_sha256: SHA256-SECRET-STAGED
          expected_size_bytes: 512
        capture: null
        kind: log
        redaction_state: none
        produced_by: lead_agent
        retention_class: task
        relation:
          task_id: TASK-001
          run_id: null
          record_kind: evidence_summary
          record_id_hint: EVID-001
        description: "Rejected because staged bytes are classified as raw secret material."
    payload:
      kind: direct
      shaping_update: null
      implementation: null
      direct:
        result_kind: no_change
        product_write: false
        direct_summary: "No product change; artifact registration was rejected."
        observed_changes:
          changed_paths: []
          diff_artifact_input_ids: []
          no_product_changes: true
        command_results: []
        tool_invocations: []
        network_accesses: []
        secret_accesses: []
        evidence_updates:
          coverage_updates:
            - claim_or_criterion: "Raw secret log must not be stored."
              coverage_state: blocked
              supporting_state_refs: []
              supporting_artifact_input_ids: ["ARTIN-010-SECRET"]
              note: "Rejected before artifact commit."
          gap_blocker_refs: []
          summary: "Forbidden raw-secret artifact input."
        user_visible_result: "Artifact storage was blocked."
        follow_up_needed: ["Provide a redacted or secret-omitted artifact."]
expected_response:
  base:
    errors:
      - code: VALIDATION_FAILED
  run_id: null
  registered_artifacts: []
expected_state_changes: {}
expected_storage_rows:
  runs:
    inserted:
      count: 0
  artifacts:
    inserted:
      count: 0
  artifact_links:
    inserted:
      count: 0
  evidence_summaries:
    updated:
      count: 0
  tool_invocations:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors:
  - code: VALIDATION_FAILED
forbidden_side_effects:
  - No raw secret bytes, token value, full sensitive log, rendered raw-secret content, non-active output, artifact row, artifact link, evidence sufficiency mutation, authorization consumption, or close state is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#artifactinput
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md#artifact-and-evidence-boundary
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-evidence-summary-insufficient
purpose: Insufficient evidence summary가 active blocker로 계속 보인다.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: blocked
      active_change_unit_id: CU-001
      state_version: 11
  evidence_summaries:
    - evidence_summary_id: EVID-011
      task_id: TASK-001
      change_unit_id: CU-001
      status: partial
      gap_blocker_ids_json: ["BLK-011"]
  blockers:
    - blocker_id: BLK-011
      task_id: TASK-001
      blocked_action: close_task
      blocker_kind: evidence
      status: open
request:
  tool: harness.status
  payload:
    envelope:
      request_id: REQ-011
      idempotency_key: null
      expected_state_version: null
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    include:
      task: true
      gates: true
      projections: false
      pending_user_judgments: true
      guarantees: true
      user_judgments: true
      autonomy_boundary: true
      write_authority: true
      residual_risk: true
expected_response:
  base:
    errors: []
  active_task:
    lifecycle_phase: blocked
  evidence_summary:
    evidence_summary_ref:
      record_kind: evidence_summary
      record_id: EVID-011
    status: partial
  blocker_refs:
    - record_kind: blocker
      record_id: BLK-011
expected_state_changes: {}
expected_storage_rows:
  evidence_summaries:
    unchanged:
      rows:
        - evidence_summary_id: EVID-011
          status: partial
  blockers:
    unchanged:
      rows:
        - blocker_id: BLK-011
          blocker_kind: evidence
          status: open
  tool_invocations:
    inserted:
      count: 0
expected_events: []
expected_artifacts: []
expected_blockers:
  - code: EVIDENCE_INSUFFICIENT
    blocker_kind: evidence
expected_errors: []
forbidden_side_effects:
  - 상태 문장, Markdown evidence text, 읽기용 보기 또는 agent summary는 missing evidence refs를 보완하거나 evidence, artifact, final acceptance, residual-risk acceptance, Task close를 만들지 않습니다.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessstatus
  schema: docs/*/reference/api/schema-core.md#current-position-display-schemas
  core: docs/*/reference/core-model.md#close_task
  storage: docs/*/reference/storage.md#fields-needed-for-close-blocker-calculation
  errors: docs/*/reference/api/errors.md#harnessclose_task-close-blockers
```

```yaml
scenario_id: MVP-ACTIVE-evidence-summary-sufficient
purpose: Sufficient evidence summary는 active Run과 artifact ref로 뒷받침된다.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: executing
      active_change_unit_id: CU-001
      state_version: 12
  write_authorizations:
    - write_authorization_id: WA-012
      task_id: TASK-001
      change_unit_id: CU-001
      surface_id: reference-local-mcp
      status: active
      basis_state_version: 12
      attempt_scope_json:
        task_id: TASK-001
        change_unit_id: CU-001
        basis_state_version: 12
        surface_id: reference-local-mcp
        intended_operation: "Update settings copy."
        intended_paths: ["app/settings/page.tsx"]
        intended_tools: ["edit"]
        intended_commands: []
        product_file_write_intended: true
        intended_network: []
        intended_secret_scope: []
        sensitive_categories: []
        baseline_ref: BASE-001
        related_user_judgment_refs: []
        guarantee_level: cooperative
request:
  tool: harness.record_run
  payload:
    envelope:
      request_id: REQ-012
      idempotency_key: IDEMP-012
      expected_state_version: 12
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    kind: implementation
    task_id: TASK-001
    change_unit_id: CU-001
    run_id: null
    baseline_ref: BASE-001
    write_authorization_id: WA-012
    summary: "Implemented and checked the scoped copy update."
    artifact_inputs:
      - input_id: ARTIN-012-DIFF
        source_kind: staged_file
        existing_artifact_ref: null
        staged:
          staged_uri: harness-staging://PROJ-001/RUN-012/settings.diff
          display_name: settings.diff
          content_type: text/x-diff
          expected_sha256: SHA256-DIFF-012
          expected_size_bytes: 4096
        capture: null
        kind: diff
        redaction_state: none
        produced_by: lead_agent
        retention_class: task
        relation:
          task_id: TASK-001
          run_id: null
          record_kind: evidence_summary
          record_id_hint: EVID-012
        description: "Diff supporting the required evidence summary."
    payload:
      kind: implementation
      shaping_update: null
      implementation:
        outcome: completed
        product_write: true
        observed_changes:
          changed_paths:
            - path: app/settings/page.tsx
              change_kind: modified
              product_file: true
              within_change_unit: true
              before_sha256: SHA256-BEFORE-012
              after_sha256: SHA256-AFTER-012
          diff_artifact_input_ids: ["ARTIN-012-DIFF"]
          no_product_changes: false
        command_results: []
        tool_invocations:
          - tool_name: edit
            purpose: "Apply scoped copy update."
            status: succeeded
            artifact_input_ids: ["ARTIN-012-DIFF"]
            summary: "Changed the allowed file."
        network_accesses: []
        secret_accesses: []
        evidence_updates:
          coverage_updates:
            - claim_or_criterion: "Settings copy is updated."
              coverage_state: supported
              supporting_state_refs: []
              supporting_artifact_input_ids: ["ARTIN-012-DIFF"]
              note: "Diff supports the required claim."
          gap_blocker_refs: []
          summary: "Evidence is sufficient for the scoped update."
        implementation_notes: []
        follow_up_needed: []
      direct: null
expected_response:
  base:
    errors: []
  run_id: RUN-012
  state:
    lifecycle_phase: executing
  evidence_summary:
    evidence_summary_ref:
      record_kind: evidence_summary
      record_id: EVID-012
    status: sufficient
  registered_artifacts:
    - artifact_id: ART-012-DIFF
      kind: diff
      redaction_state: none
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: executing
  evidence_summaries:
    EVID-012:
      status: sufficient
  write_authorizations:
    WA-012:
      status: consumed
      consumed_by_run_id: RUN-012
expected_storage_rows:
  runs:
    inserted:
      rows:
        - run_id: RUN-012
          kind: implementation
          status: completed
          product_write: true
          write_authorization_id: WA-012
  artifacts:
    inserted:
      rows:
        - artifact_id: ART-012-DIFF
          kind: diff
          redaction_state: none
          status: available
  artifact_links:
    inserted:
      rows:
        - artifact_id: ART-012-DIFF
          owner_record_kind: evidence_summary
          owner_record_id: EVID-012
  evidence_summaries:
    inserted:
      rows:
        - evidence_summary_id: EVID-012
          task_id: TASK-001
          change_unit_id: CU-001
          status: sufficient
  write_authorizations:
    updated:
      rows:
        - write_authorization_id: WA-012
          status: consumed
          consumed_by_run_id: RUN-012
expected_events: []
expected_artifacts:
  - artifact_id: ART-012-DIFF
    kind: diff
    redaction_state: none
    relation_owner:
      record_kind: evidence_summary
      record_id: EVID-012
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - No non-active evidence, verification, assurance, or risk row; final acceptance; residual-risk acceptance; authority-rendering effect; or close state is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrecord_run
  schema: docs/*/reference/api/schema-core.md#evidence-and-pre-write-scope-schemas
  core: docs/*/reference/core-model.md#record_run
  storage: docs/*/reference/storage.md#artifact-and-evidence-boundary
  errors: docs/*/reference/api/errors.md#error-taxonomy
```

```yaml
scenario_id: MVP-ACTIVE-final-acceptance-missing-close-blocker
purpose: Final acceptance 누락은 close blocker가 된다.
initial_state:
  tasks:
    - task_id: TASK-001
      mode: work
      lifecycle_phase: executing
      result: none
      active_change_unit_id: CU-001
      state_version: 13
  evidence_summaries:
    - evidence_summary_id: EVID-013
      task_id: TASK-001
      change_unit_id: CU-001
      status: sufficient
  user_judgments: []
request:
  tool: harness.close_task
  payload:
    envelope:
      request_id: REQ-013
      idempotency_key: IDEMP-013
      expected_state_version: 13
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    intent: complete
    requested_close_reason: completed_self_checked
    user_note: null
    superseded_by_task_id: null
expected_response:
  base:
    errors:
      - code: ACCEPTANCE_REQUIRED
  close_state: blocked
  closed: false
  close_reason: none
  assurance_level: none
  evidence_summary:
    evidence_summary_ref:
      record_kind: evidence_summary
      record_id: EVID-013
    status: sufficient
  acceptance_state:
    status: required
    accepted_by_ref: null
    required_before_close: true
  blockers:
    - code: ACCEPTANCE_REQUIRED
      category: final_acceptance
      required_judgment_kind: final_acceptance
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: executing
      result: none
      close_reason: none
expected_storage_rows:
  tasks:
    unchanged:
      rows:
        - task_id: TASK-001
          lifecycle_phase: executing
          result: none
  user_judgments:
    inserted:
      count: 0
  blockers:
    inserted:
      rows:
        - task_id: TASK-001
          blocked_action: close_task
          blocker_kind: final_acceptance
          status: open
expected_events: []
expected_artifacts: []
expected_blockers:
  - code: ACCEPTANCE_REQUIRED
    category: final_acceptance
    required_judgment_kind: final_acceptance
expected_errors:
  - code: ACCEPTANCE_REQUIRED
forbidden_side_effects:
  - No terminal Task update, fabricated final_acceptance judgment, residual-risk acceptance, non-active risk/assurance row, close record, final report authority, or authority-rendering effect is created.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessclose_task
  schema: docs/*/reference/api/schema-core.md#userjudgment
  core: docs/*/reference/core-model.md#close_task
  storage: docs/*/reference/storage.md#fields-needed-for-close-blocker-calculation
  errors: docs/*/reference/api/errors.md#harnessclose_task-close-blockers
```

```yaml
scenario_id: MVP-ACTIVE-residual-risk-visible-not-accepted-blocker
purpose: 보이는 residual risk가 accepted되지 않으면 close blocker가 된다.
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: executing
      active_change_unit_id: CU-001
      state_version: 14
  evidence_summaries:
    - evidence_summary_id: EVID-014
      task_id: TASK-001
      status: sufficient
  blockers:
    - blocker_id: BLK-RISK-014
      task_id: TASK-001
      blocked_action: close_task
      blocker_kind: residual_risk_visibility
      status: open
  user_judgments: []
request:
  tool: harness.close_task
  payload:
    envelope:
      request_id: REQ-014
      idempotency_key: IDEMP-014
      expected_state_version: 14
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    intent: complete
    requested_close_reason: completed_with_risk_accepted
    user_note: null
    superseded_by_task_id: null
expected_response:
  base:
    errors:
      - code: DECISION_REQUIRED
  close_state: blocked
  closed: false
  close_reason: none
  residual_risk_state:
    status: visible
    visible_refs:
      - record_kind: blocker
        record_id: BLK-RISK-014
    unaccepted_refs:
      - record_kind: blocker
        record_id: BLK-RISK-014
  blockers:
    - code: DECISION_REQUIRED
      category: residual_risk_acceptance
      required_judgment_kind: residual_risk_acceptance
      related_refs:
        - record_kind: blocker
          record_id: BLK-RISK-014
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: executing
      close_reason: none
expected_storage_rows:
  tasks:
    unchanged:
      rows:
        - task_id: TASK-001
          lifecycle_phase: executing
  user_judgments:
    inserted:
      count: 0
  blockers:
    unchanged:
      rows:
        - blocker_id: BLK-RISK-014
          blocker_kind: residual_risk_visibility
          status: open
expected_events: []
expected_artifacts: []
expected_blockers:
  - code: DECISION_REQUIRED
    category: residual_risk_acceptance
    required_judgment_kind: residual_risk_acceptance
expected_errors:
  - code: DECISION_REQUIRED
forbidden_side_effects:
  - Visible risk is not treated as accepted risk.
  - No non-active risk/assurance row, residual-risk acceptance judgment, final acceptance, terminal Task update, close report authority, or authority-rendering effect is fabricated.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessclose_task
  schema: docs/*/reference/api/schema-core.md#acceptedriskinput
  core: docs/*/reference/core-model.md#close_task
  storage: docs/*/reference/storage.md#fields-needed-for-close-blocker-calculation
  errors: docs/*/reference/api/errors.md#harnessclose_task-close-blockers
```

```yaml
scenario_id: MVP-ACTIVE-accepted-risk-close
purpose: Accepted-risk close는 active residual-risk acceptance state에서만 성공한다.
initial_state:
  tasks:
    - task_id: TASK-001
      mode: work
      lifecycle_phase: executing
      result: none
      active_change_unit_id: CU-001
      state_version: 15
  evidence_summaries:
    - evidence_summary_id: EVID-015
      task_id: TASK-001
      status: sufficient
  blockers:
    - blocker_id: BLK-RISK-015
      task_id: TASK-001
      blocked_action: close_task
      blocker_kind: residual_risk_visibility
      status: open
  user_judgments:
    - user_judgment_id: UJ-RISK-015
      task_id: TASK-001
      judgment_kind: residual_risk_acceptance
      presentation: short
      status: resolved
      judgment_payload_json:
        residual_risk_acceptance:
          risk_refs:
            - record_kind: blocker
              record_id: BLK-RISK-015
          accepted_scope: ["MVP-1 accepted-risk close path"]
request:
  tool: harness.close_task
  payload:
    envelope:
      request_id: REQ-015
      idempotency_key: IDEMP-015
      expected_state_version: 15
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    intent: complete
    requested_close_reason: completed_with_risk_accepted
    user_note: "Close with the visible accepted risk."
    superseded_by_task_id: null
expected_response:
  base:
    errors: []
  close_state: closed
  closed: true
  close_reason: completed_with_risk_accepted
  assurance_level: self_checked
  residual_risk_state:
    status: accepted
    accepted_refs:
      - record_kind: user_judgment
        record_id: UJ-RISK-015
  acceptance_state:
    status: not_required
    accepted_by_ref: null
    required_before_close: false
  state:
    lifecycle_phase: completed
    result: passed
    close_reason: completed_with_risk_accepted
  blockers: []
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: completed
      result: passed
      close_reason: completed_with_risk_accepted
  blockers:
    BLK-RISK-015:
      status: resolved
expected_storage_rows:
  tasks:
    updated:
      rows:
        - task_id: TASK-001
          lifecycle_phase: completed
          result: passed
          close_reason: completed_with_risk_accepted
  blockers:
    updated:
      rows:
        - blocker_id: BLK-RISK-015
          status: resolved
  user_judgments:
    unchanged:
      rows:
        - user_judgment_id: UJ-RISK-015
          judgment_kind: residual_risk_acceptance
          status: resolved
  tool_invocations:
    inserted:
      rows:
        - tool_name: harness.close_task
          idempotency_key: IDEMP-015
          status: committed
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - Accepted risk는 민감 동작 승인, final acceptance, non-active row/effect, active `assurance_level` 값을 넘는 assurance upgrade를 만들지 않습니다.
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessclose_task
  schema: docs/*/reference/api/schema-core.md#acceptedriskinput
  core: docs/*/reference/core-model.md#close_task
  storage: docs/*/reference/storage.md#fields-needed-for-close-blocker-calculation
  errors: docs/*/reference/api/errors.md#harnessclose_task-close-blockers
```

```yaml
scenario_id: MVP-ACTIVE-display-label-not-canonical
purpose: "`judgment_kind`가 기준 판단 식별자이며 표시 라벨과 지역화된 라벨은 렌더링 문자열일 뿐임을 증명하는 판단 fixture."
initial_state:
  tasks:
    - task_id: TASK-001
      lifecycle_phase: ready
      active_change_unit_id: CU-001
      state_version: 16
  change_units:
    - change_unit_id: CU-001
      task_id: TASK-001
      status: active
request:
  tool: harness.request_user_judgment
  payload:
    envelope:
      request_id: REQ-016
      idempotency_key: IDEMP-016
      expected_state_version: 16
      project_id: PROJ-001
      task_id: TASK-001
      surface_id: reference-local-mcp
      run_id: null
      actor_kind: lead_agent
      dry_run: false
    task_id: TASK-001
    change_unit_id: CU-001
    judgment_kind: product_decision
    presentation: short
    context:
      why_now: "A product copy choice is needed before implementation."
      source_refs:
        - record_kind: task
          record_id: TASK-001
      evidence_refs:
        state_refs: []
        artifact_refs: []
    state_summary_at_request:
      mode: work
      lifecycle_phase: ready
      result: none
      close_reason: none
      assurance_level: none
      gates:
        scope_gate: passed
        decision_gate: required
        approval_gate: not_required
        design_gate: not_required
        evidence_gate: not_required
        acceptance_gate: not_required
    question: "Which settings copy should be used?"
    what_user_is_judging: "Product wording for the settings page."
    why_agent_cannot_decide: "The choice affects product behavior and tone."
    no_decision_consequence: "Implementation waits."
    what_agent_may_decide_without_user: ["Prepare the scoped edit after the decision."]
    affected_scope:
      task_ref:
        record_kind: task
        record_id: TASK-001
      change_unit_ref:
        record_kind: change_unit
        record_id: CU-001
      affected_object_refs: []
      write_refs: []
      close_refs: []
      scope_refs:
        - record_kind: change_unit
          record_id: CU-001
      product_areas: ["settings"]
      files_or_paths: ["app/settings/page.tsx"]
      acceptance_criteria_refs: []
      note: null
    affected_gates:
      - gate: decision_gate
        blocked_action: prepare_write
    affected_acceptance_criteria: []
    judgment_payload:
      options:
        - option_id: concise
          label: "Use concise copy"
          details: null
      recommendation:
        option_id: concise
        reason: "It keeps the narrow change clear."
        uncertainty: null
        when_to_revisit: null
      rationale: "The user owns product wording."
      uncertainty: null
      deferral_consequence: "The write remains blocked."
      user_context: null
      approval_scope: null
      covers: ["Settings copy choice"]
      does_not_cover: ["Sensitive-action approval", "final acceptance", "residual-risk acceptance"]
      acceptance: null
      qa_waiver: null
      verification_risk_acceptance: null
      residual_risk_acceptance: null
      cancellation: null
      separate_judgments_required: []
    expires_at: null
expected_response:
  base:
    errors: []
  user_judgment_id: UJ-016
  user_judgment_ref:
    record_kind: user_judgment
    record_id: UJ-016
  user_judgment:
    user_judgment_id: UJ-016
    task_id: TASK-001
    judgment_kind: product_decision
    presentation: short
    status: pending_user
  state:
    lifecycle_phase: waiting_user
expected_state_changes:
  tasks:
    TASK-001:
      lifecycle_phase: waiting_user
  user_judgments:
    UJ-016:
      judgment_kind: product_decision
      presentation: short
      status: pending_user
expected_storage_rows:
  user_judgments:
    inserted:
      rows:
        - user_judgment_id: UJ-016
          task_id: TASK-001
          change_unit_id: CU-001
          judgment_kind: product_decision
          presentation: short
          status: pending_user
  tasks:
    updated:
      rows:
        - task_id: TASK-001
          lifecycle_phase: waiting_user
  tool_invocations:
    inserted:
      rows:
        - tool_name: harness.request_user_judgment
          idempotency_key: IDEMP-016
          status: committed
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - "`display_label`은 `request.payload`, `expected_response.user_judgment`, `expected_storage_rows`, validator key, gate key, blocker key, state-compatibility input, owner ref, close aggregation, 어떤 기준 식별 field에도 나타나지 않습니다."
  - "지역화된 라벨(`Product decision`, `Technical decision`, `Scope decision`, `제품 판단`, `기술 판단`, `범위 판단`)은 `judgment_kind`에서 파생해 렌더링한 출력으로만 나타날 수 있습니다. 요청의 기준 입력값으로 받지 않으며 compatibility check, validator, gate, blocker, storage identity, state compatibility, close aggregation의 비교값으로 쓰지 않습니다."
  - "대기 중인 판단은 `user_judgment_id=UJ-016`, `judgment_kind=product_decision`, `presentation=short`, `status=pending_user`로만 식별합니다. 렌더링된 라벨로는 이 판단을 해결하거나 변경하지 않습니다."
  - "별도의 permission record, Write Authorization, evidence, final acceptance, residual-risk acceptance, close state, non-active row/effect를 만들지 않습니다."
schema_owners:
  api: docs/*/reference/api/mvp-api.md#harnessrequest_user_judgment
  schema: docs/*/reference/api/schema-core.md#userjudgment
  core: docs/*/reference/core-model.md
  storage: docs/*/reference/storage.md
  errors: docs/*/reference/api/errors.md
```

### Later/Profile Fixture Boundary

Detailed clarification catalog, later-profile verification, full Evidence Manifest case, Manual QA matrix, export non-leakage, Browser QA Capture, full operations recovery/export, broad connector conformance, preventive guard expansion, isolated security profile은 owner가 stage impact와 proof expectation이 있는 더 좁은 fixture를 승격하기 전까지 later/profile 또는 Roadmap material에 남습니다. [향후 Fixtures](../later/future-fixtures.md)에 family가 있다는 사실만으로 내부 엔지니어링 점검이나 MVP-1 requirement가 되지 않습니다.

## Conformance Fixture Format

runtime conformance는 하네스 서버 구현과 fixture materialization 이후에 fixture 기반으로 진행됩니다. 동작 예시 table만으로는 충분하지 않습니다. 구체화된 각 test fixture는 하나의 request를 실행하고 structured response fact, Core state change, storage row, event, artifact, blocker, error, forbidden side effect를 검증해야 합니다.

각 structured fixture 초안은 다음 shape를 포함해야 합니다.

```yaml
scenario_id: string
purpose: string
initial_state: object
request: object
expected_response: object
expected_state_changes: object
expected_storage_rows: object
expected_events: object[]
expected_artifacts: object[]
expected_blockers: object[]
expected_errors: object[]
forbidden_side_effects: string[] | object[]
schema_owners: object
```

Fixture 형태 요약: suite metadata로 fixture를 묶을 수 있지만, fixture 본문은 향후 실행 가능한 conformance를 위한 하나의 정확한 request-and-expectation shape를 유지합니다. 위 YAML 블록이 계약 요약입니다.

향후 fixture file과 suite catalog는 fixture 본문 밖에 metadata를 가질 수 있습니다. Fixture 본문 자체는 위 field만 사용해야 향후 conformance runner가 동작을 일관되게 비교할 수 있습니다. `purpose`는 fixture가 제한하는 동작을 설명하고, `schema_owners`는 public request shape, schema value, Core transition, storage row, error를 검증할 때 쓰는 active owner docs를 이름 붙입니다. 둘 다 public MCP request field가 아니며 Core에 전달되지 않습니다. Suite delivery stage, assertion mode, docs-maintenance result, prose status, rendered Markdown, authoring note를 표현하려고 fixture body field를 추가하지 않습니다. 그런 정보는 suite catalog metadata, docs-maintenance report, display owner, 주변 문서에 둡니다.

Fixture body type notation은 API의 [Schema notation convention](api/schema-core.md#schema-notation-convention)을 따릅니다. 위 top-level fixture body field는 모두 required입니다. Fixture가 empty object, object map, array를 의도적으로 제공할 때는 `{}` 또는 `[]`를 사용합니다. Required top-level field를 생략하면 invalid fixture body이며 "not asserted"가 아닙니다. 내부 엔지니어링 점검과 MVP-1 active draft에서는 projection rendering이 보통 없고, active `expected_storage_rows`가 `projection_jobs`를 요구하면 안 됩니다. 나중에 승격된 owner가 projection freshness를 요구하면, 그 promoted later/profile fixture는 rendered Markdown matching이 아니라 `expected_state_changes.checks`, `expected_storage_rows.projection_jobs`, 또는 owner가 정의한 structured location에 Core/storage fact를 assert합니다.

MCP tool request의 경우 향후 실행 가능한 fixture `request.tool`은 public tool 또는 operator action을 이름 붙이고, `request.payload`는 API docs가 정의하는 해당 tool의 public request object입니다. 내부 엔지니어링 점검과 MVP-1의 활성 fixture 본문은 validation, canonicalization, request hashing, Core execution 전에 `envelope: ToolEnvelope`와 모든 required public request field를 포함해야 합니다. Suite metadata는 작성자가 deterministic envelope value를 고르는 데 도움을 줄 수는 있지만, 그 value가 `request.payload` 안으로 확장되기 전까지 materialized fixture 본문은 invalid입니다. Core가 받는 payload는 surface가 해당 MCP tool에 보낼 public payload와 같으며, fixture를 위한 alternate request schema는 없습니다.

Fixture shorthand는 두 번째 API가 아닙니다. 내부 엔지니어링 점검과 MVP-1의 활성 fixture 본문에서는 public request, seeded owner record, expected state, storage row, event, artifact, blocker, error, ref에 shorthand value를 쓰면 안 됩니다. 이 문서의 사람용 표는 fixture 본문 밖에서 scenario ID와 compact summary를 사용할 수 있습니다. 하지만 materialized active body는 이를 owner-defined record와 public schema로 확장해야 합니다. Later-profile shorthand detail은 [향후 Fixtures: Later-Profile Fixture Shorthand Notes](../later/future-fixtures.md#later-profile-fixture-shorthand-notes)에 두며 내부 엔지니어링 점검 또는 MVP-1의 active requirement가 아닙니다.

`write_authorizations`를 seed하는 향후 실행 가능한 fixture는 valid stored rows를 만들어야 합니다. 각 seeded authorization row는 `basis_state_version`을 명시적으로 포함해야 합니다. 또는 향후 fixture loader가 `state.sqlite`에 insert하기 전에 row의 Task에 대한 seeded affected-scope state version에서 이를 파생해야 합니다. 이는 storage-loader derivation rule일 뿐이며 fixture top-level field를 추가하거나 fixture body shape를 바꾸지 않습니다. Partial `expected_state_changes.write_authorizations` 또는 `expected_storage_rows.write_authorizations` assertions는 idempotent replay, 최신성 감지, expiry, audit behavior를 test하지 않는 한 `basis_state_version`을 생략할 수 있습니다. `basis_state_version`은 `decision=allowed` basis이지 resulting `ToolResponseBase.state_version`이 아닙니다. 향후 fixture loader는 `blocked`, `approval_required`, `decision_required`, `state_conflict` outcome을 `write_authorizations` row로 seed하면 안 됩니다. 그런 outcome에는 response decision, blocker, validator finding, error를 사용합니다.

Suite catalog metadata는 Core에 전달되지 않으며 fixture body의 일부가 아닙니다. Suite, delivery stage, tag별로 exact-shape fixture를 묶을 수 있습니다.

```yaml
suite: agency
earliest_delivery_stage: "Assurance Profile"
tags: [decision-gate, residual-risk, autonomy-boundary]
fixtures:
  - AGENCY-user-judgment-required-before-product-tradeoff-write
  - AGENCY-residual-risk-visible-before-acceptance
```

향후 runner는 이 metadata를 suite 선택, 순서 지정, reporting에 사용할 수 있습니다. Core에는 `request.tool`과 public `request.payload`만 전달됩니다. Metadata가 seed expansion, fixture comparison semantics, tool request schema, expected owner records를 바꾸면 안 됩니다.

## Conformance Execution

향후 `harness conformance run`은 MCP tool과 operator command가 사용하는 것과 같은 Core entrypoint를 통해 fixture를 실행합니다. 동작을 prose output만 검사해서 검증하면 안 됩니다. Core entrypoint를 실행한 뒤 response fact, state, storage row, event, artifact, blocker, 관련되는 경우 projection fact, error, forbidden side effect를 비교해야 합니다.

향후 runtime fixture execution 의미:

1. Fixture YAML file을 읽고 exact fixture body shape, canonical active value, public `request.payload` schema, `fixture-only shorthand`가 없는지를 검증합니다.
2. Fixture가 existing read-only sample을 명시적으로 target하지 않는 한 fresh fixture-only 하네스 런타임 홈과 임시 제품 저장소를 만듭니다. 여기서 fixture isolation은 deterministic comparison을 위한 테스트 위생입니다. `isolated` guarantee level, OS sandboxing, 권한 격리, 변조 방지 storage claim이 아닙니다. 향후 runner는 state-changing fixture execution에 developer의 실제 하네스 런타임 홈이나 제품 저장소를 재사용하면 안 됩니다.
3. `initial_state`에서 `registry.sqlite`, `project.yaml`, `state.sqlite`, artifact file, connector manifest를 seed합니다. Projection file은 projection requirement가 승격된 later/profile fixture에 한해서 seed합니다.
4. Core를 통해 `request.tool`을 execute합니다. MCP tool action은 public request schema를 사용합니다. Fixture `request.payload`는 접점이 해당 MCP tool에 보낼 request payload와 같아야 합니다. `projection_refresh`, `doctor_surface`, `recover`, `artifacts_check` 같은 operator action은 [운영과 Conformance 참조](operations-and-conformance.md)의 operator semantics를 사용합니다.
5. Returned response fact, resulting state summary, storage effect, 추가된 owner event, emitted validator result, artifact registry/file integrity, structured blocker, 관련되는 경우 projection job status와 reconcile item, returned error code를 capture합니다.
6. Captured result를 `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, `forbidden_side_effects`와 compare합니다. 비어 있는 expected section은 해당 section에 관련 effect가 없음을 단언합니다.
7. Fixture id, pass/fail, observed response/state/storage/event/artifact/blocker/error summary, 관련되는 경우 projection freshness, forbidden-side-effect comparison을 보고합니다.

향후 runner 순서 요약: 위 번호 목록이 계약 요약입니다. 향후 runner는 exact fixture body를 읽고 fixture-only runtime home을 seed한 뒤 Core를 통해 request를 실행합니다. 그런 다음 response/state/storage/events/artifacts/blockers/errors/forbidden side effects를 비교해 report를 냅니다.

Fixture `request.payload.envelope`가 `expected_state_version`을 포함하면 향후 runner는 `ToolEnvelope.task_id`만이 아니라 Core-resolved primary Task에 따라 비교합니다. Primary Task resolution order는 tool-specific `task_id`, `ToolEnvelope.task_id`, active Task resolution 순서입니다. Task-scoped actions는 seeded 또는 Core-resolved primary Task State Version과 비교하고, resolved primary Task가 없는 project-scoped actions는 Project State Version과 비교합니다. Captured response, `EventRef.state_version`, `task_events.state_version` values는 resulting affected-scope versions로 비교합니다. Read-only fixtures는 primary read scope의 unchanged version을 검증할 수 있습니다. 이 설명은 fixture body shape를 바꾸지 않고 comparison 의미만 명확히 합니다.

Stale `expected_state_version` fixture는 단순한 concurrent-write test가 아니라 stale-authority test입니다. Exact idempotent replay는 예외입니다. Committed replay row가 있고 canonical request hash가 일치하면 fixture는 original committed response가 반환되고 current state-version freshness check가 다시 실행되지 않았음을 검증해야 합니다. Replay row가 없고 state-changing action이 commit 전에 conflict되면, owner document가 다른 recovery action을 명시하지 않는 한 fixture는 current record 변경 없음, `task_events` append 없음, artifact 등록 없음, projection job enqueue 없음, conflicting request를 위한 `tool_invocations` replay row 생성 없음까지 검증해야 합니다. 같은 key가 changed canonical request hash와 함께 재사용되면 fixture는 `STATE_CONFLICT`, original replay row 보존, 새 artifact/event/projection job/response field/owner relation이 merge되지 않음을 검증해야 합니다. `dry_run=true` fixture는 diagnostic 또는 `would_create` effect가 반환되어도 current record, `task_events`, artifact, consumable Write Authorization, projection job, `tool_invocations` replay row가 생기지 않고, 나중에 non-dry-run call을 보낼 때 key가 이미 예약된 것으로 처리되지 않음을 검증해야 합니다. Replayed `prepare_write`는 duplicate authorization을 만들면 안 됩니다. Replayed `record_run`은 authorization을 두 번 consume하면 안 됩니다.

Fixture execution은 deterministic해야 합니다. Network access, wall-clock-sensitive expiry, external tool output은 suite가 integration smoke라고 명시적으로 선언하지 않는 한 stub하거나 seeded fixture input으로 표현해야 합니다.

Fixture isolation은 pass 조건의 일부입니다. Fixture는 임시 제품 저장소와 하네스 런타임 홈에 file을 seed하고, 그곳에서 하나의 Core 또는 operator action을 실행한 뒤 captured result를 비교할 수 있습니다. 이것은 product guarantee level을 올리지 않습니다. Existing local runtime record, generated operational file, 이전 실행의 prose report에 의존하면 안 됩니다.

Seed validation은 action execution 전에 수행하고, captured-state validation은 action execution 이후에 수행합니다. 비교의 양쪽은 fixture-local string label이 아니라 owner-defined state loader와 value set을 사용합니다.

향후 conformance runner는 MCP tool과 operator command가 사용하는 동일한 Core storage loader를 통해 JSON `TEXT` field를 seed하고 검사해야 합니다. `initial_state`에 malformed JSON 또는 schema-incompatible JSON이 있는 fixture는 유효하지 않은 상태를 드러내야 합니다. Fixture action이 recovery path이고 safe reconstruction이 가능한 경우에는 복구 가능한 state issue를 드러내야 합니다. 향후 runner는 JSON field를 opaque string으로 취급해서 shape validation을 건너뛰면 안 됩니다. 이 기대사항은 fixture body shape를 바꾸지 않습니다.

향후 conformance runner는 status-like `TEXT` field도 [Storage](storage.md#canonical-enum-hardening)의 owner-bound hardening map을 통해 seed하고 검사해야 합니다. 주요 내부 엔지니어링 점검 / MVP-1 path에서 향후 fixture seed loader는 active stage의 seeded record에 실제로 들어가는 owner value만 검증하고, artifact/ref enum assertion은 API [stage-specific active value sets](api/schema-core.md#stage-specific-active-value-sets)를 사용합니다. 예를 들면 registry/project surface guarantee, Run kind/status, Write Authorization status/guarantee, active judgment path가 있을 때의 민감 동작 승인 user-judgment status, evidence support가 active일 때의 minimal evidence summary coverage/status, risk visibility가 active일 때의 residual-risk visibility/status, 해당 owner record를 사용할 때의 current Task 또는 Change Unit status입니다. Projection job kind/status는 projection owner가 durable projection-job storage를 승격한 later/profile fixture에만 속합니다. Committed Approval record lifecycle status와 full Evidence Manifest status는 later/profile-gated입니다. Later-profile status field는 그 profile이 active가 되기 전까지 promoted owner docs와 future catalog에 남습니다. 유효하지 않은 state recovery를 명시적으로 test하는 scenario가 아닌 한 unknown status value는 계속 invalid입니다. Expected-state status assertion은 prose label이 아니라 captured owner value를 비교합니다.

## Fixture Assertion Semantics

Fixture assertion mode는 runner default 또는 suite catalog metadata로 정합니다. Core input이 아니고 MCP tool에 전달되지 않으며 fixture body에 field를 추가하면 안 됩니다. Fixture body는 정확히 `scenario_id`, `purpose`, `initial_state`, `request`, `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, `forbidden_side_effects`, `schema_owners`만 유지합니다.

Partial assertion object 안에서 omission은 "not asserted"를 뜻합니다. Value가 `null`인 listed field는 captured field가 present이고 JSON `null`과 같음을 assert합니다. Listed array value `[]`는 present empty array를 assert합니다. Owner schema가 해당 field를 map이라고 말하는 경우 listed object-map value `{}`는 present empty map을 assert합니다. `partial_deep` 아래의 structured object에서는 object 존재만 의도적으로 assert하는 경우가 아니라면 fixture author는 최소 하나의 child field를 나열해야 합니다.

이 omission rule은 assertion rule일 뿐입니다. Public MCP `request.payload`에서 omitted field를 valid로 만들지 않습니다. Fixture `request.payload`는 owning public request schema를 통과해야 합니다.

기본 comparison mode:

| Fixture field | Default assertion mode |
|---|---|
| `expected_response` | `partial_deep`; 나열된 response field, ref, decision, state version, primary-error summary가 재귀적으로 일치해야 합니다. Rendered prose만 맞춰서는 안 됩니다. |
| `expected_state_changes` | `partial_deep`; 나열된 Core-owned record change가 재귀적으로 일치해야 하며 나열되지 않은 field는 검증하지 않습니다. Suite metadata가 `expected_state_changes: exact`로 설정할 수 있습니다. |
| `expected_storage_rows` | `table_effects`; 나열된 table insert/update/delete/no-change count와 row filter가 captured storage effect와 일치해야 합니다. Suite metadata가 selected table에 exact table effect를 설정할 수 있습니다. |
| `expected_events` | Captured `task_events`의 stable-catalog projection에 대한 `contains_ordered`; 나열된 stable event는 ascending `task_events.event_seq` 순서대로 나타나야 하며 unrelated stable event가 앞, 사이, 뒤에 있어도 됩니다. Suite metadata가 `expected_events: exact`로 설정할 수 있습니다. |
| `expected_artifacts` | `contains_by_identity`; 나열된 각 artifact는 같은 `artifact_id`와 `kind`를 가진 등록된 아티팩트와 일치해야 하며, 그 밖에 나열된 artifact field는 재귀적으로 일치합니다. |
| `expected_blockers` | `contains_by_kind_and_code`; 나열된 각 blocker는 blocker kind와, code가 나열된 경우 API code가 같은 structured response 또는 Core/storage blocker와 일치해야 합니다. |
| `expected_errors` | `contains_primary_ordered`; `expected_errors: []`는 returned API error가 없음을 assert합니다. Object가 나열되면 `code`는 required이며 [Primary Error Code Precedence](api/errors.md#primary-error-code-precedence)가 선택한 primary API `ErrorCode`와 exact match해야 합니다. Secondary error를 명시하려면 owner-defined details 아래에 둡니다. |
| `forbidden_side_effects` | Captured state, storage, events, artifacts, projections, generated outputs, secret handling에 대한 negative assertion입니다. Draft는 readable string을 쓸 수 있습니다. Materialized executable fixture는 가능한 곳에서 owner-record absence check로 확장해야 합니다. |

`expected_events`는 기본적으로 `contains_ordered`이므로 `expected_events: []`는 fixture가 특정 stable event를 요구하지 않는다는 뜻입니다. 이것만으로 captured stable-event stream이 empty임을 assert하지 않습니다. Stable event가 없었음을 assert하려면 suite metadata에서 해당 fixture 또는 suite에 `expected_events: exact`를 설정해야 합니다. `expected_artifacts: []`, `expected_blockers: []`, `expected_errors: []`도 default mode에서는 해당 required entry가 없다는 뜻입니다. Absence 자체가 증명 대상이면 compatible exact-mode metadata나 `forbidden_side_effects`를 사용합니다.

`expected_events` comparisons는 captured `task_events`의 [Core Model Stable Event Catalog](core-model.md#stable-event-catalog) projection을 대상으로 합니다. API tool detail/audit event lists는 이 set을 확장하지 않습니다. `task_events`에 capture된 non-catalog detail 또는 local-audit events는 normal staged-delivery fixture를 fail하게 만들면 안 됩니다. Suite metadata가 `expected_events: exact`로 설정하면, future 로드맵/local suite가 implementation-specific detail-event assertions를 명시적으로 opt in하지 않는 한 exactness는 captured stream의 stable-event projection에 적용됩니다. Validator IDs, Core check names, projection status note, fixture authoring label, scenario catalog IDs는 event names가 아닙니다. Prose examples는 non-catalog event names를 illustrative 또는 future extension ideas로 언급할 수 있지만, 실행 가능한 staged-delivery fixture는 Core Model event catalog가 승격하기 전까지 이를 요구하면 안 됩니다.

향후 conformance runner는 captured `task_events`를 `event_seq`로 order합니다. `state_version`, `created_at`, `event_id`는 `expected_events` ordering의 tie-breaker가 아닙니다.

Fixture authors는 API precedence가 generic validator fallback을 선택할 때만 `VALIDATOR_FAILED`를 `expected_errors[].code`로 사용해야 합니다. `EVIDENCE_INSUFFICIENT`, readable-view freshness request의 `PROJECTION_STALE`, `ARTIFACT_MISSING` 같은 더 specific한 active typed code가 적용되면 그 code가 primary입니다. `PROJECTION_STALE`은 active MVP close blocker가 아니며, QA-specific code는 owner가 승격하기 전까지 later/profile material입니다.

`CloseTaskResponse.blockers[].code` 역시 API `ErrorCode` value입니다. Policy-specific 또는 validator-specific finding code는 `expected_state_changes.validators`, validator finding assertion, 또는 equivalent expected validator output 아래에 두어야 하며, `expected_errors[].code`나 close blocker `code`에 두면 안 됩니다. Blocked close를 다루는 fixture는 `expected_blockers` 아래 structured blocker를 assert해야 합니다. Committed state change가 기대되는 경우 captured equivalent를 `expected_state_changes.close_blockers` 또는 `expected_storage_rows.blockers`에도 둡니다. Report prose, Journey Card text, status text, agent summary만 맞춰서는 close blocker를 증명할 수 없습니다.

`expected_state_changes.validators` 아래의 validator assertion은 validator ID로 keyed됩니다. 나열된 각 validator ID는 captured validator results에 존재해야 하며 나열된 field와 부분적으로 일치해야 합니다. 나열되지 않은 validator ID와 나열되지 않은 validator field는 검증하지 않습니다.

Fixture가 design-quality impact를 검증할 때는 모든 관련 validator finding을 `expected_state_changes.validators` 아래 보이게 유지해야 합니다. Fixture는 policy-owned [Severity Composition Rule](design-quality-policies.md#severity-composition-rule)과 [활성 MVP 영향 기본값](design-quality-policies.md#활성-mvp-영향-기본값)이 산출한 merged impact class, routed action, gate, write-blocker, close-blocker, waiver, user judgment outcome을 검증합니다. Fixture는 policy schema를 추가하거나, 새 action value를 만들거나, 더 강한 merged blocker가 있다는 이유만으로 lower-severity finding을 숨기거나, Advisory/later catalog finding을 MVP blocker로 취급하면 안 됩니다.

`expected_state_changes.checks` 아래의 Core check와 precondition assertion은 check/precondition name을 key로 사용합니다. 이 entry는 captured Core check output, blocked reason, response summary, 또는 runner가 관찰한 equivalent check status와 비교합니다. [API Schema Core](api/schema-core.md#validatorresult), [API Schema Later](api/schema-later.md#validatorresult-stable-ids), [Storage](storage.md)가 해당 ID를 stable `ValidatorResult`로 명시적으로 승격하지 않는 한 이 값들은 validator ID가 아니며 `expected_state_changes.validators` 아래에 두면 안 됩니다.

`expected_state_changes.checks.projection_freshness`는 promoted owner가 이 check를 범위에 넣었을 때 Core mechanical projection freshness check를 검증합니다. `expected_state_changes.validators.context_hygiene_check`는 higher-level context hygiene에 대한 stable ValidatorResult를 검증합니다. 그 validator가 projection freshness를 고려할 수는 있지만, mechanical check 자체의 fixture assertion 위치는 아닙니다.

`secret_omitted` 또는 `blocked` artifact를 다루는 fixture는 committed artifact가 있다면 `redaction_state`를 `expected_artifacts` 아래에서 검증하고, storage effect는 `expected_storage_rows`, downstream evidence 또는 blocker effect는 `expected_state_changes`와 `expected_blockers`에서 검증해야 합니다. Fixture는 생략된 secret 또는 PII 값을 assert하면 안 됩니다. Export, Release Handoff, full Evidence Manifest, Manual QA, Eval, detached verification, broad artifact non-leakage case는 승격 전까지 later/profile catalog material입니다.

Artifact redaction, blocked-input, integrity, export non-leakage scenario family는 향후 catalog inventory입니다. [향후 Fixtures: Artifact Redaction And Export Non-Leakage Catalog Entries](../later/future-fixtures.md#artifact-redaction-and-export-non-leakage-catalog-entries)를 봅니다.

Projection assertion은 projection support가 범위에 있을 때 owner-defined freshness, enqueue status, source-state-version display, 관련 job fact만 비교합니다. 이 assertion은 `expected_state_changes`, `expected_storage_rows`, 또는 owner가 정의한 다른 structured field에 둡니다. Rendered Markdown을 비교하지 않습니다. Projection failure가 captured Core state와 event를 rollback하거나 rewrite하게 만들면 안 됩니다.

Suite catalog는 fixture를 바꾸지 않고 assertion mode를 override할 수 있습니다.

```yaml
suite: core
assertion_modes:
  expected_state_changes: exact
  expected_storage_rows.tasks: exact
  expected_events: exact
  expected_errors.details: exact
fixtures:
  - MVP-ACTIVE-task-change-unit-setup
```

향후 conformance는 captured response field, Core state, storage row, `task_events`, validator result, artifact registry/file integrity, 승격된 경우 projection job 또는 freshness state, returned error code, structured blocker, forbidden-side-effect check를 통해 동작을 증명해야 합니다. Rendered Markdown, Journey Card prose, status prose, close report prose, agent prose만 맞춰서는 fixture를 통과시킬 수 없습니다.

향후 fixture runner는 `request_hash`, baseline `tree_hash`, projection `managed_hash`에 대해 reference implementation과 같은 정규화 rule을 사용해야 합니다. 세부 알고리즘은 [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [Storage](storage.md), [Projection과 Template 참조](projection-and-templates.md)가 담당합니다. Conformance fixture는 그 기준 기록 경계를 다시 정의하지 않고 deterministic behavior를 검증합니다.

## Fixture 현재 단계 상태

현재 저장소는 문서 전용입니다. 이 문서 batch는 실행 가능한 fixture file, 실행 가능한 fixture catalog file, generated projection, runtime state, database, 하네스 서버 conformance test를 만들지 않습니다.

MVP structured draft와 fixture 작성 queue는 향후 작성 계획입니다. 문서 수락, 별도의 구현 계획 준비 결정, 하네스 서버 구현, 명시적인 fixture materialization step이 있은 뒤에야 실행 가능한 상태가 됩니다. 문서 점검은 Markdown drift를 보고할 수 있지만 runtime conformance가 아니며 Core fixture result를 만들지 않습니다.

## Catalog-Only Fixture Skeleton Guidance

Catalog skeleton guidance는 승격된 향후 catalog family를 exact-shape fixture로 옮길 때 쓰는 지침입니다. Executable fixture body, public request schema, DDL extension, runner design, stage-exit requirement가 아닙니다. Delivery-stage mapping은 suite catalog metadata에 두며 fixture body에 넣지 않습니다. "Minimum seeded records"는 Storage 규칙으로 expansion 및 validation을 거친 뒤 `initial_state`에 들어가는 owner record를 뜻합니다. Public mutation은 계속 정확한 MCP request payload를 `request.payload`로 사용합니다.

향후 scenario family 목록은 [향후 Fixtures](../later/future-fixtures.md)에 있습니다.

## Kernel Smoke Authoring Queue

이 queue는 [Kernel Smoke 동작 예시](#engineering-checkpoint-behavior-examples)를 위한 향후 작성 지침입니다. Kernel Smoke는 첫 내부 권한 루프를 위한 좁은 작성 label이지 제품 MVP, 전체 conformance suite, 향후 fixture catalog가 아닙니다. 이 row들은 실행 가능한 fixture file이 이미 존재한다고 암시하지 않습니다. Compact authoring order일 뿐이며, 첫 구현 계획은 Build가 이름 붙인 하나의 권한 루프를 증명하는 가장 작은 subset만 구체화할 수 있습니다.

Kernel Smoke는 projection requirement 없음이 기본입니다. Minimal owner path가 이미 그런 fact를 만들고 target behavior 증명에 도움이 될 때만 projection freshness 또는 enqueue/failure fact를 검증할 수 있습니다. Projection-template polish, detailed report template, 여러 projection kind, Browser QA Capture, export/recover, reconcile, stewardship, context hygiene, full operations, future guarantee-level fixture는 owner docs가 특정 좁은 path를 나중에 승격하지 않는 한 내부 엔지니어링 점검 밖에 둡니다.

표에서 `None`은 matching draft field가 `[]`, `{}`, 또는 그 밖의 empty value로 남는다는 뜻입니다. 새 sentinel value가 아닙니다.

| Queue | Fixture draft family | Request path | Minimum seeded records | Required structured assertion | Expected blockers/errors | 보존해야 하는 forbidden side effects |
|---|---|---|---|---|---|---|
| 1 | `MVP-ACTIVE-task-change-unit-setup` | `harness.intake` | Current Task가 없는 등록된 local project | Task `tasks.lifecycle_phase=ready` 하나, Change Unit 또는 scope boundary 하나, current-task pointer, write authority 없음. | None | Run, artifact, evidence, final acceptance, residual-risk acceptance, close, authority-rendering effect 없음. |
| 2 | `MVP-ACTIVE-shaping-update-persists` | `kind=shaping_update`, `payload.kind=shaping_update`, active payload branch로 표현한 `product_write=false`의 `harness.record_run` | Task `tasks.lifecycle_phase=shaping`와 Change Unit | Shaping update가 Task/Change Unit state와 `runs.kind=shaping_update` row에 저장되고 product-write authority는 만들지 않음. | None | Write Authorization, product-write Run, non-active row/effect, final acceptance, residual-risk acceptance 없음. |
| 3 | `MVP-ACTIVE-prepare-write-allowed-authorization` | `harness.prepare_write` | Task `tasks.lifecycle_phase=ready`, compatible scope, current expected state, public request 안의 proposed attempt-scope fields | `decision=allowed`, `tasks.lifecycle_phase=ready`, `attempt_scope_json`이 `WriteAuthorizationSummary.attempt_scope`와 일치하는 active Write Authorization 하나, replay row, Run 없음. | None | OS permission, sandbox, preventive, isolated, evidence, close claim 없음. |
| 4 | `MVP-ACTIVE-prepare-write-blocked-no-authorization` | `harness.prepare_write` | Task `tasks.lifecycle_phase=ready`와 incompatible requested path 또는 compatible scope 누락 | Structured blocked response, Task `tasks.lifecycle_phase=blocked`, `write_authorization_ref=null`, `write_authorization=null`, consumable Write Authorization row 없음. | API/Core path가 소유한 `SCOPE_REQUIRED`, `NO_ACTIVE_CHANGE_UNIT`, 또는 `SCOPE_VIOLATION`. | Authorization, Run, artifact, evidence mutation, non-active effect, close, final acceptance, residual-risk acceptance 없음. |
| 5 | `MVP-ACTIVE-prepare-write-idempotent-replay` | `harness.prepare_write` replay | Existing committed replay row, original stored `request_hash`, original active authorization | Original response, original stored `request_hash`, original `write_authorization_ref`, `authorization_effect=returned`를 반환. | None | Duplicate authorization, event, artifact, replay update, non-active effect, state-version increment 없음. |
| 6 | `MVP-ACTIVE-idempotency-key-hash-conflict` | 같은 idempotency key와 다른 hash를 쓰는 state-changing tool | Existing committed replay row | `STATE_CONFLICT`; original replay row와 stored `request_hash`는 unchanged. | `STATE_CONFLICT` | Merged response, new authorization, event, artifact, non-active effect, owner relation, replay row update 없음. |
| 7 | `MVP-ACTIVE-record-run-consumes-authorization` | `kind=implementation`, `payload.kind=implementation`, `payload.implementation`만 non-null인 `harness.record_run` | Task `tasks.lifecycle_phase=ready`, compatible scope, observed attempt와 맞는 stored `AuthorizedAttemptScope`를 가진 active Write Authorization | Compatible `observed_attempt_json`이 있는 Run 하나가 기록되고 authorization을 정확히 한 번 소비합니다. Task execution assertion은 `tasks.lifecycle_phase=executing`을 사용합니다. | None | Second consumption, final acceptance, residual-risk acceptance, non-active assurance state, close 없음. |
| 8 | `MVP-ACTIVE-record-run-missing-authorization-blocked` | `kind=implementation`, `payload.kind=implementation`, `payload.implementation`만 non-null이고 `write_authorization_id=null`인 `harness.record_run` | Task `tasks.lifecycle_phase=ready`와 authorization 없는 product-write Run request | Product-write Run은 commit 전에 reject되고 저장된 Task state는 바뀌지 않습니다. | Details가 reason을 assert할 때 `authorization_reason=missing`이 있는 `WRITE_AUTHORIZATION_REQUIRED` | Run, authorization consumption, completion evidence, artifact link, evidence mutation, non-active effect, event, state-version advance, replay row 없음. |
| 9 | `MVP-ACTIVE-record-run-observed-out-of-scope` | `kind=implementation`, `payload.kind=implementation`, `payload.implementation`만 non-null인 `harness.record_run` | Stored `AuthorizedAttemptScope`가 observed path, command, network target, secret, sensitive category, baseline, Task, Change Unit, 또는 surface를 제외하는 active Write Authorization | Active draft에서는 out-of-scope observation을 commit 전에 reject하고 authorization을 success로 consume하지 않습니다. | `SCOPE_VIOLATION` | Authorization은 consumed되지 않음. Run, artifact, evidence mutation, replay row, completion evidence, close readiness, non-active row 없음. |
| 10 | `MVP-ACTIVE-raw-secret-artifact-blocked` | `kind=direct`, `payload.kind=direct`, `payload.direct`만 non-null이고 `write_authorization_id=null`, `product_write=false`, active `ArtifactInput`을 쓰는 `harness.record_run` | Forbidden raw-secret evidence를 시도하는 Task `tasks.lifecycle_phase=executing`, Run path, active `ArtifactInput` shape | Active draft에서는 raw secret bytes를 mutation 전에 reject합니다. 별도 committed metadata-notice branch는 artifact/storage/error assertion도 같은 branch로 맞춰야 합니다. | Forbidden input shape/source 또는 raw secret payload before mutation은 `VALIDATION_FAILED`; missing 또는 integrity-failed committed artifact ref에만 `ARTIFACT_MISSING`. | Raw secret storage, rendering, export, evidence sufficiency, authorization consumption, close 없음. |
| 11 | `MVP-ACTIVE-evidence-summary-insufficient` | `harness.status` | Partial/missing evidence summary와 active evidence blocker가 있는 Task `tasks.lifecycle_phase=blocked` | Evidence summary는 `partial`로 남고 close-relevant evidence blocker가 mutation 없이 계속 보입니다. | Close/write path가 의존할 때 `EVIDENCE_INSUFFICIENT` blocker | Status prose 또는 Markdown evidence list가 missing refs를 repair하거나 artifact를 만들거나 Task를 close하지 않음. |
| 12 | `MVP-ACTIVE-evidence-summary-sufficient` | `kind=implementation`, `payload.kind=implementation`, `payload.implementation`만 non-null이고 active `ArtifactInput`을 쓰는 `harness.record_run` | Task `tasks.lifecycle_phase=executing`, observed attempt와 맞는 stored `AuthorizedAttemptScope`를 가진 compatible authorization, 그리고 redaction이나 omission이 필요하지 않아 `redaction_state=none`으로 허용되는 non-secret staged artifact | Registered artifact refs와 evidence summary가 owner records에서 sufficient 상태가 됩니다. Task는 close 전까지 `tasks.lifecycle_phase=executing`으로 남습니다. | None | Non-active evidence/assurance row, final acceptance, residual-risk acceptance, close state 없음. |
| 13 | `MVP-ACTIVE-final-acceptance-missing-close-blocker` | `harness.close_task` (`intent=complete`, `requested_close_reason=completed_self_checked`) | Evidence는 sufficient지만 required final acceptance가 없는 Task | Close가 final-acceptance blocker로 blocked 상태를 유지하고 terminal Task update를 만들지 않습니다. | `ACCEPTANCE_REQUIRED` | `tasks.lifecycle_phase=completed` 또는 `tasks.lifecycle_phase=cancelled`, fabricated acceptance, residual-risk acceptance, non-active assurance state, close report authority 없음. |
| 14 | `MVP-ACTIVE-residual-risk-visible-not-accepted-blocker` | `harness.close_task` (`intent=complete`, `requested_close_reason=completed_with_risk_accepted`) | Visible close-relevant residual risk가 있고 compatible `judgment_kind=residual_risk_acceptance` user judgment가 없는 Task | Residual-risk acceptance가 계속 required이고 close가 Task를 terminal로 표시하지 않습니다. | `required_judgment_kind=residual_risk_acceptance`가 있는 `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED` | Visible risk는 accepted risk가 아님. Non-active risk/assurance row 또는 close state를 fabricate하지 않음. |
| 15 | `MVP-ACTIVE-accepted-risk-close` | `harness.close_task` (`intent=complete`, `requested_close_reason=completed_with_risk_accepted`) | Sufficient evidence, visible risk, compatible `judgment_kind=residual_risk_acceptance`가 있는 Task | Task가 `tasks.lifecycle_phase=completed`, accepted-risk close reason, user judgment ref로 close됩니다. | None | Accepted risk가 Approval, final acceptance, non-active assurance state, assurance upgrade를 만들지 않음. |
| 16 | `MVP-ACTIVE-display-label-not-canonical` | `harness.request_user_judgment` | Task `tasks.lifecycle_phase=ready`, Change Unit, 이 request에 대한 기존 committed user judgment 없음 | Committed non-dry-run request가 `judgment_kind=product_decision`, `presentation=short`, `status=pending_user`인 pending `user_judgments` row 하나와 response `UserJudgment`를 만듭니다. `display_label`, Product decision, Technical decision, Scope decision, `제품 판단`, `기술 판단`, `범위 판단` 같은 문구는 렌더링 문구일 뿐입니다. | None | `display_label`과 지역화된 라벨은 authoritative request input, 기준 상태, validator key, gate key, blocker key, storage identity, state-compatibility input, compatibility check, close aggregation key가 아닙니다. |

위 queue는 의도적으로 작습니다. 내부 엔지니어링 점검은 전체 conformance suite, broad catalog family coverage, final-acceptance success semantics, later/profile 보증 확인, export/recover, reconcile, stewardship, context hygiene, Browser QA Capture, future guarantee-level check를 요구하지 않습니다. MVP-1은 나열된 user-loop judgment, evidence, close-blocker, accepted-risk draft를 더하지만 later/profile 보증 확인, export, profile fixture를 승격하지 않습니다.

## 향후 fixture catalog

Scenario family는 이 early reference가 핵심 적합성 모델에 집중하도록 [향후 Fixtures](../later/future-fixtures.md)로 이동했습니다. 그 catalog에는 Browser QA Capture, cross-surface behavior, export non-leakage, context hygiene, reconcile, stewardship, full operations, advanced projection rendering, artifact redaction/integrity, future guarantee-level check를 위한 간결한 향후 목록이 있습니다.

그 catalog entry는 promoted owner path가 exact-shape의 실행 가능한 fixture로 구체화하기 전까지 design inventory일 뿐입니다. 내부 엔지니어링 점검 requirement가 아니며, 그 자체로 MVP-1을 확장하지 않고, 이 저장소가 문서 전용인 동안 runtime conformance로 계산하지 않습니다.

## Metrics Boundary

Long-term operational metrics는 derived analytics이며 staged-delivery-critical state나 conformance requirement가 아닙니다. Approval turnaround, verification latency, projection stale duration, same-session guard frequency, surface fallback rate 같은 metric은 future version이 owner docs, fixture 또는 conformance target, fallback behavior, 관련 redaction/retention policy, projection-as-canonical dependency 없음, implementation ownership과 함께 승격하기 전까지 [roadmap](../roadmap.md)에 read-only diagnostic으로 둡니다.
