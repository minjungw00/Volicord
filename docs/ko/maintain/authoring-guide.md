# 문서 작성 가이드

하네스 문서를 고치기 전에 이 가이드를 사용합니다. 이 문서는 문서 유지보수만 다룹니다. 하네스 서버/런타임 구현, 제품 저장소 쓰기, 생성된 운영 산출물, conformance runner, 런타임 상태, projection, evidence record, QA record, Acceptance record, close record, Residual Risk record를 허가하지 않습니다.

이 저장소는 아직 문서 검토와 재설계 단계입니다. 현재 문서는 재설계 이후 검토 기준입니다. 구현 준비가 수락된 서버 계획이 아닙니다. 정리된 제품 명제, owner 경계, 한국어 품질 규칙, 구현 가능성과 충돌하는 오래된 문장은 과감하게 다시 쓰거나 옮기거나 줄이거나 삭제할 수 있습니다.

## 필수 사전 편집 체크리스트

- [ ] 먼저 root `AGENTS.md`를 읽습니다.
- [ ] 문서를 편집하기 전에 이 가이드를 읽습니다.
- [ ] 이중 언어 편집이나 용어에 영향을 주는 편집이면 `docs/en/maintain/translation-guide.md`를 읽습니다.
- [ ] 한국어 문서를 건드리기 전에는 `docs/ko/maintain/authoring-guide.md`와 `docs/ko/maintain/translation-guide.md`를 읽습니다.
- [ ] 작업이 문서 전용인지 확인합니다. Server/runtime code, product code, generated operational file, runtime state, executable fixture, conformance runner, projection, artifact output을 만들지 않습니다.
- [ ] 편집할 문서군을 확인합니다. Learn, Use, Build, Reference, Maintain, Later, Roadmap 중 어디인지 먼저 정합니다.
- [ ] 엄격한 계약을 건드릴 수 있다면 owner 문서를 찾습니다. 하나의 strict contract에는 하나의 owner 문서만 있습니다.
- [ ] 이번 편집이 의미 변경인지, 문장 정리인지, 링크 수정인지, 이름 변경/이동인지, 삭제인지 구분합니다.
- [ ] 의미가 바뀌면 `docs/en`을 먼저 고치고 같은 batch에서 `docs/ko`에 같은 의미를 반영합니다.
- [ ] 한국어를 고칠 때는 exact identifier를 보존하고 자연스러운 한국어 기술 문장으로 씁니다. 영어 문장을 줄 단위로 따라 하지 않습니다.
- [ ] 사용자 대상 문서는 사용자가 보는 상황에서 시작합니다. `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, `task_events` 같은 내부 라벨로 시작하지 않습니다.
- [ ] Stage나 MVP 표현을 고치면 관련 Build owner에서 active stage boundary를 확인합니다. Later-profile이나 Roadmap 내용을 MVP 요구사항처럼 쓰지 않습니다.
- [ ] 보안 표현을 고치면 실제 guarantee level을 먼저 확인합니다.
- [ ] 문서를 옮기거나, 이름을 바꾸거나, 나누거나, 합치거나, 삭제한다면 양쪽 언어의 link와 anchor 수정까지 계획합니다.

## 필수 사후 편집 체크리스트

- [ ] Server/runtime implementation, product code, generated operational artifact, runtime state, executable fixture, conformance runner, projection, artifact output을 만들지 않았는지 확인합니다.
- [ ] 문서 파일을 source material로 설명했는지 확인합니다. Runtime state, generated projection, evidence, QA, Acceptance, residual-risk record, close record, operational truth처럼 쓰지 않습니다.
- [ ] Strict schema, DDL, enum value, state transition, gate rule, algorithm, fixture body shape, template body, storage rule, security guarantee, official definition이 owner 문서에 남아 있는지 확인합니다.
- [ ] Owner가 아닌 문서의 중복 contract는 짧은 독자용 요약과 owner link로 바꿉니다.
- [ ] 사용자 대상 문서가 평소 사용자 상황, 에이전트가 구체화해야 할 것, 보이는 막힘, 필요한 판단, close 결과를 먼저 설명하는지 확인합니다.
- [ ] Stage 표현이 구현 상태를 과장하지 않는지, later-profile이나 Roadmap 내용을 MVP 요구사항으로 만들지 않는지 확인합니다.
- [ ] 보안 표현이 문서화된 guarantee level과 맞는지 확인합니다. OS permission, arbitrary-tool sandboxing, tamper-proof file, pre-tool blocking, isolation을 정확한 증명 없이 암시하지 않습니다.
- [ ] 영어/한국어 대응 문서가 같은 의미, owner link, stable identifier, active file coverage를 유지하는지 확인합니다.
- [ ] 한국어 문장이 자연스러운지 확인합니다. Exact identifier, file path, schema/API name, enum value, error code, validator ID, code-like string은 정확히 보존합니다.
- [ ] 이동, 이름 변경, 분리, 병합, 삭제가 있었다면 link, anchor, README route, paired-language link, 예전 title/path reference를 같은 batch에서 고쳤는지 확인합니다.
- [ ] 로컬에서 할 수 있는 docs-maintenance check를 실행합니다. 실행하지 못한 check는 handoff에 적습니다.
- [ ] 남은 문제는 문서 drift, schema/design decision, stage boundary decision, implementation-readiness criterion, future Roadmap item 중 하나로 라우팅합니다. Active docs 곳곳에 막연한 TODO를 남기지 않습니다.
- [ ] 변경한 파일과 남은 위험 또는 확인하지 못한 check를 handoff에 보고합니다.

## 재설계 중 보존할 핵심 원칙

용어, schema, 문서 구조, stage boundary는 바뀔 수 있습니다. 그래도 아래 원칙은 유지합니다.

- 하네스는 prompt 묶음이 아닙니다. Scope, 사용자 소유 판단, 근거, 검증, QA 기대치, 작업 수락, 잔여 위험 상태, 닫기 준비 상태를 다루는 local authority record입니다.
- 사용자 소유 판단은 사용자에게 남습니다. Product decision, material technical decision, QA expectation, waiver, work acceptance, residual-risk acceptance를 agent에게 조용히 넘기지 않습니다.
- Evidence, Verification, Manual QA, work acceptance, close readiness, residual risk는 서로 다른 기록과 판단입니다. 서로를 대신하지 않습니다.
- Chat, connector output, Markdown-rendered projection, generated document는 operational truth가 아닙니다. 향후 운영 기준은 Core-owned local state와 artifact reference입니다.
- 문서 파일은 하네스를 이해하고 구현하기 위한 source material입니다. Runtime object, runtime state, generated artifact, projection, evidence, QA, Acceptance, close, residual-risk record가 아닙니다.
- 현재 문서는 review baseline입니다. Maintainer handoff owner가 명시하지 않는 한 fully accepted, implementation-complete, implementation-ready, server-coding-ready라고 쓰지 않습니다.

재설계 중에는 기존 문구 보존보다 명확성, 구현 가능성, 제품 명제를 우선합니다. Harness가 broad workflow engine, ALM system, evaluation harness, QA automation platform, report generator, generic MCP wrapper, prompt pack처럼 보이게 하는 문장은 다시 쓰거나 옮기거나 줄이거나 삭제합니다.

## 문서군 소유권 규칙

문서 tree는 소유권을 나누기 위한 구조입니다.

| 문서군 | 역할 | 경계 |
|---|---|---|
| Learn | 하네스가 왜 필요한지, 개념이 무슨 뜻인지 설명합니다. | Strict schema, gate, DDL, 구현 순서, fixture mechanics를 정의하지 않습니다. |
| Use | 사용자와 agent가 하네스와 어떻게 상호작용하는지 설명합니다. | 사용자 신뢰, 보이는 막힘, 판단 요청, 근거 gap, 복구 경로, close 결과를 이해하는 데 필요할 때만 low-level contract를 이름 붙입니다. |
| Build | 문서 수락과 별도의 구현 계획 준비 결정 이후의 구현 순서를 설명합니다. | Stage goal, 순서, runnable-slice planning, exit criteria를 둡니다. Exact schema, gate, DDL, API, storage, fixture, security contract는 Reference로 연결합니다. |
| Reference | Exact contract, schema, algorithm, storage, API, security model, projection behavior, official definition을 소유합니다. | Contract를 이해하는 데 필요한 맥락은 둡니다. Tutorial, reader journey, staged implementation plan으로 만들지 않습니다. |
| Maintain | 문서 유지보수를 관리합니다. | Authoring, translation, review, link, ownership, docs-maintenance rule만 정의합니다. Runtime behavior, runtime conformance pass/fail, implementation readiness를 정의하지 않습니다. |
| Later와 Roadmap | Active MVP path 밖의 future/candidate material을 둡니다. | Owner 문서가 scope, fallback behavior, proof expectation과 함께 승격하기 전까지 active stage requirement처럼 쓰지 않습니다. |

README 문서는 긴 설명서이기 전에 길잡이입니다. 하네스가 무엇이고 무엇이 아닌지 짧게 말한 뒤 처음 읽는 사람, 사용자, 구현자, Reference 독자, 유지보수 담당자를 알맞은 owner 문서로 보냅니다.

## 계약 하나에는 owner 하나 규칙

모든 strict contract에는 owner 문서가 하나만 있습니다. Exact field, enum value, DDL, schema, algorithm, state transition, gate rule, fixture body shape, template body, storage rule, security guarantee, error precedence, official definition은 그 owner 문서에서만 정의합니다.

다른 문서군은 독자에게 보이는 결과를 설명하고 owner로 연결할 수 있습니다. 두 번째 정의를 만들면 안 됩니다. Local 설명에 full table, schema block, DDL block, transition matrix, fixture mini-language, gate matrix, validator table, template body, algorithm이 필요하다면 그 내용은 owner Reference 문서에 둡니다.

Owner가 아닌 문서에서 규범 문구가 반복되면 아래 순서로 고칩니다.

1. Owner 문서 또는 owner section을 찾습니다.
2. Contract 자체가 틀렸거나 빠졌다면 owner를 먼저 고칩니다.
3. 중복 문구는 평범한 한 문장 요약, owner link, 현재 독자에게 필요한 local consequence로 바꿉니다.
4. 의미가 바뀌면 paired language file에도 반영합니다.
5. Owner boundary가 분명해진 뒤 link와 anchor를 고칩니다.

### Reference 계약 owner 지도

엄격한 규칙을 추가하기 전에 이 지도를 봅니다.

| 계약 영역 | Owner 문서 | Owner 경계 |
|---|---|---|
| Core Model | [Core Model 참조](../reference/core-model.md) | Invariant, 상태에 영향을 주는 entity 관계 의미, lifecycle과 상태 전이, gate, `prepare_write`, Write Authorization, `record_run`, close semantics, waiver, 대체 불가능한 경계. |
| MCP API | [MVP API](../reference/api/mvp-api.md), [API Schema Core](../reference/api/schema-core.md), [API Errors](../reference/api/errors.md), [API Schema Later](../reference/api/schema-later.md) | Active MVP-1 tool, public MCP resource, common envelope, request/response schema, shared ref, public error, idempotency/replay, state conflict behavior, `ValidatorResult`, API `ArtifactRef`, later-profile API material. |
| Storage | [Storage](../reference/storage.md) | Runtime home layout, persisted state, SQLite DDL profile, storage-owned JSON `TEXT`, enum hardening, migration, lock, artifact storage, baseline capture, projection job table, validator-run storage. |
| Projection | [Projection과 Template 참조](../reference/projection-and-templates.md)와 [Template 참조](../reference/templates/README.md) | Derived view rule, output tier, managed block, human-editable section, artifact-ref rendering, projection freshness/failure behavior, 전체 rendered template body. |
| Security | [보안 참조](../reference/security.md) | Threat model, asset, trust boundary, threat/control category, high-risk control expectation, local access security posture, guarantee-level 의미와 honest-display rule. |
| Conformance | [Conformance Fixtures 참조](../reference/conformance-fixtures.md)와 [향후 Fixtures](../later/future-fixtures.md) | Conformance Fixtures는 세 층 경계, core conformance model, MVP behavior example, future fixture body shape, future runner behavior, assertion semantics, future fixture profile, suite metadata boundary, current-phase status, reduced Kernel Smoke queue를 담당합니다. Future Fixtures는 active MVP path 밖의 future scenario candidate, future fixture example, staged coverage map, suite family summary, catalog-only future candidate를 담당합니다. |
| Operations | [운영과 Conformance 참조](../reference/operations-and-conformance.md) | Operator behavior, staged operator surface, diagnostic, `connect`, `doctor`, `serve mcp`, projection refresh, reconcile, recover, export, artifact check, future conformance run entrypoint, documentation-check/docs-maintenance reporting boundary. |
| Agent Integration | [Agent 통합 참조](../reference/agent-integration.md)와 [Surface Cookbook](../reference/surface-cookbook.md) | Connector capability profile, generated manifest, context push/pull profile, fallback semantics, Role Lens, reference-surface behavior, connector conformance overview, surface-specific recipe. |
| Glossary | [용어집 참조](../reference/glossary.md) | Public/internal terminology definition, capitalization, official term wording, record-name orientation, owner routing. |
| Runtime Architecture | [런타임 아키텍처 참조](../reference/runtime-architecture.md) | 세 공간, Core process placement, Core-only canonical mutation authority, transaction ordering, artifact/projection/reconcile placement, architecture-level failure and recovery overview. |
| Design Quality | [설계 품질 정책](../reference/design-quality-policies.md) | Policy contract, policy-to-validator mapping, stable validator ID, severity composition, policy waiver semantics, evidence expectation, design-quality gate/close impact. |

## Stage와 MVP 경계 규칙

문서 검토 상태, 구현 계획 준비 상태, 런타임 구현 상태를 분리합니다.

- 문서 검토 상태: 현재 문서 세트는 재설계 이후 검토 상태입니다. Maintainer review를 위한 문서 수락 후보입니다.
- 구현 계획 준비 상태: 아직 수락되지 않았습니다. 첫 runtime-batch planning 전에 maintainer가 implementation-readiness criteria를 명시적으로 확인해야 합니다.
- 런타임 구현 상태: 시작하지 않았습니다. 이 저장소는 지금 문서 전용입니다. 향후 역할은 하네스 서버 소스 저장소이지만, server/runtime implementation은 문서 수락과 별도의 구현 계획 준비 결정 이후에만 시작할 수 있습니다.

Active delivery label은 일관되게 씁니다.

| 라벨 | 경계 |
|---|---|
| 내부 엔지니어링 점검(Engineering Checkpoint) | 내부 authority-loop smoke입니다. Product MVP도 아니고 첫 사용자 가치 slice도 아닙니다. |
| Kernel Smoke | 내부 엔지니어링 점검 아래의 좁은 future smoke-check 작성 label입니다. Stage name이 아닙니다. |
| MVP-1 사용자 작업 루프(MVP-1 User Work Loop) | 첫 좁은 사용자 가치 milestone입니다. |
| 보증 프로필(Assurance Profile) | Agency assurance behavior를 나중에 단단하게 만드는 범위입니다. |
| 운영 프로필(Operations Profile) | Operations와 handoff behavior를 나중에 단단하게 만드는 범위입니다. |
| 로드맵(Roadmap) | Owner 문서가 승격하고 증명하기 전까지 future scope입니다. |

Later-profile, diagnostic, operations, conformance-runner, Roadmap material을 MVP requirement처럼 쓰지 않습니다. Reference schema에 존재한다는 사실만으로 smallest runnable slice가 커지지 않습니다. Required field는 owning tool, record, profile이 active이거나 사용될 때 적용됩니다.

Review 중 발견한 큰 구현 결정은 [MVP-1 사용자 작업 루프 계획: 서버 코딩 전 필요한 구현 결정](../build/mvp-user-work-loop.md#서버-코딩-전-필요한-구현-결정)에 둡니다. Active docs 곳곳에 큰 결정을 `TODO_DECISION`으로 흩어 놓지 않습니다. 짧은 maintainer handoff status는 [구현 개요: 문서 인계 요약](../build/implementation-overview.md#문서-인계-요약)이 담당합니다.

이 저장소의 문서 편집에는 하네스 runtime procedure가 필요하지 않습니다. Docs work를 위해 `prepare_write`, MCP state transition, `close_task`, runtime state, `task_events`, Write Authorization, Evidence Manifest, Manual QA record, Acceptance record, Residual Risk record, Journey Card, generated projection, generated operational/projection document를 실행하거나 흉내 내지 않습니다.

## 사용자 대상 용어 규칙

사용자 대상 문서는 내부 라벨이 아니라 사용자가 보는 상황에서 시작합니다.

사용자가 무엇을 요청할 수 있는지, agent가 무엇을 구체화해야 하는지, 하네스가 무엇을 보존하는지, 무엇이 막혔는지, 어떤 판단이 필요한지, 어떤 근거가 있는지, close가 무엇을 뜻하는지 먼저 씁니다. 내부 용어는 쉬운 상황이 분명해진 뒤에만 소개합니다. 독자가 행동하거나, 보이는 막힘을 해석하거나, Reference link를 따라가는 데 도움이 될 때만 씁니다.

사용자가 `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, `task_events` 같은 라벨을 알아야 하거나 말해야 진행되는 것처럼 쓰지 않습니다. 아래처럼 평소 말을 예시로 둡니다.

