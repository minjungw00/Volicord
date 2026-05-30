# Build: 첫 실행 가능한 조각

## 이 문서가 도와주는 일

이 문서는 Build 개요를 구현자가 가장 먼저 계획해야 하는 가장 작은 runnable kernel slice로 바꿉니다.

이 문서는 구현 계획 문서입니다. 문서 세트가 구현 계획에 사용할 수 있다고 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 실행 목표는 코어 권한 조각(v0.1 Core Authority Slice)이며, 커널 스모크(Kernel Smoke)는 이 조각을 위한 좁은 conformance authoring profile입니다. 이것은 내부 구현 milestone이지 사용자 대상 MVP가 아닙니다. 첫 제품 MVP 목표는 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)입니다.

## 이런 때 읽기

- 코어 권한 조각(v0.1 Core Authority Slice)을 계획할 때.
- 처음부터 끝까지 이어지는 첫 authority path를 위한 점검 목록이 필요할 때.
- 제안된 첫 조각이 제품 MVP로 커지지 않을 만큼 작은지 검토할 때.

## 읽기 전에

[구현 개요](implementation-overview.md)를 먼저 읽고 [문서 승인 상태](implementation-overview.md#문서-승인-상태)를 확인합니다. 그 handoff 표가 Build 진입 gate입니다. Maintainer가 첫 runtime batch 계획을 승인하기 전까지 이 조각은 planning-only입니다. Storage와 DDL의 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)을 봅니다. 이 조각 이후의 staged delivery는 [MVP 계획](mvp-plan.md)을 사용합니다. v1+ Expansion 후보는 [로드맵](../roadmap.md)을 봅니다.

## 핵심 생각

Task 하나가 가장 작은 Core authority record를 통과할 수 있음을 증명합니다. 그 기록은 project registration, scope 하나, write authorization decision 하나, authorized Run 하나, evidence link 하나, structured blocker/status response 하나로 구성됩니다.

첫 조각은 하네스 state가 local, durable, authoritative하다는 점을 보여야 하지만 user-facing product 전체를 증명하려고 하면 안 됩니다. `prepare_write`는 product-write authorization decision point로, Write Authorization은 durable하고 single-use한 기록으로, `record_run`은 compatible Run 하나가 authority를 consume하는 곳으로, `close_task` 또는 status는 missing evidence나 required judgment를 structured blocker로 보고하는 곳으로 유지합니다.

정확한 contract는 [커널 참조](../reference/kernel.md#prepare_write)와 [MCP API와 스키마](../reference/mcp-api-and-schemas.md#public-tools)를 사용합니다.

## 목표

코어 권한 조각(v0.1 Core Authority Slice)을 계획합니다. 하나의 Task에 대해 local authority를 증명하는 가장 작은 Harness path입니다.

이 조각은 다음을 만들거나 seed해야 합니다.

- project 하나와 reference surface 하나
- Task 하나
- intended change를 위한 basic scope 하나
- 허용된 `prepare_write` decision 하나와 최소 하나의 blocked decision
- durable single-use Write Authorization 하나
- 그 authorization을 consume하는 compatible recorded Run 하나
- Run 또는 evidence relation에 연결된 artifact 또는 evidence ref 하나
- selected claim을 support하거나 fail할 수 있는 minimal evidence state 하나
- read-only status/next response 하나
- scope, evidence, 또는 required seeded user judgment가 missing일 때 structured blocker/status response 하나

이 문서는 특정 command에 묶이지 않는 구현 안내서입니다. CLI 문법이 아니라 기능과 관찰 가능한 동작을 설명합니다. 여기에 전체 DDL을 포함하거나 반복하지 않습니다. Storage와 DDL의 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)이 담당합니다.

첫 조각은 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP), 강화된 로컬 기준 목표(hardened local reference target) 전체, projection template polish milestone, dashboard 또는 hosted-workflow-UI milestone, broad connector ecosystem 또는 marketplace milestone, multi-surface connector expansion, Context Index, Browser QA Capture system, Cross-Surface Verification path, hook expansion, preventive guard expansion, Advanced Sidecar Watcher, Local Derived Metrics surface, team workflow, export/recover path, release handoff path, parallel automation path가 아닙니다.

