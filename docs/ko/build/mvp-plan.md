# Build: MVP 계획

## 이 문서로 할 수 있는 일

이 문서는 MVP 범위를 구현 가능한 staged delivery 계획으로 바꿉니다. 첫 실행 가능한 커널 조각과 첫 사용자 대상 MVP를 분리해, "MVP"라는 이름을 단순히 권한 루프가 존재하는 단계가 아니라 사용자가 하네스의 가치를 경험할 수 있는 단계에만 사용합니다.

이 문서는 구현 계획 문서입니다. 문서 세트가 구현 계획에 사용할 수 있다고 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture, fixture 파일, 런타임 데이터를 만들라는 뜻이 아닙니다. Conformance fixture 문서는 향후 적합성 검증 계획이며, 현재 문서 전용 저장소에는 runnable Harness Server conformance test가 없습니다. 첫 실행 목표는 코어 권한 조각(v0.1 Core Authority Slice)이며, 커널 스모크(Kernel Smoke)는 이 조각을 위한 좁은 conformance 작성 프로파일입니다. 첫 제품 MVP 목표는 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)입니다. v0.3과 v0.4는 assurance, stewardship, operations, handoff 동작을 단계적으로 단단하게 만듭니다. v1+ Expansion은 owner 문서가 승격하고 증명하기 전까지 roadmap 범위에 둡니다.

문서 승인 이후 무엇을 만들지 계획할 때 이 문서를 사용합니다. 정확한 contract는 Reference 문서를 사용합니다.

## 읽는 경우

- 첫 실행 가능한 커널 증명과 첫 사용자 대상 제품 MVP를 분리해야 할 때.
- 첫 implementation batch를 키우지 않으면서 단계별 전달 범위를 검토해야 할 때.
- 구현 순서를 storage, schema, fixture, template detail과 분리해서 보고 싶을 때.

## 먼저 읽을 것

[구현 개요](implementation-overview.md)의 [문서 승인 상태](implementation-overview.md#문서-승인-상태), [첫 실행 가능한 조각](first-runnable-slice.md), [Runtime Walkthrough](runtime-walkthrough.md)를 먼저 읽습니다. 정확한 API contract는 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)를 사용합니다. Storage detail과 DDL은 [Storage와 DDL](../reference/storage-and-ddl.md)을 사용합니다. Design-quality gate와 validator behavior는 [Design Quality Policies](../reference/design-quality-policies.md)를 사용합니다. Conformance fixture semantics는 [Conformance Fixtures 참조](../reference/conformance-fixtures.md)를 사용합니다. Operator procedure와 conformance run overview는 [운영과 Conformance](../reference/operations-and-conformance.md)를 사용합니다. v1+ Expansion 후보와 승격 기준은 [로드맵](../roadmap.md)을 사용합니다.

## 핵심 생각

하네스의 가치는 단지 write authority loop가 있다는 데 있지 않습니다. 하네스는 범위, 사용자 소유 판단, 근거, 닫기 준비 상태, 잔여 위험을 로컬 권한 기록에 보존해야 합니다. 그래서 초기 전달에는 두 단계가 있습니다.

- 코어 권한 조각(v0.1 Core Authority Slice)은 가장 작은 내부 커널 루프를 증명합니다.
- 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)는 사용자가 하네스가 work를 clarify, budget, block, accept, risk-explain하는 방식을 경험하는 첫 MVP입니다.

첫 조각은 의도적으로 좁게 유지합니다. 로컬 project 하나, Task 하나, 기본 scope 하나, 쓰기 권한 경로 하나, 기록된 Run 하나, 근거 링크 하나, 구조화된 막힘/상태 응답 하나를 증명합니다. 이것은 MVP가 아닙니다. 일반적인 work를 scope, judgment, 근거, close-readiness, 잔여 위험 언어로 바꾸고 approval, 작업 수락, 잔여 위험 수용을 혼동하지 않게 만드는 단계가 MVP입니다.