```text
구현 전에 계획을 구체화해줘.
내가 결정해야 할 것과 네가 확인할 수 있는 것을 나눠서 보여줘.
작업 범위가 커지면 먼저 알려줘.
```

개념은 엄격한 정의보다 예시로 먼저 소개합니다. Learn이나 Use 문서를 조밀한 정의 목록으로 시작하지 않습니다. Glossary나 reference table이면 예외입니다.

Use 문서는 user trust boundary에 머뭅니다. 사용자가 보는 hold, blocker, decision prompt, evidence gap, close result, recovery path를 설명해야 할 때 contract를 이름 붙일 수 있습니다. 하지만 판단에 필요하지 않은 field list, storage row, gate matrix, validator internal detail은 드러내지 않습니다.

## 보안 표현 규칙

보안 표현은 실제 문서화된 guarantee level과 맞아야 합니다.

- Cooperative 표현은 하네스가 행동을 안내하거나 기록할 수 있지만 기술적으로 막을 수는 없을 때 씁니다.
- Detective 표현은 하네스가 행동 이후 감지하거나 보고할 수 있을 때 씁니다.
- Preventive 표현은 해당 surface가 covered action 전에 막을 수 있고 그 blocking path가 해당 operation에 대해 증명되어 있을 때만 씁니다.
- Isolated 표현은 문서화된 separation boundary가 있을 때만 씁니다. 그 boundary를 이름 붙이고 더 넓은 OS sandboxing이나 permission isolation을 암시하지 않습니다.

