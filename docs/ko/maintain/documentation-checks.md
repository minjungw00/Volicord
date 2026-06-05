# 문서 점검표

최종 문서 수락 전이나 큰 리뷰 인계 전에 이 점검표를 사용합니다. Markdown 문서만 보는 실무용 docs-maintenance 점검표입니다. 즉 읽기 전용 문서 품질 점검 profile입니다.

이 점검표는 runtime conformance suite가 아닙니다. Fixture를 실행하지 않습니다. Runtime state를 seed하지 않습니다. Runtime state/events/artifacts/projections/errors를 비교하지 않습니다. `task_events`를 append하지 않습니다. artifact를 만들지 않고, projection을 refresh하지 않으며, generated operational artifact나 conformance report를 만들지 않습니다. QA 또는 acceptance state를 만들지 않습니다. 근거, QA, Acceptance, Residual Risk, close를 기록하지 않습니다. close readiness에 영향을 주지 않고 implementation readiness도 증명하지 않습니다.

docs-maintenance의 `PASS`, `WARN`, `FAIL` label은 manual review가 다음에 볼 것과 고칠 것을 정하는 데 도움이 될 수 있습니다. 하지만 manual acceptance, final acceptance, close readiness, implementation readiness, runtime fixture result가 아닙니다.

runtime conformance는 별도입니다. 구현된 Core/API/storage/surface behavior에만 적용되며, documentation prose가 아니라 실행 가능한 fixture와 state assertion으로 판단합니다. Runtime implementation과 materialized fixture suite가 생기기 전에는 runtime conformance result를 암시하면 안 됩니다.

## 점검 유형

결과를 보고할 때 아래 라벨을 사용합니다.

| 점검 유형 | 의미 |
|---|---|
| `manual` | 리뷰어 판단이 필요합니다. 검색 도구로 후보를 모을 수 있지만 script-only pass로 충분하지 않습니다. |
| `scriptable` | 로컬 문서 script나 parser가 해당 조건을 직접 확인할 수 있습니다. 문서화된 예외는 리뷰어가 확인합니다. |
| `future-runtime-only` | 향후 runtime implementation과 그 증명 경로가 있어야 확인할 수 있습니다. 현재 문서 리뷰에서는 docs가 과장하지 않는지만 확인합니다. |

## 점검표

### 링크 점검

- 점검 유형: `scriptable`.
- 볼 것: Active docs의 relative Markdown link, README route, paired-language link, owner-section link, heading anchor.
- 자주 실패하는 예: 이동된 file을 가리킵니다. 예전 heading anchor가 남아 있습니다. 영어 문서가 실수로 한국어 전용 anchor를 가리킵니다. README route가 삭제되었거나 inactive인 page를 가리킵니다.
- 통과 의미: 모든 relative link와 anchor가 active document나 명시된 예외로 연결됩니다. Owner link는 현재 owner document나 owner section으로 갑니다.

### 용어 점검

- 점검 유형: `manual`.
- 볼 것: Learn과 Use 문서의 예시, 제목, 요약, 상태 설명에서 internal label이 기본 사용자 언어처럼 쓰이는지 봅니다.
- 자주 실패하는 예: 사용자용 문서가 평소 사용자 상황을 설명하기 전에 `Discovery`, `Change Unit`, `Decision Packet`, `Write Authorization`, `Evidence Manifest`, `Projection`, `Gate`, `task_events`로 시작합니다. 사용자가 내부 라벨을 말해야 도움을 받을 수 있는 것처럼 보입니다.
- 통과 의미: 사용자용 prose는 평소 말에서 시작합니다. 내부 라벨은 보이는 경계, 막힘, record, API, template, Reference link를 설명할 때만 씁니다.

### 단계 점검

