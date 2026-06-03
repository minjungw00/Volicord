# 문서 작성 가이드

## 이 문서로 할 수 있는 일

Harness 문서를 새로 쓰거나, 나누거나, 이름을 바꾸거나, 리뷰할 때 이 가이드를 사용합니다.

목표는 현재 문서가 독자에게 읽기 쉽고, 세부 계약의 위치가 분명하며, 영어와 한국어 문서가 같은 의미를 유지하도록 돕는 것입니다.

이 문서는 Maintain 문서입니다. 문서 유지보수만 다룹니다. 첫 실행 목표는 코어 권한 스모크(v0.1 Core Authority Smoke)이며, 커널 스모크(Kernel Smoke)는 좁은 future smoke-check 작성 label입니다. 첫 사용자 가치 목표는 첫 사용자 가치 조각(v0.2 First User-Value Slice)입니다. 에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)은 agency assurance, operations, handoff behavior를 단단하게 만드는 단계이며, v1+ Expansion은 owner 문서가 승격하고 증명하기 전까지 roadmap 범위에 둡니다.

## 이런 때 읽기

- 문서를 추가, 분리, 이름 변경, review할 때.
- 어떤 문서가 strict contract를 소유하는지 판단해야 할 때.
- 영어/한국어 의미 일치, link, TODO hygiene, duplicate owner text를 확인할 때.

## 먼저 읽기

정확한 runtime contract는 아래에 연결된 Reference owner 문서를 사용합니다. 한국어 표현 규칙은 [번역 가이드](translation-guide.md)를 사용합니다.

## 핵심 생각

각 문서는 독자에게 유용해야 하며 exact contract는 owner Reference 문서에 머물러야 합니다. 문서는 하네스를 이해하고 구현하기 위한 원천 자료이지, 하네스가 관리하는 런타임 객체가 아닙니다.

## 하나의 계약에는 하나의 owner

규범 계약마다 owner 문서가 하나만 있어야 합니다. Exact field, enum value, DDL, schema, algorithm, state transition, gate rule, fixture body shape, template body, storage rule, security guarantee, error precedence, official definition은 그 owner 문서에서만 정의합니다.

다른 문서군은 독자에게 필요한 결과를 짧게 설명하고 owner로 연결할 수 있습니다. 하지만 두 번째 정의를 만들면 안 됩니다. Local 설명에 full table, schema block, DDL block, transition matrix, fixture mini-language, gate matrix, validator table, algorithm이 필요하다면 그 내용은 owner Reference 문서에 둡니다.

문서군 경계:

| 문서군 | 역할 | 경계 |
|---|---|---|
| Learn | Harness가 왜 필요한지, 개념이 무슨 뜻인지 설명합니다. | Strict schema, gate, 구현 순서를 정의하지 않습니다. |
| Use | 사용자와 agent가 어떻게 상호작용하는지 설명합니다. | 사용자가 신뢰하거나 blocker를 이해하는 데 필요할 때만 낮은 수준의 계약 detail을 씁니다. |
| Build | 구현 순서와 단계별 전달 계획을 설명합니다. | Stage goal, 순서, exit criteria를 유지하고, exact schema, gate, DDL, API, storage, fixture detail은 Reference로 연결합니다. |
| Reference | Exact contract, schema, algorithm, security model, storage model을 정의합니다. | Contract를 이해할 만큼의 설명은 두되, tutorial이나 reader journey로 만들지 않습니다. |
| Maintain | 문서 작성과 review 규칙을 정의합니다. | Docs work만 다룹니다. Runtime behavior나 conformance pass/fail을 정의하지 않습니다. |

## Reference 계약 owner 지도

엄격한 규칙을 추가하기 전에 이 지도를 사용합니다. 다른 문서에 같은 규칙이 필요하면 독자에게 필요한 결과만 짧게 요약하고, 계약을 복사하지 말고 owner로 연결합니다.

| 계약 영역 | Owner 문서 | Owner 경계 |
|---|---|---|
| Kernel | [커널 참조](../reference/kernel.md) | Invariant, 상태에 영향을 주는 entity 관계 의미, lifecycle과 상태 전이, gate, `prepare_write`, Write Authorization, `record_run`, close semantics, waiver, 대체 불가능한 경계. |
| MCP API | [MCP API와 스키마](../reference/mcp-api-and-schemas.md) | Public MCP resource와 tool, common envelope, request/response schema, shared ref, public error, idempotency/replay, state conflict behavior, `ValidatorResult`, API `ArtifactRef`. |
| Storage | [Storage와 DDL](../reference/storage-and-ddl.md) | Runtime home layout, persisted state, SQLite DDL profile, storage-owned JSON `TEXT`, enum hardening, migration, lock, artifact storage, baseline capture, projection job table, validator-run storage. |
| Projection | [문서 Projection 참조](../reference/document-projection.md)와 [Template 참조](../reference/templates/README.md) | Derived view rule, output tier, managed block, human-editable section, artifact-ref rendering, projection freshness/failure behavior, 전체 rendered template body. |
| Security | [보안 위협 모델 참조](../reference/security-threat-model.md) | Threat model, asset, trust boundary, threat/control category, high-risk control expectation, local access security posture, guarantee-level 의미와 honest-display rule. |
| Conformance | [Conformance Fixtures 참조](../reference/conformance-fixtures.md)와 [향후 Fixture Catalog](../reference/future-fixture-catalog.md) | Conformance Fixtures는 핵심 적합성 모델, 정확한 fixture body, runner 동작, assertion semantics, fixture profile, suite metadata boundary, 현재 단계 상태, 축소된 Kernel Smoke 작성 순서를 담당합니다. 향후 Fixture Catalog는 향후 상세 scenario 후보, 향후 fixture example, 단계별 fixture coverage map, fixture suite family summary, catalog-only future candidate를 담당합니다. |
| Operations | [운영과 Conformance 참조](../reference/operations-and-conformance.md) | Operator behavior, staged operator surface, diagnostic, `connect`, `doctor`, `serve mcp`, projection refresh, reconcile, recover, export, artifact check, conformance run entrypoint, docs-maintenance profile reporting boundary. |
| Agent Integration | [Agent 통합 참조](../reference/agent-integration.md)와 [Surface Cookbook](../reference/surface-cookbook.md) | Connector capability profile, generated manifest, context push/pull profile, fallback semantics, Role Lens, reference-surface behavior, connector conformance overview, surface-specific recipe. |
| Glossary | [용어집 참조](../reference/glossary.md) | Public/internal terminology definition, capitalization, official term wording, record-name orientation, owner routing. |
| Runtime Architecture | [런타임 아키텍처 참조](../reference/runtime-architecture.md) | 세 공간, Core process placement, Core-only canonical mutation authority, transaction ordering, artifact/projection/reconcile placement, architecture-level failure and recovery overview. |
| Design Quality | [설계 품질 정책](../reference/design-quality-policies.md) | Policy contract, policy-to-validator mapping, stable validator ID, severity composition, policy waiver semantics, evidence expectation, design-quality gate/close impact. |

## 현재 재설계 범위

### 현재 검토 기준

이 저장소는 문서 검토와 재설계 단계입니다. 세 상태를 분리해서 유지합니다.

- 문서 검토 상태: 현재 문서 세트는 재설계 이후 검토 상태이며 유지보수자 검토를 위한 문서 수락 후보입니다.
- 구현 계획 준비 상태: 아직 수락되지 않았습니다. 첫 런타임 배치 계획 전에 유지보수자가 구현 준비 조건을 명시적으로 확인해야 합니다.
- 런타임 구현 상태: 시작하지 않았습니다. 이 저장소는 아직 문서 전용입니다. 향후 역할은 하네스 서버 소스 저장소이지만, 서버/런타임 구현은 문서 수락과 별도의 구현 계획 준비 결정 이후에만 시작할 수 있습니다.

이 저장소 단계에서는 서버/런타임 구현 결정을 코드 작성용으로 공식 수락하지 않았습니다. 결정 기록이 비어 있다는 말은 현재 기록된 내용이 없다는 뜻일 뿐, 남은 설계 쟁점이 없다는 증거가 아닙니다.