## 성공 이야기

구현자는 임시 제품 저장소에 대해 로컬 Harness 프로세스를 실행한다고 가정했을 때 다음 흐름을 관찰할 수 있어야 합니다.

1. Harness가 project 하나와 reference surface 하나를 등록한다.
2. Task가 Core state에 존재하고 변경 시 `task_events`가 추가된다.
3. Basic scope가 intended product change를 이름 붙인다.
4. `prepare_write`가 missing 또는 incompatible scope를 차단한다.
5. `prepare_write`가 compatible scoped write 하나를 허용하고 durable single-use Write Authorization을 만든다.
6. `record_run`이 compatible Run 하나를 기록하고 그 authorization을 한 번 consume한다.
7. Artifact 또는 evidence ref 하나가 등록되어 Run 또는 evidence relation에 연결된다.
8. Status와 next read가 상태를 변경하지 않고 current Task, scope, write authority, evidence state, blocker를 보여 준다.
9. Required evidence, scope, 또는 required seeded user judgment가 missing이면 close 또는 status output이 structured blocker를 반환한다.

이 흐름을 통과하면 코어 권한 조각(v0.1 Core Authority Slice)이 동작한다는 뜻입니다. 사용자가 Harness MVP를 경험했다는 뜻은 아닙니다. 사용자 대상 MVP는 평범한 요청이 scope, judgment, evidence, close-readiness, acceptance, residual-risk language로 정리될 때 시작됩니다.

## 문서 수준 승인 점검

Executable fixture가 생기기 전에는 이 점검으로 계획된 코어 권한 조각(v0.1 Core Authority Slice)을 리뷰하고, [커널 스모크(Kernel Smoke) Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue)에 매핑할 때 다시 사용합니다. 이는 planning check이며 fixture body field, schema 추가, DDL, runtime authorization이 아닙니다.

제안된 첫 실행 가능한 조각은 다음을 만족할 때 적절합니다.