- 점검 유형: `manual`.
- 볼 것: Build, Reference, Use, Roadmap 문서의 MVP-1, Engineering Checkpoint, Kernel Smoke, Assurance Profile, Operations Profile, Later, Roadmap 표현.
- 자주 실패하는 예: Roadmap candidate가 MVP-1 requirement처럼 쓰입니다. `Kernel Smoke`가 stage처럼 쓰입니다. Later-profile export, reporting, operations, conformance-runner 내용이 smallest runnable slice의 필수 조건이 됩니다.
- 통과 의미: Engineering Checkpoint는 internal authority-loop smoke로 남습니다. MVP-1 User Work Loop는 첫 사용자 가치 milestone으로 남습니다. Later-profile과 Roadmap material은 owner가 scope, fallback behavior, proof expectation과 함께 승격하기 전까지 future scope입니다.

### 상태 점검

- 점검 유형: `manual`.
- 볼 것: Entrypoint, handoff section, Build 문서, Maintain 문서, 현재 구현 상태를 암시할 수 있는 문장.
- 자주 실패하는 예: 이 repo에 Harness Server가 이미 있다고 말합니다. Documentation acceptance가 server-coding authorization처럼 쓰입니다. Reference design prose가 future 또는 design 경계 없이 구현된 runtime behavior처럼 보입니다.
- 통과 의미: Docs는 현재 repo가 documentation-only이고 post-redesign review 상태라고 말합니다. Maintainer handoff owner가 명시하지 않는 한 implementation-ready가 아닙니다. 의도한 future behavior와 implemented behavior가 구분됩니다.

### 보안 표현 점검

- 점검 유형: 문서 표현은 `manual`. 실제 preventive 또는 isolated enforcement 증명은 `future-runtime-only`.
- 볼 것: Cooperative, detective, preventive, isolated, guard, freeze, careful-mode, sandbox, permission, blocking, tamper-proof, isolation 표현.
- 자주 실패하는 예: Write Authorization을 OS permission, sandboxing, tamper-proof enforcement, preventive blocking, isolation처럼 설명합니다. 증명된 blocking path 없이 connector가 arbitrary tool call을 막는다고 말합니다. Security boundary가 owner 문서보다 넓은 OS isolation을 암시합니다.
- 통과 의미: 모든 claim이 문서화된 guarantee level과 맞습니다. Cooperative나 detective surface가 preventive control을 주장하지 않습니다. Preventive나 isolated claim은 covered operation, mechanism, owner document, proof status를 이름 붙이거나 future-oriented로 둡니다.

### 사용자 언어 점검

- 점검 유형: `manual`.
- 볼 것: 사용자용 문서의 opening, 예시, 사용자가 말할 수 있는 요청, 상태 설명, 판단 질문, close 설명, recovery text.
- 자주 실패하는 예: Use 문서가 사용자가 할 수 있는 요청보다 record taxonomy로 시작합니다. 판단 질문이 선택과 결과보다 `Decision Packet`을 먼저 보여줍니다. Status view 설명이 보이는 요약보다 `ProjectionKind`를 먼저 말합니다.
- 통과 의미: 사용자 문서는 평소 작업, 질문, 보이는 막힘, 필요한 판단, 있는 근거, close 결과에서 시작합니다. 내부 라벨은 그 라벨이 해결하는 문제가 먼저 보인 뒤 소개합니다.

### Mermaid 점검

- 점검 유형: Mermaid parser가 있으면 문법은 `scriptable`. 유용성은 `manual`.
- 볼 것: Mermaid fenced code block, 주변 설명, diagram label, owner prose와의 일치.
- 자주 실패하는 예: Mermaid syntax가 render되지 않습니다. Diagram이 장식일 뿐 관계, 순서, 경계, lifecycle을 더 분명하게 하지 않습니다. Diagram이 주변 prose나 owner contract와 다릅니다.
- 통과 의미: Diagram은 문법상 합리적이고 예상 docs toolchain에서 render될 수 있습니다. 독자 부담을 줄일 만큼 유용합니다. 주변 prose가 무엇을 봐야 하는지 알려줍니다.

### 이중 언어 점검

