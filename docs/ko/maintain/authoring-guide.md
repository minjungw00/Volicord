# 문서 작성 가이드

## 이 문서로 할 수 있는 일

Harness 문서를 새로 쓰거나, 나누거나, 이름을 바꾸거나, 리뷰할 때 이 가이드를 사용합니다.

목표는 현재 문서가 독자에게 읽기 쉽고, 세부 계약의 위치가 분명하며, 영어와 한국어 문서가 같은 의미를 유지하도록 돕는 것입니다.

이 문서는 Maintain 문서입니다. 문서 유지보수만 다루며, 문서 세트가 구현 계획에 사용할 수 있다고 승인되기 전에는 runtime/server 구현, 제품 상태 변경, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data, 근거 기록, QA 결과, 수락 결정, Task 닫기를 승인하거나 대체하지 않습니다. 첫 구현/증명 대상은 계속 Kernel Smoke입니다. Agency-Hardened MVP와 post-MVP automation은 owner 문서가 승격하고 증명하기 전까지 범위 밖입니다.

## 이런 때 읽기

- 문서를 추가, 분리, 이름 변경, review할 때.
- 어떤 문서가 strict contract를 소유하는지 판단해야 할 때.
- 영어/한국어 의미 일치, link, TODO hygiene, duplicate owner text를 확인할 때.

## 먼저 읽기

정확한 runtime contract는 아래에 연결된 Reference owner 문서를 사용합니다. 한국어 표현 규칙은 [번역 가이드](translation-guide.md)를 사용합니다.

## 핵심 생각

각 문서는 독자에게 유용해야 하며 exact contract는 owner Reference 문서에 머물러야 합니다. Docs-maintenance checks는 drift를 보이게 하지만 runtime state, evidence, QA, acceptance, close readiness, implementation readiness를 만들지 않습니다.

## 문서 작성 원칙

문서는 독자의 다음 행동에서 출발합니다. 독자가 무엇을 이해하고, 결정하고, 사용하고, 구현하고, 검증하고, 유지해야 하는지 분명해야 합니다.

내부 구조를 빠짐없이 나열하기보다 핵심 아이디어를 적게, 선명하게 설명합니다. 엄격한 계약은 Reference 문서로 보내고, 다른 문서에서는 필요한 만큼만 요약한 뒤 링크합니다.

낯선 개념은 정의부터 던지지 않습니다. 먼저 실제 상황이나 짧은 예시로 왜 필요한지 보여주고, 그다음 이름과 정의를 붙입니다.

각 문서의 시작은 예측 가능해야 합니다. 독자가 이 문서로 무엇을 할 수 있는지, 언제 읽어야 하는지, 무엇을 먼저 알아야 하는지, 전체를 묶는 핵심 생각이 무엇인지 빨리 알 수 있어야 합니다.

현재 문서는 현재의 사실처럼 씁니다. 마이그레이션 과정, 제거된 구조, 예전 이름은 본문 설명에 넣지 않습니다. 마이그레이션 중 별도 migration note가 있는 경우에만 그곳에 두고, 그렇지 않으면 Git history나 명확히 표시된 비활성 마이그레이션 기록에 남깁니다.

## 문서 유형

### Learn

Learn 문서는 독자의 이해 모델을 만듭니다.

목적, 개념, 예시, 절충점을 구현 세부사항보다 먼저 설명합니다. 독자에게 명령, 스키마, 체크리스트보다 방향 감각이 필요할 때 사용합니다.

### Use

Use 문서는 사용자가 AI 지원 개발 세션에서 Harness를 따라가도록 돕습니다.

사용자에게 보이는 흐름, 상태 해석, 결정 지점, 복구 경로를 중심에 둡니다. 내부 gate는 사용자가 보는 막힘이나 next action을 설명할 때만 이름 붙입니다.

### Build

Build 문서는 문서 세트가 구현 계획에 사용할 수 있다고 승인된 뒤 reference system을 구현하는 사람을 돕습니다.