초기 하네스가 OS-level permission, arbitrary-tool sandboxing, tamper-proof local file, pre-tool blocking, security isolation을 제공한다고 암시하지 않습니다. 해당 operation에 대한 정확한 mechanism이 문서화되고 증명되어 있을 때만 말합니다. Write Authorization은 cooperative Harness record/check입니다. OS permission, sandboxing, tamper-proof enforcement, preventive blocking, isolation이 아닙니다.

[보안 참조](../reference/security.md)는 threat concept과 honest guarantee display를 담당합니다. Exact API, storage, Core, connector, operations, conformance behavior는 각 owner에 남습니다.

## 이중 언어 의미 일치 규칙

영어 문서는 이중 언어 문서 세트의 기준 의미를 정의합니다. 한국어 문서는 그 의미를 보존하되 자연스러운 한국어 기술 문서처럼 읽혀야 합니다.

- 영어/한국어 대응 문서는 같은 active file map, reader purpose, semantic section coverage, owner link, contractual detail을 유지합니다.
- `docs/en`의 의미가 바뀌면 같은 batch에서 `docs/ko`에 반영합니다. 반대 방향도 같습니다.
- 한국어가 더 명확하다면 heading text와 paragraph grouping은 달라도 됩니다. Reviewability는 유지해야 합니다.
- API name, schema name, enum value, DDL name, code identifier, field name, file name, path name, stable identifier, error code, validator ID, code-like string은 정확히 보존합니다.
- 한국어 사용자용 prose에서는 자연스러운 한국어를 먼저 씁니다. Stable English identifier는 인식, 검색, exact contract alignment에 필요할 때만 괄호에 둡니다.
- 한국어 문장은 필요한 곳에서 짧게 끊습니다. 영어 명사 여러 개에 조사만 붙인 문장으로 만들지 않습니다.