- 점검 유형: `manual`.
- 볼 것: 영어/한국어 active file map, paired file purpose, section coverage, owner link, stable identifier, exact code-like string, 한국어 prose 품질.
- 자주 실패하는 예: 영어에 추가된 section이 한국어에서 빠졌습니다. Path, enum value, error code, validator ID, API name이 번역되거나 바뀝니다. 한국어 문장이 영어 기술 명사에 조사만 붙인 형태가 됩니다. Paired link가 다른 owner를 가리킵니다.
- 통과 의미: 영어와 한국어 문서가 같은 의미, coverage, owner routing, exact identifier를 보존합니다. 한국어 제목과 문단은 자연스럽고 의미가 같다면 영어와 달라도 됩니다.

### Owner 점검

- 점검 유형: `manual`.
- 볼 것: Strict contract, schema, DDL, enum value, state transition, gate rule, algorithm, fixture body shape, template body, storage rule, security guarantee, official definition.
- 자주 실패하는 예: Use 문서가 full gate matrix를 반복합니다. Build 문서가 Reference owner의 enum table을 정의합니다. Maintain 문서가 projection freshness를 두 번째 규범 정의로 설명합니다. Glossary owner 밖에서 glossary definition을 복사해 바꿉니다.
- 통과 의미: Strict contract 하나는 owner 문서 하나에서 정의합니다. Owner가 아닌 문서는 짧은 독자용 요약, local consequence, owner link를 둡니다.

### 정비 대상 owner 지도 점검

- 점검 유형: `manual`.
- 볼 것: [문서 작성 가이드: 사전 구현 문서 정비 대상 owner 지도](authoring-guide.md#사전-구현-문서-정비-대상-owner-지도)의 알려진 사전 구현 정비 축을 봅니다. Owner contract, API/schema, Storage/DDL, Core transition, stage/profile, evidence/close, security/local-access, conformance proof, user-output/context, design-quality drift가 포함됩니다.
- 자주 실패하는 예: Later-profile API branch가 MVP requirement처럼 쓰입니다. Status card가 gate authority처럼 다뤄집니다. Design-quality validator가 owner activation rule 밖에서 blocker가 됩니다. Documentation check가 runtime conformance처럼 설명됩니다. 보안 문구가 증명된 owner path 없이 pre-tool blocking을 주장합니다.
- 통과 의미: 관찰된 정비 축이 기준 owner 문서군으로 라우팅됩니다. Owner가 아닌 문서는 짧은 local summary와 owner link만 둡니다. 표의 `FAIL` 증상은 docs-maintenance 실패로만 보고합니다. 이 점검은 문서 수락, manual acceptance, runtime conformance, implementation readiness를 결정하지 않습니다.

### Projection/상태 점검

- 점검 유형: `manual`.
- 볼 것: Projection, rendered template, Markdown status view, generated document, state, artifact, evidence, QA, Acceptance, close readiness, operational truth 관련 표현.
- 자주 실패하는 예: Markdown으로 렌더링된 view를 canonical state라고 부릅니다. 문서 파일을 runtime object, generated projection, evidence record, QA record, Acceptance record, Residual Risk record, close record, operational artifact처럼 다룹니다. Projection을 gate authority처럼 설명합니다.
- 통과 의미: Rendered view와 generated document는 derived display로 설명합니다. 향후 operational authority는 관련 Reference owner가 정의한 Core-owned local state와 artifact reference에 남습니다.

### Template scope 점검

- 점검 유형: `manual`.
- 볼 것: Template reference, projection/template page, Use 문서, Build 문서, Later 문서, future template이나 rendered output을 언급하는 Roadmap item.
- 자주 실패하는 예: Later-profile export template이 MVP-1 close에 필요하다고 합니다. Future template body가 current MVP requirement처럼 쓰입니다. 사용자용 문서가 Template Reference owner로 link하지 않고 full template body를 중복합니다.
- 통과 의미: Future template은 owner가 승격하기 전까지 future 또는 later-profile material로 남습니다. Active MVP requirement는 active stage에 필요한 template과 rendered view만 이름 붙입니다. Full template body는 Template Reference owner에 남습니다.