Projection template polish, detailed report, dashboard 또는 hosted workflow UI, index, broad connector ecosystem 또는 marketplace, team workflow, surface-specific connector automation, metric, parallel orchestration, broad automation은 authority record와 user-facing value path가 존재한 뒤 유용해질 수 있습니다. 첫 조각의 요구사항은 아닙니다.

초기 output model은 의도적으로 작게 둡니다.

- v0.1은 Core state에서 오는 현재 상태/다음 행동 읽기 전용 출력과 구조화된 막힘이 필요합니다.
- v0.2는 사용자 읽기용 현재 작업 상태, 판단 요청, 근거 요약, 닫기 준비 상태 / blocker 요약이 필요합니다.
- Journey Card, Journey Spine, Run Summary, TDD Trace, Module Map, Interface Contract, Export, detailed Evidence Manifest, detailed Eval output은 owner profile이 명시적으로 승격하지 않는 한 optional, diagnostic, later-profile scope로 남습니다.

## 단계별 전달 계획

| 단계 | 전달 목표 | 증명하는 것 | 아직 증명하지 않는 것 |
|---|---|---|---|
| v0.1 | 코어 권한 조각(Core Authority Slice) | 로컬 project 하나, Task 하나, 기본 scope 하나, 쓰기 권한 경로 하나, 기록된 Run 하나, 근거 링크 하나, 구조화된 막힘/상태 응답 하나로 구성된 첫 실행 가능한 내부 kernel loop. | 사용자 대상 MVP 가치, full intake/discovery, full Decision Packet 품질, residual-risk semantics, 수동 QA, 분리 검증, projection completeness, operations readiness. |
| v0.2 | 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP) | 사용자가 하네스가 scope, user-owned judgment, 근거, close readiness, 작업 수락, 잔여 위험 표시를 로컬 권한 기록에 보존한다는 것을 경험합니다. | Full agency hardening, 분리 검증 독립성, 수동 QA matrix, stewardship policy suite, feedback-loop policy, export/recover, release handoff. |
| v0.3 | 보증과 스튜어드십 팩(Assurance & Stewardship Pack) | MVP path를 assurance, 수동 QA, 검증, stewardship, design-quality, context-hygiene, TDD, feedback-loop profile로 단단하게 만듭니다. | Operator recovery/export completeness, release handoff, broad operations coverage, roadmap automation. |
| v0.4 | 운영과 인계 팩(Operations & Handoff Pack) | 같은 Core model로 doctor/readiness, recover/export, artifact integrity, release handoff, 더 넓은 conformance coverage를 지원합니다. | Dashboard, hosted workflow UI, broad connectors, Browser QA Capture automation, Cross-Surface Verification automation, Context Index, team workflow, orchestration. |

```mermaid
flowchart LR
  Core["v0.1<br/>코어 권한 조각<br/>첫 커널 실행"] --> MVP["v0.2<br/>사용자 대상 MVP<br/>첫 사용자 가치"]
  MVP --> Assurance["v0.3<br/>보증과 스튜어드십<br/>자율성 보존과 보안 정책 강화"]
  Assurance --> Ops["v0.4<br/>운영과 인계<br/>운영 준비"]
  Ops -. roadmap boundary .-> Expansion["v1+<br/>확장 후보"]
```

커널 스모크(Kernel Smoke)는 코어 권한 조각(v0.1 Core Authority Slice)을 위한 좁은 conformance authoring profile로 남습니다. 이 profile 이름은 v0.1이 제품 MVP라는 뜻이 아니라 내부 kernel path를 증명한다는 뜻입니다.

Conformance fixture 검증 프로파일은 같은 stage name을 따릅니다. 첫 실행 가능한 커널 조각 fixture는 v0.1 Core Authority Slice, 사용자 대상 MVP fixture는 v0.2 User-Facing Harness MVP, agency-hardened fixture는 v0.3 Assurance & Stewardship Pack, operations/future fixture는 v0.4 Operations & Handoff Pack과 승격된 v1+ Expansion candidate에 대응합니다.

### 단계별 전달 이후의 경계: v1+ Expansion