- Local, single-project, single-reference-surface 범위를 유지하고 Task 하나의 authority loop에 집중한다.
- [문서 승인 상태](implementation-overview.md#문서-승인-상태)가 첫 runtime batch 계획을 명시적으로 허용하기 전까지 planning-only로 남는다.
- Active Task, basic scope 하나, `prepare_write` allow/block, durable single-use Write Authorization, `record_run` consumption, artifact/evidence link, read-only status/next, structured blocker/status response로 이루어진 scoped write path 하나만 증명한다.
- Missing scope, out-of-scope intended path, product-write Run의 missing Write Authorization, consumed Write Authorization 재사용, required evidence 누락, required seeded user judgment 누락처럼 authority가 부족한 경우를 block하거나 refuse한다.
- Status read, generated prose, projection output이 있다면 모두 Core record의 downstream으로 유지한다. 이들은 write를 authorize하거나, evidence를 충족하거나, work를 close하거나, 읽히는 것만으로 state를 repair하지 않는다.
- Strict fixture body shape, assertion mode, primary error, artifact refs, projection assertion, seed validation은 여기서 복사하지 않고 [Conformance Fixtures 참조](../reference/conformance-fixtures.md#conformance-fixture-format)에 연결한다.
- 제외된 capability는 코어 권한 조각(v0.1 Core Authority Slice)의 failed requirement가 아니라 아직 첫 조각이 증명하지 않은 capability로 이름 붙인다.

아래 Build 순서는 문서 승인 이후를 위한 planning sequence입니다. Heading은 future runtime batch를 실행하기 쉽도록 구현 동사를 사용하지만, 이 문서는 문서 승인 전에 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture, runtime data를 만들라는 뜻이 아닙니다.

## Build 순서

### 1. Runtime Home And Project Registration

대화 기록이나 생성된 Markdown 밖에 로컬 하네스 권한을 만들 수 있을 만큼의 runtime home support를 계획한 뒤, 로컬 제품 저장소 하나만 등록합니다.

점검 목록:

- 구성 가능한 runtime home을 만들거나 선택한다.
- Registry store, project runtime area 하나, project state store 하나, artifact store를 초기화한다.
- 프로젝트 범위 상태 변경이 의존할 project-level state version을 먼저 기록한다.
- project id, display name, repo root, runtime path, 정적 프로젝트 설정을 저장한다.
- honest cooperative 또는 detective guarantee level을 가진 reference surface 하나를 등록한다.
- "active Task 없음"을 보고할 수 있는 read-only status를 제공한다.

완료 기준:

- 새 환경을 반복 초기화해도 중복 권한 기록이 생기지 않는다.
- Core가 이후 모든 Task-scoped action에 대해 현재 프로젝트를 찾을 수 있다.
- Status가 unregistered 또는 idle project와 active Task를 구분할 수 있다.

### 2. One Task Record

Core 또는 같은 검증 규칙을 사용하는 fixture seed path를 통해 첫 Task를 만듭니다.

점검 목록:

- Task id, lifecycle phase, state version, current summary, 첫 조각에 필요한 최소 gate/status state를 저장한다.
- Task가 생성되거나 변경될 때 `task_events`에 추가한다.
- 상태 조회에서 active Task 조회를 제공한다.
- Mode policy depth, intake quality, procedural budget routing은 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)에 둔다.

완료 기준:

- 시스템이 active Task 하나와 그 state version을 보여 줄 수 있다.
- Owner contract가 요구하는 경우 오래된 expected state version을 가진 상태 변경 request가 reject되거나 state conflict를 반환한다.

### 3. One Basic Scope

제품 파일 쓰기 하나를 constrain할 수 있는 가장 작은 scope record를 추가합니다. Change Unit이 owner shape일 수 있지만, 첫 조각은 dependency graph, full Autonomy Boundary policy, multi-lane orchestration으로 커지면 안 됩니다.

점검 목록:

- Selected write에 필요한 intended operation과 allowed paths 또는 command/tool class를 기록한다.
- Scope를 active Task에 연결한다.
- Selected claim에 필요한 최소 evidence expectation만 기록한다.
- Full Discovery와 user-facing procedural budget routing은 v0.2에 둔다.

완료 기준:

- Status가 무엇을 바꿀 수 있는지 설명할 수 있다.
- Active compatible scope 없는 product write는 write authority를 받을 수 없다.

### 4. `prepare_write` Allow/Block

첫 의미 있는 write gate를 계획합니다.

점검 목록:

- 필요한 경우 request envelope, idempotency key, project id, Task id, expected state version을 검증한다.
- Active Task와 active scope를 찾는다.
- Intended paths, tools, commands, network targets, secrets, sensitive categories를 selected write에 필요한 최소 수준에서 확인한다.
- 첫 조각에 필요한 수준으로 baseline freshness를 확인한다.
- Scope, state version, baseline, capability, 또는 seeded required judgment가 맞지 않으면 structured blocker를 반환한다.
- 허용되면 하나의 later direct Run 또는 implementation Run과 호환되는 durable single-use Write Authorization을 만든다.

완료 기준:

- Scope가 없으면 차단된다.
- 범위 밖의 intended path는 차단된다.
- Compatible scoped write가 Write Authorization ref를 반환한다.
- 그 ref 없이는 product-write Run으로 제품 쓰기를 기록할 수 없다.

### 5. `record_run`

direct Run 또는 implementation Run 하나를 기록하고 Write Authorization을 한 번 사용한 것으로 기록합니다.

점검 목록:

- Selected product-write Run에는 compatible, unexpired, unconsumed Write Authorization을 요구한다.
- Successful commit에서 Write Authorization을 정확히 한 번 consumed로 표시한다.
- Actor, surface, kind, intended operation, observed changes, artifact refs 또는 evidence inputs, summary, Run status를 첫 조각에 필요한 최소 수준으로 기록한다.
- Observed changed paths와 artifact refs를 authorization 및 scope와 비교해 validate한다.
- 현재 기록 갱신과 같은 transaction에서 `task_events`에 추가한다.

완료 기준:

- 쓰기 권한 없는 `record_run`이 차단된다.
- Compatible authority가 있는 `record_run`이 한 번 성공한다.
- 두 번째 distinct Run은 consumed authorization을 재사용할 수 없다.

### 6. Artifact Or Evidence Link

Owner path를 통해 durable evidence file 하나 또는 equivalent evidence ref 하나를 등록합니다.

점검 목록:

- Approved staged file 또는 기존 committed artifact ref를 받는다.
- 제공된 hash와 size를 검증한다.
- Relevant한 경우 최종 저장 전에 redaction 또는 secret omission을 적용한다.
- Artifact metadata와 Task, Run, evidence relation, 또는 다른 owner record와의 relation을 저장한다.
- API docs의 public shape를 따르는 `ArtifactRef` 또는 owner-defined evidence ref를 반환한다.

완료 기준:

- Run이 registered artifact 또는 evidence ref를 참조할 수 있다.
- Raw secret은 evidence로 저장하지 않고 omitted 또는 blocked 처리된다.

### 7. Minimal Evidence State

Selected claim이 supported인지 설명하는 데 필요한 가장 작은 evidence relation을 만듭니다.

점검 목록:

- Completion condition 또는 acceptance criterion 하나를 Run과 artifact/evidence ref에 매핑한다.
- Close/status blocker에 필요한 수준에서 supported, partial, insufficient evidence를 구분한다.
- Chat text, status prose, projection prose를 evidence로 취급하지 않는다.

완료 기준:

- Completed Run이 supported 또는 partial evidence state를 만들 수 있다.
- Required evidence가 없으면 close/status output이 block된다.

### 8. Status, Next, And Structured Blockers

현재 작업 상태와 다음 safe action을 변경 없이 제공하고, 첫 조각이 close 또는 proceed할 수 없을 때 structured blocker를 반환합니다.

점검 목록:

- Project, active Task, current scope, write authority summary, evidence status, close/status blockers, next safe action을 읽는다.
- Missing scope, missing evidence, missing Write Authorization, reused authorization, seeded required user judgment를 structured blocker로 보고한다.
- Read 동작에서 event를 추가하거나, projection을 대기열에 넣거나, artifact를 만들거나, gate를 충족하거나, write를 authorize하거나, Task를 닫지 않는다.

완료 기준:

- 다른 action이 상태를 바꾸지 않는 한 반복 status/next read가 같은 state version을 반환한다.
- Structured blocker를 prose matching 없이 fixture가 비교할 수 있다.
- Close/status result가 생성된 보고서가 아니라 기준 기록에 근거한다.

## 이것이 증명하는 것

첫 실행 가능한 조각은 다음을 증명합니다.

- Core가 상태 전이를 소유할 수 있다.
- State store와 `task_events`를 사용할 수 있다.
- Product write에는 scoped record가 필요하다.
- `prepare_write`가 product-write authorization decision point다.
- Write Authorization은 durable하고 single-use다.
- `record_run`이 write authority를 한 번 사용한 것으로 기록하고 observed work를 기록한다.
- Artifact/evidence link 하나가 recorded Run을 뒷받침할 수 있다.
- Evidence가 chat에 의존하지 않고 insufficient 상태일 수 있다.
- Status와 next는 read-only다.
- Structured blocker가 missing scope, evidence, authorization, seeded required user judgment를 보고할 수 있다.

## 아직 증명하지 않는 것

이 조각은 아래 항목을 아직 증명하지 않습니다. 이들은 stage boundary이지 failed v0.1 requirement가 아닙니다.

| 이후 단계 | 코어 권한 조각(v0.1 Core Authority Slice)이 아직 증명하지 않는 것 |
|---|---|
| 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP) | Natural-language intake quality, Discovery, product/UX versus architecture judgment presentation, small-change versus tracked-work budgets, residual-risk display, final acceptance separation, user-facing projection/card sufficiency. |
| 보증과 스튜어드십 팩(v0.3 Assurance & Stewardship Pack) | Full Decision Packet quality, full Approval lifecycle and drift handling, detached verification independence, Manual QA policy matrix, residual-risk accepted close, feedback-loop policy, TDD trace, codebase stewardship, stewardship validators, context hygiene. |
| 운영과 인계 팩(v0.4 Operations & Handoff Pack) | Release handoff, recover, export, artifact integrity operations, broad operator smoke, broader fixture suite coverage. |
| v1+ Expansion | Dashboard, hosted workflow UI, Context Index, connector marketplace, Browser QA Capture, Cross-Surface Verification automation, native hook expansion, Advanced Sidecar Watcher, Local Derived Metrics, preventive guard expansion, parallel orchestration, team workflow. |