구현 순서, module 경계, 실행 가능한 조각, 검증 전략을 설명합니다. 정확한 스키마, DDL, 불변 조건은 Reference 문서로 연결합니다.

### Reference

Reference 문서는 정확한 계약을 담당합니다.

엄격한 스키마, gate, DDL, enum value, state transition, 불변 조건, API shape, storage rule, projection rule, fixture 의미, 공식 정의는 Reference 문서에 둡니다.

### Maintain

Maintain 문서는 문서 시스템 자체를 관리합니다.

작성 규칙, 번역 정책, 리뷰 체크리스트, link hygiene, ownership map, documentation-maintenance expectation을 정의합니다. Maintain 문서가 런타임 conformance spec이나 product implementation plan이 되면 안 됩니다.

## 진입점 규칙

README 문서는 긴 설명서이기 전에 길잡이입니다. Harness가 무엇이고 무엇이 아닌지 짧게 말한 뒤, 처음 읽는 사람, 사용자, 구현자, Reference 독자, 유지보수 담당자를 알맞은 owner 문서로 빠르게 안내해야 합니다.

진입점은 현재 구조를 작고 명확하게 보여줘야 합니다. 명확히 비활성 migration record라고 표시한 섹션이 아니라면 migration history, 제거된 이름, 비활성 path, 예전 구조를 보존하는 장소로 쓰지 않습니다.

README 문서는 경로별 소유권을 요약할 수 있지만 엄격한 계약을 복사하면 안 됩니다. 정확한 schema, DDL, gate, state transition, fixture 의미, template 본문, 공식 정의는 Reference owner로 연결합니다.

## 문서 시작 방식

활성 문서는 짧고 예측 가능한 시작부를 둡니다. 길게 설명하지 않더라도 독자의 경로가 보여야 합니다. `reference/templates` 아래의 템플릿 참조 파일은 일반 시작 heading 대신 아래의 템플릿 전용 시작 방식을 사용합니다.

### 이 문서로 할 수 있는 일

문서가 독자에게 주는 결과를 평범한 말로 씁니다. 무엇을 "다룬다"는 설명만으로 끝내지 않습니다.

### 이런 때 읽기

이 문서를 읽어야 하는 상황을 적습니다. 짧은 목록이어도 됩니다.

### 읽기 전에

필요한 사전 맥락, 먼저 읽을 문서, 전제 조건을 적습니다. 전제 조건이 없다면 간단히 없다고 말합니다.

### 핵심 생각

나머지 내용을 이해하기 쉽게 만드는 중심 모델이나 주장을 먼저 알려줍니다.

### Reference 범위

Reference 문서에만 둡니다. 이 문서가 어떤 정확한 계약을 담당하고, 무엇을 담당하지 않는지 밝힙니다. 이렇게 해야 Learn, Use, Build 문서로 엄격한 세부사항이 퍼지지 않습니다.

### 템플릿 참조 시작 방식

템플릿 참조 파일은 위 일반 시작 heading의 명시적 예외입니다. Docs-maintenance는 directory index인 `docs/*/reference/templates/README.md`와 개별 template인 `docs/*/reference/templates/` 아래의 README가 아닌 Markdown file을 경로로 구분해야 합니다.

디렉터리 README는 `사용 시점`으로 시작한 뒤 `템플릿 계층`을 둡니다. 이 README는 디렉터리가 렌더링된 template body와 display card shape를 담당하며, projection rule, freshness behavior, authority boundary는 각 Reference owner에 남는다는 점을 설명해야 합니다.

개별 template file은 다음 section을 이 순서로 시작해야 합니다.

- `사용 시점`: 독자 목적과 projection 또는 display 상황.
- `기준 기록`: renderer가 읽을 수 있는 owner record, ref, gate, artifact, summary.
- `렌더링 섹션`: 독자가 기대해야 하는 display shape.
- `전체 템플릿`: 완전한 rendered body 또는 card body.