v1+ Expansion은 roadmap 범위이며 Build가 소유하는 staged delivery phase가 아닙니다. Dashboard, hosted workflow UI, Browser QA Capture automation, Cross-Surface Verification automation, Context Index, broader connectors, metrics, team workflow, orchestration 같은 후보는 owner 문서가 future item을 명시적으로 승격하고 증명하기 전까지 v0.1부터 v0.4 밖에 둡니다.

## 코어 권한 조각(v0.1 Core Authority Slice)

v0.1은 내부 구현 단계입니다. 하네스가 chat memory나 generated Markdown이 아니라 로컬 권한 기록임을 보여 주는 가장 작은 coherent loop만 증명해야 합니다.

v0.1은 다음을 증명해야 합니다.

- project registration과 reference surface 하나
- 현재 상태와 `task_events`를 가진 Task 하나
- intended change를 위한 기본 scope 하나(Reference 계약상 필요한 경우 Change Unit 소유자 형태로 표현된다)
- `prepare_write` allow/block path 하나
- 지속적이며 한 번만 쓸 수 있는 Write Authorization 하나
- 그 authorization을 consume하는 `record_run` 하나
- Core/API contract가 소유하는 registered `ArtifactRef` 또는 equivalent evidence link 하나
- selected claim에 대한 support 또는 insufficiency를 보고할 수 있는 minimal evidence relation 또는 Evidence Manifest state record 하나. 단 detailed `EVIDENCE-MANIFEST` projection은 요구하지 않습니다.
- 현재 Core state에서 오는 `status`/`next` 읽기 전용 응답 하나
- 누락된 근거, missing scope, 또는 seeded 필요한 사용자 판단을 위한 구조화된 막힘/상태 응답 하나

v0.1은 full natural-language intake, full Discovery, full Decision Packet quality, product/UX judgment와 architecture judgment의 presentation, 잔여 위험 표시, 작업 수락, 잔여 위험 수용, 수동 QA, 분리 검증, stewardship, feedback-loop policy, export/recover, release handoff, projection/template completeness를 증명하면 안 됩니다. 이것들은 이후 단계의 범위입니다.

v0.1 Kernel Smoke fixture candidate는 Core state, events, artifact/evidence refs, 관련되는 경우 freshness facts, structured blocker를 통해 minimal authority loop를 검증해야 합니다. Projection polish, detailed template, renderer output은 first-slice conformance truth가 아닙니다.

이 시점에 implementer 또는 operator는 Core가 상태를 소유하고, scoped write가 허용되거나 차단되며, authorization 하나가 한 번 소비되고, 근거가 기록된 Run에 연결되며, 읽기 동작이 상태를 바꾸지 않고, 닫기/상태 출력이 구조화된 막힘을 반환할 수 있음을 관찰할 수 있습니다.

### 계약 필드 단계 구분

Reference schema에는 관련 capability가 범위에 들어올 때만 필요한 field도 포함됩니다. Build는 field requiredness를 다시 정의하지 않습니다. 어떤 capability가 어느 stage에 들어오는지만 말합니다. Field는 owner contract와 active stage를 함께 보고 읽습니다.

| Stage | Build 읽기 규칙 | 적용할 owner contract |
|---|---|---|
| v0.1 Core Authority Slice | 좁은 authority loop를 증명하는 데 필요한 owner-defined field만 사용합니다. Smoke path가 Decision Packet을 사용한다면 required Decision Packet field는 그대로 적용됩니다. 다만 full user-facing Decision Packet quality는 이후 범위입니다. | [커널 참조](../reference/kernel.md), [MCP API와 스키마](../reference/mcp-api-and-schemas.md), [Storage와 DDL](../reference/storage-and-ddl.md), [Conformance Fixtures 참조](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue). |
| v0.2 User-Facing Harness MVP | 사용자가 judgment context, evidence, close readiness, 작업 수락 분리, 잔여 위험 표시를 이해하는 데 필요한 field와 display summary를 추가합니다. | [MCP API와 스키마](../reference/mcp-api-and-schemas.md), [커널 참조](../reference/kernel.md), [문서 Projection 참조](../reference/document-projection.md), [Template 참조](../reference/templates/README.md). |
| v0.3/v0.4 hardened reference | Assurance, QA, stewardship, projection/reconcile, operations, export/recover, artifact-integrity, release-handoff profile은 owner 문서가 정의한 곳에서만 추가합니다. | [설계 품질 정책](../reference/design-quality-policies.md), [운영과 Conformance](../reference/operations-and-conformance.md), [Conformance Fixtures 참조](../reference/conformance-fixtures.md), [Storage와 DDL](../reference/storage-and-ddl.md). |

