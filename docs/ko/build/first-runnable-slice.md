# Build: 첫 실행 가능한 조각

## 이 문서가 도와주는 일

이 문서는 Build 개요를 구현자가 가장 먼저 계획해야 하는 v0.1 Core Authority Smoke으로 바꿉니다.

이 문서는 구현 계획 문서입니다. 문서 수락과 별도의 구현 계획 준비 결정 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture, fixture 파일, 런타임 데이터를 만들라는 뜻이 아닙니다. Conformance fixture 문서는 향후 적합성 검증 계획이며, 현재 문서 전용 저장소에는 runnable Harness Server conformance test가 없습니다. 첫 실행 목표는 v0.1 Core Authority Smoke이며, 커널 스모크(Kernel Smoke)는 좁은 future smoke-check 작성 label입니다. 이것은 내부 smoke 단계이지 제품 MVP가 아닙니다. 첫 사용자 가치 목표는 v0.2 First User-Value Slice입니다.

## 이런 때 읽기

- v0.1 Core Authority Smoke을 계획할 때.
- 처음부터 끝까지 이어지는 첫 권한 경로를 위한 점검 목록이 필요할 때.
- 제안된 첫 조각이 제품 MVP나 v0.2 First User-Value Slice로 커지지 않을 만큼 작은지 검토할 때.

## 읽기 전에