Template file은 시작 설명이나 `메모` 근처에서 권한 없음 경계를 보여줘야 합니다. Template은 렌더링 표시일 뿐이며 기준 상태, gate authority, Approval, acceptance, evidence, schema, DDL, runtime behavior가 아닙니다.

## 개념 소개 규칙

개념은 엄격한 정의보다 예시로 먼저 소개합니다.

구체적인 상황에서 시작해 어떤 문제를 해결하는지 보여준 뒤 개념 이름을 붙입니다. 독자가 왜 중요한지 본 다음에 엄격한 정의를 둡니다.

좋은 흐름:

```text
Agent가 제품 상태를 변경하려면 Harness는 먼저 그 쓰기가 어떤 범위의 구현 단위에 속하는지 알아야 합니다. 그 단위가 Change Unit입니다. 사용자가 끝내거나 답을 얻고 싶은 더 큰 가치 단위가 Task입니다.
```

Learn 문서를 조밀한 정의 목록으로 시작하지 않습니다. Glossary나 reference table이라면 예외입니다.

## Reference 계약 규칙

엄격한 스키마, gate, DDL, enum value, state transition, 불변 조건, API shape, storage rule, projection contract detail, fixture 의미는 Reference 문서에 둡니다.

Learn, Use, Build, Maintain 문서는 필요할 때 계약을 한두 문장으로 요약하고 owner Reference 문서에 링크합니다. 전체 table, schema body, transition matrix, DDL block, fixture mini-language를 중복하지 않습니다.