## 작성할 Fixture

Core 동작을 실행하고 state, events, artifacts, applicable한 projection 또는 freshness, errors를 검증하는 fixture를 작성합니다. Rendered prose matching만으로 success를 검증하지 않습니다. 이 row들은 future authoring candidate이며 executable fixture file이 지금 존재한다고 암시하지 않습니다.

각 runtime fixture는 격리된 runtime home과 임시 제품 저장소에서 실행되어야 하며, 자기 시작 record와 file을 seed하고, Core 또는 operator action 하나를 실행한 뒤 captured executable result를 비교해야 합니다. Fixture body field, `partial_deep`과 `contains_ordered` 같은 assertion mode, JSON `TEXT` validation, owner-bound status value validation은 [Conformance Fixtures 참조](../reference/conformance-fixtures.md#conformance-fixture-format)가 담당합니다.

첫 조각의 최소 fixture candidate:

- no-active-task status read가 idle state를 반환하고 event를 추가하지 않음
- project registration이 project state와 reference surface를 만듦
- Task creation 또는 seed가 active Task 하나와 task event behavior를 만듦
- basic scope가 intended path 하나를 허용하고 그 자체로 write authority를 만들지 않음
- `prepare_write`가 scope 없을 때 차단함
- `prepare_write`가 out-of-scope path를 차단함
- `prepare_write`가 compatible scoped write 하나를 허용하고 Write Authorization 하나를 만듦
- `record_run`이 write authority 없을 때 차단함
- `record_run`이 compatible Write Authorization을 한 번 consume함
- 두 번째 distinct `record_run`이 consumed authorization을 재사용할 수 없음
- artifact 또는 evidence ref registration이 integrity/redaction metadata와 owner relation을 저장함
- minimal evidence state가 supported, partial, insufficient support를 보고함
- status와 next read가 상태 변경 없이 current Task, scope, write authority, evidence, blockers, next safe action을 표시함
- close/status output이 structured blocker와 함께 missing evidence를 차단함
- close/status output이 structured blocker와 함께 seeded required user judgment를 차단함

실제 순서, seed guidance, stable event target, artifact/projection assertion, primary-error expectation은 [커널 스모크(Kernel Smoke) Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue)를 사용합니다. Suite stage, authoring order, docs-maintenance result를 표현하기 위해 fixture body에 field를 추가하지 않습니다.

## 참고할 Reference 문서

- [커널 참조](../reference/kernel.md): Task, Change Unit, Decision Packet, gate, `prepare_write`, Write Authorization, `record_run` semantics, `close_task`.
- [런타임 아키텍처 참조](../reference/runtime-architecture.md): 세 공간, Core process model, transaction flow, artifact store, projection/reconcile, guarantee level, failure handling.
- [MCP API와 스키마](../reference/mcp-api-and-schemas.md): public resource, tool envelope, request/response schema, error taxonomy, 아티팩트 참조, `ProjectionKind`.
- [Storage와 DDL](../reference/storage-and-ddl.md): runtime layout과 DDL, migration, lock, artifact, baseline, projection job, validator-run storage를 다룹니다.
- [운영과 Conformance 참조](../reference/operations-and-conformance.md): operator 의미와 conformance 단계화.
- [Conformance Fixtures 참조](../reference/conformance-fixtures.md): fixture 형식, 실행, assertion rule, suite catalog, example.
