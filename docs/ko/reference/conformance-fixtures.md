# Conformance Fixtures 참조

## 이 문서로 할 수 있는 일

Harness conformance material의 세 층을 구분해 볼 때 이 참조 문서를 사용합니다. 세 층은 문서 점검, 활성 structured fixture draft, 향후 runtime conformance입니다. 이 문서는 향후 conformance가 무엇을 증명하는지, 활성 Kernel Smoke, MVP-1 user-loop, security/capability, artifact/evidence draft family, exact structured fixture draft shape, future runner execution behavior, fixture assertion semantics, 현재 단계 상태, 향후 fixture catalog와의 경계를 설명합니다.

이 문서는 conformance author, implementer, maintainer를 위한 lookup 문서입니다. 운영자 절차 문서가 아니므로 operator entrypoint와 `harness conformance run` overview는 [운영과 Conformance 참조](operations-and-conformance.md)를 사용합니다.

이 문서는 향후 conformance work를 위한 참조 문서입니다. 현재 저장소는 문서 전용이며 실행 가능한 Harness Server conformance test를 담고 있지 않습니다. 현재 단계와 인계 상태는 [구현 개요](../build/implementation-overview.md#문서-수락-상태)에 있습니다.

## 이런 때 읽기

- 향후 fixture 기반 conformance 설계를 작성하거나 리뷰할 때.
- 정확한 fixture body field, fixture shorthand boundary, `ToolEnvelope` expansion convention, runner isolation behavior가 필요할 때.
- Response fact, Core state, storage row, event, artifact, blocker, error, forbidden side effect, 승격된 경우 projection fact를 위한 fixture assertion mode가 필요할 때.
- 활성 Kernel Smoke, MVP-1 사용자 작업 루프, security/capability, artifact/evidence fixture draft, 또는 이 draft와 향후 fixture catalog 사이의 경계가 필요할 때.

## 읽기 전에

Conformance run entrypoint, suite-selection overview, docs-maintenance profile boundary, operator procedure는 [운영과 Conformance 참조](operations-and-conformance.md#conformance-run)를 사용합니다. Public request/response schema는 [MVP API](api/mvp-api.md)와 [API Schema Core](api/schema-core.md), storage layout과 seed-loader owner value는 [Storage](storage.md), state transition과 stable event 의미는 [Core Model 참조](core-model.md), projection freshness는 [Projection과 Template 참조](projection-and-templates.md), policy validator behavior는 [설계 품질 정책](design-quality-policies.md), connector conformance overview는 [Agent 통합 참조](agent-integration.md)를 사용합니다.

## 핵심 생각

현재 이 문서는 실행 가능한 테스트 모음이 아니라 향후 runtime conformance 계획입니다. 이후 구현 계획에서 쓸 동작 예시 ID와 필요한 동작을 정의할 뿐이며 fixture file, runner code, generated output, runtime state, 실행 가능한 Harness Server conformance suite를 만들지 않습니다. 문서 전용 단계에서는 이 예시에서 실제 fixture 파일을 만들지 않습니다.

세 층을 항상 구분합니다.

- 문서 점검은 Markdown 문서에 대한 읽기 전용 편집 점검입니다. Link integrity, terminology consistency, stage boundary, security wording, user-language check, owner-boundary drift, 영어/한국어 의미 일치를 봅니다. Markdown drift를 보고할 수 있지만 fixture action을 실행하거나, `task_events`를 append하거나, artifact를 만들거나, projection을 refresh하거나, QA 또는 acceptance state를 만들거나, close readiness에 영향을 주거나, implementation readiness 또는 runtime result를 만들지 않습니다.
- Active MVP fixture draft는 내부 엔지니어링 점검과 MVP-1을 위한 작은 structured 설계 초안입니다. Assertion field로 기대 동작을 설명하지만 아직 실행 가능한 fixture가 아니며 generated runtime artifact도 아닙니다.
- runtime conformance는 향후 Harness Server 구현 작업입니다. 구현된 Core/API/storage/surface behavior에 적용되며 documentation prose가 아니라 실행 가능한 fixture와 structured assertion으로 판단합니다. Server implementation과 fixture materialization이 있은 뒤에만 exact-shape fixture가 Core 또는 operator entrypoint를 실행하고 runtime pass/fail result를 만듭니다.

핵심 모델과 작은 active MVP fixture draft는 이 파일에 남습니다. Detailed later scenario는 [향후 Fixtures](../later/future-fixtures.md)에 둡니다. 이렇게 해야 내부 엔지니어링 점검 Kernel Smoke와 MVP-1 사용자 가치를 설명하면서도 later catalog coverage가 early implementation requirement처럼 보이지 않습니다.

구현이 시작된 뒤 conformance는 실행 가능한 fixture로 Harness behavior를 증명합니다. Runtime fixture가 pass하려면 Core 또는 operator request를 실행하고 captured response fact, Core state, storage row, event, artifact, blocker, error, forbidden side effect를 structured expectation과 비교해야 합니다.

단언(assertion)의 권한은 층위가 있습니다.

- Prose scenario description, comment, rendered Markdown, Journey Card prose, status text, close report prose, agent summary는 설명일 뿐 권한이 아닙니다.
- Captured response fact, Core state, storage row, `task_events`, validator result, returned primary error, structured blocker field, forbidden-side-effect check는 fixture pass/fail을 위한 권위 있는 단언입니다.
- Artifact ref, owner link, `sha256`, `size_bytes`, `content_type`, `redaction_state`, relation owner, retention, availability, file-integrity 단언은 scenario가 artifact 또는 증거 바이트에 의존할 때 권위 있는 단언입니다.
- Projection output은 projection support가 범위에 있을 때 freshness, source-state-version 표시, readability, availability를 확인할 수 있지만 renderer output이 Core state를 대체하거나, evidence를 충족하거나, write를 authorize하거나, close/accept/risk acceptance를 수행하거나, conformance truth의 source가 되면 안 됩니다. 내부 엔지니어링 점검은 empty 또는 "no projection requirement" field를 넘는 projection assertion을 요구하지 않습니다.

## 참조 범위

이 문서는 다음 항목을 담당합니다.

- conformance fixture body shape
- active 내부 엔지니어링 점검 / MVP-1 path의 fixture shorthand boundary
- 예시를 위한 `ToolEnvelope` expansion convention
- 테스트 위생을 위한 isolated fixture execution behavior. 이는 `isolated` 보안 보장이 아닙니다.
- fixture assertion semantics와 comparison mode
- suite catalog metadata boundary
- 검증 프로파일별 증명 동작, 축소된 내부 엔지니어링 점검 / MVP-1 structured draft, 축소된 Kernel Smoke 작성 순서
- 현재 단계 상태와 runtime conformance/docs-maintenance check 사이의 경계
- 향후 catalog scenario를 내부 엔지니어링 점검 또는 MVP-1 requirement로 만들지 않는 link boundary

## 여기서 다루지 않는 것

이 참조 문서는 operator command procedure, docs-maintenance reporting, public MCP schema, SQLite DDL, projection template body, policy contract, 간결한 향후 scenario 목록을 담당하지 않습니다. 그것들은 각 owner Reference 문서에 남습니다. 여기의 suite metadata, example, catalog row는 fixture-body field, public request field, storage row, projection kind, runtime implementation readiness를 추가하지 않습니다.

## Conformance 탐색 지도

| 찾는 것 | 볼 곳 |
|---|---|
| 정확한 fixture body field | [Conformance Fixture Format](#conformance-fixture-format) |
| Runner가 load, seed, execute, capture, compare하는 방식 | [Conformance Execution](#conformance-execution) |
| `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, `forbidden_side_effects`의 default comparison mode | [Fixture Assertion Semantics](#fixture-assertion-semantics) |
| 활성 structured fixture draft | [Kernel Smoke 동작 예시](#engineering-checkpoint-behavior-examples), [MVP-1 사용자 작업 루프 동작 예시](#mvp-1-user-work-loop-behavior-examples), [Security And Capability 동작 예시](#security-and-capability-behavior-examples), [Artifact And Evidence 동작 예시](#artifact-and-evidence-behavior-examples) |
| Suite intent와 작성 순서 | [Conformance staging](operations-and-conformance.md#conformance-staging), [Kernel Smoke Authoring Queue](#kernel-smoke-authoring-queue), [향후 Fixtures: Fixture Suites](../later/future-fixtures.md#fixture-suites) |
| 핵심 모델과 현재 단계 경계 | [핵심 적합성 모델](#핵심-적합성-모델)과 [Fixture 현재 단계 상태](#fixture-현재-단계-상태) |
| Concern별 향후 scenario 목록 | [향후 Fixtures](../later/future-fixtures.md) |

## 핵심 적합성 모델

핵심 적합성 모델은 향후 runtime conformance가 무엇을 증명하고 assertion authority가 어디에 있는지 정의합니다. Passing fixture는 하나의 Core 또는 operator request를 실행하고 captured response fact, Core state, storage row, event, artifact, blocker, error, forbidden side effect를 fixture expectation과 비교해 behavior를 증명합니다. Prose, 생성된 Markdown, Journey Card text, status prose, close prose, agent summary를 맞추는 것만으로 behavior를 증명하지 않습니다.

Assertion type은 의도적으로 작게 유지합니다.

- State와 storage assertion은 Core-owned record, storage row effect, `task_events`, validator result, returned primary error, structured blocker, owner ref, state-version behavior를 비교합니다.
- Artifact assertion은 scenario가 증거 바이트에 의존할 때 등록된 아티팩트 식별 정보, owner link, `sha256`, `size_bytes`, `content_type`, `redaction_state`, relation owner, retention class, availability, file-integrity fact를 비교합니다.
- Projection assertion은 projection support가 범위에 있을 때만 freshness, enqueue 또는 job status, source-state-version display, readability, availability를 비교합니다. Core state를 대체하거나 authority, evidence, close, acceptance, risk judgment를 충족하지 않습니다.
- Error assertion은 public schema precedence에 따른 API-owned primary `ErrorCode`와 optional details를 비교합니다.

State와 storage assertion은 "request 이후 Core가 무엇을 소유했고 어떤 durable row effect가 발생했는가?"에 답합니다. Artifact assertion은 "어떤 증거 바이트 또는 metadata가 안전하게 등록되고 link되었는가?"에 답합니다. Projection assertion은 "derived readable view가 current, stale, available, failed, queued 중 무엇인가?"에 답합니다. 이 위치들은 서로 분리되어 있으며 projection output이 state나 artifact proof를 대신하면 안 됩니다.

## 검증 프로파일별 증명 동작

검증 프로파일은 rendered output의 polish가 아니라 무엇을 증명하는지로 묶습니다. Profile 이름은 fixture-body field를 추가하지 않고, renderer를 권위 있게 만들지 않으며, 현재 문서 전용 저장소에 fixture file이 존재한다는 뜻도 아닙니다.

강화된 로컬 기준 목표(hardened local reference target)는 보증 프로필과 운영 프로필을 통해 도달하는 종합 목표입니다. 다섯 번째 fixture profile이 아니며 suite name으로 쓰면 안 됩니다.

| 프로파일 | 단계 이름 | 증명하는 동작 | 해당 프로파일 밖의 범위 |
|---|---|---|---|
| 내부 엔지니어링 점검 fixtures, 작성 label은 Kernel Smoke | 내부 엔지니어링 점검 | 첫 실행 가능한 권한 루프를 증명합니다. No-active-Task status, owner-valid setup/intake가 active Task 하나를 만드는 동작, active Change Unit requirement, in-scope/out-of-scope `prepare_write`, dry-run과 replay, single-use Write Authorization, `record_run` consumption과 invalid-authorization blocker, 최소 artifact metadata, evidence summary, close blocker, residual-risk visibility, 정직한 cooperative/detective guarantee display가 포함됩니다. | Ordinary natural-language intake 품질, full user-loop judgment UX, full Evidence Manifest, projection renderer support, final-acceptance 또는 residual-risk acceptance 성공 의미, Manual QA, detached verification, export/recover, release handoff, full conformance runner, broad future catalog coverage, hosted connector registry, cross-surface orchestration, preventive guard expansion, broad operations. |
| MVP-1 사용자 작업 루프 fixtures | MVP-1 사용자 작업 루프 | 평소 요청이 Harness vocabulary 없이 tracked work가 되고, focused user judgment, status next safe action, broad approval, sensitive approval, final acceptance, residual-risk acceptance, evidence의 non-substitution boundary와 active MVP가 detached verification을 만들어내지 않는다는 점이 Core state와 structured response로 보임을 증명합니다. | Full agency assurance hardening, detached verification independence, full 수동 QA matrix, stewardship policy suite, full TDD/module/interface/domain-language catalog, full feedback-loop audit, export/recover, release handoff, broad connector ecosystem, hosted connector registry, cross-surface orchestration, MVP-1 사용자 가치 경로 밖의 automation. |
| 보증 프로필 fixtures | 보증 프로필 | User-owned judgment, 민감 동작 승인(Approval), Write Authorization, 수동 QA, verification, 최종 수락, 잔여 위험 수락, stewardship, design-quality, context-hygiene, TDD, feedback-loop boundary가 Core record를 통해 분리되어 fixture-proven 상태임을 증명합니다. | Operator recovery/export completeness, release handoff, broad operations coverage, dashboard/hosted workflow UI, broad connector automation, 증명되지 않은 preventive 또는 isolated guarantee claim. |
| 운영 프로필 / 승격된 로드맵 fixtures | 운영 프로필 및 로드맵 | Export/recover, artifact integrity, release handoff, operator readiness, reconcile, broader conformance coverage, 승격된 future higher guarantee level 또는 automation profile을 증명합니다. | Owner 문서가 mechanism을 정의하고 fixture가 covered behavior를 증명하기 전의 stronger security, isolation, preventive guard, browser-capture, remote/shared MCP, automation claim. |

## 활성 MVP Fixture 초안 묶음

아래 structured fixture draft는 내부 엔지니어링 점검과 MVP-1을 위한 활성 향후 작성 target입니다. 아직 실행 가능한 fixture가 아니며, generated runtime artifact도 아니고 현재 pass/fail 기준도 아닙니다. `TASK-1`, `CU-1`, `WA-1` 같은 symbolic owner ref는 기대 record 관계를 보여 주기 위한 것입니다. 향후 materialized fixture는 이 symbol을 정확한 owner schema payload와 public request shape로 바꾸어야 합니다.

아래 모든 draft는 Core state, storage row effect, stable owner event가 있을 때의 `task_events` family, artifact metadata, structured blocker, public error, forbidden side effect를 assert합니다. Rendered Markdown, status prose, close prose, Journey Card text, report text, agent summary가 그럴듯하다는 이유만으로 통과하면 안 됩니다.

<a id="engineering-checkpoint-behavior-examples"></a>

### Kernel Smoke 동작 예시

Kernel Smoke는 첫 실행 가능한 권한 루프를 위한 좁은 작성 label입니다. 이 section의 draft는 향후 fixture candidate이지 현재 fixture file이 아닙니다. 여기서 "intake"는 smoke용 active Task 하나를 만드는 owner-valid setup/intake path를 뜻합니다. Ordinary natural-language intake 품질은 MVP-1 coverage에 남습니다.

```yaml
scenario_id: MVP-ACTIVE-task-change-unit-setup
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: null, state_version: 1}
  surfaces:
    - {surface_id: reference-local-mcp, max_guarantee_level: detective, pre_tool_blocking_supported: false, isolation_supported: false}
request:
  tool: harness.intake
  payload:
    user_request: "작은 문서 업데이트를 준비한다."
    requested_mode: work
    initial_scope:
      included_paths: ["docs/en/reference/conformance-fixtures.md"]
      excluded_paths: ["server/runtime implementation"]
expected_response:
  result: created_active_task
  refs: {task_id: TASK-1, change_unit_id: CU-1}
  state_version: advanced
expected_state_changes:
  project_state: {active_task_id: TASK-1}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths_contains: ["docs/en/reference/conformance-fixtures.md"]}]
expected_storage_rows:
  tasks: {inserted: 1}
  change_units: {inserted: 1}
  project_state: {updated: 1}
  tool_invocations: {inserted: 1}
  write_authorizations: {inserted: 0}
  runs: {inserted: 0}
  artifacts: {inserted: 0}
expected_events:
  - event_family: owner-promoted Task setup event
  - event_family: owner-promoted Change Unit setup event
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - no Write Authorization is created
  - no Run, evidence summary, final acceptance, residual-risk acceptance, or close state is created
  - no rendered Markdown or generated projection is treated as authority
```

```yaml
scenario_id: MVP-ACTIVE-shaping-update-persists
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 2}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, current_goal_summary: "Draft docs update"}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, success_criteria: []}]
request:
  tool: harness.record_run
  payload:
    kind: shaping
    task_id: TASK-1
    change_unit_id: CU-1
    product_write: false
    write_authorization_id: null
    shaping_update:
      task_update: {current_goal_summary: "Add structured non-executable fixture drafts"}
      change_unit_update: {success_criteria: ["Drafts assert Core state and storage effects"]}
      confirmed_facts: ["No conformance runner exists yet"]
