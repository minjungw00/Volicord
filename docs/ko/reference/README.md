# Reference 색인

Reference는 schema, gate, 상태 전이, DDL profile, 읽기용 요약(Projection) 규칙, template body, 보안 의미, conformance 규칙, connector 동작, policy, 용어의 정확한 owner 계약이 필요할 때 사용합니다.

이 owner 문서들은 향후 하네스 서버 계약을 계획하고 검토하기 위한 문서입니다. 지금 이 저장소에 서버/런타임, Harness Runtime Home, conformance runner, 생성된 읽기용 요약 시스템, 구현이 있다는 뜻이 아닙니다.

Reference 전체를 기본으로 읽지 않습니다. 지금 앞에 있는 질문의 owner를 고른 뒤, 그 owner가 더 엄격한 세부사항을 위임할 때만 링크를 따라갑니다.

## Owner-Contract 지도

| 질문 | 계약 owner |
|---|---|
| 기준 Core 상태 동작이 무엇인가? | [커널 참조](kernel.md)가 entity, gate, 상태 전이, 쓰기 권한, `prepare_write`, `record_run`, `close_task`, 닫기 의미를 담당합니다. |
| Public API 또는 schema shape가 무엇인가? | [MVP API](api/mvp-api.md)가 active MVP-1 method를 담당하고, [API Schema Core](api/schema-core.md)가 shared envelope/ref/resource를 담당하며, [API Errors](api/errors.md)가 error/idempotency/state conflict behavior를 담당하고, [API Schema Later](api/schema-later.md)가 later-profile method와 future schema material을 담당합니다. |
| Runtime state는 어디에 저장되는가? | [Storage와 DDL](storage-and-ddl.md)이 runtime layout, DDL profile, storage JSON, lock, artifact, migration, baseline, projection job, validator storage를 담당합니다. |
| 읽기용 문서는 어떻게 동작하는가? | [문서 Projection 참조](document-projection.md)가 projection 규칙, freshness, managed block, 권한 경계를 담당하고, [Template 참조](templates/README.md)가 rendered Markdown shape를 담당합니다. |
| 하네스가 어떤 보안 보장을 주장할 수 있는가? | [보안 위협 모델 참조](security-threat-model.md)가 asset, trust boundary, threat, control, guarantee level, 정직한 보안 표현을 담당합니다. |
| 에이전트 surface는 어떻게 통합하는가? | [Agent 통합 참조](agent-integration.md)가 connector profile, context push/pull, fallback behavior, generated-manifest boundary를 담당하고, [Surface Cookbook](surface-cookbook.md)이 surface recipe를 담당합니다. |
| Operator와 conformance 작성자는 무엇을 사용하는가? | [운영과 Conformance 참조](operations-and-conformance.md)가 operator behavior와 conformance run entrypoint를 담당하고, [Conformance Fixtures 참조](conformance-fixtures.md)가 fixture mechanics와 Kernel Smoke queue를 담당합니다. |
| 이후 fixture scenario는 어디에 있는가? | [향후 Fixture Catalog](future-fixture-catalog.md)가 상세 future scenario, coverage map, catalog-only candidate를 담당합니다. |
| Design-quality check는 무엇이 담당하는가? | [설계 품질 정책](design-quality-policies.md)이 policy, validator ID, severity composition, waiver semantics, close impact를 담당합니다. |
| 용어의 뜻은 어디서 확인하는가? | [용어집 참조](glossary.md)가 public/internal terminology definition과 owner routing을 담당합니다. |
| Runtime piece들은 어떻게 맞물리는가? | [런타임 아키텍처 참조](runtime-architecture.md)가 runtime space, Core transaction placement, architecture flow, artifact, projection/reconcile placement, recovery overview를 담당합니다. |

## 독자별 바로가기

- 향후 서버 구현자는 [Build](../build/implementation-overview.md)에서 시작한 뒤, 필요한 owner 계약을 여기서 고릅니다.
- 에이전트 통합자는 [에이전트 세션 흐름](../use/agent-session-flow.md)에서 시작한 뒤 [Agent 통합 참조](agent-integration.md)와 [Surface Cookbook](surface-cookbook.md)을 사용합니다.
- MVP-1 method를 확인한다면 [MVP API](api/mvp-api.md)에서 시작합니다. Shared ref나 envelope를 확인한다면 [API Schema Core](api/schema-core.md)를 사용합니다. Later method는 [API Schema Later](api/schema-later.md)를 사용합니다.
- Persisted shape를 확인한다면 [Storage와 DDL](storage-and-ddl.md)에서 시작합니다.
- `harness://` resource를 확인한다면 URI를 delivery stage requirement로 취급하기 전에 staged [Read-only resources](api/schema-core.md#read-only-resources) table에서 시작합니다.
- 사용자에게 보이는 문구 주장을 확인한다면 그 밑의 사실을 담당하는 owner에서 시작합니다. 읽기용 요약(Projection)과 template 문서는 표시를 담당하지만 권한을 만들지 않습니다.