자세한 용어 규칙은 [영어 번역 가이드](../../en/maintain/translation-guide.md)와 [한국어 번역 가이드](translation-guide.md)를 봅니다.

## 링크, 이름 변경, 삭제 위생

문서를 옮기거나, 이름을 바꾸거나, 나누거나, 합치거나, 삭제하면 양쪽 언어의 링크를 같은 batch에서 고칩니다.

편집 전에 old path, old anchor, old heading, old title text, README route, owner link, template/reference link, paired-language link를 확인합니다. 편집 뒤에는 다시 검색해서 active reference를 모두 고칩니다.

2차 요약보다 owner document나 owner section으로 링크합니다. Active owner link가 removed migration context, inactive path, old structure를 가리키면 안 됩니다.

예전 이름, 예전 구조, migration decision을 리뷰 목적으로 남겨야 한다면 명확히 non-active migration record라고 표시한 곳에 둡니다. Active docs는 현재 구조를 설명하고 current owner로 연결합니다.

삭제할 때는 해당 내용이 obsolete인지, owner 밖 중복인지, 다른 owner로 옮겨졌는지, future Roadmap material인지 먼저 정합니다. Active reference는 새 owner link로 바꾸거나 같은 batch에서 제거합니다. Dangling anchor나 stale route text를 남기지 않습니다.

## 알려진 재설계 위험과 회귀 점검

이 section은 재설계 중 자주 돌아오는 drift를 확인하는 실행 가능한 checklist입니다. 예전 known-issue tracker를 대체합니다. 아래 항목은 documentation-maintenance risk입니다. Runtime implementation task, runtime conformance, acceptance record, implementation readiness proof가 아닙니다.

Risk가 확인되면 아래 category 중 하나로 라우팅합니다. Affected owner, stage, 필요한 action을 함께 이름 붙입니다.