expected_response:
  result: recorded
  refs: {run_id: RUN-SHAPE-1}
  state_version: advanced
expected_state_changes:
  tasks: [{task_id: TASK-1, current_goal_summary: "Add structured non-executable fixture drafts"}]
  change_units: [{change_unit_id: CU-1, success_criteria_contains: ["Drafts assert Core state and storage effects"]}]
expected_storage_rows:
  runs: {inserted: 1, row_filter: {kind: shaping, product_write: false}}
  tasks: {updated: 1}
  change_units: {updated: 1}
  tool_invocations: {inserted: 1}
  write_authorizations: {inserted: 0, consumed: 0}
expected_events:
  - event_family: owner-promoted Run recording event
  - event_family: owner-promoted shaping/state update event
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - no product-file Write Authorization is required or created
  - no artifact, Evidence Manifest, projection job, final acceptance, residual-risk acceptance, or close state is created
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-allowed-authorization
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 3}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 3}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths: ["docs/en/reference/conformance-fixtures.md"]}]
request:
  tool: harness.prepare_write
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    dry_run: false
    idempotency_key: IDEMP-PW-1
    expected_state_version: 3
    intended_operation: edit_file
    intended_paths: ["docs/en/reference/conformance-fixtures.md"]
    product_write: true
expected_response:
  decision: allowed
  write_authorization_ref: {record_kind: write_authorization, record_id: WA-1}
  authorization_effect: created
  primary_error: null