Runtime conformance fixture body shape, assertion mode, isolated execution behavior, JSON `TEXT` validation, owner-bound enum/status validation은 [운영과 Conformance](../reference/operations-and-conformance.md#conformance-fixture-format)가 담당합니다. 다른 문서는 conformance가 executable-state-based라는 점만 요약하고 owner로 링크해야 하며, 전체 계약을 다시 적지 않습니다.

## 반복 규칙

긴 기준 기록 문단을 여러 문서에 반복하지 않습니다.

다른 문서에 같은 생각이 필요하면 짧게 요약하고 owner 문서로 링크합니다. 원문이 바뀌면 owner 문서를 먼저 고친 뒤 요약문이 어긋나지 않았는지 확인합니다.

독자가 다른 예시를 필요로 한다면 설명용 예시는 반복할 수 있습니다. 하지만 규범적인 계약 문구를 반복하면 불일치 위험이 큽니다.

반복되기 쉬운 권한 없음 경계는 다음 owner를 사용합니다.

| 경계 | 정확한 문구의 owner |
|---|---|
| Context Index와 retrieved/indexed context | Future feature 경계는 [Roadmap: Context Index](../roadmap.md#context-index), connector context 처리는 [Agent Integration: Context Push/Pull Principles](../reference/agent-integration.md#context-pushpull-principles) |
| Local Derived Metrics | [Roadmap: Local Derived Metrics](../roadmap.md#local-derived-metrics) |
| Role Lens | [Agent Integration: Role Lens 동작](../reference/agent-integration.md#role-lens-동작) |
| Review Stages | [Design Quality Policies: Two-stage Review Display](../reference/design-quality-policies.md#two-stage-review-display) |
| Release Handoff와 export | [Operations And Conformance: Release Handoff Export Profile](../reference/operations-and-conformance.md#release-handoff-export-profile); 렌더링 형태는 [EXPORT Template](../reference/templates/export.md) |
| Docs-maintenance | Rule body는 [Authoring Guide: Docs-maintenance checks](#docs-maintenance-checks), operator 보고는 [Operations And Conformance: docs-maintenance profile](../reference/operations-and-conformance.md#docs-maintenance-프로필) |
| Projection과 report surfaces | [Document Projection Reference](../reference/document-projection.md), 렌더링 형태는 [Template Reference](../reference/templates/README.md) |

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

## 링크와 이름 변경 규칙

문서 이름을 바꾸거나, 옮기거나, 나누거나, 합칠 때는 양쪽 언어의 링크를 같은 batch에서 고칩니다.

2차 요약보다 owner 문서나 owner section으로 링크합니다. Active owner link가 제거된 migration context를 가리키면 안 됩니다.

예전 이름, 예전 구조, migration 결정을 리뷰 목적으로 남겨야 한다면 명확히 비활성 migration record라고 표시한 곳에 둡니다. Active docs는 현재 구조를 설명하고 현재 owner로 연결해야 합니다.

이름 변경 뒤에는 이전 path, 이전 anchor, 이전 heading, 이전 title text를 검색합니다. README path, 주변 cross-reference, template/reference link, paired-language link를 함께 업데이트합니다.

## Docs-maintenance checks

Docs-maintenance checks는 읽기 전용 문서 유지보수입니다. Documentation drift, owner mismatch, 영어/한국어 의미 일치 문제, owner 밖의 중복 규범 문구, 깨진 link나 anchor, TODO hygiene 문제를 보고할 수 있습니다. Core fixture conformance, runtime validator, 기준 상태 전이, projection 새로고침, 생성된 운영 보고서, QA result, 결과 수락 기록, evidence artifact, 남은 위험을 받아들이는 판단, close readiness, 구현 준비 상태가 아닙니다. Fixture action을 실행하거나, runtime state를 seed하거나, runtime state/events/artifacts/projections/errors를 비교하지 않으며, runtime fixture pass/fail에 포함되지 않습니다.

Maintain 문서는 documentation review rule, category label, reviewer expectation을 정의할 수 있습니다. Runtime conformance pass/fail, runtime fixture semantics, Core state effect, gate behavior, implementation readiness를 정의하면 안 됩니다. Docs-maintenance finding이 runtime contract를 건드리면 그 contract를 다시 적지 말고 owner Reference 문서를 가리켜야 합니다.

### 최종 사전 수락 리뷰

Maintainer가 문서 세트를 구현 계획에 사용할 수 있다고 받아들이기 전, 마지막 docs-maintenance pass를 수행합니다. 영어/한국어 활성 파일 맵 일치, 대응 파일의 의미 섹션 일치, 깨진 link와 anchor, owner-boundary drift, owner가 아닌 문서의 중복 contract, Approval, Decision Packet, Evidence, Verification, Manual QA, Acceptance, Residual Risk, Projection, Guarantee Level 용어 drift, TODO hygiene를 확인합니다.

이 최종 리뷰는 문서 유지보수일 뿐입니다. Runtime conformance, evidence, QA, acceptance, residual-risk acceptance, close readiness, implementation readiness, 기준 상태를 만들지 않습니다. Finding을 기록할 때는 기존 docs-maintenance reporting expectation을 사용하며, 이 최종 pass를 위한 새 필수 report format을 만들지 않습니다.

Docs-maintenance review 또는 future checker는 다음 항목을 보고해야 합니다.

- category
- result: `PASS`, `WARN`, 또는 `FAIL`
- file path
- 가능한 경우 heading 또는 anchor
- owner 문서와 expected source section
- observed drift
- suggested fix
- runtime effect statement: none; 기준 상태 전이가 수행되지 않았고 runtime fixture result가 기록되지 않았음

Drift는 다음 순서로 해결합니다.

1. Exact contract의 owner 문서 또는 owner section을 식별합니다.
2. Contract 자체가 틀렸거나 불완전하면 owner를 먼저 업데이트합니다.
3. Owner가 아닌 중복 contract는 짧은 독자 중심 요약과 owner link로 바꿉니다.
4. 영어/한국어 의미 변경은 같은 batch에서 paired file에 반영합니다.
5. Owner boundary가 분명해진 뒤 link, anchor, TODO metadata, glossary phrasing을 고칩니다.

Result 의미:

| Result | Meaning |
|---|---|
| `FAIL` | 깨진 owner 링크, schema/DDL/enum/stable event/`ValidatorResult`/`ProjectionKind` 불일치, 대응되는 활성 파일 누락, 의미상 같은 섹션 범위 누락, owner 계약을 다시 정의하는 owner가 아닌 문서의 본문처럼 활성 문서를 모순되거나 실행하기 어렵게 만들 수 있는 drift입니다. Owner 링크, stable identifier, 검토 가능성이 분명하다면 자연스러운 heading text나 작은 묶음 차이는 실패가 아닙니다. |
| `WARN` | 작은 용어집 표현 차이, 규범적이지 않은 중복 설명문, 오래되었지만 차단적이지 않은 교차 참조 문구, incomplete하지만 이해 가능한 TODO metadata처럼 정리해야 하지만 아직 owner 계약과 모순되지는 않는 drift입니다. |
| `PASS` | 해당 category에서 relevant drift가 발견되지 않았습니다. |

필수 점검 범주:

| 범주 | 필수 점검 |
|---|---|
| 영어/한국어 파일 구조 일치 | 명시적인 예외가 문서화되지 않는 한 `docs/en`과 `docs/ko`는 같은 활성 문서 경로, README entry, paired route expectation을 유지합니다. |
| 영어/한국어 의미 섹션 일치 | 대응 파일은 같은 활성 파일 맵, 독자 목적, 의미상 같은 섹션 범위, owner link, 계약 세부사항을 유지합니다. Stable identifier, schema name, enum value, DDL name, validator ID, code identifier, 검토 가능성이 분명하다면 heading text와 작은 묶음 방식은 자연스럽게 조정할 수 있습니다. |
| 시작 방식 준수 | Template이 아닌 활성 문서는 표준 시작 방식을 사용합니다. `docs/*/reference/templates/README.md`는 `사용 시점`과 `템플릿 계층`을 사용하고, `docs/*/reference/templates/` 아래의 `README.md`가 아닌 개별 template file은 `사용 시점`, `기준 기록`, `렌더링 섹션`, `전체 템플릿`과 명확한 권한 없음 경계를 사용합니다. |
| 깨진 교차 참조 탐지 | Markdown links, heading anchors, template/reference links, same-language README routes, paired-language entry links, owner-section links가 활성 문서와 현재 anchor로 연결됩니다. |
| Owner 경계 불일치 | 정확한 계약은 활성 owner 문서에 머뭅니다. 여기에는 `reference/kernel.md`, `reference/mcp-api-and-schemas.md`, `reference/storage-and-ddl.md`, `reference/document-projection.md`, `reference/templates/*.md`, `reference/design-quality-policies.md`, `reference/operations-and-conformance.md`, `reference/glossary.md`가 포함됩니다. Owner가 아닌 문서는 이 contract를 다시 정의하지 않고 요약하고 link합니다. |
| Fixture/action schema 불일치 | Operations fixture examples의 `action`과 실행 가능한 `input`은 `reference/mcp-api-and-schemas.md`의 public MCP request schemas 및 `reference/operations-and-conformance.md`의 `ToolEnvelope` expansion convention과 일치해야 합니다. Docs-maintenance는 drift를 flag할 수 있지만 fixture action을 실행하거나 fixture 의미를 여기서 다시 설명하지 않습니다. |
| Enum, event, validator, projection 불일치 | State/gate/result values와 Kernel Stable Event Catalog names는 `reference/kernel.md`, error와 stable `ValidatorResult` IDs는 `reference/mcp-api-and-schemas.md`, storage values는 `reference/storage-and-ddl.md`, `ProjectionKind` tiers와 template ownership은 `reference/document-projection.md` 및 `reference/templates/*.md`와 일치해야 합니다. |
| Glossary와 기준 기록 표현 불일치 | Official terms, capitalization, record ID prefixes, source-of-truth wording, authority-boundary phrases는 `reference/glossary.md`와 relevant owner docs에 맞아야 하며 추가 상태 권한을 암시하지 않아야 합니다. |
| TODO 준수 | `TODO_DECISION`과 `TODO_IMPLEMENT`는 허용된 의미로 쓰고 gap을 명확히 이름 붙이며, action에 필요한 owner/context를 충분히 포함하고, 완료된 기준 섹션에 `TODO_REWRITE` marker를 남기지 않습니다. |
| Owner가 아닌 문서의 중복 전체 계약 | Owner doc 밖의 전체 schema, DDL, transition table, fixture mini-language, template body, enum table, validator table, projection table, glossary definition은 짧은 요약과 owner link로 바꿉니다. |

## 리뷰 체크리스트

```text
[ ] 이 문서는 분명한 독자 상황을 돕는가?
[ ] README 진입점이 처음 읽는 사람, 사용자, 구현자, Reference 독자, 유지보수 담당자를 빠르게 안내하는가?
[ ] 시작부가 표준 패턴 또는 `reference/templates` 파일의 템플릿 전용 패턴을 따르는가?
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

정확한 세부사항을 어디에 둘지 판단할 때 이 map을 사용합니다. 이 map은 현재 문서 구조의 active owner를 식별하며, 비활성 path가 authoring workflow에 남지 않게 합니다.

| Subject | Active owner |
|---|---|
| Repo와 docs 진입점, reader routes, language choice, document list, target tree summary | repo root `README.md`; docs root `docs/README.md`; language entrypoints `docs/en/README.md`와 `docs/ko/README.md` |
| Shared reader mental model and three-space overview | `learn/overview.md` |
| Small core concept introduction | `learn/concepts.md` |
| Project purpose, target users, values, scope, non-goals, automation philosophy | `learn/purpose-and-principles.md` |
| Strategic thesis, failure model, MVP boundary, principle groups | 독자 설명은 `learn/purpose-and-principles.md`; exact contract impact는 `reference/design-quality-policies.md`와 `reference/kernel.md` |
| Kernel entities, lifecycle, gates, state transitions, close semantics, `prepare_write`, `close_task` | `reference/kernel.md` |
| Runtime architecture, three spaces in implementation detail, Core process model, artifact architecture, projection/reconcile architecture, guarantee levels | `reference/runtime-architecture.md` |
| MCP resources/tools, request/response schemas, error taxonomy, validator result schema, artifact ref shape | `reference/mcp-api-and-schemas.md` |
| SQLite DDL, migrations, storage layout, lock policy, artifact directory layout, baseline capture format, projection job table | `reference/storage-and-ddl.md` |
| MVP implementation order and stage exit criteria | `build/mvp-plan.md` |
| First runnable implementation slice | `build/first-runnable-slice.md` |
| Markdown projection principles, authority matrix, managed blocks, human-editable sections, artifact 참조 표시, template tiers, projection freshness/failure rules | `reference/document-projection.md` |
| 모든 projection template 본문과 표시 카드 형태 | `reference/templates/*.md` |
| 설계 품질 정책 계약, validator ID, severity composition 규칙, 정책 waiver 의미, 근거 기대사항, close 영향 | `reference/design-quality-policies.md` |
| User-facing conversation, status reading, user judgments, close checklist | `use/user-guide.md` |
| User/agent session procedure | `use/agent-session-flow.md` |
| Agent 접점 capability profiles, 공통 커넥터 계약, fallback 의미, Role Lens, connector conformance 개요 | `reference/agent-integration.md` |
| 접점별 recipes | `reference/surface-cookbook.md` |
| Generic capability profile examples | `reference/agent-integration.md` |
| Operator procedures, conformance fixture bodies, fixture assertion 의미, doctor/recover/reconcile/export/artifact integrity, docs-maintenance 보고 | `reference/operations-and-conformance.md` |
| Official term definitions and capitalization | `reference/glossary.md` |
| Post-MVP roadmap | `roadmap.md` |
| Documentation authoring rules | `maintain/authoring-guide.md` |
| Translation and bilingual prose rules | `maintain/translation-guide.md` |
