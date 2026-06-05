# Reference 색인

Reference는 schema, gate, 상태 전이, DDL profile, 읽기용 요약(Projection) 규칙, template body, 보안 의미, conformance 규칙, connector 동작, policy, 용어의 정확한 owner 계약이 필요할 때 사용합니다.

이 owner 문서들은 향후 하네스 서버 계약을 계획하고 검토하기 위한 문서입니다. 지금 이 저장소에 서버/런타임, Harness Runtime Home, conformance runner, 생성된 읽기용 요약 시스템, 구현이 있다는 뜻이 아닙니다.

Reference 전체를 기본으로 읽지 않습니다. 지금 앞에 있는 질문의 owner를 고른 뒤, 그 owner가 더 엄격한 세부사항을 위임할 때만 링크를 따라갑니다.

## 기준 계약 소유권 지도

하나의 계약이 여러 문서에 어울려 보일 때는 이 지도를 사용합니다. Exact field, enum value, lifecycle state, DDL, request/response shape, security guarantee, projection/template body, fixture assertion, validator ID, official terminology는 owner 문서에서만 정의합니다. 다른 문서는 독자에게 보이는 결과를 짧게 요약하고 이 지도나 owner로 연결합니다.

| 계약 영역 | 기준 owner |
|---|---|
| Core 상태, gate, lifecycle, authority invariant, `prepare_write`, Write Authorization lifecycle, `record_run`, `close_task`, blocker, waiver, 대체 불가능한 경계 | [Core Model 참조](core-model.md) |
| Public MCP/API method와 method별 request/response 동작 | Active MVP-1은 [MVP API](api/mvp-api.md), later/profile-gated method는 [API Schema Later](api/schema-later.md). |
| Shared API envelope, common response shape, read-only resource schema, shared ref, `ArtifactRef`, `ValidatorResult`, API-owned staged value set, API error surface | [API Schema Core](api/schema-core.md)와 [API Errors](api/errors.md). |
| Persisted table, column, index, check constraint, storage-owned JSON `TEXT`, runtime home layout, lock, migration, artifact storage, projection-job storage, validator-run storage | [Storage](storage.md). Storage hardening은 각 field의 lifecycle/value-set owner가 정한 값을 재사용해야 합니다. |
| Local access posture, threat boundary, asset, guarantee-level 의미, 정직한 cooperative/detective/preventive/isolated 표현 | [보안 참조](security.md) |
| Surface behavior, connector fallback, agent-facing context contract, connector capability profile, generated manifest, Role Lens behavior, surface-specific recipe | [Agent 통합 참조](agent-integration.md)와 [Surface Cookbook](surface-cookbook.md) |
| Projection, compact view, projection freshness/failure behavior, managed block, 사람이 편집할 수 있는 section, template class, artifact-ref rendering | [Projection과 Template 참조](projection-and-templates.md) |
| 전체 rendered template body, card body, template display shape | [Template 참조](templates/README.md) |
| Fixture body, fixture assertion, conformance scope, runner behavior, fixture profile, suite metadata boundary, current-phase fixture status, 축소된 Kernel Smoke queue | [Conformance Fixtures 참조](conformance-fixtures.md) |
| Operator behavior, diagnostic, staged operator surface, conformance run entrypoint, recovery/export/reconcile operation, 읽기 전용 docs-maintenance reporting entrypoint | [운영과 Conformance 참조](operations-and-conformance.md). 이후 독자 경로는 [운영 프로필](../later/operations-profile.md)을 사용합니다. |
| 향후 scenario family 목록, 승격 조건, suite-family label, catalog-only future candidate | [향후 Fixtures](../later/future-fixtures.md) |
| 용어, capitalization, official term wording, record-name orientation, owner routing | [용어집 참조](glossary.md) |
| Runtime space, Core process placement, Core-only canonical mutation authority, transaction ordering, artifact/projection/reconcile placement, architecture-level recovery overview | [런타임 아키텍처 참조](runtime-architecture.md) |
| Design-quality policy, policy-to-validator mapping, stable validator ID, severity composition, waiver semantics, evidence expectation, design-quality close impact | [설계 품질 정책](design-quality-policies.md) |
| Documentation drift rule, bilingual parity, strict-contract ownership rule, link hygiene, translation guidance | [문서 작성 가이드](../maintain/authoring-guide.md), [번역 가이드](../maintain/translation-guide.md), [English Authoring Guide](../../en/maintain/authoring-guide.md), [English Translation Guide](../../en/maintain/translation-guide.md). |

이 지도는 strict contract owner를 찾기 위한 것입니다. 여러 owner 문서군을 가로지르는 사전 구현 문서 정비 축은 [문서 작성 가이드의 정비 대상 owner 지도](../maintain/authoring-guide.md#사전-구현-문서-정비-대상-owner-지도)를 사용합니다. 그 지도는 docs-maintenance 지침일 뿐이며 문서 수락, manual acceptance, runtime conformance, close readiness, implementation readiness를 결정하지 않습니다.

## 독자별 바로가기

- 향후 서버 구현자는 [구현 개요](../build/implementation-overview.md)에서 시작한 뒤 [MVP-1 사용자 작업 루프](../build/mvp-user-work-loop.md) -> [MVP API](api/mvp-api.md) -> [Storage](storage.md) 순서로 봅니다. 다른 Reference owner는 정확한 질문이 있을 때만 엽니다.
- 첫 내부 점검을 계획한다면 [내부 엔지니어링 점검](../build/engineering-checkpoint.md)에서 시작한 뒤 [Core Model 참조](core-model.md), [MVP API](api/mvp-api.md), [Storage](storage.md)를 사용합니다.
- 에이전트 지침을 작성한다면 [에이전트 가이드](../use/agent-guide.md)에서 시작한 뒤 connector-specific 계약이 필요할 때만 [Agent 통합 참조](agent-integration.md)와 [Surface Cookbook](surface-cookbook.md)을 사용합니다.
- MVP-1 method를 확인한다면 [MVP API](api/mvp-api.md)에서 시작합니다. Shared ref나 envelope를 확인한다면 [API Schema Core](api/schema-core.md)를 사용합니다. Later method는 [API Schema Later](api/schema-later.md)를 사용하되, 승격 전에는 MVP 경로에 넣지 않습니다.
- Persisted shape를 확인한다면 [Storage](storage.md)에서 시작합니다.
- `harness://` resource를 확인한다면 URI를 delivery stage requirement로 취급하기 전에 staged [Read-only resources](api/schema-core.md#read-only-resources) table에서 시작합니다.
- 사용자에게 보이는 문구 주장을 확인한다면 그 밑의 사실을 담당하는 owner에서 시작합니다. 읽기용 요약(Projection)과 template 문서는 표시를 담당하지만 권한을 만들지 않습니다.
- 향후 보증, 운영, fixture catalog material을 확인한다면 [보증 프로필](../later/assurance-profile.md), [운영 프로필](../later/operations-profile.md), [향후 Fixtures](../later/future-fixtures.md)를 사용합니다. 이 경로는 MVP 구현 경로가 아닙니다.