expected_state_changes:
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 3, consumed_by_run_id: null}]
expected_storage_rows:
  write_authorizations: {inserted: 1, updated: 0}
  tool_invocations: {inserted: 1, request_hash: canonical_hash_of_request}
  blockers: {inserted: 0}
  runs: {inserted: 0}
expected_events:
  - event_family: owner-promoted prepare_write allowed or Write Authorization created event
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - no Run is recorded
  - no artifact or evidence sufficiency is created by `prepare_write`
  - Write Authorization is not described as OS permission, sandboxing, preventive blocking, or isolation
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-blocked-no-authorization
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 4}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 4}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths: ["docs/en/reference/conformance-fixtures.md"]}]
request:
  tool: harness.prepare_write
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    dry_run: false
    idempotency_key: IDEMP-PW-BLOCKED
    expected_state_version: 4
    intended_operation: edit_file
    intended_paths: ["docs/en/reference/storage.md"]
    product_write: true
expected_response:
  decision: blocked
  write_authorization_ref: null
  primary_error: SCOPE_VIOLATION
expected_state_changes:
  write_authorizations: []
  current_scope_unchanged: true
expected_storage_rows:
  write_authorizations: {inserted: 0}
  tool_invocations: {inserted: 0}
  runs: {inserted: 0}
  artifacts: {inserted: 0}
expected_events: []
expected_artifacts: []
expected_blockers:
  - {blocker_kind: scope, code: SCOPE_VIOLATION, affected_paths: ["docs/en/reference/storage.md"]}
expected_errors:
  - {code: SCOPE_VIOLATION}
forbidden_side_effects:
  - no consumable Write Authorization row is created
  - no replay row reserves the idempotency key for the pre-commit failure
  - no projection job, artifact, Run, or evidence state is created
```

```yaml
scenario_id: MVP-ACTIVE-prepare-write-idempotent-replay
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 5}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 5}]
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 5, consumed_by_run_id: null}]
  tool_invocations: [{tool_name: harness.prepare_write, idempotency_key: IDEMP-PW-REPLAY, request_hash: HASH-A, response_ref: WA-1}]
request:
  tool: harness.prepare_write
  payload:
    idempotency_key: IDEMP-PW-REPLAY
    canonical_request_hash: HASH-A
    task_id: TASK-1
    change_unit_id: CU-1
    intended_paths: ["docs/en/reference/conformance-fixtures.md"]
expected_response:
  decision: allowed
  write_authorization_ref: {record_kind: write_authorization, record_id: WA-1}
  authorization_effect: returned
  replayed: true
expected_state_changes:
  write_authorizations: [{write_authorization_id: WA-1, status: active, consumed_by_run_id: null}]
  state_version_advanced: false
expected_storage_rows:
  write_authorizations: {inserted: 0, updated: 0}
  tool_invocations: {inserted: 0, updated: 0}
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - no duplicate Write Authorization is created
  - no duplicate event, artifact, projection job, or state-version increment is produced
```

```yaml
scenario_id: MVP-ACTIVE-idempotency-key-hash-conflict
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 6}
  tool_invocations: [{tool_name: harness.prepare_write, idempotency_key: IDEMP-PW-CONFLICT, request_hash: HASH-A, response_ref: WA-1}]
request:
  tool: harness.prepare_write
  payload:
    idempotency_key: IDEMP-PW-CONFLICT
    canonical_request_hash: HASH-B
    task_id: TASK-1
    change_unit_id: CU-1
    intended_paths: ["docs/en/reference/operations-and-conformance.md"]
expected_response:
  result: error
  primary_error: STATE_CONFLICT
  replayed: false
expected_state_changes:
  state_version_advanced: false
  current_records_changed: false
expected_storage_rows:
  tool_invocations: {inserted: 0, updated: 0, original_request_hash_preserved: HASH-A}
  write_authorizations: {inserted: 0, updated: 0}
  runs: {inserted: 0}
expected_events: []
expected_artifacts: []
expected_blockers: []
expected_errors:
  - {code: STATE_CONFLICT, reason: same_idempotency_key_different_hash}
forbidden_side_effects:
  - no merged response fields or owner relations from the conflicting request
  - no artifact, event, projection job, Run, blocker, or replay row is created for the conflict
```

```yaml
scenario_id: MVP-ACTIVE-record-run-consumes-authorization
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 7}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 7}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths: ["docs/en/reference/conformance-fixtures.md"]}]
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 7, consumed_by_run_id: null, attempt_scope_paths: ["docs/en/reference/conformance-fixtures.md"]}]
request:
  tool: harness.record_run
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-RUN-1
    expected_state_version: 7
    kind: implementation
    product_write: true
    write_authorization_id: WA-1
    observed_changes: {changed_paths: ["docs/en/reference/conformance-fixtures.md"]}
expected_response:
  result: recorded
  refs: {run_id: RUN-1, write_authorization_id: WA-1}
  primary_error: null
expected_state_changes:
  runs: [{run_id: RUN-1, task_id: TASK-1, change_unit_id: CU-1, product_write: true, write_authorization_id: WA-1}]
  write_authorizations: [{write_authorization_id: WA-1, status: consumed, consumed_by_run_id: RUN-1}]
expected_storage_rows:
  runs: {inserted: 1}
  write_authorizations: {updated: 1, inserted: 0}
  tool_invocations: {inserted: 1}
expected_events:
  - event_family: owner-promoted Run recorded event
  - event_family: owner-promoted Write Authorization consumed event
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - authorization is consumed exactly once
  - chat or tool prose is not treated as authority
  - no final acceptance, residual-risk acceptance, or close state is created
```

```yaml
scenario_id: MVP-ACTIVE-record-run-missing-authorization-blocked
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 8}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 8}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths: ["docs/en/reference/conformance-fixtures.md"]}]
request:
  tool: harness.record_run
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-RUN-MISSING-WA
    expected_state_version: 8
    kind: implementation
    product_write: true
    write_authorization_id: null
    observed_changes: {changed_paths: ["docs/en/reference/conformance-fixtures.md"]}
expected_response:
  result: blocked
  primary_error: WRITE_AUTHORIZATION_REQUIRED
expected_state_changes:
  runs: []
  write_authorizations: []
  state_version_advanced: false
expected_storage_rows:
  runs: {inserted: 0}
  write_authorizations: {updated: 0, inserted: 0}
  tool_invocations: {inserted: 0}
  evidence_summaries: {inserted: 0, updated: 0}
expected_events: []
expected_artifacts: []
expected_blockers:
  - {blocker_kind: write_compatibility, code: WRITE_AUTHORIZATION_REQUIRED}
expected_errors:
  - {code: WRITE_AUTHORIZATION_REQUIRED}
forbidden_side_effects:
  - no Run, artifact link, evidence update, projection job, or replay row is committed
  - no authorization is fabricated or consumed
```

```yaml
scenario_id: MVP-ACTIVE-record-run-observed-out-of-scope
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 9}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 9}]
  change_units: [{change_unit_id: CU-1, task_id: TASK-1, status: active, scoped_paths: ["docs/en/reference/conformance-fixtures.md"]}]
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 9, consumed_by_run_id: null, attempt_scope_paths: ["docs/en/reference/conformance-fixtures.md"]}]
request:
  tool: harness.record_run
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-RUN-SCOPE-VIOLATION
    expected_state_version: 9
    kind: implementation
    product_write: true
    write_authorization_id: WA-1
    observed_changes: {changed_paths: ["docs/en/reference/storage.md"]}