유지보수자 인계 상태가 명시적으로 정의하기 전까지 현재 문서를 완전히 수락되었거나, 구현 완료되었거나, 구현 준비가 끝났거나, 서버 코딩을 시작해도 되는 상태로 설명하지 않습니다.

문서 편집은 문서 원본을 바꿀 수 있지만, 하네스 서버/런타임 구현을 시작하거나 구현 계획을 허가하지 않습니다.

상세 단계와 상태 경고는 root README, 언어별 README, Build 인계 문서, 이 Maintain 지침에 둡니다. Learn/Use 문서는 그 담당 문서로 연결할 수 있지만, 첫 부분은 사용자가 무엇을 요청할 수 있는지, 에이전트가 무엇을 구체화해야 하는지, 하네스가 무엇을 보존하는지, 사용자가 무엇을 보게 되는지부터 시작해야 합니다.

이번 재설계에서는 용어, MVP 단계, 스키마(schema) 구조, 투영(projection) 구조, 보안 표현, 문서 구성이 바뀔 수 있습니다. 정리된 제품 명제나 구현 가능성과 충돌하는 기존 문구는 연속성만으로 보존하지 않습니다.

이 저장소에서 문서를 편집할 때는 하네스 런타임 절차가 필요하지 않습니다. 문서 편집을 위해 runtime state, `task_events`, Write Authorization, Evidence Manifest, 수동 QA record, Acceptance record, Residual Risk record, generated projection, 운영 파일, executable fixture, fixture 파일, 런타임 기록, 제품 저장소 예시를 만들지 않습니다. 이런 용어는 향후 Harness 동작을 설명할 때만 문서화할 수 있습니다.

문서 파일은 Harness를 이해하고 구현하기 위한 원천 자료입니다. 향후 Harness Server가 명시적으로 projection으로 생성하지 않는 한 Harness projection이 아닙니다. 문서 페이지가 자신이 설명하는 런타임 생명주기를 따르게 만들지 않습니다. 생명주기는 설명하고, owner contract로 연결하며, 편집 점검은 편집 점검으로 유지합니다.

### 재설계 편집 계약

재설계 중에는 기존 문구 보존보다 명확성, 구현 가능성, 제품 명제를 우선합니다.

- 이미 있다는 이유만으로 문장을 보존하지 않습니다. Harness가 넓은 workflow engine, ALM system, evaluation harness, QA automation platform, report generator, generic MCP wrapper처럼 보이게 하는 내용은 다시 쓰거나, 옮기거나, 줄이거나, 삭제합니다.
- 보존할 것은 핵심 Harness 원칙과 가치입니다. Scope, 사용자 소유 판단, 근거 참조, 닫기 준비 상태, 작업 수락, 잔여 위험을 대화 밖의 Core-owned 로컬 권한 기록에 둔다는 점을 지킵니다.
- Future, profile-specific, diagnostic, roadmap 내용은 단계화된 후보로 읽혀야 합니다. 현재 MVP requirement처럼 보이거나 구현이 이미 있다는 증거처럼 보이면 안 됩니다.
- 사용자 대상 문서는 독자가 internal Harness vocabulary를 알아야만 무엇을 요청할지, 에이전트가 무엇을 확인할지, 무엇이 막혔는지, 어떤 사용자 판단이 필요한지, close가 무슨 뜻인지 이해하는 형태가 아니어야 합니다.
- Exact contract는 owner Reference 문서에 둡니다. 다른 문서는 독자에게 보이는 결과를 요약하고 owner로 연결합니다. Schema, DDL, gate, fixture body, projection template, state-transition rule을 중복하지 않습니다.
- Documentation file은 Harness runtime object가 아닙니다. 향후 runtime Write Authorization, Evidence Manifest, `task_events`, Acceptance, residual-risk, projection, conformance, generated operational-output rule의 지배를 받지 않습니다.

### 재설계 백로그 틀

재설계 finding은 아래 틀로 작게 나누어 라우팅합니다.

- 제품 정의 drift: Harness를 local authority record와 judgment-routing layer로 유지합니다. Prompt 묶음, workflow engine, report generator, dashboard, broad hosted agent platform으로 만들지 않습니다.
- MVP/단계 경계 drift: v0.1은 내부 Core Authority Smoke, v0.2는 첫 First User-Value Slice로 둡니다. Future/profile/diagnostic 내용은 owner가 승격하기 전까지 현재 단계 요구사항 밖에 둡니다.
- 판단 모델 복잡도: 사용자 소유 판단을 보이게 유지하고, 결정의 크기에 맞춥니다. Agent 판단, sensitive-action Approval, 작업 수락, 잔여 위험 수용과 섞지 않습니다.
- Close/verification 모호성: 근거, 검증, 수동 QA, 작업 수락, close readiness, 잔여 위험을 분리합니다. 어느 것도 다른 것을 대신하지 않습니다.
- 보안 보장 과장 위험: Cooperative, detective, preventive, isolated 표현은 문서화된 mechanism과 증명 수준에 맞게 씁니다.
- Context/token 과부하 위험: 항상 주입되는 agent context는 짧고 최신으로 유지합니다. 자세한 contract는 owner 문서나 조회 경로로 보냅니다.
- 사용자 대상 용어 부담: 사용자가 보는 상황을 먼저 씁니다. 사용자가 Discovery, Change Unit, Decision Packet, Write Authorization, Evidence Manifest, Projection, Gate, `task_events` 같은 내부 라벨을 말해야만 진행되는 것처럼 쓰지 않습니다. 내부 용어는 독자가 행동하거나, 보이는 막힘을 해석하거나, Reference 계약을 자세히 확인하는 데 도움이 될 때만 소개합니다.

## 보존하는 원칙

구현 세부사항은 바뀔 수 있습니다. 다만 다음 원칙은 유지해야 합니다.

- Harness는 prompt 묶음이 아닙니다. Scope, 사용자 소유 판단, 근거, 검증, QA 기대, 작업 수락, 잔여 위험 상태, 닫기 준비 상태를 다루는 로컬 권한 기록입니다.
- 사용자 소유 판단은 보존되어야 합니다. 제품 결정, 중요한 기술 결정, QA 기대치, 작업 수락, waiver, 잔여 위험 수용은 소유자 계약이 달리 정하지 않는 한 사용자 판단으로 남습니다.
- 근거, 검증, 수동 QA, 작업 수락, 잔여 위험은 서로 다른 기록과 판단이며 서로를 대체할 수 없습니다.
- 대화, Markdown으로 렌더링된 읽기용 요약(Projection), connector 출력, 생성 문서는 운영 기준이 아닙니다. Core가 소유한 로컬 상태와 아티팩트 참조가 운영 기준입니다.

문서를 다시 쓰면서 용어, 단계, 스키마 구조, 투영 구조, 보안 표현, 문서 경계를 바꿀 때는 문장을 다듬기 전에 이 원칙이 유지되는지 먼저 확인합니다.

## 문서 인계 규칙