따라서 API schema에서 required라는 말은 해당 tool call, record, profile이 구현되거나 사용될 때 required라는 뜻입니다. 그 자체로 future-profile field가 가장 작은 runnable slice의 일부가 되지는 않습니다.

### 서버 코딩 전 필요한 구현 결정

Reference contract에는 schema ownership 또는 stage-boundary decision이 흩어진 TODO로 남아 있지 않습니다. Implementation planning 중 새 문제가 발견되면, server code나 DDL을 바꾸기 전에 owner doc, affected field 또는 behavior, stage impact, 필요한 결정을 이곳에 기록합니다.

### 코어 권한 조각 흐름

```mermaid
flowchart LR
  Register["프로젝트 등록"] --> Task["Task 생성"]
  Task --> Scope["범위 설정"]
  Scope --> Check["쓰기 확인"]
  Check -->|허용| Authorization["쓰기 허가"]
  Authorization --> Run["Run 기록"]
  Run --> Evidence["근거 연결"]
  Check -->|막힘| Blocker["구조화된 막힘"]
  Evidence --> Status["상태와 다음 행동"]
  Blocker --> Status
  Status --> Close["닫기 상태 막힘"]
```

정확한 state와 close behavior는 [커널 참조](../reference/kernel.md)가, public tool shape는 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)가, projection rule은 [문서 Projection 참조](../reference/document-projection.md)가, fixture semantics는 [Conformance Fixtures 참조](../reference/conformance-fixtures.md#conformance-fixture-format)가 담당합니다. 이 flow는 pack gate나 fixture body requirement를 추가하지 않습니다.

실제 fixture 작성 순서는 [커널 스모크(Kernel Smoke) Authoring Queue](../reference/conformance-fixtures.md#kernel-smoke-authoring-queue)를 사용합니다. 이 queue는 v0.1 fixture candidate를 이 내부 조각에 매핑하지만 executable fixture file이 이미 존재한다고 암시하지 않습니다.

## 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)

v0.2는 첫 제품 MVP입니다. 더 긴 component checklist가 아니라 사용자가 경험하는 가치로 정의합니다.

MVP는 다음을 보여야 합니다.

- 평범한 사용자 요청이 scope, user-owned judgment, 근거, close-readiness language로 정리된다
- product/UX judgment와 material technical architecture judgment가 분리되어 제시될 수 있다
- 작은 변경과 tracked work가 서로 다른 procedural budget을 가지되, small-change label이 authority를 우회하지 않는다
- status와 next-action output이 현재 scope, 누락된 decisions, 근거 상태, close blockers, 안전한 다음 행동을 설명한다
- required 근거가 없거나 required user judgment가 missing이면 close가 block된다
- 작업 수락과 close 전에 잔여 위험을 표시할 수 있다
- 사용자의 작업 수락이 sensitive-action Approval, 잔여 위험 수용과 구분된다
- readable summary 또는 card가 현재 작업 상태, 판단 요청, 근거 요약, 닫기 준비 상태/blocker를 보여 주지만, template polish가 source of truth가 되지는 않는다
- prose나 renderer output만이 아니라 Core state, events, artifacts, projection/freshness facts, structured errors로 conformance를 증명할 수 있다

v0.2는 특정 user-facing MVP scenario가 최소 display 또는 blocker hook을 요구하지 않는 한 분리 검증, full 수동 QA 정책 매트릭스, stewardship validators, feedback-loop policy, export/recover, release handoff, Journey Card/Spine polish, Run Summary, TDD Trace, Module Map, Interface Contract, detailed Evidence Manifest, detailed Eval, Export projection을 staged profile로 남겨 둡니다. Browser QA Capture, Cross-Surface Verification automation, dashboard, broad connectors, Context Index, metrics, team workflow, orchestration은 MVP 밖에 둡니다.

v0.2를 통과했다는 것은 사용자가 하네스가 authorization wrapper 이상임을 볼 수 있다는 뜻입니다. Work의 scope, decision, 근거, 작업 수락, risk boundary가 로컬에서 inspectable하게 유지됩니다.

## 보증과 스튜어드십 팩(v0.3 Assurance & Stewardship Pack)

v0.3은 MVP path를 강화하여 로컬 reference path가 assurance, policy, stewardship을 정직한 경계 안에서 route할 수 있게 합니다.

중점:

- full Decision Packet quality와 user-judgment routing
- sensitive-action Approval, Decision Packet, Write Authorization, 작업 수락, 잔여 위험 수용 separation
- same-session verification guard behavior를 포함한 분리 검증 독립성
- 수동 QA 정책 매트릭스, 수동 QA 차단 조건, 유효한 QA 면제 판단
- residual-risk accepted close full semantics
- stewardship validators와 codebase stewardship coverage
- policy가 요구하는 TDD trace behavior
- policy가 요구하는 feedback-loop policy
- context-hygiene validators와 현재 상태/오래된 context 경계
- Core state, events, artifacts, projection/freshness facts, errors를 통해 judgment, QA, verification, residual-risk, acceptance separation을 증명하는 agency-hardened conformance fixtures

이 pack을 통과하면 user-facing MVP path가 agency-preserving하고 policy-aware하다는 뜻입니다. v1+ Expansion automation을 staged delivery로 승격하지는 않습니다.

## 운영과 인계 팩(v0.4 Operations & Handoff Pack)

v0.4는 같은 Core state model 위에서 로컬 운영 증명을 완성합니다.

중점:

- runtime home, project state, artifact store, reference surface, MCP availability, projections, reconcile, validators/checks, agency/stewardship/context에 대한 doctor/readiness categories
- interrupted 또는 drifted operational state에 대한 recover handling
- state snapshots, report projection snapshots, artifact refs, redaction status, omitted-secret notes, retained/expired/unavailable artifact status에 대한 export behavior
- artifact integrity checks
- owner 문서가 정의하는 release handoff report/export profile
- connect, doctor, serve MCP, projection refresh, reconcile, recover, export, artifacts check, conformance run에 대한 operator smoke
- export/recover, artifact integrity, release handoff, operator readiness, 그리고 owner 문서가 정의하고 증명한 higher guarantee level에 대한 operations/future fixture coverage
- 별도로 증명하고 승격하기 전까지 roadmap item을 v1+ Expansion에 두는 later-boundary checks

Operator command를 위한 두 번째 state model을 만들면 안 됩니다. Operator는 같은 Core state model 위에서 diagnose, repair, export, fixture run을 수행합니다.

Docs-maintenance는 별도의 읽기 전용 문서 profile로 남습니다. Documentation drift를 보고할 수 있지만 코어 권한 조각(v0.1 Core Authority Slice)도, 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)도, agency-hardened 또는 operations runtime conformance도, 구현 준비 상태 신호도 아닙니다.

## Roadmap 범위의 v1+ Expansion 후보

아래 항목은 future plan이 owner 문서를 통해 capability profile, exact contracts, redaction/secret/PII policy, runtime surface capture 시 artifact retention과 test-environment rule, fixture 또는 conformance target, fallback behavior, no projection-as-canonical dependency를 갖춰 승격하기 전까지 staged delivery 밖에 둡니다.

| 후보 | 단계 경계 |
|---|---|
| Dashboard, hosted workflow UI, artifact dashboard, rich card expansion | State를 표시할 수는 있지만 authority, implementation readiness, close readiness, 작업 수락, 잔여 위험 수용이 되면 안 됩니다. |
| Broad connector marketplace 또는 surface ecosystem | 나중에 surface를 확장할 수 있지만 reference surface proof를 대체하거나 MCP exposure를 기본적으로 넓히면 안 됩니다. |
| Browser QA Capture automation | 승격 뒤 수동 QA를 보조할 수 있지만 human QA judgment, 작업 수락, 분리 검증을 대체하면 안 됩니다. |
| Cross-Surface Verification automation | 승격 뒤 evaluator routing을 자동화할 수 있지만 Core-owned return record 없이 Eval 또는 assurance를 충족하면 안 됩니다. |
| Preventive guard expansion, native hooks, Advanced Sidecar Watcher | Proven pre-tool blocking 또는 observation path가 있을 때 surface를 강화할 수 있지만 label만으로 주장하면 안 됩니다. |
| Context Index, Local Derived Metrics, long-term metrics | Read-only retrieval 또는 diagnostics를 제공할 수 있지만 write를 authorize하거나, gate를 충족하거나, projection을 refresh하거나, Task를 close하면 안 됩니다. |
| Team workflow, permissions, orchestration, parallel lanes | Future work를 조율할 수 있지만 staged delivery나 single-project local authority의 필수 요소가 되면 안 됩니다. |
| Deployment, canary, rollback, merge, production monitoring | Future integration work가 될 수 있습니다. Release handoff는 owner 문서가 더 많은 권한을 승격하기 전까지 report/export boundary로 남습니다. |

구현 중 향후 feature가 유용해 보이더라도 owner 문서가 권한 경로를 정의하고 증명하기 전까지는 읽기 전용 표시, metadata, artifact 후보, fixture candidate로 유지합니다.

## 단계별 종료 기준

문서 승인 이후 future runtime planning을 위한 implementation-readable checklist로 사용합니다. 이들은 staged exit을 다시 말할 뿐이며 schema, fixture, DDL, new runtime requirement를 추가하지 않습니다. [문서 승인 상태](implementation-overview.md#문서-승인-상태)가 first runtime-batch planning을 막고 있는 동안 implementation을 authorize하지 않습니다.

### 코어 권한 조각(v0.1 Core Authority Slice) exit checklist

- 프로젝트 하나와 reference surface 하나가 등록된다.
- Task 하나를 만들고, 읽고, 최소한으로 advance하고, `task_events`에 나타낼 수 있다.
- Scope record 하나가 intended change boundary를 이름 붙인다.
- Compatible scope 없는 product write는 block된다.
- Out-of-scope intended write는 block된다.
- 허용된 `prepare_write`는 지속적이며 한 번만 쓸 수 있는 Write Authorization을 만든다.
- Compatible `record_run`은 authorization을 한 번 consume한다.
- 두 번째 distinct product-write Run은 consumed authorization을 재사용할 수 없다.
- Artifact 또는 evidence ref 하나가 등록되어 Run 또는 evidence relation에 연결된다.
- Minimal 근거 상태가 selected claim에 대해 support, partial support, insufficiency를 보고할 수 있다.
- `status`와 `next`는 상태를 변경하지 않고 현재 상태를 반환한다.
- Structured blocker/status response가 missing scope, evidence, 또는 required seeded user judgment를 보고한다.

### 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP) exit checklist

- 평범한 사용자 언어가 Harness vocabulary를 요구하지 않고 tracked work를 시작하거나 resume할 수 있다.
- User-facing path가 scope, non-goals, acceptance criteria, evidence expectations, close readiness, judgment boundaries를 clarify한다.
- Product/UX judgment와 material technical architecture judgment를 분리해 제시할 수 있다.
- Small direct changes와 tracked work가 write authority, evidence, required user judgment를 우회하지 않으면서 서로 다른 procedural budget을 사용한다.
- Status/next output이 현재 scope, missing decisions, 근거 상태, 잔여 위험 표시, close blockers, 안전한 다음 행동을 설명한다.
- Required 근거가 없으면 close가 block된다.
- Required user judgment가 missing 또는 unresolved이면 close가 block된다.
- Known close-relevant risk가 있으면 작업 수락 또는 close 전에 잔여 위험이 보인다.
- 사용자의 작업 수락이 sensitive-action Approval과 잔여 위험 수용과 별도로 기록되거나 표현된다.
- 사용자에게 보이는 readable summary 또는 card는 Core record에서 파생되며, template polish를 기준 권한으로 만들지 않고 MVP path에 충분하다.

### 보증과 스튜어드십 팩(v0.3 Assurance & Stewardship Pack) exit checklist

- Decision Packet quality와 user-judgment routing이 fixture로 증명된다.
- Sensitive-action Approval이 Decision Packet, Write Authorization, 수동 QA, verification, 작업 수락, 잔여 위험 수용을 대체하지 않는다.
- 분리 검증 독립성와 same-session verification guard behavior가 fixture로 증명된다.
- Policy가 요구하는 곳에서 수동 QA 정책 매트릭스와 QA blocker가 fixture로 증명된다.
- Risk-accepted close는 owner semantics에 따라 accepted Residual Risk refs를 인용한다.
- Policy가 요구하는 곳에서 stewardship validators, feedback-loop policy, TDD trace behavior, context-hygiene behavior가 cover된다.
- Agency conformance가 Journey visibility, user judgment, Autonomy Boundary respect, distinct user judgments, residual-risk handling을 증명한다.

### 운영과 인계 팩(v0.4 Operations & Handoff Pack) exit checklist

- Doctor/readiness가 runtime home, project state, artifact store, reference surface, MCP availability, projections, reconcile, validators/checks, agency/stewardship/context category를 보고한다.
- Recover는 recovery artifact를 successful completion proof로 취급하지 않으면서 interrupted 또는 drifted 운영 상태를 처리한다.
- Export는 state snapshot, report projection snapshot, artifact refs, redaction status, omitted-secret notes, retained/expired/unavailable artifact status를 포함한다.
- Artifact integrity check는 missing 또는 mismatched artifact를 기존 diagnostics로 보고한다.
- Release handoff report/export behavior는 deployment, merge, rollback, production authority를 가져오지 않고 owner profile을 따른다.
- Operations/future fixture coverage가 export/recover, artifact integrity, release handoff, operator readiness, 승격된 higher guarantee level을 prose가 아니라 exact-shape fixture로 증명한다.
- Later-boundary check는 owner 문서가 승격하고 증명하기 전까지 v1+ Expansion item을 staged delivery 밖에 둔다.

## 단계별 관찰 가능 항목

| 단계 | 사용자 또는 operator가 볼 수 있는 것 |
|---|---|
| 코어 권한 조각(v0.1 Core Authority Slice) | Implementer/operator는 로컬 Task 하나가 scope, `prepare_write`, Write Authorization, `record_run`, artifact/evidence link, `status`/`next` 읽기 전용 응답, 구조화된 막힘을 통과하는 것을 볼 수 있습니다. |
| 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP) | 사용자는 ordinary work가 scope, judgment, 근거, close readiness, 작업 수락, 잔여 위험 언어로 정리되고 근거 또는 user judgment가 없으면 close가 block되는 것을 볼 수 있습니다. |
| 보증과 스튜어드십 팩(v0.3 Assurance & Stewardship Pack) | Local path가 verification, 수동 QA, stewardship, TDD, feedback, context hygiene, 작업 수락, 잔여 위험 수용, close behavior를 Core record와 fixture로 설명합니다. |
| 운영과 인계 팩(v0.4 Operations & Handoff Pack) | Operator는 같은 Core state 위에서 diagnose, recover, reconcile, export, artifact check, conformance run, release handoff 준비를 수행할 수 있습니다. |

단계별 전달 이후에는 promoted roadmap item이 owner 문서가 exact contract와 fixture coverage를 정의한 뒤에만 authority loop를 읽고, 표시하고, 감싸고, 확장할 수 있습니다.