| Category | 사용하는 경우 |
|---|---|
| Documentation drift | 필요한 조치가 wording, owner-boundary cleanup, link repair, open-marker hygiene, terminology, 영어/한국어 의미 일치일 때. |
| Schema/design decision | Schema, state, API, DDL, security guarantee, fixture semantics, 그 밖의 owner contract에서 실제 선택이 필요할 때. |
| Stage boundary decision | Capability가 Engineering Checkpoint, MVP-1 User Work Loop, Assurance Profile, Operations Profile, Roadmap 중 어디에 속하는지 결정해야 할 때. |
| Implementation-readiness criterion | 첫 runtime-batch planning을 수락하기 전에 maintainer가 확인해야 하는 조건일 때. |
| Future Roadmap item | 유용하지만 승격되기 전까지 Engineering Checkpoint부터 Operations Profile 밖에 남아야 하는 항목일 때. |

Finding을 non-blocking이라고 부르려면 어떤 stage에는 막힘이 아닌지, 어떤 later stage에는 막힘이 될 수 있는지 함께 씁니다. Implementation-readiness concern을 막연한 follow-up 표현으로 숨기지 않습니다.

| 재설계 위험 | 회귀 점검 | 확인되었을 때 기본 route |
|---|---|---|
| 저장소 정체성이 "이미 구현이 있다"는 식으로 흐려집니다. | Entrypoint와 handoff section은 현재 repo가 documentation-only이고, post-redesign review 상태이며, future Harness Server source repository이고, documentation acceptance와 별도 implementation-planning readiness decision 없이는 server/runtime implementation을 시작할 수 없다고 말합니다. | Implementation-readiness criterion |
| Stage 이름이 Engineering Checkpoint나 Kernel Smoke를 product MVP처럼 보이게 합니다. | Engineering Checkpoint는 internal authority-loop smoke, Kernel Smoke는 좁은 future smoke-check authoring label, MVP-1 User Work Loop는 첫 user-value slice로 설명합니다. | Stage boundary decision |
| 사용자 대상 문서가 무거운 구현 disclaimer로 시작합니다. | Learn/Use opening은 사용자가 요청할 수 있는 것, agent가 구체화하는 것, 하네스가 보존하는 것, 사용자가 보게 되는 것을 먼저 보여줍니다. 상세 상태 경고는 README, Build handoff, Maintain docs로 보냅니다. | Documentation drift |
| 사용자 대상 문서가 내부 용어를 과도하게 씁니다. | User-visible situation을 `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, `task_events`보다 먼저 씁니다. | Documentation drift |
| Discovery가 너무 빨리 Change Unit으로 수렴합니다. | Requirements clarification은 scoped implementation unit을 요구하기 전에 shared understanding과 user-owned judgment의 여지를 둡니다. | Stage boundary decision |
| Judgment terminology가 legacy alias를 current axis처럼 되살립니다. | Active owner docs는 `UserJudgment` / `user_judgment`, `harness.request_user_judgment`, `judgment_type`, `presentation`, `display_label`을 우선합니다. Legacy alias는 compatibility-only로 남깁니다. | Schema/design decision |
| Decision Packet 예시가 작은 판단에 비해 너무 무겁습니다. | 작은 판단은 `presentation=short`를 쓰고 한 화면에 들어갑니다. Full-format Decision Packet presentation은 optional, later-profile, complex judgment용입니다. | Schema/design decision |
| Sensitive-action Approval, work acceptance, residual-risk acceptance가 섞입니다. | 이름 붙은 민감 동작 permission, 작업 결과 수락, 보이는 남은 위험 수용을 예시와 routing에서 분리합니다. | Schema/design decision |
| Storage, API, DDL reference material이 wording drift 때문에 early-stage requirement가 됩니다. | Reference schema 존재와 staged implementation requirement를 분리합니다. Required field는 owning tool, record, profile이 active이거나 사용될 때만 적용됩니다. | Stage boundary decision |
| Conformance fixture 문서가 지금 executable suite가 있는 것처럼 보입니다. | Fixture 문서는 future-oriented와 staged 상태를 유지합니다. MVP behavior example을 runnable fixture file이나 current pass/fail criteria라고 부르지 않습니다. | Implementation-readiness criterion |
| Operations entrypoint가 너무 이른 필수 요소처럼 보입니다. | 관련 Build stage가 명시적으로 포함하기 전까지 operator entrypoint는 staged/future-oriented로 둡니다. | Stage boundary decision |
| 한국어 사용자용 문서에 영어 기술 명사가 쌓입니다. | Natural public concept을 먼저 쓰고, exact English identifier는 precision, searchability, contract alignment가 필요할 때만 유지합니다. | Documentation drift |
| Decision-ledger status가 readiness를 과장합니다. | Entrypoint, handoff, 이 가이드는 MVP-1 decision ledger와 맞아야 하며, documentation acceptance를 implementation-planning readiness나 server-coding authorization으로 바꾸지 않습니다. | Implementation-readiness criterion |
| MVP scope가 커지고 핵심 사용자 가치가 뒤로 밀립니다. | Build docs는 그 tension을 이름 붙이고 staging decision을 Build/Reference owner에 남깁니다. | Stage boundary decision |
| Projection/template scope가 early implementation에 비해 너무 넓어집니다. | Projection/template docs는 active early scope와 later display/export/reporting candidate를 분리합니다. | Stage boundary decision |
| Security wording이 enforcement를 과장합니다. | Cooperative, detective, preventive, isolated claim은 documented mechanism과 proof level에 맞습니다. | Schema/design decision |
| Agent context strategy가 prompt를 과도하게 채웁니다. | Always-on context는 최신 상태로 한 화면 이하에 둡니다. Detailed contract는 owner docs나 retrieval path로 보냅니다. | Implementation-readiness criterion |
| 문서가 runtime state처럼 읽힙니다. | Docs는 source material입니다. Runtime object, generated projection, operational record, conformance artifact가 아닙니다. | Documentation drift |
| Roadmap candidate가 active delivery로 흘러들어옵니다. | Roadmap item은 owner가 scope, fallback behavior, proof expectation, no projection-as-canonical dependency와 함께 승격하기 전까지 Roadmap에 둡니다. | Future Roadmap item |

### Docs-maintenance checks

Docs-maintenance checks는 편집 품질 점검입니다. Documentation drift, owner mismatch, link integrity 문제, terminology consistency 문제, stage-boundary drift, security wording drift, user-language issue, 영어/한국어 의미 일치 문제, owner 밖 중복 규범 문구, 깨진 link나 anchor, open-marker hygiene 문제를 보고할 수 있습니다.

Docs-maintenance는 runtime conformance나 implementation readiness가 아닙니다. Fixture action을 실행하지 않습니다. Runtime state를 seed하지 않습니다. Runtime state/events/artifacts/projections/errors를 비교하지 않습니다. Runtime fixture pass/fail에 포함되지 않습니다. Canonical state, runtime state, `task_events`, evidence artifact, Evidence Manifest, QA result, Manual QA record, Acceptance record, residual-risk acceptance, Residual Risk record, close readiness, projection refresh, generated conformance artifact, generated operational report, implementation readiness를 만들거나 갱신하지 않습니다.

Docs-maintenance review 또는 future checker는 다음을 보고합니다.

- item category: documentation drift, schema/design decision, stage boundary decision, implementation-readiness criterion, future Roadmap item
- result: `PASS`, `WARN`, `FAIL`
- file path
- 가능한 경우 heading 또는 anchor
- owner document와 expected source section
- observed drift
- suggested fix
- runtime effect note: 없음. Canonical state transition이나 runtime fixture result는 기록되지 않았음
- finding에 추가 맥락이 필요한 경우 maintenance note

Result 의미는 다음과 같습니다.

| Result | Meaning |
|---|---|
| `FAIL` | Broken owner link, schema/DDL/enum/stable event/`ValidatorResult`/`ProjectionKind` mismatch, paired active file 누락, semantic section coverage 누락, owner contract를 다시 정의하는 non-owner text처럼 active docs를 모순되거나 실행하기 어렵게 만드는 drift입니다. |
| `WARN` | 정리해야 하지만 아직 owner contract와 모순되지는 않는 drift입니다. Minor glossary phrasing drift, normative하지 않은 duplicate explanatory prose, affected stage가 명확한 stale cross-reference wording, 이해 가능한 incomplete open-marker metadata가 여기에 속합니다. |
| `PASS` | 해당 category에서 relevant drift가 발견되지 않았습니다. |

필수 check category:

| Category | Required check |
|---|---|
| English/Korean file structure parity | 명시적인 예외가 문서화되지 않는 한 `docs/en`과 `docs/ko`는 같은 active document path, README entry, paired route expectation을 유지합니다. |
| English/Korean semantic section parity | Paired file은 같은 reader purpose, semantic section coverage, owner link, stable identifier, contractual detail을 유지합니다. |
| Opening convention compliance | Active docs는 시작 부분에서 reader의 next useful step을 분명히 보여줍니다. Learn/Use는 workflow-first opening을 쓸 수 있고, Reference/Build/Maintain은 structured opening을 쓸 수 있으며, template reference는 template-specific opening을 씁니다. |
| Broken cross-reference detection | Markdown link, heading anchor, template/reference link, same-language README route, paired-language entry link, owner-section link가 active docs와 current anchor로 연결됩니다. |
| Owner-boundary drift | Exact contract와 active owner concept은 active owner 문서에 남습니다. Non-owner docs는 다시 정의하지 않고 요약과 link로 처리합니다. |
| Fixture/action schema drift | Future runtime fixture example의 `action`과 future executable `input`은 public MCP request schema, shared API schema, `ToolEnvelope` expansion convention과 일치합니다. MVP behavior example은 아직 executable fixture가 아닙니다. |
| Enum, event, validator, projection drift | State/gate/result value, event name, error value, stable validator ID, `ProjectionKind` value, storage value, template implementation class, projection freshness behavior가 owner와 일치합니다. |
| Glossary and source-of-truth phrasing drift | Official term, capitalization, record ID prefix, source-of-truth wording, authority-boundary phrase가 `reference/glossary.md`와 관련 owner docs에 맞습니다. |
| Open-marker hygiene | `TODO_DECISION`과 `TODO_IMPLEMENT`는 허용된 의미로 쓰고 gap을 분명히 이름 붙이며, action에 필요한 owner/context를 포함하고, finished canonical section에 `TODO_REWRITE` marker를 남기지 않습니다. |
| Non-owner duplicate full contracts | Owner 밖의 full schema, DDL, transition table, fixture mini-language, template body, enum table, validator table, projection table, glossary definition은 짧은 summary와 owner link로 바꿉니다. |

### 최종 사전 수락 리뷰

Maintainer가 문서 세트를 implementation planning에 사용할 수 있다고 받아들이기 전, 마지막 docs-maintenance pass를 수행합니다. English/Korean active file map parity, paired file의 semantic section parity, broken link와 anchor, owner-boundary drift, non-owner duplicate contract, Approval, Decision Packet, Evidence, Verification, Manual QA, Acceptance, Residual Risk, Projection, Guarantee Level terminology drift, open-marker hygiene를 확인합니다.

[구현 개요](../build/implementation-overview.md#하네스-서버-구현-준비-조건)의 implementation-readiness criteria도 확인합니다. Repository identity, 내부 용어 부담 없는 user-facing flow, premature Change Unit convergence가 아닌 requirements clarification으로서의 Discovery, canonical `user_judgment` naming과 mapped legacy alias, proportional judgment prompt, Approval/work acceptance/residual-risk acceptance separation, coherent stage, Core Model/API/storage/reference agreement, staged Storage/API scope, staged projection/template scope, honest security guarantee wording, agent context strategy, staged future-oriented conformance fixture plan, staged operations surface, Korean user-facing readability, clean links/terminology/open markers를 봅니다.

이 최종 리뷰도 editorial review입니다. Maintainer handoff에 사용할 만큼 문서가 일관적인지 요약할 뿐입니다. Runtime conformance, canonical state, evidence, QA, Acceptance, residual-risk acceptance, close readiness, implementation readiness를 만들지 않습니다.

## 상세 참고 자료

Checklist 항목에 더 자세한 기준이 필요할 때 이 section을 봅니다. 두 번째 필수 workflow로 취급하지 않습니다.

### 문서 시작 방식

모든 active document는 시작 부분에서 reader path를 짧고 분명하게 보여줘야 합니다.

Reference, Build, Maintain 문서는 독자에게 도움이 될 때 아래 구조를 쓸 수 있습니다.

- `이 문서로 할 수 있는 일`: 독자에게 주는 결과를 평범한 말로 씁니다.
- `이런 때 읽기`: 이 문서를 읽어야 하는 상황을 적습니다.
- `읽기 전에`: 필요한 맥락, 먼저 읽을 문서, 전제 조건을 적습니다.
- `핵심 생각`: 나머지를 이해하는 중심 모델이나 주장을 먼저 알려줍니다.

Learn/Use 문서는 평소 요청, 실용 예시, 사용자 흐름으로 시작할 수 있습니다. Workflow-first opening은 사용자가 무엇을 요청할 수 있는지, agent가 무엇을 구체화하는지, 하네스가 무엇을 보존하는지, 사용자가 무엇을 보게 되는지, exact owner detail이 어디에 있는지 보여줘야 합니다.

문서가 독자 상황을 돕고, 필요한 맥락이 있으며, owner link가 유지되고, exact contract detail이 Reference owner에 남아 있고, 영어/한국어가 의미상 일치한다면 heading text 차이는 drift가 아닙니다.

### 템플릿 참조 시작 방식

Template reference file은 별도 시작 방식을 씁니다. Docs-maintenance는 directory index인 `docs/*/reference/templates/README.md`와 `docs/*/reference/templates/` 아래 README가 아닌 individual template file을 경로로 구분합니다.

Directory README는 `사용 시점`으로 시작한 뒤 output tier와 template implementation class를 둡니다. 이 directory가 rendered template body와 display card shape를 담당하고, projection rule, freshness behavior, authority boundary는 각 Reference owner에 남는다는 점을 설명합니다.

Individual template file은 아래 section으로 시작합니다. 순서를 지킵니다.

- `사용 시점`
- `기준 기록`
- `렌더링 섹션`
- `전체 템플릿`

Template file은 non-authority boundary를 분명히 보여줘야 합니다. Template은 rendered display일 뿐입니다. Canonical state, gate authority, approval, acceptance, evidence, schema, DDL, runtime behavior가 아닙니다.

### Conformance와 fixture layering

세 층을 일관되게 사용합니다.

- Documentation checks: Markdown docs에 대한 editorial checks입니다. Runtime conformance가 아니며 generated operational artifact나 conformance artifact를 만들지 않습니다.
- MVP behavior examples: Engineering Checkpoint와 MVP-1의 작은 design example입니다. Expected behavior를 설명하지만 아직 executable fixture가 아니며 generated runtime artifact도 아니고 current pass/fail criteria도 아닙니다.
- Runtime conformance: 향후 server implementation test와 runner입니다. Exact-shape fixture를 materialize하고 Core 또는 operator action을 실행한 뒤 captured state, event, artifact, projection/freshness fact, error를 비교합니다.

MVP behavior example을 runnable suite라고 부르지 않습니다. Documentation check가 runtime conformance를 pass/fail한다고 말하지 않습니다. Documentation work 중 fixture file, conformance report, runtime state, projection, operational artifact를 만들지 않습니다.

### 반복 규칙

긴 source-of-truth 문단을 여러 문서에 반복하지 않습니다. 같은 생각이 다른 문서에 필요하면 짧게 요약하고 owner로 link합니다. Source text가 바뀌면 owner를 먼저 고친 뒤 summary drift를 확인합니다.

독자가 다른 예시를 필요로 하면 explanatory example은 반복할 수 있습니다. Normative contract wording 반복은 drift risk입니다.

반복되기 쉬운 경계는 아래 owner를 사용합니다.

| 경계 | 정확한 문구의 owner |
|---|---|
| Context Index와 retrieved/indexed context | Future feature boundary는 [로드맵: 후보 항목 목록](../roadmap.md#후보-항목-목록), connector context handling은 [Agent Integration: Context Push/Pull Principles](../reference/agent-integration.md#context-pushpull-principles) |
| Local Derived Metrics | [로드맵: 후보 항목 목록](../roadmap.md#후보-항목-목록) |
| Role Lens | [Agent Integration: Role Lens 동작](../reference/agent-integration.md#role-lens-동작) |
| Review Stages | [Design Quality Policies: Two-stage Review Display](../reference/design-quality-policies.md#two-stage-review-display) |
| Release Handoff와 export | [Operations And Conformance: Release Handoff Export Profile](../reference/operations-and-conformance.md#release-handoff-export-profile); rendered shape은 later-profile [EXPORT Template](../reference/templates/later-profile/export.md) |
| Docs-maintenance | Rule body는 [Authoring Guide: Docs-maintenance checks](#docs-maintenance-checks), operator reporting은 [Operations And Conformance: docs-maintenance profile](../reference/operations-and-conformance.md#docs-maintenance-프로필) |
| Projection과 report surface | [Projection And Templates Reference](../reference/projection-and-templates.md), rendered shape은 [Template Reference](../reference/templates/README.md) |
| Security asset, trust boundary, threat category, control category, guarantee-level meaning, high-risk cooperative/detective/preventive/isolated security expectation | Threat concept과 honest guarantee display는 [보안 참조](../reference/security.md)가 담당합니다. Exact API, storage, Core, connector, operations, conformance behavior는 각 owner에 남습니다. |

### Owner 링크 요약 패턴

Owner 밖에서 중복된 normative language를 찾으면 그 문장을 그대로 다듬지 않습니다. 아래 형태를 씁니다.

```text
제품 파일 쓰기에는 현재 Change Unit 범위와 Write Authorization이 필요합니다. 정확한 write-gate 동작은 [Core Model 참조](../reference/core-model.md)가 담당하고, public request shape은 [MVP API](../reference/api/mvp-api.md)가 담당합니다.
```

Gate matrix, request schema, DDL block, fixture body, template body, enum table, validator table, projection table, glossary definition을 Learn, Use, Build, Maintain 문서에 붙여 넣지 않습니다.

### 다이어그램 규칙

Diagram은 인지 부담을 줄일 때만 사용합니다. 관계, 순서, 경계, lifecycle이 prose보다 그림으로 더 분명할 때 유용합니다.

모든 diagram 근처에는 무엇을 봐야 하는지 알려주는 prose가 있어야 합니다. Diagram과 prose가 다르면 owner prose나 Reference contract를 먼저 고칩니다.

### Reference ownership map

넓은 문서 routing에는 이 map을 사용합니다. Strict Reference contract는 [Reference 계약 owner 지도](#reference-계약-owner-지도)를 사용합니다.

| Subject | Active owner |
|---|---|
| Repo와 docs entrypoint, reader route, language choice, document list, target tree summary | repo root `README.md`; docs root `docs/README.md`; language entrypoint `docs/en/README.md`와 `docs/ko/README.md` |
| Shared reader mental model과 three-space overview | `learn/overview.md` |
| Fast first-reader practical tour와 short usage scenario | `learn/harness-in-15-minutes.md` |
| Small core concept introduction | `learn/concepts.md` |
| Project purpose, target users, values, scope, non-goals, automation philosophy | `learn/purpose-and-principles.md` |
| Strategic thesis, failure model, MVP boundary, principle groups | Reader explanation은 `learn/purpose-and-principles.md`; exact contract impact는 `reference/design-quality-policies.md`와 `reference/core-model.md` |
| Core entity, lifecycle, gate, state transition, close semantics, `prepare_write`, `close_task` | `reference/core-model.md` |
| Runtime architecture, implementation detail의 세 공간, Core process model, artifact architecture, projection/reconcile architecture, guarantee-level display placement | `reference/runtime-architecture.md` |
| Security asset, trust boundary, threat category, control category, guarantee-level meaning, high-risk cooperative/detective/preventive/isolated security expectation | Threat concept과 honest guarantee display는 `reference/security.md`; exact enforcement, API, storage, Core, connector, operations, conformance behavior는 각 owner |
| MCP resource/tool, request/response schema, error taxonomy, validator result schema, artifact ref shape | `reference/api/mvp-api.md`, `reference/api/schema-core.md`, `reference/api/errors.md`, `reference/api/schema-later.md` |
| SQLite DDL, migration, storage layout, lock policy, artifact directory layout, baseline capture format, projection job table | `reference/storage.md` |
| MVP implementation order와 stage exit criteria | `build/mvp-user-work-loop.md` |
| First runnable implementation slice | `build/engineering-checkpoint.md` |
| Markdown-rendered projection principle, authority matrix, managed block, human-editable section, artifact reference rendering, output tier, template implementation class, projection freshness/failure rule | `reference/projection-and-templates.md` |
| 모든 projection template body와 display card shape | `reference/templates/*.md` |
| Design-quality policy contract, validator, severity composition, waiver semantics, evidence expectation, close impact | `reference/design-quality-policies.md` |
| User-facing conversation, status reading, user judgment, close checklist | `use/user-guide.md` |
| Practical user-owned judgment example과 user-facing judgment request pattern | Example은 `use/decision-packet-cookbook.md`; exact user judgment behavior는 `reference/core-model.md`와 `reference/api/mvp-api.md` |
| User/agent session procedure | `use/agent-guide.md` |
| Agent surface capability profile, common connector contract, fallback semantics, Role Lens, connector conformance overview | `reference/agent-integration.md` |
| Surface-specific recipe | `reference/surface-cookbook.md` |
| Generic capability profile example | `reference/agent-integration.md` |
| Operator procedure, future conformance run overview, doctor/recover/reconcile/export/artifact integrity, documentation-check/docs-maintenance reporting | `reference/operations-and-conformance.md` |
| Core conformance model, MVP behavior example, exact future fixture body shape, future runner execution, assertion semantics, current-phase status, future fixture profile by proven behavior, suite metadata boundary, reduced Kernel Smoke authoring queue | `reference/conformance-fixtures.md` |
| Active MVP path 밖의 detailed future scenario candidate, concern별 future fixture example, staged fixture coverage map, fixture suite family summary, catalog-only future candidate | `later/future-fixtures.md` |
| Official term definition과 capitalization | `reference/glossary.md` |
| Roadmap candidate | `roadmap.md` |
| Documentation authoring rule | `maintain/authoring-guide.md` |
| Translation과 bilingual prose rule | `maintain/translation-guide.md` |
| Rewrite planning category와 redesign triage | `maintain/rewrite-plan.md` |