[구현 개요: 문서 인계 요약](../build/implementation-overview.md#문서-인계-요약)이 짧은 인계 요약을 담당합니다. 여기에 문서 세트가 정의하는 것, 문서 검토 상태, 구현 계획 준비 상태, 런타임 구현 상태, 향후 저장소 역할, 보존 원칙, 현재 단계 모델, 하네스 서버 구현 준비 조건, 남은 질문 상태, 남은 문서 drift 상태, maintainer 수락 조건을 둡니다.

[MVP 계획: 서버 코딩 전 필요한 구현 결정](../build/mvp-plan.md#서버-코딩-전-필요한-구현-결정)은 maintainer review나 첫 runtime batch planning에서 발견된 큰 구현 시작 전 결정을 기록하는 단일 위치입니다. 큰 결정을 active docs 곳곳의 `TODO_DECISION`으로 남기지 않습니다. 현재 기준에서 결정 기록이 비어 있다면 정확히 그렇게 말합니다. 구현 준비 조건에 아직 maintainer 판단이 필요한 동안에는 이것을 "열린 결정 없음" 주장으로 바꾸지 않습니다. 편집 정리만 남았다면 어떤 docs-maintenance category가 담당하는지, 왜 현재 stage를 막지 않는지 함께 말합니다.

## 알려진 재설계 쟁점 트래커

이 tracker는 drift가 자주 생기는 영역을 확인하는 maintainer 검토 checklist로 사용합니다. 아래 항목은 maintainer용 review risk이지 열린 구현 결정, runtime 구현 작업, runtime conformance, acceptance record가 아닙니다. 이 tracker는 구현 준비 상태를 증명하지 않으며 server/runtime 구현을 허가하지 않습니다.

Tracker 상태 의미:

- 현재 문서에서 확인된 drift: 이번 pass 또는 유지보수자 검토에서 활성 문서 안의 drift를 확인한 상태입니다. 고칠 위치를 잡을 수 있을 만큼의 맥락을 함께 남깁니다.
- 확인 대상 후보: 그럴 가능성이 있는 risk이지만, 이번 pass에서 실제 존재를 증명하지 않았습니다. 확인 전에는 관찰된 drift나 서버 코딩 blocker로 취급하지 않습니다.
- 회귀 방지 점검: 현재 기준에서는 문구가 괜찮다고 보지만, 이후 편집에서 같은 drift를 다시 들여오면 안 됩니다.
- 기준 상태 점검: 진입점과 인계 섹션이 현재 저장소 상태를 계속 정확히 말해야 합니다.

문제를 "non-blocking"이라고 부르려면 어떤 단계에는 막지 않는지, 어떤 이후 단계에는 막을 수 있는지 함께 적어야 합니다. 구현 준비 우려를 막연한 "follow-up"으로 숨기지 말고 담당 문서, 영향을 받는 단계, 필요한 결정 또는 편집을 이름 붙입니다.

상태가 "확인 대상 후보"인 행은 현재 drift가 실제로 존재한다는 주장으로 읽지 않습니다. 확인 결과 서버 코딩 전 결정이 필요하면 [MVP 계획: 서버 코딩 전 필요한 구현 결정](../build/mvp-plan.md#서버-코딩-전-필요한-구현-결정)에 담당 문서, 영향을 받는 동작 또는 field, 영향을 받는 단계, 선택지, 필요한 결정을 기록합니다.

아래 routing table은 항목이 이미 막힘이라는 증거가 아닙니다. 검토 risk는 확인 결과 그런 종류의 문제가 드러난 뒤에만 문서 drift, 스키마/설계 결정, 단계 경계 결정, 구현 준비 막힘, 향후 로드맵 항목이 됩니다.

확인된 tracker finding이나 docs-maintenance finding을 라우팅할 때는 다음 항목 범주를 사용합니다.

| 항목 범주 | 사용하는 경우 |
|---|---|
| 문서 drift | 필요한 조치가 문구 정리, 소유자 경계 정리, link 수정, TODO 정리, 용어 정리, 영어/한국어 의미 일치일 때. |
| 스키마/설계 결정 | schema, state, API, DDL, security guarantee, fixture 의미, 그 밖의 owner contract에서 실제 선택이 필요할 때. |
| 단계 경계 결정 | capability가 코어 권한 스모크(v0.1 Core Authority Smoke), 첫 사용자 가치 조각(v0.2 First User-Value Slice), 에이전시 보증 팩(v0.3 Agency Assurance Pack), 운영과 인계 팩(v0.4 Operations & Handoff Pack), v1+ Expansion 중 어디에 속하는지 결정해야 할 때. |
| 구현 준비 조건 | 첫 런타임 배치 계획을 수락하기 전에 유지보수자가 확인해야 하는 조건일 때. |
| 향후 로드맵 항목 | 유용하지만 승격되기 전까지 v0.1부터 v0.4 밖에 남아야 하는 항목일 때. |

확인 뒤 예상 항목 범주:

| 검토 risk | 확인되었을 때 기본 routing |
|---|---|
| 이 저장소가 앞으로 하네스 서버 소스 저장소가 된다는 설명이 흐려질 수 있습니다. | 구현 준비 조건 |
| Stage 이름이 v0.1, Kernel Smoke, 또는 예전 kernel-stage label을 제품 MVP처럼 보이게 할 수 있습니다. | 단계 경계 결정 |
| 사용자용 문서가 무거운 구현 disclaimer로 시작할 수 있습니다. | 문서 drift |
| 사용자용 문서에 내부 용어가 너무 많습니다. | 문서 drift |
| 요구사항 탐색(discovery)과 확인이 Change Unit 또는 첫 안전한 구현 단위로 너무 빨리 수렴할 수 있습니다. | 단계 경계 결정 |
| 예전 판단 field alias mapping이 drift될 수 있습니다. | 스키마/설계 결정 |
| 작은 결정을 다루기에 Decision Packet schema와 예시가 너무 무거워 보일 수 있습니다. | 스키마/설계 결정 |
| Approval, 작업 수락, 잔여 위험 수용을 혼동하기 쉽습니다. | 스키마/설계 결정 |
| Storage/DDL이 future-profile table, field, gate를 너무 이른 필수 범위처럼 보이게 할 수 있습니다. | 단계 경계 결정 |
| Conformance fixture 문서가 현재 구현 단계에 비해 너무 자세할 수 있습니다. | 구현 준비 조건 |
| Operations entrypoint가 너무 이른 단계의 필수 요소처럼 보일 수 있습니다. | 단계 경계 결정 |
| 한국어 사용자용 문서에 영어 기술 명사가 과도하게 남을 수 있습니다. | 문서 drift |
| 비어 있는 결정 기록 표현이 지나치게 낙관적으로 들릴 수 있습니다. | 구현 준비 조건 |
| 현재 MVP 단계가 너무 크고, 핵심 사용자 가치를 뒤로 미룰 수 있습니다. | 단계 경계 결정 |
| Projection/template 범위가 초기 구현에 비해 넓을 수 있습니다. | 단계 경계 결정 |
| 보안 보장 표현은 실제 강제 수준과 맞아야 합니다. | 스키마/설계 결정 |
| Agent context 전략은 prompt/context 부담이 과도해지지 않게 해야 합니다. | 구현 준비 조건 |
| 문서가 런타임 객체처럼 읽힐 수 있습니다. | 문서 drift |
| 로드맵 후보가 승격 없이 단계별 전달 범위로 들어올 수 있습니다. | 향후 로드맵 항목 |

| 검토 risk | Tracker 상태 | 편집 규칙 |
|---|---|---|
| 이 저장소가 앞으로 하네스 서버 소스 저장소가 된다는 설명이 흐려질 수 있습니다. | 기준 상태 점검. | 현재는 문서 전용이고, 재설계 이후 검토 상태이며, 향후 역할은 하네스 서버 소스 저장소이고, 런타임/서버 구현은 아직 시작하지 않았으며 문서 수락과 별도의 구현 계획 준비 결정 이후에만 시작할 수 있다는 점을 진입점 문서에서 분명히 유지합니다. |
| Stage 이름이 v0.1, Kernel Smoke, 또는 예전 kernel-stage label을 첫 사용자 가치 조각처럼 보이게 할 수 있습니다. | 확인 대상 후보. | v0.1 Core Authority Smoke는 내부 authority loop milestone이고, Kernel Smoke는 그 좁은 future smoke-check 작성 label이며, v0.2 First User-Value Slice가 첫 좁은 사용자 가치 조각이라고 말합니다. |
| 사용자용 문서가 무거운 구현 disclaimer로 시작할 수 있습니다. | 확인 대상 후보. | 사용자 대상 Learn/Use 문서는 사용자가 무엇을 요청할 수 있는지, 에이전트가 무엇을 구체화해야 하는지, 하네스가 무엇을 보존하는지, 사용자가 무엇을 보게 되는지를 먼저 보여주는 사용자 흐름 우선 도입부를 선호합니다. 상세 단계와 상태 경고는 root README, 언어별 README, Build 인계 문서, Maintain 지침으로 보냅니다. 문서 안의 상태 메모는 짧게 유지합니다. |
| 사용자용 문서에 내부 용어가 너무 많습니다. | 확인 대상 후보. | 사용자가 보는 상황을 먼저 설명하고, 내부 용어는 행동에 도움이 될 때만 소개합니다. |
| 요구사항 탐색(discovery)과 확인이 Change Unit 또는 첫 안전한 구현 단위로 너무 빨리 수렴할 수 있습니다. | 확인 대상 후보. | 범위가 정해진 구현 단위를 요구하기 전에 초기 discovery, 공유 이해, 사용자 소유 판단의 여지를 남깁니다. |
| 예전 판단 field alias mapping이 drift될 수 있습니다. | 설계 해소됨; 회귀 방지 점검. | 활성 담당 문서는 `judgment_category`, `judgment_route`, `display_depth`를 사용합니다. `judgment_domain`, `decision_kind`, `decision_profile`은 오래된 request shape를 위한 compatibility alias이지 사용자가 이해해야 하는 독립 축이 아닙니다. 새 예시는 활성 judgment 이름을 우선하고, 영향을 받는 gate나 막힌 행동은 별도의 owner field에 남깁니다. |
| 작은 결정을 다루기에 Decision Packet schema와 예시가 너무 무거워 보일 수 있습니다. | 설계 해소됨; 회귀 방지 점검. | 작은 결정은 `minimal_decision`을 사용할 수 있습니다. Full trade-off, approval, waiver, acceptance, residual-risk, reconcile, mixed profile은 여전히 필요한 context를 포함해야 합니다. 이후 편집에서 모든 Decision Packet이 full trade-off field를 요구하도록 만들면 안 됩니다. |
| Approval, 작업 수락, 잔여 위험 수용을 혼동하기 쉽습니다. | 회귀 방지 점검. | 민감 동작 승인, 작업 수락, 잔여 위험 수용을 예시와 routing text에서 분리합니다. |
| Storage/DDL이 future-profile table, field, gate를 너무 이른 필수 범위처럼 보이게 할 수 있습니다. | 확인 대상 후보. | Reference schema에 존재한다는 사실과 단계별 구현 요구를 구분합니다. Required field는 담당 tool, record, profile이 구현되거나 사용될 때 적용되며, 그 자체로 가장 작은 runnable slice를 키우지 않습니다. |
| Conformance fixture 문서가 현재 구현 단계에 비해 너무 자세할 수 있습니다. | 확인 대상 후보. | Fixture 문서는 단계화된 향후 계획으로 유지합니다. 현재 executable fixture file이나 runnable Harness Server conformance test가 있다는 인상을 주지 않습니다. |
| Operations entrypoint가 너무 이른 단계의 필수 요소처럼 보일 수 있습니다. | 확인 대상 후보. | 관련 Build 단계가 명시적으로 포함하기 전까지 operator entrypoint는 단계화된 향후 범위로 둡니다. 문구 drift 때문에 v0.1 전제 조건이 되면 안 됩니다. |
| 한국어 사용자용 문서에 영어 기술 명사가 과도하게 남을 수 있습니다. | 확인 대상 후보. | 자연스러운 한국어를 먼저 씁니다. 정확한 English identifier는 stable label, schema name, file name, enum value, API field, 또는 정밀도가 필요한 곳에서만 유지합니다. |
| 비어 있는 결정 기록 표현이 지나치게 낙관적으로 들릴 수 있습니다. | 회귀 방지 점검. | "서버 코딩 전 결정 기록은 현재 기준에서 비어 있음"처럼 쓰고, 검토에서 새 결정이 발견될 수 있음을 함께 말합니다. 완전 수락, 구현 완료, 구현 준비 상태, 서버 코딩 준비 상태를 암시하지 않습니다. |
| 현재 MVP 단계가 너무 크고, 핵심 사용자 가치를 뒤로 미룰 수 있습니다. | 확인 대상 후보. | MVP 크기와 초기 사용자 가치 사이의 긴장을 드러냅니다. Staging 결정은 owning Build/Reference 문서에 남깁니다. |
| Projection/template 범위가 초기 구현에 비해 넓을 수 있습니다. | 확인 대상 후보. | 초기 범위가 넓다는 점을 표시하고, staging 결정은 projection/template owner로 보냅니다. |
| 보안 보장 표현은 실제 강제 수준과 맞아야 합니다. | 회귀 방지 점검. | Cooperative, detective, preventive, isolated 표현은 해당 surface가 그 수준을 제공할 때만 사용합니다. |
| Agent context 전략은 prompt/context 부담이 과도해지지 않게 해야 합니다. | 회귀 방지 점검. | 항상 주입되는 agent context는 짧게 유지하고, 세부사항은 담당 문서나 조회 경로로 보냅니다. |
| 문서가 런타임 객체처럼 읽힐 수 있습니다. | 회귀 방지 점검. | 현재 재설계 범위의 분리 규칙을 따릅니다. 문서는 원천 자료이며 runtime state나 generated projection이 아닙니다. |
| 로드맵 후보가 승격 없이 단계별 전달 범위로 들어올 수 있습니다. | 회귀 방지 점검. | v1+ Expansion 항목은 담당자가 scope, fixture, fallback 동작, 읽기용 요약을 기준으로 삼는 의존성 없음으로 승격하기 전까지 로드맵에 둡니다. 향후 로드맵 항목을 문서 검토, v0.1, v0.2의 선행 조건처럼 다루지 않습니다. |

## 문서 작성 원칙

문서는 독자의 다음 행동에서 출발합니다. 독자가 무엇을 이해하고, 결정하고, 사용하고, 구현하고, 검증하고, 유지해야 하는지 분명해야 합니다.

내부 구조를 빠짐없이 나열하기보다 핵심 아이디어를 적게, 선명하게 설명합니다. 엄격한 계약은 Reference 문서로 보내고, 다른 문서에서는 필요한 만큼만 요약한 뒤 링크합니다.

낯선 개념은 정의부터 던지지 않습니다. 먼저 실제 상황이나 짧은 예시로 왜 필요한지 보여주고, 그다음 이름과 정의를 붙입니다.

각 문서는 시작 부분에서 독자의 다음 행동을 분명히 보여줘야 합니다. 도입부는 예측 가능한 heading, 평소 요청, 실용 예시, 사용자 흐름 우선 도입부 중 문서에 맞는 방식을 쓸 수 있습니다. 독자가 왜 이 문서가 필요한지, 다음에 무엇을 하면 되는지, 정확한 owner 세부사항이 어디에 있는지 빨리 알 수 있으면 됩니다.

현재 문서는 현재의 사실처럼 씁니다. 마이그레이션 과정, 제거된 구조, 예전 이름은 본문 설명에 넣지 않습니다. 마이그레이션 중 별도 migration note가 있는 경우에만 그곳에 두고, 그렇지 않으면 Git history나 명확히 표시된 비활성 마이그레이션 기록에 남깁니다.

## 문서 유형

문서 tree는 소유권을 나누기 위한 구조입니다. Learn 문서는 이유를 설명합니다. Use 문서는 사용자와 agent가 어떻게 상호작용하는지 설명합니다. Build 문서는 구현 순서를 설명합니다. Reference 문서는 정확한 계약을 정의합니다. Maintain 문서는 문서 유지보수 규칙을 정의합니다. Normative contract를 여러 경로에 중복하지 말고, 필요한 곳에서는 짧게 요약한 뒤 owner로 연결합니다.

### Learn

Learn 문서는 독자의 이해 모델을 만듭니다.

Harness가 왜 필요한지, 개념이 왜 중요한지, 어떤 절충점이 있는지를 구현 세부사항보다 먼저 설명합니다. 독자에게 명령, 스키마, 체크리스트보다 방향 감각이 필요할 때 사용합니다.

### Use

Use 문서는 사용자가 AI 지원 개발 세션에서 Harness를 따라가도록 돕습니다.

사용자와 agent가 Harness와 어떻게 상호작용하는지 설명합니다. 사용자에게 보이는 흐름, 상태 해석, 결정 지점, handoff, 복구 경로를 중심에 둡니다. 내부 gate는 사용자가 보는 막힘이나 next action을 설명할 때만 이름 붙입니다.

### Build

Build 문서는 문서 수락과 별도의 구현 계획 준비 결정 이후 reference system을 구현하는 사람을 돕습니다.

문서 수락과 별도의 구현 계획 준비 결정 이후의 구현 순서를 설명합니다. 구현 순서, module 경계, 실행 가능한 조각, staging, 검증 전략을 다룹니다. 정확한 스키마, DDL, 불변 조건은 Reference 문서로 연결합니다.

### Reference

Reference 문서는 정확한 계약을 담당합니다.

엄격한 스키마, gate, DDL, enum value, state transition, 불변 조건, API shape, storage rule, projection rule, fixture 의미, 공식 정의를 정의합니다.

### Maintain

Maintain 문서는 문서 시스템 자체를 관리합니다.

문서 유지보수 규칙을 정의합니다. 작성 규칙, 번역 정책, 리뷰 체크리스트, link hygiene, ownership map, documentation-maintenance expectation이 여기에 속합니다. Maintain 문서가 런타임 conformance spec이나 product implementation plan이 되면 안 됩니다.

## 진입점 규칙

README 문서는 긴 설명서이기 전에 길잡이입니다. Harness가 무엇이고 무엇이 아닌지 짧게 말한 뒤, 처음 읽는 사람, 사용자, 구현자, Reference 독자, 유지보수 담당자를 알맞은 owner 문서로 빠르게 안내해야 합니다.

진입점은 현재 구조를 작고 명확하게 보여줘야 합니다. 명확히 비활성 migration record라고 표시한 섹션이 아니라면 migration history, 제거된 이름, 비활성 path, 예전 구조를 보존하는 장소로 쓰지 않습니다.

README 문서는 경로별 소유권을 요약할 수 있지만 엄격한 계약을 복사하면 안 됩니다. 정확한 schema, DDL, gate, state transition, fixture 의미, template 본문, 공식 정의는 Reference owner로 연결합니다.

처음 읽는 사람을 위한 경로에는 더 깊은 Learn과 Reference 경로 전에 빠른 실전 둘러보기가 포함되어야 합니다. Use 경로에는 사용자가 엄격한 Reference contract를 읽기 전에 judgment prompt를 이해할 수 있도록 사용자 가이드 근처에 실용 Decision Packet 예시를 포함해야 합니다.

## 문서 시작 방식

활성 문서는 짧은 시작부에서 독자의 경로를 보여줘야 합니다. 필요한 정보는 정확한 heading 이름, 자연스러운 heading, 본문 설명, 예시, 사용자 흐름 우선 도입부 중 문서에 맞는 방식으로 담을 수 있습니다. 아래 네 heading을 모든 문서에 강제하지 않습니다.

Reference, Build, Maintain 문서는 독자에게 도움이 될 때 다음과 같은 예측 가능한 구조를 쓸 수 있습니다.

- `이 문서로 할 수 있는 일`: 문서가 독자에게 주는 결과를 평범한 말로 씁니다. 무엇을 "다룬다"는 설명만으로 끝내지 않습니다.
- `이런 때 읽기`: 이 문서를 읽어야 하는 상황을 적습니다. 짧은 목록이어도 됩니다.
- `읽기 전에`: 필요한 사전 맥락, 먼저 읽을 문서, 전제 조건을 적습니다. 전제 조건이 없다면 간단히 없다고 말합니다.
- `핵심 생각`: 나머지 내용을 이해하기 쉽게 만드는 중심 모델이나 주장을 먼저 알려줍니다.

이 이름들은 하나의 패턴이지 모든 문서의 heading 계약이 아닙니다. 독자 상황이 다른 방식으로 더 선명해진다면 정확한 heading 이름을 쓰지 않아도 시작 규칙을 만족할 수 있습니다.

### Learn/Use 문서의 사용자 흐름 우선 도입부

사용자 대상 Learn/Use 문서는 평소 요청, 실용 예시, 사용자 흐름으로 시작하는 편이 독자에게 더 분명할 수 있습니다. 특히 기본 사용자 문서에서는 이 방식이 더 알맞을 수 있습니다.

사용자 흐름 우선 도입부는 다음을 보여줘야 합니다.

- 사용자가 무엇을 요청할 수 있는지
- 에이전트가 무엇을 구체화해야 하는지
- 하네스가 무엇을 보존하는지
- 사용자가 무엇을 보게 되는지
- 상태 메모는 짧게 유지하고 상세 상태 경고는 root README, 언어별 README, Build 인계 문서, Maintain 지침으로 보낸다는 점
- 내부 하네스 용어는 사용자에게 보이는 상황이 먼저 분명해진 뒤 소개한다는 점

문서가 독자 상황을 잘 돕고, 필요한 맥락이 있으며, owner link가 유지되고, 정확한 계약 세부사항이 Reference owner에 남아 있고, 영어/한국어 문서가 의미상 일치한다면 heading text 차이는 drift가 아닙니다. 구조 패턴을 맞추기 위해 `direct`, `work`, `Decision Packet`, `judgment_domain` 같은 내부 라벨을 도입부에 다시 넣지 않습니다.

### Reference 범위

Reference 문서에만 둡니다. 이 문서가 어떤 정확한 계약을 담당하고, 무엇을 담당하지 않는지 밝힙니다. 이렇게 해야 Learn, Use, Build 문서로 엄격한 세부사항이 퍼지지 않습니다.

### 템플릿 참조 시작 방식

템플릿 참조 파일은 별도의 시작 방식을 씁니다. Docs-maintenance는 directory index인 `docs/*/reference/templates/README.md`와 개별 template인 `docs/*/reference/templates/` 아래의 README가 아닌 Markdown file을 경로로 구분해야 합니다.

디렉터리 README는 `사용 시점`으로 시작한 뒤 산출물 계층과 템플릿 구현 계층을 둡니다. 이 README는 디렉터리가 렌더링된 template body와 display card shape를 담당하며, projection rule, freshness behavior, authority boundary는 각 Reference owner에 남는다는 점을 설명해야 합니다.

개별 template file은 다음 section을 이 순서로 시작해야 합니다.

- `사용 시점`: 독자 목적과 projection 또는 display 상황.
- `기준 기록`: renderer가 읽을 수 있는 owner record, ref, gate, artifact, summary.
- `렌더링 섹션`: 독자가 기대해야 하는 display shape.
- `전체 템플릿`: 완전한 rendered body 또는 card body.

Template file은 시작 설명이나 `메모` 근처에서 권한 없음 경계를 보여줘야 합니다. Template은 렌더링 표시일 뿐이며 기준 상태, gate authority, 민감 동작 승인, 작업 수락, evidence, schema, DDL, runtime behavior가 아닙니다.

## 개념 소개 규칙

개념은 엄격한 정의보다 예시로 먼저 소개합니다.

구체적인 상황에서 시작해 어떤 문제를 해결하는지 보여준 뒤 개념 이름을 붙입니다. 독자가 왜 중요한지 본 다음에 엄격한 정의를 둡니다.

좋은 흐름:

```text
에이전트가 제품 상태를 변경하려면 하네스는 먼저 작업 범위를 알아야 합니다. 무엇이 바뀔 수 있고, 무엇은 범위 밖이며, 어디에서 멈춰야 하는지입니다. 이 내부 scoped-write record가 Change Unit입니다. 사용자가 끝내거나 답을 얻고 싶은 더 큰 가치 단위가 Task입니다.
```

Learn 문서를 조밀한 정의 목록으로 시작하지 않습니다. Glossary나 reference table이라면 예외입니다.

## Reference 계약 규칙

엄격한 스키마, gate, DDL, enum value, state transition, 불변 조건, API shape, storage rule, projection contract detail, fixture 의미는 Reference 문서에 둡니다.

Learn, Use, Build, Maintain 문서는 필요할 때 계약을 한두 문장으로 요약하고 owner Reference 문서에 링크합니다. 전체 table, schema body, transition matrix, DDL block, gate matrix, algorithm step, fixture mini-language를 중복하지 않습니다.

Build 문서는 무엇을 먼저 만들지, 무엇을 미룰지, 무엇이 stage 완료를 증명하는지 설명합니다. Public request/response schema, DDL, storage validation rule, close-blocker taxonomy, gate compatibility matrix, fixture assertion field를 복사하지 않습니다. Build checklist에 정확한 detail이 필요하면 owner Reference section으로 연결하고, 현재 순서에서 의미하는 점만 짧게 씁니다.

Use 문서는 user trust boundary에 머뭅니다. 사용자가 보는 hold, blocker, decision prompt, evidence gap, close result를 이해해야 할 때는 관련 contract를 이름 붙일 수 있습니다. 하지만 사용자가 판단하는 데 필요하지 않다면 field list, storage row, validator 내부 detail을 드러내지 않습니다.

Reference 문서는 contract 중심이어야 합니다. 짧은 쉬운 설명은 도움이 되지만 긴 tutorial, staged delivery plan, reader walkthrough는 Learn, Use, Build로 보내고, Reference는 정확한 규칙을 이해하는 데 필요한 계약만 유지합니다.

Runtime conformance fixture body shape, assertion mode, isolated execution behavior, JSON `TEXT` validation, owner-bound enum/status validation은 [Conformance Fixtures 참조](../reference/conformance-fixtures.md#conformance-fixture-format)가 담당합니다. 다른 문서는 conformance가 executable-state-based라는 점만 요약하고 owner로 링크해야 하며, 전체 계약을 다시 적지 않습니다.

향후 상세 scenario 후보, concern별 향후 fixture example, staged fixture coverage map, fixture suite family summary, catalog-only future candidate는 [향후 Fixture Catalog](../reference/future-fixture-catalog.md)가 담당합니다. Future catalog row는 설계 inventory일 뿐이며, 정확한 fixture body, public MCP schema, DDL, stage exit, runtime readiness, generated artifact, fixture가 이미 실행된다는 증거를 다시 정의하면 안 됩니다.

## 반복 규칙

긴 기준 기록 문단을 여러 문서에 반복하지 않습니다.

다른 문서에 같은 생각이 필요하면 짧게 요약하고 owner 문서로 링크합니다. 원문이 바뀌면 owner 문서를 먼저 고친 뒤 요약문이 어긋나지 않았는지 확인합니다.

독자가 다른 예시를 필요로 한다면 설명용 예시는 반복할 수 있습니다. 하지만 규범적인 계약 문구를 반복하면 불일치 위험이 큽니다.

Build 또는 Reference에 긴 계약 문단을 추가하거나 받아들이기 전에는 같은 field, gate, API, storage, fixture, security wording이 다른 문서군에 있는지 검색합니다. Build가 Reference를 반복한다면 Build는 순서와 owner link만 남깁니다. Reference가 구현 여정을 설명하는 데 치우쳤다면 그 설명은 Build로 보내거나 Build에 연결하고, 해당 section을 이해하는 데 필요한 contract만 남깁니다.

반복되기 쉬운 권한 없음 경계는 다음 owner를 사용합니다.

| 경계 | 정확한 문구의 owner |
|---|---|
| Context Index와 retrieved/indexed context | Future feature 경계는 [로드맵: 후보 항목 목록](../roadmap.md#후보-항목-목록), connector context 처리는 [Agent Integration: Context Push/Pull Principles](../reference/agent-integration.md#context-pushpull-principles) |
| Local Derived Metrics | [로드맵: 후보 항목 목록](../roadmap.md#후보-항목-목록) |
| Role Lens | [Agent Integration: Role Lens 동작](../reference/agent-integration.md#role-lens-동작) |
| Review Stages | [Design Quality Policies: Two-stage Review Display](../reference/design-quality-policies.md#two-stage-review-display) |
| Release Handoff와 export | [Operations And Conformance: Release Handoff Export Profile](../reference/operations-and-conformance.md#release-handoff-export-profile); 렌더링 형태는 [EXPORT Template](../reference/templates/export.md) |
| Docs-maintenance | Rule body는 [Authoring Guide: Docs-maintenance checks](#docs-maintenance-checks), operator 보고는 [Operations And Conformance: docs-maintenance profile](../reference/operations-and-conformance.md#docs-maintenance-프로필) |
| Projection과 report surfaces | [Document Projection Reference](../reference/document-projection.md), 렌더링 형태는 [Template Reference](../reference/templates/README.md) |
| Security asset, trust boundary, threat category, control category, guarantee-level 의미, high-risk cooperative/detective/preventive/isolated security expectation | Threat concept과 honest guarantee display는 [보안 위협 모델 참조](../reference/security-threat-model.md)가 담당하고, exact API, storage, kernel, connector, operations, conformance behavior는 각 owner에 남습니다. |

## Owner 링크 요약 패턴

Owner 밖에서 중복된 규범 문구를 찾으면 그 중복 문구를 그대로 다듬지 않습니다. 먼저 어떤 문서가 정확한 계약을 소유하는지 정합니다. 계약 자체를 바꿔야 한다면 owner 문서를 먼저 고친 뒤, owner가 아닌 복사본은 다음 형태로 바꿉니다.

- 독자가 지금 알아야 하는 내용을 평범한 말로 한 문장
- 정확한 규칙을 담은 owner 문서나 owner section 링크
- 현재 독자 경로에서 달라지는 점

예:

```text
제품 파일 쓰기에는 현재 Change Unit 범위와 Write Authorization이 필요합니다. 정확한 write-gate 동작은 [커널 참조](../reference/kernel.md)가 담당하고, 공개 request shape은 [MCP API와 스키마](../reference/mcp-api-and-schemas.md)가 담당합니다.
```

Gate matrix, request schema, DDL block, fixture body, template body, enum table, glossary definition을 Learn, Use, Build, Maintain 문서에 붙여 넣지 않습니다.

## 다이어그램 규칙

Diagram은 인지 부담을 줄일 때만 사용합니다.

관계, 순서, 경계, lifecycle이 본문보다 그림으로 더 분명할 때 diagram이 유용합니다. 장식으로 넣거나, 이미 명확한 목록을 한 번 더 보여주거나, 아직 정리되지 않은 구조를 감추기 위해 쓰지 않습니다.

모든 diagram 근처에는 무엇을 봐야 하는지 알려주는 본문이 있어야 합니다. Diagram과 본문이 다르면 owner 본문이나 reference contract를 먼저 고칩니다.

## 영어/한국어 의미 일치 규칙

영어와 한국어 문서는 같은 활성 파일 맵, 의미상 같은 섹션 범위, 같은 계약 세부사항을 유지해야 합니다.

영어/한국어 대응 문서는 같은 활성 파일 맵과 의미상 같은 섹션 범위를 유지합니다. 다만 owner 링크, stable identifier, 검토 가능성이 분명하다면 한국어 heading과 소단락 구성은 자연스러운 한국어가 되도록 조정할 수 있습니다. 의미상 같은 한국어 heading 차이는 docs-maintenance에서 자동 `FAIL`로 보지 않습니다. Official identifier, API name, schema name, enum value, DDL name, file name, error code, validator ID, code identifier, translation guide에 있는 product term은 정확히 유지합니다.

`docs/en`의 의미가 바뀌면 같은 batch에서 `docs/ko`도 반영합니다. 반대 방향도 같습니다.

## 한국어 문서 품질 규칙

한국어 문서는 직역하지 않고 의미를 맞춰 갱신합니다. 같은 의미, owner link, stable identifier는 유지하되, 더 읽기 쉬우면 문장 순서, heading, 문단 묶음은 달라도 됩니다.

한국어 사용자용 문서는 자연스러운 한국어를 먼저 씁니다. Stable English identifier는 독자가 알아보거나 검색하거나 정확한 계약과 맞춰야 할 때만 괄호로 붙입니다. 사용자용 section에서 어색한 bilingual prose를 만들지 않습니다.

가능하면 한국어 문장은 영어 원문보다 짧게 둡니다. 긴 영어 문장 구조를 그대로 옮기지 말고 짧은 한국어 문장으로 나눕니다.

Exact schema identifier, API name, enum value, DDL name, file name, error code, validator ID, code identifier, official product term은 Reference나 maintainer-facing context에서 정확히 보존합니다. Learn과 Use 문서에서는 독자의 next action에 필요한 경우가 아니라면 평범한 한국어 개념을 먼저 소개합니다.

## 링크와 이름 변경 규칙

문서 이름을 바꾸거나, 옮기거나, 나누거나, 합칠 때는 양쪽 언어의 링크를 같은 batch에서 고칩니다.

2차 요약보다 owner 문서나 owner section으로 링크합니다. Active owner link가 제거된 migration context를 가리키면 안 됩니다.

예전 이름, 예전 구조, migration 결정을 리뷰 목적으로 남겨야 한다면 명확히 비활성 migration record라고 표시한 곳에 둡니다. Active docs는 현재 구조를 설명하고 현재 owner로 연결해야 합니다.

이름 변경 뒤에는 이전 path, 이전 anchor, 이전 heading, 이전 title text를 검색합니다. README path, 주변 cross-reference, template/reference link, paired-language link를 함께 업데이트합니다.

## Docs-maintenance checks

Docs-maintenance checks는 편집 품질 점검입니다. Documentation drift, owner mismatch, 영어/한국어 의미 일치 문제, owner 밖의 중복 규범 문구, 깨진 link나 anchor, TODO hygiene 문제를 보고할 수 있습니다. Runtime conformance나 implementation readiness가 아니며, fixture action을 실행하거나, runtime state를 seed하거나, runtime state/events/artifacts/projections/errors를 비교하거나, runtime fixture pass/fail에 포함되지 않습니다. 기준 상태, runtime state, `task_events`, evidence artifact나 Evidence Manifest, QA result나 수동 QA record, Acceptance record, 잔여 위험 수용이나 Residual Risk record, close readiness, projection refresh나 운영 보고서, implementation readiness를 만들거나 갱신하지 않습니다.

Maintain 문서는 documentation review rule, category label, reviewer expectation을 정의할 수 있습니다. Runtime conformance pass/fail, runtime fixture semantics, Core state effect, gate behavior, implementation readiness를 정의하면 안 됩니다. Docs-maintenance finding이 runtime contract를 건드리면 그 contract를 다시 적지 말고 owner Reference 문서를 가리켜야 합니다.

### 최종 사전 수락 리뷰

Maintainer가 문서 세트를 구현 계획에 사용할 수 있다고 받아들이기 전, 마지막 docs-maintenance pass를 수행합니다. 영어/한국어 활성 파일 맵 일치, 대응 파일의 의미 섹션 일치, 깨진 link와 anchor, owner-boundary drift, owner가 아닌 문서의 중복 contract, Approval, Decision Packet, Evidence, Verification, 수동 QA, Acceptance, Residual Risk, Projection, Guarantee Level 용어 drift, TODO hygiene를 확인합니다.

[구현 개요](../build/implementation-overview.md#하네스-서버-구현-준비-조건)의 하네스 서버 구현 준비 조건도 확인합니다. 저장소 정체성, 내부 용어 부담 없는 사용자 대상 흐름, Change Unit 조기 수렴이 아닌 요구사항 확인으로서의 Discovery, 활성 judgment field와 mapped legacy alias, 결정 크기에 맞는 Decision Packet profile, Approval/작업 수락/잔여 위험 수용 분리, coherent stage, Kernel/API/storage/reference agreement, 단계화된 Storage/API scope, 단계화된 projection/template scope, 실제 보장 수준에 맞는 security wording, agent context strategy, 단계화되고 future-oriented인 conformance fixture plan, 단계화된 operations surface, 한국어 사용자 대상 문서 가독성, link/TODO/terminology 정리가 포함됩니다.

이 최종 리뷰도 편집 리뷰입니다. Maintainer handoff에 사용할 만큼 문서가 일관적인지 요약합니다. Runtime conformance, 기준 상태, evidence, QA, Acceptance, 잔여 위험 수용, close readiness, implementation readiness를 만들지 않습니다. Finding을 기록할 때는 기존 docs-maintenance reporting expectation을 사용하며, 이 최종 pass를 위한 새 필수 report format을 만들지 않습니다.

Docs-maintenance review 또는 future checker는 다음 항목을 보고해야 합니다.

- 항목 범주: 문서 drift, 스키마/설계 결정, 단계 경계 결정, 구현 준비 조건, 향후 로드맵 항목
- result: `PASS`, `WARN`, 또는 `FAIL`
- file path
- 가능한 경우 heading 또는 anchor
- owner 문서와 expected source section
- observed drift
- suggested fix
- runtime effect note: 없음; 기준 상태 전이나 runtime fixture result가 기록되지 않았음
- finding에 추가 맥락이 필요할 때의 maintenance note

Finding이 문서 검토에는 막힘이 아니지만 구현 계획이나 서버 코딩 전에는 막힘이라면, maintenance note에 두 부분을 모두 명시합니다.

Drift는 다음 순서로 해결합니다.

1. Exact contract의 owner 문서 또는 owner section을 식별합니다.
2. Contract 자체가 틀렸거나 불완전하면 owner를 먼저 업데이트합니다.
3. Owner가 아닌 중복 contract는 짧은 독자 중심 요약과 owner link로 바꿉니다.
4. 영어/한국어 의미 변경은 같은 batch에서 paired file에 반영합니다.
5. Owner boundary가 분명해진 뒤 link, anchor, TODO metadata, glossary phrasing을 고칩니다.

Result 의미:

| Result | Meaning |
|---|---|
| `FAIL` | 깨진 owner 링크, schema/DDL/enum/stable event/`ValidatorResult`/`ProjectionKind` 불일치, 대응되는 활성 파일 누락, 의미상 같은 섹션 범위 누락, owner 계약을 다시 정의하는 owner가 아닌 문서의 본문처럼 활성 문서를 모순되거나 실행하기 어렵게 만들 수 있는 drift입니다. 문서가 독자 상황을 돕고, 필요한 맥락이 있으며, owner link와 stable identifier가 유지되고, 정확한 계약이 owner 문서에 남아 있고, 검토 가능성이 분명하다면 자연스러운 heading text, Learn/Use 문서의 사용자 흐름 우선 도입부, 작은 묶음 차이는 실패가 아닙니다. |
| `WARN` | 작은 용어집 표현 차이, 규범적이지 않은 중복 설명문, 영향을 받는 단계가 명확한 오래된 교차 참조 문구, incomplete하지만 이해 가능한 TODO metadata처럼 정리해야 하지만 아직 owner 계약과 모순되지는 않는 drift입니다. |
| `PASS` | 해당 category에서 relevant drift가 발견되지 않았습니다. |

필수 점검 범주:

| 범주 | 필수 점검 |
|---|---|
| 영어/한국어 파일 구조 일치 | 명시적인 예외가 문서화되지 않는 한 `docs/en`과 `docs/ko`는 같은 활성 문서 경로, README entry, paired route expectation을 유지합니다. |
| 영어/한국어 의미 섹션 일치 | 대응 파일은 같은 활성 파일 맵, 독자 목적, 의미상 같은 섹션 범위, owner link, 계약 세부사항을 유지합니다. Stable identifier, schema name, enum value, DDL name, validator ID, code identifier, 검토 가능성이 분명하다면 heading text와 작은 묶음 방식은 자연스럽게 조정할 수 있습니다. |
| 시작 방식 준수 | 활성 문서는 시작 부분에서 독자의 다음 행동을 분명히 보여줍니다. Reference, Build, Maintain 문서는 구조화된 시작 방식을 쓸 수 있고, Learn/Use 문서는 사용자 흐름 우선 도입부를 쓸 수 있습니다. 예전 네 heading 이름이 없다는 이유만으로 Learn/Use 문서를 실패로 보지 않습니다. `docs/*/reference/templates/README.md`는 `사용 시점`, 산출물 계층, 템플릿 구현 계층을 사용하고, `docs/*/reference/templates/` 아래의 `README.md`가 아닌 개별 template file은 `사용 시점`, 구현 계층, `기준 기록`, `렌더링 섹션`, `전체 템플릿`과 명확한 권한 없음 경계를 사용합니다. |
| 깨진 교차 참조 탐지 | Markdown links, heading anchors, template/reference links, same-language README routes, paired-language entry links, owner-section links가 활성 문서와 현재 anchor로 연결됩니다. |
| Owner 경계 불일치 | 정확한 계약과 active owner concept은 활성 owner 문서에 머뭅니다. 여기에는 `reference/kernel.md`, `reference/mcp-api-and-schemas.md`, `reference/storage-and-ddl.md`, `reference/document-projection.md`, `reference/templates/*.md`, `reference/design-quality-policies.md`, `reference/security-threat-model.md`, `reference/operations-and-conformance.md`, `reference/conformance-fixtures.md`, `reference/future-fixture-catalog.md`, `reference/glossary.md`가 포함됩니다. Owner가 아닌 문서는 이 contract를 다시 정의하지 않고 요약하고 link합니다. |
| Fixture/action schema 불일치 | `reference/future-fixture-catalog.md`의 exact-shape 또는 example-shaped example을 포함한 conformance fixture examples의 `action`과 실행 가능한 `input`은 `reference/mcp-api-and-schemas.md`의 public MCP request schemas 및 `reference/conformance-fixtures.md`의 `ToolEnvelope` expansion convention과 일치해야 합니다. Future catalog entry는 정확한 fixture body, public MCP schema, DDL, stage exit, runtime readiness를 다시 정의하면 안 됩니다. Docs-maintenance는 drift를 flag할 수 있지만 fixture action을 실행하거나 fixture 의미를 여기서 다시 설명하지 않습니다. |
| Enum, event, validator, projection 불일치 | State/gate/result values와 Kernel Stable Event Catalog names는 `reference/kernel.md`, error, stable `ValidatorResult` IDs, `ProjectionKind` 값, API 소유 ProjectionKind 지원 계층은 `reference/mcp-api-and-schemas.md`, storage values는 `reference/storage-and-ddl.md`, 템플릿 구현 계층과 projection 최신성 동작은 `reference/document-projection.md`, 렌더링된 template ownership은 `reference/templates/*.md`와 일치해야 합니다. |
| Glossary와 기준 기록 표현 불일치 | 공식 용어, 대소문자, record ID prefixes, source-of-truth wording, authority-boundary phrases는 `reference/glossary.md`와 관련 담당 문서에 맞아야 하며 추가 상태 권한을 암시하지 않아야 합니다. |
| TODO 준수 | `TODO_DECISION`과 `TODO_IMPLEMENT`는 허용된 의미로 쓰고 gap을 명확히 이름 붙이며, action에 필요한 owner/context를 충분히 포함하고, 완료된 기준 섹션에 `TODO_REWRITE` marker를 남기지 않습니다. |
| Owner가 아닌 문서의 중복 전체 계약 | Owner doc 밖의 전체 schema, DDL, transition table, fixture mini-language, template body, enum table, validator table, projection table, glossary definition은 짧은 요약과 owner link로 바꿉니다. Fixture 관련 내용은 정확한 mechanics는 `reference/conformance-fixtures.md`로, 향후 상세 catalog content는 `reference/future-fixture-catalog.md`로 연결합니다. |

## 리뷰 체크리스트

```text
[ ] 이 문서는 분명한 독자 상황을 돕는가?
[ ] README 진입점이 처음 읽는 사람, 사용자, 구현자, Reference 독자, 유지보수 담당자를 빠르게 안내하는가?
[ ] 시작부가 구조화된 방식, 사용자 흐름 우선 방식, 템플릿 전용 방식 중 알맞은 방식으로 독자의 다음 행동을 분명히 보여주는가?
[ ] 개념을 엄격한 정의보다 예시로 먼저 소개하는가?
[ ] strict schema, gate, DDL, enum, invariant가 Reference 문서에 머무는가?
[ ] 긴 기준 기록 문단과 중복된 규범 계약 블록을 반복하지 않고 요약과 링크로 처리했는가?
[ ] diagram이 인지 부담을 줄이는가?
[ ] 영어와 한국어 파일이 의미상 일치하는가?
[ ] official identifier가 정확히 보존되었는가?
[ ] renamed path, anchor, README link가 양쪽 언어에서 업데이트되었는가?
[ ] 현재 사실과 migration history가 분리되어 있는가?
[ ] Maintain 문서가 runtime behavior가 아니라 documentation governance에 머무는가?
```

## Reference ownership map

넓은 문서 routing을 판단할 때 이 map을 사용합니다. Strict Reference contract는 위의 [Reference 계약 owner 지도](#reference-계약-owner-지도)를 사용합니다. 이 table은 현재 문서 구조의 active owner를 식별하며, 비활성 path가 authoring workflow에 남지 않게 합니다.

| Subject | Active owner |
|---|---|
| Repo와 docs 진입점, reader routes, language choice, document list, target tree summary | repo root `README.md`; docs root `docs/README.md`; language entrypoints `docs/en/README.md`와 `docs/ko/README.md` |
| Shared reader mental model and three-space overview | `learn/overview.md` |
| Fast first-reader practical tour and short usage scenarios | `learn/harness-in-15-minutes.md` |
| Small core concept introduction | `learn/concepts.md` |
| Project purpose, target users, values, scope, non-goals, automation philosophy | `learn/purpose-and-principles.md` |
| Strategic thesis, failure model, MVP boundary, principle groups | 독자 설명은 `learn/purpose-and-principles.md`; exact contract impact는 `reference/design-quality-policies.md`와 `reference/kernel.md` |
| Kernel entities, lifecycle, gates, state transitions, close semantics, `prepare_write`, `close_task` | `reference/kernel.md` |
| Runtime architecture, three spaces in implementation detail, Core process model, artifact architecture, projection/reconcile architecture, guarantee-level display placement | `reference/runtime-architecture.md` |
| Security asset, trust boundary, threat category, control category, guarantee-level 의미, high-risk cooperative/detective/preventive/isolated security expectation | Threat concept과 honest guarantee display는 `reference/security-threat-model.md`가 담당하고, exact enforcement, API, storage, kernel, connector, operations, conformance behavior는 각 owner에 남습니다. |
| MCP resources/tools, request/response schemas, error taxonomy, validator result schema, artifact ref shape | `reference/mcp-api-and-schemas.md` |
| SQLite DDL, migrations, storage layout, lock policy, artifact directory layout, baseline capture format, projection job table | `reference/storage-and-ddl.md` |
| MVP implementation order and stage exit criteria | `build/mvp-plan.md` |
| First runnable implementation slice | `build/first-runnable-slice.md` |
| Markdown으로 렌더링되는 projection 원칙, authority matrix, managed blocks, human-editable sections, artifact 참조 표시, 산출물 계층, 템플릿 구현 계층, projection freshness/failure rules | `reference/document-projection.md` |
| 모든 projection template 본문과 표시 카드 형태 | `reference/templates/*.md` |
| 설계 품질 정책 계약, validator ID, severity composition 규칙, 정책 waiver 의미, 근거 기대사항, close 영향 | `reference/design-quality-policies.md` |
| User-facing conversation, status reading, user judgments, close checklist | `use/user-guide.md` |
| 실용 Decision Packet 예시와 사용자 대상 판단 요청 패턴 | 예시는 `use/decision-packet-cookbook.md`; exact Decision Packet behavior는 `reference/kernel.md`와 `reference/mcp-api-and-schemas.md` |
| User/agent session procedure | `use/agent-session-flow.md` |
| Agent 접점 capability profiles, 공통 커넥터 계약, fallback 의미, Role Lens, connector conformance 개요 | `reference/agent-integration.md` |
| 접점별 recipes | `reference/surface-cookbook.md` |
| Generic capability profile examples | `reference/agent-integration.md` |
| Operator procedures, conformance run overview, doctor/recover/reconcile/export/artifact integrity, docs-maintenance 보고 | `reference/operations-and-conformance.md` |
| 핵심 적합성 모델, 정확한 fixture body, runner execution, assertion semantics, 현재 단계 상태, 검증 프로파일별 증명 동작, suite metadata boundary, 축소된 Kernel Smoke 작성 순서 | `reference/conformance-fixtures.md` |
| 향후 상세 scenario 후보, concern별 향후 fixture example, staged fixture coverage map, fixture suite family summary, catalog-only future candidate | `reference/future-fixture-catalog.md` |
| Official term definitions and capitalization | `reference/glossary.md` |
| v1+ Expansion roadmap | `roadmap.md` |
| Documentation authoring rules | `maintain/authoring-guide.md` |
| Translation and bilingual prose rules | `maintain/translation-guide.md` |