expected_response:
  result: blocked
  primary_error: SCOPE_VIOLATION
expected_state_changes:
  write_authorizations: [{write_authorization_id: WA-1, status: active, consumed_by_run_id: null}]
  runs: []
expected_storage_rows:
  runs: {inserted: 0}
  write_authorizations: {updated: 0}
  tool_invocations: {inserted: 0}
  blockers: {inserted_or_reported: [{blocker_kind: scope, code: SCOPE_VIOLATION}]}
expected_events: []
expected_artifacts: []
expected_blockers:
  - {blocker_kind: scope, code: SCOPE_VIOLATION, observed_paths: ["docs/en/reference/storage.md"]}
expected_errors:
  - {code: SCOPE_VIOLATION}
forbidden_side_effects:
  - invalid authorization is not marked consumed
  - out-of-scope observation is not completion evidence
  - no final acceptance, residual-risk acceptance, or close readiness is created
```

<a id="mvp-1-user-work-loop-behavior-examples"></a>

### MVP-1 사용자 작업 루프 동작 예시

MVP-1 동작 예시는 broad assurance 또는 operations catalog로 커지지 않고 사용자에게 보이는 Harness 가치를 설명합니다. 향후 fixture가 이 draft를 구체화하면 해당 stage에서 active한 경우 정확히 `harness.status`, `harness.intake`, `harness.request_user_judgment`, `harness.record_user_judgment`, `harness.prepare_write`, `harness.record_run`, `harness.close_task`를 사용할 수 있습니다. 별도 `harness.next` fixture는 later/compatibility material에 속합니다.

```yaml
scenario_id: MVP-ACTIVE-evidence-summary-insufficient
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 10}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1}]
  evidence_summaries: [{evidence_summary_id: EVID-1, task_id: TASK-1, status: partial, required_refs_missing: ["ART-REQ-1"]}]
request:
  tool: harness.status
  payload:
    task_id: TASK-1
expected_response:
  result: ok
  evidence_summary: {status: partial, sufficient: false, missing_refs: ["ART-REQ-1"]}
  next_actions_contains: ["record missing evidence"]
expected_state_changes:
  state_version_advanced: false
  evidence_summaries: [{evidence_summary_id: EVID-1, status: partial}]
expected_storage_rows:
  tasks: {inserted: 0, updated: 0}
  evidence_summaries: {inserted: 0, updated: 0}
  tool_invocations: {inserted: 0}
expected_events: []
expected_artifacts: []
expected_blockers:
  - {blocker_kind: evidence, code: EVIDENCE_INSUFFICIENT, related_refs: ["EVID-1"]}
expected_errors: []
forbidden_side_effects:
  - status read does not create evidence, artifacts, events, acceptance, risk acceptance, or close state
  - Markdown evidence-list prose does not repair the missing ref
```

```yaml
scenario_id: MVP-ACTIVE-evidence-summary-sufficient
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 11}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 11}]
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 11, consumed_by_run_id: null}]
  staged_artifacts: [{staged_uri: staged://fixture/test-output.txt, sha256: SHA256-1, size_bytes: 128, content_type: text/plain, redaction_state: visible}]
request:
  tool: harness.record_run
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-RUN-EVIDENCE
    expected_state_version: 11
    kind: implementation
    product_write: true
    write_authorization_id: WA-1
    observed_changes: {changed_paths: ["docs/en/reference/conformance-fixtures.md"]}
    artifact_inputs:
      - {staged_uri: staged://fixture/test-output.txt, relation: {record_kind: run, record_id: RUN-1}}
    evidence_updates: {claim: "fixture drafts added", required_artifact_refs: ["ART-1"]}
expected_response:
  result: recorded
  refs: {run_id: RUN-1, evidence_summary_id: EVID-1}
  registered_artifacts: [{artifact_id: ART-1}]
expected_state_changes:
  runs: [{run_id: RUN-1, product_write: true}]
  write_authorizations: [{write_authorization_id: WA-1, status: consumed, consumed_by_run_id: RUN-1}]
  evidence_summaries: [{evidence_summary_id: EVID-1, status: sufficient, artifact_refs: ["ART-1"]}]
expected_storage_rows:
  runs: {inserted: 1}
  artifacts: {inserted: 1}
  artifact_links: {inserted: 1}
  evidence_summaries: {inserted_or_updated: 1}
  write_authorizations: {updated: 1}
  tool_invocations: {inserted: 1}
expected_events:
  - event_family: owner-promoted Run recording event
  - event_family: owner-promoted evidence summary update event
expected_artifacts:
  - {artifact_id: ART-1, sha256: SHA256-1, size_bytes: 128, content_type: text/plain, redaction_state: visible, relation_owner: {record_kind: run, record_id: RUN-1}}
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - evidence sufficiency is derived from registered refs, not prose
  - no full Evidence Manifest, Manual QA, detached verification, final acceptance, or residual-risk acceptance is created
```

```yaml
scenario_id: MVP-ACTIVE-final-acceptance-missing-close-blocker
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 12}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, requires_final_acceptance: true}]
  evidence_summaries: [{evidence_summary_id: EVID-1, task_id: TASK-1, status: sufficient}]
  user_judgments: []
request:
  tool: harness.close_task
  payload:
    task_id: TASK-1
    idempotency_key: IDEMP-CLOSE-MISSING-ACCEPTANCE
    expected_state_version: 12
    intent: complete
expected_response:
  result: blocked
  primary_error: ACCEPTANCE_REQUIRED
  terminal: false
expected_state_changes:
  tasks: [{task_id: TASK-1, lifecycle_phase: active}]
expected_storage_rows:
  tasks: {updated_terminal: 0}
  blockers: {inserted_or_reported: [{blocker_kind: final_acceptance, code: ACCEPTANCE_REQUIRED}]}
  tool_invocations: {inserted: 1, only_if_committed_blocked_close_is_owner_enabled: true}
expected_events:
  - event_family: owner-promoted close blocked event, only_if_committed_blocked_close_is_owner_enabled: true
expected_artifacts: []
expected_blockers:
  - {blocker_kind: final_acceptance, code: ACCEPTANCE_REQUIRED, required_judgment_kind: final_acceptance}
expected_errors:
  - {code: ACCEPTANCE_REQUIRED}
forbidden_side_effects:
  - Task is not marked terminal
  - no final_acceptance user judgment is fabricated
  - no residual-risk acceptance, evidence, artifact, Manual QA, detached verification, or generated close report is created
```

```yaml
scenario_id: MVP-ACTIVE-residual-risk-visible-not-accepted-blocker
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 13}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1}]
  evidence_summaries: [{evidence_summary_id: EVID-1, task_id: TASK-1, status: sufficient}]
  blockers: [{blocker_id: BLK-RISK-1, task_id: TASK-1, blocker_kind: residual_risk_acceptance, visible_to_user: true, status: open}]
  user_judgments: []
request:
  tool: harness.close_task
  payload:
    task_id: TASK-1
    idempotency_key: IDEMP-CLOSE-RISK-NOT-ACCEPTED
    expected_state_version: 13
    intent: complete
expected_response:
  result: blocked
  primary_error: DECISION_REQUIRED
  residual_risk_state: {visible: true, accepted: false}
expected_state_changes:
  tasks: [{task_id: TASK-1, lifecycle_phase: active}]
  blockers: [{blocker_id: BLK-RISK-1, blocker_kind: residual_risk_acceptance, status: open}]
expected_storage_rows:
  tasks: {updated_terminal: 0}
  user_judgments: {inserted: 0}
  blockers: {updated_or_reported: [{blocker_kind: residual_risk_acceptance}]}
expected_events:
  - event_family: owner-promoted close blocked event, only_if_committed_blocked_close_is_owner_enabled: true
expected_artifacts: []
expected_blockers:
  - {blocker_kind: residual_risk_acceptance, code: DECISION_REQUIRED, required_judgment_kind: residual_risk_acceptance, related_refs: ["BLK-RISK-1"]}
expected_errors:
  - {code: DECISION_REQUIRED}
forbidden_side_effects:
  - visible risk is not treated as accepted risk
  - no final acceptance, detached verification, Manual QA, rich Residual Risk record, or close state is fabricated