[구현 개요](implementation-overview.md)를 먼저 읽고 [문서 수락 상태](implementation-overview.md#문서-수락-상태)를 확인합니다. 그 인계 표가 Build 진입 기준입니다. 유지보수자가 첫 런타임 배치를 위한 구현 계획 준비 상태를 수락하기 전까지 이 조각은 계획 전용입니다. Storage와 DDL의 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)을 봅니다. 이 조각 이후의 단계별 전달은 [단계별 전달 계획](mvp-plan.md)을 사용합니다. v1+ Expansion 후보는 [로드맵](../roadmap.md)을 봅니다.

## 핵심 생각

active Task 하나가 가장 작은 Core 권한 기록을 통과할 수 있음을 증명합니다. 그 기록은 local project registration, active Task, scoped boundary, 쓰기 허가 판단, 허가된 Run, artifact/evidence ref, structured status/blocker 응답으로 구성됩니다.

첫 조각은 Harness 상태가 로컬에 있으며, 지속적이고, 기준이 된다는 점을 보여야 합니다. 하지만 사용자에게 보이는 제품 경험 전체를 증명하려고 하면 안 됩니다. `prepare_write`는 제품 파일 쓰기 허가 판단 지점으로, Write Authorization은 지속적이며 한 번만 쓸 수 있는 기록으로, `record_run`은 호환되는 Run 하나가 권한을 소비하는 곳으로, status/blocker 출력은 missing scope, missing write authority, 또는 missing artifact/evidence support를 구조화된 막힘으로 보고하는 곳으로 유지합니다. Owner path가 이미 그 방식을 가장 단순한 blocker 응답으로 만든 경우 `close_task` smoke를 사용할 수 있지만, v0.1은 작업 수락이나 잔여 위험 close semantics를 증명하지 않습니다.

정확한 계약은 [커널 참조](../reference/kernel.md#prepare_write)와 [MCP API와 스키마](../reference/mcp-api-and-schemas.md#public-tools)를 사용합니다.

API staging에서는 MCP API [Stage Profile Manifest](../reference/mcp-api-and-schemas.md#stage-profile-manifest)에서 시작하고 v0.1 surface만 사용합니다. 범위는 minimal `harness.status` status/blocker read, `harness.prepare_write`, `harness.record_run`, owner-valid Task/scope setup path 하나, optional minimal `harness.next` 또는 narrow `harness.close_task` blocker smoke입니다. Later-profile field는 해당 profile이 active일 때 exact하게 유지되지만 first-slice exit criteria는 아닙니다.

## 목표

v0.1 Core Authority Smoke을 계획합니다. 하나의 Task에 대해 로컬 권한을 증명하는 가장 작은 Harness 경로입니다.

이 조각은 다음을 만들거나 seed해야 합니다.

- local project registration 하나
- active Task 하나
- intended change를 위한 scoped boundary 하나
- 허용된 `prepare_write` decision 하나와 최소 하나의 blocked decision
- 지속적이며 한 번만 쓸 수 있는 Write Authorization 하나
- 그 authorization을 소비하는 호환되는 기록된 Run 하나
- Run 또는 minimal owner relation에 연결된 artifact/evidence ref 하나
- scope, write authority, 또는 artifact/evidence support가 없을 때 structured status/blocker 응답 하나

이 문서는 특정 command에 묶이지 않는 구현 안내서입니다. CLI 문법이 아니라 기능과 관찰 가능한 동작을 설명합니다. 여기에 전체 DDL을 포함하거나 반복하지 않습니다. Storage와 DDL의 세부 내용은 [Storage와 DDL](../reference/storage-and-ddl.md)이 담당합니다.

Storage planning에서는 v0.1에 [Core Authority Smoke schema](../reference/storage-and-ddl.md#core-authority-smoke-schema)만 사용합니다. Decision Packet, Approval, Evidence Manifest, Manual QA, Eval, projection job, reconcile item, validator run, Journey record, diagnostic 같은 later storage profile은 first-slice requirement가 아닙니다.

첫 조각은 v0.2 First User-Value Slice, 제품 MVP, 강화된 로컬 기준 목표(hardened local reference target) 전체, natural-language intake, full Discovery, full Decision Packet, full Evidence Manifest, Eval, Manual QA, Acceptance, residual-risk acceptance, full close semantics, detached verification, work-acceptance semantics, projection rendering, Projection 템플릿 다듬기 단계, 여러 projection kind, dashboard 또는 hosted-workflow-UI 단계, 넓은 connector ecosystem 또는 marketplace 단계, multi-surface connector expansion, Context Index, Browser QA Capture system, Cross-Surface Verification path, hook expansion, preventive guard expansion, Advanced Sidecar Watcher, Local Derived Metrics surface, team workflow, operations/export/recover path, release handoff path, conformance runner, 넓은 operator-entrypoint path, 향후 fixture catalog, parallel automation path가 아닙니다.

## 성공 이야기

향후 v0.1 implementation이 생기면, 구현자는 임시 제품 저장소에 대해 로컬 Harness 프로세스를 실행하고 다음 흐름을 관찰할 수 있어야 합니다.

1. Harness가 local project 하나를 등록한다.
2. Task가 Core-owned state에 존재한다.
3. scoped boundary가 intended product change를 이름 붙인다.
4. `prepare_write`가 missing 또는 incompatible scope를 차단한다.
5. `prepare_write`가 호환되는 scoped write 하나를 허용하고 지속적이며 한 번만 쓸 수 있는 Write Authorization을 만든다.
6. `record_run`이 호환되는 Run 하나를 기록하고 그 authorization을 한 번 소비한다.
7. Artifact/evidence ref 하나가 등록되어 Run 또는 minimal owner relation에 연결된다.
8. status/blocker 출력이 상태를 변경하지 않고 현재 Task, scope, 쓰기 권한, artifact/evidence support, blocker를 보여 준다.
9. Scope, write authority, 또는 artifact/evidence support가 없으면 status 또는 close-task smoke가 structured blocker를 반환한다.

이 흐름을 통과하면 v0.1 Core Authority Smoke이 동작한다는 뜻입니다. 사용자가 Harness value를 경험했다는 뜻은 아닙니다. v0.2 First User-Value Slice는 평범한 요청이 tracked work로 시작/재개되고 scope, non-goals, success criteria, user-owned judgment, evidence summary, close blockers, work acceptance display, residual-risk visibility로 요약될 때 시작됩니다.

## 문서 수준 수락 점검

Executable fixture가 생기기 전에는 이 점검으로 계획된 v0.1 Core Authority Smoke을 리뷰하고, [커널 스모크(Kernel Smoke) Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue)에 매핑할 때 다시 사용합니다. 이는 planning check이며 fixture body field, schema 추가, DDL, runtime authorization이 아닙니다.

제안된 첫 실행 가능한 조각은 다음을 만족할 때 적절합니다.

- 로컬, 단일 프로젝트 범위를 유지하고 Task 하나의 권한 루프에 집중한다.
- [문서 수락 상태](implementation-overview.md#문서-수락-상태)가 첫 런타임 배치에 대한 구현 계획 준비 상태를 명시적으로 수락하기 전까지 계획 전용으로 남는다.
- Active Task, scoped boundary 하나, `prepare_write` allow/block, 지속적이며 한 번만 쓸 수 있는 Write Authorization, `record_run` 소비, artifact/evidence ref, structured status/blocker 응답으로 이루어진 scoped write path 하나만 증명한다.
- Missing scope, out-of-scope intended path, product-write Run의 missing Write Authorization, consumed Write Authorization 재사용, missing artifact/evidence support처럼 권한이 부족한 경우를 block하거나 refuse한다.
- Status read, generated prose, projection output이 있다면 모두 Core record에서 파생된 것으로 유지한다. 이들은 write를 허가하거나, 근거를 충족하거나, work를 close하거나, 읽히는 것만으로 상태를 복구하거나, conformance truth가 되지 않는다.
- Projection-like output은 v0.1에서 status/blocker 출력으로 취급한다. Full projection renderer, 여러 projection kind, detailed template은 필요하지 않다.
- 향후 strict fixture body shape, assertion mode, primary error, artifact refs, optional projection assertion, seed validation은 여기서 복사하지 않고 [Conformance Fixtures 참조](../reference/conformance-fixtures.md#conformance-fixture-format)에 연결한다.
- 제외된 capability는 v0.1 Core Authority Smoke의 failed requirement가 아니라 아직 첫 조각이 증명하지 않은 capability로 이름 붙인다.

아래 Build 순서는 문서 수락과 구현 계획 준비 결정 이후를 위한 계획 순서입니다. Heading은 future runtime batch를 실행하기 쉽도록 구현 동사를 사용하지만, 이 문서는 문서 수락과 별도의 구현 계획 준비 결정 전에 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture, 런타임 데이터를 만들라는 뜻이 아닙니다.

## Build 순서

### 1. Runtime Home And Project Registration

대화 기록이나 생성된 Markdown 밖에 로컬 하네스 권한을 만들 수 있을 만큼의 runtime home support를 계획한 뒤, 로컬 제품 저장소 하나만 등록합니다.

계획 초점:

- 나중에 Task-scoped action이 사용할 로컬 project 하나를 찾을 수 있게 만든다.
- Runtime home, registry, project state, artifact store, 정적 프로젝트 설정은 storage owner path에 둔다.
- Unregistered, registered-idle, active-work 상태를 구분하는 읽기 전용 상태 응답을 제공한다.

완료 기준:

- 새 환경을 반복 초기화해도 중복 권한 기록이 생기지 않는다.
- Core가 이후 모든 Task-scoped action에 대해 현재 프로젝트를 찾을 수 있다.
- Status가 unregistered 또는 idle project와 active Task를 구분할 수 있다.

Owner contract: runtime home layout과 v0.1에서 사용하는 Core Authority Smoke schema는 [Storage와 DDL](../reference/storage-and-ddl.md#core-authority-smoke-schema)이 담당합니다. Local space와 guarantee-level placement는 [런타임 아키텍처 참조](../reference/runtime-architecture.md)가 담당하고, guarantee-level 의미는 [보안 위협 모델 참조](../reference/security-threat-model.md#정직한-guarantee-display)가 담당하며, connector reporting은 [Agent 통합 참조](../reference/agent-integration.md)가 담당합니다.

### 2. One Task Record

Core 또는 같은 검증 규칙을 사용하는 fixture seed path를 통해 첫 Task를 만듭니다.

계획 초점:

- Owner-valid path를 통해 active Task 하나만 만들거나 seed한다.
- Status와 이후 Core action이 Task를 참조할 수 있을 만큼 현재 상태를 유지한다.
- Natural-language intake quality, work-shape classification, small direct work와 tracked work의 구분은 v0.2 First User-Value Slice에 둔다.

완료 기준:

- 시스템이 active Task 하나와 그 state version을 보여 줄 수 있다.
- Owner contract가 요구하는 경우 오래된 expected state version을 가진 상태 변경 request가 reject되거나 state conflict를 반환한다.

Owner contract: Task lifecycle과 state conflict behavior는 [커널 참조](../reference/kernel.md#task), [Lifecycle and transitions](../reference/kernel.md#lifecycle-and-transitions), [MCP API와 스키마](../reference/mcp-api-and-schemas.md#state-conflict-동작)가 담당합니다.

### 3. One Basic Scope

제품 파일 쓰기 하나를 제한할 수 있는 가장 작은 scope record를 추가합니다. Change Unit이 소유자 형태일 수 있지만, 첫 조각은 dependency graph, full Autonomy Boundary policy, multi-lane orchestration으로 커지면 안 됩니다.

계획 초점:

- Owner-valid scope 하나를 active Task에 연결한다.
- 선택한 intended write를 그 scope와 비교할 수 있게 만든다.
- 첫 authority-loop claim에 필요한 artifact/evidence support만 유지한다.
- Full Discovery와 사용자에게 보이는 work-shape routing은 v0.2 First User-Value Slice에 둔다.

완료 기준:

- Status가 무엇을 바꿀 수 있는지 설명할 수 있다.
- Active compatible scope 없는 product write는 write authority를 받을 수 없다.

Owner contract: Change Unit과 Autonomy Boundary semantics는 [커널 참조](../reference/kernel.md#change-unit)와 [Autonomy Boundary](../reference/kernel.md#autonomy-boundary)가 담당합니다.

### 4. `prepare_write` Allow/Block

첫 의미 있는 write gate를 계획합니다.

계획 초점:

- 선택한 product-write attempt를 owner `prepare_write` path로 보낸다.
- 호환되는 scoped write 하나만 허용하거나 owner-shaped blocker를 반환한다.
- Candidate Approval 또는 Decision Packet material은 owning path가 commit하기 전까지 candidate context로 유지한다.

완료 기준:

- Scope가 없으면 차단된다.
- 범위 밖의 intended path는 차단된다.
- 호환되는 scoped write가 Write Authorization ref를 반환한다.
- 그 ref 없이는 product-write Run으로 제품 쓰기를 기록할 수 없다.

Owner contract: write-gate semantics는 [커널 참조: prepare_write](../reference/kernel.md#prepare_write)가 담당합니다. Public request/response shape와 error precedence는 [`harness.prepare_write`](../reference/mcp-api-and-schemas.md#harnessprepare_write)와 [Primary Error Code Precedence](../reference/mcp-api-and-schemas.md#primary-error-code-precedence)가 담당합니다.

### 5. `record_run`

direct Run 또는 implementation Run 하나를 기록하고 Write Authorization을 한 번 사용한 것으로 기록합니다.

계획 초점:

- 선택한 direct 또는 implementation write에 대해 owner-valid Run 하나를 기록한다.
- 호환되는 Write Authorization을 한 번만 소비한다.
- Observed changes, artifacts, events, state update는 Core transaction model 안에 둔다.

완료 기준:

- 쓰기 권한 없는 `record_run`이 차단된다.
- 호환되는 권한이 있는 `record_run`이 한 번 성공한다.
- 두 번째 distinct Run은 consumed authorization을 재사용할 수 없다.

Owner contract: Run semantics는 [커널 참조: record_run](../reference/kernel.md#record_run)이 담당합니다. Public schema는 [`harness.record_run`](../reference/mcp-api-and-schemas.md#harnessrecord_run)이 담당합니다. Transaction order는 [State transaction flow](../reference/runtime-architecture.md#state-transaction-flow)가 담당합니다.

### 6. Artifact Or Evidence Link

Owner path를 통해 지속적으로 보관할 evidence file 하나 또는 동등한 evidence ref 하나를 등록합니다. v0.1은 reference와 owner link만 필요하며, full Evidence Manifest model이나 rendered `EVIDENCE-MANIFEST` output은 필요하지 않습니다.

계획 초점:

- Owner path를 통해 artifact 또는 evidence ref 하나를 등록한다.
- 그 ref를 Run, evidence relation, 또는 사용하는 owner record에 연결한다.
- Redaction, omission, integrity, retention boundary는 storage/API owner를 따른다.

완료 기준:

- Run이 registered artifact 또는 evidence ref를 참조할 수 있다.
- Raw secret은 evidence로 저장하지 않고 omitted 또는 blocked 처리된다.

Owner contract: artifact ref는 [ArtifactRef](../reference/mcp-api-and-schemas.md#artifactref)가 담당합니다. Storage layout과 registration detail은 [Artifact directory layout](../reference/storage-and-ddl.md#artifact-directory-layout) 및 [Artifact Registration Contract](../reference/storage-and-ddl.md#artifact-registration-contract)가 담당합니다.

### 7. Status And Structured Blockers

현재 작업 상태를 변경 없이 제공하고, 첫 조각이 proceed할 수 없을 때 구조화된 막힘을 반환합니다.

계획 초점:

- 현재 Task, scope, 쓰기 권한 요약, artifact/evidence support, blockers를 canonical record에서 반환한다.
- Smoke check가 prose matching 없이 비교할 수 있을 만큼 blocker identity를 구조화된 형태로 유지한다.
- Read 동작에서 event를 추가하거나, projection을 대기열에 넣거나, artifact를 만들거나, gate를 충족하거나, write를 authorize하거나, Task를 닫지 않는다.

완료 기준:

- 다른 action이 상태를 바꾸지 않는 한 반복 status 읽기가 같은 state version을 반환한다.
- 구조화된 막힘을 prose matching 없이 비교할 수 있다.
- Close/status result가 생성된 보고서가 아니라 기준 기록에 근거한다.

Owner contract: status/next schema는 [`harness.status`](../reference/mcp-api-and-schemas.md#harnessstatus)와 [`harness.next`](../reference/mcp-api-and-schemas.md#harnessnext)가 담당합니다. Close behavior는 [커널 참조: close_task](../reference/kernel.md#close_task)가 담당합니다.

## 이것이 증명하는 것

첫 실행 가능한 조각은 다음을 증명합니다.

- Core가 상태 전이를 소유할 수 있다.
- Product write에는 scoped record가 필요하다.
- `prepare_write`가 제품 파일 쓰기 허가 판단 지점이다.
- Write Authorization은 지속적이며 한 번만 쓸 수 있다.
- `record_run`이 쓰기 권한을 한 번 사용한 것으로 기록하고 observed work를 기록한다.
- Artifact/evidence link 하나가 기록된 Run을 뒷받침할 수 있다.
- Artifact/evidence support가 chat에 의존하지 않고 missing 상태일 수 있다.
- status/blocker read는 읽기 전용이다.
- structured blocker가 missing scope, missing write authority, missing artifact/evidence support를 보고할 수 있다.

## 아직 증명하지 않는 것

이 조각은 아래 항목을 아직 증명하지 않습니다. 이들은 stage boundary이지 failed v0.1 requirement가 아닙니다.

| 이후 단계 | v0.1 Core Authority Smoke이 아직 증명하지 않는 것 |
|---|---|
| v0.2 First User-Value Slice | Ordinary-language start/resume, work-shape classification, natural-language intake 품질, scope/non-goals/success criteria summary, minimal user judgment request/record, product/UX judgment와 architecture judgment 제시 방식, small direct와 tracked-work의 budget 구분, evidence summary, close blocker summary, residual-risk visibility, work-acceptance display, sensitive approval display, risk-acceptance display, compact Core-derived status card 충분성. |
| 에이전시 보증 팩(v0.3 Agency Assurance Pack) | Profile별 Decision Packet 품질, full Approval lifecycle and drift handling, 분리 검증 독립성, 수동 QA 정책 매트릭스, residual-risk accepted close, 작업 수락 분리, feedback-loop policy, TDD trace, codebase stewardship, stewardship validators, context hygiene. |
| 운영과 인계 팩(v0.4 Operations & Handoff Pack) | Release handoff, recover, export, artifact integrity operations, broad operator smoke, broader fixture suite coverage, full projection/reconcile operations. |
| v1+ Expansion | Dashboard, hosted workflow UI, Context Index, connector marketplace, Browser QA Capture, Cross-Surface Verification automation, native hook expansion, Advanced Sidecar Watcher, Local Derived Metrics, preventive guard expansion, parallel orchestration, team workflow. |

## 향후 Smoke Checks

문서 수락과 구현 계획 준비 인계 이후에는 Core 동작을 실행하고 minimal owner record, artifact/evidence ref, structured status/blocker 응답, errors를 확인하는 가장 작은 Kernel Smoke check로 Core Authority Smoke을 매핑합니다. Rendered prose나 polished projection output matching만으로 success를 검증하지 않습니다. 이 행들은 future authoring candidate이며 executable fixture file이 지금 존재한다거나 전체 conformance suite가 필요하다고 암시하지 않습니다.

Build는 v0.1 scope intent만 담당합니다. 범위는 local project registration, active Task 하나, scoped boundary 하나, `prepare_write` allow/block, 한 번만 쓰는 Write Authorization 하나, `record_run` consume/block, artifact/evidence ref 하나, structured status/blocker 출력 하나입니다. Projection polish, detailed template, full Evidence Manifest behavior, conformance runner behavior, broad fixture catalog는 v0.1 requirement가 아닙니다. Exact future fixture queue, body field, seed rule, assertion mode, stable event, artifact/projection assertion, primary-error expectation은 [커널 스모크(Kernel Smoke) Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue)와 [Conformance Fixture Format](../reference/conformance-fixtures.md#conformance-fixture-format)이 담당합니다.

Suite stage, authoring order, docs-maintenance result를 표현하기 위해 fixture body에 field를 추가하지 않습니다.

## 참고할 Reference 문서

- [커널 참조](../reference/kernel.md): Task, Change Unit, Decision Packet, gate, `prepare_write`, Write Authorization, `record_run` semantics, `close_task`.
- [런타임 아키텍처 참조](../reference/runtime-architecture.md): 세 공간, Core process model, transaction flow, artifact store, projection/reconcile, guarantee level, failure handling.
- [MCP API와 스키마](../reference/mcp-api-and-schemas.md): public resource, tool envelope, request/response schema, error taxonomy, 아티팩트 참조, `ProjectionKind`.
- [Storage와 DDL](../reference/storage-and-ddl.md): runtime layout, staged schema profile, migration, lock, artifact와 later-profile baseline, projection job, validator-run candidate를 다룹니다.
- [운영과 Conformance 참조](../reference/operations-and-conformance.md): operator 의미와 conformance 단계화.
- [Conformance Fixtures 참조](../reference/conformance-fixtures.md): 핵심 적합성 모델, fixture 형식, 실행, assertion rule, 축소된 Kernel Smoke queue.
- [향후 Fixture Catalog](../reference/future-fixture-catalog.md): v0.1 requirement가 아닌 detailed later scenario candidate.