```

```yaml
scenario_id: MVP-ACTIVE-accepted-risk-close
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 14}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1}]
  evidence_summaries: [{evidence_summary_id: EVID-1, task_id: TASK-1, status: sufficient}]
  blockers: [{blocker_id: BLK-RISK-1, task_id: TASK-1, blocker_kind: residual_risk_acceptance, visible_to_user: true, status: resolved}]
  user_judgments:
    - {user_judgment_id: UJ-RISK-1, task_id: TASK-1, judgment_kind: residual_risk_acceptance, status: resolved, accepted_risks: ["BLK-RISK-1"]}
request:
  tool: harness.close_task
  payload:
    task_id: TASK-1
    idempotency_key: IDEMP-CLOSE-ACCEPTED-RISK
    expected_state_version: 14
    intent: completed_with_risk_accepted
expected_response:
  result: closed
  close_reason: completed_with_risk_accepted
  accepted_risk_refs: [{record_kind: user_judgment, record_id: UJ-RISK-1}]
  primary_error: null
expected_state_changes:
  tasks: [{task_id: TASK-1, lifecycle_phase: terminal, result: passed}]
  blockers: [{blocker_id: BLK-RISK-1, status: resolved}]
expected_storage_rows:
  tasks: {updated_terminal: 1}
  user_judgments: {inserted: 0, matched_existing: ["UJ-RISK-1"]}
  blockers: {updated: 1}
  tool_invocations: {inserted: 1}
expected_events:
  - event_family: owner-promoted successful close event
expected_artifacts: []
expected_blockers: []
expected_errors: []
forbidden_side_effects:
  - accepted risk does not create detached verification, Manual QA, final acceptance, Approval, or assurance upgrade
  - no standalone active-MVP residual_risk row is required
  - no generated close report is treated as close authority
```

```yaml
scenario_id: MVP-ACTIVE-display-label-not-canonical
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 15}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1}]
request:
  tool: harness.request_user_judgment
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-JUDGMENT-LABEL
    expected_state_version: 15
    judgment_kind: product_decision
    presentation: short
    locale: ko
    question: "이 제품 동작을 선택할까요?"
expected_response:
  result: requested
  user_judgment_ref: {record_kind: user_judgment, record_id: UJ-1}
  rendered_display_label: "제품 판단"
expected_state_changes:
  user_judgments: [{user_judgment_id: UJ-1, judgment_kind: product_decision, presentation: short, status: pending_user}]
expected_storage_rows:
  user_judgments: {inserted: 1, forbidden_columns: ["display_label"]}
  blockers: {inserted_or_updated: [{blocker_kind: user_judgment, required_judgment_kind: product_decision}]}
  tool_invocations: {inserted: 1}
expected_events:
  - event_family: owner-promoted user judgment requested event
expected_artifacts: []
expected_blockers:
  - {blocker_kind: user_judgment, required_judgment_kind: product_decision, canonical_key: product_decision}
expected_errors: []
forbidden_side_effects:
  - no canonical state, blocker key, gate key, or storage identity uses "제품 판단" or `display_label`
  - no product decision is resolved by requesting it
  - no Write Authorization, final acceptance, residual-risk acceptance, evidence, artifact, or close state is created
```

<a id="security-and-capability-behavior-examples"></a>

### Security And Capability 동작 예시

Security와 capability 예시는 정직한 local capability display와 unavailable-path behavior를 증명합니다. 이름을 붙였다는 이유만으로 더 강한 guarantee가 생기지 않습니다. 활성 MVP draft는 `CAPABILITY_INSUFFICIENT`, cooperative/detective profile fact, no-authority unavailable response를 assert할 수 있습니다. Preventive guard expansion과 isolated profile은 later/profile 또는 Roadmap material로 남습니다.

<a id="artifact-and-evidence-behavior-examples"></a>

### Artifact And Evidence 동작 예시

Artifact 예시는 보고서 문구가 아니라 등록된 bytes와 metadata를 증명합니다. Active stage가 artifact ref 또는 evidence summary를 사용할 때 적용합니다. 더 넓은 export non-leakage는 later/profile catalog material입니다.

```yaml
scenario_id: MVP-ACTIVE-raw-secret-artifact-blocked
initial_state:
  project_state: {project_id: PRJ-1, active_task_id: TASK-1, state_version: 16}
  tasks: [{task_id: TASK-1, lifecycle_phase: active, active_change_unit_id: CU-1, state_version: 16}]
  write_authorizations: [{write_authorization_id: WA-1, task_id: TASK-1, change_unit_id: CU-1, status: active, basis_state_version: 16, consumed_by_run_id: null}]
  staged_artifacts:
    - {staged_uri: staged://fixture/raw-secret.txt, content_class: raw_secret, redaction_state: blocked}
request:
  tool: harness.record_run
  payload:
    task_id: TASK-1
    change_unit_id: CU-1
    idempotency_key: IDEMP-RUN-SECRET-BLOCKED
    expected_state_version: 16
    kind: implementation
    product_write: true
    write_authorization_id: WA-1
    observed_changes: {changed_paths: ["docs/en/reference/conformance-fixtures.md"]}
    artifact_inputs:
      - {staged_uri: staged://fixture/raw-secret.txt, relation: {record_kind: run, record_id: RUN-SECRET-1}}
expected_response:
  result: blocked
  primary_error: ARTIFACT_MISSING
  registered_artifacts: []
expected_state_changes:
  write_authorizations: [{write_authorization_id: WA-1, status: active, consumed_by_run_id: null}]
  evidence_summaries: [{task_id: TASK-1, status: blocked, reason: artifact_redaction_blocked}]
expected_storage_rows:
  artifacts: {inserted: 0, raw_bytes_stored: false}
  artifact_links: {inserted: 0}
  runs: {inserted: 0}
  write_authorizations: {updated: 0}
  evidence_summaries: {inserted_or_updated: 1, status: blocked}
expected_events: []
expected_artifacts:
  - {artifact_id: null, redaction_state: blocked, raw_secret_value_asserted: false}
expected_blockers:
  - {blocker_kind: artifact_availability, code: ARTIFACT_MISSING, reason: raw_secret_blocked}
expected_errors:
  - {code: ARTIFACT_MISSING}
forbidden_side_effects:
  - raw secret or PII bytes are not stored, asserted, rendered, exported, or copied into a generated report
  - blocked artifact input does not satisfy evidence, QA, detached verification, final acceptance, residual-risk acceptance, or close
  - authorization is not consumed by a blocked artifact attempt
```

### Later/Profile Fixture Boundary

Detailed clarification catalog, later-profile verification, full Evidence Manifest case, Manual QA matrix, export non-leakage, Browser QA Capture, full operations recovery/export, broad connector conformance, preventive guard expansion, isolated security profile은 owner가 stage impact와 proof expectation이 있는 더 좁은 fixture를 승격하기 전까지 later/profile 또는 Roadmap material에 남습니다. [향후 Fixtures](../later/future-fixtures.md)에 family가 있다는 사실만으로 내부 엔지니어링 점검이나 MVP-1 requirement가 되지 않습니다.

## Conformance Fixture Format

runtime conformance는 Harness Server 구현과 fixture materialization 이후 fixture 기반입니다. 동작 예시 table만으로는 충분하지 않습니다. 구체화된 각 test fixture는 하나의 request를 실행하고 structured response fact, Core state change, storage row, event, artifact, blocker, error, forbidden side effect를 검증해야 합니다.

각 structured fixture draft는 이 shape를 포함해야 합니다.

```yaml
scenario_id: string
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
```

Fixture 형태 요약: suite metadata는 fixture를 묶을 수 있지만, fixture body는 향후 실행 가능한 conformance를 위한 하나의 정확한 request-and-expectation shape를 유지합니다. 위 YAML 블록이 계약 요약입니다.

향후 fixture file과 suite catalog는 fixture body 밖에 metadata를 가질 수 있습니다. Fixture body 자체는 위 field만 사용해야 conformance runner가 behavior를 일관되게 비교할 수 있습니다. Suite delivery stage, assertion mode, docs-maintenance result, prose status, rendered Markdown, authoring note를 표현하기 위해 fixture body field를 추가하지 않습니다. 그런 정보는 suite catalog metadata, docs-maintenance report, display owner, 주변 문서에 둡니다.

Fixture body type notation은 API의 [Schema notation convention](api/schema-core.md#schema-notation-convention)을 따릅니다. 위 top-level fixture body field는 모두 required입니다. Fixture가 empty object, object map, array를 의도적으로 제공할 때는 `{}` 또는 `[]`를 사용합니다. Required top-level field를 생략하는 것은 invalid fixture body이며 "not asserted"가 아닙니다. 내부 엔지니어링 점검과 MVP-1 active draft에서는 projection rendering이 보통 없습니다. 나중에 승격된 owner가 projection freshness를 요구하면 rendered Markdown matching이 아니라 `expected_state_changes.checks`, `expected_storage_rows.projection_jobs`, 또는 owner가 정의한 structured location에 Core/storage fact를 assert합니다.

MCP tool request의 경우 향후 실행 가능한 fixture `request.tool`은 public tool 또는 operator action을 이름 붙이고, `request.payload`는 API docs가 정의하는 해당 tool의 public request payload입니다. Runner는 schema가 요구하는 경우 `envelope: ToolEnvelope`를 포함해 `request.tool`에 해당하는 request schema로 `request.payload`를 검증해야 합니다. 이 문서의 draft는 다음 envelope-expansion convention 아래에서만 `ToolEnvelope`를 생략할 수 있습니다. Validation, 정규화, request hashing, Core execution 전에 runner가 `initial_state`, suite defaults, fixture metadata에서 deterministic valid envelope를 제공합니다. Expanded request가 Core에 전달되는 값입니다. 이 convention은 fixture field를 추가하거나 fixture body shape를 바꾸거나 alternate request schema를 만들지 않습니다.

Fixture shorthand는 두 번째 API가 아닙니다. Main 내부 엔지니어링 점검 / MVP-1 path에서는 shorthand가 `initial_state` seeding, non-executable draft의 symbolic owner ref, 또는 suite catalog metadata를 간단히 표현할 수 있을 뿐이며 owner-defined record와 public schema를 보존해야 합니다. Public mutation은 `ToolEnvelope` expansion 이후에도 선택된 `request.tool`에 대한 documented public request branch를 `request.payload` 아래에서 사용해야 합니다. Later-profile shorthand detail은 [향후 Fixtures: Later-Profile Fixture Shorthand Notes](../later/future-fixtures.md#later-profile-fixture-shorthand-notes)에 두며 내부 엔지니어링 점검 또는 MVP-1의 active requirement가 아닙니다.

`write_authorizations`를 seed하는 향후 실행 가능한 fixture는 valid stored rows를 만들어야 합니다. 각 seeded authorization row는 `basis_state_version`을 명시적으로 포함하거나, runner가 `state.sqlite`에 insert하기 전에 row의 Task에 대한 seeded affected-scope state version에서 이를 파생해야 합니다. 이는 storage-loader derivation rule일 뿐이며 fixture top-level field를 추가하거나 fixture body shape를 바꾸지 않습니다. Partial `expected_state_changes.write_authorizations` 또는 `expected_storage_rows.write_authorizations` assertions는 idempotent replay, 최신성 감지, expiry, audit behavior를 test하지 않는 한 `basis_state_version`을 생략할 수 있습니다. `basis_state_version`은 `decision=allowed` basis이지 resulting `ToolResponseBase.state_version`이 아닙니다. Fixture loader는 `blocked`, `approval_required`, `decision_required`, `state_conflict` outcome을 `write_authorizations` row로 seed하면 안 됩니다. 그런 outcome은 response decision, blocker, validator finding, error를 사용합니다.

Suite catalog metadata는 Core에 전달되지 않으며 fixture body의 일부가 아닙니다. Suite, delivery stage, tag별로 exact-shape fixture를 묶을 수 있습니다.

```yaml
suite: agency
earliest_delivery_stage: "보증 프로필"
tags: [decision-gate, residual-risk, autonomy-boundary]
fixtures:
  - AGENCY-user-judgment-required-before-product-tradeoff-write
  - AGENCY-residual-risk-visible-before-acceptance
```

Runner는 이 metadata를 suite 선택, 순서 지정, reporting에 사용할 수 있습니다. Core에는 documented envelope expansion 이후의 `request.tool`과 public `request.payload`만 전달됩니다. Metadata가 seed expansion, fixture comparison semantics, tool request schema, expected owner records를 바꾸면 안 됩니다.

## Conformance Execution

향후 `harness conformance run`은 MCP tool과 operator command가 사용하는 것과 같은 Core entrypoint를 통해 fixture를 실행합니다. 동작을 prose output만 검사해서 검증하면 안 됩니다. Core entrypoint를 실행하고 그 결과의 response fact, state, storage row, event, artifact, blocker, 관련되는 경우 projection fact, error, forbidden side effect를 비교해야 합니다.

향후 runtime fixture execution 의미:

1. Fixture YAML file을 load하고 exact fixture body shape를 검증합니다.
2. Fixture가 existing read-only sample을 명시적으로 target하지 않는 한 fresh fixture-only 하네스 런타임 홈과 임시 제품 저장소를 만듭니다. 여기서 fixture isolation은 deterministic comparison을 위한 테스트 위생입니다. `isolated` guarantee level, OS sandboxing, 권한 격리, 변조 방지 storage claim이 아닙니다. Runner는 state-changing fixture execution에 developer의 실제 하네스 런타임 홈이나 제품 저장소를 재사용하면 안 됩니다.
3. `initial_state`에서 `registry.sqlite`, `project.yaml`, `state.sqlite`, artifact file, fixture가 요구하는 경우 projection file, connector manifest를 seed합니다.
4. Core를 통해 `request.tool`을 execute합니다. MCP tool action은 public request schema를 사용합니다. Documented `ToolEnvelope` expansion 이후 fixture `request.payload`는 접점이 해당 MCP tool에 보낼 request payload와 같아야 합니다. `projection_refresh`, `doctor_surface`, `recover`, `artifacts_check` 같은 operator action은 [운영과 Conformance 참조](operations-and-conformance.md)의 operator semantics를 사용합니다.
5. Returned response fact, resulting state summary, storage effect, 추가된 owner event, emitted validator result, artifact registry/file integrity, structured blocker, 관련되는 경우 projection job status, 관련되는 경우 reconcile item, returned error code를 capture합니다.
6. Captured result를 `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, `forbidden_side_effects`와 compare합니다. Empty expected section은 해당 section에 관련 effect가 없음을 단언합니다.
7. Fixture id, pass/fail, observed response/state/storage/event/artifact/blocker/error summary, 관련되는 경우 projection freshness, forbidden-side-effect comparison을 보고합니다.

Runner 순서 요약: 위 번호 목록이 계약 요약입니다. 향후 runner는 exact fixture body를 읽고 fixture-only runtime home을 seed한 뒤 Core를 통해 request를 실행하고, response/state/storage/events/artifacts/blockers/errors/forbidden side effects를 비교해 report를 냅니다.

Fixture `request.payload`가 `expected_state_version`을 포함하면 runner는 `ToolEnvelope.task_id`만이 아니라 Core-resolved primary Task에 따라 비교합니다. Primary Task resolution order는 tool-specific `task_id`, `ToolEnvelope.task_id`, active Task resolution 순서입니다. Task-scoped actions는 seeded 또는 Core-resolved primary Task State Version과 비교하고, resolved primary Task가 없는 project-scoped actions는 Project State Version과 비교합니다. Captured response, `EventRef.state_version`, `task_events.state_version` values는 resulting affected-scope versions로 비교합니다. Read-only fixtures는 primary read scope의 unchanged version을 검증할 수 있습니다. 이 설명은 fixture body shape를 바꾸지 않고 comparison 의미만 명확히 합니다.

Stale `expected_state_version` fixture는 단순한 concurrent-write test가 아니라 stale-authority test입니다. Exact idempotent replay는 예외입니다. Committed replay row가 있고 canonical request hash가 일치하면 fixture는 original committed response가 반환되고 current state-version freshness check가 다시 실행되지 않았음을 검증해야 합니다. Replay row가 없고 state-changing action이 commit 전에 conflict되면, owner document가 다른 recovery action을 명시하지 않는 한 fixture는 current record 변경 없음, `task_events` append 없음, artifact 등록 없음, projection job enqueue 없음, conflicting request를 위한 `tool_invocations` replay row 생성 없음까지 검증해야 합니다. 같은 key가 changed canonical request hash와 함께 재사용되면 fixture는 `STATE_CONFLICT`, original replay row 보존, 새 artifact/event/projection job/response field/owner relation이 merge되지 않음을 검증해야 합니다. `dry_run=true` fixture는 diagnostic 또는 `would_create` effect가 반환되어도 current record, `task_events`, artifact, consumable Write Authorization, projection job, `tool_invocations` replay row가 생기지 않고, 나중에 non-dry-run call을 보낼 때 key가 이미 예약된 것으로 처리되지 않음을 검증해야 합니다. Replayed `prepare_write`는 duplicate authorization을 만들면 안 됩니다. Replayed `record_run`은 authorization을 두 번 consume하면 안 됩니다.

Fixture execution은 deterministic해야 합니다. Network access, wall-clock-sensitive expiry, external tool output은 suite가 integration smoke라고 명시적으로 선언하지 않는 한 stub하거나 seeded fixture input으로 표현해야 합니다.

Fixture isolation은 pass 조건의 일부입니다. Fixture는 임시 제품 저장소와 하네스 런타임 홈에 file을 seed하고, 그곳에서 하나의 Core 또는 operator action을 실행한 뒤 captured result를 비교할 수 있습니다. 이것은 product guarantee level을 올리지 않습니다. Existing local runtime record, generated operational file, 이전 실행의 prose report에 의존하면 안 됩니다.

Seed validation은 action execution 전에 수행하고, captured-state validation은 action execution 이후에 수행합니다. 비교의 양쪽은 fixture-local string label이 아니라 owner-defined state loader와 value set을 사용합니다.

Conformance runner는 MCP tool과 operator command가 사용하는 동일한 Core storage loader를 통해 JSON `TEXT` field를 seed하고 검사해야 합니다. `initial_state`에 malformed JSON 또는 schema-incompatible JSON이 있는 fixture는 유효하지 않은 상태를 드러내야 합니다. Fixture action이 recovery path이고 safe reconstruction이 가능한 경우에는 복구 가능한 state issue를 드러내야 합니다. Runner는 JSON field를 opaque string으로 취급해서 shape validation을 건너뛰면 안 됩니다. 이 기대사항은 fixture body shape를 바꾸지 않습니다.

Conformance runner는 status-like `TEXT` field도 [Storage](storage.md#canonical-enum-hardening)의 owner-bound hardening map을 통해 seed하고 검사해야 합니다. Main 내부 엔지니어링 점검 / MVP-1 path에서 fixture seed loader는 active stage의 seeded record에 실제로 들어가는 owner value만 검증하고, artifact/ref enum assertion은 API [stage-specific active value sets](api/schema-core.md#stage-specific-active-value-sets)를 사용합니다. 예를 들면 registry/project surface guarantee, Run kind/status, Write Authorization status/guarantee, 해당 owner path가 active일 때의 Approval status, evidence support가 active일 때의 minimal evidence summary coverage/status, risk visibility가 active일 때의 residual-risk visibility/status, projection assertion이 범위에 있을 때의 projection job kind/status, 그리고 해당 owner record를 사용할 때의 current Task 또는 Change Unit status입니다. Full Evidence Manifest status는 later/profile-gated입니다. Later-profile status field는 그 profile이 active가 되기 전까지 promoted owner docs와 future catalog에 남습니다. 유효하지 않은 state recovery를 명시적으로 test하는 scenario가 아닌 한 unknown status value는 계속 invalid입니다. Expected-state status assertion은 prose label이 아니라 captured owner value를 비교합니다.

## Fixture Assertion Semantics

Fixture assertion mode는 runner default 또는 suite catalog metadata입니다. Core input이 아니고 MCP tool에 전달되지 않으며 fixture body에 field를 추가하면 안 됩니다. Fixture body는 정확히 `scenario_id`, `initial_state`, `request`, `expected_response`, `expected_state_changes`, `expected_storage_rows`, `expected_events`, `expected_artifacts`, `expected_blockers`, `expected_errors`, `forbidden_side_effects`만 유지합니다.

Partial assertion object 안에서 omission은 "not asserted"를 뜻합니다. Value가 `null`인 listed field는 captured field가 present이고 JSON `null`과 같음을 assert합니다. Listed array value `[]`는 present empty array를 assert합니다. Owner schema가 해당 field를 map이라고 말하는 경우 listed object-map value `{}`는 present empty map을 assert합니다. `partial_deep` 아래의 structured object에서는 object 존재만 의도적으로 assert하는 경우가 아니라면 fixture author는 최소 하나의 child field를 나열해야 합니다.

이 omission rule은 assertion rule일 뿐입니다. Public MCP `request.payload`에서 omitted field를 valid로 만들지 않습니다. Fixture `request.payload`는 documented envelope expansion 이후에도 owning public request schema를 통과해야 합니다.

Default comparison modes:

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

`expected_events` comparisons는 captured `task_events`의 [Core Model Stable Event Catalog](core-model.md#stable-event-catalog) projection을 대상으로 합니다. API tool detail/audit event lists는 이 set을 확장하지 않습니다. `task_events`에 capture된 non-catalog detail 또는 local-audit events는 normal staged-delivery fixture를 fail하게 만들면 안 됩니다. Suite metadata가 `expected_events: exact`로 설정하면, future 로드맵/local suite가 implementation-specific detail-event assertions를 명시적으로 opt in하지 않는 한 exactness는 captured stream의 stable-event projection에 적용됩니다. Validator IDs, Core check names, projection status shorthands, fixture shorthand labels, scenario catalog IDs는 event names가 아닙니다. Prose examples는 non-catalog event names를 illustrative 또는 future extension ideas로 언급할 수 있지만, 실행 가능한 staged-delivery fixture는 Core Model event catalog가 승격하기 전까지 이를 요구하면 안 됩니다.

Conformance runner는 captured `task_events`를 `event_seq`로 order합니다. `state_version`, `created_at`, `event_id`는 `expected_events` ordering의 tie-breaker가 아닙니다.

Fixture authors는 API precedence가 generic validator fallback을 선택할 때만 `VALIDATOR_FAILED`를 `expected_errors[].code`로 사용해야 합니다. `EVIDENCE_INSUFFICIENT`, `QA_REQUIRED`, `PROJECTION_STALE`, `ARTIFACT_MISSING` 같은 더 specific한 typed blocker가 적용되면 그 code가 primary입니다.

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

향후 conformance는 captured response field, Core state, storage row, `task_events`, validator result, artifact registry/file integrity, 승격된 경우 projection job 또는 freshness state, returned error code, structured blocker, forbidden-side-effect check를 통해 behavior를 증명해야 합니다. Rendered Markdown, Journey Card prose, status prose, close report prose, agent prose만 맞춰서는 fixture를 통과시킬 수 없습니다.

Fixture runner는 `request_hash`, baseline `tree_hash`, projection `managed_hash`에 대해 reference implementation과 같은 정규화 rule을 사용해야 합니다. 세부 알고리즘은 [MVP API](api/mvp-api.md), [API Schema Core](api/schema-core.md), [Storage](storage.md), [Projection과 Template 참조](projection-and-templates.md)가 담당합니다. Conformance fixture는 그 기준 기록 경계를 다시 정의하지 않고 deterministic behavior를 검증합니다.

## Fixture 현재 단계 상태

현재 저장소는 문서 전용입니다. 이 문서 batch는 실행 가능한 fixture file, 실행 가능한 fixture catalog file, generated projection, runtime state, database, Harness Server conformance test를 만들지 않습니다.

MVP structured draft와 fixture 작성 queue는 향후 작성 계획입니다. 문서 수락, 별도의 구현 계획 준비 결정, Harness Server 구현, 명시적인 fixture materialization step이 있은 뒤에야 실행 가능한 상태가 됩니다. 문서 점검은 Markdown drift를 보고할 수 있지만 runtime conformance가 아니며 Core fixture result를 만들지 않습니다.

## Catalog-Only Fixture Skeleton Guidance

Catalog skeleton guidance는 승격된 향후 catalog family를 exact-shape fixture로 옮길 때 쓰는 지침입니다. Executable fixture body, public request schema, DDL extension, runner design, stage-exit requirement가 아닙니다. Delivery-stage mapping은 suite catalog metadata에 두며 fixture body에 넣지 않습니다. "Minimum seeded records"는 Storage 규칙으로 expansion 및 validation을 거친 뒤 `initial_state`에 들어가는 owner record를 뜻합니다. Public mutation은 계속 정확한 MCP request payload를 `request.payload`로 사용합니다.

향후 scenario family 목록은 [향후 Fixtures](../later/future-fixtures.md)에 있습니다.

## Kernel Smoke Authoring Queue

이 queue는 [Kernel Smoke 동작 예시](#engineering-checkpoint-behavior-examples)를 위한 향후 작성 지침입니다. Kernel Smoke는 첫 내부 권한 루프를 위한 좁은 작성 label이지 제품 MVP, 전체 conformance suite, 향후 fixture catalog가 아닙니다. 이 row들은 실행 가능한 fixture file이 이미 존재한다고 암시하지 않습니다. Compact authoring order일 뿐이며, 첫 구현 계획은 Build가 이름 붙인 하나의 권한 루프를 증명하는 가장 작은 subset만 구체화할 수 있습니다.

Kernel Smoke는 projection requirement 없음이 기본입니다. Minimal owner path가 이미 그런 fact를 만들고 target behavior 증명에 도움이 될 때만 projection freshness 또는 enqueue/failure fact를 검증할 수 있습니다. Projection-template polish, detailed report template, 여러 projection kind, Browser QA Capture, export/recover, reconcile, stewardship, context hygiene, full operations, future guarantee-level fixture는 owner docs가 특정 좁은 path를 나중에 승격하지 않는 한 내부 엔지니어링 점검 밖에 둡니다.

Table의 `None`은 matching draft field가 `[]`, `{}`, 또는 그 밖의 empty value로 남는다는 뜻입니다. 새 sentinel value가 아닙니다.

| Queue | Fixture draft family | Request path | Minimum seeded records | Required structured assertion | Expected blockers/errors | 보존해야 하는 forbidden side effects |
|---|---|---|---|---|---|---|
| 1 | `MVP-ACTIVE-task-change-unit-setup` | `harness.intake` | Active Task가 없는 registered local project | Active Task 하나, active Change Unit 또는 scope boundary 하나, current-task pointer, write authority 없음. | None | Run, artifact, evidence, final acceptance, residual-risk acceptance, close, projection-as-authority effect 없음. |
| 2 | `MVP-ACTIVE-shaping-update-persists` | `kind=shaping`, `product_write=false`인 `harness.record_run` | Active Task와 Change Unit | Shaping update가 Task/Change Unit state와 shaping Run에 persist되고 product-write authority는 만들지 않음. | None | Write Authorization, product-write Run, Evidence Manifest, projection job, acceptance, risk acceptance 없음. |
| 3 | `MVP-ACTIVE-prepare-write-allowed-authorization` | `harness.prepare_write` | Active Task, compatible scope, current expected state | `decision=allowed`, active Write Authorization 하나, replay row, Run 없음. | None | OS permission, sandbox, preventive, isolated, evidence, close claim 없음. |
| 4 | `MVP-ACTIVE-prepare-write-blocked-no-authorization` | `harness.prepare_write` | Active Task와 incompatible requested path 또는 compatible scope 누락 | Structured blocked response와 no consumable Write Authorization. | API/Core path가 소유한 `SCOPE_REQUIRED`, `NO_ACTIVE_CHANGE_UNIT`, 또는 `SCOPE_VIOLATION`. | Authorization, Run, artifact, pre-commit failure replay row, projection job 없음. |
| 5 | `MVP-ACTIVE-prepare-write-idempotent-replay` | `harness.prepare_write` replay | Existing committed replay row와 original active authorization | Original response와 original `write_authorization_ref` 반환. | None | Duplicate authorization, event, artifact, replay update, projection job, state-version increment 없음. |
| 6 | `MVP-ACTIVE-idempotency-key-hash-conflict` | 같은 idempotency key와 다른 hash를 쓰는 state-changing tool | Existing committed replay row | `STATE_CONFLICT`; original replay row unchanged. | `STATE_CONFLICT` | Merged response, event, artifact, projection job, owner relation, replay row update 없음. |
| 7 | `MVP-ACTIVE-record-run-consumes-authorization` | `harness.record_run` | Active Task, compatible scope, active compatible Write Authorization | Run 하나가 기록되고 authorization이 정확히 한 번 consumed됨. | None | Second consumption, final acceptance, residual-risk acceptance, detached verification, close 없음. |
| 8 | `MVP-ACTIVE-record-run-missing-authorization-blocked` | `harness.record_run` | Active Task와 authorization 없는 product-write Run request | Product-write Run이 commit 전에 blocked됨. | `WRITE_AUTHORIZATION_REQUIRED` | Run, consumption, completion evidence, artifact link, projection job, replay row 없음. |
| 9 | `MVP-ACTIVE-record-run-observed-out-of-scope` | `harness.record_run` | Stored scope가 observed path를 제외하는 active compatible Write Authorization | Out-of-scope observation은 blocked되거나 owner violation/audit path로만 기록되며 success로 authorization을 consume하지 않음. | `SCOPE_VIOLATION` 또는 owner-equivalent structured blocker | Invalid authorization이 consumed되지 않음. Observation은 completion evidence나 close readiness가 아님. |
| 10 | `MVP-ACTIVE-raw-secret-artifact-blocked` | artifact input이 있는 `harness.record_run` | Active Task/Run path와 staged raw-secret artifact input | Raw secret bytes는 blocked되거나 safe blocked/omitted metadata와 downstream evidence/blocker effect로만 표현됨. | `ARTIFACT_MISSING` 또는 owner-equivalent artifact blocker | Raw secret storage, rendering, export, evidence sufficiency, authorization consumption, close 없음. |
| 11 | `MVP-ACTIVE-evidence-summary-insufficient` | `harness.status` 또는 evidence owner read | Partial/missing evidence summary가 있는 active Task | Evidence summary가 insufficient/partial로 남고 close-relevant blocker가 structured임. | Close/write path가 의존할 때 `EVIDENCE_INSUFFICIENT` blocker | Status prose 또는 Markdown evidence list가 missing refs를 repair하지 않음. |
| 12 | `MVP-ACTIVE-evidence-summary-sufficient` | `harness.record_run` | Active Task, compatible authorization, visible staged artifact | Registered artifact refs와 evidence summary가 owner records에서 sufficient가 됨. | None | Full Evidence Manifest, Manual QA, detached verification, final acceptance, risk acceptance 없음. |
| 13 | `MVP-ACTIVE-final-acceptance-missing-close-blocker` | `harness.close_task` | Evidence는 sufficient지만 required final acceptance가 없는 active Task | Close가 final-acceptance blocker로 blocked 상태를 유지함. | `ACCEPTANCE_REQUIRED` | Terminal Task, fabricated acceptance, residual-risk acceptance, Manual QA, detached verification, close report authority 없음. |
| 14 | `MVP-ACTIVE-residual-risk-visible-not-accepted-blocker` | `harness.close_task` | Visible close-relevant residual risk가 있고 compatible acceptance judgment가 없는 active Task | Residual-risk acceptance가 계속 required이고 Task는 open 상태로 남음. | `required_judgment_kind=residual_risk_acceptance`가 있는 `DECISION_REQUIRED` 또는 `DECISION_UNRESOLVED` | Visible risk는 accepted risk가 아님. Rich Residual Risk record, detached verification, close state를 fabricate하지 않음. |
| 15 | `MVP-ACTIVE-accepted-risk-close` | `harness.close_task` | Sufficient evidence, visible risk, compatible `judgment_kind=residual_risk_acceptance`가 있는 active Task | Task가 accepted-risk close reason과 user judgment ref로 close됨. | None | Accepted risk가 detached verification, Manual QA, Approval, final acceptance, assurance upgrade를 만들지 않음. |
| 16 | `MVP-ACTIVE-display-label-not-canonical` | `harness.request_user_judgment` | Active Task와 Change Unit | Response는 localized display label을 render할 수 있지만 storage와 blocker state는 canonical `judgment_kind`를 사용함. | None | `display_label`과 localized label은 canonical state, gate key, storage identity, close aggregation key가 아님. |

위 queue는 의도적으로 작습니다. 내부 엔지니어링 점검은 전체 conformance suite, broad catalog family coverage, final-acceptance success semantics, 수동 QA, 분리 검증, export/recover, reconcile, stewardship, context hygiene, Browser QA Capture, future guarantee-level check를 요구하지 않습니다. MVP-1은 나열된 user-loop judgment, evidence, close-blocker, accepted-risk draft를 더하지만 later verification, full Evidence Manifest, full Manual QA, export, profile fixture를 승격하지 않습니다.

## 향후 fixture catalog

Scenario family는 early reference가 핵심 적합성 모델에 집중하도록 [향후 Fixtures](../later/future-fixtures.md)로 이동했습니다. 그 catalog에는 Browser QA Capture, cross-surface behavior, export non-leakage, context hygiene, reconcile, stewardship, full operations, advanced projection rendering, artifact redaction/integrity, future guarantee-level check를 위한 간결한 향후 목록이 있습니다.

그 catalog entry는 promoted owner path가 exact-shape의 실행 가능한 fixture로 구체화하기 전까지 design inventory일 뿐입니다. 내부 엔지니어링 점검 requirement가 아니며, 그 자체로 MVP-1을 확장하지 않고, 이 저장소가 문서 전용인 동안 runtime conformance로 계산하지 않습니다.

## Metrics Boundary

Long-term operational metrics는 derived analytics이지 staged-delivery-critical state나 conformance requirement가 아닙니다. Approval turnaround, verification latency, projection stale duration, same-session guard frequency, surface fallback rate 같은 metric은 future version이 owner docs, fixture 또는 conformance target, fallback behavior, 관련 redaction/retention policy, projection-as-canonical dependency 없음, implementation ownership과 함께 승격하기 전까지 [roadmap](../roadmap.md)에 read-only diagnostic으로 둡니다.
